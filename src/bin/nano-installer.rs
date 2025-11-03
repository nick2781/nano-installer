// nano-installer CLI 工具
// 用于创建、编译、验证安装器项目

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use clap::{Parser, Subcommand};
use anyhow::{Result, Context, bail};

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
}

fn main() {
    // 检查是否有嵌入资源（如果有，说明这是一个生成的安装器）
    if nano_installer::resources::has_embedded_resources() {
        // 作为安装器/卸载器运行
        if let Err(e) = run_as_installer() {
            eprintln!("❌ Error: {}", e);
            std::process::exit(1);
        }
    } else {
        // 作为 CLI 工具运行
        if let Err(e) = run_as_cli() {
            eprintln!("❌ Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_as_installer() -> Result<()> {
    use nano_installer::installer_runtime::{InstallerMode, run_installer};
    
    // 检测运行模式（安装/卸载）
    let mode = InstallerMode::detect();
    
    // 运行安装器
    run_installer(mode)
}

fn run_as_cli() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { name, output } => {
            cmd_init(&name, output.as_deref())
        }
        Commands::Build { project, output, release } => {
            cmd_build(&project, output.as_deref(), release)
        }
        Commands::Validate { config } => {
            cmd_validate(&config)
        }
        Commands::Langpack { input, output } => {
            cmd_langpack(&input, output.as_deref())
        }
    }
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
    let config_content = std::fs::read_to_string(&config_path)
        .context("Failed to read configuration")?;
    let config: serde_json::Value = serde_json::from_str(&config_content)
        .context("Failed to parse configuration")?;
    
    // 提取项目信息
    let project_name = config["project"]["name"].as_str()
        .context("Missing project.name")?;
    let output_name = output_name
        .or_else(|| config["project"]["output_name"].as_str())
        .unwrap_or(project_name);
    
    println!("📦 Product: {}", project_name);
    println!("📝 Output: {}", output_name);
    
    // 验证资源
    validate_project_resources(project_dir, &config)?;
    
    // 创建 dist 目录
    let dist_dir = project_dir.join("dist");
    std::fs::create_dir_all(&dist_dir)?;
    
    // 生成安装器（会自动打包所有资源包括 payload）
    println!("📦 Building installer executable...");
    build_installer_exe(project_dir, &config, output_name, release)?;
    
    // 生成卸载器
    println!("🗑️  Building uninstaller executable...");
    build_uninstaller_exe(project_dir, &config, output_name, release)?;
    
    println!("\n✅ Build completed successfully!");
    println!("📁 Output directory: {}", dist_dir.display());
    println!("   - {}_Setup.exe (standalone installer, includes all resources + payload)", output_name);
    println!("   - uninst.exe (standalone uninstaller, includes minimal resources)");
    println!("\n💡 To run the installer:");
    println!("   cd {}", dist_dir.display());
    println!("   .\\{}_Setup.exe", output_name);
    println!("\n💡 During installation:");
    println!("   uninst.exe will be copied to the installation directory");
    
    Ok(())
}

/// 验证配置文件
fn cmd_validate(config_path: &Path) -> Result<()> {
    println!("🔍 Validating configuration...");
    println!("📄 Config: {}", config_path.display());
    
    if !config_path.exists() {
        bail!("Configuration file not found: {}", config_path.display());
    }
    
    // 加载并解析配置
    let config_content = std::fs::read_to_string(config_path)
        .context("Failed to read configuration")?;
    
    let _config: nano_installer::config::InstallerConfig = serde_json::from_str(&config_content)
        .context("Failed to parse configuration")?;
    
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
    let json_content = std::fs::read_to_string(input)
        .context("Failed to read input file")?;
    let locale_data: serde_json::Value = serde_json::from_str(&json_content)
        .context("Failed to parse JSON")?;
    
    let _locale = locale_data["locale"].as_str()
        .context("Missing 'locale' field in JSON")?;
    
    // 确定输出文件名
    let output_path = output.map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
            input.with_extension("pak")
        });
    
    // 构建 .pak 文件
    build_langpack(&locale_data, &output_path)
        .context("Failed to build language pack")?;
    
    println!("✅ Language pack created: {}", output_path.display());
    
    Ok(())
}

