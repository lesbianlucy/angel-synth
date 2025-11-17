use std::collections::BTreeSet;
use std::f32::consts::{SQRT_2, TAU};

#[derive(Clone, PartialEq)]
pub struct SynthParams {
    pub gain: f32,
    pub attack_seconds: f32,
    pub decay_seconds: f32,
    pub sustain_level: f32,
    pub release_seconds: f32,
    pub instrument: InstrumentKind,
    pub waveform: Waveform,
    pub filter_cutoff_hz: f32,
    pub filter_resonance: f32,
    pub vibrato_depth_semitones: f32,
    pub vibrato_rate_hz: f32,
    pub unison_spread_cents: f32,
    pub autotune_amount: f32,
    pub noise_mix: f32,
    pub eq_low_gain_db: f32,
    pub eq_low_freq_hz: f32,
    pub eq_mid_gain_db: f32,
    pub eq_mid_freq_hz: f32,
    pub eq_mid_q: f32,
    pub eq_high_gain_db: f32,
    pub eq_high_freq_hz: f32,
}

impl Default for SynthParams {
    fn default() -> Self {
        Self {
            gain: 0.65,
            attack_seconds: 0.01,
            decay_seconds: 0.2,
            sustain_level: 0.7,
            release_seconds: 0.35,
            instrument: InstrumentKind::Keys,
            waveform: Waveform::Saw,
            filter_cutoff_hz: 4_000.0,
            filter_resonance: 0.2,
            vibrato_depth_semitones: 0.15,
            vibrato_rate_hz: 4.0,
            unison_spread_cents: 6.0,
            autotune_amount: 0.0,
            noise_mix: 0.03,
            eq_low_gain_db: 0.0,
            eq_low_freq_hz: 120.0,
            eq_mid_gain_db: 0.0,
            eq_mid_freq_hz: 1_000.0,
            eq_mid_q: 0.8,
            eq_high_gain_db: 0.0,
            eq_high_freq_hz: 6_000.0,
        }
    }
}

#[derive(Clone)]
pub struct SynthShared {
    pub params: SynthParams,
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
    pub fn new_with_params(params: SynthParams) -> Self {
        Self {
            params,
            pressed_notes: BTreeSet::new(),
        }
    }

    pub fn press_note(&mut self, note: u8) {
        self.pressed_notes.insert(note);
    }

    pub fn release_note(&mut self, note: u8) {
        self.pressed_notes.remove(&note);
    }

    pub fn is_pressed(&self, note: u8) -> bool {
        self.pressed_notes.contains(&note)
    }

    pub fn snapshot(&self) -> SynthSnapshot {
        SynthSnapshot {
            params: self.params.clone(),
            pressed_notes: self.pressed_notes.iter().copied().collect(),
        }
    }
}

#[derive(Clone)]
pub struct SynthSnapshot {
    pub params: SynthParams,
    pub pressed_notes: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Square,
    Saw,
    Triangle,
}

impl Waveform {
    pub const ALL: [Waveform; 4] = [
        Waveform::Sine,
        Waveform::Square,
        Waveform::Saw,
        Waveform::Triangle,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Waveform::Sine => "Sine",
            Waveform::Square => "Square",
            Waveform::Saw => "Saw",
            Waveform::Triangle => "Triangle",
        }
    }

    pub fn sample(&self, phase: f32) -> f32 {
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InstrumentKind {
    Keys,
    Bass,
    Lead,
    Pad,
}

impl InstrumentKind {
    pub const ALL: [InstrumentKind; 4] = [
        InstrumentKind::Keys,
        InstrumentKind::Bass,
        InstrumentKind::Lead,
        InstrumentKind::Pad,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            InstrumentKind::Keys => "Keys",
            InstrumentKind::Bass => "Bass",
            InstrumentKind::Lead => "Lead",
            InstrumentKind::Pad => "Pad",
        }
    }
}

#[derive(Clone, Copy)]
enum EnvStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

struct VoiceState {
    note: u8,
    phase: f32,
    env_level: f32,
    stage: EnvStage,
    gate: bool,
    filter_state: f32,
    lfo_phase: f32,
    noise_seed: u32,
}

