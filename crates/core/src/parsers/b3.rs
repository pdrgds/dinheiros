use chrono::NaiveDate;
use sha2::{Digest, Sha256};
use std::path::Path;

use calamine::{open_workbook, Data, Reader, Xlsx};

use crate::{AssetType, Income, IncomeType, Source, Transaction, TxType};

#[derive(Debug)]
pub struct B3ImportResult {
    pub transactions: Vec<Transaction>,
    pub income: Vec<Income>,
}

pub fn parse_b3_xlsx(path: &Path) -> Result<B3ImportResult, Box<dyn std::error::Error>> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    let sheet = workbook
        .worksheet_range("Movimentação")?;

    let mut transactions = Vec::new();
    let mut income = Vec::new();

    for (i, row) in sheet.rows().enumerate() {
        // Skip header row
        if i == 0 {
            continue;
        }

        if row.len() < 8 {
            continue;
        }

        let _entrada_saida = cell_str(&row[0]);
        let data_str = cell_str(&row[1]);
        let movimentacao = cell_str(&row[2]);
        let produto = cell_str(&row[3]);
        let _instituicao = cell_str(&row[4]);
        let quantidade = cell_f64(&row[5]);
        let preco_unitario = cell_f64(&row[6]);
        let valor_operacao = cell_f64(&row[7]);

        // Skip CDB entries (automatic savings, not investments)
        if produto.contains("CDB") {
            continue;
        }

        let date = parse_b3_date(&data_str)?;
        let symbol = extract_symbol(&produto);
        let asset_type = if produto.starts_with("Tesouro") {
            AssetType::Tesouro
        } else {
            AssetType::StockBr
        };

        let import_hash = make_import_hash(&data_str, &movimentacao, &produto, quantidade, preco_unitario);

        match movimentacao.as_str() {
            "Compra" => {
                transactions.push(Transaction {
                    id: None,
                    source: Source::B3,
                    asset_type,
                    symbol,
                    tx_type: TxType::Buy,
                    date,
                    quantity: quantidade,
                    unit_price: Some(preco_unitario),
                    currency: "BRL".to_string(),
                    total_value: valor_operacao,
                    brl_rate: 1.0,
                    total_brl: valor_operacao,
                    commission: None,
                    fee_brl: None,
                    notes: None,
                    import_hash,
                });
            }
            "Venda" => {
                transactions.push(Transaction {
                    id: None,
                    source: Source::B3,
                    asset_type,
                    symbol,
                    tx_type: TxType::Sell,
                    date,
                    quantity: quantidade,
                    unit_price: Some(preco_unitario),
                    currency: "BRL".to_string(),
                    total_value: valor_operacao,
                    brl_rate: 1.0,
                    total_brl: valor_operacao,
                    commission: None,
                    fee_brl: None,
                    notes: None,
                    import_hash,
                });
            }
            "Transferência - Liquidação" => {
                transactions.push(Transaction {
                    id: None,
                    source: Source::B3,
                    asset_type,
                    symbol,
                    tx_type: TxType::Sell,
                    date,
                    quantity: quantidade,
                    unit_price: Some(preco_unitario),
                    currency: "BRL".to_string(),
                    total_value: valor_operacao,
                    brl_rate: 1.0,
                    total_brl: valor_operacao,
                    commission: None,
                    fee_brl: None,
                    notes: Some("Transferência - Liquidação".to_string()),
                    import_hash,
                });
            }
            "Leilão de Fração" => {
                transactions.push(Transaction {
                    id: None,
                    source: Source::B3,
                    asset_type,
                    symbol,
                    tx_type: TxType::FractionAuction,
                    date,
                    quantity: quantidade,
                    unit_price: Some(preco_unitario),
                    currency: "BRL".to_string(),
                    total_value: valor_operacao,
                    brl_rate: 1.0,
                    total_brl: valor_operacao,
                    commission: None,
                    fee_brl: None,
                    notes: None,
                    import_hash,
                });
            }
            "Dividendo" => {
                income.push(Income {
                    id: None,
                    source: Source::B3,
                    symbol,
                    date,
                    income_type: IncomeType::Dividend,
                    currency: "BRL".to_string(),
                    gross_value: valor_operacao,
                    tax_withheld: None,
                    tax_origin: Some("BR".to_string()),
                    brl_rate: 1.0,
                    net_value_brl: valor_operacao,
                    import_hash,
                });
            }
            "Juros Sobre Capital Próprio" => {
                let tax = valor_operacao * 0.15;
                let net = valor_operacao - tax;
                income.push(Income {
                    id: None,
                    source: Source::B3,
                    symbol,
                    date,
                    income_type: IncomeType::Jcp,
                    currency: "BRL".to_string(),
                    gross_value: valor_operacao,
                    tax_withheld: Some(tax),
                    tax_origin: Some("BR".to_string()),
                    brl_rate: 1.0,
                    net_value_brl: net,
                    import_hash,
                });
            }
            other => {
                eprintln!("Unknown B3 movimentação type: {}", other);
            }
        }
    }

    Ok(B3ImportResult {
        transactions,
        income,
    })
}

pub fn extract_symbol(produto: &str) -> String {
    if produto.starts_with("Tesouro") {
        return produto.to_string();
    }
    match produto.find(" - ") {
        Some(pos) => produto[..pos].trim().to_string(),
        None => produto.trim().to_string(),
    }
}

fn parse_b3_date(s: &str) -> Result<NaiveDate, Box<dyn std::error::Error>> {
    Ok(NaiveDate::parse_from_str(s, "%d/%m/%Y")?)
}

fn cell_str(cell: &Data) -> String {
    match cell {
        Data::String(s) => s.clone(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("{:?}", e),
        Data::Empty => String::new(),
    }
}

fn cell_f64(cell: &Data) -> f64 {
    match cell {
        Data::Float(f) => *f,
        Data::Int(i) => *i as f64,
        Data::String(s) => s.replace(',', ".").parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn make_import_hash(date: &str, movimentacao: &str, produto: &str, quantidade: f64, preco_unitario: f64) -> String {
    let input = format!("b3:{}:{}:{}:{}:{}", date, movimentacao, produto, quantidade, preco_unitario);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
