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
            // egui follows system preference by default if we don't override it,
            // but we can force it to stay in sync or apply custom tweaks here.
        }
        Theme::Dark => {
            ctx.set_visuals(egui::Visuals::dark());
        }
        Theme::Light => {
            ctx.set_visuals(egui::Visuals::light());
        }
    }
    
    // This is where you can add your custom color schemes later!
    // let mut visuals = ctx.style().visuals.clone();
    // visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 180, 255);
    // ctx.set_visuals(visuals);
}
