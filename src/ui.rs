use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use egui::{self, Align2, Color32, ComboBox, FontId, Id, Layout, Rounding, Stroke};

use crate::audio::{SynthAudio, list_output_device_names};
use crate::scope::ScopeBuffer;
use crate::settings::{AppSettings, ThemeKind};
use crate::synth::{SynthParams, SynthShared, Waveform};

const LOWEST_NOTE: u8 = 36; // C2
const HIGHEST_NOTE: u8 = 84; // C6
const BASE_WHITE_KEY_WIDTH: f32 = 36.0;
const BASE_WHITE_KEY_HEIGHT: f32 = 200.0;
const BLACK_KEY_WIDTH_RATIO: f32 = 0.62;
const BLACK_KEY_HEIGHT_RATIO: f32 = 0.62;
const ACCENT: Color32 = Color32::from_rgb(255, 140, 0);

pub struct SynthApp {
    shared: Arc<Mutex<SynthShared>>,
    _audio: SynthAudio,
    mouse_note: Option<u8>,
    scope: Arc<Mutex<ScopeBuffer>>,
    last_key: Option<egui::Key>,
    octave_offset: i32,
    settings_path: PathBuf,
    settings: AppSettings,
    output_devices: Vec<String>,
    audio_error: Option<String>,
}

impl SynthApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        shared: Arc<Mutex<SynthShared>>,
        audio: SynthAudio,
        scope: Arc<Mutex<ScopeBuffer>>,
        settings_path: PathBuf,
        mut settings: AppSettings,
    ) -> Self {
        apply_theme(&cc.egui_ctx, settings.theme);
        if let Ok(mut guard) = shared.lock() {
            guard.params = settings.params.clone();
        }
        let devices = list_output_device_names();
        if settings.output_device.is_none() {
            settings.output_device = Some(audio.device_name.clone());
        }
        Self {
            shared,
            _audio: audio,
            mouse_note: None,
            scope,
            last_key: None,
            octave_offset: 0,
            settings_path,
            settings,
            output_devices: devices,
            audio_error: None,
        }
    }

    fn switch_output_device(&mut self) -> Result<(), String> {
        let target = self.settings.output_device.clone();
        let audio = SynthAudio::new_with_device(
            Arc::clone(&self.shared),
            Arc::clone(&self.scope),
            target.as_deref(),
        )?;
        self.output_devices = list_output_device_names();
        self.settings.output_device = Some(audio.device_name.clone());
        self._audio = audio;
        self.audio_error = None;
        Ok(())
    }
}

impl eframe::App for SynthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        let keyboard_events = collect_keyboard_events(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(4.0);
            let mut theme_changed = false;
            let mut reset_requested = false;
            let mut device_changed = false;
            ui.horizontal(|ui| {
                ui.strong("Angel Synth");
                ui.label("FL-style minimal layout · Left/Right = octave");
                ui.separator();
                theme_changed = theme_selector(ui, ctx, &mut self.settings);
                if ui.button("Reset sound").clicked() {
                    reset_requested = true;
                }
                ui.separator();
                device_changed = output_selector(
                    ui,
                    &self.output_devices,
                    &mut self.settings.output_device,
                    &mut self.audio_error,
                );
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(key) = self.last_key {
                        ui.label(format!("Last key: {:?}", key));
                    } else {
                        ui.label("Play with keyboard or mouse");
                    }
                });
            });
            ui.add_space(6.0);

            let mut shared = self.shared.lock().expect("Synth parameters poisoned");
            if reset_requested {
                shared.params = SynthParams::default();
            }

            handle_keyboard_events(
                &keyboard_events,
                &mut shared,
                &mut self.last_key,
                &mut self.octave_offset,
            );

            fl_card(ui, "Wave Scope", |ui| draw_scope(ui, &self.scope));
            ui.add_space(8.0);
            fl_card(ui, "Keyboard", |ui| {
                draw_piano(ui, ctx, &mut shared, &mut self.mouse_note)
            });
            ui.add_space(10.0);

            ui.columns(3, |columns| {
                columns[0].vertical(|ui| {
                    fl_card(ui, "Tone & Filter", |ui| tone_controls(ui, &mut shared))
                });
                columns[1].vertical(|ui| {
                    fl_card(ui, "Motion & Noise", |ui| {
                        modulation_controls(ui, &mut shared)
                    })
                });
                columns[2].vertical(|ui| fl_card(ui, "EQ", |ui| eq_controls(ui, &mut shared)));
            });

            let new_params = shared.params.clone();
            let params_changed = new_params != self.settings.params;
            drop(shared);

            if device_changed {
                if let Err(err) = self.switch_output_device() {
                    self.audio_error = Some(err);
                }
            }

            if params_changed || theme_changed || device_changed {
                self.settings.params = new_params;
                self.settings.output_device = Some(self._audio.device_name.clone());
                self.settings.save(&self.settings_path);
            }

            if let Some(err) = &self.audio_error {
                ui.colored_label(Color32::RED, format!("Audio: {err}"));
            }
        });
    }
}

