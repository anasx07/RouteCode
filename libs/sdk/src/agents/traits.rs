use crate::agents::types::StreamChunk;
use crate::core::Message;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;

pub type StreamResponse = Pin<Box<dyn Stream<Item = Result<StreamChunk, anyhow::Error>> + Send>>;

#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error>;

    /// `messages` and `tools` are passed as `Arc<Vec<…>>` so the orchestrator
    /// can reuse the same allocation across retry attempts without cloning
    /// the underlying data. Providers should treat them as immutable borrows
    /// (`messages.iter()`, `tools.as_ref()`) and only clone the inner `Vec`
    /// if they genuinely need to take ownership.
    async fn ask(
        &self,
        messages: Arc<Vec<Message>>,
        model: &str,
        tools: Arc<Option<Vec<serde_json::Value>>>,
        thinking_level: Option<&str>,
    ) -> Result<StreamResponse, anyhow::Error>;
}
