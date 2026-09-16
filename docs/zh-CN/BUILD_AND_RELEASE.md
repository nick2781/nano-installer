# 构建与发布

## 唯一发布基线

CLI、运行时与所有生成的安装包都使用 `x86_64-win7-windows-msvc`，最低 Windows 7 SP1 x64。可选的
GUI 面向 Windows 10+ x64 主机，不会进入安装包或运行时。

```powershell
.\scripts\build.ps1
```

正式构建使用固定的 `nightly-2025-11-08` 工具链、`rust-src`、`-Z build-std`、`panic=abort` 与静态
CRT。构建完成后，脚本会用两个运行时解开真实 ZIP 与 7z 归档并比对 SHA-256；
`scripts\smoke_backends.ps1` 可以单独执行同样的检查。

## 发布内容

```text
target/release/
├── nano-installer-native-x64.exe      # 构建器
├── nano-installer-gui-x64.exe         # Windows 10+ 可视化构建工具
└── stubs/
    ├── lzma-stub-native.exe
    ├── zlib-stub-native.exe
    └── uninst-stub-native.exe
```

运行时不含产品资源，图标、版本信息与应用程序清单都按项目注入。

构建安装包时会顺带审计：`scripts/audit_application_manifest.ps1` 读回清单资源，核对权限级别与
DPI 行为是否与项目配置一致，因此悄悄丢掉提权声明的安装包会让构建失败，而不是被发布出去。

发布流程构建构建器、GUI 和运行时，并把 5 个可执行文件作为独立 release asset 上传；不会生成
压缩包，也不会构建或发布 TapTap 示例安装包。本地显式传 `-Project` 才会生成示例安装包，用于验证
payload、布局与 bundle；脚本还会取出项目化的 `uninst.exe`，单独审计其 Windows 7 导入与版本资源。

示例 payload `examples/TapTap/payload/app.7z` 未存入仓库，因此 CI 只构建工具链；使用真实 payload
的自动化测试计划放在独立的 job。

## 发布说明

Release notes 来自 `CHANGELOG.md`：`scripts/changelog_notes.ps1` 抽取与被推送 tag 匹配的段落。
打 tag 前先写好该版本的 `## [YYYY.M.D]` 段落；找不到或段落为空会让发布步骤失败，而不是发出空正文。

`scripts/changelog_notes.ps1` 带 UTF-8 BOM：它含有一行中文尾注，而 Windows PowerShell 会用 ANSI
代码页解码没有 BOM 的脚本。少了 BOM，这一行在 UTF-8 开发机上看不出问题，却会以乱码进入已发布的
release 正文。`scripts/audit_script_encoding.ps1` 会在每次构建开始时运行，发现含非 ASCII 文本却
没有 BOM 的脚本就让构建失败。

段落中 `<!-- release-notes:end -->` 之后是技术细节，只留在仓库日志里；发布出去的正文是标记之前的
产品向说明。需要发布完整段落时加 `-Full`。

## 构建器参数

```text
nano-installer-native-x64.exe build --project <dir> [--output <exe>] [--stubs <dir>]
```

- `--project` 必需。
- `--output` 可选，默认输出到项目内的 `dist/<output.installer_name>`。
- `--stubs` 指向包含三个运行时的目录。
- `NANO_INSTALLER_NATIVE_STUB_DIR` 可以覆盖运行时搜索目录。

## 签名

构建器目前不做任何签名。生产发布必须在图标、版本资源和 bundle 全部写入之后签安装包，并在嵌入
之前单独签卸载程序。`scripts/sign.ps1` 只是预留的签名脚本，默认构建不会调用它。
