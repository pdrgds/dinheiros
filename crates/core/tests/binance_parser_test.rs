mod common;

use dinheiros_core::parsers::binance::parse_binance_csv;
use dinheiros_core::{AssetType, Source, TxType};

#[test]
fn test_parse_binance_csv_sample_a() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_binance_sample_a(tmp.path());
    let result = parse_binance_csv(&path).expect("failed to parse Binance CSV");

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

    assert_eq!(buys.len(), 7);
    assert_eq!(sends.len(), 2);
    assert_eq!(deposits.len(), 0, "Deposits must be excluded");
    assert_eq!(result.transactions.len(), 9);

    let first_buy = &buys[0];
    assert_eq!(first_buy.symbol, "BTC");
    assert_eq!(first_buy.asset_type, AssetType::Crypto);
    assert_eq!(first_buy.source, Source::Binance);
    assert_eq!(first_buy.currency, "BRL");
    assert_eq!(first_buy.date.to_string(), "2024-04-13");
    assert!((first_buy.quantity - 0.00146106).abs() < 1e-10);
    assert!((first_buy.total_value - 512.0).abs() < 1e-6);
    assert!(first_buy.unit_price.is_some());

    assert!(!first_buy.import_hash.is_empty());
    assert_eq!(first_buy.import_hash.len(), 64);

    let first_send = &sends[0];
    assert_eq!(first_send.tx_type, TxType::Send);
    assert_eq!(first_send.currency, "BTC");
    assert!(first_send.notes.as_ref().unwrap().contains("bc1q"));
}

#[test]
fn test_parse_binance_csv_sample_b() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_binance_sample_b(tmp.path());
    let result = parse_binance_csv(&path).expect("failed to parse Binance CSV");

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

    let first_buy = &buys[0];
    assert_eq!(first_buy.date.to_string(), "2025-03-24");
}

#[test]
fn test_binance_btc_holdings_calculation_sample_a() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_binance_sample_a(tmp.path());
    let result = parse_binance_csv(&path).expect("failed to parse CSV");

    let total_bought: f64 = result
        .transactions
        .iter()
        .filter(|t| t.tx_type == TxType::Buy)
        .map(|t| t.quantity)
        .sum();

    // Sample A has only BRL-denominated fees, so total_fees_btc should be zero
    // and net_btc should equal total_bought exactly.
    assert!(result.net_btc > 0.0);
    assert_eq!(result.total_fees_btc, 0.0);
    assert!(
        (result.net_btc - total_bought).abs() < 1e-10,
        "net_btc ({}) should equal total_bought ({}) when no Bitcoin-currency fees",
        result.net_btc,
        total_bought,
    );
}

#[test]
fn test_binance_btc_holdings_calculation_sample_b() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_binance_sample_b(tmp.path());
    let result = parse_binance_csv(&path).expect("failed to parse CSV");

    let total_bought: f64 = result
        .transactions
        .iter()
        .filter(|t| t.tx_type == TxType::Buy)
        .map(|t| t.quantity)
        .sum();

    // Sample B's 4 Sends each pay a 0.00003 BTC fee with currency "Bitcoin"
    // (the parser accepts both "BTC" and "Bitcoin"), totalling 0.00012 BTC.
    assert!((result.total_fees_btc - 0.00012).abs() < 1e-10);
    assert!(
        (result.net_btc - (total_bought - result.total_fees_btc)).abs() < 1e-10,
        "net_btc ({}) should equal total_bought ({}) - total_fees_btc ({})",
        result.net_btc,
        total_bought,
        result.total_fees_btc
    );
}

#[test]
fn test_import_hash_deterministic() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_binance_sample_a(tmp.path());
    let result1 = parse_binance_csv(&path).expect("parse 1");
    let result2 = parse_binance_csv(&path).expect("parse 2");

    assert_eq!(result1.transactions.len(), result2.transactions.len());
    for (t1, t2) in result1.transactions.iter().zip(result2.transactions.iter()) {
        assert_eq!(t1.import_hash, t2.import_hash);
    }
}
