use super::client::McpClient;
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;
use crate::tools::mcp_tool::DynamicMcpTool;

#[derive(Deserialize, Debug, Clone)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct McpConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

pub struct McpManager {
    clients: Mutex<HashMap<String, Arc<McpClient>>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
        }
    }

    pub async fn load_and_register_tools(&self, registry: &mut crate::tools::ToolRegistry) -> Result<()> {
        let root = crate::utils::storage::find_project_root();
        let config_path = root.join(".routecode").join("mcp.json");
        
        if !config_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(config_path)?;
        let config: McpConfig = serde_json::from_str(&content)?;

        for (server_name, server_config) in config.mcp_servers {
            match self.boot_server(&server_name, server_config).await {
                Ok(client) => {
                    if let Ok(tools_res) = client.request("tools/list", None).await {
                        if let Some(tools) = tools_res.get("tools").and_then(|t| t.as_array()) {
                            for tool_info in tools {
                                let name = tool_info["name"].as_str().unwrap_or("").to_string();
                                let description = tool_info["description"].as_str().unwrap_or("").to_string();
                                let input_schema = tool_info["inputSchema"].clone();
                                
                                let prefixed_name = format!("{}_{}", server_name, name);
                                
                                let dynamic_tool = DynamicMcpTool::new(
                                    prefixed_name,
                                    name,
                                    description,
                                    input_schema,
                                    client.clone()
                                );
                                
                                registry.register(Arc::new(dynamic_tool));
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to boot MCP server {}: {}", server_name, e);
                }
            }
        }
        
        Ok(())
    }

    async fn boot_server(&self, name: &str, config: McpServerConfig) -> Result<Arc<McpClient>> {
        let client = McpClient::spawn(&config.command, &config.args, &config.env).await?;
        let client_arc = Arc::new(client);
        
        let init_params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": true }
            },
            "clientInfo": {
                "name": "routecode",
                "version": "1.0.0"
            }
        });

        client_arc.request("initialize", Some(init_params)).await?;
        client_arc.notify("notifications/initialized", None).await?;
        
        let mut clients = self.clients.lock().await;
        clients.insert(name.to_string(), client_arc.clone());
        
        Ok(client_arc)
    }
}
