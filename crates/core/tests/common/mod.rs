//! Shared test fixtures for parser integration tests.
//!
//! Each helper writes a synthetic, fully transparent input file into the
//! supplied directory and returns its path. Designed to exercise every parser
//! branch without requiring real broker exports (which contain personal data
//! and cannot be checked in).
//!
//! Helpers are `pub` because integration-test files include this module via
//! `mod common;`; `dead_code` suppresses warnings for helpers a given test
//! file doesn't reference.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use rust_xlsxwriter::Workbook;

/// Write a synthetic B3 "Movimentação" XLSX into `dir` and return its path.
///
/// Row design — each row exercises a distinct parser branch:
///
/// | # | Entrada/Saída | Date       | Movimentação                 | Produto                            | Qty | Preço   | Valor    |
/// |---|---------------|------------|------------------------------|------------------------------------|-----|---------|----------|
/// | 1 | Debito        | 15/01/2024 | Compra                       | PETR4 - PETROLEO BRASILEIRO S/A    | 100 | 35.00   | 3500.00  |
/// | 2 | Credito       | 20/06/2024 | Venda                        | PETR4 - PETROLEO BRASILEIRO S/A    | 50  | 38.00   | 1900.00  |
/// | 3 | Credito       | 10/01/2024 | Transferência - Liquidação   | BBAS3 - BANCO DO BRASIL            | 10  | 50.00   | 500.00   |
/// | 4 | Credito       | 15/02/2024 | Transferência - Liquidação   | BBAS3 - BANCO DO BRASIL            | 12  | 52.00   | 624.00   |
/// | 5 | Credito       | 20/03/2024 | Transferência - Liquidação   | BBAS3 - BANCO DO BRASIL            | 5   | 55.00   | 275.00   |
/// | 6 | Credito       | 02/04/2024 | Transferência - Liquidação   | BBAS3 - BANCO DO BRASIL            | 8   | 56.70   | 453.60   |
/// | 7 | Credito       | 05/04/2024 | Transferência                | BBAS3 - BANCO DO BRASIL            | 8   | 0       | 0        |
/// | 8 | Debito        | 05/04/2024 | Transferência                | BBAS3 - BANCO DO BRASIL            | 8   | 0       | 0        |
/// | 9 | Credito       | 17/04/2024 | Desdobro                     | BBAS3 - BANCO DO BRASIL            | 15  | 0       | 0        |
/// | 10| Debito        | 27/09/2024 | Transferência - Liquidação   | BBAS3 - BANCO DO BRASIL            | 73  | 27.45   | 2003.85  |
/// | 11| Debito        | 10/02/2024 | Compra                       | Tesouro Selic 2027                 | 1   | 10000   | 10000    |
/// | 12| Credito       | 15/03/2024 | Dividendo                    | PETR4 - PETROLEO BRASILEIRO S/A    | 100 | 1.00    | 100.00   |
/// | 13| Credito       | 30/06/2024 | Juros Sobre Capital Próprio  | ITUB4 - ITAU UNIBANCO              | 300 | 0.1667  | 50.00    |
/// | 14| Debito        | 01/05/2024 | Compra                       | CDB 110% CDI Banco Inter           | 1   | 5000    | 5000     |
/// | 15| Credito       | 12/06/2024 | Bonificação em Ativos        | VALE3 - VALE                       | 5   | 0       | 0        |
/// | 16| Debito        | 25/07/2024 | Leilão de Fração             | VALE3 - VALE                       | 2   | 60.00   | 120.00   |
///
/// Expected parse result:
/// - 12 transactions (CDB skipped, both Transferência rows dropped)
/// - 2 income entries (1 Dividend + 1 JCP)
pub fn write_b3_sample_xlsx(dir: &Path) -> PathBuf {
    let path = dir.join("b3_sample.xlsx");

    let header: [&str; 8] = [
        "Entrada/Saída",
        "Data",
        "Movimentação",
        "Produto",
        "Instituição",
        "Quantidade",
        "Preço unitário",
        "Valor da Operação",
    ];

    let petr4 = "PETR4 - PETROLEO BRASILEIRO S/A";
    let bbas3 = "BBAS3 - BANCO DO BRASIL";
    let itub4 = "ITUB4 - ITAU UNIBANCO";
    let vale3 = "VALE3 - VALE";
    let tesouro = "Tesouro Selic 2027";
    let cdb = "CDB 110% CDI Banco Inter";
    let inst = "DEMO BROKER";

    #[rustfmt::skip]
    let rows: Vec<(&str, &str, &str, &str, f64, f64, f64)> = vec![
        ("Debito",  "15/01/2024", "Compra",                      petr4,   100.0,  35.00,   3500.00),
        ("Credito", "20/06/2024", "Venda",                       petr4,    50.0,  38.00,   1900.00),
        ("Credito", "10/01/2024", "Transferência - Liquidação",  bbas3,    10.0,  50.00,    500.00),
        ("Credito", "15/02/2024", "Transferência - Liquidação",  bbas3,    12.0,  52.00,    624.00),
        ("Credito", "20/03/2024", "Transferência - Liquidação",  bbas3,     5.0,  55.00,    275.00),
        ("Credito", "02/04/2024", "Transferência - Liquidação",  bbas3,     8.0,  56.70,    453.60),
        ("Credito", "05/04/2024", "Transferência",               bbas3,     8.0,   0.0,       0.0),
        ("Debito",  "05/04/2024", "Transferência",               bbas3,     8.0,   0.0,       0.0),
        ("Credito", "17/04/2024", "Desdobro",                    bbas3,    15.0,   0.0,       0.0),
        ("Debito",  "27/09/2024", "Transferência - Liquidação",  bbas3,    73.0,  27.45,   2003.85),
        ("Debito",  "10/02/2024", "Compra",                      tesouro,   1.0,  10000.00, 10000.00),
        ("Credito", "15/03/2024", "Dividendo",                   petr4,   100.0,   1.00,    100.00),
        ("Credito", "30/06/2024", "Juros Sobre Capital Próprio", itub4,   300.0,   0.1667,   50.00),
        ("Debito",  "01/05/2024", "Compra",                      cdb,       1.0,  5000.00,  5000.00),
        ("Credito", "12/06/2024", "Bonificação em Ativos",       vale3,     5.0,   0.0,       0.0),
        ("Debito",  "25/07/2024", "Leilão de Fração",            vale3,     2.0,  60.00,    120.00),
    ];

    let mut workbook = Workbook::new();
    let sheet = workbook
        .add_worksheet()
        .set_name("Movimentação")
        .expect("set sheet name");
    for (col, h) in header.iter().enumerate() {
        sheet
            .write_string(0, col as u16, *h)
            .expect("write header cell");
    }
    for (i, (es, data, mov, prod, qty, preco, valor)) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        sheet.write_string(row, 0, *es).unwrap();
        sheet.write_string(row, 1, *data).unwrap();
        sheet.write_string(row, 2, *mov).unwrap();
        sheet.write_string(row, 3, *prod).unwrap();
        sheet.write_string(row, 4, inst).unwrap();
        sheet.write_number(row, 5, *qty).unwrap();
        sheet.write_number(row, 6, *preco).unwrap();
        sheet.write_number(row, 7, *valor).unwrap();
    }
    workbook.save(&path).expect("save xlsx");
    path
}

