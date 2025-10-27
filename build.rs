// 构建脚本

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // 编译时环境信息
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=locales/");
    
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);
    
    // 创建语言包输出目录
    let locales_out = out_path.join("locales");
    fs::create_dir_all(&locales_out).expect("Failed to create locales output directory");
    
    // 构建语言包
    build_language_packs(&locales_out);
    
    // Windows 特定的构建步骤
    #[cfg(target_os = "windows")]
    {
        // 嵌入应用程序图标和清单
        embed_resource::compile("resources/app.rc", embed_resource::NONE);
    }
    
    println!("Build script completed");
}

/// 构建所有语言包
fn build_language_packs(output_dir: &Path) {
    let locales_dir = Path::new("locales");
    
    if !locales_dir.exists() {
        println!("cargo:warning=Locales directory not found, skipping language pack build");
        return;
    }
    
    let entries = match fs::read_dir(locales_dir) {
        Ok(entries) => entries,
        Err(e) => {
            println!("cargo:warning=Failed to read locales directory: {}", e);
            return;
        }
    };
    
    for entry in entries.flatten() {
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let locale = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            
            println!("Building language pack: {}", locale);
            
            if let Err(e) = build_single_language_pack(&path, output_dir, locale) {
                println!("cargo:warning=Failed to build language pack for {}: {}", locale, e);
            }
        }
    }
}

/// 构建单个语言包
fn build_single_language_pack(
    json_path: &Path,
    output_dir: &Path,
    locale: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    
    // 读取 JSON
    let json_content = fs::read_to_string(json_path)?;
    
    // 这里使用一个简化的构建过程
    // 实际应该使用 nano_installer::i18n::LanguagePack，但构建脚本中访问主 crate 比较复杂
    // 所以我们直接复制 JSON 文件，运行时再构建 .pak
    
    let output_path = output_dir.join(format!("{}.json", locale));
    fs::write(&output_path, json_content)?;
    
    println!("  -> {:?}", output_path);
    
    Ok(())
}

