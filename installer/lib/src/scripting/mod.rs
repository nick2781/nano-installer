//! Rhai scripting engine for nano-installer
//!
//! Three-layer architecture: config.json > TaskRunner > scripts/*.rhai

mod api_file;
mod api_process;
mod api_registry;
mod api_shortcut;
mod api_system;
mod api_ui;
pub mod context;
pub mod engine;

pub use context::ScriptContext;
pub use engine::ScriptEngine;