/// Write a synthetic Binance CSV with 7 Buys + 2 Sends + 4 Deposits (deposits
/// must be ignored by the parser). Columns mirror Binance's transaction-history
/// export: id, datetime, type, ..., sent_amount (6), ..., sent_address (9),
/// received_amount (10), ..., fee_amount (14), fee_currency (15).
///
/// First Buy: 2024-04-13, sent 512.00 BRL, received 0.00146106 BTC.
/// Sends use bech32 addresses starting with "bc1q".
/// No Bitcoin-currency fees in this file.
pub fn write_binance_sample_a(dir: &Path) -> PathBuf {
    let path = dir.join("binance_a.csv");
    let body = "\
id,datetime,type,col3,col4,col5,sent_amount,col7,col8,sent_address,received_amount,col11,col12,col13,fee_amount,fee_currency
b001,2024-04-13 10:15:00,Buy,,,,512.00,,,,0.00146106,,,,0.50,BRL
b002,2024-05-20 11:00:00,Buy,,,,300.00,,,,0.00085000,,,,0.30,BRL
b003,2024-06-11 09:45:00,Buy,,,,200.00,,,,0.00056000,,,,0.20,BRL
b004,2024-07-22 14:30:00,Buy,,,,150.00,,,,0.00042000,,,,0.15,BRL
b005,2024-08-30 08:00:00,Buy,,,,100.00,,,,0.00028000,,,,0.10,BRL
b006,2024-10-04 17:25:00,Buy,,,,250.00,,,,0.00070000,,,,0.25,BRL
b007,2024-11-15 13:10:00,Buy,,,,400.00,,,,0.00112000,,,,0.40,BRL
b008,2024-09-02 12:00:00,Send,,,,0.00050000,,,bc1qexampleaddressforpubliconedemo,,,,,0.00001000,BRL
b009,2024-12-18 16:45:00,Send,,,,0.00030000,,,bc1qexampleaddressforpublictwodemo,,,,,0.00001000,BRL
b010,2024-03-01 09:00:00,Deposit,,,,1000.00,,,,,,,,0,BRL
b011,2024-05-01 09:00:00,Deposit,,,,500.00,,,,,,,,0,BRL
b012,2024-07-01 09:00:00,Deposit,,,,800.00,,,,,,,,0,BRL
b013,2024-09-01 09:00:00,Deposit,,,,600.00,,,,,,,,0,BRL
";
    std::fs::write(&path, body).expect("write binance csv a");
    path
}

