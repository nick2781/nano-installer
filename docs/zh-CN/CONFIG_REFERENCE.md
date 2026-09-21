# 配置参考

每个项目有一个 `installer_config.json`。这一页只列现在真的会改变安装包行为的设置。本项目不读的键
会让构建失败，而不是被安静地忽略，所以摆在眼前的配置就是安装包实际的行为；它拒绝的键列在最后。

## 产品信息

| 字段 | 类型 | 作用 |
| --- | --- | --- |
| `project.name` | string | 产品名称；同时作为默认文件说明 |
| `project.version` | string | 你的产品版本，可带发布后缀 |
| `project.file_version` | string | Windows 文件/产品版本，1-4 段数字 |
| `project.description` | string | 可选的文件说明，默认 `<project.name> Installer` |
| `project.output_name` | string | 内部名称，默认取 `project.name` |
| `project.publisher` | string | 文件属性中的公司名 |
| `project.copyright` | string | 文件属性中的版权信息 |

没有配置 `project.file_version` 时，版本资源用 `project.version`，这时它必须是纯数字。

## 输出文件

| 字段 | 类型 | 作用 |
| --- | --- | --- |
| `output.installer_name` | string | 未传 `--output` 时的安装包文件名 |
| `output.installer_icon` | string | 可选的 ICO，用于安装包文件、任务栏与 Alt+Tab |
| `output.uninstaller_name` | string | 内嵌卸载程序的文件名，默认 `uninst.exe`；必须是不含目录的文件名 |
| `output.uninstaller_icon` | string | 可选，写入生成的卸载程序的 ICO |

路径都相对项目目录解析。

## 安装行为

| 字段 | 类型 | 作用 |
| --- | --- | --- |
| `install.default_path` | string | 首屏显示的初始安装目录 |
| `install.required_space_mb` | integer | 所需空间，单位 MiB：目标盘剩余空间不足时，安装会在写任何文件之前中止；同一个数字也可以用 XML 的 `value-source` 显示 |
| `install.exe_name` | string | payload 中必须存在的应用 EXE；缺失时直接中止，不部署任何文件 |
| `install.require_admin` | bool | 安装包启动前向 Windows 申请管理员权限，默认 `false` |
| `install.kill_process_on_install` | bool | 安装前关闭正在运行的产品；无法关闭则安装失败 |
| `install.kill_process_on_uninstall` | bool | 卸载前关闭正在运行的产品 |
| `install.detect_running_process` | bool | 与上面两个开关取或；任一为 `true` 都会关闭进程 |
| `registry.uninstall_key` | string | 卸载项注册表路径，只支持 HKCU/HKLM；已有同名键不会被覆盖 |
| `links.*` | string | 布局通过 `[文字](键)` 标记或 `action="open_url:键"` 打开的网址 |

## 快捷方式与自启动

| 字段 | 类型 | 作用 |
| --- | --- | --- |
| `shortcuts.desktop_shortcut` | bool | 是否允许创建桌面快捷方式 |
| `shortcuts.desktop_default` | bool | `chkShotcut` 的默认勾选状态，默认 `true` |
| `shortcuts.start_menu` | bool | 是否在开始菜单创建产品与卸载入口 |
| `shortcuts.start_menu_folder` | string | 开始菜单子目录名，默认 `project.name`；只删除本次安装创建且已清空的目录 |
| `autostart.enabled` | bool | 是否允许写入自启动项 |
| `autostart.default` | bool | `chkAutoRun` 的默认勾选状态，默认 `false` |
| `autostart.registry_key` | string | 自启动注册表路径，默认 `HKCU\...\CurrentVersion\Run` |
| `autostart.registry_value_name` | string | 自启动值名称，默认 `project.name` |

复选框出不出现，你的布局说了算：布局里没有 `chkShotcut` 或 `chkAutoRun` 时，就用上面的默认值。
安装时会把这两项记下来，卸载后机器回到安装前的状态。

## 文件、语言与页面

| 字段 | 类型 | 作用 |
| --- | --- | --- |
| `resources.layouts_dir` | string | 要打包的 XML 目录，默认 `layouts` |
| `resources.assets_dir` | string | 要打包的图片目录，默认 `assets` |
| `resources.locales_dir` | string | 语言 JSON 目录，默认 `locales` |
| `resources.payload_file` | string | 必需；ZIP 或 7z payload 路径 |
| `resources.tools_dir` | string | 可选；要打包的辅助程序目录，脚本用 `get_tools_dir()` 取回 |
| `localization.default_locale` | string | 启动时使用的语言，默认 `zh-CN` |
| `localization.supported_locales` | array | 计划提供的语言列表；构建时会对没有对应 JSON 文件的条目告警 |
| `wizard.pages[].layout` | string | 安装页列表，用 `action="next"` 与 `action="back"` 逐页走 |
| `wizard.pages[].role` | string | 可选：`progress` 标记任务汇报的那一页，`finish` 标记收尾的那一页 |
| `wizard.uninstall_pages[].layout` | string | 卸载页列表，走法相同 |
| `wizard.uninstall_pages[].role` | string | 卸载程序可用的同样两个职责 |

