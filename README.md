# Curve Sampler, Curve Jumbler

**Curve Sampler** is a VST3 and CLAP plugin designed to bridge the gap between live audio and u-he Zebra(lette) 3's spline-based oscillator. It acts as an oscilloscope and spectroscope with "capture to curve" functionality, allowing you to extract any single cycle of audio directly into a format you can paste into Zebra(lette) 3.

**Curve Jumbler** runs locally in your browser. It is meant as a playground for the vector-graphics based 
curves used in Zebra(lette) 3's OSC oscillator. 
As interface, it uses Z3's svg-based vector graphics format.
Try it here:
[https://alexandernutz.github.io/curve-sampler/](https://alexandernutz.github.io/curve-sampler/)


## Curve Sampler

Put Curve Sampler anywhere in your audio chain, and it will extract the current single cycle into a Zebra(lette) 3 (or short Zebra 3 or Z3 from now on) curve string. That curve can then be directly pasted into the Zebra 3 editor.

The width of the single cycle is determined either by incoming MIDI notes or by manually setting a frequency. The plugin uses zero-crossing detection and stabilization to ensure a clean capture.

### Use Cases
- **Post-FX Capture:** Capture a curve after Zebra 3's internal oscillator effects (useful until this becomes a native feature).
- **Cross-Synth Migration:** Create a Zebra curve from another synth's output (e.g., "migrating" a Serum wavetable cycle without going through intermediate files).
- **Chord Baking:** Capture a chord into a single curve. It detects up to three MIDI notes and finds a reasonable least common multiple for the wavelength, allowing you to "bake" a chordal timbre into a single oscillator cycle.

### Practical Setup

To determine the lenght of the captured waveform cycle, Curve Sampler needs some input.

The easiest workflow is passing the same MIDI note (or simple chord) that goes into your synth directly to Curve Sampler.
Additionally, there is the option to set up to three frequencies manually.

Curve Jumbler deals with **Chords** by trying to find a common cycle length. If it can't find a common cycle length (the common period can be impractically long), it will revert to using the lowest given note.

Note that MIDI routing workflow depends on the specific DAW and device chain.

Some examples:
- **Bitwig + Zebra 3:** Zebra 3 and many other plugins pass through MIDI notes they receive. So one can just place Curve Sampler after it, and it will receive the MIDI notes.
- **Serum / Other Synths:** Other synths, for instance Serum 2, don't pass through MIDI notes. 
  - *Bitwig Workaround:* In Bitwig, wrap the synth in an Instrument Layer and put Curve Sampler after it.
- **Other DAWs:** Setup varies and will depend on how the DAW routes MIDI. 
  - *Fallback*: A Fallback, if MIDI routing is a pain is always to play, say an A4 and set 440Hz (or whatever your audio source is tuned to) Manually.

## Curve Jumbler (Web App)

This repository also contains **Curve Jumbler**, a browser-based tool that allows generation and manipulation
of waveforms in the Zebra 3-style vector representation.
It allows domain transformations (Geometry ↔ Spectrum) and other curve operations (Flip, Fold, etc.). 

The workflow is :
 - Copy/Paste Curve from Z3 into Curve Jumbler (or start with a pre-defined or random curve), 
 - Do a bunch of pre-defined transformations,
 - Copy/Paste back into Z3.

Note that Curve Jumbler is less polished than Curve Sampler, since I realized at some point that the latter can do everything the former can do (sort of) and more.
Advantages of Jumbler that remain are:
 - higher import fidelity, as the Z3 curves are copied (but right now there is no way to e.g. get the curve "after effects")
 - web app -- no download/installation (also, it runs purely locally in your browser, so no cloud / data upload involved)

You can try the live version of Curve Jumbler here: [https://alexandernutz.github.io/curve-sampler/](https://alexandernutz.github.io/curve-sampler/)

## General Notes
These things apply to both Curve Sampler and Curve Jumbler.

### Known Limitations

For technical and funamental/mathematical reasons, parts of the Curve Sampler workflow are quite approximative. Don't expect it to always match internal waveforms with surgical accuracy—I’ve tried to get close, but I think of Curve Sampler more as a **SVG Curve playground** than a laboratory tool.

Part of the charm of such a quick tool is that it doesn't have to match the quality standards of a "grown-up" plugin like Zebra 3. To me, one benefit is developing a better sonic intuition for waveforms, making it easier to create cool sounds down the line—whether by using tools or drawing them myself.

While the outputs sometimes are a bit "noisy," Zebra 3's inbuilt tools (Line-up, Simplify, Beautify) can often help to polish the curves with a few clicks after pasting.

Note that curves with many points can be heavy on Zebra 3's performance, especially if they are being morphed, simplifying them can help a lot there.


### Technical Notes & Zebra 3 Interop

- **Point Limits:** Zebra 3 can crash if you attempt to paste a curve with an excessive number of points. Curve Sampler includes point-reduction algorithms and a hard-cap of 100 points to ensure stability.
- **Approximation:** Signal extraction from rendered audio is always approximative, but I tried to get 
  close. (For those more versed than me, Gemini gives more details: Signal extraction from rendered audio is subject to the uncertainty principle; "perfect" recovery is theoretically impossible, but the use of Blackman-Harris windowing and 4-point Catmull-Rom resampling gets us very close.)

## Disclaimer & Safety

- **Security:** While rare, it is possible to spread malware through plugin binaries. This project is fully open-source, release binaries are built through github's standard workflow, and my name is attached to it, which is the best "trust guarantee" I can offer. Always be cautious with unsigned binaries.
- **u-he Interop:** I don't want to cause extra work for the u-he team by "hacking" into an API (the clipboard format) that wasn't necessarily meant for this. If issues arise, I am happy to take this down or adjust it. If you're from u-he, feel free to reach out!

## AI Use Disclosure & Thoughts

I made this with the help of Claude Code and Gemini-CLI. 

I do have several years of industry/research programming experience, but I am not a DSP specialist. 
I wouldn't have been able to make this without. The cost of learning Rust, the various frameworks, and DSP algorithms would have been way too high for me for my current time budget (though I'd love to learn more some time). 

Musing: This was a new experience, it's a bit like acting as the manager of a programmer with a lot of domain knowledge (who is also inhumanly fast, but also has odd quirks that humans don't have, usually), going back and forth. I think this process still can be done with more or less dilligence -- suffice to say, I'm not trying to build AI-code-slop here, but make something that makes sense and is useful and practical. 
AI hopefully just allows me to operate on this more abstract level, while still doing it "all on my own". 
I guess we'll all have to see where this goes and hopefully be responsible about it ...

## Licensing

**Source Code:** MIT 

**Plugin Binaries (in releases):**
- **CLAP:** MIT
- **VST3:** GPLv3

---

**Version:** 0.1.77
