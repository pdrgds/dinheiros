use chrono::NaiveDate;
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::{AssetType, Source, Transaction, TxType};

#[derive(Debug)]
pub struct BinanceImportResult {
    pub transactions: Vec<Transaction>,
    pub net_btc: f64,
    pub total_fees_btc: f64,
}

pub fn parse_binance_csv(path: &Path) -> Result<BinanceImportResult, Box<dyn std::error::Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;

    let mut transactions = Vec::new();
    let mut total_bought_btc = 0.0;
    let mut total_fees_btc = 0.0;

    for result in reader.records() {
        let record = result?;

        let id = record.get(0).unwrap_or("").trim();
        let datetime = record.get(1).unwrap_or("").trim();
        let tx_type_str = record.get(2).unwrap_or("").trim();

        // Parse date: take first 10 chars as YYYY-MM-DD
        let date_str = &datetime[..10];
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;

        let import_hash = make_import_hash(tx_type_str, id, datetime);

        match tx_type_str {
            "Buy" => {
                let sent_amount: f64 = record.get(6).unwrap_or("0").trim().parse().unwrap_or(0.0);
                let received_amount: f64 =
                    record.get(10).unwrap_or("0").trim().parse().unwrap_or(0.0);
                let fee_amount: f64 = record.get(14).unwrap_or("0").trim().parse().unwrap_or(0.0);
                let fee_currency = record.get(15).unwrap_or("").trim();

                let unit_price = if received_amount > 0.0 {
                    sent_amount / received_amount
                } else {
                    0.0
                };

                let fee_btc = if is_btc_currency(fee_currency) {
                    fee_amount
                } else {
                    0.0
                };

                total_bought_btc += received_amount;
                total_fees_btc += fee_btc;

                transactions.push(Transaction {
                    id: None,
                    source: Source::Binance,
                    asset_type: AssetType::Crypto,
                    symbol: "BTC".to_string(),
                    tx_type: TxType::Buy,
                    date,
                    quantity: received_amount,
                    unit_price: Some(unit_price),
                    currency: "BRL".to_string(),
                    total_value: sent_amount,
                    brl_rate: 1.0,
                    total_brl: sent_amount,
                    commission: if fee_amount > 0.0 {
                        Some(fee_amount)
                    } else {
                        None
                    },
                    fee_brl: None,
                    notes: None,
                    import_hash,
                });
            }
            "Send" => {
                let sent_amount: f64 = record.get(6).unwrap_or("0").trim().parse().unwrap_or(0.0);
                let sent_address = record.get(9).unwrap_or("").trim();
                let fee_amount: f64 = record.get(14).unwrap_or("0").trim().parse().unwrap_or(0.0);
                let fee_currency = record.get(15).unwrap_or("").trim();

                let fee_btc = if is_btc_currency(fee_currency) {
                    fee_amount
                } else {
                    0.0
                };

                total_fees_btc += fee_btc;

                transactions.push(Transaction {
                    id: None,
                    source: Source::Binance,
                    asset_type: AssetType::Crypto,
                    symbol: "BTC".to_string(),
                    tx_type: TxType::Send,
                    date,
                    quantity: sent_amount,
                    unit_price: None,
                    currency: "BTC".to_string(),
                    total_value: sent_amount,
                    brl_rate: 1.0,
                    total_brl: 0.0,
                    commission: if fee_amount > 0.0 {
                        Some(fee_amount)
                    } else {
                        None
                    },
                    fee_brl: None,
                    notes: Some(format!("sent_address: {}", sent_address)),
                    import_hash,
                });
            }
            "Deposit" => {
                // Skip deposits entirely
            }
            other => {
                eprintln!("Unknown Binance transaction type: {}", other);
            }
        }
    }

    let net_btc = total_bought_btc - total_fees_btc;

    Ok(BinanceImportResult {
        transactions,
        net_btc,
        total_fees_btc,
    })
}

fn is_btc_currency(currency: &str) -> bool {
    matches!(currency, "BTC" | "Bitcoin")
}

fn make_import_hash(tx_type: &str, id: &str, datetime: &str) -> String {
    let input = format!("binance:{}:{}:{}", tx_type, id, datetime);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
