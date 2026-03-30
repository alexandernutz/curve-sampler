# Test Data

## Internal clipboard format (from Zebra3 "Copy Curve")

```
// u-he Bezier Curve
// Version 1.0
Curve ID = 1 MorphType = 'Peaks And Valleys'
PX XY = '0/3F006807' OV = '3EB65325/3E9BF21C' 
PX XY = '3E4CCCCD/3F5CD30E' IV = '3F2FE8E0/3F24346D' OV = '3EBE0644/3EA36414' 
PX XY = '3ECCCCCD/3E530DAC' IV = '3F20882E/3F3DD28A' OV = '3E8A67BA/3F1AC7AB' 
PX XY = '3F19999A/3F33C948' IV = '3F164181/3F800000' OV = '3EBA0CFA/0' 
PX XY = '3F19999A/3F006807' IV = '3F22F983/3F800000' OV = '3EAAAAAB/3EAAAAAB' 
PX XY = '3F4CCCCD/3DFA2330' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB' 
PX XY = '3F800000/3F006807' IV = '3F2AAAAB/3F2AAAAB' 
```

## Decoded values

| Point | X | Y | IV_X | IV_Y | OV_X | OV_Y |
|-------|-------|-------|---------|---------|---------|---------|
| 0 | 0.000 | 0.502 | — | — | 0.356 | 0.305 |
| 1 | 0.200 | 0.863 | 0.687 | 0.641 | 0.371 | 0.319 |
| 2 | 0.400 | 0.206 | 0.627 | 0.741 | 0.270 | 0.604 |
| 3 | 0.600 | 0.702 | 0.587 | 1.000 | 0.363 | 0.000 |
| 4 | 0.600 | 0.502 | 0.640 | 1.000 | 0.333 | 0.333 |
| 5 | 0.800 | 0.122 | 0.667 | 0.667 | 0.333 | 0.333 |
| 6 | 1.000 | 0.502 | 0.667 | 0.667 | — | — |

Notes:
- Y=0.5 is the zero line (center)
- First and last points share same Y (0.502) — cycle starts and ends at ~zero
- Points 3 and 4 share same X (0.600) — this creates a vertical discontinuity (line segment)
- X range: 0.0 to 1.0 (one complete cycle)

## Corresponding SVG (from Zebra3 "Copy Curve as SVG")

```xml
<?xml version="1.0" encoding="utf-8"?>
 <!-- Generator: u-he SVG support in Zebra3 -->
<svg version="1.2" baseProfile="tiny" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" x="0px" y="0px" viewBox="0 0 100 100" overflow="visible" xml:space="preserve">
<g id="Curve">
	<path fill="none" stroke="#00FF00" stroke-width="0.50" stroke-miterlimit="1" d="M0.00000,49.84127C7.12206,38.84563 13.74294,26.68531 20.00000,13.74046C27.42283,34.69051 32.54156,62.41868 40.00000,79.38931C45.40646,49.38963 51.73874,29.77100 60.00000,29.77100L60.00000,49.84127C66.66667,62.48960 73.33334,75.13792 80.00000,87.78625L100.00000,49.84127"/>
</g>
<g id="Segments">
	<path fill="none" stroke="#FF8800" stroke-width="0.50" stroke-miterlimit="1" d=""/>
</g>
</svg>
```

SVG coordinate mapping:
- SVG space: 100×100, y-axis inverted (0 at top)
- Internal space: 1×1, y-axis normal (0 at bottom)
- Conversion: svg_x = internal_x * 100, svg_y = (1 - internal_y) * 100

## Synthetic test curves

### Triangle wave — geometry interpretation

Load, select **Waveform**, press Play. Piecewise linear: 0 → +1 → 0 → −1 → 0.
Handle fractions (1/3, 1/3) / (2/3, 2/3) produce straight-line Bézier segments.

```
// u-he Bezier Curve
// Version 1.0
Curve ID = 1 MorphType = 'Peaks And Valleys'
PX XY = '0/3F000000' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3E800000/3F800000' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3F000000/3F000000' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3F400000/0' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3F800000/3F000000' IV = '3F2AAAAB/3F2AAAAB'
```

Decoded:

| Point | X    | Y    | meaning        |
|-------|------|------|----------------|
| 0     | 0.00 | 0.50 | zero crossing  |
| 1     | 0.25 | 1.00 | positive peak  |
| 2     | 0.50 | 0.50 | zero crossing  |
| 3     | 0.75 | 0.00 | negative trough|
| 4     | 1.00 | 0.50 | zero crossing  |

### Pure sine at the fundamental — spectral interpretation (Zebra3 reference)

This is what Zebra3 itself exports for a pure-fundamental spectral curve.
Load in Zebra3's spectral oscillator → plays as a sine at the base frequency.

```
// u-he Bezier Curve
// Version 1.0
Curve ID = 1 MorphType = 'Peaks And Valleys'
PX XY = '0/3F800000' OV = '3EAA5105/3EAA3D46'
PX XY = '3C23D70A/3BA3D700' IV = '3F2A7E40/3F2A7460' OV = '3EA73F2B/3EA14312'
PX XY = '3F800000/0' IV = '3F378657/3F346FE5'
```

Decoded:

| Point | X       | Y     | IV_X  | IV_Y  | OV_X  | OV_Y  |
|-------|---------|-------|-------|-------|-------|-------|
| 0     | 0.000   | 1.000 | —     | —     | 0.333 | 0.332 |
| 1     | 0.010   | 0.005 | 0.666 | 0.666 | 0.327 | 0.315 |
| 2     | 1.000   | 0.000 | 0.717 | 0.705 | —     | —     |

