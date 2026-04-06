use std::sync::Arc;

use gpui::{
    div, prelude::*, rgb, uniform_list, AnyElement, ClickEvent, Context, Div, FontWeight, Rgba,
    SharedString,
};

use investimentos_core::db::Database;
use investimentos_core::portfolio;
use investimentos_core::types::*;

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
            Self::ValueOrig => "Val orig",
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
    db: &Database,
    sort: SortState,
    selected: Option<usize>,
    cx: &mut Context<AppRoot>,
) -> AnyElement {
    let mut positions = portfolio::compute_positions(db).unwrap_or_default();
    sort_positions(&mut positions, sort);

    let count = positions.len();
    let positions = Arc::new(positions);
    let pos_for_list = positions.clone();
    let entity = cx.entity().clone();

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
        .child(render_header_row(sort, cx))
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .child(
                    uniform_list(
                        "positions-list",
                        count,
                        move |range, _window, cx| {
                            range
                                .map(|ix| render_position_row(ix, &pos_for_list[ix], selected, &entity, cx))
                                .collect::<Vec<_>>()
                        },
                    )
                    .size_full(),
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
        .child(cell(SortColumn::AvgCost).child(format_brl(pos.avg_cost)))
        .child(cell(SortColumn::AvgCostBrl).child(format!("R$ {}", format_brl(pos.avg_cost_brl))))
        .child(cell(SortColumn::Price).child(format_brl(price)))
        .child(cell(SortColumn::ValueOrig).child(format_brl(price * pos.quantity)))
        .child(cell(SortColumn::PlOrig).text_color(pnl_color(pnl_orig)).child(format!("{:+}", format_brl(pnl_orig))))
        .child(cell(SortColumn::PlBrl).text_color(pnl_color(pnl_brl)).child(format!("{:+}", format_brl(pnl_brl))))
        .child(cell(SortColumn::PlPct).text_color(pnl_color(pnl_pct)).child(format!("{:+.1}%", pnl_pct)))
        .child(cell(SortColumn::ValueBrl).child(format!("R$ {}", format_brl(value_brl))))
        .child(cell(SortColumn::Weight).child(format!("{:.1}%", weight)))
}

fn cell(col: SortColumn) -> Div {
    div().w(gpui::px(col.width())).overflow_hidden().whitespace_nowrap()
}

fn format_qty(q: f64) -> String {
    if (q - q.round()).abs() < 0.0001 {
        format!("{}", q as i64)
    } else {
        format!("{:.2}", q)
    }
}
