use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use egui::{self, Align2, Color32, ComboBox, FontId, Id, Layout, Rounding, Stroke};

use crate::audio::{SynthAudio, list_output_device_names};
use crate::scope::ScopeBuffer;
use crate::settings::{AppSettings, KeybindScheme, LayoutMode, ThemeKind};
use crate::synth::{InstrumentKind, SynthParams, SynthShared, Waveform};

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
    settings_open: bool,
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
            settings_open: false,
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
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
        let keyboard_events = collect_keyboard_events(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(4.0);
            let mut theme_changed = false;
            let mut reset_requested = false;
            let mut device_changed = false;
            let mut layout_changed = false;
            let mut keybinds_changed = false;

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.with_layout(Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.strong("Angel Synth");
                            ui.label("FL-style minimal layout · Left/Right = octave");
                            ui.separator();
                            if ui.button("Settings").clicked() {
                                self.settings_open = true;
                            }
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
                            ui.separator();
                            keybinds_changed = keybind_selector(ui, &mut self.settings);
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                if let Some(key) = self.last_key {
                                    ui.label(format!("Last key: {:?}", key));
                                } else {
                                    ui.label("Play with keyboard or mouse");
                                }
                            });
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

                    fl_card(
                        ui,
                        "Wave Scope",
                        self.settings.card_padding,
                        self.settings.card_rounding,
                        |ui| draw_scope(ui, self.settings.scope_height, &self.scope),
                    );
                    ui.add_space(8.0);
                    fl_card(
                        ui,
                        "Keyboard",
                        self.settings.card_padding,
                        self.settings.card_rounding,
                        |ui| {
                            if self.settings_open {
                                ui.label("Keyboard disabled while settings are open.");
                            } else {
                                draw_piano(
                                    ui,
                                    ctx,
                                    &mut shared,
                                    &mut self.mouse_note,
                                    self.settings.keyboard_scale,
                                )
                            }
                        },
                    );
                    ui.add_space(10.0);

                    layout_changed |= layout_grid(ui, &mut shared, &mut self.settings);

                    let new_params = shared.params.clone();
                    let params_changed = new_params != self.settings.params;
                    drop(shared);

                    if device_changed {
                        if let Err(err) = self.switch_output_device() {
                            self.audio_error = Some(err);
                        }
                    }

                    if params_changed
                        || theme_changed
                        || device_changed
                        || layout_changed
                        || keybinds_changed
                    {
                        self.settings.params = new_params;
                        self.settings.output_device = Some(self._audio.device_name.clone());
                        self.settings.save(&self.settings_path);
                    }

                    if let Some(err) = &self.audio_error {
                        ui.colored_label(Color32::RED, format!("Audio: {err}"));
                    }
                });
        });

        settings_popup(ctx, self);
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
        ui.label("Instrument");
        ComboBox::from_id_source("instrument")
            .selected_text(shared.params.instrument.label())
            .show_ui(ui, |ui| {
                for instrument in InstrumentKind::ALL {
                    ui.selectable_value(
                        &mut shared.params.instrument,
                        instrument,
                        instrument.label(),
                    );
                }
            });
    });

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

