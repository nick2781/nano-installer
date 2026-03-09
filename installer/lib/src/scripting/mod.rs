//! Rhai scripting engine for nano-installer
//!
//! Three-layer architecture: config.json > TaskRunner > scripts/*.rhai

pub mod context;
pub mod engine;
mod api_file;
mod api_registry;
mod api_process;
mod api_shortcut;
mod api_ui;
mod api_system;

pub use engine::ScriptEngine;
pub use context::ScriptContext;
