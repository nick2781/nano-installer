use anyhow::{Context, Result};
use image::RgbaImage;
use regex::Regex;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceLintSeverity {
    Error,
    Warning,
}

impl fmt::Display for ResourceLintSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => f.write_str("error"),
            Self::Warning => f.write_str("warning"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceLintIssue {
    pub severity: ResourceLintSeverity,
    pub code: String,
    pub asset: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceLintReport {
    pub project: String,
    pub referenced_assets: usize,
    pub checked_assets: usize,
    pub issues: Vec<ResourceLintIssue>,
}

impl ResourceLintReport {
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == ResourceLintSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == ResourceLintSeverity::Warning)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }
}

#[derive(Debug, Clone, Copy)]
struct AlphaBounds {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

#[derive(Debug, Clone)]
struct ProjectResourceDirs {
    layouts_dir: PathBuf,
    assets_prefix: String,
}

pub fn lint_project_resources(project_dir: &Path) -> Result<ResourceLintReport> {
    let referenced_assets = collect_referenced_assets(project_dir)?;
    let mut report = ResourceLintReport {
        project: project_dir.display().to_string(),
        referenced_assets: referenced_assets.len(),
        checked_assets: referenced_assets.len(),
        issues: Vec::new(),
    };

    for asset in &referenced_assets {
        lint_single_asset(project_dir, asset, &mut report)?;
    }

    report.issues.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then_with(|| left.asset.cmp(&right.asset))
            .then_with(|| left.code.cmp(&right.code))
    });
    Ok(report)
}

pub fn render_resource_lint_text(report: &ResourceLintReport) -> String {
    let mut lines = vec![
        format!("project: {}", report.project),
        format!("referenced_assets: {}", report.referenced_assets),
        format!("checked_assets: {}", report.checked_assets),
        format!("errors: {}", report.error_count()),
        format!("warnings: {}", report.warning_count()),
    ];

    if report.issues.is_empty() {
        lines.push("issues: <none>".to_string());
    } else {
        lines.push("issues:".to_string());
        for issue in &report.issues {
            lines.push(format!(
                "  [{}] {} {} - {}",
                issue.severity, issue.asset, issue.code, issue.message
            ));
        }
    }

    lines.join("\n")
}

fn collect_referenced_assets(project_dir: &Path) -> Result<BTreeSet<String>> {
    let resource_dirs = load_resource_dirs(project_dir)?;
    let layouts_dir = resource_dirs.layouts_dir;
    let assets_prefix = resource_dirs.assets_prefix;
    let mut assets = BTreeSet::new();

    if layouts_dir.exists() {
        for entry in WalkDir::new(&layouts_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "xml"))
        {
            let content = std::fs::read_to_string(entry.path())
                .with_context(|| format!("failed to read {}", entry.path().display()))?;
            let doc = roxmltree::Document::parse(&content)
                .with_context(|| format!("failed to parse {}", entry.path().display()))?;

            for node in doc.descendants().filter(|node| node.is_element()) {
                for attr in node.attributes() {
                    for asset in extract_asset_paths(attr.value()) {
                        if asset == assets_prefix
                            || asset.starts_with(&format!("{}/", assets_prefix))
                        {
                            assets.insert(asset);
                        }
                    }
                }
            }
        }
    }

    let config_path = project_dir.join("installer_config.json");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;
        let json: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", config_path.display()))?;
        for key in [
            "/output/installer_icon",
            "/output/uninstaller_icon",
            "/resources/installer_icon",
            "/resources/uninstaller_icon",
        ] {
            if let Some(asset) = json.pointer(key).and_then(|value| value.as_str()) {
                let asset = asset.replace('\\', "/");
                if asset == assets_prefix || asset.starts_with(&format!("{}/", assets_prefix)) {
                    assets.insert(asset);
                }
            }
        }
    }

    Ok(assets)
}

