// nano-installer CLI 工具
// 用于创建、编译、验证安装器项目

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::path::{Path, PathBuf};

mod icon_replacer;
mod resource_lint;
mod version_info_builder;

#[derive(Parser)]
#[command(name = "nano-installer")]
#[command(about = "Universal installer generator - Create professional Windows installers")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = "nano-installer contributors")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new installer project
    Init {
        /// Project name
        name: String,

        /// Output directory (defaults to project name)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Build installer from project
    Build {
        /// Project directory (defaults to current directory)
        #[arg(short, long, default_value = ".")]
        project: PathBuf,

        /// Output file name (defaults to config output_name)
        #[arg(short, long)]
        output: Option<String>,

        /// Release build (optimized)
        #[arg(long)]
        release: bool,
    },

    /// Validate project configuration
    Validate {
        /// Configuration file path
        #[arg(short, long, default_value = "installer_config.json")]
        config: PathBuf,
    },

    /// Build language pack from JSON locale file
    Langpack {
        /// Input JSON locale file
        input: PathBuf,

        /// Output .pak file (optional, defaults to input name with .pak extension)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Harness-oriented UI diagnostics
    Harness {
        #[command(subcommand)]
        command: HarnessCommands,
    },
}

#[derive(Subcommand)]
enum HarnessCommands {
    /// Dump a UI layout snapshot from a project directory
    Snapshot {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        project: PathBuf,

        /// Wizard mode
        #[arg(long, value_enum, default_value_t = HarnessModeArg::Install)]
        mode: HarnessModeArg,

        /// Override locale before snapshot
        #[arg(long)]
        locale: Option<String>,

        /// Snapshot page ID (defaults to current page)
        #[arg(long)]
        page: Option<String>,

        /// Dispatch XML actions before snapshot
        #[arg(long = "action")]
        actions: Vec<String>,

        /// Output format
        #[arg(long, value_enum, default_value_t = HarnessOutputFormat::Json)]
        format: HarnessOutputFormat,

        /// Write output to file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Lint project UI assets for harness-driven review
    LintResources {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        project: PathBuf,

        /// Output format
        #[arg(long, value_enum, default_value_t = HarnessOutputFormat::Text)]
        format: HarnessOutputFormat,

        /// Write output to file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum HarnessModeArg {
    Install,
    Update,
    Uninstall,
}

impl HarnessModeArg {
    fn as_wizard_mode(self) -> nano_installer::ui::WizardMode {
        match self {
            Self::Install => nano_installer::ui::WizardMode::Install,
            Self::Update => nano_installer::ui::WizardMode::Update,
            Self::Uninstall => nano_installer::ui::WizardMode::Uninstall,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum HarnessOutputFormat {
    Json,
    Text,
}

#[derive(Debug, Serialize)]
struct HarnessSnapshotOutput {
    project: String,
    mode: String,
    locale: Option<String>,
    actions: Vec<String>,
    current_page: String,
    snapshot_page: String,
    close_confirmation_pending: bool,
    text_inputs: std::collections::HashMap<String, String>,
    computed: nano_installer::layout::taffy_bridge::ComputedLayout,
}

fn main() {
    // nano-installer.exe 是纯 CLI 工具
    // 不再有双重身份，不会作为安装器运行
    if let Err(e) = run_cli() {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}

fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name, output } => cmd_init(&name, output.as_deref()),
        Commands::Build {
            project,
            output,
            release,
        } => cmd_build(&project, output.as_deref(), release),
        Commands::Validate { config } => cmd_validate(&config),
        Commands::Langpack { input, output } => cmd_langpack(&input, output.as_deref()),
        Commands::Harness { command } => match command {
            HarnessCommands::Snapshot {
                project,
                mode,
                locale,
                page,
                actions,
                format,
                output,
            } => cmd_harness_snapshot(
                &project,
                mode,
                locale.as_deref(),
                page.as_deref(),
                &actions,
                format,
                output.as_deref(),
            ),
            HarnessCommands::LintResources {
                project,
                format,
                output,
            } => cmd_harness_lint_resources(&project, format, output.as_deref()),
        },
    }
}

fn cmd_harness_snapshot(
    project_dir: &Path,
    mode: HarnessModeArg,
    locale: Option<&str>,
    page: Option<&str>,
    actions: &[String],
    format: HarnessOutputFormat,
    output_path: Option<&Path>,
) -> Result<()> {
    let report = build_harness_snapshot_report(project_dir, mode, locale, page, actions, format)?;

    if let Some(path) = output_path {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, report.as_bytes())
            .with_context(|| format!("failed to write harness snapshot: {}", path.display()))?;
        println!("✅ Harness snapshot written: {}", path.display());
    } else {
        println!("{}", report);
    }

    Ok(())
}

fn cmd_harness_lint_resources(
    project_dir: &Path,
    format: HarnessOutputFormat,
    output_path: Option<&Path>,
) -> Result<()> {
    let report = build_harness_resource_lint_report(project_dir, format)?;

    if let Some(path) = output_path {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, report.as_bytes())
            .with_context(|| format!("failed to write resource lint report: {}", path.display()))?;
        println!("✅ Resource lint report written: {}", path.display());
    } else {
        println!("{}", report);
    }

    let lint = resource_lint::lint_project_resources(project_dir)?;
    if lint.has_errors() {
        bail!(
            "resource lint failed with {} error(s) and {} warning(s)",
            lint.error_count(),
            lint.warning_count()
        );
    }

    Ok(())
}

fn build_harness_snapshot_report(
    project_dir: &Path,
    mode: HarnessModeArg,
    locale: Option<&str>,
    page: Option<&str>,
    actions: &[String],
    format: HarnessOutputFormat,
) -> Result<String> {
    use nano_installer::ui::UiHarness;

    let mut harness = UiHarness::from_project_dir(project_dir, mode.as_wizard_mode())
        .with_context(|| format!("failed to create UI harness: {}", project_dir.display()))?;

    if let Some(locale) = locale {
        harness.switch_language(locale);
    }

    for action in actions {
        harness.dispatch_action(action);
    }

    let snapshot = if let Some(page_id) = page {
        harness.snapshot_page(page_id)?
    } else {
        harness.snapshot_current_page()?
    };

    let payload = HarnessSnapshotOutput {
        project: project_dir.display().to_string(),
        mode: format!("{:?}", mode.as_wizard_mode()).to_lowercase(),
        locale: locale.map(str::to_string),
        actions: actions.to_vec(),
        current_page: harness.current_page_id().to_string(),
        snapshot_page: snapshot.page_id().to_string(),
        close_confirmation_pending: harness.has_pending_close_confirmation(),
        text_inputs: harness.text_input_values().unwrap_or_default(),
        computed: snapshot.computed().clone(),
    };

    match format {
        HarnessOutputFormat::Json => {
            serde_json::to_string_pretty(&payload).context("failed to serialize harness snapshot")
        }
        HarnessOutputFormat::Text => Ok(render_harness_snapshot_text(&payload)),
    }
}

fn build_harness_resource_lint_report(
    project_dir: &Path,
    format: HarnessOutputFormat,
) -> Result<String> {
    let report = resource_lint::lint_project_resources(project_dir).with_context(|| {
        format!(
            "failed to lint harness resources for project: {}",
            project_dir.display()
        )
    })?;

    match format {
        HarnessOutputFormat::Json => serde_json::to_string_pretty(&report)
            .context("failed to serialize resource lint report"),
        HarnessOutputFormat::Text => Ok(resource_lint::render_resource_lint_text(&report)),
    }
}

fn render_harness_snapshot_text(snapshot: &HarnessSnapshotOutput) -> String {
    let mut lines = vec![
        format!("project: {}", snapshot.project),
        format!("mode: {}", snapshot.mode),
        format!("current_page: {}", snapshot.current_page),
        format!("snapshot_page: {}", snapshot.snapshot_page),
        format!(
            "locale: {}",
            snapshot.locale.as_deref().unwrap_or("<unchanged>")
        ),
        format!(
            "close_confirmation_pending: {}",
            snapshot.close_confirmation_pending
        ),
    ];

    if snapshot.actions.is_empty() {
        lines.push("actions: <none>".to_string());
    } else {
        lines.push(format!("actions: {}", snapshot.actions.join(", ")));
    }

    if snapshot.text_inputs.is_empty() {
        lines.push("text_inputs: <empty>".to_string());
    } else {
        lines.push("text_inputs:".to_string());
        let mut text_inputs: Vec<_> = snapshot.text_inputs.iter().collect();
        text_inputs.sort_by(|a, b| a.0.cmp(b.0));
        for (key, value) in text_inputs {
            lines.push(format!("  {} = {}", key, value));
        }
    }

    lines.push(format!(
        "container: {} x {}",
        snapshot.computed.container_width, snapshot.computed.container_height
    ));
    lines.push("rects:".to_string());

    let mut rects: Vec<_> = snapshot.computed.by_id.iter().collect();
    rects.sort_by(|a, b| a.0.cmp(b.0));
    for (id, rect) in rects {
        lines.push(format!(
            "  {}: x={}, y={}, w={}, h={}",
            id, rect.x, rect.y, rect.width, rect.height
        ));
    }

    lines.join("\n")
}

/// 初始化新项目
fn cmd_init(name: &str, output_dir: Option<&Path>) -> Result<()> {
    let output_dir = output_dir.unwrap_or_else(|| Path::new(name));

    println!("🚀 Initializing project: {}", name);
    println!("📁 Output directory: {}", output_dir.display());

    if output_dir.exists() {
        bail!("Directory already exists: {}", output_dir.display());
    }

    // 创建项目目录结构
    std::fs::create_dir_all(output_dir)?;
    std::fs::create_dir_all(output_dir.join("assets"))?;
    std::fs::create_dir_all(output_dir.join("layouts"))?;
    std::fs::create_dir_all(output_dir.join("locales"))?;
    std::fs::create_dir_all(output_dir.join("files"))?;

    // 生成配置文件
    let config = generate_default_config(name);
    std::fs::write(output_dir.join("installer_config.json"), config)?;
    println!("✅ Created: installer_config.json");

    // 生成示例布局文件
    generate_example_layouts(output_dir)?;
    println!("✅ Created: layouts/*.xml");

    // 生成示例语言文件
    generate_example_locales(output_dir)?;
    println!("✅ Created: locales/*.json");

    // 生成 README
    let readme = generate_readme(name);
    std::fs::write(output_dir.join("README.md"), readme)?;
    println!("✅ Created: README.md");

    // 生成构建脚本
    generate_build_script(output_dir)?;
    println!("✅ Created: build.ps1");

    println!("\n🎉 Project '{}' initialized successfully!", name);
    println!("\n📝 Next steps:");
    println!("   1. cd {}", name);
    println!("   2. Add your application files to files/");
    println!("   3. Add your assets (logo, icons) to assets/");
    println!("   4. Run: nano-installer build");

    Ok(())
}

/// 编译项目生成安装器
fn cmd_build(project_dir: &Path, output_name: Option<&str>, release: bool) -> Result<()> {
    println!("🔨 Building installer...");
    println!("📁 Project: {}", project_dir.display());

    // 检查项目目录
    if !project_dir.exists() {
        bail!("Project directory not found: {}", project_dir.display());
    }

    let config_path = project_dir.join("installer_config.json");
    if !config_path.exists() {
        bail!("Configuration file not found: {}", config_path.display());
    }

    // 加载配置
    let config_content =
        std::fs::read_to_string(&config_path).context("Failed to read configuration")?;
    let config: serde_json::Value =
        serde_json::from_str(&config_content).context("Failed to parse configuration")?;

    // 提取项目信息
    let project_name = config["project"]["name"]
        .as_str()
        .context("Missing project.name")?;
    let output_name = output_name
        .or_else(|| config["project"]["output_name"].as_str())
        .unwrap_or(project_name);
    let installer_name = resolved_installer_name(&config, output_name);

    println!("📦 Product: {}", project_name);
    println!("📝 Output: {}", output_name);

    ensure_payload_exists(project_dir, &config)?;

    // 验证资源
    validate_project_resources(project_dir, &config)?;

    // 验证布局文件
    validate_layout_files(project_dir, &config)?;

    // 创建临时构建目录和 dist 目录
    let build_dir = project_dir.join(".build");
    let dist_dir = project_dir.join("dist");

    // 清理旧的构建目录
    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir)?;
    }
    std::fs::create_dir_all(&build_dir)?;
    std::fs::create_dir_all(&dist_dir)?;
    cleanup_stale_installer_artifacts(&dist_dir, &installer_name)?;

    println!("📁 Build directory: {}", build_dir.display());
    println!("📁 Output directory: {}", dist_dir.display());
    println!();

    // 先生成卸载器（这样安装器可以将它打包进去）
    println!("🗑️  Building uninstaller executable...");
    build_uninstaller_exe(project_dir, &config, output_name, release)?;

    // 再生成安装器（会自动打包所有资源包括 payload 和 uninst.exe）
    println!("📦 Building installer executable...");
    build_installer_exe(project_dir, &config, output_name, release)?;

    // 保留 .build 目录用于调试（包含中间构建产物）
    // uninst.exe 已经嵌入到 TapTap_Setup.exe 中，不需要复制到 dist/
    println!();
    println!("💡 Build artifacts:");
    println!("   .build/uninst.exe    - Uninstaller stub with resources (embedded in setup)");
    println!("   .build/locales/*.pak - Compiled language packs (embedded in setup)");
    println!("   dist/                - Final installer executable");

    //  清理临时构建目录（可选，注释掉以保留中间文件用于调试）
    // println!();
    // println!("🧹 Cleaning up temporary build directory...");
    // if build_dir.exists() {
    //     std::fs::remove_dir_all(&build_dir)?;
    //     println!("   ✓ Removed {}", build_dir.display());
    // }

    println!("\n✅ Build completed successfully!");
    println!("📁 Output: {}", dist_dir.display());
    println!("\n📦 Installer package structure (segmented):");
    println!("   {}", installer_name);
    println!("   ├─ lzma-x64-unicode.exe stub (~4 MB)");
    println!("   └─ Resource bundle (5 segments, ~3 MB)");
    println!("      ├─ Segment 1: Config (JSON)");
    println!("      ├─ Segment 2: UI Resources (layouts + assets → 7z)");
    println!("      ├─ Segment 3: Locales (11 .pak files → 7z)");
    println!("      ├─ Segment 4: Payload (app.7z)");
    println!("      └─ Segment 5: Uninstaller (uninst.exe)");
    println!("\n💡 To test:");
    println!("   cd {}", dist_dir.display());
    println!("   .\\{}", installer_name);

    Ok(())
}

