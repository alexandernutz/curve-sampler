use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering, AtomicU32, AtomicUsize};
use std::fs::OpenOptions;
use std::io::Write;
use curve_core::bezier::fit_bezier;
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

#[derive(Enum, PartialEq, Clone, Copy)]
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
}

impl Default for CurveSampler {
    fn default() -> Self {
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
            ema_magnitudes: Arc::new(Mutex::new(vec![0.0; 256])),
        }
    }
}

impl Default for CurveSamplerParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(700, 550),
            domain: EnumParam::new("Domain", CurveDomain::Geometry),
        }
    }
}

impl Plugin for CurveSampler {
    const NAME: &'static str = "Curve Sampler";
    const VENDOR: &'static str = "Curve Transform Project";
    const URL: &'static str = "https://github.com/alexandernutz/svg-osc_gem";
    const EMAIL: &'static str = "info@example.com";
    const VERSION: &'static str = "0.1.7";

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
        let trigger = self.trigger_capture.clone();
        let captured = self.captured_string.clone();
        let raw_cycle = self.raw_cycle.clone();
        let capture_ready = self.capture_ready.clone();
        let current_capture_domain = self.current_capture_domain.clone();
        
        let audio_buffer_mutex = self.audio_buffer.clone();
        let write_idx_atomic = self.write_idx.clone();
        let current_freq_atomic = self.current_freq.clone();
        let sample_rate_atomic = self.sample_rate.clone();

