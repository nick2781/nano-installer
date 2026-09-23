# 从 NSIS 迁移

一份 NSIS 安装脚本既是配置也是程序：`Section` 块按顺序跑，`File` 复制文件，`WriteRegStr` 写注册表，
用户看到的每一屏都是 NSIS 运行时按宏替你画的。Nano Installer 把这个脚本混在一起的三件事分开——产品是
什么、装到哪儿；页面长什么样；这一次运行到底做什么——最后一件交给你自己写的一小段 Rhai。所以迁移大
多不是重写，而是把每一行搬到如今该管它的那个地方。

这一页是那张对照表。表里的每一行不只是 [`scripts/check_nsi_migration.ps1`](../../scripts/check_nsi_migration.ps1)
的说明——那就是这个脚本读的表，所以脚本遇到这一页没有收录的写法时，会照实报 `unknown`，不去猜；中英
两份表也必须逐行对得上，对不上检查直接失败。

## 工程长什么样

| NSIS | Nano Installer |
| --- | --- |
| `setup.nsi` 与旁边那一堆 `!include` | `installer_config.json`、`layouts/` 下每页一个 XML、`locales/` 下每种语言一个 JSON，内置步骤不够用时再加 `scripts/install.rhai` 与 `scripts/uninstall.rhai` |
| 要装的文件，写成一条条 `File` | 一个 ZIP 或 7z 归档（`resources.payload_file`），外加每个可选组件一个归档 |
| `Section` 块与 `SectionIn` | `components.items`，页面上同名的 `Checkbox` 代表它 |
| `MUI_PAGE_*` 宏 | 自己写的页面，列在 `wizard.pages` 与 `wizard.uninstall_pages` 里 |
| `LangString` 与 `LoadLanguageFile` | `locales/<locale>.json` 与 `localization.supported_locales` |
| `MUI_LANGUAGE` | 同上 |
| `Uninstall` 段里一条条删掉安装时写的东西 | 机器上的 manifest，安装写下的每个文件、每个快捷方式、每个注册表值都在里面，卸载按记录收回 |
| `!finalize` 与 `!uninstfinalize` | `finalize.installer` 与 `finalize.uninstaller`——还是这两个钩子，还是同一件事 |
| 让插件去做 NSIS 做不到的事 | 脚本里的原语、声明出来的依赖，或者 `run_command`——这里没有插件 ABI |

有两件事过不来，值得早点决定。一是手写的 `Uninstall` 段不必再写：manifest 知道你装了什么，而靠人维护
一张删除清单，正是产品留垃圾的来路。二是 `SetShellVarContext all` 没有对应物——快捷方式建在运行安装
包的那个账户下，见下面的表。

## 先让检查器过一遍脚本

```powershell
.\scripts\check_nsi_migration.ps1 -Script .\legacy.nsi
```

它逐行读脚本，跟着读它旁边被 `!include` 进来的文件，把每条语句变成什么印出来：

```
NSIS migration report for D:\src\legacy.nsi

  direct   a configuration setting or a page element does this
  script   a primitive in scripts/install.rhai or scripts/uninstall.rhai does this
  manual   no equivalent: the guide says what to do instead
  none     build-time or cosmetic: nothing to migrate
  unknown  the table on this page has no row for it yet

     16  Name "${APP_NAME}"                              direct   project.name
     20  RequestExecutionLevel admin                     direct   install.require_admin
     38  !insertmacro MUI_PAGE_INSTFILES                 direct   the page the configuration gives role "progress"
     46  SetShellVarContext all                          manual   shortcuts go to the folders of the account that runs the setup
     63  WriteRegExpandStr HKLM "Software\App" "Data"     script   reg_write_expand_string
     72  Pop $0                                          manual   Rhai has variables; the NSIS stack has no counterpart

92 statement(s) read from 1 file(s)
  direct     31
  script     43
  manual      5
  none       13
  unknown     0

5 line(s) the guide has to answer for:
  legacy.nsi:19  InstallDirRegKey HKLM "${UNINST_KEY}" "InstallLocation"
  legacy.nsi:46  SetShellVarContext all
  ...
```

