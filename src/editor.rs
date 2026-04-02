use gdk4 as gdk;
use gtk4 as gtk;
use gtk4::prelude::*;
use gtk4::glib;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::tools::{Annotation, Stroke};
use crate::tools::brush::BrushTool;
use crate::utils;

struct EditorState {
    image_surface: cairo::ImageSurface,
    file_path: PathBuf,
    annotations: Vec<Stroke>,
    brush: BrushTool,
}

pub fn open_editor(app: &libadwaita::Application, path: PathBuf) {
    let surface = {
        let mut file = std::fs::File::open(&path).expect("cannot open image file");
        cairo::ImageSurface::create_from_png(&mut file).expect("cannot decode PNG")
    };

    let state = Rc::new(RefCell::new(EditorState {
        image_surface: surface,
        file_path: path.clone(),
        annotations: Vec::new(),
        brush: BrushTool::new(),
    }));

    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("snip")
        .default_width(800)
        .default_height(600)
        .build();

    // === HeaderBar ===
    let header = libadwaita::HeaderBar::new();

    let brush_btn = gtk::ToggleButton::builder()
        .label("Кисть")
        .active(true)
        .build();
    header.pack_start(&brush_btn);

    let copy_btn = gtk::Button::builder().label("Копировать").build();
    let path_btn = gtk::Button::builder().label("Путь").build();
    let save_btn = gtk::Button::builder().label("Сохранить").build();
    save_btn.add_css_class("suggested-action");

    header.pack_end(&save_btn);
    header.pack_end(&path_btn);
    header.pack_end(&copy_btn);

    // === Canvas ===
    let drawing_area = gtk::DrawingArea::new();
    drawing_area.set_vexpand(true);
    drawing_area.set_hexpand(true);

    {
        let state = state.clone();
        drawing_area.set_draw_func(move |_da, cr, width, height| {
            let state = state.borrow();

            cr.set_source_rgb(0.12, 0.12, 0.12);
            let _ = cr.paint();

            let img_w = state.image_surface.width() as f64;
            let img_h = state.image_surface.height() as f64;
            let scale = (width as f64 / img_w).min(height as f64 / img_h).min(1.0);
            let offset_x = (width as f64 - img_w * scale) / 2.0;
            let offset_y = (height as f64 - img_h * scale) / 2.0;

            cr.save().ok();
            cr.translate(offset_x, offset_y);
            cr.scale(scale, scale);

            let _ = cr.set_source_surface(&state.image_surface, 0.0, 0.0);
            let _ = cr.paint();

            for annotation in &state.annotations {
                annotation.draw(cr);
            }

            if let Some(stroke) = state.brush.current_stroke() {
                stroke.draw(cr);
            }

            cr.restore().ok();
        });
    }

    // === Mouse events ===
    let press_gesture = gtk::GestureClick::new();
    press_gesture.set_button(1);

    {
        let state = state.clone();
        let da = drawing_area.clone();
        press_gesture.connect_pressed(move |_g, _n, x, y| {
            let (ix, iy) = screen_to_image(&state.borrow(), &da, x, y);
            state.borrow_mut().brush.press(ix, iy);
            da.queue_draw();
        });
    }

    {
        let state = state.clone();
        let da = drawing_area.clone();
        press_gesture.connect_released(move |_g, _n, _x, _y| {
            let mut st = state.borrow_mut();
            if let Some(stroke) = st.brush.release() {
                st.annotations.push(stroke);
            }
            drop(st);
            da.queue_draw();
        });
    }

    let motion = gtk::EventControllerMotion::new();
    {
        let state = state.clone();
        let da = drawing_area.clone();
        motion.connect_motion(move |_ctrl, x, y| {
            let (ix, iy) = screen_to_image(&state.borrow(), &da, x, y);
            state.borrow_mut().brush.motion(ix, iy);
            da.queue_draw();
        });
    }

    drawing_area.add_controller(press_gesture);
    drawing_area.add_controller(motion);

    // === Bottom bar ===
    let bottom_bar = build_bottom_bar(state.clone(), drawing_area.clone());

    // === "Копировать" button ===
    {
        let state = state.clone();
        copy_btn.connect_clicked(move |btn| {
            let st = state.borrow();
            match render_final_surface(&st) {
                Ok(final_surface) => {
                    let texture = surface_to_texture(&final_surface);
                    let clipboard = btn.clipboard();
                    clipboard.set_texture(&texture);
                }
                Err(e) => eprintln!("Failed to render: {e}"),
            }
        });
    }

    // === "Путь" button ===
    {
        let state = state.clone();
        path_btn.connect_clicked(move |btn| {
            let st = state.borrow();
            let path_str = st.file_path.to_string_lossy().to_string();
            let clipboard = btn.clipboard();
            clipboard.set_text(&path_str);
        });
    }

    // === "Сохранить" button ===
    {
        let state = state.clone();
        save_btn.connect_clicked(move |_btn| {
            let st = state.borrow();
            match render_final_surface(&st) {
                Ok(final_surface) => {
                    if let Err(e) = utils::save_surface_as_png(&final_surface, &st.file_path) {
                        eprintln!("Failed to save: {e}");
                    }
                }
                Err(e) => eprintln!("Failed to render: {e}"),
            }
        });
    }

    // === Assemble window ===
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let toolbar_view = libadwaita::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&drawing_area));
    content.append(&toolbar_view);
    content.append(&bottom_bar);

    window.set_content(Some(&content));
    window.present();
}

