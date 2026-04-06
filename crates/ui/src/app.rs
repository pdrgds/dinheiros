use std::path::PathBuf;

use gpui::{
    div, prelude::*, rgb, AnyElement, ClickEvent, Context, FocusHandle, FontWeight, KeyDownEvent,
    SharedString, Window,
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
    syncing: bool,
    // Settings editing state
    settings_fields: [String; 3],
    settings_active_field: Option<usize>,
    settings_focus: FocusHandle,
}

impl AppRoot {
    pub fn new(db_path: PathBuf, cx: &mut Context<Self>) -> Self {
        let db = Database::open(&db_path).expect("Failed to open database");
        let focus = cx.focus_handle();
        Self {
            mode: AppMode::Tab(Tab::Overview),
            last_tab: Tab::Overview,
            db,
            status_message: None,
            syncing: false,
            settings_fields: [String::new(), String::new(), String::new()],
            settings_active_field: None,
            settings_focus: focus,
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
        if mode == AppMode::Settings {
            self.load_settings_from_db();
        }
        self.mode = mode;
        self.status_message = None;
        cx.notify();
    }

    fn load_settings_from_db(&mut self) {
        use investimentos_core::db::queries;
        let keys = ["ibkr_flex_token", "ibkr_flex_query_id", "coingecko_api_key"];
        for (i, key) in keys.iter().enumerate() {
            self.settings_fields[i] = queries::get_config(&self.db, key)
                .ok()
                .flatten()
                .unwrap_or_default();
        }
        self.settings_active_field = None;
    }

    fn save_settings_to_db(&mut self, cx: &mut Context<Self>) {
        use investimentos_core::db::queries;
        let keys = ["ibkr_flex_token", "ibkr_flex_query_id", "coingecko_api_key"];
        let mut errors = Vec::new();
        for (i, key) in keys.iter().enumerate() {
            if let Err(e) = queries::set_config(&self.db, key, &self.settings_fields[i]) {
                errors.push(format!("{}: {}", key, e));
            }
        }
        if errors.is_empty() {
            self.status_message = Some("Settings saved.".to_string());
        } else {
            self.status_message = Some(format!("Save errors: {}", errors.join(", ")));
        }
        self.settings_active_field = None;
        cx.notify();
    }

    fn handle_settings_key(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(idx) = self.settings_active_field else {
            return;
        };

        let keystroke = &event.keystroke;

        // Cmd+V (paste)
        if keystroke.modifiers.platform && keystroke.key == "v" {
            if let Some(item) = cx.read_from_clipboard() {
                if let Some(text) = item.text() {
                    // Take only the first line and trim
                    let clean: String = text.lines().next().unwrap_or("").trim().to_string();
                    self.settings_fields[idx] = clean;
                    cx.notify();
                }
            }
            return;
        }

        // Cmd+A (select all / clear for simplicity)
        if keystroke.modifiers.platform && keystroke.key == "a" {
            // No-op or select all - we just ignore
            return;
        }

        // Ignore other modifier combos (Cmd+X, Cmd+C, etc.)
        if keystroke.modifiers.platform || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.settings_fields[idx].pop();
                cx.notify();
            }
            "escape" => {
                self.settings_active_field = None;
                cx.notify();
            }
            "tab" => {
                // Move to next field
                self.settings_active_field = Some((idx + 1) % 3);
                cx.notify();
            }
            "enter" => {
                // Move to next field or deselect on last
                if idx < 2 {
                    self.settings_active_field = Some(idx + 1);
                } else {
                    self.settings_active_field = None;
                }
                cx.notify();
            }
            _ => {
                // Type the character
                if let Some(ref ch) = keystroke.key_char {
                    self.settings_fields[idx].push_str(ch);
                    cx.notify();
                }
            }
        }
    }

    fn go_back(&mut self, cx: &mut Context<Self>) {
        self.mode = AppMode::Tab(self.last_tab);
        self.status_message = None;
        cx.notify();
    }

    fn do_import(&mut self, source: &'static str, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: None,
        });

        let db_path: PathBuf = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("investimentos-v2")
            .join("data.db");

        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let paths_result = rx.await;
            let paths = match paths_result {
                Ok(Ok(Some(paths))) => paths,
                _ => return, // cancelled or error
            };

            let db = match Database::open(&db_path) {
                Ok(db) => db,
                Err(e) => {
                    let _ = this.update(cx, |this, cx: &mut Context<Self>| {
                        this.status_message = Some(format!("DB error: {e}"));
                        cx.notify();
                    });
                    return;
                }
            };

            let mut total_msg = Vec::new();

            for path in &paths {
                let msg = match source {
                    "b3" => {
                        match investimentos_core::parsers::b3::parse_b3_xlsx(path) {
                            Ok(result) => {
                                let mut tx_new = 0u32;
                                let mut inc_new = 0u32;
                                for tx in &result.transactions {
                                    if let Ok(true) = investimentos_core::db::queries::insert_transaction(&db, tx) {
                                        tx_new += 1;
                                    }
                                }
                                for inc in &result.income {
                                    if let Ok(true) = investimentos_core::db::queries::insert_income(&db, inc) {
                                        inc_new += 1;
                                    }
                                }
                                format!("{}: {} tx ({} new), {} income ({} new)",
                                    path.file_name().unwrap_or_default().to_string_lossy(),
                                    result.transactions.len(), tx_new,
                                    result.income.len(), inc_new)
                            }
                            Err(e) => format!("{}: error: {e}", path.file_name().unwrap_or_default().to_string_lossy()),
                        }
                    }
                    "binance" => {
                        match investimentos_core::parsers::binance::parse_binance_csv(path) {
                            Ok(result) => {
                                let mut tx_new = 0u32;
                                for tx in &result.transactions {
                                    if let Ok(true) = investimentos_core::db::queries::insert_transaction(&db, tx) {
                                        tx_new += 1;
                                    }
                                }
                                format!("{}: {} tx ({} new), net BTC: {:.8}",
                                    path.file_name().unwrap_or_default().to_string_lossy(),
                                    result.transactions.len(), tx_new, result.net_btc)
                            }
                            Err(e) => format!("{}: error: {e}", path.file_name().unwrap_or_default().to_string_lossy()),
                        }
                    }
                    "ibkr-csv" => {
                        match import_ibkr_csv(&db, path) {
                            Ok(msg) => msg,
                            Err(e) => format!("{}: error: {e}", path.file_name().unwrap_or_default().to_string_lossy()),
                        }
                    }
                    _ => "Unknown source".to_string(),
                };
                total_msg.push(msg);
            }

            let _ = this.update(cx, |this, cx: &mut Context<Self>| {
                this.status_message = Some(total_msg.join(". "));
                this.mode = AppMode::Tab(this.last_tab);
                cx.notify();
            });
        }).detach();
    }

    fn do_sync(&mut self, cx: &mut Context<Self>) {
        if self.syncing {
            return;
        }
        self.syncing = true;
        self.status_message = Some("Syncing...".to_string());
        cx.notify();

        // DB path for the background thread (separate connection)
        let db_path: PathBuf = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("investimentos-v2")
            .join("data.db");

        // Shared result slot
        let result_slot = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let slot_writer = result_slot.clone();

        // Run sync in a std::thread
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let msg = rt.block_on(async {
                let db = match Database::open(&db_path) {
                    Ok(db) => db,
                    Err(e) => return format!("DB error: {e}"),
                };

                let mut parts = Vec::new();

                // Step 1: Try IBKR Flex fetch if configured
                let token = investimentos_core::db::queries::get_config(&db, "ibkr_flex_token")
                    .ok().flatten().unwrap_or_default();
                let query_id = investimentos_core::db::queries::get_config(&db, "ibkr_flex_query_id")
                    .ok().flatten().unwrap_or_default();

                if !token.is_empty() && !query_id.is_empty() {
                    match investimentos_core::api::ibkr_flex::fetch_flex_statement(&token, &query_id).await {
                        Ok(xml) => {
                            // Debug: dump raw XML for inspection
                            let _ = std::fs::write("/tmp/ibkr_flex_debug.xml", &xml);

                            match investimentos_core::parsers::ibkr_flex::parse_flex_xml(&xml) {
                                Ok(result) => {
                                    let mut tx_new = 0u32;
                                    let mut inc_new = 0u32;

                                    for tx in &result.transactions {
                                        match investimentos_core::db::queries::insert_transaction(&db, tx) {
                                            Ok(true) => tx_new += 1,
                                            Ok(false) => {} // duplicate, already exists
                                            Err(e) => {
                                                parts.push(format!("IBKR tx insert err: {e}"));
                                                break;
                                            }
                                        }
                                    }

                                    for inc in &result.income {
                                        match investimentos_core::db::queries::insert_income(&db, inc) {
                                            Ok(true) => inc_new += 1,
                                            Ok(false) => {} // duplicate
                                            Err(e) => {
                                                parts.push(format!("IBKR income insert err: {e}"));
                                                break;
                                            }
                                        }
                                    }

                                    // Store daily prices from open positions
                                    for price in &result.daily_prices {
                                        let _ = investimentos_core::db::queries::upsert_daily_price(&db, price);
                                    }

                                    parts.push(format!(
                                        "IBKR: {} trades ({} new), {} income ({} new), {} positions",
                                        result.transactions.len(),
                                        tx_new,
                                        result.income.len(),
                                        inc_new,
                                        result.positions_imported,
                                    ));
                                }
                                Err(e) => parts.push(format!("IBKR parse: {e}")),
                            }
                        }
                        Err(e) => parts.push(format!("IBKR: {e}")),
                    }
                }

                // Step 2: Fetch current prices
                match investimentos_core::reconcile::fetch_current_prices(&db).await {
                    Ok(n) => parts.push(format!("{n} prices fetched")),
                    Err(e) => parts.push(format!("Price fetch: {e}")),
                }

                // Step 3: Backfill historical prices
                match investimentos_core::reconcile::backfill_prices(&db).await {
                    Ok(r) => {
                        if r.prices_backfilled > 0 {
                            parts.push(format!("{} prices backfilled", r.prices_backfilled));
                        }
                        if r.rate_limited {
                            parts.push("rate limited, will resume next sync".to_string());
                        }
                    }
                    Err(e) => parts.push(format!("Backfill: {e}")),
                }

                parts.join(". ")
            });
            *slot_writer.lock().unwrap() = Some(msg);
        });

        // Poll for the result from GPUI's async executor
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            loop {
                if let Some(msg) = result_slot.lock().unwrap().take() {
                    let _ = this.update(cx, |this, cx: &mut Context<Self>| {
                        this.syncing = false;
                        this.status_message = Some(msg);
                        cx.notify();
                    });
                    break;
                }
                gpui::Timer::after(std::time::Duration::from_millis(200)).await;
            }
        }).detach();
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
            .child({
                let sync_label = if self.syncing { "Syncing..." } else { "Sync" };
                let sync_color = if self.syncing { theme::TEXT_SECONDARY } else { rgb(0x06b6d4) };
                action_button("sync-btn", sync_label, sync_color)
                    .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                        this.do_sync(cx);
                    }))
            })
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
            AppMode::Import => self.render_import_mode(cx),
            AppMode::ManualGold => self.render_mode_with_back(
                views::manual::render_manual_gold(),
                cx,
            ),
            AppMode::Settings => self.render_settings_mode(cx),
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

    // ----- Import mode -----
    fn render_import_mode(&self, cx: &mut Context<Self>) -> AnyElement {
        let content = div()
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
                    .child("Select a file format to import. A file picker will open."),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .child(
                        action_button("do-import-b3", "Import B3 (.xlsx)", theme::GREEN)
                            .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                                this.do_import("b3", cx);
                            })),
                    )
                    .child(
                        action_button("do-import-binance", "Import Binance (.csv)", theme::YELLOW)
                            .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                                this.do_import("binance", cx);
                            })),
                    )
                    .child(
                        action_button("do-import-ibkr", "Import IBKR CSV", theme::ACCENT)
                            .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                                this.do_import("ibkr-csv", cx);
                            })),
                    ),
            );

        div()
            .flex()
            .flex_col()
            .w_full()
            .flex_1()
            .child(content)
            .child(self.render_back_bar(cx))
            .into_any_element()
    }

    // ----- Settings mode -----
    fn render_settings_mode(&self, cx: &mut Context<Self>) -> AnyElement {
        let labels = ["IBKR Flex Token", "IBKR Flex Query ID", "CoinGecko API Key"];

        let mut content = div()
            .id("settings-panel")
            .track_focus(&self.settings_focus)
            .flex()
            .flex_col()
            .gap_6()
            .p_6()
            .w_full()
            .flex_1()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                this.handle_settings_key(ev, window, cx);
            }));

        content = content.child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Settings"),
        );

        // Config fields panel
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

        for (i, label) in labels.iter().enumerate() {
            let value = self.settings_fields[i].clone();
            let is_active = self.settings_active_field == Some(i);
            panel = panel.child(self.render_editable_row(i, label, &value, is_active, cx));
        }

        content = content.child(panel);

        // Save button
        content = content.child(
            div().flex().flex_row().gap_2().child(
                div()
                    .id("save-settings-btn")
                    .px_4()
                    .py_2()
                    .rounded_md()
                    .bg(theme::GREEN)
                    .text_color(rgb(0xffffff))
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.85))
                    .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                        this.save_settings_to_db(cx);
                    }))
                    .child("Save"),
            ),
        );

        // Hint
        content = content.child(
            div()
                .text_sm()
                .text_color(theme::TEXT_SECONDARY)
                .child("Click a field to edit. Type to enter text. Cmd+V to paste. Enter/Tab to move to next field."),
        );

        // Wrap with back bar
        div()
            .flex()
            .flex_col()
            .w_full()
            .flex_1()
            .child(content)
            .child(self.render_back_bar(cx))
            .into_any_element()
    }

    fn render_editable_row(
        &self,
        index: usize,
        label: &str,
        value: &str,
        is_active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let display_value = if is_active {
            if value.is_empty() {
                String::new()
            } else {
                value.to_string()
            }
        } else if value.is_empty() {
            "(not set)".to_string()
        } else {
            mask_value(value)
        };

        let border_col = if is_active {
            theme::ACCENT
        } else {
            theme::BORDER
        };

        let field_bg = if is_active {
            rgb(0x0d0d1a)
        } else {
            theme::BG_PRIMARY
        };

        let text_col = if value.is_empty() && !is_active {
            theme::TEXT_SECONDARY
        } else {
            theme::TEXT_PRIMARY
        };

        let id = SharedString::from(format!("settings-field-{}", index));

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
                    .id(id)
                    .flex_1()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(field_bg)
                    .border_1()
                    .border_color(border_col)
                    .text_sm()
                    .text_color(text_col)
                    .cursor_pointer()
                    .min_h(gpui::px(28.0))
                    .on_click(cx.listener(move |this, _ev: &ClickEvent, window, cx| {
                        this.settings_active_field = Some(index);
                        this.settings_focus.focus(window);
                        cx.notify();
                    }))
                    .child(if is_active && display_value.is_empty() {
                        // Show blinking cursor placeholder
                        "\u{258F}".to_string() // thin cursor char
                    } else if is_active {
                        format!("{}\u{258F}", display_value)
                    } else {
                        display_value
                    }),
            )
    }
}

