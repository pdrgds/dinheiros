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
        .user_agent("investimentos/0.1")
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

/// Fetch BTC/BRL daily prices over a date range. Returns (date, price) pairs.
/// CoinGecko may return multiple points per day; we keep the last price per day.
pub async fn fetch_btc_brl_history(
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<(NaiveDate, f64)>, Box<dyn std::error::Error>> {
    let from_ts = from
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();
    let to_ts = to
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_utc()
        .timestamp();

    let url = format!(
        "{}?vs_currency=brl&from={}&to={}",
        MARKET_CHART_RANGE_URL, from_ts, to_ts
    );

    let client = build_client();
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("CoinGecko API error: {}", body).into());
    }
    let resp: MarketChartResponse = response.json().await?;

    // Deduplicate: keep last price per day
    let mut by_date: BTreeMap<NaiveDate, f64> = BTreeMap::new();
    for (timestamp_ms, price) in &resp.prices {
        let secs = (*timestamp_ms as i64) / 1000;
        let dt = chrono::DateTime::from_timestamp(secs, 0)
            .ok_or("invalid timestamp from CoinGecko")?;
        let date = dt.date_naive();
        by_date.insert(date, *price);
    }

    Ok(by_date.into_iter().collect())
}
