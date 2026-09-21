//! The settings a project may write, and the ones that only look like settings.
//!
//! A key the builder quietly ignores is worse than a feature that is missing: the author
//! believes the setting took effect and ships a setup that behaves differently from the
//! configuration in front of them. Inside the sections below, which belong to the build, every
//! key is therefore either read or refused. A section this build does not know is left alone: a
//! script reads it back through get_config_value, so a project keeps its own settings there.

use anyhow::{bail, Result};
use serde_json::Value;

/// The keys each section accepts today.
const SECTIONS: &[(&str, &[&str])] = &[
    (
        "project",
        &[
            "name",
            "version",
            "file_version",
            "description",
            "publisher",
            "copyright",
            "output_name",
        ],
    ),
    (
        "output",
        &[
            "installer_name",
            "installer_icon",
            "uninstaller_name",
            "uninstaller_icon",
        ],
    ),
    (
        "install",
        &[
            "default_path",
            "required_space_mb",
            "exe_name",
            "require_admin",
            "kill_process_on_install",
            "kill_process_on_uninstall",
            "detect_running_process",
        ],
    ),
    ("registry", &["uninstall_key"]),
    (
        "shortcuts",
        &[
            "desktop_shortcut",
            "desktop_default",
            "start_menu",
            "start_menu_folder",
        ],
    ),
    (
        "autostart",
        &["enabled", "default", "registry_key", "registry_value_name"],
    ),
    (
        "resources",
        &[
            "layouts_dir",
            "assets_dir",
            "locales_dir",
            "payload_file",
            "uninstaller_icon",
            "tools_dir",
        ],
    ),
    ("localization", &["default_locale", "supported_locales"]),
    ("ui", &["dpi_aware", "dpi_threshold", "dialog_layout"]),
    ("wizard", &["pages", "uninstall_pages"]),
    ("uninstall", &["data_paths"]),
    (
        "advanced",
        &["silent_mode_support", "uninstall_mode_support"],
    ),
];

/// Tables whose keys the project itself chooses: `links` names its own URLs, and a layout opens
/// them through `action="open_url:<key>"`.
const FREE_FORM_SECTIONS: &[&str] = &["links"];

/// The keys one entry of `wizard.pages` or `wizard.uninstall_pages` accepts.
const PAGE_KEYS: &[&str] = &["id", "title", "layout", "role"];

/// The two jobs a page can name, so a project can order its pages freely.
const PAGE_ROLES: &[&str] = &["progress", "finish"];

/// A section that parses and is read by nothing.
const INACTIVE_SECTIONS: &[(&str, &str)] = &[(
    "validation",
    "no build or runtime step reads a validation block; install.required_space_mb is the one \
     check that runs, and it runs on its own",
)];

/// A key that parses and changes nothing, with the setting that does the job instead.
const INACTIVE_KEYS: &[(&str, &str)] = &[
    (
        "install.append_to_path",
        "no setting adds a directory to PATH",
    ),
    (
        "install.mutex_name",
        "a setup does not refuse to run while another copy installs the same product",
    ),
    (
        "registry.install_path_key",
        "the install path is not written into a registry value",
    ),
    (
        "registry.help_link",
        "put the URL under links and open it with action=\"open_url:<key>\"",
    ),
    ("resources.installer_icon", "use output.installer_icon"),
    (
        "output.installer_stub",
        "the runtimes come from the build's --stubs directory, not from the project",
    ),
    (
        "output.uninstaller_stub",
        "the runtimes come from the build's --stubs directory, not from the project",
    ),
    ("wizard.update_pages", "an upgrade replays wizard.pages"),
    (
        "advanced.update_mode_support",
        "an upgrade is decided by what is already installed at the destination",
    ),
    (
        "advanced.launch_app_after_install",
        "put action=\"launch_app\" on the finish page",
    ),
    (
        "localization.show_language_selector",
        "the language list comes from the Select control in the layout",
    ),
    (
        "ui.window_width",
        "the window size comes from <Page width height> in the layout",
    ),
    (
        "ui.window_height",
        "the window size comes from <Page width height> in the layout",
    ),
    (
        "ui.expanded_height",
        "the window size comes from <Page width height> in the layout",
    ),
    (
        "ui.window_corner_radius",
        "the corner radius comes from <Page border-radius> in the layout",
    ),
];

