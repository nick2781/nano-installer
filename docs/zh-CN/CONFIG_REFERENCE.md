# 配置参考

每个项目有一个 `installer_config.json`。本页只列出当前真正会改变安装包行为的设置；被接受但
尚未生效的字段集中列在最后，不必猜某个字段是否已经生效。

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

未配置 `project.file_version` 时，版本资源使用 `project.version`，此时它必须是纯数字。

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
| `install.required_space_mb` | integer | 所需空间，单位 MiB，通过 XML 的 `value-source` 显示 |
| `install.exe_name` | string | payload 中必须存在的应用 EXE；缺失时直接中止，不部署任何文件 |
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

复选框是否存在由你的布局决定：布局里没有 `chkShotcut` 或 `chkAutoRun` 时，使用上面的默认值。
安装时会记录这两项，卸载后机器回到安装前的状态。

## 文件、语言与页面

| 字段 | 类型 | 作用 |
| --- | --- | --- |
| `resources.layouts_dir` | string | 要打包的 XML 目录，默认 `layouts` |
| `resources.assets_dir` | string | 要打包的图片目录，默认 `assets` |
| `resources.locales_dir` | string | 语言 JSON 目录，默认 `locales` |
| `resources.payload_file` | string | 必需；ZIP 或 7z payload 路径 |
| `localization.default_locale` | string | 启动时使用的语言，默认 `zh-CN` |
| `wizard.pages[].layout` | string | 安装页列表：首屏欢迎页、第二页进度、最后一页完成 |
| `wizard.update_pages[].layout` | string | 预留的升级页列表；当前安装流程使用 `wizard.pages` |
| `wizard.uninstall_pages[].layout` | string | 卸载页列表，按同样的顺序切换 |

## 界面

| 字段 | 类型 | 作用 |
| --- | --- | --- |
| `ui.dpi_aware` | bool | 是否启用 DPI 感知与布局缩放，默认 `true` |
| `ui.dpi_threshold` | integer | 达到该 DPI 时优先使用 `@2x` 图片，默认 `144` |

## 卸载时的用户数据

`uninstall.data_paths` 列出产品自己的数据目录。只有用户取消勾选保留数据时才会在卸载时删除，
并且展开后不落在 `%APPDATA%` 或 `%LOCALAPPDATA%` 之下的条目会被忽略，避免误删无关文件。
布局里没有 `chkReserveData` 时，用户数据一律保留。

```json
"uninstall": {
  "data_paths": ["%APPDATA%\\MyApp", "%LOCALAPPDATA%\\MyApp"]
}
```

## payload 格式

payload 就是你的应用文件，预先压成 ZIP 或 7z。格式按文件头识别而不是按扩展名：`PK` 选择 ZIP
运行时，`37 7A BC AF 27 1C` 选择 7z 运行时。其他格式会导致构建失败。

## 自定义安装与卸载步骤

加入 `scripts/install.rhai` 或 `scripts/uninstall.rhai` 即可替代内置步骤，见
[脚本 API](SCRIPT_API.md)。

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
  "ui": { "dpi_aware": true, "dpi_threshold": 144 },
  "wizard": {
    "pages": [
      { "id": "config", "layout": "layouts/configpage.xml" }
    ]
  }
}
```

## 已接受但尚未生效

以下设置会原样打包进安装包，但运行时不会读取。不要因为字段存在就认为功能已完成：

- 上表未列出的 `install.*` 与 `registry.*` 字段
- `validation.*` 与 `advanced.*`；`links.*` 会被链接点击和 `open_url:` 动作读取
- `localization.supported_locales` 与 `localization.show_language_selector`；语言列表与可见性由
  XML 中的 `Select` 控件决定
- `advanced.update_mode_support`；升级是按目标目录中的既有安装自动识别的，不读该开关

完整情况见[当前生产状态](PRODUCTION_STATUS.md)。
