use std::collections::{HashMap, HashSet};

use chrono::{Duration, Local, NaiveDate};

use crate::api::{bcb_ptax, coingecko, tesouro, yahoo};
use crate::calendar;
use crate::db::queries;
use crate::db::Database;
use crate::types::DailyPrice;

#[derive(Debug)]
pub struct ReconcileResult {
    pub prices_backfilled: usize,
    pub symbols_failed: usize,
    pub symbols_up_to_date: usize,
    pub rate_limited: bool,
}

/// Phase 1: Fetch today's prices for all held symbols.
/// Returns count of prices successfully fetched.
pub async fn fetch_current_prices(db: &Database) -> Result<usize, Box<dyn std::error::Error>> {
    let symbols = queries::get_distinct_symbols(db)?;
    let today = Local::now().date_naive();
    let mut count = 0;
    let mut has_crypto = false;

    for (symbol, asset_type, currency) in &symbols {
        if asset_type == "crypto" {
            has_crypto = true;
            continue;
        }

        // Don't pollute daily_prices with weekend/holiday rows: when the market is
        // closed, Yahoo returns the last traded price, not a price for today.
        // Writing it as `today` creates phantom data points that also trick
        // backfill_date_range into thinking the symbol is fully up to date.
        if !calendar::is_trading_day(today, asset_type) {
            continue;
        }

        // Gold: GC=F is USD per troy ounce, convert to BRL per gram
        if asset_type == "gold" {
            match yahoo::fetch_current_price("GC=F").await {
                Ok(usd_per_oz) => {
                    let usd_per_gram = usd_per_oz / 31.1035;
                    match bcb_ptax::fetch_rate("USD", today).await {
                        Ok(ptax) => {
                            let brl_per_gram = usd_per_gram * ptax;
                            if let Err(e) = queries::upsert_daily_price(
                                db,
                                &DailyPrice {
                                    symbol: symbol.clone(),
                                    date: today,
                                    close_price: brl_per_gram,
                                    currency: "BRL".to_string(),
                                    brl_rate: 1.0,
                                },
                            ) {
                                eprintln!("[reconcile] warning: failed to store gold price: {}", e);
                            } else {
                                count += 1;
                            }
                        }
                        Err(e) => {
                            eprintln!("[reconcile] warning: failed to fetch PTAX for gold: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[reconcile] warning: failed to fetch gold price: {}", e);
                }
            }
            continue;
        }

        // Tesouro Direto: use radaropcoes API
        if tesouro::is_tesouro(symbol) {
            match tesouro::fetch_tesouro_price(symbol).await {
                Ok(price) => {
                    if let Err(e) = queries::upsert_daily_price(
                        db,
                        &DailyPrice {
                            symbol: symbol.clone(),
                            date: today,
                            close_price: price,
                            currency: "BRL".to_string(),
                            brl_rate: 1.0,
                        },
                    ) {
                        eprintln!("[reconcile] warning: failed to store Tesouro price for {}: {}", symbol, e);
                    } else {
                        count += 1;
                    }
                }
                Err(e) => {
                    eprintln!("[reconcile] warning: failed to fetch Tesouro price for {}: {}", symbol, e);
                }
            }
            continue;
        }

        let yahoo_sym = yahoo::to_yahoo_symbol_with_db(symbol, asset_type, db);

        let price = match yahoo::fetch_current_price(&yahoo_sym).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[reconcile] warning: failed to fetch price for {}: {}", symbol, e);
                continue;
            }
        };

        let brl_rate = if currency != "BRL" {
            match bcb_ptax::fetch_rate(currency, today).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!(
                        "[reconcile] warning: failed to fetch PTAX rate for {} ({}): {}",
                        currency, symbol, e
                    );
                    continue;
                }
            }
        } else {
            1.0
        };

        if let Err(e) = queries::upsert_daily_price(
            db,
            &DailyPrice {
                symbol: symbol.clone(),
                date: today,
                close_price: price,
                currency: currency.clone(),
                brl_rate,
            },
        ) {
            eprintln!("[reconcile] warning: failed to store price for {}: {}", symbol, e);
            continue;
        }

        count += 1;
    }

    // Handle BTC separately via CoinGecko (BRL-denominated)
    // Only fetch once per symbol, not per (symbol, currency) pair.
    if has_crypto {
        let mut seen = std::collections::HashSet::new();
        for (symbol, asset_type, _currency) in &symbols {
            if asset_type != "crypto" || !seen.insert(symbol.clone()) {
                continue;
            }
            match coingecko::fetch_btc_brl_current().await {
                Ok(price) => {
                    if let Err(e) = queries::upsert_daily_price(
                        db,
                        &DailyPrice {
                            symbol: symbol.clone(),
                            date: today,
                            close_price: price,
                            currency: "BRL".to_string(),
                            brl_rate: 1.0,
                        },
                    ) {
                        eprintln!("[reconcile] warning: failed to store crypto price for {}: {}", symbol, e);
                    } else {
                        count += 1;
                    }
                }
                Err(e) => {
                    eprintln!("[reconcile] warning: failed to fetch BTC/BRL price: {}", e);
                }
            }
        }
    }

    Ok(count)
}