/// Rejects a configuration holding a key that this build does not read.
pub(super) fn audit(config: &Value) -> Result<()> {
    let Some(object) = config.as_object() else {
        bail!("installer_config.json: the configuration must be a JSON object")
    };
    let mut problems: Vec<String> = Vec::new();
    for (section, value) in object {
        if FREE_FORM_SECTIONS.contains(&section.as_str()) {
            continue;
        }
        let Some(keys) = SECTIONS
            .iter()
            .find(|(name, _)| name == section)
            .map(|(_, keys)| *keys)
        else {
            // A section of the project's own is left alone: a script reads it back with
            // get_config_value, which is how a project carries settings this build has no
            // opinion about.
            if let Some(reason) = inactive_section_reason(section) {
                problems.push(format!("{section}: {reason}; remove it"));
            }
            continue;
        };
        let Some(fields) = value.as_object() else {
            problems.push(format!("{section}: must be a JSON object"));
            continue;
        };
        for (key, entry) in fields {
            let path = format!("{section}.{key}");
            if let Some(reason) = inactive_key_reason(&path) {
                problems.push(format!("{path}: {reason}; remove it"));
            } else if !keys.contains(&key.as_str()) {
                problems.push(format!(
                    "{path}: not a setting this build knows; check the spelling, or remove it"
                ));
            }
            if section == "wizard" && matches!(key.as_str(), "pages" | "uninstall_pages") {
                audit_pages(&path, entry, &mut problems);
            }
        }
    }
    if problems.is_empty() {
        return Ok(());
    }
    problems.sort();
    bail!(
        "installer_config.json holds settings this build does not read:\n  {}\nRemove each one, or write the setting its message names.",
        problems.join("\n  ")
    )
}

fn audit_pages(path: &str, value: &Value, problems: &mut Vec<String>) {
    let Some(entries) = value.as_array() else {
        problems.push(format!("{path}: must be a list of page entries"));
        return;
    };
    for (index, entry) in entries.iter().enumerate() {
        let Some(fields) = entry.as_object() else {
            problems.push(format!("{path}[{index}]: must be a JSON object"));
            continue;
        };
        for key in fields.keys() {
            if !PAGE_KEYS.contains(&key.as_str()) {
                problems.push(format!(
                    "{path}[{index}].{key}: not a page setting this build knows; check the spelling, or remove it"
                ));
            }
        }
        if let Some(role) = fields.get("role").and_then(Value::as_str) {
            if !PAGE_ROLES.contains(&role) {
                problems.push(format!(
                    "{path}[{index}].role: {role} is not a page role; use \"progress\" or \"finish\""
                ));
            }
        }
    }
    // The runtime runs one progress page and ends on one finish page, so a
    // second page claiming either job has no meaning to give it.
    for role in PAGE_ROLES {
        let claimed: Vec<String> = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry["role"].as_str() == Some(*role))
            .map(|(index, _)| index.to_string())
            .collect();
        if claimed.len() > 1 {
            problems.push(format!(
                "{path}: the pages {} all claim the {role} role; one page carries it",
                claimed.join(", ")
            ));
        }
    }
}

fn inactive_section_reason(section: &str) -> Option<&'static str> {
    INACTIVE_SECTIONS
        .iter()
        .find(|(name, _)| *name == section)
        .map(|(_, reason)| *reason)
}

