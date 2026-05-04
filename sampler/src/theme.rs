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
        Theme::Auto => ctx.set_theme(egui::ThemePreference::System),
        Theme::Dark => ctx.set_theme(egui::ThemePreference::Dark),
        Theme::Light => ctx.set_theme(egui::ThemePreference::Light),
    }
}
