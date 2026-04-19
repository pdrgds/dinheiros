use chrono::NaiveDate;
use dinheiros_core::api::yahoo;

#[tokio::test]
async fn test_fetch_current_price_us_stock() {
    let price = yahoo::fetch_current_price("LUNR").await.unwrap();
    assert!(price > 0.0);
}

#[tokio::test]
async fn test_fetch_current_price_br_stock() {
    let price = yahoo::fetch_current_price("PETR4.SA").await.unwrap();
    assert!(price > 0.0);
}

#[tokio::test]
async fn test_fetch_current_price_gold() {
    let price = yahoo::fetch_current_price("GC=F").await.unwrap();
    assert!(price > 1000.0);
}

#[tokio::test]
async fn test_fetch_historical_prices() {
    let from = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    let to = NaiveDate::from_ymd_opt(2024, 12, 20).unwrap();
    let prices = yahoo::fetch_history("PETR4.SA", from, to).await.unwrap();
    assert!(
        prices.len() >= 10,
        "Expected at least 10 trading days, got {}",
        prices.len()
    );
}

#[test]
fn test_to_yahoo_symbol_br() {
    assert_eq!(yahoo::to_yahoo_symbol("PETR4", "stock_br"), "PETR4.SA");
}

#[test]
fn test_to_yahoo_symbol_gold() {
    assert_eq!(yahoo::to_yahoo_symbol("GOLD", "gold"), "GC=F");
}

#[test]
fn test_to_yahoo_symbol_passthrough() {
    assert_eq!(yahoo::to_yahoo_symbol("AAPL", "stock_us"), "AAPL");
}
