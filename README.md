# Angel Synth

An FL-style piano playground in Rust. The egui front-end renders a four-octave keyboard (C2–C6), shows a live oscilloscope, and feeds a beefed-up synth engine with ADSR, detuned unison, vibrato, noise, multimode filtering, and a simple 3-band EQ while `cpal` streams audio in real time.

## Running

```bash
cargo run
```

Click the keys or just mash your entire keyboard—every key produces a note, and left/right arrows transpose the computer keyboard mapping in octaves. Adjust gain, ADSR, waveform, filter cutoff/resonance, vibrato, unison spread, noise mix, and the low/mid/high EQ bands from the control panel as you play, and watch the waveform glide across the scope.

## Tweaking the sound

- Core synth/envelope/filter logic lives in `src/synth.rs`.
- The realtime audio path (and scope ring buffer) is in `src/audio.rs` + `src/scope.rs`.
- `src/ui.rs` draws the keyboard, handles all keyboard shortcuts, and renders the scope + control panels (including EQ sliders).

It’s all plain Rust—no DSP crates—so feel free to expand `SynthEngine` with more modules (filters, effects, sequencers, etc.) or tweak the visuals to taste.
