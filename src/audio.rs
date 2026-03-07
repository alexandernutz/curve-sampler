//! Web Audio API playback — single-cycle looping wavetable.

/// Play `samples` (in [0, 1] range, 0.5 = zero amplitude) as a looping
/// single-cycle wavetable at `frequency` Hz. Stops any current playback first.
#[allow(unused_variables)]
pub fn play_once(samples: &[f32], frequency: f32) {
    #[cfg(target_arch = "wasm32")]
    play_wasm(samples, frequency);
}

/// Stop any currently playing sound.
#[allow(dead_code)]
pub fn stop() {
    #[cfg(target_arch = "wasm32")]
    stop_wasm();
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    // Keep the AudioContext alive across calls (browsers limit how many you can create).
    static AUDIO_CTX: std::cell::RefCell<Option<web_sys::AudioContext>> =
        std::cell::RefCell::new(None);
    // Current source node — held so we can stop it.
    static CURRENT_SOURCE: std::cell::RefCell<Option<web_sys::AudioBufferSourceNode>> =
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
}

#[cfg(target_arch = "wasm32")]
fn play_wasm(samples: &[f32], frequency: f32) {
    stop_wasm();

    // Get or create a single shared AudioContext.
    let ctx = AUDIO_CTX.with(|cell| {
        let mut opt = cell.borrow_mut();
        if opt.is_none() {
            *opt = web_sys::AudioContext::new().ok();
        }
        opt.clone()
    });
    let ctx = match ctx {
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

    if source.connect_with_audio_node(&ctx.destination()).is_err() {
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
