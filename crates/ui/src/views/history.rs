use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{Duration, Local, NaiveDate};
use gpui::{
    canvas, div, point, prelude::*, px, rgb, AnyElement, Bounds, ClickEvent, Context, Div,
    FontWeight, PathBuilder, Pixels, Rgba, SharedString,
};
use rusqlite::params;

use dinheiros_core::db::Database;

use crate::app::AppRoot;
use crate::theme;
use crate::views::format_brl;

// ---------------------------------------------------------------------------
// Time range
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeRange {
    Days30,
    Days90,
    Year1,
    All,
}

impl TimeRange {
    pub const ALL: [TimeRange; 4] = [Self::Days30, Self::Days90, Self::Year1, Self::All];

    fn label(&self) -> &'static str {
        match self {
            Self::Days30 => "30d",
            Self::Days90 => "90d",
            Self::Year1 => "1y",
            Self::All => "All",
        }
    }

    fn filter_date(&self) -> Option<NaiveDate> {
        let today = Local::now().date_naive();
        match self {
            Self::Days30 => Some(today - Duration::days(30)),
            Self::Days90 => Some(today - Duration::days(90)),
            Self::Year1 => Some(today - Duration::days(365)),
            Self::All => None,
        }
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::All
    }
}

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

pub fn render_history(
    db: &Database,
    time_range: TimeRange,
    split_view: bool,
    cx: &mut Context<AppRoot>,
) -> AnyElement {
    let all_series = compute_history_data_by_category(db);

    // Apply time filter and build total + category series
    let cutoff = time_range.filter_date();

    let filter = |data: &BTreeMap<NaiveDate, f64>| -> Vec<(NaiveDate, f64)> {
        if let Some(c) = cutoff {
            data.range(c..).map(|(d, v)| (*d, *v)).collect()
        } else {
            data.iter().map(|(d, v)| (*d, *v)).collect()
        }
    };

    let total_points = filter(&all_series.total);
    let stocks_points = filter(&all_series.stocks);
    let tesouro_points = filter(&all_series.tesouro);
    let crypto_points = filter(&all_series.crypto);

    div()
        .flex()
        .flex_col()
        .flex_1()
        .p_6()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::BOLD)
                        .child("Portfolio Value Over Time"),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap_2()
                        .child(render_split_toggle(split_view, cx))
                        .child(render_range_selector(time_range, cx)),
                ),
        )
        .child(render_value_summary(&total_points))
        .child(if total_points.len() >= 2 {
            if split_view {
                let series = vec![
                    ("Stocks", theme::ACCENT, &stocks_points),
                    ("Tesouro", theme::YELLOW, &tesouro_points),
                    ("Crypto", theme::RED, &crypto_points),
                ];
                render_multi_chart(&total_points, &series)
            } else {
                render_chart_with_axes(&total_points)
            }
        } else {
            div()
                .h(px(300.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_lg()
                .bg(theme::BG_SECONDARY)
                .border_1()
                .border_color(theme::BORDER)
                .child(
                    div()
                        .text_sm()
                        .text_color(theme::TEXT_SECONDARY)
                        .child("Not enough data for chart. Prices are being backfilled in the background."),
                )
                .into_any_element()
        })
        .child(render_backfill_status(db))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Range selector
// ---------------------------------------------------------------------------

fn render_split_toggle(active: bool, cx: &mut Context<AppRoot>) -> gpui::Stateful<Div> {
    let bg = if active { theme::ACCENT } else { theme::BG_SECONDARY };
    let text_col = if active { rgb(0xffffff) } else { theme::TEXT_SECONDARY };
    div()
        .id("split-toggle")
        .px_3()
        .py_1()
        .rounded_md()
        .bg(bg)
        .text_color(text_col)
        .text_sm()
        .cursor_pointer()
        .hover(move |s| if active { s } else { s.bg(theme::BORDER) })
        .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
            this.history_split = !this.history_split;
            cx.notify();
        }))
        .child("By Category")
}

