use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use rdev::{listen, Event, EventType, Key};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

// Path to the Whisper model file
const WHISPER_MODEL_PATH: &str = "models/ggml-base.en.bin";

// Global state to track if we're currently recording
static APP_STATE: Mutex<AppState> = Mutex::new(AppState::Idle);

// Audio recording buffer and config
lazy_static::lazy_static! {
    static ref AUDIO_BUFFER: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    static ref SAMPLE_RATE: Mutex<u32> = Mutex::new(48000);
    static ref CHANNELS: Mutex<u16> = Mutex::new(1);
    static ref WHISPER_CTX: Mutex<Option<WhisperContext>> = Mutex::new(None);
}

// 1. Define our States
#[derive(Debug, Clone, Copy, PartialEq)]
enum AppState {
    Idle,
    Recording,
    Processing,
}

fn main() {
    println!("Lacon is running... (Press F1 to test)");

    // Load Whisper model
    println!("Loading Whisper model from {}...", WHISPER_MODEL_PATH);
    match WhisperContext::new_with_params(
        WHISPER_MODEL_PATH,
        WhisperContextParameters::default(),
    ) {
        Ok(ctx) => {
            *WHISPER_CTX.lock().unwrap() = Some(ctx);
            println!("✓ Whisper model loaded successfully");
        }
        Err(e) => {
            eprintln!("Failed to load Whisper model: {}", e);
            eprintln!("Please download the model file first. See README.md for instructions.");
            return;
        }
    }

    // Initialize audio device and config
    if let Err(e) = init_audio() {
        eprintln!("Failed to initialize audio: {}", e);
        return;
    }

    // We start a separate thread to listen for keyboard events
    // because the listener blocks the thread it runs on.
    if let Err(error) = listen(callback) {
        println!("Error: {:?}", error);
    }
}

fn init_audio() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or("No input device available")?;
    
    println!("Using input device: {}", device.name()?);
    
    let config = device.default_input_config()?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels();
    
    println!("Sample rate: {} Hz, Channels: {}", sample_rate, channels);
    
    // Store the sample rate and channels
    *SAMPLE_RATE.lock().unwrap() = sample_rate;
    *CHANNELS.lock().unwrap() = channels;
    
    Ok(())
}

// This function fires every time you press ANY key on your keyboard
fn callback(event: Event) {
    match event.event_type {
        EventType::KeyPress(key) => {
            if key == Key::F1 {
                let mut state = APP_STATE.lock().unwrap();
                
                // Only start recording if we're idle (ignore key repeat)
                if *state == AppState::Idle {
                    *state = AppState::Recording;
                    println!("[Signal] F1 Pressed -> Starting Recording...");
                    
                    if let Err(e) = start_recording() {
                        eprintln!("Failed to start recording: {}", e);
                        *state = AppState::Idle;
                    }
                }
            }
        }
        EventType::KeyRelease(key) => {
            if key == Key::F1 {
                let mut state = APP_STATE.lock().unwrap();
                
                // Only process if we were recording
                if *state == AppState::Recording {
                    *state = AppState::Processing;
                    println!("[Signal] F1 Released -> Stopping & Processing...");
                    
                    // Stop recording and save the file
                    let wav_path = match stop_recording_and_save() {
                        Ok(path) => path,
                        Err(e) => {
                            eprintln!("Failed to save recording: {}", e);
                            *state = AppState::Idle;
                            return;
                        }
                    };
                    
                    // Process the audio pipeline
                    thread::spawn(move || {
                        process_audio_pipeline(&wav_path);
                        
                        // After processing, go back to Idle
                        let mut state = APP_STATE.lock().unwrap();
                        *state = AppState::Idle;
                    });
                }
            }
        }
        _ => {} // Ignore other events (mouse, etc.)
    }
}

fn start_recording() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or("No input device available")?;
    
    let config = device.default_input_config()?;
    
    // Clear the buffer for new recording
    AUDIO_BUFFER.lock().unwrap().clear();
    
    // Clone Arc for use in the callback
    let buffer_clone = Arc::clone(&AUDIO_BUFFER);
    
    // Build the input stream
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    buffer_clone.lock().unwrap().extend_from_slice(data);
                },
                |err| eprintln!("Stream error: {}", err),
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mut buffer = buffer_clone.lock().unwrap();
                    for &sample in data {
                        buffer.push(sample as f32 / i16::MAX as f32);
                    }
                },
                |err| eprintln!("Stream error: {}", err),
                None,
            )?
        }
        cpal::SampleFormat::U16 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let mut buffer = buffer_clone.lock().unwrap();
                    for &sample in data {
                        buffer.push((sample as f32 / u16::MAX as f32) * 2.0 - 1.0);
                    }
                },
                |err| eprintln!("Stream error: {}", err),
                None,
            )?
        }
        _ => return Err("Unsupported sample format".into()),
    };
    
    stream.play()?;
    
    // NOTE: We intentionally leak the stream here because cpal::Stream is !Send on Linux (ALSA).
    // This means we can't move it to another thread or store it in a static Mutex.
    // For this PoC, the leaked stream will continue recording in the background and
    // will be cleaned up when the process exits. Each F1 press creates a new stream.
    // A production implementation would need a more sophisticated approach (e.g., dedicated audio thread).
    std::mem::forget(stream);
    
    Ok(())
}