        let ema_period_mutex = self.ema_period.clone();
        let ema_mags_mutex = self.ema_magnitudes.clone();

        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_ctx, _user_state| {},
            move |egui_ctx, _setter, _user_state| {
                // 1. Process new captures
                if capture_ready.load(Ordering::SeqCst) {
                    if let (Some(mut raw_data), Some(domain)) = (raw_cycle.try_lock(), current_capture_domain.try_lock()) {
                        if !raw_data.is_empty() {
                            let mut curve = fit_bezier(&raw_data, 20, 1, "Peaks And Valleys");
                            curve.simplify(0.005);
                            
                            if let Some(mut text) = captured.try_lock() {
                                match *domain {
                                    CurveDomain::Geometry => {
                                        *text = zebra_format::generate(&curve);
                                    }
                                    CurveDomain::Spectrum => {
                                        let spec_curve = curve_core::transform::geometry_to_spectrum_steps(&curve, 2048)
                                            .unwrap_or(curve);
                                        *text = zebra_format::generate(&spec_curve);
                                    }
                                }
                            }
                            raw_data.clear();
                            capture_ready.store(false, Ordering::SeqCst);
                        }
                    }
                }

                // 2. Stable Waveform Analysis
                let mut preview_cycle = Vec::new();
                let mut current_mags = Vec::new();
                let mut period = 100.0;
                
                // --- Step A: Snapshot a chunk of the buffer to avoid tearing ---
                let mut snapshot = vec![0.0f32; 8192];
                let mut local_w_idx = 0;
                if let Some(buffer) = audio_buffer_mutex.try_lock() {
                    local_w_idx = write_idx_atomic.load(Ordering::Relaxed);
                    // Copy the last 8192 samples
                    for i in 0..8192 {
                        snapshot[8191 - i] = buffer[(local_w_idx + BUFFER_SIZE - 1 - i) % BUFFER_SIZE];
                    }
                }
                
                // Snapshot now contains data where index 8191 is the newest sample.
                let snap_len = snapshot.len();
                let sample_rate = f32::from_bits(sample_rate_atomic.load(Ordering::Relaxed));
                let midi_freq = f32::from_bits(current_freq_atomic.load(Ordering::Relaxed));
                let midi_period = sample_rate / midi_freq;

                // --- Step B: LPF Filtered analysis for stable triggering ---
                let mut filtered = vec![0.0f32; snap_len];
                let mut lp = 0.0f32;
                for i in 0..snap_len {
                    lp = lp * 0.8 + snapshot[i] * 0.2;
                    filtered[i] = lp;
                }

                // --- Step C: Find signal range for auto-thresholding on filtered data ---
                let analysis_limit = (midi_period * 8.0) as usize;
                let start_idx = snap_len.saturating_sub(analysis_limit);
                let mut s_min = 0.0f32;
                let mut s_max = 0.0f32;
                for i in start_idx..snap_len {
                    let val = filtered[i];
                    if val < s_min { s_min = val; }
                    if val > s_max { s_max = val; }
                }
                let center = (s_min + s_max) * 0.5;
                let range = (s_max - s_min).max(0.01);
                let threshold = range * 0.15; // 15% hysteresis

                // --- Step D: Find stable crossings ---
                let mut crossings = Vec::new();
                let mut armed = false;
                for i in (start_idx..snap_len - 1).rev() {
                    let s0 = filtered[i];
                    let s1 = filtered[i + 1];
                    
                    if s0 < center - threshold { armed = true; }
                    if armed && s0 <= center && s1 > center {
                        // Fractional crossing in the filtered signal
                        let frac = (center - s0) / (s1 - s0).max(1e-6);
                        let pos = i as f32 + frac;
                        crossings.push(pos);
                        armed = false;
                        if crossings.len() >= 3 { break; }
                    }
                }

                let mut trigger_pos = (snap_len - 1) as f32;
                if crossings.len() >= 2 {
                    let t0 = crossings[0];
                    let t1 = crossings[1];
                    let p = t0 - t1;
                    
                    // Logic: prefer the measured period, but fallback to MIDI if it's crazy
                    if (p - midi_period).abs() < midi_period * 0.4 || (p - midi_period*0.5).abs() < p*0.1 || (p - midi_period*2.0).abs() < p*0.1 {
                        period = p;
                    } else {
                        period = midi_period;
                    }
                    trigger_pos = t1; // Use the older crossing to ensure we have enough "future" data in the snapshot
                } else {
                    period = midi_period;
                }

                // Smooth period
                if let Some(mut ema_p) = ema_period_mutex.try_lock() {
                    *ema_p = *ema_p * 0.95 + period * 0.05;
                }
                period = *ema_period_mutex.lock();

                // --- Step E: Resample Previews from RAW snapshot ---
                for i in 0..1024 {
                    let t = (i as f32 / 1023.0) * 2.0;
                    let pos = trigger_pos + t * period;
                    let i1 = pos.floor() as usize;
                    let i2 = (i1 + 1).min(snap_len - 1);
                    let frac = pos - i1 as f32;
                    let s = if i1 < snap_len {
                        snapshot[i1] * (1.0 - frac) + snapshot[i2] * frac
                    } else {
                        0.0
                    };
                    preview_cycle.push(s);
                }
                
                // FFT from RAW snapshot (one cycle)
                use rustfft::{num_complex::Complex, FftPlanner};
                let fft_size = 1024;
                let mut planner = FftPlanner::new();
                let fft = planner.plan_fft_forward(fft_size);
                let mut fft_buf = vec![Complex::default(); fft_size];
                for i in 0..fft_size {
                    let t = i as f32 / fft_size as f32;
                    let pos = trigger_pos + t * period;
                    let i1 = pos.floor() as usize;
                    let i2 = (i1 + 1).min(snap_len - 1);
                    let frac = pos - i1 as f32;
                    let s = if i1 < snap_len {
                        snapshot[i1] * (1.0 - frac) + snapshot[i2] * frac
                    } else {
                        0.0
                    };
                    fft_buf[i] = Complex { re: s - center, im: 0.0 };
                }
                fft.process(&mut fft_buf);
                current_mags = fft_buf[..fft_size/2].iter().map(|c| c.norm()).collect();

                // 3. Smooth spectrum
                if !current_mags.is_empty() {
                    if let Some(mut ema_mags) = ema_mags_mutex.try_lock() {
                        if ema_mags.len() != current_mags.len() {
                            *ema_mags = current_mags;
                        } else {
                            for (i, m) in current_mags.iter().enumerate() {
                                ema_mags[i] = ema_mags[i] * 0.85 + m * 0.15;
                            }
                        }
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
                            ui.label("Oscilloscope (LPF Trigger)");
                            let rect = ui.allocate_space(egui::vec2(ui.available_width(), 120.0)).1;
                            let painter = ui.painter_at(rect);
                            painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(200));
                            
                            if preview_cycle.len() >= 2 {
                                let pts: Vec<egui::Pos2> = preview_cycle.iter().enumerate().map(|(i, &y)| {
                                    let x = rect.left() + (i as f32 / (preview_cycle.len() - 1) as f32) * rect.width();
                                    let py = rect.center().y - (y - center) / range * rect.height() * 0.45;
                                    egui::pos2(x, py)
                                }).collect();
                                painter.line(pts, (1.2, egui::Color32::YELLOW));
                            }
                            
                            ui.add_space(5.0);
                            if ui.button("Capture Geo").clicked() {
                                if let Some(mut d) = current_capture_domain.try_lock() {
                                    *d = CurveDomain::Geometry;
                                    trigger.store(true, Ordering::SeqCst);
                                }
                            }
                        });

