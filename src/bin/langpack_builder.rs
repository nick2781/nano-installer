// 语言包构建工具

use nano_installer::i18n::LanguagePack;
use std::path::{Path, PathBuf};
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        print_usage();
        std::process::exit(1);
    }
    
    let input_dir = &args[1];
    let output_dir = &args[2];
    
    println!("Building language packs...");
    println!("Input directory: {}", input_dir);
    println!("Output directory: {}", output_dir);
    
    let input_path = Path::new(input_dir);
    let output_path = Path::new(output_dir);
    
    // 创建输出目录
    if let Err(e) = fs::create_dir_all(output_path) {
        eprintln!("Failed to create output directory: {}", e);
        std::process::exit(1);
    }
    
    // 处理所有 JSON 文件
    let entries = match fs::read_dir(input_path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Failed to read input directory: {}", e);
            std::process::exit(1);
        }
    };
    
    for entry in entries.flatten() {
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let locale = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            
            println!("\nProcessing: {} -> {}.pak", locale, locale);
            
            match build_language_pack(&path, output_path, locale) {
                Ok(size) => {
                    println!("  ✓ Built successfully ({} bytes)", size);
                }
                Err(e) => {
                    eprintln!("  ✗ Failed: {}", e);
                }
            }
        }
    }
    
    println!("\nLanguage pack build completed!");
}

fn build_language_pack(json_path: &Path, output_dir: &Path, locale: &str) -> Result<usize, Box<dyn std::error::Error>> {
    // 读取 JSON 文件
    let json_content = fs::read_to_string(json_path)?;
    
    // 创建语言包
    let pack = LanguagePack::from_json(locale.to_string(), &json_content)?;
    
    // 序列化为二进制
    let bytes = pack.to_bytes()?;
    
    // 写入 .pak 文件
    let output_path = output_dir.join(format!("{}.pak", locale));
    fs::write(&output_path, &bytes)?;
    
    println!("  - Keys: {}", pack.translations.len());
    println!("  - Size: {} bytes", bytes.len());
    println!("  - Output: {:?}", output_path);
    
    Ok(bytes.len())
}

fn print_usage() {
    println!("Language Pack Builder");
    println!("\nUsage:");
    println!("  langpack-builder <input-dir> <output-dir>");
    println!("\nExample:");
    println!("  langpack-builder ./locales ./dist/locales");
}

