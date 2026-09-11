# 开发

## 环境

- Windows 10/11 x64
- Visual Studio Build Tools，包含 MSVC 和 Windows SDK
- Rust stable，组件 `rustfmt`、`clippy`

workspace 声明的最低 Rust 版本是 1.88，这是 `eframe 0.33` 的实际 MSRV；
`rust-toolchain.toml` 选择本地 stable，CI 则固定到已验证的 1.91.1。

## 常用命令

```powershell
cargo fmt --all -- --check
cargo test -p nano-installer-lib --lib
cargo test -p nano-installer-cli
cargo check --workspace --locked
```

完整 release 构建：

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

Win7 兼容构建需要固定 nightly 和 `rust-src`：

```powershell
rustup toolchain install nightly-2025-11-08 --component rust-src
.\scripts\build-win7.ps1 -Project examples\TapTap
```

只重打产品资源：

```powershell
.\scripts\build.ps1 -Project examples\TapTap -SkipStubs
```

## 构建边界

- 修改 `installer/**`：重编 `nano-installer-lzma`、`nano-installer-zlib`、`uninst` 和
  `nano-installer-cli`，然后重打 setup。
- 只修改产品项目的 assets、layouts、locales、scripts 或 config：release stubs
  未变化时可以只重打 setup。
- 修改 bundle 格式或 manifest：必须同时验证已有安装升级和卸载。

## 模块职责

| 目录 | 职责 |
| --- | --- |
| `config` | 强类型配置和校验 |
| `layout` | XML 解析与布局计算 |
| `ui` | egui 渲染与交互 |
| `resources` | bundle、payload 和 manifest |
| `installer` | 任务、产物记录和 Windows 集成 |
| `uninstaller` | manifest 驱动的卸载 |
| `scripting` | Rhai API 与产品脚本上下文 |

不要提交源码备份文件。实验代码使用分支，生成内容只放 `.build/`、`dist/` 或
`target/`。

## 变更完成标准

1. 格式检查通过。
2. library 和 CLI 测试通过。
3. release 三组件构建通过。
4. TapTap 资源 lint 和 setup 构建通过。
5. 涉及安装行为时执行 smoke；涉及 UI 时做 Windows 10/11 实机验收。
