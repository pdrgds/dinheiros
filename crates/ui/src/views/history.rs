use std::collections::BTreeMap;

use chrono::NaiveDate;
use gpui::{div, prelude::*, AnyElement, Div, FontWeight};
use rusqlite::params;

use investimentos_core::db::Database;

use crate::theme;
use crate::views::format_brl;

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

pub fn render_history(db: &Database) -> AnyElement {
    let data_points = compute_history_data(db);

    let mut content = div()
        .id("history-scroll")
        .flex()
        .flex_col()
        .gap_6()
        .p_6()
        .w_full()
        .flex_1()
        .overflow_y_scroll();

    content = content.child(
        div()
            .text_2xl()
            .font_weight(FontWeight::BOLD)
            .child("Portfolio Value Over Time"),
    );

    if data_points.is_empty() {
        content = content.child(
            div()
                .text_sm()
                .text_color(theme::TEXT_SECONDARY)
                .child("No price history data available. Run reconciliation to fetch prices."),
        );
    } else {
        content = content.child(render_summary(&data_points));
        content = content.child(render_recent_values(&data_points));
    }

    content = content.child(
        div()
            .text_sm()
            .text_color(theme::TEXT_SECONDARY)
            .mt_4()
            .child("Chart rendering coming soon."),
    );

    content.into_any_element()
}

// ---------------------------------------------------------------------------
// Summary stats
// ---------------------------------------------------------------------------

fn render_summary(data: &BTreeMap<NaiveDate, f64>) -> Div {
    let count = data.len();
    let first_date = data.keys().next().unwrap();
    let last_date = data.keys().next_back().unwrap();
    let min_val = data.values().cloned().fold(f64::INFINITY, f64::min);
    let max_val = data.values().cloned().fold(f64::NEG_INFINITY, f64::max);
    let latest_val = *data.values().next_back().unwrap();

    let mut panel = div()
        .flex()
        .flex_col()
        .gap_2()
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
            .child("Summary"),
    );

    panel = panel.child(stat_row("Data Points", &format!("{}", count)));
    panel = panel.child(stat_row(
        "Date Range",
        &format!("{} to {}", first_date, last_date),
    ));
    panel = panel.child(stat_row(
        "Min Value",
        &format!("R$ {}", format_brl(min_val)),
    ));
    panel = panel.child(stat_row(
        "Max Value",
        &format!("R$ {}", format_brl(max_val)),
    ));
    panel = panel.child(stat_row(
        "Latest Value",
        &format!("R$ {}", format_brl(latest_val)),
    ));

    panel
}

fn stat_row(label: &str, value: &str) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_4()
        .child(
            div()
                .w(gpui::px(120.0))
                .text_sm()
                .text_color(theme::TEXT_SECONDARY)
                .child(label.to_string()),
        )
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .child(value.to_string()),
        )
}

// ---------------------------------------------------------------------------
// Recent values table (last 20 data points)
// ---------------------------------------------------------------------------

fn render_recent_values(data: &BTreeMap<NaiveDate, f64>) -> Div {
    let mut panel = div().flex().flex_col().gap_1();

    panel = panel.child(
        div()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme::TEXT_SECONDARY)
            .child("Recent Values"),
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
            .child(div().w(gpui::px(120.0)).child("Date"))
            .child(div().w(gpui::px(160.0)).child("Portfolio Value (BRL)")),
    );

    // Last 20 entries, newest first
    let entries: Vec<(&NaiveDate, &f64)> = data.iter().rev().take(20).collect();
    for (date, value) in entries {
        let date_str = date.format("%Y-%m-%d").to_string();
        let val_str = format!("R$ {}", format_brl(*value));
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .py_1()
                .border_b_1()
                .border_color(theme::BORDER)
                .text_xs()
                .child(div().w(gpui::px(120.0)).child(date_str))
                .child(
                    div()
                        .w(gpui::px(160.0))
                        .font_weight(FontWeight::MEDIUM)
                        .child(val_str),
                ),
        );
    }

    panel
}

// ---------------------------------------------------------------------------
// Compute portfolio value for each date with price data
// ---------------------------------------------------------------------------

