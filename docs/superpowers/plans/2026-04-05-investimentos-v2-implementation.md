# Investimentos v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust desktop app that aggregates investments from IBKR, B3, Binance, and gold into a single portfolio view with historical charts, P/L tracking, and income history.

**Architecture:** Cargo workspace with two crates: `investimentos-core` (business logic, parsers, API clients, DB) and `investimentos-ui` (GPUI-CE desktop app). Core is fully testable without UI. SQLite for persistence.

**Tech Stack:** Rust, GPUI-CE, SQLite (rusqlite), ib-flex, calamine, yahoo_finance_api, reqwest, serde, chrono

**Spec:** `docs/superpowers/specs/2026-04-05-investimentos-v2-design.md`

**Input files for testing:** `input-files/` directory contains real IBKR CSVs, B3 XLSX, and Binance CSVs.

---

## Phase 1: Foundation (Tasks 1-3)

### Task 1: Workspace Scaffolding + Domain Types

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/core/Cargo.toml`
- Create: `crates/core/src/lib.rs`
- Create: `crates/core/src/types.rs`
- Create: `crates/ui/Cargo.toml`
- Create: `crates/ui/src/main.rs`
- Create: `.gitignore`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = ["crates/core", "crates/ui"]
```

- [ ] **Step 2: Create core crate Cargo.toml**

