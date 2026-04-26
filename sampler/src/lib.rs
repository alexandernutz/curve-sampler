use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering, AtomicU32, AtomicUsize};
use std::fs::OpenOptions;
use std::io::Write;
use curve_core::bezier::{fit_bezier, fit_bezier_interp};
use curve_core::zebra_format;

const BUFFER_SIZE: usize = 32768;

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

#[derive(Enum, PartialEq, Clone, Copy, Debug)]
pub enum CurveDomain {
    Geometry,
    Spectrum,
}

struct CurveSampler {
    params: Arc<CurveSamplerParams>,
    
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    write_idx: Arc<AtomicUsize>,
    current_freq: Arc<AtomicU32>,
    sample_rate: Arc<AtomicU32>,
    
    trigger_capture: Arc<AtomicBool>,
    captured_string: Arc<Mutex<String>>,
    
    raw_cycle: Arc<Mutex<Vec<f32>>>,
    capture_ready: Arc<AtomicBool>,
    current_capture_domain: Arc<Mutex<CurveDomain>>,

    ema_period: Arc<Mutex<f32>>,
    ema_magnitudes: Arc<Mutex<Vec<f32>>>,
}

#[derive(Params)]
struct CurveSamplerParams {
    pub editor_state: Arc<EguiState>,

    #[id = "domain"]
    pub domain: EnumParam<CurveDomain>,
    
    #[id = "norm"]
    pub normalize_capture: BoolParam,
}

impl Default for CurveSampler {
    fn default() -> Self {
        log_to_file("CurveSampler::default()");
        
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
            params: Arc::new(CurveSamplerParams::default()),
            audio_buffer: Arc::new(Mutex::new(vec![0.0; BUFFER_SIZE])),
            write_idx: Arc::new(AtomicUsize::new(0)),
            current_freq: Arc::new(AtomicU32::new(440.0f32.to_bits())),
            sample_rate: Arc::new(AtomicU32::new(48000.0f32.to_bits())),
            trigger_capture: Arc::new(AtomicBool::new(false)),
            captured_string: Arc::new(Mutex::new(String::from("No curve captured yet."))),
            raw_cycle: Arc::new(Mutex::new(Vec::with_capacity(1024))),
            capture_ready: Arc::new(AtomicBool::new(false)),
            current_capture_domain: Arc::new(Mutex::new(CurveDomain::Geometry)),
            ema_period: Arc::new(Mutex::new(100.0)),
            ema_magnitudes: Arc::new(Mutex::new(vec![0.0; 1024])),
        }
    }
}

impl Default for CurveSamplerParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(700, 600),
            domain: EnumParam::new("Domain", CurveDomain::Geometry),
            normalize_capture: BoolParam::new("Normalize", true),
        }
    }
}

impl Plugin for CurveSampler {
    const NAME: &'static str = "Curve Sampler";
    const VENDOR: &'static str = "Curve Transform Project";
    const URL: &'static str = "https://github.com/alexandernutz/svg-osc_gem";
    const EMAIL: &'static str = "info@example.com";
    const VERSION: &'static str = "0.1.21";

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
        log_to_file("CurveSampler::editor() called");
        let trigger = self.trigger_capture.clone();
        let captured = self.captured_string.clone();
        let raw_cycle = self.raw_cycle.clone();
        let capture_ready = self.capture_ready.clone();
        let current_capture_domain = self.current_capture_domain.clone();
        let params = self.params.clone();
        
        let audio_buffer_mutex = self.audio_buffer.clone();
        let write_idx_atomic = self.write_idx.clone();
        let current_freq_atomic = self.current_freq.clone();
        let sample_rate_atomic = self.sample_rate.clone();

