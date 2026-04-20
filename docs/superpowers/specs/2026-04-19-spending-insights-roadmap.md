# Spending & Insights Integration — Roadmap

**Date:** 2026-04-19
**Status:** Proposal, awaiting review
**Type:** Multi-phase roadmap (each phase gets its own design spec)

## Context

Today `dinheiros` tracks investments (B3, IBKR, Binance). Personal cash flow — bank balances, credit card debt, recurring expenses, forecasts, monthly planning — lives in a separate Google Sheet (`Dinheiros - 2026`) with four tabs: Plano anual, Saldo atual, Despesas mensais, Despesas previstas.

This roadmap integrates the cash flow / spending side into `dinheiros` so it becomes the single source of truth for personal finance: investments, bank balances, credit card debt, spending transactions, forecasts, and insights.

## Current state

**dinheiros repo**
- Rust + GPUI + local-first SQLite at `~/Library/Application Support/dinheiros/data.db`
- Parsers for B3 (.xlsx), IBKR (.csv), Binance (.csv); dedup via row hashing
- UI tabs including an "Insights" view
- BRL-first with historical PTAX conversion

**Spreadsheet (to integrate)**
- **Saldo atual** — 8 balance entries: BB, Nubank, Picpay (banks) + Fatura Santander, C6, C&A Pay, Neon (credit cards). Manually typed, periodic.
- **Despesas mensais** — 16 recurring commitments totalling R$ 7.678 / €1.541/mo. BRL and EUR columns.
- **Despesas previstas** — forecast one-offs (meds, courses) with date and "Executada" flag.
- **Plano anual** — monthly cash flow model: Renda $/R$, Impostos, Despesas fixas, Despesas previstas, Gasto total, Gasto discricionário. Has forecasts through Dec 2026 including a modelled income gap Sep–Dec (Germany relocation context).

**Institutions in scope:** BB, Santander, C6, PicPay, Nubank, C&A Pay, Neon.

## Target state

`dinheiros` owns:
- Investment data (existing)
- Bank account balances and transactions
- Credit card bills and transactions
- Recurring-commitment reference table
- Forecast one-off expenses
- Annual cash flow model
- Deterministic metrics (net worth, category breakdown, runway, utilization, etc.)
- LLM insights (on-demand and monthly report)
- Dual-currency BRL/EUR reporting

The spreadsheet either sunsets or becomes a derived/exported view, decided per-phase as features come online.

## Phases

### Phase 1a — Balance snapshots (foundations)

**Goal:** replace `Saldo atual` with a structured time series you can graph.

**In scope:**
- SQLite tables: `accounts`, `balance_snapshots`
- GPUI input modal to enter a monthly set of balances in one go
- Net worth time series view (sum of bank balances − credit card faturas + investments, over time)
- Seed the 7 institutions as accounts