fn resolved_installer_name(config: &serde_json::Value, output_name: &str) -> String {
    let default_installer_name = format!("{}_Setup.exe", output_name);
    config["output"]["installer_name"]
        .as_str()
        .unwrap_or(&default_installer_name)
        .to_string()
}

fn cleanup_stale_installer_artifacts(dist_dir: &Path, active_installer_name: &str) -> Result<()> {
    for entry in std::fs::read_dir(dist_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name,
            None => continue,
        };

        if !file_name.ends_with("_Setup.exe") || file_name == active_installer_name {
            continue;
        }

        std::fs::remove_file(&path).with_context(|| {
            format!(
                "Failed to remove stale installer artifact: {}",
                path.display()
            )
        })?;
        println!("🧹 Removed stale installer artifact: {}", file_name);
    }

    Ok(())
}

fn config_resource_dir(config: &serde_json::Value, key: &str, default: &str) -> String {
    config["resources"][key]
        .as_str()
        .unwrap_or(default)
        .to_string()
}

fn project_resource_dir(
    project_dir: &Path,
    config: &serde_json::Value,
    key: &str,
    default: &str,
) -> PathBuf {
    project_dir.join(config_resource_dir(config, key, default))
}

/// 验证配置文件
fn cmd_validate(config_path: &Path) -> Result<()> {
    println!("🔍 Validating configuration...");
    println!("📄 Config: {}", config_path.display());

    if !config_path.exists() {
        bail!("Configuration file not found: {}", config_path.display());
    }

    // 加载并解析配置
    let config_content =
        std::fs::read_to_string(config_path).context("Failed to read configuration")?;

    let _config: nano_installer::config::InstallerConfig =
        serde_json::from_str(&config_content).context("Failed to parse configuration")?;

    println!("✅ Configuration is valid!");

    Ok(())
}

