mod audio;
mod scope;
mod settings;
mod synth;
mod ui;

use std::sync::{Arc, Mutex};

use audio::SynthAudio;
use scope::ScopeBuffer;
use settings::{AppSettings, default_settings_path};
use synth::SynthShared;
use ui::SynthApp;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let settings_path = default_settings_path();
    let settings = AppSettings::load(&settings_path);

    let shared = Arc::new(Mutex::new(SynthShared::new_with_params(
        settings.params.clone(),
    )));
    let scope = Arc::new(Mutex::new(ScopeBuffer::new(4096)));
    let audio = SynthAudio::new_with_device(
        Arc::clone(&shared),
        Arc::clone(&scope),
        settings.output_device.as_deref(),
    )
    .or_else(|_| SynthAudio::new(Arc::clone(&shared), Arc::clone(&scope)))
    .expect("Failed to initialize audio output. Is an output device available?");

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Angel Piano",
        options,
        Box::new(move |cc| {
            Box::new(SynthApp::new(
                cc,
                Arc::clone(&shared),
                audio.clone(),
                Arc::clone(&scope),
                settings_path.clone(),
                settings.clone(),
            ))
        }),
    )
}
