# Curve Sampler: Technical Implementation Notes

This document outlines the technical architecture and DSP techniques implemented in the **Curve Sampler** plugin.

## 1. Threading & Memory Model

The plugin follows a strict decoupling between the **Real-time Audio Thread** and the **UI Thread** to ensure DAW stability and responsive visualizations.

- **Shared State:** All shared data is wrapped in `Arc` (Atomic Reference Counting).
- **Safe Mutexes:** Using `parking_lot::Mutex` for efficient, non-poisoning locking of the audio ring buffer and the captured Bézier curves.
- **Lockless Hints:** Pitch (`current_freq`), Sample Rate, and Buffer Write Index are shared via `AtomicU32` and `AtomicUsize` for zero-latency communication from Audio to UI.
- **Decoupled Computation:** The computationally expensive Bézier least-squares fitting (`fit_bezier`) and Zebra 3 string generation are offloaded entirely to the UI thread. The audio thread only performs raw sample capture and signaling.

## 2. Visualization Stabilization Techniques

Achieving a "locked" oscilloscope and stationary spectrum analyzer required several layers of signal processing.

### A. Anti-Tearing (Buffer Snapshotting)
To prevent "jumps" caused by the audio thread updating the ring buffer while the UI thread is reading it, we implement a **Snapshotting** strategy:
- Every UI frame, the UI thread holds the mutex just long enough to copy the most recent 8192 samples into a local `snapshot` vector.
- All subsequent analysis (LPF, crossing detection, FFT) and rendering are performed on this static snapshot, ensuring visual consistency within a single frame.

### B. LPF-Based Triggering (Fundamental Locking)
To prevent high-frequency harmonics from causing the trigger to jump between multiple points in a cycle:
- A simple **1-pole Low-Pass Filter** (EMA filter) is applied to the snapshot specifically for analysis.
- The trigger search (zero-crossing) runs on this filtered signal, effectively locking onto the fundamental frequency while ignoring the "wiggles" of harmonics.

### C. Adaptive Auto-Centering & Hysteresis
- The plugin calculates the local min/max of the snapshot every frame to find the true "center line."
- A **Schmidt Trigger** logic with 15% hysteresis is used: the trigger only "arms" when the signal drops significantly below center and only "fires" on a rising edge crossing. This eliminates jitter from low-level noise.

### D. Sub-sample Accurate Phase Correction
- The exact crossing point is calculated using **Linear Interpolation** between the two samples surrounding the zero-crossing.
- This fractional offset is used when resampling the waveform for display, ensuring the wave stays perfectly still at a sub-pixel level.

### E. Time-Base Smoothing (EMA)
- The fundamental period ($T$) is measured by calculating the distance between successive crossings.
- This measured period is validated against the MIDI pitch hint to avoid octave errors.
- The period used for horizontal scaling is smoothed using an **Exponential Moving Average**, preventing "visual vibration" when the incoming pitch is slightly unstable.

## 3. Spectral Analysis & Rendering

### A. Period-Locked FFT
Standard fixed-window FFTs suffer from **Spectral Leakage**, which causes magnitudes to "twitch" and "smear." 
- We resample exactly **one full period** (as measured by our period detection) into a power-of-two FFT buffer (1024 samples).
- This aligns every harmonic frequency exactly with an FFT bin center ($k=1, 2, 3...$), resulting in stationary, distinct spectral peaks.

### B. Boxy Rendering (Zebra 3 Compatibility)
- Each harmonic magnitude is rendered as a distinct rectangle ("box").
- The X-axis uses the Zebra 3 logarithmic scale: $x = \log_2(k) / 10$.
- This results in the characteristic "wider boxes on the left, thinner on the right" look consistent with the Zebra 3 spectral editor.
- Points are exported with explicit **Linear Tangents** to preserve the sharp boxy shape when pasted into the synth.

## 4. Curve Optimization

### Collinear Simplification
When fitting a Bézier curve to a geometric shape (like a Sawtooth or Square), least-squares fitting often produces redundant segments on the linear parts.
- We implemented a **Collinear Simplify** pass.
- It calculates the cross-product of adjacent segments; if the change in slope is below a tolerance (`0.005`), the intermediate control point is removed and the segments are merged.
- This keeps the final exported SVG/Zebra format string concise and "editable."
