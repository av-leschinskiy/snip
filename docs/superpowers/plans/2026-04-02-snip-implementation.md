# Snip — план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Создать утилиту для скриншотов на Rust + GTK4/Libadwaita для Linux/Wayland/GNOME — захват области экрана, редактор с кистью, копирование в буфер.

**Architecture:** Монолитное GTK4-приложение с двумя режимами: capture (portal screenshot → fullscreen overlay → area selection → crop → save) и editor (окно с canvas, кисть, clipboard). CLI через clap: `snip` для захвата, `snip edit <path>` для открытия файла.

**Tech Stack:** Rust, gtk4, libadwaita, ashpd (XDG portals), cairo-rs, clap, chrono, dirs

**Спецификация:** `docs/superpowers/specs/2026-04-02-snip-screenshot-tool-design.md`

---

## Структура файлов

| Файл | Ответственность |
|------|----------------|
| `Cargo.toml` | Зависимости проекта |
| `src/main.rs` | CLI-парсинг (clap), создание GtkApplication, маршрутизация capture/edit |
| `src/utils.rs` | XDG paths, генерация имени файла, сохранение PNG, crop изображения |
| `src/tools/mod.rs` | Trait `Annotation`, struct `Stroke`, реестр инструментов |
| `src/tools/brush.rs` | Инструмент кисти — создание Stroke из событий мыши |
| `src/capture.rs` | Portal screenshot (ashpd), fullscreen overlay окно с area selection |
| `src/editor.rs` | Окно редактора — canvas, toolbar, кисть, clipboard, сохранение |

---

## Task 1: Scaffolding проекта

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`

- [ ] **Step 1: Инициализировать git-репозиторий**

```bash
cd /home/leschinskiy/projects/snip
git init
```

- [ ] **Step 2: Создать .gitignore**

Создать файл `.gitignore`:

```gitignore
/target
.superpowers/
```

- [ ] **Step 3: Создать Cargo.toml**

Создать файл `Cargo.toml`:

```toml
[package]
name = "snip"
version = "0.1.0"
edition = "2021"

[dependencies]
gtk4 = { version = "0.9", features = ["v4_16"] }
libadwaita = { version = "0.7", features = ["v1_6"] }
ashpd = { version = "0.11", features = ["gtk4"] }
cairo-rs = { version = "0.20", features = ["png"] }
clap = { version = "4", features = ["derive"] }
chrono = "0.4"
dirs = "6"
gdk4 = "0.9"
glib = "0.20"
```

- [ ] **Step 4: Создать минимальный main.rs**

Создать файл `src/main.rs`:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "snip", about = "Screenshot tool for GNOME/Wayland")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Open an existing file in the editor
    Edit {
        /// Path to image file
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => {
            println!("capture mode (not implemented yet)");
        }
        Some(Commands::Edit { path }) => {
            println!("edit mode: {path}");
        }
    }
}
```

- [ ] **Step 5: Проверить сборку**

```bash
cd /home/leschinskiy/projects/snip && cargo build
```

Ожидается: успешная компиляция, бинарник в `target/debug/snip`.

- [ ] **Step 6: Проверить CLI**

```bash
./target/debug/snip --help
./target/debug/snip
./target/debug/snip edit test.png
```

Ожидается: справка с подкомандами, "capture mode", "edit mode: test.png".

- [ ] **Step 7: Коммит**

```bash
git add Cargo.toml Cargo.lock src/main.rs .gitignore
git commit -m "feat: scaffolding проекта с CLI-парсингом"
```

---

## Task 2: Модуль utils — пути и сохранение файлов

**Files:**
- Create: `src/utils.rs`
- Modify: `src/main.rs` (добавить `mod utils;`)

- [ ] **Step 1: Написать тест для генерации пути скриншота**

Создать файл `src/utils.rs`:

