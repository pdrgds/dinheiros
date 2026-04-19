use chrono::NaiveDate;
use serde::Deserialize;
use std::collections::BTreeMap;

const SIMPLE_PRICE_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=brl";

const MARKET_CHART_RANGE_URL: &str =
    "https://api.coingecko.com/api/v3/coins/bitcoin/market_chart/range";

#[derive(Debug, Deserialize)]
struct SimplePriceResponse {
    bitcoin: BtcPrice,
}

#[derive(Debug, Deserialize)]
struct BtcPrice {
    brl: f64,
}

#[derive(Debug, Deserialize)]
struct MarketChartResponse {
    prices: Vec<(f64, f64)>,
}

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("dinheiros/0.1")
        .build()
        .expect("failed to build reqwest client")
}

/// Fetch current BTC/BRL price.
pub async fn fetch_btc_brl_current() -> Result<f64, Box<dyn std::error::Error>> {
    let client = build_client();
    let resp = client.get(SIMPLE_PRICE_URL).send().await?.error_for_status()?;
    let parsed: SimplePriceResponse = resp.json().await?;
    Ok(parsed.bitcoin.brl)
}

/// Fetch BTC/BRL daily prices over a date range via Binance klines API.
/// Free, no API key, BTC/BRL direct, goes back to Oct 2020.
/// Returns (date, close_price) pairs.
pub async fn fetch_btc_brl_history(
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<(NaiveDate, f64)>, Box<dyn std::error::Error>> {
    let client = build_client();
    let mut results: BTreeMap<NaiveDate, f64> = BTreeMap::new();

    let mut start_ms = from
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp() * 1000;
    let end_ms = to
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_utc()
        .timestamp() * 1000;

    // Binance returns max 1000 candles per request — paginate
    loop {
        if start_ms > end_ms {
            break;
        }

        let url = format!(
            "https://api.binance.com/api/v3/klines?symbol=BTCBRL&interval=1d&startTime={}&endTime={}&limit=1000",
            start_ms, end_ms
        );

        let resp = client.get(&url).send().await?.error_for_status()?;
        let candles: Vec<Vec<serde_json::Value>> = resp.json().await?;

        if candles.is_empty() {
            break;
        }

        for candle in &candles {
            // [openTime, open, high, low, close, volume, closeTime, ...]
            if candle.len() < 7 {
                continue;
            }
            let open_time_ms = candle[0].as_i64().unwrap_or(0);
            let close_str = candle[4].as_str().unwrap_or("0");
            let close: f64 = close_str.parse().unwrap_or(0.0);

            let secs = open_time_ms / 1000;
            if let Some(dt) = chrono::DateTime::from_timestamp(secs, 0) {
                results.insert(dt.date_naive(), close);
            }
        }

        // Next page: start after the last candle's close time
        let last_close_time = candles.last()
            .and_then(|c| c.get(6))
            .and_then(|v| v.as_i64())
            .unwrap_or(end_ms);
        start_ms = last_close_time + 1;

        if candles.len() < 1000 {
            break; // no more data
        }
    }

    Ok(results.into_iter().collect())
}
