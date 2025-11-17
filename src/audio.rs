use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::scope::ScopeBuffer;
use crate::synth::{SynthEngine, SynthShared};

const SNAPSHOT_REFRESH_INTERVAL: usize = 64;

#[derive(Clone)]
pub struct SynthAudio {
    _stream: Arc<cpal::Stream>,
    pub device_name: String,
}

impl SynthAudio {
    pub fn new(
        shared: Arc<Mutex<SynthShared>>,
        scope: Arc<Mutex<ScopeBuffer>>,
    ) -> Result<Self, String> {
        Self::new_with_device(shared, scope, None)
    }

    pub fn new_with_device(
        shared: Arc<Mutex<SynthShared>>,
        scope: Arc<Mutex<ScopeBuffer>>,
        device_name: Option<&str>,
    ) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = select_output_device(&host, device_name)?;
        let resolved_device_name = device
            .name()
            .unwrap_or_else(|_| "<unknown output>".to_string());
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
                let scope_state = Arc::clone(&scope);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _| {
                            write_samples_f32(
                                &shared_state,
                                &mut engine,
                                data,
                                channels,
                                &scope_state,
                            );
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|err| format!("Failed to build f32 stream: {err}"))?
            }
            cpal::SampleFormat::I16 => {
                let mut engine = SynthEngine::new(sample_rate);
                let shared_state = Arc::clone(&shared);
                let scope_state = Arc::clone(&scope);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [i16], _| {
                            write_samples_i16(
                                &shared_state,
                                &mut engine,
                                data,
                                channels,
                                &scope_state,
                            );
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|err| format!("Failed to build i16 stream: {err}"))?
            }
            cpal::SampleFormat::U16 => {
                let mut engine = SynthEngine::new(sample_rate);
                let shared_state = Arc::clone(&shared);
                let scope_state = Arc::clone(&scope);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [u16], _| {
                            write_samples_u16(
                                &shared_state,
                                &mut engine,
                                data,
                                channels,
                                &scope_state,
                            );
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
            device_name: resolved_device_name,
        })
    }
}

fn select_output_device(host: &cpal::Host, name: Option<&str>) -> Result<cpal::Device, String> {
    if let Some(target) = name {
        if let Ok(devices) = host.output_devices() {
            for device in devices {
                if let Ok(device_name) = device.name() {
                    if device_name == target {
                        return Ok(device);
                    }
                }
            }
        }
        return Err(format!("Output device '{target}' not found"));
    }

    host.default_output_device()
        .ok_or_else(|| "No audio output device available".to_string())
}

pub fn list_output_device_names() -> Vec<String> {
    let host = cpal::default_host();
    host.output_devices()
        .map(|devices| {
            devices
                .filter_map(|d| d.name().ok())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn write_samples_f32(
    shared: &Arc<Mutex<SynthShared>>,
    engine: &mut SynthEngine,
    buffer: &mut [f32],
    channels: usize,
    scope: &Arc<Mutex<ScopeBuffer>>,
) {
    let mut snapshot = {
        let shared = shared.lock().expect("Synth parameters poisoned");
        shared.snapshot()
    };
    engine.update_eq(&snapshot.params);
    let mut scope_block = Vec::with_capacity(buffer.len() / channels);

    for (i, frame) in buffer.chunks_mut(channels).enumerate() {
        if i % SNAPSHOT_REFRESH_INTERVAL == 0 {
            snapshot = {
                let shared = shared.lock().expect("Synth parameters poisoned");
                shared.snapshot()
            };
            engine.update_eq(&snapshot.params);
        }
        let sample = engine.next_sample(&snapshot);
        scope_block.push(sample);
        for channel in frame {
            *channel = sample;
        }
    }
    record_scope(scope, &scope_block);
}

fn write_samples_i16(
    shared: &Arc<Mutex<SynthShared>>,
    engine: &mut SynthEngine,
    buffer: &mut [i16],
    channels: usize,
    scope: &Arc<Mutex<ScopeBuffer>>,
) {
    let mut snapshot = {
        let shared = shared.lock().expect("Synth parameters poisoned");
        shared.snapshot()
    };
    engine.update_eq(&snapshot.params);
    let mut scope_block = Vec::with_capacity(buffer.len() / channels);

    for (i, frame) in buffer.chunks_mut(channels).enumerate() {
        if i % SNAPSHOT_REFRESH_INTERVAL == 0 {
            snapshot = {
                let shared = shared.lock().expect("Synth parameters poisoned");
                shared.snapshot()
            };
            engine.update_eq(&snapshot.params);
        }
        let sample = engine.next_sample(&snapshot);
        scope_block.push(sample);
        let scaled = (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        for channel in frame {
            *channel = scaled;
        }
    }
    record_scope(scope, &scope_block);
}

fn write_samples_u16(
    shared: &Arc<Mutex<SynthShared>>,
    engine: &mut SynthEngine,
    buffer: &mut [u16],
    channels: usize,
    scope: &Arc<Mutex<ScopeBuffer>>,
) {
    let mut snapshot = {
        let shared = shared.lock().expect("Synth parameters poisoned");
        shared.snapshot()
    };
    engine.update_eq(&snapshot.params);
    let mut scope_block = Vec::with_capacity(buffer.len() / channels);

    for (i, frame) in buffer.chunks_mut(channels).enumerate() {
        if i % SNAPSHOT_REFRESH_INTERVAL == 0 {
            snapshot = {
                let shared = shared.lock().expect("Synth parameters poisoned");
                shared.snapshot()
            };
            engine.update_eq(&snapshot.params);
        }
        let normalized = engine.next_sample(&snapshot).clamp(-1.0, 1.0);
        scope_block.push(normalized);
        let value = ((normalized * 0.5 + 0.5) * u16::MAX as f32) as u16;
        for channel in frame {
            *channel = value;
        }
    }
    record_scope(scope, &scope_block);
}

fn record_scope(scope: &Arc<Mutex<ScopeBuffer>>, block: &[f32]) {
    if let Ok(mut buffer) = scope.lock() {
        buffer.record(block);
    }
}
