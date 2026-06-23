# Акцентная рамка окна + инструмент выбора цвета — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Дать окну редактора snip постоянную акцентную рамку по периметру и инструмент выбора её цвета (большая палитра + произвольный цвет), с сохранением между запусками.

**Architecture:** Цвет рамки хранится в `EditorState` и в едином `config.json`. Рамка рисуется CSS-правилом на узле `window` через выделенный `CssProvider`, который перезагружается при смене цвета. Инструмент — `MenuButton` в `HeaderBar` справа с поповером (сетка пресетов + нативный `gtk::ColorDialog`).

**Tech Stack:** Rust, gtk4 0.9 (GTK 4.16), libadwaita 0.7, gdk4, serde/serde_json.

## Global Constraints

- Сборка и тесты только в Nix shell: команды вида `nix develop -c cargo ...`.
- Комментарии в коде — на русском языке.
- Сообщения коммитов — на русском, формат `тип: описание`. Без строк `Co-Authored-By`.
- Минимальная обработка ошибок: сохранение конфига best-effort, отмена диалога игнорируется молча.
- Ширина рамки фиксированная: `WINDOW_BORDER_WIDTH = 3`.
- Дефолтный цвет рамки: `#e8590c` = `[0.910, 0.349, 0.047, 1.0]` (RGBA, f32).
- Палитра рамки `BORDER_PALETTE` — ровно 40 цветов (8×5), значения заданы в Task 3.

---

## File Structure

- `src/utils.rs` — конфиг: переименование `BrushConfig`→`Config`, новое поле `border_color`, `load_config`/`save_config`, `default_border_color`, тесты.
- `src/tools/brush.rs` — единственная правка: вызов `utils::load_config()`.
- `src/editor.rs` — поле `border_color` в `EditorState`, хелперы `rgba_to_arr`/`window_border_css`/`persist_config`, провайдер рамки окна, инструмент выбора цвета (`BORDER_PALETTE`, `apply_border_color`, `build_border_color_button`), монтаж кнопки в `HeaderBar`.

---

## Task 1: Единый конфиг с цветом рамки

Переименовать `BrushConfig`→`Config`, добавить `border_color` с serde-дефолтом (старые конфиги грузятся без поломки), завести единый `persist_config(&EditorState)` в editor.rs и провести через него существующие сохранения кисти. Деливерабл проверяется юнит-тестами и зелёным существующим набором.

**Files:**
- Modify: `src/utils.rs:29-67` (struct + Default + load/save), `src/utils.rs:69-87` (tests)
- Modify: `src/tools/brush.rs:15`
- Modify: `src/editor.rs:42-50` (EditorState), `src/editor.rs:73-81` (init), `src/editor.rs:546-559` (save в кнопке цвета), `src/editor.rs:589-604` (save в кнопке толщины)

**Interfaces:**
- Produces:
  - `pub struct utils::Config { pub color: [f32;4], pub width: f64, pub border_color: [f32;4] }`
  - `pub fn utils::load_config() -> Config`
  - `pub fn utils::save_config(config: &Config)`
  - `fn editor::rgba_to_arr(c: &gdk::RGBA) -> [f32; 4]`
  - `fn editor::persist_config(st: &EditorState)`
  - Поле `EditorState.border_color: gdk::RGBA`

- [ ] **Step 1: Написать падающие тесты конфига**

В `src/utils.rs`, внутри `mod tests`, добавить:

```rust
    #[test]
    fn test_config_roundtrip_with_border_color() {
        let cfg = Config { color: [0.1, 0.2, 0.3, 1.0], width: 4.0, border_color: [0.9, 0.35, 0.05, 1.0] };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.color, cfg.color);
        assert_eq!(back.width, cfg.width);
        assert_eq!(back.border_color, cfg.border_color);
    }

    #[test]
    fn test_config_legacy_json_gets_default_border_color() {
        // Старый формат без поля border_color должен подхватить дефолт
        let json = r#"{"color":[1.0,0.2,0.2,1.0],"width":2.0}"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.border_color, [0.910, 0.349, 0.047, 1.0]);
    }
```