/// 构建语言包
fn cmd_langpack(input: &Path, output: Option<&Path>) -> Result<()> {
    println!("🌐 Building language pack...");
    println!("📄 Input: {}", input.display());

    if !input.exists() {
        bail!("Input file not found: {}", input.display());
    }

    // 读取 JSON
    let json_content = std::fs::read_to_string(input).context("Failed to read input file")?;
    let locale_data: serde_json::Value =
        serde_json::from_str(&json_content).context("Failed to parse JSON")?;

    let _locale = locale_data["locale"]
        .as_str()
        .context("Missing 'locale' field in JSON")?;

    // 确定输出文件名
    let output_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| input.with_extension("pak"));

    // 构建 .pak 文件
    build_langpack(&locale_data, &output_path).context("Failed to build language pack")?;

    println!("✅ Language pack created: {}", output_path.display());

    Ok(())
}

// ============================================================================
// 辅助函数
// ============================================================================

fn validate_project_resources(project_dir: &Path, config: &serde_json::Value) -> Result<()> {
    println!("🔍 Validating resources...");

    // 检查必要的目录
    let assets_dir = project_resource_dir(project_dir, config, "assets_dir", "assets");
    let layouts_dir = project_resource_dir(project_dir, config, "layouts_dir", "layouts");
    let locales_dir = project_resource_dir(project_dir, config, "locales_dir", "locales");

    if !assets_dir.exists() {
        bail!("assets directory not found: {}", assets_dir.display());
    }
    if !layouts_dir.exists() {
        bail!("layouts directory not found: {}", layouts_dir.display());
    }
    if !locales_dir.exists() {
        bail!("locales directory not found: {}", locales_dir.display());
    }

    // 检查 payload_file 配置（必需）
    let payload_file = config["resources"]["payload_file"]
        .as_str()
        .context("Missing 'resources.payload_file' in installer_config.json. Use a tracked archive path such as 'payload/app.7z'; nano-installer can generate it automatically from files/ during build.")?;

    let payload_path = project_dir.join(payload_file);
    if !payload_path.exists() {
        bail!(
            "Payload file not found: {}\nEither:\n  1. Put your application files under files/ and let build package them automatically\n  2. Or create an archive at the configured resources.payload_file path",
            payload_path.display(),
        );
    }

    println!("✅ All required directories and files exist");
    println!("   📦 Payload: {}", payload_file);

    let lint = resource_lint::lint_project_resources(project_dir)?;
    println!(
        "   🧪 Resource lint: {} error(s), {} warning(s)",
        lint.error_count(),
        lint.warning_count()
    );
    if lint.has_errors() {
        bail!(
            "resource lint failed with {} error(s); run `nano-installer harness lint-resources --project {}`",
            lint.error_count(),
            project_dir.display()
        );
    }

    Ok(())
}

/// 验证所有布局文件
fn validate_layout_files(project_dir: &Path, config: &serde_json::Value) -> Result<()> {
    println!("🔍 Validating layout files...");

    let layouts_dir = project_dir.join(
        config["resources"]["layouts_dir"]
            .as_str()
            .unwrap_or("layouts"),
    );

    if !layouts_dir.exists() {
        bail!("Layouts directory not found: {}", layouts_dir.display());
    }

    let mut errors = Vec::new();
    let mut validated_count = 0;

    // 查找所有 XML 布局文件
    for entry in std::fs::read_dir(&layouts_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("xml") {
            let file_name = path.file_name().unwrap().to_string_lossy();

            // 跳过备份文件
            if file_name.ends_with(".bak") {
                continue;
            }

            match validate_single_layout_file(&path) {
                Ok(_) => {
                    validated_count += 1;
                }
                Err(e) => {
                    errors.push(format!("{}: {}", file_name, e));
                }
            }
        }
    }

    if !errors.is_empty() {
        eprintln!("\n❌ Layout validation failed:");
        for error in &errors {
            eprintln!("   {}", error);
        }
        bail!("{} layout file(s) failed validation", errors.len());
    }

    println!(
        "✅ All {} layout file(s) validated successfully",
        validated_count
    );

    Ok(())
}

/// 验证单个布局文件
fn validate_single_layout_file(path: &Path) -> Result<()> {
    use nano_installer::layout::xml_parser::XmlParser;

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read layout file: {}", path.display()))?;

    let mut parser = XmlParser::new();
    match parser.parse_string(&content) {
        Ok(_) => Ok(()),
        Err(e) => {
            // 提取具体的验证错误信息
            let error_msg = format!("{}", e);
            if error_msg.contains("验证失败") {
                // 解析错误信息，提取验证失败的具体原因
                bail!("{}", error_msg);
            } else {
                bail!("{}", error_msg);
            }
        }
    }
}

fn build_langpack(locale_data: &serde_json::Value, output_path: &Path) -> Result<()> {
    use std::io::Write;

    let locale = locale_data["locale"]
        .as_str()
        .context("Missing 'locale' field")?;
    let strings = locale_data["strings"]
        .as_object()
        .context("Missing 'strings' object")?;

    let mut buffer = Vec::new();

    // 魔数 "LNGP"
    buffer.write_all(b"LNGP")?;

    // 版本号 (u16)
    buffer.write_all(&1u16.to_le_bytes())?;

    // 语言代码
    let locale_bytes = locale.as_bytes();
    buffer.write_all(&(locale_bytes.len() as u16).to_le_bytes())?;
    buffer.write_all(locale_bytes)?;

    // 预留 CRC32
    let crc_pos = buffer.len();
    buffer.write_all(&[0u8; 4])?;

    // 键值对数量
    buffer.write_all(&(strings.len() as u32).to_le_bytes())?;

    // 写入键值对
    for (key, value) in strings {
        let key_bytes = key.as_bytes();
        let value_str = value.as_str().unwrap_or("");
        let value_bytes = value_str.as_bytes();

        buffer.write_all(&(key_bytes.len() as u16).to_le_bytes())?;
        buffer.write_all(key_bytes)?;

        buffer.write_all(&(value_bytes.len() as u32).to_le_bytes())?;
        buffer.write_all(value_bytes)?;
    }

    // 计算并写入 CRC32
    let crc = crc32fast::hash(&buffer[crc_pos + 4..]);
    buffer[crc_pos..crc_pos + 4].copy_from_slice(&crc.to_le_bytes());

    std::fs::write(output_path, buffer)?;

    Ok(())
}

