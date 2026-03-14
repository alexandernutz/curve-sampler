use egui::{CentralPanel, Context, SidePanel, TopBottomPanel};

use crate::{bezier::BezierCurve, zebra_format};

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
    playing: bool,
    play_as: PlayAs,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            paste_input: String::new(),
            copy_output: String::new(),
            curve: None,
            fft_size: 2048,
            status: "Ready — paste a Zebra3 curve to begin.".into(),
            play_freq: 220.0,
            playing: false,
            play_as: PlayAs::Waveform,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("Zebra Curve Transform");
        });

        SidePanel::left("io").min_width(280.0).show(ctx, |ui| {
            // ── Presets ──────────────────────────────────────────────────────
            ui.label("Geometry presets:");
            ui.horizontal_wrapped(|ui| {
                for &(label, gen) in crate::waveforms::GEO_PRESETS {
                    if ui.small_button(label).clicked() {
                        self.paste_input = gen();
                    }
                }
            });
            ui.label("Spectrum presets:");
            ui.horizontal_wrapped(|ui| {
                for &(label, gen) in crate::waveforms::SPEC_PRESETS {
                    if ui.small_button(label).clicked() {
                        self.paste_input = gen();
                    }
                }
            });
            ui.separator();

            // ── Input ─────────────────────────────────────────────────────────
            ui.heading("Input (paste from Zebra3)");
            egui::ScrollArea::vertical()
                .id_salt("paste_scroll")
                .max_height(120.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.paste_input)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    );
                });

            if ui.button("Load Curve").clicked() {
                match zebra_format::parse(&self.paste_input) {
                    Ok(c) => {
                        self.status = format!("Loaded {} points.", c.points.len());
                        self.curve = Some(c);
                        self.copy_output.clear();
                    }
                    Err(e) => self.status = format!("Parse error: {e}"),
                }
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui.button("Geometry → Spectrum").clicked() {
                    self.run_transform(Direction::Forward);
                }
                if ui.button("Spectrum → Geometry").clicked() {
                    self.run_transform(Direction::Inverse);
                }
            });

            ui.add_space(6.0);
            ui.separator();

            ui.heading("Output (copy to Zebra3)");
            egui::ScrollArea::vertical()
                .id_salt("copy_scroll")
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

            ui.add_space(6.0);
            ui.separator();

            ui.heading("Playback");
            ui.add(
                egui::Slider::new(&mut self.play_freq, 55.0..=880.0)
                    .text("Hz")
                    .logarithmic(true),
            );
            ui.horizontal(|ui| {
                ui.label("Render as:");
                ui.radio_value(&mut self.play_as, PlayAs::Waveform, "Waveform");
                ui.radio_value(&mut self.play_as, PlayAs::Spectrum, "Spectrum");
            });
            ui.horizontal(|ui| {
                let play_label = if self.playing { "■ Stop" } else { "▶ Play" };
                if ui.button(play_label).clicked() {
                    if self.playing {
                        crate::audio::stop();
                        self.playing = false;
                    } else if let Some(c) = &self.curve {
                        let samples = match self.play_as {
                            PlayAs::Waveform => Some(c.sample(2048)),
                            PlayAs::Spectrum => {
                                let s = crate::transform::spectrum_to_waveform_samples(c, self.fft_size);
                                Some(s)
                            }
                        };
                        #[cfg(target_arch = "wasm32")]
                        if let Some(s) = samples {
                            crate::audio::play_once(&s, self.play_freq);
                            self.playing = true;
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        { let _ = samples; self.status = "Audio only available in browser.".into(); }
                    }
                }
            });

            ui.add_space(6.0);
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
    Forward,
    Inverse,
}

impl App {
    fn run_transform(&mut self, dir: Direction) {
        crate::audio::stop();
        self.playing = false;

        let Some(curve) = &self.curve else {
            self.status = "No curve loaded.".into();
            return;
        };
        let result = match dir {
            Direction::Forward => {
                crate::transform::geometry_to_spectrum(curve, self.fft_size)
            }
            Direction::Inverse => {
                crate::transform::spectrum_to_geometry(curve, self.fft_size)
            }
        };
        match result {
            Ok(new_curve) => {
                self.copy_output = zebra_format::generate(&new_curve);
                self.status = "Transformed.".into();
                self.curve = Some(new_curve);
            }
            Err(e) => self.status = format!("Transform error: {e}"),
        }
    }
}
