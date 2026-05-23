pub mod data;

use rusqlite::Result;

use crate::db::{queries, Database};

/// Count of items written to the demo DB.
///
/// `transactions` and `income` count rows actually inserted
/// (`INSERT OR IGNORE` underneath may silently skip duplicate `import_hash`).
/// `daily_prices` counts rows attempted, since `upsert_daily_price` uses
/// `INSERT OR REPLACE` and always succeeds. For a freshly-nuked demo DB the
/// two semantics coincide.
pub struct SeedSummary {
    pub transactions: usize,
    pub income: usize,
    pub daily_prices: usize,
}

/// Populate `db` with the demo dataset. Assumes the DB has already been
/// migrated (Database::open does this).
pub fn seed_demo(db: &Database) -> Result<SeedSummary> {
    let mut tx_count = 0;
    for tx in data::all_transactions() {
        if queries::insert_transaction(db, &tx)? {
            tx_count += 1;
        }
    }

    let mut income_count = 0;
    for inc in data::all_income() {
        if queries::insert_income(db, &inc)? {
            income_count += 1;
        }
    }

    let mut price_count = 0;
    for price in data::latest_prices() {
        queries::upsert_daily_price(db, &price)?;
        price_count += 1;
    }

    Ok(SeedSummary {
        transactions: tx_count,
        income: income_count,
        daily_prices: price_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_runs_on_in_memory_db_without_error() {
        let db = Database::open_in_memory().expect("open in-memory DB");
        let summary = seed_demo(&db).expect("seed succeeds");
        assert_eq!(summary.transactions, data::all_transactions().len());
        assert_eq!(summary.income, data::all_income().len());
        assert_eq!(summary.daily_prices, data::latest_prices().len());
    }
}
