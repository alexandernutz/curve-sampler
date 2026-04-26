use rustfft::{num_complex::Complex, FftPlanner};

use crate::bezier::{fit_bezier, fit_bezier_interp, BezierCurve};

/// Segments for spectrum output — one per octave band, semantically grounded.
const SPEC_FIT_SEGMENTS: usize = 10;
/// Segments for geometry output — higher resolution for time-domain waveforms.
const GEO_FIT_SEGMENTS: usize = 20;

/// Zebra3 maps harmonic k to curve x-position: x = log2(k) / LOG_FREQ_SCALE.
///
/// Confirmed by placing control points at known harmonic positions:
///   P1.x = 0x3DCCCCCD = log2(2)/10 = 0.1000  → harmonic 2
///   P2.x = 0x3E224CD7 = log2(3)/10 = 0.1585  → harmonic 3
/// A curve with harmonics 1 and 2 at full amplitude and harmonic 3 at zero is produced by
/// placing P1=(0.1, 1.0) and P2=(0.1585, 0.0), consistent with left-edge sampling.
///
/// Rule: harmonic k's amplitude = curve value at x = log2(k) / 10.
/// The X axis covers harmonics 1 (x=0) through 2^10 = 1024 (x=1).
const LOG_FREQ_SCALE: f32 = 10.0;

/// Sample the spectrum curve at Zebra3's log-frequency positions.
/// Returns amplitude for each harmonic k = 1..num_harmonics (index 0 = harmonic 1).
pub fn sample_log_freq(curve: &BezierCurve, num_harmonics: usize) -> Vec<f32> {
    (1..=num_harmonics)
        .map(|k| curve.eval((k as f32).log2() / LOG_FREQ_SCALE))
        .collect()
}

/// Time domain → frequency domain.
///
/// 1. Sample curve into N time-domain values; shift y to [-0.5, 0.5]
/// 2. FFT
/// 3. Extract magnitudes of the first N/2 positive-frequency bins
/// 4. Normalize to [0, 1]
/// 5. Fit new Bézier curve
pub fn geometry_to_spectrum(curve: &BezierCurve, fft_size: usize) -> Result<BezierCurve, String> {
    let samples = curve.sample(fft_size);
    let mut buf: Vec<Complex<f32>> = samples
        .iter()
        .map(|&y| Complex { re: y - 0.5, im: 0.0 })
        .collect();

    FftPlanner::new().plan_fft_forward(fft_size).process(&mut buf);

    let num_harmonics = fft_size / 2;
    let magnitudes: Vec<f32> = buf[..num_harmonics]
        .iter()
        .map(|c| c.norm() / fft_size as f32)
        .collect();

    // Skip magnitudes[0] (DC): waveforms with mean ≠ 0.5 produce a large DC term
    // after the y−0.5 shift, which would otherwise dominate and crush the harmonics.
    let max = magnitudes[1..].iter().cloned().fold(0.0f32, f32::max);
    let normalized: Vec<f32> = if max > 0.0 {
        magnitudes.iter().map(|&m| m / max).collect()
    } else {
        magnitudes
    };

    // Dense log-spaced sampling: map all harmonic amplitudes into an evenly-spaced
    // array over x ∈ [0, 1] and let least-squares fit_bezier find the best smooth curve.
    // This uses every harmonic (not just octave boundaries), so the fit reflects the
    // full spectral shape rather than just 11 power-of-2 waypoints.
    Ok(fit_bezier(&log_dense(&normalized), SPEC_FIT_SEGMENTS, curve.curve_id, &curve.morph_type))
}

