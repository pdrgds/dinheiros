use chrono::NaiveDate;
use dinheiros_core::parsers::ibkr_flex::{
    classify_ibkr_asset, dividend_to_income, parse_dividend_description, parse_tax_description,
    trade_to_transaction,
};
use dinheiros_core::{AssetType, TxType};

#[test]
fn test_classify_ibkr_currency() {
    assert_eq!(classify_ibkr_asset("LUNR", "USD"), AssetType::StockIntl);
    assert_eq!(classify_ibkr_asset("NKT", "DKK"), AssetType::StockIntl);
}

#[test]
fn test_parse_ibkr_dividend_description() {
    let (sym, ps) = parse_dividend_description(
        "RILY(US05580M1080) Cash Dividend USD 0.50 per Share (Ordinary Dividend)",
    );
    assert_eq!(sym, "RILY");
    assert_eq!(ps, "0.50");
}

#[test]
fn test_parse_ibkr_dividend_description_longer_decimal() {
    let (sym, ps) = parse_dividend_description(
        "NVO(US6701002056) Cash Dividend USD 0.516901 per Share (Ordinary Dividend)",
    );
    assert_eq!(sym, "NVO");
    assert_eq!(ps, "0.516901");
}

#[test]
fn test_parse_ibkr_tax_description() {
    let (sym, origin) = parse_tax_description(
        "NVO(US6701002056) Cash Dividend USD 0.516901 per Share - DK Tax",
    );
    assert_eq!(sym, "NVO");
    assert_eq!(origin, "DK");
}

#[test]
fn test_parse_ibkr_tax_description_us() {
    let (sym, origin) = parse_tax_description(
        "RILY(US05580M1080) Cash Dividend USD 0.50 per Share - US Tax",
    );
    assert_eq!(sym, "RILY");
    assert_eq!(origin, "US");
}

#[test]
fn test_trade_to_transaction_buy() {
    let tx = trade_to_transaction(
        "LUNR",
        "USD",
        NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        10.0,
        20.0,
        -200.0,
        -5.0,
        5.5,
    );
    assert_eq!(tx.tx_type, TxType::Buy);
    assert_eq!(tx.quantity, 10.0);
    assert_eq!(tx.total_value, 200.0); // abs of proceeds
    assert_eq!(tx.total_brl, 1100.0); // 200 * 5.5
    assert_eq!(tx.commission, Some(5.0));
    assert_eq!(tx.asset_type, AssetType::StockIntl);
    assert_eq!(tx.currency, "USD");
    assert!(!tx.import_hash.is_empty());
    assert_eq!(tx.import_hash.len(), 64); // SHA256 hex
}

#[test]
fn test_trade_to_transaction_sell() {
    let tx = trade_to_transaction(
        "LUNR",
        "USD",
        NaiveDate::from_ymd_opt(2024, 10, 1).unwrap(),
        -3.0,
        25.0,
        75.0,
        -2.0,
        5.2,
    );
    assert_eq!(tx.tx_type, TxType::Sell);
    assert_eq!(tx.quantity, 3.0);
    assert_eq!(tx.total_value, 75.0);
    assert_eq!(tx.total_brl, 390.0); // 75 * 5.2
    assert_eq!(tx.commission, Some(2.0));
}

#[test]
fn test_dividend_to_income() {
    let inc = dividend_to_income(
        "RILY",
        "USD",
        NaiveDate::from_ymd_opt(2024, 6, 11).unwrap(),
        3.0,
        -0.9,
        "US",
        5.5,
    );
    assert_eq!(inc.gross_value, 3.0);
    assert_eq!(inc.tax_withheld, Some(0.9));
    assert_eq!(inc.tax_origin, Some("US".to_string()));
    assert!((inc.net_value_brl - (2.1 * 5.5)).abs() < 0.01);
    assert!(!inc.import_hash.is_empty());
    assert_eq!(inc.import_hash.len(), 64);
}

#[test]
fn test_dividend_to_income_no_tax() {
    let inc = dividend_to_income(
        "LUNR",
        "USD",
        NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(),
        5.0,
        0.0,
        "",
        5.0,
    );
    assert_eq!(inc.gross_value, 5.0);
    assert_eq!(inc.tax_withheld, None);
    assert_eq!(inc.tax_origin, None);
    assert!((inc.net_value_brl - 25.0).abs() < 0.01); // 5.0 * 5.0
}

#[test]
fn test_trade_import_hash_deterministic() {
    let tx1 = trade_to_transaction(
        "LUNR",
        "USD",
        NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        10.0,
        20.0,
        -200.0,
        -5.0,
        5.5,
    );
    let tx2 = trade_to_transaction(
        "LUNR",
        "USD",
        NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        10.0,
        20.0,
        -200.0,
        -5.0,
        5.5,
    );
    assert_eq!(tx1.import_hash, tx2.import_hash);
}

#[test]
fn test_dividend_import_hash_deterministic() {
    let inc1 = dividend_to_income(
        "RILY",
        "USD",
        NaiveDate::from_ymd_opt(2024, 6, 11).unwrap(),
        3.0,
        -0.9,
        "US",
        5.5,
    );
    let inc2 = dividend_to_income(
        "RILY",
        "USD",
        NaiveDate::from_ymd_opt(2024, 6, 11).unwrap(),
        3.0,
        -0.9,
        "US",
        5.5,
    );
    assert_eq!(inc1.import_hash, inc2.import_hash);
}
