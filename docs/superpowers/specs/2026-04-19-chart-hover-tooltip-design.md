# History Chart — Hover Tooltip Design

## Goal

When the user moves the mouse over the history chart, show a small floating
tooltip with the value for the date under the cursor, and a visual indicator
(filled dot + thin vertical guide line) anchored to the same point.

In **single-line mode**, the tooltip shows the total portfolio value for that
day. In **split mode** (the chart is showing Stocks / Tesouro / Crypto as
separate lines), the tooltip shows the value of whichever line is closest to
the cursor's vertical position at the snapped x — the dot and guide line
anchor to that same line.

When the cursor leaves the chart area, the indicator and tooltip disappear.

## Non-goals

- Click-to-pin tooltips, drag-to-select ranges, multi-point selection.
- Touch / trackpad gesture support beyond what `on_mouse_move` already
  delivers.
- Showing all category values simultaneously in split mode (we only highlight
  the active series).
- Animations / transitions.

## Architecture

### State

A new optional field on `AppRoot`:

```rust
pub struct HistoryHover {
    /// Cursor position relative to the chart container's top-left corner.
    pub rel_x: gpui::Pixels,
    pub rel_y: gpui::Pixels,
}

pub history_hover: Option<HistoryHover>,
```

Initialized to `None`. Set by an `on_mouse_move` handler attached to the
chart container `div`. Cleared by `on_mouse_leave` on the same element.
Each handler calls `cx.notify()` so the UI re-renders.

### Mouse event flow

```
user moves cursor over chart container
  → on_mouse_move(MouseMoveEvent) handler reads event.position
  → translates to coords relative to chart container top-left
  → writes AppRoot.history_hover = Some(HistoryHover { rel_x, rel_y })
  → cx.notify()
  → render_history runs again, sees Some(hover), passes it down

user moves cursor off chart container
  → on_mouse_leave handler sets AppRoot.history_hover = None
  → cx.notify()
  → render_history runs, sees None, paints chart without hover indicator
```

### Render pipeline

`render_history` already builds:
- `total_points: Vec<(NaiveDate, f64)>`
- For split mode: `stocks_points`, `tesouro_points`, `crypto_points`
- Calls `render_chart` (single line) or `render_multi_chart` (split)

After today's change:
- `render_history` reads the current `AppRoot.history_hover`.
- It passes the hover down to `render_chart` / `render_multi_chart`.
- Each renderer:
  1. Computes a `HoverDecoration` describing what the canvas should paint
     and what the tooltip should show, using the pure helpers below.
  2. Captures it into the `canvas` closure so the paint callback can draw
     the dot + vertical line.
  3. If the hover is present and inside the chart, also renders an
     absolutely-positioned tooltip `div` over the chart container.

### Pure helpers (testable)

Both live in `crates/ui/src/views/history.rs` and have no GPUI dependency
beyond `Pixels` arithmetic, so they can be unit-tested without a window.

```rust
/// Map a cursor x (relative to the chart container) to the nearest data
/// index. Clamps to [0, n-1]. `chart_pad` and `chart_w` describe the
/// canvas's inner plot area (which is offset from the container by the
/// canvas padding).
fn snap_index(rel_x: Pixels, chart_pad: Pixels, chart_w: Pixels, n: usize) -> usize;

/// In split mode, given the snapped index and the cursor y, return the
/// index of the series whose painted y at that index is closest to
/// `rel_y`. Ties are broken by series order (first wins).
fn active_series_idx(
    rel_y: Pixels,
    snapped_idx: usize,
    series: &[(Rgba, Vec<(usize, f64)>)],
    chart_pad: Pixels,
    chart_h: Pixels,
    y_max: f64,
) -> Option<usize>;
```

`active_series_idx` returns `Option` because a series may not have a value
at the snapped index (each series is a sparse list aligned to the
reference timeline) — if no series has a point at that index, no
indicator is drawn.

### Paint additions

Inside `paint_chart` / `paint_multi_chart`, after the existing line/fill
code, if a `HoverDecoration` is present:

1. **Vertical guide line.** A 1px stroke in `theme::BORDER` from
   `(dot_x, chart_y)` to `(dot_x, chart_y + chart_h)`.