fn inactive_key_reason(path: &str) -> Option<&'static str> {
    INACTIVE_KEYS
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, reason)| *reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn audit_text(config: &Value) -> String {
        format!("{}", audit(config).expect_err("the audit must reject this"))
    }

    #[test]
    fn accepts_a_configuration_of_read_settings() {
        audit(&json!({
            "project": { "name": "MyApp", "version": "2026.9.22", "publisher": "Example" },
            "output": { "installer_name": "MyApp_Setup.exe" },
            "install": { "default_path": "%LOCALAPPDATA%\\MyApp", "required_space_mb": 200 },
            "registry": { "uninstall_key": "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\MyApp" },
            "shortcuts": { "desktop_shortcut": true },
            "autostart": { "enabled": false },
            "resources": { "payload_file": "payload/app.7z", "uninstaller_icon": "assets/uninst.ico" },
            "localization": { "default_locale": "zh-CN", "supported_locales": ["zh-CN"] },
            "links": { "terms_of_service": "https://example.test/terms", "anything": "https://example.test" },
            "ui": { "dpi_aware": true, "dpi_threshold": 144, "dialog_layout": "layouts/msgBox.xml" },
            "wizard": {
                "pages": [ { "id": "config", "title": "Options", "layout": "layouts/configpage.xml" } ],
                "uninstall_pages": [ { "layout": "layouts/uninstallpage.xml" } ]
            },
            "uninstall": { "data_paths": ["%APPDATA%\\MyApp"] },
            "advanced": { "silent_mode_support": true, "uninstall_mode_support": true }
        }))
        .expect("every key here is read by the build");
    }

    #[test]
    fn the_example_project_matches_the_schema() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/TapTap/installer_config.json");
        let data = std::fs::read(&path).expect("the example project ships a configuration");
        let config: Value =
            serde_json::from_slice(&data).expect("the example configuration parses");
        // The example is what a project copies from, so it may not carry a key the build
        // ignores: the payload is not needed to check that.
        audit(&config).expect("the example configuration holds settings the build reads");
    }

    #[test]
    fn refuses_a_setting_that_does_nothing() {
        let text = audit_text(&json!({ "install": { "append_to_path": "MyApp" } }));
        assert!(text.contains("install.append_to_path"), "{text}");
        assert!(
            text.contains("no setting adds a directory to PATH"),
            "{text}"
        );
    }

    #[test]
    fn refuses_a_misspelled_setting() {
        let text = audit_text(&json!({ "install": { "exe_nmae": "MyApp.exe" } }));
        assert!(
            text.contains("install.exe_nmae: not a setting this build knows"),
            "{text}"
        );
    }

    #[test]
    fn refuses_a_section_that_does_nothing() {
        let text = audit_text(&json!({ "validation": { "check_disk_space": true } }));
        assert!(text.contains("validation"), "{text}");
        assert!(
            text.contains("install.required_space_mb is the one check that runs"),
            "{text}"
        );
    }

    #[test]
    fn refuses_a_page_role_the_runtime_does_not_run() {
        let text = audit_text(&json!({
            "wizard": { "pages": [ { "layout": "layouts/a.xml", "role": "license" } ] }
        }));
        assert!(
            text.contains("wizard.pages[0].role: license is not a page role"),
            "{text}"
        );
    }

    #[test]
    fn refuses_two_pages_claiming_one_role() {
        let text = audit_text(&json!({
            "wizard": { "pages": [
                { "layout": "layouts/a.xml", "role": "progress" },
                { "layout": "layouts/b.xml", "role": "progress" }
            ] }
        }));
        assert!(
            text.contains("wizard.pages: the pages 0, 1 all claim the progress role"),
            "{text}"
        );
    }

    #[test]
    fn refuses_an_unknown_page_key() {
        let text = audit_text(&json!({
            "wizard": { "pages": [ { "layout": "layouts/a.xml", "layouts": "layouts/b.xml" } ] }
        }));
        assert!(
            text.contains("wizard.pages[0].layouts: not a page setting this build knows"),
            "{text}"
        );
    }

    #[test]
    fn reports_every_problem_at_once() {
        let text = audit_text(&json!({
            "install": { "mutex_name": "MyApp_Setup", "exe_nmae": "MyApp.exe" },
            "ui": { "window_width": 720 }
        }));
        assert!(text.contains("install.mutex_name"), "{text}");
        assert!(text.contains("install.exe_nmae"), "{text}");
        assert!(text.contains("ui.window_width"), "{text}");
    }

    #[test]
    fn leaves_a_section_of_the_projects_own_alone() {
        audit(&json!({
            "project": { "name": "MyApp" },
            "test": { "registry_key": "HKCU\\\\Software\\\\E2e" },
            "my_company": { "channel": "beta" }
        }))
        .expect("a project may carry settings of its own");
    }
}
