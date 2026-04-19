# Dinheiros

A personal investment portfolio tracker for Brazilian and international assets. Native desktop app written in Rust with a local-first SQLite database — no cloud, no accounts, no tracking.

## Features

- **Multi-broker imports** — B3 (`.xlsx`), IBKR activity statement (`.csv`), Binance (`.csv`). Manual entry also supported.
- **Multi-asset** — Brazilian stocks/ETFs, international stocks, Tesouro Direto, crypto (BTC via Binance), gold.
- **BRL-first** — Every position is converted to BRL using historical PTAX rates for accurate cost basis and PnL.
- **Live prices** — Yahoo Finance (equities), CoinGecko (crypto), Tesouro Nacional API (bonds), BCB PTAX (FX).
- **Views** — Overview, positions, transaction history with chart, income (dividends/JCP), insights, import, manual entry, settings.
- **Deduplicated imports** — Every row is hashed so re-importing the same file is a no-op.

## Architecture

Cargo workspace with two crates:

- `crates/core` — data model, SQLite schema/queries, parsers (B3/Binance), price/FX API clients, portfolio math, reconciliation, JSON export/import. (IBKR CSV is parsed inline in the UI crate.)
- `crates/ui` — GPUI desktop app, built on upstream [`gpui`](https://github.com/zed-industries/zed) and [`gpui-component`](https://github.com/longbridge/gpui-component).

The database lives at `~/Library/Application Support/dinheiros/data.db` on macOS (via `dirs::data_local_dir()`).

## Run

```bash
cargo run --bin dinheiros
```

## Build

Produces a release binary at `target/release/dinheiros`:

```bash
cargo build --release --bin dinheiros
```

## Tests

```bash
cargo test
```

## Importing data

Place broker export files under `input-files/` (gitignored) in the matching subdirectory:

- `input-files/b3/*.xlsx` — B3 "Movimentação" sheet exports
- `input-files/ibkr/*.csv` — IBKR activity statement CSV
- `input-files/binance/*.csv` — Binance transaction history

Then use the **Import** tab in the app.

## Data model

Core tables: `transactions`, `daily_prices`, `income`, `config`. Every transaction carries both the native-currency value and the BRL-converted value at the trade-date FX rate. Schema and migrations live in `crates/core/src/db/schema.rs`.

## License

Private. Not licensed for redistribution.