fn draw_scope(ui: &mut egui::Ui, height: f32, scope: &Arc<Mutex<ScopeBuffer>>) {
    let desired = egui::vec2(ui.available_width().max(200.0), height);
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
    scale: f32,
) {
    let white_key_count = (LOWEST_NOTE..=HIGHEST_NOTE)
        .filter(|n| !is_black(*n))
        .count();
    let aspect = BASE_WHITE_KEY_HEIGHT / BASE_WHITE_KEY_WIDTH;
    let available_width = ui.available_width().max(white_key_count as f32 * 12.0);
    let white_key_width =
        (available_width / white_key_count as f32).clamp(16.0, 80.0) * scale.clamp(0.7, 1.4);
    let white_key_height = white_key_width * aspect * scale.clamp(0.7, 1.4);
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

fn keybind_selector(ui: &mut egui::Ui, settings: &mut AppSettings) -> bool {
    let before = settings.keybinds;
    ComboBox::from_id_source("keybinds_selector")
        .selected_text(settings.keybinds.label())
        .show_ui(ui, |ui| {
            for scheme in KeybindScheme::ALL {
                ui.selectable_value(&mut settings.keybinds, scheme, scheme.label());
            }
        });
    settings.keybinds != before
}

fn settings_popup(ctx: &egui::Context, app: &mut SynthApp) {
    if app.settings_open {
        egui::Window::new("Settings")
            .open(&mut app.settings_open)
            .collapsible(false)
            .resizable(true)
            .default_size(egui::vec2(420.0, 320.0))
            .show(ctx, |ui| {
                ui.heading("App Settings");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Theme");
                    let _ = theme_selector(ui, ctx, &mut app.settings);
                });
                ui.horizontal(|ui| {
                    ui.label("Keybinds");
                    let _ = keybind_selector(ui, &mut app.settings);
                });
                ui.horizontal(|ui| {
                    ui.label("Output");
                    let _ = output_selector(
                        ui,
                        &app.output_devices,
                        &mut app.settings.output_device,
                        &mut app.audio_error,
                    );
                });
                ui.separator();
                ui.label("Layout & sizing");
                let _ = layout_controls(ui, &mut app.settings);
            });
    }
}

fn layout_grid(ui: &mut egui::Ui, shared: &mut SynthShared, settings: &mut AppSettings) -> bool {
    let mut changed = false;
    let resolved = match settings.layout_mode {
        LayoutMode::Auto => auto_layout_for_width(ui.available_width()),
        other => other,
    };

    match resolved {
        LayoutMode::Stacked => {
            changed |= layout_controls(ui, settings);
            ui.add_space(6.0);
            fl_card(
                ui,
                "Tone & Filter",
                settings.card_padding,
                settings.card_rounding,
                |ui| tone_controls(ui, shared),
            );
            ui.add_space(6.0);
            fl_card(
                ui,
                "Motion & Noise",
                settings.card_padding,
                settings.card_rounding,
                |ui| modulation_controls(ui, shared),
            );
            ui.add_space(6.0);
            fl_card(
                ui,
                "EQ",
                settings.card_padding,
                settings.card_rounding,
                |ui| eq_controls(ui, shared),
            );
        }
        LayoutMode::TwoColumn => {
            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    changed |= layout_controls(ui, settings);
                    fl_card(
                        ui,
                        "Tone & Filter",
                        settings.card_padding,
                        settings.card_rounding,
                        |ui| tone_controls(ui, shared),
                    );
                    ui.add_space(6.0);
                    fl_card(
                        ui,
                        "Motion & Noise",
                        settings.card_padding,
                        settings.card_rounding,
                        |ui| modulation_controls(ui, shared),
                    );
                });
                columns[1].vertical(|ui| {
                    fl_card(
                        ui,
                        "EQ",
                        settings.card_padding,
                        settings.card_rounding,
                        |ui| eq_controls(ui, shared),
                    );
                });
            });
        }
        LayoutMode::ThreeColumn | LayoutMode::Auto => {
            ui.columns(3, |columns| {
                columns[0].vertical(|ui| {
                    fl_card(
                        ui,
                        "Tone & Filter",
                        settings.card_padding,
                        settings.card_rounding,
                        |ui| tone_controls(ui, shared),
                    );
                });
                columns[1].vertical(|ui| {
                    fl_card(
                        ui,
                        "Motion & Noise",
                        settings.card_padding,
                        settings.card_rounding,
                        |ui| modulation_controls(ui, shared),
                    );
                });
                columns[2].vertical(|ui| {
                    fl_card(
                        ui,
                        "EQ",
                        settings.card_padding,
                        settings.card_rounding,
                        |ui| eq_controls(ui, shared),
                    );
                    ui.add_space(6.0);
                    changed |= layout_controls(ui, settings);
                });
            });
        }
    }
    changed
}

