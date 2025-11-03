# TapTap 安装器示例

这是一个完整的 nano-installer 示例项目，展示了如何为 TapTap 游戏平台创建专业的安装程序。

## 项目结构

```
TapTap/
├── installer_config.json    # 主配置文件
├── assets/                  # 图片资源
│   ├── logo.png            # 应用图标（1x）
│   ├── logo@2x.png         # 应用图标（2x，高DPI）
│   ├── bg_main.png         # 主背景
│   ├── bg_main@2x.png      # 主背景（高DPI）
│   ├── btn_*.png           # 按钮素材
│   ├── checkbox-*.png      # 复选框素材
│   └── ...
├── layouts/                 # XML 布局文件
│   ├── welcome.xml         # 欢迎页面
│   ├── welcome@2x.xml      # 欢迎页面（高DPI）
│   ├── config.xml          # 配置页面
│   ├── installing.xml      # 安装进度页面
│   └── finish.xml          # 完成页面
├── locales/                 # 多语言文件
│   ├── zh-CN.json          # 简体中文
│   ├── zh-TW.json          # 繁体中文
│   ├── en-US.json          # 英文
│   ├── ja.json             # 日语
│   ├── ko.json             # 韩语
│   └── ...（11种语言）
├── files/                   # 待安装的文件
│   └── （你的应用程序文件）
├── dist/                    # 构建输出
│   └── TapTap_Setup.exe    # 生成的安装程序
├── build.ps1               # Windows 构建脚本
├── build.sh                # Linux/Mac 构建脚本
└── run_installer.ps1       # 测试脚本
```

## 快速开始

### 1. 准备文件

将你的应用程序文件放入 `files/` 目录：

```
files/
├── TapTap.exe
├── config/
├── resources/
└── ...
```

### 2. 构建安装程序

**Windows:**
```powershell
.\build.ps1
```

**Linux/Mac:**
```bash
./build.sh
```

### 3. 测试

```powershell
.\run_installer.ps1
```

或直接运行：
```powershell
.\dist\TapTap_Setup.exe
```

## 配置说明

### 基本信息 (`installer_config.json`)

```json
{
  "project": {
    "name": "TapTap",
    "version": "3.24.0",
    "publisher": "TapTap",
    "website": "https://www.taptap.cn"
  }
}
```

修改为你自己的应用信息。

### 安装设置

```json
{
  "install": {
    "exe_name": "TapTap.exe",
    "default_path": "{localappdata}\\TapTap",
    "required_space_mb": 500,
    "require_admin": false
  }
}
```

**路径变量：**
- `{pf64}` - `C:\Program Files`
- `{pf32}` - `C:\Program Files (x86)`
- `{localappdata}` - `C:\Users\用户名\AppData\Local`
- `{appdata}` - `C:\Users\用户名\AppData\Roaming`

### 快捷方式

```json
{
  "shortcuts": {
    "desktop": true,
    "start_menu": true,
    "quick_launch": false,
    "common_desktop": false
  }
}
```

- `desktop` - 当前用户桌面
- `start_menu` - 开始菜单
- `quick_launch` - 快速启动栏
- `common_desktop` - 所有用户桌面（需管理员权限）

### 自启动

```json
{
  "autostart": {
    "enabled": true,
    "name": "TapTap",
    "path": "{install_dir}\\TapTap.exe",
    "args": "--background"
  }
}
```

### 注册表

```json
{
  "registry": {
    "app_paths": true,
    "uninstall_info": true,
    "file_associations": [],
    "custom_keys": []
  }
}
```

### UI 配置

```json
{
  "ui": {
    "window_width": 574,
    "window_height": 358,
    "expanded_height": 450,
    "theme": "dark",
    "dpi_threshold": 144
  }
}
```

- `window_width/height` - 窗口尺寸
- `expanded_height` - 展开时的高度（显示详情时）
- `theme` - 主题：`"dark"` 或 `"light"`
- `dpi_threshold` - DPI 阈值，超过此值使用 2x 资源

### 语言

```json
{
  "localization": {
    "default_locale": "zh-CN",
    "fallback_locale": "en-US",
    "supported_locales": [
      "zh-CN", "zh-TW", "en-US", "ja", "ko",
      "th", "vi", "id", "pt", "es", "ru"
    ]
  }
}
```

## 自定义界面

### 修改欢迎页面

编辑 `layouts/welcome.xml`：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="Welcome" version="1.0.0">
  <Page>
    <VBox>
      <!-- 修改 Logo -->
      <Image icon="assets/logo.png" width="200" height="60" />
      
      <!-- 修改标题 -->
      <Label text="欢迎使用 TapTap" font_size="24" />
      
      <!-- 修改描述 -->
      <Label text="TapTap 是领先的移动游戏社区平台" />
      
      <!-- 按钮 -->
      <HBox>
        <Button id="next" text="下一步" />
        <Button id="cancel" text="取消" />
      </HBox>
    </VBox>
  </Page>
