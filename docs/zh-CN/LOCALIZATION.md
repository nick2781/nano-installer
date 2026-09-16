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

## 尚未实现

- 自动校验各语言文件的键集合是否一致。
- 缺失键的告警；当前缺失键会回退显示键名本身。
- 独立的语言包格式。语言文件就是项目内的普通 JSON。
