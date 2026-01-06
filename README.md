# Lacon

Voice-to-text keyboard automation tool.

## Setup

### 1. Download Whisper Model

Before first use, download a Whisper model file:

```bash
# Create models directory
mkdir -p models

# Download base English model (~142MB, recommended for speed)
curl -L -o models/ggml-base.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin

# Alternative: small model for better accuracy (~466MB)
# curl -L -o models/ggml-small.bin \
#   https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin
```

### 2. Build and Run

```bash
cargo build --release
cargo run --release
```

## Usage

Press and hold F1 to record audio, release to stop recording and process.

## Status

Early proof-of-concept:
- ✅ Hotkey detection (F1 press/release)
- ✅ Audio recording (cpal + hound)
- ✅ WAV file output to `/tmp/lacon_recording.wav`
- ✅ Transcription (Whisper via whisper-rs)
- ⏳ Text refinement (Ollama - mocked)
- ⏳ Keyboard typing (enigo - mocked)

## Implementation Notes

Audio recording uses `cpal` for microphone capture and `hound` for WAV file writing. The recorded audio is saved as 16-bit PCM mono WAV format at the device's native sample rate.

Transcription uses `whisper-rs` (Rust bindings to whisper.cpp). Audio is automatically resampled to 16kHz mono before inference, which is Whisper's required format.

Due to `cpal::Stream` being `!Send` on Linux/ALSA, the current implementation intentionally leaks audio streams. This is acceptable for a PoC but would need refinement for production use.

