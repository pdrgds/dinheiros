use std::path::PathBuf;

use dinheiros_core::db::Database;
use dinheiros_core::maintenance;

fn main() {
    let db_path: PathBuf = match std::env::args().nth(1) {
        Some(p) => PathBuf::from(p),
        None => {
            // macOS default: ~/Library/Application Support/dinheiros/data.db
            let home = std::env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join("Library/Application Support/dinheiros/data.db")
        }
    };

    println!("Opening DB at {}", db_path.display());
    let db = Database::open(&db_path).expect("failed to open database");

    let deleted = maintenance::cleanup_non_trading_day_prices(&db).expect("cleanup failed");

    println!("Deleted {deleted} non-trading-day rows from daily_prices.");
}
