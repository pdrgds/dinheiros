use gpui::{div, prelude::*, AnyElement, FontWeight};

use crate::theme;

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

pub fn render_manual_gold() -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_6()
        .p_6()
        .w_full()
        .flex_1()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Add Gold Purchase"),
        )
        .child(
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
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme::YELLOW)
                        .child("Manual Entry"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme::TEXT_SECONDARY)
                        .child(
                            "Gold entries are managed via JSON export/import for now. \
                             Use the CLI to add gold purchase transactions.",
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::TEXT_SECONDARY)
                        .mt_2()
                        .child("Tip: run `investimentos import --file gold.json` from the terminal"),
                ),
        )
        .into_any_element()
}