fn stop_recording_and_save() -> Result<String, Box<dyn std::error::Error>> {
    // NOTE: Since the stream was leaked with std::mem::forget, it technically continues
    // recording in the background. For this PoC, we snapshot the buffer at this point.
    // The buffer gets cleared on the next F1 press in start_recording().
    
    // Give any pending audio samples a moment to be processed
    thread::sleep(Duration::from_millis(50));
    
    // Get the recorded samples
    let buffer = AUDIO_BUFFER.lock().unwrap();
    let sample_count = buffer.len();
    
    if sample_count == 0 {
        return Err("No audio data recorded".into());
    }
    
    println!("   -> Recorded {} samples", sample_count);
    
    // Create WAV file path
    let wav_path = "/tmp/lacon_recording.wav";
    
    // Write WAV file with correct channel count
    let sample_rate = *SAMPLE_RATE.lock().unwrap();
    let channels = *CHANNELS.lock().unwrap();
    
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let mut writer = hound::WavWriter::create(wav_path, spec)?;
    
    // Convert f32 samples to i16 and write
    for &sample in buffer.iter() {
        let sample_i16 = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(sample_i16)?;
    }
    
    writer.finalize()?;
    
    println!("   -> Saved to {} ({} channels, {} Hz)", wav_path, channels, sample_rate);
    
    Ok(wav_path.to_string())
}

fn process_audio_pipeline(wav_path: &str) {
    // Step 1: Transcribe
    println!("   -> 1. Transcribing Audio from {} (Whisper)...", wav_path);
    let raw_text = match transcribe_audio(wav_path) {
        Ok(text) => {
            println!("   -> Transcribed: \"{}\"", text);
            text
        }
        Err(e) => {
            eprintln!("   -> Transcription failed: {}", e);
            return;
        }
    };

    // Step 2: Refine (Mock LLM Call)
    println!("   -> 2. Cleaning Text (Ollama)...");
    let refined_text = call_ollama(&raw_text);
    
    // Step 3: Type (Mock)
    println!("   -> 3. Typing: '{}'", refined_text);
    // TODO: enigo.key_sequence(&refined_text);
}

fn call_ollama(_text: &str) -> String {
    // In the future, this will be a real HTTP request to localhost:11434
    format!("Hello world. I am building Lacon.")
}

fn prepare_audio_for_whisper(
    audio_data: &[f32],
    original_sample_rate: u32,
    original_channels: u16,
) -> Vec<f32> {
    let target_sample_rate = 16000;
    
    // Step 1: Convert stereo to mono if needed
    let mono_audio: Vec<f32> = if original_channels == 2 {
        audio_data
            .chunks_exact(2)
            .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
            .collect()
    } else {
        audio_data.to_vec()
    };
    
    // Step 2: Resample to 16kHz if needed
    if original_sample_rate == target_sample_rate {
        return mono_audio;
    }
    
    // Simple linear interpolation resampling
    let ratio = original_sample_rate as f32 / target_sample_rate as f32;
    let new_length = (mono_audio.len() as f32 / ratio) as usize;
    
    let mut resampled = Vec::with_capacity(new_length);
    for i in 0..new_length {
        let src_index = i as f32 * ratio;
        let src_index_floor = src_index.floor() as usize;
        let src_index_ceil = (src_index_floor + 1).min(mono_audio.len() - 1);
        let fraction = src_index - src_index_floor as f32;
        
        let sample = mono_audio[src_index_floor] * (1.0 - fraction)
            + mono_audio[src_index_ceil] * fraction;
        resampled.push(sample);
    }
    
    resampled
}

fn transcribe_audio(wav_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Read the WAV file
    let mut reader = hound::WavReader::open(wav_path)?;
    let spec = reader.spec();
    
    // Read audio samples
    let audio_data: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
        hound::SampleFormat::Int => {
            if spec.bits_per_sample == 16 {
                reader
                    .samples::<i16>()
                    .map(|s| s.unwrap() as f32 / i16::MAX as f32)
                    .collect()
            } else {
                return Err("Unsupported bit depth".into());
            }
        }
    };
    
    // Prepare audio for Whisper (16kHz mono)
    let prepared_audio = prepare_audio_for_whisper(&audio_data, spec.sample_rate, spec.channels);
    
    // Get the Whisper context
    let ctx_guard = WHISPER_CTX.lock().unwrap();
    let ctx = ctx_guard
        .as_ref()
        .ok_or("Whisper context not initialized")?;
    
    // Create transcription parameters
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    
    // Set language to English for better performance
    params.set_language(Some("en"));
    params.set_print_progress(false);
    params.set_print_realtime(false);
    
    // Create a state and run inference
    let mut state = ctx.create_state()?;
    state.full(params, &prepared_audio)?;
    
    // Collect all transcribed segments
    let num_segments = state.full_n_segments()?;
    let mut full_text = String::new();
    
    for i in 0..num_segments {
        let segment = state.full_get_segment_text(i)?;
        full_text.push_str(&segment);
    }
    
    Ok(full_text.trim().to_string())
}