Shape: drops from Y=1 at x=0 to Y=0.005 at x=0.01, then smoothly decays to Y=0 at x=1.
The initial drop occupies the first 1% of the X axis.

**Confirmed: Zebra3 uses x = log₂(k) / 10, left-edge sampling.**

x = log₂(k) / 10    where k = harmonic number (1 = fundamental)

The scale constant 10 = log₂(1024), covering harmonics 1..1024.
Sampling rule: harmonic k's amplitude = curve Y value at x = log₂(k) / 10.
Any curve shape within the cell beyond the leftmost point does not affect that harmonic.

Evidence — control point positions in the curves below match harmonic positions exactly:
- P1.x = 0x3DCCCCCD = 0.10000 = log₂(2)/10  (harmonic 2)  ← float32-exact
- P2.x = 0x3E224CD7 = 0.15850 = log₂(3)/10  (harmonic 3)  ← float32-exact

### Second Zebra3 reference pure-sine — less steep slope (P1.x ≈ 0.0863)

Same spectral intent (pure sine at the fundamental). P1.x < log₂(2)/10 = 0.1, so harmonic 2
is sampled in the flat-zero second segment → pure sine.

```
// u-he Bezier Curve
// Version 1.0
Curve ID = 1 MorphType = 'Peaks And Valleys'
PX XY = '0/3F800000' OV = '3EAA5105/3EAA3D46'
PX XY = '3DB0A3D7/0' IV = '3F2A7E40/3F2A7460' OV = '3EA73F2B/3EA14312'
PX XY = '3F800000/0' IV = '3F378657/3F346FE5'
```

Decoded: P0=(0, 1.0), P1=(0.08625, 0), P2=(1.0, 0). Harmonic 2 at x=0.1 > P1.x → y=0. Pure sine ✓

### Boundary pure-sine — P1.x at harmonic-2 position (maximum width for pure sine)

P1.x = 0x3DCCCCCD = log₂(2)/10 = 0.1 exactly. Harmonic 2 sampled at its own left edge (P1.y=0).
Moving P1 to the right causes P1.y=0 to slip past x=0.1, and harmonic 2 acquires amplitude.

```
// u-he Bezier Curve
// Version 1.0
Curve ID = 1 MorphType = 'Peaks And Valleys'
PX XY = '0/3F800000' OV = '3EAA5105/3EAA3D46'
PX XY = '3DCCCCCD/0' IV = '3F2A7E40/3F2A7460' OV = '3EA73F2B/3EA14312'
PX XY = '3F800000/0' IV = '3F378657/3F346FE5'
```

### Harmonics 1 and 2 at full amplitude

Control points placed exactly at harmonic 2 (P1.x=0.1, P1.y=1.0) and harmonic 3 (P2.x=0.1585,
P2.y=0). Left-edge sampling reads Y=1 at x=0 (h1) and x=0.1 (h2), Y=0 at x=0.1585 (h3+).

```
// u-he Bezier Curve
// Version 1.0
Curve ID = 1 MorphType = 'Peaks And Valleys'
PX XY = '0/3F800000' OV = '3EA4D9A6/3E848243'
PX XY = '3DCCCCCD/3F800000' IV = '3F375916/3F278D33' OV = '3EB43703/3EAE6644'
PX XY = '3E224CD7/0' IV = '3F2FC0F9/3F2CC9AF' OV = '3EA73F2B/3EA14312'
PX XY = '3F800000/0' IV = '3F378657/3F346FE5'
```

Decoded:

| Point | X      | Y     | Meaning                        |
|-------|--------|-------|--------------------------------|
| 0     | 0.0000 | 1.000 | harmonic 1 sample (x=0) = 1.0 |
| 1     | 0.1000 | 1.000 | harmonic 2 sample (x=0.1) = 1.0 |
| 2     | 0.1585 | 0.000 | harmonic 3 sample (x=0.1585) = 0 |
| 3     | 1.0000 | 0.000 | all higher harmonics = 0 |

P1.x = log₂(2)/10, P2.x = log₂(3)/10 — both float32-exact to within 5 ULP.

### Single-harmonic spectral curves — confirmed 2026-03-30

All three confirmed working in Zebra3: each plays as a pure sine at the indicated harmonic.
Structure: flat-zero segment up to the target harmonic's x position, straight rise to y=1 at
that position, straight drop to y=0 at the next harmonic's x position, flat zero to end.
Handles are (1/3, 1/3) / (2/3, 2/3) throughout (straight-line Béziers).

Hex positions: h(k) = log₂(k)/10 → h(2)=3DCCCCCD, h(3)=3E224CD7, h(4)=3E4CCCCD, h(5)=3E6DC3F3.

**Harmonic 2 only:**
```
// u-he Bezier Curve
// Version 1.0
Curve ID = 1 MorphType = 'Peaks And Valleys'
PX XY = '0/0' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3DCCCCCD/3F800000' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3E224CD7/0' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3F800000/0' IV = '3F2AAAAB/3F2AAAAB'
```

**Harmonic 3 only:**
```
// u-he Bezier Curve
// Version 1.0
Curve ID = 1 MorphType = 'Peaks And Valleys'
PX XY = '0/0' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3DCCCCCD/0' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3E224CD7/3F800000' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3E4CCCCD/0' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3F800000/0' IV = '3F2AAAAB/3F2AAAAB'
```

**Harmonic 4 only:**
```
// u-he Bezier Curve
// Version 1.0
Curve ID = 1 MorphType = 'Peaks And Valleys'
PX XY = '0/0' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3E224CD7/0' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3E4CCCCD/3F800000' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3E6DC3F3/0' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3F800000/0' IV = '3F2AAAAB/3F2AAAAB'
```
