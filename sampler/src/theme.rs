use nih_plug::prelude::Enum;

#[derive(Debug, PartialEq, Clone, Copy, Enum)]
pub enum ThemeMode {
    #[id = "auto"]
    Auto,
    #[id = "dark"]
    Dark,
    #[id = "light"]
    Light,
}

// ─── Accent colours (same in both themes) ────────────────────────────────────

/// Geometry panel accent — blue
pub const GEO:         nih_plug_egui::egui::Color32 = nih_plug_egui::egui::Color32::from_rgb(0x3d, 0x78, 0xf0);
/// Spectrum panel accent — warm orange
pub const SPEC:        nih_plug_egui::egui::Color32 = nih_plug_egui::egui::Color32::from_rgb(0xd4, 0x5e, 0x20);
/// Stored-curve preview accent — soft green
pub const STORED:      nih_plug_egui::egui::Color32 = nih_plug_egui::egui::Color32::from_rgb(0x3a, 0xa8, 0x5a);
/// Live oscilloscope trace colour (matches Zebralette yellow)
pub const SCOPE_TRACE: nih_plug_egui::egui::Color32 = nih_plug_egui::egui::Color32::from_rgb(0xc4, 0xa0, 0x20);

// ─── Dark palette ────────────────────────────────────────────────────────────

pub mod dark {
    use nih_plug_egui::egui::Color32;
    /// Main window background
    pub const BG_APP:       Color32 = Color32::from_rgb(0x25, 0x28, 0x30);
    /// Recessed panel / canvas interior
    pub const BG_PANEL:     Color32 = Color32::from_rgb(0x1c, 0x1e, 0x25);
    /// Raised surface — title bar, section headers
    pub const BG_SURFACE:   Color32 = Color32::from_rgb(0x2d, 0x30, 0x40);
    /// Primary text
    pub const TEXT:         Color32 = Color32::from_rgb(0xc8, 0xca, 0xd8);
    /// Secondary / label text
    pub const MUTED:        Color32 = Color32::from_rgb(0x5a, 0x60, 0x78);
    /// Placeholder / disabled text
    pub const FAINT:        Color32 = Color32::from_rgb(0x32, 0x38, 0x4a);
    /// Normal border
    pub const BORDER:       Color32 = Color32::from_rgb(0x38, 0x3d, 0x50);
    /// Subtle internal divider
    pub const BORDER_FAINT: Color32 = Color32::from_rgb(0x27, 0x2b, 0x38);
}

// ─── Light palette ───────────────────────────────────────────────────────────

pub mod light {
    use nih_plug_egui::egui::Color32;
    /// Main window background — light cool gray
    pub const BG_APP:       Color32 = Color32::from_rgb(0xe8, 0xe9, 0xed);
    /// Recessed panel / canvas interior — a touch darker
    pub const BG_PANEL:     Color32 = Color32::from_rgb(0xd8, 0xda, 0xe0);
    /// Raised surface — title bar, section headers
    pub const BG_SURFACE:   Color32 = Color32::from_rgb(0xf0, 0xf1, 0xf4);
    /// Primary text — near-black with a cool tint
    pub const TEXT:         Color32 = Color32::from_rgb(0x1e, 0x20, 0x2a);
    /// Secondary / label text
    pub const MUTED:        Color32 = Color32::from_rgb(0x70, 0x74, 0x88);
    /// Placeholder / disabled text
    pub const FAINT:        Color32 = Color32::from_rgb(0xa8, 0xaa, 0xb8);
    /// Normal border
    pub const BORDER:       Color32 = Color32::from_rgb(0xb8, 0xba, 0xc8);
    /// Subtle internal divider
    pub const BORDER_FAINT: Color32 = Color32::from_rgb(0xcc, 0xce, 0xd8);
}

pub fn apply(ctx: &nih_plug_egui::egui::Context, mode: ThemeMode) {
    match mode {
        ThemeMode::Auto => ctx.set_theme(nih_plug_egui::egui::ThemePreference::System),
        ThemeMode::Dark => ctx.set_theme(nih_plug_egui::egui::ThemePreference::Dark),
        ThemeMode::Light => ctx.set_theme(nih_plug_egui::egui::ThemePreference::Light),
    }
}
