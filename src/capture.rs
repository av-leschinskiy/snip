use gdk4 as gdk;
use gtk4 as gtk;
use gtk4::prelude::*;
use gtk4::glib;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::utils;

/// Запускает capture flow: portal screenshot → overlay → crop → save → return path.
/// Вызывает `on_done(path)` когда скриншот сохранён.
pub fn start_capture(app: &libadwaita::Application, on_done: impl Fn(PathBuf) + 'static) {
    let app = app.clone();
    glib::spawn_future_local(async move {
        match take_portal_screenshot().await {
            Ok(uri) => {
                let path = uri_to_path(&uri);
                match load_screenshot(&path) {
                    Ok(surface) => show_overlay(&app, surface, on_done),
                    Err(e) => eprintln!("Failed to load screenshot: {e}"),
                }
            }
            Err(e) => {
                eprintln!("Portal screenshot cancelled or failed: {e}");
                app.quit();
            }
        }
    });
}

async fn take_portal_screenshot() -> Result<String, Box<dyn std::error::Error>> {
    use ashpd::desktop::screenshot::Screenshot;

    let response = Screenshot::request()
        .interactive(false)
        .send()
        .await?
        .response()?;
    Ok(response.uri().to_string())
}

fn uri_to_path(uri: &str) -> PathBuf {
    if let Some(path) = uri.strip_prefix("file://") {
        PathBuf::from(path)
    } else {
        PathBuf::from(uri)
    }
}

fn load_screenshot(path: &std::path::Path) -> Result<cairo::ImageSurface, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(path)?;
    let surface = cairo::ImageSurface::create_from_png(&mut file)?;
    Ok(surface)
}

fn show_overlay(
    app: &libadwaita::Application,
    surface: cairo::ImageSurface,
    on_done: impl Fn(PathBuf) + 'static,
) {
    let window = gtk::Window::builder()
        .application(app)
        .decorated(false)
        .fullscreened(true)
        .build();

    let img_width = surface.width();
    let img_height = surface.height();

    let surface = Rc::new(surface);
    let on_done = Rc::new(on_done);

    let start_x = Rc::new(Cell::new(0.0f64));
    let start_y = Rc::new(Cell::new(0.0f64));
    let cur_x = Rc::new(Cell::new(0.0f64));
    let cur_y = Rc::new(Cell::new(0.0f64));
    let selecting = Rc::new(Cell::new(false));

    let drawing_area = gtk::DrawingArea::new();
    drawing_area.set_content_width(img_width);
    drawing_area.set_content_height(img_height);

    {
        let surface = surface.clone();
        let start_x = start_x.clone();
        let start_y = start_y.clone();
        let cur_x = cur_x.clone();
        let cur_y = cur_y.clone();
        let selecting = selecting.clone();

        drawing_area.set_draw_func(move |_da, cr, width, height| {
            let scale_x = width as f64 / surface.width() as f64;
            let scale_y = height as f64 / surface.height() as f64;
            cr.scale(scale_x, scale_y);
            let _ = cr.set_source_surface(&*surface, 0.0, 0.0);
            let _ = cr.paint();
            cr.identity_matrix();

            cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
            let _ = cr.paint();

            if selecting.get() {
                let sx = start_x.get();
                let sy = start_y.get();
                let cx = cur_x.get();
                let cy = cur_y.get();

                let rx = sx.min(cx);
                let ry = sy.min(cy);
                let rw = (cx - sx).abs();
                let rh = (cy - sy).abs();

                cr.save().ok();
                cr.rectangle(rx, ry, rw, rh);
                cr.clip();
                cr.scale(scale_x, scale_y);
                let _ = cr.set_source_surface(&*surface, 0.0, 0.0);
                let _ = cr.paint();
                cr.restore().ok();

                cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
                cr.set_line_width(1.5);
                cr.rectangle(rx, ry, rw, rh);
                let _ = cr.stroke();
            }
        });
    }

    drawing_area.set_cursor_from_name(Some("crosshair"));

    let press_gesture = gtk::GestureClick::new();
    press_gesture.set_button(1);

    {
        let start_x = start_x.clone();
        let start_y = start_y.clone();
        let selecting = selecting.clone();

        press_gesture.connect_pressed(move |_gesture, _n, x, y| {
            start_x.set(x);
            start_y.set(y);
            selecting.set(true);
        });
    }

    {
        let surface = surface.clone();
        let start_x = start_x.clone();
        let start_y = start_y.clone();
        let selecting = selecting.clone();
        let window_clone = window.clone();
        let on_done = on_done.clone();

        press_gesture.connect_released(move |gesture, _n, x, y| {
            if !selecting.get() {
                return;
            }
            selecting.set(false);

            let sx = start_x.get();
            let sy = start_y.get();

            let widget = gesture.widget().expect("gesture has no widget");
            let w = widget.width() as f64;
            let h = widget.height() as f64;
            let scale_x = surface.width() as f64 / w;
            let scale_y = surface.height() as f64 / h;

            let rx = (sx.min(x) * scale_x) as i32;
            let ry = (sy.min(y) * scale_y) as i32;
            let rw = ((x - sx).abs() * scale_x) as i32;
            let rh = ((y - sy).abs() * scale_y) as i32;

            if rw < 5 || rh < 5 {
                return;
            }

            match utils::crop_surface(&surface, rx, ry, rw, rh) {
                Ok(cropped) => {
                    match utils::new_screenshot_path() {
                        Ok(path) => {
                            if let Err(e) = utils::save_surface_as_png(&cropped, &path) {
                                eprintln!("Failed to save: {e}");
                                return;
                            }
                            window_clone.close();
                            on_done(path);
                        }
                        Err(e) => eprintln!("Failed to create path: {e}"),
                    }
                }
                Err(e) => eprintln!("Failed to crop: {e}"),
            }
        });
    }

    let motion_controller = gtk::EventControllerMotion::new();
    {
        let cur_x = cur_x.clone();
        let cur_y = cur_y.clone();
        let selecting = selecting.clone();
        let da = drawing_area.clone();

        motion_controller.connect_motion(move |_ctrl, x, y| {
            if selecting.get() {
                cur_x.set(x);
                cur_y.set(y);
                da.queue_draw();
            }
        });
    }

    let key_controller = gtk::EventControllerKey::new();
    {
        let window_clone = window.clone();
        let app_clone = app.clone();
        key_controller.connect_key_pressed(move |_ctrl, key, _code, _mods| {
            if key == gdk::Key::Escape {
                window_clone.close();
                app_clone.quit();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }

    drawing_area.add_controller(press_gesture);
    drawing_area.add_controller(motion_controller);
    window.add_controller(key_controller);
    window.set_child(Some(&drawing_area));
    window.present();
}