```rust
use std::path::PathBuf;

/// Возвращает директорию для скриншотов: XDG_PICTURES_DIR/Screenshots/
pub fn screenshots_dir() -> PathBuf {
    let pictures = dirs::picture_dir().unwrap_or_else(|| {
        let home = dirs::home_dir().expect("no home directory");
        home.join("Pictures")
    });
    pictures.join("Screenshots")
}

/// Генерирует имя файла: screenshot-YYYY-MM-DD_HH-MM-SS.png
pub fn screenshot_filename() -> String {
    let now = chrono::Local::now();
    now.format("screenshot-%Y-%m-%d_%H-%M-%S.png").to_string()
}

/// Полный путь для нового скриншота. Создаёт директорию если не существует.
pub fn new_screenshot_path() -> std::io::Result<PathBuf> {
    let dir = screenshots_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join(screenshot_filename()))
}

/// Обрезает cairo::ImageSurface по прямоугольнику (x, y, w, h).
/// Возвращает новый ImageSurface с вырезанной областью.
pub fn crop_surface(
    source: &cairo::ImageSurface,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<cairo::ImageSurface, cairo::Error> {
    let cropped = cairo::ImageSurface::create(cairo::Format::ARgb32, width, height)?;
    let cr = cairo::Context::new(&cropped)?;
    cr.set_source_surface(source, -x as f64, -y as f64)?;
    cr.paint()?;
    drop(cr);
    Ok(cropped)
}

/// Сохраняет cairo::ImageSurface в PNG-файл.
pub fn save_surface_as_png(
    surface: &cairo::ImageSurface,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = std::fs::File::create(path)?;
    surface.write_to_png(&mut file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screenshots_dir_ends_with_screenshots() {
        let dir = screenshots_dir();
        assert_eq!(dir.file_name().unwrap(), "Screenshots");
    }

    #[test]
    fn test_screenshot_filename_format() {
        let name = screenshot_filename();
        assert!(name.starts_with("screenshot-"));
        assert!(name.ends_with(".png"));
        // Формат: screenshot-YYYY-MM-DD_HH-MM-SS.png — длина фиксирована
        assert_eq!(name.len(), "screenshot-2026-04-02_14-35-22.png".len());
    }

    #[test]
    fn test_crop_surface() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 100, 100).unwrap();
        let cropped = crop_surface(&surface, 10, 10, 50, 50).unwrap();
        assert_eq!(cropped.width(), 50);
        assert_eq!(cropped.height(), 50);
    }

    #[test]
    fn test_save_surface_as_png() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 10, 10).unwrap();
        let tmp = std::env::temp_dir().join("snip-test-output.png");
        save_surface_as_png(&surface, &tmp).unwrap();
        assert!(tmp.exists());
        assert!(std::fs::metadata(&tmp).unwrap().len() > 0);
        std::fs::remove_file(&tmp).ok();
    }
}
```

- [ ] **Step 2: Подключить модуль в main.rs**

Добавить в начало `src/main.rs`:

```rust
mod utils;
```

- [ ] **Step 3: Запустить тесты**

```bash
cd /home/leschinskiy/projects/snip && cargo test
```

Ожидается: 4 теста пройдены.

- [ ] **Step 4: Коммит**

```bash
git add src/utils.rs src/main.rs
git commit -m "feat: модуль utils — пути, crop, сохранение PNG"
```

---

## Task 3: Модуль tools — trait Annotation и кисть

**Files:**
- Create: `src/tools/mod.rs`
- Create: `src/tools/brush.rs`
- Modify: `src/main.rs` (добавить `mod tools;`)

- [ ] **Step 1: Создать trait Annotation и структуру Stroke**

Создать файл `src/tools/mod.rs`:

```rust
pub mod brush;

use gdk4 as gdk;

/// Одна аннотация (завершённый штрих, фигура и т.д.)
/// Каждый инструмент создаёт свой тип аннотации, реализующий этот trait.
pub trait Annotation {
    /// Отрисовать аннотацию на cairo context.
    fn draw(&self, cr: &cairo::Context);
}

/// Штрих кисти — набор точек с цветом и толщиной.
#[derive(Clone, Debug)]
pub struct Stroke {
    pub points: Vec<(f64, f64)>,
    pub color: gdk::RGBA,
    pub width: f64,
}

impl Stroke {
    pub fn new(color: gdk::RGBA, width: f64) -> Self {
        Self {
            points: Vec::new(),
            color,
            width,
        }
    }

    pub fn add_point(&mut self, x: f64, y: f64) {
        self.points.push((x, y));
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

impl Annotation for Stroke {
    fn draw(&self, cr: &cairo::Context) {
        if self.points.len() < 2 {
            return;
        }

        cr.set_source_rgba(
            self.color.red() as f64,
            self.color.green() as f64,
            self.color.blue() as f64,
            self.color.alpha() as f64,
        );
        cr.set_line_width(self.width);
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_line_join(cairo::LineJoin::Round);

        let (x0, y0) = self.points[0];
        cr.move_to(x0, y0);
        for &(x, y) in &self.points[1..] {
            cr.line_to(x, y);
        }
        let _ = cr.stroke();
    }
}
```

- [ ] **Step 2: Создать модуль brush — фабрика инструмента**

Создать файл `src/tools/brush.rs`:

```rust
use gdk4 as gdk;

use super::Stroke;

/// Состояние инструмента "Кисть".
/// Хранит текущие настройки (цвет, толщина) и незавершённый штрих.
pub struct BrushTool {
    pub color: gdk::RGBA,
    pub width: f64,
    current_stroke: Option<Stroke>,
}

impl BrushTool {
    pub fn new() -> Self {
        Self {
            color: gdk::RGBA::new(1.0, 0.0, 0.0, 1.0), // красный по умолчанию
            width: 3.0,
            current_stroke: None,
        }
    }

    /// Начать новый штрих.
    pub fn press(&mut self, x: f64, y: f64) {
        let mut stroke = Stroke::new(self.color, self.width);
        stroke.add_point(x, y);
        self.current_stroke = Some(stroke);
    }

    /// Добавить точку к текущему штриху.
    pub fn motion(&mut self, x: f64, y: f64) {
        if let Some(ref mut stroke) = self.current_stroke {
            stroke.add_point(x, y);
        }
    }

    /// Завершить штрих и вернуть его. Возвращает None если штрих пуст.
    pub fn release(&mut self) -> Option<Stroke> {
        self.current_stroke.take().filter(|s| s.points.len() >= 2)
    }

    /// Получить текущий незавершённый штрих для отрисовки в реальном времени.
    pub fn current_stroke(&self) -> Option<&Stroke> {
        self.current_stroke.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brush_stroke_lifecycle() {
        let mut brush = BrushTool::new();

        // Нажатие — начало штриха
        brush.press(10.0, 20.0);
        assert!(brush.current_stroke().is_some());

        // Движение — добавление точек
        brush.motion(30.0, 40.0);
        brush.motion(50.0, 60.0);
        assert_eq!(brush.current_stroke().unwrap().points.len(), 3);

        // Отпускание — завершение штриха
        let stroke = brush.release().unwrap();
        assert_eq!(stroke.points.len(), 3);
        assert!(brush.current_stroke().is_none());
    }

    #[test]
    fn test_brush_single_point_returns_none() {
        let mut brush = BrushTool::new();
        brush.press(10.0, 20.0);
        // Один клик без движения — не считается штрихом
        let stroke = brush.release();
        assert!(stroke.is_none());
    }

    #[test]
    fn test_brush_uses_configured_color_and_width() {
        let mut brush = BrushTool::new();
        brush.color = gdk::RGBA::new(0.0, 1.0, 0.0, 1.0);
        brush.width = 5.0;

        brush.press(0.0, 0.0);
        brush.motion(10.0, 10.0);
        let stroke = brush.release().unwrap();

        assert_eq!(stroke.color, gdk::RGBA::new(0.0, 1.0, 0.0, 1.0));
        assert_eq!(stroke.width, 5.0);
    }
}
```

- [ ] **Step 3: Подключить модуль в main.rs**

Добавить в начало `src/main.rs`:

```rust
mod tools;
```

- [ ] **Step 4: Запустить тесты**

```bash
cd /home/leschinskiy/projects/snip && cargo test
```

Ожидается: все тесты (4 из utils + 3 из brush) пройдены.

- [ ] **Step 5: Коммит**

