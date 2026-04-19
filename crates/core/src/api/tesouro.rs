use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;

const BASE_URL: &str = "https://api.radaropcoes.com/bonds";

#[derive(Debug, Deserialize)]
struct BondResponse {
    #[serde(rename = "treasuryBondName")]
    treasury_bond_name: String,
    #[serde(rename = "unitaryRedemptionValue")]
    unitary_redemption_value: f64,
    #[serde(rename = "unitaryInvestmentValue")]
    unitary_investment_value: f64,
    updated_at: Option<String>,
}

/// Fetch current price (PU de venda/resgate) for a Tesouro Direto bond.
/// Symbol should be e.g. "Tesouro Selic 2029", "Tesouro IPCA+ 2045".
/// Returns the unitary redemption value (mark-to-market price per unit).
pub async fn fetch_tesouro_price(symbol: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let client = Client::new();
    let encoded = urlencoding::encode(symbol);
    let url = format!("{}/{}", BASE_URL, encoded);

    let resp = client
        .get(&url)
        .header("User-Agent", "dinheiros/0.1")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(format!("Tesouro API error {} for {}", resp.status(), symbol).into());
    }

    let bond: BondResponse = resp.json().await?;
    Ok(bond.unitary_redemption_value)
}

/// Check if a symbol is a Tesouro Direto bond.
pub fn is_tesouro(symbol: &str) -> bool {
    symbol.starts_with("Tesouro")
}
