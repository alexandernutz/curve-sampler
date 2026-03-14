use rustfft::{num_complex::Complex, FftPlanner};

use crate::bezier::{fit_bezier, BezierCurve};

/// Number of Bézier segments used when fitting the transform output.
/// Matches LOG_FREQ_SCALE so that each fit point at x=j/10 corresponds to harmonic 2^j.
const FIT_SEGMENTS: usize = 10;

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
fn sample_log_freq(curve: &BezierCurve, num_harmonics: usize) -> Vec<f32> {
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

    let max = magnitudes.iter().cloned().fold(0.0f32, f32::max);
    let normalized: Vec<f32> = if max > 0.0 {
        magnitudes.iter().map(|&m| m / max).collect()
    } else {
        magnitudes
    };

    // Place control points at x = j/10 (j = 0..FIT_SEGMENTS), each corresponding to
    // harmonic k = 2^j via the formula x = log2(k)/10.
    // normalized[k] holds the FFT magnitude for harmonic k (buf[k] in the forward FFT).
    let log_samples: Vec<f32> = (0..=FIT_SEGMENTS)
        .map(|j| {
            let k = 1usize << j; // 2^j = harmonic at x = j/10
            normalized.get(k).copied().unwrap_or(0.0)
        })
        .collect();

    Ok(fit_bezier(&log_samples, FIT_SEGMENTS, curve.curve_id, &curve.morph_type))
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

    Ok(fit_bezier(&normalized, FIT_SEGMENTS, curve.curve_id, &curve.morph_type))
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
