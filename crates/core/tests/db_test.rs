use chrono::NaiveDate;
use dinheiros_core::db::queries;
use dinheiros_core::db::Database;
use dinheiros_core::*;

#[test]
fn test_create_database_and_tables() {
    let db = Database::open_in_memory().unwrap();
    let tables = db.list_tables().unwrap();

    assert!(tables.contains(&"transactions".to_string()));
    assert!(tables.contains(&"daily_prices".to_string()));
    assert!(tables.contains(&"income".to_string()));
    assert!(tables.contains(&"config".to_string()));
}

#[test]
fn test_insert_and_query_transactions() {
    let db = Database::open_in_memory().unwrap();

    let tx = Transaction {
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
        import_hash: "abc123".to_string(),
    };

    // First insert should succeed
    let inserted = queries::insert_transaction(&db, &tx).unwrap();
    assert!(inserted);

    // Duplicate should be ignored (same import_hash)
    let duplicate = queries::insert_transaction(&db, &tx).unwrap();
    assert!(!duplicate);

    // Query by symbol
    let results = queries::get_transactions_by_symbol(&db, "PETR4").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].symbol, "PETR4");
    assert_eq!(results[0].quantity, 100.0);
    assert_eq!(results[0].source, Source::B3);
    assert_eq!(results[0].tx_type, TxType::Buy);
    assert!(results[0].id.is_some());

    // Query all
    let all = queries::get_all_transactions(&db).unwrap();
    assert_eq!(all.len(), 1);

    // Empty symbol query
    let empty = queries::get_transactions_by_symbol(&db, "VALE3").unwrap();
    assert!(empty.is_empty());
}

#[test]
fn test_daily_prices_upsert_and_query() {
    let db = Database::open_in_memory().unwrap();

    let price = DailyPrice {
        symbol: "PETR4".to_string(),
        date: NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(),
        close_price: 28.50,
        currency: "BRL".to_string(),
        brl_rate: 1.0,
    };

    // Insert
    queries::upsert_daily_price(&db, &price).unwrap();

    // Get latest price
    let latest = queries::get_latest_price(&db, "PETR4").unwrap();
    assert!(latest.is_some());
    let latest = latest.unwrap();
    assert_eq!(latest.close_price, 28.50);
    assert_eq!(latest.symbol, "PETR4");

    // Get latest date
    let latest_date = queries::get_latest_price_date(&db, "PETR4").unwrap();
    assert_eq!(
        latest_date,
        Some(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap())
    );

    // Upsert with updated price
    let updated_price = DailyPrice {
        symbol: "PETR4".to_string(),
        date: NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(),
        close_price: 29.00,
        currency: "BRL".to_string(),
        brl_rate: 1.0,
    };
    queries::upsert_daily_price(&db, &updated_price).unwrap();

    // Verify updated
    let latest = queries::get_latest_price(&db, "PETR4").unwrap().unwrap();
    assert_eq!(latest.close_price, 29.00);

    // Unknown symbol returns None
    let unknown = queries::get_latest_price(&db, "UNKNOWN").unwrap();
    assert!(unknown.is_none());

    let unknown_date = queries::get_latest_price_date(&db, "UNKNOWN").unwrap();
    assert!(unknown_date.is_none());

    // Date range query
    let price2 = DailyPrice {
        symbol: "PETR4".to_string(),
        date: NaiveDate::from_ymd_opt(2025, 3, 16).unwrap(),
        close_price: 30.00,
        currency: "BRL".to_string(),
        brl_rate: 1.0,
    };
    queries::upsert_daily_price(&db, &price2).unwrap();

    let range = queries::get_daily_prices(
        &db,
        "PETR4",
        NaiveDate::from_ymd_opt(2025, 3, 14).unwrap(),
        NaiveDate::from_ymd_opt(2025, 3, 16).unwrap(),
    )
    .unwrap();
    assert_eq!(range.len(), 2);
}

#[test]
fn test_config_get_set() {
    let db = Database::open_in_memory().unwrap();

    // Missing key returns None
    let val = queries::get_config(&db, "last_sync").unwrap();
    assert!(val.is_none());

    // Set and get
    queries::set_config(&db, "last_sync", "2025-03-15").unwrap();
    let val = queries::get_config(&db, "last_sync").unwrap();
    assert_eq!(val, Some("2025-03-15".to_string()));

    // Overwrite
    queries::set_config(&db, "last_sync", "2025-03-16").unwrap();
    let val = queries::get_config(&db, "last_sync").unwrap();
    assert_eq!(val, Some("2025-03-16".to_string()));
}

#[test]
fn test_income_insert_and_query() {
    let db = Database::open_in_memory().unwrap();

    let income = Income {
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
        import_hash: "income_abc".to_string(),
    };

    let inserted = queries::insert_income(&db, &income).unwrap();
    assert!(inserted);

    // Duplicate is ignored
    let dup = queries::insert_income(&db, &income).unwrap();
    assert!(!dup);

    let all = queries::get_all_income(&db).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].symbol, "PETR4");
    assert_eq!(all[0].gross_value, 150.0);
    assert!(all[0].id.is_some());
}

