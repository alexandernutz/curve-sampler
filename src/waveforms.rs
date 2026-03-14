//! Preset waveform generators.
//!
//! Each preset produces a Zebra3 clipboard string ready to fill the input box.
//! Simple geometric shapes are direct-construct BezierCurves (minimal points, exact shape).
//! Smooth or irregular shapes go through fit_bezier on a sample array.

use crate::bezier::{fit_bezier, BezierCurve, ControlPoint};
use crate::zebra_format;

// ── constants ──────────────────────────────────────────────────────────────────

const GEO_SAMPLES: usize = 2048;
const GEO_SEGMENTS: usize = 20;

const SPEC_HARMONICS: usize = 1024;
/// Dense log-space sample count fed to fit_bezier for spectral presets.
const SPEC_DENSE: usize = 1024;
/// One segment per octave band; matches LOG_FREQ_SCALE.
const SPEC_SEGMENTS: usize = 10;
const LOG_FREQ_SCALE: f32 = 10.0;

// ── LCG random number generator ────────────────────────────────────────────────
//
// Thread-local state persists across button presses so each "Chaos" click
// gives a different result without any external crate.

thread_local! {
    static LCG: std::cell::Cell<u32> = const { std::cell::Cell::new(0x6D2B50A3) };
}

fn rand_f32() -> f32 {
    LCG.with(|s| {
        let v = s.get().wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s.set(v);
        (v >> 8) as f32 / 16_777_216.0
    })
}

// ── helpers ────────────────────────────────────────────────────────────────────

fn cp(x: f32, y: f32) -> ControlPoint {
    ControlPoint { position: (x, y), incoming: None, outgoing: None }
}

fn make_curve(points: Vec<ControlPoint>) -> BezierCurve {
    BezierCurve { points, curve_id: 1, morph_type: "Peaks And Valleys".to_string() }
}

fn to_text(curve: &BezierCurve) -> String {
    zebra_format::generate(curve)
}

// ── geometry: direct-construct (exact shapes, minimal points) ─────────────────

fn sawtooth_curve() -> BezierCurve {
    // Falling straight line y=1→0. Two points with None handles = linear.
    make_curve(vec![cp(0.0, 1.0), cp(1.0, 0.0)])
}

fn square_curve() -> BezierCurve {
    // Flat top, steep near-vertical drop at x=0.5, flat bottom.
    // Transition over Δx=0.002 — no grid constraint in Zebra3.
    make_curve(vec![
        cp(0.0,   1.0),
        cp(0.499, 1.0),
        cp(0.501, 0.0),
        cp(1.0,   0.0),
    ])
}

fn triangle_curve() -> BezierCurve {
    // Peak at x=0.25, trough at x=0.75, zero-crossings at x=0/0.5/1.
    make_curve(vec![
        cp(0.0,  0.5),
        cp(0.25, 1.0),
        cp(0.75, 0.0),
        cp(1.0,  0.5),
    ])
}

fn pulse_curve() -> BezierCurve {
    // 5% duty cycle. Transition over Δx=0.002 like the square wave.
    make_curve(vec![
        cp(0.0,   1.0),
        cp(0.049, 1.0),
        cp(0.051, 0.0),
        cp(1.0,   0.0),
    ])
}

// ── geometry: sample-fitted curves ────────────────────────────────────────────

fn sine_samples() -> Vec<f32> {
    (0..GEO_SAMPLES)
        .map(|i| {
            let x = i as f32 / GEO_SAMPLES as f32;
            0.5 + 0.5 * (std::f32::consts::TAU * x).sin()
        })
        .collect()
}

fn geo_chaos_samples() -> Vec<f32> {
    (0..GEO_SAMPLES).map(|_| rand_f32()).collect()
}

// ── spectral: direct-construct ────────────────────────────────────────────────

fn spec_fundamental_curve() -> BezierCurve {
    // Linear ramp 1→0 across the first harmonic cell [x=0, x=0.1], then flat zero.
    // Harmonic 1 (x=0) reads 1.0; harmonic 2+ (x≥0.1) reads 0.0.
    make_curve(vec![
        cp(0.0, 1.0),
        cp(0.1, 0.0),
        cp(1.0, 0.0),
    ])
}

