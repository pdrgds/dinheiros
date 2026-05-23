use chrono::NaiveDate;

use crate::calendar;
use crate::db::Database;

/// One-off cleanup: delete rows in `daily_prices` whose date is not a trading day
/// for the symbol's asset_type. These rows are written by an older version of
/// `fetch_current_prices` that upserted "today" even on weekends/holidays.
///
/// Returns the number of rows deleted. Safe to run repeatedly (idempotent).
pub fn cleanup_non_trading_day_prices(db: &Database) -> Result<usize, Box<dyn std::error::Error>> {
    let symbol_asset_types: Vec<(String, String)> = {
        let mut stmt = db
            .conn()
            .prepare("SELECT DISTINCT symbol, asset_type FROM transactions")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let mut deleted = 0usize;
    for (symbol, asset_type) in &symbol_asset_types {
        let dates: Vec<NaiveDate> = {
            let mut stmt = db
                .conn()
                .prepare("SELECT date FROM daily_prices WHERE symbol = ?")?;
            let rows = stmt.query_map([symbol], |row| row.get::<_, String>(0))?;
            rows.filter_map(|r| r.ok())
                .filter_map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
                .collect()
        };

        for date in dates {
            if !calendar::is_trading_day(date, asset_type) {
                let n = db.conn().execute(
                    "DELETE FROM daily_prices WHERE symbol = ? AND date = ?",
                    rusqlite::params![symbol, date.format("%Y-%m-%d").to_string()],
                )?;
                deleted += n;
            }
        }
    }

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries;
    use crate::types::{AssetType, DailyPrice, Source, Transaction, TxType};

    fn mk_tx(symbol: &str, asset_type: AssetType) -> Transaction {
        Transaction {
            id: None,
            source: Source::Manual,
            asset_type,
            symbol: symbol.to_string(),
            tx_type: TxType::Buy,
            date: NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
            quantity: 1.0,
            unit_price: Some(1.0),
            currency: "BRL".to_string(),
            total_value: 1.0,
            brl_rate: 1.0,
            total_brl: 1.0,
            commission: None,
            fee_brl: None,
            notes: None,
            import_hash: format!("{}-tx", symbol),
        }
    }

    fn mk_price(symbol: &str, date: NaiveDate) -> DailyPrice {
        DailyPrice {
            symbol: symbol.to_string(),
            date,
            close_price: 1.0,
            currency: "BRL".to_string(),
            brl_rate: 1.0,
        }
    }

    #[test]
    fn deletes_weekend_rows_for_stocks_only() {
        let db = Database::open_in_memory().unwrap();

        queries::insert_transaction(&db, &mk_tx("KULR", AssetType::StockIntl)).unwrap();
        queries::insert_transaction(&db, &mk_tx("BTC", AssetType::Crypto)).unwrap();

        // KULR: Wed (trading), Sat (bogus), Sun (bogus), Mon (trading), Good Friday (bogus holiday)
        for d in [
            NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 4, 4).unwrap(),
            NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(),
            NaiveDate::from_ymd_opt(2026, 4, 6).unwrap(),
            NaiveDate::from_ymd_opt(2026, 4, 3).unwrap(), // Good Friday 2026
        ] {
            queries::upsert_daily_price(&db, &mk_price("KULR", d)).unwrap();
        }

        // BTC: Sat + Sun should remain (crypto trades every day)
        for d in [
            NaiveDate::from_ymd_opt(2026, 4, 4).unwrap(),
            NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(),
        ] {
            queries::upsert_daily_price(&db, &mk_price("BTC", d)).unwrap();
        }

        let deleted = cleanup_non_trading_day_prices(&db).unwrap();
        assert_eq!(deleted, 3, "should delete Sat, Sun, Good Friday for KULR");

        let kulr_dates: Vec<NaiveDate> = {
            let mut stmt = db
                .conn()
                .prepare("SELECT date FROM daily_prices WHERE symbol = 'KULR' ORDER BY date")
                .unwrap();
            stmt.query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .filter_map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
                .collect()
        };
        assert_eq!(
            kulr_dates,
            vec![
                NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 4, 6).unwrap(),
            ]
        );

        let btc_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM daily_prices WHERE symbol = 'BTC'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(btc_count, 2, "crypto rows must not be touched");
    }

    #[test]
    fn idempotent_when_clean() {
        let db = Database::open_in_memory().unwrap();
        queries::insert_transaction(&db, &mk_tx("KULR", AssetType::StockIntl)).unwrap();
        queries::upsert_daily_price(
            &db,
            &mk_price("KULR", NaiveDate::from_ymd_opt(2026, 4, 1).unwrap()), // Wed
        )
        .unwrap();

        assert_eq!(cleanup_non_trading_day_prices(&db).unwrap(), 0);
        assert_eq!(cleanup_non_trading_day_prices(&db).unwrap(), 0);
    }
}
