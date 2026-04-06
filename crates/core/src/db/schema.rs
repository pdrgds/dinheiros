use rusqlite::{Connection, Result};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Database { conn };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn list_tables(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
        )?;
        let names = stmt.query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>>>()?;
        Ok(names)
    }

    pub fn run_migrations(&self) -> Result<()> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS transactions (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                source         TEXT NOT NULL,
                asset_type     TEXT NOT NULL,
                symbol         TEXT NOT NULL,
                tx_type        TEXT NOT NULL,
                date           TEXT NOT NULL,
                quantity       REAL NOT NULL,
                unit_price     REAL,
                currency       TEXT NOT NULL,
                total_value    REAL NOT NULL,
                brl_rate       REAL NOT NULL,
                total_brl      REAL NOT NULL,
                commission     REAL,
                fee_brl        REAL,
                notes          TEXT,
                import_hash    TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS daily_prices (
                symbol         TEXT NOT NULL,
                date           TEXT NOT NULL,
                close_price    REAL NOT NULL,
                currency       TEXT NOT NULL,
                brl_rate       REAL NOT NULL,
                PRIMARY KEY (symbol, date)
            );

            CREATE TABLE IF NOT EXISTS income (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                source         TEXT NOT NULL,
                symbol         TEXT NOT NULL,
                date           TEXT NOT NULL,
                income_type    TEXT NOT NULL,
                currency       TEXT NOT NULL,
                gross_value    REAL NOT NULL,
                tax_withheld   REAL,
                tax_origin     TEXT,
                brl_rate       REAL NOT NULL,
                net_value_brl  REAL NOT NULL,
                import_hash    TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS config (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_transactions_symbol ON transactions(symbol);
            CREATE INDEX IF NOT EXISTS idx_transactions_date   ON transactions(date);
            CREATE INDEX IF NOT EXISTS idx_daily_prices_symbol ON daily_prices(symbol);
            CREATE INDEX IF NOT EXISTS idx_daily_prices_date   ON daily_prices(date);
            CREATE INDEX IF NOT EXISTS idx_income_symbol       ON income(symbol);
            CREATE INDEX IF NOT EXISTS idx_income_date         ON income(date);
        ")?;
        Ok(())
    }
}