只要报告出得来，退出码就是 0，不管报告里写了什么——它是给人读的，不是一道闸。想让它当闸用就加
`-FailOnUnknown`：本仓库的 CI 就是这么跑 `examples/nsis-migration/legacy.nsi` 的，一份「每种写法都带
一条」的样例只有在表把它们全收下之后才有意义。

下面表里的判定词就是上面那五个：`direct` 是配置项或页面元素直接就能做，`script` 是
`scripts/install.rhai` 或 `scripts/uninstall.rhai` 里的某个原语，`manual` 是没有对应物、得按本页说的
另想办法，`none` 是构建期或纯外观、没有要迁移的东西，`unknown` 是这一页还没收。

## 对照表

一个命令有多种写法时共用一行。名字末尾的 `*` 表示后面接什么都算，所以 `MUI_PAGE_*` 接住了那些没有
单独一行的 MUI 页面。

### 产品与它构建出来的文件

| 命令 | 判定 | 变成什么 |
| --- | --- | --- |
| `Name` | direct | `project.name` |
| `OutFile` | direct | `output.installer_name`，或者构建时给的 `--output` |
| `InstallDir` | direct | `install.default_path`；`TextInput` 写 `value-source="config:install.default_path"`，用户就能改 |
| `InstallDirRegKey` | manual | 没有设置会把上次的目录读回来；原目录里的旧版本再装一次本来就走在位升级，真要读那个值可以用 `reg_read_expand_string` |
| `RequestExecutionLevel` | direct | `install.require_admin` |
| `Icon` | direct | `output.installer_icon` |
| `UninstallIcon` | direct | `output.uninstaller_icon` |
| `VIProductVersion` | direct | `project.file_version` |
| `VIAddVersionKey` | direct | `project.publisher`、`project.copyright`、`project.description` |
| `ManifestDPIAware` | direct | `ui.dpi_aware`；应用程序清单由构建器写 |
| `ManifestSupportedOS` | none | 安装包本来就声明支持 Windows 7 SP1 x64 及以后 |
| `SetCompressor`, `SetCompressorFinal` | none | 载荷由构建器自己压 |
| `SetDatablockOptimize` | none | 没有要迁移的东西 |
| `CRCCheck` | none | 载荷自带摘要；没签名的安装包在运行时也校验不了它自己 |
| `Unicode` | none | 一律是 Unicode |
| `Target` | none | 安装包是 x64 |
| `XPStyle` | none | 页面是自己画的 |
| `ShowInstDetails`, `ShowUninstDetails` | none | 任务在带 `role: "progress"` 的那一页上汇报 |
| `SetOverwrite` | none | 文件按载荷里的位置落地；升级就是替换上一版 |
| `BrandingText`, `SetBrandingImage` | none | 文字和图片都在页面上 |
| `InstallColors`, `SetFont`, `SetCtlColors` | none | 页面的颜色和字体自己带着 |
| `AutoCloseWindow`, `SetAutoClose` | none | 完成页由用户关 |
| `LoadLanguageFile`, `LangFile` | none | 运行时读 `locales/<locale>.json` |
| `LangString` | direct | `locales/<locale>.json` 里的一个字段，页面用 `text="@键"` 取 |
| `BringToFront` | none | 向导有自己的窗口 |

### 构建期，以及页面

