mod app;
#[allow(dead_code)]
mod theme;
mod views;

use gpui::{
    prelude::*, point, px, size, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};
use dinheiros_core::db::{queries, Database};

const WINDOW_BOUNDS_CONFIG_KEY: &str = "window_bounds";

/// Serialize a window bounds as "x,y,w,h" in float pixels, so we can stash it
/// in the existing key/value `config` table alongside other preferences.
fn format_bounds(b: Bounds<gpui::Pixels>) -> String {
    format!(
        "{},{},{},{}",
        f32::from(b.origin.x),
        f32::from(b.origin.y),
        f32::from(b.size.width),
        f32::from(b.size.height),
    )
}

fn parse_bounds(s: &str) -> Option<Bounds<gpui::Pixels>> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return None;
    }
    let x: f32 = parts[0].parse().ok()?;
    let y: f32 = parts[1].parse().ok()?;
    let w: f32 = parts[2].parse().ok()?;
    let h: f32 = parts[3].parse().ok()?;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    Some(Bounds { origin: point(px(x), px(y)), size: size(px(w), px(h)) })
}

fn main() {
    gpui_platform::application()
        .with_assets(gpui_component_assets::Assets)
        .run(|cx: &mut gpui::App| {
        gpui_component::init(cx);
        gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);
        // Flatten the Table surface onto the app's BG_PRIMARY (our dark navy)
        // so the positions grid reads as part of the window, not as a pure-black
        // card floating on navy. gpui-component's default dark `background` is
        // pitch black, so we override every table-related colour to our navy.
        {
            let theme = gpui_component::Theme::global_mut(cx);
            let bg: gpui::Hsla = theme::BG_PRIMARY.into();
            theme.colors.background = bg;
            theme.colors.table = bg;
            theme.colors.table_even = bg;
            theme.colors.table_head = bg;
        }
        let db_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("dinheiros")
            .join("data.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        // Restore the last-known window bounds from the config table if present,
        // otherwise fall back to a centered 1050x850.
        let saved_bounds: Option<Bounds<gpui::Pixels>> = Database::open(&db_path)
            .ok()
            .and_then(|db| queries::get_config(&db, WINDOW_BOUNDS_CONFIG_KEY).ok().flatten())
            .and_then(|s| parse_bounds(&s));
        let bounds = saved_bounds
            .unwrap_or_else(|| Bounds::centered(None, size(px(1050.), px(850.)), cx));

        let db_path_for_close = db_path.clone();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Investimentos v2".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |window, cx| {
                // Cmd+W path: fires when the user explicitly closes the window.
                let db_path = db_path_for_close.clone();
                window.on_window_should_close(cx, move |window, _cx| {
                    if let WindowBounds::Windowed(rect) = window.window_bounds() {
                        if let Ok(db) = Database::open(&db_path) {
                            let _ = queries::set_config(
                                &db,
                                WINDOW_BOUNDS_CONFIG_KEY,
                                &format_bounds(rect),
                            );
                        }
                    }
                    true
                });
                cx.new(|cx| app::AppRoot::new(db_path_for_close.clone(), cx))
            },
        )
        .unwrap();

        // Cmd+Q / app-quit path: the window-close hook above isn't invoked when
        // macOS quits the app outright. Register an app-quit observer that
        // walks the open windows and writes each's bounds before shutdown.
        let db_path_for_quit = db_path.clone();
        cx.on_app_quit(move |cx| {
            for handle in cx.windows() {
                let _ = handle.update(cx, |_, window, _| {
                    if let WindowBounds::Windowed(rect) = window.window_bounds() {
                        if let Ok(db) = Database::open(&db_path_for_quit) {
                            let _ = queries::set_config(
                                &db,
                                WINDOW_BOUNDS_CONFIG_KEY,
                                &format_bounds(rect),
                            );
                        }
                    }
                });
            }
            async {}
        })
        .detach();

        cx.activate(true);
    });
}
