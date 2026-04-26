use rusqlite::{Connection, Result};
use std::path::Path;

/// Config key that stores how many rows the one-time `Transferência - Liquidação`
/// reclassification migration touched. Set on first migration when count > 0;
/// the UI reads this to display a backfill notice and then clears the key.
pub const CONFIG_KEY_TRANSFER_RECLASSIFY_COUNT: &str = "migration_transfers_reclassified_count";

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

        self.reclassify_misimported_transferencia_liquidacao()?;
        Ok(())
    }

    /// One-time data fix: an older B3 importer wrote every "Transferência - Liquidação"
    /// row as `tx_type='sell'`, double-counting custody transfers as taxable disposals.
    /// This UPDATE flips them to `transfer_out` so positions and realized-gain math
    /// stop treating them as sells.
    ///
    /// Idempotent by construction: after the first run nothing matches the criteria,
    /// so subsequent calls find 0 rows and do nothing. We persist the count on the
    /// first run that actually changes data so the UI can surface a backfill notice
    /// once and then clear the key.
    fn reclassify_misimported_transferencia_liquidacao(&self) -> Result<()> {
        let count = self.conn.execute(
            "UPDATE transactions
             SET tx_type = 'transfer_out'
             WHERE tx_type = 'sell'
               AND notes LIKE '%Transferência - Liquidação%'",
            [],
        )?;

        if count > 0 {
            // Don't overwrite an existing pending notice — the UI clears the key
            // after displaying it. A repeat migration that touches more rows
            // (re-imported legacy data) accumulates into the pending count.
            let existing: Option<String> = self
                .conn
                .query_row(
                    "SELECT value FROM config WHERE key = ?1",
                    [CONFIG_KEY_TRANSFER_RECLASSIFY_COUNT],
                    |row| row.get(0),
                )
                .ok();
            let total = existing
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0)
                + count as i64;
            self.conn.execute(
                "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
                rusqlite::params![CONFIG_KEY_TRANSFER_RECLASSIFY_COUNT, total.to_string()],
            )?;
            eprintln!(
                "[db] reclassified {} 'Transferência - Liquidação' rows from sell to transfer_out",
                count
            );
        }

        Ok(())
    }
}
