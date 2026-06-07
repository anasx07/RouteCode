use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, Mutex};

pub struct McpClient {
    request_tx: mpsc::Sender<(Value, Option<oneshot::Sender<Value>>)>,
    next_id: Arc<AtomicUsize>,
}

impl McpClient {
    pub async fn spawn(command: &str, args: &[String], envs: &HashMap<String, String>) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
           .envs(envs)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::inherit()); // Pass stderr to console for debugging

        let mut child = cmd.spawn()?;

        let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to open stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to open stdout"))?;

        let (request_tx, mut request_rx) = mpsc::channel::<(Value, Option<oneshot::Sender<Value>>)>(32);

        let pending_requests: Arc<Mutex<HashMap<usize, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let pending_clone = pending_requests.clone();

        // Output reader loop (newline-delimited JSON)
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                    break; // EOF
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                
                if let Ok(msg) = serde_json::from_str::<Value>(trimmed) {
                    if let Some(id) = msg.get("id").and_then(|id| id.as_u64()) {
                        let mut pending = pending_clone.lock().await;
                        if let Some(tx) = pending.remove(&(id as usize)) {
                            let _ = tx.send(msg);
                        }
                    }
                }
            }
        });

        // Input writer loop
        let pending_writer_clone = pending_requests.clone();
        tokio::spawn(async move {
            while let Some((req, response_tx_opt)) = request_rx.recv().await {
                if let Some(response_tx) = response_tx_opt {
                    if let Some(id) = req.get("id").and_then(|id| id.as_u64()) {
                        pending_writer_clone.lock().await.insert(id as usize, response_tx);
                    }
                }

                let mut body = serde_json::to_string(&req).unwrap();
                body.push('\n'); // MCP requires \n
                
                if stdin.write_all(body.as_bytes()).await.is_err() {
                    break;
                }
                let _ = stdin.flush().await;
            }
        });

        Ok(Self {
            request_tx,
            next_id: Arc::new(AtomicUsize::new(1)),
        })
    }

    pub async fn request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let mut req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
        });
        if let Some(p) = params {
            req["params"] = p;
        }

        let (tx, rx) = oneshot::channel();
        self.request_tx.send((req, Some(tx))).await?;

        let resp = rx.await?;
        if let Some(error) = resp.get("error") {
            return Err(anyhow!("MCP Error: {}", error));
        }
        
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    pub async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        let mut req = json!({
            "jsonrpc": "2.0",
            "method": method,
        });
        if let Some(p) = params {
            req["params"] = p;
        }

        self.request_tx.send((req, None)).await?;
        Ok(())
    }
}
