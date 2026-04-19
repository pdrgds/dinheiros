use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gpui::{
    div, prelude::*, rgb, AnyElement, ClickEvent, Context, FocusHandle, FontWeight, KeyDownEvent,
    SharedString, Window,
};

use dinheiros_core::db::Database;
use dinheiros_core::export;

use crate::theme;
use crate::views;

// ---------------------------------------------------------------------------
// Background backfill state
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct BackfillStatus {
    pub running: bool,
    pub last_message: Option<String>,
    pub total_backfilled: usize,
}

// ---------------------------------------------------------------------------
// Tab enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Positions,
    Income,
    History,
    Insights,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Overview, Tab::Positions, Tab::Income, Tab::History, Tab::Insights];

    pub fn label(&self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Positions => "Positions",
            Tab::Income => "Income",
            Tab::History => "History",
            Tab::Insights => "Insights",
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
    pub db: Database,
    status_message: Option<String>,
    syncing: bool,
    // Settings editing state
    settings_fields: [String; 1],
    settings_active_field: Option<usize>,
    settings_focus: FocusHandle,
    // Gold form state
    gold_fields: [String; 3], // 0=date, 1=quantity(g), 2=unit_price(BRL/g)
    gold_active_field: Option<usize>,
    gold_focus: FocusHandle,
    // Positions state
    pub positions_sort: views::positions::SortState,
    pub selected_position: Option<usize>,
    pub history_range: views::history::TimeRange,
    pub history_split: bool,
    pub backfill_status: Arc<Mutex<BackfillStatus>>,
    sync_menu_open: bool,
    /// Persistent gpui-component Table state for the Positions tab. Lazy-initialised
    /// on first render so resize/scroll state survives across renders. Wrapped in an
    /// `Entity` as required by `gpui_component::table::TableState`.
    pub positions_table: Option<
        gpui::Entity<
            gpui_component::table::TableState<views::positions::PositionsTableDelegate>,
        >,
    >,
}

