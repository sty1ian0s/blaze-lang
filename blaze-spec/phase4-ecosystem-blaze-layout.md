# Phase 4 – Ecosystem Crate: `blaze‑layout`

> **Goal:** Provide a pure, data‑oriented, constraint‑based layout engine for the Blaze GUI system.  The layout engine takes a flat widget tree and a set of available dimensions, and produces a list of rectangles, one per widget.  It supports pixel, percentage, and auto‑sizing for width/height, padding, margin, alignment, flex‑box distribution, and absolute positioning.  The solver is deterministic, O(N) with respect to the number of widgets, and does not allocate on the heap after initial setup.  It is used by `blaze‑gui` and any other Blaze component that needs 2D layout.

---

## 1. Core Concepts

- **Constraints** – each widget node carries a `LayoutConstraints` struct that specifies its desired size and position.
- **Parent‑relative sizing** – sizes may be defined as pixels, percentage of parent, or automatic (determined by content).
- **Flex distribution** – remaining space in a `Row` or `Column` is distributed among children according to their `flex` weights.
- **Alignment** – children are aligned within their parent according to horizontal and vertical alignment.
- **Absolute positioning** – widgets inside a `Stack` may specify exact pixel positions.
- **Pure function** – the solver is a single function `fn solve_layout(tree: &[LayoutNode], available_width: f64, available_height: f64) -> Vec<Rect>`.  It has no side effects and is trivially parallelisable.

---

## 2. Input: `LayoutNode`

Each widget in the tree is described by a `LayoutNode`:

```
pub struct LayoutNode {
    pub id: u32,                    // unique widget identifier
    pub parent_id: Option<u32>,     // parent widget id, None for root
    pub children_ids: Vec<u32>,     // child widget ids
    pub constraints: LayoutConstraints,
    pub intrinsic_size: Option<(f64, f64)>,  // content‑based size (e.g., text)
}

pub struct LayoutConstraints {
    pub width: SizeConstraint,
    pub height: SizeConstraint,
    pub min_width: Option<f64>,
    pub max_width: Option<f64>,
    pub min_height: Option<f64>,
    pub max_height: Option<f64>,
    pub padding: EdgeInsets,
    pub margin: EdgeInsets,
    pub alignment: Alignment,
    pub flex: f64,
    pub position: Option<Position>,  // for absolute positioning in Stack
}

pub enum SizeConstraint {
    Pixels(f64),
    Percent(f64),
    Auto,
}

pub struct EdgeInsets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

pub struct Alignment {
    pub horizontal: HorizontalAlign,
    pub vertical: VerticalAlign,
}

pub enum HorizontalAlign { Left, Center, Right, Stretch }
pub enum VerticalAlign { Top, Center, Bottom, Stretch }

pub struct Position {
    pub x: f64,
    pub y: f64,
}
```

The `LayoutNode` tree is built by the GUI framework during widget tree construction.  It is a flat vector of nodes indexed by `u32` handles.

---

## 3. Output: `Rect`

```
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
```

The solver returns a `Vec<Rect>` with one entry per widget, indexed by the widget’s `id`.  The x/y coordinates are relative to the root widget’s origin (top‑left corner of the window).

---

## 4. Solver Algorithm

The solver runs in two phases:

### 4.1 Bottom‑Up Size Computation

Traverse the tree from leaves to root.  For each node:

- If `SizeConstraint::Pixels`, the size is fixed.
- If `SizeConstraint::Percent`, the size is computed relative to the parent’s computed size (deferred until the parent size is known).
- If `SizeConstraint::Auto`, the size is determined by the widget’s intrinsic size (content), plus padding.  For containers, the auto size is the size needed to fit all children (after padding).

If both width and height are known, the node’s intrinsic size is stored for the top‑down phase.

### 4.2 Top‑Down Position Assignment

Traverse the tree from root to leaves.  For each node:

1. Compute the content area by subtracting padding and margin from the available space.
2. For each child in a `Row` or `Column`:
   - Sum the fixed and auto sizes of all children.
   - Distribute remaining space among `Stretch` children or according to `flex` weights.
   - Compute the child’s final size and position, enforcing min/max constraints.
3. For absolute‑positioned children (in a `Stack`), use the explicit position and their desired size.
4. Apply alignment: if a child is smaller than the available space in the cross‑axis, shift it according to `Alignment`.

Special cases:

- **Text** reports its intrinsic size as the rendered width/height for the given font and content.
- **Image** uses the image’s native dimensions, scaled if constrained.
- **Custom widgets** provide an `intrinsic_size` callback.

The entire solver is a single‑pass walk of the flat node array in the correct order (first depth‑first for sizing, then breadth‑first for positioning).  It is O(N) with a small constant factor.

---

## 5. Integration with `blaze‑gui`

The `App` actor in `blaze‑gui` calls the solver during its update loop:

```
let layout_nodes: Vec<LayoutNode> = self.widget_tree.to_layout_nodes();
let rects = solve_layout(&layout_nodes, window_width, window_height);
self.renderer.draw(&self.widget_tree, &rects, &self.style);
self.accessibility_backend.update(&self.widget_tree, &rects);
```

The `LayoutNode` tree is produced from the `WidgetNode` tree by the GUI framework; the developer never constructs layout nodes manually.

---

## 6. Deterministic and Reproducible Behaviour

The solver is a pure function.  Given the same input and deterministic floating‑point settings (`--reproducible`), the output rectangles are bit‑identical across runs and platforms.  This is critical for testing and for consistent UI appearance.

---

## 7. Testing

- **Basic sizing:** create a simple widget with `Pixels(100)`, verify it receives a 100×100 rectangle.
- **Percentage sizing:** place a widget inside a parent with `Percent(50.0)`, verify it gets half the parent’s size.
- **Auto sizing:** a `Text` widget with known content reports its intrinsic size correctly, and the solver assigns that size.
- **Flex distribution:** three children in a `Row` with flex weights 1,2,1; verify the middle child gets twice the extra space.
- **Alignment:** a child with `Center` alignment in a larger parent; verify its position is centered.
- **Absolute positioning:** a child in a `Stack` with `Position {x: 10, y: 20}`; verify its coordinates.
- **Min/max constraints:** a child with `min_width` and `max_width` that conflict; verify the solver clamps correctly.
- **Edge cases:** a deep tree (depth 1000) and many nodes (10,000 widgets) must complete in under 1 ms.
- **Reproducibility:** run the same layout twice with `--reproducible` and verify identical rectangles.

All tests must pass on all platforms.