fn render_range_selector(active: TimeRange, cx: &mut Context<AppRoot>) -> Div {
    let mut row = div().flex().flex_row().gap_1();
    for &range in &TimeRange::ALL {
        let is_active = range == active;
        let bg = if is_active { theme::ACCENT } else { theme::BG_SECONDARY };
        let text_col = if is_active { rgb(0xffffff) } else { theme::TEXT_SECONDARY };
        row = row.child(
            div()
                .id(SharedString::from(format!("range-{:?}", range)))
                .px_3()
                .py_1()
                .rounded_md()
                .bg(bg)
                .text_color(text_col)
                .text_sm()
                .cursor_pointer()
                .hover(move |s| if is_active { s } else { s.bg(theme::BORDER) })
                .on_click(cx.listener(move |this, _ev: &ClickEvent, _window, cx| {
                    this.history_range = range;
                    cx.notify();
                }))
                .child(range.label()),
        );
    }
    row
}

// ---------------------------------------------------------------------------
// Value summary
// ---------------------------------------------------------------------------

fn render_value_summary(points: &[(NaiveDate, f64)]) -> Div {
    if points.is_empty() {
        return div();
    }

    let first_val = points.first().map(|(_, v)| *v).unwrap_or(0.0);
    let latest_val = points.last().map(|(_, v)| *v).unwrap_or(0.0);
    let latest_date = points.last().map(|(d, _)| *d);

    let change = latest_val - first_val;
    let change_pct = if first_val.abs() > 0.01 {
        change / first_val * 100.0
    } else {
        0.0
    };

    let change_color = if change >= 0.0 { theme::GREEN } else { theme::RED };
    let sign = if change >= 0.0 { "+" } else { "" };
    let date_str = latest_date
        .map(|d| d.format("%d %b %Y").to_string())
        .unwrap_or_default();

    div()
        .flex()
        .flex_row()
        .items_end()
        .gap_4()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child(format!("R$ {}", format_brl(latest_val))),
        )
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(change_color)
                .pb(px(2.0))
                .child(format!(
                    "{}R$ {} ({}{:.1}%)",
                    sign,
                    format_brl(change.abs()),
                    sign,
                    change_pct
                )),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme::TEXT_SECONDARY)
                .pb(px(3.0))
                .child(date_str),
        )
}

// ---------------------------------------------------------------------------
// Chart with Y-axis labels and X-axis labels
// ---------------------------------------------------------------------------

