use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, Mutex};

pub struct LspClient {
    request_tx: mpsc::Sender<(Value, Option<oneshot::Sender<Value>>)>,
    next_id: Arc<AtomicUsize>,
}

impl LspClient {
    pub async fn spawn(program: &str, args: &[&str]) -> Result<Self> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to open stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to open stdout"))?;

        let (request_tx, mut request_rx) = mpsc::channel::<(Value, Option<oneshot::Sender<Value>>)>(32);

        let pending_requests: Arc<Mutex<HashMap<usize, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let pending_clone = pending_requests.clone();

        // Output reader loop
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut content_length = 0;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                        return; // EOF
                    }
                    let line = line.trim();
                    if line.is_empty() {
                        break;
                    }
                    if line.starts_with("Content-Length:") {
                        if let Some(len_str) = line.split(':').nth(1) {
                            content_length = len_str.trim().parse().unwrap_or(0);
                        }
                    }
                }

                if content_length > 0 {
                    let mut buf = vec![0u8; content_length];
                    if reader.read_exact(&mut buf).await.is_ok() {
                        if let Ok(msg) = serde_json::from_slice::<Value>(&buf) {
                            if let Some(id) = msg.get("id").and_then(|id| id.as_u64()) {
                                let mut pending = pending_clone.lock().await;
                                if let Some(tx) = pending.remove(&(id as usize)) {
                                    let _ = tx.send(msg);
                                }
                            }
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

                let body = serde_json::to_string(&req).unwrap();
                let msg = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
                if stdin.write_all(msg.as_bytes()).await.is_err() {
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

    pub async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let (tx, rx) = oneshot::channel();
        self.request_tx.send((req, Some(tx))).await?;

        let resp = rx.await?;
        if let Some(error) = resp.get("error") {
            return Err(anyhow!("LSP Error: {}", error));
        }
        
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    pub async fn notify(&self, method: &str, params: Value) -> Result<()> {
        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        self.request_tx.send((req, None)).await?;
        Ok(())
    }
}