/// Write a synthetic Binance CSV with 4 Buys + 4 Sends + 4 Deposits.
///
/// First Buy: 2025-03-24. Each Send pays a 0.00003 BTC fee with currency
/// "Bitcoin" (the parser accepts both "BTC" and "Bitcoin"); total Bitcoin
/// fee across the four sends is 0.00012 BTC.
pub fn write_binance_sample_b(dir: &Path) -> PathBuf {
    let path = dir.join("binance_b.csv");
    let body = "\
id,datetime,type,col3,col4,col5,sent_amount,col7,col8,sent_address,received_amount,col11,col12,col13,fee_amount,fee_currency
c001,2025-03-24 10:00:00,Buy,,,,600.00,,,,0.00150000,,,,0.60,BRL
c002,2025-05-12 11:30:00,Buy,,,,400.00,,,,0.00095000,,,,0.40,BRL
c003,2025-08-08 14:00:00,Buy,,,,250.00,,,,0.00058000,,,,0.25,BRL
c004,2025-11-29 09:15:00,Buy,,,,350.00,,,,0.00080000,,,,0.35,BRL
c005,2025-04-10 12:00:00,Send,,,,0.00020000,,,bc1qexamplesendaddressonedemodata,,,,,0.00003000,Bitcoin
c006,2025-06-22 13:00:00,Send,,,,0.00030000,,,bc1qexamplesendaddresstwodemodata,,,,,0.00003000,Bitcoin
c007,2025-09-18 16:00:00,Send,,,,0.00015000,,,bc1qexamplesendaddressthredmodata,,,,,0.00003000,Bitcoin
c008,2025-12-05 18:00:00,Send,,,,0.00025000,,,bc1qexamplesendaddressfourdemodata,,,,,0.00003000,Bitcoin
c009,2025-02-01 09:00:00,Deposit,,,,1500.00,,,,,,,,0,BRL
c010,2025-04-01 09:00:00,Deposit,,,,800.00,,,,,,,,0,BRL
c011,2025-07-01 09:00:00,Deposit,,,,600.00,,,,,,,,0,BRL
c012,2025-10-01 09:00:00,Deposit,,,,700.00,,,,,,,,0,BRL
";
    std::fs::write(&path, body).expect("write binance csv b");
    path
}
