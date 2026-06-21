use crate::agents::types::{ConfirmationResponse, StreamChunk};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub fn spawn_approver(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<StreamChunk>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(chunk) = rx.recv().await {
            if let StreamChunk::RequestConfirmation {
                tx: Some(resp_tx), ..
            } = chunk
            {
                let mut lock = resp_tx.lock().await;
                if let Some(sender) = lock.take() {
                    let _ = sender.send(ConfirmationResponse::AllowSession);
                }
            }
        }
    })
}

pub fn send_status(tx: &Option<UnboundedSender<StreamChunk>>, message: impl Into<String>) {
    if let Some(sender) = tx {
        let _ = sender.send(StreamChunk::Status {
            content: message.into(),
        });
    }
}

pub fn empty_sender() -> Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<ConfirmationResponse>>>> {
    Arc::new(tokio::sync::Mutex::new(None))
}
