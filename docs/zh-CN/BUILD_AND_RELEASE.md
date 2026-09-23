# 构建与发布

## 唯一发布基线

CLI、运行时与所有生成的安装包都使用 `x86_64-win7-windows-msvc`，最低 Windows 7 SP1 x64。可选的
GUI 面向 Windows 10+ x64 主机，不会进入安装包或运行时。

```powershell
.\scripts\build.ps1
```

正式构建使用固定的 `nightly-2025-11-08` 工具链、`rust-src`、`-Z build-std`、`panic=abort` 与静态
CRT。构建完成后，脚本会拿两个运行时去解真实的 ZIP 与 7z 归档，比对 SHA-256；
`scripts\smoke_backends.ps1` 也能单独跑同样的检查。

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
DPI 行为是否与项目配置一致，所以悄悄丢掉提权声明的安装包会让构建失败。

发布流程会构建构建器、GUI 和运行时，把 5 个可执行文件各自作为 release asset 上传；它不生成压缩
包，也不构建或发布 TapTap 示例安装包。本地显式传 `-Project` 才会生成示例安装包，再加 `-Msi` 就会在
它旁边多写出一个企业按 MSI 分发的包，用来验证 payload、布局与 bundle；脚本还会取出项目化的
`uninst.exe`，单独审计它的 Windows 7 导入与版本资源。

示例 payload `examples/TapTap/payload/app.7z` 未存入仓库，所以上面的 CI job 只构建工具链。
安装包级验证放在独立的 `setup-end-to-end` job：`crates/nano-installer-core/tests/e2e_setup.rs`
自己写出一份项目、用它构建安装包，然后运行这个安装包与它部署出来的卸载程序。fixture 不含任何
产品 payload 与第三方素材，装到临时目录里，并注册到每个用例自己的注册表键下，所以这个 job 既不需要
虚拟机，也不会与其它运行互相干扰。

这个 job 会设置 `NANO_INSTALLER_E2E_REQUIRE_STUBS=1`，把「运行时 stub 缺失」从跳过改为失败；
否则一个什么都没构建的 job 会把所有用例都记为跳过，却依然显示通过。

每次运行还会先结束自己被替代的那些运行，这一步就是 `supersede` job，两个构建 job 都要等它。
`Test workspace` 偶尔会永远不结束，原因在构建机上而不在某个提交：步骤要等自己的输出关闭才算结束，
而这一步启动的某个进程有时会一直握着那份输出，构建机上任何超时都够不着这样一步；它待在那里的同时
也占着流水线的并发组，于是下一次推送只会在它后面排队，而不是开始构建。同一个提交重新派发就能通过、
同一次运行里另一个 job 是全绿的，所以这个 job 会在两个构建 job 之前，把同一分支上还在进行和排队的
运行强制取消掉。它允许失败：取消被 API 拒绝，不能让一次本来全绿的构建变成红的。

每个 job 还会执行 `scripts/audit_test_targets.ps1`：它向 Cargo 询问工作区包含哪些包，只要有
`tests/*.rs` 落在所有包之外就失败。虚拟清单旁边的 `tests/` 目录看起来像集成测试，却永远进不了
编译，其中的用例也就永远不会执行；这个坑在本仓库真实发生过，检查就是为这个加的。

## 版本号

版本号是发布当天的日期，采用 <https://calver.org/> 的 CalVer：完整年份 + 不补零的月 + 不补零的日，
例如 `2026.9.17`，tag 写作 `v2026.9.17`。日历日取项目自选的 UTC+08:00（CalVer 允许项目自选
日历日，写明即可），所以北京时间的深夜发布仍算当天。

`scripts/release_version.ps1` 是唯一的口径来源：

```powershell
# 今天该用哪个版本号
.\scripts\release_version.ps1
# 检查 Cargo.toml 里的版本号
.\scripts\release_version.ps1 -Version 2026.9.17
# 检查 tag、它标在当天的提交上，且与 Cargo.toml 的版本一致
.\scripts\release_version.ps1 -Tag v2026.9.17 -Version 2026.9.17 -Commit <sha>
```

