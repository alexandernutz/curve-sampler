use egui::{CentralPanel, Context, SidePanel, TopBottomPanel};

use curve_core::bezier::BezierCurve;
use curve_core::zebra_format;

#[derive(PartialEq)]
enum PlayAs {
    Waveform,
    Spectrum,
}

pub struct App {
    paste_input: String,
    copy_output: String,
    curve: Option<BezierCurve>,
    fft_size: usize,
    status: String,
    play_freq: f32,
    volume: f32,
    playing: bool,
    play_as: PlayAs,
    /// All curve states; `history[history_idx]` is the current state.
    history: Vec<BezierCurve>,
    history_idx: usize,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            paste_input: String::new(),
            copy_output: String::new(),
            curve: None,
            fft_size: 2048,
            status: "Ready — paste a Zebra(lette) 3 curve or click a preset.".into(),
            play_freq: 220.0,
            volume: 0.7,
            playing: false,
            play_as: PlayAs::Waveform,
            history: Vec::new(),
            history_idx: 0,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts — suppressed while any text widget has focus.
        if !ctx.wants_keyboard_input() {
            let (ctrl_z, ctrl_y, ctrl_shift_z) = ctx.input(|i| (
                i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Z),
                i.modifiers.ctrl && i.key_pressed(egui::Key::Y),
                i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z),
            ));
            if ctrl_z { self.undo(); }
            if ctrl_y || ctrl_shift_z { self.redo(); }
            if ctx.input(|i| i.key_pressed(egui::Key::L)) {
                self.load_curve();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                self.toggle_play();
            }
        }

        TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("Curve Jumbler");
        });

        SidePanel::left("io").min_width(280.0).show(ctx, |ui| {
            // ── SVG input ─────────────────────────────────────────────────────
            ui.heading("Curve Input");
            ui.label("1. Paste a curve from Zebra(lette) 3, or click a preset below.");
            ui.label("2. Press Load Curve [L].");
            ui.label("3. Transform / Playback / Export.");
            ui.add_space(4.0);
            ui.label("Geometry presets:");
            ui.horizontal_wrapped(|ui| {
                for &(label, gen) in crate::waveforms::GEO_PRESETS {
                    if ui.small_button(label).clicked() {
                        let text = gen();
                        self.paste_input = text.clone();
                        self.copy_output = text;
                    }
                }
            });
            ui.label("Spectrum presets:");
            ui.horizontal_wrapped(|ui| {
                for &(label, gen) in crate::waveforms::SPEC_PRESETS {
                    if ui.small_button(label).clicked() {
                        let text = gen();
                        self.paste_input = text.clone();
                        self.copy_output = text;
                    }
                }
            });
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .id_salt("paste_scroll")
                .min_scrolled_height(120.0)
                .max_height(120.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.paste_input)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    );
                });
            if ui.button("Load Curve  [L]").clicked() {
                self.load_curve();
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(self.history_idx > 0, egui::Button::new("Undo [Ctrl+Z]")).clicked() {
                    self.undo();
                }
                if ui.add_enabled(self.history_idx + 1 < self.history.len(), egui::Button::new("Redo [Ctrl+Y]")).clicked() {
                    self.redo();
                }
            });
            ui.separator();

            // ── Curve Transforms ──────────────────────────────────────────────
            ui.heading("Curve Transforms");
            ui.horizontal(|ui| {
                if ui.button("G->S Smooth").clicked() {
                    self.run_transform(Direction::ForwardSmooth);
                }
                if ui.button("G->S Steps").clicked() {
                    self.run_transform(Direction::ForwardSteps);
                }
                if ui.button("S->G").clicked() {
                    self.run_transform(Direction::Inverse);
                }
            });

            ui.label("Geometry:");
            ui.horizontal_wrapped(|ui| {
                use crate::curve_ops as ops;
                let btns: &[(&str, fn(&BezierCurve) -> BezierCurve)] = &[
                    ("Flip Y",    ops::flip_y),
                    ("Flip X",    ops::flip_x),
                    ("Rectify",   ops::rectify),
                    ("Fold",      ops::fold),
                    ("Quantize",  ops::quantize),
                    ("DC",        ops::dc_remove),
                    ("Jitter",    ops::geo_jitter),
                ];
                for &(label, op) in btns {
                    if ui.small_button(label).clicked() {
                        self.apply_op(op, label);
                    }
                }
            });

            ui.label("Spectrum:");
            ui.horizontal_wrapped(|ui| {
                use crate::curve_ops as ops;
                let btns: &[(&str, fn(&BezierCurve) -> BezierCurve)] = &[
                    ("Brighten",  ops::brighten),
                    ("Darken",    ops::darken),
                    ("Thin",      ops::thin),
                    ("Oct Up",    ops::octave_up),
                    ("Jitter",    ops::spec_jitter),
                ];
                for &(label, op) in btns {
                    if ui.small_button(label).clicked() {
                        self.apply_op(op, label);
                    }
                }
            });

            if ui.small_button("Harmonize").clicked() {
                self.harmonize();
            }
            ui.separator();

            // ── Playback ──────────────────────────────────────────────────────
            ui.heading("Playback");
            ui.add(
                egui::Slider::new(&mut self.play_freq, 55.0..=880.0)
                    .text("Hz")
                    .logarithmic(true),
            );
            let vol_resp = ui.add(
                egui::Slider::new(&mut self.volume, 0.0..=1.0)
                    .text("Volume"),
            );
            if vol_resp.changed() {
                crate::audio::set_gain(self.volume);
            }
            ui.horizontal(|ui| {
                ui.label("Render as:");
                ui.radio_value(&mut self.play_as, PlayAs::Waveform, "Waveform");
                ui.radio_value(&mut self.play_as, PlayAs::Spectrum, "Spectrum");
            });
            let play_label = if self.playing { "■ Stop  [Space]" } else { "▶ Play  [Space]" };
            if ui.button(play_label).clicked() {
                self.toggle_play();
            }
            ui.separator();

            // ── Export ────────────────────────────────────────────────────────
            ui.heading("Export");
            egui::ScrollArea::vertical()
                .id_salt("copy_scroll")
                .min_scrolled_height(120.0)
                .max_height(120.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.copy_output)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    );
                });
            if ui.button("Copy to clipboard").clicked() {
                ctx.copy_text(self.copy_output.clone());
            }
            ui.separator();

            ui.label(&self.status);
        }); // SidePanel

        CentralPanel::default().show(ctx, |ui| match &self.curve {
            Some(c) => crate::curve_editor::show(ui, c),
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label("No curve loaded");
                });
            }
        });
    }
}

