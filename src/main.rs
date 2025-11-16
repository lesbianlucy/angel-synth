use std::collections::BTreeSet;
use std::f32::consts::TAU;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use egui::{self, Align2, Color32, ComboBox, FontId, Rounding};

fn main() -> eframe::Result<()> {
    env_logger::init();

    let shared = Arc::new(Mutex::new(SynthShared::default()));
    let audio = SynthAudio::new(Arc::clone(&shared))
        .expect("Failed to initialize audio output. Is an output device available?");

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Angel Piano",
        options,
        Box::new(move |cc| Box::new(SynthApp::new(cc, Arc::clone(&shared), audio.clone()))),
    )
}

struct SynthApp {
    shared: Arc<Mutex<SynthShared>>,
    _audio: SynthAudio,
    mouse_note: Option<u8>,
}

impl SynthApp {
    fn new(
        _: &eframe::CreationContext<'_>,
        shared: Arc<Mutex<SynthShared>>,
        audio: SynthAudio,
    ) -> Self {
        Self {
            shared,
            _audio: audio,
            mouse_note: None,
        }
    }
}

impl eframe::App for SynthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let keyboard_actions = collect_keyboard_actions(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Angel Piano");
            ui.label(
                "Click the keyboard or use Z-M/Q-I for white keys and S,D,G,H,J plus 2,3,5,6,7 for sharps.",
            );

            let mut shared = self
                .shared
                .lock()
                .expect("Synth parameters poisoned");

            for (note, pressed) in keyboard_actions {
                if pressed {
                    shared.press_note(note);
                } else {
                    shared.release_note(note);
                }
            }

            draw_piano(ui, &mut shared, &mut self.mouse_note);

            ui.separator();
            ui.heading("Tone controls");

            ui.add(egui::Slider::new(&mut shared.params.gain, 0.0..=1.0).text("Master gain"));
            ui.add(
                egui::Slider::new(&mut shared.params.attack_seconds, 0.002..=1.0)
                    .logarithmic(true)
                    .text("Attack (s)"),
            );
            ui.add(
                egui::Slider::new(&mut shared.params.release_seconds, 0.01..=2.0)
                    .logarithmic(true)
                    .text("Release (s)"),
            );

            ui.horizontal(|ui| {
                ui.label("Waveform");
                ComboBox::from_id_source("waveform")
                    .selected_text(shared.params.waveform.label())
                    .show_ui(ui, |ui| {
                        for waveform in Waveform::ALL {
                            ui.selectable_value(
                                &mut shared.params.waveform,
                                waveform,
                                waveform.label(),
                            );
                        }
                    });
            });
        });
    }
}

#[derive(Clone)]
struct SynthParams {
    gain: f32,
    attack_seconds: f32,
    release_seconds: f32,
    waveform: Waveform,
}

impl Default for SynthParams {
    fn default() -> Self {
        Self {
            gain: 0.65,
            attack_seconds: 0.01,
            release_seconds: 0.25,
            waveform: Waveform::Saw,
        }
    }
}

#[derive(Clone)]
struct SynthShared {
    params: SynthParams,
    pressed_notes: BTreeSet<u8>,
}

impl Default for SynthShared {
    fn default() -> Self {
        Self {
            params: SynthParams::default(),
            pressed_notes: BTreeSet::new(),
        }
    }
}

impl SynthShared {
    fn press_note(&mut self, note: u8) {
        self.pressed_notes.insert(note);
    }

    fn release_note(&mut self, note: u8) {
        self.pressed_notes.remove(&note);
    }

    fn is_pressed(&self, note: u8) -> bool {
        self.pressed_notes.contains(&note)
    }

    fn snapshot(&self) -> SynthSnapshot {
        SynthSnapshot {
            params: self.params.clone(),
            pressed_notes: self.pressed_notes.iter().copied().collect(),
        }
    }
}

#[derive(Clone)]
struct SynthSnapshot {
    params: SynthParams,
    pressed_notes: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Waveform {
    Sine,
    Square,
    Saw,
    Triangle,
}

impl Waveform {
    const ALL: [Waveform; 4] = [
        Waveform::Sine,
        Waveform::Square,
        Waveform::Saw,
        Waveform::Triangle,
    ];

    fn label(&self) -> &'static str {
        match self {
            Waveform::Sine => "Sine",
            Waveform::Square => "Square",
            Waveform::Saw => "Saw",
            Waveform::Triangle => "Triangle",
        }
    }

    fn sample(&self, phase: f32) -> f32 {
        match self {
            Waveform::Sine => (TAU * phase).sin(),
            Waveform::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Saw => 2.0 * (phase - 0.5),
            Waveform::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
        }
    }
}

#[derive(Clone, Copy)]
struct PianoKey {
    note: u8,
    label: &'static str,
    is_black: bool,
}

