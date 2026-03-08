use crate::bezier::{BezierCurve, ControlPoint};

/// Parse a Zebra3 clipboard curve string into a `BezierCurve`.
pub fn parse(input: &str) -> Result<BezierCurve, String> {
    let mut curve_id = 1u32;
    let mut morph_type = String::from("Peaks And Valleys");
    let mut points: Vec<ControlPoint> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.starts_with("//") || line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("Curve ID =") {
            if let Some(id_end) = rest.find("MorphType") {
                curve_id = rest[..id_end].trim().parse::<u32>().unwrap_or(1);
            }
            if let Some(s) = rest.find('\'') {
                if let Some(e) = rest[s + 1..].find('\'') {
                    morph_type = rest[s + 1..s + 1 + e].to_string();
                }
            }
            continue;
        }

        if line.starts_with("PX XY") {
            points.push(parse_point(line)?);
        }
    }

    if points.is_empty() {
        return Err("No control points found".into());
    }

    Ok(BezierCurve { points, curve_id, morph_type })
}

fn parse_point(line: &str) -> Result<ControlPoint, String> {
    let position = extract_pair(line, "XY")
        .ok_or_else(|| format!("Missing XY in: {line}"))?;
    let incoming = extract_pair(line, "IV");
    let outgoing = extract_pair(line, "OV");
    Ok(ControlPoint { position, incoming, outgoing })
}

fn extract_pair(line: &str, key: &str) -> Option<(f32, f32)> {
    let needle = format!("{key} = '");
    let start = line.find(&needle)? + needle.len();
    let end = line[start..].find('\'')? + start;
    let (a, b) = line[start..end].split_once('/')?;
    Some((parse_hex_f32(a)?, parse_hex_f32(b)?))
}

/// Decode an IEEE 754 single-precision float from an uppercase hex string.
/// The literal "0" is accepted as 0.0.
fn parse_hex_f32(s: &str) -> Option<f32> {
    let s = s.trim();
    if s == "0" {
        return Some(0.0);
    }
    Some(f32::from_bits(u32::from_str_radix(s, 16).ok()?))
}

/// Generate a Zebra3 clipboard format string from a `BezierCurve`.
pub fn generate(curve: &BezierCurve) -> String {
    let mut out = String::new();
    out.push_str("// u-he Bezier Curve\n");
    out.push_str("// Version 1.0\n");
    out.push_str(&format!(
        "Curve ID = {} MorphType = '{}'\n",
        curve.curve_id, curve.morph_type
    ));

    for pt in &curve.points {
        let mut line = format!("PX XY = '{}'", fmt_pair(pt.position));
        if let Some(iv) = pt.incoming {
            line.push_str(&format!(" IV = '{}'", fmt_pair(iv)));
        }
        if let Some(ov) = pt.outgoing {
            line.push_str(&format!(" OV = '{}'", fmt_pair(ov)));
        }
        out.push_str(&line);
        out.push('\n');
    }

    out
}

fn fmt_pair((x, y): (f32, f32)) -> String {
    format!("{}/{}", fmt_f32(x), fmt_f32(y))
}

