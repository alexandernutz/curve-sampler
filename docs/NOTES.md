# Notes & Design Observations

## Status / next review

Intended workflow is in place. Letting it rest to get perspective on:
- Is this something worth showing to people / useful to the Z3 community?
- What's the right next priority: polish, features, or more algorithmic work on the basics?

## Open questions / pending thoughts

**No domain state on the curve** *(decided)*
A curve is just a curve. There is no "current domain" — both transform buttons are always
available and applying the same transform twice is intentional (exploring round-trips,
double-spectralization, etc.). The `Domain` enum has been removed entirely.

**Spectrum-aware fitting when generating a spectral curve** *(decided)*
The G→S stepped mode now uses **per-harmonic steps**: each integer harmonic k gets its own
step covering x = [log₂(k)/10, log₂(k+1)/10] with that harmonic's exact FFT amplitude.
Steps narrow as frequency increases (log spacing), which is semantically exact for Z3's
left-edge sampling. Contrast: the old octave-band version gave all harmonics 4-7 the same
amplitude when only harmonic 4 was present — a bug, now fixed.

Adjacent steps with amplitude difference < 1% (SIMPLIFY_TOL = 0.01) are merged to avoid
emitting hundreds of redundant double-points for flat spectral regions. The result is a
minimal step curve: long silent bands collapse to one step, individual peaks stay distinct.
Capped at 128 harmonics (MAX_STEP_HARMONICS) with a noise floor of 0.2% (STEP_FLOOR).

The G→S smooth mode still uses a least-squares Bézier fit to a log-dense amplitude array.
That is a separate quality concern (the smooth fit may underrepresent narrow spectral peaks).

## Positioning relative to Zebra3

Fits conceptually as a **right-click context menu entry on the curve panel** — same
menu as Sine-o-matic and other curve-to-curve transforms, not a button in the editor.

## Why the conversion can't be mathematically clean

**The spectral representation is inherently discrete**
- Zebra3 samples the spectrum curve only at left-edge positions (x = log₂(k)/10).
  Everything between sample points is irrelevant — infinitely many curves sound identical.
- This might be an argument against using a continuous curve for spectral data at all;
  a plain list of harmonic amplitudes would be equally expressive and more honest.

**Bézier curves cannot exactly represent a sine wave**
- A cubic Bézier is a polynomial; sin(x) is transcendental. Even the round-trip
  (sine → spectrum → geometry → sine) is not exact.

## Possible Zebralette 3 bugs

**Pasted spectral curves sometimes inaudible in Zebralette 3 (not in Zebra 3)**
- Observed: curves that play correctly in Zebra 3 produce no sound in Zebralette 3 when pasted, especially in spectral mode.
- Hypothesis: Zebralette 3 may have a stricter parser, a different spectral sampling range, or a gain-staging difference vs. Zebra 3.
- Not yet isolated to a specific curve or format property. Worth testing with a known-good minimal curve (e.g. pure fundamental) to narrow down.

## Known limitations

**Phase loss in geometry↔spectrum conversion**
- Spectrum curve has no phase channel; only magnitudes are stored.
- IFFT reconstruction assumes zero-phase → always produces a symmetric waveform.
- Round-trip is lossy for asymmetric sources (e.g. sawtooth); largely inaudible for
  steady-state tones (Ohm's Acoustic Law). Accepted by design.
- Likely also why Zebra3 itself doesn't ship this conversion.
