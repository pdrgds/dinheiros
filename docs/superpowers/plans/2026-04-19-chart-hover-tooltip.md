# History Chart Hover Tooltip — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a mouse-hover tooltip + dot + vertical guide to the history chart, showing the value at the hovered date. In single-line mode, show total. In split mode, snap to the line closest to the cursor's Y and show that category's value.

**Architecture:** Cursor position is stored on `AppRoot` as `history_hover: Option<HistoryHover>`. Mouse handlers attached to the chart container update it and call `cx.notify()`. The chart container's bounds are captured in a shared `Arc<Mutex<Option<Bounds<Pixels>>>>` written from the canvas prepaint and read by the mouse handlers, so we can convert window-absolute mouse coords to chart-relative coords. The canvas paints the existing chart plus a vertical guide line + dot at the snapped data point. A separate absolutely-positioned `div` overlays the tooltip text. Pure helpers (`snap_index`, `active_series_idx`) are unit-tested; the rendering itself is verified manually.

**Tech Stack:** Rust, GPUI (`gpui-ce` patched at `../gpui-ce`). The chart code lives in `crates/ui/src/views/history.rs`. `AppRoot` lives in `crates/ui/src/app.rs`.

**Spec:** `docs/superpowers/specs/2026-04-19-chart-hover-tooltip-design.md`

---

### Task 1: Pure helpers — `snap_index` and `active_series_idx`

These are stateless functions that map cursor coords + chart geometry to the snapped data index and the active series. They contain all the math that's worth testing automatically; everything else is GPUI rendering verified manually in the running app.

**Files:**
- Modify: `crates/ui/src/views/history.rs` (add the two functions and a `#[cfg(test)] mod hover_tests`)

- [ ] **Step 1.1: Add the failing tests for `snap_index`**

Append to `crates/ui/src/views/history.rs`:

```rust
#[cfg(test)]
mod hover_tests {
    use super::*;
    use gpui::px;

    #[test]
    fn snap_index_clamps_to_left_edge() {
        // Cursor before the plot area → first data point.
        assert_eq!(snap_index(px(0.0), px(10.0), px(200.0), 5), 0);
        assert_eq!(snap_index(px(-30.0), px(10.0), px(200.0), 5), 0);
    }

    #[test]
    fn snap_index_clamps_to_right_edge() {
        // Cursor past the plot area → last data point.
        // chart_pad = 10, chart_w = 200 ⇒ plot ends at rel_x = 210.
        assert_eq!(snap_index(px(210.0), px(10.0), px(200.0), 5), 4);
        assert_eq!(snap_index(px(500.0), px(10.0), px(200.0), 5), 4);
    }

    #[test]
    fn snap_index_picks_nearest_interior_point() {
        // 5 points spread across 200px ⇒ 50px between points, starting at rel_x=10.
        // rel_x=85 is 75px into the plot ⇒ between point 1 (50px) and point 2 (100px),
        // closer to point 2 (distance 25 vs 25 — tie goes to round-half-away-from-zero).
        assert_eq!(snap_index(px(85.0), px(10.0), px(200.0), 5), 2);
        // rel_x=84.9 is unambiguously closer to point 1.
        assert_eq!(snap_index(px(84.9), px(10.0), px(200.0), 5), 1);
    }

    #[test]
    fn snap_index_single_point() {
        // n == 1 always returns 0.
        assert_eq!(snap_index(px(50.0), px(10.0), px(200.0), 1), 0);
    }

    #[test]
    fn snap_index_zero_points() {
        // n == 0 still returns 0 (caller is responsible for not rendering when empty).
        assert_eq!(snap_index(px(50.0), px(10.0), px(200.0), 0), 0);
    }
}
```

- [ ] **Step 1.2: Run the test — confirm it fails for the right reason**

```
cargo test -p investimentos-ui hover_tests::snap_index 2>&1 | tail -20
```

Expected: compile error `cannot find function 'snap_index' in this scope`.

