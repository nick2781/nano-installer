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
    ("components", &["items"]),
    ("dependencies", &["items"]),
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

/// The keys one entry of `components.items` accepts. A component is a part of what a product
/// installs, chosen on the page by a checkbox that carries the component's id.
const COMPONENT_KEYS: &[&str] = &["id", "payload", "default", "required"];

/// The keys one entry of `dependencies.items` accepts. A dependency is something the product
/// needs from the machine rather than something it installs: the setup checks for it, and
/// puts it there when it is missing.
const DEPENDENCY_KEYS: &[&str] = &[
    "id",
    "detect",
    "payload",
    "download",
    "arguments",
    "required",
];

/// The keys a dependency's `detect` rule accepts, whichever kind it is.
const DETECT_KEYS: &[&str] = &["file", "registry"];

/// The keys a registry detection rule accepts.
const REGISTRY_RULE_KEYS: &[&str] = &["key", "name", "equals", "at_least"];

/// The keys a dependency's `download` accepts.
const DOWNLOAD_KEYS: &[&str] = &["url", "sha256"];

/// How long a SHA-256 is, in hexadecimal characters.
const SHA256_LENGTH: usize = 64;

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
    // A component that named the project's own payload would travel twice in one
    // setup and could never be left out, so the audit needs to know which file that
    // is before it reads the components.
    let base_payload = object
        .get("resources")
        .and_then(|resources| resources.get("payload_file"))
        .and_then(Value::as_str)
        .unwrap_or("");
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
            match (section.as_str(), key.as_str()) {
                ("wizard", "pages" | "uninstall_pages") => audit_pages(&path, entry, &mut problems),
                ("components", "items") => {
                    audit_components(&path, entry, base_payload, &mut problems)
                }
                ("dependencies", "items") => audit_dependencies(&path, entry, &mut problems),
                _ => {}
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

fn audit_components(path: &str, value: &Value, base_payload: &str, problems: &mut Vec<String>) {
    let Some(entries) = value.as_array() else {
        problems.push(format!("{path}: must be a list of component entries"));
        return;
    };
    let mut ids: Vec<String> = Vec::new();
    let mut payloads: Vec<String> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let Some(fields) = entry.as_object() else {
            problems.push(format!("{path}[{index}]: must be a JSON object"));
            continue;
        };
        for key in fields.keys() {
            if !COMPONENT_KEYS.contains(&key.as_str()) {
                problems.push(format!(
                    "{path}[{index}].{key}: not a component setting this build knows; check the spelling, or remove it"
                ));
            }
        }
        let id = fields.get("id").and_then(Value::as_str).unwrap_or("");
        if id.trim().is_empty() {
            problems.push(format!(
                "{path}[{index}].id: a component needs the id its checkbox carries"
            ));
        } else if ids.iter().any(|seen| seen == id) {
            problems.push(format!(
                "{path}[{index}].id: {id} names a second component; a checkbox has one id, so two components cannot share one"
            ));
        } else {
            ids.push(id.to_string());
        }
        let payload = fields.get("payload").and_then(Value::as_str).unwrap_or("");
        if payload.trim().is_empty() {
            problems.push(format!(
                "{path}[{index}].payload: a component needs the payload archive it installs"
            ));
        } else if payload == base_payload {
            problems.push(format!(
                "{path}[{index}].payload: {payload} is resources.payload_file, which every install unfolds; a component needs an archive of its own"
            ));
        } else if payloads.iter().any(|seen| seen == payload) {
            problems.push(format!(
                "{path}[{index}].payload: {payload} is the payload of another component; two components that install one archive cannot be chosen apart"
            ));
        } else {
            payloads.push(payload.to_string());
        }
        let required = fields
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if required && fields.get("default").and_then(Value::as_bool) == Some(false) {
            problems.push(format!(
                "{path}[{index}].default: a required component is installed whatever the page says, so a default of false changes nothing; write true or leave it out"
            ));
        }
    }
}

