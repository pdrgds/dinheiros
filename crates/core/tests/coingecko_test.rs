use chrono::NaiveDate;
use dinheiros_core::api::coingecko;

#[tokio::test]
async fn test_fetch_btc_brl_current() {
    let price = coingecko::fetch_btc_brl_current().await.unwrap();
    assert!(price > 100_000.0, "BTC/BRL was {}", price);
}

#[tokio::test]
async fn test_fetch_btc_brl_history() {
    // CoinGecko free API limits historical data to past 365 days.
    let from = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    let to = NaiveDate::from_ymd_opt(2026, 3, 20).unwrap();
    let prices = coingecko::fetch_btc_brl_history(from, to).await.unwrap();
    assert!(
        prices.len() >= 15,
        "Expected at least 15 days, got {}",
        prices.len()
    );
}