```bash
git add src/tools/mod.rs src/tools/brush.rs src/main.rs
git commit -m "feat: модуль tools — trait Annotation, Stroke, BrushTool"
```

---

## Task 4: Capture — portal screenshot + overlay selection

**Files:**
- Create: `src/capture.rs`
- Modify: `src/main.rs` (подключить модуль, запустить capture flow)

- [ ] **Step 1: Создать модуль capture**

Создать файл `src/capture.rs`:

```rust
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
                // Пользователь отклонил portal или ошибка — тихо выходим
                eprintln!("Portal screenshot cancelled or failed: {e}");
                app.quit();
            }
        }
    });
}

async fn take_portal_screenshot() -> Result<String, Box<dyn std::error::Error>> {
    let proxy = ashpd::desktop::screenshot::Screenshot::new().await?;
    let uri = proxy
        .screenshot()
        .interactive(false)
        .send()
        .await?
        .response()?;
    Ok(uri.uri().to_string())
}

fn uri_to_path(uri: &str) -> PathBuf {
    // file:///path/to/file → /path/to/file
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

/// Показывает fullscreen overlay для выделения области.
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

    // Состояние выделения
    let start_x = Rc::new(Cell::new(0.0f64));
    let start_y = Rc::new(Cell::new(0.0f64));
    let cur_x = Rc::new(Cell::new(0.0f64));
    let cur_y = Rc::new(Cell::new(0.0f64));
    let selecting = Rc::new(Cell::new(false));

    let drawing_area = gtk::DrawingArea::new();
    drawing_area.set_content_width(img_width);
    drawing_area.set_content_height(img_height);

    // Отрисовка
    {
        let surface = surface.clone();
        let start_x = start_x.clone();
        let start_y = start_y.clone();
        let cur_x = cur_x.clone();
        let cur_y = cur_y.clone();
        let selecting = selecting.clone();

        drawing_area.set_draw_func(move |_da, cr, width, height| {
            // 1. Рисуем скриншот
            let scale_x = width as f64 / surface.width() as f64;
            let scale_y = height as f64 / surface.height() as f64;
            cr.scale(scale_x, scale_y);
            let _ = cr.set_source_surface(&*surface, 0.0, 0.0);
            let _ = cr.paint();
            cr.identity_matrix();

            // 2. Затемнение
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
            let _ = cr.paint();

            // 3. Вырез выделенной области
            if selecting.get() {
                let sx = start_x.get();
                let sy = start_y.get();
                let cx = cur_x.get();
                let cy = cur_y.get();

                let rx = sx.min(cx);
                let ry = sy.min(cy);
                let rw = (cx - sx).abs();
                let rh = (cy - sy).abs();

                // Рисуем оригинальное изображение в вырезе
                cr.save().ok();
                cr.rectangle(rx, ry, rw, rh);
                cr.clip();
                cr.scale(scale_x, scale_y);
                let _ = cr.set_source_surface(&*surface, 0.0, 0.0);
                let _ = cr.paint();
                cr.restore().ok();

                // Белая рамка
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
                cr.set_line_width(1.5);
                cr.rectangle(rx, ry, rw, rh);
                let _ = cr.stroke();
            }
        });
    }

    // Курсор crosshair
    drawing_area.set_cursor_from_name(Some("crosshair"));

    // Обработка мыши
    let press_gesture = gtk::GestureClick::new();
    press_gesture.set_button(1); // ЛКМ

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
        let cur_x = cur_x.clone();
        let cur_y = cur_y.clone();
        let selecting = selecting.clone();
        let window_clone = window.clone();
        let on_done = on_done.clone();

        press_gesture.connect_released(move |_gesture, _n, x, y| {
            if !selecting.get() {
                return;
            }
            selecting.set(false);

            let sx = start_x.get();
            let sy = start_y.get();

            // Получаем размеры окна для пересчёта координат
            let alloc = _gesture.widget().allocation();
            let scale_x = surface.width() as f64 / alloc.width() as f64;
            let scale_y = surface.height() as f64 / alloc.height() as f64;

            let rx = (sx.min(x) * scale_x) as i32;
            let ry = (sy.min(y) * scale_y) as i32;
            let rw = ((x - sx).abs() * scale_x) as i32;
            let rh = ((y - sy).abs() * scale_y) as i32;

            // Игнорируем слишком маленькое выделение
            if rw < 5 || rh < 5 {
                return;
            }

            // Crop
            match utils::crop_surface(&surface, rx, ry, rw, rh) {
                Ok(cropped) => {
                    // Сохранение
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

    // Escape — отмена
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
```