impl PianoKey {
    const fn white(note: u8, label: &'static str) -> Self {
        Self {
            note,
            label,
            is_black: false,
        }
    }

    const fn black(note: u8, label: &'static str) -> Self {
        Self {
            note,
            label,
            is_black: true,
        }
    }
}

const PIANO_KEYS: [PianoKey; 25] = [
    PianoKey::white(48, "C3"),
    PianoKey::black(49, "C#"),
    PianoKey::white(50, "D3"),
    PianoKey::black(51, "D#"),
    PianoKey::white(52, "E3"),
    PianoKey::white(53, "F3"),
    PianoKey::black(54, "F#"),
    PianoKey::white(55, "G3"),
    PianoKey::black(56, "G#"),
    PianoKey::white(57, "A3"),
    PianoKey::black(58, "A#"),
    PianoKey::white(59, "B3"),
    PianoKey::white(60, "C4"),
    PianoKey::black(61, "C#"),
    PianoKey::white(62, "D4"),
    PianoKey::black(63, "D#"),
    PianoKey::white(64, "E4"),
    PianoKey::white(65, "F4"),
    PianoKey::black(66, "F#"),
    PianoKey::white(67, "G4"),
    PianoKey::black(68, "G#"),
    PianoKey::white(69, "A4"),
    PianoKey::black(70, "A#"),
    PianoKey::white(71, "B4"),
    PianoKey::white(72, "C5"),
];

const KEYBOARD_SHORTCUTS: &[(egui::Key, u8)] = &[
    (egui::Key::Z, 48),
    (egui::Key::S, 49),
    (egui::Key::X, 50),
    (egui::Key::D, 51),
    (egui::Key::C, 52),
    (egui::Key::V, 53),
    (egui::Key::G, 54),
    (egui::Key::B, 55),
    (egui::Key::H, 56),
    (egui::Key::N, 57),
    (egui::Key::J, 58),
    (egui::Key::M, 59),
    (egui::Key::Q, 60),
    (egui::Key::Num2, 61),
    (egui::Key::W, 62),
    (egui::Key::Num3, 63),
    (egui::Key::E, 64),
    (egui::Key::R, 65),
    (egui::Key::Num5, 66),
    (egui::Key::T, 67),
    (egui::Key::Num6, 68),
    (egui::Key::Y, 69),
    (egui::Key::Num7, 70),
    (egui::Key::U, 71),
    (egui::Key::I, 72),
];

const WHITE_KEY_WIDTH: f32 = 40.0;
const WHITE_KEY_HEIGHT: f32 = 180.0;
const BLACK_KEY_WIDTH: f32 = WHITE_KEY_WIDTH * 0.6;
const BLACK_KEY_HEIGHT: f32 = WHITE_KEY_HEIGHT * 0.62;
const SNAPSHOT_REFRESH_INTERVAL: usize = 64;

fn collect_keyboard_actions(ctx: &egui::Context) -> Vec<(u8, bool)> {
    let mut actions = Vec::new();
    ctx.input(|input| {
        for &(key, note) in KEYBOARD_SHORTCUTS {
            if input.key_pressed(key) {
                actions.push((note, true));
            }
            if input.key_released(key) {
                actions.push((note, false));
            }
        }
    });
    actions
}

