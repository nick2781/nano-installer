# installer_config.json 配置参考

本文档是 `installer_config.json` 的完整字段参考。该文件是 nano-installer 的核心配置文件，定义了安装程序的所有行为和属性。

## 配置文件位置

```
your_project/
  installer_config.json   <-- 此文件
  layouts/
  assets/
  locales/
```

---

## project - 项目基本信息

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `name` | string | **必需** | 产品名称，用于窗口标题、快捷方式等 |
| `version` | string | `"1.0.0"` | 版本号，显示在 UI 中并写入注册表 |
| `publisher` | string | `""` | 发布者/公司名，写入注册表卸载信息 |
| `copyright` | string | `""` | 版权声明 |
| `output_name` | string | `"{name}_Setup"` | 输出文件名前缀 |

```json
{
  "project": {
    "name": "TapTap",
    "version": "3.0.0",
    "publisher": "TapTap",
    "copyright": "Copyright 2024 TapTap",
    "output_name": "TapTap_Setup"
  }
}
```

---

## output - 输出文件配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `installer_name` | string | `"{project.name}_Setup.exe"` | 安装程序输出文件名 |
| `installer_icon` | string | `""` | 安装程序图标路径（.ico） |
| `uninstaller_name` | string | `"uninst.exe"` | 卸载程序文件名 |
| `uninstaller_icon` | string | `""` | 卸载程序图标路径（.ico） |

```json
{
  "output": {
    "installer_name": "TapTap_Setup.exe",
    "installer_icon": "assets/installer.ico",
    "uninstaller_name": "uninst.exe",
    "uninstaller_icon": "assets/uninstaller.ico"
  }
}
```

---

## install - 安装行为配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `exe_name` | string | **必需** | 主程序可执行文件名（如 `"TapTap.exe"`） |
| `default_path` | string | `"C:\\Program Files\\{project.name}"` | 默认安装路径，支持 `{product_name}` 变量 |
| `append_to_path` | string | `""` | 追加到安装路径末尾的子目录 |
| `required_space_mb` | number | `500` | 所需磁盘空间（MB），用于空间检查和 UI 显示 |
| `require_admin` | boolean | `false` | 是否需要管理员权限 |
| `mutex_name` | string | `""` | 互斥锁名称，防止多实例安装 |
| `detect_running_process` | string | `""` | 安装前检测的进程名（如 `"TapTap.exe"`） |
| `kill_process_on_install` | boolean | `false` | 安装时是否自动结束正在运行的目标进程 |
| `kill_process_on_uninstall` | boolean | `false` | 卸载时是否自动结束正在运行的目标进程 |

```json
{
  "install": {
    "exe_name": "TapTap.exe",
    "default_path": "C:\\Program Files\\TapTap",
    "required_space_mb": 500,
    "require_admin": false,
    "mutex_name": "TapTapInstaller",
    "detect_running_process": "TapTap.exe",
    "kill_process_on_install": true,
    "kill_process_on_uninstall": true
  }
}
```

---

