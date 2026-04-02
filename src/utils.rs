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
