use std::collections::HashMap;

use rusqlite::Result;

use crate::db::queries;
use crate::db::Database;
use crate::types::*;

/// Intermediate struct to accumulate per-symbol position state while iterating transactions.
struct PositionAccum {
    asset_type: AssetType,
    currency: String,
    net_qty: f64,
    total_cost_orig: f64,
    total_cost_brl: f64,
}

/// Compute current positions from all transactions in the DB.
///
/// Groups transactions by symbol, calculates net quantity, average cost, and current value
/// using the latest available price from the daily_prices table.
pub fn compute_positions(db: &Database) -> Result<Vec<Position>> {
    let transactions = queries::get_all_transactions(db)?;

    // Group by (symbol, currency) to handle same symbol traded in different currencies.
    let mut accum: HashMap<(String, String), PositionAccum> = HashMap::new();

    for tx in &transactions {
        let key = (tx.symbol.clone(), tx.currency.clone());
        let entry = accum.entry(key).or_insert_with(|| PositionAccum {
            asset_type: tx.asset_type.clone(),
            currency: tx.currency.clone(),
            net_qty: 0.0,
            total_cost_orig: 0.0,
            total_cost_brl: 0.0,
        });

        match tx.tx_type {
            TxType::Buy => {
                entry.net_qty += tx.quantity;
                // Corporate actions (splits, CUSIP changes) have total_value=0.
                // They adjust quantity but cost basis transfers from the old shares.
                if tx.total_value > 0.001 {
                    entry.total_cost_orig += tx.total_value;
                    entry.total_cost_brl += tx.total_brl;
                }
            }
            TxType::Sell | TxType::FractionAuction => {
                if entry.net_qty > 0.0 {
                    // Only reduce cost basis for real sells, not $0 corporate actions
                    if tx.total_value > 0.001 {
                        let fraction_sold = tx.quantity / entry.net_qty;
                        entry.total_cost_orig -= entry.total_cost_orig * fraction_sold;
                        entry.total_cost_brl -= entry.total_cost_brl * fraction_sold;
                    }
                    entry.net_qty -= tx.quantity;
                }
            }
            TxType::Send => {
                // Crypto transfers: do nothing to cost basis or quantity
            }
            TxType::Dividend | TxType::Jcp | TxType::Deposit => {
                // Not position-changing
            }
        }
    }

    let mut positions: Vec<Position> = Vec::new();

    for ((symbol, _currency), acc) in &accum {
        // Skip fully closed positions
        if acc.net_qty < 0.00001 {
            continue; // closed position or sells-only (no matching buys)
        }

        let avg_cost = acc.total_cost_orig / acc.net_qty;
        let avg_cost_brl = acc.total_cost_brl / acc.net_qty;

        let latest = queries::get_latest_price(db, symbol)?;

        let (current_price, current_brl_rate, current_value_brl, pnl_brl, pnl_pct) =
            match latest {
                Some(price) => {
                    let value_brl = acc.net_qty * price.close_price * price.brl_rate;
                    let pnl = value_brl - acc.total_cost_brl;
                    let pct = if acc.total_cost_brl.abs() > 0.0001 {
                        pnl / acc.total_cost_brl * 100.0
                    } else {
                        0.0
                    };
                    (
                        Some(price.close_price),
                        Some(price.brl_rate),
                        Some(value_brl),
                        Some(pnl),
                        Some(pct),
                    )
                }
                None => (None, None, None, None, None),
            };

        positions.push(Position {
            symbol: symbol.clone(),
            asset_type: acc.asset_type.clone(),
            quantity: acc.net_qty,
            avg_cost,
            avg_cost_brl,
            currency: acc.currency.clone(),
            current_price,
            current_brl_rate,
            current_value_brl,
            pnl_brl,
            pnl_pct,
            weight: None, // filled in below
        });
    }

    // Compute weights: value / total_value * 100
    let total_value: f64 = positions
        .iter()
        .filter_map(|p| p.current_value_brl)
        .sum();

    if total_value > 0.0 {
        for pos in &mut positions {
            if let Some(val) = pos.current_value_brl {
                pos.weight = Some(val / total_value * 100.0);
            }
        }
    }

    // Sort by current_value_brl descending (positions without price go last)
    positions.sort_by(|a, b| {
        let va = a.current_value_brl.unwrap_or(0.0);
        let vb = b.current_value_brl.unwrap_or(0.0);
        vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(positions)
}

/// Compute allocation breakdown by asset type from a list of positions.
///
/// Groups positions by asset_type, sums current_value_brl per group, and computes
/// each group's weight as a percentage of total portfolio value.
pub fn compute_allocations(positions: &[Position]) -> Vec<Allocation> {
    let mut by_type: HashMap<String, f64> = HashMap::new();

    for pos in positions {
        let val = pos.current_value_brl.unwrap_or(0.0);
        *by_type.entry(pos.asset_type.as_str().to_string()).or_insert(0.0) += val;
    }

    let total: f64 = by_type.values().sum();

    let mut allocations: Vec<Allocation> = by_type
        .into_iter()
        .map(|(type_str, value)| {
            let weight = if total > 0.0 {
                value / total * 100.0
            } else {
                0.0
            };
            Allocation {
                asset_type: AssetType::from_str(&type_str).unwrap_or(AssetType::StockBr),
                value_brl: value,
                weight,
            }
        })
        .collect();

    // Sort by value descending
    allocations.sort_by(|a, b| {
        b.value_brl
            .partial_cmp(&a.value_brl)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    allocations
}
