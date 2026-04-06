use std::collections::HashMap;
use std::fs;
use std::path::Path;

use chrono::{NaiveDate, Utc};
use rusqlite::params;

use crate::db::queries;
use crate::db::Database;
use crate::types::ExportData;

const KNOWN_CONFIG_KEYS: &[&str] = &[
    "ibkr_flex_token",
    "ibkr_flex_query_id",
    "coingecko_api_key",
];

const PRICE_DATE_MIN: &str = "2000-01-01";
const PRICE_DATE_MAX: &str = "2099-12-31";

/// Export all data from the database to a pretty-printed JSON file.
pub fn export_to_json(db: &Database, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let transactions = queries::get_all_transactions(db)?;
    let income = queries::get_all_income(db)?;

    // Collect all distinct symbols from daily_prices
    let symbols = get_distinct_price_symbols(db)?;

    let from = NaiveDate::parse_from_str(PRICE_DATE_MIN, "%Y-%m-%d").unwrap();
    let to = NaiveDate::parse_from_str(PRICE_DATE_MAX, "%Y-%m-%d").unwrap();

    let mut daily_prices = Vec::new();
    for symbol in &symbols {
        let prices = queries::get_daily_prices(db, symbol, from, to)?;
        daily_prices.extend(prices);
    }

    // Collect known config values
    let mut config = HashMap::new();
    for key in KNOWN_CONFIG_KEYS {
        if let Some(value) = queries::get_config(db, key)? {
            config.insert(key.to_string(), value);
        }
    }

    let data = ExportData {
        version: 1,
        exported_at: Utc::now().to_rfc3339(),
        transactions,
        daily_prices,
        income,
        config,
    };

    let json = serde_json::to_string_pretty(&data)?;
    fs::write(path, json)?;

    Ok(())
}

/// Import data from a JSON file into the database.
pub fn import_from_json(db: &Database, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let json = fs::read_to_string(path)?;
    let data: ExportData = serde_json::from_str(&json)?;

    for tx in &data.transactions {
        queries::insert_transaction(db, tx)?;
    }

    for inc in &data.income {
        queries::insert_income(db, inc)?;
    }

    for price in &data.daily_prices {
        queries::upsert_daily_price(db, price)?;
    }

    for (key, value) in &data.config {
        queries::set_config(db, key, value)?;
    }

    Ok(())
}

/// Get all distinct symbols from the daily_prices table.
fn get_distinct_price_symbols(db: &Database) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut stmt = db
        .conn()
        .prepare("SELECT DISTINCT symbol FROM daily_prices ORDER BY symbol")?;
    let rows = stmt.query_map(params![], |row| row.get::<_, String>(0))?;
    let symbols: Vec<String> = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(symbols)
}
