use chrono::NaiveDate;
use investimentos_core::db::queries;
use investimentos_core::db::Database;
use investimentos_core::portfolio;
use investimentos_core::*;

fn make_tx(
    symbol: &str,
    asset_type: AssetType,
    tx_type: TxType,
    date: NaiveDate,
    quantity: f64,
    unit_price: f64,
    currency: &str,
    brl_rate: f64,
    hash: &str,
) -> Transaction {
    let total_value = quantity * unit_price;
    Transaction {
        id: None,
        source: Source::Manual,
        asset_type,
        symbol: symbol.to_string(),
        tx_type,
        date,
        quantity,
        unit_price: Some(unit_price),
        currency: currency.to_string(),
        total_value,
        brl_rate,
        total_brl: total_value * brl_rate,
        commission: None,
        fee_brl: None,
        notes: None,
        import_hash: hash.to_string(),
    }
}

#[test]
fn test_compute_positions() {
    let db = Database::open_in_memory().unwrap();

    // Buy 10 LUNR at $20, PTAX 5.0
    queries::insert_transaction(
        &db,
        &make_tx(
            "LUNR",
            AssetType::StockIntl,
            TxType::Buy,
            NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
            10.0,
            20.0,
            "USD",
            5.0,
            "lunr_buy1",
        ),
    )
    .unwrap();

    // Buy 5 more LUNR at $30, PTAX 5.5
    queries::insert_transaction(
        &db,
        &make_tx(
            "LUNR",
            AssetType::StockIntl,
            TxType::Buy,
            NaiveDate::from_ymd_opt(2025, 2, 15).unwrap(),
            5.0,
            30.0,
            "USD",
            5.5,
            "lunr_buy2",
        ),
    )
    .unwrap();

    // Sell 3 LUNR at $25, PTAX 5.2
    queries::insert_transaction(
        &db,
        &make_tx(
            "LUNR",
            AssetType::StockIntl,
            TxType::Sell,
            NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            3.0,
            25.0,
            "USD",
            5.2,
            "lunr_sell1",
        ),
    )
    .unwrap();

    // Buy 100 PETR4 at R$35 (BRL, rate 1.0)
    queries::insert_transaction(
        &db,
        &make_tx(
            "PETR4",
            AssetType::StockBr,
            TxType::Buy,
            NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(),
            100.0,
            35.0,
            "BRL",
            1.0,
            "petr4_buy1",
        ),
    )
    .unwrap();

    // Current price LUNR $40 @ PTAX 6.0
    queries::upsert_daily_price(
        &db,
        &DailyPrice {
            symbol: "LUNR".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            close_price: 40.0,
            currency: "USD".to_string(),
            brl_rate: 6.0,
        },
    )
    .unwrap();

    // Current price PETR4 R$38
    queries::upsert_daily_price(
        &db,
        &DailyPrice {
            symbol: "PETR4".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            close_price: 38.0,
            currency: "BRL".to_string(),
            brl_rate: 1.0,
        },
    )
    .unwrap();

    let positions = portfolio::compute_positions(&db).unwrap();
    assert_eq!(positions.len(), 2);

    // LUNR: started with 15 shares, sold 3 => net 12
    let lunr = positions.iter().find(|p| p.symbol == "LUNR").unwrap();
    assert_eq!(lunr.quantity, 12.0);
    assert!((lunr.current_value_brl.unwrap() - 2880.0).abs() < 0.01); // 12 * 40 * 6

    // PETR4: 100 shares, value = 100 * 38 * 1.0 = 3800 BRL
    let petr4 = positions.iter().find(|p| p.symbol == "PETR4").unwrap();
    assert_eq!(petr4.quantity, 100.0);
    assert!((petr4.current_value_brl.unwrap() - 3800.0).abs() < 0.01);

    // Weights should sum to ~100%
    let weight_sum: f64 = positions.iter().filter_map(|p| p.weight).sum();
    assert!((weight_sum - 100.0).abs() < 0.01);

    // PETR4 (3800) > LUNR (2880), so PETR4 should be first (sorted desc)
    assert_eq!(positions[0].symbol, "PETR4");
    assert_eq!(positions[1].symbol, "LUNR");
}