## registry - 注册表配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `install_path_key` | string | `""` | 安装路径注册表键，用于检测已安装的版本 |
| `uninstall_key` | string | `""` | 卸载信息注册表键路径（在 `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\` 下） |
| `help_link` | string | `""` | 帮助链接，写入注册表卸载信息的 HelpLink 字段 |

```json
{
  "registry": {
    "install_path_key": "Software\\TapTap\\InstallPath",
    "uninstall_key": "TapTap",
    "help_link": "https://www.taptap.cn/help"
  }
}
```

---

## shortcuts - 快捷方式配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `desktop_shortcut` | boolean | `true` | 是否支持创建桌面快捷方式 |
| `desktop_default` | boolean | `true` | 桌面快捷方式默认是否勾选 |
| `start_menu` | boolean | `true` | 是否支持创建开始菜单快捷方式 |
| `start_menu_folder` | string | `"{project.name}"` | 开始菜单文件夹名称 |

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

---

## autostart - 开机自启配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | boolean | `false` | 是否支持开机自启选项 |
| `default` | boolean | `false` | 开机自启默认是否勾选 |
| `registry_key` | string | `"Software\\Microsoft\\Windows\\CurrentVersion\\Run"` | 自启注册表键路径 |
| `registry_value_name` | string | `"{project.name}"` | 自启注册表值名称 |

```json
{
  "autostart": {
    "enabled": true,
    "default": false,
    "registry_key": "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
    "registry_value_name": "TapTap"
  }
}
```

---

## localization - 多语言配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `default_locale` | string | `"zh-CN"` | 默认语言代码 |
| `supported_locales` | string[] | `["zh-CN"]` | 支持的语言列表 |
| `show_language_selector` | boolean | `false` | 是否在 UI 中显示语言选择器 |

```json
{
  "localization": {
    "default_locale": "zh-CN",
    "supported_locales": ["zh-CN", "zh-TW", "en-US", "ja", "ko"],
    "show_language_selector": false
  }
}
```

---

## links - 外部链接

`links` 是一个 `HashMap<String, String>`，键为链接标识符，值为 URL。在 XML 布局中通过 `action="open_url:KEY"` 引用。

```json
{
  "links": {
    "homepage": "https://www.taptap.cn",
    "agreement": "https://www.taptap.cn/agreement",
    "privacy": "https://www.taptap.cn/privacy"
  }
}
```

---

## resources - 资源路径配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `layouts_dir` | string | `"layouts"` | XML 布局文件目录（相对于项目根目录） |
| `assets_dir` | string | `"assets"` | 图片等资源文件目录 |
| `locales_dir` | string | `"locales"` | 语言文件目录 |
| `payload_dir` | string | `""` | 待打包文件的源目录 |
| `payload_file` | string | `""` | 预打包的 7z/payload 文件路径 |
| `installer_icon` | string | `""` | 安装程序图标（覆盖 output.installer_icon） |
| `uninstaller_icon` | string | `""` | 卸载程序图标（覆盖 output.uninstaller_icon） |

```json
{
  "resources": {
    "layouts_dir": "layouts",
    "assets_dir": "assets",
    "locales_dir": "locales",
    "payload_dir": "payload",
    "payload_file": "payload.7z"
  }
}
```

---

## ui - 界面配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `window_width` | number | `800` | 窗口宽度（逻辑像素） |
| `window_height` | number | `460` | 窗口高度（逻辑像素） |
| `expanded_height` | number | `600` | 展开面板后的窗口高度（逻辑像素） |
| `dpi_aware` | boolean | `true` | 是否启用 DPI 感知 |
| `dpi_threshold` | number | `1.5` | DPI 缩放阈值，超过此值使用 @2x 资源 |

```json
{
  "ui": {
    "window_width": 800,
    "window_height": 460,
    "expanded_height": 600,
    "dpi_aware": true,
    "dpi_threshold": 1.5
  }
}
```

**注意：** 窗口尺寸使用逻辑像素，egui 会根据系统 DPI 自动缩放。不要设置 `pixels_per_point`。

---

## wizard - 向导页面配置

定义安装和卸载的页面流程。每个页面由字符串 `id` 标识，对应一个 XML 布局文件。

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `pages` | Page[] | `[]` | 安装向导页面列表 |
| `uninstall_pages` | Page[] | `[]` | 卸载向导页面列表 |

**Page 对象字段：**

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | string | 页面唯一标识符（如 `"config"`、`"installing"`、`"finish"`） |
| `layout` | string | 对应的 XML 布局文件名（不含扩展名） |
| `title` | string | 页面标题（可选，用于调试） |

```json
{
  "wizard": {
    "pages": [
      { "id": "config", "layout": "configpage", "title": "配置" },
      { "id": "installing", "layout": "installingpage", "title": "安装中" },
      { "id": "finish", "layout": "finishpage", "title": "完成" }
    ],
    "uninstall_pages": [
      { "id": "confirm", "layout": "uninstallpage", "title": "确认卸载" },
      { "id": "uninstalling", "layout": "uninstallingpage", "title": "卸载中" },
      { "id": "finish", "layout": "uninstallfinishpage", "title": "卸载完成" }
    ]
  }
}
```

---

## uninstall - 卸载配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `show_keep_data_option` | boolean | `false` | 是否显示"保留用户数据"选项 |
| `keep_data_default` | boolean | `true` | "保留用户数据"默认是否勾选 |
| `data_paths` | string[] | `[]` | 卸载时需要清理的用户数据路径（支持环境变量） |

```json
{
  "uninstall": {
    "show_keep_data_option": true,
    "keep_data_default": true,
    "data_paths": [
      "%APPDATA%\\TapTap"
    ]
  }
}
```

---

## validation - 路径验证配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `check_path_legal` | boolean | `true` | 是否检查安装路径合法性（非法字符等） |
| `check_disk_type` | boolean | `false` | 是否检查磁盘类型（如拒绝网络驱动器） |
| `check_disk_space` | boolean | `true` | 是否检查磁盘可用空间 |

```json
{
  "validation": {
    "check_path_legal": true,
    "check_disk_type": false,
    "check_disk_space": true
  }
}
```

---

## advanced - 高级配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `silent_mode_support` | boolean | `false` | 是否支持静默安装（`/S` 命令行参数） |
| `update_mode_support` | boolean | `false` | 是否支持更新模式 |
| `uninstall_mode_support` | boolean | `true` | 是否支持卸载模式 |
| `launch_app_after_install` | boolean | `true` | 安装完成后是否启动应用 |

```json
{
  "advanced": {
    "silent_mode_support": false,
    "update_mode_support": false,
    "uninstall_mode_support": true,
    "launch_app_after_install": true
  }
}
```

---

## 完整配置示例

```json
{
  "project": {
    "name": "TapTap",
    "version": "3.0.0",
    "publisher": "TapTap",
    "copyright": "Copyright 2024 TapTap",
    "output_name": "TapTap_Setup"
  },
  "output": {
    "installer_name": "TapTap_Setup.exe",
    "installer_icon": "assets/installer.ico",
    "uninstaller_name": "uninst.exe",
    "uninstaller_icon": "assets/uninstaller.ico"
  },
  "install": {
    "exe_name": "TapTap.exe",
    "default_path": "C:\\Program Files\\TapTap",
    "required_space_mb": 500,
    "require_admin": false,
    "mutex_name": "TapTapInstaller",
    "detect_running_process": "TapTap.exe",
    "kill_process_on_install": true,
    "kill_process_on_uninstall": true
  },
  "registry": {
    "install_path_key": "Software\\TapTap\\InstallPath",
    "uninstall_key": "TapTap",
    "help_link": "https://www.taptap.cn/help"
  },
  "shortcuts": {
    "desktop_shortcut": true,
    "desktop_default": true,
    "start_menu": true,
    "start_menu_folder": "TapTap"
  },
  "autostart": {
    "enabled": true,
    "default": false
  },
  "localization": {
    "default_locale": "zh-CN",
    "supported_locales": ["zh-CN", "zh-TW", "en-US", "ja", "ko"],
    "show_language_selector": false
  },
  "links": {
    "homepage": "https://www.taptap.cn",
    "agreement": "https://www.taptap.cn/agreement",
    "privacy": "https://www.taptap.cn/privacy"
  },
  "resources": {
    "layouts_dir": "layouts",
    "assets_dir": "assets",
    "locales_dir": "locales",
    "payload_dir": "payload"
  },
  "ui": {
    "window_width": 800,
    "window_height": 460,
    "expanded_height": 600,
    "dpi_aware": true,
    "dpi_threshold": 1.5
  },
  "wizard": {
    "pages": [
      { "id": "config", "layout": "configpage", "title": "配置" },
      { "id": "installing", "layout": "installingpage", "title": "安装中" },
      { "id": "finish", "layout": "finishpage", "title": "完成" }
    ],
    "uninstall_pages": [
      { "id": "confirm", "layout": "uninstallpage", "title": "确认卸载" },
      { "id": "uninstalling", "layout": "uninstallingpage", "title": "卸载中" },
      { "id": "finish", "layout": "uninstallfinishpage", "title": "卸载完成" }
    ]
  },
  "uninstall": {
    "show_keep_data_option": true,
    "keep_data_default": true,
    "data_paths": ["%APPDATA%\\TapTap"]
  },
  "validation": {
    "check_path_legal": true,
    "check_disk_type": false,
    "check_disk_space": true
  },
  "advanced": {
    "silent_mode_support": false,
    "update_mode_support": false,
    "uninstall_mode_support": true,
    "launch_app_after_install": true
  }
}
```

## 什么时候使用脚本

配置只描述安装器的通用能力，例如：

- 快捷方式
- 开机自启
- 通用注册表键
- 卸载时的用户数据保留选项

产品业务副作用应放到安装或卸载脚本里实现，例如：

- 写入 `channel.conf`
- 写入渠道文件或业务配置文件
- 清理产品自定义目录
- 删除产品自定义注册表键

通用能力优先走配置；无法抽象成通用能力的产品逻辑，统一走脚本。

---

## 相关文档

- [XML 布局指南](XML_LAYOUT_GUIDE.md) - XML 布局格式详解
- [多语言键值参考](LOCALE_KEYS.md) - 所有 locale 键值说明
- [示例项目](../examples/TapTap/README.md) - TapTap 安装程序示例
