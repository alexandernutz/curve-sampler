//! Web Audio API playback — single-cycle looping wavetable.

/// Play `samples` (in [0, 1] range, 0.5 = zero amplitude) as a looping
/// single-cycle wavetable at `frequency` Hz. Stops any current playback first.
#[allow(unused_variables)]
pub fn play_once(samples: &[f32], frequency: f32, gain: f32) {
    #[cfg(target_arch = "wasm32")]
    play_wasm(samples, frequency, gain);
}

/// Play a spectrum via additive synthesis using Web Audio PeriodicWave.
/// `harmonics[k]` = amplitude of harmonic k+1 (index 0 = fundamental).
/// The browser bandlimits automatically — no aliasing at any pitch.
#[allow(unused_variables)]
pub fn play_spectrum(harmonics: &[f32], frequency: f32, gain: f32) {
    #[cfg(target_arch = "wasm32")]
    play_spectrum_wasm(harmonics, frequency, gain);
}

/// Stop any currently playing sound.
#[allow(dead_code)]
pub fn stop() {
    #[cfg(target_arch = "wasm32")]
    stop_wasm();
}

/// Set the master output gain (0.0 = silent, 1.0 = full).
#[allow(unused_variables)]
pub fn set_gain(value: f32) {
    #[cfg(target_arch = "wasm32")]
    GAIN_NODE.with(|cell| {
        if let Some(g) = cell.borrow().as_ref() {
            g.gain().set_value(value);
        }
    });
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    // Keep the AudioContext alive across calls (browsers limit how many you can create).
    static AUDIO_CTX: std::cell::RefCell<Option<web_sys::AudioContext>> =
        std::cell::RefCell::new(None);
    // Current buffer source node — held so we can stop it.
    static CURRENT_SOURCE: std::cell::RefCell<Option<web_sys::AudioBufferSourceNode>> =
        std::cell::RefCell::new(None);
    // Current oscillator node (used for spectral playback).
    static CURRENT_OSC: std::cell::RefCell<Option<web_sys::OscillatorNode>> =
        std::cell::RefCell::new(None);
    // Shared gain node — all sources route through this for volume control.
    static GAIN_NODE: std::cell::RefCell<Option<web_sys::GainNode>> =
        std::cell::RefCell::new(None);
}

