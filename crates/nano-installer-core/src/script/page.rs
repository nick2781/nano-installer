//! The project's page hook: `scripts/pages.rhai`.
//!
//! A wizard walks the pages its project declares. A project that wants to
//! decide that order while the wizard runs -- skip the licence page once it has
//! been accepted, or send the user to a page only a choice on the page before
//! asks for -- ships one function in `scripts/pages.rhai`:
//!
//! ```rhai
//! fn next_page(from) {
//!     if from == "options" && !get_checkbox_value("expert") {
//!         return "tasks";   // the components page is for the expert path
//!     }
//!     ""                    // an empty answer keeps the declared order
//! }
//! ```
//!
//! The runtime asks it every time the user leaves a page forward, and the
//! wizard walks back along the pages it actually visited, so a page the hook
//! skipped is never shown by a back button either.
//!
//! A hook reads the values the wizard holds and the machine it runs on; it
//! cannot write, because none of the primitives that change something is
//! registered against it. Where the wizard goes is the whole of what a page
//! hook decides.

use anyhow::{anyhow, bail, Result};
use rhai::{Dynamic, Engine, ImmutableString, Scope};

use super::api_file;
use super::api_registry;
use super::api_system;
use super::api_ui;
use super::context::{log_tail, reset_log, PageEnvironment, ScriptContext};

/// The entry point a project ships when it wants to decide the page order.
pub(crate) const PAGE_SCRIPT: &str = "scripts/pages.rhai";

/// The function a project's page hook is asked through.
const PAGE_HOOK: &str = "next_page";

/// A ceiling on the work one navigation decision may do.
///
/// A hook answers a question about a page, and the click that asks it is handled
/// on the thread that draws the window: a project that loops here would freeze
/// its own wizard, so the ceiling is far below the one a task script gets.
const MAX_PAGE_OPERATIONS: u64 = 1_000_000;

/// What the wizard was holding when it asked where to go next.
pub(crate) struct PageRequest {
    /// The values the hook may read.
    pub(crate) environment: PageEnvironment,
    /// The id of the page the user is leaving, empty for a page whose project
    /// gave it no id.
    pub(crate) from: String,
    /// The source of `scripts/pages.rhai`.
    pub(crate) hook: String,
}

/// The page the project's hook sends the wizard to, if it names one.
///
/// `None` means the hook has nothing to say about this move and the order the
/// project declares stands. A hook that fails, or that answers with something
/// other than a page id, comes back as the error the wizard reports.
pub(crate) fn next_page(request: PageRequest) -> Result<Option<String>> {
    let context = ScriptContext::for_page(request.environment);
    let mut engine = Engine::new();
    engine.set_max_operations(MAX_PAGE_OPERATIONS);
    api_ui::register_queries(&mut engine, context.clone());
    api_system::register_queries(&mut engine, context.clone());
    api_registry::register_queries(&mut engine);
    api_file::register_queries(&mut engine);
    let ast = engine
        .compile(&request.hook)
        .map_err(|error| anyhow!("{PAGE_SCRIPT} does not parse: {error}"))?;
    // A project that ships the file for reasons of its own, or that is halfway
    // through writing one, keeps the order it declares.
    if !ast
        .iter_functions()
        .any(|function| function.name == PAGE_HOOK)
    {
        return Ok(None);
    }
    let mut scope = Scope::new();
    reset_log();
    let chosen: Dynamic = engine
        .call_fn(&mut scope, &ast, PAGE_HOOK, (request.from.clone(),))
        .map_err(|error| {
            let tail = log_tail();
            if tail.is_empty() {
                anyhow!("{PAGE_SCRIPT} failed: {error}")
            } else {
                anyhow!("{PAGE_SCRIPT} failed: {error}\n{tail}")
            }
        })?;
    let Some(chosen) = chosen.try_cast::<ImmutableString>() else {
        bail!("{PAGE_HOOK} returns the id of a page as text, or \"\" to keep the declared order")
    };
    let chosen = chosen.trim().to_string();
    Ok((!chosen.is_empty()).then_some(chosen))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A request whose hook may read one checkbox.
    fn request(hook: &str, from: &str, expert: bool) -> PageRequest {
        PageRequest {
            environment: PageEnvironment {
                mode: super::super::Mode::Install,
                config: serde_json::Value::Null,
                install_path: std::path::PathBuf::new(),
                checkboxes: HashMap::from([("expert".to_string(), expert)]),
                texts: HashMap::new(),
                choices: HashMap::new(),
                components: Vec::new(),
            },
            from: from.to_string(),
            hook: hook.to_string(),
        }
    }

    /// A hook decides the page the wizard goes to, and a quiet answer leaves the
    /// question to the declared order.
    #[test]
    fn a_hook_names_the_page_the_wizard_goes_to() -> Result<()> {
        let hook = r#"
            fn next_page(from) {
                if from == "options" && get_checkbox_value("expert") { "components" } else { "" }
            }
        "#;
        assert_eq!(
            next_page(request(hook, "options", true))?,
            Some("components".to_string())
        );
        assert_eq!(next_page(request(hook, "options", false))?, None);
        // A page the hook says nothing about keeps the declared order too.
        assert_eq!(next_page(request(hook, "welcome", true))?, None);
        Ok(())
    }

    /// The values a hook reads are the ones the wizard holds, and the machine
    /// it can ask about is the one the machine answers with.
    #[test]
    fn a_hook_reads_the_wizard_and_the_machine() -> Result<()> {
        let hook = r#"
            fn next_page(from) {
                log_info("asked about " + from);
                if get_mode() != "install" { "" }
                else if file_exists(get_install_path()) { "somewhere" }
                else { "nowhere" }
            }
        "#;
        // An empty install path is a directory that does not exist, so the hook
        // answers from the value it was handed rather than from a constant.
        assert_eq!(
            next_page(request(hook, "config", false))?,
            Some("nowhere".to_string())
        );
        Ok(())
    }

    /// A hook that fails says so, and the project's own messages travel with the
    /// failure the way a task script's do.
    #[test]
    fn a_hook_that_fails_reports_the_project_message() {
        let hook = r#"
            fn next_page(from) {
                log_error("the licence page has no text yet");
                throw "no page to go to";
            }
        "#;
        let error = next_page(request(hook, "options", false)).expect_err("the hook throws");
        let text = format!("{error:#}");
        assert!(
            text.contains("scripts/pages.rhai failed:"),
            "the failure does not name the hook: {text}"
        );
        assert!(
            text.contains(&"error: the licence page has no text yet".to_string()),
            "the project's own message is missing: {text}"
        );
    }

    /// A hook that answers with something other than a page id is refused rather
    /// than read as a page named `true`.
    #[test]
    fn a_hook_must_answer_with_page_text() {
        let hook = "fn next_page(from) { true }";
        let error = next_page(request(hook, "options", false)).expect_err("a flag is not a page");
        assert!(
            format!("{error:#}").contains("returns the id of a page as text"),
            "the wrong answer is not explained: {error:#}"
        );
    }

    /// A script without the hook -- or one that does not parse -- leaves the
    /// wizard the order the project declares.
    #[test]
    fn a_script_without_the_hook_says_nothing() -> Result<()> {
        assert_eq!(
            next_page(request("fn unrelated() {}", "options", false))?,
            None
        );
        let error = next_page(request("fn next_page(", "options", false))
            .expect_err("a broken file is reported");
        assert!(
            format!("{error:#}").contains("does not parse"),
            "a broken hook is not explained: {error:#}"
        );
        Ok(())
    }
}
