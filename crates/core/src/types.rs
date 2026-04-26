use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Source {
    B3,
    Ibkr,
    Binance,
    Manual,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::B3 => "b3",
            Source::Ibkr => "ibkr",
            Source::Binance => "binance",
            Source::Manual => "manual",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "b3" => Some(Source::B3),
            "ibkr" => Some(Source::Ibkr),
            "binance" => Some(Source::Binance),
            "manual" => Some(Source::Manual),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssetType {
    StockBr,
    StockIntl,
    Tesouro,
    Crypto,
    Gold,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::StockBr => "stock_br",
            AssetType::StockIntl => "stock_intl",
            AssetType::Tesouro => "tesouro",
            AssetType::Crypto => "crypto",
            AssetType::Gold => "gold",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "stock_br" => Some(AssetType::StockBr),
            "stock_intl" => Some(AssetType::StockIntl),
            "tesouro" => Some(AssetType::Tesouro),
            "crypto" => Some(AssetType::Crypto),
            "gold" => Some(AssetType::Gold),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TxType {
    Buy,
    Sell,
    Dividend,
    Jcp,
    FractionAuction,
    Send,
    Deposit,
    /// Custody transfer out of one of the user's brokers. Reduces position
    /// quantity at the prevailing avg cost; does NOT realize a gain.
    TransferOut,
    /// Custody transfer into one of the user's brokers. Adds position
    /// quantity at whatever cost the source recorded (may be zero when the
    /// source export has no price column, e.g. plain B3 "Transferência").
    TransferIn,
}

impl TxType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TxType::Buy => "buy",
            TxType::Sell => "sell",
            TxType::Dividend => "dividend",
            TxType::Jcp => "jcp",
            TxType::FractionAuction => "fraction_auction",
            TxType::Send => "send",
            TxType::Deposit => "deposit",
            TxType::TransferOut => "transfer_out",
            TxType::TransferIn => "transfer_in",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "buy" => Some(TxType::Buy),
            "sell" => Some(TxType::Sell),
            "dividend" => Some(TxType::Dividend),
            "jcp" => Some(TxType::Jcp),
            "fraction_auction" => Some(TxType::FractionAuction),
            "send" => Some(TxType::Send),
            "deposit" => Some(TxType::Deposit),
            "transfer_out" => Some(TxType::TransferOut),
            "transfer_in" => Some(TxType::TransferIn),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IncomeType {
    Dividend,
    Jcp,
}

impl IncomeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IncomeType::Dividend => "dividend",
            IncomeType::Jcp => "jcp",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "dividend" => Some(IncomeType::Dividend),
            "jcp" => Some(IncomeType::Jcp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Option<i64>,
    pub source: Source,
    pub asset_type: AssetType,
    pub symbol: String,
    pub tx_type: TxType,
    pub date: NaiveDate,
    pub quantity: f64,
    pub unit_price: Option<f64>,
    pub currency: String,
    pub total_value: f64,
    pub brl_rate: f64,
    pub total_brl: f64,
    pub commission: Option<f64>,
    pub fee_brl: Option<f64>,
    pub notes: Option<String>,
    pub import_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyPrice {
    pub symbol: String,
    pub date: NaiveDate,
    pub close_price: f64,
    pub currency: String,
    pub brl_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Income {
    pub id: Option<i64>,
    pub source: Source,
    pub symbol: String,
    pub date: NaiveDate,
    pub income_type: IncomeType,
    pub currency: String,
    pub gross_value: f64,
    pub tax_withheld: Option<f64>,
    pub tax_origin: Option<String>,
    pub brl_rate: f64,
    pub net_value_brl: f64,
    pub import_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub asset_type: AssetType,
    pub quantity: f64,
    pub avg_cost: f64,
    pub avg_cost_brl: f64,
    pub currency: String,
    pub current_price: Option<f64>,
    pub current_brl_rate: Option<f64>,
    pub current_value_brl: Option<f64>,
    pub pnl_brl: Option<f64>,
    pub pnl_pct: Option<f64>,
    pub weight: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Allocation {
    pub asset_type: AssetType,
    pub value_brl: f64,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub version: u32,
    pub exported_at: String,
    pub transactions: Vec<Transaction>,
    pub daily_prices: Vec<DailyPrice>,
    pub income: Vec<Income>,
    pub config: std::collections::HashMap<String, String>,
}