#[test]
fn test_get_distinct_symbols() {
    let db = Database::open_in_memory().unwrap();

    let tx1 = Transaction {
        id: None,
        source: Source::B3,
        asset_type: AssetType::StockBr,
        symbol: "PETR4".to_string(),
        tx_type: TxType::Buy,
        date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        quantity: 100.0,
        unit_price: Some(28.0),
        currency: "BRL".to_string(),
        total_value: 2800.0,
        brl_rate: 1.0,
        total_brl: 2800.0,
        commission: None,
        fee_brl: None,
        notes: None,
        import_hash: "hash1".to_string(),
    };

    let tx2 = Transaction {
        id: None,
        source: Source::Ibkr,
        asset_type: AssetType::StockIntl,
        symbol: "AAPL".to_string(),
        tx_type: TxType::Sell,
        date: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        quantity: 10.0,
        unit_price: Some(175.0),
        currency: "USD".to_string(),
        total_value: 1750.0,
        brl_rate: 5.0,
        total_brl: 8750.0,
        commission: None,
        fee_brl: None,
        notes: None,
        import_hash: "hash2".to_string(),
    };

    // A dividend transaction should NOT appear in distinct symbols
    let tx3 = Transaction {
        id: None,
        source: Source::B3,
        asset_type: AssetType::StockBr,
        symbol: "VALE3".to_string(),
        tx_type: TxType::Dividend,
        date: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
        quantity: 0.0,
        unit_price: None,
        currency: "BRL".to_string(),
        total_value: 50.0,
        brl_rate: 1.0,
        total_brl: 50.0,
        commission: None,
        fee_brl: None,
        notes: None,
        import_hash: "hash3".to_string(),
    };

    queries::insert_transaction(&db, &tx1).unwrap();
    queries::insert_transaction(&db, &tx2).unwrap();
    queries::insert_transaction(&db, &tx3).unwrap();

    let symbols = queries::get_distinct_symbols(&db).unwrap();
    assert_eq!(symbols.len(), 2);

    // Ordered by symbol
    assert_eq!(symbols[0].0, "AAPL");
    assert_eq!(symbols[0].1, "stock_intl");
    assert_eq!(symbols[0].2, "USD");

    assert_eq!(symbols[1].0, "PETR4");
    assert_eq!(symbols[1].1, "stock_br");
    assert_eq!(symbols[1].2, "BRL");
}

fn make_legacy_sell(symbol: &str, notes: Option<&str>, hash: &str) -> Transaction {
    Transaction {
        id: None,
        source: Source::B3,
        asset_type: AssetType::StockBr,
        symbol: symbol.to_string(),
        tx_type: TxType::Sell,
        date: NaiveDate::from_ymd_opt(2024, 4, 17).unwrap(),
        quantity: 7.0,
        unit_price: Some(56.66),
        currency: "BRL".to_string(),
        total_value: 396.62,
        brl_rate: 1.0,
        total_brl: 396.62,
        commission: None,
        fee_brl: None,
        notes: notes.map(|s| s.to_string()),
        import_hash: hash.to_string(),
    }
}

/// The legacy B3 parser wrote every "Transferência - Liquidação" row as
/// `tx_type='sell'` regardless of `Entrada/Saída`, hiding the actual buy side
/// of every Nu Invest purchase. The cleanup migration deletes those rows so
/// a re-import (with the direction-aware parser) can re-create them correctly.
#[test]
fn test_migration_purges_misclassified_transferencia_liquidacao() {
    let path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
    {
        let db = Database::open(&path).unwrap();
        queries::insert_transaction(
            &db,
            &make_legacy_sell("BBAS3", Some("Transferência - Liquidação"), "h1"),
        )
        .unwrap();
        queries::insert_transaction(
            &db,
            &make_legacy_sell("PETR4", Some("Transferência - Liquidação"), "h2"),
        )
        .unwrap();
        // Genuine Sell with a different note — must be left alone.
        queries::insert_transaction(
            &db,
            &make_legacy_sell("VALE3", Some("Venda em pregão"), "h3"),
        )
        .unwrap();
        // Sell with no note — also left alone.
        queries::insert_transaction(&db, &make_legacy_sell("ITUB4", None, "h4")).unwrap();
    }

    // Re-open: run_migrations() runs again. The misclassified rows must be
    // gone; the second invocation also exercises idempotency.
    let db = Database::open(&path).unwrap();
    let symbols: Vec<String> = queries::get_all_transactions(&db)
        .unwrap()
        .into_iter()
        .map(|t| t.symbol)
        .collect();

    assert!(
        !symbols.contains(&"BBAS3".to_string()),
        "misclassified BBAS3 row must be deleted, got {:?}",
        symbols
    );
    assert!(
        !symbols.contains(&"PETR4".to_string()),
        "misclassified PETR4 row must be deleted, got {:?}",
        symbols
    );
    assert!(
        symbols.contains(&"VALE3".to_string()),
        "real Venda must be kept"
    );
    assert!(
        symbols.contains(&"ITUB4".to_string()),
        "Sell without 'Transferência' note must be kept"
    );
}

/// Regression: the new parser writes Debito Liquidação rows as `tx_type='sell'`,
/// so its `notes` must NOT exactly equal "Transferência - Liquidação" — otherwise
/// the cleanup migration would delete freshly-imported rows on every restart.
#[test]
fn test_migration_does_not_delete_correctly_classified_sells() {
    let path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
    {
        let db = Database::open(&path).unwrap();
        // Mirrors what the new parser writes for a Liquidação Debito.
        queries::insert_transaction(&db, &make_legacy_sell("BBAS3", Some("Liquidação"), "h1"))
            .unwrap();
    }

    let db = Database::open(&path).unwrap();
    let symbols: Vec<String> = queries::get_all_transactions(&db)
        .unwrap()
        .into_iter()
        .map(|t| t.symbol)
        .collect();
    assert!(
        symbols.contains(&"BBAS3".to_string()),
        "new-parser sell with notes='Liquidação' must survive the migration"
    );
}
