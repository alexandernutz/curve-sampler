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

### Pure sine at the fundamental — spectral interpretation

Load, select **Spectrum**, press Play. Only harmonic 1 carries energy → pure cosine output.

The spike at x = 1/1024 ≈ 0.001 is analytically exact:
`1/1024 = 2^(-10) = 0x3A800000`, `2/1024 = 2^(-9) = 0x3B000000`.
The curve evaluates to exactly 1.0 at x = 1/1024 and 0 everywhere else.

```
// u-he Bezier Curve
// Version 1.0
Curve ID = 1 MorphType = 'Peaks And Valleys'
PX XY = '0/0' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3A800000/3F800000' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3B000000/0' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB'
PX XY = '3F800000/0' IV = '3F2AAAAB/3F2AAAAB'
```

Note: the spike is invisible in the visualiser (it occupies 0.1% of the X axis).
