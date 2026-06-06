use anyhow::Result;

/// Fetch a URL and convert HTML to markdown.
pub struct WebFetch;

impl WebFetch {
    pub async fn fetch(url: &str, max_chars: Option<usize>) -> Result<String> {
        let max = max_chars.unwrap_or(8000).min(50000);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("VO1D/1.0")
            .build()?;
        let resp = client.get(url).send().await?;
        let content_type = resp.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body = resp.text().await?;
        if content_type.contains("application/json") {
            let truncated: String = body.chars().take(max).collect();
            return Ok(format!("JSON response from {}:\n\n```json\n{}\n```", url, truncated));
        }
        if body.len() > max * 2 {
            let truncated = &body[..max * 2];
            let md = html2md::parse_html(truncated);
            let md: String = md.chars().take(max).collect();
            return Ok(format!(
                "Content from {} (truncated to ~{} chars):\n\n{}\n\n---\n(Content truncated)",
                url, max, md
            ));
        }
        let md = html2md::parse_html(&body);
        if md.len() > max {
            let truncated: String = md.chars().take(max).collect();
            Ok(format!(
                "Content from {} (truncated to ~{} chars):\n\n{}\n\n---\n(Content truncated)",
                url, max, truncated
            ))
        } else {
            Ok(format!("Content from {}:\n\n{}", url, md))
        }
    }
}
