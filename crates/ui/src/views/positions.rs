use gpui::{div, prelude::*, AnyElement, Div, FontWeight, Rgba};

use investimentos_core::db::Database;
use investimentos_core::portfolio;
use investimentos_core::types::*;

use crate::theme;
use crate::views::format_brl;

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

fn pnl_color(pct: f64) -> Rgba {
    if pct >= 0.0 {
        theme::GREEN
    } else {
        theme::RED
    }
}

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

pub fn render_positions(db: &Database) -> AnyElement {
    let positions = portfolio::compute_positions(db).unwrap_or_default();

    let mut content = div()
        .id("positions-scroll")
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .w_full()
        .flex_1()
        .overflow_y_scroll();

    content = content.child(
        div()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme::TEXT_SECONDARY)
            .child(format!("All Positions ({})", positions.len())),
    );

    content = content.child(render_header_row());

    for pos in &positions {
        content = content.child(render_position_row(pos));
    }

    content.into_any_element()
}

// ---------------------------------------------------------------------------
// Table header
// ---------------------------------------------------------------------------

fn render_header_row() -> Div {
    div()
        .flex()
        .flex_row()
        .py_1()
        .border_b_1()
        .border_color(theme::BORDER)
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::TEXT_SECONDARY)
        .child(div().w(gpui::px(80.0)).child("Symbol"))
        .child(div().w(gpui::px(80.0)).child("Type"))
        .child(div().w(gpui::px(80.0)).child("Qty"))
        .child(div().w(gpui::px(100.0)).child("Avg Cost"))
        .child(div().w(gpui::px(110.0)).child("Avg Cost BRL"))
        .child(div().w(gpui::px(100.0)).child("Price"))
        .child(div().w(gpui::px(120.0)).child("Value (BRL)"))
        .child(div().w(gpui::px(80.0)).child("P/L %"))
        .child(div().w(gpui::px(70.0)).child("Weight"))
}

// ---------------------------------------------------------------------------
// Table row
// ---------------------------------------------------------------------------

fn render_position_row(pos: &Position) -> Div {
    let pnl_pct = pos.pnl_pct.unwrap_or(0.0);
    let value_brl = pos.current_value_brl.unwrap_or(0.0);
    let price = pos.current_price.unwrap_or(0.0);
    let weight = pos.weight.unwrap_or(0.0);

    div()
        .flex()
        .flex_row()
        .py_1()
        .border_b_1()
        .border_color(theme::BORDER)
        .text_xs()
        .child(
            div()
                .w(gpui::px(80.0))
                .font_weight(FontWeight::MEDIUM)
                .child(pos.symbol.clone()),
        )
        .child(
            div()
                .w(gpui::px(80.0))
                .text_color(asset_color(&pos.asset_type))
                .child(asset_label(&pos.asset_type)),
        )
        .child(
            div()
                .w(gpui::px(80.0))
                .child(format_qty(pos.quantity)),
        )
        .child(
            div()
                .w(gpui::px(100.0))
                .child(format!("{} {}", pos.currency, format_brl(pos.avg_cost))),
        )
        .child(
            div()
                .w(gpui::px(110.0))
                .child(format!("R$ {}", format_brl(pos.avg_cost_brl))),
        )
        .child(
            div()
                .w(gpui::px(100.0))
                .child(format!("{} {}", pos.currency, format_brl(price))),
        )
        .child(
            div()
                .w(gpui::px(120.0))
                .child(format!("R$ {}", format_brl(value_brl))),
        )
        .child(
            div()
                .w(gpui::px(80.0))
                .text_color(pnl_color(pnl_pct))
                .child(format!("{:+.2}%", pnl_pct)),
        )
        .child(
            div()
                .w(gpui::px(70.0))
                .child(format!("{:.1}%", weight)),
        )
}

/// Format quantity: show decimals only if fractional.
fn format_qty(q: f64) -> String {
    if (q - q.round()).abs() < 0.0001 {
        format!("{}", q as i64)
    } else {
        format!("{:.4}", q)
    }
}