/// Phase 2: Backfill historical price gaps.
/// Runs CoinGecko (crypto) and Yahoo (stocks) backfills in parallel.
pub async fn backfill_prices(db: &Database) -> Result<ReconcileResult, Box<dyn std::error::Error>> {
    let symbols = queries::get_distinct_symbols(db)?;

    let crypto_symbols: Vec<_> = symbols
        .iter()
        .filter(|(_, at, _)| at == "crypto")
        .cloned()
        .collect();

    // For Tesouro, include ALL symbols ever held (not just active)
    // because the history chart needs prices for past holdings too.
    let tesouro_symbols: Vec<_> = {
        let mut stmt = db.conn().prepare(
            "SELECT DISTINCT symbol, asset_type, currency FROM transactions WHERE asset_type = 'tesouro'"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let stock_symbols: Vec<_> = symbols
        .iter()
        .filter(|(sym, at, _)| at != "crypto" && at != "gold" && !tesouro::is_tesouro(sym))
        .cloned()
        .collect();

    // Run all three in parallel
    let (crypto_result, stock_result, tesouro_result) = tokio::join!(
        backfill_crypto(db, &crypto_symbols),
        backfill_stocks(db, &stock_symbols),
        backfill_tesouro(db, &tesouro_symbols),
    );

    let (crypto_bf, crypto_fail, crypto_upd, crypto_rl) = crypto_result;
    let (stock_bf, stock_fail, stock_upd, stock_rl) = stock_result;
    let (tesouro_bf, tesouro_fail, tesouro_upd, _) = tesouro_result;

    Ok(ReconcileResult {
        prices_backfilled: crypto_bf + stock_bf + tesouro_bf,
        symbols_failed: crypto_fail + stock_fail + tesouro_fail,
        symbols_up_to_date: crypto_upd + stock_upd + tesouro_upd,
        rate_limited: crypto_rl || stock_rl,
    })
}

/// Backfill crypto prices via CoinGecko.
/// Returns (backfilled, failed, up_to_date, rate_limited).
async fn backfill_crypto(
    db: &Database,
    symbols: &[(String, String, String)],
) -> (usize, usize, usize, bool) {
    let today = Local::now().date_naive();
    let mut prices_backfilled = 0;
    let mut symbols_failed = 0;
    let mut symbols_up_to_date = 0;

    for (symbol, asset_type, _currency) in symbols {
        let (start, end, missing) = match backfill_date_range(db, symbol, asset_type, today) {
            Ok(Some(plan)) => plan,
            Ok(None) => { symbols_up_to_date += 1; continue; }
            Err(_) => continue,
        };

        // Binance klines API handles any range with pagination, no limit
        match coingecko::fetch_btc_brl_history(start, end).await {
            Ok(prices) => {
                for (date, price) in &prices {
                    if !missing.contains(date) {
                        continue;
                    }
                    if queries::upsert_daily_price(db, &DailyPrice {
                        symbol: symbol.clone(), date: *date, close_price: *price,
                        currency: "BRL".to_string(), brl_rate: 1.0,
                    }).is_ok() {
                        prices_backfilled += 1;
                    }
                }
            }
            Err(e) => {
                if is_rate_limited(&e) {
                    return (prices_backfilled, symbols_failed, symbols_up_to_date, true);
                }
                symbols_failed += 1;
                eprintln!("[reconcile] crypto backfill failed for {}: {}", symbol, e);
            }
        }
    }
    (prices_backfilled, symbols_failed, symbols_up_to_date, false)
}

/// Backfill stock/ETF prices via Yahoo Finance + BCB PTAX.
async fn backfill_stocks(
    db: &Database,
    symbols: &[(String, String, String)],
) -> (usize, usize, usize, bool) {
    let today = Local::now().date_naive();
    let mut prices_backfilled = 0;
    let mut symbols_failed = 0;
    let mut symbols_up_to_date = 0;

    for (symbol, asset_type, currency) in symbols {
        let (start, end, missing) = match backfill_date_range(db, symbol, asset_type, today) {
            Ok(Some(plan)) => plan,
            Ok(None) => { symbols_up_to_date += 1; continue; }
            Err(_) => continue,
        };

        let yahoo_sym = yahoo::to_yahoo_symbol_with_db(symbol, asset_type, db);
        let history = match yahoo::fetch_history(&yahoo_sym, start, end).await {
            Ok(h) => h,
            Err(e) => {
                if is_rate_limited(&e) {
                    return (prices_backfilled, symbols_failed, symbols_up_to_date, true);
                }
                symbols_failed += 1;
                eprintln!("[reconcile] stock backfill failed for {}: {}", symbol, e);
                continue;
            }
        };

        let rate_map: Option<HashMap<NaiveDate, f64>> = if currency != "BRL" {
            match bcb_ptax::fetch_rates_range(currency, start, end).await {
                Ok(rates) => Some(rates.into_iter().collect()),
                Err(e) => {
                    if is_rate_limited(&e) {
                        return (prices_backfilled, symbols_failed, symbols_up_to_date, true);
                    }
                    eprintln!("[reconcile] PTAX failed for {} ({}): {}", currency, symbol, e);
                    continue;
                }
            }
        } else {
            None
        };

        for (date, close_price) in &history {
            if !missing.contains(date) {
                continue;
            }
            let brl_rate = if currency != "BRL" {
                match rate_map.as_ref().and_then(|m| find_closest_rate(m, *date)) {
                    Some(rate) => rate,
                    None => continue,
                }
            } else {
                1.0
            };

            if queries::upsert_daily_price(db, &DailyPrice {
                symbol: symbol.clone(), date: *date, close_price: *close_price,
                currency: currency.clone(), brl_rate,
            }).is_ok() {
                prices_backfilled += 1;
            }
        }
    }
    (prices_backfilled, symbols_failed, symbols_up_to_date, false)
}

/// Backfill Tesouro Direto prices from the government CSV.
async fn backfill_tesouro(
    db: &Database,
    symbols: &[(String, String, String)],
) -> (usize, usize, usize, bool) {
    if symbols.is_empty() {
        return (0, 0, 0, false);
    }

    let today = Local::now().date_naive();

    // Check if any symbol actually needs backfilling
    let needs_backfill: Vec<_> = symbols
        .iter()
        .filter(|(sym, at, _)| {
            backfill_date_range(db, sym, at, today)
                .ok()
                .flatten()
                .is_some()
        })
        .collect();

    if needs_backfill.is_empty() {
        return (0, 0, symbols.len(), false);
    }

    // Fetch the full CSV once
    let csv_url = "https://www.tesourotransparente.gov.br/ckan/dataset/df56aa42-484a-4a59-8184-7676580c81e3/resource/796d2059-14e9-44e3-80c9-2d9e30b405c1/download/precotaxatesourodireto.csv";

    let client = reqwest::Client::builder()
        .user_agent("dinheiros/0.1")
        .build()
        .unwrap();

    let body = match client.get(csv_url).send().await {
        Ok(resp) => match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[reconcile] failed to read Tesouro CSV: {}", e);
                return (0, symbols.len(), 0, false);
            }
        },
        Err(e) => {
            eprintln!("[reconcile] failed to fetch Tesouro CSV: {}", e);
            return (0, symbols.len(), 0, false);
        }
    };

    let mut prices_backfilled = 0;
    let mut symbols_up_to_date = 0;

    for (symbol, asset_type, _) in symbols {
        let (start, end, missing) = match backfill_date_range(db, symbol, asset_type, today) {
            Ok(Some(plan)) => plan,
            Ok(None) => {
                symbols_up_to_date += 1;
                continue;
            }
            Err(_) => continue,
        };

        // Match our symbol to CSV entries
        // Our: "Tesouro Selic 2031" → CSV: "Tesouro Selic" with maturity year 2031
        // Our: "Tesouro IPCA+ 2029" → CSV: "Tesouro IPCA+" with maturity year 2029
        let (search_type, search_year) = parse_tesouro_symbol(symbol);

        for line in body.lines().skip(1) {
            let fields: Vec<&str> = line.split(';').collect();
            if fields.len() < 7 {
                continue;
            }

            let tipo = fields[0];
            let vencimento = fields[1]; // dd/mm/yyyy
            let data_base = fields[2];  // dd/mm/yyyy
            let pu_venda = fields[6];   // "PU Venda Manha" - sell price

            // Match bond type and maturity year
            if !tipo.starts_with(&search_type) {
                continue;
            }
            if !vencimento.ends_with(&search_year) {
                continue;
            }

            // Parse date
            let date = match NaiveDate::parse_from_str(data_base, "%d/%m/%Y") {
                Ok(d) => d,
                Err(_) => continue,
            };

            if date < start || date > end {
                continue;
            }
            if !missing.contains(&date) {
                continue;
            }

            // Parse price (Brazilian format: "1234,56")
            let price: f64 = match pu_venda.replace('.', "").replace(',', ".").parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            if queries::upsert_daily_price(
                db,
                &DailyPrice {
                    symbol: symbol.clone(),
                    date,
                    close_price: price,
                    currency: "BRL".to_string(),
                    brl_rate: 1.0,
                },
            )
            .is_ok()
            {
                prices_backfilled += 1;
            }
        }
    }

    (prices_backfilled, 0, symbols_up_to_date, false)
}