#[test]
fn test_compute_positions_avg_cost() {
    let db = Database::open_in_memory().unwrap();

    // Buy 10 LUNR at $20, PTAX 5.0 => cost_orig = 200, cost_brl = 1000
    queries::insert_transaction(
        &db,
        &make_tx(
            "LUNR",
            AssetType::StockIntl,
            TxType::Buy,
            NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
            10.0,
            20.0,
            "USD",
            5.0,
            "lunr_buy1",
        ),
    )
    .unwrap();

    // Buy 5 more LUNR at $30, PTAX 5.5 => cost_orig += 150 = 350, cost_brl += 825 = 1825
    queries::insert_transaction(
        &db,
        &make_tx(
            "LUNR",
            AssetType::StockIntl,
            TxType::Buy,
            NaiveDate::from_ymd_opt(2025, 2, 15).unwrap(),
            5.0,
            30.0,
            "USD",
            5.5,
            "lunr_buy2",
        ),
    )
    .unwrap();

    // Sell 3 LUNR => fraction_sold = 3/15 = 0.2
    // cost_orig = 350 - 350*0.2 = 280, cost_brl = 1825 - 1825*0.2 = 1460
    // net_qty = 12
    // avg_cost = 280/12 ~ 23.333, avg_cost_brl = 1460/12 ~ 121.667
    queries::insert_transaction(
        &db,
        &make_tx(
            "LUNR",
            AssetType::StockIntl,
            TxType::Sell,
            NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            3.0,
            25.0,
            "USD",
            5.2,
            "lunr_sell1",
        ),
    )
    .unwrap();

    let positions = portfolio::compute_positions(&db).unwrap();
    let lunr = positions.iter().find(|p| p.symbol == "LUNR").unwrap();

    assert!((lunr.avg_cost - 23.3333).abs() < 0.01);
    assert!((lunr.avg_cost_brl - 121.6667).abs() < 0.01);
}

#[test]
fn test_compute_positions_pnl() {
    let db = Database::open_in_memory().unwrap();

    // Buy 100 PETR4 at R$35 => total_cost_brl = 3500
    queries::insert_transaction(
        &db,
        &make_tx(
            "PETR4",
            AssetType::StockBr,
            TxType::Buy,
            NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(),
            100.0,
            35.0,
            "BRL",
            1.0,
            "petr4_buy1",
        ),
    )
    .unwrap();

    // Current price R$38 => value = 3800, pnl = 300, pct = 300/3500*100 ~ 8.571
    queries::upsert_daily_price(
        &db,
        &DailyPrice {
            symbol: "PETR4".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            close_price: 38.0,
            currency: "BRL".to_string(),
            brl_rate: 1.0,
        },
    )
    .unwrap();

    let positions = portfolio::compute_positions(&db).unwrap();
    let petr4 = positions.iter().find(|p| p.symbol == "PETR4").unwrap();

    assert!((petr4.pnl_brl.unwrap() - 300.0).abs() < 0.01);
    assert!((petr4.pnl_pct.unwrap() - 8.5714).abs() < 0.01);
}

#[test]
fn test_closed_position_excluded() {
    let db = Database::open_in_memory().unwrap();

    // Buy 10 AAPL
    queries::insert_transaction(
        &db,
        &make_tx(
            "AAPL",
            AssetType::StockIntl,
            TxType::Buy,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            10.0,
            150.0,
            "USD",
            5.0,
            "aapl_buy",
        ),
    )
    .unwrap();

    // Sell 10 AAPL (fully close)
    queries::insert_transaction(
        &db,
        &make_tx(
            "AAPL",
            AssetType::StockIntl,
            TxType::Sell,
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            10.0,
            170.0,
            "USD",
            5.2,
            "aapl_sell",
        ),
    )
    .unwrap();

    let positions = portfolio::compute_positions(&db).unwrap();
    assert!(
        positions.iter().find(|p| p.symbol == "AAPL").is_none(),
        "Fully closed position should not appear"
    );
}