impl AppRoot {
    pub fn new(db_path: PathBuf, cx: &mut Context<Self>) -> Self {
        let db = Database::open(&db_path).expect("Failed to open database");
        let settings_focus = cx.focus_handle();
        let gold_focus = cx.focus_handle();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let backfill_status = Arc::new(Mutex::new(BackfillStatus::default()));

        // Start background backfill worker
        Self::start_backfill_worker(db_path.clone(), backfill_status.clone(), cx);

        Self {
            mode: AppMode::Tab(Tab::Overview),
            last_tab: Tab::Overview,
            db,
            status_message: None,
            syncing: false,
            settings_fields: [String::new()],
            settings_active_field: None,
            settings_focus,
            gold_fields: [today, String::new(), String::new()],
            gold_active_field: None,
            gold_focus,
            positions_sort: views::positions::SortState::default(),
            selected_position: None,
            history_range: views::history::TimeRange::default(),
            history_split: false,
            backfill_status,
            sync_menu_open: false,
            positions_table: None,
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
        use dinheiros_core::db::queries;
        self.settings_fields[0] = queries::get_config(&self.db, "coingecko_api_key")
            .ok()
            .flatten()
            .unwrap_or_default();
        self.settings_active_field = None;
    }

    fn save_settings_to_db(&mut self, cx: &mut Context<Self>) {
        use dinheiros_core::db::queries;
        match queries::set_config(&self.db, "coingecko_api_key", &self.settings_fields[0]) {
            Ok(_) => {
                self.status_message = Some("Settings saved.".to_string());
            }
            Err(e) => {
                self.status_message = Some(format!("Save error: {}", e));
            }
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
            "tab" | "enter" => {
                self.settings_active_field = None;
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

    fn handle_gold_key(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(idx) = self.gold_active_field else {
            return;
        };
        let keystroke = &event.keystroke;

        if keystroke.modifiers.platform && keystroke.key == "v" {
            if let Some(item) = cx.read_from_clipboard() {
                if let Some(text) = item.text() {
                    let clean: String = text.lines().next().unwrap_or("").trim().to_string();
                    self.gold_fields[idx] = clean;
                    cx.notify();
                }
            }
            return;
        }
        if keystroke.modifiers.platform || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.gold_fields[idx].pop();
                cx.notify();
            }
            "escape" => {
                self.gold_active_field = None;
                cx.notify();
            }
            "tab" | "enter" => {
                if idx < 2 {
                    self.gold_active_field = Some(idx + 1);
                } else {
                    self.gold_active_field = None;
                }
                cx.notify();
            }
            _ => {
                if let Some(ref ch) = keystroke.key_char {
                    self.gold_fields[idx].push_str(ch);
                    cx.notify();
                }
            }
        }
    }

    fn save_gold(&mut self, cx: &mut Context<Self>) {
        let date_str = self.gold_fields[0].trim();
        let qty_str = self.gold_fields[1].trim();
        let price_str = self.gold_fields[2].trim();

        let date = match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => {
                self.status_message = Some("Invalid date format (use YYYY-MM-DD)".to_string());
                cx.notify();
                return;
            }
        };
        let quantity: f64 = match qty_str.replace(',', ".").parse() {
            Ok(q) if q > 0.0 => q,
            _ => {
                self.status_message = Some("Invalid quantity".to_string());
                cx.notify();
                return;
            }
        };
        let unit_price: f64 = match price_str.replace(',', ".").parse() {
            Ok(p) if p > 0.0 => p,
            _ => {
                self.status_message = Some("Invalid unit price".to_string());
                cx.notify();
                return;
            }
        };

        let total = quantity * unit_price;
        let import_hash = dinheiros_core::hash_string(
            &format!("manual:gold:{}:{}:{}", date, quantity, unit_price),
        );

        let tx = dinheiros_core::Transaction {
            id: None,
            source: dinheiros_core::Source::Manual,
            asset_type: dinheiros_core::AssetType::Gold,
            symbol: "GOLD".to_string(),
            tx_type: dinheiros_core::TxType::Buy,
            date,
            quantity,
            unit_price: Some(unit_price),
            currency: "BRL".to_string(),
            total_value: total,
            brl_rate: 1.0,
            total_brl: total,
            commission: None,
            fee_brl: None,
            notes: Some("manual gold entry".to_string()),
            import_hash,
        };

        match dinheiros_core::db::queries::insert_transaction(&self.db, &tx) {
            Ok(true) => {
                self.status_message =
                    Some(format!("Gold added: {:.4}g at R$ {:.2}/g = R$ {:.2}", quantity, unit_price, total));
                // Reset qty and price fields, keep date
                self.gold_fields[1].clear();
                self.gold_fields[2].clear();
                self.gold_active_field = None;
            }
            Ok(false) => {
                self.status_message = Some("Duplicate entry (already exists)".to_string());
            }
            Err(e) => {
                self.status_message = Some(format!("Error: {}", e));
            }
        }
        cx.notify();
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
            .join("dinheiros")
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
                        match dinheiros_core::parsers::b3::parse_b3_xlsx(path) {
                            Ok(result) => {
                                let mut tx_new = 0u32;
                                let mut inc_new = 0u32;
                                for tx in &result.transactions {
                                    if let Ok(true) = dinheiros_core::db::queries::insert_transaction(&db, tx) {
                                        tx_new += 1;
                                    }
                                }
                                for inc in &result.income {
                                    if let Ok(true) = dinheiros_core::db::queries::insert_income(&db, inc) {
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
                        match dinheiros_core::parsers::binance::parse_binance_csv(path) {
                            Ok(result) => {
                                let mut tx_new = 0u32;
                                for tx in &result.transactions {
                                    if let Ok(true) = dinheiros_core::db::queries::insert_transaction(&db, tx) {
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

    fn start_backfill_worker(
        db_path: PathBuf,
        status: Arc<Mutex<BackfillStatus>>,
        cx: &mut Context<Self>,
    ) {
        let status_writer = status.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Initial delay — let the app render first
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                let mut has_fetched_current = false;

                loop {
                    let db = match Database::open(&db_path) {
                        Ok(db) => db,
                        Err(_) => {
                            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                            continue;
                        }
                    };

                    // Fetch current prices once at startup before backfilling
                    if !has_fetched_current {
                        let mut s = status_writer.lock().unwrap();
                        s.running = true;
                        s.last_message = Some("Fetching current prices...".to_string());
                        drop(s);

                        match dinheiros_core::reconcile::fetch_current_prices(&db).await {
                            Ok(n) => {
                                let mut s = status_writer.lock().unwrap();
                                s.last_message = Some(format!("{} current prices fetched, backfilling...", n));
                            }
                            Err(e) => {
                                eprintln!("[startup] current price fetch failed: {}", e);
                            }
                        }
                        has_fetched_current = true;
                    }

                    {
                        let mut s = status_writer.lock().unwrap();
                        s.running = true;
                        s.last_message = Some("Backfilling...".to_string());
                    }

                    match dinheiros_core::reconcile::backfill_prices(&db).await {
                        Ok(r) => {
                            let mut s = status_writer.lock().unwrap();
                            s.total_backfilled += r.prices_backfilled;

                            // Done when no prices were backfilled AND not rate-limited
                            // (all symbols are either up-to-date or permanently failed)
                            if r.prices_backfilled == 0 && !r.rate_limited {
                                s.running = false;
                                if r.symbols_failed > 0 {
                                    s.last_message = Some(format!(
                                        "Backfill done ({} prices, {} symbols failed)",
                                        s.total_backfilled, r.symbols_failed
                                    ));
                                } else {
                                    s.last_message = Some(format!(
                                        "Backfill complete ({} prices)",
                                        s.total_backfilled
                                    ));
                                }
                                break;
                            }

                            if r.rate_limited {
                                s.last_message = Some(format!(
                                    "Backfilling... {} prices ({} up to date, {} pending) — rate limited, waiting 60s",
                                    s.total_backfilled, r.symbols_up_to_date,
                                    r.symbols_up_to_date.max(1) - 1 // rough pending count
                                ));
                            } else {
                                s.last_message = Some(format!(
                                    "Backfilling... +{} prices this batch ({} total)",
                                    r.prices_backfilled, s.total_backfilled
                                ));
                            }
                        }
                        Err(e) => {
                            let mut s = status_writer.lock().unwrap();
                            s.last_message = Some(format!("Backfill error: {}", e));
                        }
                    }

                    // Wait between batches (longer if rate-limited)
                    let wait = {
                        let s = status_writer.lock().unwrap();
                        if s.last_message.as_ref().map_or(false, |m| m.contains("rate limited")) {
                            60
                        } else {
                            5
                        }
                    };
                    tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                }
            });
        });

        // Poll backfill status to update UI periodically
        let status_reader = status;
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            loop {
                cx.background_executor().timer(std::time::Duration::from_secs(5)).await;
                let still_running = {
                    let s = status_reader.lock().unwrap();
                    s.running
                };
                // Trigger UI refresh so history tab picks up new data
                let _ = this.update(cx, |_this, cx: &mut Context<Self>| {
                    cx.notify();
                });
                if !still_running {
                    break;
                }
            }
        }).detach();
    }

    fn do_full_resync(&mut self, cx: &mut Context<Self>) {
        // Clear all cached prices so everything gets re-fetched
        let _ = self.db.conn().execute("DELETE FROM daily_prices", []);
        self.status_message = Some("Cleared price cache, resyncing...".to_string());
        cx.notify();
        self.do_sync(cx);
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
            .join("dinheiros")
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

                match dinheiros_core::reconcile::fetch_current_prices(&db).await {
                    Ok(n) => format!("{n} current prices fetched"),
                    Err(e) => format!("Price fetch error: {e}"),
                }
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
                cx.background_executor().timer(std::time::Duration::from_millis(200)).await;
            }
        }).detach();
    }

    fn do_export(&mut self, cx: &mut Context<Self>) {
        let default_dir = dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let suggested_name = format!(
            "dinheiros-export-{}.json",
            chrono::Local::now().format("%Y-%m-%dT%H-%M-%S")
        );
        let rx = cx.prompt_for_new_path(&default_dir, Some(&suggested_name));

        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let path = match rx.await {
                Ok(Ok(Some(p))) => p,
                _ => return,
            };

            let _ = this.update(cx, |this, cx: &mut Context<Self>| {
                match export::export_to_json(&this.db, &path) {
                    Ok(()) => {
                        this.status_message = Some(format!("Exported to {}", path.display()));
                        println!("[export] success: {}", path.display());
                    }
                    Err(e) => {
                        this.status_message = Some(format!("Export failed: {}", e));
                        eprintln!("[export] error: {}", e);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for AppRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .child(self.render_content(window, cx));

        // Status bar at the bottom
        let backfill_msg = self.backfill_status.lock().unwrap().last_message.clone();
        let status_text = match (&self.status_message, &backfill_msg) {
            (Some(msg), Some(bf)) => Some(format!("{} | {}", msg, bf)),
            (Some(msg), None) => Some(msg.clone()),
            (None, Some(bf)) => Some(bf.clone()),
            (None, None) => None,
        };
        if let Some(msg) = status_text {
            root = root.child(
                div()
                    .px_4()
                    .py_1()
                    .bg(theme::BG_SECONDARY)
                    .border_t_1()
                    .border_color(theme::BORDER)
                    .text_xs()
                    .text_color(theme::TEXT_SECONDARY)
                    .child(msg),
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
                let menu_open = self.sync_menu_open;

                div()
                    .relative()
                    .flex()
                    .flex_row()
                    .child(
                        // Main sync button
                        div()
                            .id("sync-btn")
                            .px_3()
                            .py_1()
                            .rounded_l_md()
                            .bg(sync_color)
                            .text_color(rgb(0xffffff))
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .cursor_pointer()
                            .hover(|s| s.opacity(0.85))
                            .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                                this.sync_menu_open = false;
                                this.do_sync(cx);
                            }))
                            .child(sync_label),
                    )
                    .child(
                        // Dropdown arrow
                        div()
                            .id("sync-menu-btn")
                            .px_1()
                            .py_1()
                            .rounded_r_md()
                            .bg(sync_color)
                            .text_color(rgb(0xffffff))
                            .text_sm()
                            .cursor_pointer()
                            .border_l_1()
                            .border_color(rgb(0xffffff))
                            .hover(|s| s.opacity(0.85))
                            .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                                this.sync_menu_open = !this.sync_menu_open;
                                cx.notify();
                            }))
                            .child("\u{25BC}"),
                    )
                    .when(menu_open, |el| {
                        el.child(
                            div()
                                .absolute()
                                .top(gpui::px(30.0))
                                .right_0()
                                .rounded_md()
                                .bg(theme::BG_SECONDARY)
                                .border_1()
                                .border_color(theme::BORDER)
                                .shadow_md()
                                .child(
                                    div()
                                        .id("full-resync-btn")
                                        .px_3()
                                        .py_2()
                                        .text_sm()
                                        .whitespace_nowrap()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme::BORDER))
                                        .rounded_md()
                                        .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                                            this.sync_menu_open = false;
                                            this.do_full_resync(cx);
                                        }))
                                        .child("Full Resync"),
                                ),
                        )
                    })
            })
            .child(
                action_button("settings-btn", "Settings", theme::BORDER)
                    .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                        this.set_mode(AppMode::Settings, cx);
                    })),
            )
    }

    // ----- content area -----
    fn render_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        match self.mode {
            AppMode::Tab(Tab::Overview) => views::overview::render_overview(&self.db),
            AppMode::Tab(Tab::Positions) => views::positions::render_positions(self, window, cx),
            AppMode::Tab(Tab::Income) => views::income::render_income(&self.db),
            AppMode::Tab(Tab::History) => views::history::render_history(&self.db, self.history_range, self.history_split, cx),
            AppMode::Tab(Tab::Insights) => views::insights::render_insights(&self.db),
            AppMode::Import => self.render_import_mode(cx),
            AppMode::ManualGold => self.render_gold_mode(cx),
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
        let labels = ["CoinGecko API Key"];

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
                        this.settings_focus.focus(window, cx);
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

    // ----- Gold entry mode -----
    fn render_gold_mode(&self, cx: &mut Context<Self>) -> AnyElement {
        let labels = ["Date (YYYY-MM-DD)", "Quantity (grams)", "Unit Price (BRL/g)"];

        let mut content = div()
            .id("gold-panel")
            .track_focus(&self.gold_focus)
            .flex()
            .flex_col()
            .gap_6()
            .p_6()
            .w_full()
            .flex_1()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                this.handle_gold_key(ev, window, cx);
            }));

        content = content.child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Add Gold Purchase"),
        );

        let mut panel = div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .rounded_lg()
            .bg(theme::BG_SECONDARY)
            .border_1()
            .border_color(theme::BORDER);

        for (i, label) in labels.iter().enumerate() {
            let value = self.gold_fields[i].clone();
            let is_active = self.gold_active_field == Some(i);
            panel = panel.child(self.render_gold_field(i, label, &value, is_active, cx));
        }

        content = content.child(panel);

        // Save button
        content = content.child(
            div().flex().flex_row().gap_2().child(
                div()
                    .id("save-gold-btn")
                    .px_4()
                    .py_2()
                    .rounded_md()
                    .bg(theme::YELLOW)
                    .text_color(rgb(0x000000))
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.85))
                    .on_click(cx.listener(|this, _ev: &ClickEvent, _window, cx| {
                        this.save_gold(cx);
                    }))
                    .child("Add Gold"),
            ),
        );

        content = content.child(
            div()
                .text_sm()
                .text_color(theme::TEXT_SECONDARY)
                .child("Click a field to edit. Tab/Enter to move to next field."),
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

    fn render_gold_field(
        &self,
        index: usize,
        label: &str,
        value: &str,
        is_active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let display_value = if value.is_empty() && !is_active {
            "(empty)".to_string()
        } else {
            value.to_string()
        };

        let border_col = if is_active { theme::YELLOW } else { theme::BORDER };
        let field_bg = if is_active { rgb(0x0d0d1a) } else { theme::BG_PRIMARY };
        let text_col = if value.is_empty() && !is_active {
            theme::TEXT_SECONDARY
        } else {
            theme::TEXT_PRIMARY
        };

        let id = SharedString::from(format!("gold-field-{}", index));

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
                        this.gold_active_field = Some(index);
                        this.gold_focus.focus(window, cx);
                        cx.notify();
                    }))
                    .child(if is_active && display_value == "(empty)" {
                        "\u{258F}".to_string()
                    } else if is_active {
                        format!("{}\u{258F}", value)
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

/// Intermediate struct for a parsed but not-yet-inserted IBKR trade.
struct ParsedTrade {
    symbol: String,
    currency: String,
    date: chrono::NaiveDate,
    quantity: f64,
    trade_price: f64,
    proceeds: f64,
    commission: f64,
}

/// Intermediate struct for a parsed but not-yet-inserted IBKR dividend.
struct ParsedDividend {
    symbol: String,
    currency: String,
    date: chrono::NaiveDate,
    gross_amount: f64,
    tax_amount: f64,
    tax_origin: String,
}

/// Parse IBKR CSV activity statement, fetch BCB PTAX rates, and insert into DB.
fn import_ibkr_csv(
    db: &Database,
    path: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let filename = path.file_name().unwrap_or_default().to_string_lossy();

    // Phase 1: Parse CSV into intermediate structs
    let mut trades = Vec::new();
    let mut dividends = Vec::new();

    for line in content.lines() {
        if !line.starts_with("Trades,Data,Order,") {
            continue;
        }
        let fields = split_csv_line(line);
        if fields.len() < 15 {
            continue;
        }

        let currency = fields[4].trim().to_string();
        let symbol = fields[5].trim().to_string();
        let datetime = fields[6].trim().trim_matches('"');
        let quantity: f64 = parse_ibkr_number(&fields[7]);
        let trade_price: f64 = parse_ibkr_number(&fields[8]);
        let proceeds: f64 = parse_ibkr_number(&fields[10]);
        let commission: f64 = parse_ibkr_number(&fields[11]);

        if quantity == 0.0 || symbol.is_empty() || symbol.contains('.') {
            continue;
        }

        let date_str = &datetime[..datetime.len().min(10)];
        let date = match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        trades.push(ParsedTrade {
            symbol, currency, date, quantity, trade_price, proceeds, commission,
        });
    }

    for line in content.lines() {
        if !line.starts_with("Dividends,Data,") || line.starts_with("Dividends,Data,Total") {
            continue;
        }
        let fields = split_csv_line(line);
        if fields.len() < 6 {
            continue;
        }

        let currency = fields[2].trim().to_string();
        let date_str = fields[3].trim();
        let description = fields[4].trim();
        let amount: f64 = parse_ibkr_number(&fields[5]);

        let date = match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        let (symbol, _) =
            dinheiros_core::parsers::ibkr_flex::parse_dividend_description(description);
        if symbol.is_empty() {
            continue;
        }

        let mut tax_amount = 0.0f64;
        let mut tax_origin = String::new();
        for tax_line in content.lines() {
            if !tax_line.starts_with("Withholding Tax,Data,")
                || tax_line.starts_with("Withholding Tax,Data,Total")
            {
                continue;
            }
            let tf = split_csv_line(tax_line);
            if tf.len() < 6 {
                continue;
            }
            if tf[3].trim() == date_str && tf[4].trim().starts_with(&format!("{}(", symbol)) {
                tax_amount += parse_ibkr_number(&tf[5]);
                let (_, origin) =
                    dinheiros_core::parsers::ibkr_flex::parse_tax_description(tf[4].trim());
                if !origin.is_empty() {
                    tax_origin = origin;
                }
            }
        }

        dividends.push(ParsedDividend {
            symbol, currency, date, gross_amount: amount, tax_amount, tax_origin,
        });
    }

    // Parse Corporate Actions section (splits, mergers, delistings, symbol changes)
    for line in content.lines() {
        if !line.starts_with("Corporate Actions,Data,")
            || line.starts_with("Corporate Actions,Data,Total")
        {
            continue;
        }
        let fields = split_csv_line(line);
        // Fields: 0=section, 1="Data", 2=asset_category, 3=currency, 4=report_date,
        //         5=date_time, 6=description, 7=quantity, 8=proceeds, 9=value, 10=realized_pnl
        if fields.len() < 8 {
            continue;
        }

        let currency = fields[3].trim().to_string();
        let date_str = fields[4].trim();
        let description = fields[6].trim();
        let quantity: f64 = parse_ibkr_number(&fields[7]);
        let proceeds: f64 = if fields.len() > 8 {
            parse_ibkr_number(&fields[8])
        } else {
            0.0
        };

        if quantity == 0.0 {
            continue;
        }

        let date = match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Extract the target symbol from description.
        // Format: "SYMBOL(ISIN) Action... (NEW_SYMBOL, FULL NAME, NEW_ISIN)"
        // For lines with ".OLD" in the parenthetical, the symbol is being removed.
        // For lines without ".OLD", the symbol is being added.
        // We need the symbol from the last parenthetical group.
        let symbol = if let Some(last_paren_start) = description.rfind('(') {
            let inner = &description[last_paren_start + 1..];
            if let Some(comma_pos) = inner.find(',') {
                let sym = inner[..comma_pos].trim();
                // Skip .OLD entries — they remove shares from the old symbol
                if sym.ends_with(".OLD") {
                    sym.trim_end_matches(".OLD").to_uppercase()
                } else {
                    sym.to_uppercase()
                }
            } else {
                continue;
            }
        } else {
            continue;
        };

        if symbol.is_empty() {
            continue;
        }

        // For mergers/acquisitions with proceeds, use proceeds as total value.
        // For splits, proceeds is 0 — quantity adjustment only.
        let unit_price = if quantity.abs() > 0.0001 {
            proceeds.abs() / quantity.abs()
        } else {
            0.0
        };

        trades.push(ParsedTrade {
            symbol,
            currency,
            date,
            quantity,
            trade_price: unit_price,
            proceeds: proceeds.abs(),
            commission: 0.0,
        });
    }

    // Parse Financial Instrument Information to get exchange mappings
    for line in content.lines() {
        if !line.starts_with("Financial Instrument Information,Data,") {
            continue;
        }
        let fields = split_csv_line(line);
        // Fields: 0=section, 1="Data", 2=asset_category, 3=symbol, 4=name, 5=con_id,
        //         6=isin, 7=listing_exchange_symbol, 8=exchange, 9=multiplier, 10=type
        if fields.len() < 9 {
            continue;
        }
        let symbol = fields[3].trim();
        let exchange = fields[8].trim();
        if !symbol.is_empty() && !exchange.is_empty() {
            let key = format!("exchange:{}", symbol);
            let _ = dinheiros_core::db::queries::set_config(db, &key, exchange);
        }
    }

    // Phase 2: Collect unique (currency, date) pairs and fetch PTAX rates
    let mut rate_keys: std::collections::HashSet<(String, chrono::NaiveDate)> =
        std::collections::HashSet::new();
    for t in &trades {
        if t.currency != "BRL" {
            rate_keys.insert((t.currency.clone(), t.date));
        }
    }
    for d in &dividends {
        if d.currency != "BRL" {
            rate_keys.insert((d.currency.clone(), d.date));
        }
    }

    let rates = fetch_ptax_rates_blocking(&rate_keys);

    // Phase 3: Create transactions/income with real BRL rates and insert
    let mut tx_new = 0u32;
    let mut tx_dup = 0u32;
    let mut inc_new = 0u32;
    let mut inc_dup = 0u32;
    let mut rate_misses = 0u32;

    for t in &trades {
        let brl_rate = if t.currency == "BRL" {
            1.0
        } else {
            match rates.get(&(t.currency.clone(), t.date)) {
                Some(&r) => r,
                None => {
                    rate_misses += 1;
                    continue;
                }
            }
        };

        let tx = dinheiros_core::parsers::ibkr_flex::trade_to_transaction(
            &t.symbol, &t.currency, t.date, t.quantity, t.trade_price,
            t.proceeds, t.commission, brl_rate,
        );
        match dinheiros_core::db::queries::insert_transaction(db, &tx) {
            Ok(true) => tx_new += 1,
            Ok(false) => tx_dup += 1,
            Err(_) => {}
        }
    }

    for d in &dividends {
        let brl_rate = if d.currency == "BRL" {
            1.0
        } else {
            match rates.get(&(d.currency.clone(), d.date)) {
                Some(&r) => r,
                None => {
                    rate_misses += 1;
                    continue;
                }
            }
        };

        let inc = dinheiros_core::parsers::ibkr_flex::dividend_to_income(
            &d.symbol, &d.currency, d.date, d.gross_amount,
            d.tax_amount, &d.tax_origin, brl_rate,
        );
        match dinheiros_core::db::queries::insert_income(db, &inc) {
            Ok(true) => inc_new += 1,
            Ok(false) => inc_dup += 1,
            Err(_) => {}
        }
    }

    let mut msg = format!(
        "{}: {} trades ({} new), {} income ({} new)",
        filename,
        tx_new + tx_dup,
        tx_new,
        inc_new + inc_dup,
        inc_new,
    );
    if rate_misses > 0 {
        msg.push_str(&format!(", {} skipped (no PTAX rate)", rate_misses));
    }
    Ok(msg)
}

/// Fetch BCB PTAX rates for a set of (currency, date) pairs.
/// Groups by currency and uses range fetch when possible, falls back to individual fetches.
fn fetch_ptax_rates_blocking(
    keys: &std::collections::HashSet<(String, chrono::NaiveDate)>,
) -> std::collections::HashMap<(String, chrono::NaiveDate), f64> {
    use std::collections::HashMap;

    if keys.is_empty() {
        return HashMap::new();
    }

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("[import] failed to create tokio runtime: {e}");
            return HashMap::new();
        }
    };

    rt.block_on(async {
        let mut results: HashMap<(String, chrono::NaiveDate), f64> = HashMap::new();

        // Group dates by currency
        let mut by_currency: HashMap<String, Vec<chrono::NaiveDate>> = HashMap::new();
        for (currency, date) in keys {
            by_currency
                .entry(currency.clone())
                .or_default()
                .push(*date);
        }

        for (currency, dates) in &by_currency {
            let min_date = *dates.iter().min().unwrap();
            let max_date = *dates.iter().max().unwrap();

            // Try range fetch first (one API call for all dates of this currency)
            match dinheiros_core::api::bcb_ptax::fetch_rates_range(currency, min_date, max_date).await {
                Ok(range_rates) => {
                    let rate_map: HashMap<chrono::NaiveDate, f64> =
                        range_rates.into_iter().collect();

                    for &date in dates {
                        // Find exact or closest rate (weekends/holidays)
                        if let Some(rate) = find_closest_rate(&rate_map, date) {
                            results.insert((currency.clone(), date), rate);
                        } else {
                            eprintln!("[import] no PTAX rate for {} on {}", currency, date);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[import] range PTAX fetch failed for {}: {}, trying individual", currency, e);
                    // Fallback: fetch individually
                    for &date in dates {
                        match dinheiros_core::api::bcb_ptax::fetch_rate(currency, date).await {
                            Ok(rate) => {
                                results.insert((currency.clone(), date), rate);
                            }
                            Err(e) => {
                                eprintln!("[import] PTAX fetch failed for {} on {}: {}", currency, date, e);
                            }
                        }
                    }
                }
            }
        }

        results
    })
}

/// Find the closest PTAX rate for a target date, looking back up to 5 days.
fn find_closest_rate(
    rate_map: &std::collections::HashMap<chrono::NaiveDate, f64>,
    target: chrono::NaiveDate,
) -> Option<f64> {
    for days_back in 0..6 {
        if let Some(&rate) = rate_map.get(&(target - chrono::Duration::days(days_back))) {
            return Some(rate);
        }
    }
    None
}

/// Parse a number from IBKR CSV, stripping thousand-separator commas.
/// Handles values like "1,000", "-3,595", "0.829780693".
fn parse_ibkr_number(s: &str) -> f64 {
    s.trim().replace(',', "").parse().unwrap_or(0.0)
}

/// Split a CSV line respecting quoted fields (handles commas inside quotes).
fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(current.clone());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current);
    fields
}
