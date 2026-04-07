use gdk4 as gdk;
use gtk4 as gtk;
use gtk4::prelude::*;
use gtk4::glib;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::tools::{Annotation, AnnotationItem};
use crate::tools::brush::BrushTool;
use crate::tools::rect::RectTool;
use crate::utils;

#[derive(Clone, Copy, PartialEq)]
enum ActiveTool {
    Brush,
    Rect,
}

struct EditorState {
    image_surface: cairo::ImageSurface,
    file_path: PathBuf,
    annotations: Vec<AnnotationItem>,
    redo_stack: Vec<AnnotationItem>,
    active_tool: ActiveTool,
    brush: BrushTool,
    rect: RectTool,
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
        active_tool: ActiveTool::Brush,
        brush: BrushTool::new(),
        rect: RectTool::new(),
    }));

    let filename = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "snip".to_string());

    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title(&filename)
        .default_width(img_w)
        .default_height(img_h + 90) // запас на headerbar + toolbar
        .build();

    // === HeaderBar (минимальный — только title + close) ===
    let header = libadwaita::HeaderBar::new();

    // Кнопки создаём здесь, компонуем в bottom bar
    let undo_btn = gtk::Button::builder().icon_name("edit-undo-symbolic").tooltip_text("Отменить (Ctrl+Z)").sensitive(false).build();
    let redo_btn = gtk::Button::builder().icon_name("edit-redo-symbolic").tooltip_text("Повторить (Ctrl+Shift+Z)").sensitive(false).build();
    let copy_btn = gtk::Button::builder().label("Копировать").build();
    let path_btn = gtk::Button::builder().label("Путь").build();

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

            // Preview текущего инструмента
            if let Some(stroke) = state.brush.current_stroke() {
                stroke.draw(cr);
            }
            if let Some(rect) = state.rect.current_rect(state.brush.color, state.brush.width) {
                rect.draw(cr);
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
            let mut st = state.borrow_mut();
            match st.active_tool {
                ActiveTool::Brush => st.brush.press(ix, iy),
                ActiveTool::Rect => st.rect.press(ix, iy),
            }
            drop(st);
            da.queue_draw();
        });
    }

    {
        let state = state.clone();
        let da = drawing_area.clone();
        let undo_btn = undo_btn.clone();
        let redo_btn = redo_btn.clone();
        let window = window.clone();
        let filename = filename.clone();
        press_gesture.connect_released(move |_g, _n, _x, _y| {
            let mut st = state.borrow_mut();
            match st.active_tool {
                ActiveTool::Brush => {
                    if let Some(stroke) = st.brush.release() {
                        st.redo_stack.clear();
                        st.annotations.push(AnnotationItem::Stroke(stroke));
                    }
                }
                ActiveTool::Rect => {
                    let color = st.brush.color;
                    let width = st.brush.width;
                    if let Some(rect) = st.rect.release(color, width) {
                        st.redo_stack.clear();
                        st.annotations.push(AnnotationItem::Rect(rect));
                    }
                }
            }
            let has_changes = !st.annotations.is_empty();
            undo_btn.set_sensitive(has_changes);
            redo_btn.set_sensitive(!st.redo_stack.is_empty());
            drop(st);
            update_title(&window, &filename, has_changes);
            da.queue_draw();
        });
    }

    let motion = gtk::EventControllerMotion::new();
    {
        let state = state.clone();
        let da = drawing_area.clone();
        motion.connect_motion(move |_ctrl, x, y| {
            let (ix, iy) = screen_to_image(&state.borrow(), &da, x, y);
            let mut st = state.borrow_mut();
            match st.active_tool {
                ActiveTool::Brush => st.brush.motion(ix, iy),
                ActiveTool::Rect => st.rect.motion(ix, iy),
            }
            drop(st);
            da.queue_draw();
        });
    }

    drawing_area.add_controller(press_gesture);
    drawing_area.add_controller(motion);

    // === Undo button ===
    {
        let state = state.clone();
        let da = drawing_area.clone();
        let undo_btn2 = undo_btn.clone();
        let redo_btn2 = redo_btn.clone();
        let window = window.clone();
        let filename = filename.clone();
        undo_btn.connect_clicked(move |_| {
            let mut st = state.borrow_mut();
            if let Some(stroke) = st.annotations.pop() {
                st.redo_stack.push(stroke);
            }
            let has_changes = !st.annotations.is_empty();
            undo_btn2.set_sensitive(has_changes);
            redo_btn2.set_sensitive(!st.redo_stack.is_empty());
            drop(st);
            update_title(&window, &filename, has_changes);
            da.queue_draw();
        });
    }

    // === Redo button ===
    {
        let state = state.clone();
        let da = drawing_area.clone();
        let undo_btn2 = undo_btn.clone();
        let redo_btn2 = redo_btn.clone();
        let window = window.clone();
        let filename = filename.clone();
        redo_btn.connect_clicked(move |_| {
            let mut st = state.borrow_mut();
            if let Some(stroke) = st.redo_stack.pop() {
                st.annotations.push(stroke);
            }
            let has_changes = !st.annotations.is_empty();
            undo_btn2.set_sensitive(has_changes);
            redo_btn2.set_sensitive(!st.redo_stack.is_empty());
            drop(st);
            update_title(&window, &filename, has_changes);
            da.queue_draw();
        });
    }

    // === Keyboard shortcuts: Ctrl+Z (undo), Ctrl+Shift+Z (redo) ===
    let key_controller = gtk::EventControllerKey::new();
    {
        let state = state.clone();
        let da = drawing_area.clone();
        let undo_btn2 = undo_btn.clone();
        let redo_btn2 = redo_btn.clone();
        let window = window.clone();
        let filename = filename.clone();
        key_controller.connect_key_pressed(move |_ctrl, key, _code, mods| {
            let ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
            let shift = mods.contains(gdk::ModifierType::SHIFT_MASK);

            if ctrl && key == gdk::Key::z && !shift {
                let mut st = state.borrow_mut();
                if let Some(stroke) = st.annotations.pop() {
                    st.redo_stack.push(stroke);
                }
                let has_changes = !st.annotations.is_empty();
                undo_btn2.set_sensitive(has_changes);
                redo_btn2.set_sensitive(!st.redo_stack.is_empty());
                drop(st);
                update_title(&window, &filename, has_changes);
                da.queue_draw();
                return glib::Propagation::Stop;
            }

            if ctrl && (key == gdk::Key::Z || (key == gdk::Key::z && shift)) {
                let mut st = state.borrow_mut();
                if let Some(stroke) = st.redo_stack.pop() {
                    st.annotations.push(stroke);
                }
                let has_changes = !st.annotations.is_empty();
                undo_btn2.set_sensitive(has_changes);
                redo_btn2.set_sensitive(!st.redo_stack.is_empty());
                drop(st);
                update_title(&window, &filename, has_changes);
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

    // === Автосохранение при закрытии ===
    {
        let state = state.clone();
        window.connect_close_request(move |_| {
            let st = state.borrow();
            if !st.annotations.is_empty() {
                if let Ok(final_surface) = render_final_surface(&st) {
                    let mut file = match std::fs::File::create(&st.file_path) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Failed to save on close: {e}");
                            return glib::Propagation::Proceed;
                        }
                    };
                    if let Err(e) = final_surface.write_to_png(&mut file) {
                        eprintln!("Failed to write PNG on close: {e}");
                    }
                }
            }
            glib::Propagation::Proceed
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

fn update_title(window: &libadwaita::ApplicationWindow, filename: &str, has_changes: bool) {
    if has_changes {
        window.set_title(Some(&format!("{} •", filename)));
    } else {
        window.set_title(Some(filename));
    }
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
) -> gtk::Box {
    let bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    // --- Переключатель инструментов ---
    let brush_toggle = gtk::ToggleButton::with_label("Кисть");
    let rect_toggle = gtk::ToggleButton::with_label("Прямоугольник");
    rect_toggle.set_group(Some(&brush_toggle));
    brush_toggle.set_active(true);

    {
        let state = state.clone();
        brush_toggle.connect_clicked(move |_| {
            state.borrow_mut().active_tool = ActiveTool::Brush;
        });
    }
    {
        let state = state.clone();
        rect_toggle.connect_clicked(move |_| {
            state.borrow_mut().active_tool = ActiveTool::Rect;
        });
    }

    bar.append(&brush_toggle);
    bar.append(&rect_toggle);

    bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));

    let display = gdk::Display::default().expect("cannot get default display");

    // --- Выбор цвета (MenuButton + Popover) ---
    let colors: Vec<(&str, gdk::RGBA)> = vec![
        ("red", gdk::RGBA::new(1.0, 0.2, 0.2, 1.0)),
        ("green", gdk::RGBA::new(0.2, 0.8, 0.2, 1.0)),
        ("yellow", gdk::RGBA::new(1.0, 0.85, 0.0, 1.0)),
        ("blue", gdk::RGBA::new(0.3, 0.5, 1.0, 1.0)),
        ("pink", gdk::RGBA::new(1.0, 0.4, 1.0, 1.0)),
    ];

    // Кнопка показывает текущий цвет
    let color_btn = gtk::MenuButton::new();
    color_btn.set_size_request(28, 28);
    color_btn.add_css_class("snip-color-btn");

    // CSS для кнопки текущего цвета (из сохранённого состояния)
    let color_btn_css = gtk::CssProvider::new();
    let current_color = state.borrow().brush.color;
    color_btn_css.load_from_string(&format!(
        "menubutton.snip-color-btn > button {{ background: rgba({},{},{},{}); border-radius: 4px; min-width: 28px; min-height: 28px; padding: 0; }}",
        (current_color.red() * 255.0) as u8,
        (current_color.green() * 255.0) as u8,
        (current_color.blue() * 255.0) as u8,
        current_color.alpha(),
    ));
    gtk::style_context_add_provider_for_display(
        &display,
        &color_btn_css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Popover с сеткой цветов
    let popover = gtk::Popover::new();
    let color_grid = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .margin_start(6)
        .margin_end(6)
        .margin_top(6)
        .margin_bottom(6)
        .build();

    for (name, rgba) in &colors {
        let btn = gtk::Button::new();
        btn.set_size_request(32, 32);

        let css_class = format!("snip-popover-color-{}", name);
        let css = format!(
            "button.{} {{ background: rgba({},{},{},{}); border-radius: 4px; min-width: 32px; min-height: 32px; padding: 0; }}",
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
        let popover = popover.clone();
        let color_btn_css = color_btn_css.clone();
        btn.connect_clicked(move |_| {
            let mut st = state.borrow_mut();
            st.brush.color = color;
            // Сохранить настройки
            utils::save_brush_config(&utils::BrushConfig {
                color: [color.red(), color.green(), color.blue(), color.alpha()],
                width: st.brush.width,
            });
            drop(st);
            // Обновить цвет кнопки
            color_btn_css.load_from_string(&format!(
                "menubutton.snip-color-btn > button {{ background: rgba({},{},{},{}); border-radius: 4px; min-width: 28px; min-height: 28px; padding: 0; }}",
                (color.red() * 255.0) as u8,
                (color.green() * 255.0) as u8,
                (color.blue() * 255.0) as u8,
                color.alpha(),
            ));
            popover.popdown();
        });
        color_grid.append(&btn);
    }

    popover.set_child(Some(&color_grid));
    color_btn.set_popover(Some(&popover));
    bar.append(&color_btn);

    bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));

    // --- Толщина (MenuButton + Popover) ---
    let widths: Vec<f64> = vec![2.0, 4.0, 8.0];
    let current_width = state.borrow().brush.width;

    let width_btn = gtk::MenuButton::new();
    width_btn.set_label(&format!("{}px", current_width as i32));

    let width_popover = gtk::Popover::new();
    let width_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .margin_start(6)
        .margin_end(6)
        .margin_top(6)
        .margin_bottom(6)
        .build();

    for line_width in &widths {
        let label = format!("{}px", *line_width as i32);
        let btn = gtk::Button::with_label(&label);

        let w = *line_width;
        let state = state.clone();
        let width_btn = width_btn.clone();
        let width_popover = width_popover.clone();
        btn.connect_clicked(move |_| {
            let mut st = state.borrow_mut();
            st.brush.width = w;
            let color = st.brush.color;
            utils::save_brush_config(&utils::BrushConfig {
                color: [color.red(), color.green(), color.blue(), color.alpha()],
                width: w,
            });
            drop(st);
            width_btn.set_label(&format!("{}px", w as i32));
            width_popover.popdown();
        });
        width_box.append(&btn);
    }

    width_popover.set_child(Some(&width_box));
    width_btn.set_popover(Some(&width_popover));
    bar.append(&width_btn);

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

    bar
}