</Layout>
```

### 添加新页面

1. 在 `layouts/` 中创建 `my_page.xml`
2. 在 `installer_config.json` 中注册：

```json
{
  "wizard": {
    "pages": [
      { "id": "welcome", "layout": "welcome.xml" },
      { "id": "my_page", "layout": "my_page.xml" },
      { "id": "config", "layout": "config.xml" }
    ]
  }
}
```

## 多语言

### 添加新语言

1. 复制 `locales/en-US.json` 为 `locales/fr.json`
2. 翻译所有字符串
3. 在配置中添加：

```json
{
  "localization": {
    "supported_locales": ["zh-CN", "en-US", "fr"]
  }
}
```

### 在布局中使用

```xml
<Label text="{welcome_message}" />
```

对应 `locales/zh-CN.json`：

```json
{
  "strings": {
    "welcome_message": "欢迎使用 TapTap"
  }
}
```

## 高 DPI 支持

### 图片资源

为每个图片提供 2 倍版本：

```
assets/
├── logo.png       # 100x30 像素
└── logo@2x.png    # 200x60 像素（2倍大小）
```

系统会根据屏幕 DPI 自动选择。

### 布局文件（可选）

如果需要为高 DPI 屏幕定制布局：

```
layouts/
├── welcome.xml       # 标准 DPI
└── welcome@2x.xml    # 高 DPI
```

大多数情况下不需要 `@2x.xml`，只需提供 `@2x` 图片即可。

## 构建脚本说明

### `build.ps1` (Windows)

```powershell
# 构建
.\build.ps1

# 清理构建
.\build.ps1 -Clean
```

**脚本功能：**
1. 检查 nano-installer 是否已编译
2. 如未编译，自动编译 `installer.exe`
3. 复制 `installer.exe` 到 `dist/TapTap_Setup.exe`
4. 创建 `payload.7z`（从 `files/` 目录）
5. 复制 `installer_config.json` 到 `dist/`

### `build.sh` (Linux/Mac)

```bash
# 构建
./build.sh

# 清理构建
./build.sh --clean
```

功能与 `build.ps1` 相同。

## 测试

### 测试安装

```powershell
.\run_installer.ps1
```

### 静默安装

```powershell
.\dist\TapTap_Setup.exe --silent --install-path "D:\TapTap"
```

### 指定语言

```powershell
.\dist\TapTap_Setup.exe --config installer_config.json
```

### 调试模式

在 `installer_config.json` 中启用：

```json
{
  "advanced": {
    "debug_mode": true,
    "log_level": "debug"
  }
}
```

查看日志：`%TEMP%\nano-installer-XXXX\install.log`

## 常见问题

### Q: 如何修改窗口图标？

A: 替换 `assets/logo.ico`，并在配置中指定：

```json
{
  "resources": {
    "icon": "assets/logo.ico"
  }
}
```

### Q: 如何添加许可协议？

A: 在配置中指定许可文件：

```json
{
  "wizard": {
    "show_license": true
  },
  "resources": {
    "license_file": "LICENSE.txt"
  }
}
```

### Q: 如何检测已安装版本？

A: 配置升级检测：

```json
{
  "wizard": {
    "detect_installed": true,
    "allow_downgrade": false
  }
}
```

### Q: 如何自定义按钮颜色？

A: 在 UI 配置中设置：

```json
{
  "ui": {
    "colors": {
      "primary": "#1890FF",
      "background": "#1A1D28"
    }
  }
}
```

## 进阶功能

### 渠道标识

为不同渠道创建不同的安装包：

```json
{
  "channel": {
    "enabled": true,
    "default": "official",
    "options": ["official", "steam", "epic"]
  }
}
```

### 卸载配置

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

### 安装后启动

```json
{
  "install": {
    "launch_after_install": true,
    "launch_exe": "{install_dir}\\TapTap.exe",
    "launch_args": ""
  }
}
```

## 相关文档

- [配置参考](../../docs/CONFIG_REFERENCE.md) - 完整配置说明
- [XML 布局指南](../../docs/XML_LAYOUT_GUIDE.md) - 界面自定义
- [多语言指南](../../docs/LOCALIZATION.md) - 添加语言

## 获取帮助

- 查看主文档：[../../README.md](../../README.md)
- 问题反馈：[GitHub Issues](https://github.com/yourusername/nano-installer/issues)

---

**提示：** 这个示例包含了 nano-installer 的大部分功能。你可以基于此创建自己的安装程序！