fn audit_dependencies(path: &str, value: &Value, problems: &mut Vec<String>) {
    let Some(entries) = value.as_array() else {
        problems.push(format!("{path}: must be a list of dependency entries"));
        return;
    };
    let mut ids: Vec<String> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let Some(fields) = entry.as_object() else {
            problems.push(format!("{path}[{index}]: must be a JSON object"));
            continue;
        };
        let at = format!("{path}[{index}]");
        for key in fields.keys() {
            if !DEPENDENCY_KEYS.contains(&key.as_str()) {
                problems.push(format!(
                    "{at}.{key}: not a dependency setting this build knows; check the spelling, or remove it"
                ));
            }
        }
        let id = fields.get("id").and_then(Value::as_str).unwrap_or("");
        if id.trim().is_empty() {
            problems.push(format!(
                "{at}.id: a dependency needs the id a script and a log line call it by"
            ));
        } else if ids.iter().any(|seen| seen == id) {
            problems.push(format!(
                "{at}.id: {id} names a second dependency; one id names one check"
            ));
        } else {
            ids.push(id.to_string());
        }
        let payload = fields.get("payload").and_then(Value::as_str);
        match (payload, fields.get("download")) {
            (None, None) => problems.push(format!(
                "{at}: a dependency needs the payload it ships or the download it takes"
            )),
            (Some(_), Some(_)) => problems.push(format!(
                "{at}: a dependency is installed from a payload or from a download, not from both"
            )),
            _ => {}
        }
        if let Some(payload) = payload {
            // A dependency is put there by running a program, and Windows runs
            // only what it recognizes as one.
            if !payload.to_ascii_lowercase().ends_with(".exe") {
                problems.push(format!(
                    "{at}.payload: {payload} is not an executable; a dependency is installed by running one"
                ));
            }
        }
        if let Some(download) = fields.get("download") {
            audit_download(&format!("{at}.download"), download, problems);
        }
        match fields.get("detect") {
            Some(detect) => audit_detect(&format!("{at}.detect"), detect, problems),
            None => problems.push(format!(
                "{at}.detect: a dependency needs the rule that says whether the machine already has it"
            )),
        }
        if let Some(arguments) = fields.get("arguments") {
            match arguments.as_array() {
                Some(arguments) if arguments.iter().all(Value::is_string) => {}
                _ => problems.push(format!(
                    "{at}.arguments: must be a list of the arguments the installer is run with"
                )),
            }
        }
        if let Some(required) = fields.get("required") {
            if !required.is_boolean() {
                problems.push(format!(
                    "{at}.required: must be true or false; a dependency that is required stops the install when it cannot be installed"
                ));
            }
        }
    }
}

fn audit_detect(path: &str, value: &Value, problems: &mut Vec<String>) {
    let Some(fields) = value.as_object() else {
        problems.push(format!(
            "{path}: must be an object naming the file or the registry value to look at"
        ));
        return;
    };
    for key in fields.keys() {
        if !DETECT_KEYS.contains(&key.as_str()) {
            problems.push(format!(
                "{path}.{key}: not a detection this build knows; write \"file\" or \"registry\""
            ));
        }
    }
    match (fields.get("file"), fields.get("registry")) {
        (Some(_), Some(_)) => problems.push(format!(
            "{path}: names both a file and a registry value; one rule answers one question"
        )),
        (None, None) => problems.push(format!(
            "{path}: names neither a file nor a registry value, so nothing is checked"
        )),
        (Some(file), None) => {
            if file.as_str().map(str::trim).is_none_or(str::is_empty) {
                problems.push(format!(
                    "{path}.file: must be the path the rule looks for, as text"
                ));
            }
        }
        (None, Some(rule)) => audit_registry_rule(&format!("{path}.registry"), rule, problems),
    }
}

fn audit_registry_rule(path: &str, value: &Value, problems: &mut Vec<String>) {
    let Some(fields) = value.as_object() else {
        problems.push(format!(
            "{path}: must be a JSON object naming the key to read"
        ));
        return;
    };
    for key in fields.keys() {
        if !REGISTRY_RULE_KEYS.contains(&key.as_str()) {
            problems.push(format!(
                "{path}.{key}: not a registry detection this build knows; check the spelling, or remove it"
            ));
        }
    }
    let key = fields.get("key").and_then(Value::as_str).unwrap_or("");
    if key.trim().is_empty() {
        problems.push(format!(
            "{path}.key: a registry detection needs the key to open, under HKCU or HKLM"
        ));
    }
    let name = fields.get("name").and_then(Value::as_str).unwrap_or("");
    let compared = fields.get("equals").is_some() || fields.get("at_least").is_some();
    if compared && name.trim().is_empty() {
        problems.push(format!(
            "{path}.name: a rule that compares a value needs the value's name; without one the rule only asks whether the key exists"
        ));
    }
    if fields.get("equals").is_some() && fields.get("at_least").is_some() {
        problems.push(format!(
            "{path}: compares the value in two ways at once; write either \"equals\" or \"at_least\""
        ));
    }
    for key in ["equals", "at_least"] {
        if let Some(value) = fields.get(key) {
            if value.as_str().map(str::trim).is_none_or(str::is_empty) {
                problems.push(format!(
                    "{path}.{key}: must be the value to compare against, as text"
                ));
            }
        }
    }
}