fn fmt_f32(v: f32) -> String {
    if v == 0.0 {
        "0".to_string()
    } else {
        format!("{:08X}", v.to_bits())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "// u-he Bezier Curve\n\
        // Version 1.0\n\
        Curve ID = 1 MorphType = 'Peaks And Valleys'\n\
        PX XY = '0/3F006807' OV = '3EB65325/3E9BF21C' \n\
        PX XY = '3E4CCCCD/3F5CD30E' IV = '3F2FE8E0/3F24346D' OV = '3EBE0644/3EA36414' \n\
        PX XY = '3ECCCCCD/3E530DAC' IV = '3F20882E/3F3DD28A' OV = '3E8A67BA/3F1AC7AB' \n\
        PX XY = '3F19999A/3F33C948' IV = '3F164181/3F800000' OV = '3EBA0CFA/0' \n\
        PX XY = '3F19999A/3F006807' IV = '3F22F983/3F800000' OV = '3EAAAAAB/3EAAAAAB' \n\
        PX XY = '3F4CCCCD/3DFA2330' IV = '3F2AAAAB/3F2AAAAB' OV = '3EAAAAAB/3EAAAAAB' \n\
        PX XY = '3F800000/3F006807' IV = '3F2AAAAB/3F2AAAAB' \n";

    // Pure sine wave approximation — Zebra3's 5-point representation.
    // Uses 1/π and 2/π handle fractions (optimal for sine quarter-period).
    // Notable: points 2 and 3 share XY = (0.25, 1.0) — a "double point" that
    // decouples the handles on each side of the peak without a C1 constraint.
    // The zero-length segment P2→P3 is silently skipped by eval().
    const SINE: &str = "// u-he Bezier Curve\n\
        // Version 1.0\n\
        Curve ID = 1 MorphType = 'Peaks And Valleys'\n\
        PX XY = '0/3F000000' OV = '3EA2F980/3F000000' \n\
        PX XY = '3E800000/3F800000' IV = '3F22F980/3F800000' OV = '3EBA0CFA/0' \n\
        PX XY = '3E800000/3F800000' IV = '3F22F983/3F800000' OV = '3EBA0CFA/0' \n\
        PX XY = '3F400000/0' IV = '3F22F983/3F800000' OV = '3EBA0CF8/0' \n\
        PX XY = '3F800000/3F000000' IV = '3F2E8340/3F000000' \n";

    #[test]
    fn sine_parses() {
        let curve = parse(SINE).unwrap();
        assert_eq!(curve.points.len(), 5);
        // Double point: pts[1] and pts[2] share the same position
        let p1 = &curve.points[1];
        let p2 = &curve.points[2];
        assert!((p1.position.0 - p2.position.0).abs() < 1e-6);
        assert!((p1.position.1 - p2.position.1).abs() < 1e-6);
    }

    #[test]
    fn sine_eval_shape() {
        let curve = parse(SINE).unwrap();
        // At the quarter points the sine should be: 0→0.5, 0.25→1.0, 0.5→0.5, 0.75→0.0, 1.0→0.5
        assert!((curve.eval(0.0)  - 0.5).abs() < 1e-4, "x=0:   {}", curve.eval(0.0));
        assert!((curve.eval(0.25) - 1.0).abs() < 1e-4, "x=0.25:{}", curve.eval(0.25));
        assert!((curve.eval(0.75) - 0.0).abs() < 1e-4, "x=0.75:{}", curve.eval(0.75));
        assert!((curve.eval(1.0)  - 0.5).abs() < 1e-4, "x=1.0: {}", curve.eval(1.0));
    }

    #[test]
    fn parse_point_count() {
        let curve = parse(SAMPLE).unwrap();
        assert_eq!(curve.points.len(), 7);
        assert_eq!(curve.morph_type, "Peaks And Valleys");
    }

    #[test]
    fn first_point_values() {
        let curve = parse(SAMPLE).unwrap();
        let p0 = &curve.points[0];
        assert!((p0.position.0 - 0.0).abs() < 1e-5, "x={}", p0.position.0);
        assert!((p0.position.1 - 0.502).abs() < 1e-2, "y={}", p0.position.1);
        assert!(p0.incoming.is_none());
        assert!(p0.outgoing.is_some());
    }

    #[test]
    fn round_trip() {
        let curve = parse(SAMPLE).unwrap();
        let text = generate(&curve);
        let reparsed = parse(&text).unwrap();
        assert_eq!(reparsed.points.len(), curve.points.len());
        for (a, b) in curve.points.iter().zip(reparsed.points.iter()) {
            assert!((a.position.0 - b.position.0).abs() < 1e-6);
            assert!((a.position.1 - b.position.1).abs() < 1e-6);
        }
    }
}
