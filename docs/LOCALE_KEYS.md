# 多语言键值参考 (Locale Keys)

本文档列出 nano-installer 使用的所有 locale 键。这些键定义在 `locales/*.json` 文件中，在 XML 布局中通过 `{key}` 语法引用。

## 语言文件格式

```json
{
  "language_name": "简体中文",
  "strings": {
    "install_button": "立即安装",
    "status.preparing": "正在准备...",
    "error.path_empty": "安装路径不能为空"
  }
}
```

- 文件路径：`locales/{locale_code}.json`（如 `locales/zh-CN.json`、`locales/en-US.json`）
- 所有键都定义在 `strings` 对象中
- 使用 `.` 分隔的层级键名（如 `status.preparing`）

---

## UI 键 - 界面文案

这些键用于按钮文字、标签文本等常规 UI 元素。

| 键 | 说明 | 示例值 (zh-CN) |
|-----|------|----------------|
| `install_button` | 安装按钮文字 | `"立即安装"` |
| `agree_prefix` | 协议同意前缀文字 | `"同意"` |
| `agreement_link` | 用户协议链接文字 | `"《用户协议》"` |
| `policy_link` | 隐私政策链接文字 | `"《隐私政策》"` |
| `show_more` | 展开更多选项按钮 | `"自定义安装"` |
| `hide_more` | 收起选项按钮 | `"收起"` |
| `lbl_and` | 连接词"和" | `"和"` |
| `browse` | 浏览按钮文字 | `"浏览"` |
| `required_space` | 所需空间标签 | `"所需空间"` |
| `available_space` | 可用空间标签 | `"可用空间"` |
| `shortcut_checkbox` | 桌面快捷方式复选框 | `"创建桌面快捷方式"` |
| `autorun_checkbox` | 开机自启复选框 | `"开机自动启动"` |
| `installing_text` | 安装中文字 | `"正在安装..."` |
| `install_complete` | 安装完成文字 | `"安装完成"` |
| `launch_button` | 启动应用按钮 | `"立即体验"` |
| `uninstall_confirm` | 卸载确认文字 | `"确定要卸载 {product_name} 吗？"` |
| `not_now` | "暂不"按钮 | `"暂不卸载"` |
| `uninstall_button` | 卸载按钮文字 | `"确认卸载"` |
| `reserve_data` | 保留数据复选框 | `"保留用户数据"` |
| `uninstalling_text` | 卸载中文字 | `"正在卸载..."` |
| `uninstall_complete` | 卸载完成文字 | `"卸载完成"` |
| `uninstall_done` | 卸载完成按钮 | `"完成"` |
| `cancel` | 取消按钮 | `"取消"` |
| `ok` | 确定按钮 | `"确定"` |
| `close_confirm_message` | 关闭确认消息 | `"安装尚未完成，确定要退出吗？"` |

### 在 XML 中使用

```xml
<Button name="install_btn" text="{install_button}" action="install" />
<Label text="{agree_prefix}{agreement_link}{lbl_and}{policy_link}" />
<CheckBox name="desktop_shortcut" text="{shortcut_checkbox}" selected="true" />
```

---

## 状态键 - 安装/卸载进度

这些键用于显示安装和卸载过程中的状态文字。

### 安装状态

| 键 | 说明 | 示例值 (zh-CN) |
|-----|------|----------------|
| `status.preparing` | 准备安装 | `"正在准备..."` |
| `status.install_complete` | 安装完成状态 | `"安装完成！"` |

### 卸载状态

| 键 | 说明 | 示例值 (zh-CN) |
|-----|------|----------------|
| `uninstall.status.closing_app` | 正在关闭应用 | `"正在关闭应用程序..."` |
| `uninstall.status.removing_shortcuts` | 正在删除快捷方式 | `"正在删除快捷方式..."` |
| `uninstall.status.cleaning_registry` | 正在清理注册表 | `"正在清理注册表..."` |
| `uninstall.status.removing_user_data` | 正在删除用户数据 | `"正在删除用户数据..."` |
| `uninstall.status.removing_files` | 正在删除文件 | `"正在删除文件..."` |
| `uninstall.status.finishing` | 正在完成 | `"正在完成..."` |
| `uninstall.status.complete` | 卸载完成 | `"卸载完成"` |

### 在 XML 中使用

状态键通常不直接写在 XML 中，而是由安装/卸载引擎在运行时动态设置到 `status_text` 元素：

```xml
<!-- 状态文字标签，内容在运行时更新 -->
<Label name="status_text" text="{status.preparing}" />
```

---

## 错误键 - 验证和错误消息

这些键用于路径验证和错误提示。

| 键 | 说明 | 示例值 (zh-CN) |
|-----|------|----------------|
| `error.path_empty` | 路径为空 | `"安装路径不能为空"` |
| `error.path_illegal_char` | 路径包含非法字符 | `"安装路径包含非法字符"` |
| `error.no_write_permission` | 没有写入权限 | `"没有写入权限，请选择其他路径"` |
| `error.not_enough_space` | 磁盘空间不足 | `"磁盘空间不足"` |
| `error.path_too_long` | 路径过长 | `"安装路径过长"` |
| `error.network_drive` | 网络驱动器 | `"不支持安装到网络驱动器"` |
| `error.extraction_failed` | 解压失败 | `"文件解压失败"` |
| `error.process_running` | 目标进程正在运行 | `"{product_name} 正在运行，请先关闭后重试"` |

### 在 XML 中使用

错误消息通常由验证逻辑动态设置：

```xml
<!-- 错误提示标签，默认隐藏 -->
<Label name="error_text" text="" textcolor="#FF4444" visible="false" />
```

---

## 添加新语言

1. 在 `locales/` 目录下创建新的语言文件（如 `locales/fr.json`）
2. 翻译所有键值
3. 在 `installer_config.json` 的 `localization.supported_locales` 中添加语言代码

```json
{
  "language_name": "Francais",
  "strings": {
    "install_button": "Installer",
    "cancel": "Annuler",
    "ok": "OK",
    ...
  }
}
```

**注意：** 所有语言文件必须包含相同的键集合。缺失的键会回退到 `default_locale` 中的值。

---

## 变量替换

locale 值中支持以下变量：

| 变量 | 说明 |
|------|------|
| `{product_name}` | 产品名称（来自 `project.name`） |
| `{version}` | 版本号（来自 `project.version`） |
| `{publisher}` | 发布者（来自 `project.publisher`） |
| `{install_path}` | 当前安装路径 |
| `{required_space}` | 所需磁盘空间 |

示例：

```json
{
  "uninstall_confirm": "确定要卸载 {product_name} 吗？",
  "error.process_running": "{product_name} 正在运行，请先关闭后重试"
}
```

---

## 相关文档

- [XML 布局指南](XML_LAYOUT_GUIDE.md) - 如何在 XML 中引用 locale 键
- [配置参考](CONFIG_REFERENCE.md) - 多语言配置说明
- [示例项目](../examples/TapTap/README.md) - 查看完整的多语言配置示例