/// Parse our Tesouro symbol into (type_prefix, maturity_year).
/// "Tesouro Selic 2031" → ("Tesouro Selic", "2031")
/// "Tesouro IPCA+ 2029" → ("Tesouro IPCA+", "2029")
fn parse_tesouro_symbol(symbol: &str) -> (String, String) {
    // Last 4 chars are the year
    if symbol.len() > 5 {
        let year = &symbol[symbol.len() - 4..];
        let prefix = symbol[..symbol.len() - 5].trim().to_string();
        (prefix, year.to_string())
    } else {
        (symbol.to_string(), String::new())
    }
}

/// Compute a backfill plan for a symbol: the date range to fetch, plus the exact
/// set of trading days inside it that are actually missing.
///
/// The range is the tightest `[min_missing, max_missing]` window so a single API
/// call covers every gap (prefix, middle, or suffix). The `missing` set lets the
/// caller increment `prices_backfilled` only for genuinely new dates — without
/// this, idempotent re-upserts of already-present dates (notably the Tesouro CSV
/// re-download) would keep the worker counter > 0 forever and prevent
/// `start_backfill_worker` from terminating.
fn backfill_date_range(
    db: &Database,
    symbol: &str,
    asset_type: &str,
    today: NaiveDate,
) -> Result<Option<(NaiveDate, NaiveDate, HashSet<NaiveDate>)>, Box<dyn std::error::Error>> {
    let txs = queries::get_transactions_by_symbol(db, symbol)?;
    let first_tx_date = match txs.first() {
        Some(tx) => tx.date,
        None => return Ok(None),
    };

    let present: HashSet<NaiveDate> = {
        let mut stmt = db
            .conn()
            .prepare("SELECT date FROM daily_prices WHERE symbol = ?")?;
        let rows = stmt.query_map([symbol], |row| row.get::<_, String>(0))?;
        rows.filter_map(|r| r.ok())
            .filter_map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
            .collect()
    };

    let missing_vec =
        calendar::missing_trading_days(first_tx_date, today, asset_type, &present);
    if missing_vec.is_empty() {
        return Ok(None);
    }
    let start = *missing_vec.first().unwrap();
    let end = *missing_vec.last().unwrap();
    let missing: HashSet<NaiveDate> = missing_vec.into_iter().collect();
    Ok(Some((start, end, missing)))
}

