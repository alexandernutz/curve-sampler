use nih_plug_egui::egui;
use nih_plug::prelude::Enum;

#[derive(Debug, PartialEq, Clone, Copy, Enum)]
pub enum Theme {
    Auto,
    Dark,
    Light,
}

pub fn apply(ctx: &egui::Context, theme: Theme) {
    match theme {
        Theme::Auto => {
            // Detect system theme from egui context
            let is_dark = ctx.style().visuals.dark_mode;
            // We can also check system_theme() if available in this egui version
            if let Some(system_theme) = ctx.system_theme() {
                match system_theme {
                    egui::Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
                    egui::Theme::Light => ctx.set_visuals(egui::Visuals::light()),
                }
            } else {
                // Fallback to whatever egui thinks is right
                if is_dark { ctx.set_visuals(egui::Visuals::dark()); }
                else { ctx.set_visuals(egui::Visuals::light()); }
            }
        }
        Theme::Dark => {
            ctx.set_visuals(egui::Visuals::dark());
        }
        Theme::Light => {
            ctx.set_visuals(egui::Visuals::light());
        }
    }
}
