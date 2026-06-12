# AGENTS.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Проект

Snip — инструмент скриншотов и аннотаций для GNOME/Wayland на Rust. Два режима:
- `snip` — захват скриншота через XDG Desktop Portal, выбор области, обрезка, открытие в редакторе
- `snip edit <path>` — редактирование существующего PNG

## Команды

```bash
nix develop              # dev shell с GTK4, cairo, libadwaita и т.д.
cargo build              # сборка
cargo run                # capture mode
cargo run -- edit <path> # edit mode
cargo test               # все тесты
cargo test <test_name>   # один тест
cargo clippy             # линтер
cargo fmt                # форматирование
```

Без `nix develop` сборка не пройдёт — pkg-config и GTK4-библиотеки нужны из Nix shell.

## Архитектура

```
main.rs          CLI (clap) + tokio runtime + libadwaita::Application
capture.rs       Portal screenshot (ashpd) → копирование в ~/Pictures/Screenshots → callback
editor.rs        GTK4 окно: canvas, brush, undo/redo, clipboard, save
utils.rs         Пути (XDG screenshots_dir/new_screenshot_path), save_surface_as_png
tools/mod.rs     trait Annotation { fn draw(&self, cr) } + struct Stroke
tools/brush.rs   BrushTool — state machine: press → motion → release
```

### Ключевые паттерны

- **GTK state**: `Rc<RefCell<EditorState>>` — GTK замыкания требуют shared ownership, `RefCell` для interior mutability
- **Async**: tokio runtime (worker_threads=1) создаётся для zbus/ashpd D-Bus I/O, но futures выполняются через GLib executor (`glib::spawn_future_local`)
- **Tool system**: trait `Annotation` с методом `draw(cr)` — новые инструменты реализуют этот трейт. `BrushTool` — state machine, возвращает `Option<Stroke>` при `release()`
- **Координаты**: editor масштабирует canvas, координаты мыши пересчитываются через `scale` в координаты изображения

## Спецификация

Полная спецификация проекта — `docs/superpowers/specs/2026-04-02-snip-screenshot-tool-design.md`.
План реализации — `docs/superpowers/plans/2026-04-02-snip-implementation.md`.

## Соглашения

- Комментарии в коде на русском языке
- Сообщения коммитов на русском, формат: `тип: описание` (feat, fix, refactor, docs, test)
- Минимальная обработка ошибок — только реальные сценарии, без лишних абстракций
- Тесты встроены в модули (`#[cfg(test)]`)
