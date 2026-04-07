use gdk4 as gdk;

use super::RectAnnotation;

/// Состояние инструмента "Прямоугольник".
pub struct RectTool {
    start: Option<(f64, f64)>,
    current: Option<(f64, f64)>,
}

const MIN_RECT_SIZE: f64 = 2.0;

impl RectTool {
    pub fn new() -> Self {
        Self {
            start: None,
            current: None,
        }
    }

    pub fn press(&mut self, x: f64, y: f64) {
        self.start = Some((x, y));
        self.current = Some((x, y));
    }

    pub fn motion(&mut self, x: f64, y: f64) {
        if self.start.is_some() {
            self.current = Some((x, y));
        }
    }

    /// Нормализация координат: start/end → (x, y, w, h) с положительными размерами.
    fn normalize(start: (f64, f64), end: (f64, f64)) -> (f64, f64, f64, f64) {
        let (x, w) = if end.0 >= start.0 {
            (start.0, end.0 - start.0)
        } else {
            (end.0, start.0 - end.0)
        };
        let (y, h) = if end.1 >= start.1 {
            (start.1, end.1 - start.1)
        } else {
            (end.1, start.1 - end.1)
        };
        (x, y, w, h)
    }

    pub fn release(&mut self, color: gdk::RGBA, line_width: f64) -> Option<RectAnnotation> {
        let start = self.start.take()?;
        let end = self.current.take()?;
        let (x, y, w, h) = Self::normalize(start, end);
        // Игнорировать слишком маленькие прямоугольники (клик без drag)
        if w < MIN_RECT_SIZE && h < MIN_RECT_SIZE {
            return None;
        }
        Some(RectAnnotation { x, y, width: w, height: h, color, line_width })
    }

    /// Preview текущего прямоугольника для отрисовки во время drag.
    pub fn current_rect(&self, color: gdk::RGBA, line_width: f64) -> Option<RectAnnotation> {
        let start = self.start?;
        let end = self.current?;
        let (x, y, w, h) = Self::normalize(start, end);
        Some(RectAnnotation { x, y, width: w, height: h, color, line_width })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_lifecycle() {
        let mut tool = RectTool::new();
        let color = gdk::RGBA::new(1.0, 0.0, 0.0, 1.0);
        tool.press(10.0, 20.0);
        tool.motion(50.0, 60.0);
        let rect = tool.release(color, 2.0).unwrap();
        assert_eq!(rect.x, 10.0);
        assert_eq!(rect.y, 20.0);
        assert_eq!(rect.width, 40.0);
        assert_eq!(rect.height, 40.0);
    }

    #[test]
    fn test_rect_reverse_direction() {
        let mut tool = RectTool::new();
        let color = gdk::RGBA::new(1.0, 0.0, 0.0, 1.0);
        tool.press(50.0, 60.0);
        tool.motion(10.0, 20.0);
        let rect = tool.release(color, 2.0).unwrap();
        assert_eq!(rect.x, 10.0);
        assert_eq!(rect.y, 20.0);
        assert_eq!(rect.width, 40.0);
        assert_eq!(rect.height, 40.0);
    }

    #[test]
    fn test_rect_click_without_drag_returns_none() {
        let mut tool = RectTool::new();
        let color = gdk::RGBA::new(1.0, 0.0, 0.0, 1.0);
        tool.press(10.0, 20.0);
        let rect = tool.release(color, 2.0);
        assert!(rect.is_none());
    }
}