- [ ] **Step 2: Обновить main.rs — подключить capture и GTK**

Заменить содержимое `src/main.rs`:

```rust
mod capture;
mod tools;
mod utils;

use clap::{Parser, Subcommand};
use gtk4::prelude::*;

#[derive(Parser)]
#[command(name = "snip", about = "Screenshot tool for GNOME/Wayland")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Open an existing file in the editor
    Edit {
        /// Path to image file
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let app = libadwaita::Application::builder()
        .application_id("dev.snip.app")
        .build();

    match cli.command {
        None => {
            app.connect_activate(|app| {
                capture::start_capture(app, |path| {
                    println!("Screenshot saved to: {}", path.display());
                    // TODO: открыть редактор (Task 6)
                });
            });
        }
        Some(Commands::Edit { path }) => {
            let path = std::path::PathBuf::from(path);
            if !path.exists() {
                eprintln!("File not found: {}", path.display());
                std::process::exit(1);
            }
            app.connect_activate(move |_app| {
                println!("editor mode: {}", path.display());
                // TODO: открыть редактор (Task 6)
            });
        }
    }

    app.run_with_args::<String>(&[]);
}
```

- [ ] **Step 3: Проверить сборку**

```bash
cd /home/leschinskiy/projects/snip && cargo build
```

Ожидается: успешная компиляция.

- [ ] **Step 4: Ручной тест capture flow**

```bash
cd /home/leschinskiy/projects/snip && cargo run
```

Ожидается: portal запрашивает разрешение на скриншот → fullscreen overlay → можно выделить область → файл сохраняется в `~/Pictures/Screenshots/`.

- [ ] **Step 5: Проверить Escape**

Запустить `cargo run`, нажать Escape в overlay. Ожидается: приложение закрывается.

- [ ] **Step 6: Запустить все тесты**

```bash
cd /home/leschinskiy/projects/snip && cargo test
```

Ожидается: все предыдущие тесты по-прежнему проходят.

- [ ] **Step 7: Коммит**

```bash
git add src/capture.rs src/main.rs
git commit -m "feat: capture flow — portal screenshot + overlay selection"
```

---

## Task 5: Редактор — окно, canvas, toolbar

**Files:**
- Create: `src/editor.rs`
- Modify: `src/main.rs` (подключить editor, вызывать из capture и edit)

- [ ] **Step 1: Создать модуль editor**

Создать файл `src/editor.rs`:

