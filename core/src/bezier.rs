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

    pub fn simplify(&mut self, tolerance: f32) {
        if self.points.len() <= 2 {
            return;
        }

        let mut new_points = Vec::new();
        new_points.push(self.points[0].clone());

        let mut i = 0;
        while i < self.points.len() - 1 {
            let mut j = i + 1;
            while j < self.points.len() - 1 {
                let p_prev = self.points[i].position;
                let p_curr = self.points[j].position;
                let p_next = self.points[j + 1].position;

                // Distance from point p_curr to line (p_prev, p_next)
                let dx = p_next.0 - p_prev.0;
                let dy = p_next.1 - p_prev.1;
                let len_sq = dx * dx + dy * dy;
                
                let dist = if len_sq < 1e-9 {
                    ((p_curr.0 - p_prev.0).powi(2) + (p_curr.1 - p_prev.1).powi(2)).sqrt()
                } else {
                    let t = ((p_curr.0 - p_prev.0) * dx + (p_curr.1 - p_prev.1) * dy) / len_sq;
                    let t = t.clamp(0.0, 1.0);
                    let proj_x = p_prev.0 + t * dx;
                    let proj_y = p_prev.1 + t * dy;
                    ((p_curr.0 - proj_x).powi(2) + (p_curr.1 - proj_y).powi(2)).sqrt()
                };

                if dist < tolerance {
                    j += 1;
                } else {
                    break;
                }
            }
            new_points.push(self.points[j].clone());
            i = j;
        }

        // Fix tangents for simplified segments to be perfectly linear
        for k in 0..new_points.len() - 1 {
            // If the distance between points is large but segments were merged,
            // force linear handles (1/3 and 2/3) to ensure a straight line.
            new_points[k].outgoing = Some((1.0 / 3.0, 1.0 / 3.0));
            if k + 1 < new_points.len() {
                new_points[k+1].incoming = Some((2.0 / 3.0, 2.0 / 3.0));
            }
        }

        self.points = new_points;
    }
}

