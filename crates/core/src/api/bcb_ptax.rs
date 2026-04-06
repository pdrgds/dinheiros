use chrono::NaiveDate;
use serde::Deserialize;

const BASE_URL: &str = "https://olinda.bcb.gov.br/olinda/servico/PTAX/versao/v1/odata";

#[derive(Debug, Deserialize)]
struct PtaxResponse {
    value: Vec<PtaxQuote>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PtaxQuote {
    #[serde(rename = "cotacaoCompra")]
    cotacao_compra: f64,
    #[serde(rename = "cotacaoVenda")]
    cotacao_venda: f64,
    #[serde(rename = "dataHoraCotacao")]
    data_hora_cotacao: String,
    // Present on multi-currency endpoints but absent on USD endpoints.
    #[serde(rename = "paridadeCompra")]
    #[serde(default)]
    paridade_compra: Option<f64>,
    #[serde(rename = "paridadeVenda")]
    #[serde(default)]
    paridade_venda: Option<f64>,
    #[serde(rename = "tipoBoletim")]
    #[serde(default)]
    tipo_boletim: Option<String>,
}

/// Fetch PTAX sell rate for a currency on a given date.
/// For weekends/holidays, tries up to 5 previous days.
pub async fn fetch_rate(
    currency: &str,
    date: NaiveDate,
) -> Result<f64, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut current = date;

    for _ in 0..5 {
        let formatted = current.format("%m-%d-%Y").to_string();
        let url = if currency == "USD" {
            format!(
                "{}/CotacaoDolarDia(dataCotacao=@dataCotacao)?@dataCotacao='{}'&$format=json",
                BASE_URL, formatted
            )
        } else {
            format!(
                "{}/CotacaoMoedaDia(moeda=@moeda,dataCotacao=@dataCotacao)?@moeda='{}'&@dataCotacao='{}'&$format=json",
                BASE_URL, currency, formatted
            )
        };

        let resp: PtaxResponse = client.get(&url).send().await?.json().await?;

        if let Some(quote) = resp.value.last() {
            return Ok(quote.cotacao_venda);
        }

        current -= chrono::Duration::days(1);
    }

    Err(format!(
        "No PTAX rate found for {} near {} after 5 attempts",
        currency, date
    )
    .into())
}

/// Fetch rates for a currency over a date range.
/// Returns (date, rate) pairs for all business days in the range.
/// Each business day has one entry using the last available quote of that day.
pub async fn fetch_rates_range(
    currency: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<(NaiveDate, f64)>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let from_fmt = from.format("%m-%d-%Y").to_string();
    let to_fmt = to.format("%m-%d-%Y").to_string();

    let url = if currency == "USD" {
        format!(
            "{}/CotacaoDolarPeriodo(dataInicial=@di,dataFinalCotacao=@df)?@di='{}'&@df='{}'&$format=json",
            BASE_URL, from_fmt, to_fmt
        )
    } else {
        format!(
            "{}/CotacaoMoedaPeriodo(moeda=@moeda,dataInicial=@di,dataFinalCotacao=@df)?@moeda='{}'&@di='{}'&@df='{}'&$format=json",
            BASE_URL, currency, from_fmt, to_fmt
        )
    };

    let resp: PtaxResponse = client.get(&url).send().await?.json().await?;

    // Group by date and keep the last quote for each day (the closing rate).
    let mut by_date: std::collections::BTreeMap<NaiveDate, f64> = std::collections::BTreeMap::new();
    for quote in &resp.value {
        let date_str = &quote.data_hora_cotacao[..10];
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
        by_date.insert(date, quote.cotacao_venda);
    }

    Ok(by_date.into_iter().collect())
}