fn draw_piano(ui: &mut egui::Ui, shared: &mut SynthShared, mouse_note: &mut Option<u8>) {
    let white_keys = PIANO_KEYS.iter().filter(|k| !k.is_black).count();
    let desired_size = egui::vec2(white_keys as f32 * WHITE_KEY_WIDTH, WHITE_KEY_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    let pointer_down = ui.input(|i| i.pointer.primary_down());
    let pointer_pos = ui.input(|i| i.pointer.interact_pos());

    let mut white_layout = Vec::new();
    let mut black_layout = Vec::new();
    let mut white_index = 0usize;

    for key in PIANO_KEYS.iter() {
        if !key.is_black {
            let x = rect.min.x + white_index as f32 * WHITE_KEY_WIDTH;
            let key_rect = egui::Rect::from_min_size(
                egui::pos2(x, rect.min.y),
                egui::vec2(WHITE_KEY_WIDTH - 1.0, WHITE_KEY_HEIGHT),
            );
            white_layout.push((key, key_rect));
            white_index += 1;
        } else {
            let preceding_white = white_index.saturating_sub(1) as f32;
            let base = rect.min.x + preceding_white * WHITE_KEY_WIDTH;
            let x = base + WHITE_KEY_WIDTH * 0.7 - BLACK_KEY_WIDTH / 2.0;
            let key_rect = egui::Rect::from_min_size(
                egui::pos2(x, rect.min.y),
                egui::vec2(BLACK_KEY_WIDTH, BLACK_KEY_HEIGHT),
            );
            black_layout.push((key, key_rect));
        }
    }

    let mut pointer_note = None;
    if let (Some(pos), true) = (pointer_pos, pointer_down) {
        if rect.contains(pos) {
            for (key, key_rect) in &black_layout {
                if key_rect.contains(pos) {
                    pointer_note = Some(key.note);
                    break;
                }
            }
            if pointer_note.is_none() {
                for (key, key_rect) in &white_layout {
                    if key_rect.contains(pos) {
                        pointer_note = Some(key.note);
                        break;
                    }
                }
            }
        }
    }

    if pointer_down {
        if let Some(note) = pointer_note {
            if mouse_note != &Some(note) {
                if let Some(prev) = mouse_note.take() {
                    shared.release_note(prev);
                }
                shared.press_note(note);
                *mouse_note = Some(note);
            }
        } else if let Some(prev) = mouse_note.take() {
            shared.release_note(prev);
        }
    } else if let Some(prev) = mouse_note.take() {
        shared.release_note(prev);
    }

    let white_idle = Color32::from_rgb(245, 245, 245);
    let pressed_fill = ui.visuals().selection.bg_fill;

    for (key, key_rect) in &white_layout {
        let active = shared.is_pressed(key.note);
        painter.rect(
            *key_rect,
            Rounding::same(4.0),
            if active { pressed_fill } else { white_idle },
            (1.0, Color32::BLACK),
        );
        painter.text(
            egui::pos2(key_rect.center().x, key_rect.max.y - 6.0),
            Align2::CENTER_BOTTOM,
            key.label,
            FontId::monospace(12.0),
            Color32::from_rgb(40, 40, 40),
        );
    }

    for (key, key_rect) in &black_layout {
        let active = shared.is_pressed(key.note);
        let fill = if active {
            pressed_fill
        } else {
            Color32::from_rgb(20, 20, 20)
        };
        painter.rect(
            *key_rect,
            Rounding::same(3.0),
            fill,
            (1.0, Color32::from_rgb(15, 15, 15)),
        );
    }

    response.on_hover_text("Click and drag to play");
}

#[derive(Clone)]
struct SynthAudio {
    _stream: Arc<cpal::Stream>,
}

impl SynthAudio {
    fn new(shared: Arc<Mutex<SynthShared>>) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No audio output device available".to_string())?;
        let supported_config = device
            .default_output_config()
            .map_err(|err| format!("Could not query default output: {err}"))?;
        let sample_format = supported_config.sample_format();
        let config: cpal::StreamConfig = supported_config.into();
        let sample_rate = config.sample_rate.0 as f32;
        let channels = config.channels as usize;

        let err_fn = |err| eprintln!("Audio stream error: {err}");
        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let mut engine = SynthEngine::new(sample_rate);
                let shared_state = Arc::clone(&shared);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _| {
                            write_samples_f32(&shared_state, &mut engine, data, channels);
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|err| format!("Failed to build f32 stream: {err}"))?
            }
            cpal::SampleFormat::I16 => {
                let mut engine = SynthEngine::new(sample_rate);
                let shared_state = Arc::clone(&shared);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [i16], _| {
                            write_samples_i16(&shared_state, &mut engine, data, channels);
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|err| format!("Failed to build i16 stream: {err}"))?
            }
            cpal::SampleFormat::U16 => {
                let mut engine = SynthEngine::new(sample_rate);
                let shared_state = Arc::clone(&shared);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [u16], _| {
                            write_samples_u16(&shared_state, &mut engine, data, channels);
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|err| format!("Failed to build u16 stream: {err}"))?
            }
            other => {
                return Err(format!("Unsupported sample format: {other:?}"));
            }
        };
        stream
            .play()
            .map_err(|err| format!("Failed to start audio: {err}"))?;
        Ok(Self {
            _stream: Arc::new(stream),
        })
    }
}

#[derive(Clone, Copy)]
enum EnvStage {
    Idle,
    Attack,
    Sustain,
    Release,
}

struct VoiceState {
    note: u8,
    phase: f32,
    env_level: f32,
    stage: EnvStage,
    gate: bool,
}

impl VoiceState {
    fn new(note: u8) -> Self {
        Self {
            note,
            phase: 0.0,
            env_level: 0.0,
            stage: EnvStage::Idle,
            gate: false,
        }
    }

    fn set_gate(&mut self, gate: bool) {
        if gate && !self.gate {
            self.stage = EnvStage::Attack;
        } else if !gate && self.gate {
            if !matches!(self.stage, EnvStage::Idle) {
                self.stage = EnvStage::Release;
            }
        }
        self.gate = gate;
    }