| 命令 | 判定 | 变成什么 |
| --- | --- | --- |
| `!define`, `!undef`, `!searchparse`, `!searchreplace`, `!addincludedir` | none | 构建期的文本替换，正式跑之前就展开了 |
| `!macro`, `!macroend` | none | 宏体里的语句按它们本来写的位置逐条报出来 |
| `!insertmacro` | none | 脚本自己定义的宏；宏体里的语句按它们写的位置报 |
| `!addplugindir` | manual | 这里没有插件 ABI；插件做过的事要么是某个原语，要么是脚本跑的一条命令 |
| `!system`, `!execute` | manual | 构建器自己不在构建期间跑命令；`finalize.installer` 跑在成品上 |
| `!finalize` | direct | `finalize.installer` |
| `!uninstfinalize` | direct | `finalize.uninstaller` |
| `!packhdr` | manual | 构建完到签名之间没有东西会改写安装包 |
| `Page` | direct | `wizard.pages` 里的一项，版面写成 XML |
| `UninstPage` | direct | `wizard.uninstall_pages` 里的一项 |
| `PageCustom` | direct | 自己写的 XML 页面；页序有条件时再配 `scripts/pages.rhai` |
| `PageComponents` | direct | 一个页面，每个组件一个 `Checkbox` |
| `PageDirectory` | direct | 一个带 `TextInput` 与 `action="pick_directory"` 的页面 |
| `PageLicense` | direct | 一个放过许可协议文字或链接的页面 |
| `PageInstFiles` | direct | 配置里 `role: "progress"` 的那一页 |
| `PageEx` | direct | `wizard.pages` 里的一项 |
| `MUI_PAGE_*` | manual | 没有单独一行的 MUI 页面：用页面元素自己搭出来 |
| `MUI_UNPAGE_*` | manual | 没有单独一行的卸载页面：用卸载页面的元素自己搭出来 |
| `MUI_PAGE_WELCOME` | direct | `wizard.pages` 里的一页 |
| `MUI_PAGE_LICENSE` | direct | 一个放过许可协议文字或链接的页面 |
| `MUI_PAGE_COMPONENTS` | direct | 一个页面，每个组件一个 `Checkbox` |
| `MUI_PAGE_DIRECTORY` | direct | 一个带 `TextInput` 与 `action="pick_directory"` 的页面 |
| `MUI_PAGE_INSTFILES` | direct | 配置里 `role: "progress"` 的那一页 |
| `MUI_PAGE_FINISH` | direct | 配置里 `role: "finish"` 的那一页；`action="launch_app"` 跑起来的正是刚装好的程序 |
| `MUI_PAGE_STARTMENU` | direct | 一个页面，每个开始菜单选项一个 `Checkbox`，或者直接用快捷方式那几个设置 |
| `MUI_PAGE_UNINSTCONFIRM` | direct | 一个带「保留数据」复选框（`chkReserveData`）的页面 |
| `MUI_UNPAGE_CONFIRM` | direct | 带「保留数据」复选框（`chkReserveData`）的卸载页 |
| `MUI_UNPAGE_INSTFILES` | direct | 配置里 `role: "progress"` 的卸载页 |
| `MUI_UNPAGE_LICENSE` | direct | 放过许可协议文字的卸载页 |
| `MUI_UNPAGE_COMPONENTS` | direct | 每个组件一个 `Checkbox` 的卸载页 |
| `MUI_UNPAGE_DIRECTORY` | direct | 带 `TextInput` 与 `action="pick_directory"` 的卸载页 |
| `MUI_LANGUAGE` | direct | `localization.supported_locales`，配上同名的 `locales/<locale>.json` |

### 组件，以及一次运行真正做的事