- [ ] **Step 1.3: Implement `snap_index`**

Add to `crates/ui/src/views/history.rs` (above the test module):

```rust
/// Map a cursor x (relative to the chart container's top-left) to the nearest
/// data index. `chart_pad` is the canvas's inner padding; `chart_w` is the
/// width of the plot area (i.e. canvas width minus 2× pad). Clamps outside
/// the plot to the closest endpoint. Returns 0 when `n` <= 1.
fn snap_index(
    rel_x: gpui::Pixels,
    chart_pad: gpui::Pixels,
    chart_w: gpui::Pixels,
    n: usize,
) -> usize {
    if n <= 1 {
        return 0;
    }
    let x = f32::from(rel_x - chart_pad);
    let w = f32::from(chart_w);
    if x <= 0.0 {
        return 0;
    }
    if x >= w {
        return n - 1;
    }
    let step = w / (n - 1) as f32;
    (x / step).round() as usize
}
```

- [ ] **Step 1.4: Run the test — confirm it passes**

```
cargo test -p investimentos-ui hover_tests::snap_index 2>&1 | tail -10
```

Expected: 5 passed.

- [ ] **Step 1.5: Add the failing tests for `active_series_idx`**

Append inside the same `mod hover_tests`:

```rust
    #[test]
    fn active_series_idx_picks_closest_y() {
        // Two series, snapped at index 0. Series A value is 100 (high), series B is 10 (low).
        // y_max = 100, chart_pad = 10, chart_h = 200 ⇒
        //   series A paints at chart_y + 0  (top)    = 10
        //   series B paints at chart_y + 180        = 190
        // Cursor near top picks series A (index 0); cursor near bottom picks series B (index 1).
        let series = vec![
            (gpui::rgb(0x111111).into(), vec![(0usize, 100.0_f64)]),
            (gpui::rgb(0x222222).into(), vec![(0usize, 10.0_f64)]),
        ];
        assert_eq!(
            active_series_idx(px(20.0), 0, &series, px(10.0), px(200.0), 100.0),
            Some(0)
        );
        assert_eq!(
            active_series_idx(px(180.0), 0, &series, px(10.0), px(200.0), 100.0),
            Some(1)
        );
    }

    #[test]
    fn active_series_idx_ties_pick_first() {
        // Both series at the same value ⇒ tie ⇒ first wins (defined tiebreaker).
        let series = vec![
            (gpui::rgb(0x111111).into(), vec![(0usize, 50.0_f64)]),
            (gpui::rgb(0x222222).into(), vec![(0usize, 50.0_f64)]),
        ];
        assert_eq!(
            active_series_idx(px(100.0), 0, &series, px(10.0), px(200.0), 100.0),
            Some(0)
        );
    }

    #[test]
    fn active_series_idx_skips_series_without_value_at_index() {
        // Series A has no point at index 0 (sparse). Only series B is considered.
        let series = vec![
            (gpui::rgb(0x111111).into(), vec![(1usize, 100.0_f64)]),
            (gpui::rgb(0x222222).into(), vec![(0usize, 50.0_f64)]),
        ];
        assert_eq!(
            active_series_idx(px(50.0), 0, &series, px(10.0), px(200.0), 100.0),
            Some(1)
        );
    }

    #[test]
    fn active_series_idx_returns_none_when_no_series_has_value() {
        let series = vec![
            (gpui::rgb(0x111111).into(), vec![(1usize, 100.0_f64)]),
            (gpui::rgb(0x222222).into(), vec![(2usize, 50.0_f64)]),
        ];
        assert_eq!(
            active_series_idx(px(50.0), 0, &series, px(10.0), px(200.0), 100.0),
            None
        );
    }
```

- [ ] **Step 1.6: Run the new tests — confirm they fail for the right reason**

```
cargo test -p investimentos-ui hover_tests::active_series_idx 2>&1 | tail -20
```

