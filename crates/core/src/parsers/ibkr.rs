use chrono::NaiveDate;
use sha2::{Digest, Sha256};

use crate::{AssetType, Income, IncomeType, Source, Transaction, TxType};

/// Classify an IBKR asset. All IBKR assets are international from a Brazilian perspective.
pub fn classify_ibkr_asset(_symbol: &str, _currency: &str) -> AssetType {
    AssetType::StockIntl
}

/// Parse an IBKR dividend description.
///
/// Input example: "RILY(US05580M1080) Cash Dividend USD 0.50 per Share (Ordinary Dividend)"
/// Returns (symbol, per_share_amount_str) e.g. ("RILY", "0.50")
pub fn parse_dividend_description(desc: &str) -> (String, String) {
    let symbol = desc
        .find('(')
        .map(|i| desc[..i].trim().to_string())
        .unwrap_or_default();

    let per_share = extract_per_share(desc);

    (symbol, per_share)
}

/// Parse an IBKR withholding tax description.
///
/// Input example: "NVO(US6701002056) Cash Dividend USD 0.516901 per Share - DK Tax"
/// Returns (symbol, tax_origin) e.g. ("NVO", "DK")
pub fn parse_tax_description(desc: &str) -> (String, String) {
    let symbol = desc
        .find('(')
        .map(|i| desc[..i].trim().to_string())
        .unwrap_or_default();

    // Tax origin is the two-letter code before " Tax" at the end.
    let tax_origin = desc
        .rfind(" Tax")
        .and_then(|tax_pos| {
            let before = desc[..tax_pos].trim();
            before
                .rfind(|c: char| c == ' ' || c == '-')
                .map(|sep| before[sep + 1..].trim().to_string())
        })
        .unwrap_or_default();

    (symbol, tax_origin)
}

/// Convert trade data into a Transaction.
///
/// - quantity > 0 means Buy, quantity < 0 means Sell.
/// - Uses absolute values for quantity and total_value.
/// - Hash: SHA256("ibkr:trade:{symbol}:{date}:{quantity}:{trade_price}")
pub fn trade_to_transaction(
    symbol: &str,
    currency: &str,
    date: NaiveDate,
    quantity: f64,
    trade_price: f64,
    proceeds: f64,
    commission: f64,
    brl_rate: f64,
) -> Transaction {
    let tx_type = if quantity > 0.0 {
        TxType::Buy
    } else {
        TxType::Sell
    };

    let abs_quantity = quantity.abs();
    let abs_total = proceeds.abs();
    let abs_commission = commission.abs();

    let hash_input = format!(
        "ibkr:trade:{}:{}:{}:{}",
        symbol, date, quantity, trade_price
    );
    let import_hash = sha256_hex(&hash_input);

    Transaction {
        id: None,
        source: Source::Ibkr,
        asset_type: classify_ibkr_asset(symbol, currency),
        symbol: symbol.to_string(),
        tx_type,
        date,
        quantity: abs_quantity,
        unit_price: Some(trade_price),
        currency: currency.to_string(),
        total_value: abs_total,
        brl_rate,
        total_brl: abs_total * brl_rate,
        commission: if abs_commission > 0.0 {
            Some(abs_commission)
        } else {
            None
        },
        fee_brl: None,
        notes: None,
        import_hash,
    }
}

/// Convert dividend data into an Income record.
///
/// - net = gross - abs(tax_withheld)
/// - Hash: SHA256("ibkr:div:{symbol}:{date}:{gross_amount}")
pub fn dividend_to_income(
    symbol: &str,
    currency: &str,
    date: NaiveDate,
    gross_amount: f64,
    tax_withheld: f64,
    tax_origin: &str,
    brl_rate: f64,
) -> Income {
    let abs_tax = tax_withheld.abs();
    let net = gross_amount - abs_tax;

    let hash_input = format!("ibkr:div:{}:{}:{}", symbol, date, gross_amount);
    let import_hash = sha256_hex(&hash_input);

    Income {
        id: None,
        source: Source::Ibkr,
        symbol: symbol.to_string(),
        date,
        income_type: IncomeType::Dividend,
        currency: currency.to_string(),
        gross_value: gross_amount,
        tax_withheld: if abs_tax > 0.0 { Some(abs_tax) } else { None },
        tax_origin: if tax_origin.is_empty() {
            None
        } else {
            Some(tax_origin.to_string())
        },
        brl_rate,
        net_value_brl: net * brl_rate,
        import_hash,
    }
}

/// Extract "X.XX" from "... USD X.XX per Share ..."
fn extract_per_share(desc: &str) -> String {
    let per_pos = match desc.find("per Share") {
        Some(p) => p,
        None => return String::new(),
    };

    let before = desc[..per_pos].trim();
    before
        .rsplit_once(' ')
        .map(|(_, amount)| amount.to_string())
        .unwrap_or_default()
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
