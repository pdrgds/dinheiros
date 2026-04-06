use gpui::{div, prelude::*, AnyElement, Div, FontWeight, Rgba};

use investimentos_core::db::Database;
use investimentos_core::portfolio;
use investimentos_core::types::*;

use crate::theme;
use crate::views::format_brl;

const TOP_N: usize = 5;

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

pub fn render_overview(db: &Database) -> AnyElement {
    let positions = portfolio::compute_positions(db).unwrap_or_default();
    let allocations = portfolio::compute_allocations(&positions);
    let total_value_brl: f64 = positions.iter().filter_map(|p| p.current_value_brl).sum();

    // Top N by value
    let mut top = positions.clone();
    top.sort_by(|a, b| {
        b.current_value_brl
            .unwrap_or(0.0)
            .partial_cmp(&a.current_value_brl.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    top.truncate(TOP_N);

    // Compute "rest" weight
    let top_weight: f64 = top.iter().filter_map(|p| p.weight).sum();
    let rest_count = positions.len().saturating_sub(TOP_N);

    div()
        .id("overview-scroll")
        .flex()
        .flex_col()
        .flex_1()
        .overflow_y_scroll()
        .p_6()
        .gap_6()
        // Total value
        .child(render_total(total_value_brl))
        // Allocation chips
        .child(render_allocation_panel(&allocations))
        // Top holdings
        .child(render_top_holdings(&top, rest_count, top_weight))
        // Placeholder for charts
        .child(
            div()
                .flex()
                .flex_row()
                .gap_6()
                .child(render_chart_placeholder("Allocation Chart"))
                .child(render_chart_placeholder("Portfolio Value (90d)")),
        )
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Total portfolio value
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
    let mut panel = div().flex().flex_col().gap_2();

    panel = panel.child(
        div()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme::TEXT_SECONDARY)
            .child("Allocation"),
    );

    let mut row = div().flex().flex_row().gap_3().flex_wrap();
    for alloc in allocations {
        let color = asset_color(&alloc.asset_type);
        row = row.child(
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
                ),
        );
    }
    panel.child(row)
}

// ---------------------------------------------------------------------------
// Top holdings summary
// ---------------------------------------------------------------------------

fn render_top_holdings(top: &[Position], rest_count: usize, top_weight: f64) -> Div {
    let mut panel = div()
        .flex()
        .flex_col()
        .gap_1()
        .p_4()
        .rounded_lg()
        .bg(theme::BG_SECONDARY)
        .border_1()
        .border_color(theme::BORDER);

    panel = panel.child(
        div()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme::TEXT_SECONDARY)
            .pb_1()
            .child(format!("Top {} Holdings", TOP_N)),
    );

    // Header
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .py_1()
            .border_b_1()
            .border_color(theme::BORDER)
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme::TEXT_SECONDARY)
            .child(div().flex_1().child("Symbol"))
            .child(div().w(gpui::px(90.0)).child("Value (BRL)"))
            .child(div().w(gpui::px(70.0)).child("P/L %"))
            .child(div().w(gpui::px(55.0)).child("Wt %")),
    );

    for pos in top {
        let pnl_pct = pos.pnl_pct.unwrap_or(0.0);
        let value_brl = pos.current_value_brl.unwrap_or(0.0);
        let weight = pos.weight.unwrap_or(0.0);

        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .py_1()
                .border_b_1()
                .border_color(theme::BORDER)
                .text_xs()
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_row()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .child(pos.symbol.clone()),
                        )
                        .child(
                            div()
                                .text_color(asset_color(&pos.asset_type))
                                .child(asset_label(&pos.asset_type)),
                        ),
                )
                .child(
                    div()
                        .w(gpui::px(90.0))
                        .child(format!("R$ {}", format_brl(value_brl))),
                )
                .child(
                    div()
                        .w(gpui::px(70.0))
                        .text_color(pnl_color(pnl_pct))
                        .child(format!("{:+.1}%", pnl_pct)),
                )
                .child(
                    div()
                        .w(gpui::px(55.0))
                        .child(format!("{:.1}%", weight)),
                ),
        );
    }

    // "And X others" row
    if rest_count > 0 {
        let rest_weight = 100.0 - top_weight;
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .py_1()
                .text_xs()
                .text_color(theme::TEXT_SECONDARY)
                .child(
                    div()
                        .flex_1()
                        .child(format!("+ {} others", rest_count)),
                )
                .child(div().w(gpui::px(90.0)))
                .child(div().w(gpui::px(70.0)))
                .child(
                    div()
                        .w(gpui::px(55.0))
                        .child(format!("{:.1}%", rest_weight)),
                ),
        );
    }

    panel
}

// ---------------------------------------------------------------------------
// Chart placeholders
// ---------------------------------------------------------------------------

fn render_chart_placeholder(title: &str) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .h(gpui::px(180.0))
        .rounded_lg()
        .bg(theme::BG_SECONDARY)
        .border_1()
        .border_color(theme::BORDER)
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::TEXT_SECONDARY)
                .child(title.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme::TEXT_SECONDARY)
                .pt_2()
                .child("Coming soon"),
        )
}
