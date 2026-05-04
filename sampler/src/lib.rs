use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering, AtomicU32, AtomicUsize};
use std::fs::OpenOptions;
use std::io::Write;
use curve_core::bezier::{fit_bezier, fit_bezier_interp, fit_bezier_adaptive};
use curve_core::zebra_format;

mod theme;

pub use theme::Theme;

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

#[derive(Enum, PartialEq, Clone, Copy, Debug)]
pub enum TrackingMode {
    Auto,
    Manual,
}

#[derive(Enum, PartialEq, Clone, Copy, Debug)]
pub enum ChordPriority {
    LowestNote,
    CommonPeriod,
}

/// A handle that sets an atomic boolean to false when dropped.
/// Used to track GUI open/close state.
struct GuiHandle {
    is_open: Arc<AtomicBool>,
}

impl Drop for GuiHandle {
    fn drop(&mut self) {
        self.is_open.store(false, Ordering::Relaxed);
        log_to_file("GUI Closed - Going to sleep");
    }
}

struct CurveSampler {
    params: Arc<CurveSamplerParams>,
    
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    write_idx: Arc<AtomicUsize>,
    sample_rate: Arc<AtomicU32>,
    
    target_freq: Arc<AtomicU32>,
    active_midi_notes: Arc<Mutex<Vec<u8>>>,
    
    trigger_capture: Arc<AtomicBool>,
    captured_string: Arc<Mutex<String>>,
    
    raw_cycle: Arc<Mutex<Vec<f32>>>,
    capture_ready: Arc<AtomicBool>,
    current_capture_domain: Arc<Mutex<CurveDomain>>,

    ema_period: Arc<Mutex<f32>>,
    ema_magnitudes: Arc<Mutex<Vec<f32>>>,
    
    fft_planner: Arc<Mutex<rustfft::FftPlanner<f32>>>,

    /// Flag to tell the audio thread if the UI is active.
    gui_is_open: Arc<AtomicBool>,
}

#[derive(Params)]
struct CurveSamplerParams {
    #[persist = "editor_state"]
    pub editor_state: Arc<EguiState>,

    #[id = "theme"]
    pub theme: EnumParam<Theme>,

    #[id = "domain"]
    pub domain: EnumParam<CurveDomain>,
    
    #[id = "norm"]
    pub normalize_capture: BoolParam,

    #[id = "track_mode"]
    pub tracking_mode: EnumParam<TrackingMode>,

    #[id = "chord_pri"]
    pub chord_priority: EnumParam<ChordPriority>,

    #[id = "freq1"]
    pub manual_freq1: FloatParam,
    #[id = "freq2"]
    pub manual_freq2: FloatParam,
    #[id = "freq3"]
    pub manual_freq3: FloatParam,
}

impl Default for CurveSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(CurveSamplerParams::default()),
            audio_buffer: Arc::new(Mutex::new(vec![0.0; BUFFER_SIZE])),
            write_idx: Arc::new(AtomicUsize::new(0)),
            sample_rate: Arc::new(AtomicU32::new(48000.0f32.to_bits())),
            target_freq: Arc::new(AtomicU32::new(440.0f32.to_bits())),
            active_midi_notes: Arc::new(Mutex::new(Vec::with_capacity(16))),
            trigger_capture: Arc::new(AtomicBool::new(false)),
            captured_string: Arc::new(Mutex::new(String::from("No curve captured yet."))),
            raw_cycle: Arc::new(Mutex::new(Vec::with_capacity(1024))),
            capture_ready: Arc::new(AtomicBool::new(false)),
            current_capture_domain: Arc::new(Mutex::new(CurveDomain::Geometry)),
            ema_period: Arc::new(Mutex::new(100.0)),
            ema_magnitudes: Arc::new(Mutex::new(vec![0.0; 1024])),
            fft_planner: Arc::new(Mutex::new(rustfft::FftPlanner::new())),
            gui_is_open: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for CurveSamplerParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(700, 500),
            theme: EnumParam::new("Theme", Theme::Auto),
            domain: EnumParam::new("Domain", CurveDomain::Geometry),
            normalize_capture: BoolParam::new("Normalize", true),
            tracking_mode: EnumParam::new("Tracking", TrackingMode::Auto),
            chord_priority: EnumParam::new("Chord Priority", ChordPriority::LowestNote),
            manual_freq1: FloatParam::new("Freq 1", 440.0, FloatRange::Skewed { min: 20.0, max: 20000.0, factor: 0.2 }),
            manual_freq2: FloatParam::new("Freq 2", 0.0, FloatRange::Skewed { min: 0.0, max: 20000.0, factor: 0.2 }),
            manual_freq3: FloatParam::new("Freq 3", 0.0, FloatRange::Skewed { min: 0.0, max: 20000.0, factor: 0.2 }),
        }
    }
}

