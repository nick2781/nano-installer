//! UI bridge API — progress, status, cancel, checkbox values

use super::context::ScriptContext;
use rhai::Engine;

pub fn register(engine: &mut Engine, ctx: ScriptContext) {
    let c = ctx.clone();
    engine.register_fn("set_progress", move |pct: f64| {
        c.set_progress(pct as f32);
    });

    let c = ctx.clone();
    engine.register_fn("set_status", move |text: &str| {
        c.set_status(text);
    });

    let c = ctx.clone();
    engine.register_fn("is_cancelled", move || -> bool { c.is_cancelled() });

    let c = ctx.clone();
    engine.register_fn("get_install_path", move || -> String {
        c.get_install_path()
    });

    let c = ctx.clone();
    engine.register_fn("get_checkbox_value", move |id: &str| -> bool {
        c.checkbox_values.read().get(id).copied().unwrap_or(false)
    });

    engine.register_fn("log_info", |msg: &str| {
        tracing::info!("[script] {}", msg);
    });

    engine.register_fn("log_warn", |msg: &str| {
        tracing::warn!("[script] {}", msg);
    });

    engine.register_fn("log_error", |msg: &str| {
        tracing::error!("[script] {}", msg);
    });
}