#[cfg(target_arch = "wasm32")]
fn stop_wasm() {
    CURRENT_SOURCE.with(|cell| {
        if let Some(src) = cell.borrow_mut().take() {
            #[allow(deprecated)]
            let _ = src.stop();
        }
    });
    CURRENT_OSC.with(|cell| {
        if let Some(osc) = cell.borrow_mut().take() {
            let _ = osc.stop();
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn get_or_create_ctx() -> Option<web_sys::AudioContext> {
    AUDIO_CTX.with(|cell| {
        let mut opt = cell.borrow_mut();
        if opt.is_none() {
            *opt = web_sys::AudioContext::new().ok();
        }
        opt.clone()
    })
}

/// Returns the shared gain node, creating it (and connecting to destination) if needed.
/// `initial_gain` is applied only on first creation.
#[cfg(target_arch = "wasm32")]
fn get_or_create_gain(ctx: &web_sys::AudioContext, initial_gain: f32) -> Option<web_sys::GainNode> {
    GAIN_NODE.with(|cell| {
        let mut opt = cell.borrow_mut();
        if opt.is_none() {
            if let Ok(g) = ctx.create_gain() {
                g.gain().set_value(initial_gain);
                let _ = g.connect_with_audio_node(&ctx.destination());
                *opt = Some(g);
            }
        }
        opt.clone()
    })
}

#[cfg(target_arch = "wasm32")]
fn play_wasm(samples: &[f32], frequency: f32, gain: f32) {
    stop_wasm();

    let ctx = match get_or_create_ctx() {
        Some(c) => c,
        None => {
            log::error!("audio: failed to create AudioContext");
            return;
        }
    };

    let n = samples.len() as u32;
    let sample_rate = ctx.sample_rate();

    let buffer = match ctx.create_buffer(1, n, sample_rate) {
        Ok(b) => b,
        Err(e) => {
            log::error!("audio: create_buffer failed: {:?}", e);
            return;
        }
    };

    // Convert [0, 1] → [-1, 1]
    let mut audio: Vec<f32> = samples.iter().map(|&y| (y - 0.5) * 2.0).collect();
    if buffer.copy_to_channel(&mut audio, 0).is_err() {
        log::error!("audio: copy_to_channel failed");
        return;
    }

    let source = match ctx.create_buffer_source() {
        Ok(s) => s,
        Err(e) => {
            log::error!("audio: create_buffer_source failed: {:?}", e);
            return;
        }
    };

    source.set_buffer(Some(&buffer));
    source.set_loop(true);

    // Pitch: one buffer cycle should repeat at `frequency` Hz.
    // playback_rate = frequency * buffer_length / sample_rate
    let playback_rate = frequency * n as f32 / sample_rate;
    source.playback_rate().set_value(playback_rate);

    let dest = match get_or_create_gain(&ctx, gain) {
        Some(g) => g.into(),
        None => ctx.destination().into(),
    };
    if source.connect_with_audio_node(&dest).is_err() {
        log::error!("audio: connect failed");
        return;
    }

    if source.start().is_err() {
        log::error!("audio: start failed");
        return;
    }

    CURRENT_SOURCE.with(|cell| {
        *cell.borrow_mut() = Some(source);
    });
}

#[cfg(target_arch = "wasm32")]
fn play_spectrum_wasm(harmonics: &[f32], frequency: f32, gain: f32) {
    stop_wasm();

    let ctx = match get_or_create_ctx() {
        Some(c) => c,
        None => {
            log::error!("audio: failed to create AudioContext");
            return;
        }
    };

    let n = harmonics.len();
    // PeriodicWave arrays: index 0 = DC, index k = harmonic k.
    // Signal = Σ (real[k]*cos - imag[k]*sin); use real[k]=amp for cosine-phase output.
    //
    // RMS-normalize: scale so sqrt(Σ aₖ²) = 1 → consistent loudness regardless of
    // harmonic count. Browser peak normalization is disabled to preserve this scaling.
    // (Default peak normalization makes dense spectra sound very faint because the
    // cosine-phase peak is large but the RMS is low.)
    let sum_sq: f32 = harmonics.iter().map(|&a| a * a).sum();
    let rms_scale = if sum_sq > 1e-12 { sum_sq.sqrt().recip() } else { 1.0 };

    let mut real = vec![0.0f32; n + 1];
    let mut imag = vec![0.0f32; n + 1];
    for (k, &amp) in harmonics.iter().enumerate() {
        real[k + 1] = (amp * rms_scale).max(0.0);
    }

    let mut constraints = web_sys::PeriodicWaveConstraints::new();
    constraints.disable_normalization(true);

    let wave = match ctx.create_periodic_wave_with_constraints(&mut real, &mut imag, &constraints) {
        Ok(w) => w,
        Err(e) => {
            log::error!("audio: create_periodic_wave failed: {:?}", e);
            return;
        }
    };

    let osc = match ctx.create_oscillator() {
        Ok(o) => o,
        Err(e) => {
            log::error!("audio: create_oscillator failed: {:?}", e);
            return;
        }
    };

    osc.set_periodic_wave(&wave);
    osc.frequency().set_value(frequency);

    let dest = match get_or_create_gain(&ctx, gain) {
        Some(g) => g.into(),
        None => ctx.destination().into(),
    };
    if osc.connect_with_audio_node(&dest).is_err() {
        log::error!("audio: oscillator connect failed");
        return;
    }

    if osc.start().is_err() {
        log::error!("audio: oscillator start failed");
        return;
    }

    CURRENT_OSC.with(|cell| {
        *cell.borrow_mut() = Some(osc);
    });
}
