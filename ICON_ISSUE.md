# nano-installer 图标问题排查

## 问题描述
`nano-installer.exe` 没有显示图标

## 已完成的检查

### 1. ✅ 图标文件存在
- `assets/nano-installer.ico` 存在
- 文件大小正常

### 2. ✅ Cargo.toml 配置
```toml
[target.'cfg(windows)'.build-dependencies]
winres = "0.1"
```

### 3. ✅ build.rs 配置
```rust
#[cfg(target_os = "windows")]
fn set_nano_installer_icon() {
    use winres::WindowsResource;
    
    let icon_path = "assets/nano-installer.ico";
    let mut res = WindowsResource::new();
    res.set_icon(icon_path);
    res.compile()
}
```

### 4. ✅ 只有一个 bin
- `Cargo.toml` 中只声明了 `nano-installer`
- `src/bin/` 下只有 `nano-installer.rs`
- `installer.exe` 已删除

## 可能的原因

### 原因 1: winres 需要特定的编译环境
`winres` 需要 Windows SDK 或 MSVC 工具链才能正确嵌入资源。

**解决方案**：
- 确保使用 `msvc` 工具链：`rustup default stable-msvc`
- 或安装 Visual Studio Build Tools

### 原因 2: 图标文件格式问题
ICO 文件可能格式不正确或不兼容。

**解决方案**：
- 检查 ICO 文件是否包含多种尺寸（16x16, 32x32, 48x48, 256x256）
- 使用专业工具（如 IcoFX）重新生成 ICO 文件

### 原因 3: 缓存问题
构建缓存可能导致资源未更新。

**解决方案**：
```powershell
cargo clean
cargo build --release --bin nano-installer
```

### 原因 4: winres 版本问题
`winres = "0.1"` 可能过旧。

**解决方案**：
更新 Cargo.toml：
```toml
[target.'cfg(windows)'.build-dependencies]
winres = "0.1.12"  # 或最新版本
```

### 原因 5: cargo-wix 或其他工具干扰
某些 cargo 插件可能覆盖资源设置。

## 诊断步骤

### 步骤 1: 检查编译输出
运行编译时注意 `cargo:warning` 信息：
```powershell
cargo build --release --bin nano-installer 2>&1 | Select-String "warning"
```

应该看到：
```
cargo:warning=Setting icon: assets/nano-installer.ico
cargo:warning=Successfully set icon for nano-installer.exe
```

### 步骤 2: 检查工具链
```powershell
rustup show
```

确认使用 `stable-x86_64-pc-windows-msvc`

### 步骤 3: 检查图标资源
使用 Resource Hacker 或类似工具打开 `nano-installer.exe`，查看是否有图标资源。

### 步骤 4: 手动嵌入图标
如果 winres 不工作，可以手动使用 Resource Hacker：
1. 编译 `nano-installer.exe`（无图标）
2. 使用 Resource Hacker 打开
3. Action → Add a new Resource
4. 选择 `nano-installer.ico`
5. 保存

## 临时解决方案

如果图标嵌入始终失败，可以：
1. 使用 post-build 脚本手动添加图标
2. 或在打包时使用 cargo-wix 等工具

## 测试命令

```powershell
# 运行测试脚本
.\test_icon_simple.ps1

# 或手动执行
cargo clean
cargo build --release --bin nano-installer
explorer /select,"target\release\nano-installer.exe"
# 右键 → 属性 → 查看图标
```

## 参考资料

- [winres crate documentation](https://docs.rs/winres/)
- [Resource Hacker](http://www.angusj.com/resourcehacker/)
- [Windows Icon 格式规范](https://docs.microsoft.com/en-us/previous-versions/ms997538(v=msdn.10))