/// Geometry → Spectrum using per-harmonic step functions (Zebra3 style).
///
/// Each harmonic k gets its own step covering x = [log₂(k)/10, log₂(k+1)/10], with
/// height equal to that harmonic's FFT magnitude. Steps become progressively narrower
/// at higher frequencies, matching the log-frequency spacing Z3 uses internally.
///
/// Harmonics above a noise floor and up to MAX_STEP_HARMONICS are included explicitly;
/// the rest are set to zero. This avoids the octave-band artifact where all harmonics
/// in one octave (e.g. 4, 5, 6, 7) would incorrectly share the same amplitude.
pub fn geometry_to_spectrum_steps(curve: &BezierCurve, fft_size: usize) -> Result<BezierCurve, String> {
    let fft_size = fft_size.max(2048); // Ensure enough resolution for 1024 harmonics
    let samples = curve.sample(fft_size);
    let mut buf: Vec<Complex<f32>> = samples
        .iter()
        .map(|&y| Complex { re: y - 0.5, im: 0.0 })
        .collect();
    FftPlanner::new().plan_fft_forward(fft_size).process(&mut buf);

    let num_harmonics = fft_size / 2;
    // Scaling: norm / fft_size * 2.0 (for real FFT) * 2.0 (to map 0.5 center amplitude to 1.0 magnitude)
    let magnitudes: Vec<f32> = buf[..num_harmonics]
        .iter()
        .map(|c| c.norm() * 4.0 / fft_size as f32)
        .collect();
    
    // We don't normalize to 'max' anymore, as our input is already 0..1 peak-normalized 
    // and we want absolute matching with Zebra3 levels.
    let normalized = magnitudes;

    // Per-harmonic steps. Cap at MAX_STEP_HARMONICS; only go as far as the last harmonic
    // above STEP_FLOOR.
    const MAX_STEP_HARMONICS: usize = 1024;
    const STEP_FLOOR: f32 = 0.002;
    // Adjacent steps are merged when their amplitudes differ by less than SIMPLIFY_TOL,
    // removing redundant double-points (e.g. long runs of near-zero harmonics).
    const SIMPLIFY_TOL: f32 = 0.001; // Reduced from 0.01

    let highest = (1..num_harmonics.min(MAX_STEP_HARMONICS + 1))
        .rev()
        .find(|&k| normalized.get(k).copied().unwrap_or(0.0) > STEP_FLOOR)
        .unwrap_or(1);

    let amp = |k: usize| normalized.get(k).copied().unwrap_or(0.0);
    let xpos = |k: usize| (k as f32).log2() / LOG_FREQ_SCALE;

    // Build a step list. We MUST emit points for every harmonic that differs from 
    // its PREVIOUS neighbor to ensure narrow spikes (single-bin) are preserved.
    let mut steps: Vec<(f32, f32)> = vec![(0.0, amp(1))];
    for k in 2..=highest {
        let a = amp(k);
        let prev_a = amp(k-1);
        if (a - prev_a).abs() > SIMPLIFY_TOL {
            steps.push((xpos(k), a));
        }
    }
    // Terminate with a zero step if the last step was high.
    let last_level = steps.last().map(|s| s.1).unwrap_or(0.0);
    if last_level > SIMPLIFY_TOL {
        let x_tail = xpos(highest + 1).min(1.0);
        steps.push((x_tail, 0.0));
    }

    // Convert step list to Bezier points using the double-point trick.
    let mut points = vec![step_cp(steps[0].0, steps[0].1)];
    for i in 1..steps.len() {
        let (x, a) = steps[i];
        let prev_a = steps[i - 1].1;
        points.push(step_cp(x, prev_a)); // end of previous step
        points.push(step_cp(x, a));      // start of current step
    }
    points.push(step_cp(1.0, steps.last().map(|s| s.1).unwrap_or(0.0)));

    Ok(BezierCurve { points, curve_id: curve.curve_id, morph_type: curve.morph_type.clone() })
}

/// Map normalized harmonic amplitudes to a dense evenly-spaced array over x ∈ [0,1].
/// Each output position x maps to harmonic k = 2^(x * LOG_FREQ_SCALE).
fn log_dense(normalized: &[f32]) -> Vec<f32> {
    const DENSE: usize = 1024;
    (0..DENSE)
        .map(|i| {
            let x = i as f32 / (DENSE - 1) as f32;
            let k = (2.0f32.powf(x * LOG_FREQ_SCALE) as usize)
                .max(1)
                .min(normalized.len() - 1);
            normalized[k]
        })
        .collect()
}

fn step_cp(x: f32, y: f32) -> crate::bezier::ControlPoint {
    crate::bezier::ControlPoint { 
        position: (x, y), 
        // 0.33 and 0.66 with 0.33/0.66 Y-values creates a perfectly linear segment in Zebra3's fractional format
        incoming: Some((2.0 / 3.0, 2.0 / 3.0)), 
        outgoing: Some((1.0 / 3.0, 1.0 / 3.0)) 
    }
}

