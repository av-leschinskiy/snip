use gdk4 as gdk;

use super::Stroke;
use crate::utils;

/// Состояние инструмента "Кисть".
pub struct BrushTool {
    color: gdk::RGBA,
    width: f64,
    current_stroke: Option<Stroke>,
}

impl BrushTool {
    pub fn new() -> Self {
        let config = utils::load_brush_config();
        Self {
            color: gdk::RGBA::new(
                config.color[0],
                config.color[1],
                config.color[2],
                config.color[3],
            ),
            width: config.width,
            current_stroke: None,
        }
    }

    pub fn color(&self) -> gdk::RGBA {
        self.color
    }

    pub fn set_color(&mut self, color: gdk::RGBA) {
        self.color = color;
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn set_width(&mut self, width: f64) {
        self.width = width;
    }

    pub fn press(&mut self, x: f64, y: f64) {
        let mut stroke = Stroke::new(self.color, self.width);
        stroke.add_point(x, y);
        self.current_stroke = Some(stroke);
    }

    pub fn motion(&mut self, x: f64, y: f64) {
        if let Some(ref mut stroke) = self.current_stroke {
            stroke.add_point(x, y);
        }
    }

    pub fn release(&mut self) -> Option<Stroke> {
        self.current_stroke.take().filter(|s| s.points.len() >= 2)
    }

    pub fn current_stroke(&self) -> Option<&Stroke> {
        self.current_stroke.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brush_stroke_lifecycle() {
        let mut brush = BrushTool::new();
        brush.press(10.0, 20.0);
        assert!(brush.current_stroke().is_some());
        brush.motion(30.0, 40.0);
        brush.motion(50.0, 60.0);
        assert_eq!(brush.current_stroke().unwrap().points.len(), 3);
        let stroke = brush.release().unwrap();
        assert_eq!(stroke.points.len(), 3);
        assert!(brush.current_stroke().is_none());
    }

    #[test]
    fn test_brush_single_point_returns_none() {
        let mut brush = BrushTool::new();
        brush.press(10.0, 20.0);
        let stroke = brush.release();
        assert!(stroke.is_none());
    }

    #[test]
    fn test_brush_uses_configured_color_and_width() {
        let mut brush = BrushTool::new();
        brush.set_color(gdk::RGBA::new(0.0, 1.0, 0.0, 1.0));
        brush.set_width(5.0);
        brush.press(0.0, 0.0);
        brush.motion(10.0, 10.0);
        let stroke = brush.release().unwrap();
        assert_eq!(stroke.color, gdk::RGBA::new(0.0, 1.0, 0.0, 1.0));
        assert_eq!(stroke.width, 5.0);
    }
}
