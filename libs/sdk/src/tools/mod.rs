pub mod bash;
pub mod file_ops;
pub mod lsp;
pub mod mcp;
pub mod navigation;
pub mod plan;
pub mod registry;
pub mod subagent;
pub mod traits;
pub mod web;

pub use registry::ToolRegistry;
pub use traits::Tool;
