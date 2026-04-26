use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::fs::OpenOptions;
use std::io::Write;
use curve_core::bezier::fit_bezier;
use curve_core::zebra_format;

const BUFFER_SIZE: usize = 16384;

fn log_to_file(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/curve_extractor.log")
    {
        let _ = writeln!(file, "[{}] {}", 
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(),
            msg);
    }
}

struct CurveExtractor {
    params: Arc<CurveExtractorParams>,
    audio_buffer: Vec<f32>,
    write_idx: usize,
    current_freq: f32,
    trigger_capture: Arc<AtomicBool>,
    captured_string: Arc<Mutex<String>>,
}

#[derive(Params)]
struct CurveExtractorParams {
    pub editor_state: Arc<EguiState>,

    #[id = "domain"]
    pub domain: EnumParam<CurveDomain>,
}

#[derive(Enum, PartialEq, Clone, Copy)]
pub enum CurveDomain {
    Geometry,
    Spectrum,
}

impl Default for CurveExtractor {
    fn default() -> Self {
        log_to_file("CurveExtractor::default()");
        
        std::panic::set_hook(Box::new(|info| {
            let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };
            let location = info.location().map(|l| format!(" at {}:{}", l.file(), l.line())).unwrap_or_default();
            log_to_file(&format!("PANIC: {}{}", msg, location));
        }));

        Self {
            params: Arc::new(CurveExtractorParams::default()),
            audio_buffer: vec![0.0; BUFFER_SIZE],
            write_idx: 0,
            current_freq: 440.0,
            trigger_capture: Arc::new(AtomicBool::new(false)),
            captured_string: Arc::new(Mutex::new(String::from("No curve captured yet."))),
        }
    }
}

impl Default for CurveExtractorParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(400, 300),
            domain: EnumParam::new("Domain", CurveDomain::Geometry),
        }
    }
}

