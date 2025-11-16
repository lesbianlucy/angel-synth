# Angel Synth

A pint-sized FL-style piano written in Rust. The app opens an egui window with a clickable keyboard (two octaves, C3–C5) and a couple of tone controls while a `cpal` audio stream renders the oscillator in real time.

## Running

```bash
cargo run
```

Click on the keys or use the computer keyboard:
- White keys: `Z X C V B N M` for C3–B3 and `Q W E R T Y U I` for C4–C5
- Black keys: `S D G H J` for the lower octave, plus `2 3 5 6 7` for the upper octave sharps

Adjust gain, attack/release, and waveform from the right-hand controls while notes are playing.

## Tweaking the sound

- All synth code lives in `src/main.rs`. Modify `PIANO_KEYS`, `KEYBOARD_SHORTCUTS`, or the ADSR/waveform logic to explore different layouts and voices.
- `SynthEngine` is intentionally tiny—it's a good spot to add more modulation (LFOs, filters, etc.) if you want to grow this into a fuller instrument.
