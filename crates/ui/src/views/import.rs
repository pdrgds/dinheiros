use gpui::{div, prelude::*, rgb, AnyElement, FontWeight};

use crate::theme;

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

pub fn render_import(back_label: &str) -> AnyElement {
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
                .child("Import Transactions"),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme::TEXT_SECONDARY)
                .child("Select a file format to import transactions from:"),
        )
        .child(render_import_buttons())
        .child(
            div()
                .text_sm()
                .text_color(theme::TEXT_SECONDARY)
                .mt_4()
                .child(format!("Press Back to return to {}", back_label)),
        )
        .into_any_element()
}

fn render_import_buttons() -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .gap_4()
        .child(import_button(
            "import-b3",
            "Import B3 (.xlsx)",
            theme::GREEN,
        ))
        .child(import_button(
            "import-binance",
            "Import Binance (.csv)",
            theme::YELLOW,
        ))
}

fn import_button(id: &'static str, label: &'static str, color: gpui::Rgba) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .px_6()
        .py_4()
        .rounded_lg()
        .bg(color)
        .text_color(rgb(0xffffff))
        .font_weight(FontWeight::SEMIBOLD)
        .cursor_pointer()
        .hover(|style| style.opacity(0.85))
        .on_click(move |_, _, _| {
            println!(
                "[import] {} — file picker not yet implemented, use CLI import",
                label
            );
        })
        .child(label)
}
