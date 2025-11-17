use std::fs;
use std::path::{Path, PathBuf};

use crate::synth::{SynthParams, Waveform};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    Fl,
    Light,
    Midnight,
    Sunset,
    Neon,
    Forest,
    Ocean,
    Vapor,
    Mono,
    Pastel,
    SolarizedDark,
    SolarizedLight,
    Industrial,
    Candy,
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutMode {
    Stacked,
    TwoColumn,
    ThreeColumn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeybindScheme {
    Default,
    Vim,
    Emacs,
    Sublime,
    VSCode,
}

impl ThemeKind {
    pub const ALL: [ThemeKind; 15] = [
        ThemeKind::Fl,
        ThemeKind::Light,
        ThemeKind::Midnight,
        ThemeKind::Sunset,
        ThemeKind::Neon,
        ThemeKind::Forest,
        ThemeKind::Ocean,
        ThemeKind::Vapor,
        ThemeKind::Mono,
        ThemeKind::Pastel,
        ThemeKind::SolarizedDark,
        ThemeKind::SolarizedLight,
        ThemeKind::Industrial,
        ThemeKind::Candy,
        ThemeKind::Terminal,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeKind::Fl => "FL Dark",
            ThemeKind::Light => "Light",
            ThemeKind::Midnight => "Midnight",
            ThemeKind::Sunset => "Sunset",
            ThemeKind::Neon => "Neon",
            ThemeKind::Forest => "Forest",
            ThemeKind::Ocean => "Ocean",
            ThemeKind::Vapor => "Vapor",
            ThemeKind::Mono => "Mono",
            ThemeKind::Pastel => "Pastel",
            ThemeKind::SolarizedDark => "Solarized Dark",
            ThemeKind::SolarizedLight => "Solarized Light",
            ThemeKind::Industrial => "Industrial",
            ThemeKind::Candy => "Candy",
            ThemeKind::Terminal => "Terminal",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "light" => ThemeKind::Light,
            "midnight" => ThemeKind::Midnight,
            "sunset" => ThemeKind::Sunset,
            "neon" => ThemeKind::Neon,
            "forest" => ThemeKind::Forest,
            "ocean" => ThemeKind::Ocean,
            "vapor" => ThemeKind::Vapor,
            "mono" => ThemeKind::Mono,
            "pastel" => ThemeKind::Pastel,
            "solarized_dark" | "solarized-dark" => ThemeKind::SolarizedDark,
            "solarized_light" | "solarized-light" => ThemeKind::SolarizedLight,
            "industrial" => ThemeKind::Industrial,
            "candy" => ThemeKind::Candy,
            "terminal" => ThemeKind::Terminal,
            _ => ThemeKind::Fl,
        }
    }

    pub fn as_key(self) -> &'static str {
        match self {
            ThemeKind::Fl => "fl",
            ThemeKind::Light => "light",
            ThemeKind::Midnight => "midnight",
            ThemeKind::Sunset => "sunset",
            ThemeKind::Neon => "neon",
            ThemeKind::Forest => "forest",
            ThemeKind::Ocean => "ocean",
            ThemeKind::Vapor => "vapor",
            ThemeKind::Mono => "mono",
            ThemeKind::Pastel => "pastel",
            ThemeKind::SolarizedDark => "solarized_dark",
            ThemeKind::SolarizedLight => "solarized_light",
            ThemeKind::Industrial => "industrial",
            ThemeKind::Candy => "candy",
            ThemeKind::Terminal => "terminal",
        }
    }
}

impl LayoutMode {
    pub const ALL: [LayoutMode; 3] = [
        LayoutMode::Stacked,
        LayoutMode::TwoColumn,
        LayoutMode::ThreeColumn,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LayoutMode::Stacked => "Stacked",
            LayoutMode::TwoColumn => "Two columns",
            LayoutMode::ThreeColumn => "Three columns",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "stacked" => LayoutMode::Stacked,
            "two_column" | "two-column" => LayoutMode::TwoColumn,
            _ => LayoutMode::ThreeColumn,
        }
    }

    pub fn as_key(self) -> &'static str {
        match self {
            LayoutMode::Stacked => "stacked",
            LayoutMode::TwoColumn => "two_column",
            LayoutMode::ThreeColumn => "three_column",
        }
    }
}

impl KeybindScheme {
    pub const ALL: [KeybindScheme; 5] = [
        KeybindScheme::Default,
        KeybindScheme::Vim,
        KeybindScheme::Emacs,
        KeybindScheme::Sublime,
        KeybindScheme::VSCode,
    ];

    pub fn label(self) -> &'static str {
        match self {
            KeybindScheme::Default => "Default",
            KeybindScheme::Vim => "Vim",
            KeybindScheme::Emacs => "Emacs",
            KeybindScheme::Sublime => "Sublime",
            KeybindScheme::VSCode => "VS Code",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "vim" => KeybindScheme::Vim,
            "emacs" => KeybindScheme::Emacs,
            "sublime" => KeybindScheme::Sublime,
            "vscode" | "vs code" | "vs_code" => KeybindScheme::VSCode,
            _ => KeybindScheme::Default,
        }
    }

    pub fn as_key(self) -> &'static str {
        match self {
            KeybindScheme::Default => "default",
            KeybindScheme::Vim => "vim",
            KeybindScheme::Emacs => "emacs",
            KeybindScheme::Sublime => "sublime",
            KeybindScheme::VSCode => "vscode",
        }
    }
}

#[derive(Clone)]
pub struct AppSettings {
    pub theme: ThemeKind,
    pub params: SynthParams,
    pub output_device: Option<String>,
    pub layout_mode: LayoutMode,
    pub card_padding: f32,
    pub card_rounding: f32,
    pub scope_height: f32,
    pub keyboard_scale: f32,
    pub keybinds: KeybindScheme,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeKind::Fl,
            params: SynthParams::default(),
            output_device: None,
            layout_mode: LayoutMode::ThreeColumn,
            card_padding: 12.0,
            card_rounding: 8.0,
            scope_height: 140.0,
            keyboard_scale: 1.0,
            keybinds: KeybindScheme::Default,
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
        buf.push_str(&format!("layout_mode={}\n", self.layout_mode.as_key()));
        buf.push_str(&format!("card_padding={}\n", self.card_padding));
        buf.push_str(&format!("card_rounding={}\n", self.card_rounding));
        buf.push_str(&format!("scope_height={}\n", self.scope_height));
        buf.push_str(&format!("keyboard_scale={}\n", self.keyboard_scale));
        buf.push_str(&format!("keybinds={}\n", self.keybinds.as_key()));
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
        "layout_mode" => settings.layout_mode = LayoutMode::from_str(value),
        "card_padding" => parse_f32(value, &mut settings.card_padding),
        "card_rounding" => parse_f32(value, &mut settings.card_rounding),
        "scope_height" => parse_f32(value, &mut settings.scope_height),
        "keyboard_scale" => parse_f32(value, &mut settings.keyboard_scale),
        "keybinds" => settings.keybinds = KeybindScheme::from_str(value),
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
