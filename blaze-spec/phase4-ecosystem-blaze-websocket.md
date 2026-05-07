# Phase 4 – Ecosystem Crate: `blaze‑web`

> **Goal:** Provide a safe, data‑oriented, actor‑based browser front‑end framework that renders Blaze GUI applications to a `<canvas>` element via WebAssembly, without any DOM manipulation.  The crate builds on `blaze‑gui` and `blaze‑wgpu` to deliver a single, unified UI system that works identically on desktop, mobile, and the web.  All rendering is done through the GPU, and accessibility is handled by a parallel semantic tree exposed to the browser’s accessibility engine.

---

## 1. Core Concepts

- **No DOM** – the entire UI is rendered to a full‑surface canvas using `blaze‑wgpu` (backed by WebGPU or WebGL).
- **Same GUI code** – the exact same widget tree and application actor from `blaze‑gui` is used; no fork or re‑implementation.
- **Event translation** – browser input events (mouse, touch, keyboard) are captured on the canvas and translated into Blaze’s `InputEvent` messages, identical to desktop input.
- **Accessibility bridge** – the `blaze‑accessibility` tree is serialised and sent to a small JavaScript shim that creates ARIA‑based live regions, enabling screen reader support.
- **Responsive layout** – the canvas automatically resizes to the viewport, and the constraint‑based layout system adapts the UI seamlessly.

---

## 2. The WebAssembly Runtime

The crate compiles the developer’s Blaze application (GUI + logic) into a `.wasm` binary, linked with a minimal JavaScript loader.

### 2.1 `blaze‑web‑loader.js`

This is a static, zero‑configuration JavaScript file (100 LOC) that:

- Creates a full‑screen `<canvas>` element.
- Requests a WebGPU (or WebGL 2.0) context.
- Fetches and instantiates the Blaze `.wasm` module.
- Passes the canvas context to the Blaze runtime.
- Forwards all DOM events on the canvas to the Blaze event loop.
- Receives accessibility tree updates from Blaze and updates ARIA live regions.

The developer never writes JavaScript; this loader is embedded by `blaze build --target web`.

---

## 3. Integration with `blaze‑gui`

The `App` actor from `blaze‑gui` is used unchanged.  The `Renderer` trait is implemented by `WebGpuRenderer` (a thin wrapper over `blaze‑wgpu` that handles the web context).

No modifications to the application code are required.  A desktop application written with `blaze‑gui` can be targeted to the web by simply changing the build target:

```
blaze build --target web --out-dir web/
```

---

## 4. Input Handling

The JavaScript loader listens to `mousedown`, `mouseup`, `mousemove`, `wheel`, `touchstart`, `touchmove`, `touchend`, and `keydown`/`keyup` events on the canvas.  Each event is translated into a binary format and pushed into a ring buffer shared with the Blaze runtime (via WebAssembly linear memory).

On the Blaze side, the `InputHandler` actor reads these events and converts them to `InputEvent` values, which are then dispatched to the `App` actor exactly as on desktop.

---

## 5. Accessibility Bridge

The browser does not allow WebAssembly to directly manipulate the DOM or ARIA properties.  Therefore, the `blaze‑accessibility` backend for the web uses a **shadow ARIA tree**:

- The Blaze runtime serialises the `AccessibilityNode` tree (as defined in `blaze‑accessibility`) into a compact binary format and writes it to a dedicated memory buffer.
- The JavaScript loader polls this buffer on each animation frame.
- When changes are detected, the loader creates or updates a parallel DOM subtree (hidden, non‑visual) containing `<div>` elements with ARIA roles, labels, and values.  This subtree is placed at the start of `<body>`.
- Screen readers interact with this ARIA tree while the visual canvas remains untouched.

This ensures full compliance with WCAG, without breaking the canvas‑based rendering model.

---

## 6. Styling and Theming

The same `Style` struct defined in `blaze‑gui` is used.  There is no CSS.  The `Style` values are passed as typed parameters to widget builders and applied by the renderer.  Theming works identically across desktop and web: the application can define a global `Theme` and pass it to the `App` actor; all widgets inherit it unless overridden.

---

## 7. Navigation and Routing

Blaze‑web applications are single‑page.  Navigation within the app is handled by the `App` actor’s state machine (e.g., using a `#[route]` attribute on message handlers).  `pushState` and `popState` events from the browser are captured by the loader and forwarded to the Blaze runtime, allowing the application to update the URL without a page reload.  This enables deep‑linking and the back button without any JavaScript code from the developer.

---

## 8. Performance

- All rendering is GPU‑accelerated, bypassing the DOM’s layout engine entirely.
- The widget tree is flat and SoA‑optimised, rebuilt each frame from pure functions – no virtual DOM diffing overhead.
- The accessibility tree is only serialised when assistive technologies are active (detected via a browser API), saving memory and CPU in typical usage.
- The JavaScript loader is minimal and never allocates objects after initialisation; it passes raw pointers and byte buffers.

The result is a web application that feels like a native desktop app, with smooth 120 FPS rendering and sub‑millisecond event latency.

---

## 9. Error Handling

```
pub enum WebError {
    GpuNotAvailable,
    WasmLoadFailure(Text),
    AccessibilityBridgeFailure,
    Io(std::io::Error),
}
```

These errors are caught during initialisation and reported to the developer via the console.  The `App` actor can handle them by displaying a fallback message (e.g., “WebGPU not supported” rendered on a placeholder canvas).

---

## 10. Testing

- **Rendering:** Use a headless browser (e.g., Playwright) to load a test application, capture a screenshot, and compare with a known reference.
- **Input:** Simulate mouse clicks and keyboard events via the test harness; verify that the correct Blaze messages are received by the actor.
- **Accessibility:** Inspect the ARIA subtree via JavaScript and check that buttons, sliders, and text have the expected roles and labels.
- **Routing:** Trigger `popState`, verify that the application updates its state and the canvas re‑renders.
- **Performance:** Measure frame time for a UI with 1000 widgets, ensure it stays under 8 ms on a modern browser.

All tests run on a headless WebGPU‑capable browser in CI.