        let ema_period_mutex = self.ema_period.clone();
        let ema_mags_mutex = self.ema_magnitudes.clone();

        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_ctx, _user_state| {
                log_to_file("egui init closure");
            },
            move |egui_ctx, setter, _user_state| {
                // Check if a new capture is ready to be processed
                if capture_ready.load(Ordering::SeqCst) {
                    if let (Some(mut raw_data), Some(domain)) = (raw_cycle.try_lock(), current_capture_domain.try_lock()) {
                        if !raw_data.is_empty() {
                            let mut curve = match *domain {
                                CurveDomain::Geometry => {
                                    let mut c = fit_bezier_interp(&raw_data, 64, 1, "Peaks And Valleys");
                                    c.simplify(0.002);
                                    c
                                }
                                CurveDomain::Spectrum => {
                                    curve_core::transform::samples_to_spectrum_steps(&raw_data, 1, "Peaks And Valleys").unwrap_or_else(|_| {
                                        fit_bezier(&raw_data, 20, 1, "Peaks And Valleys")
                                    })
                                }
                            };

                            if let Some(mut text) = captured.try_lock() {
                                if params.normalize_capture.value() {
                                    let max_y = curve.points.iter().map(|p| p.position.1).fold(0.0f32, f32::max);
                                    if max_y > 0.001 { for p in &mut curve.points { p.position.1 /= max_y; } }
                                }
                                *text = zebra_format::generate(&curve);
                            }
                            raw_data.clear();
                            capture_ready.store(false, Ordering::SeqCst);
                        }
                    }
                }

                let mut preview_cycle = Vec::new();
                let mut current_mags = Vec::new();
                let mut period = 100.0;
                let mut center = 0.0;
                let mut range = 1.0;
                
                // --- Step A: Snapshot (Local stack buffer to be ultra-safe) ---
                let mut snapshot = vec![0.0f32; 4096]; // Smaller snapshot
                if let Some(buffer) = audio_buffer_mutex.try_lock() {
                    let w_idx = write_idx_atomic.load(Ordering::Relaxed);
                    for i in 0..4096 { 
                        snapshot[4095 - i] = buffer[(w_idx + BUFFER_SIZE - 1 - i) % BUFFER_SIZE]; 
                    }
                } else {
                    // Just fallback to some UI loop to avoid block
                    egui_ctx.request_repaint();
                }
                
                let snap_len = snapshot.len();
                let sample_rate = f32::from_bits(sample_rate_atomic.load(Ordering::Relaxed));
                let midi_freq = f32::from_bits(current_freq_atomic.load(Ordering::Relaxed));
                let midi_period = (sample_rate / midi_freq).clamp(10.0, 2000.0);

                // Simple Linear Crossing for Preview (Safer)
                let mut trigger_idx = snap_len - 1;
                for i in (0..snap_len - 1).rev() {
                    if snapshot[i] <= 0.0 && snapshot[i+1] > 0.0 {
                        trigger_idx = i;
                        break;
                    }
                }
                period = midi_period;
                let trigger_pos = trigger_idx as f32;

                for i in 0..1024 {
                    let t = (i as f32 / 1023.0) * 2.0;
                    let pos = (trigger_pos + t * period) as usize;
                    let s = snapshot.get(pos % snap_len).copied().unwrap_or(0.0);
                    preview_cycle.push(s);
                }

                // Simple FFT Analysis
                use rustfft::{num_complex::Complex, FftPlanner};
                let mut planner = FftPlanner::new();
                let fft_size = 1024;
                let fft = planner.plan_fft_forward(fft_size);
                let mut fft_buf = vec![Complex::default(); fft_size];
                for i in 0..fft_size {
                    let s = snapshot.get(i).copied().unwrap_or(0.0);
                    fft_buf[i] = Complex { re: s, im: 0.0 };
                }
                fft.process(&mut fft_buf);
                current_mags = fft_buf[..fft_size/2].iter().map(|c| c.norm() * 2.0 / fft_size as f32).collect();

                if !current_mags.is_empty() {
                    if let Some(mut ema_mags) = ema_mags_mutex.try_lock() {
                        if ema_mags.len() != current_mags.len() { *ema_mags = current_mags; }
                        else { for (i, m) in current_mags.iter().enumerate() { ema_mags[i] = ema_mags[i] * 0.8 + m * 0.2; } }
                    }
                }

                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Curve Sampler");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("v{}", Self::VERSION));
                        });
                    });
                    ui.add_space(10.0);

                    ui.columns(2, |columns| {
                        columns[0].vertical_centered(|ui| {
                            ui.label("Oscilloscope");
                            let rect = ui.allocate_space(egui::vec2(ui.available_width(), 120.0)).1;
                            let painter = ui.painter_at(rect);
                            painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(200));
                            if preview_cycle.len() >= 2 {
                                let pts: Vec<egui::Pos2> = preview_cycle.iter().enumerate().map(|(i, &y)| {
                                    let x = rect.left() + (i as f32 / (preview_cycle.len() - 1) as f32) * rect.width();
                                    let py = rect.center().y - y * rect.height() * 0.45;
                                    egui::pos2(x, py)
                                }).collect();
                                painter.line(pts, (1.2, egui::Color32::YELLOW));
                            }
                            ui.add_space(5.0);
                            if ui.button("Capture Geo").clicked() {
                                if let Some(mut d) = current_capture_domain.try_lock() { *d = CurveDomain::Geometry; trigger.store(true, Ordering::SeqCst); }
                            }
                        });

                        columns[1].vertical_centered(|ui| {
                            ui.label("Spectrum");
                            let rect = ui.allocate_space(egui::vec2(ui.available_width(), 120.0)).1;
                            let painter = ui.painter_at(rect);
                            painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(200));
                            if let Some(mags) = ema_mags_mutex.try_lock() {
                                let max_mag = mags[1..].iter().cloned().fold(0.001f32, f32::max);
                                let log_freq_scale = 10.0;
                                for k in 1..1024.min(mags.len()) {
                                    let mag = (mags.get(k).copied().unwrap_or(0.0) / max_mag).min(1.0);
                                    if mag < 0.001 { continue; }
                                    let x_start = (k as f32).log2() / log_freq_scale;
                                    let x_end = ((k + 1) as f32).log2() / log_freq_scale;
                                    let px_start = rect.left() + x_start * rect.width();
                                    let px_end = rect.left() + x_end * rect.width();
                                    let py = rect.bottom() - mag * 0.95 * rect.height();
                                    painter.rect_filled(egui::Rect::from_min_max(egui::pos2(px_start, py), egui::pos2(px_end, rect.bottom())), 0.0, egui::Color32::from_rgb(0, 180, 255));
                                }
                            }
                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                if ui.button("Capture Spec").clicked() {
                                    if let Some(mut d) = current_capture_domain.try_lock() { *d = CurveDomain::Spectrum; trigger.store(true, Ordering::SeqCst); }
                                }
                                ui.add_space(10.0);
                                let mut norm = params.normalize_capture.value();
                                if ui.checkbox(&mut norm, "Normalize").clicked() {
                                    setter.begin_set_parameter(&params.normalize_capture);
                                    setter.set_parameter(&params.normalize_capture, norm);
                                    setter.end_set_parameter(&params.normalize_capture);
                                }
                            });
                        });
                    });

                    ui.add_space(20.0);
                    ui.label("Zebra 3 Clipboard String:");
                    ui.columns(2, |columns| {
                        if let Some(mut text) = captured.try_lock() {
                            egui::ScrollArea::vertical().id_salt("log_scroll").max_height(140.0).show(&mut columns[0], |ui| {
                                ui.add(egui::TextEdit::multiline(&mut *text).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
                            });
                            columns[0].add_space(10.0);
                            if columns[0].button("📋 Copy to Clipboard").clicked() { egui_ctx.copy_text(text.clone()); }
                        }
                        columns[1].vertical_centered(|ui| {
                            ui.label("Stored Curve Preview");
                            let rect = ui.allocate_space(egui::vec2(ui.available_width(), 140.0)).1;
                            let painter = ui.painter_at(rect);
                            painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(200));
                            if let Some(text) = captured.try_lock() {
                                if let Ok(curve) = curve_core::zebra_format::parse(&text) {
                                    let is_spec = text.contains("MorphType = 'Peaks And Valleys'") && curve.points.len() > 40;
                                    let mut last_pos: Option<egui::Pos2> = None;
                                    for i in 0..=256 {
                                        let t = i as f32 / 256.0;
                                        let y = curve.eval(t);
                                        let px = rect.left() + t * rect.width();
                                        let py = rect.bottom() - y * rect.height();
                                        let pos = egui::pos2(px, py);
                                        if let Some(prev) = last_pos {
                                            let color = if is_spec { egui::Color32::from_rgb(0, 180, 255) } else { egui::Color32::YELLOW };
                                            painter.line_segment([prev, pos], (1.2, color));
                                        }
                                        last_pos = Some(pos);
                                    }
                                }
                            }
                        });
                    });
                });
                egui_ctx.request_repaint();
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
        self.sample_rate.store(sample_rate.to_bits(), Ordering::Relaxed);
        while let Some(event) = context.next_event() {
            if let NoteEvent::NoteOn { note, .. } = event {
                let freq = 440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0);
                self.current_freq.store(freq.to_bits(), Ordering::Relaxed);
            }
        }
        let mut trigger = self.trigger_capture.load(Ordering::SeqCst);
        let mut w_idx = self.write_idx.load(Ordering::Relaxed);
        if let Some(mut audio_buf) = self.audio_buffer.try_lock() {
            for mut samples in buffer.iter_samples() {
                let sample = *samples.iter_mut().next().unwrap_or(&mut 0.0);
                let prev_idx = if w_idx == 0 { BUFFER_SIZE - 1 } else { w_idx - 1 };
                let prev_sample = audio_buf[prev_idx];
                audio_buf[w_idx] = sample;
                if trigger && prev_sample <= 0.0 && sample > 0.0 {
                    self.perform_extraction(sample_rate, &audio_buf, w_idx);
                    self.trigger_capture.store(false, Ordering::SeqCst);
                    trigger = false;
                }
                w_idx = (w_idx + 1) % BUFFER_SIZE;
            }
        }
        self.write_idx.store(w_idx, Ordering::Relaxed);
        ProcessStatus::Normal
    }
}

