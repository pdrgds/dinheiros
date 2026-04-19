use std::sync::Arc;

use gpui::{
    div, prelude::*, px, rgb, uniform_list, AnyElement, ClickEvent, Context, Div, FontWeight,
    IntoElement, Rgba, SharedString, Window,
};
use gpui_component::table::{Column, ColumnSort, DataTable, TableDelegate, TableState};
use gpui_component::{Sizable, Size};

use dinheiros_core::portfolio;
use dinheiros_core::types::*;

use crate::app::AppRoot;
use crate::theme;
use crate::views::format_brl;

// ---------------------------------------------------------------------------
// Sort state (reused from overview's pattern)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Symbol,
    Type,
    Currency,
    Qty,
    AvgCost,
    AvgCostBrl,
    Price,
    ValueOrig,
    PlOrig,
    PlBrl,
    PlPct,
    ValueBrl,
    Weight,
}

impl SortColumn {
    pub const ALL: [SortColumn; 13] = [
        Self::Symbol, Self::Type, Self::Currency, Self::Qty,
        Self::AvgCost, Self::AvgCostBrl, Self::Price, Self::ValueOrig,
        Self::PlOrig, Self::PlBrl, Self::PlPct, Self::ValueBrl, Self::Weight,
    ];

    fn label(&self) -> &'static str {
        match self {
            Self::Symbol => "Symbol",
            Self::Type => "Type",
            Self::Currency => "Ccy",
            Self::Qty => "Qty",
            Self::AvgCost => "Avg Cost",
            Self::AvgCostBrl => "Cost BRL",
            Self::Price => "Price",
            Self::ValueOrig => "Value",
            Self::PlOrig => "P/L orig",
            Self::PlBrl => "P/L BRL",
            Self::PlPct => "P/L %",
            Self::ValueBrl => "Value BRL",
            Self::Weight => "Wt %",
        }
    }

    fn index(&self) -> usize {
        Self::ALL.iter().position(|c| c == self).unwrap()
    }

    fn width(&self) -> f32 {
        // ~1000px available (1050 window - 48px horizontal padding)
        match self {
            Self::Symbol => 105.0,
            Self::Type => 62.0,
            Self::Currency => 30.0,
            Self::Qty => 50.0,
            Self::AvgCost => 80.0,
            Self::AvgCostBrl => 90.0,
            Self::Price => 80.0,
            Self::ValueOrig => 80.0,
            Self::PlOrig => 78.0,
            Self::PlBrl => 78.0,
            Self::PlPct => 55.0,
            Self::ValueBrl => 95.0,
            Self::Weight => 40.0,
        }
    }

    fn sort_key(&self, pos: &Position) -> f64 {
        match self {
            Self::Symbol | Self::Type | Self::Currency => 0.0,
            Self::Qty => pos.quantity,
            Self::AvgCost => pos.avg_cost,
            Self::AvgCostBrl => pos.avg_cost_brl,
            Self::Price => pos.current_price.unwrap_or(0.0),
            Self::ValueOrig => pos.current_price.unwrap_or(0.0) * pos.quantity,
            Self::PlOrig => {
                let price = pos.current_price.unwrap_or(0.0);
                (price - pos.avg_cost) * pos.quantity
            }
            Self::PlBrl => pos.pnl_brl.unwrap_or(0.0),
            Self::PlPct => pos.pnl_pct.unwrap_or(0.0),
            Self::ValueBrl => pos.current_value_brl.unwrap_or(0.0),
            Self::Weight => pos.weight.unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SortState {
    pub column: SortColumn,
    pub ascending: bool,
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            column: SortColumn::ValueBrl,
            ascending: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

fn asset_color(at: &AssetType) -> Rgba {
    match at {
        AssetType::StockIntl => theme::ACCENT,
        AssetType::StockBr => theme::GREEN,
        AssetType::Tesouro => theme::YELLOW,
        AssetType::Crypto => theme::RED,
        AssetType::Gold => theme::PURPLE,
    }
}

fn asset_label(at: &AssetType) -> &'static str {
    match at {
        AssetType::StockIntl => "Intl Stocks",
        AssetType::StockBr => "BR Stocks",
        AssetType::Tesouro => "Tesouro",
        AssetType::Crypto => "Crypto",
        AssetType::Gold => "Gold",
    }
}

fn pnl_color(v: f64) -> Rgba {
    if v >= 0.0 { theme::GREEN } else { theme::RED }
}

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

pub fn render_positions(
    app: &mut AppRoot,
    window: &mut Window,
    cx: &mut Context<AppRoot>,
) -> AnyElement {
    // Sorting is owned by the delegate now (it reacts to header clicks via
    // `perform_sort`). We just hand it the raw positions list.
    let positions = portfolio::compute_positions(&app.db).unwrap_or_default();
    let count = positions.len();

    // Lazy-init the TableState entity the first time we render Positions, so
    // column resize state survives across renders. On subsequent renders we
    // just refresh the delegate's data (and recomputed widths).
    if app.positions_table.is_none() {
        let delegate = PositionsTableDelegate::new(positions.clone());
        app.positions_table = Some(cx.new(|cx| TableState::new(delegate, window, cx)));
    } else if let Some(table) = app.positions_table.as_ref() {
        let new_positions = positions.clone();
        table.update(cx, |state, _cx| {
            state.delegate_mut().update_data(new_positions);
        });
    }

    let table = app.positions_table.as_ref().expect("just initialised above");

    div()
        .flex()
        .flex_col()
        .flex_1()
        .child(
            div()
                .px_6()
                .pt_4()
                .pb_2()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::TEXT_SECONDARY)
                .child(format!("All Positions ({})", count)),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .text_xs()
                .px_6()
                .child(
                    DataTable::new(table)
                        .with_size(Size::XSmall)
                        .stripe(false)
                        .bordered(false),
                ),
        )
        .into_any_element()
}

fn sort_positions(positions: &mut [Position], sort: SortState) {
    match sort.column {
        SortColumn::Symbol => {
            positions.sort_by(|a, b| {
                let cmp = a.symbol.cmp(&b.symbol);
                if sort.ascending { cmp } else { cmp.reverse() }
            });
        }
        SortColumn::Type => {
            positions.sort_by(|a, b| {
                let cmp = asset_label(&a.asset_type).cmp(asset_label(&b.asset_type));
                if sort.ascending { cmp } else { cmp.reverse() }
            });
        }
        SortColumn::Currency => {
            positions.sort_by(|a, b| {
                let cmp = a.currency.cmp(&b.currency);
                if sort.ascending { cmp } else { cmp.reverse() }
            });
        }
        _ => {
            positions.sort_by(|a, b| {
                let ka = sort.column.sort_key(a);
                let kb = sort.column.sort_key(b);
                let cmp = ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal);
                if sort.ascending { cmp } else { cmp.reverse() }
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Table
// ---------------------------------------------------------------------------

fn render_header_row(sort: SortState, cx: &mut Context<AppRoot>) -> AnyElement {
    let mut row = row_base()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::TEXT_SECONDARY);

    for &col in &SortColumn::ALL {
        let is_active = sort.column == col;
        let arrow = if is_active {
            if sort.ascending { " \u{25B2}" } else { " \u{25BC}" }
        } else {
            ""
        };
        let label = format!("{}{}", col.label(), arrow);
        let text_col = if is_active { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY };

        row = row.child(
            div()
                .w(gpui::px(col.width()))
                .id(SharedString::from(format!("pos-sort-{:?}", col)))
                .text_color(text_col)
                .cursor_pointer()
                .hover(|s| s.text_color(rgb(0xffffff)))
                .on_click(cx.listener(move |this, _ev: &ClickEvent, _window, cx| {
                    let sort = &mut this.positions_sort;
                    if sort.column == col {
                        sort.ascending = !sort.ascending;
                    } else {
                        sort.column = col;
                        sort.ascending = false;
                    }
                    cx.notify();
                }))
                .child(label),
        );
    }

    row.into_any_element()
}


fn row_base() -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .px_6()
        .py_1()
        .border_b_1()
        .border_color(theme::BORDER)
        .text_xs()
        .whitespace_nowrap()
}

fn render_position_row(
    ix: usize,
    pos: &Position,
    selected: Option<usize>,
    entity: &gpui::Entity<AppRoot>,
    cx: &mut gpui::App,
) -> gpui::Stateful<Div> {
    let pnl_pct = pos.pnl_pct.unwrap_or(0.0);
    let pnl_brl = pos.pnl_brl.unwrap_or(0.0);
    let value_brl = pos.current_value_brl.unwrap_or(0.0);
    let price = pos.current_price.unwrap_or(0.0);
    let pnl_orig = (price - pos.avg_cost) * pos.quantity;
    let weight = pos.weight.unwrap_or(0.0);
    let is_selected = selected == Some(ix);
    let ccy = currency_symbol(&pos.currency);

    let bg = if is_selected { theme::BG_SECONDARY } else { theme::BG_PRIMARY };

    let entity = entity.clone();
    row_base()
        .bg(bg)
        .id(SharedString::from(format!("pos-row-{}", ix)))
        .cursor_pointer()
        .hover(|s| s.bg(theme::BG_SECONDARY))
        .on_click(move |_ev, _window, cx| {
            entity.update(cx, |this, cx| {
                this.selected_position = if this.selected_position == Some(ix) {
                    None
                } else {
                    Some(ix)
                };
                cx.notify();
            });
        })
        .child(cell(SortColumn::Symbol).font_weight(FontWeight::MEDIUM).truncate().child(pos.symbol.clone()))
        .child(cell(SortColumn::Type).text_color(asset_color(&pos.asset_type)).child(asset_label(&pos.asset_type)))
        .child(cell(SortColumn::Currency).text_color(theme::TEXT_SECONDARY).child(pos.currency.clone()))
        .child(cell(SortColumn::Qty).child(format_qty(pos.quantity)))
        .child(cell(SortColumn::AvgCost).child(format!("{} {}", ccy, format_brl(pos.avg_cost))))
        .child(cell(SortColumn::AvgCostBrl).child(format!("R$ {}", format_brl(pos.avg_cost_brl))))
        .child(cell(SortColumn::Price).child(format!("{} {}", ccy, format_brl(price))))
        .child(cell(SortColumn::ValueOrig).child(format!("{} {}", ccy, format_brl(price * pos.quantity))))
        .child(cell(SortColumn::PlOrig).text_color(pnl_color(pnl_orig)).child(format!("{} {:+}", ccy, format_brl(pnl_orig))))
        .child(cell(SortColumn::PlBrl).text_color(pnl_color(pnl_brl)).child(format!("R$ {:+}", format_brl(pnl_brl))))
        .child(cell(SortColumn::PlPct).text_color(pnl_color(pnl_pct)).child(format!("{:+.1}%", pnl_pct)))
        .child(cell(SortColumn::ValueBrl).child(format!("R$ {}", format_brl(value_brl))))
        .child(cell(SortColumn::Weight).child(format!("{:.1}%", weight)))
}

fn cell(col: SortColumn) -> Div {
    div().w(gpui::px(col.width())).overflow_hidden().whitespace_nowrap()
}

fn currency_symbol(code: &str) -> &'static str {
    match code {
        "BRL" => "R$",
        "USD" => "$",
        "EUR" => "\u{20AC}",
        "GBP" => "\u{00A3}",
        "CAD" => "C$",
        "DKK" => "kr",
        _ => "$",
    }
}

fn format_qty(q: f64) -> String {
    if (q - q.round()).abs() < 0.0001 {
        format!("{}", q as i64)
    } else {
        format!("{:.2}", q)
    }
}

// ---------------------------------------------------------------------------
// gpui-component Table delegate
// ---------------------------------------------------------------------------

/// Table delegate for the Positions tab. Holds the pre-sorted positions and
/// maps the existing `SortColumn` enum to `gpui_component::table::Column`
/// definitions — each one resizable, widths matching the old hand-rolled
/// layout. For the prototype, sort is driven from outside (via
/// `AppRoot.positions_sort`); wiring up header-click sorting through the
/// delegate's `perform_sort` hook is a follow-up.
pub struct PositionsTableDelegate {
    pub positions: Vec<Position>,
    /// Per-column width in pixels, computed from the widest cell (including the
    /// header label) each time the data changes. Recomputed when `update_data`
    /// is called from `render_positions`.
    pub column_widths: Vec<f32>,
    /// Which column is the active sort and in which direction. The Table reads
    /// this via `column()` to render the sort arrow and calls `perform_sort`
    /// when the user clicks a header.
    pub sort: (usize, ColumnSort),
}

impl PositionsTableDelegate {
    pub fn new(mut positions: Vec<Position>) -> Self {
        let sort = (
            SortColumn::ALL
                .iter()
                .position(|c| *c == SortColumn::ValueBrl)
                .expect("ValueBrl is in SortColumn::ALL"),
            ColumnSort::Descending,
        );
        apply_sort(&mut positions, sort);
        let column_widths = compute_column_widths(&positions);
        Self { positions, column_widths, sort }
    }

    pub fn update_data(&mut self, mut positions: Vec<Position>) {
        apply_sort(&mut positions, self.sort);
        self.column_widths = compute_column_widths(&positions);
        self.positions = positions;
    }
}

/// Sort `positions` in place according to a `(col_ix, direction)` pair from
/// `gpui_component`. `ColumnSort::Default` means no sort is active (leave as-is).
fn apply_sort(positions: &mut [Position], (col_ix, direction): (usize, ColumnSort)) {
    let ascending = match direction {
        ColumnSort::Ascending => true,
        ColumnSort::Descending => false,
        ColumnSort::Default => return,
    };
    let sc = SortColumn::ALL[col_ix];
    sort_positions(positions, SortState { column: sc, ascending });
}

/// Display form of a symbol for the Positions table.
///
/// Only Tesouro symbols get special treatment: the `Type` column already
/// shows "Tesouro", and maturity years in the 2000s are unambiguous at
/// 2 digits (e.g. "Tesouro Selic 2031" → "Selic 31"). Non-Tesouro symbols
/// (tickers, BTC, etc.) are passed through unchanged — a stock whose ticker
/// happened to end in a 4-digit year should not be rewritten.
///
/// `pos.symbol` itself is untouched — it's still the DB key used by prices,
/// backfill, cost basis, etc.
fn display_symbol(pos: &Position) -> String {
    let Some(stripped) = pos.symbol.strip_prefix("Tesouro ") else {
        return pos.symbol.clone();
    };
    let bytes = stripped.as_bytes();
    if bytes.len() >= 5
        && bytes[bytes.len() - 5] == b' '
        && &bytes[bytes.len() - 4..bytes.len() - 2] == b"20"
        && bytes[bytes.len() - 2].is_ascii_digit()
        && bytes[bytes.len() - 1].is_ascii_digit()
    {
        let cut = bytes.len() - 4;
        format!("{}{}", &stripped[..cut], &stripped[cut + 2..])
    } else {
        stripped.to_string()
    }
}

/// Plain-text render of a cell, used only for width measurement.
/// Must stay in sync with the formatting in `render_td`.
fn cell_text(sc: SortColumn, pos: &Position) -> String {
    let ccy = currency_symbol(&pos.currency);
    let price = pos.current_price.unwrap_or(0.0);
    let pnl_orig = (price - pos.avg_cost) * pos.quantity;
    let pnl_brl = pos.pnl_brl.unwrap_or(0.0);
    let pnl_pct = pos.pnl_pct.unwrap_or(0.0);
    let value_brl = pos.current_value_brl.unwrap_or(0.0);
    let weight = pos.weight.unwrap_or(0.0);
    match sc {
        SortColumn::Symbol => display_symbol(pos),
        SortColumn::Type => asset_label(&pos.asset_type).to_string(),
        SortColumn::Currency => pos.currency.clone(),
        SortColumn::Qty => format_qty(pos.quantity),
        SortColumn::AvgCost => format!("{} {}", ccy, format_brl(pos.avg_cost)),
        SortColumn::AvgCostBrl => format!("R$ {}", format_brl(pos.avg_cost_brl)),
        SortColumn::Price => format!("{} {}", ccy, format_brl(price)),
        SortColumn::ValueOrig => format!("{} {}", ccy, format_brl(price * pos.quantity)),
        SortColumn::PlOrig => format!("{} {:+}", ccy, format_brl(pnl_orig)),
        SortColumn::PlBrl => format!("R$ {:+}", format_brl(pnl_brl)),
        SortColumn::PlPct => format!("{:+.1}%", pnl_pct),
        SortColumn::ValueBrl => format!("R$ {}", format_brl(value_brl)),
        SortColumn::Weight => format!("{:.1}%", weight),
    }
}

/// Approximate text-xs glyph width for sizing. The gpui-component table adds
/// 4px left + 4px right internal padding at `Size::XSmall`, and when a column
/// is sortable it also renders a ~12px sort-indicator icon next to the header
/// label. `CELL_PADDING_PX` reserves enough room for both without being too
/// generous on numeric columns.
const APPROX_CHAR_PX: f32 = 6.2;
const CELL_PADDING_PX: f32 = 18.0;
const MIN_COLUMN_PX: f32 = 40.0;

fn compute_column_widths(positions: &[Position]) -> Vec<f32> {
    SortColumn::ALL
        .iter()
        .map(|sc| {
            let header_len = sc.label().chars().count();
            let max_body_len = positions
                .iter()
                .map(|p| cell_text(*sc, p).chars().count())
                .max()
                .unwrap_or(0);
            let max_chars = header_len.max(max_body_len) as f32;
            (max_chars * APPROX_CHAR_PX + CELL_PADDING_PX).max(MIN_COLUMN_PX)
        })
        .collect()
}

impl TableDelegate for PositionsTableDelegate {
    fn columns_count(&self, _: &gpui::App) -> usize {
        SortColumn::ALL.len()
    }

    fn rows_count(&self, _: &gpui::App) -> usize {
        self.positions.len()
    }

    fn column(&self, col_ix: usize, _: &gpui::App) -> Column {
        let sc = SortColumn::ALL[col_ix];
        let width = self.column_widths.get(col_ix).copied().unwrap_or(sc.width());
        let mut col = Column::new(SharedString::from(format!("{:?}", sc)), sc.label())
            .width(px(width));
        // Mark the active sort column with its direction; leave others as plain
        // sortable so clicking still works.
        col = if self.sort.0 == col_ix {
            col.sort(self.sort.1)
        } else {
            col.sortable()
        };
        match sc {
            SortColumn::Qty
            | SortColumn::AvgCost
            | SortColumn::AvgCostBrl
            | SortColumn::Price
            | SortColumn::ValueOrig
            | SortColumn::PlOrig
            | SortColumn::PlBrl
            | SortColumn::PlPct
            | SortColumn::ValueBrl
            | SortColumn::Weight => col.text_right(),
            _ => col,
        }
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        // gpui-component cycles Default → Descending → Ascending → Default on
        // repeat clicks. We never want the "unsorted" state: collapse it into
        // Descending so clicks on the active column just flip direction.
        let direction = match sort {
            ColumnSort::Default => ColumnSort::Descending,
            s => s,
        };
        self.sort = (col_ix, direction);
        apply_sort(&mut self.positions, self.sort);

        // The Table already mutated its internal `col_groups[col_ix].sort` to
        // whatever value it passed to us — including `Default`, which would
        // flash the "unsorted" icon. Schedule a refresh so `column()` re-runs
        // with our overridden direction and the sort icon reflects reality.
        let entity = cx.entity();
        cx.defer(move |cx| {
            entity.update(cx, |state, cx| state.refresh(cx));
        });
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        // Match the pre-table look: muted headers, highlighted when this column
        // is the active sort. The sort arrow itself is drawn by the Table.
        let color = if self.sort.0 == col_ix {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_SECONDARY
        };
        // NOTE: don't use `.size_full()` here — the Table wraps this output in
        // an `h_flex` alongside the sort-indicator icon, and taking all the
        // width would push the arrow out of the cell.
        div()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(color)
            .child(SortColumn::ALL[col_ix].label())
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let pos = &self.positions[row_ix];
        let sc = SortColumn::ALL[col_ix];
        let ccy = currency_symbol(&pos.currency);
        let price = pos.current_price.unwrap_or(0.0);
        let pnl_orig = (price - pos.avg_cost) * pos.quantity;
        let pnl_brl = pos.pnl_brl.unwrap_or(0.0);
        let pnl_pct = pos.pnl_pct.unwrap_or(0.0);
        let value_brl = pos.current_value_brl.unwrap_or(0.0);
        let weight = pos.weight.unwrap_or(0.0);

        let cell = |el: Div| el.text_xs();
        match sc {
            SortColumn::Symbol => cell(div())
                .font_weight(FontWeight::MEDIUM)
                .child(display_symbol(pos))
                .into_any_element(),
            SortColumn::Type => cell(div())
                .text_color(asset_color(&pos.asset_type))
                .child(asset_label(&pos.asset_type))
                .into_any_element(),
            SortColumn::Currency => cell(div())
                .text_color(theme::TEXT_SECONDARY)
                .child(pos.currency.clone())
                .into_any_element(),
            SortColumn::Qty => cell(div()).child(format_qty(pos.quantity)).into_any_element(),
            SortColumn::AvgCost => cell(div())
                .child(format!("{} {}", ccy, format_brl(pos.avg_cost)))
                .into_any_element(),
            SortColumn::AvgCostBrl => cell(div())
                .child(format!("R$ {}", format_brl(pos.avg_cost_brl)))
                .into_any_element(),
            SortColumn::Price => cell(div())
                .child(format!("{} {}", ccy, format_brl(price)))
                .into_any_element(),
            SortColumn::ValueOrig => cell(div())
                .child(format!("{} {}", ccy, format_brl(price * pos.quantity)))
                .into_any_element(),
            SortColumn::PlOrig => cell(div())
                .text_color(pnl_color(pnl_orig))
                .child(format!("{} {:+}", ccy, format_brl(pnl_orig)))
                .into_any_element(),
            SortColumn::PlBrl => cell(div())
                .text_color(pnl_color(pnl_brl))
                .child(format!("R$ {:+}", format_brl(pnl_brl)))
                .into_any_element(),
            SortColumn::PlPct => cell(div())
                .text_color(pnl_color(pnl_pct))
                .child(format!("{:+.1}%", pnl_pct))
                .into_any_element(),
            SortColumn::ValueBrl => cell(div())
                .child(format!("R$ {}", format_brl(value_brl)))
                .into_any_element(),
            SortColumn::Weight => cell(div())
                .child(format!("{:.1}%", weight))
                .into_any_element(),
        }
    }
}