Expected: compile error `cannot find function 'active_series_idx' in this scope`.

- [ ] **Step 1.7: Implement `active_series_idx`**

Add to `crates/ui/src/views/history.rs` (next to `snap_index`):

```rust
/// In split mode, given the snapped data index and the cursor y, return the
/// index of the series whose painted y at that index is closest to `rel_y`.
/// Series without a value at `snapped_idx` are skipped. Ties are broken by
/// series order (first wins). Returns `None` if no series has a value at
/// the snapped index.
fn active_series_idx(
    rel_y: gpui::Pixels,
    snapped_idx: usize,
    series: &[(gpui::Rgba, Vec<(usize, f64)>)],
    chart_pad: gpui::Pixels,
    chart_h: gpui::Pixels,
    y_max: f64,
) -> Option<usize> {
    let cursor = f32::from(rel_y);
    let pad = f32::from(chart_pad);
    let h = f32::from(chart_h);

    let mut best: Option<(usize, f32)> = None;
    for (idx, (_color, points)) in series.iter().enumerate() {
        let Some((_, v)) = points.iter().find(|(i, _)| *i == snapped_idx) else {
            continue;
        };
        let painted_y = pad + h * (1.0 - (*v / y_max) as f32);
        let dist = (painted_y - cursor).abs();
        match best {
            Some((_, best_dist)) if dist >= best_dist => {} // first-wins tiebreak
            _ => best = Some((idx, dist)),
        }
    }
    best.map(|(idx, _)| idx)
}
```

- [ ] **Step 1.8: Run the new tests — confirm they pass**

```
cargo test -p investimentos-ui hover_tests::active_series_idx 2>&1 | tail -10
```

Expected: 4 passed.

- [ ] **Step 1.9: Run the full hover-tests module and the whole UI suite**

```
cargo test -p investimentos-ui hover_tests 2>&1 | tail -10
cargo test -p investimentos-ui 2>&1 | tail -10
```

Expected: 9 hover tests pass; UI suite green (or unchanged baseline).

- [ ] **Step 1.10: Commit**

```bash
git add crates/ui/src/views/history.rs
git commit -m "feat(history): pure snap_index and active_series_idx helpers"
```

---

### Task 2: Add `HistoryHover` state to `AppRoot`

State only — no behaviour yet. Splitting it out keeps the next task's diff small and focused on wiring.

**Files:**
- Modify: `crates/ui/src/app.rs`

- [ ] **Step 2.1: Add the `HistoryHover` struct and the `AppRoot` field**

In `crates/ui/src/app.rs`, alongside the existing `BackfillStatus` block (around line 19):

```rust
#[derive(Clone, Copy, Debug)]
pub struct HistoryHover {
    /// Cursor x relative to the chart container's top-left, in pixels.
    pub rel_x: gpui::Pixels,
    /// Cursor y relative to the chart container's top-left, in pixels.
    pub rel_y: gpui::Pixels,
}
```

Add the field to `AppRoot` (alongside `pub backfill_status`):

```rust
pub history_hover: Option<HistoryHover>,
```

In `AppRoot::new` initializer block (next to `backfill_status,`):

```rust
history_hover: None,
```

- [ ] **Step 2.2: Confirm the workspace still compiles**

```
cargo check -p investimentos-ui 2>&1 | tail -5
```

Expected: no errors (warnings about unused field are OK; we use it next).

- [ ] **Step 2.3: Commit**

```bash
git add crates/ui/src/app.rs
git commit -m "feat(history): add HistoryHover state to AppRoot"
```

---

### Task 3: Wire mouse handlers and capture chart bounds

The chart container currently does not handle mouse input. Add `on_mouse_move` and `on_hover` listeners that update `AppRoot.history_hover`. We need the container's window-absolute bounds to convert `event.position` (also window-absolute) to container-relative coords; capture them via the canvas prepaint into an `Arc<Mutex<Option<Bounds<Pixels>>>>` shared with the listeners.