fn tone_controls(ui: &mut egui::Ui, shared: &mut SynthShared) {
    ui.add(egui::Slider::new(&mut shared.params.gain, 0.0..=1.0).text("Master gain"));
    ui.add(
        egui::Slider::new(&mut shared.params.attack_seconds, 0.001..=1.0)
            .logarithmic(true)
            .text("Attack (s)"),
    );
    ui.add(
        egui::Slider::new(&mut shared.params.decay_seconds, 0.001..=1.5)
            .logarithmic(true)
            .text("Decay (s)"),
    );
    ui.add(egui::Slider::new(&mut shared.params.sustain_level, 0.0..=1.0).text("Sustain"));
    ui.add(
        egui::Slider::new(&mut shared.params.release_seconds, 0.01..=3.0)
            .logarithmic(true)
            .text("Release (s)"),
    );

    ui.horizontal(|ui| {
        ui.label("Waveform");
        ComboBox::from_id_source("waveform")
            .selected_text(shared.params.waveform.label())
            .show_ui(ui, |ui| {
                for waveform in Waveform::ALL {
                    ui.selectable_value(&mut shared.params.waveform, waveform, waveform.label());
                }
            });
    });

    ui.add(
        egui::Slider::new(&mut shared.params.filter_cutoff_hz, 80.0..=16_000.0)
            .logarithmic(true)
            .text("Filter cutoff (Hz)"),
    );
    ui.add(egui::Slider::new(&mut shared.params.filter_resonance, 0.0..=0.95).text("Resonance"));
}

fn modulation_controls(ui: &mut egui::Ui, shared: &mut SynthShared) {
    ui.add(
        egui::Slider::new(&mut shared.params.vibrato_rate_hz, 0.1..=12.0)
            .logarithmic(true)
            .text("Vibrato rate (Hz)"),
    );
    ui.add(
        egui::Slider::new(&mut shared.params.vibrato_depth_semitones, 0.0..=1.2)
            .text("Vibrato depth (semitones)"),
    );
    ui.add(
        egui::Slider::new(&mut shared.params.autotune_amount, 0.0..=1.0)
            .text("Autotune (0=free,1=hard)"),
    );
    ui.add(
        egui::Slider::new(&mut shared.params.unison_spread_cents, 0.0..=25.0)
            .text("Unison spread (cents)"),
    );
    ui.add(egui::Slider::new(&mut shared.params.noise_mix, 0.0..=0.5).text("Noise mix"));
}

fn eq_controls(ui: &mut egui::Ui, shared: &mut SynthShared) {
    ui.columns(2, |columns| {
        columns[0].add(
            egui::Slider::new(&mut shared.params.eq_low_gain_db, -12.0..=12.0)
                .text("Low gain (dB)"),
        );
        columns[0].add(
            egui::Slider::new(&mut shared.params.eq_low_freq_hz, 40.0..=400.0)
                .logarithmic(true)
                .text("Low cutoff (Hz)"),
        );

        columns[1].add(
            egui::Slider::new(&mut shared.params.eq_high_gain_db, -12.0..=12.0)
                .text("High gain (dB)"),
        );
        columns[1].add(
            egui::Slider::new(&mut shared.params.eq_high_freq_hz, 2_000.0..=16_000.0)
                .logarithmic(true)
                .text("High cutoff (Hz)"),
        );
    });

    ui.add(
        egui::Slider::new(&mut shared.params.eq_mid_gain_db, -12.0..=12.0).text("Mid gain (dB)"),
    );
    ui.add(
        egui::Slider::new(&mut shared.params.eq_mid_freq_hz, 200.0..=4_000.0)
            .logarithmic(true)
            .text("Mid freq (Hz)"),
    );
    ui.add(egui::Slider::new(&mut shared.params.eq_mid_q, 0.3..=2.5).text("Mid Q"));
}

