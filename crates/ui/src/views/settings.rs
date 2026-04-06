use gpui::{div, prelude::*, AnyElement, Div, FontWeight};

use investimentos_core::db::{queries, Database};

use crate::theme;

// ---------------------------------------------------------------------------
// Config keys we display
// ---------------------------------------------------------------------------

const CONFIG_KEYS: &[(&str, &str)] = &[
    ("ibkr_flex_token", "IBKR Flex Token"),
    ("ibkr_flex_query_id", "IBKR Flex Query ID"),
    ("coingecko_api_key", "CoinGecko API Key"),
];

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

pub fn render_settings(db: &Database) -> AnyElement {
    let mut content = div()
        .flex()
        .flex_col()
        .gap_6()
        .p_6()
        .w_full()
        .flex_1();

    content = content.child(
        div()
            .text_2xl()
            .font_weight(FontWeight::BOLD)
            .child("Settings"),
    );

    // Config values panel
    content = content.child(render_config_panel(db));

    // Instructions
    content = content.child(
        div()
            .text_sm()
            .text_color(theme::TEXT_SECONDARY)
            .child("Edit settings via JSON export/import or CLI."),
    );

    content.into_any_element()
}

fn render_config_panel(db: &Database) -> Div {
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
            .child("Configuration"),
    );

    for (key, label) in CONFIG_KEYS {
        let value = queries::get_config(db, key)
            .ok()
            .flatten()
            .unwrap_or_default();
        panel = panel.child(render_config_row(label, &value));
    }

    panel
}

fn render_config_row(label: &str, value: &str) -> Div {
    let display_value = if value.is_empty() {
        "(not set)".to_string()
    } else {
        mask_value(value)
    };

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_4()
        .py_1()
        .border_b_1()
        .border_color(theme::BORDER)
        .child(
            div()
                .w(gpui::px(160.0))
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .child(label.to_string()),
        )
        .child(
            div()
                .text_sm()
                .text_color(if display_value == "(not set)" {
                    theme::TEXT_SECONDARY
                } else {
                    theme::TEXT_PRIMARY
                })
                .child(display_value),
        )
}

/// Mask a config value, showing only the first 4 and last 2 characters.
fn mask_value(v: &str) -> String {
    if v.len() <= 6 {
        return "****".to_string();
    }
    let first = &v[..4];
    let last = &v[v.len() - 2..];
    format!("{}...{}", first, last)
}