impl VoiceState {
    fn new(note: u8) -> Self {
        Self {
            note,
            phase: 0.0,
            env_level: 0.0,
            stage: EnvStage::Idle,
            gate: false,
            filter_state: 0.0,
            lfo_phase: 0.0,
            noise_seed: (note as u32).wrapping_mul(1_104_607),
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

        let vibrato_depth = params.vibrato_depth_semitones * (1.0 - params.autotune_amount);
        let vibrato = if vibrato_depth > 0.0 {
            (TAU * self.lfo_phase).sin() * vibrato_depth
        } else {
            0.0
        };
        self.lfo_phase += params.vibrato_rate_hz / sample_rate;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }

        let freq = midi_to_freq(self.note) * 2_f32.powf(vibrato / 12.0);
        self.phase += freq / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let base_phase = self.phase;

        let mut sample = self.unison_sample(params, base_phase);
        sample = VoiceState::apply_instrument_color(sample, base_phase, params.instrument);
        if params.noise_mix > 0.0 {
            let noise = self.next_noise();
            sample = sample * (1.0 - params.noise_mix) + noise * params.noise_mix;
        }

        let filtered = self.apply_filter(sample, params, sample_rate);
        filtered * self.env_level * params.gain
    }

    fn apply_instrument_color(sample: f32, base_phase: f32, instrument: InstrumentKind) -> f32 {
        match instrument {
            InstrumentKind::Keys => sample,
            InstrumentKind::Bass => {
                let sub = (TAU * (base_phase * 0.5)).sin();
                ((sample * 0.75) + (sub * 0.35)).clamp(-1.0, 1.0)
            }
            InstrumentKind::Lead => {
                let overtone = (TAU * (base_phase * 2.0)).sin() * 0.2 + sample;
                (overtone * 1.2).tanh()
            }
            InstrumentKind::Pad => sample * 0.9,
        }
    }

    fn unison_sample(&self, params: &SynthParams, base_phase: f32) -> f32 {
        let detune =
            ((params.unison_spread_cents * (1.0 - params.autotune_amount)) / 1200.0).min(0.2);
        let offsets: &[f32] = if detune > 0.0 {
            &[-detune, 0.0, detune]
        } else {
            &[0.0]
        };
        let mut acc = 0.0;
        for offset in offsets {
            let phase = (base_phase + offset).fract();
            acc += params.waveform.sample(phase);
        }
        acc / offsets.len() as f32
    }

    fn apply_filter(&mut self, input: f32, params: &SynthParams, sample_rate: f32) -> f32 {
        let cutoff = params
            .filter_cutoff_hz
            .clamp(60.0, sample_rate.min(48_000.0) * 0.45);
        let x = (TAU * cutoff / sample_rate).min(0.99);
        let alpha = x / (1.0 + x);
        self.filter_state += alpha * (input - self.filter_state);
        let resonance = params.filter_resonance.clamp(0.0, 0.95);
        self.filter_state + resonance * (self.filter_state - input)
    }

    fn next_noise(&mut self) -> f32 {
        // simple LCG mapped to [-1, 1]
        self.noise_seed = self
            .noise_seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let value = ((self.noise_seed >> 9) & 0x7FFFFF) as f32 / 0x7FFFFF as f32;
        value * 2.0 - 1.0
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
                    self.stage = EnvStage::Decay;
                }
            }
            EnvStage::Decay => {
                let target = params.sustain_level.clamp(0.0, 1.0);
                let step = if params.decay_seconds <= 0.0 {
                    1.0
                } else {
                    1.0 / (params.decay_seconds * sample_rate)
                };
                self.env_level -= step * (1.0 - target);
                if self.env_level <= target {
                    self.env_level = target;
                    self.stage = EnvStage::Sustain;
                }
            }
            EnvStage::Sustain => self.env_level = params.sustain_level.clamp(0.0, 1.0),
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

pub struct SynthEngine {
    voices: Vec<VoiceState>,
    sample_rate: f32,
    eq_chain: EqChain,
}