| 命令 | 判定 | 变成什么 |
| --- | --- | --- |
| `Section` | direct | `components.items` 里的一个组件；写了 `SectionIn RO` 或名字里带 `!` 就是 `required: true` |
| `SectionEnd` | script | `install.rhai` 里那一段的结尾 |
| `SectionGroup` | direct | 一个把这一组组件画出来的页面 |
| `SectionGroupEnd` | none | 没有要迁移的东西 |
| `SectionIn` | direct | `RO` 对应 `required: true`，其余交给页面上的复选框 |
| `SectionInGroup` | direct | 页面画出来的那几个组件 |
| `SetOutPath` | none | 路径由载荷归档自己带着 |
| `File` | none | 把文件放进载荷归档，或者放进某个组件的归档 |
| `CreateDirectory` | script | `create_dir` |
| `RMDir` | script | `delete_dir` |
| `Delete` | script | `delete_file` |
| `CopyFiles` | script | `copy_file` |
| `Rename` | manual | 没有重命名原语：`copy_file` 再 `delete_file` |
| `WriteUninstaller` | direct | 卸载程序是工程自己的（`output.uninstaller_name`）；`install.rhai` 调 `copy_uninstaller()` |
| `CreateShortCut` | script | `create_desktop_shortcut`、`create_start_menu_shortcut` 或 `create_uninstall_shortcut` |
| `WriteRegStr` | script | `reg_write_string` |
| `WriteRegExpandStr` | script | `reg_write_expand_string` |
| `WriteRegDWORD` | script | `reg_write_dword` |
| `WriteRegBin` | script | `reg_write_binary` |
| `WriteRegNone` | manual | 没有原语写一个没有类型的值 |
| `ReadRegStr` | script | `reg_read`；值里带引用时用 `reg_read_expand_string` |
| `ReadRegDWORD` | script | `reg_read_dword` |
| `DeleteRegKey` | script | `reg_delete_key` |
| `DeleteRegValue` | script | `reg_delete_value` |
| `EnumRegKey`, `EnumRegValue` | manual | 没有原语列出一个键的子键或值 |
| `SetRegView` | direct | 键名上的 `HKLM64` / `HKLM32` 前缀 |
| `WriteINIStr`, `ReadINIStr`, `DeleteINISec` | manual | 没有 INI 原语：`read_text_file` 与 `write_file`，或者用 `run_command` 跑一条命令 |
| `RegDLL`, `UnRegDLL` | manual | 用 `run_command` 跑 `regsvr32` |
| `SetShellVarContext` | manual | 快捷方式建在运行安装包的那个账户下；没有全局开关 |
| `DetailPrint` | script | 写字面文本用 `set_status`，写语言键用 `set_status_key` |
| `MessageBox` | script | `show_message`、`show_error` 或 `ask_yes_no`，按 `ui.dialog_layout` 画在窗口里 |
| `Sleep` | script | `sleep_ms` |
| `GetTempFileName` | script | `get_temp_path`，再用 `path_join` 拼出文件名 |
| `GetSize` | script | `get_file_size` |
| `GetFileTime` | manual | 没有原语读文件时间 |
| `SetFileAttributes` | manual | 没有原语设文件属性 |
| `SearchPath` | manual | `get_env` 能取一个点名的变量；没有东西去搜路径列表 |
| `GetFullPathName` | manual | 收路径的原语都要求绝对路径 |
| `GetParent` | script | `path_parent` |
| `ReadEnvStr` | script | `get_env` |
| `ExpandEnvStrings` | none | 多数原语自己展开 `%NAME%`，注册表值则由 `reg_read_expand_string` 展开 |
| `FileRead` | script | `read_text_file` |
| `FileWrite` | script | `write_file` |
| `FileOpen`, `FileClose` | manual | 没有原语追加写文件，也没有东西替你拿着句柄 |
| `GetDLLVersion`, `GetDLLVersionLocal` | manual | 没有原语读文件版本；可以用 `run_command_output` 跑一个能读的程序 |
| `ExecWait` | script | `run_command`；要看退出码和输出就用 `run_command_output` |
| `Exec` | script | `run_detached` |
| `ExecShell` | manual | 没有原语打开网址或文档；页面上写了 `action="open_url:<links 的键>"` 的元素可以 |
| `Reboot`, `IfRebootFlag` | manual | 一次运行绝不重启机器；依赖答 3010 或 1641 就算装好了，继续往下走 |

### 插件、流程，以及 NSIS 自己的运行时

