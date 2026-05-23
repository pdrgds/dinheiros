# Demo Mode — Design

**Date:** 2026-05-23
**Purpose:** Provide a reusable way to launch dinheiros against a fresh, plausibly-populated SQLite database so screenshots (README, GitHub social cards, future marketing) can be regenerated on demand without touching the user's real portfolio data.

## Goals

- Launching the demo is a single shell command.
- The real database at `~/Library/Application Support/dinheiros/data.db` is never read or written during a demo session.
- The seed dataset exercises every primary view (Overview, Positions, Transactions chart, Income, Insights) with non-empty, visually interesting content.
- Re-running the demo regenerates the DB from scratch (no stale state from previous runs).
- The demo path remains useful after future UI changes — no per-feature update treadmill.

## Non-Goals

- No live network fetches in the seed binary (no Yahoo, no CoinGecko, no PTAX). All prices and FX rates are hard-coded.
- No fixture for `import_hash` collision with real broker exports — the demo data is unmistakably synthetic.
- No CLI flag (`--demo`) on the UI binary. Override happens via env var only; the wrapper script is the user-facing entry point.
- No reproducibility guarantee across seed-binary versions. The dataset can drift when the demo is improved; that is fine because each run rebuilds from scratch.

## Architecture

Three pieces, all additive (no behavior change for non-demo users):

### 1. Centralized DB path resolution

Currently the production DB path is constructed inline at three sites:

- `crates/ui/src/main.rs:74`
- `crates/ui/src/app.rs:372`
- `crates/ui/src/app.rs:599`

Replace these with a single helper in `crates/core/src/db/mod.rs`:

```rust
pub fn default_db_path() -> std::path::PathBuf {
    if let Ok(override_path) = std::env::var("DINHEIROS_DB_PATH") {
        return std::path::PathBuf::from(override_path);
    }
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("dinheiros")
        .join("data.db")
}
```

The UI crate calls `dinheiros_core::db::default_db_path()` at all three sites. Behavior with no env var set is byte-identical to today.

`dirs` is already a transitive dependency of the UI crate; add it as a direct dependency of `dinheiros-core` if it is not already there.

### 2. Seed binary

New binary target `dinheiros-seed` at `crates/core/src/bin/seed_demo.rs`.

**Invocation:**

```
dinheiros-seed [<db-path>]
```

If `<db-path>` is omitted, fall back to `$DINHEIROS_DB_PATH`, then to `/tmp/dinheiros-demo.db`.

**Behavior:**

1. If the target file exists, delete it. (Fresh start every run.)
2. Open via `Database::open(path)` — this triggers migrations to create the schema.
3. Insert the seed dataset (see "Seed Data" below) within a single transaction, using direct `rusqlite` `INSERT` statements. Bypassing the broker parsers is intentional: parsers expect specific file formats we are not faking.
4. Print a one-line summary: `seeded <path>: N transactions, M income, K daily_prices`.

Add an entry under `[[bin]]` in `crates/core/Cargo.toml`:

```toml
[[bin]]
name = "dinheiros-seed"
path = "src/bin/seed_demo.rs"
```

### 3. Wrapper script

`scripts/run-demo.sh` (chmod +x), checked in:

```bash
#!/usr/bin/env bash
set -euo pipefail
DB="${DINHEIROS_DEMO_DB:-/tmp/dinheiros-demo.db}"
cargo run --quiet -p dinheiros-core --bin dinheiros-seed -- "$DB"
DINHEIROS_DB_PATH="$DB" cargo run --bin dinheiros
```

User runs `./scripts/run-demo.sh` from the repo root. Window opens against the demo DB. Quit the app, run again, get a fresh demo. Real DB untouched.

## Seed Data

Span: **2024-11-23 → 2026-05-23** (~18 months), so the transactions chart shows a full arc.

All `total_brl`, `brl_rate`, `import_hash` fields are populated (the schema requires them). `import_hash` is constructed as `"demo-{symbol}-{date}-{seq}"` — deterministic but unmistakably synthetic.