```rust
use gdk4 as gdk;
use gtk4 as gtk;
use gtk4::prelude::*;
use gtk4::glib;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::tools::{Annotation, Stroke};
use crate::tools::brush::BrushTool;
use crate::utils;

/// Состояние редактора, разделяемое между callback'ами.
struct EditorState {
    image_surface: cairo::ImageSurface,
    file_path: PathBuf,
    annotations: Vec<Stroke>,
    brush: BrushTool,
}

/// Открывает окно редактора для указанного файла.
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

    // Кнопка кисти (слева)
    let brush_btn = gtk::ToggleButton::builder()
        .label("Кисть")
        .active(true)
        .build();
    header.pack_start(&brush_btn);

    // Кнопки действий (справа)
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

    // Draw function
    {
        let state = state.clone();
        drawing_area.set_draw_func(move |_da, cr, width, height| {
            let state = state.borrow();

            // Фон
            cr.set_source_rgb(0.12, 0.12, 0.12);
            let _ = cr.paint();

            // Масштабирование изображения чтобы поместилось
            let img_w = state.image_surface.width() as f64;
            let img_h = state.image_surface.height() as f64;
            let scale = (width as f64 / img_w).min(height as f64 / img_h).min(1.0);
            let offset_x = (width as f64 - img_w * scale) / 2.0;
            let offset_y = (height as f64 - img_h * scale) / 2.0;

            cr.save().ok();
            cr.translate(offset_x, offset_y);
            cr.scale(scale, scale);

            // Изображение
            let _ = cr.set_source_surface(&state.image_surface, 0.0, 0.0);
            let _ = cr.paint();

            // Завершённые аннотации
            for annotation in &state.annotations {
                annotation.draw(cr);
            }

            // Текущий незавершённый штрих
            if let Some(stroke) = state.brush.current_stroke() {
                stroke.draw(cr);
            }

            cr.restore().ok();
        });
    }

    // === Mouse events на canvas ===
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

    // === Нижняя панель — цвет + толщина ===
    let bottom_bar = build_bottom_bar(state.clone(), drawing_area.clone());

    // === Кнопка "Копировать" — копирует изображение в clipboard ===
    {
        let state = state.clone();
        let da = drawing_area.clone();
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
            drop(st);
        });
    }

    // === Кнопка "Путь" — копирует путь к файлу ===
    {
        let state = state.clone();
        path_btn.connect_clicked(move |btn| {
            let st = state.borrow();
            let path_str = st.file_path.to_string_lossy().to_string();
            let clipboard = btn.clipboard();
            clipboard.set_text(&path_str);
        });
    }

    // === Кнопка "Сохранить" — впекает аннотации в файл ===
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

    // === Сборка окна ===
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let toolbar_view = libadwaita::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&drawing_area));
    content.append(&toolbar_view);
    content.append(&bottom_bar);

    window.set_content(Some(&content));
    window.present();
}

/// Пересчёт координат экрана → координаты изображения.
fn screen_to_image(state: &EditorState, da: &gtk::DrawingArea, x: f64, y: f64) -> (f64, f64) {
    let img_w = state.image_surface.width() as f64;
    let img_h = state.image_surface.height() as f64;
    let alloc = da.allocation();
    let w = alloc.width() as f64;
    let h = alloc.height() as f64;
    let scale = (w / img_w).min(h / img_h).min(1.0);
    let offset_x = (w - img_w * scale) / 2.0;
    let offset_y = (h - img_h * scale) / 2.0;
    ((x - offset_x) / scale, (y - offset_y) / scale)
}

/// Рендерит финальное изображение: оригинал + аннотации.
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

/// Конвертирует cairo::ImageSurface в gdk::Texture для clipboard.
fn surface_to_texture(surface: &cairo::ImageSurface) -> gdk::Texture {
    let mut png_data: Vec<u8> = Vec::new();
    surface.write_to_png(&mut png_data).expect("PNG write failed");
    let bytes = glib::Bytes::from(&png_data);
    gdk::Texture::from_bytes(&bytes).expect("Texture from PNG failed")
}

/// Создаёт нижнюю панель с выбором цвета и толщины.
fn build_bottom_bar(state: Rc<RefCell<EditorState>>, da: gtk::DrawingArea) -> gtk::Box {
    let bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    // Метка "Цвет:"
    let color_label = gtk::Label::new(Some("Цвет:"));
    color_label.add_css_class("dim-label");
    bar.append(&color_label);

    // Преднастроенные цвета
    let colors: Vec<(&str, gdk::RGBA)> = vec![
        ("red", gdk::RGBA::new(1.0, 0.2, 0.2, 1.0)),
        ("green", gdk::RGBA::new(0.2, 0.8, 0.2, 1.0)),
        ("yellow", gdk::RGBA::new(1.0, 0.85, 0.0, 1.0)),
        ("blue", gdk::RGBA::new(0.3, 0.5, 1.0, 1.0)),
        ("pink", gdk::RGBA::new(1.0, 0.4, 1.0, 1.0)),
    ];

    for (_name, rgba) in &colors {
        let btn = gtk::Button::new();
        btn.set_size_request(28, 28);
        let css = format!(
            "button {{ background: rgba({},{},{},{}); border-radius: 50%; min-width: 28px; min-height: 28px; padding: 0; }}",
            (rgba.red() * 255.0) as u8,
            (rgba.green() * 255.0) as u8,
            (rgba.blue() * 255.0) as u8,
            rgba.alpha(),
        );
        let provider = gtk::CssProvider::new();
        provider.load_from_string(&css);
        btn.style_context().add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let color = *rgba;
        let state = state.clone();
        btn.connect_clicked(move |_| {
            state.borrow_mut().brush.color = color;
        });
        bar.append(&btn);
    }

    // Кнопка произвольного цвета через GtkColorDialog
    let custom_color_btn = gtk::Button::new();
    custom_color_btn.set_size_request(28, 28);
    custom_color_btn.set_tooltip_text(Some("Выбрать цвет"));
    let css_custom = "button { background: conic-gradient(red, yellow, lime, aqua, blue, magenta, red); border-radius: 50%; min-width: 28px; min-height: 28px; padding: 0; }";
    let provider_custom = gtk::CssProvider::new();
    provider_custom.load_from_string(css_custom);
    custom_color_btn.style_context().add_provider(&provider_custom, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    {
        let state = state.clone();
        let window = da.root().and_downcast::<gtk::Window>();
        custom_color_btn.connect_clicked(move |_btn| {
            let dialog = gtk::ColorDialog::new();
            let state = state.clone();
            dialog.choose_rgba(
                window.as_ref(),
                Some(&state.borrow().brush.color),
                None::<&gtk4::gio::Cancellable>,
                move |result| {
                    if let Ok(color) = result {
                        state.borrow_mut().brush.color = color;
                    }
                },
            );
        });
    }
    bar.append(&custom_color_btn);

    // Разделитель
    let sep = gtk::Separator::new(gtk::Orientation::Vertical);
    bar.append(&sep);

    // Метка "Толщина:"
    let width_label = gtk::Label::new(Some("Толщина:"));
    width_label.add_css_class("dim-label");
    bar.append(&width_label);

    // Кнопки толщины
    let widths: Vec<(f64, i32)> = vec![(2.0, 10), (4.0, 16), (8.0, 22)];
    for (line_width, btn_size) in &widths {
        let btn = gtk::Button::new();
        btn.set_size_request(*btn_size, *btn_size);
        let css = format!(
            "button {{ border-radius: 50%; min-width: {}px; min-height: {}px; padding: 0; }}",
            btn_size, btn_size,
        );
        let provider = gtk::CssProvider::new();
        provider.load_from_string(&css);
        btn.style_context().add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let w = *line_width;
        let state = state.clone();
        btn.connect_clicked(move |_| {
            state.borrow_mut().brush.width = w;
        });
        bar.append(&btn);
    }

    bar
}
```

