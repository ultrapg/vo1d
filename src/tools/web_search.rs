use anyhow::Result;

/// Search the web using DuckDuckGo (no API key required).
pub struct WebSearch;

impl WebSearch {
    pub async fn search(query: &str, num_results: Option<usize>) -> Result<String> {
        let n = num_results.unwrap_or(5).min(10);
        let results = websearch::web_search(websearch::SearchOptions {
            query: query.to_string(),
            max_results: Some(n as u32),
            provider: Box::new(websearch::providers::DuckDuckGoProvider::new()),
            ..Default::default()
        })
        .await
        .map_err(|e| anyhow::anyhow!("Web search failed: {}", e))?;

        if results.is_empty() {
            return Ok(format!("No results found for '{}'", query));
        }

        let mut out = format!("Search results for '{}' ({} results):\n\n", query, results.len());
        for (i, r) in results.iter().enumerate() {
            out.push_str(&format!("{}. [{}]({})\n", i + 1, r.title, r.url));
            if let Some(ref snippet) = r.snippet {
                out.push_str(&format!("   {}\n\n", snippet));
            } else {
                out.push('\n');
            }
        }
        Ok(out)
    }
}