    fn next_sample(&mut self, params: &SynthParams, sample_rate: f32) -> f32 {
        self.advance_envelope(params, sample_rate);
        if matches!(self.stage, EnvStage::Idle) {
            return 0.0;
        }

        let freq = midi_to_freq(self.note);
        self.phase += freq / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        params.waveform.sample(self.phase) * self.env_level * params.gain
    }

    fn advance_envelope(&mut self, params: &SynthParams, sample_rate: f32) {
        match self.stage {
            EnvStage::Idle => {
                self.env_level = 0.0;
            }
            EnvStage::Attack => {
                let step = if params.attack_seconds <= 0.0 {
                    1.0
                } else {
                    1.0 / (params.attack_seconds * sample_rate)
                };
                self.env_level += step;
                if self.env_level >= 1.0 {
                    self.env_level = 1.0;
                    self.stage = EnvStage::Sustain;
                }
            }
            EnvStage::Sustain => self.env_level = 1.0,
            EnvStage::Release => {
                let step = if params.release_seconds <= 0.0 {
                    1.0
                } else {
                    1.0 / (params.release_seconds * sample_rate)
                };
                self.env_level -= step;
                if self.env_level <= 0.0 {
                    self.env_level = 0.0;
                    self.stage = EnvStage::Idle;
                }
            }
        }
    }

    fn is_finished(&self) -> bool {
        matches!(self.stage, EnvStage::Idle) && !self.gate
    }
}

struct SynthEngine {
    voices: Vec<VoiceState>,
    sample_rate: f32,
}

impl SynthEngine {
    fn new(sample_rate: f32) -> Self {
        Self {
            voices: Vec::new(),
            sample_rate,
        }
    }

    fn sync_voices(&mut self, pressed: &[u8]) {
        for voice in &mut self.voices {
            let gate = pressed.contains(&voice.note);
            voice.set_gate(gate);
        }
        for &note in pressed {
            if !self.voices.iter().any(|voice| voice.note == note) {
                let mut voice = VoiceState::new(note);
                voice.set_gate(true);
                self.voices.push(voice);
            }
        }
    }

    fn next_sample(&mut self, snapshot: &SynthSnapshot) -> f32 {
        self.sync_voices(&snapshot.pressed_notes);
        let mut mix = 0.0;
        for voice in &mut self.voices {
            mix += voice.next_sample(&snapshot.params, self.sample_rate);
        }
        self.voices.retain(|voice| !voice.is_finished());
        mix
    }
}

fn midi_to_freq(note: u8) -> f32 {
    440.0 * 2_f32.powf((note as f32 - 69.0) / 12.0)
}

fn write_samples_f32(
    shared: &Arc<Mutex<SynthShared>>,
    engine: &mut SynthEngine,
    buffer: &mut [f32],
    channels: usize,
) {
    let mut snapshot = {
        let shared = shared.lock().expect("Synth parameters poisoned");
        shared.snapshot()
    };

    for (i, frame) in buffer.chunks_mut(channels).enumerate() {
        if i % SNAPSHOT_REFRESH_INTERVAL == 0 {
            snapshot = {
                let shared = shared.lock().expect("Synth parameters poisoned");
                shared.snapshot()
            };
        }
        let sample = engine.next_sample(&snapshot);
        for channel in frame {
            *channel = sample;
        }
    }
}

fn write_samples_i16(
    shared: &Arc<Mutex<SynthShared>>,
    engine: &mut SynthEngine,
    buffer: &mut [i16],
    channels: usize,
) {
    let mut snapshot = {
        let shared = shared.lock().expect("Synth parameters poisoned");
        shared.snapshot()
    };

    for (i, frame) in buffer.chunks_mut(channels).enumerate() {
        if i % SNAPSHOT_REFRESH_INTERVAL == 0 {
            snapshot = {
                let shared = shared.lock().expect("Synth parameters poisoned");
                shared.snapshot()
            };
        }
        let sample = engine.next_sample(&snapshot);
        let scaled = (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        for channel in frame {
            *channel = scaled;
        }
    }
}

fn write_samples_u16(
    shared: &Arc<Mutex<SynthShared>>,
    engine: &mut SynthEngine,
    buffer: &mut [u16],
    channels: usize,
) {
    let mut snapshot = {
        let shared = shared.lock().expect("Synth parameters poisoned");
        shared.snapshot()
    };

    for (i, frame) in buffer.chunks_mut(channels).enumerate() {
        if i % SNAPSHOT_REFRESH_INTERVAL == 0 {
            snapshot = {
                let shared = shared.lock().expect("Synth parameters poisoned");
                shared.snapshot()
            };
        }
        let normalized = engine.next_sample(&snapshot).clamp(-1.0, 1.0);
        let value = ((normalized * 0.5 + 0.5) * u16::MAX as f32) as u16;
        for channel in frame {
            *channel = value;
        }
    }
}