- [ ] **Step 2: Запустить тесты — убедиться, что не компилируется/падает**

Run: `nix develop -c cargo test --lib config 2>&1 | tail -20`
Expected: ошибка компиляции — `cannot find type Config` / `Config` ещё не определён.

- [ ] **Step 3: Переименовать и расширить конфиг в `src/utils.rs`**

Заменить блок `src/utils.rs:29-67` (от `/// Настройки кисти...` до конца `save_brush_config`) на:

```rust
/// Дефолтный цвет рамки окна (#e8590c).
fn default_border_color() -> [f32; 4] {
    [0.910, 0.349, 0.047, 1.0]
}

/// Настройки приложения, сохраняемые между запусками.
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub color: [f32; 4], // RGBA кисти
    pub width: f64,      // толщина кисти
    #[serde(default = "default_border_color")]
    pub border_color: [f32; 4], // RGBA рамки окна
}

impl Default for Config {
    fn default() -> Self {
        Self {
            color: [1.0, 0.2, 0.2, 1.0],
            width: 2.0,
            border_color: default_border_color(),
        }
    }
}

fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join(".config"));
    config_dir.join("snip").join("config.json")
}

pub fn load_config() -> Config {
    let path = config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(config: &Config) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(config) {
        let _ = std::fs::write(&path, json);
    }
}
```

- [ ] **Step 4: Обновить вызов в `src/tools/brush.rs`**

Заменить строку `src/tools/brush.rs:15`:

```rust
        let config = utils::load_brush_config();
```

на:

```rust
        let config = utils::load_config();
```

- [ ] **Step 5: Добавить поле `border_color` в `EditorState` и инициализацию**

В `src/editor.rs` в объявление `struct EditorState` (после `rect: RectTool,`) добавить поле:

```rust
    border_color: gdk::RGBA,
```

В `open_editor`, перед `let state = Rc::new(RefCell::new(EditorState {` (сейчас строка ~73), добавить загрузку конфига:

```rust
    let cfg = utils::load_config();
    let border_color = gdk::RGBA::new(
        cfg.border_color[0],
        cfg.border_color[1],
        cfg.border_color[2],
        cfg.border_color[3],
    );
```

В литерал `EditorState { ... }` добавить поле (после `rect: RectTool::new(),`):

```rust
        border_color,
```

- [ ] **Step 6: Добавить хелперы `rgba_to_arr` и `persist_config` в `src/editor.rs`**

Рядом с `fn color_button_css` (перед ней, ~строка 439) добавить:

```rust
/// Конвертация gdk::RGBA в массив для конфига.
fn rgba_to_arr(c: &gdk::RGBA) -> [f32; 4] {
    [c.red(), c.green(), c.blue(), c.alpha()]
}

/// Сохраняет весь конфиг из текущего состояния (кисть + цвет рамки),
/// чтобы изменение одного не затирало другое.
fn persist_config(st: &EditorState) {
    let c = st.brush.color();
    utils::save_config(&utils::Config {
        color: rgba_to_arr(&c),
        width: st.brush.width(),
        border_color: rgba_to_arr(&st.border_color),
    });
}
```

- [ ] **Step 7: Провести существующие сохранения через `persist_config`**

В кнопке выбора цвета кисти (`src/editor.rs`, блок `btn.connect_clicked` около строк 547-559) заменить вызов сохранения:

```rust
            let mut st = state.borrow_mut();
            st.brush.set_color(color);
            utils::save_brush_config(&utils::BrushConfig {
                color: [color.red(), color.green(), color.blue(), color.alpha()],
                width: st.brush.width(),
            });
            drop(st);
```

на:

```rust
            let mut st = state.borrow_mut();
            st.brush.set_color(color);
            persist_config(&st);
            drop(st);
```

В кнопке выбора толщины (`src/editor.rs`, блок `btn.connect_clicked` около строк 593-604) заменить:

```rust
            let mut st = state.borrow_mut();
            st.brush.set_width(w);
            let color = st.brush.color();
            utils::save_brush_config(&utils::BrushConfig {
                color: [color.red(), color.green(), color.blue(), color.alpha()],
                width: w,
            });
            drop(st);
```

