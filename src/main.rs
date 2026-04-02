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
