# JSON 配置规范

本文档定义 `installer_config.json` 配置文件的完整 JSON Schema 规范。

## 📖 目录

- [根结构](#根结构)
- [类型定义](#类型定义)
- [必需字段](#必需字段)
- [默认值](#默认值)
- [验证规则](#验证规则)
- [完整 Schema](#完整-schema)

## 根结构

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

## 类型定义

### ProjectConfig

项目基本信息。

```typescript
interface ProjectConfig {
  name: string;           // ✅ 必需 - 产品名称
  version: string;        // ✅ 必需 - 版本号（语义化版本）
  publisher: string;      // ✅ 必需 - 发布者名称
  copyright?: string;     // ❌ 可选 - 版权信息
  output_name?: string;   // ❌ 可选 - 输出文件名（默认为 name）
}
```

**约束：**
- `name`: 非空字符串，1-100 字符
- `version`: 符合语义化版本格式 (X.Y.Z)
- `publisher`: 非空字符串，1-100 字符
- `output_name`: 只能包含字母、数字、下划线、连字符

**示例：**
```json
{
  "project": {
    "name": "TapTap",
    "version": "3.24.0",
    "publisher": "TapTap Inc.",
    "copyright": "© 2025 TapTap Inc.",
    "output_name": "TapTap_Setup"
  }
}
```

### InstallConfig

安装配置。

```typescript
interface InstallConfig {
  exe_name: string;                    // ✅ 必需 - 主程序文件名
  default_path: string;                // ✅ 必需 - 默认安装路径
  append_to_path?: string;             // ❌ 可选 - 追加到路径的子目录
  required_space_mb: number;           // ✅ 必需 - 所需磁盘空间（MB）
  require_admin: boolean;              // ✅ 必需 - 是否需要管理员权限
  mutex_name?: string;                 // ❌ 可选 - 互斥锁名称
  detect_running_process?: boolean;    // ❌ 可选 - 是否检测运行中的进程
  kill_process_on_install?: boolean;   // ❌ 可选 - 安装时终止进程
  kill_process_on_uninstall?: boolean; // ❌ 可选 - 卸载时终止进程
}
```

**约束：**
- `exe_name`: 必须以 `.exe` 结尾
- `default_path`: 有效的路径字符串，支持变量 `{pf64}`, `{pf32}`, `{localappdata}` 等
- `required_space_mb`: > 0
- `mutex_name`: 只能包含字母、数字、下划线

**示例：**
```json
{
  "install": {
    "exe_name": "TapTap.exe",
    "default_path": "{localappdata}\\TapTap",
    "required_space_mb": 500,
    "require_admin": false,
    "detect_running_process": true
  }
}
```

### RegistryConfig

注册表配置。

```typescript
interface RegistryConfig {
  install_path_key: string;  // ✅ 必需 - 安装路径注册表键
  uninstall_key: string;     // ✅ 必需 - 卸载信息注册表键
  help_link?: string;        // ❌ 可选 - 帮助链接
}
```

**约束：**
- 键路径格式：`SOFTWARE\\公司\\产品`
- 不需要 `HKEY_LOCAL_MACHINE` 或 `HKEY_CURRENT_USER` 前缀
- `help_link`: 有效的 URL

**示例：**
```json
{
  "registry": {
    "install_path_key": "SOFTWARE\\TapTap",
    "uninstall_key": "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\TapTap",
    "help_link": "https://www.taptap.io/help"
  }
}
```

### ShortcutsConfig

快捷方式配置。

```typescript
interface ShortcutsConfig {
  desktop_shortcut?: boolean;   // ❌ 可选 - 显示桌面快捷方式选项
  desktop_default?: boolean;    // ❌ 可选 - 桌面快捷方式默认勾选
  start_menu?: boolean;         // ❌ 可选 - 创建开始菜单快捷方式
  start_menu_folder?: string;   // ❌ 可选 - 开始菜单文件夹名
}
```

**默认值：**
```json
{
  "desktop_shortcut": true,
  "desktop_default": true,
  "start_menu": true,
  "start_menu_folder": "<project.name>"
}
```

**示例：**
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

### AutostartConfig

开机自启配置。

```typescript
interface AutostartConfig {
  enabled?: boolean;              // ❌ 可选 - 显示自启选项
  default?: boolean;              // ❌ 可选 - 默认勾选
  registry_key?: string;          // ❌ 可选 - 注册表键
  registry_value_name?: string;   // ❌ 可选 - 注册表值名称
}
```

**默认值：**
```json
{
  "enabled": false,
  "default": false,
  "registry_key": "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
  "registry_value_name": "<project.name>"
}
```

**示例：**
```json
{
  "autostart": {
    "enabled": true,
    "default": false
  }
}
```

### LocalizationConfig

多语言配置。

```typescript
interface LocalizationConfig {
  default_locale: string;           // ✅ 必需 - 默认语言代码
  supported_locales: string[];      // ✅ 必需 - 支持的语言列表
  show_language_selector?: boolean; // ❌ 可选 - 显示语言选择器
}
```

**约束：**
- `default_locale`: 必须在 `supported_locales` 中
- `supported_locales`: 至少包含一个语言
- 语言代码格式：`zh-CN`, `en-US`, `ja` 等（BCP 47）

**示例：**
```json
{
  "localization": {
    "default_locale": "zh-CN",
    "supported_locales": ["zh-CN", "zh-TW", "en-US", "ja", "ko"],
    "show_language_selector": true
  }
}
```

### LinksConfig

外部链接配置。

```typescript
interface LinksConfig {
  terms_of_service?: string;  // ❌ 可选 - 服务条款URL
  privacy_policy?: string;    // ❌ 可选 - 隐私政策URL
}
```

**约束：**
- 必须是有效的 HTTP/HTTPS URL

**示例：**
```json
{
  "links": {
    "terms_of_service": "https://www.taptap.io/terms",
    "privacy_policy": "https://www.taptap.io/privacy"
  }
}
```

### ResourcesConfig

资源路径配置。

```typescript
interface ResourcesConfig {
  layouts_dir?: string;        // ❌ 可选 - 布局文件目录
  assets_dir?: string;         // ❌ 可选 - 图片资源目录
  locales_dir?: string;        // ❌ 可选 - 语言文件目录
  payload_file?: string;       // ❌ 可选 - 安装包文件
  installer_icon?: string;     // ❌ 可选 - 安装程序图标
  uninstaller_icon?: string;   // ❌ 可选 - 卸载程序图标
}
```

**默认值：**
```json
{
  "layouts_dir": "layouts",
  "assets_dir": "assets",
  "locales_dir": "locales",
  "payload_file": "app.7z",
  "installer_icon": "assets/logo.ico",
  "uninstaller_icon": "assets/uninst.ico"
}
```

**约束：**
- 所有路径相对于配置文件所在目录
- 图标文件必须是 `.ico` 格式

**示例：**
```json
{
  "resources": {
    "layouts_dir": "layouts",
    "assets_dir": "assets",
    "locales_dir": "locales",
    "payload_file": "payload/app.7z",
    "installer_icon": "assets/logo.ico"
  }
}
```

### UiConfig

界面配置。

```typescript
interface UiConfig {
  window_width?: number;       // ❌ 可选 - 窗口宽度（像素）
  window_height?: number;      // ❌ 可选 - 窗口高度（像素）
  expanded_height?: number;    // ❌ 可选 - 展开高度（像素）
  dpi_aware?: boolean;         // ❌ 可选 - DPI 感知
  dpi_threshold?: number;      // ❌ 可选 - DPI 阈值
}
```

**默认值：**
```json
{
  "window_width": 574,
  "window_height": 358,
  "expanded_height": 450,
  "dpi_aware": true,
  "dpi_threshold": 144
}
```

**约束：**
- `window_width`, `window_height`, `expanded_height`: > 0
- `dpi_threshold`: 通常为 96, 120, 144, 192

**示例：**
```json
{
  "ui": {
    "window_width": 574,
    "window_height": 358,
    "dpi_aware": true,
    "dpi_threshold": 144
  }
}
```

### WizardConfig

向导流程配置。

```typescript
interface WizardConfig {
  pages: PageConfig[];              // ✅ 必需 - 安装页面列表
  uninstall_pages?: PageConfig[];   // ❌ 可选 - 卸载页面列表
}

interface PageConfig {
  id: string;      // ✅ 必需 - 页面标识符
  layout: string;  // ✅ 必需 - 布局文件路径
  title?: string;  // ❌ 可选 - 页面标题
}
```

**约束：**
- `pages`: 至少包含一个页面
- `id`: 只能包含字母、数字、下划线
- `layout`: 相对于 `layouts_dir` 的路径

**示例：**
```json
{
  "wizard": {
    "pages": [
      { "id": "welcome", "layout": "welcome.xml", "title": "欢迎" },
      { "id": "install_path", "layout": "config.xml", "title": "配置" },
      { "id": "installing", "layout": "installing.xml", "title": "安装中" },
      { "id": "finish", "layout": "finish.xml", "title": "完成" }
    ]
  }
}
```

### ChannelConfig

渠道标识配置。

```typescript
interface ChannelConfig {
  extract_from_filename?: boolean;  // ❌ 可选 - 从文件名提取渠道
  filename_regex?: string;          // ❌ 可选 - 提取正则表达式
  default_channel?: string;         // ❌ 可选 - 默认渠道
  output_channel_conf?: boolean;    // ❌ 可选 - 生成 channel.conf
}
```

**默认值：**
```json
{
  "extract_from_filename": false,
  "filename_regex": ".*_([^_]+)\\.exe$",
  "default_channel": "official",
  "output_channel_conf": false
}
```

**示例：**
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

### UninstallConfig

卸载配置。

```typescript
interface UninstallConfig {
  show_keep_data_option?: boolean;  // ❌ 可选 - 显示保留数据选项
  keep_data_default?: boolean;      // ❌ 可选 - 保留数据默认勾选
  data_paths?: string[];            // ❌ 可选 - 用户数据路径列表
  cleanup_game_registry?: boolean;  // ❌ 可选 - 清理游戏注册表
  game_registry_path?: string;      // ❌ 可选 - 游戏注册表路径
}
```

**默认值：**
```json
{
  "show_keep_data_option": true,
  "keep_data_default": false,
  "cleanup_game_registry": false
}
```

**示例：**
```json
{
  "uninstall": {
    "show_keep_data_option": true,
    "keep_data_default": false,
    "data_paths": [
      "%APPDATA%\\TapTap",
      "%LOCALAPPDATA%\\TapTap"
    ],
    "cleanup_game_registry": true,
    "game_registry_path": "SOFTWARE\\TapTap\\Games"
  }
}
```

### ValidationConfig

路径校验配置。

```typescript
interface ValidationConfig {
  check_path_legal?: boolean;   // ❌ 可选 - 检查路径合法性
  check_disk_type?: DiskType;   // ❌ 可选 - 检查磁盘类型
  check_disk_space?: boolean;   // ❌ 可选 - 检查磁盘空间
}

type DiskType = "Any" | "Fixed" | "Removable" | "Network";
```

**默认值：**
```json
{
  "check_path_legal": true,
  "check_disk_type": "Any",
  "check_disk_space": true
}
```

**示例：**
```json
{
  "validation": {
    "check_path_legal": true,
    "check_disk_type": "Fixed",
    "check_disk_space": true
  }
}
```

### AdvancedConfig

高级选项配置。

```typescript
interface AdvancedConfig {
  silent_mode_support?: boolean;     // ❌ 可选 - 支持静默模式
  update_mode_support?: boolean;     // ❌ 可选 - 支持更新模式
  uninstall_mode_support?: boolean;  // ❌ 可选 - 支持卸载模式
  launch_app_after_install?: boolean; // ❌ 可选 - 安装后启动应用
  debug_mode?: boolean;              // ❌ 可选 - 调试模式
  log_level?: LogLevel;              // ❌ 可选 - 日志级别
}

type LogLevel = "trace" | "debug" | "info" | "warn" | "error";
```

**默认值：**
```json
{
  "silent_mode_support": true,
  "update_mode_support": false,
  "uninstall_mode_support": true,
  "launch_app_after_install": false,
  "debug_mode": false,
  "log_level": "info"
}
```

**示例：**
```json
{
  "advanced": {
    "silent_mode_support": true,
    "launch_app_after_install": true,
    "debug_mode": false,
    "log_level": "info"
  }
}
```

## 必需字段

以下字段**必须**提供：

```json
{
  "project": {
    "name": "...",        // ✅ 必需
    "version": "...",     // ✅ 必需
    "publisher": "..."    // ✅ 必需
  },
  "install": {
    "exe_name": "...",       // ✅ 必需
    "default_path": "...",   // ✅ 必需
    "required_space_mb": 100, // ✅ 必需
    "require_admin": false   // ✅ 必需
  },
  "registry": {
    "install_path_key": "...", // ✅ 必需
    "uninstall_key": "..."     // ✅ 必需
  },
  "localization": {
    "default_locale": "...",       // ✅ 必需
    "supported_locales": [...]     // ✅ 必需
  },
  "wizard": {
    "pages": [...]  // ✅ 必需（至少一个页面）
  }
}
```

其他所有字段都有合理的默认值。

## 默认值

如果不提供可选字段，将使用以下默认值：

```json
{
  "project": {
    "copyright": "",
    "output_name": "<name>"
  },
  "install": {
    "append_to_path": "",
    "mutex_name": "<name>_Installer",
    "detect_running_process": true,
    "kill_process_on_install": false,
    "kill_process_on_uninstall": false
  },
  "shortcuts": {
    "desktop_shortcut": true,
    "desktop_default": true,
    "start_menu": true,
    "start_menu_folder": "<name>"
  },
  "autostart": {
    "enabled": false,
    "default": false
  },
  "resources": {
    "layouts_dir": "layouts",
    "assets_dir": "assets",
    "locales_dir": "locales",
    "payload_file": "app.7z"
  },
  "ui": {
    "window_width": 574,
    "window_height": 358,
    "expanded_height": 450,
    "dpi_aware": true,
    "dpi_threshold": 144
  },
  "channel": {
    "extract_from_filename": false,
    "default_channel": "official",
    "output_channel_conf": false
  },
  "uninstall": {
    "show_keep_data_option": true,
    "keep_data_default": false
  },
  "validation": {
    "check_path_legal": true,
    "check_disk_type": "Any",
    "check_disk_space": true
  },
  "advanced": {
    "silent_mode_support": true,
    "update_mode_support": false,
    "uninstall_mode_support": true,
    "launch_app_after_install": false,
    "debug_mode": false,
    "log_level": "info"
  }
}
```

## 验证规则

### 语义化版本

`project.version` 必须符合语义化版本规范：

```
X.Y.Z
X.Y.Z-prerelease
X.Y.Z+build
```

**有效：**
- `"1.0.0"`
- `"2.1.5"`
- `"1.0.0-beta"`
- `"1.0.0+20250101"`

**无效：**
- `"1.0"` （缺少修订号）
- `"v1.0.0"` （不应包含 v 前缀）
- `"1.0.0.0"` （太多部分）

### 路径变量

`install.default_path` 支持以下变量：

| 变量 | Windows 展开 |
|------|--------------|
| `{pf64}` | `C:\Program Files` |
| `{pf32}` | `C:\Program Files (x86)` |
| `{localappdata}` | `C:\Users\<user>\AppData\Local` |
| `{appdata}` | `C:\Users\<user>\AppData\Roaming` |
| `{programdata}` | `C:\ProgramData` |

### 文件路径

所有文件路径（`resources.*`，`wizard.pages[].layout`）必须：
- 使用正斜杠 `/` 或双反斜杠 `\\`
- 相对于配置文件所在目录
- 指向存在的文件

### 语言代码

`localization.supported_locales` 中的语言代码必须：
- 符合 BCP 47 标准
- 对应的 `locales/<locale>.json` 文件存在

### 页面 ID

`wizard.pages[].id` 的约束：
- 只能包含字母、数字、下划线
- 不能为空
- 在同一数组中唯一

## 完整 Schema

完整的 JSON Schema（TypeScript 类型定义）：

```typescript
interface InstallerConfig {
  project: ProjectConfig;
  install: InstallConfig;
  registry: RegistryConfig;
  shortcuts?: ShortcutsConfig;
  autostart?: AutostartConfig;
  localization: LocalizationConfig;
  links?: LinksConfig;
  resources?: ResourcesConfig;
  ui?: UiConfig;
  wizard: WizardConfig;
  channel?: ChannelConfig;
  uninstall?: UninstallConfig;
  validation?: ValidationConfig;
  advanced?: AdvancedConfig;
}
```

## 相关文档

- [配置参考](CONFIG_REFERENCE.md) - 每个字段的详细说明和示例
- [示例配置](../examples/TapTap/installer_config.json) - 完整的实际配置

---

有问题？查看 [主文档](../README.md) 或提交 Issue。