impl Plugin for CurveSampler {
    const NAME: &'static str = "Curve Sampler";
    const VENDOR: &'static str = "Curve Transform Project";
    const URL: &'static str = "https://github.com/alexandernutz/svg-osc_gem";
    const EMAIL: &'static str = "info@example.com";
    const VERSION: &'static str = "0.1.54";

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
        log_to_file("GUI Opened - Waking up");
        self.gui_is_open.store(true, Ordering::Relaxed);
        
        let trigger = self.trigger_capture.clone();
        let captured = self.captured_string.clone();
        let raw_cycle = self.raw_cycle.clone();
        let capture_ready = self.capture_ready.clone();
        let current_capture_domain = self.current_capture_domain.clone();
        let params = self.params.clone();
        
        let audio_buffer_mutex = self.audio_buffer.clone();
        let write_idx_atomic = self.write_idx.clone();
        let target_freq_atomic = self.target_freq.clone();
        let sample_rate_atomic = self.sample_rate.clone();
        let active_midi_notes = self.active_midi_notes.clone();

        let ema_period_mutex = self.ema_period.clone();
        let ema_mags_mutex = self.ema_magnitudes.clone();
        let fft_planner_mutex = self.fft_planner.clone();

        // Create the drop handle for lifecycle tracking
        let gui_handle = GuiHandle { is_open: self.gui_is_open.clone() };