enum Direction {
    ForwardSmooth,
    ForwardSteps,
    Inverse,
}

impl App {
    /// Push `curve` as the new current state, discarding any forward redo history.
    fn set_curve(&mut self, curve: BezierCurve) {
        self.history.truncate(self.history_idx + if self.history.is_empty() { 0 } else { 1 });
        self.history.push(curve.clone());
        self.history_idx = self.history.len() - 1;
        // Keep at most 50 entries
        if self.history.len() > 50 {
            self.history.drain(0..self.history.len() - 50);
            self.history_idx = self.history.len() - 1;
        }
        self.curve = Some(curve);
    }

    fn undo(&mut self) {
        if self.history_idx > 0 {
            self.history_idx -= 1;
            let c = self.history[self.history_idx].clone();
            self.copy_output = zebra_format::generate(&c);
            self.curve = Some(c);
            self.status = format!("Undo ({}/{}).", self.history_idx + 1, self.history.len());
        } else {
            self.status = "Nothing to undo.".into();
        }
    }

    fn redo(&mut self) {
        if self.history_idx + 1 < self.history.len() {
            self.history_idx += 1;
            let c = self.history[self.history_idx].clone();
            self.copy_output = zebra_format::generate(&c);
            self.curve = Some(c);
            self.status = format!("Redo ({}/{}).", self.history_idx + 1, self.history.len());
        } else {
            self.status = "Nothing to redo.".into();
        }
    }

    fn load_curve(&mut self) {
        match zebra_format::parse(&self.paste_input) {
            Ok(c) => {
                self.status = format!("Loaded {} points.", c.points.len());
                self.set_curve(c);
            }
            Err(e) => self.status = format!("Parse error: {e}"),
        }
    }

    fn toggle_play(&mut self) {
        if self.playing {
            crate::audio::stop();
            self.playing = false;
        } else if let Some(c) = &self.curve {
            #[cfg(target_arch = "wasm32")]
            {
                match self.play_as {
                    PlayAs::Waveform => {
                        let samples = c.sample(2048);
                        crate::audio::play_once(&samples, self.play_freq, self.volume);
                    }
                    PlayAs::Spectrum => {
                        // Use PeriodicWave additive synthesis — browser bandlimits per pitch,
                        // no aliasing (unlike an IFFT buffer played at high playback_rate).
                        let harmonics = curve_core::transform::sample_log_freq(c, self.fft_size / 2);
                        crate::audio::play_spectrum(&harmonics, self.play_freq, self.volume);
                    }
                }
                self.playing = true;
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.status = "Audio only available in browser.".into();
            }
        }
    }

    fn apply_op(&mut self, op: fn(&curve_core::bezier::BezierCurve) -> curve_core::bezier::BezierCurve, label: &str) {
        crate::audio::stop();
        self.playing = false;
        if let Some(curve) = self.curve.clone() {
            let new_curve = op(&curve);
            self.copy_output = zebra_format::generate(&new_curve);
            self.status = format!("{label}.");
            self.set_curve(new_curve);
        } else {
            self.status = "No curve loaded.".into();
        }
    }

    fn harmonize(&mut self) {
        // G→S→G in one click: symmetrises the waveform (phase is discarded, then reconstructed).
        self.run_transform(Direction::ForwardSmooth);
        self.run_transform(Direction::Inverse);
    }

    fn run_transform(&mut self, dir: Direction) {
        crate::audio::stop();
        self.playing = false;

        let Some(curve) = &self.curve else {
            self.status = "No curve loaded.".into();
            return;
        };
        let result = match dir {
            Direction::ForwardSmooth => {
                curve_core::transform::geometry_to_spectrum(curve, self.fft_size)
            }
            Direction::ForwardSteps => {
                curve_core::transform::geometry_to_spectrum_steps(curve, self.fft_size)
            }
            Direction::Inverse => {
                curve_core::transform::spectrum_to_geometry(curve, self.fft_size)
            }
        };
        match result {
            Ok(new_curve) => {
                self.copy_output = zebra_format::generate(&new_curve);
                self.status = "Transformed.".into();
                self.set_curve(new_curve);
            }
            Err(e) => self.status = format!("Transform error: {e}"),
        }
    }
}
