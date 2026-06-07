use crate::core::ToolResult;
use crate::tools::traits::Tool;
use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use serde_json::json;

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "webfetch"
    }

    fn description(&self) -> &str {
        "Fetches the content of a URL and extracts readable text. Useful for reading documentation or external context."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch (must start with http:// or https://)"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let url = params["url"].as_str().unwrap_or("");
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(ToolResult::error("URL must start with http:// or https://"));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        let response = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    return Ok(ToolResult::error(format!("HTTP Error: {}", resp.status())));
                }

                let html = resp.text().await?;

                // Extremely simple HTML to text conversion using regex
                // 1. Remove script tags and their contents
                let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
                let text = re_script.replace_all(&html, "");

                // 2. Remove style tags and their contents
                let re_style = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
                let text = re_style.replace_all(&text, "");

                // 3. Replace typical block elements with newlines
                let re_block = Regex::new(r"(?i)</(p|div|br|h1|h2|h3|h4|h5|h6|li|tr)>").unwrap();
                let text = re_block.replace_all(&text, "\n");

                // 4. Strip all remaining HTML tags
                let re_tags = Regex::new(r"(?is)<[^>]+>").unwrap();
                let text = re_tags.replace_all(&text, "");

                // 5. Decode basic HTML entities
                let text = text
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&amp;", "&")
                    .replace("&quot;", "\"")
                    .replace("&#39;", "'")
                    .replace("&nbsp;", " ");

                // 6. Condense multiple newlines and spaces
                let re_newlines = Regex::new(r"\n\s*\n").unwrap();
                let mut text = re_newlines.replace_all(&text, "\n\n").to_string();

                // Truncate if it's too large (e.g. > 100k characters)
                if text.len() > 100_000 {
                    text.truncate(100_000);
                    text.push_str("\n\n...[Content truncated due to length]");
                }

                Ok(ToolResult::success(text.trim().to_string()))
            }
            Err(e) => Ok(ToolResult::error(format!("Failed to fetch URL: {}", e))),
        }
    }
}