fn generate_default_config(name: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "project": {
            "name": name,
            "version": "1.0.0",
            "publisher": "Your Company",
            "copyright": format!("© 2025 {}", name),
            "output_name": format!("{}_Setup", name)
        },
        "output": {
            "installer_name": format!("{}_Setup.exe", name),
            "installer_icon": "assets/logo.ico",
            "uninstaller_name": "uninst.exe",
            "uninstaller_icon": "assets/logo.ico"
        },
        "install": {
            "exe_name": format!("{}.exe", name),
            "default_path": format!("C:\\Program Files\\{}", name),
            "append_to_path": name,
            "required_space_mb": 100,
            "require_admin": true,
            "mutex_name": format!("{}_Installer", name),
            "detect_running_process": true,
            "kill_process_on_install": false,
            "kill_process_on_uninstall": true,
            "close_targets": [
                {
                    "name": format!("{}.exe", name),
                    "kind": "process",
                    "detect_on_install": true,
                    "close_on_install": false,
                    "close_on_uninstall": true,
                    "force": true
                }
            ]
        },
        "registry": {
            "install_path_key": format!("HKLM\\Software\\{}", name),
            "uninstall_key": format!("HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", name),
            "help_link": "https://example.com/help"
        },
        "shortcuts": {
            "desktop_shortcut": true,
            "desktop_default": true,
            "start_menu": true,
            "start_menu_folder": name
        },
        "autostart": {
            "enabled": true,
            "default": false,
            "registry_key": "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "registry_value_name": name
        },
        "localization": {
            "default_locale": "en-US",
            "supported_locales": ["en-US", "zh-CN"],
            "show_language_selector": true
        },
        "links": {
            "terms_of_service": "https://example.com/terms",
            "privacy_policy": "https://example.com/privacy"
        },
        "resources": {
            "layouts_dir": "layouts",
            "assets_dir": "assets",
            "locales_dir": "locales",
            "payload_file": "payload/app.7z",
            "installer_icon": "assets/logo.ico",
            "uninstaller_icon": "assets/logo.ico"
        },
        "ui": {
            "window_width": 574,
            "window_height": 358,
            "expanded_height": 518,
            "dpi_aware": true,
            "dpi_threshold": 144
        },
        "wizard": {
            "pages": [
                {"id": "welcome", "layout": "welcome.xml", "title": "欢迎"},
                {"id": "config", "layout": "config.xml", "title": "配置"},
                {"id": "installing", "layout": "installing.xml", "title": "安装中"},
                {"id": "finish", "layout": "finish.xml", "title": "完成"}
            ],
            "uninstall_pages": []
        },
        "uninstall": {
            "show_keep_data_option": true,
            "keep_data_default": true,
            "data_paths": []
        },
        "validation": {
            "check_path_legal": true,
            "check_disk_type": "Any",
            "check_disk_space": true
        },
        "advanced": {
            "silent_mode_support": true,
            "update_mode_support": false,
            "uninstall_mode_support": true,
            "launch_app_after_install": true
        }
    })).unwrap()
}

fn generate_example_layouts(output_dir: &Path) -> Result<()> {
    let layouts_dir = output_dir.join("layouts");

    // 从嵌入的模板复制布局文件
    const WELCOME_XML: &str = include_str!("../../../templates/layouts/welcome.xml");
    const CONFIG_XML: &str = include_str!("../../../templates/layouts/config.xml");
    const INSTALLING_XML: &str = include_str!("../../../templates/layouts/installing.xml");
    const FINISH_XML: &str = include_str!("../../../templates/layouts/finish.xml");

    std::fs::write(layouts_dir.join("welcome.xml"), WELCOME_XML)?;
    std::fs::write(layouts_dir.join("config.xml"), CONFIG_XML)?;
    std::fs::write(layouts_dir.join("installing.xml"), INSTALLING_XML)?;
    std::fs::write(layouts_dir.join("finish.xml"), FINISH_XML)?;

    Ok(())
}

fn generate_example_locales(output_dir: &Path) -> Result<()> {
    let locales_dir = output_dir.join("locales");

    // 从嵌入的模板复制语言文件
    const EN_US_JSON: &str = include_str!("../../../templates/locales/en-US.json");
    const ZH_CN_JSON: &str = include_str!("../../../templates/locales/zh-CN.json");

    std::fs::write(locales_dir.join("en-US.json"), EN_US_JSON)?;
    std::fs::write(locales_dir.join("zh-CN.json"), ZH_CN_JSON)?;

    Ok(())
}

fn generate_readme(name: &str) -> String {
    format!(
        r#"# {} Installer

This is a nano-installer project for {}.

## Quick Start

1. Add your application files to `files/`
2. Add your assets (logo, icons) to `assets/`
3. Edit `installer_config.json` to configure your installer
4. Build: `nano-installer build`

## Project Structure

```
{}/
├── installer_config.json    # Main configuration
├── assets/                  # Images, icons
├── layouts/                 # UI layout XML files
├── locales/                 # Language files
├── files/                   # Application files to install
└── dist/                    # Build output
```

## Configuration

See `installer_config.json` for all configuration options.

## Building

```bash
nano-installer build
```

## Documentation

For more information, see the nano-installer documentation.
"#,
        name, name, name
    )
}

#[cfg(test)]
mod tests {
    use super::{
        build_harness_resource_lint_report, build_harness_snapshot_report, generate_default_config,
        validate_project_resources, HarnessModeArg, HarnessOutputFormat,
    };
    use anyhow::Result;
    use nano_installer::config::InstallerConfig;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn generated_default_config_matches_runtime_schema() {
        let raw = generate_default_config("SmokeApp");

        let parsed: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            parsed["resources"]["payload_file"].as_str(),
            Some("payload/app.7z")
        );
        assert_eq!(
            parsed["advanced"]["silent_mode_support"].as_bool(),
            Some(true)
        );
        assert_eq!(
            parsed["advanced"]["update_mode_support"].as_bool(),
            Some(false)
        );
        assert_eq!(
            parsed["advanced"]["uninstall_mode_support"].as_bool(),
            Some(true)
        );
        assert_eq!(
            parsed["advanced"]["launch_app_after_install"].as_bool(),
            Some(true)
        );