This task does the wiring for **both** chart modes (single-line and split). No visuals yet — verify wiring by adding a temporary debug print and confirming the state updates as the cursor moves.

**Files:**
- Modify: `crates/ui/src/views/history.rs`

- [ ] **Step 3.1: Extract a small helper that builds the chart container with handlers**

Add near the other helpers in `crates/ui/src/views/history.rs`:

```rust
use std::sync::Mutex;
use gpui::{Bounds, MouseMoveEvent};

use crate::app::{AppRoot, HistoryHover};

/// Wrap the chart container with mouse handlers that update
/// `AppRoot.history_hover`. `bounds_cell` is written by the canvas prepaint
/// so the handlers can convert window-absolute mouse coords to container-
/// relative coords.
fn attach_hover_handlers(
    container: Div,
    bounds_cell: Arc<Mutex<Option<Bounds<Pixels>>>>,
    cx: &mut Context<AppRoot>,
) -> Div {
    let bounds_for_move = bounds_cell.clone();
    container
        .relative()
        .on_mouse_move(cx.listener(move |app, event: &MouseMoveEvent, _window, cx| {
            let Some(bounds) = *bounds_for_move.lock().unwrap() else {
                return;
            };
            let rel_x = event.position.x - bounds.origin.x;
            let rel_y = event.position.y - bounds.origin.y;
            app.history_hover = Some(HistoryHover { rel_x, rel_y });
            cx.notify();
        }))
        .on_hover(cx.listener(move |app, hovered: &bool, _window, cx| {
            if !*hovered {
                app.history_hover = None;
                cx.notify();
            }
        }))
}
```

- [ ] **Step 3.2: Use the helper for the single-line container and capture bounds in the canvas prepaint**

Find the single-line container in `render_chart` (the `div().flex_1().h(px(300.0)).rounded_lg().bg(theme::BG_SECONDARY).border_1()...child(canvas(...))` block, around line 316). Refactor it as:

```rust
let bounds_cell: Arc<Mutex<Option<Bounds<Pixels>>>> = Arc::new(Mutex::new(None));
let bounds_for_canvas = bounds_cell.clone();
let chart_div = div()
    .flex_1()
    .h(px(300.0))
    .rounded_lg()
    .bg(theme::BG_SECONDARY)
    .border_1()
    .border_color(theme::BORDER)
    .child(
        canvas(
            move |bounds, _window, _cx| {
                *bounds_for_canvas.lock().unwrap() = Some(bounds);
                points_arc.clone()
            },
            move |bounds, points, window, _cx| {
                paint_chart(bounds, &points, window);
            },
        )
        .size_full(),
    );
let chart_div = attach_hover_handlers(chart_div, bounds_cell, cx);
```

Then replace the existing inline child with `.child(chart_div)`.

`render_chart` doesn't currently take `cx`. Update its signature to:

```rust
fn render_chart(
    points: &[(NaiveDate, f64)],
    cx: &mut Context<AppRoot>,
) -> AnyElement {
```

…and update its caller in `render_history` accordingly (pass `cx` through).

- [ ] **Step 3.3: Apply the same change to the multi-line container**

In `render_multi_chart`, mirror the same refactor: build a `bounds_cell`, capture it in the canvas prepaint, wrap the container with `attach_hover_handlers`. Add `cx: &mut Context<AppRoot>` to the function signature and update the call site in `render_history`.

- [ ] **Step 3.4: Add a temporary debug print to confirm wiring**

Inside the `on_mouse_move` listener (in `attach_hover_handlers`), temporarily add:

```rust
eprintln!("[hover] rel=({:?}, {:?})", rel_x, rel_y);
```

- [ ] **Step 3.5: Build, run the app, and verify mouse events reach the listener**

```
cargo build -p investimentos-ui --release
target/release/investimentos-ui
```

