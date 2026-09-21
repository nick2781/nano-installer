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

    // True once the user asked the running task to stop. The runtime gives up
    // at the checkpoint after the step that is running, so a script that would
    // rather stop in its own time -- between two files it deploys, say -- asks
    // this and returns by itself.
    let c = context.clone();
    engine.register_fn("is_cancelled", move || c.cancellation().requested());

    let c = context.clone();
    engine.register_fn("get_install_path", move || -> String {
        c.install_path_text()
    });

    let c = context.clone();
    engine.register_fn("get_checkbox_value", move |id: &str| -> bool {
        c.checkbox(id)
    });

    // What the page held when the user started the task. A page that declares
    // neither the field nor the choice reads as empty text, so a project's
    // script can ask without its layout having to carry every control.
    let c = context.clone();
    engine.register_fn("get_text_value", move |id: &str| -> String {
        c.text_value(id)
    });

    let c = context.clone();
    engine.register_fn("get_choice_value", move |id: &str| -> String {
        c.choice_value(id)
    });

    // What the run installs. A page chooses with a checkbox that carries the
    // component's id; a component the project marked required is always there.
    let c = context.clone();
    engine.register_fn("is_component_selected", move |id: &str| -> bool {
        c.component_selected(id)
    });

    let c = context.clone();
    engine.register_fn("selected_components", move || -> rhai::Array {
        c.components()
            .iter()
            .map(|id| rhai::Dynamic::from(id.clone()))
            .collect()
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