        let config: InstallerConfig = serde_json::from_str(&raw).unwrap();
        assert_eq!(config.resources.payload_file, "payload/app.7z");
        assert!(config.advanced.silent_mode_support);
        assert!(!config.advanced.update_mode_support);
        assert!(config.advanced.uninstall_mode_support);
    }

    #[test]
    fn ensure_payload_exists_packages_files_directory() -> Result<()> {
        let temp = tempdir()?;
        let project_dir = temp.path();
        let files_dir = project_dir.join("files");
        fs::create_dir_all(&files_dir)?;
        fs::write(files_dir.join("hello.txt"), "hello world")?;

        let config: Value = serde_json::json!({
            "resources": {
                "payload_file": "payload/app.7z"
            }
        });

        super::ensure_payload_exists(project_dir, &config)?;

        let payload_path = project_dir.join("payload").join("app.7z");
        assert!(payload_path.exists());
        assert!(fs::metadata(payload_path)?.len() > 0);
        Ok(())
    }

    #[test]
    fn harness_snapshot_report_contains_real_project_layout_state() -> Result<()> {
        let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/TapTap");

        let report = build_harness_snapshot_report(
            &project_dir,
            HarnessModeArg::Install,
            Some("ru"),
            Some("config"),
            &[String::from("toggle_panel:moreconfiginfo:show")],
            HarnessOutputFormat::Json,
        )?;

        let json: Value = serde_json::from_str(&report)?;
        assert_eq!(json["snapshot_page"].as_str(), Some("config"));
        assert_eq!(json["text_inputs"]["langSelect"].as_str(), Some("ru"));
        assert_eq!(json["close_confirmation_pending"].as_bool(), Some(false));
        assert!(json["computed"]["by_id"]["langSelect"].is_object());
        assert!(json["computed"]["by_id"]["moreconfiginfo"].is_object());
        Ok(())
    }

    #[test]
    fn harness_resource_lint_report_contains_project_summary() -> Result<()> {
        let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/TapTap");

        let report = build_harness_resource_lint_report(&project_dir, HarnessOutputFormat::Json)?;
        let json: Value = serde_json::from_str(&report)?;

        assert_eq!(
            json["project"].as_str(),
            Some(project_dir.to_string_lossy().as_ref())
        );
        assert!(json["referenced_assets"].as_u64().unwrap_or_default() > 0);
        assert!(json["checked_assets"].as_u64().unwrap_or_default() > 0);
        assert!(json["issues"].is_array());
        Ok(())
    }

    #[test]
    fn validate_project_resources_honors_custom_resource_directories() -> Result<()> {
        let temp = tempdir()?;
        let project_dir = temp.path();
        fs::create_dir_all(project_dir.join("skin"))?;
        fs::create_dir_all(project_dir.join("ui"))?;
        fs::create_dir_all(project_dir.join("lang"))?;
        fs::create_dir_all(project_dir.join("payload"))?;
        fs::write(
            project_dir.join("installer_config.json"),
            serde_json::json!({
                "resources": {
                    "assets_dir": "skin",
                    "layouts_dir": "ui",
                    "locales_dir": "lang",
                    "payload_file": "payload/app.7z"
                }
            })
            .to_string(),
        )?;
        fs::write(
            project_dir.join("ui").join("config.xml"),
            r#"<Page><Icon src="skin/logo.png" /></Page>"#,
        )?;
        let image =
            image::ImageBuffer::from_pixel(12, 12, image::Rgba([255u8, 255u8, 255u8, 255u8]));
        image.save(project_dir.join("skin").join("logo.png"))?;
        let retina =
            image::ImageBuffer::from_pixel(24, 24, image::Rgba([255u8, 255u8, 255u8, 255u8]));
        retina.save(project_dir.join("skin").join("logo@2x.png"))?;
        fs::write(project_dir.join("payload").join("app.7z"), [1u8, 2, 3, 4])?;

        let config: Value = serde_json::json!({
            "resources": {
                "assets_dir": "skin",
                "layouts_dir": "ui",
                "locales_dir": "lang",
                "payload_file": "payload/app.7z"
            }
        });

        validate_project_resources(project_dir, &config)?;
        Ok(())
    }
}

fn generate_build_script(output_dir: &Path) -> Result<()> {
    let script = r#"# Build script for nano-installer project

Write-Host "Building installer..." -ForegroundColor Yellow

# Run nano-installer build
nano-installer build

Write-Host "`n✅ Build complete!" -ForegroundColor Green
Write-Host "Output: dist/" -ForegroundColor Gray
"#;

    std::fs::write(output_dir.join("build.ps1"), script)?;

    Ok(())
}

fn ensure_payload_exists(project_dir: &Path, config: &serde_json::Value) -> Result<()> {
    let payload_file = config["resources"]["payload_file"]
        .as_str()
        .context("Missing 'resources.payload_file' in installer_config.json")?;
    let payload_path = project_dir.join(payload_file);
    if payload_path.exists() {
        return Ok(());
    }

    let files_dir = project_dir.join("files");
    if !files_dir.exists() {
        return Ok(());
    }

    println!("📦 Payload archive not found, packaging files/ automatically...");
    if let Some(parent) = payload_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    package_files_dir_to_zip(&files_dir, &payload_path)?;
    println!("   ✓ Created payload archive: {}", payload_path.display());
    Ok(())
}

fn package_files_dir_to_zip(files_dir: &Path, output_path: &Path) -> Result<()> {
    use std::io::Write;
    use zip::write::FileOptions;

    let file = std::fs::File::create(output_path).with_context(|| {
        format!(
            "Failed to create payload archive: {}",
            output_path.display()
        )
    })?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut added_any = false;
    for entry in walkdir::WalkDir::new(files_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let relative = match path.strip_prefix(files_dir) {
            Ok(p) if !p.as_os_str().is_empty() => p,
            _ => continue,
        };
        let name = relative.to_string_lossy().replace('\\', "/");
        if entry.file_type().is_dir() {
            zip.add_directory(name, options)?;
        } else {
            zip.start_file(name, options)?;
            let data = std::fs::read(path)?;
            zip.write_all(&data)?;
            added_any = true;
        }
    }

    zip.finish()?;
    if !added_any {
        bail!("files/ exists but does not contain any files to package");
    }
    Ok(())
}

fn estimate_archive_uncompressed_size(path: &Path) -> Result<u64> {
    let data = std::fs::read(path)?;
    if data.len() >= 2 && data[0] == 0x50 && data[1] == 0x4B {
        let reader = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader)
            .with_context(|| format!("Failed to open zip payload: {}", path.display()))?;
        let mut total = 0u64;
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            total += file.size();
        }
        return Ok(total);
    }

    let mut total = 0u64;
    let mut source = std::fs::File::open(path)?;
    let _ = sevenz_rust::decompress_with_extract_fn(&mut source, ".", |entry, _, _| {
        total += entry.size();
        Ok(true)
    });
    Ok(total)
}

/// 编译 JSON 语言文件为 .pak 格式
fn compile_locales_to_pak(
    project_dir: &Path,
    locales_dir: &Path,
) -> Result<Vec<(String, Vec<u8>)>> {
    // 创建 .build/locales/ 目录用于存放编译后的 .pak 文件
    let build_locales_dir = project_dir.join(".build").join("locales");
    std::fs::create_dir_all(&build_locales_dir)?;

    let mut pak_files = Vec::new();

    for entry in std::fs::read_dir(locales_dir)? {
        let entry = entry?;
        let path = entry.path();

        // 只处理 .json 文件
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let locale_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .context("Invalid locale file name")?;

        // 读取并解析 JSON
        let json_content =
            std::fs::read_to_string(&path).context(format!("Failed to read {}", path.display()))?;

        let translations: std::collections::HashMap<String, String> =
            serde_json::from_str(&json_content)
                .context(format!("Failed to parse {}", path.display()))?;

        // 编译为 .pak 格式
        let pak_data = compile_langpack_binary(locale_name, &translations)?;

        // 保存 .pak 文件到 .build/locales/ 目录
        let pak_filename = format!("{}.pak", locale_name);
        let pak_path = build_locales_dir.join(&pak_filename);
        std::fs::write(&pak_path, &pak_data)?;

        // 添加到列表（文件名，数据）
        pak_files.push((pak_filename, pak_data));
    }

    Ok(pak_files)
}

