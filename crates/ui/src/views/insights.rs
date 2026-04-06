use std::collections::BTreeMap;

use gpui::{div, prelude::*, px, AnyElement, Div, FontWeight};

use investimentos_core::db::{queries, Database};
use investimentos_core::types::*;

use crate::theme;
use crate::views::format_brl;

pub fn render_insights(db: &Database) -> AnyElement {
    div()
        .id("insights-scroll")
        .flex()
        .flex_col()
        .flex_1()
        .overflow_y_scroll()
        .p_6()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Insights"),
        )
        .child(render_category_insights_n(db, "Stocks (Top 5)", theme::ACCENT, |at| at == "stock_intl" || at == "stock_br", Some(5)))
        .child(render_category_insights_n(db, "Tesouro Direto", theme::YELLOW, |at| at == "tesouro", None))
        .child(render_category_insights_n(db, "Crypto", theme::RED, |at| at == "crypto", None))
        .child(render_portfolio_summary(db))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Per-category performance (generic)
// ---------------------------------------------------------------------------

struct SymbolPnl {
    symbol: String,
    total_bought: f64,
    total_sold: f64,
    current_qty: f64,
    current_value: f64,
    realized_pnl: f64,
    unrealized_pnl: f64,
    total_pnl: f64,
    status: &'static str,
}

fn compute_category_pnl(db: &Database, filter: impl Fn(&str) -> bool) -> Vec<SymbolPnl> {
    let txs = queries::get_all_transactions(db).unwrap_or_default();

    let mut by_symbol: BTreeMap<String, Vec<&Transaction>> = BTreeMap::new();
    for tx in &txs {
        if !filter(tx.asset_type.as_str()) {
            continue;
        }
        by_symbol.entry(tx.symbol.clone()).or_default().push(tx);
    }

    let mut results = Vec::new();

    for (symbol, symbol_txs) in &by_symbol {
        let mut total_bought = 0.0_f64;
        let mut total_sold = 0.0_f64;
        let mut qty = 0.0_f64;
        let mut cost_basis = 0.0_f64;

        for tx in symbol_txs {
            match tx.tx_type {
                TxType::Buy => {
                    if tx.total_brl > 0.001 {
                        total_bought += tx.total_brl;
                        cost_basis += tx.total_brl;
                    }
                    qty += tx.quantity;
                }
                TxType::Sell => {
                    if qty > 0.0001 && tx.total_brl > 0.001 {
                        let fraction = tx.quantity / qty;
                        let cost_of_sold = cost_basis * fraction;
                        cost_basis -= cost_of_sold;
                        total_sold += tx.total_brl;
                    }
                    qty -= tx.quantity;
                }
                _ => {}
            }
        }

        let current_price = queries::get_latest_price(db, symbol)
            .ok()
            .flatten()
            .map(|p| p.close_price * p.brl_rate)
            .unwrap_or(0.0);

        let current_value = if qty > 0.0001 { qty * current_price } else { 0.0 };
        let unrealized_pnl = current_value - cost_basis;
        let realized_pnl = total_sold - (total_bought - cost_basis);
        let total_pnl = realized_pnl + unrealized_pnl;

        let status = if qty > 0.0001 { "Holding" } else { "Sold" };

        results.push(SymbolPnl {
            symbol: symbol.clone(),
            total_bought,
            total_sold,
            current_qty: qty.max(0.0),
            current_value,
            realized_pnl,
            unrealized_pnl,
            total_pnl,
            status,
        });
    }

    results.sort_by(|a, b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));
    results
}

