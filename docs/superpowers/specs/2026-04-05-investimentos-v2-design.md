# Investimentos v2 — Design Spec

Personal investment portfolio aggregator built with Rust + GPUI-CE + SQLite.

## Goal

A desktop app that aggregates all investment data (Brazilian stocks, international stocks, Tesouro Direto, BTC, gold) into a single view with:
- Total portfolio value in BRL
- Allocation breakdown by asset type
- Historical portfolio value graph (daily granularity)
- Per-position P/L in both original currency and BRL
- Dividend/JCP income tracking
- JSON export for backup

## Data Sources

| Source | Transactions | Daily Prices |
|---|---|---|
| IBKR | Flex Web Service API (automatic, token-based) | Yahoo Finance |
| B3 | XLSX import via file picker | Yahoo Finance (`.SA` suffix) |
| Binance | CSV import via file picker | CoinGecko (BTC/BRL) |
| Gold | Manual entry (form in UI) | Yahoo Finance (`GC=F`) + BCB PTAX |
| FX rates | — | BCB PTAX |

**Future:** DEGIRO import (parser TBD), Binance API (replaces CSV).

## Architecture

Cargo workspace with two crates:

```
investimentos-v2/
  Cargo.toml                (workspace)
  crates/
    core/                   (investimentos-core)
      src/
        lib.rs
        db/                 (SQLite schema, queries, migrations)
        parsers/            (ibkr_flex.rs, b3.rs, binance.rs)
        api/                (bcb_ptax.rs, yahoo.rs, coingecko.rs, ibkr_flex_client.rs)
        portfolio/          (position calc, P/L, allocation)
        export.rs           (JSON backup/restore)
        reconcile.rs        (startup price fetch + backfill)
    ui/                     (investimentos-ui)
      src/
        main.rs             (GPUI app entry, window setup)
        app.rs              (root app state, navigation)
        views/
          overview.rs       (landing page)
          positions.rs      (full position list with P/L)
          history.rs        (full-width historical graph)
          income.rs         (dividend/JCP income view)
          manual.rs         (gold entry form)
          import.rs         (file picker with source selection)
          settings.rs       (IBKR Flex token/query ID config)
        components/
          chart.rs          (line chart widget)
          table.rs          (sortable table widget)
          allocation.rs     (donut/pie breakdown widget)
        theme.rs
```

### Core crate (`investimentos-core`)

All business logic, no UI dependency. Testable independently.

**Parsers:**
- `ibkr_flex.rs` — Parse Flex Query XML via the `ib-flex` crate. Extracts trades, open positions, dividends, withholding tax.
- `b3.rs` — Parse B3 "Movimentacao" XLSX via `calamine`. Columns: Entrada/Saida, Data, Movimentacao (Compra, Venda, Dividendo, JCP, Leilao de Fracao, Transferencia - Liquidacao), Produto, Instituicao, Quantidade, Preco unitario, Valor da Operacao. Handles Tesouro Direto as portfolio holdings.
- `binance.rs` — Parse Binance CSV export. Columns: id, datetime, type (Buy, Send, Deposit), sent/received amounts and currencies, fees. Sends are transfers to the user's own wallet, not sales. BTC holdings = sum(received_amount from Buy txs) - sum(fee_amount from all txs).

**API clients:**
- `ibkr_flex_client.rs` — Two-step Flex Web Service flow: SendRequest (get reference code) → GetStatement (get XML). Uses stored token + query ID.
- `bcb_ptax.rs` — BCB OLINDA API. `CotacaoDolarDia` for USD/BRL on a specific date, `CotacaoMoedaPeriodo` for other currencies over a range. Date format: `dd-MM-yyyy`. No auth.
- `yahoo.rs` — `yahoo_finance_api` crate. Historical daily quotes for stocks (US, BR via `.SA`, EU) and gold (`GC=F`).
- `coingecko.rs` — CoinGecko API. `coins/bitcoin/market_chart` for BTC/BRL historical. Free tier: 30 calls/min, 10k/month, requires free API key.

**Portfolio:**
- Compute current holdings from transactions (net quantity per symbol).
- Cost basis uses BCB PTAX rate on trade date for foreign-currency transactions (BRL rate = 1.0 for B3 assets).
- Current value = latest price * current PTAX rate (for foreign assets) or latest price (for BRL assets).
- P/L = current value - cost basis, both in original currency and BRL.
- Allocation = group by asset_type, sum current BRL values.

**Reconciliation (startup price fetch):**
1. Fetch today's prices for all held symbols — portfolio overview works immediately.
2. Then backfill historical gaps: for each symbol, find last stored price date, fetch forward from there.
3. If rate-limited by any API, stop gracefully and persist progress. Next launch resumes from the last stored date.
4. If offline, skip entirely and use cached data.

**Export/Import:**
- Export: serialize all tables (transactions, daily_prices, income, manual_entries, config) to a single JSON file. Includes metadata (export date, app version).
- Import: restore from JSON file, merging or replacing data.
- User saves the exported file to their Dropbox/Drive folder manually.

### UI crate (`investimentos-ui`)

GPUI-CE desktop app. Dark theme. No network calls — all data access through core.

**Navigation:** Top tab bar with 4 views: Overview, Positions, Income, History.

**Action buttons:** Import, + Gold (manual entry), Export, Settings (gear icon).

**Views:**

1. **Overview (landing page):**
   - Total portfolio value in BRL with 30d change (amount + percentage)
   - Two-column layout: allocation donut (by asset type) + mini 90d line chart
   - Top holdings table: symbol, asset type, value in original currency, value in BRL, P/L %, portfolio weight %