Move the cursor over the chart. The terminal should print `[hover] rel=(...)` lines with values that increase from left to right and top to bottom inside the chart. Move the cursor off the chart — printing should stop.

If the values look wrong (negative, way too large, jumping around), inspect: are the bounds being captured? Add `eprintln!("[bounds] {:?}", bounds)` inside the canvas prepaint to compare.

- [ ] **Step 3.6: Remove the debug print**

Delete the `eprintln!` line added in step 3.4.

- [ ] **Step 3.7: Confirm tests still pass**

```
cargo test -p investimentos-ui 2>&1 | tail -10
```

- [ ] **Step 3.8: Commit**

```bash
git add crates/ui/src/views/history.rs
git commit -m "feat(history): mouse handlers populate AppRoot.history_hover"
```

---

### Task 4: Single-line chart — paint dot + vertical guide + render tooltip

Now the visual. Compute hover info in `render_chart`, pass it into the canvas paint, draw the indicator, and add a sibling tooltip `div`.

**Files:**
- Modify: `crates/ui/src/views/history.rs`

- [ ] **Step 4.1: Define a `HoverDecoration` struct and compute it in `render_chart`**

Above `render_chart`, add:

```rust
/// What the canvas paint and the tooltip overlay need to know to render the
/// hover indicator. All coordinates are window-absolute (from the canvas's
/// own paint bounds), except `dot_x_in_container` / `dot_y_in_container`
/// which are relative to the chart container's top-left (used by the
/// absolutely-positioned tooltip `div`).
#[derive(Clone)]
struct HoverDecoration {
    snapped_idx: usize,
    series_color: Rgba,
    /// Position of the dot relative to the chart container, for the tooltip.
    dot_x_in_container: Pixels,
    dot_y_in_container: Pixels,
    /// Rendered tooltip text (date + value, e.g. "Apr 17, 2026 — R$ 89.4k").
    tooltip_text: String,
}
```

In `render_chart`, after `points` is bound and before the `div()` builder starts, read the current hover and compute the decoration. The chart container has internal padding `pad = px(10.0)` (matching what `paint_chart` uses); compute everything relative to that.

```rust
let pad = px(10.0);
// The chart container is 300px tall and `flex_1` wide. We can't know the exact
// width here (it's flex), but we don't need to — bounds are captured in the
// canvas prepaint, and we recompute snap inside the canvas paint using those
// bounds. For the tooltip's `dot_x_in_container` we need the *container-relative*
// coords. The canvas's bounds.origin equals the container's origin (no offset),
// so `dot_x_in_container = pad + chart_w * (snapped_idx / (n-1))`.

let hover_for_render: Option<HoverDecoration> = {
    let hover = cx.entity().read(cx).history_hover;
    hover.and_then(|h| {
        let n = points.len();
        if n < 2 {
            return None;
        }
        // We don't know chart_w until paint, so snap and tooltip math both
        // happen inside a closure that the canvas can also call. To keep the
        // tooltip render outside the canvas, we approximate using the cursor
        // position alone for the tooltip text + index, and rely on the canvas
        // to know exact bounds. The cursor is in container coords; the plot
        // starts at `pad`. We pass the bounds_cell into the closure.
        let bounds = (*bounds_cell.lock().unwrap())?;
        let chart_w = bounds.size.width - pad * 2.0;
        let chart_h = bounds.size.height - pad * 2.0;
        let snapped = snap_index(h.rel_x, pad, chart_w, n);
        let (date, value) = points[snapped];
        let chart_x_in_container = pad;
        let chart_y_in_container = pad;
        let to_px_x = |i: usize| -> Pixels {
            chart_x_in_container + chart_w * (i as f32 / (n - 1).max(1) as f32)
        };
        // Reconstruct y_min / y_max identically to `paint_chart`:
        let min_val = points.iter().map(|(_, v)| *v).fold(f64::INFINITY, f64::min);
        let max_val = points.iter().map(|(_, v)| *v).fold(f64::NEG_INFINITY, f64::max);
        let range = (max_val - min_val).max(1.0);
        let y_min = min_val - range * 0.05;
        let y_max = max_val + range * 0.05;
        let y_range = y_max - y_min;
        let to_px_y = |v: f64| -> Pixels {
            chart_y_in_container + chart_h * (1.0 - ((v - y_min) / y_range) as f32)
        };
        Some(HoverDecoration {
            snapped_idx: snapped,
            series_color: gpui::rgb(0x06b6d4).into(),
            dot_x_in_container: to_px_x(snapped),
            dot_y_in_container: to_px_y(value),
            tooltip_text: format!("{} — {}", date.format("%b %d, %Y"), format_brl(value)),
        })
    })
};
```

