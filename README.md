# Lacon

Voice-to-text keyboard automation tool.

## Usage

Press and hold F1 to record audio, release to stop recording and process.

## Status

Early proof-of-concept:
- ✅ Hotkey detection (F1 press/release)
- ✅ Audio recording (cpal + hound)
- ✅ WAV file output to `/tmp/lacon_recording.wav`
- ⏳ Transcription (Whisper - mocked)
- ⏳ Text refinement (Ollama - mocked)
- ⏳ Keyboard typing (enigo - mocked)

## Implementation Notes

Audio recording uses `cpal` for microphone capture and `hound` for WAV file writing. The recorded audio is saved as 16-bit PCM mono WAV format at the device's native sample rate.

Due to `cpal::Stream` being `!Send` on Linux/ALSA, the current implementation intentionally leaks audio streams. This is acceptable for a PoC but would need refinement for production use.