fn render_chart_with_axes(points: &[(NaiveDate, f64)]) -> AnyElement {
    let n = points.len();
    if n < 2 {
        return div().into_any_element();
    }

    let min_val = points.iter().map(|(_, v)| *v).fold(f64::INFINITY, f64::min);
    let max_val = points.iter().map(|(_, v)| *v).fold(f64::NEG_INFINITY, f64::max);
    let range = (max_val - min_val).max(1.0);
    let y_min = min_val - range * 0.05;
    let y_max = max_val + range * 0.05;

    let y_labels: Vec<String> = (0..=4)
        .map(|i| {
            let val = y_max - (i as f64 / 4.0) * (y_max - y_min);
            if val >= 1_000_000.0 {
                format!("R$ {:.1}M", val / 1_000_000.0)
            } else if val >= 1_000.0 {
                format!("R$ {:.0}k", val / 1_000.0)
            } else {
                format!("R$ {:.0}", val)
            }
        })
        .collect();

    let x_label_count = 5.min(n);
    let x_labels: Vec<String> = (0..x_label_count)
        .map(|i| {
            let idx = if x_label_count <= 1 { 0 } else { i * (n - 1) / (x_label_count - 1) };
            points[idx].0.format("%b %y").to_string()
        })
        .collect();

    let points_arc = Arc::new(points.to_vec());

    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .flex_row()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .justify_between()
                        .w(px(70.0))
                        .h(px(300.0))
                        .pr_2()
                        .children(y_labels.into_iter().map(|label| {
                            div().text_xs().text_color(theme::TEXT_SECONDARY).text_right().child(label)
                        })),
                )
                .child(
                    div()
                        .flex_1()
                        .h(px(300.0))
                        .rounded_lg()
                        .bg(theme::BG_SECONDARY)
                        .border_1()
                        .border_color(theme::BORDER)
                        .child(
                            canvas(
                                move |_bounds, _window, _cx| points_arc.clone(),
                                move |bounds, points, window, _cx| {
                                    paint_chart(bounds, &points, window);
                                },
                            )
                            .size_full(),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .ml(px(70.0))
                .justify_between()
                .pt_1()
                .children(x_labels.into_iter().map(|label| {
                    div().text_xs().text_color(theme::TEXT_SECONDARY).child(label)
                })),
        )
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Canvas paint
// ---------------------------------------------------------------------------

fn paint_chart(bounds: Bounds<Pixels>, points: &[(NaiveDate, f64)], window: &mut gpui::Window) {
    if points.len() < 2 {
        return;
    }

    let pad = px(10.0);
    let chart_x = bounds.origin.x + pad;
    let chart_y = bounds.origin.y + pad;
    let chart_w = bounds.size.width - pad * 2.0;
    let chart_h = bounds.size.height - pad * 2.0;

    let min_val = points.iter().map(|(_, v)| *v).fold(f64::INFINITY, f64::min);
    let max_val = points.iter().map(|(_, v)| *v).fold(f64::NEG_INFINITY, f64::max);
    let range = (max_val - min_val).max(1.0);
    let y_min = min_val - range * 0.05;
    let y_max = max_val + range * 0.05;
    let y_range = y_max - y_min;
    let n = points.len();

    let to_px_x = |i: usize| -> Pixels {
        chart_x + chart_w * (i as f32 / (n - 1).max(1) as f32)
    };
    let to_px_y = |v: f64| -> Pixels {
        chart_y + chart_h * (1.0 - ((v - y_min) / y_range) as f32)
    };

    // Grid lines
    for i in 0..=4 {
        let y = chart_y + chart_h * (i as f32 / 4.0);
        let mut grid = PathBuilder::stroke(px(1.0));
        grid.move_to(point(chart_x, y));
        grid.line_to(point(chart_x + chart_w, y));
        if let Ok(path) = grid.build() {
            window.paint_path(path, rgb(0x333344));
        }
    }

    // Fill under line
    let mut fill_builder = PathBuilder::fill();
    fill_builder.move_to(point(to_px_x(0), to_px_y(points[0].1)));
    for i in 1..n {
        fill_builder.line_to(point(to_px_x(i), to_px_y(points[i].1)));
    }
    fill_builder.line_to(point(to_px_x(n - 1), chart_y + chart_h));
    fill_builder.line_to(point(to_px_x(0), chart_y + chart_h));
    fill_builder.close();
    if let Ok(path) = fill_builder.build() {
        window.paint_path(path, gpui::rgba(0x06b6d415));
    }

    // Line
    let mut line_builder = PathBuilder::stroke(px(2.0));
    line_builder.move_to(point(to_px_x(0), to_px_y(points[0].1)));
    for i in 1..n {
        line_builder.line_to(point(to_px_x(i), to_px_y(points[i].1)));
    }
    if let Ok(path) = line_builder.build() {
        window.paint_path(path, rgb(0x06b6d4));
    }
}

// ---------------------------------------------------------------------------
// Multi-line chart (split by category)
// ---------------------------------------------------------------------------

fn render_multi_chart(
    total_points: &[(NaiveDate, f64)],
    series: &[(&str, Rgba, &Vec<(NaiveDate, f64)>)],
) -> AnyElement {
    let n = total_points.len();
    if n < 2 {
        return div().into_any_element();
    }

    // Find global max across all series for Y-axis scaling
    let max_val = total_points.iter().map(|(_, v)| *v).fold(f64::NEG_INFINITY, f64::max);
    let y_max = max_val * 1.05;

    let y_labels: Vec<String> = (0..=4)
        .map(|i| {
            let val = y_max * (1.0 - i as f64 / 4.0);
            if val >= 1_000_000.0 {
                format!("R$ {:.1}M", val / 1_000_000.0)
            } else if val >= 1_000.0 {
                format!("R$ {:.0}k", val / 1_000.0)
            } else {
                format!("R$ {:.0}", val)
            }
        })
        .collect();

    let x_label_count = 5.min(n);
    let x_labels: Vec<String> = (0..x_label_count)
        .map(|i| {
            let idx = if x_label_count <= 1 { 0 } else { i * (n - 1) / (x_label_count - 1) };
            total_points[idx].0.format("%b %y").to_string()
        })
        .collect();

    // Build date→index lookup from total_points (reference timeline)
    let date_to_idx: BTreeMap<NaiveDate, usize> = total_points
        .iter()
        .enumerate()
        .map(|(i, (d, _))| (*d, i))
        .collect();

    // Prepare series data for the canvas (align each to the reference timeline)
    let mut canvas_series: Vec<(Rgba, Vec<(usize, f64)>)> = Vec::new();
    for &(_label, color, points) in series {
        let aligned: Vec<(usize, f64)> = points
            .iter()
            .filter_map(|(d, v)| date_to_idx.get(d).map(|&i| (i, *v)))
            .collect();
        if !aligned.is_empty() {
            canvas_series.push((color, aligned));
        }
    }
    let canvas_series = Arc::new(canvas_series);
    let n_total = n;
    let y_max_f = y_max;

    // Legend
    let legend = div()
        .flex()
        .flex_row()
        .gap_4()
        .pb_2()
        .children(series.iter().map(|&(label, color, _)| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .child(div().w(px(10.0)).h(px(3.0)).bg(color))
                .child(div().text_xs().text_color(theme::TEXT_SECONDARY).child(label.to_string()))
        }));

    div()
        .flex()
        .flex_col()
        .child(legend)
        .child(
            div()
                .flex()
                .flex_row()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .justify_between()
                        .w(px(70.0))
                        .h(px(300.0))
                        .pr_2()
                        .children(y_labels.into_iter().map(|label| {
                            div().text_xs().text_color(theme::TEXT_SECONDARY).text_right().child(label)
                        })),
                )
                .child(
                    div()
                        .flex_1()
                        .h(px(300.0))
                        .rounded_lg()
                        .bg(theme::BG_SECONDARY)
                        .border_1()
                        .border_color(theme::BORDER)
                        .child(
                            canvas(
                                move |_bounds, _window, _cx| canvas_series.clone(),
                                move |bounds, series, window, _cx| {
                                    paint_multi_chart(bounds, &series, n_total, y_max_f, window);
                                },
                            )
                            .size_full(),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .ml(px(70.0))
                .justify_between()
                .pt_1()
                .children(x_labels.into_iter().map(|label| {
                    div().text_xs().text_color(theme::TEXT_SECONDARY).child(label)
                })),
        )
        .into_any_element()
}

fn paint_multi_chart(
    bounds: Bounds<Pixels>,
    series: &[(Rgba, Vec<(usize, f64)>)],
    n_total: usize,
    y_max: f64,
    window: &mut gpui::Window,
) {
    let pad = px(10.0);
    let chart_x = bounds.origin.x + pad;
    let chart_y = bounds.origin.y + pad;
    let chart_w = bounds.size.width - pad * 2.0;
    let chart_h = bounds.size.height - pad * 2.0;

    let to_px_x = |i: usize| -> Pixels {
        chart_x + chart_w * (i as f32 / (n_total - 1).max(1) as f32)
    };
    let to_px_y = |v: f64| -> Pixels {
        chart_y + chart_h * (1.0 - (v / y_max) as f32)
    };

    // Grid lines
    for i in 0..=4 {
        let y = chart_y + chart_h * (i as f32 / 4.0);
        let mut grid = PathBuilder::stroke(px(1.0));
        grid.move_to(point(chart_x, y));
        grid.line_to(point(chart_x + chart_w, y));
        if let Ok(path) = grid.build() {
            window.paint_path(path, rgb(0x333344));
        }
    }

    // Draw each series
    for (color, points) in series {
        if points.len() < 2 {
            continue;
        }

        // Fill
        let mut fill_builder = PathBuilder::fill();
        fill_builder.move_to(point(to_px_x(points[0].0), to_px_y(points[0].1)));
        for &(i, v) in &points[1..] {
            fill_builder.line_to(point(to_px_x(i), to_px_y(v)));
        }
        fill_builder.line_to(point(to_px_x(points.last().unwrap().0), chart_y + chart_h));
        fill_builder.line_to(point(to_px_x(points[0].0), chart_y + chart_h));
        fill_builder.close();
        if let Ok(path) = fill_builder.build() {
            // Semi-transparent fill
            let fill_color = gpui::rgba(
                ((f32::from(color.r) * 255.0) as u32) << 24
                    | ((f32::from(color.g) * 255.0) as u32) << 16
                    | ((f32::from(color.b) * 255.0) as u32) << 8
                    | 0x15,
            );
            window.paint_path(path, fill_color);
        }

        // Line
        let mut line_builder = PathBuilder::stroke(px(2.0));
        line_builder.move_to(point(to_px_x(points[0].0), to_px_y(points[0].1)));
        for &(i, v) in &points[1..] {
            line_builder.line_to(point(to_px_x(i), to_px_y(v)));
        }
        if let Ok(path) = line_builder.build() {
            window.paint_path(path, *color);
        }
    }
}

// ---------------------------------------------------------------------------
// Backfill status
// ---------------------------------------------------------------------------

fn render_backfill_status(db: &Database) -> AnyElement {
    let today = Local::now().date_naive();

    let query = "
        SELECT t.symbol, MIN(t.date) as first_tx, t.asset_type
        FROM transactions t
        WHERE t.tx_type IN ('buy','sell')
        GROUP BY t.symbol
        HAVING SUM(CASE WHEN t.tx_type='buy' THEN t.quantity WHEN t.tx_type='sell' THEN -t.quantity END) > 0.0001
    ";

    let conn = db.conn();
    let mut stmt = match conn.prepare(query) {
        Ok(s) => s,
        Err(_) => return div().into_any_element(),
    };

    let symbols: Vec<(String, NaiveDate, String)> = stmt
        .query_map([], |row| {
            let symbol: String = row.get(0)?;
            let date_str: String = row.get(1)?;
            let asset_type: String = row.get(2)?;
            Ok((symbol, date_str, asset_type))
        })
        .ok()
        .map(|rows| {
            rows.filter_map(|r| r.ok())
                .filter_map(|(sym, ds, at)| {
                    NaiveDate::parse_from_str(&ds, "%Y-%m-%d")
                        .ok()
                        .map(|d| (sym, d, at))
                })
                .collect()
        })
        .unwrap_or_default();

    if symbols.is_empty() {
        return div().into_any_element();
    }

    let backfillable: Vec<_> = symbols
        .iter()
        .filter(|(sym, _, at)| at != "gold" && !sym.starts_with("Tesouro"))
        .collect();

    let mut total_needed: i64 = 0;
    let mut total_have: i64 = 0;

    for (symbol, first_tx, at) in &backfillable {
        let needed = dinheiros_core::calendar::trading_days(*first_tx, today, at);
        total_needed += needed;

        let have: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM daily_prices WHERE symbol = ?",
                [symbol],
                |row| row.get(0),
            )
            .unwrap_or(0);
        total_have += have.min(needed);
    }

    if total_needed == 0 {
        return div().into_any_element();
    }

    let pct = (total_have as f64 / total_needed as f64 * 100.0).min(100.0);
    let pct_f32 = pct as f32 / 100.0;

    let bar_color = if pct >= 90.0 {
        theme::GREEN
    } else if pct >= 50.0 {
        theme::YELLOW
    } else {
        theme::ACCENT
    };

    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_lg()
        .bg(theme::BG_SECONDARY)
        .border_1()
        .border_color(theme::BORDER)
        .child(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme::TEXT_SECONDARY)
                        .child("Price History Coverage"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::TEXT_SECONDARY)
                        .child(format!(
                            "{:.0}% — {} of ~{} data points across {} symbols",
                            pct, total_have, total_needed, backfillable.len()
                        )),
                ),
        )
        .child(
            div()
                .w_full()
                .h(px(6.0))
                .rounded_full()
                .bg(theme::BORDER)
                .child(
                    div()
                        .h_full()
                        .rounded_full()
                        .bg(bar_color)
                        .w(gpui::relative(pct_f32)),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme::TEXT_SECONDARY)
                .child("Historical prices are backfilled automatically in the background."),
        )
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Compute portfolio value for each date
// ---------------------------------------------------------------------------

