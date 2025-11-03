# 配置文件完整参考

本文档详细说明 `installer_config.json` 中所有配置项的含义和用法。

## 📖 目录

- [基本结构](#基本结构)
- [project - 项目信息](#project---项目信息)
- [install - 安装设置](#install---安装设置)
- [registry - 注册表](#registry---注册表)
- [shortcuts - 快捷方式](#shortcuts---快捷方式)
- [autostart - 开机自启](#autostart---开机自启)
- [localization - 多语言](#localization---多语言)
- [links - 外部链接](#links---外部链接)
- [resources - 资源路径](#resources---资源路径)
- [ui - 界面配置](#ui---界面配置)
- [wizard - 安装向导](#wizard---安装向导)
- [channel - 渠道标识](#channel---渠道标识)
- [uninstall - 卸载配置](#uninstall---卸载配置)
- [validation - 路径校验](#validation---路径校验)
- [advanced - 高级选项](#advanced---高级选项)

## 基本结构

```json
{
  "project": { ... },
  "install": { ... },
  "registry": { ... },
  "shortcuts": { ... },
  "autostart": { ... },
  "localization": { ... },
  "links": { ... },
  "resources": { ... },
  "ui": { ... },
  "wizard": { ... },
  "channel": { ... },
  "uninstall": { ... },
  "validation": { ... },
  "advanced": { ... }
}
```

## project - 项目信息

应用程序的基本信息。

```json
{
  "project": {
    "name": "TapTap",
    "version": "3.24.0",
    "publisher": "TapTap",
    "copyright": "© 2025 TapTap. All rights reserved.",
    "output_name": "TapTap_Setup"
  }
}
```

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `name` | string | ✅ | 产品名称，显示在安装界面和注册表中 |
| `version` | string | ✅ | 版本号，格式如 "1.0.0" |
| `publisher` | string | ✅ | 发布者名称，显示在注册表和快捷方式属性中 |
| `copyright` | string | ❌ | 版权信息，可选 |
| `output_name` | string | ❌ | 生成的安装程序文件名（不含扩展名），默认为 name |

**示例：**
- `name`: "MyApp"
- `version`: "2.1.5"
- `publisher`: "My Company Ltd."

## install - 安装设置

控制安装过程的核心配置。

```json
{
  "install": {
    "exe_name": "TapTap.exe",
    "default_path": "{localappdata}\\TapTap",
    "append_to_path": "",
    "required_space_mb": 500,
    "require_admin": false,
    "mutex_name": "TapTap_Installer_Mutex",
    "detect_running_process": true,
    "kill_process_on_install": false,
    "kill_process_on_uninstall": false
  }
}
```

| 字段 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `exe_name` | string | ✅ | - | 主程序文件名（含扩展名） |
| `default_path` | string | ✅ | - | 默认安装路径，支持变量（见下表） |
| `append_to_path` | string | ❌ | "" | 追加到用户选择路径的子目录名 |
| `required_space_mb` | number | ❌ | 100 | 所需磁盘空间（MB） |
| `require_admin` | boolean | ❌ | false | 是否需要管理员权限 |
| `mutex_name` | string | ❌ | - | 防止多实例安装的互斥锁名称 |
| `detect_running_process` | boolean | ❌ | true | 安装前检测应用是否在运行 |
| `kill_process_on_install` | boolean | ❌ | false | 安装时是否强制结束进程 |
| `kill_process_on_uninstall` | boolean | ❌ | false | 卸载时是否强制结束进程 |

### 路径变量

在 `default_path` 中可以使用以下变量：

| 变量 | 展开为 | 说明 |
|------|--------|------|
| `{pf64}` | `C:\Program Files` | 64 位程序目录 |
| `{pf32}` | `C:\Program Files (x86)` | 32 位程序目录 |
| `{localappdata}` | `C:\Users\用户名\AppData\Local` | 本地应用数据目录 |
| `{appdata}` | `C:\Users\用户名\AppData\Roaming` | 漫游应用数据目录 |
| `{programdata}` | `C:\ProgramData` | 公共应用数据目录 |
| `{desktop}` | 桌面路径 | 当前用户桌面 |
| `{documents}` | 文档路径 | 当前用户文档 |

**示例：**
```json
"default_path": "{pf64}\\MyCompany\\MyApp"
// 展开为: C:\Program Files\MyCompany\MyApp
```

### append_to_path 说明

如果设置了 `append_to_path`，最终安装路径 = 用户选择的路径 + append_to_path。

**示例：**
```json
{
  "default_path": "{localappdata}",
  "append_to_path": "MyApp"
}
// 用户选择 D:\Games
// 实际安装到 D:\Games\MyApp
```

## registry - 注册表

Windows 注册表集成配置。

```json
{
  "registry": {
    "install_path_key": "SOFTWARE\\TapTap",
    "uninstall_key": "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\TapTap",
    "help_link": "https://www.taptap.cn/help"
  }
}
```

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `install_path_key` | string | ✅ | 存储安装路径的注册表键 |
| `uninstall_key` | string | ✅ | 卸载信息注册表键（显示在"控制面板 - 程序和功能"） |
| `help_link` | string | ❌ | 帮助链接，显示在卸载信息中 |

**注意：**
- 不需要 `HKEY_LOCAL_MACHINE` 或 `HKEY_CURRENT_USER` 前缀
- 如果 `require_admin` 为 true，使用 `HKEY_LOCAL_MACHINE`
- 否则使用 `HKEY_CURRENT_USER`

## shortcuts - 快捷方式

快捷方式创建配置。

```json
{
  "shortcuts": {
    "desktop_shortcut": true,
    "desktop_default": true,
    "start_menu": true,
    "start_menu_folder": "TapTap"
  }
}
```

| 字段 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `desktop_shortcut` | boolean | ❌ | true | 是否显示"创建桌面快捷方式"选项 |
| `desktop_default` | boolean | ❌ | true | 桌面快捷方式选项的默认勾选状态 |
| `start_menu` | boolean | ❌ | true | 是否创建开始菜单快捷方式 |
| `start_menu_folder` | string | ❌ | 项目名 | 开始菜单文件夹名称 |

**快捷方式位置：**
- 桌面：`C:\Users\用户名\Desktop\应用名.lnk`
- 开始菜单：`C:\Users\用户名\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\文件夹名\应用名.lnk`

## autostart - 开机自启

开机自动启动配置。

```json
{
  "autostart": {
    "enabled": true,
    "default": false,
    "registry_key": "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
    "registry_value_name": "TapTap"
  }
}
```

| 字段 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `enabled` | boolean | ❌ | false | 是否显示"开机自动启动"选项 |
| `default` | boolean | ❌ | false | 开机自启选项的默认勾选状态 |
| `registry_key` | string | ❌ | Run 键 | 自启动注册表键路径 |
| `registry_value_name` | string | ❌ | 项目名 | 注册表值名称 |

**工作原理：**

在注册表中创建值：
```
HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Run
名称: TapTap
值: "C:\安装路径\TapTap.exe" --background
```

## localization - 多语言

多语言支持配置。

```json
{
  "localization": {
    "default_locale": "zh-CN",
    "fallback_locale": "en-US",
    "supported_locales": [
      "zh-CN", "zh-TW", "en-US", "ja", "ko",
      "th", "vi", "id", "pt", "es", "ru"
    ],
    "show_language_selector": true
  }
}
```

| 字段 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `default_locale` | string | ✅ | - | 默认语言代码 |
| `fallback_locale` | string | ❌ | "en-US" | 备用语言（当字符串缺失时使用） |
| `supported_locales` | array | ✅ | - | 支持的语言列表 |
| `show_language_selector` | boolean | ❌ | true | 是否在界面显示语言选择器 |

### 语言代码

| 代码 | 语言 |
|------|------|
| `zh-CN` | 简体中文 |
| `zh-TW` | 繁体中文 |
| `en-US` | 英语（美国） |
| `ja` | 日语 |
| `ko` | 韩语 |
| `th` | 泰语 |
| `vi` | 越南语 |
| `id` | 印尼语 |
| `pt` | 葡萄牙语 |
| `es` | 西班牙语 |
| `ru` | 俄语 |

**语言文件位置：** `locales/zh-CN.json`

## links - 外部链接

嵌入到安装程序中的外部链接。

```json
{
  "links": {
    "terms_of_service": "https://www.taptap.cn/terms",
    "privacy_policy": "https://www.taptap.cn/privacy"
  }
}
```

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `terms_of_service` | string | ❌ | 服务条款链接 |
| `privacy_policy` | string | ❌ | 隐私政策链接 |

这些链接会显示为可点击的超链接（如果在布局中使用）。

## resources - 资源路径

指定资源文件的位置。

```json
{
  "resources": {
    "layouts_dir": "layouts",
    "assets_dir": "assets",
    "locales_dir": "locales",
    "payload_file": "app.7z",
    "installer_icon": "assets/logo.ico",
    "uninstaller_icon": "assets/uninst.ico"
  }
}
```

| 字段 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `layouts_dir` | string | ❌ | "layouts" | XML 布局文件目录 |
| `assets_dir` | string | ❌ | "assets" | 图片资源目录 |
| `locales_dir` | string | ❌ | "locales" | 语言文件目录 |
| `payload_file` | string | ❌ | "app.7z" | 要安装的 7z 压缩包文件名 |
| `installer_icon` | string | ❌ | - | 安装程序图标（.ico 文件） |
| `uninstaller_icon` | string | ❌ | - | 卸载程序图标（.ico 文件） |

**路径说明：**
- 所有路径都是相对于配置文件 `installer_config.json` 所在目录
- 例如：`assets/logo.png` 指向 `项目目录/assets/logo.png`

## ui - 界面配置

安装程序窗口和UI配置。

```json
{
  "ui": {
    "window_width": 574,
    "window_height": 358,
    "expanded_height": 450,
    "dpi_aware": true,
    "dpi_threshold": 144
  }
}
```

| 字段 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `window_width` | number | ❌ | 574 | 窗口宽度（像素） |
| `window_height` | number | ❌ | 358 | 窗口高度（像素） |
| `expanded_height` | number | ❌ | 450 | 展开详细信息时的窗口高度 |
| `dpi_aware` | boolean | ❌ | true | 是否启用 DPI 自适应 |
| `dpi_threshold` | number | ❌ | 144 | DPI 阈值（达到此值使用 2x 资源） |

### DPI 说明

- **96 DPI**: 标准显示器（100% 缩放）
- **120 DPI**: 125% 缩放
- **144 DPI**: 150% 缩放（高 DPI）
- **192 DPI**: 200% 缩放

当系统 DPI ≥ `dpi_threshold` 时，自动加载 `@2x` 资源。

## wizard - 安装向导

定义安装和卸载的页面流程。

```json
{
  "wizard": {
    "pages": [
      { "id": "welcome", "layout": "welcome.xml", "title": "欢迎" },
      { "id": "install_path", "layout": "config.xml", "title": "安装选项" },
      { "id": "installing", "layout": "installing.xml", "title": "正在安装" },
      { "id": "finish", "layout": "finish.xml", "title": "完成" }
    ],
    "uninstall_pages": [
      { "id": "uninstall_confirm", "layout": "uninstall.xml", "title": "确认卸载" }
    ]
  }
}
```

### pages - 安装页面

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `id` | string | ✅ | 页面标识符，用于导航 |
| `layout` | string | ✅ | XML 布局文件名（相对于 layouts_dir） |
| `title` | string | ❌ | 页面标题（显示在标题栏） |

### 预定义页面 ID

| ID | 功能 | 说明 |
|------|------|------|
| `welcome` | 欢迎页面 | 显示欢迎信息和产品介绍 |
| `license` | 许可协议 | 显示并要求接受许可协议 |
| `install_path` | 安装路径 | 让用户选择安装位置和选项 |
| `installing` | 安装进度 | 显示安装进度条和状态 |
| `finish` | 完成页面 | 安装完成，显示后续操作 |

你可以使用自定义的 ID 和布局。

## channel - 渠道标识

用于区分不同分发渠道的安装包。

```json
{
  "channel": {
    "extract_from_filename": true,
    "filename_regex": "TapTap_.*?_(.*?)\\.exe",
    "default_channel": "official",
    "output_channel_conf": true
  }
}
```

| 字段 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `extract_from_filename` | boolean | ❌ | false | 是否从文件名提取渠道标识 |
| `filename_regex` | string | ❌ | - | 提取渠道的正则表达式（第一个捕获组） |
| `default_channel` | string | ❌ | "official" | 默认渠道名称 |
| `output_channel_conf` | boolean | ❌ | false | 是否生成 channel.conf 文件 |

### 工作原理

**文件名：** `TapTap_3.24.0_steam.exe`

**正则表达式：** `TapTap_.*?_(.*?)\\.exe`

**提取结果：** `steam`

**生成文件：** `安装目录/channel.conf` 内容为 `steam`

## uninstall - 卸载配置

卸载程序的行为配置。

```json
{
  "uninstall": {
    "show_keep_data_option": true,
    "keep_data_default": false,
    "cleanup_registry": true,
    "cleanup_game_registry": true,
    "game_registry_path": "SOFTWARE\\TapTap\\Games"
  }
}
```

| 字段 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `show_keep_data_option` | boolean | ❌ | true | 是否显示"保留用户数据"选项 |
| `keep_data_default` | boolean | ❌ | false | "保留用户数据"的默认勾选状态 |
| `cleanup_registry` | boolean | ❌ | true | 是否清理注册表 |
| `cleanup_game_registry` | boolean | ❌ | false | 是否清理游戏相关注册表 |
| `game_registry_path` | string | ❌ | - | 游戏注册表路径（如果启用清理） |

### 卸载行为

1. **删除文件**：删除安装目录下的所有文件
2. **保留数据**：如果用户勾选，保留 `AppData` 中的用户数据
3. **清理注册表**：删除 `install_path_key` 和 `uninstall_key`
4. **删除快捷方式**：删除桌面和开始菜单快捷方式
5. **移除自启动**：删除开机自启动项

## validation - 路径校验

安装路径的验证规则。

```json
{
  "validation": {
    "allow_non_empty_dir": false,
    "check_path_legal": true,
    "check_disk_space": true,
    "check_disk_type": "Any"
  }
}
```

| 字段 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `allow_non_empty_dir` | boolean | ❌ | false | 是否允许安装到非空目录 |
| `check_path_legal` | boolean | ❌ | true | 是否检查路径合法性 |
| `check_disk_space` | boolean | ❌ | true | 是否检查磁盘空间 |
| `check_disk_type` | string | ❌ | "Any" | 允许的磁盘类型 |

### check_disk_type 选项

| 值 | 说明 |
|------|------|
| `"Any"` | 任何类型的磁盘 |
| `"Fixed"` | 仅固定磁盘（HDD/SSD） |
| `"Removable"` | 仅可移动磁盘 |
| `"Network"` | 仅网络磁盘 |

### 路径合法性检查

拒绝以下路径：
- 系统关键目录（`C:\Windows`, `C:\Program Files\WindowsApps`）
- 包含非法字符（`< > : " | ? *`）
- 盘符不存在
- 路径过长（> 240 字符）

## advanced - 高级选项

开发和调试选项。

```json
{
  "advanced": {
    "debug_mode": false,
    "log_level": "info",
    "log_to_file": true,
    "keep_temp_files": false,
    "disable_animations": false
  }
}
```

| 字段 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `debug_mode` | boolean | ❌ | false | 启用调试模式（显示详细日志） |
| `log_level` | string | ❌ | "info" | 日志级别：`"debug"` \| `"info"` \| `"warn"` \| `"error"` |
| `log_to_file` | boolean | ❌ | true | 是否将日志写入文件 |
| `keep_temp_files` | boolean | ❌ | false | 是否保留临时文件（用于调试） |
| `disable_animations` | boolean | ❌ | false | 禁用 UI 动画（性能优化） |

### 日志文件位置

**安装日志：** `%TEMP%\nano-installer-XXXX\install.log`

**卸载日志：** `%TEMP%\nano-installer-XXXX\uninstall.log`

## 完整示例

查看 [examples/TapTap/installer_config.json](../examples/TapTap/installer_config.json) 获取完整的配置示例。

## 相关文档

- [XML 布局指南](XML_LAYOUT_GUIDE.md) - 自定义安装界面
- [多语言支持](LOCALIZATION.md) - 添加语言文件
- [示例项目](../examples/TapTap/README.md) - 完整示例说明

---

有问题？查看 [开发文档](DEVELOPMENT.md) 或提交 Issue。