/// 编译语言包为二进制 .pak 格式
fn compile_langpack_binary(
    locale: &str,
    translations: &std::collections::HashMap<String, String>,
) -> Result<Vec<u8>> {
    use std::io::Write;

    let mut buffer = Vec::new();

    // 魔数 "LNGP"
    buffer.write_all(b"LNGP")?;

    // 版本号 (u16)
    buffer.write_all(&1u16.to_le_bytes())?;

    // 语言代码长度和内容
    let locale_bytes = locale.as_bytes();
    buffer.write_all(&(locale_bytes.len() as u16).to_le_bytes())?;
    buffer.write_all(locale_bytes)?;

    // 预留 CRC32 位置
    let crc_pos = buffer.len();
    buffer.write_all(&[0u8; 4])?;

    // 键值对数量
    buffer.write_all(&(translations.len() as u32).to_le_bytes())?;

    // 写入每个键值对
    for (key, value) in translations {
        let key_bytes = key.as_bytes();
        let value_bytes = value.as_bytes();

        buffer.write_all(&(key_bytes.len() as u16).to_le_bytes())?;
        buffer.write_all(key_bytes)?;

        buffer.write_all(&(value_bytes.len() as u32).to_le_bytes())?;
        buffer.write_all(value_bytes)?;
    }

    // 计算并写入 CRC32
    let crc = crc32fast::hash(&buffer[crc_pos + 4..]);
    buffer[crc_pos..crc_pos + 4].copy_from_slice(&crc.to_le_bytes());

    Ok(buffer)
}

