# 多语言

每种语言是 `locales` 目录下的一个 UTF-8 JSON 文件，文件名即语言代码：`en-US.json`、`ja.json`、
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

启动时运行时读取 `<locales_dir>/<default_locale>.json`，`default_locale` 来自配置，默认
`zh-CN`。文字使用 Unicode API 绘制，没有 ANSI 代码路径，也没有 ANSI 版本的运行时。

## 提供多种语言

在页面上加入语言选择控件，每项对应一个语言代码：

```xml
<Select action="switch_language" ...>
  <Option value="zh-CN" text="简体中文" />
  <Option value="en-US" text="English" />
</Select>
```

选择后立即重新加载对应的语言文件并重绘页面文字。Option 的 value 必须与 locales 目录中的 JSON
文件名一致。

## 不点击也能验证某种语言

自动截图和快速检查可以预选语言：

```powershell
$env:NANO_INSTALLER_TEST_LOCALE = "en-US"
.\examples\TapTap\dist\TapTap_Setup.exe
```

## 构建会替你检查什么

每次构建都会拿各语言文件与默认语言、以及页面实际引用的键做比对，把缺口写进构建器的告警列表：

- 某个语言缺少默认语言里已有的文案，并列出具体键名。
- `localization.supported_locales` 里写了、但没有对应 JSON 文件的语言。

只报告默认语言里存在的键，因此页面可以故意引用某个语种不翻译的文案。运行时缺失的键仍回退到
默认语言，所以翻到一半的安装包能正常跑完，而不是显示空白。

## 尚未实现

- 独立的语言包格式。语言文件就是项目内的普通 JSON。
- 两种译文之间的互相比对；目前只以默认语言为基准。
