use gpui::{div, prelude::*, AnyElement, Div, FontWeight, Rgba};

use investimentos_core::db::Database;
use investimentos_core::portfolio;
use investimentos_core::types::*;

use crate::theme;
use crate::views::format_brl;

/// Color for a given asset type.
fn asset_color(at: &AssetType) -> Rgba {
    match at {
        AssetType::StockIntl => theme::ACCENT,
        AssetType::StockBr => theme::GREEN,
        AssetType::Tesouro => theme::YELLOW,
        AssetType::Crypto => theme::RED,
        AssetType::Gold => theme::PURPLE,
    }
}

/// Human label for asset type.
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

pub fn render_overview(db: &Database) -> AnyElement {
    let positions = portfolio::compute_positions(db).unwrap_or_default();
    let allocations = portfolio::compute_allocations(&positions);
    let total_value_brl: f64 = positions.iter().filter_map(|p| p.current_value_brl).sum();

    div()
        .id("overview-scroll")
        .flex()
        .flex_col()
        .gap_6()
        .p_6()
        .w_full()
        .flex_1()
        .overflow_y_scroll()
        .child(render_total(total_value_brl))
        .child(render_allocation_panel(&allocations))
        .child(render_holdings_table(&positions))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Total portfolio value header
// ---------------------------------------------------------------------------

fn render_total(total: f64) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_sm()
                .text_color(theme::TEXT_SECONDARY)
                .child("Total Portfolio Value"),
        )
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child(format!("R$ {}", format_brl(total))),
        )
}

// ---------------------------------------------------------------------------
// Allocation breakdown
// ---------------------------------------------------------------------------

fn render_allocation_panel(allocations: &[Allocation]) -> Div {
    let mut panel = div().flex().flex_col().gap_4();

    panel = panel.child(
        div()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme::TEXT_SECONDARY)
            .child("Allocation"),
    );

    let mut row = div().flex().flex_row().gap_4().flex_wrap();
    for alloc in allocations {
        row = row.child(render_allocation_chip(alloc));
    }
    panel.child(row)
}

fn render_allocation_chip(alloc: &Allocation) -> Div {
    let color = asset_color(&alloc.asset_type);
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .py_1()
        .rounded_md()
        .bg(theme::BG_SECONDARY)
        .child(
            div()
                .w(gpui::px(10.0))
                .h(gpui::px(10.0))
                .rounded_md()
                .bg(color),
        )
        .child(
            div()
                .text_sm()
                .child(format!(
                    "{} — {:.1}%",
                    asset_label(&alloc.asset_type),
                    alloc.weight
                )),
        )
}

// ---------------------------------------------------------------------------
// Holdings table
// ---------------------------------------------------------------------------

fn render_holdings_table(positions: &[Position]) -> Div {
    let mut table = div().flex().flex_col().gap_1();

    // Header
    table = table.child(
        div()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme::TEXT_SECONDARY)
            .child("Top Holdings"),
    );

    table = table.child(render_header_row());

    for pos in positions {
        table = table.child(render_position_row(pos));
    }

    table
}

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
        .child(div().w(gpui::px(90.0)).child("Symbol"))
        .child(div().w(gpui::px(80.0)).child("Type"))
        .child(div().w(gpui::px(120.0)).child("Value (orig)"))
        .child(div().w(gpui::px(130.0)).child("Value (BRL)"))
        .child(div().w(gpui::px(80.0)).child("P/L %"))
        .child(div().w(gpui::px(70.0)).child("Weight"))
}

fn render_position_row(pos: &Position) -> Div {
    let pnl_pct = pos.pnl_pct.unwrap_or(0.0);
    let value_brl = pos.current_value_brl.unwrap_or(0.0);
    let value_orig = pos.current_price.map(|p| p * pos.quantity).unwrap_or(0.0);
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
                .w(gpui::px(90.0))
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
                .w(gpui::px(120.0))
                .child(format!("{} {}", pos.currency, format_brl(value_orig))),
        )
        .child(
            div()
                .w(gpui::px(130.0))
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
