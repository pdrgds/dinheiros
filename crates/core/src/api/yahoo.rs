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
/// Everything else: return as-is
pub fn to_yahoo_symbol(symbol: &str, asset_type: &str) -> String {
    match asset_type {
        "stock_br" => format!("{}.SA", symbol),
        "gold" => "GC=F".to_string(),
        _ => symbol.to_string(),
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
