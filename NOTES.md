# Notes & Design Observations

## Status / next review

Intended workflow is in place. Letting it rest to get perspective on:
- Is this something worth showing to people / useful to the Z3 community?
- What's the right next priority: polish, features, or more algorithmic work on the basics?

## Open questions / pending thoughts

**No domain state on the curve**
The transforms are just curve-to-curve operations; there's no tag saying "this is geometry"
or "this is spectrum". The App tracks `domain` as a display hint but doesn't gate anything.
This is intentional (experimenting with double-applying etc. is the point), but it means the
UI gives no warning if you apply the wrong transform.

**Spectrum-aware fitting when generating a spectral curve**
Zebra3's own spectral curves use an "L-shape": a steep drop to y=0, then a flat line.
This guarantees that left-edge harmonic samples land on the flat zero portion.
Our Catmull-Rom fit produces smooth curves instead — which might accidentally put nonzero
values at harmonic positions that should be silent.
Worth considering: when fitting a spectrum output curve, should we snap control points to
step-function-like shapes (honoring the discrete-cell semantics) rather than smooth
interpolation? Not obvious this is the right call — smooth curves are also valid and may
be what the user wants. Needs more thought.

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

## Known limitations

**Phase loss in geometry↔spectrum conversion**
- Spectrum curve has no phase channel; only magnitudes are stored.
- IFFT reconstruction assumes zero-phase → always produces a symmetric waveform.
- Round-trip is lossy for asymmetric sources (e.g. sawtooth); largely inaudible for
  steady-state tones (Ohm's Acoustic Law). Accepted by design.
- Likely also why Zebra3 itself doesn't ship this conversion.
