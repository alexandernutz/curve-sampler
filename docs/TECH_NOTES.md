# Curve Sampler: Technical Implementation Notes

This document outlines the technical architecture and DSP techniques implemented in the **Curve Sampler** plugin.

## 1. Threading & Memory Model

The plugin follows a strict decoupling between the **Real-time Audio Thread** and the **UI Thread** to ensure DAW stability and responsive visualizations.

- **Shared State:** All shared data is wrapped in `Arc` (Atomic Reference Counting).
- **Safe Mutexes:** Using `parking_lot::Mutex` for efficient, non-poisoning locking.
- **UI/Audio Decoupling:** Heavy computations (Bézier fitting, FFT analysis, and Zebra 3 string generation) are offloaded entirely to the UI thread. The audio thread only performs raw sample capture and signaling.

## 2. Visualization & Analysis Stabilization

### A. Buffer Snapshotting (Anti-Tearing)
Every UI frame, the plugin takes a static "snapshot" of the audio ring buffer while holding a mutex. Analysis and rendering are performed on this snapshot, ensuring visual consistency and preventing "wave tearing" where the display jumps mid-draw.

### B. Precision Period Locking
Instead of relying solely on MIDI pitch, we perform **Measured Period Detection**:
- **Adaptive Auto-Center:** We calculate the local min/max to find the true "zero line," handling signals with DC offset.
- **Multi-Crossing Averaging:** We find the last 4 rising crossings and average the distance between them to find the true fundamental frequency of the audio.
- **Sub-sample Interpolation:** We find the exact fractional crossing point using linear interpolation.

### C. High-Order Cubic Resampling
For both the Oscilloscope and the FFT data extraction, we use a **4-point Catmull-Rom Cubic Sampler**. In the UI thread, this is "Hardened" with strict bounds clamping to prevent memory access panics.

## 3. The "Spectral Round Trip" Experiment

We conducted a "Round Trip" fidelity test:
1. **Zebra 3 Source:** Created a curve with 3 sharp spikes (partials 1, 5, and 7) at 100% amplitude.
2. **Capture:** Sampled the resulting audio in Curve Sampler using the **Direct-to-Spectrum** path.
3. **Pasting:** Pasted the resulting curve back into Zebra 3.

### Findings & Fidelity Limits:
- **Spike Preservation:** Higher frequency spikes (like partial 7) are now successfully preserved in the export.
- **The "Spill" Phenomenon:** Even with Blackman-Harris windowing and phase-locking, the captured curve shows energy leakage ("spill") into bins adjacent to the primary spikes.
- **Amplitude Scaling:** We implemented an **8.0/N scaling factor** (Real FFT compensation * Window Gain * Zebra convention) to ensure that a full-scale time-domain wave correctly produces a 1.0 magnitude spectral spike.

### Conclusion on Spectral Recovery:
The "perfectly zero" bins in Zebra 3's editor are an additive ideal. Recovering them from rendered audio is subject to the **Uncertainty Principle of Signal Analysis**. Any sub-sample phase jitter or slight mismatch between the extracted cycle and the true oscillator period results in spectral smearing. The current implementation uses **Blackman-Harris 4-term windowing** to achieve the best possible sidelobe suppression (-92dB theoretical), which is the current state-of-the-art for this type of recovery.

## 4. DAW Compatibility: macOS Bundle Requirements

Bitwig and Studio One are lenient about plugin bundle structure. Ableton Live requires two additional things or it crashes silently during plugin scan:

**`PkgInfo` file**
A file at `Contents/PkgInfo` containing the literal bytes `BNDL????` (no newline). Without it, Ableton rejects the bundle.

```bash
echo -n "BNDL????" > Contents/PkgInfo
```

**Ad-hoc deep code signature**
Ableton verifies that the bundle has a consistent code signature. Ad-hoc signing (no certificate) is sufficient:

```bash
codesign --force --sign - --deep <bundle>
```

Both steps are handled automatically in `deploy.sh` (local builds) and `.github/workflows/release.yml` (CI releases).

## 5. Geometry Optimization

### Interpolation Fitting
For the "Capture Geo" path, we switched to **64-segment Interpolation Fitting**. This ensures the Bézier curve passes **exactly** through its knot points, preserving sharp "jagged" edges (like Sawtooths) that are often smoothed out by traditional least-squares fitting.

### Collinear Simplify
To keep the final SVG/Clipboard strings concise:
- We implement a **Point-to-Line Distance** simplification pass.
- If a point is within `0.002` units of the straight line between its neighbors, it is pruned.
- **Linear Tangents:** For merged segments, we force handles to be at 1/3 and 2/3 positions, ensuring perfectly straight lines for geometric shapes.
