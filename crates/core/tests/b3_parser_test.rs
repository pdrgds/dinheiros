mod common;

use chrono::NaiveDate;
use dinheiros_core::parsers::b3::{extract_symbol, parse_b3_xlsx};
use dinheiros_core::{AssetType, IncomeType, Source, TxType};

#[test]
fn test_parse_b3_xlsx_basic_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_b3_sample_xlsx(tmp.path());
    let result = parse_b3_xlsx(&path).expect("failed to parse B3 XLSX");

    assert!(!result.transactions.is_empty());
    assert!(!result.income.is_empty());

    assert!(
        result.transactions.iter().any(|t| t.asset_type == AssetType::StockBr),
        "Should have StockBr transactions"
    );
    assert!(
        result.transactions.iter().any(|t| t.asset_type == AssetType::Tesouro),
        "Should have Tesouro transactions"
    );
    assert!(
        result.income.iter().any(|i| i.income_type == IncomeType::Dividend),
        "Should have Dividend income"
    );
    assert!(
        result.income.iter().any(|i| i.income_type == IncomeType::Jcp),
        "Should have JCP income"
    );

    for tx in &result.transactions {
        assert_eq!(tx.source, Source::B3);
        assert_eq!(tx.currency, "BRL");
        assert_eq!(tx.brl_rate, 1.0);
    }
    for inc in &result.income {
        assert_eq!(inc.source, Source::B3);
        assert_eq!(inc.currency, "BRL");
        assert_eq!(inc.brl_rate, 1.0);
    }

    let petr4_div = result
        .income
        .iter()
        .find(|i| i.symbol == "PETR4" && i.income_type == IncomeType::Dividend)
        .expect("PETR4 dividend missing");
    assert_eq!(petr4_div.tax_origin, Some("BR".to_string()));
    assert!(petr4_div.tax_withheld.is_none(), "Dividends are tax-exempt in Brazil");
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
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_b3_sample_xlsx(tmp.path());
    let result = parse_b3_xlsx(&path).expect("failed to parse B3 XLSX");

    let jcp: Vec<_> = result
        .income
        .iter()
        .filter(|i| i.income_type == IncomeType::Jcp)
        .collect();
    assert!(!jcp.is_empty(), "fixture must include a JCP row");

    for j in &jcp {
        let expected_tax = j.gross_value * 0.15;
        let expected_net = j.gross_value - expected_tax;
        let tax = j.tax_withheld.expect("JCP must carry tax_withheld");
        assert!(
            (tax - expected_tax).abs() < 0.01,
            "JCP tax should be 15% of gross: expected {}, got {}",
            expected_tax,
            tax
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
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_b3_sample_xlsx(tmp.path());
    let r1 = parse_b3_xlsx(&path).expect("parse 1");
    let r2 = parse_b3_xlsx(&path).expect("parse 2");

    assert_eq!(r1.transactions.len(), r2.transactions.len());
    for (a, b) in r1.transactions.iter().zip(r2.transactions.iter()) {
        assert_eq!(a.import_hash, b.import_hash);
        assert_eq!(a.import_hash.len(), 64, "SHA256 hex should be 64 chars");
    }
    assert_eq!(r1.income.len(), r2.income.len());
    for (a, b) in r1.income.iter().zip(r2.income.iter()) {
        assert_eq!(a.import_hash, b.import_hash);
    }
}

#[test]
fn test_b3_priced_transactions_have_unit_price() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_b3_sample_xlsx(tmp.path());
    let result = parse_b3_xlsx(&path).expect("failed to parse B3 XLSX");

    assert!(
        result.transactions.iter().any(|t| t.tx_type == TxType::Buy),
        "fixture must include at least one Buy"
    );

    // Corporate-action rows (Desdobro, Bonificação) legitimately have no price;
    // every other priced trade must carry a non-zero unit_price.
    for tx in &result.transactions {
        if tx.total_value > 0.001 {
            let pu = tx.unit_price.unwrap_or(0.0);
            assert!(pu > 0.0, "priced tx must have unit_price > 0: {:?}", tx);
        }
    }
}

/// A `Transferência - Liquidação` row with `Entrada/Saída = Credito` is the
/// B3 settlement record for a buy: the shares are credited to the broker. The
/// parser must classify it as TxType::Buy (the legacy parser misclassified all
/// of these as Sell, dropping entire buy histories).
#[test]
fn test_b3_liquidacao_credito_classified_as_buy() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_b3_sample_xlsx(tmp.path());
    let result = parse_b3_xlsx(&path).expect("failed to parse B3 XLSX");

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

/// Symmetrically, `Liquidação Debito` is a settled sell.
#[test]
fn test_b3_liquidacao_debito_classified_as_sell() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_b3_sample_xlsx(tmp.path());
    let result = parse_b3_xlsx(&path).expect("failed to parse B3 XLSX");

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
/// in the export; `compute_positions` groups by symbol, so processing both legs
/// would double-count and corrupt avg-cost. The parser drops them.
#[test]
fn test_b3_plain_transferencia_dropped() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_b3_sample_xlsx(tmp.path());
    let result = parse_b3_xlsx(&path).expect("failed to parse B3 XLSX");

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
/// new shares without a price. They emit Buy rows with `total_value=0` so
/// `compute_positions` adds quantity but preserves cost basis.
#[test]
fn test_b3_desdobro_emits_buy_with_zero_cost() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_b3_sample_xlsx(tmp.path());
    let result = parse_b3_xlsx(&path).expect("failed to parse B3 XLSX");

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

/// End-to-end check on the inter-broker pattern: a symbol that arrives via
/// multiple Liquidação Credito buys, undergoes a Desdobro, and is eventually
/// sold should surface both buys and the sell — not the all-sell pattern the
/// legacy parser produced.
#[test]
fn test_b3_transferred_symbol_has_buys_and_sell() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_b3_sample_xlsx(tmp.path());
    let result = parse_b3_xlsx(&path).expect("failed to parse B3 XLSX");

    let bbas3: Vec<_> = result.transactions.iter().filter(|t| t.symbol == "BBAS3").collect();
    let buys = bbas3.iter().filter(|t| t.tx_type == TxType::Buy).count();
    let sells = bbas3.iter().filter(|t| t.tx_type == TxType::Sell).count();

    // Fixture: 4 Liquidação Credito Buys + 1 Desdobro Buy + 1 Liquidação Debito Sell.
    assert_eq!(buys, 5, "expected 5 BBAS3 Buys (4 Liq.Cr + 1 Desdobro), got {}", buys);
    assert_eq!(sells, 1, "expected exactly 1 BBAS3 Sell (the Liq.Db), got {}", sells);
}

/// CDB rows are automatic savings, not investments — the parser must skip them.
#[test]
fn test_b3_cdb_rows_are_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::write_b3_sample_xlsx(tmp.path());
    let result = parse_b3_xlsx(&path).expect("failed to parse B3 XLSX");

    assert!(
        !result.transactions.iter().any(|t| t.symbol.contains("CDB")),
        "CDB rows must be skipped"
    );
}