fn layout_controls(ui: &mut egui::Ui, settings: &mut AppSettings) -> bool {
    let mut changed = false;
    fl_card(
        ui,
        "Layout",
        settings.card_padding,
        settings.card_rounding,
        |ui| {
            let before = settings.layout_mode;
            ComboBox::from_id_source("layout_mode")
                .selected_text(settings.layout_mode.label())
                .show_ui(ui, |ui| {
                    for mode in LayoutMode::ALL {
                        ui.selectable_value(&mut settings.layout_mode, mode, mode.label());
                    }
                });
            if settings.layout_mode != before {
                changed = true;
            }

            if ui
                .add(
                    egui::Slider::new(&mut settings.card_padding, 4.0..=24.0)
                        .text("Card padding (px)"),
                )
                .changed()
            {
                changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut settings.card_rounding, 0.0..=18.0)
                        .text("Card rounding (px)"),
                )
                .changed()
            {
                changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut settings.scope_height, 80.0..=220.0)
                        .text("Scope height (px)"),
                )
                .changed()
            {
                changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut settings.keyboard_scale, 0.7..=1.4)
                        .text("Keyboard height scale"),
                )
                .changed()
            {
                changed = true;
            }
        },
    );
    changed
}

fn auto_layout_for_width(width: f32) -> LayoutMode {
    if width < 720.0 {
        LayoutMode::Stacked
    } else if width < 1080.0 {
        LayoutMode::TwoColumn
    } else {
        LayoutMode::ThreeColumn
    }
}
fn fl_card(
    ui: &mut egui::Ui,
    title: &str,
    padding: f32,
    rounding: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let visuals = ui.visuals().clone();
    egui::Frame::none()
        .fill(visuals.widgets.noninteractive.bg_fill)
        .stroke(visuals.widgets.noninteractive.bg_stroke)
        .rounding(Rounding::same(rounding.clamp(0.0, 18.0)))
        .inner_margin(egui::Margin::same(padding.clamp(4.0, 24.0)))
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
            for theme in ThemeKind::ALL {
                ui.selectable_value(&mut selected, theme, theme.label());
            }
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
        ThemeKind::Midnight => apply_midnight_theme(ctx),
        ThemeKind::Sunset => apply_sunset_theme(ctx),
        ThemeKind::Neon => apply_neon_theme(ctx),
        ThemeKind::Forest => apply_forest_theme(ctx),
        ThemeKind::Ocean => apply_ocean_theme(ctx),
        ThemeKind::Vapor => apply_vapor_theme(ctx),
        ThemeKind::Mono => apply_mono_theme(ctx),
        ThemeKind::Pastel => apply_pastel_theme(ctx),
        ThemeKind::SolarizedDark => apply_solarized_dark(ctx),
        ThemeKind::SolarizedLight => apply_solarized_light(ctx),
        ThemeKind::Industrial => apply_industrial_theme(ctx),
        ThemeKind::Candy => apply_candy_theme(ctx),
        ThemeKind::Terminal => apply_terminal_theme(ctx),
    }
}

fn apply_fl_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::dark(),
        Some(Color32::from_rgb(235, 235, 235)),
        ACCENT,
        Stroke::new(1.0, Color32::from_rgb(12, 12, 12)),
        Color32::from_rgb(18, 18, 18),
        Color32::from_rgb(30, 30, 30),
        Color32::from_rgb(40, 40, 40),
        Color32::from_rgb(45, 45, 45),
        Stroke::new(1.0, Color32::from_rgb(60, 60, 60)),
        Color32::from_rgb(14, 14, 14),
    );
}

fn apply_light_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::light(),
        None,
        Color32::from_rgb(255, 187, 92),
        Stroke::new(1.0, Color32::from_rgb(70, 50, 20)),
        Color32::from_rgb(245, 245, 245),
        Color32::from_rgb(250, 250, 250),
        Color32::from_rgb(240, 240, 240),
        Color32::from_rgb(235, 235, 235),
        Stroke::new(1.0, Color32::from_rgb(200, 200, 200)),
        Color32::from_rgb(250, 250, 250),
    );
}

