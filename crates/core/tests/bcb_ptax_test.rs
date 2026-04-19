use chrono::NaiveDate;
use dinheiros_core::api::bcb_ptax;

#[tokio::test]
async fn test_fetch_usd_brl_rate() {
    let date = NaiveDate::from_ymd_opt(2024, 12, 20).unwrap(); // Known business day
    let rate = bcb_ptax::fetch_rate("USD", date).await.unwrap();
    assert!(rate > 5.0 && rate < 7.0, "USD/BRL was {}", rate);
}

#[tokio::test]
async fn test_fetch_eur_brl_rate() {
    let date = NaiveDate::from_ymd_opt(2024, 12, 20).unwrap();
    let rate = bcb_ptax::fetch_rate("EUR", date).await.unwrap();
    assert!(rate > 5.0 && rate < 8.0, "EUR/BRL was {}", rate);
}

#[tokio::test]
async fn test_fetch_rate_weekend_falls_back() {
    let date = NaiveDate::from_ymd_opt(2024, 12, 21).unwrap(); // Saturday
    let rate = bcb_ptax::fetch_rate("USD", date).await.unwrap();
    assert!(rate > 5.0 && rate < 7.0);
}

#[tokio::test]
async fn test_fetch_usd_rates_range() {
    let from = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    let to = NaiveDate::from_ymd_opt(2024, 12, 20).unwrap();
    let rates = bcb_ptax::fetch_rates_range("USD", from, to).await.unwrap();
    assert!(
        rates.len() >= 10,
        "Should have ~14 business days, got {}",
        rates.len()
    );
}
