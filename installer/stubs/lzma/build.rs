// 构建脚本 - 为 lzma-x64-unicode 安装器 stub 设置图标

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../../assets/stub.ico");
    
    // 为安装器 stub 设置图标
    #[cfg(target_os = "windows")]
    {
        set_icon();
    }
}

#[cfg(target_os = "windows")]
fn set_icon() {
    let icon_path = "../../../assets/stub.ico";
    let icon_file = Path::new(icon_path);
    
    if !icon_file.exists() {
        println!("cargo:warning=Icon file not found: {}", icon_path);
        println!("cargo:warning=Using default icon for installer stub");
        return;
    }
    
    // 使用 winres 嵌入图标
    if let Err(e) = winres::WindowsResource::new()
        .set_icon(icon_path)
        .compile()
    {
        println!("cargo:warning=Failed to compile Windows resources: {}", e);
    } else {
        println!("cargo:warning=Successfully set installer stub icon: {}", icon_path);
    }
}


