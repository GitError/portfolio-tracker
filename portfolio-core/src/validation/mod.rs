//! Transport-neutral validation and domain policy shared by the Tauri command
//! layer (`src-tauri/src/commands/`) and the MCP server
//! (`portfolio-mcp/src/validation.rs`).
//!
//! Every function here returns `Result<T, String>`: the `String` is the exact
//! user-facing error message. Callers adapt it to their own error type —
//! `AppError::Validation` for Tauri, `McpError::invalid_params` for MCP —
//! so both surfaces reject the same input with the same message, and a rule
//! change here applies to both automatically.

mod accounts;
mod alerts;
mod common;
mod config;
mod dividends;
mod holdings;
mod transactions;
mod watchlists;

pub use accounts::*;
pub use alerts::*;
pub use common::*;
pub use config::*;
pub use dividends::*;
pub use holdings::*;
pub use transactions::*;
pub use watchlists::*;
