use rustfft::{num_complex::Complex, FftPlanner};

use crate::bezier::{fit_bezier, BezierCurve};

/// Which domain the curve currently represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Domain {
    #[default]
    /// Time domain — x is phase (0–1 cycle), y is amplitude (0.5 = zero)
    Geometry,
    /// Frequency domain — x is harmonic index, y is magnitude
    Spectrum,
}

/// Number of Bézier segments used when fitting the transform output.
const FIT_SEGMENTS: usize = 16;

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

    Ok(fit_bezier(&normalized, FIT_SEGMENTS, curve.curve_id, &curve.morph_type))
}

/// Frequency domain → time domain (zero-phase reconstruction).
///
/// 1. Sample curve as harmonic amplitudes
/// 2. Build a real-valued spectrum with zero phase
/// 3. IFFT
/// 4. Normalize to [0, 1]
/// 5. Fit new Bézier curve
pub fn spectrum_to_geometry(curve: &BezierCurve, fft_size: usize) -> Result<BezierCurve, String> {
    let num_harmonics = fft_size / 2;
    let harmonic_amps = curve.sample(num_harmonics);

    let mut buf: Vec<Complex<f32>> = vec![Complex::default(); fft_size];
    for (i, &amp) in harmonic_amps.iter().enumerate() {
        let mag = amp * fft_size as f32 * 0.5;
        buf[i] = Complex { re: mag, im: 0.0 };
        if i > 0 {
            buf[fft_size - i] = Complex { re: mag, im: 0.0 };
        }
    }

    FftPlanner::new().plan_fft_inverse(fft_size).process(&mut buf);

    let real: Vec<f32> = buf.iter().map(|c| c.re / fft_size as f32).collect();
    let min = real.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = real.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(1e-6);
    let normalized: Vec<f32> = real.iter().map(|&x| (x - min) / range).collect();

    Ok(fit_bezier(&normalized, FIT_SEGMENTS, curve.curve_id, &curve.morph_type))
}

/// Frequency domain → audio samples (zero-phase reconstruction, no Bézier fit).
///
/// Suitable for direct audio playback. DC component is zeroed.
/// Returns values in [0, 1] (0.5 = zero amplitude).
pub fn spectrum_to_waveform_samples(curve: &BezierCurve, fft_size: usize) -> Vec<f32> {
    let num_harmonics = fft_size / 2;
    let harmonic_amps = curve.sample(num_harmonics);

    let mut buf: Vec<Complex<f32>> = vec![Complex::default(); fft_size];
    // Skip i=0 (DC) — oscillators should be zero-mean
    for (i, &amp) in harmonic_amps.iter().enumerate().skip(1) {
        let mag = amp * fft_size as f32 * 0.5;
        buf[i] = Complex { re: mag, im: 0.0 };
        buf[fft_size - i] = Complex { re: mag, im: 0.0 };
    }

    FftPlanner::new().plan_fft_inverse(fft_size).process(&mut buf);

    let real: Vec<f32> = buf.iter().map(|c| c.re / fft_size as f32).collect();
    let min = real.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = real.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(1e-6);
    real.iter().map(|&x| (x - min) / range).collect()
}
