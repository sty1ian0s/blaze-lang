# Phase 4 – Ecosystem Crate: `blaze‑accessibility`

> **Goal:** Provide the platform‑specific backend that translates the semantic accessibility tree (built automatically by `blaze‑gui`) into the operating system’s native accessibility API.  This crate is a companion to `blaze‑gui` and is responsible for pushing accessibility nodes to AT‑SPI on Linux, UIA on Windows, NSAccessibility on macOS, and a generic null backend for unsupported targets.  It exposes a single, data‑oriented trait `AccessibilityBackend` that the GUI framework calls after each frame.

---

## 1. Core Trait

### 1.1 `AccessibilityBackend`

```
pub trait AccessibilityBackend: Send + Sync {
    fn update(&mut self, nodes: &[AccessibilityNode]);
    fn flush(&mut self);
}
```

- `update` receives a flat list of `AccessibilityNode` values representing the current UI state.  The backend compares this with the previous snapshot and performs incremental updates (add, remove, modify) on the native accessibility tree.
- `flush` ensures all pending changes are committed to the platform service.

---

## 2. `AccessibilityNode`

This type is defined in `blaze‑gui` and re‑exported here for backend implementations.

```
pub struct AccessibilityNode {
    pub id: u64,                   // stable ID across frames (WidgetId)
    pub role: AccessibilityRole,
    pub label: String,
    pub value: Option<String>,
    pub description: Option<String>,
    pub actions: Vec<AccessibilityAction>,
    pub bounding_rect: Rect,       // in logical pixels, relative to window
    pub parent_id: Option<u64>,
    pub children_ids: Vec<u64>,
}

pub enum AccessibilityRole {
    Button,
    Slider,
    CheckBox,
    StaticText,
    Image,
    Link,
    List,
    ListItem,
    Heading,
    Window,
    Dialog,
    Menu,
    MenuItem,
    TabList,
    Tab,
    Tree,
    TreeItem,
    Custom(String),
}

pub struct AccessibilityAction {
    pub name: String,
    pub action_type: ActionType,
}

pub enum ActionType {
    Activate,
    Increment,
    Decrement,
    Toggle,
    ScrollIntoView,
}
```

---

## 3. Backend Implementations

### 3.1 Linux (AT‑SPI2)

The backend connects to the AT‑SPI registry over D‑Bus, creates an accessible application, and maps `AccessibilityNode` roles to AT‑SPI interfaces (`atk::Component`, `atk::Action`, etc.).  The implementation is pure Blaze with unsafe bindings to `libdbus` or `zbus`.

```
pub struct AtSpiBackend {
    bus: dbus::Connection,
    app: atspi::Application,
    // ...
}
impl AccessibilityBackend for AtSpiBackend { … }
```

### 3.2 Windows (UIA)

Uses the Windows UI Automation COM API.  Nodes are mapped to `IUIAutomationElement` objects.  The backend maintains a cache of previous elements and uses `UiaRaiseAutomationPropertyChangedEvent` for incremental updates.

```
pub struct UiaBackend {
    provider: uia::Provider,
    // ...
}
impl AccessibilityBackend for UiaBackend { … }
```

### 3.3 macOS (NSAccessibility)

Uses the `NSAccessibility` protocol via Objective‑C FFI.  The backend creates `NSView`‑like objects and posts `NSAccessibilityPostNotification`.

```
pub struct NsAccessibilityBackend {
    workspace: ns::Workspace,
    // ...
}
impl AccessibilityBackend for NsAccessibilityBackend { … }
```

### 3.4 Null Backend

For targets without an accessibility API (e.g., embedded, headless CI), a `NullBackend` does nothing.  This is the default when no platform‑specific backend is compiled.

```
pub struct NullBackend;
impl AccessibilityBackend for NullBackend {
    fn update(&mut self, _: &[AccessibilityNode]) {}
    fn flush(&mut self) {}
}
```

The GUI framework automatically selects the appropriate backend via conditional compilation (`@cfg(target_os = …)`) and instantiates it when the `App` actor starts.

---

## 4. Integration with `blaze‑gui`

The `App` actor in `blaze‑gui` owns an `AccessibilityBackend` instance.  After each UI rebuild, it calls:

```
self.accessibility_backend.update(&self.accessibility_tree.nodes);
self.accessibility_backend.flush();
```

The accessibility tree is derived directly from the widget tree by the `App` actor; no user code is required.  Widgets that need custom roles or labels use the `aria_role` and `aria_label` methods in the widget builder.

The `AccessibilityBackend` trait is object‑safe, so the `App` actor stores it as `Box<dyn AccessibilityBackend>`, allowing runtime selection of the backend (though usually it’s compile‑time fixed).

---

## 5. Testing

- **Null backend:** verify it accepts any update and is a no‑op.
- **Mock backend:** implement the trait to capture calls, then build a simple GUI, force an accessibility update, and assert that the expected nodes (with correct roles and labels) are emitted.
- **Platform‑specific tests (manual):** run a sample GUI on each supported OS with a screen reader enabled and confirm that buttons, sliders, and text are announced correctly.

All automated tests use the mock backend and run on all platforms.