fn render_category_insights_n(
    db: &Database,
    title: &str,
    title_color: gpui::Rgba,
    filter: impl Fn(&str) -> bool,
    max_rows: Option<usize>,
) -> Div {
    let items = compute_category_pnl(db, filter);

    if items.is_empty() {
        return div();
    }

    let total_bought: f64 = items.iter().map(|i| i.total_bought).sum();
    let total_sold: f64 = items.iter().map(|i| i.total_sold).sum();
    let total_current: f64 = items.iter().map(|i| i.current_value).sum();
    let total_realized: f64 = items.iter().map(|i| i.realized_pnl).sum();
    let total_unrealized: f64 = items.iter().map(|i| i.unrealized_pnl).sum();
    let total_pnl: f64 = items.iter().map(|i| i.total_pnl).sum();
    let total_pnl_pct = if total_bought > 0.0 { total_pnl / total_bought * 100.0 } else { 0.0 };

    let pnl_color = if total_pnl >= 0.0 { theme::GREEN } else { theme::RED };

    let mut panel = div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .rounded_lg()
        .bg(theme::BG_SECONDARY)
        .border_1()
        .border_color(theme::BORDER);

    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .text_color(title_color)
                    .child(format!("{} Performance", title)),
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .text_color(pnl_color)
                    .child(format!("R$ {:+} ({:+.1}%)", format_brl(total_pnl), total_pnl_pct)),
            ),
    );

    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .gap_6()
            .text_xs()
            .child(stat("Total Invested", &format!("R$ {}", format_brl(total_bought))))
            .child(stat("Total Sold", &format!("R$ {}", format_brl(total_sold))))
            .child(stat("Current Value", &format!("R$ {}", format_brl(total_current))))
            .child(stat_colored("Realized P/L", &format!("R$ {:+}", format_brl(total_realized)), total_realized))
            .child(stat_colored("Unrealized P/L", &format!("R$ {:+}", format_brl(total_unrealized)), total_unrealized)),
    );

    // Per-instrument table
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .py_1()
            .mt_2()
            .border_b_1()
            .border_color(theme::BORDER)
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme::TEXT_SECONDARY)
            .child(div().w(px(130.0)).child("Symbol"))
            .child(div().w(px(55.0)).child("Status"))
            .child(div().w(px(90.0)).child("Invested"))
            .child(div().w(px(90.0)).child("Current"))
            .child(div().w(px(90.0)).child("Realized"))
            .child(div().w(px(90.0)).child("Unrealized"))
            .child(div().w(px(100.0)).child("Total P/L")),
    );

    let display_items: &[SymbolPnl] = match max_rows {
        Some(n) => &items[..n.min(items.len())],
        None => &items,
    };

    let hidden_count = items.len() - display_items.len();

    for item in display_items {
        let pnl_col = if item.total_pnl >= 0.0 { theme::GREEN } else { theme::RED };
        let status_col = if item.status == "Holding" { theme::GREEN } else { theme::TEXT_SECONDARY };

        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .py_1()
                .border_b_1()
                .border_color(theme::BORDER)
                .text_xs()
                .child(div().w(px(130.0)).font_weight(FontWeight::MEDIUM).child(item.symbol.clone()))
                .child(div().w(px(55.0)).text_color(status_col).child(item.status))
                .child(div().w(px(90.0)).child(format!("R$ {}", format_brl(item.total_bought))))
                .child(div().w(px(90.0)).child(
                    if item.current_qty > 0.0001 {
                        format!("R$ {}", format_brl(item.current_value))
                    } else {
                        "—".to_string()
                    },
                ))
                .child(
                    div().w(px(90.0))
                        .text_color(if item.realized_pnl >= 0.0 { theme::GREEN } else { theme::RED })
                        .child(format!("{:+}", format_brl(item.realized_pnl))),
                )
                .child(
                    div().w(px(90.0))
                        .text_color(if item.unrealized_pnl >= 0.0 { theme::GREEN } else { theme::RED })
                        .child(if item.current_qty > 0.0001 {
                            format!("{:+}", format_brl(item.unrealized_pnl))
                        } else {
                            "—".to_string()
                        }),
                )
                .child(
                    div().w(px(100.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(pnl_col)
                        .child(format!("R$ {:+}", format_brl(item.total_pnl))),
                ),
        );
    }

    if hidden_count > 0 {
        panel = panel.child(
            div()
                .py_1()
                .text_xs()
                .text_color(theme::TEXT_SECONDARY)
                .child(format!(
                    "Showing top {} of {} symbols. Summary above covers all {}.",
                    display_items.len(),
                    items.len(),
                    items.len()
                )),
        );
    }

    panel
}

// ---------------------------------------------------------------------------
// Portfolio-wide summary stats
// ---------------------------------------------------------------------------

fn render_portfolio_summary(db: &Database) -> Div {
    let txs = queries::get_all_transactions(db).unwrap_or_default();

    // Skip $0 corporate actions
    let mut total_invested = 0.0_f64;
    let mut total_withdrawn = 0.0_f64;

    for tx in &txs {
        match tx.tx_type {
            TxType::Buy if tx.total_brl > 0.001 => total_invested += tx.total_brl,
            TxType::Sell if tx.total_brl > 0.001 => total_withdrawn += tx.total_brl,
            _ => {}
        }
    }

    let positions = investimentos_core::portfolio::compute_positions(db).unwrap_or_default();
    let current_value: f64 = positions.iter().filter_map(|p| p.current_value_brl).sum();
    let cost_basis: f64 = positions.iter().map(|p| p.avg_cost_brl * p.quantity).sum();

    let realized_pnl = total_withdrawn - (total_invested - cost_basis);
    let unrealized_pnl = current_value - cost_basis;
    let total_pnl = realized_pnl + unrealized_pnl;
    let pnl_pct = if total_invested > 0.0 { total_pnl / total_invested * 100.0 } else { 0.0 };
    let pnl_color = if total_pnl >= 0.0 { theme::GREEN } else { theme::RED };

    div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .rounded_lg()
        .bg(theme::BG_SECONDARY)
        .border_1()
        .border_color(theme::BORDER)
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::BOLD)
                .text_color(theme::ACCENT)
                .child("Portfolio Summary (All Time)"),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap_6()
                .text_xs()
                .child(stat("Total Invested", &format!("R$ {}", format_brl(total_invested))))
                .child(stat("Total Withdrawn", &format!("R$ {}", format_brl(total_withdrawn))))
                .child(stat("Current Value", &format!("R$ {}", format_brl(current_value))))
                .child(stat_colored("Realized P/L", &format!("R$ {:+}", format_brl(realized_pnl)), realized_pnl))
                .child(stat_colored("Unrealized P/L", &format!("R$ {:+}", format_brl(unrealized_pnl)), unrealized_pnl))
                .child(stat_colored(
                    "Total P/L",
                    &format!("R$ {:+} ({:+.1}%)", format_brl(total_pnl), pnl_pct),
                    total_pnl,
                )),
        )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn stat(label: &str, value: &str) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_color(theme::TEXT_SECONDARY)
                .child(label.to_string()),
        )
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .child(value.to_string()),
        )
}

fn stat_colored(label: &str, value: &str, v: f64) -> Div {
    let color = if v >= 0.0 { theme::GREEN } else { theme::RED };
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_color(theme::TEXT_SECONDARY)
                .child(label.to_string()),
        )
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .text_color(color)
                .child(value.to_string()),
        )
}