#[test]
fn test_compute_allocations() {
    let positions = vec![
        Position {
            symbol: "PETR4".to_string(),
            asset_type: AssetType::StockBr,
            quantity: 100.0,
            avg_cost: 35.0,
            avg_cost_brl: 35.0,
            currency: "BRL".to_string(),
            current_price: Some(38.0),
            current_brl_rate: Some(1.0),
            current_value_brl: Some(3800.0),
            pnl_brl: Some(300.0),
            pnl_pct: Some(8.57),
            weight: Some(50.0),
        },
        Position {
            symbol: "LUNR".to_string(),
            asset_type: AssetType::StockIntl,
            quantity: 12.0,
            avg_cost: 23.33,
            avg_cost_brl: 121.67,
            currency: "USD".to_string(),
            current_price: Some(40.0),
            current_brl_rate: Some(6.0),
            current_value_brl: Some(2880.0),
            pnl_brl: Some(1420.0),
            pnl_pct: Some(97.26),
            weight: Some(30.0),
        },
        Position {
            symbol: "BTC".to_string(),
            asset_type: AssetType::Crypto,
            quantity: 0.1,
            avg_cost: 30000.0,
            avg_cost_brl: 150000.0,
            currency: "USD".to_string(),
            current_price: Some(60000.0),
            current_brl_rate: Some(5.5),
            current_value_brl: Some(33000.0),
            pnl_brl: Some(18000.0),
            pnl_pct: Some(120.0),
            weight: Some(20.0),
        },
    ];

    let allocations = portfolio::compute_allocations(&positions);

    // 3 asset types
    assert_eq!(allocations.len(), 3);

    let total_value = 3800.0 + 2880.0 + 33000.0; // 39680

    let crypto = allocations
        .iter()
        .find(|a| a.asset_type == AssetType::Crypto)
        .unwrap();
    assert!((crypto.value_brl - 33000.0).abs() < 0.01);
    assert!((crypto.weight - (33000.0 / total_value * 100.0)).abs() < 0.01);

    let br = allocations
        .iter()
        .find(|a| a.asset_type == AssetType::StockBr)
        .unwrap();
    assert!((br.value_brl - 3800.0).abs() < 0.01);

    let intl = allocations
        .iter()
        .find(|a| a.asset_type == AssetType::StockIntl)
        .unwrap();
    assert!((intl.value_brl - 2880.0).abs() < 0.01);

    // Weights should sum to ~100%
    let weight_sum: f64 = allocations.iter().map(|a| a.weight).sum();
    assert!((weight_sum - 100.0).abs() < 0.01);

    // Sorted by value descending: crypto (33000) > stock_br (3800) > stock_intl (2880)
    assert_eq!(allocations[0].asset_type, AssetType::Crypto);
    assert_eq!(allocations[1].asset_type, AssetType::StockBr);
    assert_eq!(allocations[2].asset_type, AssetType::StockIntl);
}

#[test]
fn test_non_position_tx_types_ignored() {
    let db = Database::open_in_memory().unwrap();

    // Buy 10 PETR4
    queries::insert_transaction(
        &db,
        &make_tx(
            "PETR4",
            AssetType::StockBr,
            TxType::Buy,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            10.0,
            30.0,
            "BRL",
            1.0,
            "petr4_buy",
        ),
    )
    .unwrap();

    // Dividend on PETR4 (should not change position)
    let div = Transaction {
        id: None,
        source: Source::B3,
        asset_type: AssetType::StockBr,
        symbol: "PETR4".to_string(),
        tx_type: TxType::Dividend,
        date: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
        quantity: 0.0,
        unit_price: None,
        currency: "BRL".to_string(),
        total_value: 15.0,
        brl_rate: 1.0,
        total_brl: 15.0,
        commission: None,
        fee_brl: None,
        notes: None,
        import_hash: "petr4_div".to_string(),
    };
    queries::insert_transaction(&db, &div).unwrap();

    let positions = portfolio::compute_positions(&db).unwrap();
    let petr4 = positions.iter().find(|p| p.symbol == "PETR4").unwrap();

    // Quantity unchanged at 10
    assert_eq!(petr4.quantity, 10.0);
    // Cost unchanged at 300 / 10 = 30
    assert!((petr4.avg_cost - 30.0).abs() < 0.01);
}

