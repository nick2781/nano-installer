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
DPI 行为是否与项目配置一致，因此悄悄丢掉提权声明的安装包会让构建失败。

发布流程构建构建器、GUI 和运行时，并把 5 个可执行文件作为独立 release asset 上传；不会生成
压缩包，也不会构建或发布 TapTap 示例安装包。本地显式传 `-Project` 才会生成示例安装包，用于验证
payload、布局与 bundle；脚本还会取出项目化的 `uninst.exe`，单独审计其 Windows 7 导入与版本资源。

示例 payload `examples/TapTap/payload/app.7z` 未存入仓库，因此上面的 CI job 只构建工具链。
安装包级验证放在独立的 `setup-end-to-end` job：`crates/nano-installer-core/tests/e2e_setup.rs`
自己写出一份项目、用它构建安装包，然后运行这个安装包与它部署出来的卸载程序。fixture 不含任何产品
payload 与第三方素材，安装到临时目录下，并注册到每例独立的注册表键，因此该 job 既不需要虚拟机，
也不会与其它运行互相干扰。

该 job 会设置 `NANO_INSTALLER_E2E_REQUIRE_STUBS=1`，把「运行时 stub 缺失」从跳过改为失败；
否则一个什么都没构建的 job 会把所有用例都记为跳过，却依然显示通过。

每个 job 还会执行 `scripts/audit_test_targets.ps1`：它向 Cargo 询问工作区包含哪些包，只要有
`tests/*.rs` 落在所有包之外就失败。虚拟清单旁边的 `tests/` 目录看起来像集成测试，却永远不会被
编译，其中的用例也就永远不会执行；这个坑在本仓库真实发生过，检查就是为此而加。

## 版本号

版本号是发布当天的日期，采用 <https://calver.org/> 的 CalVer：完整年份 + 不补零的月 + 不补零的日，
例如 `2026.9.17`，tag 写作 `v2026.9.17`。日历日取项目自选的 UTC+08:00（CalVer 允许项目自选
日历日，写明即可），因此北京时间的深夜发布仍算当天。

`scripts/release_version.ps1` 是唯一的口径来源：

```powershell
# 今天该用哪个版本号
.\scripts\release_version.ps1
# 检查 Cargo.toml 里的版本号
.\scripts\release_version.ps1 -Version 2026.9.17
# 检查 tag、它标在当天的提交上，且与 Cargo.toml 的版本一致
.\scripts\release_version.ps1 -Tag v2026.9.17 -Version 2026.9.17 -Commit <sha>
```

构建开始时 `scripts/build.ps1` 会校验 `Cargo.toml` 的版本号；发布任务在构建前用 `-Tag`/`-Commit`
再校验一次 tag，并要求它与 `Cargo.toml` 的版本号指向同一个发布，否则安装包内嵌的版本资源会与
它所属的 release 不符。同一个日历日第二次发布要加修饰后缀，例如 `v2026.9.17-r2`，而不是把日期往后
写一天或加第四段数字：CalVer 建议最多三段数字。补零（`2026.09.17`）、不存在的日期
（`2026.13.1`）、以及日期早于被发布提交或晚于今天，都会被拦下，因此不会再出现「今天才 9.17，却
发出 9.19/9.20」这种版本号。

## 发布说明

Release notes 来自 `CHANGELOG.md`：`scripts/changelog_notes.ps1` 抽取与被推送 tag 匹配的段落。
打 tag 前先写好该版本的 `## [YYYY.M.D]` 段落；找不到或段落为空会让发布步骤失败。

`scripts/changelog_notes.ps1` 带 UTF-8 BOM：它含有一行中文尾注，而 Windows PowerShell 会用 ANSI
代码页解码没有 BOM 的脚本。少了 BOM，这一行在 UTF-8 开发机上看不出问题，却会以乱码进入已发布的
release 正文。`scripts/audit_script_encoding.ps1` 会在每次构建开始时运行，发现含非 ASCII 文本却没有 BOM 的脚本
就让构建失败。`scripts/verify_release_notes.ps1` 则按发布流程的方式跑一遍生成器，并把产出的尾注与
源码中解码后的字面量逐字比对，从「发布出去的正文」这一侧再兜一次；它只在 ANSI 代码页不是 UTF-8 的
机器上才会失败，所以放在 CI 上最有意义。

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
