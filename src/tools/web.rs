use anyhow::{Context, Result};
use std::collections::HashMap;

/// HTTP request tool using reqwest.
pub async fn http_request(
    url: &str,
    method: Option<&str>,
    headers: Option<&HashMap<String, String>>,
    body: Option<&str>,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("VO1D/1.0")
        .build()
        .context("Failed to create HTTP client")?;

    let method = method.unwrap_or("GET");
    let mut request = match method.to_uppercase().as_str() {
        "GET" => client.get(url),
        "POST" => {
            let mut r = client.post(url);
            if let Some(b) = body {
                r = r.body(b.to_string());
            }
            r
        }
        "PUT" => {
            let mut r = client.put(url);
            if let Some(b) = body {
                r = r.body(b.to_string());
            }
            r
        }
        "DELETE" => client.delete(url),
        "PATCH" => {
            let mut r = client.patch(url);
            if let Some(b) = body {
                r = r.body(b.to_string());
            }
            r
        }
        "HEAD" => client.head(url),
        _ => anyhow::bail!("Unsupported HTTP method: {}", method),
    };

    // Add headers
    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            request = request.header(key.as_str(), value.as_str());
        }
    }

    let response = request.send().await
        .with_context(|| format!("HTTP request failed: {} {}", method, url))?;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await
        .context("Failed to read response body")?;

    let mut output = format!("HTTP {} {}\n", status.as_u16(), status.canonical_reason().unwrap_or("Unknown"));

    // Include relevant headers
    for name in &["content-type", "content-length", "date", "server"] {
        if let Some(value) = headers.get(*name) {
            output.push_str(&format!("{}: {}\n", name, value.to_str().unwrap_or("?")));
        }
    }

    output.push('\n');

    // Truncate large responses
    if body.len() > 50_000 {
        output.push_str(&body[..50_000]);
        output.push_str(&format!("\n... [Body truncated: {} bytes total]", body.len()));
    } else {
        output.push_str(&body);
    }

    Ok(output)
}