fn draw_scope(ui: &mut egui::Ui, scope: &Arc<Mutex<ScopeBuffer>>) {
    let desired = egui::vec2(ui.available_width().max(200.0), 140.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect(
        rect,
        Rounding::same(6.0),
        ui.visuals().faint_bg_color,
        Stroke::new(1.0, ui.visuals().weak_text_color()),
    );

    match scope.lock() {
        Ok(buffer) => {
            let data = buffer.snapshot();
            if data.len() >= 2 {
                let len = data.len().saturating_sub(1).max(1);
                let mut points = Vec::with_capacity(data.len());
                for (i, sample) in data.iter().enumerate() {
                    let t = i as f32 / len as f32;
                    let x = egui::lerp(rect.x_range(), t);
                    let norm = (*sample + 1.0) * 0.5;
                    let y = egui::lerp(rect.y_range(), 1.0 - norm);
                    points.push(egui::pos2(x, y));
                }
                painter.add(egui::Shape::line(
                    points,
                    Stroke::new(2.0, ui.visuals().selection.bg_fill),
                ));
            } else {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "Scope warming up...",
                    FontId::proportional(14.0),
                    ui.visuals().weak_text_color(),
                );
            }
        }
        Err(_) => {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Scope busy...",
                FontId::proportional(14.0),
                ui.visuals().weak_text_color(),
            );
        }
    }
}

fn handle_keyboard_events(
    events: &[(egui::Key, bool)],
    shared: &mut SynthShared,
    last_key: &mut Option<egui::Key>,
    octave_offset: &mut i32,
) {
    for (key, pressed) in events.iter().copied() {
        *last_key = Some(key);
        match key {
            egui::Key::ArrowLeft if pressed => {
                *octave_offset = (*octave_offset - 1).clamp(-2, 2);
            }
            egui::Key::ArrowRight if pressed => {
                *octave_offset = (*octave_offset + 1).clamp(-2, 2);
            }
            _ => {
                let note = map_key_to_note(key, *octave_offset);
                if pressed {
                    shared.press_note(note);
                } else {
                    shared.release_note(note);
                }
            }
        }
    }
}

fn collect_keyboard_events(ctx: &egui::Context) -> Vec<(egui::Key, bool)> {
    let mut events = Vec::new();
    ctx.input(|input| {
        for event in &input.events {
            if let egui::Event::Key { key, pressed, .. } = event {
                events.push((*key, *pressed));
            }
        }
    });
    events
}

fn map_key_to_note(key: egui::Key, octave_offset: i32) -> u8 {
    let span = (HIGHEST_NOTE - LOWEST_NOTE + 1) as u8;
    let idx = egui::Key::ALL
        .iter()
        .position(|candidate| *candidate == key)
        .unwrap_or(0) as u8;
    let mut note = LOWEST_NOTE + (idx % span);
    let shift = (octave_offset * 12) as i32;
    note = (note as i32 + shift).clamp(LOWEST_NOTE as i32, HIGHEST_NOTE as i32) as u8;
    note
}