impl SynthEngine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voices: Vec::new(),
            sample_rate,
            eq_chain: EqChain::new(sample_rate),
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

    pub fn next_sample(&mut self, snapshot: &SynthSnapshot) -> f32 {
        self.sync_voices(&snapshot.pressed_notes);
        let mut mix = 0.0;
        for voice in &mut self.voices {
            mix += voice.next_sample(&snapshot.params, self.sample_rate);
        }
        self.voices.retain(|voice| !voice.is_finished());
        self.eq_chain.process(mix)
    }

    pub fn update_eq(&mut self, params: &SynthParams) {
        self.eq_chain.update(params);
    }
}

fn midi_to_freq(note: u8) -> f32 {
    440.0 * 2_f32.powf((note as f32 - 69.0) / 12.0)
}

struct EqChain {
    sample_rate: f32,
    low: BiquadState,
    mid: BiquadState,
    high: BiquadState,
}

impl EqChain {
    fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            low: BiquadState::new(),
            mid: BiquadState::new(),
            high: BiquadState::new(),
        }
    }

    fn update(&mut self, params: &SynthParams) {
        let low = low_shelf_coeffs(
            self.sample_rate,
            params.eq_low_freq_hz,
            params.eq_low_gain_db,
        );
        let mid = peaking_coeffs(
            self.sample_rate,
            params.eq_mid_freq_hz,
            params.eq_mid_q,
            params.eq_mid_gain_db,
        );
        let high = high_shelf_coeffs(
            self.sample_rate,
            params.eq_high_freq_hz,
            params.eq_high_gain_db,
        );

        self.low.set_coeffs(low);
        self.mid.set_coeffs(mid);
        self.high.set_coeffs(high);
    }

    fn process(&mut self, sample: f32) -> f32 {
        let low = self.low.process(sample);
        let mid = self.mid.process(low);
        self.high.process(mid)
    }
}

#[derive(Clone, Copy)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl BiquadCoeffs {
    fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    fn from_raw(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        let inv_a0 = if a0.abs() < f32::EPSILON {
            1.0
        } else {
            1.0 / a0
        };
        Self {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
        }
    }
}

struct BiquadState {
    coeffs: BiquadCoeffs,
    z1: f32,
    z2: f32,
}

impl BiquadState {
    fn new() -> Self {
        Self {
            coeffs: BiquadCoeffs::identity(),
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn set_coeffs(&mut self, coeffs: BiquadCoeffs) {
        self.coeffs = coeffs;
    }

    fn process(&mut self, input: f32) -> f32 {
        let y = self.coeffs.b0 * input + self.z1;
        self.z1 = self.coeffs.b1 * input - self.coeffs.a1 * y + self.z2;
        self.z2 = self.coeffs.b2 * input - self.coeffs.a2 * y;
        y
    }
}

fn low_shelf_coeffs(sample_rate: f32, freq: f32, gain_db: f32) -> BiquadCoeffs {
    let freq = freq.clamp(10.0, sample_rate * 0.45);
    let a = 10_f32.powf(gain_db / 40.0);
    let w0 = TAU * freq / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let sqrt_a = a.sqrt();
    let alpha = sin_w0 / 2.0 * SQRT_2;

    let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
    let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
    let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
    let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
    let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
    let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;

    BiquadCoeffs::from_raw(b0, b1, b2, a0, a1, a2)
}

fn high_shelf_coeffs(sample_rate: f32, freq: f32, gain_db: f32) -> BiquadCoeffs {
    let freq = freq.clamp(10.0, sample_rate * 0.45);
    let a = 10_f32.powf(gain_db / 40.0);
    let w0 = TAU * freq / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let sqrt_a = a.sqrt();
    let alpha = sin_w0 / 2.0 * SQRT_2;

    let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
    let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
    let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
    let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
    let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
    let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;

    BiquadCoeffs::from_raw(b0, b1, b2, a0, a1, a2)
}

fn peaking_coeffs(sample_rate: f32, freq: f32, q: f32, gain_db: f32) -> BiquadCoeffs {
    let freq = freq.clamp(10.0, sample_rate * 0.45);
    let q = q.clamp(0.1, 4.0);
    let a = 10_f32.powf(gain_db / 40.0);
    let w0 = TAU * freq / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);

    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * cos_w0;
    let b2 = 1.0 - alpha * a;
    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha / a;

    BiquadCoeffs::from_raw(b0, b1, b2, a0, a1, a2)
}
