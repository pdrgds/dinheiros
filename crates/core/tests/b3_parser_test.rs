use std::path::PathBuf;

use dinheiros_core::parsers::b3::{extract_symbol, parse_b3_xlsx};
use dinheiros_core::{AssetType, IncomeType, Source, TxType};

fn fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../input-files/b3");
    path.push(filename);
    path
}

#[test]
fn test_parse_b3_xlsx_2024() {
    let path = fixture_path("movimentacao-2026-04-05-19-45-03.xlsx");
    let result = parse_b3_xlsx(&path).expect("failed to parse 2024 XLSX");

    // Should have both transactions and income
    assert!(!result.transactions.is_empty());
    assert!(!result.income.is_empty());

    // Verify has StockBr transactions
    let stock_br: Vec<_> = result
        .transactions
        .iter()
        .filter(|t| t.asset_type == AssetType::StockBr)
        .collect();
    assert!(!stock_br.is_empty(), "Should have StockBr transactions");

    // Verify has Tesouro transactions
    let tesouro: Vec<_> = result
        .transactions
        .iter()
        .filter(|t| t.asset_type == AssetType::Tesouro)
        .collect();
    assert!(!tesouro.is_empty(), "Should have Tesouro transactions");

    // Verify has Dividend income
    let dividends: Vec<_> = result
        .income
        .iter()
        .filter(|i| i.income_type == IncomeType::Dividend)
        .collect();
    assert!(!dividends.is_empty(), "Should have Dividend income");

    // Verify has JCP income
    let jcp: Vec<_> = result
        .income
        .iter()
        .filter(|i| i.income_type == IncomeType::Jcp)
        .collect();
    assert!(!jcp.is_empty(), "Should have JCP income");

    // All transactions should be from B3 source
    for tx in &result.transactions {
        assert_eq!(tx.source, Source::B3);
        assert_eq!(tx.currency, "BRL");
        assert_eq!(tx.brl_rate, 1.0);
    }

    // All income should be from B3 source
    for inc in &result.income {
        assert_eq!(inc.source, Source::B3);
        assert_eq!(inc.currency, "BRL");
        assert_eq!(inc.brl_rate, 1.0);
    }

    // Verify a specific PETR4 dividend exists
    let petr4_divs: Vec<_> = result
        .income
        .iter()
        .filter(|i| i.symbol == "PETR4" && i.income_type == IncomeType::Dividend)
        .collect();
    assert!(
        !petr4_divs.is_empty(),
        "Should have at least one PETR4 dividend"
    );
    let petr4_div = &petr4_divs[0];
    assert_eq!(petr4_div.currency, "BRL");
    assert_eq!(petr4_div.brl_rate, 1.0);
    assert!(petr4_div.tax_withheld.is_none(), "Dividends are tax-exempt in Brazil");
    assert_eq!(petr4_div.tax_origin, Some("BR".to_string()));
}

#[test]
fn test_parse_b3_xlsx_2025() {
    let path = fixture_path("movimentacao-2026-04-05-19-45-26.xlsx");
    let result = parse_b3_xlsx(&path).expect("failed to parse 2025 XLSX");

    assert!(
        !result.transactions.is_empty() || !result.income.is_empty(),
        "Should have some records"
    );

    // All records should have BRL currency
    for tx in &result.transactions {
        assert_eq!(tx.currency, "BRL");
    }
    for inc in &result.income {
        assert_eq!(inc.currency, "BRL");
    }
}

#[test]
fn test_parse_b3_xlsx_2026() {
    let path = fixture_path("movimentacao-2026-04-05-19-47-06.xlsx");
    let result = parse_b3_xlsx(&path).expect("failed to parse 2026 XLSX");

    assert!(
        !result.transactions.is_empty() || !result.income.is_empty(),
        "Should have some records"
    );
}

#[test]
fn test_b3_symbol_extraction() {
    assert_eq!(extract_symbol("CMIG3 - CIA. ENERGETICA DE MINAS GERAIS"), "CMIG3");
    assert_eq!(extract_symbol("Tesouro Selic 2029"), "Tesouro Selic 2029");
    assert_eq!(
        extract_symbol("PETR4 - PETROLEO BRASILEIRO S/A - PETROBRAS"),
        "PETR4"
    );
    assert_eq!(extract_symbol("BBAS3 - BANCO DO BRASIL"), "BBAS3");
    assert_eq!(extract_symbol("Tesouro IPCA+ 2035"), "Tesouro IPCA+ 2035");
}

#[test]
fn test_b3_jcp_tax_calculation() {
    let path = fixture_path("movimentacao-2026-04-05-19-45-03.xlsx");
    let result = parse_b3_xlsx(&path).expect("failed to parse XLSX");

    let jcp: Vec<_> = result
        .income
        .iter()
        .filter(|i| i.income_type == IncomeType::Jcp)
        .collect();

    for j in &jcp {
        let expected_tax = j.gross_value * 0.15;
        let expected_net = j.gross_value - expected_tax;
        assert!(
            j.tax_withheld.is_some(),
            "JCP should have tax withheld"
        );
        assert!(
            (j.tax_withheld.unwrap() - expected_tax).abs() < 0.01,
            "JCP tax should be 15% of gross: expected {}, got {}",
            expected_tax,
            j.tax_withheld.unwrap()
        );
        assert!(
            (j.net_value_brl - expected_net).abs() < 0.01,
            "JCP net should be gross - tax: expected {}, got {}",
            expected_net,
            j.net_value_brl
        );
        assert_eq!(j.tax_origin, Some("BR".to_string()));
    }
}

#[test]
fn test_b3_import_hash_deterministic() {
    let path = fixture_path("movimentacao-2026-04-05-19-45-03.xlsx");
    let result1 = parse_b3_xlsx(&path).expect("parse 1");
    let result2 = parse_b3_xlsx(&path).expect("parse 2");

    assert_eq!(result1.transactions.len(), result2.transactions.len());
    for (t1, t2) in result1.transactions.iter().zip(result2.transactions.iter()) {
        assert_eq!(t1.import_hash, t2.import_hash);
        assert_eq!(t1.import_hash.len(), 64, "SHA256 hex should be 64 chars");
    }

    assert_eq!(result1.income.len(), result2.income.len());
    for (i1, i2) in result1.income.iter().zip(result2.income.iter()) {
        assert_eq!(i1.import_hash, i2.import_hash);
    }
}

#[test]
fn test_b3_transaction_types() {
    let path = fixture_path("movimentacao-2026-04-05-19-45-03.xlsx");
    let result = parse_b3_xlsx(&path).expect("failed to parse XLSX");

    // Verify we have Buy transactions
    let buys: Vec<_> = result
        .transactions
        .iter()
        .filter(|t| t.tx_type == TxType::Buy)
        .collect();
    assert!(!buys.is_empty(), "Should have Buy transactions");

    // All transactions should have valid unit_price
    for tx in &result.transactions {
        assert!(tx.unit_price.is_some());
        assert!(tx.unit_price.unwrap() > 0.0);
    }
}
