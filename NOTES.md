# Notes & Design Observations

## Positioning relative to Zebra3

Fits conceptually as a **right-click context menu entry on the curve panel** — same
menu as Sine-o-matic and other curve-to-curve transforms, not a button in the editor.

## Known limitations

**Phase loss in geometry↔spectrum conversion**
- Spectrum curve has no phase channel; only magnitudes are stored.
- IFFT reconstruction assumes zero-phase → always produces a symmetric waveform.
- Round-trip is lossy for asymmetric sources (e.g. sawtooth); largely inaudible for
  steady-state tones (Ohm's Acoustic Law). Accepted by design.
- Likely also why Zebra3 itself doesn't ship this conversion.
