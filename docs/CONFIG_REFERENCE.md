# installer_config.json 配置参考

配置文件位于产品项目根目录。除非表格标记“可选”，字段都必须存在；Rust 中的
`Default` 实现不会自动补全缺失的 JSON 字段。可构建的完整配置以
[examples/TapTap/installer_config.json](../examples/TapTap/installer_config.json) 为准。

## 顶层结构

```json
{
  "project": {},
  "output": {},
  "install": {},
  "registry": {},
  "shortcuts": {},
  "autostart": {},
  "localization": {},
  "links": {},
  "resources": {},
  "ui": {},
  "wizard": {},
  "validation": {},
  "advanced": {},
  "install_tasks": []
}
```

`install_tasks` 可省略，其余顶层对象必须存在。

## project

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `name` | string | 产品显示名称 |
| `version` | string | `x.y.z` 版本，写入 PE 和卸载信息 |
| `publisher` | string | 发布者 |
| `copyright` | string | 版权信息 |
| `output_name` | string | 默认输出名前缀 |

## output

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `installer_name` | string | 最终 setup 文件名，包含 `.exe` |
| `installer_icon` | string | 相对项目目录的安装器 ICO |
| `uninstaller_name` | string | 安装后的卸载器文件名 |
| `uninstaller_icon` | string | 相对项目目录的卸载器 ICO |
| `installer_stub` | string，可选 | 默认 `lzma-x64.exe` |
| `uninstaller_stub` | string，可选 | 默认 `uninst-x64.exe` |

可用 installer stub 为 `lzma-x64.exe` 和 `zlib-x64.exe`，uninstaller stub 为
`uninst-x64.exe`。所有 stub 仅支持 Windows x64 和 Unicode，不提供 x86 或 ANSI 变体。
压缩格式必须与 payload 匹配。构建脚本可通过环境变量
`NANO_INSTALLER_INSTALLER_STUB`、`NANO_INSTALLER_UNINSTALLER_STUB` 临时覆盖配置。

当前 schema 同时在 `resources` 保留了两个 icon 字段。构建输出优先读取 `output`，
配置校验会检查 `resources` 中的路径；接入时应把两处写成相同值。

## install

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `exe_name` | string | payload 内的主程序文件名 |
| `default_path` | string | 默认安装目录 |
| `append_to_path` | string | 用户选择目录后追加的子目录 |
| `required_space_mb` | u32 | 所需空间，必须大于 0 |
| `require_admin` | boolean | 是否请求管理员权限 |
| `mutex_name` | string | 安装器单实例 mutex |
| `detect_running_process` | boolean | 兼容旧配置的主进程检测开关 |
| `kill_process_on_install` | boolean | 兼容旧配置的安装前关闭开关 |
| `kill_process_on_uninstall` | boolean | 兼容旧配置的卸载前关闭开关 |
| `close_targets` | array，可选 | 明确的进程/服务列表，默认空数组 |

配置了 `close_targets` 后，它会覆盖三个 legacy 开关组合出的主进程规则：

```json
{
  "close_targets": [
    {
      "name": "MyApp.exe",
      "kind": "process",
      "detect_on_install": true,
      "close_on_install": false,
      "close_on_uninstall": true,
      "force": true
    }
  ]
}
```

`kind` 为 `process` 或 `service`，默认 `process`；各行为开关默认 `false`，`force`
默认 `true`。

## registry

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `install_path_key` | string | 保存/检测安装目录的完整 HKLM/HKCU/HKCR 路径 |
| `uninstall_key` | string | 控制面板卸载信息的完整注册表路径 |
| `help_link` | string | 卸载信息中的帮助链接 |

不要在版本间随意修改前两个键，否则更新检测和旧版本卸载会失去关联。

## shortcuts

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `desktop_shortcut` | boolean | 产品是否提供桌面快捷方式能力 |
| `desktop_default` | boolean | 默认是否创建桌面快捷方式 |
| `start_menu` | boolean | 是否创建开始菜单项 |
| `start_menu_folder` | string | 开始菜单文件夹名 |

选项是否显示由 XML 布局决定，配置值决定通用任务的默认行为。

