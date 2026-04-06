use std::collections::HashMap;

use chrono::{Duration, Local, NaiveDate};

use crate::api::{bcb_ptax, coingecko, tesouro, yahoo};
use crate::db::queries;
use crate::db::Database;
use crate::types::DailyPrice;

#[derive(Debug)]
pub struct ReconcileResult {
    pub prices_backfilled: usize,
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
/// For each symbol, finds last stored date, fetches forward.
/// Stops gracefully on rate limits.
pub async fn backfill_prices(db: &Database) -> Result<ReconcileResult, Box<dyn std::error::Error>> {
    let symbols = queries::get_distinct_symbols(db)?;
    let today = Local::now().date_naive();
    let mut prices_backfilled: usize = 0;
    let mut rate_limited = false;

    for (symbol, asset_type, currency) in &symbols {
        if rate_limited {
            break;
        }

        // Skip Tesouro Direto for backfill — radaropcoes has no historical API
        if tesouro::is_tesouro(symbol) {
            continue;
        }

        // Determine start date: day after last stored price, or earliest transaction date
        let start = match queries::get_latest_price_date(db, symbol)? {
            Some(last) => last + Duration::days(1),
            None => {
                let txs = queries::get_transactions_by_symbol(db, symbol)?;
                match txs.first() {
                    Some(tx) => tx.date,
                    None => continue, // no transactions, skip
                }
            }
        };

        if start >= today {
            continue; // already up to date
        }

        if asset_type == "crypto" {
            // Use CoinGecko for crypto
            match coingecko::fetch_btc_brl_history(start, today).await {
                Ok(prices) => {
                    for (date, price) in &prices {
                        if let Err(e) = queries::upsert_daily_price(
                            db,
                            &DailyPrice {
                                symbol: symbol.clone(),
                                date: *date,
                                close_price: *price,
                                currency: "BRL".to_string(),
                                brl_rate: 1.0,
                            },
                        ) {
                            eprintln!(
                                "[reconcile] warning: failed to store backfill price for {} on {}: {}",
                                symbol, date, e
                            );
                        } else {
                            prices_backfilled += 1;
                        }
                    }
                }
                Err(e) => {
                    if is_rate_limited(&e) {
                        eprintln!("[reconcile] rate limited while backfilling {}, stopping", symbol);
                        rate_limited = true;
                        break;
                    }
                    eprintln!(
                        "[reconcile] warning: failed to backfill crypto {}: {}",
                        symbol, e
                    );
                }
            }
        } else {
            // Use Yahoo Finance for stocks/bonds/gold
            let yahoo_sym = yahoo::to_yahoo_symbol_with_db(symbol, asset_type, db);

            let history = match yahoo::fetch_history(&yahoo_sym, start, today).await {
                Ok(h) => h,
                Err(e) => {
                    if is_rate_limited(&e) {
                        eprintln!("[reconcile] rate limited while backfilling {}, stopping", symbol);
                        rate_limited = true;
                        break;
                    }
                    eprintln!(
                        "[reconcile] warning: failed to backfill {}: {}",
                        symbol, e
                    );
                    continue;
                }
            };

            // For non-BRL symbols, fetch PTAX rates for the range
            let rate_map: Option<HashMap<NaiveDate, f64>> = if currency != "BRL" {
                match bcb_ptax::fetch_rates_range(currency, start, today).await {
                    Ok(rates) => {
                        let map: HashMap<NaiveDate, f64> = rates.into_iter().collect();
                        Some(map)
                    }
                    Err(e) => {
                        if is_rate_limited(&e) {
                            eprintln!(
                                "[reconcile] rate limited fetching PTAX for {}, stopping",
                                symbol
                            );
                            rate_limited = true;
                            break;
                        }
                        eprintln!(
                            "[reconcile] warning: failed to fetch PTAX range for {} ({}): {}",
                            currency, symbol, e
                        );
                        continue;
                    }
                }
            } else {
                None
            };

            for (date, close_price) in &history {
                let brl_rate = if currency != "BRL" {
                    match rate_map
                        .as_ref()
                        .and_then(|m| find_closest_rate(m, *date))
                    {
                        Some(rate) => rate,
                        None => {
                            // Skip this date if we can't find a rate
                            eprintln!(
                                "[reconcile] warning: no PTAX rate found for {} on {}, skipping",
                                symbol, date
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
                        date: *date,
                        close_price: *close_price,
                        currency: currency.clone(),
                        brl_rate,
                    },
                ) {
                    eprintln!(
                        "[reconcile] warning: failed to store backfill price for {} on {}: {}",
                        symbol, date, e
                    );
                } else {
                    prices_backfilled += 1;
                }
            }
        }
    }

    Ok(ReconcileResult {
        prices_backfilled,
        rate_limited,
    })
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