fn load_resource_dirs(project_dir: &Path) -> Result<ProjectResourceDirs> {
    let config_path = project_dir.join("installer_config.json");
    let default = ProjectResourceDirs {
        layouts_dir: project_dir.join("layouts"),
        assets_prefix: "assets".to_string(),
    };

    if !config_path.exists() {
        return Ok(default);
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;

    let layouts_rel = json
        .pointer("/resources/layouts_dir")
        .and_then(|v| v.as_str());
    let assets_rel = json
        .pointer("/resources/assets_dir")
        .and_then(|v| v.as_str());

    Ok(ProjectResourceDirs {
        layouts_dir: project_dir.join(layouts_rel.unwrap_or("layouts")),
        assets_prefix: assets_rel.unwrap_or("assets").replace('\\', "/"),
    })
}

fn extract_asset_paths(raw: &str) -> Vec<String> {
    let file_regex =
        Regex::new(r#"file\s*=\s*['"](?P<path>[^'"]+\.(?i:png|jpe?g|ico))['"]"#).unwrap();
    let plain_regex = Regex::new(r#"(?P<path>[A-Za-z0-9_./\\-]+\.(?i:png|jpe?g|ico))"#).unwrap();
    let mut results = BTreeSet::new();

    for capture in file_regex.captures_iter(raw) {
        if let Some(path) = capture.name("path") {
            results.insert(path.as_str().replace('\\', "/"));
        }
    }

    for capture in plain_regex.captures_iter(raw) {
        if let Some(path) = capture.name("path") {
            results.insert(path.as_str().replace('\\', "/"));
        }
    }

    results.into_iter().collect()
}

fn lint_single_asset(
    project_dir: &Path,
    asset: &str,
    report: &mut ResourceLintReport,
) -> Result<()> {
    let asset_path = project_dir.join(asset);
    if !asset_path.exists() {
        push_issue(
            report,
            ResourceLintSeverity::Error,
            "missing-asset",
            asset,
            format!("referenced asset is missing: {}", asset_path.display()),
        );
        return Ok(());
    }

    let ext = asset_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg") {
        return Ok(());
    }

    let image = image::open(&asset_path)
        .with_context(|| format!("failed to decode image asset: {}", asset_path.display()))?
        .to_rgba8();
    let (width, height) = image.dimensions();

    if width == 0 || height == 0 {
        push_issue(
            report,
            ResourceLintSeverity::Error,
            "empty-image",
            asset,
            "image has zero width or height".to_string(),
        );
        return Ok(());
    }

    let alpha_bounds = compute_alpha_bounds(&image);
    if alpha_bounds.is_none() {
        push_issue(
            report,
            ResourceLintSeverity::Error,
            "fully-transparent",
            asset,
            "image does not contain any visible pixels".to_string(),
        );
        return Ok(());
    }

    if !asset.contains("@2x.") {
        lint_retina_pair(project_dir, asset, width, height, report)?;
    }

    if width <= 32 && height <= 32 {
        if let Some(bounds) = alpha_bounds {
            let left_padding = bounds.left;
            let right_padding = width.saturating_sub(bounds.right + 1);
            let top_padding = bounds.top;
            let bottom_padding = height.saturating_sub(bounds.bottom + 1);
            let horizontal_delta = left_padding.abs_diff(right_padding);
            let vertical_delta = top_padding.abs_diff(bottom_padding);
            if horizontal_delta >= 2 || vertical_delta >= 2 {
                push_issue(
                    report,
                    ResourceLintSeverity::Warning,
                    "alpha-padding-offset",
                    asset,
                    format!(
                        "visible pixels are not optically centered (left={}, right={}, top={}, bottom={})",
                        left_padding, right_padding, top_padding, bottom_padding
                    ),
                );
            }
        }
    }

    Ok(())
}

fn lint_retina_pair(
    project_dir: &Path,
    asset: &str,
    width: u32,
    height: u32,
    report: &mut ResourceLintReport,
) -> Result<()> {
    let retina_rel = retina_variant(asset);
    let retina_path = project_dir.join(&retina_rel);
    if !retina_path.exists() {
        push_issue(
            report,
            ResourceLintSeverity::Warning,
            "missing-2x",
            asset,
            format!("missing retina counterpart: {}", retina_rel),
        );
        return Ok(());
    }

    let retina = image::open(&retina_path)
        .with_context(|| format!("failed to decode retina asset: {}", retina_path.display()))?
        .to_rgba8();
    let (retina_width, retina_height) = retina.dimensions();
    if retina_width != width * 2 || retina_height != height * 2 {
        push_issue(
            report,
            ResourceLintSeverity::Error,
            "invalid-2x-size",
            asset,
            format!(
                "{} is {}x{}, expected {}x{}",
                retina_rel,
                retina_width,
                retina_height,
                width * 2,
                height * 2
            ),
        );
    }
    Ok(())
}

fn retina_variant(asset: &str) -> String {
    let path = Path::new(asset);
    let parent = path
        .parent()
        .map(|path| path.to_string_lossy().replace('\\', "/"));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(asset);
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("png");
    match parent {
        Some(parent) if !parent.is_empty() => format!("{}/{}@2x.{}", parent, stem, ext),
        _ => format!("{}@2x.{}", stem, ext),
    }
}

fn compute_alpha_bounds(image: &RgbaImage) -> Option<AlphaBounds> {
    let (width, height) = image.dimensions();
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut has_visible_pixel = false;

    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel.0[3] > 0 {
            has_visible_pixel = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    has_visible_pixel.then_some(AlphaBounds {
        left: min_x,
        top: min_y,
        right: max_x,
        bottom: max_y,
    })
}

fn push_issue(
    report: &mut ResourceLintReport,
    severity: ResourceLintSeverity,
    code: &str,
    asset: &str,
    message: String,
) {
    report.issues.push(ResourceLintIssue {
        severity,
        code: code.to_string(),
        asset: asset.to_string(),
        message,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn extract_asset_paths_supports_plain_and_nsis_styles() {
        let raw = "assets/logo.png file='assets/btn_close.png' dest='0,0,32,32'";
        let paths = extract_asset_paths(raw);

        assert!(paths.contains(&"assets/logo.png".to_string()));
        assert!(paths.contains(&"assets/btn_close.png".to_string()));
    }

    #[test]
    fn lint_report_flags_missing_assets() -> Result<()> {
        let temp = tempdir()?;
        fs::create_dir_all(temp.path().join("layouts"))?;
        fs::write(
            temp.path().join("layouts").join("configpage.xml"),
            r#"<Page><Icon src="assets/missing.png" /></Page>"#,
        )?;

        let report = lint_project_resources(temp.path())?;

        assert!(report.has_errors());
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "missing-asset"));
        Ok(())
    }

    #[test]
    fn lint_report_flags_bad_retina_size_and_alpha_offset() -> Result<()> {
        let temp = tempdir()?;
        let layouts_dir = temp.path().join("layouts");
        let assets_dir = temp.path().join("assets");
        fs::create_dir_all(&layouts_dir)?;
        fs::create_dir_all(&assets_dir)?;
        fs::write(
            layouts_dir.join("configpage.xml"),
            r#"<Page><Icon src="assets/arrow-down.png" /></Page>"#,
        )?;

        let mut base = ImageBuffer::from_pixel(10, 10, Rgba([0u8, 0u8, 0u8, 0u8]));
        for x in 2..8 {
            for y in 5..7 {
                base.put_pixel(x, y, Rgba([255u8, 255u8, 255u8, 255u8]));
            }
        }
        base.save(assets_dir.join("arrow-down.png"))?;

        let retina = ImageBuffer::from_pixel(18, 18, Rgba([255u8, 255u8, 255u8, 255u8]));
        retina.save(assets_dir.join("arrow-down@2x.png"))?;

        let report = lint_project_resources(temp.path())?;

        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "invalid-2x-size"));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "alpha-padding-offset"));
        Ok(())
    }

    #[test]
    fn lint_report_uses_custom_resource_directories() -> Result<()> {
        let temp = tempdir()?;
        let ui_dir = temp.path().join("ui");
        let skin_dir = temp.path().join("skin");
        fs::create_dir_all(&ui_dir)?;
        fs::create_dir_all(&skin_dir)?;
        fs::write(
            temp.path().join("installer_config.json"),
            serde_json::json!({
                "resources": {
                    "layouts_dir": "ui",
                    "assets_dir": "skin"
                }
            })
            .to_string(),
        )?;
        fs::write(
            ui_dir.join("configpage.xml"),
            r#"<Page><Icon src="skin/arrow-down.png" /></Page>"#,
        )?;
        let image = ImageBuffer::from_pixel(12, 12, Rgba([255u8, 255u8, 255u8, 255u8]));
        image.save(skin_dir.join("arrow-down.png"))?;
        let retina = ImageBuffer::from_pixel(24, 24, Rgba([255u8, 255u8, 255u8, 255u8]));
        retina.save(skin_dir.join("arrow-down@2x.png"))?;

        let report = lint_project_resources(temp.path())?;

        assert_eq!(report.referenced_assets, 1);
        assert_eq!(report.checked_assets, 1);
        assert!(report.issues.is_empty());
        Ok(())
    }
}
