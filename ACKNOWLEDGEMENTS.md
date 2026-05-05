# Acknowledgements & Dependencies

## Direct Dependencies

### Curve Sampler (VST3/CLAP Plugin)
- **nih-plug** — Plugin framework (github: robbert-vdh/nih-plug) — ISC, except for VST3 bindings, those are under GPLv3
- **nih_plug_egui** — part of nih-plug, see above
- **rustfft** v6 — FFT/IFFT implementation (github: ejmahler/RustFFT) — MIT
- **parking_lot** v0.12 — Synchronization primitives (github: Amanieu/parking_lot) — MIT
- **log** v0.4 — Logging facade (github: rust-lang/log) — MIT

### Curve Core (DSP Library)
- **rustfft** v6 — FFT/IFFT implementation — MIT
- **log** v0.4 — Logging facade — MIT

### Curve Jumbler (Web App)
- **egui** v0.31 — Immediate-mode GUI library (github: emilk/egui) — MIT
- **wasm-bindgen** v0.2 — Rust-to-JavaScript bindings (github: wasm-bindgen/wasm-bindgen) — MIT
- **env_logger** v0.11 — Logging implementation (github: rust-cli/env_logger) — MIT
- **console_log** v1 — Browser console logging for WASM (github: iamcodemaker/console_log) — MIT
- **console_error_panic_hook** v0.1 — Better panic messages in browser console (github: rustwasm/console_error_panic_hook) — MIT

---

## Tools & Services

- **GitHub Actions** — CI/CD for builds and releases
- **Rust & Cargo** — Build system and package manager

---

## Transitive Dependencies

This project also relies on numerous transitive dependencies pulled in by the above crates. For a complete dependency tree, run:

```bash
cargo tree
```

---

## Special Thanks

- **u-he** for designing the Zebra/Zebralette 3 synthesizers and spline format that this tool interoperates with
- **Claude Code & Gemini-CLI** for assistance with development
