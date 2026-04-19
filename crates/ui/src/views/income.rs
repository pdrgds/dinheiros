use gpui::{div, prelude::*, AnyElement, Div, FontWeight};

use dinheiros_core::db::queries;
use dinheiros_core::db::Database;
use dinheiros_core::types::*;

use crate::theme;
use crate::views::format_brl;

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

pub fn render_income(db: &Database) -> AnyElement {
    let mut events = queries::get_all_income(db).unwrap_or_default();
    // Newest first
    events.reverse();

    let total_net: f64 = events.iter().map(|e| e.net_value_brl).sum();

    let mut content = div()
        .id("income-scroll")
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .w_full()
        .flex_1()
        .overflow_y_scroll();

    // Summary header
    content = content.child(
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .text_color(theme::TEXT_SECONDARY)
                    .child("Total Net Income"),
            )
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme::GREEN)
                    .child(format!("R$ {}", format_brl(total_net))),
            ),
    );

    content = content.child(
        div()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme::TEXT_SECONDARY)
            .child(format!("Income Events ({})", events.len())),
    );

    content = content.child(render_header_row());

    for ev in &events {
        content = content.child(render_income_row(ev));
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
        .child(div().w(gpui::px(90.0)).child("Date"))
        .child(div().w(gpui::px(80.0)).child("Symbol"))
        .child(div().w(gpui::px(70.0)).child("Source"))
        .child(div().w(gpui::px(80.0)).child("Type"))
        .child(div().w(gpui::px(100.0)).child("Gross"))
        .child(div().w(gpui::px(80.0)).child("Tax"))
        .child(div().w(gpui::px(80.0)).child("Origin"))
        .child(div().w(gpui::px(110.0)).child("Net (BRL)"))
}

// ---------------------------------------------------------------------------
// Table row
// ---------------------------------------------------------------------------

fn render_income_row(ev: &Income) -> Div {
    let type_label = match ev.income_type {
        IncomeType::Dividend => "Dividend",
        IncomeType::Jcp => "JCP",
    };

    let tax_str = match ev.tax_withheld {
        Some(t) => format!("{} {}", ev.currency, format_brl(t)),
        None => "—".to_string(),
    };

    let origin_str = ev.tax_origin.clone().unwrap_or_else(|| "—".to_string());

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
                .child(ev.date.format("%Y-%m-%d").to_string()),
        )
        .child(
            div()
                .w(gpui::px(80.0))
                .font_weight(FontWeight::MEDIUM)
                .child(ev.symbol.clone()),
        )
        .child(
            div()
                .w(gpui::px(70.0))
                .child(ev.source.as_str()),
        )
        .child(div().w(gpui::px(80.0)).child(type_label))
        .child(
            div()
                .w(gpui::px(100.0))
                .child(format!("{} {}", ev.currency, format_brl(ev.gross_value))),
        )
        .child(div().w(gpui::px(80.0)).child(tax_str))
        .child(div().w(gpui::px(80.0)).child(origin_str))
        .child(
            div()
                .w(gpui::px(110.0))
                .text_color(theme::GREEN)
                .font_weight(FontWeight::MEDIUM)
                .child(format!("R$ {}", format_brl(ev.net_value_brl))),
        )
}