/// For each date in the daily_prices table, compute total portfolio value
/// by summing (quantity_held * close_price * brl_rate) across all symbols.
fn compute_history_data(db: &Database) -> BTreeMap<NaiveDate, f64> {
    let mut result = BTreeMap::new();

    // Get all transactions to compute holdings at each date
    let transactions = match investimentos_core::db::queries::get_all_transactions(db) {
        Ok(txs) => txs,
        Err(_) => return result,
    };

    if transactions.is_empty() {
        return result;
    }

    // Get all distinct dates from daily_prices
    let dates = match get_all_price_dates(db) {
        Ok(d) => d,
        Err(_) => return result,
    };

    // Get all daily prices grouped by (symbol, date)
    let all_prices = match get_all_daily_prices(db) {
        Ok(p) => p,
        Err(_) => return result,
    };

    // Build a lookup: symbol -> BTreeMap<date, close_price * brl_rate>
    let mut price_lookup: BTreeMap<String, BTreeMap<NaiveDate, f64>> = BTreeMap::new();
    for &(ref symbol, date, value_brl) in &all_prices {
        price_lookup
            .entry(symbol.clone())
            .or_default()
            .insert(date, value_brl);
    }

    // Compute holdings at each date
    // Sort transactions by date first
    let mut sorted_txs = transactions.clone();
    sorted_txs.sort_by_key(|t| t.date);

    // For each price date, compute total portfolio value
    for date in &dates {
        // Compute holdings as of this date
        let mut holdings: BTreeMap<String, f64> = BTreeMap::new();
        for tx in &sorted_txs {
            if tx.date > *date {
                break;
            }
            let entry = holdings.entry(tx.symbol.clone()).or_insert(0.0);
            match tx.tx_type {
                investimentos_core::types::TxType::Buy
                | investimentos_core::types::TxType::FractionAuction => {
                    *entry += tx.quantity;
                }
                investimentos_core::types::TxType::Sell => {
                    *entry -= tx.quantity;
                }
                _ => {}
            }
        }

        // Sum portfolio value using the price for this date
        let mut total = 0.0;
        let mut has_any_price = false;

        for (symbol, qty) in &holdings {
            if *qty <= 0.0001 {
                continue;
            }
            // Find price for this symbol on this date (or most recent before)
            if let Some(symbol_prices) = price_lookup.get(symbol) {
                let range_end: NaiveDate = *date;
                if let Some((_price_date, value_per_unit)) =
                    symbol_prices.range(..=range_end).next_back()
                {
                    total += qty * value_per_unit;
                    has_any_price = true;
                }
            }
        }

        if has_any_price {
            result.insert(*date, total);
        }
    }

    result
}

/// Get all distinct dates from daily_prices, sorted ascending.
fn get_all_price_dates(db: &Database) -> Result<Vec<NaiveDate>, Box<dyn std::error::Error>> {
    let mut stmt = db
        .conn()
        .prepare("SELECT DISTINCT date FROM daily_prices ORDER BY date")?;
    let rows = stmt.query_map(params![], |row| {
        let s: String = row.get(0)?;
        Ok(s)
    })?;
    let dates: Vec<NaiveDate> = rows
        .filter_map(|r| r.ok())
        .filter_map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .collect();
    Ok(dates)
}

/// Get all daily prices as (symbol, date, close_price * brl_rate).
fn get_all_daily_prices(
    db: &Database,
) -> Result<Vec<(String, NaiveDate, f64)>, Box<dyn std::error::Error>> {
    let mut stmt = db.conn().prepare(
        "SELECT symbol, date, close_price * brl_rate FROM daily_prices ORDER BY date",
    )?;
    let rows = stmt.query_map(params![], |row| {
        let symbol: String = row.get(0)?;
        let date_str: String = row.get(1)?;
        let value: f64 = row.get(2)?;
        Ok((symbol, date_str, value))
    })?;
    let prices: Vec<(String, NaiveDate, f64)> = rows
        .filter_map(|r| r.ok())
        .filter_map(|(sym, ds, val)| {
            NaiveDate::parse_from_str(&ds, "%Y-%m-%d")
                .ok()
                .map(|d| (sym, d, val))
        })
        .collect();
    Ok(prices)
}