/// Directly convert raw audio samples (one cycle) to a stepped spectral BezierCurve.
/// This bypasses the intermediate Geometry-fitting stage to preserve high-frequency fidelity.
pub fn samples_to_spectrum_steps(
    samples: &[f32], 
    curve_id: u32, 
    morph_type: &str
) -> Result<BezierCurve, String> {
    let n = samples.len();
    let fft_size = n.next_power_of_two().max(2048);
    
    // 1. Calculate actual mean to remove DC offset artifacts
    let mean = samples.iter().sum::<f32>() / n as f32;
    
    let mut buf: Vec<Complex<f32>> = samples
        .iter()
        .enumerate()
        .map(|(i, &y)| {
            // 2. Subtract mean and apply Hann window to reduce leakage
            let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos());
            Complex { re: (y - mean) * window, im: 0.0 }
        })
        .collect();
    
    buf.resize(fft_size, Complex::default());
    FftPlanner::new().plan_fft_forward(fft_size).process(&mut buf);

    let num_harmonics = fft_size / 2;
    // 3. Scaling:
    // norm * 2 / n (Real FFT compensation) 
    // * 2 (Hann window compensation)
    // * 2 (Zebra 0.5 amplitude -> 1.0 magnitude)
    // Total = 8 / n
    let magnitudes: Vec<f32> = buf[..num_harmonics]
        .iter()
        .map(|c| c.norm() * 8.0 / n as f32)
        .collect();

    let normalized = magnitudes;

    const MAX_STEP_HARMONICS: usize = 1024;
    const STEP_FLOOR: f32 = 0.001;
    const SIMPLIFY_TOL: f32 = 0.001;

    let highest = (1..num_harmonics.min(MAX_STEP_HARMONICS + 1))
        .rev()
        .find(|&k| normalized.get(k).copied().unwrap_or(0.0) > STEP_FLOOR)
        .unwrap_or(1);

    let amp = |k: usize| normalized.get(k).copied().unwrap_or(0.0);
    let xpos = |k: usize| (k as f32).log2() / LOG_FREQ_SCALE;

    let mut steps: Vec<(f32, f32)> = vec![(0.0, amp(1))];
    for k in 2..=highest {
        let a = amp(k);
        let prev_a = amp(k-1);
        if (a - prev_a).abs() > SIMPLIFY_TOL {
            steps.push((xpos(k), a));
        }
    }
    
    let last_level = steps.last().map(|s| s.1).unwrap_or(0.0);
    if last_level > SIMPLIFY_TOL {
        let x_tail = xpos(highest + 1).min(1.0);
        steps.push((x_tail, 0.0));
    }

    let mut points = vec![step_cp(steps[0].0, steps[0].1)];
    for i in 1..steps.len() {
        let (x, a) = steps[i];
        let prev_a = steps[i - 1].1;
        points.push(step_cp(x, prev_a));
        points.push(step_cp(x, a));
    }
    points.push(step_cp(1.0, steps.last().map(|s| s.1).unwrap_or(0.0)));

    Ok(BezierCurve { 
        points, 
        curve_id, 
        morph_type: morph_type.to_string() 
    })
}
/// 2. Build a real-valued spectrum with zero phase
/// 3. IFFT
/// 4. Normalize to [0, 1]
/// 5. Phase-normalize: cyclic shift to first upward zero crossing
/// 6. Fit new Bézier curve
pub fn spectrum_to_geometry(curve: &BezierCurve, fft_size: usize) -> Result<BezierCurve, String> {
    let num_harmonics = fft_size / 2;
    let harmonic_amps = sample_log_freq(curve, num_harmonics);

    let mut buf: Vec<Complex<f32>> = vec![Complex::default(); fft_size];
    for (k, &amp) in harmonic_amps.iter().enumerate() {
        let k1 = k + 1; // harmonic_amps[0] = harmonic 1
        let mag = amp * fft_size as f32 * 0.5;
        buf[k1] = Complex { re: mag, im: 0.0 };
        buf[fft_size - k1] = Complex { re: mag, im: 0.0 };
    }

    FftPlanner::new().plan_fft_inverse(fft_size).process(&mut buf);

    let real: Vec<f32> = buf.iter().map(|c| c.re / fft_size as f32).collect();
    let min = real.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = real.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(1e-6);
    let mut normalized: Vec<f32> = real.iter().map(|&x| (x - min) / range).collect();
    phase_normalize(&mut normalized);

    Ok(fit_bezier_interp(&normalized, GEO_FIT_SEGMENTS, curve.curve_id, &curve.morph_type))
}

/// Frequency domain → audio samples (zero-phase reconstruction, no Bézier fit).
///
/// Suitable for direct audio playback. DC component is zeroed.
/// Returns values in [0, 1] (0.5 = zero amplitude).
pub fn spectrum_to_waveform_samples(curve: &BezierCurve, fft_size: usize) -> Vec<f32> {
    let num_harmonics = fft_size / 2;
    let harmonic_amps = sample_log_freq(curve, num_harmonics);

    let mut buf: Vec<Complex<f32>> = vec![Complex::default(); fft_size];
    for (k, &amp) in harmonic_amps.iter().enumerate() {
        let k1 = k + 1; // harmonic_amps[0] = harmonic 1; DC is implicitly zero
        let mag = amp * fft_size as f32 * 0.5;
        buf[k1] = Complex { re: mag, im: 0.0 };
        buf[fft_size - k1] = Complex { re: mag, im: 0.0 };
    }

    FftPlanner::new().plan_fft_inverse(fft_size).process(&mut buf);

    let real: Vec<f32> = buf.iter().map(|c| c.re / fft_size as f32).collect();
    let min = real.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = real.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(1e-6);
    let mut normalized: Vec<f32> = real.iter().map(|&x| (x - min) / range).collect();
    phase_normalize(&mut normalized);
    normalized
}

/// Cyclically rotate `samples` so the cycle starts at the first upward zero crossing
/// (where y crosses 0.5 going upward). This converts cosine-phase IFFT output into
/// the conventional sine-like shape without affecting the audio (looping waveform).
/// Falls back to no shift if no crossing is found (e.g. flat or DC-only signal).
fn phase_normalize(samples: &mut Vec<f32>) {
    let n = samples.len();
    let shift = (0..n).find(|&i| {
        samples[i] < 0.5 && samples[(i + 1) % n] >= 0.5
    });
    if let Some(offset) = shift {
        samples.rotate_left(offset + 1);
    }
}
