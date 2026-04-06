use chrono::NaiveDate;
use rusqlite::{params, Result};

use crate::db::Database;
use crate::types::*;

pub fn insert_transaction(db: &Database, tx: &Transaction) -> Result<bool> {
    let rows = db.conn().execute(
        "INSERT OR IGNORE INTO transactions
            (source, asset_type, symbol, tx_type, date, quantity, unit_price,
             currency, total_value, brl_rate, total_brl, commission, fee_brl,
             notes, import_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            tx.source.as_str(),
            tx.asset_type.as_str(),
            tx.symbol,
            tx.tx_type.as_str(),
            tx.date.to_string(),
            tx.quantity,
            tx.unit_price,
            tx.currency,
            tx.total_value,
            tx.brl_rate,
            tx.total_brl,
            tx.commission,
            tx.fee_brl,
            tx.notes,
            tx.import_hash,
        ],
    )?;
    Ok(rows > 0)
}

pub fn get_transactions_by_symbol(db: &Database, symbol: &str) -> Result<Vec<Transaction>> {
    let mut stmt = db.conn().prepare(
        "SELECT id, source, asset_type, symbol, tx_type, date, quantity, unit_price,
                currency, total_value, brl_rate, total_brl, commission, fee_brl,
                notes, import_hash
         FROM transactions WHERE symbol = ?1 ORDER BY date",
    )?;
    let rows = stmt.query_map(params![symbol], row_to_transaction)?;
    rows.collect()
}

pub fn get_all_transactions(db: &Database) -> Result<Vec<Transaction>> {
    let mut stmt = db.conn().prepare(
        "SELECT id, source, asset_type, symbol, tx_type, date, quantity, unit_price,
                currency, total_value, brl_rate, total_brl, commission, fee_brl,
                notes, import_hash
         FROM transactions ORDER BY date",
    )?;
    let rows = stmt.query_map([], row_to_transaction)?;
    rows.collect()
}

fn row_to_transaction(row: &rusqlite::Row) -> Result<Transaction> {
    let source_str: String = row.get(1)?;
    let asset_type_str: String = row.get(2)?;
    let tx_type_str: String = row.get(4)?;
    let date_str: String = row.get(5)?;

    Ok(Transaction {
        id: row.get(0)?,
        source: Source::from_str(&source_str)
            .unwrap_or(Source::Manual),
        asset_type: AssetType::from_str(&asset_type_str)
            .unwrap_or(AssetType::StockBr),
        symbol: row.get(3)?,
        tx_type: TxType::from_str(&tx_type_str)
            .unwrap_or(TxType::Buy),
        date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()),
        quantity: row.get(6)?,
        unit_price: row.get(7)?,
        currency: row.get(8)?,
        total_value: row.get(9)?,
        brl_rate: row.get(10)?,
        total_brl: row.get(11)?,
        commission: row.get(12)?,
        fee_brl: row.get(13)?,
        notes: row.get(14)?,
        import_hash: row.get(15)?,
    })
}

pub fn insert_income(db: &Database, income: &Income) -> Result<bool> {
    let rows = db.conn().execute(
        "INSERT OR IGNORE INTO income
            (source, symbol, date, income_type, currency, gross_value,
             tax_withheld, tax_origin, brl_rate, net_value_brl, import_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            income.source.as_str(),
            income.symbol,
            income.date.to_string(),
            income.income_type.as_str(),
            income.currency,
            income.gross_value,
            income.tax_withheld,
            income.tax_origin,
            income.brl_rate,
            income.net_value_brl,
            income.import_hash,
        ],
    )?;
    Ok(rows > 0)
}

pub fn get_all_income(db: &Database) -> Result<Vec<Income>> {
    let mut stmt = db.conn().prepare(
        "SELECT id, source, symbol, date, income_type, currency, gross_value,
                tax_withheld, tax_origin, brl_rate, net_value_brl, import_hash
         FROM income ORDER BY date",
    )?;
    let rows = stmt.query_map([], |row| {
        let source_str: String = row.get(1)?;
        let date_str: String = row.get(3)?;
        let income_type_str: String = row.get(4)?;

        Ok(Income {
            id: row.get(0)?,
            source: Source::from_str(&source_str)
                .unwrap_or(Source::Manual),
            symbol: row.get(2)?,
            date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()),
            income_type: IncomeType::from_str(&income_type_str)
                .unwrap_or(IncomeType::Dividend),
            currency: row.get(5)?,
            gross_value: row.get(6)?,
            tax_withheld: row.get(7)?,
            tax_origin: row.get(8)?,
            brl_rate: row.get(9)?,
            net_value_brl: row.get(10)?,
            import_hash: row.get(11)?,
        })
    })?;
    rows.collect()
}

pub fn upsert_daily_price(db: &Database, price: &DailyPrice) -> Result<()> {
    db.conn().execute(
        "INSERT OR REPLACE INTO daily_prices
            (symbol, date, close_price, currency, brl_rate)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            price.symbol,
            price.date.to_string(),
            price.close_price,
            price.currency,
            price.brl_rate,
        ],
    )?;
    Ok(())
}

pub fn get_latest_price_date(db: &Database, symbol: &str) -> Result<Option<NaiveDate>> {
    let mut stmt = db
        .conn()
        .prepare("SELECT MAX(date) FROM daily_prices WHERE symbol = ?1")?;
    let result: Option<String> = stmt.query_row(params![symbol], |row| row.get(0))?;
    Ok(result.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()))
}

pub fn get_latest_price(db: &Database, symbol: &str) -> Result<Option<DailyPrice>> {
    let mut stmt = db.conn().prepare(
        "SELECT symbol, date, close_price, currency, brl_rate
         FROM daily_prices WHERE symbol = ?1 ORDER BY date DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(params![symbol], row_to_daily_price)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn get_daily_prices(
    db: &Database,
    symbol: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<DailyPrice>> {
    let mut stmt = db.conn().prepare(
        "SELECT symbol, date, close_price, currency, brl_rate
         FROM daily_prices WHERE symbol = ?1 AND date >= ?2 AND date <= ?3 ORDER BY date",
    )?;
    let rows = stmt.query_map(
        params![symbol, from.to_string(), to.to_string()],
        row_to_daily_price,
    )?;
    rows.collect()
}

fn row_to_daily_price(row: &rusqlite::Row) -> Result<DailyPrice> {
    let date_str: String = row.get(1)?;
    Ok(DailyPrice {
        symbol: row.get(0)?,
        date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()),
        close_price: row.get(2)?,
        currency: row.get(3)?,
        brl_rate: row.get(4)?,
    })
}

pub fn get_config(db: &Database, key: &str) -> Result<Option<String>> {
    let mut stmt = db
        .conn()
        .prepare("SELECT value FROM config WHERE key = ?1")?;
    let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn set_config(db: &Database, key: &str, value: &str) -> Result<()> {
    db.conn().execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

/// Get symbols with positive net holdings (active positions only).
pub fn get_distinct_symbols(db: &Database) -> Result<Vec<(String, String, String)>> {
    let mut stmt = db.conn().prepare(
        "SELECT symbol, asset_type, currency
         FROM transactions
         WHERE tx_type IN ('buy', 'sell')
         GROUP BY symbol, currency
         HAVING SUM(CASE WHEN tx_type='buy' THEN quantity ELSE -quantity END) > 0.0001
         ORDER BY symbol",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    rows.collect()
}
