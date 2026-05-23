use chrono::NaiveDate;

use crate::types::{AssetType, DailyPrice, Income, IncomeType, Source, Transaction, TxType};

const TODAY: (i32, u32, u32) = (2026, 5, 23);

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
}

fn hash(symbol: &str, date: NaiveDate, seq: usize) -> String {
    format!("demo-{}-{}-{}", symbol, date, seq)
}

fn brl_buy(
    source: Source,
    asset_type: AssetType,
    symbol: &str,
    date: NaiveDate,
    qty: f64,
    unit_price: f64,
    seq: usize,
) -> Transaction {
    let total = qty * unit_price;
    Transaction {
        id: None,
        source,
        asset_type,
        symbol: symbol.to_string(),
        tx_type: TxType::Buy,
        date,
        quantity: qty,
        unit_price: Some(unit_price),
        currency: "BRL".to_string(),
        total_value: total,
        brl_rate: 1.0,
        total_brl: total,
        commission: Some(2.50),
        fee_brl: Some(2.50),
        notes: None,
        import_hash: hash(symbol, date, seq),
    }
}

fn usd_buy(
    symbol: &str,
    date: NaiveDate,
    qty: f64,
    unit_price_usd: f64,
    brl_rate: f64,
    seq: usize,
) -> Transaction {
    let total_usd = qty * unit_price_usd;
    Transaction {
        id: None,
        source: Source::Ibkr,
        asset_type: AssetType::StockIntl,
        symbol: symbol.to_string(),
        tx_type: TxType::Buy,
        date,
        quantity: qty,
        unit_price: Some(unit_price_usd),
        currency: "USD".to_string(),
        total_value: total_usd,
        brl_rate,
        total_brl: total_usd * brl_rate,
        commission: Some(1.00),
        fee_brl: Some(1.00 * brl_rate),
        notes: None,
        import_hash: hash(symbol, date, seq),
    }
}

pub fn all_transactions() -> Vec<Transaction> {
    let mut txs = Vec::new();

    // ---- B3 stocks (BRL) ----
    txs.push(brl_buy(
        Source::B3,
        AssetType::StockBr,
        "PETR4",
        d(2024, 12, 4),
        100.0,
        38.20,
        1,
    ));
    txs.push(brl_buy(
        Source::B3,
        AssetType::StockBr,
        "PETR4",
        d(2025, 6, 17),
        100.0,
        36.05,
        2,
    ));
    txs.push(brl_buy(
        Source::B3,
        AssetType::StockBr,
        "VALE3",
        d(2025, 1, 22),
        100.0,
        63.40,
        1,
    ));
    txs.push(brl_buy(
        Source::B3,
        AssetType::StockBr,
        "VALE3",
        d(2025, 9, 3),
        50.0,
        67.90,
        2,
    ));
    txs.push(brl_buy(
        Source::B3,
        AssetType::StockBr,
        "ITUB4",
        d(2024, 12, 12),
        300.0,
        30.10,
        1,
    ));
    txs.push(brl_buy(
        Source::B3,
        AssetType::StockBr,
        "BBAS3",
        d(2025, 3, 18),
        50.0,
        27.40,
        1,
    ));
    txs.push(brl_buy(
        Source::B3,
        AssetType::StockBr,
        "BBAS3",
        d(2025, 11, 7),
        50.0,
        25.80,
        2,
    ));
    txs.push(brl_buy(
        Source::B3,
        AssetType::StockBr,
        "WEGE3",
        d(2025, 4, 28),
        80.0,
        41.90,
        1,
    ));
    txs.push(brl_buy(
        Source::B3,
        AssetType::StockBr,
        "BOVA11",
        d(2025, 2, 5),
        50.0,
        118.20,
        1,
    ));
    txs.push(brl_buy(
        Source::B3,
        AssetType::StockBr,
        "IVVB11",
        d(2025, 11, 19),
        40.0,
        332.40,
        1,
    ));

    // ---- Tesouro Direto (BRL) ----
    txs.push(brl_buy(
        Source::B3,
        AssetType::Tesouro,
        "Tesouro Selic 2027",
        d(2025, 2, 14),
        1.0,
        5000.00,
        1,
    ));
    txs.push(brl_buy(
        Source::B3,
        AssetType::Tesouro,
        "Tesouro Selic 2027",
        d(2025, 8, 22),
        1.0,
        5000.00,
        2,
    ));
    txs.push(brl_buy(
        Source::B3,
        AssetType::Tesouro,
        "Tesouro IPCA+ 2035",
        d(2025, 5, 30),
        1.0,
        10000.00,
        1,
    ));

    // ---- IBKR (USD) ----
    txs.push(usd_buy("AAPL", d(2025, 10, 9), 15.0, 228.50, 5.18, 1));
    txs.push(usd_buy("MSFT", d(2025, 11, 14), 10.0, 415.30, 5.22, 1));
    txs.push(usd_buy("GOOGL", d(2026, 2, 20), 8.0, 175.10, 5.05, 1));

    // ---- Crypto (Binance, BRL) ----
    txs.push(brl_buy(
        Source::Binance,
        AssetType::Crypto,
        "BTC",
        d(2024, 12, 28),
        0.02,
        540_000.0,
        1,
    ));
    txs.push(brl_buy(
        Source::Binance,
        AssetType::Crypto,
        "BTC",
        d(2025, 4, 12),
        0.02,
        580_000.0,
        2,
    ));
    txs.push(brl_buy(
        Source::Binance,
        AssetType::Crypto,
        "BTC",
        d(2025, 9, 6),
        0.02,
        620_000.0,
        3,
    ));
    txs.push(brl_buy(
        Source::Binance,
        AssetType::Crypto,
        "BTC",
        d(2026, 2, 11),
        0.02,
        670_000.0,
        4,
    ));

    txs
}

