# Phase 4 – Ecosystem Crate: `blaze‑input`

> **Goal:** Provide a unified, data‑oriented, cross‑platform input abstraction for keyboard, mouse, touch, pen, and gamepad devices.  The crate collects raw device events from the operating system, normalises them into a small set of enums, and delivers them as messages to the Blaze actor system.  It is used by `blaze‑gui` and can be used independently by games, multimedia applications, and other interactive software.  All event processing is pure, deterministic, and low‑latency, with no heap allocation in the hot path.

---

## 1. Core Concepts

- **Device‑agnostic events** – all hardware‑specific details are abstracted into `InputEvent`, `Key`, `MouseButton`, etc.
- **Actor delivery** – events are pushed into a lock‑free SPSC queue and consumed by an actor, avoiding global callbacks.
- **Focus tracking** – the crate maintains a simple focus model (which window/widget has focus) to route keyboard events.
- **Modifier state** – keyboard modifiers (Ctrl, Shift, Alt, Meta) are tracked per event.
- **Zero‑copy** – raw event buffers are pre‑allocated; parsed events are views into those buffers.

---

## 2. Input Events

### 2.1 `InputEvent`

```
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Wheel(ScrollEvent),
    Touch(TouchEvent),
    Pen(PenEvent),
    Gamepad(GamepadEvent),
    Window(WindowEvent),
}
```

All variants carry a `timestamp: Instant` for precise timing.

### 2.2 `KeyEvent`

```
pub struct KeyEvent {
    pub key: Key,
    pub state: ButtonState,
    pub modifiers: Modifiers,
    pub repeat: bool,
    pub scancode: u32,        // platform‑specific, for advanced usage
}

pub enum Key {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    // Digits
    Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Navigation
    Escape, Tab, CapsLock, Shift, Control, Alt, Meta, Enter, Space, Backspace,
    Insert, Delete, Home, End, PageUp, PageDown,
    Left, Right, Up, Down,
    // Numpad
    Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    NumpadAdd, NumpadSubtract, NumpadMultiply, NumpadDivide, NumpadDecimal, NumpadEnter,
    // Media
    VolumeUp, VolumeDown, Mute, PlayPause, NextTrack, PrevTrack, MediaStop,
    // Unknown
    Unknown,
}

pub enum ButtonState { Pressed, Released }
```

- `repeat` is `true` for key repeat events (only for `Pressed` state).
- `Modifiers` is a bitflag: `Ctrl`, `Shift`, `Alt`, `Meta`, `NumLock`, `CapsLock`.

### 2.3 `MouseEvent`

```
pub struct MouseEvent {
    pub kind: MouseKind,
    pub button: MouseButton,
    pub position: (f64, f64),
    pub modifiers: Modifiers,
}

pub enum MouseKind { Moved, Pressed, Released, Entered, Exited }
pub enum MouseButton { Left, Right, Middle, Back, Forward, Other(u8) }
```

Coordinates are in logical pixels relative to the window’s client area.

### 2.4 `ScrollEvent`

```
pub struct ScrollEvent {
    pub delta: (f64, f64),       // lines (vertical, horizontal)
    pub position: (f64, f64),
    pub modifiers: Modifiers,
}
```

### 2.5 `TouchEvent`

```
pub struct TouchEvent {
    pub phase: TouchPhase,
    pub touches: Vec<Touch>,
}

pub enum TouchPhase { Started, Moved, Ended, Cancelled }
pub struct Touch {
    pub id: u64,
    pub position: (f64, f64),
    pub pressure: Option<f64>,
}
```

### 2.6 `PenEvent`

```
pub struct PenEvent {
    pub phase: TouchPhase,
    pub position: (f64, f64),
    pub pressure: f64,
    pub tilt: (f64, f64),
    pub buttons: u32,
}
```

### 2.7 `GamepadEvent`

```
pub struct GamepadEvent {
    pub gamepad_id: u32,
    pub kind: GamepadEventKind,
}
pub enum GamepadEventKind {
    Button { button: GamepadButton, state: ButtonState },
    Axis { axis: GamepadAxis, value: f64 },
    Connected,
    Disconnected,
}
pub enum GamepadButton { A, B, X, Y, DpadUp, DpadDown, DpadLeft, DpadRight, LeftBumper, RightBumper, … }
pub enum GamepadAxis { LeftX, LeftY, RightX, RightY, LeftTrigger, RightTrigger }
```

### 2.8 `WindowEvent`

```
pub enum WindowEvent {
    Resize(f64, f64),
    CloseRequested,
    Focused(bool),
    Minimized(bool),
    Maximized(bool),
    Moved(f64, f64),
    DroppedFile(Text),
}
```

---

## 3. Input Backend

The crate provides a platform‑specific backend via a `InputBackend` trait, but typical usage is handled by `blaze‑gui` or a top‑level actor.

```
pub trait InputBackend: Send + 'static {
    fn poll_events(&mut self) -> Vec<InputEvent>;
}
```

The runtime creates the appropriate backend (Win32, Cocoa, X11/Wayland, Web, etc.) and forwards events to the application actor.

---

## 4. Integration with `blaze‑gui`

The `App` actor receives `InputEvent` values directly.  The GUI framework performs hit‑testing and converts them into high‑level `Message` variants.  The developer never interacts with raw input events unless they want to (e.g., for game‑specific handling).

```
// Inside the App actor
fn handle_message(&mut self, msg: InputEvent) {
    match msg {
        InputEvent::Key(key) => self.dispatch_key(key),
        InputEvent::Mouse(mouse) => self.dispatch_mouse(mouse),
        _ => {}
    }
}
```

---

## 5. Determinism

Under `--reproducible`, the input event stream may be recorded and replayed from a file, ensuring deterministic GUI tests.  The `InputBackend` implementation for testing can replay recorded sequences.

---

## 6. Testing

- **Event conversion:** simulate raw OS events (e.g., a Win32 MSG struct) and verify they map to the correct `InputEvent`.
- **Focus tracking:** move the mouse, click, then type; ensure keyboard events are delivered to the focused window.
- **Multi‑touch:** simulate multiple finger touches; verify all touches are reported.
- **Gamepad:** connect a virtual gamepad, press a button, verify the event.
- **Replay:** record a sequence of events with `--reproducible`, replay, and compare the resulting UI state.

All tests use mock backends; platform‑specific tests run on CI with OS‑level headless display servers where needed.
