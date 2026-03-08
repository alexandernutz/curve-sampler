/// A single control point on the Bézier curve.
#[derive(Debug, Clone)]
pub struct ControlPoint {
    /// Position in 0–1 space (x = phase, y = amplitude where 0.5 = zero)
    pub position: (f32, f32),
    /// Incoming tangent handle (IV), absent on first point
    pub incoming: Option<(f32, f32)>,
    /// Outgoing tangent handle (OV), absent on last point
    pub outgoing: Option<(f32, f32)>,
}

/// A complete piecewise cubic Bézier curve (one waveform cycle).
#[derive(Debug, Clone)]
pub struct BezierCurve {
    pub points: Vec<ControlPoint>,
    pub curve_id: u32,
    pub morph_type: String,
}

impl BezierCurve {
    /// Sample the curve into `n` evenly spaced values over [0, 1].
    /// Returns y values; y=0.5 is zero amplitude.
    pub fn sample(&self, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| self.eval(i as f32 / n as f32))
            .collect()
    }

    /// Evaluate the curve at x-parameter `t` in [0, 1].
    pub fn eval(&self, t: f32) -> f32 {
        let pts = &self.points;
        if pts.is_empty() {
            return 0.5;
        }
        if pts.len() == 1 {
            return pts[0].position.1;
        }

        // Find the segment whose x-range contains t
        let n = pts.len();
        let mut seg = n - 2;
        for i in 0..n - 1 {
            if t <= pts[i + 1].position.0 {
                seg = i;
                break;
            }
        }

        let p0 = pts[seg].position;
        let p3 = pts[seg + 1].position;

        // OV/IV are fractional parameters: (tx, ty) in [0,1] where 0=segment start, 1=segment end,
        // independently in X and Y. Convert to absolute positions before evaluating.
        let frac_to_abs = |(tx, ty): (f32, f32)| {
            (p0.0 + tx * (p3.0 - p0.0), p0.1 + ty * (p3.1 - p0.1))
        };
        let p1 = pts[seg].outgoing.map(frac_to_abs).unwrap_or(lerp_pt(p0, p3, 1.0 / 3.0));
        let p2 = pts[seg + 1].incoming.map(frac_to_abs).unwrap_or(lerp_pt(p0, p3, 2.0 / 3.0));

        // Reparametrize: find cubic parameter u such that x(u) ≈ t
        let u = find_u_for_x(p0.0, p1.0, p2.0, p3.0, t);
        cubic_bezier(p0.1, p1.1, p2.1, p3.1, u)
    }
}

fn cubic_bezier(a: f32, b: f32, c: f32, d: f32, t: f32) -> f32 {
    let s = 1.0 - t;
    s * s * s * a + 3.0 * s * s * t * b + 3.0 * s * t * t * c + t * t * t * d
}

/// Binary-search for the Bézier parameter u where x(u) ≈ target_x.
fn find_u_for_x(x0: f32, x1: f32, x2: f32, x3: f32, target: f32) -> f32 {
    let mut lo = 0.0f32;
    let mut hi = 1.0f32;
    for _ in 0..32 {
        let mid = (lo + hi) * 0.5;
        if cubic_bezier(x0, x1, x2, x3, mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) * 0.5
}

fn lerp_pt(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Fit a piecewise cubic Bézier to `samples` (evenly spaced over [0, 1]).
/// Produces `num_segments` segments (num_segments + 1 control points).
///
/// Uses Catmull-Rom tangents: each knot's slope is estimated from its neighbours
/// (central difference for interior points, one-sided for endpoints). Handles are
/// placed 1/3 of the segment span away in X, with Y derived from the slope.
///
/// Handle storage uses Zebra3's fractional format: (tx, ty) where
///   handle = P_start + (tx, ty) * (P_end - P_start)
/// For nearly-flat segments (|Δy| < ε) the fractional Y is ill-defined, so
/// we fall back to ty = 1/3 or 2/3, effectively keeping those handles flat.
pub fn fit_bezier(
    samples: &[f32],
    num_segments: usize,
    curve_id: u32,
    morph_type: &str,
) -> BezierCurve {
    let n = num_segments.max(1);
    let num_pts = n + 1;

    // Linear interpolation into samples array.
    let lookup = |x: f32| -> f32 {
        let t = (x * (samples.len() - 1) as f32).clamp(0.0, (samples.len() - 1) as f32);
        let lo = t.floor() as usize;
        let hi = (lo + 1).min(samples.len() - 1);
        let frac = t - t.floor();
        samples[lo] * (1.0 - frac) + samples[hi] * frac
    };

    let xs: Vec<f32> = (0..num_pts).map(|i| i as f32 / n as f32).collect();
    let ys: Vec<f32> = xs.iter().map(|&x| lookup(x)).collect();

    // Catmull-Rom slopes (dy/dx) at each knot.
    let slopes: Vec<f32> = (0..num_pts)
        .map(|i| {
            if i == 0 {
                (ys[1] - ys[0]) / (xs[1] - xs[0])
            } else if i == num_pts - 1 {
                (ys[i] - ys[i - 1]) / (xs[i] - xs[i - 1])
            } else {
                (ys[i + 1] - ys[i - 1]) / (xs[i + 1] - xs[i - 1])
            }
        })
        .collect();

    let mut points = Vec::with_capacity(num_pts);
    for i in 0..num_pts {
        // Outgoing handle for segment i → i+1.
        let outgoing = if i < n {
            let dx = xs[i + 1] - xs[i];
            let dy_seg = ys[i + 1] - ys[i];
            let handle_y = ys[i] + slopes[i] * dx / 3.0;
            let ty = if dy_seg.abs() > 1e-6 {
                ((handle_y - ys[i]) / dy_seg).clamp(-4.0, 5.0)
            } else {
                1.0 / 3.0
            };
            Some((1.0_f32 / 3.0, ty))
        } else {
            None
        };

        // Incoming handle for segment i-1 → i.
        let incoming = if i > 0 {
            let dx = xs[i] - xs[i - 1];
            let dy_seg = ys[i] - ys[i - 1];
            let handle_y = ys[i] - slopes[i] * dx / 3.0;
            let ty = if dy_seg.abs() > 1e-6 {
                ((handle_y - ys[i - 1]) / dy_seg).clamp(-4.0, 5.0)
            } else {
                2.0 / 3.0
            };
            Some((2.0_f32 / 3.0, ty))
        } else {
            None
        };

        points.push(ControlPoint { position: (xs[i], ys[i]), incoming, outgoing });
    }

    BezierCurve { points, curve_id, morph_type: morph_type.to_string() }
}
