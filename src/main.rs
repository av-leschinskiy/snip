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
        .build();

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