fn dividend(symbol: &str, date: NaiveDate, gross: f64, seq: usize) -> Income {
    Income {
        id: None,
        source: Source::B3,
        symbol: symbol.to_string(),
        date,
        income_type: IncomeType::Dividend,
        currency: "BRL".to_string(),
        gross_value: gross,
        tax_withheld: None,
        tax_origin: None,
        brl_rate: 1.0,
        net_value_brl: gross,
        import_hash: hash(symbol, date, 100 + seq),
    }
}

fn jcp(symbol: &str, date: NaiveDate, gross: f64, seq: usize) -> Income {
    let tax = gross * 0.15;
    Income {
        id: None,
        source: Source::B3,
        symbol: symbol.to_string(),
        date,
        income_type: IncomeType::Jcp,
        currency: "BRL".to_string(),
        gross_value: gross,
        tax_withheld: Some(tax),
        tax_origin: Some("BR".to_string()),
        brl_rate: 1.0,
        net_value_brl: gross - tax,
        import_hash: hash(symbol, date, 200 + seq),
    }
}

pub fn all_income() -> Vec<Income> {
    vec![
        dividend("PETR4", d(2025, 3, 14), 180.00, 1),
        dividend("PETR4", d(2025, 6, 20), 165.00, 2),
        dividend("PETR4", d(2025, 9, 19), 190.00, 3),
        dividend("PETR4", d(2025, 12, 18), 175.00, 4),
        dividend("PETR4", d(2026, 3, 20), 195.00, 5),
        dividend("VALE3", d(2025, 4, 10), 240.00, 1),
        dividend("VALE3", d(2025, 10, 8), 260.00, 2),
        dividend("VALE3", d(2026, 4, 9), 285.00, 3),
        jcp("ITUB4", d(2025, 3, 3), 90.00, 1),
        jcp("ITUB4", d(2025, 6, 3), 105.00, 2),
        jcp("ITUB4", d(2025, 9, 2), 110.00, 3),
        jcp("ITUB4", d(2025, 12, 1), 120.00, 4),
        jcp("ITUB4", d(2026, 3, 2), 115.00, 5),
        jcp("BBAS3", d(2025, 8, 29), 55.00, 1),
        jcp("BBAS3", d(2026, 2, 27), 65.00, 2),
    ]
}

fn price(symbol: &str, close: f64, currency: &str, brl_rate: f64) -> DailyPrice {
    DailyPrice {
        symbol: symbol.to_string(),
        date: d(TODAY.0, TODAY.1, TODAY.2),
        close_price: close,
        currency: currency.to_string(),
        brl_rate,
    }
}

pub fn latest_prices() -> Vec<DailyPrice> {
    vec![
        // BRL — mix of winners and losers vs the avg buy prices above.
        price("PETR4", 34.10, "BRL", 1.0),   // loser
        price("VALE3", 72.50, "BRL", 1.0),   // winner
        price("ITUB4", 35.80, "BRL", 1.0),   // winner
        price("BBAS3", 24.90, "BRL", 1.0),   // loser
        price("WEGE3", 48.60, "BRL", 1.0),   // winner
        price("BOVA11", 112.40, "BRL", 1.0), // loser
        price("IVVB11", 318.70, "BRL", 1.0), // loser
        price("Tesouro Selic 2027", 5210.00, "BRL", 1.0),
        price("Tesouro IPCA+ 2035", 10840.00, "BRL", 1.0),
        // USD — winners
        price("AAPL", 248.20, "USD", 5.20),
        price("MSFT", 462.10, "USD", 5.20),
        price("GOOGL", 168.50, "USD", 5.20), // loser
        // Crypto — winner
        price("BTC", 720_000.00, "BRL", 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn transactions_have_unique_import_hashes() {
        let hashes: HashSet<_> = all_transactions()
            .into_iter()
            .map(|t| t.import_hash)
            .collect();
        assert_eq!(
            hashes.len(),
            all_transactions().len(),
            "duplicate import_hash in transactions"
        );
    }

    #[test]
    fn income_has_unique_import_hashes() {
        let hashes: HashSet<_> = all_income().into_iter().map(|i| i.import_hash).collect();
        assert_eq!(
            hashes.len(),
            all_income().len(),
            "duplicate import_hash in income"
        );
    }

    #[test]
    fn every_transacted_symbol_has_a_latest_price() {
        let symbols: HashSet<String> = all_transactions()
            .iter()
            .map(|t| t.symbol.clone())
            .collect();
        let priced: HashSet<String> = latest_prices().iter().map(|p| p.symbol.clone()).collect();
        let missing: Vec<_> = symbols.difference(&priced).collect();
        assert!(
            missing.is_empty(),
            "transacted symbols without a price: {:?}",
            missing
        );
    }

    #[test]
    fn dataset_is_non_empty() {
        assert!(!all_transactions().is_empty());
        assert!(!all_income().is_empty());
        assert!(!latest_prices().is_empty());
    }

    #[test]
    fn income_symbols_match_transacted_symbols() {
        let tx_symbols: HashSet<String> = all_transactions()
            .iter()
            .map(|t| t.symbol.clone())
            .collect();
        let orphans: Vec<_> = all_income()
            .iter()
            .filter(|i| !tx_symbols.contains(&i.symbol))
            .map(|i| i.symbol.clone())
            .collect();
        assert!(
            orphans.is_empty(),
            "income for un-held symbols: {:?}",
            orphans
        );
    }
}
