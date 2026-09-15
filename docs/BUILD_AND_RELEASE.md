# 构建与发布

## 唯一发布基线

CLI、stubs 和生成的 setup 使用 `x86_64-win7-windows-msvc`，最低 Windows 7 SP1 x64。
可选 GUI 使用 Windows 10+ host x64 构建，但不会进入 setup/runtime。

```powershell
.\scripts\build.ps1
```

正式构建使用 `nightly-2025-11-08`、rust-src、build-std、panic=abort 和静态 CRT。
构建完成后会自动用真实 ZIP 和 7z 小归档验证两个 release stub，并比较解压文件的
SHA-256；也可以单独运行 `.\scripts\smoke_backends.ps1`。

## 发布目录

```text
target/release/
├── nano-installer-native-x64.exe      # builder
├── nano-installer-gui-x64.exe         # Windows 10+ GUI
└── stubs/
    ├── lzma-stub-native.exe
    ├── zlib-stub-native.exe
    └── uninst-stub-native.exe
```

原始 stubs 不含产品资源；生成的 setup 与自包含 uninstaller 的资源由项目 `output` 配置决定。

Release workflow 构建并打包 builder、GUI 与 `stubs/`，不会生成或发布 TapTap setup。
开发侧显式传 `-Project` 时才生成 example setup，用于验证 payload、layout 和 bundle；脚本
也会从 setup 中取出项目化 `uninst.exe` 并单独审计其 Win7 PE 导入和 VERSIONINFO。

example payload `examples/TapTap/payload/app.7z` 未入库（被 `.gitignore` 的 `*.7z` 排除），
因此 CI 只构建工具链，不做 setup 端到端验证；真实 payload 的自动化测试计划放在独立的测试 job。

## Builder 参数

```text
nano-installer-native-x64.exe build --project <dir> [--output <exe>] [--stubs <dir>]
```

- `--project` 必需。
- `--output` 可选；缺省为项目 `dist/<output.installer_name>`。
- `--stubs` 可选；指定包含 `lzma-stub-native.exe`、`zlib-stub-native.exe` 和
  `uninst-stub-native.exe` 的目录。
- `NANO_INSTALLER_NATIVE_STUB_DIR` 可覆盖 stub 搜索目录。

## 签名状态

当前 native builder 尚未接入签名回调。生产版本必须在图标、版本资源和 bundle 全部写入
后签 setup，并在嵌入前单独签 uninstaller。仓库中的 `scripts/sign.ps1` 只是预留 signer，
当前默认构建不会调用它。