на:

```rust
            let mut st = state.borrow_mut();
            st.brush.set_width(w);
            persist_config(&st);
            drop(st);
```

- [ ] **Step 8: Запустить тесты — убедиться, что всё зелёное**

Run: `nix develop -c cargo test 2>&1 | tail -25`
Expected: PASS — включая `test_config_roundtrip_with_border_color` и `test_config_legacy_json_gets_default_border_color`; старые тесты brush/utils проходят.

- [ ] **Step 9: Проверить сборку без предупреждений о мёртвом коде**

Run: `nix develop -c cargo build 2>&1 | tail -15`
Expected: успешная сборка; нет warning `field border_color is never read` (его читает `persist_config`).

- [ ] **Step 10: Коммит**

```bash
git add src/utils.rs src/tools/brush.rs src/editor.rs
git commit -m "refactor: единый Config с border_color и persist_config"
```

---

## Task 2: Рамка окна редактора

Нарисовать акцентную рамку на окне через выделенный `CssProvider`. Деливерабл — видимая рамка при запуске (цвет из конфига или дефолтный). Проверка визуальная (CSS/окно GTK не покрывается юнит-тестами); существующий набор тестов остаётся зелёным.

**Files:**
- Modify: `src/editor.rs` — константа `WINDOW_BORDER_WIDTH`, функция `window_border_css`, настройка провайдера и класса окна в `open_editor` (после создания `window`, ~строка 93).

**Interfaces:**
- Consumes: `EditorState.border_color`, `rgba_to_arr` (Task 1).
- Produces:
  - `const editor::WINDOW_BORDER_WIDTH: i32`
  - `fn editor::window_border_css(color: &gdk::RGBA) -> String`
  - Локальная переменная `border_css: gtk::CssProvider` в `open_editor` (передаётся в Task 3).

- [ ] **Step 1: Добавить константу ширины рамки**

В `src/editor.rs` рядом с другими константами (после `const WINDOW_HEIGHT_PADDING: i32 = 90;`, ~строка 18) добавить:

```rust
// Ширина акцентной рамки окна
const WINDOW_BORDER_WIDTH: i32 = 3;
```

- [ ] **Step 2: Добавить функцию `window_border_css`**

В `src/editor.rs` рядом с `rgba_to_arr` (Task 1, ~строка 439) добавить:

```rust
/// CSS-правило акцентной рамки окна для заданного цвета.
fn window_border_css(color: &gdk::RGBA) -> String {
    format!(
        "window.snip-bordered {{ border: {}px solid rgba({},{},{},{}); }}",
        WINDOW_BORDER_WIDTH,
        (color.red() * 255.0) as u8,
        (color.green() * 255.0) as u8,
        (color.blue() * 255.0) as u8,
        color.alpha(),
    )
}
```

- [ ] **Step 3: Навесить класс и провайдер рамки на окно**

В `open_editor`, сразу после блока создания `window` (после `.build();`, ~строка 93) и до создания `header` (`let header = ...`), вставить:

```rust
    // === Акцентная рамка окна ===
    window.add_css_class("snip-bordered");
    let display = gdk::Display::default().expect("cannot get default display");
    let border_css = gtk::CssProvider::new();
    border_css.load_from_string(&window_border_css(&state.borrow().border_color));
    gtk::style_context_add_provider_for_display(
        &display,
        &border_css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
```

- [ ] **Step 4: Собрать и запустить редактор для визуальной проверки**

Подготовить тестовый PNG и запустить:

```bash
nix develop -c cargo build 2>&1 | tail -5
nix develop -c cargo run -- edit "$(ls -1 ~/Pictures/Screenshots/snip/*.png 2>/dev/null | head -1 || echo /tmp/nonexistent.png)"
```

Если своего PNG нет — создать заглушку и открыть её:

```bash
nix develop -c bash -c 'cat > /tmp/snip_border_test.png.b64 <<EOF
iVBORw0KGgoAAAANSUhEUgAAAGQAAABkCAYAAABw4pVUAAAAHElEQVR4nO3BAQ0AAADCoPdPbQ43oAAAAAAAAAAAvg0hAAABw6f7lwAAAABJRU5ErkJggg==
EOF
base64 -d /tmp/snip_border_test.png.b64 > /tmp/snip_border_test.png'
nix develop -c cargo run -- edit /tmp/snip_border_test.png
```