### B3 stocks (BRL, `source='b3'`, `asset_type='stock_br'`)

Staggered buy dates across the 18-month window so the cost-basis line ramps up:

| Symbol | Lots | Total qty | Notes |
|--------|------|-----------|-------|
| PETR4  | 2    | 200       | Two buys ~6 months apart |
| VALE3  | 2    | 150       | One buy, one add-on |
| ITUB4  | 1    | 300       | Single early buy |
| BBAS3  | 2    | 100       | DCA |
| WEGE3  | 1    | 80        | Mid-period buy |
| BOVA11 | 1    | 50        | ETF, early buy |
| IVVB11 | 1    | 40        | ETF, recent buy |

Unit prices roughly match real ranges (PETR4 ~35–42, VALE3 ~60–70, ITUB4 ~28–34, etc.) but exact values are at the seed author's discretion. `currency='BRL'`, `brl_rate=1.0`.

### Tesouro Direto (BRL, `asset_type='tesouro'`)

- Tesouro Selic 2027 — 2 buys (R$ 5,000 each)
- Tesouro IPCA+ 2035 — 1 buy (R$ 10,000)

### IBKR (USD, `source='ibkr'`, `asset_type='stock_intl'`)

Bought in the last 8 months so the USD slice is recent:

- AAPL — 15 shares, 1 buy
- MSFT — 10 shares, 1 buy
- GOOGL — 8 shares, 1 buy

`currency='USD'`, `brl_rate` ~5.10–5.30 depending on date.

### Crypto (Binance, `source='binance'`, `asset_type='crypto'`)

BTC, 4 DCA buys of ~0.02 BTC each spread across the 18 months. `currency='BRL'` (matches the Binance parser's output).

### Income (`income` table)

12–15 entries spread across the period: monthly-ish dividends/JCP on the BR equities. Values R$ 50–300 each. Mix of `income_type='dividend'` and `income_type='jcp'`. `tax_withheld` populated for JCP, null for dividends.

### Daily prices (`daily_prices` table)

`positions::compute_positions` uses `get_latest_price(symbol)`. Insert **one row per symbol dated 2026-05-23** with plausible "today" prices. Mix winners and losers so the Positions PnL column shows both green and red:

- Winners: VALE3, ITUB4, WEGE3, AAPL, MSFT, BTC
- Losers: PETR4, BBAS3, BOVA11, IVVB11, GOOGL

USD rows use `brl_rate=5.20`. BRL rows use `brl_rate=1.0`. BTC uses `currency='BRL'` with the latest BRL price directly.

For the transactions-chart view, if it also reads `daily_prices` historically (rather than just latest), we can add monthly snapshot rows per symbol in a later iteration; for the first pass, latest-only is sufficient to make Positions and Overview look populated. (Implementer: verify by reading the chart view's data source during the plan phase and expand the seed if needed.)

## Refactor Scope

Only the duplicated path resolution. Three call sites collapse to one helper call each. No other cleanup, no behavior changes.

## Testing / Verification

1. `cargo build --bin dinheiros-seed` — builds clean.
2. `cargo build --bin dinheiros` — builds clean (refactor did not break anything).
3. `cargo test` — existing tests still pass.
4. Record mtime of `~/Library/Application Support/dinheiros/data.db` before running the demo.
5. `./scripts/run-demo.sh` — seed runs, app window opens.
6. Manually click through Overview, Positions, Transactions, Income, Insights tabs and confirm each shows non-empty data.
7. Quit app. Verify the real DB's mtime is unchanged (the demo session never touched it).
8. Re-run the script. Verify it works against a fresh `/tmp/dinheiros-demo.db` (previous contents replaced).

## Out of Scope (Possible Follow-ups)

- Adding historical `daily_prices` snapshots so the chart line reflects synthetic price movement over time, not just current value.
- Adding a `--seed-only` flag to `dinheiros-seed` for use in CI.
- Snapshotting screenshots automatically.