Ordering note: place this `hover_for_render` block immediately *after* the `let bounds_cell = ...` line from Task 3.2 and *before* the `let chart_div = div()...` block, so `bounds_cell` is in scope and `hover_for_render` is available when the canvas closures and the tooltip child are wired up.

- [ ] **Step 4.2: Pass the decoration into the canvas paint**

Wrap the decoration in an `Arc` so the paint callback can capture it:

```rust
let hover_for_canvas: Arc<Option<HoverDecoration>> = Arc::new(hover_for_render.clone());
```

Update the canvas paint callback:

```rust
move |bounds, points, window, _cx| {
    paint_chart(bounds, &points, window);
    if let Some(deco) = hover_for_canvas.as_ref().as_ref() {
        paint_hover_indicator(bounds, deco, window);
    }
}
```

- [ ] **Step 4.3: Implement `paint_hover_indicator`**

Add to `crates/ui/src/views/history.rs`:

```rust
fn paint_hover_indicator(
    bounds: Bounds<Pixels>,
    deco: &HoverDecoration,
    window: &mut gpui::Window,
) {
    let pad = px(10.0);
    let chart_x = bounds.origin.x + pad;
    let chart_y = bounds.origin.y + pad;
    let chart_h = bounds.size.height - pad * 2.0;

    // The dot's position inside the canvas equals the container-relative
    // position (pad-based), since canvas bounds.origin == container.origin.
    let dot_x = bounds.origin.x + deco.dot_x_in_container;
    let dot_y = bounds.origin.y + deco.dot_y_in_container;

    // Vertical guide line.
    let mut guide = PathBuilder::stroke(px(1.0));
    guide.move_to(point(dot_x, chart_y));
    guide.line_to(point(dot_x, chart_y + chart_h));
    if let Ok(path) = guide.build() {
        window.paint_path(path, theme::BORDER);
    }

    // Dot: two stacked filled quads with full corner-radii (= filled circles).
    // Outer ring in BG_SECONDARY for contrast against the line, inner in the
    // series color. The `quad(bounds, corner_radii, background, border_widths,
    // border_color, border_style)` signature is verified in
    // `../gpui-ce/src/window.rs:5650`.
    let radius = px(4.0);
    let stroke = px(1.5);
    let outer_r = radius + stroke;
    let outer = Bounds {
        origin: point(dot_x - outer_r, dot_y - outer_r),
        size: gpui::size(outer_r * 2.0, outer_r * 2.0),
    };
    window.paint_quad(gpui::quad(
        outer,
        outer_r,
        theme::BG_SECONDARY,
        gpui::Edges::default(),
        gpui::transparent_black(),
        gpui::BorderStyle::Solid,
    ));
    let inner = Bounds {
        origin: point(dot_x - radius, dot_y - radius),
        size: gpui::size(radius * 2.0, radius * 2.0),
    };
    window.paint_quad(gpui::quad(
        inner,
        radius,
        deco.series_color,
        gpui::Edges::default(),
        gpui::transparent_black(),
        gpui::BorderStyle::Solid,
    ));
}
```

Imports needed at the top of `crates/ui/src/views/history.rs` (additions to the existing `use gpui::{...}` line): `BorderStyle, Edges, quad, size, transparent_black`.

- [ ] **Step 4.4: Add the tooltip overlay to the chart container**

Inside `render_chart`, after computing `hover_for_render` and *before* wrapping with `attach_hover_handlers`, add the tooltip as a sibling of the canvas inside the chart_div. The container is already `.relative()` (added by `attach_hover_handlers`). Build the chart_div like:

```rust
let chart_div = chart_div.children(hover_for_render.as_ref().map(|deco| {
    let tooltip_w = px(200.0);
    let tooltip_offset = px(12.0);
    // Edge flip: if the right edge of the would-be tooltip falls outside the
    // container width (read from bounds_cell), place it to the left of the dot.
    let container_w = bounds_cell
        .lock()
        .unwrap()
        .map(|b| b.size.width)
        .unwrap_or(px(0.0));
    let left = if deco.dot_x_in_container + tooltip_offset + tooltip_w > container_w {
        deco.dot_x_in_container - tooltip_offset - tooltip_w
    } else {
        deco.dot_x_in_container + tooltip_offset
    };
    div()
        .absolute()
        .left(left)
        .top(deco.dot_y_in_container - px(14.0))
        .w(tooltip_w)
        .px(px(8.0))
        .py(px(4.0))
        .rounded_md()
        .bg(theme::BG_SECONDARY)
        .border_1()
        .border_color(theme::BORDER)
        .text_xs()
        .text_color(theme::TEXT_PRIMARY)
        .child(deco.tooltip_text.clone())
}));
```

(`theme::TEXT_PRIMARY` is defined in `crates/ui/src/theme.rs` as `rgb_const(0xe0, 0xe0, 0xe0)`.)

- [ ] **Step 4.5: Build and run the app, verify single-line hover works**

```
cargo build -p investimentos-ui --release
target/release/investimentos-ui
```

In the History tab with **split off**, hover over the chart. Verify:
- A small dot appears on the line, snapping to the nearest day as the cursor moves.
- A thin vertical guide line spans the chart height through the dot.
- A tooltip appears next to the dot showing `<date> — <BRL value>`.
- Moving near the right edge flips the tooltip to the left of the dot.
- Moving the cursor out of the chart hides everything.

If a step misbehaves, inspect with targeted `eprintln!`s; common issues are
sign errors in `dot_x_in_container` math or wrong y_min/y_max when
recomputing.

- [ ] **Step 4.6: Run the test suite**

```
cargo test -p investimentos-ui 2>&1 | tail -10
```

- [ ] **Step 4.7: Commit**

```bash
git add crates/ui/src/views/history.rs
git commit -m "feat(history): hover dot, guide, and tooltip in single-line chart"
```

---

### Task 5: Split-line chart — same indicator + per-category tooltip

Mirror Task 4 in `render_multi_chart`, with two changes: pick the active series via `active_series_idx`, and prepend a color swatch + category label to the tooltip text.

**Files:**
- Modify: `crates/ui/src/views/history.rs`

- [ ] **Step 5.1: Compute `HoverDecoration` for split mode**

Inside `render_multi_chart`, after `canvas_series` is built (around line 470), add:

```rust
let hover_for_render: Option<HoverDecoration> = {
    let hover = cx.entity().read(cx).history_hover;
    hover.and_then(|h| {
        let bounds = (*bounds_cell.lock().unwrap())?;
        let pad = px(10.0);
        let chart_w = bounds.size.width - pad * 2.0;
        let chart_h = bounds.size.height - pad * 2.0;
        let snapped = snap_index(h.rel_x, pad, chart_w, n_total);
        let active = active_series_idx(h.rel_y, snapped, &canvas_series, pad, chart_h, y_max_f)?;
        let (color, points_for_series) = &canvas_series[active];
        let (_, value) = *points_for_series.iter().find(|(i, _)| *i == snapped)?;
        // Category label and date come from the original `series` slice and `total_points`.
        let label = series[active].0;
        let date = total_points[snapped].0;
        let to_px_x = |i: usize| -> Pixels {
            pad + chart_w * (i as f32 / (n_total - 1).max(1) as f32)
        };
        let to_px_y = |v: f64| -> Pixels {
            pad + chart_h * (1.0 - (v / y_max_f) as f32)
        };
        Some(HoverDecoration {
            snapped_idx: snapped,
            series_color: *color,
            dot_x_in_container: to_px_x(snapped),
            dot_y_in_container: to_px_y(value),
            tooltip_text: format!(
                "{} — {} — {}",
                label,
                date.format("%b %d, %Y"),
                format_brl(value),
            ),
        })
    })
};
```

`bounds_cell` here is the one built in Task 3 for the multi-chart container.

- [ ] **Step 5.2: Wire `paint_hover_indicator` into the multi-chart paint**

Update the canvas paint closure in `render_multi_chart`:

```rust
let hover_for_canvas: Arc<Option<HoverDecoration>> = Arc::new(hover_for_render.clone());
// inside canvas(...):
move |bounds, series, window, _cx| {
    paint_multi_chart(bounds, &series, n_total, y_max_f, window);
    if let Some(deco) = hover_for_canvas.as_ref().as_ref() {
        paint_hover_indicator(bounds, deco, window);
    }
}
```

- [ ] **Step 5.3: Render the split-mode tooltip overlay**

Same shape as Task 4.4, with the same edge-flip math. The tooltip text already includes the category prefix, so no additional swatch is required for now (color is already conveyed by the dot itself). If you decide a swatch in the tooltip body adds clarity, prepend `div().w(px(8.0)).h(px(8.0)).bg(deco.series_color).rounded_full()` as the first child of the tooltip; otherwise keep it text-only.

- [ ] **Step 5.4: Build and run, verify split-mode hover works**

```
cargo build -p investimentos-ui --release
target/release/investimentos-ui
```

In the History tab with **split on**:
- Hover near the upper line — dot + guide attach to that line, tooltip shows that category's value.
- Move the cursor down to a lower line — dot snaps to the closer line, tooltip updates.
- Tooltip text reads `<Category> — <date> — <BRL value>`.
- Edge-flip behavior matches single-line mode.
- Cursor leaving the chart hides everything.

- [ ] **Step 5.5: Run the test suite**

```
cargo test -p investimentos-ui 2>&1 | tail -10
```

- [ ] **Step 5.6: Commit**

```bash
git add crates/ui/src/views/history.rs
git commit -m "feat(history): hover dot, guide, and tooltip in split chart"
```

---

### Task 6: Final manual verification + cleanup

A short sanity sweep across the time-range buttons and edge cases, then a final commit if anything was tweaked.

**Files:** none (verification + optional polish)

- [ ] **Step 6.1: Manual sweep**

Run the release binary. For each time range (30d, 90d, 1y, All) and each split state (off, on):

- Move the cursor across the full width — no flicker, snap is smooth.
- Move the cursor outside the chart — indicator + tooltip disappear cleanly.
- Resize the window — hover continues to work after the next mouse move (one-frame staleness is acceptable).
- Switch tabs and come back — chart re-renders correctly, hover state isn't carrying garbage.

- [ ] **Step 6.2: Run the full Rust test suite**

```
cargo test --workspace 2>&1 | grep -E "test result|FAILED" | tail -20
```

The 3 pre-existing failures (`test_get_distinct_symbols`, `test_fraction_auction_adds_to_position`, `test_fetch_btc_brl_current`) are unrelated baseline failures; everything else should pass.

- [ ] **Step 6.3: If anything was tweaked during the sweep, commit it**

```bash
git add -p crates/ui/src/views/history.rs
git commit -m "fix(history): <whatever was tweaked>"
```

If nothing was tweaked, skip.

- [ ] **Step 6.4: Push**

```bash
git push
```
