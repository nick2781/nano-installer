# 多语言支持指南

本文档说明如何为 nano-installer 添加和自定义多语言支持。

## 📖 目录

- [快速开始](#快速开始)
- [语言文件结构](#语言文件结构)
- [添加新语言](#添加新语言)
- [在 XML 中使用](#在-xml-中使用)
- [语言代码规范](#语言代码规范)
- [最佳实践](#最佳实践)

## 快速开始

### 1. 查看现有语言

查看 `locales/` 目录：

```
locales/
├── zh-CN.json      # 简体中文
├── zh-TW.json      # 繁体中文
├── en-US.json      # 英语
├── ja.json         # 日语
└── ...
```

### 2. 配置支持的语言

在 `installer_config.json` 中：

```json
{
  "localization": {
    "default_locale": "zh-CN",
    "fallback_locale": "en-US",
    "supported_locales": [
      "zh-CN",
      "zh-TW",
      "en-US",
      "ja",
      "ko"
    ],
    "show_language_selector": true
  }
}
```

### 3. 在布局中引用

在 XML 布局文件中使用 `{key}` 语法：

```xml
<Label text="{welcome_message}" />
<Button text="{install_button}" />
```

## 语言文件结构

语言文件采用 JSON 格式：

```json
{
  "language_name": "简体中文",
  "strings": {
    "welcome_title": "欢迎使用",
    "welcome_message": "欢迎使用 TapTap 游戏平台",
    "install_button": "立即安装",
    "cancel_button": "取消",
    "next_button": "下一步",
    "back_button": "上一步"
  }
}
```

### 字段说明

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `language_name` | string | ✅ | 语言的本地名称（显示在语言选择器中） |
| `strings` | object | ✅ | 字符串键值对 |

## 添加新语言

### 步骤 1：创建语言文件

复制现有语言文件：

```bash
# 以法语为例
cp locales/en-US.json locales/fr.json
```

### 步骤 2：翻译字符串

编辑 `locales/fr.json`：

```json
{
  "language_name": "Français",
  "strings": {
    "welcome_title": "Bienvenue",
    "welcome_message": "Bienvenue sur la plateforme de jeu TapTap",
    "install_button": "Installer maintenant",
    "cancel_button": "Annuler",
    "next_button": "Suivant",
    "back_button": "Précédent"
  }
}
```

### 步骤 3：更新配置

在 `installer_config.json` 中添加新语言：

```json
{
  "localization": {
    "supported_locales": [
      "zh-CN",
      "en-US",
      "fr"  // 新添加的法语
    ]
  }
}
```

### 步骤 4：测试

运行安装程序，在语言选择器中应该能看到新语言。

## 在 XML 中使用

### 基本用法

使用 `{key}` 引用语言字符串：

```xml
<Label text="{welcome_message}" />
```

### 带变量的文本

语言文件中可以包含变量占位符：

**locales/zh-CN.json:**
```json
{
  "strings": {
    "install_to": "安装到：{install_path}",
    "version_info": "版本 {version}",
    "required_space": "需要 {space} MB 空间"
  }
}
```

**XML 布局：**
```xml
<Label text="{install_to}" />
<Label text="{version_info}" />
```

变量会自动替换为实际值。

### 系统变量

以下变量自动可用：

| 变量 | 来源 | 示例值 |
|------|------|--------|
| `{product_name}` | `project.name` | "TapTap" |
| `{version}` | `project.version` | "3.24.0" |
| `{publisher}` | `project.publisher` | "TapTap" |
| `{install_path}` | 用户选择 | "C:\Program Files\TapTap" |
| `{required_space}` | `install.required_space_mb` | "500" |

### 直接文本 vs 语言键

**直接文本（不推荐）：**
```xml
<Label text="欢迎使用 TapTap" />
```

**语言键（推荐）：**
```xml
<Label text="{welcome_message}" />
```

使用语言键的好处：
- ✅ 支持多语言
- ✅ 易于维护和更新
- ✅ 专业的国际化

## 语言代码规范

### 格式

语言代码遵循 BCP 47 标准：

```
语言-地区
```

### 常用语言代码

| 代码 | 语言 | 原生名称 |
|------|------|----------|
| `zh-CN` | 简体中文 | 简体中文 |
| `zh-TW` | 繁体中文 | 繁體中文 |
| `en-US` | 英语（美国） | English (US) |
| `en-GB` | 英语（英国） | English (UK) |
| `ja` | 日语 | 日本語 |
| `ko` | 韩语 | 한국어 |
| `de` | 德语 | Deutsch |
| `fr` | 法语 | Français |
| `es` | 西班牙语 | Español |
| `pt` | 葡萄牙语 | Português |
| `pt-BR` | 葡萄牙语（巴西） | Português (Brasil) |
| `ru` | 俄语 | Русский |
| `ar` | 阿拉伯语 | العربية |
| `th` | 泰语 | ไทย |
| `vi` | 越南语 | Tiếng Việt |
| `id` | 印尼语 | Bahasa Indonesia |
| `it` | 意大利语 | Italiano |
| `pl` | 波兰语 | Polski |
| `tr` | 土耳其语 | Türkçe |

### 文件命名

- ✅ `zh-CN.json`（推荐）
- ✅ `en-US.json`
- ❌ `chinese.json`（不推荐）
- ❌ `中文.json`（不推荐）

## 最佳实践

### 1. 使用描述性的键名

**好：**
```json
{
  "welcome_title": "欢迎",
  "install_button": "安装",
  "error_insufficient_space": "磁盘空间不足"
}
```

**不好：**
```json
{
  "text1": "欢迎",
  "btn1": "安装",
  "err1": "磁盘空间不足"
}
```

### 2. 保持一致的命名风格

使用 `snake_case` 或 `camelCase`，在整个项目中保持一致：

```json
{
  "welcome_title": "...",
  "install_button": "...",
  "error_message": "..."
}
```

### 3. 分组相关的字符串

```json
{
  "strings": {
    "welcome_title": "欢迎",
    "welcome_message": "欢迎使用...",
    "welcome_subtitle": "开始安装",
    
    "install_button": "安装",
    "install_progress": "正在安装...",
    "install_complete": "安装完成",
    
    "error_no_space": "空间不足",
    "error_permission": "权限不足",
    "error_network": "网络错误"
  }
}
```

### 4. 处理复数和性别

某些语言有复数形式或性别差异，使用变量处理：

```json
{
  "strings": {
    "files_count": "{count} 个文件"
  }
}
```

### 5. 考虑文本长度

不同语言的文本长度差异很大：

- 德语通常比英语长 30%
- 中文通常比英语短 30%
- 阿拉伯语是从右到左

在设计 UI 时预留足够空间。

### 6. 提供上下文注释

对于可能产生歧义的文本，在翻译文件中添加注释：

```json
{
  "strings": {
    "_comment_install_button": "按钮文本，动词，表示开始安装",
    "install_button": "安装",
    
    "_comment_install_noun": "名词，指安装过程或安装包",
    "install_noun": "安装程序"
  }
}
```

### 7. 测试所有语言

- 在每种语言下测试 UI
- 检查文本是否被截断
- 验证特殊字符显示正确
- 确保从右到左语言（如阿拉伯语）正确显示

### 8. 使用回退机制

配置 `fallback_locale`，当某个字符串缺失时使用回退语言：

```json
{
  "localization": {
    "default_locale": "fr",
    "fallback_locale": "en-US"
  }
}
```

如果 `fr.json` 中缺少某个键，会自动使用 `en-US.json` 中的值。

## 常见字符串参考

### 按钮

```json
{
  "next_button": "下一步",
  "back_button": "上一步",
  "cancel_button": "取消",
  "install_button": "安装",
  "finish_button": "完成",
  "browse_button": "浏览...",
  "agree_button": "我同意",
  "close_button": "关闭"
}
```

### 页面标题

```json
{
  "welcome_title": "欢迎",
  "license_title": "许可协议",
  "install_path_title": "安装位置",
  "installing_title": "正在安装",
  "finish_title": "安装完成"
}
```

### 选项

```json
{
  "desktop_shortcut": "创建桌面快捷方式",
  "start_menu_shortcut": "添加到开始菜单",
  "auto_start": "开机自动启动",
  "agree_license": "我已阅读并同意许可协议"
}
```

### 消息

```json
{
  "installing_message": "正在安装，请稍候...",
  "install_complete": "安装已完成",
  "install_failed": "安装失败",
  "disk_space_warning": "磁盘空间不足",
  "admin_required": "需要管理员权限"
}
```

### 状态

```json
{
  "status_extracting": "正在解压文件...",
  "status_copying": "正在复制文件...",
  "status_creating_shortcuts": "正在创建快捷方式...",
  "status_registering": "正在注册应用...",
  "status_cleaning": "正在清理临时文件..."
}
```

## 完整示例

查看 [examples/TapTap/locales/](../examples/TapTap/locales/) 中的完整语言文件示例。

## 相关文档

- [配置参考](CONFIG_REFERENCE.md) - localization 配置详解
- [XML 布局指南](XML_LAYOUT_GUIDE.md) - 如何在布局中使用语言字符串
- [示例项目](../examples/TapTap/README.md) - 查看多语言实际应用

---

有问题？查看 [主文档](../README.md) 或提交 Issue。