// ============================================================================
// 辅助函数
// ============================================================================

fn validate_project_resources(project_dir: &Path, _config: &serde_json::Value) -> Result<()> {
    println!("🔍 Validating resources...");
    
    // 检查必要的目录
    let assets_dir = project_dir.join("assets");
    let layouts_dir = project_dir.join("layouts");
    let locales_dir = project_dir.join("locales");
    let files_dir = project_dir.join("files");
    
    if !assets_dir.exists() {
        bail!("assets/ directory not found");
    }
    if !layouts_dir.exists() {
        bail!("layouts/ directory not found");
    }
    if !locales_dir.exists() {
        bail!("locales/ directory not found");
    }
    if !files_dir.exists() {
        bail!("files/ directory not found");
    }
    
    println!("✅ All required directories exist");
    
    Ok(())
}


fn build_langpack(locale_data: &serde_json::Value, output_path: &Path) -> Result<()> {
    use std::io::Write;
    
    let locale = locale_data["locale"].as_str()
        .context("Missing 'locale' field")?;
    let strings = locale_data["strings"].as_object()
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
        "install": {
            "exe_name": format!("{}.exe", name),
            "default_path": format!("C:\\Program Files\\{}", name),
            "append_to_path": name,
            "required_space_mb": 100,
            "require_admin": true,
            "mutex_name": format!("{}_Installer", name),
            "detect_running_process": true,
            "kill_process_on_install": false,
            "kill_process_on_uninstall": true
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
            "payload_file": "payload.7z",
            "installer_icon": "assets/logo.ico",
            "uninstaller_icon": "assets/logo.ico"
        },
        "ui": {
            "window_width": 574,
            "window_height": 358,
            "expanded_height": 518,
            "dpi_aware": true,
            "dpi_threshold": 144.0
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
        "channel": {
            "extract_from_filename": true,
            "filename_regex": ".*_([^_]+)\\.exe$",
            "default_channel": "default",
            "output_channel_conf": false
        },
        "uninstall": {
            "show_keep_data_option": true,
            "keep_data_default": true,
            "cleanup_game_registry": false,
            "game_registry_path": ""
        },
        "validation": {
            "check_path_legal": true,
            "check_disk_type": "Any",
            "check_disk_space": true
        },
        "advanced": {
            "silent_install_support": true,
            "update_support": false,
            "repair_support": false,
            "launch_after_install": true
        }
    })).unwrap()
}

fn generate_example_layouts(output_dir: &Path) -> Result<()> {
    let layouts_dir = output_dir.join("layouts");
    
    // 从嵌入的模板复制布局文件
    const WELCOME_XML: &str = include_str!("../../templates/layouts/welcome.xml");
    const CONFIG_XML: &str = include_str!("../../templates/layouts/config.xml");
    const INSTALLING_XML: &str = include_str!("../../templates/layouts/installing.xml");
    const FINISH_XML: &str = include_str!("../../templates/layouts/finish.xml");
    
    std::fs::write(layouts_dir.join("welcome.xml"), WELCOME_XML)?;
    std::fs::write(layouts_dir.join("config.xml"), CONFIG_XML)?;
    std::fs::write(layouts_dir.join("installing.xml"), INSTALLING_XML)?;
    std::fs::write(layouts_dir.join("finish.xml"), FINISH_XML)?;
    
    Ok(())
}

fn generate_example_locales(output_dir: &Path) -> Result<()> {
    let locales_dir = output_dir.join("locales");
    
    // 从嵌入的模板复制语言文件
    const EN_US_JSON: &str = include_str!("../../templates/locales/en-US.json");
    const ZH_CN_JSON: &str = include_str!("../../templates/locales/zh-CN.json");
    
    std::fs::write(locales_dir.join("en-US.json"), EN_US_JSON)?;
    std::fs::write(locales_dir.join("zh-CN.json"), ZH_CN_JSON)?;
    
    Ok(())
}

