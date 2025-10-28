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
        // 根据编译目标设置不同的图标
        // 检查 CARGO_BIN_NAME 环境变量来判断正在编译哪个binary
        if let Ok(bin_name) = env::var("CARGO_BIN_NAME") {
            match bin_name.as_str() {
                "installer" => {
                    embed_resource::compile("resources/installer.rc", embed_resource::NONE);
                    println!("cargo:warning=Compiling installer with logo.ico");
                }
                "uninst" => {
                    embed_resource::compile("resources/uninstaller.rc", embed_resource::NONE);
                    println!("cargo:warning=Compiling uninstaller with uninst.ico");
                }
                _ => {}
            }
        } else {
            // 默认使用installer资源
            embed_resource::compile("resources/installer.rc", embed_resource::NONE);
        }
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
    
    // 解析 JSON 并构建 .pak 文件
    let translations: std::collections::HashMap<String, String> = 
        serde_json::from_str(&json_content)?;
    
    // 构建 .pak 文件
    let pak_data = build_pak_file(locale, &translations)?;
    
    let output_path = output_dir.join(format!("{}.pak", locale));
    fs::write(&output_path, pak_data)?;
    
    println!("  -> {:?}", output_path);
    
    Ok(())
}

/// 构建 .pak 文件格式
fn build_pak_file(
    locale: &str,
    translations: &std::collections::HashMap<String, String>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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

