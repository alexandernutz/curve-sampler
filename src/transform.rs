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

/// Geometry → Spectrum using step functions (L-shapes, Zebra3 style).
///
/// Produces a curve built from double-point steps at octave boundaries — the same
/// idiom Zebra3 uses in its own spectral curves. Each octave band holds a constant
/// amplitude, with instant jumps at x = j/10 boundaries. Semantically exact: the
/// curve value at each harmonic's left-edge x position matches the FFT magnitude.
pub fn geometry_to_spectrum_steps(curve: &BezierCurve, fft_size: usize) -> Result<BezierCurve, String> {
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
    let max = magnitudes[1..].iter().cloned().fold(0.0f32, f32::max);
    let normalized: Vec<f32> = if max > 0.0 {
        magnitudes.iter().map(|&m| m / max).collect()
    } else {
        magnitudes
    };

    // Sample at each octave boundary: harmonic 2^j at x = j/SPEC_FIT_SEGMENTS
    let amps: Vec<f32> = (0..=SPEC_FIT_SEGMENTS)
        .map(|j| normalized.get(1 << j).copied().unwrap_or(0.0))
        .collect();

    // Build step-function curve with double-point trick (same idiom as spec_chaos_steps).
    // Two consecutive points at the same x create a C0 discontinuity (instant step).
    let n = SPEC_FIT_SEGMENTS as f32;
    let mut points = vec![step_cp(0.0, amps[0])];
    for j in 1..SPEC_FIT_SEGMENTS {
        let x = j as f32 / n;
        points.push(step_cp(x, amps[j - 1])); // end of previous step
        points.push(step_cp(x, amps[j]));      // start of next step
    }
    points.push(step_cp(1.0, amps[SPEC_FIT_SEGMENTS]));

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
    crate::bezier::ControlPoint { position: (x, y), incoming: None, outgoing: None }
}

/// Frequency domain → time domain (zero-phase reconstruction).
///
/// 1. Sample curve as harmonic amplitudes
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
