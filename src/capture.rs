use gtk4::glib;
use gtk4::prelude::*;
use std::path::PathBuf;

use crate::utils;

/// Запускает capture flow: GNOME screenshot dialog → копирование в нашу папку → on_done(path).
pub fn start_capture(app: &libadwaita::Application, on_done: impl Fn(PathBuf) + 'static) {
    let app = app.clone();
    let hold_guard = gio::prelude::ApplicationExtManual::hold(&app);
    glib::spawn_future_local(async move {
        match take_portal_screenshot().await {
            Ok(uri) => {
                let source_path = uri_to_path(&uri);

                // Копируем в нашу папку с нашим форматом имени
                match utils::new_screenshot_path() {
                    Ok(dest_path) => {
                        if let Err(e) = std::fs::copy(&source_path, &dest_path) {
                            eprintln!("Failed to copy screenshot: {e}");
                            return;
                        }
                        on_done(dest_path);
                    }
                    Err(e) => eprintln!("Failed to create screenshot path: {e}"),
                }
                // Дропаем hold ПОСЛЕ on_done — окно редактора уже создано и держит app
                drop(hold_guard);
            }
            Err(e) => {
                eprintln!("Portal screenshot cancelled or failed: {e}");
                drop(hold_guard);
                app.quit();
            }
        }
    });
}

async fn take_portal_screenshot() -> Result<String, Box<dyn std::error::Error>> {
    use ashpd::desktop::screenshot::Screenshot;

    let response = Screenshot::request()
        .interactive(true)
        .send()
        .await?
        .response()?;
    Ok(response.uri().to_string())
}

fn uri_to_path(uri: &str) -> PathBuf {
    if let Ok(url) = url::Url::parse(uri) {
        if let Ok(path) = url.to_file_path() {
            return path;
        }
    }
    PathBuf::from(uri)
}
