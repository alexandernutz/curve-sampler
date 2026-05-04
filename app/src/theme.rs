use eframe::egui;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Theme {
    Auto,
    Dark,
    Light,
}

pub fn apply(ctx: &egui::Context, theme: Theme) {
    match theme {
        Theme::Auto => {
            // Browsers/eframe handle this well by default.
        }
        Theme::Dark => {
            ctx.set_visuals(egui::Visuals::dark());
        }
        Theme::Light => {
            ctx.set_visuals(egui::Visuals::light());
        }
    }
}