2. **Positions:**
   - Full sortable table of every holding
   - Columns: symbol, asset type, quantity, avg cost (orig currency), avg cost (BRL), current price, current value (BRL), P/L (amount + %), weight
   - Filterable by asset type (Stock BR, Stock Intl, Tesouro, Crypto, Gold)

3. **Income:**
   - Table of dividend/JCP events: date, symbol, source (B3/IBKR), type (dividend/JCP), gross value, tax withheld, tax origin, net value in BRL
   - Monthly bar chart of income over time

4. **History:**
   - Full-width line chart of total portfolio value in BRL over time
   - Time range selector: 30d, 90d, 1y, All
   - Chart fills in progressively as backfill completes over multiple launches

5. **Import:**
   - User selects data source: B3, Binance (IBKR is automatic via Flex API)
   - Native file picker opens
   - Parser runs, shows count of new transactions imported
   - Deduplication: hash of (source, date, symbol, quantity, unit_price) prevents double-imports

6. **Manual Entry (Gold):**
   - Form: date, quantity (oz/g), unit price (BRL), total value (BRL)
   - Stored in transactions table with source=manual, asset_type=gold

7. **Settings:**
   - IBKR Flex token input
   - IBKR Flex query ID input
   - CoinGecko API key input

## Data Model

### transactions

| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | autoincrement |
| source | TEXT NOT NULL | `b3`, `ibkr`, `binance`, `manual` |
| asset_type | TEXT NOT NULL | `stock_br`, `stock_intl`, `tesouro`, `crypto`, `gold` |
| symbol | TEXT NOT NULL | `PETR4`, `LUNR`, `BTC`, `GOLD`, `Tesouro Selic 2029` |
| tx_type | TEXT NOT NULL | `buy`, `sell`, `dividend`, `jcp`, `fraction_auction`, `send`, `deposit` |
| date | TEXT NOT NULL | ISO 8601 (YYYY-MM-DD) |
| quantity | REAL NOT NULL | |
| unit_price | REAL | in original currency, nullable for sends |
| currency | TEXT NOT NULL | `BRL`, `USD`, `EUR`, `DKK`, `CAD`, `BTC` |
| total_value | REAL NOT NULL | in original currency |
| brl_rate | REAL NOT NULL | BCB PTAX on trade date; 1.0 for BRL assets |
| total_brl | REAL NOT NULL | total_value * brl_rate |
| commission | REAL | nullable, in original currency |
| fee_brl | REAL | nullable, fee in BRL (Binance reports fees in BRL) |
| notes | TEXT | nullable |
| import_hash | TEXT UNIQUE | deduplication hash |

### daily_prices

| Column | Type | Notes |
|---|---|---|
| symbol | TEXT NOT NULL | |
| date | TEXT NOT NULL | ISO 8601 |
| close_price | REAL NOT NULL | in original currency |
| currency | TEXT NOT NULL | |
| brl_rate | REAL NOT NULL | PTAX on that date |
| PRIMARY KEY | | (symbol, date) |

### income

| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | autoincrement |
| source | TEXT NOT NULL | `b3`, `ibkr` |
| symbol | TEXT NOT NULL | |
| date | TEXT NOT NULL | ISO 8601 |
| income_type | TEXT NOT NULL | `dividend`, `jcp` |
| currency | TEXT NOT NULL | `BRL`, `USD`, etc. |
| gross_value | REAL NOT NULL | in original currency |
| tax_withheld | REAL | nullable |
| tax_origin | TEXT | nullable — `US`, `DK`, `TW`, `BR` |
| brl_rate | REAL NOT NULL | PTAX on that date |
| net_value_brl | REAL NOT NULL | |
| import_hash | TEXT UNIQUE | deduplication |

### config

| Column | Type | Notes |
|---|---|---|
| key | TEXT PK | |
| value | TEXT | |

Keys: `ibkr_flex_token`, `ibkr_flex_query_id`, `coingecko_api_key`.

## Key Dependencies (Rust crates)

| Crate | Purpose |
|---|---|
| `gpui-ce` | UI framework |
| `rusqlite` | SQLite |
| `ib-flex` (with `api-client`) | IBKR Flex XML parse + fetch |
| `calamine` | XLSX parsing (B3) |
| `csv` | CSV parsing (Binance) |
| `yahoo_finance_api` | Stock + gold prices |
| `reqwest` | HTTP client (BCB, CoinGecko) |
| `serde` / `serde_json` | JSON serialization (export, API responses) |
| `chrono` | Date handling |
| `rust_decimal` | Financial precision (optional, can start with f64) |

## Import Flow

1. User clicks "Import" in UI
2. Selects data source: B3 or Binance
3. Native OS file picker opens (GPUI file dialog)
4. Core parser processes the file based on selected source
5. For foreign-currency transactions: fetch BCB PTAX rate for each unique trade date
6. Normalize to transaction records, compute import_hash
7. Insert into DB, skip duplicates (ON CONFLICT DO NOTHING on import_hash)
8. UI shows summary: "Imported X new transactions (Y skipped as duplicates)"

## Startup Flow

1. Open/create SQLite DB, run migrations
2. If IBKR Flex configured: fetch latest activity → parse → deduplicate → insert
3. Fetch today's prices for all held symbols (priority — makes overview work immediately)
4. Render UI with current data
5. Background: backfill historical price gaps, oldest first, stopping gracefully on rate limits
6. If offline: skip steps 2-3-5, render from cached data

## Export Format

Single JSON file:

```json
{
  "version": 1,
  "exported_at": "2026-04-05T20:00:00Z",
  "transactions": [...],
  "daily_prices": [...],
  "income": [...],
  "config": {...}
}
```
