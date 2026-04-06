use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Source {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "b3" => Ok(Source::B3),
            "ibkr" => Ok(Source::Ibkr),
            "binance" => Ok(Source::Binance),
            "manual" => Ok(Source::Manual),
            _ => Err(format!("unknown source: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
}

impl fmt::Display for AssetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AssetType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stock_br" => Ok(AssetType::StockBr),
            "stock_intl" => Ok(AssetType::StockIntl),
            "tesouro" => Ok(AssetType::Tesouro),
            "crypto" => Ok(AssetType::Crypto),
            "gold" => Ok(AssetType::Gold),
            _ => Err(format!("unknown asset type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TxType {
    Buy,
    Sell,
    Dividend,
    Jcp,
    FractionAuction,
    Send,
    Deposit,
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
        }
    }
}

impl fmt::Display for TxType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TxType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "buy" => Ok(TxType::Buy),
            "sell" => Ok(TxType::Sell),
            "dividend" => Ok(TxType::Dividend),
            "jcp" => Ok(TxType::Jcp),
            "fraction_auction" => Ok(TxType::FractionAuction),
            "send" => Ok(TxType::Send),
            "deposit" => Ok(TxType::Deposit),
            _ => Err(format!("unknown tx type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
}

impl fmt::Display for IncomeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for IncomeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dividend" => Ok(IncomeType::Dividend),
            "jcp" => Ok(IncomeType::Jcp),
            _ => Err(format!("unknown income type: {}", s)),
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
