# Dinheiros

A personal investment portfolio tracker for Brazilian and international assets. Native desktop app written in Rust with a local-first SQLite database — no cloud, no accounts, no tracking.

## Features

- **Multi-broker imports** — B3 (`.xlsx`), Interactive Brokers Flex Query (`.xml`), Binance (`.csv`). Manual entry also supported.
- **Multi-asset** — Brazilian stocks/ETFs, international stocks, Tesouro Direto, crypto (BTC via Binance), gold.
- **BRL-first** — Every position is converted to BRL using historical PTAX rates for accurate cost basis and PnL.
- **Live prices** — Yahoo Finance (equities), CoinGecko (crypto), Tesouro Nacional API (bonds), BCB PTAX (FX).
- **Views** — Overview, positions, transaction history with chart, income (dividends/JCP), insights, manual entry, settings.
- **Deduplicated imports** — Every row is hashed so re-importing the same file is a no-op.

## Architecture

Cargo workspace with two crates:

- `crates/core` — data model, SQLite schema/queries, parsers (B3/IBKR/Binance), price/FX API clients, portfolio math, reconciliation, JSON export/import.
- `crates/ui` — GPUI desktop app (uses a local `gpui-ce` patch via `../gpui-ce`).

The database lives at `~/Library/Application Support/dinheiros/data.db` on macOS (via `dirs::data_local_dir()`).

## Build & run

Requires a local checkout of `gpui-ce` as a sibling directory (`../gpui-ce`) — see `Cargo.toml` patch section.

```bash
cargo run -p dinheiros-ui --release
```

Tests:

```bash
cargo test
```

## Importing data

Place broker export files under `input-files/` (gitignored) in the matching subdirectory:

- `input-files/b3/*.xlsx` — B3 "Movimentação" sheet exports
- `input-files/ibkr/*.xml` — IBKR Flex Query XML
- `input-files/binance/*.csv` — Binance transaction history

Then use the **Import** tab in the app.

## Data model

Core tables: `transactions`, `daily_prices`, `income`, `config`. Every transaction carries both the native-currency value and the BRL-converted value at the trade-date FX rate. Schema and migrations live in `crates/core/src/db/schema.rs`.

## License

Private. Not licensed for redistribution.
