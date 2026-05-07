# Phase 5 – Ecosystem Crate: `blaze‑midi`

> **Goal:** Specify the `blaze‑midi` crate, which provides a data‑oriented, pure‑Blaze library for reading, writing, and manipulating MIDI (Musical Instrument Digital Interface) data.  It supports the SMF (Standard MIDI File) format, real‑time MIDI I/O via platform‑specific backends, and a simple event‑based composition API.  All parsing and writing operations are pure; real‑time I/O carries the `io` effect.  The crate is designed for music software, game audio, and embedded synthesizers.

---

## 1. Core Concepts

MIDI is a protocol and file format for music performance data.  The crate deals with:

- **`MidiEvent`** – a single MIDI message with a delta‑time (for files) or immediate timestamp (for real‑time).
- **`MidiFile`** – a Standard MIDI File with one or more tracks, each containing a sequence of events.
- **`MidiPort`** – an abstract connection to a system MIDI input or output device (optional, feature `midi‑io`).
- **`MidiSynthesizer`** – a simple built‑in wavetable synthesizer (optional, feature `synth`).

All types are linear where they own resources (files, ports); events and tracks are plain data structs (often `@copy` for small items).  The crate uses `blaze‑serde` for optional serialization of MIDI data to/from a custom binary format (not SMF, but for storage in Blaze‑native formats) and `blaze‑time` for timestamps.

---

## 2. `MidiEvent`

### 2.1 Definition

```
pub enum MidiEvent {
    NoteOff {
        channel: u8,
        key: u8,
        velocity: u8,
    },
    NoteOn {
        channel: u8,
        key: u8,
        velocity: u8,
    },
    PolyphonicKeyPressure {
        channel: u8,
        key: u8,
        pressure: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    ChannelPressure {
        channel: u8,
        pressure: u8,
    },
    PitchBend {
        channel: u8,
        value: i16,            // 14‑bit value, -8192 to 8191 (centered)
    },
    SystemExclusive {
        data: Vec<u8>,         // includes manufacturer id, any length
    },
    TimeCodeQuarterFrame(u8),
    SongPositionPointer(u16),
    SongSelect(u8),
    TuneRequest,
    TimingClock,
    Start,
    Continue,
    Stop,
    ActiveSensing,
    Reset,
    Meta(MetaEvent),
}

pub enum MetaEvent {
    SequenceNumber(u16),
    Text(Text),
    CopyrightNotice(Text),
    TrackName(Text),
    InstrumentName(Text),
    Lyric(Text),
    Marker(Text),
    CuePoint(Text),
    ChannelPrefix(u8),
    EndOfTrack,
    Tempo(u32),              // microseconds per quarter note
    SmpteOffset(SmpteOffset),
    TimeSignature(TimeSignature),
    KeySignature(KeySignature),
    SequencerSpecific(Vec<u8>),
}
```

- All events are `@copy` when they contain no heap data (most do not).  `SystemExclusive` and `Meta::Text` variants own a `Vec<u8>` or `Text`, so they are linear (move semantics).  Consumers can `.clone()` explicitly.
- `SmpteOffset`, `TimeSignature`, `KeySignature` are small `@copy` structs.

### 2.2 Timestamps

For MIDI files, events are stored with a delta‑time (in ticks) relative to the previous event.  The crate uses a `MidiTimestamp` type:

```
pub struct MidiTimestamp(u64);   // ticks
```

For real‑time MIDI, events use `std::time::Instant`.

---

## 3. `MidiTrack` and `MidiFile`

### 3.1 `MidiTrack`

```
pub struct MidiTrack {
    pub name: Option<Text>,
    pub events: Vec<(MidiTimestamp, MidiEvent)>,
}
```

- Tracks own their events in a linear `Vec`.  They can be merged, split, transposed, and quantized.

### 3.2 `MidiFile`

```
pub struct MidiFile {
    pub format: MidiFileFormat,
    pub division: MidiDivision,
    pub tracks: Vec<MidiTrack>,
}

pub enum MidiFileFormat { SingleTrack, MultiTrack, MultiSong }
pub enum MidiDivision { TicksPerQuarterNote(u16), FramesPerSecond(u8, u8) }
```