fn spec_chaos_steps_curve() -> BezierCurve {
    // 24-step staircase on a log-frequency grid matching Zebra3's harmonic sampling.
    // Steps 0–22 each cover one harmonic (k = 1, 2, ..., 23).
    // Step 23 covers harmonic 24 and all higher harmonics to x=1.
    //
    // Boundary between step j-1 and step j sits at x = log₂(j+1) / LOG_FREQ_SCALE,
    // which is exactly where Zebra3 samples harmonic (j+1).
    //
    // The double-point trick: two consecutive points with the same x but different y
    // produce a zero-x-span segment that eval() skips → instantaneous level jump.
    const N_STEPS: usize = 24;
    let amps: Vec<f32> = (0..N_STEPS).map(|_| rand_f32()).collect();

    let mut pts = Vec::with_capacity(2 * N_STEPS);
    pts.push(cp(0.0, amps[0]));
    for j in 1..N_STEPS {
        let x = (j as f32 + 1.0).log2() / LOG_FREQ_SCALE;
        pts.push(cp(x, amps[j - 1])); // end of outgoing step
        pts.push(cp(x, amps[j]));     // start of incoming step (same x → jump)
    }
    // Slope the tail (harmonics 24+) linearly to zero rather than holding
    // the last random level — avoids uncontrolled high-frequency energy.
    pts.push(cp(1.0, 0.0));
    make_curve(pts)
}

// ── spectral: sample-fitted curves ────────────────────────────────────────────

/// Build a dense log-space sample array from a 1024-element harmonic amplitude
/// array. Each sample at evenly-spaced x reads the amplitude of harmonic
/// k = 2^(x * LOG_FREQ_SCALE), matching Zebra3's sampling formula.
fn amps_to_dense(amps: &[f32]) -> Vec<f32> {
    (0..SPEC_DENSE)
        .map(|i| {
            let x = i as f32 / (SPEC_DENSE - 1) as f32;
            let k = (2.0f32.powf(x * LOG_FREQ_SCALE) as usize)
                .max(1)
                .min(amps.len());
            amps[k - 1]
        })
        .collect()
}

fn normalize(mut v: Vec<f32>) -> Vec<f32> {
    let max = v.iter().cloned().fold(0.0f32, f32::max);
    if max > 0.0 {
        v.iter_mut().for_each(|a| *a /= max);
    }
    v
}

fn spec_odd_samples() -> Vec<f32> {
    // Odd harmonics with 1/k rolloff — classic square-wave spectrum
    let amps = normalize(
        (1..=SPEC_HARMONICS)
            .map(|k| if k % 2 == 1 { 1.0 / k as f32 } else { 0.0 })
            .collect(),
    );
    amps_to_dense(&amps)
}

fn spec_even_samples() -> Vec<f32> {
    // Even harmonics with 1/k rolloff — hollow/clarinet-like
    let amps = normalize(
        (1..=SPEC_HARMONICS)
            .map(|k| if k % 2 == 0 { 1.0 / k as f32 } else { 0.0 })
            .collect(),
    );
    amps_to_dense(&amps)
}

fn spec_bright_samples() -> Vec<f32> {
    // Flat spectrum across all 1024 harmonics — maximum brilliance
    amps_to_dense(&vec![1.0f32; SPEC_HARMONICS])
}

fn spec_dark_samples() -> Vec<f32> {
    // 1/k² rolloff — only the lowest harmonics survive
    let amps = normalize(
        (1..=SPEC_HARMONICS)
            .map(|k| 1.0 / (k as f32 * k as f32))
            .collect(),
    );
    amps_to_dense(&amps)
}

// ── preset tables ──────────────────────────────────────────────────────────────

fn geo_direct(c: BezierCurve) -> String   { to_text(&c) }
fn geo_fit(s: Vec<f32>) -> String          { to_text(&fit_bezier(&s, GEO_SEGMENTS,  1, "Peaks And Valleys")) }
fn spec_direct(c: BezierCurve) -> String  { to_text(&c) }
fn spec_fit(s: Vec<f32>) -> String         { to_text(&fit_bezier(&s, SPEC_SEGMENTS, 1, "Peaks And Valleys")) }

/// Geometry-domain presets. Each entry is (button label, generator fn → Zebra3 text).
pub const GEO_PRESETS: &[(&str, fn() -> String)] = &[
    ("Sine",   || geo_fit(sine_samples())),
    ("Square", || geo_direct(square_curve())),
    ("Saw",    || geo_direct(sawtooth_curve())),
    ("Tri",    || geo_direct(triangle_curve())),
    ("Pulse",  || geo_direct(pulse_curve())),
    ("Chaos",  || geo_fit(geo_chaos_samples())),
];

/// Spectrum-domain presets. Each entry is (button label, generator fn → Zebra3 text).
pub const SPEC_PRESETS: &[(&str, fn() -> String)] = &[
    ("Fund.",  || spec_direct(spec_fundamental_curve())),
    ("Odd",    || spec_fit(spec_odd_samples())),
    ("Even",   || spec_fit(spec_even_samples())),
    ("Bright", || spec_fit(spec_bright_samples())),
    ("Dark",   || spec_fit(spec_dark_samples())),
    ("Chaos",  || spec_direct(spec_chaos_steps_curve())),
];
