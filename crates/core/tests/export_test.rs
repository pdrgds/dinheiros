use chrono::NaiveDate;
use tempfile::NamedTempFile;

use dinheiros_core::db::queries;
use dinheiros_core::db::Database;
use dinheiros_core::export;
use dinheiros_core::*;

#[test]
fn test_export_and_import_roundtrip() {
    let db = Database::open_in_memory().unwrap();

    // Insert a transaction
    queries::insert_transaction(
        &db,
        &Transaction {
            id: None,
            source: Source::B3,
            asset_type: AssetType::StockBr,
            symbol: "PETR4".to_string(),
            tx_type: TxType::Buy,
            date: NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(),
            quantity: 100.0,
            unit_price: Some(28.50),
            currency: "BRL".to_string(),
            total_value: 2850.0,
            brl_rate: 1.0,
            total_brl: 2850.0,
            commission: Some(4.90),
            fee_brl: None,
            notes: None,
            import_hash: "tx_hash_001".to_string(),
        },
    )
    .unwrap();

    // Insert a price
    queries::upsert_daily_price(
        &db,
        &DailyPrice {
            symbol: "PETR4".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(),
            close_price: 38.0,
            currency: "BRL".to_string(),
            brl_rate: 1.0,
        },
    )
    .unwrap();

    // Insert config
    queries::set_config(&db, "coingecko_api_key", "test_token").unwrap();

    // Insert income
    queries::insert_income(
        &db,
        &Income {
            id: None,
            source: Source::B3,
            symbol: "PETR4".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            income_type: IncomeType::Dividend,
            currency: "BRL".to_string(),
            gross_value: 150.0,
            tax_withheld: Some(22.5),
            tax_origin: Some("BR".to_string()),
            brl_rate: 1.0,
            net_value_brl: 127.5,
            import_hash: "income_hash_001".to_string(),
        },
    )
    .unwrap();

    // Export
    let tmp = NamedTempFile::new().unwrap();
    export::export_to_json(&db, tmp.path()).unwrap();

    // Import into fresh DB
    let db2 = Database::open_in_memory().unwrap();
    export::import_from_json(&db2, tmp.path()).unwrap();

    // Verify transactions survived
    let txs = queries::get_all_transactions(&db2).unwrap();
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].symbol, "PETR4");
    assert_eq!(txs[0].quantity, 100.0);
    assert_eq!(txs[0].source, Source::B3);
    assert_eq!(txs[0].tx_type, TxType::Buy);

    // Verify price survived
    let price = queries::get_latest_price(&db2, "PETR4").unwrap().unwrap();
    assert_eq!(price.close_price, 38.0);

    // Verify config survived
    let token = queries::get_config(&db2, "coingecko_api_key")
        .unwrap()
        .unwrap();
    assert_eq!(token, "test_token");

    // Verify income survived
    let inc = queries::get_all_income(&db2).unwrap();
    assert_eq!(inc.len(), 1);
    assert_eq!(inc[0].symbol, "PETR4");
    assert_eq!(inc[0].gross_value, 150.0);
}

#[test]
fn test_export_produces_valid_json() {
    let db = Database::open_in_memory().unwrap();

    queries::insert_transaction(
        &db,
        &Transaction {
            id: None,
            source: Source::Ibkr,
            asset_type: AssetType::StockIntl,
            symbol: "AAPL".to_string(),
            tx_type: TxType::Buy,
            date: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
            quantity: 10.0,
            unit_price: Some(175.0),
            currency: "USD".to_string(),
            total_value: 1750.0,
            brl_rate: 5.0,
            total_brl: 8750.0,
            commission: None,
            fee_brl: None,
            notes: Some("test note".to_string()),
            import_hash: "aapl_hash".to_string(),
        },
    )
    .unwrap();

    let tmp = NamedTempFile::new().unwrap();
    export::export_to_json(&db, tmp.path()).unwrap();

    let content = std::fs::read_to_string(tmp.path()).unwrap();
    let data: ExportData = serde_json::from_str(&content).unwrap();

    assert_eq!(data.version, 1);
    assert!(!data.exported_at.is_empty());
    assert_eq!(data.transactions.len(), 1);
    assert_eq!(data.transactions[0].symbol, "AAPL");
}

#[test]
fn test_import_deduplicates_transactions() {
    let db = Database::open_in_memory().unwrap();

    let tx = Transaction {
        id: None,
        source: Source::B3,
        asset_type: AssetType::StockBr,
        symbol: "VALE3".to_string(),
        tx_type: TxType::Buy,
        date: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        quantity: 50.0,
        unit_price: Some(60.0),
        currency: "BRL".to_string(),
        total_value: 3000.0,
        brl_rate: 1.0,
        total_brl: 3000.0,
        commission: None,
        fee_brl: None,
        notes: None,
        import_hash: "vale_hash".to_string(),
    };

    // Pre-insert the transaction
    queries::insert_transaction(&db, &tx).unwrap();

    // Export from a source DB that has the same transaction
    let source_db = Database::open_in_memory().unwrap();
    queries::insert_transaction(&source_db, &tx).unwrap();

    let tmp = NamedTempFile::new().unwrap();
    export::export_to_json(&source_db, tmp.path()).unwrap();

    // Import into db that already has the record -- should not duplicate
    export::import_from_json(&db, tmp.path()).unwrap();

    let txs = queries::get_all_transactions(&db).unwrap();
    assert_eq!(txs.len(), 1);
}

#[test]
fn test_empty_database_export_import() {
    let db = Database::open_in_memory().unwrap();

    let tmp = NamedTempFile::new().unwrap();
    export::export_to_json(&db, tmp.path()).unwrap();

    let db2 = Database::open_in_memory().unwrap();
    export::import_from_json(&db2, tmp.path()).unwrap();

    assert!(queries::get_all_transactions(&db2).unwrap().is_empty());
    assert!(queries::get_all_income(&db2).unwrap().is_empty());
}