- `MidiFile` is linear; `Dispose` drops all tracks.
- Methods:
  - `pub fn new(format: MidiFileFormat, division: MidiDivision) -> MidiFile;`
  - `pub fn add_track(&mut self, track: MidiTrack);`
  - `pub fn read_from_smf(path: &str) -> Result<MidiFile, MidiError>;`
  - `pub fn write_to_smf(&self, path: &str) -> Result<(), MidiError>;`
  - `pub fn to_bytes(&self) -> Vec<u8>;`

---

## 4. SMF (Standard MIDI File) Parser

The parser reads the header chunk (`MThd`) and track chunk (`MTrk`) and decodes the variable‑length delta‑time and event bytes.  It handles running status (if enabled) and system exclusive events.  The parser is a pure state machine over a byte slice, returning a `MidiFile`.  Errors include invalid header, missing or extra chunks, truncated events, etc.

The writer serialises a `MidiFile` into the SMF binary format, correctly encoding delta times as variable‑length quantities and emitting the required header.  It does not support running status for simplicity (each event is written with its own status byte), but future versions may add it.

---

## 5. Real‑Time MIDI I/O (Optional, feature `midi‑io`)

This feature depends on `blaze‑os` (platform‑specific MIDI APIs) and provides:

### 5.1 `MidiInput` and `MidiOutput`

```
pub struct MidiInputPort { name: Text, id: u32 }
pub struct MidiOutputPort { name: Text, id: u32 }

pub fn enumerate_input_ports() -> Vec<MidiInputPort>;
pub fn enumerate_output_ports() -> Vec<MidiOutputPort>;

pub fn open_input(port: &MidiInputPort) -> Result<MidiInput, MidiError>;
pub fn open_output(port: &MidiOutputPort) -> Result<MidiOutput, MidiError>;
```

- `MidiInput` is a stream of `MidiEvent` (with timestamps).  It can be used as an iterator or via a callback registered at open time.
- `MidiOutput` has a `send(event: &MidiEvent)` method.  It is linear and must be closed (on drop).

---

## 6. Simple Wavetable Synthesizer (Optional, feature `synth`)

The crate includes a simple wavetable synthesizer for rendering MIDI files or streams to audio.  It uses `blaze‑audio` for output.

```
pub struct Synth {
    sample_rate: u32,
    voices: Vec<Voice>,
    wave_table: WaveTable,
}

pub struct SynthConfig {
    pub max_voices: usize,
    pub wave_table_path: Option<Text>,
}

impl Synth {
    pub fn new(config: &SynthConfig) -> Synth;
    pub fn process_midi(&mut self, events: &[MidiEvent], timestamp: u64);
    pub fn render(&mut self, out: &mut [f32], channels: u16);
}
```

- `WaveTable` is a collection of samples mapped to MIDI program numbers; a default General MIDI set is embedded.
- `render` produces interleaved stereo PCM.  The synth is pure (no I/O) once initialised.

---

## 7. Error Handling

```
pub enum MidiError {
    Io(std::io::Error),
    InvalidFormat(Text),
    InvalidTrack,
    TruncatedEvent,
    UnsupportedFeature(Text),
    PortNotFound,
    PortBusy,
    SynthesizerError(Text),
}
```

---

## 8. Implementation Notes

- The SMF parser is hand‑written, reading a byte slice and building events without allocation until needed (heap strings for meta text events).  It is designed to be fast and deterministic.
- The real‑time MIDI I/O module uses platform‑specific backends: CoreMIDI on macOS, ALSA/JACK on Linux, and WinMM/UWP on Windows.  These are abstracted behind a simple trait.
- The wavetable synthesizer uses a simple band‑limited oscillator and a low‑pass filter, all in pure Blaze with `blaze‑simd` for vectorisation.

---

## 9. Testing

- **SMF round‑trip:** Create a `MidiFile`, write to bytes, parse back, verify events match.
- **Meta events:** Write a track with a tempo change and a time signature, read back, ensure values are correct.
- **Synthesizer:** Feed a note‑on event, render a few frames, verify non‑zero output.
- **Real‑time I/O (if backend available):** Open a virtual port, send an event, read from a loopback, verify receipt.
- **Error handling:** Provide a truncated SMF, expect `TruncatedEvent` error.

All tests must pass on all platforms; MIDI I/O tests are platform‑specific and may be skipped on systems without a MIDI device.
