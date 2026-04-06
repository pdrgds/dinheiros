use gpui::{
    div, prelude::*, rgb, ClickEvent, Context, FontWeight, SharedString, Window,
};

use crate::theme;

// ---------------------------------------------------------------------------
// Tab enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Positions,
    Income,
    History,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Overview, Tab::Positions, Tab::Income, Tab::History];

    pub fn label(&self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Positions => "Positions",
            Tab::Income => "Income",
            Tab::History => "History",
        }
    }
}

// ---------------------------------------------------------------------------
// Root view
// ---------------------------------------------------------------------------

pub struct AppRoot {
    active_tab: Tab,
}

impl AppRoot {
    pub fn new() -> Self {
        Self {
            active_tab: Tab::Overview,
        }
    }

    fn set_tab(&mut self, tab: Tab, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.active_tab = tab;
        cx.notify();
    }
}

impl Render for AppRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.active_tab;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme::BG_PRIMARY)
            .text_color(theme::TEXT_PRIMARY)
            .child(self.render_navbar(active, cx))
            .child(self.render_content(active))
    }
}

impl AppRoot {
    // ----- top navigation bar -----
    fn render_navbar(
        &self,
        active: Tab,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .w_full()
            .px_4()
            .py_2()
            .bg(theme::BG_SECONDARY)
            .border_b_1()
            .border_color(theme::BORDER)
            .child(self.render_tabs(active, cx))
            .child(self.render_actions(cx))
    }

    fn render_tabs(
        &self,
        active: Tab,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut row = div().flex().flex_row().gap_1();
        for tab in Tab::ALL {
            row = row.child(self.render_tab_button(tab, tab == active, cx));
        }
        row
    }

    fn render_tab_button(
        &self,
        tab: Tab,
        is_active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let bg = if is_active {
            theme::ACCENT
        } else {
            theme::BG_SECONDARY
        };
        let text_col = if is_active {
            rgb(0xffffff)
        } else {
            theme::TEXT_SECONDARY
        };
        let hover_bg = if is_active {
            theme::ACCENT
        } else {
            theme::BORDER
        };

        div()
            .id(SharedString::from(tab.label()))
            .px_3()
            .py_1()
            .rounded_md()
            .bg(bg)
            .text_color(text_col)
            .text_sm()
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .on_click(cx.listener(move |this, ev, window, cx| {
                this.set_tab(tab, ev, window, cx);
            }))
            .child(tab.label())
    }

    fn render_actions(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .gap_2()
            .child(action_button("import-btn", "Import", theme::ACCENT, cx))
            .child(action_button("gold-btn", "+ Gold", theme::YELLOW, cx))
            .child(action_button("export-btn", "Export", theme::GREEN, cx))
    }

    // ----- content area -----
    fn render_content(&self, active: Tab) -> impl IntoElement {
        div()
            .flex()
            .flex_1()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme::TEXT_SECONDARY)
                    .child(format!("{} (coming soon)", active.label())),
            )
    }
}

// ---------------------------------------------------------------------------
// Action button (placeholder — logs to stdout for now)
// ---------------------------------------------------------------------------

fn action_button(
    id: &'static str,
    label: &'static str,
    color: gpui::Rgba,
    _cx: &mut Context<AppRoot>,
) -> impl IntoElement {
    div()
        .id(id)
        .px_3()
        .py_1()
        .rounded_md()
        .bg(color)
        .text_color(rgb(0xffffff))
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .cursor_pointer()
        .hover(move |style| style.opacity(0.85))
        .on_click(move |_, _, _| {
            println!("Action: {label}");
        })
        .child(label)
}