struct HistorySeries {
    total: BTreeMap<NaiveDate, f64>,
    stocks: BTreeMap<NaiveDate, f64>,
    tesouro: BTreeMap<NaiveDate, f64>,
    crypto: BTreeMap<NaiveDate, f64>,
}

fn compute_history_data_by_category(db: &Database) -> HistorySeries {
    let empty = HistorySeries {
        total: BTreeMap::new(), stocks: BTreeMap::new(),
        tesouro: BTreeMap::new(), crypto: BTreeMap::new(),
    };

    let transactions = match dinheiros_core::db::queries::get_all_transactions(db) {
        Ok(txs) => txs,
        Err(_) => return empty,
    };

    if transactions.is_empty() {
        return empty;
    }

    let dates = match get_all_price_dates(db) {
        Ok(d) => d,
        Err(_) => return empty,
    };

    let all_prices = match get_all_daily_prices(db) {
        Ok(p) => p,
        Err(_) => return empty,
    };

    let mut price_lookup: BTreeMap<String, BTreeMap<NaiveDate, f64>> = BTreeMap::new();
    for &(ref symbol, date, value_brl) in &all_prices {
        price_lookup
            .entry(symbol.clone())
            .or_default()
            .insert(date, value_brl);
    }

    let asset_types: BTreeMap<String, String> = transactions
        .iter()
        .map(|tx| (tx.symbol.clone(), tx.asset_type.as_str().to_string()))
        .collect();

    let mut sorted_txs = transactions.clone();
    sorted_txs.sort_by_key(|t| t.date);

    // Detect split adjustments
    let mut split_adjustments: BTreeMap<String, Vec<(NaiveDate, f64)>> = BTreeMap::new();
    {
        let mut by_sym_date: BTreeMap<(String, NaiveDate), (f64, f64)> = BTreeMap::new();
        for tx in &sorted_txs {
            let key = (tx.symbol.clone(), tx.date);
            let entry = by_sym_date.entry(key).or_insert((0.0, 0.0));
            match tx.tx_type {
                dinheiros_core::types::TxType::Buy => entry.0 += tx.quantity,
                dinheiros_core::types::TxType::Sell => entry.1 += tx.quantity,
                _ => {}
            }
        }
        for ((symbol, date), (bought, sold)) in &by_sym_date {
            if *sold > 10.0 && *bought > 0.1 && (*sold / *bought > 1.5 || *bought / *sold > 1.5) {
                split_adjustments.entry(symbol.clone()).or_default().push((*date, *bought / *sold));
            }
        }
    }

    let mut result = HistorySeries {
        total: BTreeMap::new(), stocks: BTreeMap::new(),
        tesouro: BTreeMap::new(), crypto: BTreeMap::new(),
    };

    for date in &dates {
        let mut holdings: BTreeMap<String, f64> = BTreeMap::new();
        for tx in &sorted_txs {
            if tx.date > *date {
                break;
            }
            let entry = holdings.entry(tx.symbol.clone()).or_insert(0.0);
            match tx.tx_type {
                dinheiros_core::types::TxType::Buy
                | dinheiros_core::types::TxType::FractionAuction => {
                    *entry += tx.quantity;
                }
                dinheiros_core::types::TxType::Sell => {
                    *entry -= tx.quantity;
                }
                _ => {}
            }
        }

        // Apply split adjustments: for dates BEFORE a split, Yahoo's prices are
        // adjusted by the split factor, so we must adjust holdings by the same factor
        // to keep value = holdings × price correct.
        for (symbol, adjustments) in &split_adjustments {
            if let Some(qty) = holdings.get_mut(symbol) {
                for (split_date, factor) in adjustments {
                    if date < split_date {
                        *qty *= factor;
                    }
                }
            }
        }

        let mut total = 0.0;
        let mut stocks_val = 0.0;
        let mut tesouro_val = 0.0;
        let mut crypto_val = 0.0;
        let mut has_any_price = false;

        for (symbol, qty) in &holdings {
            if *qty <= 0.0001 {
                continue;
            }
            if let Some(symbol_prices) = price_lookup.get(symbol) {
                if let Some((_price_date, value_per_unit)) =
                    symbol_prices.range(..=*date).next_back()
                {
                    let val = qty * value_per_unit;
                    total += val;
                    has_any_price = true;

                    match asset_types.get(symbol).map(|s| s.as_str()).unwrap_or("") {
                        "tesouro" => tesouro_val += val,
                        "crypto" => crypto_val += val,
                        _ => stocks_val += val,
                    }
                }
            }
        }

        if has_any_price {
            result.total.insert(*date, total);
            result.stocks.insert(*date, stocks_val);
            result.tesouro.insert(*date, tesouro_val);
            result.crypto.insert(*date, crypto_val);
        }
    }

    result
}

fn get_all_price_dates(db: &Database) -> Result<Vec<NaiveDate>, Box<dyn std::error::Error>> {
    let mut stmt = db
        .conn()
        .prepare("SELECT DISTINCT date FROM daily_prices ORDER BY date")?;
    let rows = stmt.query_map(params![], |row| {
        let s: String = row.get(0)?;
        Ok(s)
    })?;
    Ok(rows
        .filter_map(|r| r.ok())
        .filter_map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .collect())
}

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
    Ok(rows
        .filter_map(|r| r.ok())
        .filter_map(|(sym, ds, val)| {
            NaiveDate::parse_from_str(&ds, "%Y-%m-%d")
                .ok()
                .map(|d| (sym, d, val))
        })
        .collect())
}
