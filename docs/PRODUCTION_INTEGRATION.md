# 生产接入

## 推荐模型

把 `nano-installer` 当作有版本的构建工具，不要把产品代码写进 runtime。产品仓库维护
自己的 installer 目录，CI 下载或固定一版工具包：

```text
product-repo/
├── app-output/                 # 应用构建产物
└── installer/
    ├── installer_config.json
    ├── assets/
    ├── layouts/
    ├── locales/
    ├── scripts/
    └── payload/app.7z

build-tools/nano-installer/<version>/
├── nano-installer.exe
├── lzma-x64.exe
├── zlib-x64.exe
├── uninst-x64.exe
└── 7za.exe
```

可以从 `examples/TapTap` 复制完整结构，也可以用 `nano-installer init <name>` 创建骨架。
骨架仍需补齐图标、产品布局、语言和 payload 才能发布。

运行时仅支持 Unicode，不提供 ANSI stub。产品名、路径、注册表值和脚本文本应按 Unicode 输入处理。

完整 release bundle 同时提供两种压缩算法，产品流水线可以按需裁剪：

| Payload | 最小构建工具集合 |
| --- | --- |
| 7z/LZMA | `nano-installer.exe`、`lzma-x64.exe`、`uninst-x64.exe`、`7za.exe` |
| ZIP/Deflate | `nano-installer.exe`、`zlib-x64.exe`、`uninst-x64.exe` |

这里的 `x64` 是唯一受支持的架构；不是省略架构后的“通用”二进制。

## 构建步骤

```powershell
$toolDir = "C:\build-tools\nano-installer\0.1.0"
$projectDir = Join-Path $PWD "installer"
$env:NANO_INSTALLER_STUB_DIR = $toolDir

& "$toolDir\nano-installer.exe" validate `
  --config "$projectDir\installer_config.json"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& "$toolDir\nano-installer.exe" harness lint-resources `
  --project $projectDir --format text
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& "$toolDir\nano-installer.exe" build `
  --project $projectDir --release `
  --sign-script "$projectDir\ci\sign-artifact.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
```

CLI 也会从自身目录找所选的 installer/uninstaller stub，所以同目录部署时环境变量可以省略。

## Payload

生产流水线应先从应用构建产物生成全新的 `resources.payload_file`，通常是
`payload/app.7z`，再执行安装器构建。不要复用开发机残留的 payload。

CLI 在 payload 不存在且项目有 `files/` 时会自动打包，适合本地试用；生产构建应显式
生成和校验 payload，以便控制压缩参数、文件清单和 SHA-256。

CLI 按归档魔数校验 payload，而不是依赖扩展名：7z 必须选择 `lzma-x64.exe`，ZIP 必须
选择 `zlib-x64.exe`。非标准 stub 名称、旧的 `*-unicode.exe` 名称和任何 x86 名称都会
在清理旧产物前失败。

## 配置职责

- `installer_config.json`：名称、版本、安装路径、主 EXE、注册表、快捷方式、页面流。
- `layouts/`：控件结构、动作和视觉布局。
- `locales/`：所有发布语言文案，key 集合应保持一致。
- `scripts/`：渠道文件、URI scheme、私有数据保留等业务动作。
- `assets/`：ICO、背景、按钮和对应 `@2x` 图片。

详细字段和边界见[配置参考](CONFIG_REFERENCE.md)与
[配置和脚本边界](CONFIG_VS_SCRIPT.md)。

## 签名顺序

`--sign-script` 接收 `.exe`、`.cmd`、`.bat` 或 `.ps1`。CLI 会按以下顺序调用它：

1. 生成包含产品资源的 `uninst.exe`。
2. 调用 signer 签名并验证卸载器。
3. 无损压缩卸载器并嵌入 setup。
4. 完成 setup 图标、版本和 bundle 写入。
5. 调用 signer 签名并验证最终 setup。

仓库的 `scripts/sign.ps1` 使用证书存储区指纹和 SHA-256 可信时间戳。企业签名服务只需
提供一个接收单个文件路径、失败时返回非零退出码的 wrapper。

外部生产发布还需要从 CI 注入产品版本，保存 setup 哈希、构建日志和依赖版本，并在
目标 Windows、DPI 和 locale 矩阵执行安装、升级、静默与卸载测试。

## 升级策略

更新模式通过卸载注册表键检测已安装版本。生产项目必须保持以下标识稳定：

- `registry.install_path_key`
- `registry.uninstall_key`
- `install.mutex_name`
- `output.uninstaller_name`

变更这些字段会让新 setup 无法可靠识别旧版本。每次 schema、manifest 或脚本 API 变化
都应至少验证从上一生产版本升级，而不是只验证全新安装。
