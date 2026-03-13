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

    // 使用 winres 嵌入图标和子系统设置
    let mut res = winres::WindowsResource::new();
    res.set_icon(icon_path);

    // Release 模式下使用 Windows 子系统（无控制台）
    // Debug 模式下使用 Console 子系统（有控制台，方便调试）
    #[cfg(not(debug_assertions))]
    {
        // 注意：winres 会自动处理这个，我们通过 Cargo.toml 控制
        println!("cargo:warning=Building in RELEASE mode - GUI application (no console)");
    }

    #[cfg(debug_assertions)]
    {
        println!("cargo:warning=Building in DEBUG mode - Console application (for debugging)");
    }

    if let Err(e) = res.compile() {
        println!("cargo:warning=Failed to compile Windows resources: {}", e);
    } else {
        println!(
            "cargo:warning=Successfully set installer stub icon: {}",
            icon_path
        );
    }
}
