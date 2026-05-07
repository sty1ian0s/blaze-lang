# Phase 5 – Ecosystem Crate: `blaze‑plot` (updated for `blaze‑gui` builder API)

> **Goal:** Provide a data‑oriented, declarative plotting library that integrates seamlessly with the `blaze‑gui` widget‑tree system.  Plots are created by composing mark layers on a `Plot` builder, and the final result is a `WidgetHandle` that can be placed directly inside any `blaze‑gui` layout.  All rendering is pure (or carries the `gpu` effect when a GPU backend is used), and the plot is automatically accessible, responsive, and stylable with typed `Style` values.

## 1. Core Concepts

A plot is a **pure function** that takes data and configuration and returns a `WidgetHandle`.  Internally, the plot is just another widget node – a `WidgetNode::Canvas` with a specialised draw function.  There is no separate widget trait; the plot becomes a first‑class citizen of the `blaze‑gui` tree.

```
fn plot_widget(state: &AppState) -> WidgetHandle {
    Plot::new()
        .scatter(state.points, |p| (p.x, p.y))
        .line(state.trend, |t| (t.x, t.y))
        .x_axis_label("X")
        .y_axis_label("Y")
        .width(600.0)
        .height(400.0)
        .handlem()   // returns WidgetHandle
}
```

The returned `WidgetHandle` can be used inside any layout container (`Row`, `Column`, `Container`) just like a `Button` or `Text` widget.  The plot participates in layout, accessibility, and event handling automatically.

## 2. `Plot` Builder API

The `Plot` builder is unchanged from the previous version, but its final method is now `handlem()` instead of `render()`.

### 2.1 `Plot`

```
pub struct Plot {
    layers: Vec<Layer>,
    width: f64,
    height: f64,
    title: Option<Text>,
    x_axis_label: Option<Text>,
    y_axis_label: Option<Text>,
    color_scale: ColorScale,
    legend: Option<LegendConfig>,
    margin: EdgeInsets,
    background: Color,
}

impl Plot {
    pub fn new() -> Plot;

    pub fn width(mut self, w: f64) -> Self;
    pub fn height(mut self, h: f64) -> Self;
    pub fn title(mut self, title: &str) -> Self;
    pub fn x_axis_label(mut self, label: &str) -> Self;
    pub fn y_axis_label(mut self, label: &str) -> Self;
    pub fn color_scale(mut self, scale: ColorScale) -> Self;
    pub fn legend(mut self, config: LegendConfig) -> Self;
    pub fn margin(mut self, margin: EdgeInsets) -> Self;
    pub fn background(mut self, color: Color) -> Self;

    pub fn layer(mut self, mark: impl IntoLayer) -> Self;
    pub fn handlem(self) -> WidgetHandle;
}
```

- `handlem` consumes the `Plot` builder, creates a `WidgetNode::Canvas` with the plot’s data and a pure drawing function, and returns a `WidgetHandle`.

### 2.2 Marks and Layers (identical to previous spec)

Marks are defined as enum variants or structs implementing `IntoLayer`.  The builder closures remain the same:

```blaze
Plot::new()
    .scatter(data, |row| (row.x, row.y))
    .line(data, |row| (row.x, row.y))
    .bar(data, |row| (row.category, row.value))
    .histogram(data, |val| val, bins = 30)
    .heatmap(data, |row| (row.x, row.y, row.intensity))
    .contour(data, |row| (row.x, row.y, row.z))
    .text(data, |row| (row.x, row.y, row.label))
    .error_bar(data, |row| (row.x, row.y, row.y_err))
    .area(data, |row| (row.x, row.y_upper, row.y_lower))
    .box_plot(data, |row| (row.category, values: &[f64]))
    .handlem()
```

## 3. Integration with `blaze‑gui`

### 3.1 Layout

The plot respects the `width` and `height` properties like any other widget.  If these are not set, it uses `Auto` and fills the parent container’s available space.  The plot’s `margin` is applied during layout by the `blaze‑gui` solver.

### 3.2 Styling

The plot’s appearance is controlled by the `Style` passed down from the parent widget, plus the plot‑specific properties (`color_scale`, `legend`, `background`).  The style’s `foreground` and `font_size` influence axis labels and title text, while the plot’s own `background` overrides the parent’s `background` for the plot area.

### 3.3 Responsiveness

Because the plot is just a widget, it automatically participates in `blaze‑gui`’s responsive layout.  The user can branch on available width:

```blaze
fn dashboard_ui(state: &AppState, window_size: (f64, f64)) -> WidgetHandle {
    let plot_width = window_size.0 * 0.6;
    let plot_height = 400.0;
    Plot::new()
        .scatter(state.data, |p| (p.x, p.y))
        .width(plot_width)
        .height(plot_height)
        .handlem()
}
```

### 3.4 Interactivity

The plot supports built‑in zoom/pan interactions via the canvas’s draw function.  Events like mouse drags or scrolls are captured by the canvas widget and translated into plot transformations stored in the app state.  The user can also add custom click handlers by wrapping the plot in a `Container` with an `on_click` message.

## 4. Accessibility

The plot widget automatically exposes a `StaticText` role and, if a title is set, provides the title as its accessible name.  Internally, the data is also exposed as a simple textual summary (e.g., “Scatter plot with 120 points”) for screen readers.  This can be overridden with an explicit `a11y` method on the builder:

```blaze
Plot::new()
    .scatter(…)
    .a11y(Accessibility::new().label("Quarterly sales trend"))
    .handlem()
```

## 5. Rendering and Export

In addition to being a live widget, the plot can also be exported to static images (PNG, SVG) using the same code paths as before.  The `Plot` builder retains `to_png` and `to_svg` methods for headless use, which internally create a temporary canvas and render offline.

## 6. Testing

- **Widget integration:** Construct a plot via the builder, insert it into a `Column`, run the layout solver, verify the plot’s allocated rectangle matches its requested size.
- **Accessibility:** Build a plot with a title, extract the auto‑generated `A11yNode`, and check the role and name.
- **Export:** Generate a PNG from a plot and compare with a reference image using `blaze‑image`.
- **Event handling:** Simulate a scroll event on a plot canvas widget and verify that the zoom factor in the application state is updated.

All tests must pass on all platforms.
