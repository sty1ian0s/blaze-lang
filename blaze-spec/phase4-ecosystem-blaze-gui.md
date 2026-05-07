# Phase 4 – Ecosystem Crate: `blaze‑gui`

> **Goal:** Provide a high‑performance, data‑oriented, actor‑based, and fully accessible graphical user interface system for all Blaze applications.  The system is not a wrapper around HTML/CSS; it is a native Blaze solution that uses immediate‑mode rendering, constraint‑based layout, typed styles, and automatic accessibility tree generation.  Applications describe their UI as a pure function of state, returning a flat, SoA‑optimised widget tree that is rendered by `blaze‑wgpu`.  The GUI works identically on desktop, game, embedded, and canvas‑based web targets.

---

## 1. Core Concepts

- **Widgets as data** – every UI element is a variant of the `WidgetNode` enum, stored in a flat array.
- **Pure UI functions** – `fn ui(state: &AppState) -> WidgetHandle`.  No closures, no side effects.
- **Messages as enums** – user interactions produce messages that are sent to the application actor.
- **Layout as constraints** – each widget carries a `Layout` struct specifying size, padding, flex, etc.  The layout solver is a pure function.
- **Styles as typed values** – a `Style` struct defines visual properties; styles are passed explicitly, never cascaded.
- **Accessibility by construction** – the framework automatically generates a semantic accessibility tree from the widget roles.
- **Backend agnostic** – rendering is delegated to a `Renderer` trait, defaulting to `blaze‑wgpu`.

---

## 2. Widget Tree

### 2.1 `WidgetNode` Enum

```
pub enum WidgetNode {
    Container { id: WidgetId, child: WidgetHandle },
    Row { id: WidgetId, children: Vec<WidgetHandle> },
    Column { id: WidgetId, children: Vec<WidgetHandle> },
    Stack { id: WidgetId, children: Vec<WidgetHandle> },
    Text { id: WidgetId, content: String },
    Button { id: WidgetId, label: String, on_click: Message },
    Slider { id: WidgetId, value: f64, min: f64, max: f64, on_change: Message },
    Checkbox { id: WidgetId, checked: bool, label: String, on_toggle: Message },
    Image { id: WidgetId, source: ImageSource },
    Canvas { id: WidgetId, draw_fn: fn(&mut CanvasContext) },
    Custom { id: WidgetId, shape: Box<dyn CustomWidget> },
    // ... more as needed
}
```

- `WidgetId` is a unique integer assigned when building the tree.
- `WidgetHandle` is `usize` – an index into the tree array.
- `Message` is a user‑defined enum representing the application’s event set (see below).

### 2.2 Building the Tree

Widget nodes are built using ergonomic builder methods.  Each builder returns a `WidgetHandle`.

Example:

```
let handle = Column::new()
    .push(Text::new("Hello").handle())
    .push(Button::new("Click me", AppMsg::Clicked).handle())
    .handle();
```

Under the hood, these push nodes into a global `UiBuilder` arena that is reset every frame.  There is no virtual DOM diffing.

---

## 3. Application Architecture

### 3.1 `AppState` and `Message`

The application owns a state struct and a message enum.  The UI function reads the state and maps UI actions to messages.

```
struct MyState {
    count: i32,
    text: String,
}

enum MyMsg {
    Increment,
    Decrement,
    SetText(String),
}
```

### 3.2 `UiActor`

The runtime provides an `App` actor that manages the event loop:

```
pub actor App<S: 'static> {
    state: S,
    ui_fn: fn(&S) -> WidgetHandle,
    renderer: Renderer,
    accessibility: AccessibilityTree,
    input_handler: InputHandler,
}
```

- On startup, the actor builds the widget tree via `ui_fn`.
- On each event (user input, window resize), the actor updates `state` via the message, then rebuilds the UI tree.
- The tree is passed to the renderer, which calls `draw` on each node; simultaneously, the accessibility tree is updated and pushed to the platform’s accessibility API.

---

## 4. Layout System

### 4.1 `Layout` Struct

Every widget has an optional `Layout` field:

```
pub struct Layout {
    pub width: SizeConstraint,
    pub height: SizeConstraint,
    pub padding: EdgeInsets,
    pub margin: EdgeInsets,
    pub alignment: Alignment,
    pub flex: f64,                // only meaningful inside Row/Column
}

pub enum SizeConstraint {
    Pixels(f64),
    Percent(f64),                 // relative to parent
    Auto,                          // size to content
}
```

### 4.2 Layout Solver

The layout algorithm is a pure function:

```
fn compute_layout(tree: &[WidgetNode], available_width: f64, available_height: f64) -> Vec<Rect>;
```

It walks the tree once, resolving constraints top‑down, and returns a list of `Rect`s indexed by `WidgetId`.  The solver respects minimum sizes of widgets (e.g., text content) and distributes remaining space according to `flex` values.

---

## 5. Styling

