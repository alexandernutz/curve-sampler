//! Web MIDI API input handling — stub for MVP.

/// Request MIDI access and register message handlers.
/// Currently a no-op; implementation deferred post-MVP.
pub fn init() {
    #[cfg(target_arch = "wasm32")]
    init_wasm();
}

#[cfg(target_arch = "wasm32")]
fn init_wasm() {
    use wasm_bindgen_futures::spawn_local;

    spawn_local(async {
        // navigator.requestMIDIAccess() returns a Promise<MIDIAccess>.
        // TODO: call it, iterate ports, attach onmidimessage callbacks.
        log::warn!("midi: MIDI access not yet requested (stub)");
    });
}