// ---------------------------------------------------------------------------
// Action button helper
// ---------------------------------------------------------------------------

/// Mask a config value, showing only the first 4 and last 2 characters.
fn mask_value(v: &str) -> String {
    if v.len() <= 6 {
        return "****".to_string();
    }
    let first = &v[..4];
    let last = &v[v.len() - 2..];
    format!("{}...{}", first, last)
}

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

/// Parse IBKR CSV activity statement and insert trades + dividends into DB.
fn import_ibkr_csv(
    db: &Database,
    path: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let filename = path.file_name().unwrap_or_default().to_string_lossy();

    let mut tx_new = 0u32;
    let mut tx_dup = 0u32;
    let mut inc_new = 0u32;
    let mut inc_dup = 0u32;

    // Parse Trades section
    for line in content.lines() {
        if !line.starts_with("Trades,Data,Order,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 15 {
            continue;
        }

        let currency = fields[4].trim();
        let symbol = fields[5].trim();
        let datetime = fields[6].trim().trim_matches('"');
        let quantity: f64 = fields[7].trim().parse().unwrap_or(0.0);
        let trade_price: f64 = fields[8].trim().parse().unwrap_or(0.0);
        let proceeds: f64 = fields[10].trim().parse().unwrap_or(0.0);
        let commission: f64 = fields[11].trim().parse().unwrap_or(0.0);

        if quantity == 0.0 || symbol.is_empty() {
            continue;
        }

        let date_str = datetime.split(',').next().unwrap_or("").trim();
        let date = match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        let tx = investimentos_core::parsers::ibkr_flex::trade_to_transaction(
            symbol, currency, date, quantity, trade_price, proceeds, commission, 1.0,
        );
        match investimentos_core::db::queries::insert_transaction(db, &tx) {
            Ok(true) => tx_new += 1,
            Ok(false) => tx_dup += 1,
            Err(_) => {}
        }
    }

    // Parse Dividends section
    for line in content.lines() {
        if !line.starts_with("Dividends,Data,") || line.starts_with("Dividends,Data,Total") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 6 {
            continue;
        }

        let currency = fields[2].trim();
        let date_str = fields[3].trim();
        let description = fields[4].trim();
        let amount: f64 = fields[5].trim().parse().unwrap_or(0.0);

        let date = match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        let (symbol, _) =
            investimentos_core::parsers::ibkr_flex::parse_dividend_description(description);
        if symbol.is_empty() {
            continue;
        }

        // Find matching withholding tax
        let mut tax_amount = 0.0f64;
        let mut tax_origin = String::new();
        for tax_line in content.lines() {
            if !tax_line.starts_with("Withholding Tax,Data,")
                || tax_line.starts_with("Withholding Tax,Data,Total")
            {
                continue;
            }
            let tf: Vec<&str> = tax_line.split(',').collect();
            if tf.len() < 6 {
                continue;
            }
            if tf[3].trim() == date_str && tf[4].trim().starts_with(&format!("{}(", symbol)) {
                tax_amount += tf[5].trim().parse::<f64>().unwrap_or(0.0);
                let (_, origin) =
                    investimentos_core::parsers::ibkr_flex::parse_tax_description(tf[4].trim());
                if !origin.is_empty() {
                    tax_origin = origin;
                }
            }
        }

        let inc = investimentos_core::parsers::ibkr_flex::dividend_to_income(
            &symbol,
            currency,
            date,
            amount,
            tax_amount,
            &tax_origin,
            1.0,
        );
        match investimentos_core::db::queries::insert_income(db, &inc) {
            Ok(true) => inc_new += 1,
            Ok(false) => inc_dup += 1,
            Err(_) => {}
        }
    }

    Ok(format!(
        "{}: {} trades ({} new), {} income ({} new)",
        filename,
        tx_new + tx_dup,
        tx_new,
        inc_new + inc_dup,
        inc_new,
    ))
}
