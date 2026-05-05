# Acknowledgements & Dependencies

## Direct Dependencies

### Curve Sampler (VST3/CLAP Plugin)
- **nih-plug** — Plugin framework (github: robbert-vdh/nih-plug) — 
- **nih_plug_egui** — egui integration for nih-plug (github: robbert-vdh/nih-plug) — 
- **rustfft** v6 — FFT/IFFT implementation — 
- **parking_lot** v0.12 — Synchronization primitives — 
- **log** v0.4 — Logging facade — 

### Curve Core (DSP Library)
- **rustfft** v6 — FFT/IFFT implementation — 
- **log** v0.4 — Logging facade — 

### Curve Jumbler (Web App)
- **eframe** v0.29 — egui framework for native + web — 
- **egui** v0.29 — Immediate-mode GUI library — 
- **wasm-bindgen** v0.2 — Rust-to-JavaScript bindings — 
- **web-sys** v0.3 — Web API bindings — 
- **env_logger** v0.11 — Logging implementation — 
- **console_log** v1 — Browser console logging for WASM — 
- **console_error_panic_hook** v0.1 — Better panic messages in browser console — 

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

- **u-he** for designing the Zebra/Zebralette 3 spline format that this tool interoperates with
- **Claude Code & Gemini-CLI** for assistance with development