fn apply_midnight_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::dark(),
        Some(Color32::from_rgb(220, 230, 245)),
        Color32::from_rgb(80, 135, 255),
        Stroke::new(1.0, Color32::from_rgb(10, 25, 60)),
        Color32::from_rgb(12, 16, 28),
        Color32::from_rgb(22, 30, 48),
        Color32::from_rgb(32, 42, 64),
        Color32::from_rgb(40, 52, 78),
        Stroke::new(1.0, Color32::from_rgb(50, 70, 110)),
        Color32::from_rgb(10, 14, 24),
    );
}

fn apply_sunset_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::dark(),
        Some(Color32::from_rgb(245, 230, 220)),
        Color32::from_rgb(255, 109, 96),
        Stroke::new(1.0, Color32::from_rgb(25, 10, 10)),
        Color32::from_rgb(20, 16, 24),
        Color32::from_rgb(32, 26, 36),
        Color32::from_rgb(42, 34, 48),
        Color32::from_rgb(50, 40, 56),
        Stroke::new(1.0, Color32::from_rgb(80, 60, 70)),
        Color32::from_rgb(18, 14, 22),
    );
}

fn apply_neon_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::dark(),
        Some(Color32::from_rgb(220, 255, 240)),
        Color32::from_rgb(0, 255, 170),
        Stroke::new(1.0, Color32::from_rgb(10, 40, 30)),
        Color32::from_rgb(10, 12, 20),
        Color32::from_rgb(18, 20, 30),
        Color32::from_rgb(30, 32, 50),
        Color32::from_rgb(40, 42, 60),
        Stroke::new(1.0, Color32::from_rgb(40, 90, 70)),
        Color32::from_rgb(8, 10, 18),
    );
}

fn apply_forest_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::dark(),
        Some(Color32::from_rgb(220, 235, 210)),
        Color32::from_rgb(120, 200, 120),
        Stroke::new(1.0, Color32::from_rgb(30, 70, 30)),
        Color32::from_rgb(14, 20, 16),
        Color32::from_rgb(24, 34, 26),
        Color32::from_rgb(32, 46, 36),
        Color32::from_rgb(38, 54, 42),
        Stroke::new(1.0, Color32::from_rgb(50, 85, 55)),
        Color32::from_rgb(10, 16, 12),
    );
}

fn apply_ocean_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::dark(),
        Some(Color32::from_rgb(215, 240, 255)),
        Color32::from_rgb(80, 200, 255),
        Stroke::new(1.0, Color32::from_rgb(15, 45, 70)),
        Color32::from_rgb(10, 18, 26),
        Color32::from_rgb(18, 28, 40),
        Color32::from_rgb(26, 38, 52),
        Color32::from_rgb(32, 46, 60),
        Stroke::new(1.0, Color32::from_rgb(50, 90, 120)),
        Color32::from_rgb(8, 14, 22),
    );
}

fn apply_vapor_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::dark(),
        Some(Color32::from_rgb(238, 220, 255)),
        Color32::from_rgb(255, 105, 180),
        Stroke::new(1.0, Color32::from_rgb(60, 10, 40)),
        Color32::from_rgb(18, 12, 26),
        Color32::from_rgb(26, 18, 36),
        Color32::from_rgb(34, 24, 46),
        Color32::from_rgb(42, 30, 56),
        Stroke::new(1.0, Color32::from_rgb(90, 50, 110)),
        Color32::from_rgb(14, 10, 22),
    );
}

fn apply_mono_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::dark(),
        Some(Color32::from_rgb(230, 230, 230)),
        Color32::from_rgb(180, 180, 180),
        Stroke::new(1.0, Color32::from_rgb(60, 60, 60)),
        Color32::from_rgb(18, 18, 18),
        Color32::from_rgb(26, 26, 26),
        Color32::from_rgb(34, 34, 34),
        Color32::from_rgb(42, 42, 42),
        Stroke::new(1.0, Color32::from_rgb(70, 70, 70)),
        Color32::from_rgb(12, 12, 12),
    );
}

fn apply_pastel_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::light(),
        None,
        Color32::from_rgb(255, 170, 200),
        Stroke::new(1.0, Color32::from_rgb(200, 130, 150)),
        Color32::from_rgb(248, 246, 242),
        Color32::from_rgb(242, 238, 232),
        Color32::from_rgb(238, 232, 226),
        Color32::from_rgb(234, 226, 220),
        Stroke::new(1.0, Color32::from_rgb(210, 200, 195)),
        Color32::from_rgb(248, 246, 242),
    );
}

