// 构建脚本 - 资源内嵌和代码生成

use std::fs;
use std::path::Path;
use std::io::Write;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/nano-installer.ico");
    
    // 为 nano-installer CLI 工具设置图标
    // stub 文件的图标会在编译时使用默认图标，
    // 然后在 `nano-installer build` 时根据项目配置动态替换
    #[cfg(target_os = "windows")]
    {
        set_nano_installer_icon();
    }
}

// 以下函数不再使用，保留仅用于参考
// nano-installer 不嵌入语言包，由用户项目提供

#[allow(dead_code)]
fn build_language_packs(output_dir: &Path) {
    let locales_dir = Path::new("locales");
    
    // 项目根目录不再需要 locales 目录，使用动态加载
    if !locales_dir.exists() {
        // println!("cargo:warning=Locales directory not found, skipping language pack build");
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

#[allow(dead_code)]
fn build_single_language_pack(
    json_path: &Path,
    output_dir: &Path,
    locale: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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

#[allow(dead_code)]
fn build_pak_file(
    locale: &str,
    translations: &std::collections::HashMap<String, String>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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

#[allow(dead_code)]
fn build_embedded_resources(output_dir: &Path) {
    let assets_dir = Path::new("assets");
    let layouts_dir = Path::new("layouts");
    
    let mut resources = Vec::new();
    
    // 扫描资源文件
    if assets_dir.exists() {
        scan_directory(&assets_dir, "assets", &mut resources);
    }
    
    if layouts_dir.exists() {
        scan_directory(&layouts_dir, "layouts", &mut resources);
    }
    
    // 生成资源清单
    let manifest_path = output_dir.join("resources.json");
    let manifest_json = serde_json::to_string_pretty(&resources).unwrap();
    fs::write(&manifest_path, manifest_json).unwrap();
    
    println!("Generated resource manifest: {:?}", manifest_path);
}

#[allow(dead_code)]
fn scan_directory(base_path: &Path, prefix: &str, resources: &mut Vec<ResourceInfo>) {
    if let Ok(entries) = fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let relative_path = path.strip_prefix(base_path).unwrap();
            let resource_name = format!("{}/{}", prefix, relative_path.to_string_lossy());
            
            if path.is_file() {
                if let Ok(metadata) = fs::metadata(&path) {
                    let resource_type = determine_resource_type(&path);
                    let mime_type = determine_mime_type(&path);
                    
                    resources.push(ResourceInfo {
                        name: resource_name,
                        resource_type,
                        mime_type,
                        size: metadata.len() as usize,
                        path: path.to_string_lossy().to_string(),
                    });
                }
            } else if path.is_dir() {
                scan_directory(&path, &resource_name, resources);
            }
        }
    }
}

#[allow(dead_code)]
fn determine_resource_type(path: &Path) -> String {
    if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
        match extension.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "bmp" | "gif" | "ico" => "image".to_string(),
            "json" | "xml" | "txt" => "text".to_string(),
            "7z" | "zip" | "tar" | "gz" => "archive".to_string(),
            "ttf" | "otf" | "woff" | "woff2" => "font".to_string(),
            _ => "binary".to_string(),
        }
    } else {
        "binary".to_string()
    }
}

#[allow(dead_code)]
fn determine_mime_type(path: &Path) -> String {
    if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
        match extension.to_lowercase().as_str() {
            "png" => "image/png".to_string(),
            "jpg" | "jpeg" => "image/jpeg".to_string(),
            "bmp" => "image/bmp".to_string(),
            "gif" => "image/gif".to_string(),
            "ico" => "image/x-icon".to_string(),
            "json" => "application/json".to_string(),
            "xml" => "application/xml".to_string(),
            "txt" => "text/plain".to_string(),
            "7z" => "application/x-7z-compressed".to_string(),
            "zip" => "application/zip".to_string(),
            "ttf" => "font/ttf".to_string(),
            "otf" => "font/otf".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    } else {
        "application/octet-stream".to_string()
    }
}

#[allow(dead_code)]
fn generate_resource_code(output_dir: &Path) {
    let code_path = output_dir.join("embedded_resources.rs");
    let mut code = String::new();
    
    code.push_str("// 自动生成的资源代码\n");
    code.push_str("// 请勿手动修改此文件\n\n");
    code.push_str("use crate::resources::{EmbeddedResources, ResourceInfo, ResourceType};\n\n");
    
    // 读取资源清单
    let manifest_path = output_dir.join("resources.json");
    if let Ok(manifest_content) = fs::read_to_string(&manifest_path) {
        if let Ok(resources) = serde_json::from_str::<Vec<ResourceInfo>>(&manifest_content) {
            code.push_str("/// 获取内嵌资源\n");
            code.push_str("pub fn get_embedded_resources() -> EmbeddedResources {\n");
            code.push_str("    let mut resources = EmbeddedResources::new();\n\n");
            
            for resource in resources {
                code.push_str(&format!("    // {}\n", resource.name));
                code.push_str(&format!("    resources.add_resource(\"{}\".to_string(), ResourceInfo {{\n", resource.name));
                code.push_str(&format!("        name: \"{}\".to_string(),\n", resource.name));
                code.push_str(&format!("        resource_type: ResourceType::{:?},\n", resource.resource_type));
                code.push_str(&format!("        mime_type: Some(\"{}\".to_string()),\n", resource.mime_type));
                code.push_str(&format!("        size: {},\n", resource.size));
                code.push_str("        data: include_bytes!(\"../");
                code.push_str(&resource.path);
                code.push_str("\").to_vec(),\n");
                code.push_str("    });\n\n");
            }
            
            code.push_str("    resources\n");
            code.push_str("}\n");
        }
    }
    
    fs::write(&code_path, code).unwrap();
    println!("Generated resource code: {:?}", code_path);
}

/// 为 nano-installer.exe 设置图标
#[cfg(target_os = "windows")]
fn set_nano_installer_icon() {
    let icon_path = "assets/nano-installer.ico";
    let icon_file = Path::new(icon_path);
    
    if !icon_file.exists() {
        println!("cargo:warning=Icon file not found: {}", icon_path);
        return;
    }
    
    // 使用 winres 嵌入图标
    if let Err(e) = winres::WindowsResource::new()
        .set_icon(icon_path)
        .compile()
    {
        println!("cargo:warning=Failed to compile Windows resources: {}", e);
    }
}

/// 资源信息结构
#[derive(serde::Serialize, serde::Deserialize)]
struct ResourceInfo {
    name: String,
    resource_type: String,
    mime_type: String,
    size: usize,
    path: String,
}