| 命令 | 判定 | 变成什么 |
| --- | --- | --- |
| `*::*` | manual | 插件调用：这里没有插件 ABI，插件做过的事要么是某个原语，要么是脚本跑的一条命令 |
| `nsProcess::_FindProcess` | script | `is_process_running` |
| `nsProcess::_KillProcess` | script | `kill_process` |
| `nsExec::Exec` | script | `run_command` |
| `nsExec::ExecToLog` | script | `run_command_output`——退出码与两个输出流都回得来 |
| `nsExec::ExecShellEx` | script | `run_command` |
| `EnvVar::set` | script | `set_env` |
| `EnvVar::unset` | script | `remove_env` |
| `Var` | script | Rhai 的 `let`；脚本自己的变量不用声明 |
| `Function`, `FunctionEnd` | script | `install.rhai` 或 `uninstall.rhai` 里的一个函数 |
| `Call` | script | Rhai 里的函数调用 |
| `Return` | script | `return` |
| `Goto` | manual | Rhai 有 `if`/`else` 和循环；标号没有对应物 |
| `Abort` | script | 用 `show_error` 说明原因之后从脚本 `return` |
| `Quit` | script | `return` |
| `SetErrors`, `ClearErrors` | none | 原语自己返回结果 |
| `SetErrorLevel` | manual | 无窗口运行的退出码是运行时自己的 |
| `IfErrors` | script | 原语自己返回结果 |
| `IfFileExists` | script | `file_exists` 与 `is_dir` |
| `IfSilent` | script | `get_mode()`，加上这次运行拿到的参数 |
| `StrCmp`, `StrCmpS`, `StrICmp` | script | Rhai 的 `==` |
| `StrCpy` | script | Rhai 的 `let` |
| `StrLen` | script | Rhai 的 `.len` |
| `StrReplace`, `StrStr`, `StrTok`, `StrTrim` | script | Rhai 的字符串方法 |
| `IntOp` | script | Rhai 的算术 |
| `IntCmp` | script | Rhai 的比较 |
| `IntFmt` | manual | 没有东西把数字补成定宽的字符串 |
| `Push`, `Pop`, `Exch` | manual | Rhai 有变量；NSIS 那个栈没有对应物 |
| `GetLabelAddress` | manual | 标号没有对应物 |
| `SetSilent`, `SilentInstall`, `SilentUninstall` | direct | `advanced.silent_mode_support` 与 `advanced.uninstall_mode_support`；无窗口运行就是 `--silent` |
| `${If}`, `${Unless}`, `${IfNot}`, `${AndIf}`, `${OrIf}` | script | Rhai 的 `if` |
| `${ElseIf}`, `${Else}` | script | Rhai 的 `else if` 与 `else` |
| `${EndIf}`, `${While}`, `${EndWhile}`, `${ForEach}`, `${Next}`, `${Do}`, `${Loop}`, `${Break}`, `${Continue}` | script | Rhai 自己的流程控制 |

## 没有对应物的那些，以及怎么办

费时间的正是这几条，所以值得先看。

- **插件。** `nsDialogs`、`nsisXML`、`InetLoad`、`nsis7z` 之类在这里没有对应物，因为这里没有插件 ABI。
  用 `nsDialogs` 搭出来的页面，改写成 `layouts/` 里的一页；行为还要看用户怎么选时，再写一个
  `scripts/pages.rhai` 的 `next_page(from)`。下载改成 `dependencies.items` 里带 `sha256` 的一条，或者
  脚本里的 `download_file_with_hash`。解压归档改成载荷或某个组件。
- **`SetShellVarContext all`。** 快捷方式建在运行安装包的那个账户下。原本写全局快捷方式的安装包得另找
  办法：即使 `require_admin` 装了全机器用的程序，快捷方式仍然写在跑这个安装包的那个账户下。
- **重启机器。** 这里没有任何东西会重启机器。依赖答 3010 或 1641 就算装好了，继续走；产品非重启不能
  用，就得自己在完成页上说清楚。
- **NSIS 的栈。** `Push`、`Pop`、`Exch` 之所以存在，是因为 NSIS 只有一个栈、没有函数自己的变量。
  Rhai 两个问题都没有。