fn audit_download(path: &str, value: &Value, problems: &mut Vec<String>) {
    let Some(fields) = value.as_object() else {
        problems.push(format!(
            "{path}: must be a JSON object naming the URL to fetch"
        ));
        return;
    };
    for key in fields.keys() {
        if !DOWNLOAD_KEYS.contains(&key.as_str()) {
            problems.push(format!(
                "{path}.{key}: not a download setting this build knows; check the spelling, or remove it"
            ));
        }
    }
    let url = fields.get("url").and_then(Value::as_str).unwrap_or("");
    if url.trim().is_empty() {
        problems.push(format!(
            "{path}.url: a download needs the URL to fetch, as text"
        ));
    } else {
        let scheme = url.to_ascii_lowercase();
        if !scheme.starts_with("http://") && !scheme.starts_with("https://") {
            problems.push(format!(
                "{path}.url: {url} is not an http or https URL, which is all a download can fetch"
            ));
        }
    }
    // The machine a setup runs on is not the one the project was written on, so
    // what arrives is checked: a URL without a digest would install whatever the
    // server happened to answer with.
    match fields.get("sha256") {
        Some(value) => match value.as_str() {
            Some(hash)
                if hash.len() == SHA256_LENGTH
                    && hash.chars().all(|character| character.is_ascii_hexdigit()) => {}
            Some(_) => problems.push(format!(
                "{path}.sha256: must be the {SHA256_LENGTH} hexadecimal characters of the file's SHA-256"
            )),
            None => problems.push(format!(
                "{path}.sha256: must be the file's SHA-256, as text"
            )),
        },
        None => problems.push(format!(
            "{path}.sha256: a download without the file's SHA-256 would install whatever the server answered with"
        )),
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
            "components": {
                "items": [
                    { "id": "core", "payload": "payload/core.7z", "required": true },
                    { "id": "docs", "payload": "payload/docs.7z" }
                ]
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
    fn refuses_two_components_that_share_an_id_or_a_payload() {
        let text = audit_text(&json!({
            "components": { "items": [
                { "id": "docs", "payload": "payload/docs.7z" },
                { "id": "docs", "payload": "payload/samples.7z" },
                { "id": "samples", "payload": "payload/samples.7z" }
            ] }
        }));
        assert!(
            text.contains("components.items[1].id: docs names a second component"),
            "{text}"
        );
        assert!(
            text.contains("components.items[2].payload: payload/samples.7z is the payload of another component"),
            "{text}"
        );
    }

    #[test]
    fn accepts_a_dependency_the_machine_is_asked_about() {
        audit(&json!({
            "dependencies": { "items": [
                {
                    "id": "vcredist_x64",
                    "detect": { "registry": {
                        "key": "HKLM\\SOFTWARE\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x64",
                        "name": "Installed",
                        "equals": "1"
                    } },
                    "payload": "payload/vc_redist.x64.exe",
                    "arguments": ["/install", "/quiet", "/norestart"],
                    "required": true
                },
                {
                    "id": "webview2",
                    "detect": { "file": "%ProgramFiles(x86)%\\Microsoft\\EdgeWebView\\Application\\msedgewebview2.exe" },
                    "download": {
                        "url": "https://go.microsoft.com/fwlink/?linkid=2124703",
                        "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    }
                }
            ] }
        }))
        .expect("both shapes are settings the build reads");
    }

    #[test]
    fn refuses_a_dependency_that_is_not_checked_or_not_installed() {
        let text = audit_text(&json!({
            "dependencies": { "items": [
                { "id": "vcredist", "payload": "payload/vc_redist.x64.exe" },
                { "id": "webview2", "detect": { "file": "webview2.dll" } }
            ] }
        }));
        assert!(text.contains("dependencies.items[0].detect"), "{text}");
        assert!(
            text.contains("dependencies.items[1]: a dependency needs the payload"),
            "{text}"
        );
    }

    #[test]
    fn refuses_a_download_that_does_not_say_what_should_arrive() {
        let text = audit_text(&json!({
            "dependencies": { "items": [
                {
                    "id": "webview2",
                    "detect": { "file": "webview2.dll" },
                    "download": { "url": "https://example.test/webview2.exe" }
                },
                {
                    "id": "short_hash",
                    "detect": { "file": "webview2.dll" },
                    "download": { "url": "https://example.test/webview2.exe", "sha256": "abc123" }
                },
                {
                    "id": "wrong_scheme",
                    "detect": { "file": "webview2.dll" },
                    "download": { "url": "file://C:/webview2.exe", "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" }
                }
            ] }
        }));
        assert!(
            text.contains("dependencies.items[0].download.sha256"),
            "{text}"
        );
        assert!(
            text.contains("dependencies.items[1].download.sha256"),
            "{text}"
        );
        assert!(
            text.contains("file://C:/webview2.exe is not an http or https URL"),
            "{text}"
        );
    }

    #[test]
    fn refuses_a_detection_rule_that_asks_the_wrong_thing() {
        let text = audit_text(&json!({
            "dependencies": { "items": [
                {
                    "id": "both",
                    "detect": { "file": "a.dll", "registry": { "key": "HKLM\\Software\\A" } },
                    "payload": "payload/a.exe"
                },
                {
                    "id": "compared_without_a_name",
                    "detect": { "registry": { "key": "HKLM\\Software\\A", "at_least": "1" } },
                    "payload": "payload/a.exe"
                },
                {
                    "id": "compared_twice",
                    "detect": { "registry": { "key": "HKLM\\Software\\A", "name": "v", "equals": "1", "at_least": "1" } },
                    "payload": "payload/a.exe"
                },
                {
                    "id": "misspelled",
                    "detect": { "registy": { "key": "HKLM\\Software\\A" } },
                    "payload": "payload/a.exe"
                }
            ] }
        }));
        assert!(
            text.contains("names both a file and a registry value"),
            "{text}"
        );
        assert!(
            text.contains("a rule that compares a value needs the value's name"),
            "{text}"
        );
        assert!(
            text.contains("compares the value in two ways at once"),
            "{text}"
        );
        assert!(
            text.contains("dependencies.items[3].detect.registy"),
            "{text}"
        );
    }

    #[test]
    fn refuses_a_dependency_that_cannot_be_run_or_told_apart() {
        let text = audit_text(&json!({
            "dependencies": { "items": [
                {
                    "id": "script",
                    "detect": { "file": "a.dll" },
                    "payload": "payload/install.cmd"
                },
                {
                    "id": "twice",
                    "detect": { "file": "a.dll" },
                    "payload": "payload/a.exe",
                    "download": { "url": "https://example.test/a.exe", "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" }
                },
                {
                    "id": "twice",
                    "detect": { "file": "a.dll" },
                    "payload": "payload/a.exe"
                },
                {
                    "id": "typed",
                    "detect": { "file": "a.dll" },
                    "payload": "payload/a.exe",
                    "arguments": "/quiet",
                    "required": "yes"
                }
            ] }
        }));
        assert!(
            text.contains("payload/install.cmd is not an executable"),
            "{text}"
        );
        assert!(text.contains("not from both"), "{text}");
        assert!(
            text.contains("dependencies.items[2].id: twice names a second dependency"),
            "{text}"
        );
        assert!(text.contains("dependencies.items[3].arguments"), "{text}");
        assert!(text.contains("dependencies.items[3].required"), "{text}");
    }

    #[test]
    fn refuses_a_component_without_an_id_or_a_payload() {
        let text = audit_text(&json!({
            "components": { "items": [ { "default": true } ] }
        }));
        assert!(
            text.contains("components.items[0].id: a component needs the id"),
            "{text}"
        );
        assert!(
            text.contains("components.items[0].payload: a component needs the payload"),
            "{text}"
        );
    }

    #[test]
    fn refuses_a_required_component_whose_default_is_false() {
        let text = audit_text(&json!({
            "components": { "items": [
                { "id": "core", "payload": "payload/core.7z", "required": true, "default": false }
            ] }
        }));
        assert!(
            text.contains("components.items[0].default: a required component is installed whatever the page says"),
            "{text}"
        );
    }

    #[test]
    fn refuses_an_unknown_component_key() {
        let text = audit_text(&json!({
            "components": { "items": [ { "id": "docs", "payload": "payload/docs.7z", "optional": true } ] }
        }));
        assert!(
            text.contains("components.items[0].optional: not a component setting this build knows"),
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