impl CurveSampler {
    fn perform_extraction(&self, sample_rate: f32, audio_buf: &[f32], write_idx: usize) {
        let freq = f32::from_bits(self.current_freq.load(Ordering::Relaxed));
        let period_samples = sample_rate / freq;
        if period_samples < 2.0 || period_samples > (BUFFER_SIZE / 4) as f32 { return; }
        let mut cycle = Vec::with_capacity(1024);
        let start_pos = (write_idx as f32 + BUFFER_SIZE as f32 - period_samples) % BUFFER_SIZE as f32;
        for i in 0..1024 {
            let t = i as f32 / 1024.0;
            let offset = t * period_samples;
            let pos = (start_pos + offset) % BUFFER_SIZE as f32;
            let sample = audio_buf[pos.floor() as usize]; // Linear/Floor for extraction speed
            cycle.push(sample);
        }
        let mut min = f32::INFINITY; let mut max = f32::NEG_INFINITY;
        for &s in &cycle { if s < min { min = s; } if s > max { max = s; } }
        let range = (max - min).max(1e-6);
        for s in &mut cycle { *s = (*s - min) / range; }
        if let Some(mut raw_data) = self.raw_cycle.try_lock() {
            *raw_data = cycle;
            self.capture_ready.store(true, Ordering::SeqCst);
        }
    }
}

impl ClapPlugin for CurveSampler {
    const CLAP_ID: &'static str = "com.alexandernutz.curve-sampler";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Extract curves from audio");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Utility];
}

impl Vst3Plugin for CurveSampler {
    const VST3_CLASS_ID: [u8; 16] = *b"CurveSampler0001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(CurveSampler);
nih_export_vst3!(CurveSampler);
