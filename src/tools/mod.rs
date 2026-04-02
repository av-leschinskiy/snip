pub mod brush;

use gdk4 as gdk;

/// Одна аннотация (завершённый штрих, фигура и т.д.)
pub trait Annotation {
    fn draw(&self, cr: &cairo::Context);
}

/// Штрих кисти — набор точек с цветом и толщиной.
#[derive(Clone, Debug)]
pub struct Stroke {
    pub points: Vec<(f64, f64)>,
    pub color: gdk::RGBA,
    pub width: f64,
}

impl Stroke {
    pub fn new(color: gdk::RGBA, width: f64) -> Self {
        Self {
            points: Vec::new(),
            color,
            width,
        }
    }

    pub fn add_point(&mut self, x: f64, y: f64) {
        self.points.push((x, y));
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

impl Annotation for Stroke {
    fn draw(&self, cr: &cairo::Context) {
        if self.points.len() < 2 {
            return;
        }

        cr.set_source_rgba(
            self.color.red() as f64,
            self.color.green() as f64,
            self.color.blue() as f64,
            self.color.alpha() as f64,
        );
        cr.set_line_width(self.width);
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_line_join(cairo::LineJoin::Round);

        let (x0, y0) = self.points[0];
        cr.move_to(x0, y0);
        for &(x, y) in &self.points[1..] {
            cr.line_to(x, y);
        }
        let _ = cr.stroke();
    }
}