**Out of scope:**
- Any transaction import (that's 1b)
- Categorization, recurring commitments, forecasts (Phase 2)
- EUR conversion (Phase 5)

**Deliverable:** you can stop using the Saldo atual tab after this.

---

### Phase 1b — First transaction import

**Goal:** ingest transaction-level detail for the two highest-volume institutions; establish the reconciliation pattern.

**In scope:**
- SQLite tables: `card_bills`, `card_transactions`, `bank_transactions`
- Parsers for BB (CSV/OFX) and Santander (closed fatura CSV/PDF) — the two largest signals
- `input-files/banks/` and `input-files/cards/` directories following existing pattern
- Reconciliation check: closed-bill total vs nearest snapshot; flag divergence
- Deduplication via row hash (follow existing pattern)

**Out of scope:**
- Remaining 5 institutions (Phase 5)
- Categorization (Phase 2)

**Deliverable:** BB and Santander transactions queryable in the DB.

---

### Phase 2 — Reference tables & categorization

**Goal:** teach the system what your recurring commitments and forecasts are; categorize transactions.

**In scope:**
- SQLite tables: `recurring_commitments`, `forecast_expenses`, `categories`, `merchant_rules`
- Import Despesas mensais and Despesas previstas from CSV/manual UI
- Rules-based categorizer (merchant → category); seeded from recurring-commitment list (e.g., "Aluguel" merchant pattern → Moradia)
- Forecast validation: mark forecast expense as Executada when a matching transaction appears
- Missing-expected-charge detector: flag recurring commitments not seen this month

**Out of scope:**
- LLM-driven categorization (LLM insights layer handles ambiguity, not core categorization)

**Deliverable:** you can stop using the Despesas mensais / Despesas previstas tabs.

---

### Phase 3 — Deterministic insights

**Goal:** graphs and metrics in the Insights tab.

**In scope (must-have metrics):**
- Spend by category per month (stacked bar)
- 3-month moving average per category
- Enrich the Phase 1a net worth graph with trend line and Plano-anual target line
- Recurring commitment baseline vs actuals
- Credit utilization per card
- Runway in months (liquid ÷ avg discretionary spend)

**In scope (nice-to-have if cheap):**
- Year-over-year same-month delta
- Essential vs discretionary split
- Top 10 merchants per month

**Out of scope:**
- Anomaly detection / subscription creep detection (LLM layer handles this)

**Deliverable:** Insights tab answers "where did my money go" and "how is my net worth trending" without typing a prompt.

---

### Phase 4 — LLM insights

**Goal:** open-ended "find what you can" analysis, both on-demand and scheduled.

**In scope:**
- On-demand: structured export of the DB to a Claude-readable folder (or a context-window-sized JSON); questions asked in a Claude session
- Scheduled: monthly report generator produces `reports/YYYY-MM.md` summarizing findings, cross-referencing recurring/forecast tables
- Anomaly flagging (unusual merchants, amounts)
- Subscription creep detection (new recurring pattern not in reference table)

**Out of scope:**
- Real-time LLM integration inside the GPUI app (post-roadmap, if valuable)

**Deliverable:** you can ask "what changed this month?" and get a real answer.

---

### Phase 5 — Rest of institutions + dual currency

**Goal:** full coverage and Germany-relocation readiness.

**In scope:**
- Parsers for C6, PicPay, Nubank, C&A Pay, Neon (formats TBD per institution)
- EUR column throughout reports, using BCB or ECB daily rate
- Plano anual import/mirror as a forecasting layer

**Out of scope:**
- International account support (Wise, German banks) — post-roadmap

**Deliverable:** all 7 institutions ingested; reports bilingual BRL/EUR.

## Non-goals (whole roadmap)

- Real-time bank API integration (Pluggy, Belvo) — file-based only
- Automating payments or money movement
- Tax reporting / IRPF — handled by `irpf-maker` as a separate project that reads investment transactions from this DB (integration shape TBD)
- Budgeting enforcement ("you spent too much on X") — surface data, don't moralize
- Multi-user / shared access

## Open questions (resolve as phases begin)

- **Spreadsheet sunset policy:** keep editing it in parallel as a safety net, or cut cold after each phase?
- **Reconciliation policy:** when snapshot and closed-bill diverge, flag-only or attempt auto-reconcile with a heuristic?
- **Historical backfill:** import past closed faturas you already have, or start fresh from today?
- **Annual plan (Plano anual) location:** ingested as another reference table, or stays in spreadsheet as forecast input?
- **Balance snapshot cadence:** enforce monthly discipline in the UI, or let it be ad-hoc?
- **irpf-maker integration shape:** direct SQLite read (couple via schema) vs. a `dinheiros tax-export` command (clean boundary, extra artifact). Defer until irpf-maker needs it; do not disturb the investment-side tables in the meantime.

## Sequencing assumptions

- Phases are sequential; each unlocks the next.
- Phase 1a is the minimum useful increment; can ship and stop any time after that.
- Phase 4 (LLM) becomes meaningfully better after Phase 2 (reference tables) because cross-referencing is where most value lives.
- Phase 5 parsers can be interleaved opportunistically — e.g., do Nubank in Phase 1b if it's the easier format.
