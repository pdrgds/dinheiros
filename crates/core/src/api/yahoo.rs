use chrono::{Datelike, NaiveDate};
use yahoo_finance_api as yahoo;

/// Fetch the latest closing price for a symbol.
pub async fn fetch_current_price(symbol: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let provider = yahoo::YahooConnector::new()?;
    let response = provider.get_latest_quotes(symbol, "1d").await?;
    let quote = response.last_quote()?;
    Ok(quote.close)
}

/// Fetch daily closing prices for a date range. Returns (date, close_price) pairs.
pub async fn fetch_history(
    symbol: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<(NaiveDate, f64)>, Box<dyn std::error::Error>> {
    let provider = yahoo::YahooConnector::new()?;

    let start = naive_date_to_offset_datetime(from);
    let end = naive_date_to_offset_datetime_eod(to);

    let response = provider.get_quote_history(symbol, start, end).await?;
    let quotes = response.quotes()?;

    let mut results = Vec::new();
    for q in &quotes {
        let dt = time::OffsetDateTime::from_unix_timestamp(q.timestamp)?;
        let date = NaiveDate::from_ymd_opt(dt.year(), dt.month() as u32, dt.day() as u32)
            .ok_or("invalid date from timestamp")?;
        results.push((date, q.close));
    }

    Ok(results)
}

/// Map internal symbols to Yahoo Finance tickers.
/// "stock_br" assets: add ".SA" suffix (e.g., "PETR4" -> "PETR4.SA")
/// "gold" assets: return "GC=F"
/// For IBKR international stocks, checks the exchange stored in config
/// and adds the appropriate Yahoo suffix.
pub fn to_yahoo_symbol(symbol: &str, asset_type: &str) -> String {
    match asset_type {
        "stock_br" => format!("{}.SA", symbol),
        "gold" => "GC=F".to_string(),
        _ => symbol.to_string(),
    }
}

/// Like to_yahoo_symbol but also checks the DB for a stored exchange mapping.
pub fn to_yahoo_symbol_with_db(symbol: &str, asset_type: &str, db: &crate::db::Database) -> String {
    match asset_type {
        "stock_br" => return format!("{}.SA", symbol),
        "gold" => return "GC=F".to_string(),
        _ => {}
    }

    // Check if we have an exchange mapping for this symbol
    let key = format!("exchange:{}", symbol);
    if let Ok(Some(exchange)) = crate::db::queries::get_config(db, &key) {
        if let Some(suffix) = ibkr_exchange_to_yahoo_suffix(&exchange) {
            return format!("{}.{}", symbol, suffix);
        }
    }

    symbol.to_string()
}

/// Map IBKR exchange codes to Yahoo Finance suffixes.
/// US exchanges return None (no suffix needed).
fn ibkr_exchange_to_yahoo_suffix(exchange: &str) -> Option<&'static str> {
    match exchange {
        "EBS" => Some("SW"),            // SIX Swiss Exchange
        "SWB2" | "SWB" => Some("SG"),   // Stuttgart
        "FWB" | "FWB2" => Some("F"),    // Frankfurt
        "IBIS" | "XETRA" => Some("DE"), // Xetra
        "HEX" => Some("HE"),            // Helsinki
        "CPH" => Some("CO"),            // Copenhagen
        "AEB" => Some("AS"),            // Amsterdam (Euronext)
        "SBF" => Some("PA"),            // Paris (Euronext)
        "BM" => Some("MC"),             // Madrid
        "LSE" => Some("L"),             // London
        "TSE" => Some("TO"),            // Toronto
        "ASX" => Some("AX"),            // Australia
        _ => None,                      // US exchanges: NASDAQ, NYSE, AMEX, ARCA, BATS, PINK, etc.
    }
}

fn naive_date_to_offset_datetime(date: NaiveDate) -> time::OffsetDateTime {
    let td = time::Date::from_calendar_date(
        date.year(),
        time::Month::try_from(date.month() as u8).unwrap(),
        date.day() as u8,
    )
    .unwrap();
    td.with_hms(0, 0, 0).unwrap().assume_utc()
}

fn naive_date_to_offset_datetime_eod(date: NaiveDate) -> time::OffsetDateTime {
    let td = time::Date::from_calendar_date(
        date.year(),
        time::Month::try_from(date.month() as u8).unwrap(),
        date.day() as u8,
    )
    .unwrap();
    td.with_hms(23, 59, 59).unwrap().assume_utc()
}
