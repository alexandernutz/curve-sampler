# Curve Transforms — Ideas & Notes

## Zebra(lette) 3 built-in transforms (for reference / avoid duplicating)
- Flip X — reverse in time
- Flip Y — invert amplitude
- Simplify — reduce point count
- Beautify — make sharp edges curvy
- Sine-o-matic — make sine-like
- Distribute on X — even-space all points
- Line-up — put points on a straight line
- Clean-up — erase redundant points

## Ideas for Curve Jumbler

### Geometry-domain
- **Flip Y** — map y → 1−y (invert amplitude). Mirror of Z3's flip y, genuinely useful.
- **Flip X** — reverse waveform in time. Sawtooth ↔ reverse-sawtooth.
- **Rectify** — fold negative half up: y → |y − 0.5| + 0.5. Doubles frequency, sounds like FM.
- **Fold** — wavefolder: reflect values that exceed [0,1] back inward.
- **Quantize** — step-quantize Y to N levels (bit-crush character).
- **DC Remove** — shift waveform so its mean sits at y=0.5.
- **Geo Jitter** — add small random offsets to control point Y values;
  starts from the current curve rather than a blank slate.

### Spectrum-domain
- **Tilt** — apply a linear gain slope across harmonics (boost lows / cut highs or vice versa).
- **Comb** — zero out every other harmonic (or every Nth).
- **Harmonic shift** — shift harmonic content up by N (multiply harmonic indices).
- **Spec Jitter** — randomize each harmonic amplitude slightly around its current value;
  spectral analogue of Geo Jitter.

### Meta / cross-domain
- **Harmonize** — shortcut for G→S→G in one click (symmetrizes, removes phase).
- **Round-trip N times** — apply the same pair of transforms repeatedly and watch it converge.

## Design notes
- "Jitter" variants are the spectral/geometric "jumble from current curve" the user asked for.
- Prefer transforms that feel distinct from Z3's list; overlap a little for convenience
  (Flip X/Y are too useful to leave out) but lean toward things Z3 doesn't do.
- Each transform should work on whatever curve is currently loaded, same as the domain
  transforms — no domain state, apply anything to anything.