- [ ] **Step 2: Обновить main.rs — интегрировать editor**

Заменить содержимое `src/main.rs`:

```rust
mod capture;
mod editor;
mod tools;
mod utils;

use clap::{Parser, Subcommand};
use gtk4::prelude::*;

#[derive(Parser)]
#[command(name = "snip", about = "Screenshot tool for GNOME/Wayland")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Open an existing file in the editor
    Edit {
        /// Path to image file
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let app = libadwaita::Application::builder()
        .application_id("dev.snip.app")
        .build();

    match cli.command {
        None => {
            app.connect_activate(|app| {
                capture::start_capture(app, {
                    let app = app.clone();
                    move |path| {
                        editor::open_editor(&app, path);
                    }
                });
            });
        }
        Some(Commands::Edit { path }) => {
            let path = std::path::PathBuf::from(path);
            if !path.exists() {
                eprintln!("File not found: {}", path.display());
                std::process::exit(1);
            }
            app.connect_activate(move |app| {
                editor::open_editor(app, path.clone());
            });
        }
    }

    app.run_with_args::<String>(&[]);
}
```

- [ ] **Step 3: Проверить сборку**

```bash
cd /home/leschinskiy/projects/snip && cargo build
```

Ожидается: успешная компиляция.

- [ ] **Step 4: Ручной тест полного flow**

```bash
cd /home/leschinskiy/projects/snip && cargo run
```

Ожидается: portal → overlay → выделение → скриншот сохраняется → открывается окно редактора с изображением.

- [ ] **Step 5: Ручной тест edit mode**

