use std::path::PathBuf;

use chrono::NaiveDate;
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

    let buys: Vec<_> = result
        .transactions
        .iter()
        .filter(|t| t.tx_type == TxType::Buy)
        .collect();
    assert!(!buys.is_empty(), "Should have Buy transactions");

    // Priced trades must carry a unit_price; corporate-action rows
    // (Desdobro, Bonificação) legitimately have no price column.
    for tx in &result.transactions {
        if tx.total_value > 0.001 {
            let pu = tx.unit_price.unwrap_or(0.0);
            assert!(pu > 0.0, "priced tx must have unit_price > 0: {:?}", tx);
        }
    }
}

/// Maps a B3 "Transferência - Liquidação" row whose `Entrada/Saída` is `Credito`
/// to TxType::Buy. These rows are settlement records of a real purchase — the
/// shares are credited to the broker. The legacy parser misclassified them as
/// Sell, which dropped Pedro's entire Nu Invest buy history (those buys never
/// surface as `Compra` in B3 because Nu Invest reports differently from BB).
#[test]
fn test_b3_liquidacao_credito_classified_as_buy() {
    let path = fixture_path("movimentacao-2026-04-05-19-45-03.xlsx");
    let result = parse_b3_xlsx(&path).expect("failed to parse XLSX");

    let tx = result
        .transactions
        .iter()
        .find(|t| {
            t.symbol == "BBAS3"
                && t.date == NaiveDate::from_ymd_opt(2024, 4, 2).unwrap()
                && (t.quantity - 8.0).abs() < 0.001
        })
        .expect("BBAS3 02/04/2024 Liquidação Credito row missing");

    assert_eq!(tx.tx_type, TxType::Buy);
    assert!((tx.total_value - 453.60).abs() < 0.01, "total_value: {}", tx.total_value);
    assert!((tx.unit_price.unwrap() - 56.70).abs() < 0.01);
}

/// Maps a `Liquidação Debito` row to TxType::Sell with proceeds.
#[test]
fn test_b3_liquidacao_debito_classified_as_sell() {
    let path = fixture_path("movimentacao-2026-04-05-19-45-03.xlsx");
    let result = parse_b3_xlsx(&path).expect("failed to parse XLSX");

    let tx = result
        .transactions
        .iter()
        .find(|t| {
            t.symbol == "BBAS3"
                && t.date == NaiveDate::from_ymd_opt(2024, 9, 27).unwrap()
                && (t.quantity - 73.0).abs() < 0.001
        })
        .expect("BBAS3 27/09/2024 Liquidação Debito row missing");

    assert_eq!(tx.tx_type, TxType::Sell);
    assert!((tx.total_value - 2003.85).abs() < 0.01, "total_value: {}", tx.total_value);
}

/// Plain `Transferência` rows (without "- Liquidação") record an inter-broker
/// custody change. Both legs (Debito at source, Credito at destination) appear
/// in the same B3 export, and `compute_positions` groups by symbol — so
/// processing both legs would double-count and corrupt the avg-cost math. The
/// parser drops them; the priced Liquidação rows on each side cover the real
/// buy/sell economics.
#[test]
fn test_b3_plain_transferencia_dropped() {
    let path = fixture_path("movimentacao-2026-04-05-19-45-03.xlsx");
    let result = parse_b3_xlsx(&path).expect("failed to parse XLSX");

    // BBAS3 has both a Credito (8 shares to BB) and Debito (8 shares from Nu)
    // plain Transferência on 05/04/2024. Neither should reach transactions.
    let on_05_04: Vec<_> = result
        .transactions
        .iter()
        .filter(|t| {
            t.symbol == "BBAS3"
                && t.date == NaiveDate::from_ymd_opt(2024, 4, 5).unwrap()
        })
        .collect();
    assert!(
        on_05_04.is_empty(),
        "Plain Transferência rows must not appear in transactions, got {:?}",
        on_05_04
    );
}

/// `Desdobro` (stock split) and `Bonificação em Ativos` (bonus shares) credit
/// new shares without a price. They must produce Buy rows with `total_value=0`
/// so `compute_positions` adds quantity but preserves cost basis from the
/// existing position.
#[test]
fn test_b3_desdobro_emits_buy_with_zero_cost() {
    let path = fixture_path("movimentacao-2026-04-05-19-45-03.xlsx");
    let result = parse_b3_xlsx(&path).expect("failed to parse XLSX");

    let tx = result
        .transactions
        .iter()
        .find(|t| {
            t.symbol == "BBAS3"
                && t.date == NaiveDate::from_ymd_opt(2024, 4, 17).unwrap()
                && (t.quantity - 15.0).abs() < 0.001
        })
        .expect("BBAS3 17/04/2024 Desdobro row missing");

    assert_eq!(tx.tx_type, TxType::Buy);
    assert!(tx.total_value.abs() < 0.01, "Desdobro must have zero cost basis");
}

/// End-to-end check: a symbol Pedro bought at Nu, transferred to BB, and
/// eventually sold should now have both Buys and a Sell — not the all-Sell
/// pattern the legacy parser produced.
#[test]
fn test_b3_transferred_symbol_has_buys_and_sell() {
    let path = fixture_path("movimentacao-2026-04-05-19-45-03.xlsx");
    let result = parse_b3_xlsx(&path).expect("failed to parse XLSX");

    let bbas3: Vec<_> = result
        .transactions
        .iter()
        .filter(|t| t.symbol == "BBAS3")
        .collect();
    let buys = bbas3.iter().filter(|t| t.tx_type == TxType::Buy).count();
    let sells = bbas3.iter().filter(|t| t.tx_type == TxType::Sell).count();

    // Expected: 4 Liquidação Credito Buys + 1 Desdobro Buy = 5 Buys, 1 Liquidação Debito Sell.
    assert!(
        buys >= 4,
        "expected ≥4 BBAS3 Buys, got {} (Pedro's Nu buys + BB buys + Desdobro)",
        buys
    );
    assert_eq!(sells, 1, "expected exactly 1 BBAS3 Sell (the Liquidação Debito)");
}
