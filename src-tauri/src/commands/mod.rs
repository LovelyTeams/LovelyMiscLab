//! `#[tauri::command]` surface. Grouped by concern.

mod ai_request;
pub mod ai_solver;
pub mod ai_workflow;
pub mod artifacts;
pub mod graph;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod modules;
pub mod project;
pub mod script_modules;
pub mod settings;
pub mod system;
pub mod update;