                        columns[1].vertical_centered(|ui| {
                            ui.label("Spectrum (Stable)");
                            let rect = ui.allocate_space(egui::vec2(ui.available_width(), 120.0)).1;
                            let painter = ui.painter_at(rect);
                            painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(200));
                            
                            if let Some(mags) = ema_mags_mutex.try_lock() {
                                let max_mag = mags[1..128.min(mags.len())].iter().cloned().fold(0.001f32, f32::max);
                                let log_freq_scale = 10.0;
                                for k in 1..128 {
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
                            if ui.button("Capture Spec").clicked() {
                                if let Some(mut d) = current_capture_domain.try_lock() {
                                    *d = CurveDomain::Spectrum;
                                    trigger.store(true, Ordering::SeqCst);
                                }
                            }
                        });
                    });

                    ui.add_space(20.0);
                    ui.label("Zebra 3 Clipboard String:");
                    if let Some(mut text) = captured.try_lock() {
                        egui::ScrollArea::vertical().id_salt("log_scroll").max_height(120.0).show(ui, |ui| {
                            ui.add(egui::TextEdit::multiline(&mut *text).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
                        });
                        ui.add_space(10.0);
                        if ui.button("📋 Copy to Clipboard").clicked() { egui_ctx.copy_text(text.clone()); }
                    }
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
        if period_samples < 2.0 || period_samples > (BUFFER_SIZE / 4) as f32 {
            return;
        }

        let mut cycle = Vec::with_capacity(1024);
        let start_pos = (write_idx as f32 + BUFFER_SIZE as f32 - period_samples) % BUFFER_SIZE as f32;
        
        for i in 0..1024 {
            let t = i as f32 / 1024.0;
            let offset = t * period_samples;
            let pos = (start_pos + offset) % BUFFER_SIZE as f32;
            let sample = self.get_cubic_sample_at(audio_buf, pos);
            cycle.push(sample);
        }

        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for &s in &cycle {
            if s < min { min = s; }
            if s > max { max = s; }
        }
        let range = (max - min).max(1e-6);
        for s in &mut cycle {
            *s = (*s - min) / range;
        }

        if let Some(mut raw_data) = self.raw_cycle.try_lock() {
            *raw_data = cycle;
            self.capture_ready.store(true, Ordering::SeqCst);
        }
    }

    fn get_cubic_sample_at(&self, audio_buf: &[f32], pos: f32) -> f32 {
        let i1 = pos.floor() as usize;
        let i0 = if i1 == 0 { BUFFER_SIZE - 1 } else { i1 - 1 };
        let i2 = (i1 + 1) % BUFFER_SIZE;
        let i3 = (i1 + 2) % BUFFER_SIZE;
        let t = pos - i1 as f32;
        let y0 = audio_buf[i0];
        let y1 = audio_buf[i1];
        let y2 = audio_buf[i2];
        let y3 = audio_buf[i3];
        let a = -0.5 * y0 + 1.5 * y1 - 1.5 * y2 + 0.5 * y3;
        let b = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
        let c = -0.5 * y0 + 0.5 * y2;
        let d = y1;
        a * t * t * t + b * t * t + c * t + d
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
