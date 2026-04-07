use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Возвращает директорию для снипов: XDG_PICTURES_DIR/Screenshots/snip/
pub fn screenshots_dir() -> PathBuf {
    let pictures = dirs::picture_dir().unwrap_or_else(|| {
        let home = dirs::home_dir().expect("no home directory");
        home.join("Pictures")
    });
    pictures.join("Screenshots").join("snip")
}

/// Генерирует имя файла: screenshot-YYYY-MM-DD_HH-MM-SS.png
pub fn screenshot_filename() -> String {
    let now = chrono::Local::now();
    now.format("snip-shot-%Y-%m-%d_%H-%M-%S.png").to_string()
}

/// Полный путь для нового скриншота. Создаёт директорию если не существует.
pub fn new_screenshot_path() -> std::io::Result<PathBuf> {
    let dir = screenshots_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join(screenshot_filename()))
}



/// Настройки кисти, сохраняемые между запусками.
#[derive(Serialize, Deserialize)]
pub struct BrushConfig {
    pub color: [f32; 4], // RGBA
    pub width: f64,
}

impl Default for BrushConfig {
    fn default() -> Self {
        Self {
            color: [1.0, 0.2, 0.2, 1.0],
            width: 2.0,
        }
    }
}

fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().expect("no home directory").join(".config"));
    config_dir.join("snip").join("config.json")
}

pub fn load_brush_config() -> BrushConfig {
    let path = config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_brush_config(config: &BrushConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(config) {
        let _ = std::fs::write(&path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screenshots_dir_ends_with_screenshots() {
        let dir = screenshots_dir();
        assert_eq!(dir.file_name().unwrap(), "snip");
    }

    #[test]
    fn test_screenshot_filename_format() {
        let name = screenshot_filename();
        assert!(name.starts_with("snip-shot-"));
        assert!(name.ends_with(".png"));
        assert_eq!(name.len(), "snip-shot-2026-04-02_14-35-22.png".len());
    }

}
