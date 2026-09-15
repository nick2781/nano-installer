# 配置参考

Native builder 读取 `installer_config.json`。当前沿用 TapTap example 的完整 schema，但只有
下表字段已经参与 native 行为。

## 已生效字段

| 字段 | 类型 | 当前作用 |
| --- | --- | --- |
| `project.name` | string | 产品名称；同时用于默认文件说明 |
| `project.version` | string | 项目业务版本，可包含发布后缀；未配置 file_version 时作为 PE 版本回退值 |
| `project.file_version` | string | Windows 文件/产品版本，必须是 1-4 段 `u16` 数字 |
| `project.description` | string | 可选；文件说明，默认 `<project.name> Installer` |
| `project.output_name` | string | PE InternalName，缺省使用 `project.name` |
| `project.publisher` | string | PE CompanyName |
| `project.copyright` | string | PE LegalCopyright |
| `output.installer_name` | string | 未传 `--output` 时的默认 setup 文件名 |
| `output.installer_icon` | string | 可选；相对项目目录的 ICO，用于 setup 文件、任务栏和 Alt+Tab 图标 |
| `output.uninstaller_name` | string | 嵌入 setup 的卸载程序文件名；必须是不含目录的文件名，默认 `uninst.exe` |
| `output.uninstaller_icon` | string | 可选；写入自包含卸载程序的 ICO |
| `install.default_path` | string | 自定义面板中 `TextInput id="editDir"` 的初始安装目录 |
| `install.required_space_mb` | integer | 通过 XML `value-source` 显示的所需空间，单位 MiB |
| `install.exe_name` | string | 解压后必须存在的根目录应用 EXE；缺失会中止安装，不部署文件 |
| `install.kill_process_on_install` | bool | 为 `true` 时安装前终止同名进程，无法终止会导致安装失败 |
| `install.kill_process_on_uninstall` | bool | 为 `true` 时卸载前终止同名进程 |
| `install.detect_running_process` | bool | 与上面两个开关取或；为 `true` 时同样会终止进程 |
| `registry.uninstall_key` | string | 卸载注册表路径，仅支持 HKCU/HKLM；安装前拒绝覆盖现有键 |
| `shortcuts.desktop_shortcut` | bool | 是否允许创建桌面快捷方式 |
| `shortcuts.desktop_default` | bool | `chkShotcut` 未出现在布局时的桌面快捷方式默认勾选状态，默认 `true` |
| `shortcuts.start_menu` | bool | 是否在开始菜单创建产品与卸载快捷方式 |
| `shortcuts.start_menu_folder` | string | 开始菜单子目录名，默认 `project.name`；只删除本安装创建且已清空的目录 |
| `autostart.enabled` | bool | 是否允许写入开机自启动项 |
| `autostart.default` | bool | `chkAutoRun` 未出现在布局时的默认勾选状态，默认 `false` |
| `autostart.registry_key` | string | 自启动注册表路径，缺省 `HKCU\...\CurrentVersion\Run` |
| `autostart.registry_value_name` | string | 自启动值名称，默认 `project.name` |
| `uninstall.data_paths` | string[] | 取消勾选 `chkReserveData` 时要删除的用户数据路径，需要环境变量展开 |
| `resources.layouts_dir` | string | 要打包的 XML 目录，默认 `layouts` |
| `resources.assets_dir` | string | 要打包的图片目录，默认 `assets` |
| `resources.locales_dir` | string | 要打包/读取的 JSON 语言目录，默认 `locales` |
| `resources.payload_file` | string | 必需；ZIP 或 7z payload 路径 |
| `localization.default_locale` | string | 默认运行语言，默认 `zh-CN` |
| `wizard.pages[].layout` | string | 安装页列表；首屏启动显示，安装时切到第二页，完成后切到最后一页 |
| `wizard.update_pages[].layout` | string | 预留的升级页列表，当前安装流程使用 `wizard.pages` |
| `wizard.uninstall_pages[].layout` | string | 卸载页列表，按同样的首屏/进度/完成顺序切换 |
| `ui.dpi_aware` | bool | 是否启用 DPI 感知并缩放布局，默认 `true` |
| `ui.dpi_threshold` | integer | 选择 `@2x` 图片的 DPI 阈值，默认 `144` |

最终 setup 的 `OriginalFilename` 使用实际输出文件名，`Language` 根据
`localization.default_locale` 映射；无法识别的 locale 回退为 `en-US`。所有详细属性均写入
Unicode `VS_VERSION_INFO` 资源。

Payload 选择不依赖扩展名：`PK` 文件头选择 `zlib-stub-native.exe`，7z 文件头
`37 7A BC AF 27 1C` 选择 `lzma-stub-native.exe`。其他格式构建失败。

`chkReserveData` 未出现在布局时，卸载一律保留用户数据；`uninstall.data_paths` 里展开后
不落在 `%APPDATA%`/`%LOCALAPPDATA%` 之下的条目会被忽略，避免误删。

运行时语言切换只保留布局、图片、配置和 locale 文件，不会为了重新加载页面而把 payload
长期保留在内存中。Select 的 Option value 必须对应 locales 目录中的 JSON 文件名。

## 脚本

`scripts/install.rhai` 存在时，安装步骤由脚本决定；`scripts/uninstall.rhai` 存在时，卸载同样
如此。两者缺失时走内置流程。脚本可用的原语见 [脚本 API](SCRIPT_API.md)。

## 已打包但尚未执行

以下配置会原样进入 setup，但 native runtime 目前不执行：

- 除上述生效字段外的 `install.*`、`registry.*`
- `links.*`、`validation.*`、`advanced.*`
- `localization.supported_locales`、`localization.show_language_selector`（当前菜单范围和可见性由 XML Select 决定）
- `advanced.update_mode_support`（升级是按目标目录的既有安装自动识别的，不读该开关）

因此不要依据“字段存在”判断功能已经完成。生产状态见
[当前生产状态](PRODUCTION_STATUS.md)。

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
