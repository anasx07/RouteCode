use super::client::LspClient;
use anyhow::{anyhow, Result};
use lsp_types::{ClientCapabilities, InitializeParams, Url, WorkspaceFolder};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct LspManager {
    clients: Mutex<HashMap<String, Arc<LspClient>>>,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get_or_spawn_client(&self, file_path: &Path) -> Result<Arc<LspClient>> {
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        let mut clients = self.clients.lock().await;

        if let Some(client) = clients.get(&ext) {
            return Ok(client.clone());
        }

        let (program, args) = match ext.as_str() {
            "rs" => ("rust-analyzer", vec![]),
            "ts" | "js" | "tsx" | "jsx" => ("typescript-language-server", vec!["--stdio"]),
            "py" => ("pylsp", vec![]),
            "go" => ("gopls", vec![]),
            "c" | "cpp" | "h" | "hpp" => ("clangd", vec![]),
            _ => return Err(anyhow!("No LSP configured for extension: {}", ext)),
        };

        let client = LspClient::spawn(program, &args).await.map_err(|e| {
            anyhow!(
                "Failed to spawn {} (is it installed?). Error: {}",
                program, e
            )
        })?;

        let client_arc = Arc::new(client);

        // Initialize handshake
        let root_dir = crate::utils::storage::find_project_root();
        let root_uri = Url::from_file_path(&root_dir).unwrap();

        let init_params = InitializeParams {
            process_id: Some(std::process::id()),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: root_uri.clone(),
                name: root_dir.file_name().unwrap_or_default().to_string_lossy().to_string(),
            }]),
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };

        let _init_result = client_arc
            .request(
                "initialize",
                serde_json::to_value(init_params)?,
            )
            .await?;

        // Send initialized notification
        client_arc.notify("initialized", json!({})).await?;

        clients.insert(ext.clone(), client_arc.clone());

        Ok(client_arc)
    }
}
