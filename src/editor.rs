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
    redo_stack: Vec<Stroke>,
    brush: BrushTool,
}

pub fn open_editor(app: &libadwaita::Application, path: PathBuf) {
    let surface = {
        let mut file = std::fs::File::open(&path).expect("cannot open image file");
        cairo::ImageSurface::create_from_png(&mut file).expect("cannot decode PNG")
    };

    let img_w = surface.width();
    let img_h = surface.height();

    let state = Rc::new(RefCell::new(EditorState {
        image_surface: surface,
        file_path: path.clone(),
        annotations: Vec::new(),
        redo_stack: Vec::new(),
        brush: BrushTool::new(),
    }));

    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("snip")
        .default_width(img_w)
        .default_height(img_h + 90) // запас на headerbar + toolbar
        .build();

    // === HeaderBar (минимальный — только title + close) ===
    let header = libadwaita::HeaderBar::new();

    // Кнопки создаём здесь, компонуем в bottom bar
    let undo_btn = gtk::Button::builder().icon_name("edit-undo-symbolic").tooltip_text("Отменить (Ctrl+Z)").build();
    let redo_btn = gtk::Button::builder().icon_name("edit-redo-symbolic").tooltip_text("Повторить (Ctrl+Shift+Z)").build();
    let copy_btn = gtk::Button::builder().label("Копировать").build();
    let path_btn = gtk::Button::builder().label("Путь").build();
    let save_btn = gtk::Button::builder().label("Сохранить").build();
    save_btn.add_css_class("suggested-action");

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
                st.redo_stack.clear(); // новое действие сбрасывает redo
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

    // === Undo button ===
    {
        let state = state.clone();
        let da = drawing_area.clone();
        undo_btn.connect_clicked(move |_| {
            let mut st = state.borrow_mut();
            if let Some(stroke) = st.annotations.pop() {
                st.redo_stack.push(stroke);
            }
            drop(st);
            da.queue_draw();
        });
    }

    // === Redo button ===
    {
        let state = state.clone();
        let da = drawing_area.clone();
        redo_btn.connect_clicked(move |_| {
            let mut st = state.borrow_mut();
            if let Some(stroke) = st.redo_stack.pop() {
                st.annotations.push(stroke);
            }
            drop(st);
            da.queue_draw();
        });
    }

    // === Keyboard shortcuts: Ctrl+Z (undo), Ctrl+Shift+Z (redo) ===
    let key_controller = gtk::EventControllerKey::new();
    {
        let state = state.clone();
        let da = drawing_area.clone();
        key_controller.connect_key_pressed(move |_ctrl, key, _code, mods| {
            let ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
            let shift = mods.contains(gdk::ModifierType::SHIFT_MASK);

            if ctrl && key == gdk::Key::z && !shift {
                // Undo
                let mut st = state.borrow_mut();
                if let Some(stroke) = st.annotations.pop() {
                    st.redo_stack.push(stroke);
                }
                drop(st);
                da.queue_draw();
                return glib::Propagation::Stop;
            }

            if ctrl && (key == gdk::Key::Z || (key == gdk::Key::z && shift)) {
                // Redo
                let mut st = state.borrow_mut();
                if let Some(stroke) = st.redo_stack.pop() {
                    st.annotations.push(stroke);
                }
                drop(st);
                da.queue_draw();
                return glib::Propagation::Stop;
            }

            glib::Propagation::Proceed
        });
    }
    window.add_controller(key_controller);

    // === Bottom bar ===
    let bottom_bar = build_bottom_bar(
        state.clone(),
        drawing_area.clone(),
        &undo_btn,
        &redo_btn,
        &copy_btn,
        &path_btn,
        &save_btn,
    );

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

fn build_bottom_bar(
    state: Rc<RefCell<EditorState>>,
    _da: gtk::DrawingArea,
    undo_btn: &gtk::Button,
    redo_btn: &gtk::Button,
    copy_btn: &gtk::Button,
    path_btn: &gtk::Button,
    save_btn: &gtk::Button,
) -> gtk::Box {
    let bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    let display = gdk::Display::default().expect("cannot get default display");

    // CSS для цветовых кнопок: белая обводка при активном состоянии
    let global_css = gtk::CssProvider::new();
    global_css.load_from_string(
        "button.snip-color:checked { outline: 2px solid white; outline-offset: 2px; }
         button.snip-color { outline: none; }",
    );
    gtk::style_context_add_provider_for_display(
        &display,
        &global_css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // --- Цвета ---
    let colors: Vec<(&str, gdk::RGBA)> = vec![
        ("red", gdk::RGBA::new(1.0, 0.2, 0.2, 1.0)),
        ("green", gdk::RGBA::new(0.2, 0.8, 0.2, 1.0)),
        ("yellow", gdk::RGBA::new(1.0, 0.85, 0.0, 1.0)),
        ("blue", gdk::RGBA::new(0.3, 0.5, 1.0, 1.0)),
        ("pink", gdk::RGBA::new(1.0, 0.4, 1.0, 1.0)),
    ];

    let mut color_group: Option<gtk::ToggleButton> = None;

    for (i, (name, rgba)) in colors.iter().enumerate() {
        let btn = gtk::ToggleButton::new();
        btn.set_size_request(28, 28);
        btn.add_css_class("snip-color");
        if i == 0 {
            btn.set_active(true);
            color_group = Some(btn.clone());
        } else if let Some(ref group) = color_group {
            btn.set_group(Some(group));
        }

        let css_class = format!("snip-color-{}", name);
        let css = format!(
            "button.{} {{ background: rgba({},{},{},{}); border-radius: 4px; min-width: 28px; min-height: 28px; padding: 0; }}",
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

    bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));

    // --- Толщина ---
    let widths: Vec<f64> = vec![2.0, 4.0, 8.0];
    let mut width_group: Option<gtk::ToggleButton> = None;

    for (i, line_width) in widths.iter().enumerate() {
        let label = format!("{}px", *line_width as i32);
        let btn = gtk::ToggleButton::with_label(&label);
        if i == 0 {
            btn.set_active(true);
            width_group = Some(btn.clone());
        } else if let Some(ref group) = width_group {
            btn.set_group(Some(group));
        }

        let w = *line_width;
        let state = state.clone();
        btn.connect_clicked(move |_| {
            state.borrow_mut().brush.width = w;
        });
        bar.append(&btn);
    }

    bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));

    // --- Undo / Redo ---
    bar.append(undo_btn);
    bar.append(redo_btn);

    // Spacer — прижимает действия с файлом вправо
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    bar.append(&spacer);

    // --- Действия с файлом ---
    bar.append(copy_btn);
    bar.append(path_btn);
    bar.append(save_btn);

    bar
}
