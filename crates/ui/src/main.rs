mod app;
#[allow(dead_code)]
mod theme;
mod views;

use gpui::{
    prelude::*, px, size, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};

fn main() {
    Application::new().run(|cx: &mut gpui::App| {
        let db_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("investimentos-v2")
            .join("data.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let bounds = Bounds::maximized(None, cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Maximized(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Investimentos v2".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_window, cx| cx.new(|cx| app::AppRoot::new(db_path, cx)),
        )
        .unwrap();

        cx.activate(true);
    });
}