/// Check if an error indicates rate limiting.
/// Looks for "429", "rate", or "limit" in the lowercased error string.
fn is_rate_limited(err: &Box<dyn std::error::Error>) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("429") || msg.contains("rate") || msg.contains("limit")
}

/// Find the closest PTAX rate for a target date, looking back up to 5 days
/// to handle weekends and holidays.
fn find_closest_rate(rate_map: &HashMap<NaiveDate, f64>, target: NaiveDate) -> Option<f64> {
    for days_back in 0..6 {
        if let Some(&rate) = rate_map.get(&(target - Duration::days(days_back))) {
            return Some(rate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DailyPrice, Source, Transaction, TxType};

    fn mk_tx(symbol: &str, date: NaiveDate, asset_type: crate::types::AssetType) -> Transaction {
        Transaction {
            id: None,
            source: Source::Manual,
            asset_type,
            symbol: symbol.to_string(),
            tx_type: TxType::Buy,
            date,
            quantity: 1.0,
            unit_price: Some(1.0),
            currency: "BRL".to_string(),
            total_value: 1.0,
            brl_rate: 1.0,
            total_brl: 1.0,
            commission: None,
            fee_brl: None,
            notes: None,
            import_hash: format!("{}-{}", symbol, date),
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
    fn backfill_date_range_detects_middle_gap() {
        let db = Database::open_in_memory().unwrap();
        let first_tx = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 4, 17).unwrap();

        queries::insert_transaction(&db, &mk_tx("KULR", first_tx, crate::types::AssetType::StockIntl))
            .unwrap();

        // Cover the prefix and one late day, but leave Apr 7–10, 13–14, 16–17 missing.
        for (m, d) in [(4, 1), (4, 2), (4, 6), (4, 15)] {
            queries::upsert_daily_price(
                &db,
                &mk_price("KULR", NaiveDate::from_ymd_opt(2026, m, d).unwrap()),
            )
            .unwrap();
        }

        let plan = backfill_date_range(&db, "KULR", "stock_intl", today)
            .unwrap()
            .expect("expected a backfill plan when middle gaps exist");

        assert_eq!(plan.0, NaiveDate::from_ymd_opt(2026, 4, 7).unwrap());
        assert_eq!(plan.1, NaiveDate::from_ymd_opt(2026, 4, 17).unwrap());
        // Missing set must contain only the actually-absent trading days. This is
        // what callers gate the prices_backfilled counter on so re-upserting a
        // dense fetch doesn't keep the worker looping forever.
        let expected: HashSet<NaiveDate> = [
            (4, 7), (4, 8), (4, 9), (4, 10), (4, 13), (4, 14), (4, 16), (4, 17),
        ]
        .iter()
        .map(|(m, d)| NaiveDate::from_ymd_opt(2026, *m, *d).unwrap())
        .collect();
        assert_eq!(plan.2, expected);
    }

    #[test]
    fn backfill_date_range_none_when_complete() {
        let db = Database::open_in_memory().unwrap();
        let first_tx = NaiveDate::from_ymd_opt(2026, 4, 13).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 4, 17).unwrap();

        queries::insert_transaction(&db, &mk_tx("KULR", first_tx, crate::types::AssetType::StockIntl))
            .unwrap();
        for d in 13..=17 {
            queries::upsert_daily_price(
                &db,
                &mk_price("KULR", NaiveDate::from_ymd_opt(2026, 4, d).unwrap()),
            )
            .unwrap();
        }

        let range = backfill_date_range(&db, "KULR", "stock_intl", today).unwrap();
        assert!(range.is_none(), "complete data should not need backfill");
    }

    #[test]
    fn backfill_date_range_ignores_weekend_pollution() {
        // A bogus weekend "today" row from the live-price fetcher must not trick the
        // gap detector into thinking the symbol is up to date.
        let db = Database::open_in_memory().unwrap();
        let first_tx = NaiveDate::from_ymd_opt(2026, 4, 13).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 4, 19).unwrap(); // Sunday

        queries::insert_transaction(&db, &mk_tx("KULR", first_tx, crate::types::AssetType::StockIntl))
            .unwrap();
        // Only the bogus Sunday row exists — every weekday in between is missing.
        queries::upsert_daily_price(&db, &mk_price("KULR", today)).unwrap();

        let plan = backfill_date_range(&db, "KULR", "stock_intl", today)
            .unwrap()
            .expect("weekend pollution must not mark symbol as up-to-date");

        assert_eq!(plan.0, NaiveDate::from_ymd_opt(2026, 4, 13).unwrap()); // Mon
        assert_eq!(plan.1, NaiveDate::from_ymd_opt(2026, 4, 17).unwrap()); // Fri
        assert_eq!(plan.2.len(), 5); // Mon..Fri all missing
    }

    #[test]
    fn backfill_date_range_crypto_includes_every_day() {
        let db = Database::open_in_memory().unwrap();
        let first_tx = NaiveDate::from_ymd_opt(2026, 4, 13).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 4, 19).unwrap();

        queries::insert_transaction(&db, &mk_tx("BTC", first_tx, crate::types::AssetType::Crypto))
            .unwrap();
        // Have only the first and last days; middle 5 (incl. Sat/Sun) are missing for crypto.
        queries::upsert_daily_price(&db, &mk_price("BTC", first_tx)).unwrap();
        queries::upsert_daily_price(&db, &mk_price("BTC", today)).unwrap();

        let plan = backfill_date_range(&db, "BTC", "crypto", today)
            .unwrap()
            .expect("crypto gaps must be detected on weekend days too");

        assert_eq!(plan.0, NaiveDate::from_ymd_opt(2026, 4, 14).unwrap());
        assert_eq!(plan.1, NaiveDate::from_ymd_opt(2026, 4, 18).unwrap());
        assert_eq!(plan.2.len(), 5);
    }
}
