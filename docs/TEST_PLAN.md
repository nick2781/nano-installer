# 测试

## 自动化门禁

每次提交至少运行：

```powershell
cargo fmt --all -- --check
cargo test -p nano-installer-lib --lib
cargo test -p nano-installer-cli
cargo check --workspace --locked
.\target\release\nano-installer.exe `
  harness lint-resources --project .\examples\TapTap --format text
```

修改 runtime 或 stub 后还要执行：

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

仓库中的自动化分为：

- Rust 单元/集成测试：配置、布局、bundle、manifest、脚本和 CLI。
- UI harness：无需打开窗口即可检查页面矩形、动作、locale 和资源。
- `scripts/smoke_test.ps1`：真实静默安装、升级和卸载主链路。
- `scripts/smoke_script_mode.ps1`：Rhai 模式副作用与 manifest 清理。
- `scripts/gui_real_test.ps1`：真实 GUI 启动链路。

## 发布前矩阵

| 维度 | 最低覆盖 |
| --- | --- |
| OS | Windows 10 22H2、Windows 11 当前生产版本 |
| DPI | 100%、150%、200% |
| Locale | 默认语言、英文、最长文案语言 |
| Mode | install、update、uninstall、silent install、silent uninstall |
| Path | 系统盘、非系统盘、中文路径、空格路径、无权限路径 |
| State | 首装、覆盖安装、旧版本升级、进程运行中、磁盘不足 |

Win7 当前不在这个矩阵内，因此任何 Win7 启动成功都不能当作正式支持证明。兼容性项目
启动后，应为 Win7 SP1 建立独立 VM 门禁。

Win7 构建会先运行 `scripts/audit_win7_imports.ps1`，禁止 Win8+ API、动态 UCRT 和
`combase.dll` 进入产物。PE 审计只能证明加载依赖基线，不能替代 Win7 SP1 VM 测试。

## 产物检查

- setup 文件名、图标、版本、publisher 与配置一致。
- 最终 setup 哈希已记录。
- setup 和解压后的 `uninst.exe` 都有有效 Authenticode 签名。
- 安装 manifest 完整记录文件、快捷方式和注册表副作用。
- 静默模式返回码可被部署平台识别。
- 卸载后只删除 manifest 所属内容，保留策略与产品要求一致。
