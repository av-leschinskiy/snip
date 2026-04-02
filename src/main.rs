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
                    // TODO: открыть редактор (Task 5)
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
                // TODO: открыть редактор (Task 5)
            });
        }
    }

    app.run_with_args::<String>(&[]);
}
