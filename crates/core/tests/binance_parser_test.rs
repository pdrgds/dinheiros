use std::path::PathBuf;

use investimentos_core::parsers::binance::parse_binance_csv;
use investimentos_core::{AssetType, Source, TxType};

fn fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../input-files/binance");
    path.push(filename);
    path
}

#[test]
fn test_parse_binance_csv_2024() {
    let path = fixture_path("2025_03_21_16_01_30.csv");
    let result = parse_binance_csv(&path).expect("failed to parse 2024 CSV");

    // Should have transactions (Buy + Send only, no Deposits)
    assert!(!result.transactions.is_empty());

    // Count by type
    let buys: Vec<_> = result
        .transactions
        .iter()
        .filter(|t| t.tx_type == TxType::Buy)
        .collect();
    let sends: Vec<_> = result
        .transactions
        .iter()
        .filter(|t| t.tx_type == TxType::Send)
        .collect();
    let deposits: Vec<_> = result
        .transactions
        .iter()
        .filter(|t| t.tx_type == TxType::Deposit)
        .collect();

    // 2024 file has 7 Buy, 2 Send, 4 Deposit rows. Deposits should be excluded.
    assert_eq!(buys.len(), 7);
    assert_eq!(sends.len(), 2);
    assert_eq!(deposits.len(), 0, "Deposits must be excluded");

    // Total transactions = buys + sends
    assert_eq!(result.transactions.len(), 9);

    // Verify first Buy transaction
    let first_buy = &buys[0];
    assert_eq!(first_buy.symbol, "BTC");
    assert_eq!(first_buy.asset_type, AssetType::Crypto);
    assert_eq!(first_buy.source, Source::Binance);
    assert_eq!(first_buy.currency, "BRL");
    assert_eq!(first_buy.date.to_string(), "2024-04-13");
    assert!((first_buy.quantity - 0.00146106).abs() < 1e-10);
    assert!((first_buy.total_value - 512.0).abs() < 1e-6);
    assert!(first_buy.unit_price.is_some());

    // Verify import_hash is non-empty and deterministic
    assert!(!first_buy.import_hash.is_empty());
    assert_eq!(first_buy.import_hash.len(), 64); // SHA256 hex

    // Verify Send transaction has address in notes
    let first_send = &sends[0];
    assert_eq!(first_send.tx_type, TxType::Send);
    assert_eq!(first_send.currency, "BTC");
    assert!(first_send.notes.as_ref().unwrap().contains("bc1q"));
}

#[test]
fn test_parse_binance_csv_2025() {
    let path = fixture_path("2026_02_26_05_32_16.csv");
    let result = parse_binance_csv(&path).expect("failed to parse 2025 CSV");

    assert!(!result.transactions.is_empty());

    // 2025 file has 4 Buy, 4 Send, 4 Deposit rows
    let buys: Vec<_> = result
        .transactions
        .iter()
        .filter(|t| t.tx_type == TxType::Buy)
        .collect();
    let sends: Vec<_> = result
        .transactions
        .iter()
        .filter(|t| t.tx_type == TxType::Send)
        .collect();

    assert_eq!(buys.len(), 4);
    assert_eq!(sends.len(), 4);
    assert_eq!(result.transactions.len(), 8);

    // Verify date parsing with space-separated format
    let first_buy = &buys[0];
    assert_eq!(first_buy.date.to_string(), "2025-03-24");
}

#[test]
fn test_binance_btc_holdings_calculation_2024() {
    let path = fixture_path("2025_03_21_16_01_30.csv");
    let result = parse_binance_csv(&path).expect("failed to parse CSV");

    // Sum of all Buy received_amounts
    let total_bought: f64 = result
        .transactions
        .iter()
        .filter(|t| t.tx_type == TxType::Buy)
        .map(|t| t.quantity)
        .sum();

    // net_btc should equal total_bought - total_fees_btc
    assert!(result.net_btc > 0.0);
    assert!(result.total_fees_btc > 0.0);
    assert!(
        (result.net_btc - (total_bought - result.total_fees_btc)).abs() < 1e-10,
        "net_btc ({}) should equal total_bought ({}) - total_fees_btc ({})",
        result.net_btc,
        total_bought,
        result.total_fees_btc
    );
}

#[test]
fn test_binance_btc_holdings_calculation_2025() {
    let path = fixture_path("2026_02_26_05_32_16.csv");
    let result = parse_binance_csv(&path).expect("failed to parse CSV");

    let total_bought: f64 = result
        .transactions
        .iter()
        .filter(|t| t.tx_type == TxType::Buy)
        .map(|t| t.quantity)
        .sum();

    assert!(result.net_btc > 0.0);
    assert!(result.total_fees_btc > 0.0);
    assert!(
        (result.net_btc - (total_bought - result.total_fees_btc)).abs() < 1e-10,
        "net_btc ({}) should equal total_bought ({}) - total_fees_btc ({})",
        result.net_btc,
        total_bought,
        result.total_fees_btc
    );

    // 2025 file has fees in "Bitcoin" currency - verify they're counted
    // 4 Send transactions each with 0.00003 BTC fee
    assert!((result.total_fees_btc - 0.00012).abs() < 1e-10);
}

#[test]
fn test_import_hash_deterministic() {
    let path = fixture_path("2025_03_21_16_01_30.csv");
    let result1 = parse_binance_csv(&path).expect("parse 1");
    let result2 = parse_binance_csv(&path).expect("parse 2");

    assert_eq!(result1.transactions.len(), result2.transactions.len());
    for (t1, t2) in result1.transactions.iter().zip(result2.transactions.iter()) {
        assert_eq!(t1.import_hash, t2.import_hash);
    }
}