#[test]
fn test_position_without_price() {
    let db = Database::open_in_memory().unwrap();

    // Buy 50 shares with no price data in daily_prices
    queries::insert_transaction(
        &db,
        &make_tx(
            "VALE3",
            AssetType::StockBr,
            TxType::Buy,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            50.0,
            60.0,
            "BRL",
            1.0,
            "vale3_buy",
        ),
    )
    .unwrap();

    let positions = portfolio::compute_positions(&db).unwrap();
    let vale3 = positions.iter().find(|p| p.symbol == "VALE3").unwrap();

    assert_eq!(vale3.quantity, 50.0);
    assert!(vale3.current_price.is_none());
    assert!(vale3.current_value_brl.is_none());
    assert!(vale3.pnl_brl.is_none());
    assert!(vale3.weight.is_none());
}

#[test]
fn test_fraction_auction_adds_to_position() {
    let db = Database::open_in_memory().unwrap();

    // Buy 10 PETR4
    queries::insert_transaction(
        &db,
        &make_tx(
            "PETR4",
            AssetType::StockBr,
            TxType::Buy,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            10.0,
            30.0,
            "BRL",
            1.0,
            "petr4_buy",
        ),
    )
    .unwrap();

    // FractionAuction adds 0.5
    queries::insert_transaction(
        &db,
        &make_tx(
            "PETR4",
            AssetType::StockBr,
            TxType::FractionAuction,
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            0.5,
            32.0,
            "BRL",
            1.0,
            "petr4_frac",
        ),
    )
    .unwrap();

    let positions = portfolio::compute_positions(&db).unwrap();
    let petr4 = positions.iter().find(|p| p.symbol == "PETR4").unwrap();

    assert!((petr4.quantity - 10.5).abs() < 0.001);
    // total_cost = 300 + 16 = 316, avg = 316/10.5 ~ 30.095
    assert!((petr4.avg_cost - 316.0 / 10.5).abs() < 0.01);
}

#[test]
fn test_send_does_not_affect_position() {
    let db = Database::open_in_memory().unwrap();

    // Buy 1 BTC
    queries::insert_transaction(
        &db,
        &make_tx(
            "BTC",
            AssetType::Crypto,
            TxType::Buy,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            1.0,
            50000.0,
            "USD",
            5.0,
            "btc_buy",
        ),
    )
    .unwrap();

    // Send 0.5 BTC to another wallet
    let send_tx = Transaction {
        id: None,
        source: Source::Binance,
        asset_type: AssetType::Crypto,
        symbol: "BTC".to_string(),
        tx_type: TxType::Send,
        date: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        quantity: 0.5,
        unit_price: Some(55000.0),
        currency: "USD".to_string(),
        total_value: 27500.0,
        brl_rate: 5.0,
        total_brl: 137500.0,
        commission: None,
        fee_brl: None,
        notes: None,
        import_hash: "btc_send".to_string(),
    };
    queries::insert_transaction(&db, &send_tx).unwrap();

    let positions = portfolio::compute_positions(&db).unwrap();
    let btc = positions.iter().find(|p| p.symbol == "BTC").unwrap();

    // Quantity should still be 1.0 (send doesn't reduce position)
    assert!((btc.quantity - 1.0).abs() < 0.001);
    // Cost should still be 50000 / 1 = 50000
    assert!((btc.avg_cost - 50000.0).abs() < 0.01);
}

#[test]
fn test_empty_db_returns_empty() {
    let db = Database::open_in_memory().unwrap();

    let positions = portfolio::compute_positions(&db).unwrap();
    assert!(positions.is_empty());

    let allocations = portfolio::compute_allocations(&positions);
    assert!(allocations.is_empty());
}