/// 构建安装器可执行文件
fn build_installer_exe(
    project_dir: &Path,
    config: &serde_json::Value,
    output_name: &str,
    _release: bool,
) -> Result<()> {
    use nano_installer::resources::bundle::{append_bundle_to_exe, ResourceBundle};

    let dist_dir = project_dir.join("dist");
    std::fs::create_dir_all(&dist_dir)?;

    // 从配置中读取输出文件名
    let installer_name = resolved_installer_name(config, output_name);
    let output_path = dist_dir.join(&installer_name);

    println!("   📦 Packing resources (segmented format)...");

    // 创建资源包（分段式）
    let mut bundle = ResourceBundle::new();

    // 1. 计算 payload 解压后大小（从 .7z 文件读取）
    let payload_file_path = project_dir.join(
        config["resources"]["payload_file"]
            .as_str()
            .unwrap_or("payload/app.7z"),
    );

    let uncompressed_size_bytes = if payload_file_path.exists() {
        estimate_archive_uncompressed_size(&payload_file_path).unwrap_or(0)
    } else {
        0
    };
    let uncompressed_size_mb = uncompressed_size_bytes as f64 / 1024.0 / 1024.0;
    // 加 20% 缓冲 (注册表、快捷方式、临时文件等)
    let final_required_size = ((uncompressed_size_mb * 1.2) as u64).max(1);
    println!(
        "      ℹ️  Payload uncompressed: {:.2} MB, required space: {} MB",
        uncompressed_size_mb, final_required_size
    );

    // 2. 动态更新配置中的 required_space_mb
    let mut config_json: serde_json::Value = config.clone();

    if let Some(install) = config_json.get_mut("install") {
        install["required_space_mb"] = serde_json::json!(final_required_size);
    }

    let updated_config_data = serde_json::to_vec_pretty(&config_json)?;
    bundle.add_config(updated_config_data.clone())?;

    // 保存更新后的 config 到 .build/ 目录以便调试
    let build_dir = project_dir.join(".build");
    let config_path = build_dir.join("installer_config.json");
    std::fs::write(&config_path, &updated_config_data)?;
    println!(
        "      ✓ Segment 1: Config (installer_config.json, {} MB required space, saved to .build/)",
        final_required_size
    );

    // 2. 添加 UI 资源段（layouts + assets 打包为 7z）
    let mut ui_files = std::collections::HashMap::new();
    let mut ui_count = 0;

    // 2.1 收集 layouts
    let layouts_dir = project_resource_dir(project_dir, config, "layouts_dir", "layouts");
    if layouts_dir.exists() {
        for entry in walkdir::WalkDir::new(&layouts_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
            if file_name.ends_with(".bak") || file_name.ends_with(".backup") {
                continue;
            }
            let relative_path = path
                .strip_prefix(project_dir)
                .context("Failed to get relative path")?;
            let name = relative_path.to_string_lossy().replace('\\', "/");
            let data = std::fs::read(path)?;
            eprintln!("      [DEBUG] Collecting layout: {}", name);
            ui_files.insert(name, data);
            ui_count += 1;
        }
    }

    eprintln!("      [DEBUG] Total layouts collected: {}", ui_count);

    // 2.2 收集 assets
    let assets_dir = project_resource_dir(project_dir, config, "assets_dir", "assets");
    if assets_dir.exists() {
        for entry in walkdir::WalkDir::new(&assets_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let relative_path = path
                .strip_prefix(project_dir)
                .context("Failed to get relative path")?;
            let name = relative_path.to_string_lossy().replace('\\', "/");
            let data = std::fs::read(path)?;
            eprintln!("      [DEBUG] Collecting asset: {}", name);
            ui_files.insert(name, data);
            ui_count += 1;
        }
    }

    eprintln!(
        "      [DEBUG] Total UI files before adding to bundle: {}",
        ui_files.len()
    );

    if ui_count > 0 {
        bundle.add_ui_resources(ui_files)?;

        // 保存 ui_resources.7z 到 .build/ 目录以便调试
        let build_dir = project_dir.join(".build");
        if let Some(ui_segment) = bundle.segments.iter().find(|s| {
            matches!(
                s.segment_type,
                nano_installer::resources::bundle::SegmentType::UIResources
            )
        }) {
            let ui_7z_path = build_dir.join("ui_resources.7z");
            let data_size = ui_segment.data.len();
            std::fs::write(&ui_7z_path, &ui_segment.data)?;
            println!("      ✓ Segment 2: UI Resources ({} files compressed to {:.2} KB, saved to .build/ui_resources.7z)", 
                ui_count, data_size as f64 / 1024.0);
        }
    } else {
        println!("      ⚠️  Warning: No UI resources found");
    }

    // 3. 添加语言包段（编译 JSON -> .pak，然后打包为 7z）
    let locales_dir = project_resource_dir(project_dir, config, "locales_dir", "locales");
    if locales_dir.exists() {
        let pak_files = compile_locales_to_pak(&project_dir, &locales_dir)?;
        let locale_count = pak_files.len();
        let locale_files: std::collections::HashMap<String, Vec<u8>> =
            pak_files.into_iter().collect();
        bundle.add_locales(locale_files)?;

        // 保存 locales.7z 到 .build/ 目录以便调试
        let build_dir = project_dir.join(".build");
        if let Some(locale_segment) = bundle.segments.iter().find(|s| {
            matches!(
                s.segment_type,
                nano_installer::resources::bundle::SegmentType::Locales
            )
        }) {
            let locales_7z_path = build_dir.join("locales.7z");
            let data_size = locale_segment.data.len();
            std::fs::write(&locales_7z_path, &locale_segment.data)?;
            println!("      ✓ Segment 3: Locales ({} languages compressed to {:.2} KB, saved to .build/locales.7z)", 
                locale_count, data_size as f64 / 1024.0);
        }
    }

    // 4. 添加 payload 段（从配置指定的 7z 文件读取）
    // 注意：payload_file 已经在 validate_project_resources 中验证过，这里直接使用
    let configured_payload_file = config["resources"]["payload_file"]
        .as_str()
        .expect("payload_file should be validated in validate_project_resources");

    let payload_path = project_dir.join(configured_payload_file);

    let payload_data = std::fs::read(&payload_path)
        .with_context(|| format!("Failed to read payload file: {}", payload_path.display()))?;

    let size_mb = payload_data.len() as f64 / 1024.0 / 1024.0;

    // 复制 payload 到 .build/ 目录以便调试
    let build_dir = project_dir.join(".build");
    let build_payload_filename = payload_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("payload.7z");
    let build_payload_path = build_dir.join(build_payload_filename);
    std::fs::copy(&payload_path, &build_payload_path)?;

    println!(
        "      ✓ Segment 4: Payload ({:.2} MB, {} → .build/{})",
        size_mb, configured_payload_file, build_payload_filename
    );

    bundle.add_payload(payload_data)?;

    // 5. 添加卸载器段
    let uninstaller_name = config["output"]["uninstaller_name"]
        .as_str()
        .unwrap_or("uninst.exe");
    let uninstaller_path = project_dir.join(".build").join(uninstaller_name);

    if uninstaller_path.exists() {
        let uninst_data = std::fs::read(&uninstaller_path)?;
        let uninst_size_kb = uninst_data.len() as f64 / 1024.0;
        bundle.add_uninstaller(uninst_data)?;
        println!("      ✓ Segment 5: Uninstaller ({:.1} KB)", uninst_size_kb);
    } else {
        println!(
            "      ⚠️  Warning: Uninstaller not found at {}",
            uninstaller_path.display()
        );
    }

    // 6. 收集脚本文件 (scripts/*.rhai, 可选)
    let scripts_dir = project_dir.join("scripts");
    if scripts_dir.exists() {
        let mut script_files = std::collections::HashMap::new();
        for entry in walkdir::WalkDir::new(&scripts_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rhai"))
        {
            let relative = entry
                .path()
                .strip_prefix(project_dir)
                .context("Failed to get relative path")?;
            let name = relative.to_string_lossy().replace('\\', "/");
            let data = std::fs::read(entry.path())?;
            script_files.insert(name, data);
        }
        if !script_files.is_empty() {
            let script_count = script_files.len();
            bundle.add_scripts(script_files)?;
            println!("      ✓ Segment 6: Scripts ({} .rhai files)", script_count);
        }
    }

    // 打包资源
    let bundle_data = bundle.pack()?;
    let bundle_size_mb = bundle_data.len() as f64 / 1024.0 / 1024.0;
    println!("      ✓ Bundle size: {:.2} MB", bundle_size_mb);

    // 复制 lzma-x64-unicode.exe 作为基础（完整的安装器，类似 NSIS 的 lzma-x86-unicode）
    // 优先使用 debug 版本（带控制台输出），如果不存在则使用 release 版本
    let possible_stub_paths: Vec<Option<PathBuf>> = vec![
        Some(PathBuf::from("target/release/lzma-x64-unicode.exe")),
        Some(PathBuf::from("../../target/release/lzma-x64-unicode.exe")),
        Some(PathBuf::from("target/debug/lzma-x64-unicode.exe")),
        Some(PathBuf::from("../../target/debug/lzma-x64-unicode.exe")),
    ];

    let stub_exe = possible_stub_paths
        .into_iter()
        .flatten()
        .find(|p| p.exists())
        .context("lzma-x64-unicode.exe not found. Please compile it first with: cargo build --release -p lzma-x64-unicode")?;

    println!("   📋 Creating installer executable...");
    let stub_size_mb = std::fs::metadata(&stub_exe)?.len() as f64 / 1024.0 / 1024.0;
    println!(
        "      Using stub: {} ({:.2} MB)",
        stub_exe.display(),
        stub_size_mb
    );
    println!("      Copying stub to: {}", output_path.display());
    std::fs::copy(&stub_exe, &output_path).context("Failed to copy installer-stub.exe")?;
    let copied_size = std::fs::metadata(&output_path)?.len();
    println!(
        "      ✓ Stub copied ({:.2} MB)",
        copied_size as f64 / 1024.0 / 1024.0
    );

    // 先替换图标为项目配置的图标（必须在追加资源之前，因为 UpdateResourceW 会重写 PE 文件）
    let size_after_copy = std::fs::metadata(&output_path)?.len();
    println!(
        "      File size after stub copy: {:.2} MB",
        size_after_copy as f64 / 1024.0 / 1024.0
    );

    println!("   🎨 Replacing installer icon...");
    let installer_icon = config["output"]["installer_icon"]
        .as_str()
        .or_else(|| config["resources"]["installer_icon"].as_str())
        .unwrap_or("assets/logo.ico");
    let icon_path = project_dir.join(installer_icon);

    if icon_path.exists() {
        if let Err(e) = icon_replacer::replace_exe_icon(&output_path, &icon_path) {
            println!("      ⚠️  Warning: Failed to replace icon: {}", e);
            println!("      The installer will use the default runtime-stub icon");
        } else {
            println!("      ✓ Icon replaced: {}", installer_icon);
            // 等待 Windows 释放文件锁
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    } else {
        println!(
            "      ⚠️  Warning: Icon file not found: {}",
            icon_path.display()
        );
    }

    // 替换版本信息
    println!("   📝 Updating version info...");
    let product_name = config["project"]["name"].as_str().unwrap_or("Unknown");
    let version = config["project"]["version"].as_str().unwrap_or("1.0.0");
    let publisher = config["project"]["publisher"].as_str().unwrap_or("");
    let copyright = config["project"]["copyright"].as_str();

    let version_info = icon_replacer::VersionInfo {
        product_name: product_name.to_string(),
        product_version: version.to_string(),
        file_description: format!("{} Installer", product_name),
        file_version: version.to_string(),
        company_name: if !publisher.is_empty() {
            Some(publisher.to_string())
        } else {
            None
        },
        copyright: copyright.map(|s| s.to_string()),
    };

    if let Err(e) = icon_replacer::replace_version_info(&output_path, &version_info) {
        println!("      ⚠️  Warning: Failed to update version info: {}", e);
    } else {
        let company_info = if !publisher.is_empty() {
            format!(" by {}", publisher)
        } else {
            String::new()
        };
        println!(
            "      ✓ Version info: {} v{}{}",
            product_name, version, company_info
        );
    }

    // 等待 Windows 释放文件锁后再追加资源（更长的延迟，确保文件锁被释放）
    std::thread::sleep(std::time::Duration::from_secs(2));

    // 最后追加资源包（必须在图标和版本信息之后）
    println!("   📦 Appending resource bundle...");
    let before_append = std::fs::metadata(&output_path)?.len();
    println!(
        "      File size before append: {:.2} MB",
        before_append as f64 / 1024.0 / 1024.0
    );
    append_bundle_to_exe(&output_path, &bundle_data)?;
    let after_append = std::fs::metadata(&output_path)?.len();
    println!(
        "      File size after append: {:.2} MB (added {:.2} MB)",
        after_append as f64 / 1024.0 / 1024.0,
        (after_append - before_append) as f64 / 1024.0 / 1024.0
    );
    println!(
        "      ✓ Resource bundle appended ({:.2} MB)",
        bundle_data.len() as f64 / 1024.0 / 1024.0
    );

    let final_size_mb = std::fs::metadata(&output_path)?.len() as f64 / 1024.0 / 1024.0;
    println!(
        "   ✅ Installer: {} ({:.2} MB)",
        output_path.display(),
        final_size_mb
    );

    Ok(())
}

/// 构建卸载器可执行文件
fn build_uninstaller_exe(
    project_dir: &Path,
    config: &serde_json::Value,
    _output_name: &str,
    _release: bool,
) -> Result<()> {
    use nano_installer::resources::bundle::{append_bundle_to_exe, ResourceBundle};

    // 卸载器输出到临时构建目录，而不是 dist/
    let build_dir = project_dir.join(".build");
    std::fs::create_dir_all(&build_dir)?;

    // 从配置中读取卸载器文件名
    let uninstaller_name = config["output"]["uninstaller_name"]
        .as_str()
        .unwrap_or("uninst.exe");
    let output_path = build_dir.join(uninstaller_name);

    println!("   📦 Packing uninstaller resources...");

    // 创建卸载器资源包（不包含 payload.7z）
    let mut bundle = ResourceBundle::new();

    // 1. 添加配置文件
    let config_data = std::fs::read(project_dir.join("installer_config.json"))?;
    bundle.add_config(config_data)?;
    println!("      ✓ Config: installer_config.json");

    // 2. 卸载器不需要 payload.7z

    // 3. 收集 UI 资源（布局和资源文件）
    let mut ui_files = std::collections::HashMap::new();
    let mut ui_count = 0;

    // 3.1 添加所有布局文件（uninst 是完整引擎，需要 uninstall + msgBox 等通用布局）
    let layouts_dir = project_resource_dir(project_dir, config, "layouts_dir", "layouts");
    if layouts_dir.exists() {
        for entry in walkdir::WalkDir::new(&layouts_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let relative_path = path
                .strip_prefix(project_dir)
                .context("Failed to get relative path")?;
            let name = relative_path.to_string_lossy().replace('\\', "/");
            let data = std::fs::read(path)?;
            ui_files.insert(name, data);
            ui_count += 1;
        }
    }
    println!("      ✓ Layouts: {}", ui_count);

    // 3.2 添加所有资源文件（uninst 现在是完整 egui 引擎，需要全部 UI 资源）
    let assets_dir = project_resource_dir(project_dir, config, "assets_dir", "assets");
    let mut asset_count = 0;
    if assets_dir.exists() {
        for entry in walkdir::WalkDir::new(&assets_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let relative_path = path
                .strip_prefix(project_dir)
                .context("Failed to get relative path")?;
            let name = relative_path.to_string_lossy().replace('\\', "/");
            let data = std::fs::read(path)?;
            ui_files.insert(name, data);
            asset_count += 1;
        }
    }
    println!("      ✓ Assets: {}", asset_count);

    // 3.3 将 UI 资源添加到 bundle（自动压缩为 7z）
    bundle.add_ui_resources(ui_files)?;

    // 4. 编译并添加语言文件（JSON -> .pak）
    let locales_dir = project_resource_dir(project_dir, config, "locales_dir", "locales");
    if locales_dir.exists() {
        let pak_files = compile_locales_to_pak(&project_dir, &locales_dir)?;
        let locale_count = pak_files.len();
        let locale_files: std::collections::HashMap<String, Vec<u8>> =
            pak_files.into_iter().collect();
        bundle.add_locales(locale_files)?;
        println!(
            "      ✓ Locales: {}/ ({} compiled to .pak)",
            locales_dir.display(),
            locale_count
        );
    }

    // 打包资源
    let bundle_data = bundle.pack()?;
    let bundle_size_kb = bundle_data.len() as f64 / 1024.0;
    println!("      ✓ Bundle size: {:.2} KB", bundle_size_kb);

    // 复制 uninst.exe stub（完整 egui 引擎，和安装器共用同一套代码）
    let possible_stub_paths = [
        "target/release/uninst.exe",
        "target/debug/uninst.exe",
        "../../target/release/uninst.exe",
        "../../target/debug/uninst.exe",
    ];

    let stub_exe = possible_stub_paths
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
        .context("uninst.exe not found. Please compile it first with: cargo build -p uninst")?;

    println!(
        "      Using stub: {} ({} KB)",
        stub_exe.display(),
        std::fs::metadata(&stub_exe)?.len() / 1024
    );
    std::fs::copy(&stub_exe, &output_path).context("Failed to copy uninstaller-stub.exe")?;

    // 先替换图标和版本信息（必须在追加资源之前）
    println!("   🎨 Replacing uninstaller icon...");
    let uninstaller_icon = config["output"]["uninstaller_icon"]
        .as_str()
        .or_else(|| config["resources"]["uninstaller_icon"].as_str())
        .unwrap_or("assets/uninstall.ico");
    let icon_path = project_dir.join(uninstaller_icon);

    if icon_path.exists() {
        if let Err(e) = icon_replacer::replace_exe_icon(&output_path, &icon_path) {
            println!("      ⚠️  Warning: Failed to replace icon: {}", e);
        } else {
            println!("      ✓ Icon replaced: {}", uninstaller_icon);
        }
    } else {
        println!(
            "      ⚠️  Warning: Icon file not found: {}",
            icon_path.display()
        );
    }

    // 替换版本信息
    println!("   📝 Updating version info...");
    let product_name = config["project"]["name"].as_str().unwrap_or("Unknown");
    let version = config["project"]["version"].as_str().unwrap_or("1.0.0");
    let publisher = config["project"]["publisher"].as_str().unwrap_or("");
    let copyright = config["project"]["copyright"].as_str();

    let version_info = icon_replacer::VersionInfo {
        product_name: product_name.to_string(),
        product_version: version.to_string(),
        file_description: format!("{} Uninstaller", product_name),
        file_version: version.to_string(),
        company_name: if !publisher.is_empty() {
            Some(publisher.to_string())
        } else {
            None
        },
        copyright: copyright.map(|s| s.to_string()),
    };

    if let Err(e) = icon_replacer::replace_version_info(&output_path, &version_info) {
        println!("      ⚠️  Warning: Failed to update version info: {}", e);
    } else {
        let company_info = if !publisher.is_empty() {
            format!(" by {}", publisher)
        } else {
            String::new()
        };
        println!(
            "      ✓ Version info: {} v{}{}",
            product_name, version, company_info
        );
    }

    // 最后追加资源包（必须在图标和版本信息之后）
    println!("   📦 Appending resource bundle...");
    append_bundle_to_exe(&output_path, &bundle_data)?;
    println!(
        "      ✓ Resource bundle appended ({:.2} KB)",
        bundle_data.len() as f64 / 1024.0
    );

    let final_size_kb = std::fs::metadata(&output_path)?.len() as f64 / 1024.0;
    println!(
        "   ✅ Uninstaller: {} ({:.2} KB)",
        output_path.display(),
        final_size_kb
    );

    Ok(())
}