```bash
# Используем скриншот из предыдущего шага
cd /home/leschinskiy/projects/snip && cargo run -- edit ~/Pictures/Screenshots/screenshot-*.png
```

Ожидается: открывается редактор с указанным изображением.

- [ ] **Step 6: Тест кисти в редакторе**

В открытом редакторе: нажать ЛКМ на canvas и рисовать. Ожидается: красные линии появляются в реальном времени.

- [ ] **Step 7: Тест кнопок**

- "Копировать" — после нажатия вставить в другое приложение (e.g. GIMP), должно вставиться изображение
- "Путь" — вставить в терминал, должен вставиться путь к файлу
- "Сохранить" — нарисовать линию, нажать сохранить, переоткрыть файл — линия должна быть на изображении

- [ ] **Step 8: Запустить все тесты**

```bash
cd /home/leschinskiy/projects/snip && cargo test
```

Ожидается: все предыдущие тесты проходят.

- [ ] **Step 9: Коммит**

```bash
git add src/editor.rs src/main.rs
git commit -m "feat: редактор — canvas, кисть, clipboard, сохранение"
```

---

## Task 6: Финальная интеграция и polish

**Files:**
- Modify: `src/editor.rs` (исправления по результатам ручного тестирования)
- Modify: `src/capture.rs` (исправления по результатам ручного тестирования)

- [ ] **Step 1: Полный end-to-end тест capture → editor → save**

```bash
cd /home/leschinskiy/projects/snip && cargo run
```

1. Portal → overlay → выделить область → overlay закрывается
2. Редактор открывается с обрезанным скриншотом
3. Нарисовать линию кистью
4. Сменить цвет → нарисовать другую линию
5. Сменить толщину → нарисовать третью линию
6. "Копировать" → вставить в другое приложение
7. "Путь" → вставить в терминал
8. "Сохранить" → проверить что файл обновился

- [ ] **Step 2: Тест edit mode с несуществующим файлом**

```bash
cd /home/leschinskiy/projects/snip && cargo run -- edit /nonexistent.png
```

Ожидается: `File not found: /nonexistent.png`, exit code 1.

- [ ] **Step 3: Исправить обнаруженные проблемы**

Если на предыдущих шагах обнаружены баги — исправить. Типичные проблемы:
- Координаты мыши не совпадают с рисованием (масштабирование)
- Clipboard не работает (проверить что Wayland clipboard правильно инициализирован)
- Overlay не fullscreen (проверить что window manager правильно обрабатывает fullscreen)

- [ ] **Step 4: Запустить все тесты**

```bash
cd /home/leschinskiy/projects/snip && cargo test
```

Ожидается: все тесты проходят.

- [ ] **Step 5: Коммит (если были изменения)**

```bash
git add -u
git commit -m "fix: исправления по результатам интеграционного тестирования"
```

---

## Контрольная проверка спецификации

| Требование из спецификации | Задача |
|---------------------------|--------|
| CLI: `snip` — capture mode | Task 4 |
| CLI: `snip edit <path>` — editor mode | Task 5 |
| Portal screenshot через ashpd | Task 4 |
| Fullscreen overlay с затемнением и вырезом | Task 4 |
| Escape для отмены | Task 4 |
| Выделение < 5×5 px игнорируется | Task 4 |
| Crop и сохранение в XDG Pictures/Screenshots | Task 2 + Task 4 |
| Формат имени screenshot-YYYY-MM-DD_HH-MM-SS.png | Task 2 |
| Редактор: canvas с изображением | Task 5 |
| Редактор: кисть (свободное рисование) | Task 3 + Task 5 |
| Расширяемость инструментов (trait) | Task 3 |
| Кнопка "Копировать" (буфер обмена) | Task 5 |
| Кнопка "Путь" (путь в буфер) | Task 5 |
| Кнопка "Сохранить" (впекание аннотаций) | Task 5 |
| Выбор цвета (5 преднастроенных) | Task 5 |
| Выбор толщины (3 варианта) | Task 5 |
| Несуществующий файл → stderr + exit 1 | Task 5 |
| Папка Screenshots создаётся автоматически | Task 2 |
