# Zebra Curve Transform

## Overview

A browser-based tool for transforming oscillator curves between time domain and spectral domain representations, designed to work with u-he Zebra3's spline-based oscillator.

Zebra3 uses cubic Bézier splines to define oscillator waveforms. It has two interpretation modes:
- **Geometry** (time domain): the curve shape IS the waveform — x is phase (one cycle), y is amplitude
- **Spectrum** (frequency domain): x is harmonic number, y is harmonic amplitude

This tool implements the "conversion button" discussed on KVR: transform a curve so it maintains the same sound when switching between interpretation modes. It also enables a dual-domain editing workflow (edit in time domain, transform, edit in spectral domain, transform back, etc.) and exploration of lossy round-trip artifacts.

## Key Technical Concepts

### The Transform
- **Time → Spectral**: Evaluate Bézier curve into samples → FFT → extract magnitudes → fit new Béziers to magnitude spectrum
- **Spectral → Time**: Evaluate Bézier curve as harmonic amplitudes → IFFT (with chosen phase convention) → fit new Béziers to resulting waveform
- Round trip is inherently lossy because spectrum mode discards phase information
- Phase convention options: zero phase, minimum phase (configurable)

### Zebra3 Clipboard Format
Zebra3 supports copy/paste of curves. The internal format is:

```
// u-he Bezier Curve
// Version 1.0
Curve ID = 1 MorphType = 'Peaks And Valleys'
PX XY = '0/3F006807' OV = '3EB65325/3E9BF21C'
PX XY = '3E4CCCCD/3F5CD30E' IV = '3F2FE8E0/3F24346D' OV = '3EBE0644/3EA36414'
PX XY = '3ECCCCCD/3E530DAC' IV = '3F20882E/3F3DD28A' OV = '3E8A67BA/3F1AC7AB'
...
PX XY = '3F800000/3F006807' IV = '3F2AAAAB/3F2AAAAB'
```

- Values are IEEE 754 single-precision floats encoded as hex strings
- `PX XY = 'X_HEX/Y_HEX'` — control point position in 0–1 space
- `IV = 'X_HEX/Y_HEX'` — incoming tangent handle
- `OV = 'X_HEX/Y_HEX'` — outgoing tangent handle
- Y: 0.5 = zero crossing (center line), range 0–1
- X: 0 = start of cycle, 1 = end of cycle
- First point has no IV, last point has no OV

Zebra3 also exports SVG (cubic Bézier paths in 100×100 space, y-axis inverted relative to internal format). SVG export is useful for visualization but the internal format is needed for round-trip paste.

### Bézier Curve Details
The curves are piecewise cubic Béziers. Between two consecutive control points P0 and P1:
- P0's outgoing handle (OV) defines the first control point offset
- P1's incoming handle (IV) defines the second control point offset
- The exact relationship between OV/IV values and Bézier control points needs to be verified (they may be tangent vectors, handle lengths, or absolute positions)

Line segments (L in SVG) appear when OV/IV are absent or indicate linear interpolation.

## Architecture

### Tech Stack
- **Language**: Rust, compiled to WASM
- **UI framework**: egui (via eframe) — compiles to both WASM and native
- **FFT**: rustfft
- **Audio**: Web Audio API via web-sys
- **MIDI**: Web MIDI API via web-sys
- **Hosting**: GitHub Pages (fully client-side, no backend)
- **Build**: wasm-pack + cargo

### Modules

```
zebra-curve-transform/
├── Cargo.toml
├── src/
│   ├── lib.rs              # WASM entry point
│   ├── app.rs              # egui app struct and main UI
│   ├── bezier.rs           # Bézier evaluation, sampling, fitting
│   ├── zebra_format.rs     # Parse/generate Zebra3 clipboard format
│   ├── transform.rs        # FFT/IFFT domain transforms
│   ├── curve_editor.rs     # Interactive Bézier curve editor widget
│   ├── audio.rs            # Web Audio playback (wavetable from curve)
│   └── midi.rs             # Web MIDI input handling
├── index.html              # Minimal HTML shell
├── SPEC.md
└── README.md
```

### Core Data Types

```rust
/// A single control point on the curve
struct ControlPoint {
    position: (f32, f32),      // x, y in 0–1 space
    incoming: Option<(f32, f32)>, // IV tangent handle (None for first point)
    outgoing: Option<(f32, f32)>, // OV tangent handle (None for last point)
}

/// A complete Bézier curve (one waveform cycle)
struct BezierCurve {
    points: Vec<ControlPoint>,
    curve_id: u32,
    morph_type: String,
}

/// Which domain the curve currently represents
enum Domain {
    Geometry,  // time domain — x is phase, y is amplitude
    Spectrum,  // frequency domain — x is harmonic, y is magnitude
}
```

## Features (Priority Order)

### MVP
1. Parse Zebra3 clipboard format (paste text area)
2. Evaluate and display Bézier curve
3. Transform: Geometry ↔ Spectrum
4. Generate Zebra3 clipboard format (copy text area)
5. Basic audio playback (single pitch, computer keyboard trigger)

### Next
6. Interactive curve editor (add/remove/drag points)
7. Grid overlay (harmonics grid for spectrum mode)
8. MIDI note input
9. Both rendering modes (hear curve as geometry OR spectrum)
10. SVG import/export

### Later
11. Undo/redo
12. Multiple curves / morphing
13. Round-trip iteration controls (apply transform N times)
14. Polyphony
15. Port core logic to NIH-plug CLAP/VST3 plugin

## Open Questions

1. **OV/IV interpretation**: Need to verify exactly how the tangent handle values map to cubic Bézier control points. Compare parsed internal format against SVG output to establish the mapping.
2. **Bézier fitting quality**: What algorithm and how many control points produce good results? Least-squares fitting with adjustable point count is the plan.
3. **Phase convention**: Which default phase reconstruction sounds best? Zero phase produces symmetric waveforms, minimum phase concentrates energy at cycle start.
4. **Curve constraints**: Zebra3 curves appear to be functions (single y per x). Need to confirm and enforce this in the editor.

## References

- KVR thread discussing the feature idea: Zebralette 3 forum
- Zebra3 public beta: https://u-he.com/products/freeware/zebralette/
- egui: https://github.com/emilk/egui
- NIH-plug (future VST/CLAP): https://github.com/robbert-vdh/nih-plug
- rustfft: https://github.com/ejmahler/RustFFT
