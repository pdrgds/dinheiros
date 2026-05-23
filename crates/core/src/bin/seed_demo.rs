use std::path::PathBuf;

use dinheiros_core::db::{default_db_path, Database};
use dinheiros_core::seed::seed_demo;

fn main() {
    let path = resolve_path();

    if path == default_db_path() {
        panic!(
            "refusing to seed onto the default user DB path ({}); \
             pass a different path as the first CLI arg or unset DINHEIROS_DB_PATH",
            path.display()
        );
    }

    if path.exists() {
        std::fs::remove_file(&path)
            .unwrap_or_else(|e| panic!("failed to remove existing {}: {}", path.display(), e));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let db = Database::open(&path)
        .unwrap_or_else(|e| panic!("failed to open {}: {}", path.display(), e));
    let summary = seed_demo(&db)
        .unwrap_or_else(|e| panic!("seed failed: {}", e));

    println!(
        "seeded {}: {} transactions, {} income, {} daily_prices",
        path.display(),
        summary.transactions,
        summary.income,
        summary.daily_prices,
    );
}

// Path resolution: CLI arg → DINHEIROS_DB_PATH → /tmp fallback. The main()
// guard prevents this from ever resolving to the real user DB path.
fn resolve_path() -> PathBuf {
    if let Some(arg) = std::env::args().nth(1) {
        return PathBuf::from(arg);
    }
    if let Ok(env) = std::env::var("DINHEIROS_DB_PATH") {
        return PathBuf::from(env);
    }
    PathBuf::from("/tmp/dinheiros-demo.db")
}