fn apply_solarized_dark(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::dark(),
        Some(Color32::from_rgb(238, 232, 213)),
        Color32::from_rgb(181, 137, 0),
        Stroke::new(1.0, Color32::from_rgb(88, 110, 117)),
        Color32::from_rgb(0, 43, 54),
        Color32::from_rgb(7, 54, 66),
        Color32::from_rgb(23, 69, 79),
        Color32::from_rgb(36, 87, 100),
        Stroke::new(1.0, Color32::from_rgb(88, 110, 117)),
        Color32::from_rgb(0, 43, 54),
    );
}

fn apply_solarized_light(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::light(),
        Some(Color32::from_rgb(101, 123, 131)),
        Color32::from_rgb(181, 137, 0),
        Stroke::new(1.0, Color32::from_rgb(88, 110, 117)),
        Color32::from_rgb(253, 246, 227),
        Color32::from_rgb(238, 232, 213),
        Color32::from_rgb(231, 225, 206),
        Color32::from_rgb(223, 216, 198),
        Stroke::new(1.0, Color32::from_rgb(133, 153, 0)),
        Color32::from_rgb(253, 246, 227),
    );
}

fn apply_industrial_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::dark(),
        Some(Color32::from_rgb(220, 220, 220)),
        Color32::from_rgb(255, 140, 0),
        Stroke::new(1.0, Color32::from_rgb(80, 60, 20)),
        Color32::from_rgb(18, 18, 18),
        Color32::from_rgb(28, 28, 30),
        Color32::from_rgb(38, 38, 40),
        Color32::from_rgb(46, 46, 48),
        Stroke::new(1.0, Color32::from_rgb(70, 70, 72)),
        Color32::from_rgb(14, 14, 14),
    );
}

fn apply_candy_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::light(),
        None,
        Color32::from_rgb(255, 105, 180),
        Stroke::new(1.0, Color32::from_rgb(180, 70, 120)),
        Color32::from_rgb(252, 244, 248),
        Color32::from_rgb(248, 236, 244),
        Color32::from_rgb(244, 230, 240),
        Color32::from_rgb(240, 224, 236),
        Stroke::new(1.0, Color32::from_rgb(210, 170, 190)),
        Color32::from_rgb(252, 244, 248),
    );
}

fn apply_terminal_theme(ctx: &egui::Context) {
    apply_palette(
        ctx,
        egui::Visuals::dark(),
        Some(Color32::from_rgb(120, 255, 120)),
        Color32::from_rgb(120, 255, 120),
        Stroke::new(1.0, Color32::from_rgb(30, 80, 30)),
        Color32::from_rgb(8, 12, 8),
        Color32::from_rgb(12, 18, 12),
        Color32::from_rgb(16, 24, 16),
        Color32::from_rgb(20, 28, 20),
        Stroke::new(1.0, Color32::from_rgb(40, 80, 40)),
        Color32::from_rgb(6, 10, 6),
    );
}

fn apply_palette(
    ctx: &egui::Context,
    mut visuals: egui::Visuals,
    override_text: Option<Color32>,
    selection_fill: Color32,
    selection_stroke: Stroke,
    noninteractive: Color32,
    inactive: Color32,
    hovered: Color32,
    active: Color32,
    widget_stroke: Stroke,
    window_fill: Color32,
) {
    let mut style = (*ctx.style()).clone();
    visuals.override_text_color = override_text;
    visuals.selection.bg_fill = selection_fill;
    visuals.selection.stroke = selection_stroke;
    visuals.widgets.noninteractive.bg_fill = noninteractive;
    visuals.widgets.inactive.bg_fill = inactive;
    visuals.widgets.hovered.bg_fill = hovered;
    visuals.widgets.active.bg_fill = active;
    visuals.widgets.inactive.bg_stroke = widget_stroke;
    visuals.window_fill = window_fill;
    style.visuals = visuals;
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