- **窗口外的 `MessageBox`。** 这里的提示是按 `ui.dialog_layout` 画在向导里的卡片，脚本在工作线程上等
  这一下点击。工程没带这份版面就退回系统对话框；无窗口运行则一次也不弹。
- **`WriteINIStr`、枚举注册表、文件时间、注册 DLL。** 几个具体的小缺口，上面每一行都写清了顶替它的
  原语或命令。

## 一个完整的例子

`examples/nsis-migration/` 里放着一份有代表性的 NSIS 脚本，以及它迁移之后的工程：

```powershell
.\scripts\check_nsi_migration.ps1 -Script .\examples\nsis-migration\legacy.nsi -FailOnUnknown
```

`legacy.nsi` 有 111 行，检查器报 92 条语句：31 条由工程里的配置项直接顶替，43 条落在脚本原语上，
13 条是构建期或纯外观的，还有 5 条表上写着「没有对应物」。它迁移的结果是
`examples/nsis-migration/migrated/`：

- `installer_config.json`——产品、两个组件、那个依赖、安装登记和页面。`Name`、`OutFile`、`InstallDir`、
  `RequestExecutionLevel`、那几条 `VIAddVersionKey`、`SectionIn RO` 与 `Section /o` 都在这里。
- `layouts/`——七页：欢迎、选项、进度、完成、对话框，加上两个卸载页。`MUI_PAGE_COMPONENTS` 与
  `MUI_PAGE_DIRECTORY` 在这里合成了一页，因为这里的一页是一个版面，不是一个宏。
- `locales/zh-CN.json` 与 `locales/en-US.json`——原来 `LangString` 写下的东西。
- `scripts/install.rhai`——核心那段，按 NSIS 脚本原来的顺序走：关掉正在运行的那份、展开载荷与这次选
  中的组件、装上必需的依赖、跑随包带的迁移工具并在它失败时停下、按页面上的勾选建快捷方式、写下工程
  自己留着的那一个注册表值。
- `scripts/uninstall.rhai`——一次 `run_tracked_uninstall` 收走 manifest 记下的一切，再清掉产品自己
  往自己目录里写的那两处。

缩得最多的是卸载那段：八条 `Delete`、`RMDir`、`DeleteRegKey` 变成一次调用，因为安装时已经把写下的
东西记下来了。这一步值得最先做——靠人维护删除清单的产品，等哪天新版本多装了一个文件，就会把它留在
用户机器上。

## 迁移完就自带的东西

这套安装器本来就会做那些 NSIS 脚本通常手写的事，所以这些不必迁移：

- 卸载登记项——显示名、版本、发布者、图标、静默卸载命令、占用大小、`NoModify`、`NoRepair`——按
  `registry.uninstall_key` 与工程自己的信息写全。
- 安装写下的每个文件、快捷方式和注册表值都记进 manifest，卸载照着记录收回。
- 一步出错就把机器退回安装前的样子。
- 无窗口运行只要工程声明 `advanced.silent_mode_support`；它认 `--dir` 与 `--log`，别的参数一律拒绝。
- 每次运行都在磁盘上留一份日志，失败时把路径交出来。
- 再运行一次安装包就是原位升级；更新包可以只带字节变了的文件（`--delta-from`）。
- 签名是流水线的事，通过 `finalize.installer` 与 `finalize.uninstaller` 叫进来。

## 发布前过一遍

1. `check_nsi_migration.ps1` 报出的 `unknown` 是零。
2. 每一条 `manual` 都有结论——已经换个法子做了，或者有意不做。
3. 页面按原来 NSIS 的路子走得通：`wizard.pages`、进度页与完成页的 `role`，以及原本有条件的那几页交给
   `scripts/pages.rhai`。
4. 组件装出来的东西与原来那几个 section 一致，包括原本默认不勾的那个（`default: false`）。
5. 在一台一次性虚拟机里静默跑一遍：`MyApp_Setup.exe --silent --dir <路径> --log <文件>`，再
   `uninst.exe --silent`，然后读那份日志。
6. 卸载没有留下原来 `Uninstall` 段会删掉的东西。
