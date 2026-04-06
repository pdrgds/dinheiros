use thiserror::Error;

const BASE_URL: &str = "https://ndcdyn.interactivebrokers.com/AccountManagement/FlexWebService";
const USER_AGENT: &str = "investimentos-v2/0.1";

#[derive(Debug, Error)]
pub enum IbkrFlexError {
    #[error("IBKR Flex not configured: token or query_id is empty")]
    NotConfigured,
    #[error("Failed to extract reference code from IBKR response")]
    NoReferenceCode,
    #[error("Statement not ready after retries")]
    StatementNotReady,
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

/// Fetch a Flex statement from IBKR's two-step web service.
///
/// Step 1: SendRequest to get a reference code.
/// Step 2: GetStatement using the reference code (with retries).
pub async fn fetch_flex_statement(
    token: &str,
    query_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if token.is_empty() || query_id.is_empty() {
        return Err(Box::new(IbkrFlexError::NotConfigured));
    }

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()?;

    // Step 1: SendRequest
    let send_url = format!(
        "{}/SendRequest?t={}&q={}&v=3",
        BASE_URL, token, query_id
    );
    let send_resp = client.get(&send_url).send().await?.text().await?;

    let reference_code = extract_reference_code(&send_resp)
        .ok_or(IbkrFlexError::NoReferenceCode)?;

    // Step 2: GetStatement with retries
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    for attempt in 0..3 {
        let get_url = format!(
            "{}/GetStatement?t={}&q={}&v=3",
            BASE_URL, token, reference_code
        );
        let body = client.get(&get_url).send().await?.text().await?;

        if body.contains("<FlexQueryResponse") || body.contains("<FlexStatementResponse") {
            return Ok(body);
        }

        if attempt < 2 {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }

    Err(Box::new(IbkrFlexError::StatementNotReady))
}

/// Extract the reference code from IBKR's SendRequest XML response.
/// Looks for `<ReferenceCode>VALUE</ReferenceCode>`.
fn extract_reference_code(xml: &str) -> Option<String> {
    let start_tag = "<ReferenceCode>";
    let end_tag = "</ReferenceCode>";
    let start = xml.find(start_tag)? + start_tag.len();
    let end = xml[start..].find(end_tag)? + start;
    let code = xml[start..end].trim();
    if code.is_empty() {
        None
    } else {
        Some(code.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_reference_code() {
        let xml = r#"<SendRequest><ReferenceCode>123456789</ReferenceCode></SendRequest>"#;
        assert_eq!(
            extract_reference_code(xml),
            Some("123456789".to_string())
        );
    }

    #[test]
    fn test_extract_reference_code_missing() {
        let xml = r#"<SendRequest><Error>Invalid token</Error></SendRequest>"#;
        assert_eq!(extract_reference_code(xml), None);
    }

    #[test]
    fn test_extract_reference_code_empty() {
        let xml = r#"<SendRequest><ReferenceCode></ReferenceCode></SendRequest>"#;
        assert_eq!(extract_reference_code(xml), None);
    }
}