不写 `role` 时，第二页汇报、最后一页收尾，也就是普通工程声明的那三页的意思。页面还可以写
`id` 与 `title`：前者是给你自己看的名字，后者是报告里给它的小标题。

## 界面

| 字段 | 类型 | 作用 |
| --- | --- | --- |
| `ui.dpi_aware` | bool | 是否启用 DPI 感知与布局缩放，默认 `true` |
| `ui.dpi_threshold` | integer | 达到该 DPI 时优先使用 `@2x` 图片，默认 `144` |
| `ui.dialog_layout` | string | 确认框使用的布局文件，默认 `layouts/msgBox.xml` |

`ui.dpi_aware` 还会写进安装包的应用程序清单，让系统知道窗口自己会缩放，而不是把窗口当成一张位图
去拉伸。打开后清单申请逐显示器感知：窗口被拖到缩放比例不同的显示器上时，界面按那块显示器重新
排版，文字和图片保持清晰。关闭时清单写 `unaware`，缩放统一交给系统做。

`ui.dpi_threshold` 决定什么时候改用 `@2x` 图片，跟显示器无关，只看当前生效的 DPI。窗口移到另一块
显示器上之后，会按新的 DPI 重新判断。

`ui.dialog_layout` 指向的那份布局画在窗口内部：「确定要退出安装？」这类提问、需要用户知晓的提示，
以及脚本用 `show_message`、`show_error`、`ask_yes_no` 说出去的提示与提问，都用它，所以对话框和
安装界面是同一套皮肤，也不会跑到安装窗口后面。项目不提供这份布局时，确认框不再弹出，点关闭按钮
直接退出，脚本的消息退回系统对话框。布局怎么写见[页面布局](XML_LAYOUT_GUIDE.md#对话框)。

## 管理员权限

`install.require_admin` 决定安装包的 `requestedExecutionLevel`：

```json
"install": { "require_admin": true }
```

- `true` 写入 `requireAdministrator`：安装包启动前 Windows 会弹出 UAC 确认框，窗口以提权身份
  运行。安装目录位于 `Program Files` 时选它。
- `false`（默认）写入 `asInvoker`：不弹框，使用用户当前的权限。按用户安装到 `%LOCALAPPDATA%`
  的产品选它。

内嵌的卸载程序用同一份配置生成，所以申请同样的权限级别；否则卸载项无法撤销一次提权安装。

## 卸载时的用户数据

`uninstall.data_paths` 列出产品自己的数据目录。卸载时只有用户取消勾选保留数据才会删，展开之后
不落在 `%APPDATA%` 或 `%LOCALAPPDATA%` 之下的条目就忽略，免得误删无关文件。布局里没有
`chkReserveData` 时，用户数据一律保留。

```json
"uninstall": {
  "data_paths": ["%APPDATA%\\MyApp", "%LOCALAPPDATA%\\MyApp"]
}
```

## payload 格式

payload 就是你的应用文件，事先压成 ZIP 或 7z。格式按文件头认，不按扩展名：`PK` 选 ZIP 运行时，
`37 7A BC AF 27 1C` 选 7z 运行时。其他格式会让构建失败。

## 组件

`resources.payload_file` 是每次安装都会装上的基础载荷。想再让用户挑一部分装，就用 `components.items`
把它切成若干组件：

```json
"components": {
  "items": [
    { "id": "core", "payload": "payload/core.7z", "required": true },
    { "id": "docs", "payload": "payload/docs.7z" },
    { "id": "samples", "payload": "payload/samples.7z", "default": true }
  ]
}
```

| 字段 | 类型 | 作用 |
| --- | --- | --- |
| `components.items[].id` | string | 组件名；页面上同名的复选框代表它，脚本用 `is_component_selected()` 问它 |
| `components.items[].payload` | string | 该组件自己的 ZIP 或 7z 归档 |
| `components.items[].default` | bool | 页面上没有这个复选框（静默安装也算）时装不装，默认 `false` |
| `components.items[].required` | bool | 必需组件：页面上勾不掉，也没有装不装的问题，默认 `false` |

一个组件装不装，按这个顺序定：写了 `required` 的一律装；页面上有同名复选框就听页面的，写法见
[页面布局](XML_LAYOUT_GUIDE.md#可滚动容器)；页面没有这个复选框（静默安装也算）就听 `default`。所以
同一个安装包在窗口里跑和带 `--silent` 跑，装出来的东西可以不一样，这正是 `default` 的用处。

组件是基础载荷之外的部分，因此每个组件的归档要与基础载荷同格式：整个安装包由一个运行时解压，混用
ZIP 与 7z 会在构建时被拒绝。安装时基础载荷先落地，各组件再依次解到同一个目录；两个归档带同一个相对
路径会当场失败，而不是按声明顺序互相覆盖。构建还会拒绝这样的组件：没有 `id` 或 `payload`；`id` 或
`payload` 与另一个组件重复；`payload` 就是 `resources.payload_file`；以及写了 `required: true` 又写
`default: false`（必需的组件不看默认值）。

## 依赖

产品需要的 VC++ 运行库、WebView2 运行库、某个 .NET Framework 版本不属于产品自己：它们装在机器上
一次，供机器上所有产品共用。`dependencies.items` 声明这类依赖——怎么判断机器上有没有，以及没有时
用哪个程序把它装上。

```json
"dependencies": {
  "items": [
    {
      "id": "vcredist_x64",
      "detect": {
        "registry": {
          "key": "HKLM\\SOFTWARE\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x64",
          "name": "Installed",
          "equals": "1"
        }
      },
      "payload": "payload/vc_redist.x64.exe",
      "arguments": ["/install", "/quiet", "/norestart"],
      "required": true
    },
    {
      "id": "webview2",
      "detect": {
        "file": "%ProgramFiles(x86)%\\Microsoft\\EdgeWebView\\Application\\msedgewebview2.exe"
      },
      "download": {
        "url": "https://go.microsoft.com/fwlink/?linkid=2124703",
        "sha256": "e5f5a4b0b1b7c0d4a4f0d2a0f9c1e8b6d3a7c2f4b8e6d1a3c5f7b9d0e2a4c6f8"
      },
      "arguments": ["/silent", "/install"]
    }
  ]
}
```

| 字段 | 类型 | 作用 |
| --- | --- | --- |
| `dependencies.items[].id` | string | 依赖名；脚本用 `dependency_installed()`、`install_dependency()` 问它 |
| `dependencies.items[].detect` | object | 判断机器上有没有它，`file` 或 `registry` 二选一 |
| `dependencies.items[].payload` | string | 随安装包带上的安装程序，必须是 `.exe` |
| `dependencies.items[].download` | object | 安装时现取的安装程序，见下 |
| `dependencies.items[].arguments` | array | 跑安装程序时带的参数，默认不传 |
| `dependencies.items[].required` | bool | 必需依赖：装不上就中止安装，默认 `false` |

`detect` 有两种写法，一次只说一件事：

- `{ "file": "%ProgramFiles(x86)%\\...\\msedgewebview2.exe" }`：这个文件在就算有。路径里的环境
  变量会先展开。
- `{ "registry": { ... } }`：读注册表。`key` 要打开的键，`HKCU` 或 `HKLM` 开头；`name` 要读的值，
  不写就只问这个键在不在；写了 `name` 还能比一次，`equals` 是逐字相等，`at_least` 按点分数字比较。
  值的类型是文本还是 DWORD 都认，所以 VC++ 运行库写的 `1`、WebView2 写的版本号、.NET 记的那串数字
  都能比。`at_least` 里缺的段算 0，写 `14.0.1` 与写 `14.0.1.0` 是一回事。

`payload` 与 `download` 二选一。带在包里的那个由构建收进安装包，安装时解到临时目录再跑；`download`
的那个在安装时现取，取回来先按下 `sha256` 校验，对不上就把文件删掉，绝不执行——所以 `sha256` 是必填
的，用 `certutil -hashfile <文件> SHA256` 之类算出来填进去。URL 最后一段是可执行文件名时按它命名，
否则用依赖自己的 id 加 `.exe`，Windows 能跑起来才有意义。

依赖在 payload 之前处理：内置流程先逐个检查，缺的装上，都就绪了才开解压。装不上的依赖，写了
`required: true` 的中止安装并说明原因，此时产品一个字节都还没写；没写的记一条日志继续装。安装程序
报告「已装过更新版本」（退出码 1638）或「装好了但要重启」（3010、1641）都算成功，其余非零退出码算
失败。内置流程处理依赖时的状态文案取 locale 的 `status.dependencies` 键。

依赖不随产品卸载：它属于机器，装它的也不止这一个产品。

工程带了 `scripts/install.rhai` 时内置流程不再插手，脚本自己决定什么时候检查、要不要装，用
`dependency_installed()` 与 `install_dependency()`，读的是同一份声明，见[脚本 API](SCRIPT_API.md#依赖与下载)。

## 自定义安装与卸载步骤

加入 `scripts/install.rhai` 或 `scripts/uninstall.rhai` 即可替代内置步骤，见
[脚本 API](SCRIPT_API.md)。

## 无人值守运行

项目可以完全不开窗口就把安装或卸载做完。前提是项目自己先声明：

```json
"advanced": {
  "silent_mode_support": true,
  "uninstall_mode_support": true
}
```

打开开关后，同一批可执行文件就接受 `--silent`：

```powershell
MyApp_Setup.exe --silent --dir "%LOCALAPPDATA%\MyApp"
MyApp_Setup.exe --silent
uninst.exe --silent
```

- `--silent` 全程不开任何界面。静默运行绝不弹框，因为弹框会等一个永远不会到来的点击。
- `--dir` 指定本次安装目录，优先级高于 `install.default_path`。两者都支持环境变量，使用前会先
  展开。
- 其它任何参数都会被拒绝。写错的参数会让运行直接失败，而不是静默装到配置的默认目录。
- `silent_mode_support` 与 `uninstall_mode_support` 互相独立：产品可以允许无人值守安装，
  但不允许无人值守卸载。
- 没有复选框可读，所以快捷方式与自启动按 `shortcuts.desktop_default` 与 `autostart.default`
  取值，卸载时用户数据一律保留。

无窗口运行会把过程汇报给启动它的那个程序：写控制台，不画窗口；失败时返回非零退出码，并把原因写
到标准错误。

## 最小配置

```json
{
  "project": {
    "name": "MyApp",
    "version": "2026.9.22-rel.1",
    "file_version": "2026.9.22",
    "publisher": "Example Company",
    "copyright": "Copyright 2026 Example Company"
  },
  "output": {
    "installer_name": "MyApp_Setup.exe",
    "installer_icon": "assets/logo.ico"
  },
  "resources": {
    "layouts_dir": "layouts",
    "assets_dir": "assets",
    "locales_dir": "locales",
    "payload_file": "payload/app.7z"
  },
  "localization": { "default_locale": "zh-CN" },
  "install": { "default_path": "C:\\Program Files\\MyApp", "require_admin": true },
  "ui": { "dpi_aware": true, "dpi_threshold": 144 },
  "wizard": {
    "pages": [
      { "id": "config", "layout": "layouts/configpage.xml" }
    ]
  }
}
```

## 会被拒绝的设置

写了却能不起作用的设置，比没有这个设置更糟：工程看起来是好的，发出去的安装包却没有这一条。
所以构建器拒绝含自己不读的键的配置，并指出该改用哪个设置。

| 被拒绝的键 | 该改成什么 |
| --- | --- |
| `install.append_to_path` | 目前没有任何设置能往 PATH 里加目录 |
| `install.mutex_name` | 安装包目前不会在同一个产品的另一份安装运行时拒绝启动 |
| `registry.install_path_key` | 安装路径不会被写进注册表值 |
| `registry.help_link` | 把网址放到 `links` 里，用 `action="open_url:键"` 打开 |
| `resources.installer_icon` | 改用 `output.installer_icon` |
| `output.installer_stub`、`output.uninstaller_stub` | 运行时来自构建命令的 `--stubs` 目录 |
| `validation.*` | 构建与运行时都不读它；真正会跑的是 `install.required_space_mb` |
| `wizard.update_pages` | 升级会重放 `wizard.pages` |
| `advanced.update_mode_support` | 是否升级由目标目录里已有的安装决定 |
| `advanced.launch_app_after_install` | 在完成页上放 `action="launch_app"` |
| `localization.show_language_selector` | 语言列表来自版面里的 `Select` 控件 |
| `ui.window_width`、`ui.window_height`、`ui.expanded_height` | 窗口尺寸来自版面里的 `<Page width height>` |
| `ui.window_corner_radius` | 圆角半径来自版面里的 `<Page border-radius>` |

运行时读的是 `advanced.silent_mode_support` 与 `advanced.uninstall_mode_support`，见
[无人值守运行](#无人值守运行)。

上面这些区块里任何拼错的键同样会被拒绝，不会安静地不起作用。两处例外：`links` 是工程自己命名的表；
本项目完全不认识的区块一律不管，因为脚本会用 `get_config_value` 把它读回去。

完整情况见[当前生产状态](PRODUCTION_STATUS.md)。