2. **Dot.** A filled circle of radius 4px in the active series color
   (in single-line mode that's the existing line color `0x06b6d4`),
   surrounded by a 1.5px stroke in `theme::BG_SECONDARY` so it stays
   readable when overlapping the line.

Implement the dot via `window.paint_quad` with a square of side `2 *
radius` and `border_radius = radius`, which renders as a filled circle.
This avoids constructing a multi-segment arc path.

### Tooltip element

Rendered outside the canvas, in the same container, as an
absolutely-positioned `div`:

```rust
div()
    .absolute()
    .left(dot_x_in_container + tooltip_offset_x)
    .top(dot_y_in_container - tooltip_height / 2)
    .px(px(8.0)).py(px(4.0))
    .rounded_md()
    .bg(theme::BG_SECONDARY)
    .border_1().border_color(theme::BORDER)
    .text_xs()
    .child(/* tooltip content, see below */)
```

**Content:**
- Single-line mode: `{date} — {brl_value}`
  e.g. `Apr 17, 2026 — R$ 89.4k`
- Split mode: small color swatch + `{category} — {date} — {brl_value}`

Date format: `%b %d, %Y`. BRL format: reuse `views::format_brl`.

**Tooltip dimensions.** Treat the tooltip as a fixed-size box (width
200px, height 28px) for positioning math. We don't measure the text
precisely; 200px is wide enough for the longest expected content
(`Crypto — Apr 17, 2026 — R$ 1,234.5k`).

**Tooltip offset from the dot.** 12px horizontal gap.

**Edge flip:** If `dot_x_in_container + 12 + 200 > container_width`,
position the tooltip to the *left* of the dot instead:
`left(dot_x_in_container - 12 - 200)`.

### Edge cases

| Case | Behavior |
|------|----------|
| `n < 2` data points | No hover indicator (existing renderers already early-return). |
| Cursor outside chart container | `on_mouse_leave` clears `history_hover`. |
| Cursor inside container but inside the y-axis label gutter (left 70px) | The chart container starts after the gutter; mouse handlers are only attached to the canvas-bearing div, so this case doesn't arise. |
| Sparse series in split mode (some categories don't have a value at the snapped index) | `active_series_idx` returns `None` → no indicator/tooltip painted that frame. |
| Time range changes while hovering | The new range produces new data; the next `render_history` recomputes everything from current state. No special handling. |
| Window resize | The canvas re-paints with new bounds; hover state is in container-relative coords, but on resize the container's mouse handlers will fire again on the next move. Stale state for the brief gap between resize and next move is acceptable. |

## Components

| Component | Responsibility |
|-----------|----------------|
| `AppRoot.history_hover` | Owns hover state. |
| Mouse handlers on the chart container | Translate raw mouse events to relative coords, update state, request repaint. |
| `snap_index`, `active_series_idx` | Pure math; testable. |
| `paint_chart` / `paint_multi_chart` | Paint indicator (dot + guide) at the snapped point. |
| Tooltip `div` | Render the textual readout, positioned over the chart. |

## Testing

**Unit tests** in `crates/ui/src/views/history.rs` (or a dedicated `tests`
module if practical given GPUI's test harness):

- `snap_index`:
  - cursor at exactly the left edge → index 0
  - cursor at exactly the right edge → index n-1
  - cursor before the left edge (negative rel_x) → index 0
  - cursor past the right edge → index n-1
  - cursor exactly at the midpoint with odd n → middle index
  - n == 1 → 0

- `active_series_idx`:
  - two series, cursor closer to the upper one → upper index
  - two series, cursor exactly equidistant → first wins (defined tiebreaker)
  - one series → its index
  - empty series at the snapped index → `None`

**Manual verification.** Visual / interaction behavior — tooltip text
content, edge-flip near right edge, behavior across the time-range
buttons (30d / 90d / 1y / All) and split toggle — checked by running the
app and exercising the chart.

## Files touched

- `crates/ui/src/app.rs` — add `history_hover: Option<HistoryHover>` and
  the `HistoryHover` struct.
- `crates/ui/src/views/history.rs` — pure helpers, hover-aware
  `render_chart` / `render_multi_chart`, mouse handlers wired onto the
  chart container, tooltip element.

No `core` changes. No DB changes.
