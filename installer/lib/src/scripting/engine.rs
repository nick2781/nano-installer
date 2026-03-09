//! Rhai script engine — creates Engine, registers all API functions, runs scripts

use rhai::{Engine, Scope, Dynamic, EvalAltResult};
use super::context::ScriptContext;

/// Script engine wrapping Rhai with nano-installer API
pub struct ScriptEngine {
    engine: Engine,
    scope: Scope<'static>,
    context: ScriptContext,
}

impl ScriptEngine {
    /// Create a new script engine with all API functions registered
    pub fn new(context: ScriptContext) -> Self {
        let mut engine = Engine::new();

        // Limit script execution to prevent infinite loops
        engine.set_max_operations(10_000_000);

        // Register all API modules
        super::api_ui::register(&mut engine, context.clone());
        super::api_file::register(&mut engine, context.clone());
        super::api_registry::register(&mut engine, context.clone());
        super::api_process::register(&mut engine, context.clone());
        super::api_shortcut::register(&mut engine, context.clone());
        super::api_system::register(&mut engine, context.clone());

        let scope = Scope::new();

        Self { engine, scope, context }
    }

    /// Run a script from source code
    pub fn run_script(&mut self, source: &str) -> Result<(), String> {
        tracing::info!("Running Rhai script ({} bytes)", source.len());

        self.engine
            .run_with_scope(&mut self.scope, source)
            .map_err(|e| {
                let msg = format!("Script error: {}", e);
                tracing::error!("{}", msg);
                msg
            })
    }

    /// Evaluate a script and return a value
    pub fn eval_script<T: Clone + Send + Sync + 'static>(&mut self, source: &str) -> Result<T, String> {
        self.engine
            .eval_with_scope::<T>(&mut self.scope, source)
            .map_err(|e| format!("Script error: {}", e))
    }
}
