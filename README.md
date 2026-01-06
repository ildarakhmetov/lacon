# Lacon

Voice-to-text keyboard automation tool for Linux.

## Setup

### System Requirements

- **Linux** (X11 or Wayland)
- For Fedora/RHEL: `sudo dnf install libxdo-devel`
- For Ubuntu/Debian: `sudo apt install libxdo-dev`
- For Arch: `sudo pacman -S xdotool`

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

### 2. Install Ollama

This project uses Ollama for text refinement. Install and run it:

```bash
# Install Ollama (see https://ollama.ai for other methods)
curl -fsSL https://ollama.com/install.sh | sh

# Pull the model used by lacon
ollama pull qwen2.5:3b-instruct
```

### 3. Build and Run

```bash
cargo build --release
cargo run --release
```

## Usage

Press and hold **F1** to record audio, release to stop recording and process.

The app will:
1. Record your voice
2. Transcribe it using Whisper
3. Refine the text using Ollama
4. Type the result into the focused window

**Note for Wayland users**: You may need to grant permission when the app attempts to inject keystrokes. Alternatively, run with X11 compatibility: `GDK_BACKEND=x11 cargo run --release`

## Status

Proof-of-concept complete:
- ✅ Hotkey detection (F1 press/release via rdev)
- ✅ Audio recording (cpal + hound)
- ✅ Transcription (Whisper via whisper-rs)
- ✅ Text refinement (Ollama API integration)
- ✅ Keyboard typing (enigo input injection)

## Implementation Notes

**Audio Recording**: Uses `cpal` for microphone capture and `hound` for WAV file writing. Audio is saved to `/tmp/lacon_recording.wav` as 16-bit PCM at the device's native sample rate.

**Transcription**: Uses `whisper-rs` (Rust bindings to whisper.cpp). Audio is automatically resampled to 16kHz mono before inference, which is Whisper's required format.

**Text Refinement**: Calls Ollama's `/api/generate` endpoint with the `qwen2.5:3b-instruct` model to clean up transcription errors and improve grammar.

**Keyboard Typing**: Uses `enigo` to inject keypresses into the focused window. Works on X11 and Wayland (with permission prompt on Wayland).

**Linux-Only**: This tool currently only supports Linux due to platform-specific dependencies (libxdo for enigo, ALSA for cpal).

**Known Limitations**: Due to `cpal::Stream` being `!Send` on Linux/ALSA, the current implementation intentionally leaks audio streams. This is acceptable for a PoC but would need refinement for production use.

