# 发布

## 工具链发布

release bundle 包含：

```text
nano-installer.exe
lzma-x64.exe
zlib-x64.exe
uninst-x64.exe
7za.exe
```

这些文件可以平铺使用。CLI 优先从自身目录找 stub，也可以使用
`NANO_INSTALLER_STUB_DIR` 指向受控工具目录。
`7za.exe` 供 CLI 检查或自动创建 7z payload；LZMA runtime 已内嵌自己的副本，不要求
终端用户机器预装 7-Zip。

发布物仅提供 Unicode stub，不构建 ANSI 变体。stub 文件名中的维度只有压缩算法和架构。

必须用独立的 Cargo 命令分别构建三个 stub，避免 Cargo 合并共享库 features 后把所有 payload
后端链接进每个二进制。`scripts/build.ps1` 已按这个规则执行。

## 当前尺寸

以下是同一份 TapTap payload（压缩后 143.69 MiB）的本地 release 实测值：

| 产物 | 标准 x64 | Win7 x64 |
| --- | ---: | ---: |
| `lzma-x64.exe` | 6.95 MiB | 6.94 MiB |
| `zlib-x64.exe` | 4.83 MiB | 4.80 MiB |
| `uninst-x64.exe` | 4.80 MiB | 4.77 MiB |
| TapTap setup（LZMA） | 159.14 MiB | 159.12 MiB |

优化前同 payload 的 setup 为 170.66 MiB，当前标准构建减少约 11.52 MiB。stub 仍明显大于
NSIS 的原因和进一步缩到几百 KB 所需的架构变化见 [FAQ](FAQ.md)。

## 发布前命令

```powershell
cargo fmt --all -- --check
cargo test -p nano-installer-lib --lib
cargo test -p nano-installer-cli
cargo check --workspace --locked
.\scripts\build.ps1 -Project examples\TapTap
```

GitHub Actions 的 CI 与 release workflow 固定使用 Rust 1.91.1，并只构建 TapTap 示例。

## 版本与可复现性

- 提交并使用 `Cargo.lock`。
- 构建使用 `--locked`。
- release tag 对应唯一工具链版本和 commit。
- 产品配置中的版本应由生产流水线更新并校验。
- 保存最终 setup、SHA-256、签名时间戳和构建日志。

## 签名

最终 setup 必须在所有图标、版本资源和 bundle 追加完成后签名。嵌入 setup 的卸载器也
必须单独签名。构建命令通过同一个 signer 依次签名卸载器和最终 setup：

```powershell
.\scripts\build.ps1 -Project examples\TapTap -SignScript scripts\sign.ps1
```

默认 signer 从 `NANO_INSTALLER_CERT_THUMBPRINT` 读取证书指纹；生产环境也可以传入企业
签名服务的 `.exe`、`.cmd` 或 `.ps1` wrapper。签名后的卸载器再被无损压缩并嵌入。

## 发布验收

按[测试计划](TEST_PLAN.md)完成 Windows 10/11、DPI、语言、首装、升级、静默和卸载
矩阵。系统支持口径必须与[Windows 兼容性](WINDOWS_COMPATIBILITY.md)一致。