/// Fits a piecewise cubic Bézier to `samples` using adaptive knot placement.
///
/// This version concentrates points where the waveform is changing most rapidly
/// (e.g. sharp jumps in a sawtooth), preserving transients and jagged edges.
pub fn fit_bezier_adaptive(
    samples: &[f32],
    max_segments: usize,
    curve_id: u32,
    morph_type: &str,
) -> BezierCurve {
    let ns = samples.len();
    if ns < 2 {
        return BezierCurve { points: vec![], curve_id, morph_type: morph_type.to_string() };
    }

    // 1. Calculate Importance density map
    // We look at the absolute first derivative (slope) and second derivative (curvature)
    let mut importance = vec![0.05f32; ns]; // Baseline importance for even distribution
    for i in 1..ns - 1 {
        let slope = (samples[i] - samples[i-1]).abs();
        let curvature = (samples[i+1] - 2.0*samples[i] + samples[i-1]).abs();
        // Emphasize slope heavily for jumps (saw/square)
        importance[i] += slope * 5.0 + curvature * 2.0;
    }

    // 2. Cumulative importance
    let mut cumulative = vec![0.0f32; ns];
    let mut total = 0.0;
    for i in 0..ns {
        total += importance[i];
        cumulative[i] = total;
    }

    // 3. Pick knot indices based on equal-importance intervals
    let n_knots = (max_segments + 1).min(ns);
    let mut knot_indices = Vec::with_capacity(n_knots);
    knot_indices.push(0);
    
    for k in 1..n_knots-1 {
        let target = (k as f32 / (n_knots - 1) as f32) * total;
        let idx = match cumulative.binary_search_by(|v| v.partial_cmp(&target).unwrap()) {
            Ok(i) => i,
            Err(i) => i.min(ns - 1),
        };
        if !knot_indices.contains(&idx) {
            knot_indices.push(idx);
        }
    }
    if !knot_indices.contains(&(ns - 1)) {
        knot_indices.push(ns - 1);
    }
    knot_indices.sort();

    // 4. Build control points with Catmull-Rom tangents
    let mut points = Vec::with_capacity(knot_indices.len());
    for i in 0..knot_indices.len() {
        let idx = knot_indices[i];
        let x = idx as f32 / (ns - 1) as f32;
        let y = samples[idx];

        // Slope calculation for tangents
        let slope = if i == 0 {
            (samples[knot_indices[1]] - samples[0]) / (knot_indices[1] as f32 / (ns - 1) as f32)
        } else if i == knot_indices.len() - 1 {
            let prev_idx = knot_indices[i - 1];
            (samples[ns - 1] - samples[prev_idx]) / ((ns - 1 - prev_idx) as f32 / (ns - 1) as f32)
        } else {
            let next_idx = knot_indices[i + 1];
            let prev_idx = knot_indices[i - 1];
            (samples[next_idx] - samples[prev_idx]) / ((next_idx - prev_idx) as f32 / (ns - 1) as f32)
        };

        let outgoing = if i < knot_indices.len() - 1 {
            let next_idx = knot_indices[i + 1];
            let dx = (next_idx - idx) as f32 / (ns - 1) as f32;
            let dy_seg = samples[next_idx] - y;
            let handle_y = y + slope * dx / 3.0;
            let ty = if dy_seg.abs() > 1e-6 {
                ((handle_y - y) / dy_seg).clamp(-4.0, 5.0)
            } else {
                1.0 / 3.0
            };
            Some((1.0_f32 / 3.0, ty))
        } else {
            None
        };

        let incoming = if i > 0 {
            let prev_idx = knot_indices[i - 1];
            let dx = (idx - prev_idx) as f32 / (ns - 1) as f32;
            let dy_seg = y - samples[prev_idx];
            let handle_y = y - slope * dx / 3.0;
            let ty = if dy_seg.abs() > 1e-6 {
                ((handle_y - samples[prev_idx]) / dy_seg).clamp(-4.0, 5.0)
            } else {
                2.0 / 3.0
            };
            Some((2.0_f32 / 3.0, ty))
        } else {
            None
        };

        points.push(ControlPoint { position: (x, y), incoming, outgoing });
    }

    BezierCurve { points, curve_id, morph_type: morph_type.to_string() }
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

/// Fit a piecewise cubic Bézier to `samples` by interpolating through knot positions.
///
/// Knot Y values are read directly from `samples` at evenly-spaced x positions,
/// then Catmull-Rom tangents are applied. This preserves the full variation of the
/// input (including random/chaotic data) at the cost of not minimizing fit error
/// between knots. Use for geometry (time-domain) curves where detail matters.
pub fn fit_bezier_interp(
    samples: &[f32],
    num_segments: usize,
    curve_id: u32,
    morph_type: &str,
) -> BezierCurve {
    let n = num_segments.max(1);
    let num_pts = n + 1;
    let ns = samples.len();

    let xs: Vec<f32> = (0..num_pts).map(|i| i as f32 / n as f32).collect();

    // Sample at each knot x via linear interpolation into samples
    let ys: Vec<f32> = xs
        .iter()
        .map(|&x| {
            let fi = x * (ns - 1) as f32;
            let lo = (fi as usize).min(ns - 2);
            let t = fi - lo as f32;
            samples[lo] * (1.0 - t) + samples[lo + 1] * t
        })
        .collect();

    // Catmull-Rom slopes at each knot
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

/// Fit a piecewise cubic Bézier to `samples` (evenly spaced over [0, 1]).
/// Produces `num_segments` segments (num_segments + 1 control points).
///
/// Uses least-squares optimisation over knot Y positions with Catmull-Rom tangents.
/// With Catmull-Rom tangents, the curve value at any x is a linear function of the
/// knot Y values (basis functions derived below), so the optimal Y values are the
/// solution to a small (num_pts × num_pts) normal-equations system.
///
/// Catmull-Rom basis for segment i (knots i−1, i, i+1, i+2), local parameter t:
///   φ[i−1] = −s²t/2
///   φ[i]   = s³ + 3s²t + st²/2   (interior)
///   φ[i+1] = s²t/2 + 3st² + t³   (interior)
///   φ[i+2] = −st²/2
/// with modified coefficients at the first and last segments where one-sided slopes
/// replace central differences (see inline comments).
///
/// Handle storage uses Zebra3's fractional format: (tx, ty) where
///   handle = P_start + (tx, ty) × (P_end − P_start)
pub fn fit_bezier(
    samples: &[f32],
    num_segments: usize,
    curve_id: u32,
    morph_type: &str,
) -> BezierCurve {
    let n = num_segments.max(1);
    let num_pts = n + 1;
    let ns = samples.len();

    // ── Build normal equations (AᵀA) y = Aᵀb ──────────────────────────────────
    // A[j, k] = contribution of knot y_k to the curve value at sample position x_j.
    let mut ata = vec![vec![0.0f64; num_pts]; num_pts];
    let mut atb = vec![0.0f64; num_pts];

    const ABSENT: usize = usize::MAX;

    for j in 0..ns {
        // Map sample index to curve x in [0, 1]
        let x = j as f64 / (ns - 1).max(1) as f64;
        // Segment index and local parameter
        let fi = (x * n as f64).min(n as f64 - 1e-9);
        let i = fi as usize; // segment [0, n−1]
        let t = fi - i as f64;
        let s = 1.0 - t;
        let (s2, s3) = (s * s, s * s * s);
        let (t2, t3) = (t * t, t * t * t);

        // (knot_index, coefficient) for up to 4 contributing knots.
        // ABSENT entries are skipped.
        //
        // Derivation sketch (interior segment, 1 ≤ i ≤ n−2):
        //   B1_y = y_i  + (y_{i+1} − y_{i−1}) / 6        [central diff]
        //   B2_y = y_{i+1} − (y_{i+2} − y_i) / 6         [central diff]
        //   B(t) = s³·y_i + 3s²t·B1_y + 3st²·B2_y + t³·y_{i+1}
        //   → φ[i−1] = −s²t/2,  φ[i] = s³+3s²t+st²/2,
        //     φ[i+1] = s²t/2+3st²+t³,  φ[i+2] = −st²/2
        //
        // First segment (i=0, n≥2): one-sided slope at knot 0 changes B1_y:
        //   B1_y = (2/3)y_0 + (1/3)y_1  →  φ[0] = s³+2s²t+st²/2, φ[1] = s²t+3st²+t³, φ[2] = −st²/2
        //
        // Last segment (i=n−1, n≥2): one-sided slope at knot n changes B2_y:
        //   B2_y = (1/3)y_{n−1} + (2/3)y_n  →  φ[n−2] = −s²t/2, φ[n−1] = s³+3s²t+st², φ[n] = s²t/2+2st²+t³
        //
        // Single segment (n=1): one-sided at both ends:
        //   φ[0] = s³+2s²t+st²,  φ[1] = s²t+2st²+t³
        let basis: [(usize, f64); 4] = if n == 1 {
            [(0,      s3 + 2.0*s2*t + s*t2),
             (1,      s2*t + 2.0*s*t2 + t3),
             (ABSENT, 0.0),
             (ABSENT, 0.0)]
        } else if i == 0 {
            // First segment
            [(0,      s3 + 2.0*s2*t + s*t2/2.0),
             (1,      s2*t + 3.0*s*t2 + t3),
             (2,      -s*t2/2.0),
             (ABSENT, 0.0)]
        } else if i == n - 1 {
            // Last segment
            [(i - 1,  -s2*t/2.0),
             (i,       s3 + 3.0*s2*t + s*t2),
             (i + 1,   s2*t/2.0 + 2.0*s*t2 + t3),
             (ABSENT,  0.0)]
        } else {
            // Interior segment
            [(i - 1,  -s2*t/2.0),
             (i,       s3 + 3.0*s2*t + s*t2/2.0),
             (i + 1,   s2*t/2.0 + 3.0*s*t2 + t3),
             (i + 2,  -s*t2/2.0)]
        };

        let b = samples[j] as f64;
        for &(ki, ci) in &basis {
            if ki == ABSENT { continue; }
            atb[ki] += ci * b;
            for &(kj, cj) in &basis {
                if kj == ABSENT { continue; }
                ata[ki][kj] += ci * cj;
            }
        }
    }

    // ── Solve AᵀA y = Aᵀb ─────────────────────────────────────────────────────
    let ys_f64 = gauss_solve(ata, atb);
    // Clamp to [0, 1] (valid Bézier Y range; optimizer may push slightly outside)
    let ys: Vec<f32> = ys_f64.iter().map(|&y| (y as f32).clamp(0.0, 1.0)).collect();

    // ── Build control points with Catmull-Rom handles ──────────────────────────
    let xs: Vec<f32> = (0..num_pts).map(|i| i as f32 / n as f32).collect();

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

/// Gaussian elimination with partial pivoting. Solves ax = b, returns x.
fn gauss_solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();

    for col in 0..n {
        // Partial pivoting: swap in the row with the largest pivot element
        let pivot = (col..n)
            .max_by(|&i, &j| {
                a[i][col].abs().partial_cmp(&a[j][col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(col);
        a.swap(col, pivot);
        b.swap(col, pivot);

        if a[col][col].abs() < 1e-12 {
            continue; // near-singular column; leave as zero
        }

        for row in col + 1..n {
            let factor = a[row][col] / a[col][col];
            for k in col..n {
                let v = a[col][k] * factor;
                a[row][k] -= v;
            }
            let v = b[col] * factor;
            b[row] -= v;
        }
    }

    // Back substitution
    let mut x = vec![0.0f64; n];
    for i in (0..n).rev() {
        if a[i][i].abs() < 1e-12 {
            continue;
        }
        let sum: f64 = (i + 1..n).map(|j| a[i][j] * x[j]).sum();
        x[i] = (b[i] - sum) / a[i][i];
    }
    x
}
