use std::path::PathBuf;

use dinheiros_core::db::Database;
use dinheiros_core::seed::seed_demo;

/// The production DB location, computed without honoring DINHEIROS_DB_PATH.
/// We compare the seed target against this (not against `default_db_path()`,
/// which honors the env var and could be tricked into agreeing with a
/// CLI-arg target that points at the real DB).
fn production_db_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dinheiros")
        .join("data.db")
}

fn main() {
    let path = resolve_path();

    let real = production_db_path();
    let canonical_match = std::fs::canonicalize(&path)
        .ok()
        .zip(std::fs::canonicalize(&real).ok())
        .map(|(a, b)| a == b)
        .unwrap_or(false);
    if path == real || canonical_match {
        panic!(
            "refusing to seed onto the production user DB path ({}); \
             pass a different path as the first CLI arg",
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
    let summary = seed_demo(&db).unwrap_or_else(|e| panic!("seed failed: {}", e));

    println!(
        "seeded {}: {} transactions, {} income, {} daily_prices",
        path.display(),
        summary.transactions,
        summary.income,
        summary.daily_prices,
    );
}

// Path resolution: CLI arg → DINHEIROS_DB_PATH → /tmp fallback. The main()
// guard prevents this from ever resolving to the production user DB path.
fn resolve_path() -> PathBuf {
    if let Some(arg) = std::env::args().nth(1) {
        return PathBuf::from(arg);
    }
    if let Ok(env) = std::env::var("DINHEIROS_DB_PATH") {
        return PathBuf::from(env);
    }
    PathBuf::from("/tmp/dinheiros-demo.db")
}
