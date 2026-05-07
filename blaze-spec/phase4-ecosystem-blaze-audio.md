# Phase 4 – Ecosystem Crate: `blaze‑audio`

> **Goal:** Specify the `blaze‑audio` crate, which provides a low‑latency, data‑oriented audio input and output framework built on Blaze’s async and actor model.  It supports real‑time audio streaming, playback of audio files, generation of waveforms, and integration with the ECS for spatial audio.  All audio I/O carries the `io` effect and is designed for deterministic, glitch‑free performance.

---

## 1. Core Concepts

The crate models audio as a stream of frames, where each frame contains one or more channels of PCM samples.  A **device** represents an audio input or output endpoint (e.g., a speaker, microphone).  A **stream** is an actor that continuously pushes or pulls audio frames to/from a device.  The crate provides built‑in support for common audio formats (WAV, MP3, Ogg Vorbis via feature flags) and a synthesis engine for generating waveforms and applying effects.

---

## 2. Audio Devices

### 2.1 `AudioDevice`

```
pub struct AudioDevice {
    name: Text,
    device_type: DeviceType,
    default_sample_rate: u32,
    num_channels: u16,
}
```

- Enumerated via `AudioDevice::enumerate() -> Vec<AudioDevice>`.

### 2.2 `DeviceType`

```
pub enum DeviceType { Playback, Capture }
```

### 2.3 Opening a Stream

```
pub fn open_playback_stream(
    device: &AudioDevice,
    config: &StreamConfig,
    callback: impl FnMut(&mut [f32]) + Send + 'static,
) -> Result<AudioStream, AudioError>;

pub fn open_capture_stream(
    device: &AudioDevice,
    config: &StreamConfig,
    callback: impl FnMut(&[f32]) + Send + 'static,
) -> Result<AudioStream, AudioError>;
```

- `StreamConfig` contains sample rate, channel count, buffer size.
- The callback is called from a high‑priority I/O actor, guaranteed not to be preempted by the garbage collector (since Blaze has no GC).  It must be pure (no I/O, allocation) to avoid glitches.  The compiler will enforce this via the effect system (callback is `fn(&mut [f32]) / pure`).

---

## 3. `AudioStream`

```
pub struct AudioStream {
    sender: Sender<StreamCommand>,
    config: StreamConfig,
}

enum StreamCommand { Pause, Resume, Close }
```

- `AudioStream::play(&self)` / `pause(&self)` / `close(self)` control the stream.
- `Dispose` for `AudioStream` gracefully closes the stream.

---

## 4. Audio Buffer and Format

### 4.1 `AudioBuffer`

```
pub struct AudioBuffer {
    samples: Vec<f32>,      // interleaved frames
    sample_rate: u32,
    channels: u16,
    duration: Duration,
}
```

- Represents a complete audio clip, suitable for playback or editing.

### 4.2 Loading and Saving

```
pub fn load_wav(path: &str) -> Result<AudioBuffer, AudioError>;
pub fn save_wav(path: &str, buffer: &AudioBuffer) -> Result<(), AudioError>;

#[cfg(feature = "mp3")]
pub fn load_mp3(path: &str) -> Result<AudioBuffer, AudioError>;

#[cfg(feature = "vorbis")]
pub fn load_ogg(path: &str) -> Result<AudioBuffer, AudioError>;
```

- These functions are synchronous (carry `io` effect) but may be called from a blocking thread pool to avoid blocking the main thread.

---

## 5. Waveform Generator and Effects

### 5.1 `Waveform`

```
pub enum Waveform {
    Sine,
    Square,
    Sawtooth,
    Triangle,
    Noise,
    Custom(fn(f32) -> f32),
}
```

- `fn generate_waveform(wave: Waveform, frequency: f32, sample_rate: u32, duration: Duration) -> AudioBuffer;`

### 5.2 Effects

```
pub trait AudioEffect {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32);
}
```

- Built‑in effects: `Gain`, `Delay`, `Reverb`, `LowPass`, `HighPass`, `Compressor`.  Each is a struct with parameters, `impl AudioEffect`.
- Effects can be chained: `EffectChain { effects: Vec<Box<dyn AudioEffect>> }`.

---

## 6. Spatial Audio and ECS Integration

For 3D applications, the crate provides a `SpatialAudio` component and system that integrate with `blaze‑ecs` and `blaze‑physics3d` (for listener and emitter positions).

### 6.1 `AudioEvent`

```
pub enum AudioEvent {
    PlayOneShot(Entity, AudioBuffer),   // entity is the emitter
    SetListener(Entity),
    StopEmitter(Entity),
}
```

- Sent to the audio actor, which maintains a map of active emitters and mixes 3D audio based on their positions relative to the listener.

### 6.2 `occlusion` and `reverb` can be applied via raycasting or rooms, but that is outside the scope of the core crate.

---

## 7. Error Handling

```
pub enum AudioError {
    Io(std::io::Error),
    DeviceUnavailable,
    FormatNotSupported,
    LoadError(Text),
    StreamClosed,
}
```

---

## 8. Implementation Notes

- The crate uses a platform‑specific audio backend: `cpal` (native) or a pure‑Blaze implementation via `std::os` bindings to CoreAudio (macOS), PulseAudio/ALSA (Linux), WASAPI (Windows).  The backend is abstracted behind a trait, so adding new backends is straightforward.
- The audio I/O actor runs on a dedicated thread (the audio thread), which is pinned to a specific CPU core and runs at high priority.  The callback is not an actor but a bare function to minimise latency.  The effect system ensures it is pure.
- `AudioBuffer` holds data in SoA format per channel?  Actually, interleaved frames are more common for audio I/O; we keep them interleaved for compatibility with hardware, but the `AudioEffect::process` can use sliding windows for cache efficiency.
- For spatial audio, the distance attenuation and panning are computed using simple linear gain or HRTF (if feature `hrtf` is enabled).  The ECS system reads emitter positions from `RigidBody3D` and writes audio events to a channel consumed by the audio actor.

---

## 9. Testing

- **Playback:** Open a playback stream, fill with sine wave, verify no errors.
- **Capture:** Open a capture stream, record 1 second, verify buffer length.
- **WAV round‑trip:** Generate a buffer, save to WAV, load back, compare samples (within epsilon).
- **Effect chain:** Apply a gain effect to a buffer, verify amplitude change.
- **Spatial audio:** Set up a world with listener and emitter at known positions, simulate, verify the output gain matches expected attenuation.
- **Loopback test:** (Manual) Play a known signal to a loopback device and capture; verify signal integrity.

All tests must run with a working audio device; an optional mock audio backend can be enabled for CI.