## autostart

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `enabled` | boolean | 是否启用自启动能力 |
| `default` | boolean | 默认是否开启 |
| `registry_key` | string | 通常是 HKLM/HKCU 下的 `Run` 键 |
| `registry_value_name` | string | 写入的 value 名称 |

## localization

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `default_locale` | string | 默认 locale，必须有同名 JSON |
| `supported_locales` | string[] | 打包的 locale 列表 |
| `show_language_selector` | boolean | 是否启用语言选择能力 |

locale 文件路径为 `<resources.locales_dir>/<locale>.json`。布局仍需包含对应 Select
控件，用户才能切换语言。

## links

`links` 是任意 `key -> URL` 对象。XML 可通过 `open_url:<key>` 或带链接的富文本引用。

```json
{
  "links": {
    "terms_of_service": "https://example.com/terms",
    "privacy_policy": "https://example.com/privacy"
  }
}
```

## resources

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `layouts_dir` | string | XML 目录，相对项目根目录 |
| `assets_dir` | string | 图片和 ICO 目录 |
| `locales_dir` | string | locale JSON 目录 |
| `payload_file` | string | payload 归档文件 |
| `installer_icon` | string | 校验使用的安装器 ICO |
| `uninstaller_icon` | string | 校验使用的卸载器 ICO |

不存在 `payload_dir` 字段。本地开发时，如果 `payload_file` 不存在且项目根目录有
`files/`，CLI 会自动生成归档；生产流水线应显式生成 `payload_file`。

## ui

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `window_width` | u32 | 普通窗口宽度，逻辑像素 |
| `window_height` | u32 | 普通窗口高度 |
| `expanded_height` | u32 | 展开配置区域后的高度，不得小于普通高度 |
| `window_corner_radius` | u32，可选 | 默认 8 |
| `dialog_width` | u32，可选 | 默认 400 |
| `dialog_height` | u32，可选 | 默认 230 |
| `dpi_aware` | boolean | DPI 能力开关字段 |
| `dpi_threshold` | u32 | 使用 `@2x` 资源的 DPI 阈值，如 144 |

`dpi_threshold` 是 DPI 值，不是 `1.5` 这样的缩放倍数。
`dpi_aware` 当前已进入 schema，但 runtime 仍会统一计算 DPI；它尚不能用于彻底关闭
DPI 适配。

## wizard

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `pages` | Page[] | install 页面流 |
| `update_pages` | Page[]，可选 | update 页面流，默认空数组 |
| `uninstall_pages` | Page[] | uninstall 页面流 |

每个 Page 都需要 `id`、`layout` 和 `title`。`layout` 是相对于
`resources.layouts_dir` 的文件名，应包含 `.xml`：

```json
{
  "id": "config",
  "layout": "configpage.xml",
  "title": "安装选项"
}
```

## validation

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `check_path_legal` | boolean | 路径合法性策略字段 |
| `check_disk_type` | string | `Any`、`HDD` 或 `SSD` 策略字段 |
| `check_disk_space` | boolean | 空间检查策略字段 |

这三个字段当前已进入 schema，但 runtime 尚未读取 `config.validation` 来切换检查行为。
在补齐实现前，不要依赖把它们设为 `false` 来绕过运行时校验。

## advanced

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `silent_mode_support` | boolean | 是否允许静默安装/卸载 |
| `update_mode_support` | boolean | 是否启用已安装版本检测和 update 模式 |
| `uninstall_mode_support` | boolean | 是否允许卸载模式 |
| `launch_app_after_install` | boolean | 安装完成页的启动策略 |

## install_tasks

省略时执行默认顺序：`extract`、`copy_uninstaller`、`create_shortcuts`、
`write_registry`。自定义数组会完全替换默认顺序，配置错误可能产出不可卸载的安装结果。

```json
{
  "install_tasks": [
    { "type": "extract" },
    { "type": "copy_uninstaller" },
    { "type": "create_shortcuts" },
    { "type": "write_registry" }
  ]
}
```

`payload` 和 `params` 是可选字段。除非确实需要改变核心流水线，产品定制优先放到 Rhai
脚本，边界见[配置与脚本边界](CONFIG_VS_SCRIPT.md)。
