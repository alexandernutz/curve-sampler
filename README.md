# Curve Sampler

**Curve Sampler** is a VST3 and CLAP plugin designed to bridge the gap between live audio and u-he Zebra(lette) 3's spline-based oscillator. It acts as an oscilloscope and spectroscope with "capture to curve" functionality, allowing you to extract any single cycle of audio directly into a format you can paste into Zebra 3.

## Overview

Put Curve Sampler anywhere in your audio chain, and it will extract the current single cycle into a Zebra(lette) 3 curve string. That curve can then be directly pasted into the Zebra(lette) 3 editor.

The width of the single cycle is determined either by incoming MIDI notes or by manually setting a frequency. The plugin uses zero-crossing detection and stabilization to ensure a clean capture.

### Use Cases
- **Post-FX Capture:** Capture a curve after Zebra 3's internal oscillator effects (useful until this becomes a native feature).
- **Cross-Synth Migration:** Create a Zebra curve from another synth's output (e.g., "migrating" a Serum wavetable cycle without going through intermediate files).
- **Chord Baking:** Capture a chord into a single curve. It detects up to three MIDI notes and finds a reasonable least common multiple for the wavelength, allowing you to "bake" a chordal timbre into a single oscillator cycle.

## Philosophically Speaking...

For technical and mathematical reasons, parts of the Curve Sampler workflow are quite approximative. Don't expect it to always match internal waveforms with surgical accuracy—I’ve tried to get close, but it’s more of a **playground** than a laboratory tool.

Part of the charm of such a quick tool is that it doesn't have to match the quality standards of a "grown-up" plugin like Zebra 3. To me, the main benefit is developing a better sonic intuition for waveforms, making it easier to create cool sounds down the line—whether by using tools or drawing them yourself.

While the outputs sometimes look a bit "noisy," Zebra 3's inbuilt tools (Line-up, Simplify, Beautify) make it easy to polish the curves with a few clicks after pasting.

## Practical Setup

The easiest workflow is passing the same MIDI note (or simple chord) that goes into your synth directly to Curve Sampler.

- **Bitwig:** Place Curve Sampler in the device chain after your synth (e.g., Zebra 3). Bitwig passes MIDI through the chain automatically.
- **Serum / Other Synths:** Some synths don't pass MIDI notes through to the next plugin in the chain. In this case, Curve Sampler will revert to "Manual Frequency Mode," which might cause jumps in the geometry view.
  - *Workaround:* In Bitwig, wrap the synth in an Instrument Layer and put Curve Sampler after it within the layer or use a Note Receiver.
- **Other DAWs:** Setup varies, but ensuring Curve Sampler receives the same MIDI as the source synth is key for stable "Locked" visualization.

## Curve Jumbler (Web App)

This repository also contains **Curve Jumbler**, a browser-based companion tool for exploring domain transformations (Geometry ↔ Spectrum) and batch curve operations (Flip, Fold, etc.). It serves as a visual playground for the `curve-core` library.

You can try the live version of Curve Jumbler here: [https://YOUR_USERNAME.github.io/curve-sampler/](https://YOUR_USERNAME.github.io/curve-sampler/)

## Technical Notes & Zebra 3 Interop

- **Point Limits:** Zebra 3 can crash if you attempt to paste a curve with an excessive number of points. Curve Sampler includes point-reduction algorithms and a hard-cap of 100 points to ensure stability.
- **Approximation:** Signal extraction from rendered audio is subject to the uncertainty principle; "perfect" recovery is theoretically impossible, but the use of Blackman-Harris windowing and 4-point Catmull-Rom resampling gets us very close.

## Disclaimer & Safety

- **Security:** While rare, it is possible to spread malware through plugin binaries. This project is fully open-source, and my name is attached to it, which is the best "trust guarantee" I can offer. Always be cautious with unsigned binaries.
- **u-he Interop:** I don't want to cause extra work for the u-he team by "hacking" into an API (the clipboard format) that wasn't necessarily meant for this. If issues arise, I am happy to take this down or adjust it. If you're from u-he, feel free to reach out!

## AI Use Disclosure

This project was built with the help of **Claude Code** and **Gemini-CLI**. 

I have several years of industry/research programming experience, but I am not a DSP specialist. I wouldn't have been able to bridge the gap into Rust, the NIH-plug framework, and complex DSP algorithms without these tools. 

The process felt like acting as a "manager" for an incredibly fast programmer with deep domain knowledge and occasional odd quirks. This isn't "AI slop"—it’s a diligent, back-and-forth creative process that allows me to operate at a higher level of abstraction while still building something meaningful and functional on my own.

---

**License:** MIT
**Version:** 0.1.38
