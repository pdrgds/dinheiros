use std::path::PathBuf;

use gpui::{
    div, prelude::*, rgb, AnyElement, ClickEvent, Context, FontWeight, SharedString, Window,
};

use investimentos_core::db::Database;
use investimentos_core::export;

use crate::theme;
use crate::views;

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
// AppMode — what the main content area shows
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Tab(Tab),
    Import,
    ManualGold,
    Settings,
}

// ---------------------------------------------------------------------------
// Root view
// ---------------------------------------------------------------------------

pub struct AppRoot {
    mode: AppMode,
    last_tab: Tab,
    db: Database,
    status_message: Option<String>,
}

impl AppRoot {
    pub fn new(db_path: PathBuf) -> Self {
        let db = Database::open(&db_path).expect("Failed to open database");
        Self {
            mode: AppMode::Tab(Tab::Overview),
            last_tab: Tab::Overview,
            db,
            status_message: None,
        }
    }

    fn set_tab(&mut self, tab: Tab, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.last_tab = tab;
        self.mode = AppMode::Tab(tab);
        self.status_message = None;
        cx.notify();
    }

    fn set_mode(&mut self, mode: AppMode, cx: &mut Context<Self>) {
        if let AppMode::Tab(t) = self.mode {
            self.last_tab = t;
        }
        self.mode = mode;
        self.status_message = None;
        cx.notify();
    }

    fn go_back(&mut self, cx: &mut Context<Self>) {
        self.mode = AppMode::Tab(self.last_tab);
        self.status_message = None;
        cx.notify();
    }

    fn do_export(&mut self, cx: &mut Context<Self>) {
        let out_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("investimentos-v2")
            .join("export.json");

        match export::export_to_json(&self.db, &out_path) {
            Ok(()) => {
                self.status_message = Some(format!("Exported to {}", out_path.display()));
                println!("[export] success: {}", out_path.display());
            }
            Err(e) => {
                self.status_message = Some(format!("Export failed: {}", e));
                eprintln!("[export] error: {}", e);
            }
        }
        cx.notify();
    }
}

impl Render for AppRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab = match self.mode {
            AppMode::Tab(t) => Some(t),
            _ => None,
        };

        let mut root = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme::BG_PRIMARY)
            .text_color(theme::TEXT_PRIMARY)
            .child(self.render_navbar(active_tab, cx))
            .child(self.render_content(cx));

        // Status bar at the bottom
        if let Some(ref msg) = self.status_message {
            root = root.child(
                div()
                    .px_4()
                    .py_1()
                    .bg(theme::BG_SECONDARY)
                    .border_t_1()
                    .border_color(theme::BORDER)
                    .text_xs()
                    .text_color(theme::TEXT_SECONDARY)
                    .child(msg.clone()),
            );
        }

        root
    }
}

impl AppRoot {
    // ----- top navigation bar -----
    fn render_navbar(
        &self,
        active_tab: Option<Tab>,
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
            .child(self.render_tabs(active_tab, cx))
            .child(self.render_actions(cx))
    }

    fn render_tabs(
        &self,
        active_tab: Option<Tab>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut row = div().flex().flex_row().gap_1();
        for tab in Tab::ALL {
            let is_active = active_tab == Some(tab);
            row = row.child(self.render_tab_button(tab, is_active, cx));
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
            .child(
                action_button("import-btn", "Import", theme::ACCENT)
                    .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                        this.set_mode(AppMode::Import, cx);
                    })),
            )
            .child(
                action_button("gold-btn", "+ Gold", theme::YELLOW)
                    .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                        this.set_mode(AppMode::ManualGold, cx);
                    })),
            )
            .child(
                action_button("export-btn", "Export", theme::GREEN)
                    .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                        this.do_export(cx);
                    })),
            )
            .child(
                action_button("settings-btn", "Settings", theme::BORDER)
                    .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                        this.set_mode(AppMode::Settings, cx);
                    })),
            )
    }

    // ----- content area -----
    fn render_content(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.mode {
            AppMode::Tab(Tab::Overview) => views::overview::render_overview(&self.db),
            AppMode::Tab(Tab::Positions) => views::positions::render_positions(&self.db),
            AppMode::Tab(Tab::Income) => views::income::render_income(&self.db),
            AppMode::Tab(Tab::History) => views::history::render_history(&self.db),
            AppMode::Import => self.render_mode_with_back(
                views::import::render_import(self.last_tab.label()),
                cx,
            ),
            AppMode::ManualGold => self.render_mode_with_back(
                views::manual::render_manual_gold(),
                cx,
            ),
            AppMode::Settings => self.render_mode_with_back(
                views::settings::render_settings(&self.db),
                cx,
            ),
        }
    }

    /// Wrap a view's content in a container that includes a Back button at the bottom.
    fn render_mode_with_back(&self, inner: AnyElement, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .flex_1()
            .child(inner)
            .child(self.render_back_bar(cx))
            .into_any_element()
    }

    fn render_back_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let last_tab_label = self.last_tab.label();
        div()
            .px_6()
            .pb_4()
            .child(
                div()
                    .id("back-btn")
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(theme::BG_SECONDARY)
                    .border_1()
                    .border_color(theme::BORDER)
                    .text_sm()
                    .text_color(theme::TEXT_SECONDARY)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme::BORDER))
                    .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                        this.go_back(cx);
                    }))
                    .child(format!("Back to {}", last_tab_label)),
            )
    }
}

// ---------------------------------------------------------------------------
// Action button helper
// ---------------------------------------------------------------------------

fn action_button(
    id: &'static str,
    label: &'static str,
    color: gpui::Rgba,
) -> gpui::Stateful<gpui::Div> {
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
        .child(label)
}
