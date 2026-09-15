//! Wizard bridge: progress, step text, cancellation, and checkbox state.

use rhai::Engine;

use super::context::{log, ScriptContext};

pub(super) fn register(engine: &mut Engine, context: ScriptContext) {
    let c = context.clone();
    engine.register_fn("set_progress", move |percent: f64| c.progress(percent));

    let c = context.clone();
    engine.register_fn("set_status", move |text: &str| c.status(text));

    let c = context.clone();
    engine.register_fn("set_status_key", move |key: &str| c.status_key(key));

    // The wizard refuses to close while a task runs, so the user has no way to
    // cancel mid-step. The primitive stays for scripts written against it.
    engine.register_fn("is_cancelled", || -> bool { false });

    let c = context.clone();
    engine.register_fn("get_install_path", move || -> String {
        c.install_path_text()
    });

    let c = context.clone();
    engine.register_fn("get_checkbox_value", move |id: &str| -> bool {
        c.checkbox(id)
    });

    let c = context.clone();
    engine.register_fn("get_mode", move || -> String {
        match c.mode() {
            super::Mode::Install => "install".to_string(),
            super::Mode::Uninstall => "uninstall".to_string(),
        }
    });

    engine.register_fn("log_info", |message: &str| log("info", message));
    engine.register_fn("log_warn", |message: &str| log("warn", message));
    engine.register_fn("log_error", |message: &str| log("error", message));
}