fn screen_to_image(state: &EditorState, da: &gtk::DrawingArea, x: f64, y: f64) -> (f64, f64) {
    let img_w = state.image_surface.width() as f64;
    let img_h = state.image_surface.height() as f64;
    let w = da.width() as f64;
    let h = da.height() as f64;
    let scale = (w / img_w).min(h / img_h).min(1.0);
    let offset_x = (w - img_w * scale) / 2.0;
    let offset_y = (h - img_h * scale) / 2.0;
    ((x - offset_x) / scale, (y - offset_y) / scale)
}

fn render_final_surface(state: &EditorState) -> Result<cairo::ImageSurface, cairo::Error> {
    let w = state.image_surface.width();
    let h = state.image_surface.height();
    let result = cairo::ImageSurface::create(cairo::Format::ARgb32, w, h)?;
    let cr = cairo::Context::new(&result)?;

    cr.set_source_surface(&state.image_surface, 0.0, 0.0)?;
    cr.paint()?;

    for annotation in &state.annotations {
        annotation.draw(&cr);
    }

    drop(cr);
    Ok(result)
}

fn surface_to_texture(surface: &cairo::ImageSurface) -> gdk::Texture {
    let mut png_data: Vec<u8> = Vec::new();
    surface.write_to_png(&mut png_data).expect("PNG write failed");
    let bytes = glib::Bytes::from(&png_data);
    gdk::Texture::from_bytes(&bytes).expect("Texture from PNG failed")
}

fn build_bottom_bar(state: Rc<RefCell<EditorState>>, _da: gtk::DrawingArea) -> gtk::Box {
    let bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    let color_label = gtk::Label::new(Some("Цвет:"));
    color_label.add_css_class("dim-label");
    bar.append(&color_label);

    let colors: Vec<(&str, gdk::RGBA)> = vec![
        ("red", gdk::RGBA::new(1.0, 0.2, 0.2, 1.0)),
        ("green", gdk::RGBA::new(0.2, 0.8, 0.2, 1.0)),
        ("yellow", gdk::RGBA::new(1.0, 0.85, 0.0, 1.0)),
        ("blue", gdk::RGBA::new(0.3, 0.5, 1.0, 1.0)),
        ("pink", gdk::RGBA::new(1.0, 0.4, 1.0, 1.0)),
    ];

    // Получаем display для добавления CSS-провайдеров
    let display = gdk::Display::default().expect("cannot get default display");

    for (name, rgba) in &colors {
        let btn = gtk::Button::new();
        btn.set_size_request(28, 28);
        // Уникальный CSS-класс для каждой кнопки цвета
        let css_class = format!("snip-color-{}", name);
        let css = format!(
            "button.{} {{ background: rgba({},{},{},{}); border-radius: 50%; min-width: 28px; min-height: 28px; padding: 0; }}",
            css_class,
            (rgba.red() * 255.0) as u8,
            (rgba.green() * 255.0) as u8,
            (rgba.blue() * 255.0) as u8,
            rgba.alpha(),
        );
        let provider = gtk::CssProvider::new();
        provider.load_from_string(&css);
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        btn.add_css_class(&css_class);

        let color = *rgba;
        let state = state.clone();
        btn.connect_clicked(move |_| {
            state.borrow_mut().brush.color = color;
        });
        bar.append(&btn);
    }

    // Separator
    let sep = gtk::Separator::new(gtk::Orientation::Vertical);
    bar.append(&sep);

    // Thickness label
    let width_label = gtk::Label::new(Some("Толщина:"));
    width_label.add_css_class("dim-label");
    bar.append(&width_label);

    let widths: Vec<(f64, i32, usize)> = vec![(2.0, 10, 0), (4.0, 16, 1), (8.0, 22, 2)];
    for (line_width, btn_size, idx) in &widths {
        let btn = gtk::Button::new();
        btn.set_size_request(*btn_size, *btn_size);
        let css_class = format!("snip-thickness-{}", idx);
        let css = format!(
            "button.{} {{ border-radius: 50%; min-width: {}px; min-height: {}px; padding: 0; }}",
            css_class, btn_size, btn_size,
        );
        let provider = gtk::CssProvider::new();
        provider.load_from_string(&css);
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        btn.add_css_class(&css_class);

        let w = *line_width;
        let state = state.clone();
        btn.connect_clicked(move |_| {
            state.borrow_mut().brush.width = w;
        });
        bar.append(&btn);
    }

    bar
}
