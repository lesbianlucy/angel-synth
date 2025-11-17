use std::fs;
use std::path::{Path, PathBuf};

use crate::synth::{SynthParams, Waveform};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    Fl,
    Light,
}

impl ThemeKind {
    pub fn label(self) -> &'static str {
        match self {
            ThemeKind::Fl => "FL Dark",
            ThemeKind::Light => "Light",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "light" => ThemeKind::Light,
            _ => ThemeKind::Fl,
        }
    }

    pub fn as_key(self) -> &'static str {
        match self {
            ThemeKind::Fl => "fl",
            ThemeKind::Light => "light",
        }
    }
}

#[derive(Clone)]
pub struct AppSettings {
    pub theme: ThemeKind,
    pub params: SynthParams,
    pub output_device: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeKind::Fl,
            params: SynthParams::default(),
            output_device: None,
        }
    }
}

impl AppSettings {
    pub fn load(path: &Path) -> Self {
        let mut settings = AppSettings::default();
        if let Ok(raw) = fs::read_to_string(path) {
            for line in raw.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    apply_kv(key.trim(), value.trim(), &mut settings);
                }
            }
        }
        settings
    }

    pub fn save(&self, path: &Path) {
        let mut buf = String::new();
        buf.push_str(&format!("theme={}\n", self.theme.as_key()));
        if let Some(name) = &self.output_device {
            buf.push_str(&format!("output_device={name}\n"));
        }
        append_param_lines(&mut buf, &self.params);

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, buf);
    }
}

pub fn default_settings_path() -> PathBuf {
    config_dir().join("angel_settings.cfg")
}

fn apply_kv(key: &str, value: &str, settings: &mut AppSettings) {
    match key {
        "theme" => settings.theme = ThemeKind::from_str(value),
        "output_device" => {
            settings.output_device = if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        }
        "gain" => parse_f32(value, &mut settings.params.gain),
        "attack_seconds" => parse_f32(value, &mut settings.params.attack_seconds),
        "decay_seconds" => parse_f32(value, &mut settings.params.decay_seconds),
        "sustain_level" => parse_f32(value, &mut settings.params.sustain_level),
        "release_seconds" => parse_f32(value, &mut settings.params.release_seconds),
        "waveform" => {
            if let Some(wf) = parse_waveform(value) {
                settings.params.waveform = wf;
            }
        }
        "filter_cutoff_hz" => parse_f32(value, &mut settings.params.filter_cutoff_hz),
        "filter_resonance" => parse_f32(value, &mut settings.params.filter_resonance),
        "vibrato_depth_semitones" => parse_f32(value, &mut settings.params.vibrato_depth_semitones),
        "vibrato_rate_hz" => parse_f32(value, &mut settings.params.vibrato_rate_hz),
        "unison_spread_cents" => parse_f32(value, &mut settings.params.unison_spread_cents),
        "autotune_amount" => parse_f32(value, &mut settings.params.autotune_amount),
        "noise_mix" => parse_f32(value, &mut settings.params.noise_mix),
        "eq_low_gain_db" => parse_f32(value, &mut settings.params.eq_low_gain_db),
        "eq_low_freq_hz" => parse_f32(value, &mut settings.params.eq_low_freq_hz),
        "eq_mid_gain_db" => parse_f32(value, &mut settings.params.eq_mid_gain_db),
        "eq_mid_freq_hz" => parse_f32(value, &mut settings.params.eq_mid_freq_hz),
        "eq_mid_q" => parse_f32(value, &mut settings.params.eq_mid_q),
        "eq_high_gain_db" => parse_f32(value, &mut settings.params.eq_high_gain_db),
        "eq_high_freq_hz" => parse_f32(value, &mut settings.params.eq_high_freq_hz),
        _ => {}
    }
}

fn parse_f32(value: &str, target: &mut f32) {
    if let Ok(v) = value.parse::<f32>() {
        *target = v;
    }
}

fn parse_waveform(value: &str) -> Option<Waveform> {
    match value.to_ascii_lowercase().as_str() {
        "sine" => Some(Waveform::Sine),
        "square" => Some(Waveform::Square),
        "saw" | "sawtooth" => Some(Waveform::Saw),
        "triangle" => Some(Waveform::Triangle),
        _ => None,
    }
}

fn append_param_lines(buf: &mut String, params: &SynthParams) {
    buf.push_str(&format!("gain={}\n", params.gain));
    buf.push_str(&format!("attack_seconds={}\n", params.attack_seconds));
    buf.push_str(&format!("decay_seconds={}\n", params.decay_seconds));
    buf.push_str(&format!("sustain_level={}\n", params.sustain_level));
    buf.push_str(&format!("release_seconds={}\n", params.release_seconds));
    buf.push_str(&format!("waveform={}\n", waveform_key(params.waveform)));
    buf.push_str(&format!("filter_cutoff_hz={}\n", params.filter_cutoff_hz));
    buf.push_str(&format!("filter_resonance={}\n", params.filter_resonance));
    buf.push_str(&format!(
        "vibrato_depth_semitones={}\n",
        params.vibrato_depth_semitones
    ));
    buf.push_str(&format!("vibrato_rate_hz={}\n", params.vibrato_rate_hz));
    buf.push_str(&format!(
        "unison_spread_cents={}\n",
        params.unison_spread_cents
    ));
    buf.push_str(&format!("autotune_amount={}\n", params.autotune_amount));
    buf.push_str(&format!("noise_mix={}\n", params.noise_mix));
    buf.push_str(&format!("eq_low_gain_db={}\n", params.eq_low_gain_db));
    buf.push_str(&format!("eq_low_freq_hz={}\n", params.eq_low_freq_hz));
    buf.push_str(&format!("eq_mid_gain_db={}\n", params.eq_mid_gain_db));
    buf.push_str(&format!("eq_mid_freq_hz={}\n", params.eq_mid_freq_hz));
    buf.push_str(&format!("eq_mid_q={}\n", params.eq_mid_q));
    buf.push_str(&format!("eq_high_gain_db={}\n", params.eq_high_gain_db));
    buf.push_str(&format!("eq_high_freq_hz={}\n", params.eq_high_freq_hz));
}

fn waveform_key(waveform: Waveform) -> &'static str {
    match waveform {
        Waveform::Sine => "sine",
        Waveform::Square => "square",
        Waveform::Saw => "saw",
        Waveform::Triangle => "triangle",
    }
}

fn config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(roaming) = std::env::var("APPDATA") {
            return PathBuf::from(roaming).join("Angel");
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("Angel");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = home_dir() {
            return home
                .join("Library")
                .join("Application Support")
                .join("Angel");
        }
    }

    // Default: XDG-ish on Linux/other Unix
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("angel");
    }

    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("angel")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from).or_else(|| {
        #[cfg(target_os = "windows")]
        {
            std::env::var("USERPROFILE").ok().map(PathBuf::from)
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    })
}
