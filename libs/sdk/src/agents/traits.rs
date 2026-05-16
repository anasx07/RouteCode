use crate::agents::types::StreamChunk;
use crate::core::Message;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

pub type StreamResponse = Pin<Box<dyn Stream<Item = Result<StreamChunk, anyhow::Error>> + Send>>;

#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error>;
    async fn ask(
        &self,
        messages: Vec<Message>,
        model: &str,
        tools: Option<Vec<serde_json::Value>>,
    ) -> Result<StreamResponse, anyhow::Error>;
}