构建一开始，`scripts/build.ps1` 就会校验 `Cargo.toml` 里的版本号；发布任务在构建前再用
`-Tag`/`-Commit` 校验一次 tag，要求它和 `Cargo.toml` 的版本号指向同一次发布，否则安装包内嵌的
版本资源会与它所属的 release 不符。同一个日历日第二次发布要加修饰后缀，例如 `v2026.9.17-r2`，
而不是把日期往后写一天或加第四段数字：CalVer 建议最多三段数字。补零的日期（`2026.09.17`）、
不存在的日期（`2026.13.1`）、早于要发布的那个提交或晚于今天的日期，都会被拦下，所以不会再出现
「今天才 9.17，却发出 9.19/9.20」这种版本号。

## 发布说明

Release notes 来自 `CHANGELOG.md`：`scripts/changelog_notes.ps1` 抽出与这次推送的 tag 匹配的段落。
打 tag 前先写好这个版本的 `## [YYYY.M.D]` 段落；找不到或段落为空会让发布步骤失败。

`scripts/changelog_notes.ps1` 带 UTF-8 BOM：它含有一行中文尾注，而 Windows PowerShell 会用 ANSI
代码页解码没有 BOM 的脚本。少了 BOM，这一行在 UTF-8 开发机上看不出问题，却会以乱码进入已发布的
release 正文。`scripts/audit_script_encoding.ps1` 会在每次构建开始时运行，发现含非 ASCII 文本却没
有 BOM 的脚本就让构建失败。`scripts/verify_release_notes.ps1` 则按发布流程的方式跑一遍生成器，并
把生成的尾注与源码中解码后的字面量逐字比对，从「发布出去的正文」这一侧再兜一次；它只在 ANSI
代码页不是 UTF-8 的机器上才会失败，所以放在 CI 上最有意义。

段落中 `<!-- release-notes:end -->` 之后是技术细节，只留在仓库日志里；发布出去的正文是标记之前
的产品向说明。需要发布完整段落时加 `-Full`。

## 构建器参数

```text
nano-installer-native-x64.exe build --project <dir> [--output <exe>] [--stubs <dir>] [--delta-from <archive>] [--msi <package>]
```

- `--project` 必需。
- `--output` 可选，默认输出到项目内的 `dist/<output.installer_name>`。
- `--stubs` 指向包含三个运行时的目录。
- `--delta-from` 点名上一版的 payload 归档，构建的是更新包而不是完整安装包，见下一节。
- `--msi` 把做完的安装包封成企业分发的那个 MSI 包。
- `NANO_INSTALLER_NATIVE_STUB_DIR` 可以覆盖运行时搜索目录。

## 构建时间与内存

安装包是把 payload 追加到自己后面做出来的：构建器按 1 MiB 分块读它，同一块一边写进安装包，一边喂给
捆绑数据里记下的那个 SHA-256。所以构建占用的内存不随 payload 增长，耗时大致随 payload 的字节数增长。
本机（Windows 11，x86_64，构建器与运行时都是 `cargo build --release` 的产物）实测：

| payload | 安装包 | 构建耗时 | 峰值工作集 |
| --- | --- | --- | --- |
| 8 MiB | 11.9 MiB | 1.5 s | 2.5 MiB |
| 128 MiB | 131.9 MiB | 2.0 s | 2.5 MiB |
| 512 MiB | 516 MiB | 3.1 s | 2.5 MiB |
| 1024 MiB | 1028.1 MiB | 4.7 s | 2.5 MiB |

安装包比 payload 大出来的那 4 MiB 左右，是三个运行时、工程自己的界面资源与内嵌卸载程序。工作集每
20 ms 采样一次，上表取整次构建里最高的那次。

```powershell
# 先构建发布版构建器，再量一次
cargo build --release -p nano-installer-native-cli
.\scripts\measure_build.ps1 -PayloadMiB 8,128,512,1024
```

脚本自己造一个 payload（ZIP 里一个 stored 条目，不压缩）、能构建成功的工程，逐档构建并记下耗时与峰值
工作集，写完 `target/build-cost.txt` 后把中间文件删掉。具体数值随机器、磁盘与 payload 的可压缩程度
变化，这里要钉住的是「内存不跟着 payload 走」——那是分块复制换来的。

## 更新包

```text
nano-installer-native-x64.exe build --project <dir> --delta-from <上一版的 payload 归档> [--output <exe>]
```

