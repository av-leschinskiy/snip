mod capture;
mod editor;
mod tools;
mod utils;

use clap::{Parser, Subcommand};
use gtk4 as gtk;
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

    // Tokio reactor нужен для zbus/ashpd (D-Bus I/O).
    // GLib executor поллит futures, но I/O операции zbus используют Tokio reactor.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");
    let _guard = rt.enter();

    let app = libadwaita::Application::builder()
        .application_id("dev.snip.app")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    // Регистрируем пути к иконкам приложения. Системные XDG-пути
    // (/usr/share/icons и т.д.) уже учитываются IconTheme по умолчанию,
    // здесь добавляем пути для запуска из исходников/cargo-сборки.
    app.connect_startup(|_| {
        if let Some(display) = gdk4::Display::default() {
            let theme = gtk::IconTheme::for_display(&display);
            for candidate in icon_search_candidates() {
                if candidate.is_dir() {
                    theme.add_search_path(&candidate);
                }
            }
        }
    });

    match cli.command {
        None => {
            let activated = std::cell::Cell::new(false);
            app.connect_activate(move |app| {
                if activated.get() {
                    return;
                }
                activated.set(true);
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

/// Кандидаты директорий с иконками, которые стоит проверить при старте.
/// Возвращаются пути к hicolor-корню (без `/hicolor/...` суффикса) — IconTheme
/// сам разберётся со структурой `hicolor/scalable/apps/...`.
fn icon_search_candidates() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    // Запуск из корня проекта (cargo run).
    paths.push(std::path::PathBuf::from("data/icons"));

    // Рядом с исполняемым файлом: target/debug/snip → ../../data/icons.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("../../data/icons"));
            paths.push(dir.join("../share/icons"));
        }
    }

    paths
}
