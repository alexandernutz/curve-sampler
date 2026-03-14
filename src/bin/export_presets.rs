//! Writes all preset curves to a timestamped subdirectory under `curves/`.
//! Each run produces a new folder so existing exports are never overwritten.
//! Run with:  cargo run --bin export_presets

use std::fs;

fn main() {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let dir = format!("curves/{ts}");
    fs::create_dir_all(&dir).expect("failed to create output directory");

    for &(name, gen) in zebra_curve_transform::waveforms::GEO_PRESETS {
        let text = gen();
        let path = format!("{dir}/geo_{}.txt", name.to_lowercase().replace('.', ""));
        fs::write(&path, &text).unwrap_or_else(|e| panic!("write {path}: {e}"));
        println!("wrote {path}");
    }

    for &(name, gen) in zebra_curve_transform::waveforms::SPEC_PRESETS {
        let text = gen();
        let path = format!("{dir}/spec_{}.txt", name.to_lowercase().replace('.', ""));
        fs::write(&path, &text).unwrap_or_else(|e| panic!("write {path}: {e}"));
        println!("wrote {path}");
    }

    println!("done → {dir}/");
}