`--delta-from` 点名它要替代的那一版随安装包发出去的 payload 归档，也就是工程写在
`resources.payload_file` 里的那个文件。构建器用运行时分别展开这份归档和项目当前的 payload，按
「字节数 + SHA-256」判定哪些文件没变，只把变了的文件写成一份 ZIP，再以工程声明的 payload 名字嵌进
安装包——所以这份安装包按 ZIP 运行时打包，工程原来的 payload 是 7z 也一样。构建汇报里会给出留在
机器上的文件数、带走的文件数和这份归档的大小。

更新包只装在做出来的那一版之上。运行时会核对它没有带走的每个文件：在不在，字节数对不对，摘要是不是
那一个；核对在写任何文件之前做完，所以机器上不对的时候，用户看到的是「改用完整安装包」，而不是一个
半新半旧的产品。留在原地的文件照样记入 manifest，卸载时和其它文件一起收掉。

两条限制。内容切成组件的工程没有一个归档可以拿来比对，构建更新包时会被拒绝。更新包只覆盖 payload
里的文件，layouts、assets、locales、scripts 始终随安装包一起走，所以改了页面的那一版仍然是完整
安装包。

`BuildRequest.delta_from` 是同一件事的 API 入口，构建结果里的 `BuildResult.update` 回报保留了多少、
带走了多少。

安装包只负责装，不负责自己换掉自己。NSIS 也没有内置的更新器：用它的产品各自在家里决定什么时候
去问新版本，然后运行一个新的安装包。这个框架照同一条线走，所以配置里没有「检查更新」的开关。
要做自动更新的工程拿手上的原语自己搭：用 `download_file_with_hash` 取回新安装包并核对摘要，
用 `run_command` 把它静默跑起来；manifest 里的版本号就是机器上装的是哪一版的依据。

## 安装包外的 MSI

```text
nano-installer-native-x64.exe build --project <目录> --msi <安装包.msi>
```

`--msi` 把这次构建做完的安装包封成企业按 MSI 分发的那个包，域策略、Intune、Configuration Manager
都能直接推。封包发生在工程自己的命令处理过安装包之后，所以安装包签过名，包里带的镜像就是签过名的
那一份；构建日志会报出这个包的产品码和它默认装到哪个目录。

```powershell
# 无窗口装到指定目录
msiexec /i TapTap.msi /qn /norestart INSTALLDIR="D:\Programs\TapTap"
# 再卸掉；包会到它自己记下的位置去找产品
msiexec /x TapTap.msi /qn /norestart
```

工程要求管理员权限（`install.require_admin`）时装给整台机器，否则装给调用它的那个用户；不另指目录
时，落点是 `%ProgramFiles%\<名字>` 或 `%LOCALAPPDATA%\Programs\<名字>`。新版本的 MSI 会升级旧
版本 MSI 装出来的那份产品；不支持无窗口运行的工程会被拒绝出包，而不是拿到一个装不上的包。

管理安装（`msiexec /a`）只把包摊开、不装产品：这个 MSI 是安装包的投放载体，不是它的第二份安装。
通过 API 用的是同一件事：`BuildRequest.msi` 与 `BuildResult.msi`。

## 签名

构建器自己不签名，它把成品交给工程自己的命令去签：安装包在图标、版本资源和 bundle 全部写入之后
触发 `finalize.installer`，卸载程序在嵌进安装包之前、还是独立文件的时候触发 `finalize.uninstaller`。
两个字段的写法见[配置参考](CONFIG_REFERENCE.md#构建完成后的命令)；命令返回非零就中止构建，那个没签成的
文件也不会留在磁盘上。

`scripts/sign.ps1` 就是那条命令：用 `NANO_INSTALLER_CERT_THUMBPRINT` 指定的证书签名，时间戳默认打
`http://timestamp.digicert.com`（`NANO_INSTALLER_TIMESTAMP_URL` 可以换掉），签完立刻用
`signtool verify /pa` 自检一遍，证书没配或签名失败都报错退出。

```json
"finalize": {
  "uninstaller": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sign.ps1 -File \"%1\"",
  "installer": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sign.ps1 -File \"%1\""
}
```

签过名的安装包还是安装包：Authenticode 把证书表追加在文件所有内容之后，运行时读的是文件末尾那
一段里的尾标记，因此照常安装、照常卸载。
