//! Rhai script engine — creates Engine, registers all API functions, runs scripts

use super::context::ScriptContext;
use rhai::{Dynamic, Engine, EvalAltResult, Scope};

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

        Self {
            engine,
            scope,
            context,
        }
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
    pub fn eval_script<T: Clone + Send + Sync + 'static>(
        &mut self,
        source: &str,
    ) -> Result<T, String> {
        self.engine
            .eval_with_scope::<T>(&mut self.scope, source)
            .map_err(|e| format!("Script error: {}", e))
    }

    pub fn context(&self) -> &ScriptContext {
        &self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::InstallerConfig;
    use crate::resources::manifest::UninstallManifest;
    use parking_lot::RwLock;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn script_install_records_manifest_artifacts() {
        let temp = tempdir().unwrap();
        let install_dir = temp.path().join("DemoApp");
        std::fs::create_dir_all(&install_dir).unwrap();

        let mut config = InstallerConfig::default();
        config.project.name = "DemoApp".to_string();
        config.project.version = "1.0.0".to_string();
        config.project.publisher = "Test Publisher".to_string();

        let state =
            crate::installer::state::InstallState::new(install_dir.to_string_lossy().to_string());
        let ctx = ScriptContext::for_install(state, config);
        let mut engine = ScriptEngine::new(ctx);

        engine
            .run_script(
                r#"
                let logs = path_join(get_install_path(), "logs");
                create_dir(logs);
                write_file(path_join(get_install_path(), "app.txt"), "demo");
            "#,
            )
            .unwrap();
        engine.context().finalize_install().unwrap();

        let manifest = UninstallManifest::load(&install_dir.join("uninstall.json")).unwrap();
        assert!(manifest
            .files_to_remove
            .iter()
            .any(|p| p.ends_with("app.txt")));
        assert!(manifest
            .directories_to_remove
            .iter()
            .any(|p| p.ends_with("logs")));
    }

    #[test]
    fn script_uninstall_can_run_manifest_cleanup() {
        let temp = tempdir().unwrap();
        let install_dir = temp.path().join("TrackedApp");
        std::fs::create_dir_all(&install_dir).unwrap();

        let tracked_file = install_dir.join("tracked.txt");
        std::fs::write(&tracked_file, b"demo").unwrap();

        let manifest = UninstallManifest {
            version: "1.0.0".to_string(),
            product_name: "CodexTrackedUninstallTest".to_string(),
            publisher: "Test Publisher".to_string(),
            install_path: install_dir.to_string_lossy().to_string(),
            locale: "en-US".to_string(),
            files_to_remove: vec![tracked_file.to_string_lossy().to_string()],
            directories_to_remove: vec![],
            registry_keys_to_remove: vec![],
            registry_values_to_remove: vec![],
            shortcuts_to_remove: vec![],
            close_targets: vec![],
        };
        manifest.save(&install_dir.join("uninstall.json")).unwrap();

        let progress = Arc::new(RwLock::new(0.0));
        let status = Arc::new(RwLock::new(String::new()));
        let finished = Arc::new(AtomicBool::new(false));

        let mut config = InstallerConfig::default();
        config.project.name = "TrackedApp".to_string();
        let ctx = ScriptContext::for_uninstall(
            config,
            install_dir.to_string_lossy().to_string(),
            progress.clone(),
            status,
            finished,
            false,
        );
        let mut engine = ScriptEngine::new(ctx);

        engine
            .run_script(
                r#"
                if !run_tracked_uninstall(10.0, 90.0) {
                    throw("tracked uninstall failed");
                }
            "#,
            )
            .unwrap();

        assert!(!tracked_file.exists());
        assert!(*progress.read() >= 0.9);
    }

    #[test]
    fn script_uninstall_context_can_implicitly_run_manifest_cleanup() {
        let temp = tempdir().unwrap();
        let install_dir = temp.path().join("ImplicitTrackedApp");
        std::fs::create_dir_all(&install_dir).unwrap();

        let tracked_file = install_dir.join("tracked.txt");
        std::fs::write(&tracked_file, b"demo").unwrap();

        let manifest = UninstallManifest {
            version: "1.0.0".to_string(),
            product_name: "ImplicitTrackedApp".to_string(),
            publisher: "Test Publisher".to_string(),
            install_path: install_dir.to_string_lossy().to_string(),
            locale: "en-US".to_string(),
            files_to_remove: vec![tracked_file.to_string_lossy().to_string()],
            directories_to_remove: vec![],
            registry_keys_to_remove: vec![],
            registry_values_to_remove: vec![],
            shortcuts_to_remove: vec![],
            close_targets: vec![],
        };
        manifest.save(&install_dir.join("uninstall.json")).unwrap();

        let progress = Arc::new(RwLock::new(0.0));
        let status = Arc::new(RwLock::new(String::new()));
        let finished = Arc::new(AtomicBool::new(false));

        let config = InstallerConfig::default();
        let ctx = ScriptContext::for_uninstall(
            config,
            install_dir.to_string_lossy().to_string(),
            progress.clone(),
            status,
            finished,
            false,
        );
        let mut engine = ScriptEngine::new(ctx);

        engine
            .run_script(r#"let note = "legacy uninstall script";"#)
            .unwrap();
        assert!(!engine.context().tracked_uninstall_invoked());
        assert!(engine.context().run_manifest_uninstall(10.0, 90.0).unwrap());

        assert!(!tracked_file.exists());
        assert!(*progress.read() >= 0.9);
    }
}