### 5.1 `Style` Struct

```
pub struct Style {
    pub background: Color,
    pub foreground: Color,
    pub font_size: f64,
    pub font_family: String,
    pub corner_radius: f64,
    pub border: Option<(f64, Color)>,
    pub shadow: Option<(f64, f64, Color)>,
}
```

### 5.2 Applying Styles

Styles are applied via a `apply_style(style: Style)` method on widget builders.  The style is stored in the widget node.  If a widget does not specify a style, a default application‑wide style is used, which can be set globally when creating the `App`.

There is **no CSS cascading, no global stylesheets, and no inheritance**.  This ensures that the appearance of every widget is local and deterministic.

---

## 6. Accessibility

### 6.1 Automatic Semantic Tree

Each `WidgetNode` variant implicitly defines a **semantic role**:

| Widget | Semantic Role |
|--------|---------------|
| Button | `Button` |
| Slider | `Slider` |
| Checkbox | `CheckBox` |
| Text | `StaticText` |
| Image (with alt text) | `Graphic` |

The `App` actor builds an `AccessibilityNode` tree in parallel with the visual tree.

### 6.2 `AccessibilityNode`

```
pub struct AccessibilityNode {
    pub role: AccessibilityRole,
    pub label: String,
    pub value: Option<String>,
    pub actions: Vec<AccessibilityAction>,
    pub bounding_rect: Rect,
}

pub enum AccessibilityRole {
    Button, Slider, CheckBox, StaticText, Image, List, ListItem, Heading, Window,
}

pub struct AccessibilityAction {
    pub name: String,
    pub message: Message,
}
```

The label is taken from the widget’s text content or an explicit `aria_label` field.  For sliders, the current value and range are exposed automatically.

The framework pushes these nodes to the operating system’s accessibility service (AT‑SPI, UIA, NSAccessibility) through a platform‑specific backend.  Screen readers and other assistive technologies can then interact with the application.

### 6.3 Explicit Label Overrides

For icon buttons or custom widgets, the developer can set an explicit label:

```
Button::new("⊕")
    .aria_label("Zoom in")
    .on_click(MyMsg::ZoomIn)
    .handle()
```

---

## 7. Input Handling

Input events (keyboard, mouse, touch, stylus) are translated into a unified `InputEvent` enum by the system.  The framework determines which widget is under the pointer via hit‑testing the layout rectangles, and sends the appropriate message to the `App` actor.

```
pub enum InputEvent {
    Mouse(MouseEvent),
    Key(KeyEvent),
    Scroll(ScrollEvent),
    Touch(TouchEvent),
}

pub struct MouseEvent {
    pub kind: MouseKind,
    pub button: MouseButton,
    pub position: (f64, f64),
    pub modifiers: Modifiers,
}
// ...
```

The developer never handles raw input; they deal only with high‑level messages like `ButtonClicked` or `SliderChanged`.

---

## 8. Rendering Backend

The default renderer is `blaze‑wgpu`.  The `Renderer` trait abstracts the GPU work:

```
pub trait Renderer {
    fn draw_widget(&mut self, node: &WidgetNode, rect: Rect, style: &Style, state: &AppState);
    fn present(&mut self);
}
```

The `App` actor iterates over the widget tree and calls `draw_widget` for each node, passing the computed rectangle and style.  The renderer captures drawing commands (text, rectangles, images) and submits them to the GPU.

---

## 9. Immediate‑Mode Efficiency

Despite rebuilding the widget tree every frame, Blaze’s flat array representation and SoA layout make the process extremely fast:

- Nodes are allocated in a bump arena, freed at the end of the frame.
- The tree is traversed linearly, with excellent cache locality.
- The layout solver is O(N) with a small constant.
- Accessibility tree construction is also O(N) and only runs if assistive technologies are active (detected lazily).
- For static parts of the UI, the developer can memoize subtrees using a simple `memo` helper that caches the `WidgetHandle` and only rebuilds when the relevant state slice changes.

This allows 120 FPS rendering of scenes with thousands of widgets.

---

## 10. Integration with Games and Other Frameworks

The GUI system is entirely self‑contained and can be embedded in any Blaze application that uses `blaze‑wgpu`.  It does not assume a windowing environment; a game engine can call the `App` update loop directly and composite the GUI on top of its own rendering.

---

## 11. Testing

- **Layout solver:** validate that various constraint combinations produce correct rectangles.
- **Accessibility tree:** build a UI, extract the accessibility tree, verify roles, labels, and actions.
- **Event handling:** simulate a mouse click on a button, ensure the correct message is sent to the actor.
- **Style propagation:** check that a globally set style is used by widgets that do not override it.
- **Snapshot tests:** render a simple UI off‑screen and compare pixel output with a known reference.
- **Performance benchmark:** measure frame time for a UI with 10,000 widgets, ensure it stays under 8 ms.

All tests must pass with the `blaze‑wgpu` backend (using software emulation in CI).