Expected: окно открывается с оранжевой (#e8590c) рамкой 3px по периметру, рамка следует скруглению углов окна.

**Контингенция (если рамка рендерится некорректно):** если `border` на узле `window` рисуется за пределами видимой области (по контуру тени) или углы квадратные поверх скруглённых — переключиться на inset-обводку на контейнере содержимого. Для этого: (а) убрать `window.add_css_class("snip-bordered");` и вместо него после сборки `content` (в конце `open_editor`, перед `window.set_content`) добавить `content.add_css_class("snip-bordered");`; (б) в `window_border_css` заменить правило на
`"box.snip-bordered {{ box-shadow: inset 0 0 0 {}px rgba(...); }}"` (inset box-shadow следует border-radius и не сдвигает layout). Селектор провайдера менять синхронно. Выбор фиксируется этим визуальным шагом.

- [ ] **Step 5: Убедиться, что тесты не сломаны**

Run: `nix develop -c cargo test 2>&1 | tail -10`
Expected: PASS (все прежние тесты зелёные).

- [ ] **Step 6: Коммит**

```bash
git add src/editor.rs
git commit -m "feat: акцентная рамка окна редактора"
```

---

## Task 3: Инструмент выбора цвета рамки

Добавить в `HeaderBar` справа кнопку-образец с поповером: сетка 40 пресетов + «Свой цвет…» (нативный `gtk::ColorDialog`). Любой выбор перекрашивает рамку вживую, обновляет образец, сохраняет конфиг. Проверка визуальная + персистентность.

**Files:**
- Modify: `src/editor.rs` — импорт `gtk4::gio`; константа `BORDER_PALETTE`; функции `apply_border_color`, `build_border_color_button`; монтаж кнопки в `HeaderBar` в `open_editor`.

**Interfaces:**
- Consumes: `border_css` (Task 2), `EditorState.border_color`, `persist_config`, `window_border_css`, `color_button_css`, `COLOR_BTN_SIZE`, `COLOR_POPOVER_BTN_SIZE`.
- Produces:
  - `const editor::BORDER_PALETTE: &[(u8, u8, u8)]` (40 элементов)
  - `fn editor::apply_border_color(color: gdk::RGBA, state: &Rc<RefCell<EditorState>>, border_css: &gtk::CssProvider, border_btn_css: &gtk::CssProvider)`
  - `fn editor::build_border_color_button(state: Rc<RefCell<EditorState>>, border_css: &gtk::CssProvider, window: &libadwaita::ApplicationWindow) -> gtk::MenuButton`

- [ ] **Step 1: Добавить импорт gio**

В начало `src/editor.rs` (рядом с `use gtk4::glib;`, ~строка 4) добавить:

```rust
use gtk4::gio;
```

- [ ] **Step 2: Добавить палитру `BORDER_PALETTE`**

В `src/editor.rs` рядом с `COLOR_PALETTE` (после строки 34) добавить:

```rust
// Палитра цветов рамки окна (8 столбцов × 5 строк = 40 пресетов), row-major
const BORDER_PALETTE: &[(u8, u8, u8)] = &[
    (0xff,0x8a,0x80),(0xff,0xcc,0x80),(0xff,0xf1,0x76),(0xc5,0xe1,0xa5),(0x80,0xcb,0xc4),(0x90,0xca,0xf9),(0xce,0x93,0xd8),(0xf8,0xbb,0xd0),
    (0xff,0x52,0x52),(0xff,0xa7,0x26),(0xff,0xe1,0x4d),(0x66,0xbb,0x6a),(0x26,0xa6,0x9a),(0x42,0xa5,0xf5),(0xab,0x47,0xbc),(0xf0,0x62,0x92),
    (0xe0,0x1b,0x24),(0xe8,0x59,0x0c),(0xf5,0xc2,0x11),(0x2e,0x9e,0x44),(0x1a,0x9b,0x8a),(0x35,0x84,0xe4),(0x91,0x41,0xac),(0xe8,0x43,0x93),
    (0xb7,0x1c,0x1c),(0xbf,0x36,0x0c),(0xc7,0x91,0x00),(0x1b,0x5e,0x20),(0x00,0x69,0x5c),(0x15,0x65,0xc0),(0x6a,0x1b,0x9a),(0xad,0x14,0x57),
    (0xff,0xff,0xff),(0xde,0xdd,0xda),(0xc0,0xbf,0xbc),(0x9a,0x99,0x96),(0x77,0x76,0x7b),(0x5e,0x5c,0x64),(0x3d,0x38,0x46),(0x00,0x00,0x00),
];
```

- [ ] **Step 3: Добавить `apply_border_color`**

В `src/editor.rs` рядом с `window_border_css` (~строка 440) добавить:

```rust
/// Применяет выбранный цвет рамки: перекрашивает окно, обновляет образец
/// на кнопке, сохраняет конфиг.
fn apply_border_color(
    color: gdk::RGBA,
    state: &Rc<RefCell<EditorState>>,
    border_css: &gtk::CssProvider,
    border_btn_css: &gtk::CssProvider,
) {
    state.borrow_mut().border_color = color;
    border_css.load_from_string(&window_border_css(&color));
    border_btn_css.load_from_string(&color_button_css(
        "menubutton.snip-border-btn > button", &color, COLOR_BTN_SIZE,
    ));
    let st = state.borrow();
    persist_config(&st);
}
```

- [ ] **Step 4: Добавить `build_border_color_button`**

В `src/editor.rs` рядом с `build_bottom_bar` (например, перед ней, ~строка 451) добавить:

```rust
/// Кнопка выбора цвета рамки окна (в HeaderBar): образец текущего цвета +
/// поповер с сеткой пресетов и нативным пикером «Свой цвет…».
fn build_border_color_button(
    state: Rc<RefCell<EditorState>>,
    border_css: &gtk::CssProvider,
    window: &libadwaita::ApplicationWindow,
) -> gtk::MenuButton {
    let display = gdk::Display::default().expect("cannot get default display");

    // Кнопка-образец текущего цвета рамки
    let border_btn = gtk::MenuButton::new();
    border_btn.set_size_request(COLOR_BTN_SIZE, COLOR_BTN_SIZE);
    border_btn.set_tooltip_text(Some("Цвет рамки окна"));
    border_btn.add_css_class("snip-border-btn");

    let border_btn_css = gtk::CssProvider::new();
    let current = state.borrow().border_color;
    border_btn_css.load_from_string(&color_button_css(
        "menubutton.snip-border-btn > button", &current, COLOR_BTN_SIZE,
    ));
    gtk::style_context_add_provider_for_display(
        &display,
        &border_btn_css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Поповер: сетка пресетов + «Свой цвет…»
    let popover = gtk::Popover::new();
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_start(8).margin_end(8).margin_top(8).margin_bottom(8)
        .build();

    let grid = gtk::Grid::builder().row_spacing(6).column_spacing(6).build();

    for (i, (r, g, b)) in BORDER_PALETTE.iter().enumerate() {
        let rgba = gdk::RGBA::new(*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0, 1.0);
        let btn = gtk::Button::new();
        btn.set_size_request(COLOR_POPOVER_BTN_SIZE, COLOR_POPOVER_BTN_SIZE);

        let css_class = format!("snip-border-sw-{i}");
        let provider = gtk::CssProvider::new();
        provider.load_from_string(&color_button_css(
            &format!("button.{}", css_class), &rgba, COLOR_POPOVER_BTN_SIZE,
        ));
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        btn.add_css_class(&css_class);

        let state = state.clone();
        let border_css = border_css.clone();
        let border_btn_css = border_btn_css.clone();
        let popover = popover.clone();
        btn.connect_clicked(move |_| {
            apply_border_color(rgba, &state, &border_css, &border_btn_css);
            popover.popdown();
        });

        let col = (i % 8) as i32;
        let row = (i / 8) as i32;
        grid.attach(&btn, col, row, 1, 1);
    }

    vbox.append(&grid);

    // «Свой цвет…» — нативный GtkColorDialog
    let custom_btn = gtk::Button::with_label("Свой цвет…");
    {
        let state = state.clone();
        let border_css = border_css.clone();
        let border_btn_css = border_btn_css.clone();
        let popover = popover.clone();
        let window = window.clone();
        custom_btn.connect_clicked(move |_| {
            let dialog = gtk::ColorDialog::new();
            dialog.set_title("Цвет рамки окна");
            dialog.set_with_alpha(false);
            let initial = state.borrow().border_color;
            let state = state.clone();
            let border_css = border_css.clone();
            let border_btn_css = border_btn_css.clone();
            dialog.choose_rgba(
                Some(&window),
                Some(&initial),
                gio::Cancellable::NONE,
                move |result| {
                    if let Ok(color) = result {
                        apply_border_color(color, &state, &border_css, &border_btn_css);
                    }
                },
            );
            popover.popdown();
        });
    }
    vbox.append(&custom_btn);

    popover.set_child(Some(&vbox));
    border_btn.set_popover(Some(&popover));

    border_btn
}
```

- [ ] **Step 5: Смонтировать кнопку в HeaderBar**

В `open_editor`, сразу после `let header = libadwaita::HeaderBar::new();` (~строка 96) добавить:

```rust
    // Кнопка выбора цвета рамки — справа в шапке
    let border_color_btn = build_border_color_button(state.clone(), &border_css, &window);
    header.pack_end(&border_color_btn);
```

- [ ] **Step 6: Собрать**

Run: `nix develop -c cargo build 2>&1 | tail -15`
Expected: успешная сборка без ошибок.

- [ ] **Step 7: Визуальная проверка инструмента**

```bash
nix develop -c cargo run -- edit /tmp/snip_border_test.png
```

Проверить вручную:
1. Справа в шапке — кнопка-образец цвета рамки (оранжевая по умолчанию).
2. Клик открывает поповер с сеткой 8×5 = 40 свотчей и кнопкой «Свой цвет…».
3. Клик по пресету — рамка окна и образец мгновенно перекрашиваются, поповер закрывается.
4. «Свой цвет…» открывает нативный пикер (колесо + HEX); выбор цвета перекрашивает рамку; отмена ничего не меняет.

Expected: все 4 пункта работают.

- [ ] **Step 8: Проверка персистентности**

Закрыть окно после выбора цвета (например, бирюзового), затем снова открыть:

```bash
nix develop -c cargo run -- edit /tmp/snip_border_test.png
```

Expected: рамка открывается выбранным ранее цветом; в `~/.config/snip/config.json` присутствует поле `border_color` с этим значением.

- [ ] **Step 9: Убедиться, что тесты зелёные**

Run: `nix develop -c cargo test 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 10: Коммит**

```bash
git add src/editor.rs
git commit -m "feat: инструмент выбора цвета рамки окна (палитра + свой цвет)"
```

---

## Self-Review

**Spec coverage:**
- Рамка окна (всегда видима, 3px, цвет из конфига) → Task 2. ✓
- Инструмент в HeaderBar справа, сетка 40 пресетов + «Свой цвет…» (ColorDialog) → Task 3. ✓
- Живое перекрашивание + обновление образца + сохранение → `apply_border_color`, Task 3. ✓
- Персистентность, дефолт #e8590c, совместимость старых конфигов → Task 1 (serde default) + тест. ✓
- Только окно редактора (capture не трогаем) → план не касается `capture.rs`. ✓
- Нет опции «выключить рамку» → отсутствует. ✓

**Placeholder scan:** код приведён полностью в каждом шаге; «контингенция» в Task 2 Step 4 содержит конкретный запасной код, а не заглушку.

**Type consistency:** `Config`/`load_config`/`save_config`/`rgba_to_arr`/`persist_config`/`window_border_css`/`apply_border_color`/`build_border_color_button`, поле `border_color`, классы `snip-bordered`/`snip-border-btn`/`snip-border-sw-{i}` используются единообразно во всех задачах.
