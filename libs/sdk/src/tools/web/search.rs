use crate::core::ToolResult;
use crate::tools::traits::Tool;
use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use serde_json::json;

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "websearch"
    }

    fn description(&self) -> &str {
        "Searches the web for a query using DuckDuckGo Lite. Returns titles and URLs. You can chain this with webfetch to read the content of the URLs."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let query = params["query"].as_str().unwrap_or("");
        if query.is_empty() {
            return Ok(ToolResult::error("Search query cannot be empty"));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        let response = client
            .post("https://lite.duckduckgo.com/lite/")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .form(&[("q", query)])
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    return Ok(ToolResult::error(format!("HTTP Error: {}", resp.status())));
                }

                let html = resp.text().await?;

                let mut results = vec![];

                // Matches <a ... class='result-link' href='URL'>TITLE</a>
                let re_link = Regex::new(r#"(?is)<a[^>]*class=['"]result-link['"][^>]*href=['"]([^'"]+)['"][^>]*>(.*?)</a>"#).unwrap();
                let re_tags = Regex::new(r"(?is)<[^>]+>").unwrap();

                for cap in re_link.captures_iter(&html) {
                    let url = &cap[1];
                    let title = &cap[2];

                    if url.starts_with("/")
                        || url.contains("duckduckgo.com")
                        || url.starts_with("?q=")
                    {
                        continue;
                    }

                    let clean_title = re_tags.replace_all(title, "").to_string();
                    let clean_title = clean_title
                        .replace("&quot;", "\"")
                        .replace("&#39;", "'")
                        .replace("&amp;", "&");

                    if !clean_title.trim().is_empty() {
                        results.push(json!({
                            "title": clean_title.trim(),
                            "url": url,
                        }));
                    }

                    if results.len() >= 10 {
                        break;
                    }
                }

                if results.is_empty() {
                    return Ok(ToolResult::success("No search results found.".to_string()));
                }

                Ok(ToolResult::success(serde_json::to_string_pretty(&results)?))
            }
            Err(e) => Ok(ToolResult::error(format!("Search failed: {}", e))),
        }
    }
}