fn draw_piano(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    shared: &mut SynthShared,
    mouse_note: &mut Option<u8>,
) {
    let white_key_count = (LOWEST_NOTE..=HIGHEST_NOTE)
        .filter(|n| !is_black(*n))
        .count();
    let aspect = BASE_WHITE_KEY_HEIGHT / BASE_WHITE_KEY_WIDTH;
    let available_width = ui.available_width().max(white_key_count as f32 * 12.0);
    let white_key_width = (available_width / white_key_count as f32).clamp(18.0, 80.0);
    let white_key_height = white_key_width * aspect;
    let black_key_width = white_key_width * BLACK_KEY_WIDTH_RATIO;
    let black_key_height = white_key_height * BLACK_KEY_HEIGHT_RATIO;
    let desired_size = egui::vec2(white_key_count as f32 * white_key_width, white_key_height);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    let pointer_down = ui.input(|i| i.pointer.primary_down());
    let pointer_pos = ui.input(|i| i.pointer.interact_pos());

    let mut white_layout = Vec::new();
    let mut black_layout = Vec::new();
    let mut white_index = 0usize;

    for note in LOWEST_NOTE..=HIGHEST_NOTE {
        if !is_black(note) {
            let x = rect.min.x + white_index as f32 * white_key_width;
            let key_rect = egui::Rect::from_min_size(
                egui::pos2(x, rect.min.y),
                egui::vec2(white_key_width - 1.0, white_key_height),
            );
            white_layout.push((note, key_rect));
            white_index += 1;
        } else {
            let preceding_white = white_index.saturating_sub(1) as f32;
            let base = rect.min.x + preceding_white * white_key_width;
            let x = base + white_key_width * 0.7 - black_key_width / 2.0;
            let key_rect = egui::Rect::from_min_size(
                egui::pos2(x, rect.min.y),
                egui::vec2(black_key_width, black_key_height),
            );
            black_layout.push((note, key_rect));
        }
    }

    let mut pointer_note = None;
    if let (Some(pos), true) = (pointer_pos, pointer_down) {
        if rect.contains(pos) {
            for (note, key_rect) in &black_layout {
                if key_rect.contains(pos) {
                    pointer_note = Some(*note);
                    break;
                }
            }
            if pointer_note.is_none() {
                for (note, key_rect) in &white_layout {
                    if key_rect.contains(pos) {
                        pointer_note = Some(*note);
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

    let white_idle = Color32::from_rgb(250, 250, 250);
    let pressed_fill = ui.visuals().selection.bg_fill;

    for (note, key_rect) in &white_layout {
        let active = shared.is_pressed(*note);
        let anim = ctx.animate_bool(Id::new(("white", note)), active);
        let fill = blend_color(white_idle, pressed_fill, anim);
        painter.rect(*key_rect, Rounding::same(4.0), fill, (1.0, Color32::BLACK));
        painter.text(
            egui::pos2(key_rect.center().x, key_rect.max.y - 6.0),
            Align2::CENTER_BOTTOM,
            note_label(*note),
            FontId::monospace(12.0),
            Color32::from_rgb(40, 40, 40),
        );
    }

    for (note, key_rect) in &black_layout {
        let active = shared.is_pressed(*note);
        let anim = ctx.animate_bool(Id::new(("black", note)), active);
        let fill = blend_color(Color32::from_rgb(20, 20, 20), pressed_fill, anim);
        painter.rect(
            *key_rect,
            Rounding::same(3.0),
            fill,
            (1.0, Color32::from_rgb(15, 15, 15)),
        );
    }

    response.on_hover_text("Click and drag to glide");
}

fn output_selector(
    ui: &mut egui::Ui,
    devices: &[String],
    selected: &mut Option<String>,
    audio_error: &mut Option<String>,
) -> bool {
    let before = selected.clone();
    ComboBox::from_id_source("output_selector")
        .width(180.0)
        .selected_text(selected.as_deref().unwrap_or("Default output"))
        .show_ui(ui, |ui| {
            ui.selectable_value(selected, None, "Default output");
            for name in devices {
                ui.selectable_value(selected, Some(name.clone()), name);
            }
        });
    let changed = before != *selected;
    if changed {
        *audio_error = None;
    }
    changed
}

fn fl_card(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(Color32::from_rgb(24, 24, 24))
        .stroke(Stroke::new(1.0, Color32::from_rgb(45, 45, 45)))
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(ACCENT, title);
                ui.add_space(6.0);
                ui.separator();
            });
            ui.add_space(6.0);
            add_contents(ui);
        });
}

fn theme_selector(ui: &mut egui::Ui, ctx: &egui::Context, settings: &mut AppSettings) -> bool {
    let mut selected = settings.theme;
    ComboBox::from_id_source("theme_selector")
        .selected_text(selected.label())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut selected, ThemeKind::Fl, ThemeKind::Fl.label());
            ui.selectable_value(&mut selected, ThemeKind::Light, ThemeKind::Light.label());
        });

    if selected != settings.theme {
        settings.theme = selected;
        apply_theme(ctx, selected);
        true
    } else {
        false
    }
}

fn apply_theme(ctx: &egui::Context, theme: ThemeKind) {
    match theme {
        ThemeKind::Fl => apply_fl_theme(ctx),
        ThemeKind::Light => apply_light_theme(ctx),
    }
}

fn apply_fl_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.override_text_color = Some(Color32::from_rgb(235, 235, 235));
    style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(18, 18, 18);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 30, 30);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(60, 60, 60));
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(40, 40, 40);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(45, 45, 45);
    style.visuals.selection.bg_fill = ACCENT;
    style.visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(12, 12, 12));
    style.visuals.window_fill = Color32::from_rgb(14, 14, 14);
    ctx.set_style(style);
}

fn apply_light_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::light();
    style.visuals.selection.bg_fill = Color32::from_rgb(255, 187, 92);
    style.visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(70, 50, 20));
    style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(245, 245, 245);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(250, 250, 250);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(200, 200, 200));
    ctx.set_style(style);
}

fn is_black(note: u8) -> bool {
    matches!(note % 12, 1 | 3 | 6 | 8 | 10)
}

fn note_label(note: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (note / 12).saturating_sub(1);
    format!("{}{}", NAMES[(note % 12) as usize], octave)
}

fn blend_color(from: Color32, to: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| -> u8 { ((a as f32) + (b as f32 - a as f32) * t).round() as u8 };
    Color32::from_rgb(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
    )
}
