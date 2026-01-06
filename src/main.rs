use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use rdev::{listen, Event, EventType, Key};

// Global state to track if we're currently recording
static APP_STATE: Mutex<AppState> = Mutex::new(AppState::Idle);

// 1. Define our States
#[derive(Debug, Clone, Copy, PartialEq)]
enum AppState {
    Idle,
    Recording,
    Processing,
}

fn main() {
    println!("Lacon is running... (Press F1 to test)");

    // We start a separate thread to listen for keyboard events
    // because the listener blocks the thread it runs on.
    if let Err(error) = listen(callback) {
        println!("Error: {:?}", error);
    }
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
                    // TODO: Start cpal stream here
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
                    
                    // SIMULATION OF THE PIPELINE:
                    thread::spawn(|| {
                        process_audio_pipeline();
                        
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

fn process_audio_pipeline() {
    // Step 1: Transcribe (Mock)
    println!("   -> 1. Transcribing Audio (Whisper)...");
    let raw_text = "um hello world i am building lacon";
    thread::sleep(Duration::from_millis(500)); // Fake delay

    // Step 2: Refine (Mock LLM Call)
    println!("   -> 2. Cleaning Text (Ollama)...");
    let refined_text = call_ollama(raw_text);
    
    // Step 3: Type (Mock)
    println!("   -> 3. Typing: '{}'", refined_text);
    // TODO: enigo.key_sequence(&refined_text);
}

fn call_ollama(_text: &str) -> String {
    // In the future, this will be a real HTTP request to localhost:11434
    format!("Hello world. I am building Lacon.")
}
