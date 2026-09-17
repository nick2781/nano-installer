# 多语言

每种语言就是 `locales` 目录下的一个 UTF-8 JSON 文件，文件名就是语言代码：`en-US.json`、`ja.json`、
`zh-CN.json` 等。

```json
{
  "install_button": "立即安装",
  "version_info": "版本号 1.11.3.10000"
}
```

XML 布局用 `@` 引用键：

```xml
<Button text="@install_button" ... />
<Label text="@version_info" ... />
```

启动时，运行时读 `<locales_dir>/<default_locale>.json`；`default_locale` 来自配置，
默认是 `zh-CN`。文字用 Unicode API 绘制，所以不存在 ANSI 代码路径，运行时也没有 ANSI 版本。

## 提供多种语言

在页面上放一个语言选择控件，每项对应一个语言代码：

```xml
<Select action="switch_language" ...>
  <Option value="zh-CN" text="简体中文" />
  <Option value="en-US" text="English" />
</Select>
```

选中一项后，运行时会立刻重新加载对应的语言文件，并把页面文字重绘一遍。
`Option` 的 `value` 必须和 `locales` 目录里的 JSON 文件名一致。

## 不点击也能验证某种语言

自动截图和快速检查可以先把语言选好：

```powershell
$env:NANO_INSTALLER_TEST_LOCALE = "en-US"
.\examples\TapTap\dist\TapTap_Setup.exe
```

## 构建会替你检查什么

每次构建都会把各语言文件与默认语言、页面实际引用的键比一遍，把缺口写进构建器的告警列表：

- 某个语言缺少默认语言里已有的文案，并列出具体的键名。
- `localization.supported_locales` 里写了、但没有对应 JSON 文件的语言。

构建只报告默认语言里存在的键，所以页面可以故意引用某个语言没有翻的键。
运行时遇到缺失的键仍然回退到默认语言，只翻了一半的安装包也能正常跑完。

## 尚未实现

- 没有独立的语言包格式：语言文件就是项目里的普通 JSON。
- 不做两种译文之间的互相比对；现在只以默认语言为基准。