impl Plugin for CurveExtractor {
    const NAME: &'static str = "Curve Extractor";
    const VENDOR: &'static str = "Curve Transform Project";
    const URL: &'static str = "https://github.com/alexandernutz/svg-osc_gem";
    const EMAIL: &'static str = "info@example.com";
    const VERSION: &'static str = "0.1.1777204830";

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: std::num::NonZeroU32::new(1),
            main_output_channels: std::num::NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: std::num::NonZeroU32::new(2),
            main_output_channels: std::num::NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        log_to_file("CurveExtractor::editor() called");
        let trigger = self.trigger_capture.clone();
        let captured = self.captured_string.clone();
        let params = self.params.clone();
        
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_ctx, _user_state| {
                log_to_file("egui init closure");
            },
            move |egui_ctx, setter, _user_state| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Curve Extractor");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("v{}", Self::VERSION));
                        });
                    });
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("Capture Single Cycle").clicked() {
                            trigger.store(true, Ordering::SeqCst);
                            log_to_file("Capture button clicked");
                        }

                        ui.add_space(20.0);
                        
                        // Handle the EnumParam correctly
                        let mut current_domain = params.domain.value();
                        ui.label("Format:");
                        let r1 = ui.radio_value(&mut current_domain, CurveDomain::Geometry, "Geo");
                        let r2 = ui.radio_value(&mut current_domain, CurveDomain::Spectrum, "Spec");
                        
                        if r1.changed() || r2.changed() {
                            setter.begin_set_parameter(&params.domain);
                            setter.set_parameter(&params.domain, current_domain);
                            setter.end_set_parameter(&params.domain);
                        }
                    });

                    ui.add_space(10.0);
                    ui.label("Zebra 3 Clipboard String:");

                    if let Some(mut text) = captured.try_lock() {
                        egui::ScrollArea::vertical()
                            .id_salt("log_scroll")
                            .max_height(150.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut *text)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                );
                            });

                        ui.add_space(10.0);
                        if ui.button("📋 Copy to Clipboard").clicked() {
                            egui_ctx.copy_text(text.clone());
                        }
                    } else {
                        ui.label("Waiting for audio thread...");
                    }
                });
            },
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let sample_rate = context.transport().sample_rate;
        
        while let Some(event) = context.next_event() {
            if let NoteEvent::NoteOn { note, .. } = event {
                self.current_freq = 440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0);
            }
        }

        let mut trigger = self.trigger_capture.load(Ordering::SeqCst);
        
        for mut samples in buffer.iter_samples() {
            let sample = *samples.iter_mut().next().unwrap_or(&mut 0.0);
            
            let prev_idx = if self.write_idx == 0 { BUFFER_SIZE - 1 } else { self.write_idx - 1 };
            let prev_sample = self.audio_buffer[prev_idx];
            
            self.audio_buffer[self.write_idx] = sample;
            
            // Check for positive-going zero crossing
            if trigger && prev_sample <= 0.0 && sample > 0.0 {
                self.perform_extraction(sample_rate);
                self.trigger_capture.store(false, Ordering::SeqCst);
                trigger = false;
            }
            
            self.write_idx = (self.write_idx + 1) % BUFFER_SIZE;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for CurveExtractor {
    const CLAP_ID: &'static str = "com.alexandernutz.curve-extractor";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Extract curves from audio");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Utility];
}

impl Vst3Plugin for CurveExtractor {
    const VST3_CLASS_ID: [u8; 16] = *b"CurveExtractor01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

impl CurveExtractor {
    fn perform_extraction(&self, sample_rate: f32) {
        let period_samples = sample_rate / self.current_freq;
        if period_samples < 2.0 || period_samples > (BUFFER_SIZE / 2) as f32 {
            return;
        }

        // We just crossed zero at self.write_idx.
        // We want the samples from the cycle that just FINISHED, 
        // which starts at (write_idx - period) and ends at write_idx.
        
        let mut cycle = Vec::with_capacity(1024);
        let start_pos = (self.write_idx as f32 + BUFFER_SIZE as f32 - period_samples) % BUFFER_SIZE as f32;
        
        for i in 0..1024 {
            let t = i as f32 / 1024.0;
            let offset = t * period_samples;
            let pos = (start_pos + offset) % BUFFER_SIZE as f32;
            let sample = self.get_cubic_sample_at(pos);
            cycle.push(sample);
        }

        // Peak normalization
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for &s in &cycle {
            if s < min { min = s; }
            if s > max { max = s; }
        }
        let range = (max - min).max(1e-6);
        for s in &mut cycle {
            // Map [min, max] to [0.0, 1.0]
            *s = (*s - min) / range;
        }

        let mut curve = fit_bezier(&cycle, 20, 1, "Peaks And Valleys");
        curve.simplify(0.005);
        
        if let Some(mut captured) = self.captured_string.try_lock() {
            match self.params.domain.value() {
                CurveDomain::Geometry => {
                    *captured = zebra_format::generate(&curve);
                }
                CurveDomain::Spectrum => {
                    let spec_curve = curve_core::transform::geometry_to_spectrum_steps(&curve, 2048)
                        .unwrap_or(curve);
                    *captured = zebra_format::generate(&spec_curve);
                }

            }
        }
    }

    fn get_cubic_sample_at(&self, pos: f32) -> f32 {
        let i1 = pos.floor() as usize;
        let i0 = if i1 == 0 { BUFFER_SIZE - 1 } else { i1 - 1 };
        let i2 = (i1 + 1) % BUFFER_SIZE;
        let i3 = (i1 + 2) % BUFFER_SIZE;
        
        let t = pos - i1 as f32;
        
        let y0 = self.audio_buffer[i0];
        let y1 = self.audio_buffer[i1];
        let y2 = self.audio_buffer[i2];
        let y3 = self.audio_buffer[i3];
        
        let a = -0.5 * y0 + 1.5 * y1 - 1.5 * y2 + 0.5 * y3;
        let b = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
        let c = -0.5 * y0 + 0.5 * y2;
        let d = y1;
        
        a * t * t * t + b * t * t + c * t + d
    }
}

nih_export_clap!(CurveExtractor);
nih_export_vst3!(CurveExtractor);