fn generate_readme(name: &str) -> String {
    format!(r#"# {} Installer

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
"#, name, name, name)
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        bail!("Source directory not found: {}", src.display());
    }
    
    std::fs::create_dir_all(dst)?;
    
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
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

/// 构建安装器可执行文件
fn build_installer_exe(
    project_dir: &Path,
    config: &serde_json::Value,
    output_name: &str,
    _release: bool,
) -> Result<()> {
    use nano_installer::resources::bundle::{ResourceBundle, ResourceType, append_bundle_to_exe};
    
    let dist_dir = project_dir.join("dist");
    std::fs::create_dir_all(&dist_dir)?;
    let output_path = dist_dir.join(format!("{}_Setup.exe", output_name));
    
    println!("   📦 Packing resources...");
    
    // 创建资源包
    let mut bundle = ResourceBundle::new();
    
    // 1. 添加配置文件
    let config_data = std::fs::read(project_dir.join("installer_config.json"))?;
    bundle.add_config(config_data)?;
    println!("      ✓ Config: installer_config.json");
    
    // 2. 添加 payload
    let possible_payloads = vec![
        project_dir.join("payload.7z"),
        project_dir.join("app.7z"),
        project_dir.join("payload").join("app.7z"),
        project_dir.join("dist").join("payload.7z"),
    ];
    
    let payload_path = possible_payloads
        .iter()
        .find(|p| p.exists())
        .context("Payload archive not found (app.7z or payload.7z)")?;
    
    let payload_data = std::fs::read(payload_path)?;
    let payload_size_mb = payload_data.len() as f64 / 1024.0 / 1024.0;
    bundle.add_payload(payload_data)?;
    println!("      ✓ Payload: {} ({:.2} MB)", payload_path.file_name().unwrap().to_string_lossy(), payload_size_mb);
    
    // 3. 添加布局文件
    let layouts_dir = project_dir.join("layouts");
    if layouts_dir.exists() {
        bundle.add_directory(&project_dir, &layouts_dir, ResourceType::Layout)?;
        println!("      ✓ Layouts: {}/ ({})", layouts_dir.display(), 
            bundle.get_by_type(ResourceType::Layout).len());
    }
    
    // 4. 添加资源文件
    let assets_dir = project_dir.join("assets");
    if assets_dir.exists() {
        bundle.add_directory(&project_dir, &assets_dir, ResourceType::Asset)?;
        println!("      ✓ Assets: {}/ ({})", assets_dir.display(),
            bundle.get_by_type(ResourceType::Asset).len());
    }
    
    // 5. 添加语言文件
    let locales_dir = project_dir.join("locales");
    if locales_dir.exists() {
        bundle.add_directory(&project_dir, &locales_dir, ResourceType::Locale)?;
        println!("      ✓ Locales: {}/ ({})", locales_dir.display(),
            bundle.get_by_type(ResourceType::Locale).len());
    }
    
    // 打包资源
    let bundle_data = bundle.pack()?;
    let bundle_size_mb = bundle_data.len() as f64 / 1024.0 / 1024.0;
    println!("      ✓ Bundle size: {:.2} MB", bundle_size_mb);
    
    // 复制 nano-installer.exe 作为基础
    let possible_paths = vec![
        PathBuf::from("target/release/nano-installer.exe"),
        PathBuf::from("../../target/release/nano-installer.exe"),
        std::env::current_exe().ok(),
    ];
    
    let installer_exe = possible_paths
        .into_iter()
        .flatten()
        .find(|p| p.exists())
        .context("nano-installer.exe not found. Please compile it first")?;
    
    println!("   📋 Creating installer executable...");
    std::fs::copy(&installer_exe, &output_path)
        .context("Failed to copy installer.exe")?;
    
    // 将资源包追加到 exe
    append_bundle_to_exe(&output_path, &bundle_data)?;
    
    let final_size_mb = std::fs::metadata(&output_path)?.len() as f64 / 1024.0 / 1024.0;
    println!("   ✅ Installer: {} ({:.2} MB)", output_path.display(), final_size_mb);
    
    Ok(())
}

/// 构建卸载器可执行文件
fn build_uninstaller_exe(
    project_dir: &Path,
    config: &serde_json::Value,
    output_name: &str,
    _release: bool,
) -> Result<()> {
    use nano_installer::resources::bundle::{ResourceBundle, ResourceType, append_bundle_to_exe};
    
    let dist_dir = project_dir.join("dist");
    let output_path = dist_dir.join("uninst.exe");
    
    println!("   📦 Packing uninstaller resources...");
    
    // 创建卸载器资源包（不包含 payload.7z）
    let mut bundle = ResourceBundle::new();
    
    // 1. 添加配置文件
    let config_data = std::fs::read(project_dir.join("installer_config.json"))?;
    bundle.add_config(config_data)?;
    println!("      ✓ Config: installer_config.json");
    
    // 2. 卸载器不需要 payload.7z
    
    // 3. 添加布局文件（只需要卸载相关的布局）
    let layouts_dir = project_dir.join("layouts");
    if layouts_dir.exists() {
        // 只添加卸载相关的布局
        for entry in std::fs::read_dir(&layouts_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name().unwrap().to_string_lossy();
                // 只包含卸载、语言选择等通用布局
                if file_name.contains("uninstall") || 
                   file_name.contains("language") || 
                   file_name.contains("finish") {
                    let relative_path = path.strip_prefix(project_dir)
                        .context("Failed to get relative path")?;
                    let name = relative_path.to_string_lossy().replace('\\', "/");
                    let data = std::fs::read(&path)?;
                    
                    bundle.items.push(nano_installer::resources::bundle::ResourceItem {
                        resource_type: ResourceType::Layout,
                        name,
                        data,
                    });
                }
            }
        }
        println!("      ✓ Layouts: {} uninstall-related", 
            bundle.get_by_type(ResourceType::Layout).len());
    }
    
    // 4. 添加资源文件（图标、背景等）
    let assets_dir = project_dir.join("assets");
    if assets_dir.exists() {
        bundle.add_directory(&project_dir, &assets_dir, ResourceType::Asset)?;
        println!("      ✓ Assets: {}/ ({})", assets_dir.display(),
            bundle.get_by_type(ResourceType::Asset).len());
    }
    
    // 5. 添加语言文件
    let locales_dir = project_dir.join("locales");
    if locales_dir.exists() {
        bundle.add_directory(&project_dir, &locales_dir, ResourceType::Locale)?;
        println!("      ✓ Locales: {}/ ({})", locales_dir.display(),
            bundle.get_by_type(ResourceType::Locale).len());
    }
    
    // 打包资源
    let bundle_data = bundle.pack()?;
    let bundle_size_kb = bundle_data.len() as f64 / 1024.0;
    println!("      ✓ Bundle size: {:.2} KB", bundle_size_kb);
    
    // 复制 nano-installer.exe 作为基础
    let possible_paths = vec![
        PathBuf::from("target/release/nano-installer.exe"),
        PathBuf::from("../../target/release/nano-installer.exe"),
        std::env::current_exe().ok(),
    ];
    
    let base_exe = possible_paths
        .into_iter()
        .flatten()
        .find(|p| p.exists())
        .context("nano-installer.exe not found. Please compile it first")?;
    
    std::fs::copy(&base_exe, &output_path)
        .context("Failed to copy base exe")?;
    
    // 将资源包追加到 exe
    append_bundle_to_exe(&output_path, &bundle_data)?;
    
    let final_size_kb = std::fs::metadata(&output_path)?.len() as f64 / 1024.0;
    println!("   ✅ Uninstaller: {} ({:.2} KB)", output_path.display(), final_size_kb);
    
    Ok(())
}
