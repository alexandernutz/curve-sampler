use rustfft::{num_complex::Complex, FftPlanner};

use crate::bezier::{fit_bezier, fit_bezier_interp, fit_bezier_adaptive, BezierCurve, ControlPoint};

/// Segments for spectrum output — one per octave band, semantically grounded.
const SPEC_FIT_SEGMENTS: usize = 10;

/// The X axis covers harmonics 1 (x=0) through 2^10 = 1024 (x=1).
const LOG_FREQ_SCALE: f32 = 10.0;

/// Transform a piecewise cubic Bézier waveform (Geometry domain) into a 
/// stepped spectral envelope (Spectrum domain).
pub fn geometry_to_spectrum_steps(curve: &BezierCurve, fft_size: usize) -> Result<BezierCurve, String> {
    let fft_size = fft_size.max(2048);
    let samples = curve.sample(fft_size);
    let mut buf: Vec<Complex<f32>> = samples
        .iter()
        .map(|&y| Complex { re: y - 0.5, im: 0.0 })
        .collect();
    FftPlanner::new().plan_fft_forward(fft_size).process(&mut buf);

    let num_harmonics = fft_size / 2;
    let magnitudes: Vec<f32> = buf[..num_harmonics]
        .iter()
        .map(|c| c.norm() * 4.0 / fft_size as f32)
        .collect();
    
    let bin_ratio = fft_size as f32 / 1024.0;
    let mut refined_mags = vec![0.0f32; 1025];
    let db_range = 60.0f32;

    for k in 1..=1024 {
        let center_bin = (k as f32 * bin_ratio) as usize;
        let mut peak = 0.0f32;
        for b in (center_bin.saturating_sub(2))..(center_bin + 3).min(num_harmonics) {
            peak = peak.max(magnitudes[b]);
        }
        if peak > 1e-6 {
            let db = 20.0 * peak.log10();
            refined_mags[k] = ((db + db_range) / db_range).clamp(0.0, 1.0);
        } else {
            refined_mags[k] = 0.0;
        }
    }

    let mut points = Vec::with_capacity(1024);
    let xpos = |k: usize| (k as f32).log2() / LOG_FREQ_SCALE;
    for k in 1..=1024 {
        if refined_mags[k] > 0.001 || k == 1 {
            points.push(ControlPoint {
                position: (xpos(k), refined_mags[k]),
                incoming: Some((2.0 / 3.0, 2.0 / 3.0)), 
                outgoing: Some((1.0 / 3.0, 1.0 / 3.0)) 
            });
        }
    }
    
    if let Some(last) = points.last() {
        if last.position.0 < 1.0 {
            points.push(ControlPoint { position: (1.0, 0.0), incoming: Some((2.0 / 3.0, 2.0 / 3.0)), outgoing: None });
        }
    }

    let mut new_curve = BezierCurve { points, curve_id: curve.curve_id, morph_type: curve.morph_type.clone() };
    new_curve.simplify_to_budget(100);
    Ok(new_curve)
}

/// Directly convert raw audio samples (one cycle) to a stepped spectral BezierCurve.
pub fn samples_to_spectrum_steps(
    samples: &[f32], 
    curve_id: u32, 
    morph_type: &str
) -> Result<BezierCurve, String> {
    let n = samples.len();
    let fft_size = (n * 2).next_power_of_two().max(4096);
    let mean = samples.iter().sum::<f32>() / n as f32;
    
    let mut buf: Vec<Complex<f32>> = samples
        .iter()
        .map(|&y| Complex { re: y - mean, im: 0.0 })
        .collect();
    
    buf.resize(fft_size, Complex::default());
    FftPlanner::new().plan_fft_forward(fft_size).process(&mut buf);

    let num_harmonics = fft_size / 2;
    let magnitudes: Vec<f32> = buf[..num_harmonics]
        .iter()
        .map(|c| c.norm() * 4.0 / n as f32)
        .collect();

    let bin_ratio = fft_size as f32 / n as f32;
    let mut refined_mags = vec![0.0f32; 1025];
    let db_range = 60.0f32;

    for k in 1..=1024 {
        let center_bin = (k as f32 * bin_ratio) as usize;
        let mut peak = 0.0f32;
        for b in (center_bin.saturating_sub(2))..(center_bin + 3).min(num_harmonics) {
            peak = peak.max(magnitudes[b]);
        }
        if peak > 1e-6 {
            let db = 20.0 * peak.log10();
            refined_mags[k] = ((db + db_range) / db_range).clamp(0.0, 1.0);
        } else {
            refined_mags[k] = 0.0;
        }
    }

    let mut points = Vec::with_capacity(1024);
    let xpos = |k: usize| (k as f32).log2() / LOG_FREQ_SCALE;
    for k in 1..=1024 {
        if refined_mags[k] > 0.001 || k == 1 {
            points.push(ControlPoint {
                position: (xpos(k), refined_mags[k]),
                incoming: Some((2.0 / 3.0, 2.0 / 3.0)), 
                outgoing: Some((1.0 / 3.0, 1.0 / 3.0)) 
            });
        }
    }
    
    if let Some(last) = points.last() {
        if last.position.0 < 1.0 {
            points.push(ControlPoint { position: (1.0, 0.0), incoming: Some((2.0 / 3.0, 2.0 / 3.0)), outgoing: None });
        }
    }

    let mut curve = BezierCurve { points, curve_id, morph_type: morph_type.to_string() };
    curve.simplify_to_budget(100);
    Ok(curve)
}

