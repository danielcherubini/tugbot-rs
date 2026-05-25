use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const EXA_API_URL: &str = "https://api.exa.ai/search";

#[derive(Serialize)]
struct ExaSearchRequest {
    query: String,
    #[serde(rename = "type")]
    search_type: String,
    num_results: u8,
    contents: ExaContents,
}

#[derive(Serialize)]
struct ExaContents {
    highlights: bool,
}

#[derive(Deserialize)]
struct ExaSearchResponse {
    results: Vec<ExaResult>,
}

#[derive(Deserialize)]
struct ExaResult {
    title: String,
    #[serde(default)]
    highlights: Vec<String>,
}

pub async fn search(query: &str) -> Result<Vec<(String, String)>> {
    let api_key = std::env::var("EXA_API_KEY").context("EXA_API_KEY not set")?;

    let request = ExaSearchRequest {
        query: query.to_string(),
        search_type: "auto".to_string(),
        num_results: 3,
        contents: ExaContents { highlights: true },
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("Failed to build HTTP client")?;

    let response = client
        .post(EXA_API_URL)
        .header("x-api-key", &api_key)
        .json(&request)
        .send()
        .await
        .context("Failed to send Exa search request")?;

    let search_response: ExaSearchResponse = response
        .json()
        .await
        .context("Failed to parse Exa search response")?;

    let results = search_response
        .results
        .into_iter()
        .map(|r| {
            let snippet = r.highlights.join(" ");
            (r.title, snippet)
        })
        .collect();

    Ok(results)
}