        create_egui_editor(
            self.params.editor_state.clone(),
            gui_handle, // Pass it into the editor's user state
            |_ctx, _user_state| {},
            move |egui_ctx, setter, _gui_handle| {
                if capture_ready.load(Ordering::SeqCst) {
                    if let (Some(mut raw_data), Some(domain)) = (raw_cycle.try_lock(), current_capture_domain.try_lock()) {
                        if !raw_data.is_empty() {
                            let mut curve = match *domain {
                                CurveDomain::Geometry => {
                                    let mut c = fit_bezier_adaptive(&raw_data, 128, 1, "Peaks And Valleys");
                                    c.simplify(0.001);
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
                let mut period;
                let mut center = 0.0;
                let mut range = 1.0;
                
                let mut snapshot = vec![0.0f32; 8192];
                let mut buffer_locked = false;
                if let Some(buffer) = audio_buffer_mutex.try_lock() {
                    let w_idx = write_idx_atomic.load(Ordering::Relaxed);
                    for i in 0..8192 { snapshot[8191 - i] = buffer[(w_idx + BUFFER_SIZE - 1 - i) % BUFFER_SIZE]; }
                    buffer_locked = true;
                }
                
                let snap_len = snapshot.len();
                let sample_rate = f32::from_bits(sample_rate_atomic.load(Ordering::Relaxed));
                
                let mut base_freq = 440.0;
                let mut target_reason = "Manual / Single note".to_string();
                if buffer_locked {
                    let mut active_freqs = Vec::new();
                    if params.tracking_mode.value() == TrackingMode::Auto {
                        if let Some(notes) = active_midi_notes.try_lock() {
                            for &note in notes.iter() {
                                active_freqs.push(440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0));
                            }
                        }
                        if !active_freqs.is_empty() { target_reason = "MIDI input".to_string(); }
                    } else {
                        for &f in &[params.manual_freq1.value(), params.manual_freq2.value(), params.manual_freq3.value()] {
                            if f > 0.1 { active_freqs.push(f); }
                        }
                        active_freqs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        if !active_freqs.is_empty() { target_reason = "Manual frequency".to_string(); }
                    }

                    if !active_freqs.is_empty() {
                        base_freq = active_freqs[0];
                        if active_freqs.len() >= 2 {
                            if params.chord_priority.value() == ChordPriority::LowestNote {
                                target_reason = format!("Lowest note of {} active freqs", active_freqs.len());
                            } else {
                                let f1 = active_freqs[0];
                                let f2 = active_freqs[1];
                                let ratio = f2 / f1;
                                
                                let mut matched = true;
                                // Check common musical ratios for a shared period
                                if (ratio - 1.5).abs() < 0.02 { 
                                    base_freq = f1 / 2.0; 
                                    target_reason = "Common period (LCM of 3:2 fifth)".to_string();
                                } else if (ratio - 2.0).abs() < 0.02 { 
                                    base_freq = f1; 
                                    target_reason = "Common period (Octave)".to_string();
                                } else if (ratio - 1.333).abs() < 0.02 { 
                                    base_freq = f1 / 3.0; 
                                    target_reason = "Common period (LCM of 4:3 fourth)".to_string();
                                } else if (ratio - 1.25).abs() < 0.02 { 
                                    base_freq = f1 / 4.0; 
                                    target_reason = "Common period (LCM of 5:4 major third)".to_string();
                                } else if (ratio - 1.2).abs() < 0.02 { 
                                    base_freq = f1 / 5.0; 
                                    target_reason = "Common period (LCM of 6:5 minor third)".to_string();
                                } else {
                                    matched = false;
                                    target_reason = "Lowest note (fallback, complex ratio)".to_string();
                                }
                                
                                if matched && active_freqs.len() >= 3 {
                                    let f3 = active_freqs[2];
                                    let ratio3 = f3 / base_freq;
                                    let r_round = ratio3.round();
                                    if (ratio3 - r_round).abs() > 0.05 {
                                        // target_reason remains the 2-note LCM or fallback
                                    } else {
                                        target_reason = format!("Common period (LCM of 3-note chord)");
                                    }
                                }
                            }
                        }
                    }

                    target_freq_atomic.store(base_freq.to_bits(), Ordering::Relaxed);
                    
                    let target_period = (sample_rate / base_freq).clamp(8.0, 4000.0);

                    let mut filtered = vec![0.0f32; snap_len];
                    let mut lp = 0.0f32;
                    for i in 0..snap_len { lp = lp * 0.8 + snapshot[i] * 0.2; filtered[i] = lp; }

                    let analysis_limit = (target_period * 8.0) as usize;
                    let start_idx = snap_len.saturating_sub(analysis_limit);
                    let mut s_min = 0.0f32; let mut s_max = 0.0f32;
                    for i in start_idx..snap_len { let val = filtered[i]; if val < s_min { s_min = val; } if val > s_max { s_max = val; } }
                    center = (s_min + s_max) * 0.5;
                    range = (s_max - s_min).max(0.01);
                    let threshold = range * 0.15;

                    let mut crossings = Vec::new();
                    let mut armed = false;
                    for i in (start_idx..snap_len - 1).rev() {
                        let s0 = filtered[i]; let s1 = filtered[i + 1];
                        if s0 < center - threshold { armed = true; }
                        if armed && s0 <= center && s1 > center {
                            let frac = (center - s0) / (s1 - s0).max(1e-6);
                            crossings.push(i as f32 + frac);
                            armed = false;
                            if crossings.len() >= 3 { break; }
                        }
                    }

                    let mut trigger_pos = (snap_len - 1) as f32;
                    if crossings.len() >= 2 {
                        let p = crossings[0] - crossings[1];
                        if (p - target_period).abs() < target_period * 0.4 || (p - target_period*0.5).abs() < p*0.1 || (p - target_period*2.0).abs() < p*0.1 {
                            period = p;
                        } else { period = target_period; }
                        trigger_pos = crossings[0] - period; 
                    } else { period = target_period; }

                    if let Some(mut ema_p) = ema_period_mutex.try_lock() { *ema_p = *ema_p * 0.95 + period * 0.05; }
                    period = *ema_period_mutex.lock();

                    let get_snapshot_cubic = |pos: f32, snap: &[f32]| -> f32 {
                        let snap_len = snap.len();
                        if snap_len < 4 { return 0.0; }
                        let p = pos.clamp(2.0, (snap_len - 3) as f32);
                        let i1 = p.floor() as usize; let i2 = i1 + 1; let i0 = i1 - 1; let i3 = i1 + 2;
                        let t = p - i1 as f32;
                        let y0 = snap[i0]; let y1 = snap[i1]; let y2 = snap[i2]; let y3 = snap[i3];
                        let a = -0.5 * y0 + 1.5 * y1 - 1.5 * y2 + 0.5 * y3;
                        let b = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
                        let c = -0.5 * y0 + 0.5 * y2;
                        let d = y1;
                        a * t * t * t + b * t * t + c * t + d
                    };

                    for i in 0..1024 {
                        let t = (i as f32 / 1023.0) * 2.0;
                        let pos = trigger_pos + t * period;
                        preview_cycle.push(get_snapshot_cubic(pos, &snapshot));
                    }
                    
                    if let Some(mut planner) = fft_planner_mutex.try_lock() {
                        use rustfft::num_complex::Complex;
                        let fft_size = 2048;
                        let fft = planner.plan_fft_forward(fft_size);
                        let mut fft_buf = vec![Complex::default(); fft_size];
                        let mut sum = 0.0f32;
                        for i in 0..fft_size {
                            let t = i as f32 / fft_size as f32;
                            let pos = trigger_pos + t * period;
                            sum += get_snapshot_cubic(pos, &snapshot);
                        }
                        let mean_val = sum / fft_size as f32;
                        for i in 0..fft_size {
                            let t = i as f32 / fft_size as f32;
                            let pos = trigger_pos + t * period;
                            let s = get_snapshot_cubic(pos, &snapshot);
                            let t_win = i as f32 / (fft_size - 1) as f32;
                            let a0 = 0.35875; let a1 = 0.48829; let a2 = 0.14128; let a3 = 0.01168;
                            let window = a0 - a1 * (2.0 * std::f32::consts::PI * t_win).cos() + a2 * (4.0 * std::f32::consts::PI * t_win).cos() - a3 * (6.0 * std::f32::consts::PI * t_win).cos();
                            fft_buf[i] = Complex { re: (s - mean_val) * window, im: 0.0 };
                        }
                        fft.process(&mut fft_buf);
                        current_mags = fft_buf[..fft_size/2].iter().map(|c| c.norm() * 12.8 / fft_size as f32).collect();
                    }
                } else {
                    period = *ema_period_mutex.lock();
                }

                if !current_mags.is_empty() {
                    if let Some(mut ema_mags) = ema_mags_mutex.try_lock() {
                        if ema_mags.len() != current_mags.len() { *ema_mags = current_mags; }
                        else { for (i, m) in current_mags.iter().enumerate() { ema_mags[i] = ema_mags[i] * 0.85 + m * 0.15; } }
                    }
                }

                theme::apply(egui_ctx, params.theme.value());

                egui::CentralPanel::default().show(egui_ctx, |ui| {

                    ui.horizontal(|ui| {
                        ui.heading("Curve Sampler");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("v{}", Self::VERSION));
                        });
                    });
                    ui.add_space(12.0);
                    
                    // --- Tracking Panel ---
                    let mut current_mode = params.tracking_mode.value();
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Tracking:");
                                if ui.radio_value(&mut current_mode, TrackingMode::Auto, "Auto (MIDI)")
                                    .on_hover_text("Sets capture length based on incoming MIDI notes.")
                                    .clicked() 
                                {
                                    setter.begin_set_parameter(&params.tracking_mode);
                                    setter.set_parameter(&params.tracking_mode, current_mode);
                                    setter.end_set_parameter(&params.tracking_mode);
                                }

                                if ui.radio_value(&mut current_mode, TrackingMode::Manual, "Manual")
                                    .on_hover_text("Sets capture length based on the manual frequency boxes below. 0Hz means 'ignore'.")
                                    .clicked() 
                                {
                                    setter.begin_set_parameter(&params.tracking_mode);
                                    setter.set_parameter(&params.tracking_mode, current_mode);
                                    setter.end_set_parameter(&params.tracking_mode);
                                }
                            });

                            ui.horizontal(|ui| {
                                if current_mode == TrackingMode::Auto {
                                    ui.label("Chord:");
                                    let mut pri = params.chord_priority.value();
                                    if ui.radio_value(&mut pri, ChordPriority::LowestNote, "Lowest")
                                        .on_hover_text("Captures based on the lowest incoming MIDI note.")
                                        .clicked() 
                                    {
                                        setter.begin_set_parameter(&params.chord_priority);
                                        setter.set_parameter(&params.chord_priority, pri);
                                        setter.end_set_parameter(&params.chord_priority);
                                    }
                                    if ui.radio_value(&mut pri, ChordPriority::CommonPeriod, "Common")
                                        .on_hover_text("Captures the full chord cycle by finding a shared period (LCM). Note: The resulting curve will have a lower root pitch than the individual notes.")
                                        .clicked() 
                                    {
                                        setter.begin_set_parameter(&params.chord_priority);
                                        setter.set_parameter(&params.chord_priority, pri);
                                        setter.end_set_parameter(&params.chord_priority);
                                    }
                                } else {
                                    ui.label("Freqs:");
                                    for p in &[&params.manual_freq1, &params.manual_freq2, &params.manual_freq3] {
                                        let mut val = p.value();
                                        if ui.add(egui::DragValue::new(&mut val).suffix(" Hz").speed(1.0)).changed() {
                                            setter.begin_set_parameter(*p);
                                            setter.set_parameter(*p, val);
                                            setter.end_set_parameter(*p);
                                        }
                                    }
                                }
                            });

                            let display_freq = f32::from_bits(target_freq_atomic.load(Ordering::Relaxed));
                            ui.label(format!("Active Target: {:.2} Hz", display_freq)).on_hover_text(target_reason);
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            ui.horizontal(|ui| {
                                let mut current_theme = params.theme.value();
                                if ui.radio_value(&mut current_theme, Theme::Light, "Light").clicked() {
                                    setter.begin_set_parameter(&params.theme);
                                    setter.set_parameter(&params.theme, current_theme);
                                    setter.end_set_parameter(&params.theme);
                                }
                                if ui.radio_value(&mut current_theme, Theme::Dark, "Dark").clicked() {
                                    setter.begin_set_parameter(&params.theme);
                                    setter.set_parameter(&params.theme, current_theme);
                                    setter.end_set_parameter(&params.theme);
                                }
                                if ui.radio_value(&mut current_theme, Theme::Auto, "Auto").clicked() {
                                    setter.begin_set_parameter(&params.theme);
                                    setter.set_parameter(&params.theme, current_theme);
                                    setter.end_set_parameter(&params.theme);
                                }
                                ui.label("Theme:");
                            });
                        });
                    });


                    ui.add_space(14.0);

                    ui.columns(2, |columns| {
                        columns[0].vertical(|ui| {
                            ui.label("Oscilloscope (Locked)");
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
                                if let Some(mut d) = current_capture_domain.try_lock() { *d = CurveDomain::Geometry; trigger.store(true, Ordering::SeqCst); }
                            }
                        });

                        columns[1].vertical(|ui| {
                            ui.label("Spectrum (Stable dB)");
                            let rect = ui.allocate_space(egui::vec2(ui.available_width(), 120.0)).1;
                            let painter = ui.painter_at(rect);
                            painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(200));
                            painter.line_segment([egui::pos2(rect.left(), rect.top() + 5.0), egui::pos2(rect.right(), rect.top() + 5.0)], (1.0, egui::Color32::from_gray(60)));

                            if let Some(mags) = ema_mags_mutex.try_lock() {
                                let log_freq_scale = 10.0;
                                let db_range = 60.0; 
                                for k in 1..1024.min(mags.len()) {
                                    let linear_mag = mags.get(k).copied().unwrap_or(0.0);
                                    if linear_mag < 1e-6 { continue; }
                                    let db = 20.0 * linear_mag.log10();
                                    let norm_db = ((db + db_range) / db_range).clamp(0.0, 1.0);
                                    if norm_db < 0.01 { continue; }
                                    let x_start = (k as f32).log2() / log_freq_scale;
                                    let x_end = ((k + 1) as f32).log2() / log_freq_scale;
                                    let px_start = rect.left() + x_start * rect.width();
                                    let px_end = rect.left() + x_end * rect.width();
                                    let py = rect.bottom() - norm_db * rect.height();
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

                    ui.add_space(10.0);
                    ui.columns(2, |columns| {
                        columns[0].label("Curve Clipboard String:");
                        columns[1].label("Stored Curve Preview:");
                    });

                    ui.columns(2, |columns| {
                        if let Some(mut text) = captured.try_lock() {
                            // Column 0: Multiline text edit
                            egui::ScrollArea::vertical()
                                .id_salt("log_scroll")
                                .max_height(140.0)
                                .min_scrolled_height(140.0) // Force alignment
                                .show(&mut columns[0], |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut *text)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                    );
                                });
                            columns[0].add_space(10.0);
                            if columns[0].button("📋 Copy to Clipboard").clicked() { 
                                egui_ctx.copy_text(text.clone()); 
                            }
                        } else {
                            // Empty state to keep layout stable
                            columns[0].allocate_space(egui::vec2(columns[0].available_width(), 140.0));
                            columns[0].add_space(10.0);
                            columns[0].add_enabled(false, egui::Button::new("📋 Copy to Clipboard"));
                        }

                        // Column 1: Curve Preview
                        let rect = columns[1].allocate_space(egui::vec2(columns[1].available_width(), 140.0)).1;
                        let painter = columns[1].painter_at(rect);
                        painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(200));
                        if let Some(text) = captured.try_lock() {
                            if let Ok(curve) = curve_core::zebra_format::parse(&text) {
                                let is_spec = text.contains("MorphType = 'Peaks And Valleys'") && curve.points.len() > 40;
                                let mut last_pos: Option<egui::Pos2> = None;
                                let preview_steps = 512;
                                for i in 0..=preview_steps {
                                    let t = i as f32 / preview_steps as f32;
                                    let y = curve.eval(t);
                                    let px = rect.left() + t * rect.width();
                                    let py = rect.bottom() - y * rect.height();
                                    let pos = egui::pos2(px, py);
                                    if let Some(prev) = last_pos {
                                        painter.line_segment([prev, pos], (1.2, columns[1].visuals().text_color()));
                                    }

                                    last_pos = Some(pos);
                                }
                            } else {
                                painter.text(rect.center(), egui::Align2::CENTER_CENTER, "[No Data]", egui::FontId::proportional(14.0), egui::Color32::GRAY);
                            }
                        }
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
            match event {
                NoteEvent::NoteOn { note, .. } => {
                    if let Some(mut notes) = self.active_midi_notes.try_lock() {
                        notes.push(note);
                        notes.sort();
                    }
                }
                NoteEvent::NoteOff { note, .. } => {
                    if let Some(mut notes) = self.active_midi_notes.try_lock() {
                        notes.retain(|&n| n != note);
                    }
                }
                _ => (),
            }
        }
        
        let mut trigger = self.trigger_capture.load(Ordering::SeqCst);
        let mut w_idx = self.write_idx.load(Ordering::Relaxed);
        
        // --- Sleep Mode Logic ---
        // We only buffer samples if the GUI is open OR a capture was triggered.
        if self.gui_is_open.load(Ordering::Relaxed) || trigger {
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
        }
        
        self.write_idx.store(w_idx, Ordering::Relaxed);
        ProcessStatus::Normal
    }
}

impl CurveSampler {
    fn perform_extraction(&self, sample_rate: f32, audio_buf: &[f32], write_idx: usize) {
        let freq = f32::from_bits(self.target_freq.load(Ordering::Relaxed));
        let period_samples = sample_rate / freq;
        if period_samples < 2.0 || period_samples > (BUFFER_SIZE / 4) as f32 { return; }

        let mut cycle = Vec::with_capacity(1024);
        let start_pos = (write_idx as f32 + BUFFER_SIZE as f32 - period_samples) % BUFFER_SIZE as f32;
        for i in 0..1024 {
            let t = i as f32 / 1024.0;
            let offset = t * period_samples;
            let pos = (start_pos + offset) % BUFFER_SIZE as f32;
            let sample = audio_buf[pos.floor() as usize];
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
