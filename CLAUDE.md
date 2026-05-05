# CLAUDE.md

## Project Context

This is a browser-based tool for transforming Bézier curves between time domain and spectral domain representations, designed to interoperate with u-he Zebra3's spline-based oscillator via clipboard copy/paste.

See `docs/SPEC.md` for full project specification and `docs/TEST_DATA.md` for example Zebra3 curve data with decoded values.

## Tech Stack

- Rust compiled to WASM (wasm-pack + wasm-bindgen)
- egui (via eframe) for UI — targets both WASM and native
- rustfft for FFT/IFFT
- web-sys for Web Audio API and Web MIDI API
- No backend — fully client-side, hosted on GitHub Pages

## Key Design Decisions

- Zebra3 interop is via clipboard: user pastes curve text into a text area, tool outputs transformed curve text for user to copy back
- The Zebra3 internal format uses IEEE 754 hex-encoded floats for Bézier control points (see TEST_DATA.md)
- Curves are cubic Béziers in 0–1 coordinate space, where Y=0.5 is zero amplitude
- The transform is inherently lossy (spectrum mode discards phase) — this is a feature, not a bug

## Build & Run

```bash
# Install wasm-pack if needed
cargo install wasm-pack

# Build for web
wasm-pack build --target web

# Serve locally (for testing)
python3 -m http.server 9000
# Then open http://localhost:9000
```

## Current Priority

Start with the foundation modules in this order:
1. `zebra_format.rs` — parse and generate the Zebra3 clipboard format
2. `bezier.rs` — evaluate Bézier curves into sample arrays
3. `transform.rs` — FFT/IFFT domain conversion + Bézier fitting
4. `app.rs` + `curve_editor.rs` — minimal UI to visualize and test
5. `audio.rs` — Web Audio playback
6. `midi.rs` — Web MIDI input

## Important Notes

- The OV/IV tangent handle values' exact relationship to Bézier control points needs verification. Cross-reference the parsed internal format against the SVG export (both provided in `docs/TEST_DATA.md`) to establish the mapping.
- Bézier fitting (step 4 of the transform: fitting new cubic Béziers to FFT output) is the hardest algorithmic problem. Least-squares B-spline fitting with configurable number of control points is the planned approach.
- egui's custom painting API (`egui::Painter`) supports drawing Bézier paths and handling mouse interaction — use this for the curve editor.
