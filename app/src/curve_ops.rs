//! Point-wise curve transforms that operate on the current curve.
//! All functions take a BezierCurve, operate on a sample array, and return a new BezierCurve.

use curve_core::bezier::{fit_bezier, fit_bezier_interp, BezierCurve};

const GEO_SAMPLES: usize = 2048;
const GEO_SEGMENTS: usize = 20;

const SPEC_HARMONICS: usize = 1024;
const SPEC_DENSE: usize = 1024;
const SPEC_SEGMENTS: usize = 10;
const LOG_FREQ_SCALE: f32 = 10.0; // must match transform.rs

// ── LCG (separate state from waveforms.rs so jitter and presets don't interfere) ──

thread_local! {
    static LCG: std::cell::Cell<u32> = const { std::cell::Cell::new(0xA3D1F4B2) };
}

fn rand_f32() -> f32 {
    LCG.with(|s| {
        let v = s.get().wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s.set(v);
        (v >> 8) as f32 / 16_777_216.0
    })
}

// ── shared helpers ─────────────────────────────────────────────────────────────

fn geo_resample(curve: &BezierCurve) -> Vec<f32> {
    curve.sample(GEO_SAMPLES)
}

fn geo_fit(samples: Vec<f32>, src: &BezierCurve) -> BezierCurve {
    fit_bezier_interp(&samples, GEO_SEGMENTS, src.curve_id, &src.morph_type)
}

fn spec_resample(curve: &BezierCurve) -> Vec<f32> {
    (1..=SPEC_HARMONICS)
        .map(|k| curve.eval((k as f32).log2() / LOG_FREQ_SCALE).max(0.0))
        .collect()
}

fn amps_to_curve(amps: Vec<f32>, src: &BezierCurve) -> BezierCurve {
    let dense: Vec<f32> = (0..SPEC_DENSE)
        .map(|i| {
            let x = i as f32 / (SPEC_DENSE - 1) as f32;
            let k = (2.0f32.powf(x * LOG_FREQ_SCALE) as usize)
                .max(1)
                .min(amps.len());
            amps[k - 1]
        })
        .collect();
    fit_bezier(&dense, SPEC_SEGMENTS, src.curve_id, &src.morph_type)
}

fn normalize_amps(mut v: Vec<f32>) -> Vec<f32> {
    let max = v.iter().cloned().fold(0.0f32, f32::max);
    if max > 0.0 {
        v.iter_mut().for_each(|a| *a /= max);
    }
    v
}

// ── geometry transforms ────────────────────────────────────────────────────────

/// Mirror amplitude around y=0.5: peaks become troughs and vice versa.
pub fn flip_y(curve: &BezierCurve) -> BezierCurve {
    let s = geo_resample(curve).into_iter().map(|y| 1.0 - y).collect();
    geo_fit(s, curve)
}

/// Reverse the waveform in time.
pub fn flip_x(curve: &BezierCurve) -> BezierCurve {
    let mut s = geo_resample(curve);
    s.reverse();
    geo_fit(s, curve)
}

/// Fold the negative half upward: y → |y − 0.5| + 0.5.
/// Doubles apparent frequency; all energy lands above the centre line.
pub fn rectify(curve: &BezierCurve) -> BezierCurve {
    let s = geo_resample(curve).into_iter().map(|y| (y - 0.5).abs() + 0.5).collect();
    geo_fit(s, curve)
}

/// Wavefolder (gain × 2): values that exceed [0,1] after scaling fold back inward.
/// Introduces new harmonics; peaks fold down towards centre.
pub fn fold(curve: &BezierCurve) -> BezierCurve {
    let s = geo_resample(curve)
        .into_iter()
        .map(|y| {
            let x = 2.0 * (2.0 * y - 1.0); // scale to [-2, 2]
            // Zigzag fold into [-1, 1], then back to [0, 1]
            let x_sh = (x + 1.0).rem_euclid(4.0);
            let folded = if x_sh <= 2.0 { x_sh - 1.0 } else { 3.0 - x_sh };
            (folded + 1.0) * 0.5
        })
        .collect();
    geo_fit(s, curve)
}

/// Step-quantize amplitude to 8 levels — bit-crush character.
pub fn quantize(curve: &BezierCurve) -> BezierCurve {
    const LEVELS: f32 = 8.0;
    let s = geo_resample(curve)
        .into_iter()
        .map(|y| (y * LEVELS).round() / LEVELS)
        .collect();
    geo_fit(s, curve)
}

/// Shift the waveform so its DC component sits at y=0.5.
pub fn dc_remove(curve: &BezierCurve) -> BezierCurve {
    let mut s = geo_resample(curve);
    let mean = s.iter().sum::<f32>() / s.len() as f32;
    let shift = 0.5 - mean;
    s.iter_mut().for_each(|y| *y = (*y + shift).clamp(0.0, 1.0));
    geo_fit(s, curve)
}

/// Add small random perturbations to each sample — organic mutation of the current curve.
pub fn geo_jitter(curve: &BezierCurve) -> BezierCurve {
    const AMOUNT: f32 = 0.05;
    let s = geo_resample(curve)
        .into_iter()
        .map(|y| (y + AMOUNT * (rand_f32() * 2.0 - 1.0)).clamp(0.0, 1.0))
        .collect();
    geo_fit(s, curve)
}

// ── spectral transforms ────────────────────────────────────────────────────────

/// Boost high harmonics (√k weighting) — makes the timbre brighter.
pub fn brighten(curve: &BezierCurve) -> BezierCurve {
    let amps = normalize_amps(
        spec_resample(curve)
            .into_iter()
            .enumerate()
            .map(|(i, a)| a * ((i + 1) as f32).sqrt())
            .collect(),
    );
    amps_to_curve(amps, curve)
}

/// Attenuate high harmonics (1/√k weighting) — makes the timbre darker.
pub fn darken(curve: &BezierCurve) -> BezierCurve {
    let amps = normalize_amps(
        spec_resample(curve)
            .into_iter()
            .enumerate()
            .map(|(i, a)| a / ((i + 1) as f32).sqrt())
            .collect(),
    );
    amps_to_curve(amps, curve)
}

/// Zero out even harmonics — keeps only odd (1, 3, 5, …). Square-wave character.
pub fn thin(curve: &BezierCurve) -> BezierCurve {
    let amps: Vec<f32> = spec_resample(curve)
        .into_iter()
        .enumerate()
        .map(|(i, a)| if i % 2 == 0 { a } else { 0.0 }) // i=0 → k=1 (odd), keep
        .collect();
    amps_to_curve(amps, curve)
}

/// Shift the spectrum up by one octave: harmonic k moves to harmonic 2k.
pub fn octave_up(curve: &BezierCurve) -> BezierCurve {
    let src = spec_resample(curve);
    let mut amps = vec![0.0f32; SPEC_HARMONICS];
    for (i, &a) in src.iter().enumerate() {
        let new_k = 2 * (i + 1);
        if new_k <= SPEC_HARMONICS {
            amps[new_k - 1] = a;
        }
    }
    amps_to_curve(amps, curve)
}

/// Nudge each harmonic amplitude by a small random amount — adds grit while
/// preserving the overall spectral shape.
pub fn spec_jitter(curve: &BezierCurve) -> BezierCurve {
    const AMOUNT: f32 = 0.1;
    let amps: Vec<f32> = spec_resample(curve)
        .into_iter()
        .map(|a| (a + AMOUNT * (rand_f32() * 2.0 - 1.0)).clamp(0.0, 1.0))
        .collect();
    amps_to_curve(amps, curve)
}