```toml
# crates/core/Cargo.toml
[package]
name = "investimentos-core"
version = "0.1.0"
edition = "2021"

[dependencies]
rusqlite = { version = "0.32", features = ["bundled"] }
calamine = "0.26"
csv = "1.3"
ib-flex = { version = "0.1", features = ["api-client"] }
yahoo_finance_api = "4"
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
sha2 = "0.10"
hex = "0.4"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
thiserror = "2"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Create domain types**

```rust
// crates/core/src/types.rs
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Source {
    B3,
    Ibkr,
    Binance,
    Manual,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::B3 => "b3",
            Source::Ibkr => "ibkr",
            Source::Binance => "binance",
            Source::Manual => "manual",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "b3" => Some(Source::B3),
            "ibkr" => Some(Source::Ibkr),
            "binance" => Some(Source::Binance),
            "manual" => Some(Source::Manual),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssetType {
    StockBr,
    StockIntl,
    Tesouro,
    Crypto,
    Gold,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::StockBr => "stock_br",
            AssetType::StockIntl => "stock_intl",
            AssetType::Tesouro => "tesouro",
            AssetType::Crypto => "crypto",
            AssetType::Gold => "gold",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "stock_br" => Some(AssetType::StockBr),
            "stock_intl" => Some(AssetType::StockIntl),
            "tesouro" => Some(AssetType::Tesouro),
            "crypto" => Some(AssetType::Crypto),
            "gold" => Some(AssetType::Gold),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TxType {
    Buy,
    Sell,
    Dividend,
    Jcp,
    FractionAuction,
    Send,
    Deposit,
}

impl TxType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TxType::Buy => "buy",
            TxType::Sell => "sell",
            TxType::Dividend => "dividend",
            TxType::Jcp => "jcp",
            TxType::FractionAuction => "fraction_auction",
            TxType::Send => "send",
            TxType::Deposit => "deposit",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "buy" => Some(TxType::Buy),
            "sell" => Some(TxType::Sell),
            "dividend" => Some(TxType::Dividend),
            "jcp" => Some(TxType::Jcp),
            "fraction_auction" => Some(TxType::FractionAuction),
            "send" => Some(TxType::Send),
            "deposit" => Some(TxType::Deposit),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IncomeType {
    Dividend,
    Jcp,
}

impl IncomeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IncomeType::Dividend => "dividend",
            IncomeType::Jcp => "jcp",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "dividend" => Some(IncomeType::Dividend),
            "jcp" => Some(IncomeType::Jcp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Option<i64>,
    pub source: Source,
    pub asset_type: AssetType,
    pub symbol: String,
    pub tx_type: TxType,
    pub date: NaiveDate,
    pub quantity: f64,
    pub unit_price: Option<f64>,
    pub currency: String,
    pub total_value: f64,
    pub brl_rate: f64,
    pub total_brl: f64,
    pub commission: Option<f64>,
    pub fee_brl: Option<f64>,
    pub notes: Option<String>,
    pub import_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyPrice {
    pub symbol: String,
    pub date: NaiveDate,
    pub close_price: f64,
    pub currency: String,
    pub brl_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Income {
    pub id: Option<i64>,
    pub source: Source,
    pub symbol: String,
    pub date: NaiveDate,
    pub income_type: IncomeType,
    pub currency: String,
    pub gross_value: f64,
    pub tax_withheld: Option<f64>,
    pub tax_origin: Option<String>,
    pub brl_rate: f64,
    pub net_value_brl: f64,
    pub import_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub asset_type: AssetType,
    pub quantity: f64,
    pub avg_cost: f64,
    pub avg_cost_brl: f64,
    pub currency: String,
    pub current_price: Option<f64>,
    pub current_brl_rate: Option<f64>,
    pub current_value_brl: Option<f64>,
    pub pnl_brl: Option<f64>,
    pub pnl_pct: Option<f64>,
    pub weight: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Allocation {
    pub asset_type: AssetType,
    pub value_brl: f64,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub version: u32,
    pub exported_at: String,
    pub transactions: Vec<Transaction>,
    pub daily_prices: Vec<DailyPrice>,
    pub income: Vec<Income>,
    pub config: std::collections::HashMap<String, String>,
}
```

- [ ] **Step 4: Create lib.rs exposing modules**

```rust
// crates/core/src/lib.rs
pub mod types;

pub use types::*;
```

- [ ] **Step 5: Create minimal UI crate**

```toml
# crates/ui/Cargo.toml
[package]
name = "investimentos-ui"
version = "0.1.0"
edition = "2021"

[dependencies]
investimentos-core = { path = "../core" }
gpui = { package = "gpui-ce", version = "0.3" }
```

```rust
// crates/ui/src/main.rs
fn main() {
    println!("investimentos-ui — placeholder");
}
```

- [ ] **Step 6: Create .gitignore**

```
/target
*.db
*.db-journal
.DS_Store
.superpowers/
```

- [ ] **Step 7: Verify workspace compiles**

Run: `cargo build`
Expected: successful compilation of both crates.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml crates/ .gitignore
git commit -m "scaffold workspace: core + ui crates, domain types"
```

---

### Task 2: SQLite Database Layer

**Files:**
- Create: `crates/core/src/db/mod.rs`
- Create: `crates/core/src/db/schema.rs`
- Create: `crates/core/src/db/queries.rs`
- Modify: `crates/core/src/lib.rs`

- [ ] **Step 1: Write the failing test for DB creation and schema**

```rust
// crates/core/src/db/mod.rs
pub mod schema;
pub mod queries;

pub use schema::Database;
```

```rust
// crates/core/src/db/schema.rs
// (empty for now — test first)
```

Create a test file:

```rust
// crates/core/tests/db_test.rs
use investimentos_core::db::Database;
use tempfile::NamedTempFile;

#[test]
fn test_create_database_and_tables() {
    let tmp = NamedTempFile::new().unwrap();
    let db = Database::open(tmp.path()).unwrap();

    // Verify tables exist by querying sqlite_master
    let tables: Vec<String> = db.list_tables().unwrap();
    assert!(tables.contains(&"transactions".to_string()));
    assert!(tables.contains(&"daily_prices".to_string()));
    assert!(tables.contains(&"income".to_string()));
    assert!(tables.contains(&"config".to_string()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package investimentos-core test_create_database_and_tables`
Expected: FAIL — `Database` not defined.

- [ ] **Step 3: Implement Database struct with schema creation**

```rust
// crates/core/src/db/schema.rs
use rusqlite::{Connection, Result};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    fn run_migrations(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                symbol TEXT NOT NULL,
                tx_type TEXT NOT NULL,
                date TEXT NOT NULL,
                quantity REAL NOT NULL,
                unit_price REAL,
                currency TEXT NOT NULL,
                total_value REAL NOT NULL,
                brl_rate REAL NOT NULL,
                total_brl REAL NOT NULL,
                commission REAL,
                fee_brl REAL,
                notes TEXT,
                import_hash TEXT UNIQUE
            );

            CREATE TABLE IF NOT EXISTS daily_prices (
                symbol TEXT NOT NULL,
                date TEXT NOT NULL,
                close_price REAL NOT NULL,
                currency TEXT NOT NULL,
                brl_rate REAL NOT NULL,
                PRIMARY KEY (symbol, date)
            );

            CREATE TABLE IF NOT EXISTS income (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                symbol TEXT NOT NULL,
                date TEXT NOT NULL,
                income_type TEXT NOT NULL,
                currency TEXT NOT NULL,
                gross_value REAL NOT NULL,
                tax_withheld REAL,
                tax_origin TEXT,
                brl_rate REAL NOT NULL,
                net_value_brl REAL NOT NULL,
                import_hash TEXT UNIQUE
            );

            CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_transactions_symbol ON transactions(symbol);
            CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(date);
            CREATE INDEX IF NOT EXISTS idx_daily_prices_symbol ON daily_prices(symbol);
            CREATE INDEX IF NOT EXISTS idx_income_symbol ON income(symbol);
            CREATE INDEX IF NOT EXISTS idx_income_date ON income(date);
            "
        )
    }

    pub fn list_tables(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
        )?;
        let tables = stmt.query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>>>()?;
        Ok(tables)
    }
}
```

- [ ] **Step 4: Update lib.rs to expose db module**

```rust
// crates/core/src/lib.rs
pub mod db;
pub mod types;

pub use types::*;
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --package investimentos-core test_create_database_and_tables`
Expected: PASS

- [ ] **Step 6: Write failing test for transaction CRUD**

Add to `crates/core/tests/db_test.rs`:

```rust
use investimentos_core::db::queries;
use investimentos_core::types::*;
use chrono::NaiveDate;

#[test]
fn test_insert_and_query_transactions() {
    let db = Database::open_in_memory().unwrap();

    let tx = Transaction {
        id: None,
        source: Source::Binance,
        asset_type: AssetType::Crypto,
        symbol: "BTC".to_string(),
        tx_type: TxType::Buy,
        date: NaiveDate::from_ymd_opt(2024, 4, 13).unwrap(),
        quantity: 0.00146106,
        unit_price: Some(350427.0),
        currency: "BRL".to_string(),
        total_value: 512.0,
        brl_rate: 1.0,
        total_brl: 512.0,
        commission: None,
        fee_brl: None,
        notes: None,
        import_hash: "test_hash_001".to_string(),
    };

    let inserted = queries::insert_transaction(&db, &tx).unwrap();
    assert!(inserted);

    // Duplicate should be skipped
    let duplicate = queries::insert_transaction(&db, &tx).unwrap();
    assert!(!duplicate);

    let txs = queries::get_transactions_by_symbol(&db, "BTC").unwrap();
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].source, Source::Binance);
    assert_eq!(txs[0].total_brl, 512.0);
}
```

- [ ] **Step 7: Run test to verify it fails**

Run: `cargo test --package investimentos-core test_insert_and_query_transactions`
Expected: FAIL — `queries` module not found or functions not defined.

- [ ] **Step 8: Implement transaction queries**

```rust
// crates/core/src/db/queries.rs
use crate::db::schema::Database;
use crate::types::*;
use chrono::NaiveDate;
use rusqlite::{params, Result};

pub fn insert_transaction(db: &Database, tx: &Transaction) -> Result<bool> {
    let result = db.conn().execute(
        "INSERT OR IGNORE INTO transactions
            (source, asset_type, symbol, tx_type, date, quantity, unit_price,
             currency, total_value, brl_rate, total_brl, commission, fee_brl, notes, import_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            tx.source.as_str(),
            tx.asset_type.as_str(),
            tx.symbol,
            tx.tx_type.as_str(),
            tx.date.format("%Y-%m-%d").to_string(),
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
    Ok(result > 0)
}

pub fn get_transactions_by_symbol(db: &Database, symbol: &str) -> Result<Vec<Transaction>> {
    let mut stmt = db.conn().prepare(
        "SELECT id, source, asset_type, symbol, tx_type, date, quantity, unit_price,
                currency, total_value, brl_rate, total_brl, commission, fee_brl, notes, import_hash
         FROM transactions WHERE symbol = ?1 ORDER BY date"
    )?;
    let rows = stmt.query_map(params![symbol], |row| {
        Ok(Transaction {
            id: Some(row.get(0)?),
            source: Source::from_str(&row.get::<_, String>(1)?).unwrap_or(Source::Manual),
            asset_type: AssetType::from_str(&row.get::<_, String>(2)?).unwrap_or(AssetType::StockBr),
            symbol: row.get(3)?,
            tx_type: TxType::from_str(&row.get::<_, String>(4)?).unwrap_or(TxType::Buy),
            date: NaiveDate::parse_from_str(&row.get::<_, String>(5)?, "%Y-%m-%d").unwrap(),
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
    })?;
    rows.collect()
}

pub fn get_all_transactions(db: &Database) -> Result<Vec<Transaction>> {
    let mut stmt = db.conn().prepare(
        "SELECT id, source, asset_type, symbol, tx_type, date, quantity, unit_price,
                currency, total_value, brl_rate, total_brl, commission, fee_brl, notes, import_hash
         FROM transactions ORDER BY date"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Transaction {
            id: Some(row.get(0)?),
            source: Source::from_str(&row.get::<_, String>(1)?).unwrap_or(Source::Manual),
            asset_type: AssetType::from_str(&row.get::<_, String>(2)?).unwrap_or(AssetType::StockBr),
            symbol: row.get(3)?,
            tx_type: TxType::from_str(&row.get::<_, String>(4)?).unwrap_or(TxType::Buy),
            date: NaiveDate::parse_from_str(&row.get::<_, String>(5)?, "%Y-%m-%d").unwrap(),
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
    })?;
    rows.collect()
}

pub fn insert_income(db: &Database, income: &Income) -> Result<bool> {
    let result = db.conn().execute(
        "INSERT OR IGNORE INTO income
            (source, symbol, date, income_type, currency, gross_value,
             tax_withheld, tax_origin, brl_rate, net_value_brl, import_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            income.source.as_str(),
            income.symbol,
            income.date.format("%Y-%m-%d").to_string(),
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
    Ok(result > 0)
}

pub fn get_all_income(db: &Database) -> Result<Vec<Income>> {
    let mut stmt = db.conn().prepare(
        "SELECT id, source, symbol, date, income_type, currency, gross_value,
                tax_withheld, tax_origin, brl_rate, net_value_brl, import_hash
         FROM income ORDER BY date"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Income {
            id: Some(row.get(0)?),
            source: Source::from_str(&row.get::<_, String>(1)?).unwrap_or(Source::B3),
            symbol: row.get(2)?,
            date: NaiveDate::parse_from_str(&row.get::<_, String>(3)?, "%Y-%m-%d").unwrap(),
            income_type: IncomeType::from_str(&row.get::<_, String>(4)?).unwrap_or(IncomeType::Dividend),
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
        "INSERT OR REPLACE INTO daily_prices (symbol, date, close_price, currency, brl_rate)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            price.symbol,
            price.date.format("%Y-%m-%d").to_string(),
            price.close_price,
            price.currency,
            price.brl_rate,
        ],
    )?;
    Ok(())
}

pub fn get_latest_price_date(db: &Database, symbol: &str) -> Result<Option<NaiveDate>> {
    let mut stmt = db.conn().prepare(
        "SELECT MAX(date) FROM daily_prices WHERE symbol = ?1"
    )?;
    let result: Option<String> = stmt.query_row(params![symbol], |row| row.get(0))?;
    Ok(result.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()))
}

pub fn get_latest_price(db: &Database, symbol: &str) -> Result<Option<DailyPrice>> {
    let mut stmt = db.conn().prepare(
        "SELECT symbol, date, close_price, currency, brl_rate
         FROM daily_prices WHERE symbol = ?1 ORDER BY date DESC LIMIT 1"
    )?;
    let mut rows = stmt.query_map(params![symbol], |row| {
        Ok(DailyPrice {
            symbol: row.get(0)?,
            date: NaiveDate::parse_from_str(&row.get::<_, String>(1)?, "%Y-%m-%d").unwrap(),
            close_price: row.get(2)?,
            currency: row.get(3)?,
            brl_rate: row.get(4)?,
        })
    })?;
    match rows.next() {
        Some(Ok(price)) => Ok(Some(price)),
        _ => Ok(None),
    }
}

pub fn get_daily_prices(db: &Database, symbol: &str, from: NaiveDate, to: NaiveDate) -> Result<Vec<DailyPrice>> {
    let mut stmt = db.conn().prepare(
        "SELECT symbol, date, close_price, currency, brl_rate
         FROM daily_prices WHERE symbol = ?1 AND date >= ?2 AND date <= ?3 ORDER BY date"
    )?;
    let rows = stmt.query_map(
        params![symbol, from.format("%Y-%m-%d").to_string(), to.format("%Y-%m-%d").to_string()],
        |row| {
            Ok(DailyPrice {
                symbol: row.get(0)?,
                date: NaiveDate::parse_from_str(&row.get::<_, String>(1)?, "%Y-%m-%d").unwrap(),
                close_price: row.get(2)?,
                currency: row.get(3)?,
                brl_rate: row.get(4)?,
            })
        },
    )?;
    rows.collect()
}

pub fn get_config(db: &Database, key: &str) -> Result<Option<String>> {
    let mut stmt = db.conn().prepare("SELECT value FROM config WHERE key = ?1")?;
    let mut rows = stmt.query_map(params![key], |row| row.get(0))?;
    match rows.next() {
        Some(Ok(value)) => Ok(Some(value)),
        _ => Ok(None),
    }
}

pub fn set_config(db: &Database, key: &str, value: &str) -> Result<()> {
    db.conn().execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_distinct_symbols(db: &Database) -> Result<Vec<(String, String, String)>> {
    let mut stmt = db.conn().prepare(
        "SELECT DISTINCT symbol, asset_type, currency FROM transactions
         WHERE tx_type IN ('buy', 'sell') ORDER BY symbol"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    rows.collect()
}
```

- [ ] **Step 9: Run test to verify it passes**

Run: `cargo test --package investimentos-core test_insert_and_query_transactions`
Expected: PASS

- [ ] **Step 10: Write failing test for daily prices and config**

Add to `crates/core/tests/db_test.rs`:

```rust
#[test]
fn test_daily_prices_upsert_and_query() {
    let db = Database::open_in_memory().unwrap();

    let price = DailyPrice {
        symbol: "PETR4".to_string(),
        date: NaiveDate::from_ymd_opt(2024, 12, 20).unwrap(),
        close_price: 36.50,
        currency: "BRL".to_string(),
        brl_rate: 1.0,
    };
    queries::upsert_daily_price(&db, &price).unwrap();

    let latest = queries::get_latest_price(&db, "PETR4").unwrap().unwrap();
    assert_eq!(latest.close_price, 36.50);

    let latest_date = queries::get_latest_price_date(&db, "PETR4").unwrap().unwrap();
    assert_eq!(latest_date, NaiveDate::from_ymd_opt(2024, 12, 20).unwrap());

    // No data for unknown symbol
    let none = queries::get_latest_price_date(&db, "UNKNOWN").unwrap();
    assert!(none.is_none());
}

#[test]
fn test_config_get_set() {
    let db = Database::open_in_memory().unwrap();

    assert!(queries::get_config(&db, "ibkr_flex_token").unwrap().is_none());

    queries::set_config(&db, "ibkr_flex_token", "my_token_123").unwrap();
    let value = queries::get_config(&db, "ibkr_flex_token").unwrap().unwrap();
    assert_eq!(value, "my_token_123");

    // Overwrite
    queries::set_config(&db, "ibkr_flex_token", "new_token").unwrap();
    let value = queries::get_config(&db, "ibkr_flex_token").unwrap().unwrap();
    assert_eq!(value, "new_token");
}
```

- [ ] **Step 11: Run tests to verify they pass**

Run: `cargo test --package investimentos-core`
Expected: all 4 tests PASS.

- [ ] **Step 12: Commit**

```bash
git add crates/core/src/db/ crates/core/tests/
git commit -m "feat: SQLite database layer with schema, CRUD queries, and tests"
```

---

### Task 3: Binance CSV Parser

**Files:**
- Create: `crates/core/src/parsers/mod.rs`
- Create: `crates/core/src/parsers/binance.rs`
- Create: `crates/core/tests/binance_parser_test.rs`
- Modify: `crates/core/src/lib.rs`

Uses real test data from `input-files/binance/`.

- [ ] **Step 1: Write the failing test**

```rust
// crates/core/tests/binance_parser_test.rs
use investimentos_core::parsers::binance;
use investimentos_core::types::*;
use std::path::Path;

#[test]
fn test_parse_binance_csv() {
    let path = Path::new("../../input-files/binance/2025_03_21_16_01_30.csv");
    let result = binance::parse(path).unwrap();

    // File has Buy, Send, Deposit transactions
    assert!(!result.transactions.is_empty());

    // First transaction is a P2P Buy
    let first_buy = result.transactions.iter()
        .find(|t| t.tx_type == TxType::Buy)
        .unwrap();
    assert_eq!(first_buy.symbol, "BTC");
    assert_eq!(first_buy.asset_type, AssetType::Crypto);
    assert_eq!(first_buy.source, Source::Binance);
    assert_eq!(first_buy.currency, "BRL");

    // Should have Send transactions (withdrawals)
    let sends: Vec<_> = result.transactions.iter()
        .filter(|t| t.tx_type == TxType::Send)
        .collect();
    assert!(!sends.is_empty());

    // Deposits should be excluded (they're BRL deposits, not investment transactions)
    let deposits: Vec<_> = result.transactions.iter()
        .filter(|t| t.tx_type == TxType::Deposit)
        .collect();
    assert!(deposits.is_empty());
}

#[test]
fn test_binance_btc_holdings_calculation() {
    let path = Path::new("../../input-files/binance/2025_03_21_16_01_30.csv");
    let result = binance::parse(path).unwrap();

    // BTC holdings = sum(received_amount from Buy) - sum(fee_amount from all)
    let bought: f64 = result.transactions.iter()
        .filter(|t| t.tx_type == TxType::Buy)
        .map(|t| t.quantity)
        .sum();

    let fee_btc: f64 = result.total_fees_btc;

    // Net BTC should be buys minus fees
    assert!(bought > 0.0);
    assert!(fee_btc > 0.0);
    assert!((result.net_btc - (bought - fee_btc)).abs() < 0.00000001);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package investimentos-core test_parse_binance_csv`
Expected: FAIL — `parsers` module not found.

- [ ] **Step 3: Implement the Binance parser**

```rust
// crates/core/src/parsers/mod.rs
pub mod binance;
```

```rust
// crates/core/src/parsers/binance.rs
use crate::types::*;
use chrono::NaiveDate;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug)]
pub struct BinanceImportResult {
    pub transactions: Vec<Transaction>,
    pub net_btc: f64,
    pub total_fees_btc: f64,
}

pub fn parse(path: &Path) -> Result<BinanceImportResult, Box<dyn std::error::Error>> {
    let mut rdr = csv::Reader::from_path(path)?;
    let mut transactions = Vec::new();
    let mut total_bought_btc = 0.0;
    let mut total_fees_btc = 0.0;

    for result in rdr.records() {
        let record = result?;
        let id = record.get(0).unwrap_or("");
        let datetime_str = record.get(1).unwrap_or("");
        let tx_type_str = record.get(2).unwrap_or("");

        let date = parse_binance_date(datetime_str)?;

        match tx_type_str {
            "Buy" => {
                let sent_amount: f64 = record.get(6).unwrap_or("0").parse().unwrap_or(0.0);
                let sent_currency = record.get(7).unwrap_or("");
                let received_amount: f64 = record.get(10).unwrap_or("0").parse().unwrap_or(0.0);
                let received_currency = record.get(11).unwrap_or("");
                let fee_amount: f64 = record.get(14).unwrap_or("0").parse().unwrap_or(0.0);
                let fee_currency = record.get(15).unwrap_or("");
                let fee_value_brl: f64 = record.get(16).unwrap_or("0").parse().unwrap_or(0.0);

                if received_currency != "BTC" {
                    continue;
                }

                let unit_price = if received_amount > 0.0 {
                    Some(sent_amount / received_amount)
                } else {
                    None
                };

                let hash = compute_hash(&format!("binance:buy:{}:{}", id, datetime_str));

                transactions.push(Transaction {
                    id: None,
                    source: Source::Binance,
                    asset_type: AssetType::Crypto,
                    symbol: "BTC".to_string(),
                    tx_type: TxType::Buy,
                    date,
                    quantity: received_amount,
                    unit_price,
                    currency: sent_currency.to_string(),
                    total_value: sent_amount,
                    brl_rate: 1.0, // Binance data is already in BRL
                    total_brl: sent_amount,
                    commission: None,
                    fee_brl: if fee_value_brl > 0.0 { Some(fee_value_brl) } else { None },
                    notes: None,
                    import_hash: hash,
                });

                total_bought_btc += received_amount;

                if fee_currency == "BRL" || fee_currency == "Brazilian Real" {
                    // BRL fees don't reduce BTC
                } else if fee_currency == "BTC" || fee_currency == "Bitcoin" {
                    total_fees_btc += fee_amount;
                }
            }
            "Send" => {
                let sent_amount: f64 = record.get(6).unwrap_or("0").parse().unwrap_or(0.0);
                let fee_amount: f64 = record.get(14).unwrap_or("0").parse().unwrap_or(0.0);
                let fee_currency = record.get(15).unwrap_or("");
                let sent_address = record.get(9).unwrap_or("");

                let hash = compute_hash(&format!("binance:send:{}:{}", id, datetime_str));

                transactions.push(Transaction {
                    id: None,
                    source: Source::Binance,
                    asset_type: AssetType::Crypto,
                    symbol: "BTC".to_string(),
                    tx_type: TxType::Send,
                    date,
                    quantity: sent_amount,
                    unit_price: None,
                    currency: "BTC".to_string(),
                    total_value: sent_amount,
                    brl_rate: 1.0,
                    total_brl: 0.0, // Value in BRL not meaningful for transfers
                    commission: None,
                    fee_brl: None,
                    notes: Some(format!("to:{}", sent_address)),
                    import_hash: hash,
                });

                if fee_currency == "BTC" || fee_currency == "Bitcoin" {
                    total_fees_btc += fee_amount;
                }
            }
            "Deposit" => {
                // BRL deposits into Binance — skip, not investment transactions
                continue;
            }
            _ => continue,
        }
    }

    let net_btc = total_bought_btc - total_fees_btc;

    Ok(BinanceImportResult {
        transactions,
        net_btc,
        total_fees_btc,
    })
}

fn parse_binance_date(s: &str) -> Result<NaiveDate, Box<dyn std::error::Error>> {
    // Handles both "2024-04-13-20:51:53" and "2025-03-24 17:01:04" formats
    let date_part = if s.len() >= 10 { &s[..10] } else { s };
    Ok(NaiveDate::parse_from_str(date_part, "%Y-%m-%d")?)
}

fn compute_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
```

- [ ] **Step 4: Update lib.rs**

```rust
// crates/core/src/lib.rs
pub mod db;
pub mod parsers;
pub mod types;

pub use types::*;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --package investimentos-core binance`
Expected: both tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/parsers/ crates/core/tests/binance_parser_test.rs crates/core/src/lib.rs
git commit -m "feat: Binance CSV parser with BTC holdings calculation"
```

---

## Phase 2: Parsers (Tasks 4-5)

### Task 4: B3 XLSX Parser

**Files:**
- Create: `crates/core/src/parsers/b3.rs`
- Create: `crates/core/tests/b3_parser_test.rs`
- Modify: `crates/core/src/parsers/mod.rs`

Uses real test data from `input-files/b3/`.

The B3 XLSX "Movimentacao" sheet has columns:
- A: Entrada/Saida (Credito/Debito)
- B: Data (dd/mm/yyyy)
- C: Movimentacao (Compra, Venda, Dividendo, Juros Sobre Capital Proprio, Leilao de Fracao, Transferencia - Liquidacao)
- D: Produto (e.g. "PETR4 - PETROLEO BRASILEIRO S/A - PETROBRAS" or "Tesouro Selic 2029")
- E: Instituicao
- F: Quantidade
- G: Preco unitario
- H: Valor da Operacao

- [ ] **Step 1: Write the failing test**

```rust
// crates/core/tests/b3_parser_test.rs
use investimentos_core::parsers::b3;
use investimentos_core::types::*;
use std::path::Path;

#[test]
fn test_parse_b3_xlsx() {
    let path = Path::new("../../input-files/b3/movimentacao-2026-04-05-19-45-03.xlsx");
    let result = b3::parse(path).unwrap();

    assert!(!result.transactions.is_empty());
    assert!(!result.income.is_empty());

    // Should have stock transactions
    let stocks: Vec<_> = result.transactions.iter()
        .filter(|t| t.asset_type == AssetType::StockBr)
        .collect();
    assert!(!stocks.is_empty());

    // Should have Tesouro Direto transactions
    let tesouro: Vec<_> = result.transactions.iter()
        .filter(|t| t.asset_type == AssetType::Tesouro)
        .collect();
    assert!(!tesouro.is_empty());

    // Should have dividend income
    let dividends: Vec<_> = result.income.iter()
        .filter(|i| i.income_type == IncomeType::Dividend)
        .collect();
    assert!(!dividends.is_empty());

    // Should have JCP income
    let jcp: Vec<_> = result.income.iter()
        .filter(|i| i.income_type == IncomeType::Jcp)
        .collect();
    assert!(!jcp.is_empty());

    // PETR4 dividend should exist
    let petr4_div = result.income.iter()
        .find(|i| i.symbol == "PETR4")
        .unwrap();
    assert_eq!(petr4_div.currency, "BRL");
    assert_eq!(petr4_div.brl_rate, 1.0);
}

#[test]
fn test_b3_symbol_extraction() {
    // "CMIG3 - CIA. ENERGETICA DE MINAS GERAIS- CEMIG" -> "CMIG3"
    assert_eq!(b3::extract_symbol("CMIG3 - CIA. ENERGETICA DE MINAS GERAIS- CEMIG"), "CMIG3");
    // "Tesouro Selic 2029" -> "Tesouro Selic 2029"
    assert_eq!(b3::extract_symbol("Tesouro Selic 2029"), "Tesouro Selic 2029");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package investimentos-core test_parse_b3_xlsx`
Expected: FAIL — `b3` module not found.

- [ ] **Step 3: Implement the B3 parser**

```rust
// crates/core/src/parsers/b3.rs
use crate::types::*;
use calamine::{open_workbook, Reader, Xlsx};
use chrono::NaiveDate;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug)]
pub struct B3ImportResult {
    pub transactions: Vec<Transaction>,
    pub income: Vec<Income>,
}

pub fn parse(path: &Path) -> Result<B3ImportResult, Box<dyn std::error::Error>> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    let sheet_name = workbook.sheet_names().first()
        .ok_or("No sheets in workbook")?.clone();
    let range = workbook.worksheet_range(&sheet_name)?;

    let mut transactions = Vec::new();
    let mut income = Vec::new();

    for (i, row) in range.rows().enumerate() {
        if i == 0 { continue; } // skip header

        let direction = cell_str(&row[0]);
        let date_str = cell_str(&row[1]);
        let movimentacao = cell_str(&row[2]);
        let produto = cell_str(&row[3]);
        let _instituicao = cell_str(&row[4]);
        let quantidade = cell_f64(&row[5]);
        let preco_unitario = cell_f64(&row[6]);
        let valor_operacao = cell_f64(&row[7]);

        let date = parse_b3_date(&date_str)?;
        let symbol = extract_symbol(&produto);
        let asset_type = classify_asset(&produto);

        let hash_input = format!("b3:{}:{}:{}:{}:{}", date_str, movimentacao, produto, quantidade, preco_unitario);
        let hash = compute_hash(&hash_input);

        match movimentacao.as_str() {
            "Compra" => {
                transactions.push(Transaction {
                    id: None,
                    source: Source::B3,
                    asset_type: asset_type.clone(),
                    symbol: symbol.clone(),
                    tx_type: TxType::Buy,
                    date,
                    quantity: quantidade,
                    unit_price: Some(preco_unitario),
                    currency: "BRL".to_string(),
                    total_value: valor_operacao,
                    brl_rate: 1.0,
                    total_brl: valor_operacao,
                    commission: None,
                    fee_brl: None,
                    notes: None,
                    import_hash: hash,
                });
            }
            "Venda" | "Transferência - Liquidação" => {
                transactions.push(Transaction {
                    id: None,
                    source: Source::B3,
                    asset_type: asset_type.clone(),
                    symbol: symbol.clone(),
                    tx_type: TxType::Sell,
                    date,
                    quantity: quantidade,
                    unit_price: Some(preco_unitario),
                    currency: "BRL".to_string(),
                    total_value: valor_operacao,
                    brl_rate: 1.0,
                    total_brl: valor_operacao,
                    commission: None,
                    fee_brl: None,
                    notes: if movimentacao == "Transferência - Liquidação" {
                        Some("transferencia_liquidacao".to_string())
                    } else {
                        None
                    },
                    import_hash: hash,
                });
            }
            "Leilão de Fração" => {
                transactions.push(Transaction {
                    id: None,
                    source: Source::B3,
                    asset_type: asset_type.clone(),
                    symbol: symbol.clone(),
                    tx_type: TxType::FractionAuction,
                    date,
                    quantity: quantidade,
                    unit_price: Some(preco_unitario),
                    currency: "BRL".to_string(),
                    total_value: valor_operacao,
                    brl_rate: 1.0,
                    total_brl: valor_operacao,
                    commission: None,
                    fee_brl: None,
                    notes: None,
                    import_hash: hash,
                });
            }
            "Dividendo" => {
                income.push(Income {
                    id: None,
                    source: Source::B3,
                    symbol: symbol.clone(),
                    date,
                    income_type: IncomeType::Dividend,
                    currency: "BRL".to_string(),
                    gross_value: valor_operacao,
                    tax_withheld: None, // Dividends are tax-exempt in Brazil
                    tax_origin: Some("BR".to_string()),
                    brl_rate: 1.0,
                    net_value_brl: valor_operacao,
                    import_hash: hash,
                });
            }
            "Juros Sobre Capital Próprio" => {
                let tax = valor_operacao * 0.15; // 15% IR withheld at source
                income.push(Income {
                    id: None,
                    source: Source::B3,
                    symbol: symbol.clone(),
                    date,
                    income_type: IncomeType::Jcp,
                    currency: "BRL".to_string(),
                    gross_value: valor_operacao,
                    tax_withheld: Some(tax),
                    tax_origin: Some("BR".to_string()),
                    brl_rate: 1.0,
                    net_value_brl: valor_operacao - tax,
                    import_hash: hash,
                });
            }
            _ => {
                // Unknown transaction type — skip
            }
        }
    }

    Ok(B3ImportResult { transactions, income })
}

pub fn extract_symbol(produto: &str) -> String {
    if produto.starts_with("Tesouro") {
        return produto.to_string();
    }
    // "PETR4 - PETROLEO BRASILEIRO S/A - PETROBRAS" -> "PETR4"
    match produto.find(" - ") {
        Some(pos) => produto[..pos].trim().to_string(),
        None => produto.trim().to_string(),
    }
}

fn classify_asset(produto: &str) -> AssetType {
    if produto.starts_with("Tesouro") {
        AssetType::Tesouro
    } else {
        AssetType::StockBr
    }
}

fn parse_b3_date(s: &str) -> Result<NaiveDate, Box<dyn std::error::Error>> {
    Ok(NaiveDate::parse_from_str(s, "%d/%m/%Y")?)
}

fn cell_str(cell: &calamine::Data) -> String {
    match cell {
        calamine::Data::String(s) => s.clone(),
        calamine::Data::Float(f) => f.to_string(),
        calamine::Data::Int(i) => i.to_string(),
        _ => String::new(),
    }
}

fn cell_f64(cell: &calamine::Data) -> f64 {
    match cell {
        calamine::Data::Float(f) => *f,
        calamine::Data::Int(i) => *i as f64,
        calamine::Data::String(s) => s.replace(',', ".").parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn compute_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
```

- [ ] **Step 4: Update parsers/mod.rs**

```rust
// crates/core/src/parsers/mod.rs
pub mod b3;
pub mod binance;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --package investimentos-core b3`
Expected: both tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/parsers/b3.rs crates/core/src/parsers/mod.rs crates/core/tests/b3_parser_test.rs
git commit -m "feat: B3 XLSX parser — stocks, Tesouro Direto, dividends, JCP"
```

---

### Task 5: BCB PTAX API Client

**Files:**
- Create: `crates/core/src/api/mod.rs`
- Create: `crates/core/src/api/bcb_ptax.rs`
- Create: `crates/core/tests/bcb_ptax_test.rs`
- Modify: `crates/core/src/lib.rs`

BCB OLINDA API. No auth required. Date format: `dd-MM-yyyy`.

- USD endpoint: `CotacaoDolarDia(dataCotacao=@dataCotacao)?@dataCotacao='dd-MM-yyyy'`
- Other currencies: `CotacaoMoedaPeriodo(moeda=@moeda,dataInicial=@di,dataFinal=@df)`

- [ ] **Step 1: Write the failing test**

```rust
// crates/core/tests/bcb_ptax_test.rs
use investimentos_core::api::bcb_ptax;
use chrono::NaiveDate;

#[tokio::test]
async fn test_fetch_usd_brl_rate() {
    // Fetch USD/BRL for a known past business day
    let date = NaiveDate::from_ymd_opt(2024, 12, 20).unwrap();
    let rate = bcb_ptax::fetch_rate("USD", date).await.unwrap();
    // USD/BRL on 2024-12-20 was around 6.07
    assert!(rate > 5.0 && rate < 7.0, "USD/BRL rate was {}", rate);
}

#[tokio::test]
async fn test_fetch_eur_brl_rate() {
    let date = NaiveDate::from_ymd_opt(2024, 12, 20).unwrap();
    let rate = bcb_ptax::fetch_rate("EUR", date).await.unwrap();
    // EUR/BRL on 2024-12-20 was around 6.30
    assert!(rate > 5.0 && rate < 8.0, "EUR/BRL rate was {}", rate);
}

#[tokio::test]
async fn test_fetch_rate_weekend_falls_back() {
    // 2024-12-21 is a Saturday — BCB has no rate. Should fall back to previous business day.
    let date = NaiveDate::from_ymd_opt(2024, 12, 21).unwrap();
    let rate = bcb_ptax::fetch_rate("USD", date).await.unwrap();
    assert!(rate > 5.0 && rate < 7.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package investimentos-core test_fetch_usd_brl_rate`
Expected: FAIL — `api` module not found.

- [ ] **Step 3: Implement BCB PTAX client**

```rust
// crates/core/src/api/mod.rs
pub mod bcb_ptax;
```

```rust
// crates/core/src/api/bcb_ptax.rs
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;

const BASE_URL: &str = "https://olinda.bcb.gov.br/olinda/servico/PTAX/versao/v1/odata";

#[derive(Debug, Deserialize)]
struct PtaxResponse {
    value: Vec<PtaxQuote>,
}

#[derive(Debug, Deserialize)]
struct PtaxQuote {
    #[serde(alias = "cotacaoVenda")]
    cotacao_venda: f64,
}

/// Fetch the PTAX sell rate for a currency against BRL on a given date.
/// For weekends/holidays, tries up to 5 previous days.
pub async fn fetch_rate(currency: &str, date: NaiveDate) -> Result<f64, Box<dyn std::error::Error>> {
    let client = Client::new();

    // Try the given date, then up to 5 previous days (for weekends/holidays)
    for days_back in 0..6 {
        let try_date = date - chrono::Duration::days(days_back);
        let result = fetch_rate_for_date(&client, currency, try_date).await?;
        if let Some(rate) = result {
            return Ok(rate);
        }
    }

    Err(format!("No PTAX rate found for {} near {}", currency, date).into())
}

/// Fetch rates for a currency over a date range. Returns (date, rate) pairs.
pub async fn fetch_rates_range(
    currency: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<(NaiveDate, f64)>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let from_str = from.format("%d-%m-%Y").to_string();
    let to_str = to.format("%d-%m-%Y").to_string();

    let url = if currency == "USD" {
        format!(
            "{}/CotacaoDolarPeriodo(dataInicial=@di,dataFinal=@df)?@di='{}'&@df='{}'&$format=json",
            BASE_URL, from_str, to_str
        )
    } else {
        format!(
            "{}/CotacaoMoedaPeriodo(moeda=@moeda,dataInicial=@di,dataFinal=@df)?@moeda='{}'&@di='{}'&@df='{}'&$format=json",
            BASE_URL, currency, from_str, to_str
        )
    };

    let resp: PtaxPeriodResponse = client.get(&url).send().await?.json().await?;
    let mut rates = Vec::new();
    for quote in resp.value {
        if let Some(date) = parse_bcb_datetime(&quote.data_hora_cotacao) {
            rates.push((date, quote.cotacao_venda));
        }
    }
    Ok(rates)
}

#[derive(Debug, Deserialize)]
struct PtaxPeriodResponse {
    value: Vec<PtaxPeriodQuote>,
}

#[derive(Debug, Deserialize)]
struct PtaxPeriodQuote {
    #[serde(alias = "cotacaoVenda")]
    cotacao_venda: f64,
    #[serde(alias = "dataHoraCotacao")]
    data_hora_cotacao: String,
}

fn parse_bcb_datetime(s: &str) -> Option<NaiveDate> {
    // BCB returns dates like "2024-12-20 13:08:14.792"
    let date_part = s.split(' ').next()?;
    NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()
}

async fn fetch_rate_for_date(
    client: &Client,
    currency: &str,
    date: NaiveDate,
) -> Result<Option<f64>, Box<dyn std::error::Error>> {
    let date_str = date.format("%d-%m-%Y").to_string();

    let url = if currency == "USD" {
        format!(
            "{}/CotacaoDolarDia(dataCotacao=@dataCotacao)?@dataCotacao='{}'&$format=json",
            BASE_URL, date_str
        )
    } else {
        // For non-USD, use period query with same start/end date
        format!(
            "{}/CotacaoMoedaPeriodo(moeda=@moeda,dataInicial=@di,dataFinal=@df)?@moeda='{}'&@di='{}'&@df='{}'&$format=json",
            BASE_URL, currency, date_str, date_str
        )
    };

    let resp: PtaxResponse = client.get(&url).send().await?.json().await?;
    Ok(resp.value.last().map(|q| q.cotacao_venda))
}
```

- [ ] **Step 4: Update lib.rs**

```rust
// crates/core/src/lib.rs
pub mod api;
pub mod db;
pub mod parsers;
pub mod types;

pub use types::*;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --package investimentos-core bcb_ptax`
Expected: all 3 tests PASS (requires internet connection).

Note: these are integration tests that hit the real BCB API. They should pass reliably since BCB is a government API with no auth. If they fail, check internet connectivity.

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/api/ crates/core/tests/bcb_ptax_test.rs crates/core/src/lib.rs
git commit -m "feat: BCB PTAX API client — USD and multi-currency rate fetching"
```

---

## Phase 3: API Clients (Tasks 6-7)

### Task 6: Yahoo Finance + CoinGecko API Clients

**Files:**
- Create: `crates/core/src/api/yahoo.rs`
- Create: `crates/core/src/api/coingecko.rs`
- Create: `crates/core/tests/yahoo_test.rs`
- Create: `crates/core/tests/coingecko_test.rs`
- Modify: `crates/core/src/api/mod.rs`

- [ ] **Step 1: Write failing test for Yahoo Finance**

```rust
// crates/core/tests/yahoo_test.rs
use investimentos_core::api::yahoo;
use chrono::NaiveDate;

#[tokio::test]
async fn test_fetch_current_price_us_stock() {
    let price = yahoo::fetch_current_price("LUNR").await.unwrap();
    assert!(price > 0.0, "LUNR price should be positive: {}", price);
}

#[tokio::test]
async fn test_fetch_current_price_br_stock() {
    let price = yahoo::fetch_current_price("PETR4.SA").await.unwrap();
    assert!(price > 0.0, "PETR4 price should be positive: {}", price);
}

#[tokio::test]
async fn test_fetch_current_price_gold() {
    let price = yahoo::fetch_current_price("GC=F").await.unwrap();
    assert!(price > 1000.0, "Gold should be above $1000: {}", price);
}

#[tokio::test]
async fn test_fetch_historical_prices() {
    let from = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    let to = NaiveDate::from_ymd_opt(2024, 12, 20).unwrap();
    let prices = yahoo::fetch_history("PETR4.SA", from, to).await.unwrap();
    assert!(!prices.is_empty(), "Should have historical prices");
    // ~14 business days in this range
    assert!(prices.len() >= 10 && prices.len() <= 20);
}
```

- [ ] **Step 2: Implement Yahoo Finance client**

```rust
// crates/core/src/api/yahoo.rs
use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use yahoo_finance_api as yahoo;

/// Fetch the latest closing price for a symbol.
pub async fn fetch_current_price(symbol: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let provider = yahoo::YahooConnector::new()?;
    let resp = provider.get_latest_quotes(symbol, "1d").await?;
    let quote = resp.last_quote()?;
    Ok(quote.close)
}

/// Fetch daily closing prices for a symbol over a date range.
/// Returns (date, close_price) pairs.
pub async fn fetch_history(
    symbol: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<(NaiveDate, f64)>, Box<dyn std::error::Error>> {
    let provider = yahoo::YahooConnector::new()?;

    let start = from.and_hms_opt(0, 0, 0).unwrap();
    let end = to.and_hms_opt(23, 59, 59).unwrap();

    let start_utc = Utc.from_utc_datetime(&start);
    let end_utc = Utc.from_utc_datetime(&end);

    let resp = provider
        .get_quote_history(symbol, start_utc, end_utc)
        .await?;
    let quotes = resp.quotes()?;

    let mut prices = Vec::new();
    for q in quotes {
        let dt = NaiveDateTime::from_timestamp_opt(q.timestamp as i64, 0);
        if let Some(dt) = dt {
            prices.push((dt.date(), q.close));
        }
    }
    Ok(prices)
}

/// Map our internal symbols to Yahoo Finance tickers.
pub fn to_yahoo_symbol(symbol: &str, asset_type: &str) -> String {
    match asset_type {
        "stock_br" => {
            if symbol.ends_with(".SA") {
                symbol.to_string()
            } else {
                format!("{}.SA", symbol)
            }
        }
        "gold" => "GC=F".to_string(),
        _ => symbol.to_string(),
    }
}
```

- [ ] **Step 3: Run Yahoo tests**

Run: `cargo test --package investimentos-core yahoo`
Expected: all 4 tests PASS (requires internet).

- [ ] **Step 4: Write failing test for CoinGecko**

```rust
// crates/core/tests/coingecko_test.rs
use investimentos_core::api::coingecko;
use chrono::NaiveDate;

#[tokio::test]
async fn test_fetch_btc_brl_current() {
    let price = coingecko::fetch_btc_brl_current().await.unwrap();
    assert!(price > 100_000.0, "BTC/BRL should be above R$100k: {}", price);
}

#[tokio::test]
async fn test_fetch_btc_brl_history() {
    let from = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    let to = NaiveDate::from_ymd_opt(2024, 12, 20).unwrap();
    let prices = coingecko::fetch_btc_brl_history(from, to).await.unwrap();
    assert!(!prices.is_empty(), "Should have BTC price history");
    assert!(prices.len() >= 15);
}
```

- [ ] **Step 5: Implement CoinGecko client**

```rust
// crates/core/src/api/coingecko.rs
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;

const BASE_URL: &str = "https://api.coingecko.com/api/v3";

/// Fetch current BTC/BRL price.
pub async fn fetch_btc_brl_current() -> Result<f64, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!(
        "{}/simple/price?ids=bitcoin&vs_currencies=brl",
        BASE_URL
    );
    let resp: SimplePrice = client.get(&url).send().await?.json().await?;
    Ok(resp.bitcoin.brl)
}

/// Fetch BTC/BRL daily prices over a date range.
/// Returns (date, price) pairs.
pub async fn fetch_btc_brl_history(
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<(NaiveDate, f64)>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let from_ts = from.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    let to_ts = to.and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp();

    let url = format!(
        "{}/coins/bitcoin/market_chart/range?vs_currency=brl&from={}&to={}",
        BASE_URL, from_ts, to_ts
    );

    let resp: MarketChartResponse = client.get(&url).send().await?.json().await?;
    let mut prices = Vec::new();
    for point in resp.prices {
        let ts = (point[0] / 1000.0) as i64;
        if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
            let date = dt.date_naive();
            let price = point[1];
            // CoinGecko may return multiple points per day — keep last per day
            if prices.last().map_or(true, |(d, _): &(NaiveDate, f64)| *d != date) {
                prices.push((date, price));
            } else if let Some(last) = prices.last_mut() {
                last.1 = price;
            }
        }
    }
    Ok(prices)
}

#[derive(Debug, Deserialize)]
struct SimplePrice {
    bitcoin: BtcPrice,
}

#[derive(Debug, Deserialize)]
struct BtcPrice {
    brl: f64,
}

#[derive(Debug, Deserialize)]
struct MarketChartResponse {
    prices: Vec<Vec<f64>>,
}
```

- [ ] **Step 6: Update api/mod.rs**

```rust
// crates/core/src/api/mod.rs
pub mod bcb_ptax;
pub mod coingecko;
pub mod yahoo;
```

- [ ] **Step 7: Run CoinGecko tests**

Run: `cargo test --package investimentos-core coingecko`
Expected: PASS (requires internet). Note: CoinGecko free tier may rate-limit if tests run too frequently.

- [ ] **Step 8: Commit**

```bash
git add crates/core/src/api/ crates/core/tests/yahoo_test.rs crates/core/tests/coingecko_test.rs
git commit -m "feat: Yahoo Finance + CoinGecko API clients for stock and BTC prices"
```

---

### Task 7: IBKR Flex Client

**Files:**
- Create: `crates/core/src/api/ibkr_flex.rs`
- Create: `crates/core/src/parsers/ibkr_flex.rs`
- Create: `crates/core/tests/ibkr_flex_test.rs`
- Modify: `crates/core/src/api/mod.rs`
- Modify: `crates/core/src/parsers/mod.rs`

The `ib-flex` crate handles both fetching (via `api-client` feature) and XML parsing. We wrap it to produce our domain types.

Note: Integration tests for the Flex API fetch require a real IBKR token + query ID, so we test the parser with a mock XML fixture. The fetch client is thin enough to trust the `ib-flex` crate.

- [ ] **Step 1: Write failing test for IBKR Flex parser**

First, create a minimal XML fixture. The `ib-flex` crate expects Flex Query XML format. We'll test with a small inline fixture.

```rust
// crates/core/tests/ibkr_flex_test.rs
use investimentos_core::parsers::ibkr_flex;
use investimentos_core::types::*;

#[test]
fn test_classify_ibkr_currency() {
    assert_eq!(ibkr_flex::classify_ibkr_asset("LUNR", "USD"), AssetType::StockIntl);
    assert_eq!(ibkr_flex::classify_ibkr_asset("NKT", "DKK"), AssetType::StockIntl);
    assert_eq!(ibkr_flex::classify_ibkr_asset("2QT", "EUR"), AssetType::StockIntl);
}

#[test]
fn test_parse_ibkr_dividend_description() {
    let (symbol, per_share) = ibkr_flex::parse_dividend_description(
        "RILY(US05580M1080) Cash Dividend USD 0.50 per Share (Ordinary Dividend)"
    );
    assert_eq!(symbol, "RILY");
    assert_eq!(per_share, "0.50");
}

#[test]
fn test_parse_ibkr_tax_description() {
    let (symbol, origin) = ibkr_flex::parse_tax_description(
        "NVO(US6701002056) Cash Dividend USD 0.516901 per Share - DK Tax"
    );
    assert_eq!(symbol, "NVO");
    assert_eq!(origin, "DK");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package investimentos-core test_classify_ibkr`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement IBKR Flex parser helpers and fetch client**

```rust
// crates/core/src/parsers/ibkr_flex.rs
use crate::types::*;
use chrono::NaiveDate;
use sha2::{Digest, Sha256};

/// Classify an IBKR stock by its trading currency.
pub fn classify_ibkr_asset(symbol: &str, currency: &str) -> AssetType {
    // All IBKR assets are international from a Brazilian investor's perspective
    let _ = (symbol, currency);
    AssetType::StockIntl
}

/// Parse dividend description to extract symbol and per-share amount.
/// Input: "RILY(US05580M1080) Cash Dividend USD 0.50 per Share (Ordinary Dividend)"
/// Output: ("RILY", "0.50")
pub fn parse_dividend_description(desc: &str) -> (String, String) {
    let symbol = desc.split('(').next().unwrap_or("").trim().to_string();
    let per_share = desc
        .split("USD ")
        .nth(1)
        .and_then(|s| s.split(" per").next())
        .unwrap_or("")
        .trim()
        .to_string();
    (symbol, per_share)
}

/// Parse withholding tax description to extract symbol and tax origin country.
/// Input: "NVO(US6701002056) Cash Dividend USD 0.516901 per Share - DK Tax"
/// Output: ("NVO", "DK")
pub fn parse_tax_description(desc: &str) -> (String, String) {
    let symbol = desc.split('(').next().unwrap_or("").trim().to_string();
    let origin = desc
        .rsplit("- ")
        .next()
        .and_then(|s| s.strip_suffix(" Tax"))
        .unwrap_or("")
        .trim()
        .to_string();
    (symbol, origin)
}

/// Convert IBKR Flex trade data into our Transaction type.
pub fn trade_to_transaction(
    symbol: &str,
    currency: &str,
    date: NaiveDate,
    quantity: f64,
    trade_price: f64,
    proceeds: f64,
    commission: f64,
    brl_rate: f64,
) -> Transaction {
    let is_buy = quantity > 0.0;
    let qty_abs = quantity.abs();
    let total_value = proceeds.abs();

    let hash_input = format!(
        "ibkr:trade:{}:{}:{}:{}",
        symbol,
        date.format("%Y-%m-%d"),
        quantity,
        trade_price
    );

    Transaction {
        id: None,
        source: Source::Ibkr,
        asset_type: classify_ibkr_asset(symbol, currency),
        symbol: symbol.to_string(),
        tx_type: if is_buy { TxType::Buy } else { TxType::Sell },
        date,
        quantity: qty_abs,
        unit_price: Some(trade_price),
        currency: currency.to_string(),
        total_value,
        brl_rate,
        total_brl: total_value * brl_rate,
        commission: if commission != 0.0 { Some(commission.abs()) } else { None },
        fee_brl: None,
        notes: None,
        import_hash: compute_hash(&hash_input),
    }
}

/// Convert IBKR dividend + withholding tax into our Income type.
pub fn dividend_to_income(
    symbol: &str,
    currency: &str,
    date: NaiveDate,
    gross_amount: f64,
    tax_withheld: f64,
    tax_origin: &str,
    brl_rate: f64,
) -> Income {
    let net = gross_amount - tax_withheld.abs();
    let hash_input = format!(
        "ibkr:div:{}:{}:{}",
        symbol,
        date.format("%Y-%m-%d"),
        gross_amount
    );

    Income {
        id: None,
        source: Source::Ibkr,
        symbol: symbol.to_string(),
        date,
        income_type: IncomeType::Dividend,
        currency: currency.to_string(),
        gross_value: gross_amount,
        tax_withheld: if tax_withheld != 0.0 { Some(tax_withheld.abs()) } else { None },
        tax_origin: if tax_origin.is_empty() { None } else { Some(tax_origin.to_string()) },
        brl_rate,
        net_value_brl: net * brl_rate,
        import_hash: compute_hash(&hash_input),
    }
}

fn compute_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
```

```rust
// crates/core/src/api/ibkr_flex.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FlexError {
    #[error("Flex API request failed: {0}")]
    RequestFailed(String),
    #[error("No token or query ID configured")]
    NotConfigured,
}

/// Fetch a Flex statement XML from IBKR.
/// Two-step flow: SendRequest -> GetStatement.
/// Uses the `ib-flex` crate's api-client feature.
pub async fn fetch_flex_statement(
    token: &str,
    query_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if token.is_empty() || query_id.is_empty() {
        return Err(FlexError::NotConfigured.into());
    }

    let client = reqwest::Client::new();

    // Step 1: Send request to get reference code
    let send_url = format!(
        "https://ndcdyn.interactivebrokers.com/AccountManagement/FlexWebService/SendRequest?t={}&q={}&v=3",
        token, query_id
    );
    let send_resp = client
        .get(&send_url)
        .header("User-Agent", "investimentos-v2/0.1")
        .send()
        .await?
        .text()
        .await?;

    let reference_code = extract_reference_code(&send_resp)
        .ok_or_else(|| FlexError::RequestFailed(format!("Could not parse reference code from: {}", send_resp)))?;

    // Step 2: Wait briefly, then fetch the statement
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let get_url = format!(
        "https://ndcdyn.interactivebrokers.com/AccountManagement/FlexWebService/GetStatement?t={}&q={}&v=3",
        token, reference_code
    );

    // Retry up to 3 times (IBKR may need time to generate the report)
    for attempt in 0..3 {
        let get_resp = client
            .get(&get_url)
            .header("User-Agent", "investimentos-v2/0.1")
            .send()
            .await?
            .text()
            .await?;

        if get_resp.contains("<FlexQueryResponse") || get_resp.contains("<FlexStatementResponse") {
            return Ok(get_resp);
        }

        if attempt < 2 {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }

    Err(FlexError::RequestFailed("Statement not ready after retries".to_string()).into())
}

fn extract_reference_code(xml: &str) -> Option<String> {
    // Simple XML extraction: <ReferenceCode>12345</ReferenceCode>
    let start = xml.find("<ReferenceCode>")? + "<ReferenceCode>".len();
    let end = xml[start..].find("</ReferenceCode>")?;
    Some(xml[start..start + end].to_string())
}
```

- [ ] **Step 4: Update mod.rs files**

```rust
// crates/core/src/api/mod.rs
pub mod bcb_ptax;
pub mod coingecko;
pub mod ibkr_flex;
pub mod yahoo;
```

```rust
// crates/core/src/parsers/mod.rs
pub mod b3;
pub mod binance;
pub mod ibkr_flex;
```

- [ ] **Step 5: Run tests**

Run: `cargo test --package investimentos-core ibkr_flex`
Expected: all 3 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/api/ibkr_flex.rs crates/core/src/parsers/ibkr_flex.rs crates/core/tests/ibkr_flex_test.rs crates/core/src/api/mod.rs crates/core/src/parsers/mod.rs
git commit -m "feat: IBKR Flex fetch client + trade/dividend parser helpers"
```

---

## Phase 4: Business Logic (Tasks 8-10)

### Task 8: Portfolio Computation

**Files:**
- Create: `crates/core/src/portfolio/mod.rs`
- Create: `crates/core/src/portfolio/positions.rs`
- Create: `crates/core/tests/portfolio_test.rs`
- Modify: `crates/core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
// crates/core/tests/portfolio_test.rs
use investimentos_core::db::{queries, Database};
use investimentos_core::portfolio;
use investimentos_core::types::*;
use chrono::NaiveDate;

fn make_tx(symbol: &str, asset_type: AssetType, tx_type: TxType, qty: f64, price: f64, currency: &str, brl_rate: f64, date: &str) -> Transaction {
    let total = qty * price;
    Transaction {
        id: None,
        source: Source::Ibkr,
        asset_type,
        symbol: symbol.to_string(),
        tx_type,
        date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
        quantity: qty,
        unit_price: Some(price),
        currency: currency.to_string(),
        total_value: total,
        brl_rate,
        total_brl: total * brl_rate,
        commission: None,
        fee_brl: None,
        notes: None,
        import_hash: format!("test:{}:{}:{}", symbol, date, qty),
    }
}

#[test]
fn test_compute_positions() {
    let db = Database::open_in_memory().unwrap();

    // Buy 10 LUNR at $20, PTAX 5.0
    queries::insert_transaction(&db, &make_tx("LUNR", AssetType::StockIntl, TxType::Buy, 10.0, 20.0, "USD", 5.0, "2024-06-01")).unwrap();
    // Buy 5 more LUNR at $30, PTAX 5.5
    queries::insert_transaction(&db, &make_tx("LUNR", AssetType::StockIntl, TxType::Buy, 5.0, 30.0, "USD", 5.5, "2024-09-01")).unwrap();
    // Sell 3 LUNR at $25
    queries::insert_transaction(&db, &make_tx("LUNR", AssetType::StockIntl, TxType::Sell, 3.0, 25.0, "USD", 5.2, "2024-10-01")).unwrap();

    // Buy 100 PETR4 at R$35
    queries::insert_transaction(&db, &make_tx("PETR4", AssetType::StockBr, TxType::Buy, 100.0, 35.0, "BRL", 1.0, "2024-07-01")).unwrap();

    // Insert current prices
    queries::upsert_daily_price(&db, &DailyPrice {
        symbol: "LUNR".to_string(),
        date: NaiveDate::from_ymd_opt(2024, 12, 20).unwrap(),
        close_price: 40.0,
        currency: "USD".to_string(),
        brl_rate: 6.0,
    }).unwrap();
    queries::upsert_daily_price(&db, &DailyPrice {
        symbol: "PETR4".to_string(),
        date: NaiveDate::from_ymd_opt(2024, 12, 20).unwrap(),
        close_price: 38.0,
        currency: "BRL".to_string(),
        brl_rate: 1.0,
    }).unwrap();

    let positions = portfolio::compute_positions(&db).unwrap();

    // LUNR: net 12 shares (10 + 5 - 3)
    let lunr = positions.iter().find(|p| p.symbol == "LUNR").unwrap();
    assert_eq!(lunr.quantity, 12.0);
    // Current value: 12 * 40 * 6.0 = 2880 BRL
    assert!((lunr.current_value_brl.unwrap() - 2880.0).abs() < 0.01);

    // PETR4: net 100 shares
    let petr4 = positions.iter().find(|p| p.symbol == "PETR4").unwrap();
    assert_eq!(petr4.quantity, 100.0);
    // Current value: 100 * 38 * 1.0 = 3800 BRL
    assert!((petr4.current_value_brl.unwrap() - 3800.0).abs() < 0.01);
}

#[test]
fn test_compute_allocations() {
    let positions = vec![
        Position {
            symbol: "LUNR".to_string(),
            asset_type: AssetType::StockIntl,
            quantity: 12.0,
            avg_cost: 23.33,
            avg_cost_brl: 119.97,
            currency: "USD".to_string(),
            current_price: Some(40.0),
            current_brl_rate: Some(6.0),
            current_value_brl: Some(2880.0),
            pnl_brl: Some(1440.0),
            pnl_pct: Some(100.0),
            weight: None,
        },
        Position {
            symbol: "PETR4".to_string(),
            asset_type: AssetType::StockBr,
            quantity: 100.0,
            avg_cost: 35.0,
            avg_cost_brl: 35.0,
            currency: "BRL".to_string(),
            current_price: Some(38.0),
            current_brl_rate: Some(1.0),
            current_value_brl: Some(3800.0),
            pnl_brl: Some(300.0),
            pnl_pct: Some(8.57),
            weight: None,
        },
    ];

    let allocations = portfolio::compute_allocations(&positions);
    let total: f64 = allocations.iter().map(|a| a.value_brl).sum();
    assert!((total - 6680.0).abs() < 0.01);

    let intl = allocations.iter().find(|a| a.asset_type == AssetType::StockIntl).unwrap();
    assert!((intl.weight - 43.11).abs() < 1.0); // ~43%
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package investimentos-core test_compute_positions`
Expected: FAIL — `portfolio` module not found.

- [ ] **Step 3: Implement portfolio computation**

```rust
// crates/core/src/portfolio/mod.rs
pub mod positions;

pub use positions::{compute_allocations, compute_positions};
```

```rust
// crates/core/src/portfolio/positions.rs
use crate::db::{queries, schema::Database};
use crate::types::*;
use rusqlite::Result;
use std::collections::HashMap;

pub fn compute_positions(db: &Database) -> Result<Vec<Position>> {
    let transactions = queries::get_all_transactions(db)?;

    // Group transactions by symbol
    let mut by_symbol: HashMap<String, Vec<&Transaction>> = HashMap::new();
    for tx in &transactions {
        by_symbol.entry(tx.symbol.clone()).or_default().push(tx);
    }

    let mut positions = Vec::new();

    for (symbol, txs) in &by_symbol {
        let mut net_qty = 0.0_f64;
        let mut total_cost_orig = 0.0_f64;
        let mut total_cost_brl = 0.0_f64;
        let mut asset_type = txs[0].asset_type.clone();
        let mut currency = txs[0].currency.clone();

        for tx in txs {
            match tx.tx_type {
                TxType::Buy | TxType::FractionAuction => {
                    net_qty += tx.quantity;
                    total_cost_orig += tx.total_value;
                    total_cost_brl += tx.total_brl;
                }
                TxType::Sell => {
                    // Reduce position proportionally
                    if net_qty > 0.0 {
                        let fraction_sold = tx.quantity / net_qty;
                        total_cost_orig -= total_cost_orig * fraction_sold;
                        total_cost_brl -= total_cost_brl * fraction_sold;
                    }
                    net_qty -= tx.quantity;
                }
                TxType::Send => {
                    // For crypto sends (transfers to own wallet), don't reduce position.
                    // Fees are already accounted for in the Binance parser.
                }
                _ => {}
            }
        }

        if net_qty.abs() < 0.00001 {
            continue; // Position fully closed
        }

        let avg_cost = if net_qty > 0.0 { total_cost_orig / net_qty } else { 0.0 };
        let avg_cost_brl = if net_qty > 0.0 { total_cost_brl / net_qty } else { 0.0 };

        // Get latest price
        let latest = queries::get_latest_price(db, symbol)?;

        let (current_price, current_brl_rate, current_value_brl, pnl_brl, pnl_pct) =
            if let Some(ref price) = latest {
                let value_brl = net_qty * price.close_price * price.brl_rate;
                let pnl = value_brl - total_cost_brl;
                let pct = if total_cost_brl > 0.0 { (pnl / total_cost_brl) * 100.0 } else { 0.0 };
                (Some(price.close_price), Some(price.brl_rate), Some(value_brl), Some(pnl), Some(pct))
            } else {
                (None, None, None, None, None)
            };

        positions.push(Position {
            symbol: symbol.clone(),
            asset_type,
            quantity: net_qty,
            avg_cost,
            avg_cost_brl,
            currency,
            current_price,
            current_brl_rate,
            current_value_brl,
            pnl_brl,
            pnl_pct,
            weight: None, // Set after all positions computed
        });
    }

    // Compute weights
    let total_value: f64 = positions.iter()
        .filter_map(|p| p.current_value_brl)
        .sum();

    if total_value > 0.0 {
        for pos in &mut positions {
            if let Some(val) = pos.current_value_brl {
                pos.weight = Some((val / total_value) * 100.0);
            }
        }
    }

    // Sort by value descending
    positions.sort_by(|a, b| {
        b.current_value_brl.unwrap_or(0.0)
            .partial_cmp(&a.current_value_brl.unwrap_or(0.0))
            .unwrap()
    });

    Ok(positions)
}

pub fn compute_allocations(positions: &[Position]) -> Vec<Allocation> {
    let mut by_type: HashMap<String, f64> = HashMap::new();

    for pos in positions {
        let val = pos.current_value_brl.unwrap_or(0.0);
        *by_type.entry(pos.asset_type.as_str().to_string()).or_default() += val;
    }

    let total: f64 = by_type.values().sum();

    let mut allocations: Vec<Allocation> = by_type.into_iter()
        .map(|(type_str, value_brl)| {
            let weight = if total > 0.0 { (value_brl / total) * 100.0 } else { 0.0 };
            Allocation {
                asset_type: AssetType::from_str(&type_str).unwrap_or(AssetType::StockBr),
                value_brl,
                weight,
            }
        })
        .collect();

    allocations.sort_by(|a, b| b.value_brl.partial_cmp(&a.value_brl).unwrap());
    allocations
}
```

- [ ] **Step 4: Update lib.rs**

```rust
// crates/core/src/lib.rs
pub mod api;
pub mod db;
pub mod parsers;
pub mod portfolio;
pub mod types;

pub use types::*;
```

- [ ] **Step 5: Run tests**

Run: `cargo test --package investimentos-core portfolio`
Expected: both tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/portfolio/ crates/core/tests/portfolio_test.rs crates/core/src/lib.rs
git commit -m "feat: portfolio position and allocation computation"
```

---

### Task 9: Reconciliation (Startup Price Fetch + Backfill)

**Files:**
- Create: `crates/core/src/reconcile.rs`
- Modify: `crates/core/src/lib.rs`

No unit test for this module — it orchestrates API calls + DB writes. We test it manually or through integration tests.

- [ ] **Step 1: Implement reconciliation logic**

```rust
// crates/core/src/reconcile.rs
use crate::api::{bcb_ptax, coingecko, yahoo};
use crate::db::{queries, schema::Database};
use crate::types::*;
use chrono::{Local, NaiveDate};

#[derive(Debug)]
pub struct ReconcileResult {
    pub prices_fetched_today: usize,
    pub prices_backfilled: usize,
    pub rate_limited: bool,
}

/// Phase 1: Fetch today's prices for all held symbols.
pub async fn fetch_current_prices(db: &Database) -> Result<usize, Box<dyn std::error::Error>> {
    let symbols = queries::get_distinct_symbols(db)?;
    let today = Local::now().date_naive();
    let mut count = 0;

    for (symbol, asset_type, currency) in &symbols {
        let result = fetch_and_store_price(db, symbol, asset_type, currency, today).await;
        match result {
            Ok(()) => count += 1,
            Err(e) => {
                eprintln!("Warning: could not fetch price for {}: {}", symbol, e);
            }
        }
    }

    // Also fetch BTC if we have crypto transactions
    let has_btc = symbols.iter().any(|(_, at, _)| at == "crypto");
    if has_btc {
        match fetch_and_store_btc_price(db, today).await {
            Ok(()) => count += 1,
            Err(e) => eprintln!("Warning: could not fetch BTC price: {}", e),
        }
    }

    Ok(count)
}

/// Phase 2: Backfill historical prices, oldest gaps first.
/// Stops gracefully on rate limits and returns progress.
pub async fn backfill_prices(db: &Database) -> Result<ReconcileResult, Box<dyn std::error::Error>> {
    let symbols = queries::get_distinct_symbols(db)?;
    let today = Local::now().date_naive();
    let mut total_backfilled = 0;
    let mut rate_limited = false;

    for (symbol, asset_type, currency) in &symbols {
        if rate_limited { break; }

        let last_date = queries::get_latest_price_date(db, &symbol)?;
        let start = match last_date {
            Some(d) => d + chrono::Duration::days(1),
            None => {
                // Find earliest transaction date for this symbol
                let txs = queries::get_transactions_by_symbol(db, &symbol)?;
                txs.first().map(|t| t.date).unwrap_or(today)
            }
        };

        if start >= today {
            continue; // Already up to date
        }

        let result = backfill_symbol(db, &symbol, &asset_type, &currency, start, today).await;
        match result {
            Ok(n) => total_backfilled += n,
            Err(e) => {
                let err_str = e.to_string().to_lowercase();
                if err_str.contains("429") || err_str.contains("rate") || err_str.contains("limit") {
                    rate_limited = true;
                    eprintln!("Rate limited while backfilling {}. Will resume next launch.", symbol);
                } else {
                    eprintln!("Warning: backfill failed for {}: {}", symbol, e);
                }
            }
        }
    }

    Ok(ReconcileResult {
        prices_fetched_today: 0, // Set by caller
        prices_backfilled: total_backfilled,
        rate_limited,
    })
}

async fn fetch_and_store_price(
    db: &Database,
    symbol: &str,
    asset_type: &str,
    currency: &str,
    date: NaiveDate,
) -> Result<(), Box<dyn std::error::Error>> {
    let yahoo_sym = yahoo::to_yahoo_symbol(symbol, asset_type);
    let price = yahoo::fetch_current_price(&yahoo_sym).await?;

    let brl_rate = if currency == "BRL" {
        1.0
    } else {
        bcb_ptax::fetch_rate(currency, date).await.unwrap_or(1.0)
    };

    queries::upsert_daily_price(db, &DailyPrice {
        symbol: symbol.to_string(),
        date,
        close_price: price,
        currency: currency.to_string(),
        brl_rate,
    })?;

    Ok(())
}

async fn fetch_and_store_btc_price(
    db: &Database,
    date: NaiveDate,
) -> Result<(), Box<dyn std::error::Error>> {
    let price = coingecko::fetch_btc_brl_current().await?;
    queries::upsert_daily_price(db, &DailyPrice {
        symbol: "BTC".to_string(),
        date,
        close_price: price,
        currency: "BRL".to_string(),
        brl_rate: 1.0, // Already in BRL
    })?;
    Ok(())
}

async fn backfill_symbol(
    db: &Database,
    symbol: &str,
    asset_type: &str,
    currency: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;

    if asset_type == "crypto" && symbol == "BTC" {
        // Use CoinGecko for BTC
        let prices = coingecko::fetch_btc_brl_history(from, to).await?;
        for (date, price) in &prices {
            queries::upsert_daily_price(db, &DailyPrice {
                symbol: "BTC".to_string(),
                date: *date,
                close_price: *price,
                currency: "BRL".to_string(),
                brl_rate: 1.0,
            })?;
            count += 1;
        }
    } else {
        // Use Yahoo Finance for stocks + gold
        let yahoo_sym = yahoo::to_yahoo_symbol(symbol, asset_type);
        let prices = yahoo::fetch_history(&yahoo_sym, from, to).await?;

        // Fetch PTAX rates for the same range if non-BRL
        let brl_rates = if currency != "BRL" {
            bcb_ptax::fetch_rates_range(currency, from, to).await.unwrap_or_default()
        } else {
            Vec::new()
        };

        let rate_map: std::collections::HashMap<NaiveDate, f64> = brl_rates.into_iter().collect();

        for (date, close_price) in &prices {
            let brl_rate = if currency == "BRL" {
                1.0
            } else {
                // Find closest rate (same day or most recent before)
                find_closest_rate(&rate_map, *date).unwrap_or(1.0)
            };

            queries::upsert_daily_price(db, &DailyPrice {
                symbol: symbol.to_string(),
                date: *date,
                close_price: *close_price,
                currency: currency.to_string(),
                brl_rate,
            })?;
            count += 1;
        }
    }

    Ok(count)
}

fn find_closest_rate(
    rate_map: &std::collections::HashMap<NaiveDate, f64>,
    target: NaiveDate,
) -> Option<f64> {
    // Try exact date first, then up to 5 days back
    for days_back in 0..6 {
        let check = target - chrono::Duration::days(days_back);
        if let Some(&rate) = rate_map.get(&check) {
            return Some(rate);
        }
    }
    None
}
```

- [ ] **Step 2: Update lib.rs**

```rust
// crates/core/src/lib.rs
pub mod api;
pub mod db;
pub mod parsers;
pub mod portfolio;
pub mod reconcile;
pub mod types;

pub use types::*;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build --package investimentos-core`
Expected: compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add crates/core/src/reconcile.rs crates/core/src/lib.rs
git commit -m "feat: startup reconciliation — fetch current prices, then backfill with rate-limit recovery"
```

---

### Task 10: JSON Export/Import

**Files:**
- Create: `crates/core/src/export.rs`
- Create: `crates/core/tests/export_test.rs`
- Modify: `crates/core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
// crates/core/tests/export_test.rs
use investimentos_core::db::{queries, Database};
use investimentos_core::export;
use investimentos_core::types::*;
use chrono::NaiveDate;
use tempfile::NamedTempFile;

#[test]
fn test_export_and_import_roundtrip() {
    let db = Database::open_in_memory().unwrap();

    // Insert a transaction
    let tx = Transaction {
        id: None,
        source: Source::B3,
        asset_type: AssetType::StockBr,
        symbol: "PETR4".to_string(),
        tx_type: TxType::Buy,
        date: NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(),
        quantity: 100.0,
        unit_price: Some(35.0),
        currency: "BRL".to_string(),
        total_value: 3500.0,
        brl_rate: 1.0,
        total_brl: 3500.0,
        commission: None,
        fee_brl: None,
        notes: None,
        import_hash: "test_export_001".to_string(),
    };
    queries::insert_transaction(&db, &tx).unwrap();

    // Insert a price
    queries::upsert_daily_price(&db, &DailyPrice {
        symbol: "PETR4".to_string(),
        date: NaiveDate::from_ymd_opt(2024, 12, 20).unwrap(),
        close_price: 38.0,
        currency: "BRL".to_string(),
        brl_rate: 1.0,
    }).unwrap();

    // Insert config
    queries::set_config(&db, "ibkr_flex_token", "test_token").unwrap();

    // Export to file
    let tmp = NamedTempFile::new().unwrap();
    export::export_to_json(&db, tmp.path()).unwrap();

    // Import into a fresh DB
    let db2 = Database::open_in_memory().unwrap();
    export::import_from_json(&db2, tmp.path()).unwrap();

    // Verify data
    let txs = queries::get_all_transactions(&db2).unwrap();
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].symbol, "PETR4");

    let price = queries::get_latest_price(&db2, "PETR4").unwrap().unwrap();
    assert_eq!(price.close_price, 38.0);

    let token = queries::get_config(&db2, "ibkr_flex_token").unwrap().unwrap();
    assert_eq!(token, "test_token");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package investimentos-core test_export_and_import_roundtrip`
Expected: FAIL — `export` module not found.

- [ ] **Step 3: Implement export/import**

```rust
// crates/core/src/export.rs
use crate::db::{queries, schema::Database};
use crate::types::*;
use chrono::Utc;
use std::path::Path;

pub fn export_to_json(db: &Database, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let transactions = queries::get_all_transactions(db)?;
    let income = queries::get_all_income(db)?;

    // Get all daily prices
    let symbols = queries::get_distinct_symbols(db)?;
    let mut daily_prices = Vec::new();
    for (symbol, _, _) in &symbols {
        let earliest = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let latest = chrono::NaiveDate::from_ymd_opt(2099, 12, 31).unwrap();
        let prices = queries::get_daily_prices(db, symbol, earliest, latest)?;
        daily_prices.extend(prices);
    }
    // Also get BTC prices
    {
        let earliest = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let latest = chrono::NaiveDate::from_ymd_opt(2099, 12, 31).unwrap();
        let prices = queries::get_daily_prices(db, "BTC", earliest, latest)?;
        daily_prices.extend(prices);
    }

    // Get config
    let mut config = std::collections::HashMap::new();
    for key in &["ibkr_flex_token", "ibkr_flex_query_id", "coingecko_api_key"] {
        if let Some(value) = queries::get_config(db, key)? {
            config.insert(key.to_string(), value);
        }
    }

    let export = ExportData {
        version: 1,
        exported_at: Utc::now().to_rfc3339(),
        transactions,
        daily_prices,
        income,
        config,
    };

    let json = serde_json::to_string_pretty(&export)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn import_from_json(db: &Database, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(path)?;
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
```

- [ ] **Step 4: Update lib.rs**

```rust
// crates/core/src/lib.rs
pub mod api;
pub mod db;
pub mod export;
pub mod parsers;
pub mod portfolio;
pub mod reconcile;
pub mod types;

pub use types::*;
```

- [ ] **Step 5: Run test**

Run: `cargo test --package investimentos-core test_export_and_import_roundtrip`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/export.rs crates/core/tests/export_test.rs crates/core/src/lib.rs
git commit -m "feat: JSON export/import with full roundtrip support"
```

---

## Phase 5: GPUI-CE UI (Tasks 11-15)

**Note:** GPUI-CE (v0.3) has minimal docs. The code below is based on Zed's source patterns and the gpui-ce README. Expect to consult GPUI source/examples and iterate. The UI tasks are less rigidly TDD — manual visual testing is the primary verification.

### Task 11: GPUI-CE App Shell + Theme + Navigation

**Files:**
- Modify: `crates/ui/Cargo.toml`
- Modify: `crates/ui/src/main.rs`
- Create: `crates/ui/src/app.rs`
- Create: `crates/ui/src/theme.rs`

- [ ] **Step 1: Update UI Cargo.toml with all dependencies**

```toml
# crates/ui/Cargo.toml
[package]
name = "investimentos-ui"
version = "0.1.0"
edition = "2021"

[dependencies]
investimentos-core = { path = "../core" }
gpui = { package = "gpui-ce", version = "0.3" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

- [ ] **Step 2: Create theme module**

```rust
// crates/ui/src/theme.rs
use gpui::Hsla;

pub const BG_PRIMARY: Hsla = Hsla { h: 232.0 / 360.0, s: 0.45, l: 0.13, a: 1.0 };
pub const BG_SECONDARY: Hsla = Hsla { h: 219.0 / 360.0, s: 0.45, l: 0.16, a: 1.0 };
pub const TEXT_PRIMARY: Hsla = Hsla { h: 0.0, s: 0.0, l: 0.88, a: 1.0 };
pub const TEXT_SECONDARY: Hsla = Hsla { h: 0.0, s: 0.0, l: 0.53, a: 1.0 };
pub const ACCENT: Hsla = Hsla { h: 231.0 / 360.0, s: 0.78, l: 0.72, a: 1.0 };
pub const GREEN: Hsla = Hsla { h: 142.0 / 360.0, s: 0.69, l: 0.58, a: 1.0 };
pub const RED: Hsla = Hsla { h: 0.0, s: 0.84, l: 0.60, a: 1.0 };
pub const YELLOW: Hsla = Hsla { h: 38.0 / 360.0, s: 0.92, l: 0.50, a: 1.0 };
pub const PURPLE: Hsla = Hsla { h: 258.0 / 360.0, s: 0.58, l: 0.59, a: 1.0 };
pub const BORDER: Hsla = Hsla { h: 240.0 / 360.0, s: 0.20, l: 0.22, a: 1.0 };
```

- [ ] **Step 3: Create app state and navigation**

```rust
// crates/ui/src/app.rs
use gpui::*;
use crate::theme;
use investimentos_core::db::schema::Database;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Overview,
    Positions,
    Income,
    History,
}

pub struct AppState {
    pub db: Arc<Database>,
    pub active_tab: Tab,
}

impl AppState {
    pub fn new(db_path: PathBuf) -> Self {
        let db = Database::open(&db_path).expect("Failed to open database");
        Self {
            db: Arc::new(db),
            active_tab: Tab::Overview,
        }
    }
}

pub struct AppRoot {
    state: AppState,
}

impl AppRoot {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            state: AppState::new(db_path),
        }
    }
}

impl Render for AppRoot {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(theme::BG_PRIMARY)
            .text_color(theme::TEXT_PRIMARY)
            .flex()
            .flex_col()
            .child(self.render_nav(cx))
            .child(self.render_content(cx))
    }
}

impl AppRoot {
    fn render_nav(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let tabs = [
            (Tab::Overview, "Overview"),
            (Tab::Positions, "Positions"),
            (Tab::Income, "Income"),
            (Tab::History, "History"),
        ];

        div()
            .flex()
            .justify_between()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(theme::BORDER)
            .child(
                div().flex().gap_4().children(
                    tabs.map(|(tab, label)| {
                        let is_active = self.state.active_tab == tab;
                        div()
                            .cursor_pointer()
                            .px_2()
                            .py_1()
                            .text_sm()
                            .when(is_active, |el| {
                                el.text_color(theme::ACCENT)
                                    .border_b_2()
                                    .border_color(theme::ACCENT)
                            })
                            .when(!is_active, |el| {
                                el.text_color(theme::TEXT_SECONDARY)
                            })
                            .on_mouse_down(MouseButton::Left, move |_, cx| {
                                // Will be wired to tab switching
                            })
                            .child(label)
                    })
                )
            )
            .child(
                div().flex().gap_2().children([
                    self.render_button("Import"),
                    self.render_button("+ Gold"),
                    self.render_button("Export"),
                ])
            )
    }

    fn render_button(&self, label: &str) -> impl IntoElement {
        div()
            .cursor_pointer()
            .px_3()
            .py_1()
            .text_xs()
            .text_color(theme::TEXT_SECONDARY)
            .border_1()
            .border_color(theme::BORDER)
            .rounded_md()
            .child(label.to_string())
    }

    fn render_content(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .p_6()
            .child(match self.state.active_tab {
                Tab::Overview => "Overview — coming next".to_string(),
                Tab::Positions => "Positions — coming soon".to_string(),
                Tab::Income => "Income — coming soon".to_string(),
                Tab::History => "History — coming soon".to_string(),
            })
    }
}
```

- [ ] **Step 4: Wire up main.rs**

```rust
// crates/ui/src/main.rs
mod app;
mod theme;

use app::AppRoot;
use gpui::*;
use std::path::PathBuf;

fn main() {
    App::new().run(|cx: &mut AppContext| {
        // DB lives next to the executable or in a known location
        let db_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("investimentos-v2")
            .join("data.db");

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Default::default(),
                    size: size(px(1200.0), px(800.0)),
                })),
                titlebar: Some(TitlebarOptions {
                    title: Some("Investimentos v2".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |cx| cx.new_view(|_| AppRoot::new(db_path)),
        )
        .unwrap();
    });
}
```

Note: `dirs` crate may be needed. Add to `crates/ui/Cargo.toml`:

```toml
dirs = "5"
```

- [ ] **Step 5: Verify it compiles and launches**

Run: `cargo run --package investimentos-ui`
Expected: a dark-themed window opens with the navigation bar visible. The exact GPUI API may need adjustments — consult gpui-ce examples if compilation fails and fix API mismatches.

- [ ] **Step 6: Commit**

```bash
git add crates/ui/
git commit -m "feat: GPUI-CE app shell with dark theme and tab navigation"
```

---

### Task 12: Overview View

**Files:**
- Create: `crates/ui/src/views/mod.rs`
- Create: `crates/ui/src/views/overview.rs`
- Create: `crates/ui/src/components/mod.rs`
- Modify: `crates/ui/src/app.rs`

This is the landing page: total portfolio value, allocation breakdown, mini chart placeholder, and top holdings table. Reads computed data from core.

- [ ] **Step 1: Create views module and overview view**

The overview view should:
1. Call `portfolio::compute_positions(&db)` and `portfolio::compute_allocations(&positions)`
2. Display total value, 30d change
3. Show allocation by asset type with colored labels
4. Show top holdings table

```rust
// crates/ui/src/views/mod.rs
pub mod overview;
```

```rust
// crates/ui/src/views/overview.rs
use gpui::*;
use investimentos_core::db::schema::Database;
use investimentos_core::portfolio;
use investimentos_core::types::*;
use crate::theme;
use std::sync::Arc;

pub struct OverviewView {
    db: Arc<Database>,
    positions: Vec<Position>,
    allocations: Vec<Allocation>,
    total_value_brl: f64,
}

impl OverviewView {
    pub fn new(db: Arc<Database>) -> Self {
        let positions = portfolio::compute_positions(&db).unwrap_or_default();
        let allocations = portfolio::compute_allocations(&positions);
        let total_value_brl: f64 = positions.iter()
            .filter_map(|p| p.current_value_brl)
            .sum();

        Self { db, positions, allocations, total_value_brl }
    }

    pub fn render(&self, cx: &mut ViewContext<impl Render>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(self.render_total())
            .child(self.render_allocation_and_chart())
            .child(self.render_holdings_table())
    }

    fn render_total(&self) -> impl IntoElement {
        div()
            .child(
                div()
                    .text_xs()
                    .text_color(theme::TEXT_SECONDARY)
                    .child("TOTAL PORTFOLIO")
            )
            .child(
                div()
                    .text_3xl()
                    .font_weight(FontWeight::BOLD)
                    .child(format!("R$ {}", format_brl(self.total_value_brl)))
            )
    }

    fn render_allocation_and_chart(&self) -> impl IntoElement {
        div()
            .flex()
            .gap_6()
            .child(self.render_allocation_panel())
            .child(self.render_chart_placeholder())
    }

    fn render_allocation_panel(&self) -> impl IntoElement {
        let mut children: Vec<Div> = Vec::new();
        for alloc in &self.allocations {
            let color = asset_type_color(&alloc.asset_type);
            let label = asset_type_label(&alloc.asset_type);
            children.push(
                div()
                    .flex()
                    .justify_between()
                    .py_1()
                    .child(
                        div().flex().gap_2().items_center()
                            .child(div().w_3().h_3().rounded_sm().bg(color))
                            .child(div().text_xs().child(label.to_string()))
                    )
                    .child(
                        div().text_xs().text_color(theme::TEXT_SECONDARY)
                            .child(format!("{:.1}%", alloc.weight))
                    )
            );
        }

        div()
            .flex_1()
            .bg(theme::BG_SECONDARY)
            .rounded_lg()
            .p_5()
            .child(
                div().text_xs().text_color(theme::TEXT_SECONDARY).mb_3()
                    .child("Allocation")
            )
            .children(children)
    }

    fn render_chart_placeholder(&self) -> impl IntoElement {
        div()
            .flex_1()
            .bg(theme::BG_SECONDARY)
            .rounded_lg()
            .p_5()
            .child(
                div().text_xs().text_color(theme::TEXT_SECONDARY)
                    .child("Portfolio Value (90d)")
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .h(px(100.0))
                    .text_color(theme::TEXT_SECONDARY)
                    .text_xs()
                    .child("Chart coming in Task 14")
            )
    }

    fn render_holdings_table(&self) -> impl IntoElement {
        let mut rows: Vec<Div> = Vec::new();

        for pos in self.positions.iter().take(10) {
            let pnl_color = if pos.pnl_pct.unwrap_or(0.0) >= 0.0 { theme::GREEN } else { theme::RED };
            let pnl_prefix = if pos.pnl_pct.unwrap_or(0.0) >= 0.0 { "+" } else { "" };

            rows.push(
                div()
                    .flex()
                    .py_2()
                    .border_b_1()
                    .border_color(theme::BORDER)
                    .child(div().flex_1().text_xs().font_weight(FontWeight::MEDIUM).child(pos.symbol.clone()))
                    .child(div().w(px(100.0)).text_xs().text_color(asset_type_color(&pos.asset_type)).child(asset_type_label(&pos.asset_type).to_string()))
                    .child(div().w(px(140.0)).text_xs().text_right().child(
                        if pos.currency == "BRL" { "—".to_string() }
                        else { format!("{} {}", pos.currency, format_number(pos.current_price.unwrap_or(0.0) * pos.quantity)) }
                    ))
                    .child(div().w(px(140.0)).text_xs().text_right().child(
                        format!("R$ {}", format_brl(pos.current_value_brl.unwrap_or(0.0)))
                    ))
                    .child(div().w(px(80.0)).text_xs().text_right().text_color(pnl_color).child(
                        format!("{}{:.1}%", pnl_prefix, pos.pnl_pct.unwrap_or(0.0))
                    ))
                    .child(div().w(px(60.0)).text_xs().text_right().text_color(theme::TEXT_SECONDARY).child(
                        format!("{:.1}%", pos.weight.unwrap_or(0.0))
                    ))
            );
        }

        div()
            .bg(theme::BG_SECONDARY)
            .rounded_lg()
            .p_5()
            .child(
                div().text_xs().text_color(theme::TEXT_SECONDARY).mb_3()
                    .child("Top Holdings")
            )
            .child(
                // Header
                div()
                    .flex()
                    .pb_2()
                    .border_b_1()
                    .border_color(theme::BORDER)
                    .text_color(theme::TEXT_SECONDARY)
                    .child(div().flex_1().text_xs().child("Symbol"))
                    .child(div().w(px(100.0)).text_xs().child("Type"))
                    .child(div().w(px(140.0)).text_xs().text_right().child("Value (orig)"))
                    .child(div().w(px(140.0)).text_xs().text_right().child("Value (BRL)"))
                    .child(div().w(px(80.0)).text_xs().text_right().child("P/L"))
                    .child(div().w(px(60.0)).text_xs().text_right().child("Weight"))
            )
            .children(rows)
    }
}

fn asset_type_color(at: &AssetType) -> Hsla {
    match at {
        AssetType::StockIntl => theme::ACCENT,
        AssetType::StockBr => theme::GREEN,
        AssetType::Tesouro => theme::YELLOW,
        AssetType::Crypto => theme::RED,
        AssetType::Gold => theme::PURPLE,
    }
}

fn asset_type_label(at: &AssetType) -> &'static str {
    match at {
        AssetType::StockIntl => "Stock Intl",
        AssetType::StockBr => "Stock BR",
        AssetType::Tesouro => "Tesouro",
        AssetType::Crypto => "Crypto",
        AssetType::Gold => "Gold",
    }
}

fn format_brl(v: f64) -> String {
    let abs = v.abs();
    let int_part = abs as u64;
    let dec_part = ((abs - int_part as f64) * 100.0).round() as u64;

    let int_str = format_with_dots(int_part);
    if v < 0.0 {
        format!("-{},{:02}", int_str, dec_part)
    } else {
        format!("{},{:02}", int_str, dec_part)
    }
}

fn format_number(v: f64) -> String {
    format!("{:.2}", v)
}

fn format_with_dots(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push('.');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
```

- [ ] **Step 2: Wire overview into app.rs content area**

Update the `render_content` method in `crates/ui/src/app.rs` to render the overview view when `Tab::Overview` is active.

- [ ] **Step 3: Verify it compiles and renders**

Run: `cargo run --package investimentos-ui`
Expected: overview tab shows total value (R$ 0,00 if no data imported yet), empty allocation panel, and empty holdings table.

- [ ] **Step 4: Commit**

```bash
git add crates/ui/src/views/ crates/ui/src/components/ crates/ui/src/app.rs
git commit -m "feat: overview landing page — total value, allocation, top holdings"
```

---

### Task 13: Positions + Income Views

**Files:**
- Create: `crates/ui/src/views/positions.rs`
- Create: `crates/ui/src/views/income.rs`
- Modify: `crates/ui/src/views/mod.rs`
- Modify: `crates/ui/src/app.rs`

- [ ] **Step 1: Implement positions view**

Full table of all holdings with columns: symbol, type, quantity, avg cost, current price, value (BRL), P/L, weight. Reuse the table layout pattern from overview.

```rust
// crates/ui/src/views/positions.rs
use gpui::*;
use investimentos_core::db::schema::Database;
use investimentos_core::portfolio;
use investimentos_core::types::*;
use crate::theme;
use crate::views::overview::{asset_type_color, asset_type_label, format_brl};
use std::sync::Arc;

pub struct PositionsView {
    positions: Vec<Position>,
}

impl PositionsView {
    pub fn new(db: &Database) -> Self {
        let positions = portfolio::compute_positions(db).unwrap_or_default();
        Self { positions }
    }

    pub fn render(&self, cx: &mut ViewContext<impl Render>) -> impl IntoElement {
        let mut rows: Vec<Div> = Vec::new();

        for pos in &self.positions {
            let pnl_color = if pos.pnl_pct.unwrap_or(0.0) >= 0.0 { theme::GREEN } else { theme::RED };
            let pnl_prefix = if pos.pnl_pct.unwrap_or(0.0) >= 0.0 { "+" } else { "" };

            rows.push(
                div()
                    .flex()
                    .py_2()
                    .border_b_1()
                    .border_color(theme::BORDER)
                    .child(div().w(px(100.0)).text_xs().font_weight(FontWeight::MEDIUM).child(pos.symbol.clone()))
                    .child(div().w(px(80.0)).text_xs().text_color(asset_type_color(&pos.asset_type)).child(asset_type_label(&pos.asset_type).to_string()))
                    .child(div().w(px(80.0)).text_xs().text_right().child(format!("{:.4}", pos.quantity)))
                    .child(div().w(px(100.0)).text_xs().text_right().child(format!("{} {:.2}", pos.currency, pos.avg_cost)))
                    .child(div().w(px(100.0)).text_xs().text_right().child(format!("R$ {:.2}", pos.avg_cost_brl)))
                    .child(div().w(px(100.0)).text_xs().text_right().child(
                        pos.current_price.map_or("—".to_string(), |p| format!("{:.2}", p))
                    ))
                    .child(div().w(px(120.0)).text_xs().text_right().child(
                        format!("R$ {}", format_brl(pos.current_value_brl.unwrap_or(0.0)))
                    ))
                    .child(div().w(px(100.0)).text_xs().text_right().text_color(pnl_color).child(
                        format!("{}{:.1}%  (R$ {})", pnl_prefix, pos.pnl_pct.unwrap_or(0.0), format_brl(pos.pnl_brl.unwrap_or(0.0)))
                    ))
                    .child(div().w(px(60.0)).text_xs().text_right().text_color(theme::TEXT_SECONDARY).child(
                        format!("{:.1}%", pos.weight.unwrap_or(0.0))
                    ))
            );
        }

        div()
            .flex()
            .flex_col()
            .child(
                // Header row
                div()
                    .flex()
                    .pb_2()
                    .border_b_1()
                    .border_color(theme::BORDER)
                    .text_color(theme::TEXT_SECONDARY)
                    .child(div().w(px(100.0)).text_xs().child("Symbol"))
                    .child(div().w(px(80.0)).text_xs().child("Type"))
                    .child(div().w(px(80.0)).text_xs().text_right().child("Qty"))
                    .child(div().w(px(100.0)).text_xs().text_right().child("Avg Cost"))
                    .child(div().w(px(100.0)).text_xs().text_right().child("Avg (BRL)"))
                    .child(div().w(px(100.0)).text_xs().text_right().child("Price"))
                    .child(div().w(px(120.0)).text_xs().text_right().child("Value (BRL)"))
                    .child(div().w(px(100.0)).text_xs().text_right().child("P/L"))
                    .child(div().w(px(60.0)).text_xs().text_right().child("Weight"))
            )
            .children(rows)
    }
}
```

- [ ] **Step 2: Implement income view**

```rust
// crates/ui/src/views/income.rs
use gpui::*;
use investimentos_core::db::{queries, schema::Database};
use investimentos_core::types::*;
use crate::theme;
use crate::views::overview::format_brl;

pub struct IncomeView {
    income: Vec<Income>,
}

impl IncomeView {
    pub fn new(db: &Database) -> Self {
        let income = queries::get_all_income(db).unwrap_or_default();
        Self { income }
    }

    pub fn render(&self, cx: &mut ViewContext<impl Render>) -> impl IntoElement {
        let mut rows: Vec<Div> = Vec::new();

        for inc in self.income.iter().rev() { // newest first
            let type_label = match inc.income_type {
                IncomeType::Dividend => "Dividend",
                IncomeType::Jcp => "JCP",
            };

            rows.push(
                div()
                    .flex()
                    .py_2()
                    .border_b_1()
                    .border_color(theme::BORDER)
                    .child(div().w(px(100.0)).text_xs().child(inc.date.format("%Y-%m-%d").to_string()))
                    .child(div().w(px(80.0)).text_xs().font_weight(FontWeight::MEDIUM).child(inc.symbol.clone()))
                    .child(div().w(px(60.0)).text_xs().child(inc.source.as_str().to_string()))
                    .child(div().w(px(70.0)).text_xs().child(type_label.to_string()))
                    .child(div().w(px(100.0)).text_xs().text_right().child(
                        format!("{} {:.2}", inc.currency, inc.gross_value)
                    ))
                    .child(div().w(px(80.0)).text_xs().text_right().text_color(theme::RED).child(
                        inc.tax_withheld.map_or("—".to_string(), |t| format!("-{:.2}", t))
                    ))
                    .child(div().w(px(40.0)).text_xs().text_center().child(
                        inc.tax_origin.clone().unwrap_or_default()
                    ))
                    .child(div().w(px(120.0)).text_xs().text_right().text_color(theme::GREEN).child(
                        format!("R$ {}", format_brl(inc.net_value_brl))
                    ))
            );
        }

        let total_net: f64 = self.income.iter().map(|i| i.net_value_brl).sum();

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div().text_sm().text_color(theme::TEXT_SECONDARY)
                    .child(format!("Total net income: R$ {}", format_brl(total_net)))
            )
            .child(
                div()
                    .flex()
                    .pb_2()
                    .border_b_1()
                    .border_color(theme::BORDER)
                    .text_color(theme::TEXT_SECONDARY)
                    .child(div().w(px(100.0)).text_xs().child("Date"))
                    .child(div().w(px(80.0)).text_xs().child("Symbol"))
                    .child(div().w(px(60.0)).text_xs().child("Source"))
                    .child(div().w(px(70.0)).text_xs().child("Type"))
                    .child(div().w(px(100.0)).text_xs().text_right().child("Gross"))
                    .child(div().w(px(80.0)).text_xs().text_right().child("Tax"))
                    .child(div().w(px(40.0)).text_xs().text_center().child("Origin"))
                    .child(div().w(px(120.0)).text_xs().text_right().child("Net (BRL)"))
            )
            .children(rows)
    }
}
```

- [ ] **Step 3: Update views/mod.rs and wire into app.rs**

```rust
// crates/ui/src/views/mod.rs
pub mod income;
pub mod overview;
pub mod positions;
```

Update `render_content` in `app.rs` to render the appropriate view per tab.

- [ ] **Step 4: Verify compilation and rendering**

Run: `cargo run --package investimentos-ui`
Expected: can switch between Overview, Positions, and Income tabs.

- [ ] **Step 5: Commit**

```bash
git add crates/ui/src/views/ crates/ui/src/app.rs
git commit -m "feat: positions table and income history views"
```

---

### Task 14: Import + Manual Entry + Settings Views

**Files:**
- Create: `crates/ui/src/views/import.rs`
- Create: `crates/ui/src/views/manual.rs`
- Create: `crates/ui/src/views/settings.rs`
- Modify: `crates/ui/src/views/mod.rs`
- Modify: `crates/ui/src/app.rs`

These views are triggered by the action buttons, not the tab bar. They can be implemented as modal-style overlays or as separate states in the app.

- [ ] **Step 1: Implement import view**

The import view shows two buttons (B3, Binance), opens a file picker, runs the parser, and shows results.

```rust
// crates/ui/src/views/import.rs
use gpui::*;
use investimentos_core::db::{queries, schema::Database};
use investimentos_core::parsers;
use crate::theme;
use std::sync::Arc;

pub struct ImportView {
    db: Arc<Database>,
    result_message: Option<String>,
}

impl ImportView {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db, result_message: None }
    }

    pub fn render(&self, cx: &mut ViewContext<impl Render>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .child(
                div().text_lg().font_weight(FontWeight::BOLD)
                    .child("Import Transactions")
            )
            .child(
                div().flex().gap_4()
                    .child(
                        div()
                            .cursor_pointer()
                            .px_6().py_3()
                            .bg(theme::BG_SECONDARY)
                            .rounded_lg()
                            .text_sm()
                            .child("Import B3 (.xlsx)")
                            // Wire to file picker + b3::parse
                    )
                    .child(
                        div()
                            .cursor_pointer()
                            .px_6().py_3()
                            .bg(theme::BG_SECONDARY)
                            .rounded_lg()
                            .text_sm()
                            .child("Import Binance (.csv)")
                            // Wire to file picker + binance::parse
                    )
            )
            .when_some(self.result_message.as_ref(), |el, msg| {
                el.child(
                    div().text_sm().text_color(theme::GREEN).child(msg.clone())
                )
            })
    }
}

/// Run B3 import: parse file, insert transactions and income into DB.
/// Returns (new_transactions, new_income, skipped).
pub fn run_b3_import(
    db: &Database,
    path: &std::path::Path,
) -> Result<(usize, usize, usize), Box<dyn std::error::Error>> {
    let result = parsers::b3::parse(path)?;
    let mut new_tx = 0;
    let mut new_inc = 0;
    let mut skipped = 0;

    for tx in &result.transactions {
        if queries::insert_transaction(db, tx)? { new_tx += 1; } else { skipped += 1; }
    }
    for inc in &result.income {
        if queries::insert_income(db, inc)? { new_inc += 1; } else { skipped += 1; }
    }

    Ok((new_tx, new_inc, skipped))
}

/// Run Binance import: parse file, insert transactions into DB.
/// Returns (new_transactions, skipped).
pub fn run_binance_import(
    db: &Database,
    path: &std::path::Path,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let result = parsers::binance::parse(path)?;
    let mut new_tx = 0;
    let mut skipped = 0;

    for tx in &result.transactions {
        if queries::insert_transaction(db, tx)? { new_tx += 1; } else { skipped += 1; }
    }

    Ok((new_tx, skipped))
}
```

- [ ] **Step 2: Implement manual gold entry view**

```rust
// crates/ui/src/views/manual.rs
use gpui::*;
use investimentos_core::db::{queries, schema::Database};
use investimentos_core::types::*;
use chrono::NaiveDate;
use sha2::{Digest, Sha256};
use crate::theme;
use std::sync::Arc;

pub struct ManualEntryView {
    db: Arc<Database>,
    date_input: String,
    quantity_input: String,
    price_input: String,
    result_message: Option<String>,
}

impl ManualEntryView {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            date_input: String::new(),
            quantity_input: String::new(),
            price_input: String::new(),
            result_message: None,
        }
    }

    pub fn render(&self, cx: &mut ViewContext<impl Render>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .child(div().text_lg().font_weight(FontWeight::BOLD).child("Add Gold Purchase"))
            .child(
                div().flex().flex_col().gap_2()
                    .child(div().text_xs().text_color(theme::TEXT_SECONDARY).child("Date (YYYY-MM-DD)"))
                    .child(div().text_xs().text_color(theme::TEXT_SECONDARY).child("Quantity (grams)"))
                    .child(div().text_xs().text_color(theme::TEXT_SECONDARY).child("Unit Price (BRL per gram)"))
                    // GPUI text inputs will be wired here — exact API depends on gpui-ce version
            )
            .when_some(self.result_message.as_ref(), |el, msg| {
                el.child(div().text_sm().text_color(theme::GREEN).child(msg.clone()))
            })
    }
}

/// Insert a manual gold transaction.
pub fn insert_gold_purchase(
    db: &Database,
    date: NaiveDate,
    quantity_grams: f64,
    price_per_gram_brl: f64,
) -> Result<bool, Box<dyn std::error::Error>> {
    let total = quantity_grams * price_per_gram_brl;
    let hash_input = format!("manual:gold:{}:{}:{}", date, quantity_grams, price_per_gram_brl);
    let mut hasher = Sha256::new();
    hasher.update(hash_input.as_bytes());
    let hash = hex::encode(hasher.finalize());

    let tx = Transaction {
        id: None,
        source: Source::Manual,
        asset_type: AssetType::Gold,
        symbol: "GOLD".to_string(),
        tx_type: TxType::Buy,
        date,
        quantity: quantity_grams,
        unit_price: Some(price_per_gram_brl),
        currency: "BRL".to_string(),
        total_value: total,
        brl_rate: 1.0,
        total_brl: total,
        commission: None,
        fee_brl: None,
        notes: None,
        import_hash: hash,
    };

    Ok(queries::insert_transaction(db, &tx)?)
}
```

- [ ] **Step 3: Implement settings view**

```rust
// crates/ui/src/views/settings.rs
use gpui::*;
use investimentos_core::db::{queries, schema::Database};
use crate::theme;
use std::sync::Arc;

pub struct SettingsView {
    db: Arc<Database>,
    ibkr_token: String,
    ibkr_query_id: String,
    coingecko_key: String,
}

impl SettingsView {
    pub fn new(db: Arc<Database>) -> Self {
        let ibkr_token = queries::get_config(&db, "ibkr_flex_token")
            .ok().flatten().unwrap_or_default();
        let ibkr_query_id = queries::get_config(&db, "ibkr_flex_query_id")
            .ok().flatten().unwrap_or_default();
        let coingecko_key = queries::get_config(&db, "coingecko_api_key")
            .ok().flatten().unwrap_or_default();

        Self { db, ibkr_token, ibkr_query_id, coingecko_key }
    }

    pub fn render(&self, cx: &mut ViewContext<impl Render>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .child(div().text_lg().font_weight(FontWeight::BOLD).child("Settings"))
            .child(
                div().flex().flex_col().gap_3()
                    .child(self.render_field("IBKR Flex Token", &self.ibkr_token))
                    .child(self.render_field("IBKR Flex Query ID", &self.ibkr_query_id))
                    .child(self.render_field("CoinGecko API Key", &self.coingecko_key))
            )
            // Save button will call queries::set_config for each field
    }

    fn render_field(&self, label: &str, value: &str) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(theme::TEXT_SECONDARY).child(label.to_string()))
            .child(
                div()
                    .px_3().py_2()
                    .bg(theme::BG_SECONDARY)
                    .rounded_md()
                    .text_xs()
                    .child(if value.is_empty() { "Not configured".to_string() } else { "••••••••".to_string() })
            )
    }
}
```

- [ ] **Step 4: Update views/mod.rs**

```rust
// crates/ui/src/views/mod.rs
pub mod import;
pub mod income;
pub mod manual;
pub mod overview;
pub mod positions;
pub mod settings;
```

- [ ] **Step 5: Wire action buttons in app.rs**

Update `app.rs` to handle Import/Gold/Export/Settings button clicks — either as modal overlays or by swapping the content area.

- [ ] **Step 6: Verify compilation**

Run: `cargo run --package investimentos-ui`
Expected: compiles and action buttons trigger the appropriate views.

- [ ] **Step 7: Commit**

```bash
git add crates/ui/src/views/ crates/ui/src/app.rs
git commit -m "feat: import, manual gold entry, and settings views"
```

---

### Task 15: History View + Startup Wiring

**Files:**
- Create: `crates/ui/src/views/history.rs`
- Modify: `crates/ui/src/views/mod.rs`
- Modify: `crates/ui/src/main.rs`
- Modify: `crates/ui/src/app.rs`

- [ ] **Step 1: Implement history view**

The history view shows a line chart of total portfolio value over time. For now, render using simple GPUI drawing primitives (lines). A full charting library can be added later.

```rust
// crates/ui/src/views/history.rs
use gpui::*;
use investimentos_core::db::{queries, schema::Database};
use investimentos_core::types::*;
use crate::theme;
use chrono::{Local, NaiveDate};
use std::sync::Arc;

pub struct HistoryView {
    data_points: Vec<(NaiveDate, f64)>, // (date, total_portfolio_brl)
}

impl HistoryView {
    pub fn new(db: &Database) -> Self {
        let data_points = compute_portfolio_history(db).unwrap_or_default();
        Self { data_points }
    }

    pub fn render(&self, cx: &mut ViewContext<impl Render>) -> impl IntoElement {
        if self.data_points.is_empty() {
            return div()
                .flex()
                .items_center()
                .justify_center()
                .h(px(400.0))
                .text_color(theme::TEXT_SECONDARY)
                .child("No historical data yet. Data fills in as the app fetches prices over time.");
        }

        let min_val = self.data_points.iter().map(|(_, v)| *v).fold(f64::MAX, f64::min);
        let max_val = self.data_points.iter().map(|(_, v)| *v).fold(f64::MIN, f64::max);

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div().flex().justify_between()
                    .child(div().text_sm().child("Portfolio Value Over Time"))
                    .child(div().text_xs().text_color(theme::TEXT_SECONDARY)
                        .child(format!("{} data points", self.data_points.len())))
            )
            .child(
                div()
                    .h(px(400.0))
                    .bg(theme::BG_SECONDARY)
                    .rounded_lg()
                    .p_4()
                    // Chart rendering will use GPUI canvas/paint when available
                    // For now, show text summary
                    .child(
                        div().flex().flex_col().gap_2()
                            .child(div().text_xs().text_color(theme::TEXT_SECONDARY)
                                .child(format!("Range: R$ {:.0} — R$ {:.0}", min_val, max_val)))
                            .child(div().text_xs().text_color(theme::TEXT_SECONDARY)
                                .child(format!("Period: {} to {}",
                                    self.data_points.first().map(|(d, _)| d.to_string()).unwrap_or_default(),
                                    self.data_points.last().map(|(d, _)| d.to_string()).unwrap_or_default(),
                                )))
                    )
            )
    }
}

/// Compute daily total portfolio value from daily_prices + transactions.
fn compute_portfolio_history(db: &Database) -> Result<Vec<(NaiveDate, f64)>, Box<dyn std::error::Error>> {
    let transactions = queries::get_all_transactions(db)?;
    let symbols = queries::get_distinct_symbols(db)?;

    if transactions.is_empty() {
        return Ok(Vec::new());
    }

    let earliest_tx = transactions.iter().map(|t| t.date).min().unwrap();
    let today = Local::now().date_naive();

    // For each date that has price data, compute total portfolio value
    let mut daily_totals: std::collections::BTreeMap<NaiveDate, f64> = std::collections::BTreeMap::new();

    for (symbol, _asset_type, _currency) in &symbols {
        let prices = queries::get_daily_prices(db, symbol, earliest_tx, today)?;

        // Compute holdings at each price date
        let symbol_txs: Vec<_> = transactions.iter()
            .filter(|t| t.symbol == *symbol)
            .collect();

        for price in &prices {
            let mut qty = 0.0_f64;
            for tx in &symbol_txs {
                if tx.date > price.date { break; }
                match tx.tx_type {
                    TxType::Buy | TxType::FractionAuction => qty += tx.quantity,
                    TxType::Sell => qty -= tx.quantity,
                    _ => {}
                }
            }
            let value = qty * price.close_price * price.brl_rate;
            *daily_totals.entry(price.date).or_default() += value;
        }
    }

    Ok(daily_totals.into_iter().collect())
}
```

- [ ] **Step 2: Wire startup reconciliation in main.rs**

Update `main.rs` to run `reconcile::fetch_current_prices` and `reconcile::backfill_prices` after the window opens, using a background async task.

```rust
// In main.rs, after opening the window:
// Spawn a background task for reconciliation
cx.spawn(|mut cx| async move {
    // Phase 1: fetch current prices
    let _ = investimentos_core::reconcile::fetch_current_prices(&db).await;
    // Phase 2: backfill (stops on rate limits)
    let _ = investimentos_core::reconcile::backfill_prices(&db).await;
    // Notify UI to refresh (implementation depends on GPUI-CE patterns)
}).detach();
```

The exact async spawning API depends on GPUI-CE. Consult examples and adjust.

- [ ] **Step 3: Update views/mod.rs**

```rust
// crates/ui/src/views/mod.rs
pub mod history;
pub mod import;
pub mod income;
pub mod manual;
pub mod overview;
pub mod positions;
pub mod settings;
```

- [ ] **Step 4: Wire history view into app.rs tab switching**

- [ ] **Step 5: Verify full app compiles and runs**

Run: `cargo run --package investimentos-ui`
Expected: all 4 tabs work, action buttons open views, startup triggers price fetch.

- [ ] **Step 6: Final commit**

```bash
git add crates/ui/
git commit -m "feat: history view + startup reconciliation wiring"
```

---

## Summary

| Phase | Tasks | What it delivers |
|---|---|---|
| 1: Foundation | 1-3 | Workspace, DB, Binance parser |
| 2: Parsers | 4-5 | B3 + BCB PTAX |
| 3: API Clients | 6-7 | Yahoo, CoinGecko, IBKR Flex |
| 4: Business Logic | 8-10 | Portfolio computation, reconciliation, export |
| 5: UI | 11-15 | Full GPUI-CE desktop app |

After Task 10, the entire core crate is functional and testable via unit/integration tests. Phase 5 adds the GUI on top.