/// Map normalized harmonic amplitudes to a dense evenly-spaced array over x ∈ [0,1].
fn log_dense(normalized: &[f32]) -> Vec<f32> {
    const DENSE: usize = 1024;
    (0..DENSE)
        .map(|i| {
            let x = i as f32 / (DENSE - 1) as f32;
            let k = (2.0f32.powf(x * LOG_FREQ_SCALE) as usize).max(1).min(normalized.len() - 1);
            normalized[k]
        })
        .collect()
}

/// Frequency domain → time domain (zero-phase reconstruction).
pub fn spectrum_to_geometry(curve: &BezierCurve, fft_size: usize) -> Result<BezierCurve, String> {
    let num_harmonics = fft_size / 2;
    let harmonic_amps = sample_log_freq(curve, num_harmonics);

    let mut buf = vec![Complex::default(); fft_size];
    for (k, &amp) in harmonic_amps.iter().enumerate() {
        if k == 0 { continue; }
        buf[k] = Complex { re: amp, im: 0.0 };
        buf[fft_size - k] = Complex { re: amp, im: 0.0 };
    }

    let mut planner = FftPlanner::new();
    planner.plan_fft_inverse(fft_size).process(&mut buf);

    let mut samples: Vec<f32> = buf.iter().map(|c| c.re).collect();
    
    let min = samples.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let max = samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let range = (max - min).max(1e-6);
    for s in &mut samples { *s = (*s - min) / range; }

    phase_normalize(&mut samples);

    let mut new_curve = fit_bezier_adaptive(&samples, 128, curve.curve_id, "Peaks And Valleys");
    new_curve.simplify(0.001);
    Ok(new_curve)
}

/// Transform Geometry curve to Spectrum via dense sampling.
pub fn geometry_to_spectrum(curve: &BezierCurve, fft_size: usize) -> Result<BezierCurve, String> {
    let samples = curve.sample(fft_size);
    let mut buf: Vec<Complex<f32>> = samples.iter().map(|&y| Complex { re: y - 0.5, im: 0.0 }).collect();
    let mut planner = FftPlanner::new();
    planner.plan_fft_forward(fft_size).process(&mut buf);

    let num_harmonics = fft_size / 2;
    let magnitudes: Vec<f32> = buf[..num_harmonics].iter().map(|c| c.norm() / fft_size as f32).collect();
    let max = magnitudes[1..].iter().cloned().fold(0.001f32, f32::max);
    let normalized: Vec<f32> = if max > 0.0 { magnitudes.iter().map(|&m| m / max).collect() } else { magnitudes };

    let dense = log_dense(&normalized);
    let mut new_curve = fit_bezier(&dense, SPEC_FIT_SEGMENTS, curve.curve_id, "Peaks And Valleys");
    new_curve.simplify(0.001);
    Ok(new_curve)
}

fn sample_log_freq(curve: &BezierCurve, num_harmonics: usize) -> Vec<f32> {
    (0..num_harmonics)
        .map(|k| {
            if k == 0 { 0.0 }
            else { curve.eval((k as f32).log2() / LOG_FREQ_SCALE) }
        })
        .collect()
}

fn phase_normalize(samples: &mut Vec<f32>) {
    let n = samples.len();
    let shift = (0..n).find(|&i| {
        samples[i] < 0.5 && samples[(i + 1) % n] >= 0.5
    });
    if let Some(offset) = shift {
        samples.rotate_left(offset + 1);
    }
}
