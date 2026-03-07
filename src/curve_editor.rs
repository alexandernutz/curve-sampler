use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};

use crate::bezier::BezierCurve;

/// Draw a read-only Bézier curve visualization inside a UI panel.
/// Interactive editing will be added here later.
pub fn show(ui: &mut Ui, curve: &BezierCurve) {
    let desired = Vec2::new(ui.available_width(), ui.available_height().min(400.0));
    let (response, painter) = ui.allocate_painter(desired, Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 0.0, Color32::from_gray(28));

    // Grid lines
    let dim = Stroke::new(1.0, Color32::from_gray(55));
    for &frac in &[0.25f32, 0.5, 0.75] {
        let x = lerp_x(rect, frac);
        painter.vline(x, rect.y_range(), dim);
    }
    // Zero-amplitude line (y = 0.5)
    let mid_y = lerp_y(rect, 0.5);
    painter.hline(rect.x_range(), mid_y, Stroke::new(1.0, Color32::from_gray(80)));

    // Curve polyline
    let n = 512usize;
    let pts: Vec<Pos2> = curve
        .sample(n)
        .into_iter()
        .enumerate()
        .map(|(i, y)| Pos2::new(lerp_x(rect, i as f32 / (n - 1) as f32), lerp_y(rect, y)))
        .collect();
    painter.add(Shape::line(pts, Stroke::new(2.0, Color32::from_rgb(80, 220, 80))));

    // Control points and handles
    for pt in &curve.points {
        let sp = to_screen(rect, pt.position);
        for handle in pt.outgoing.into_iter().chain(pt.incoming) {
            let hp = to_screen(rect, handle);
            painter.line_segment([sp, hp], Stroke::new(1.0, Color32::from_gray(150)));
            painter.circle_filled(hp, 3.0, Color32::from_gray(180));
        }
        painter.circle_filled(sp, 5.0, Color32::WHITE);
    }
}

fn to_screen(rect: Rect, (x, y): (f32, f32)) -> Pos2 {
    Pos2::new(lerp_x(rect, x), lerp_y(rect, y))
}

/// Map curve x (0–1) to screen x.
fn lerp_x(rect: Rect, t: f32) -> f32 {
    rect.left() + t * rect.width()
}

/// Map curve y (0–1, 0 = bottom) to screen y (0 = top).
fn lerp_y(rect: Rect, y: f32) -> f32 {
    rect.top() + (1.0 - y) * rect.height()
}
