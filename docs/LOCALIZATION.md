# 本地化

语言文件是 UTF-8 JSON 对象：

```json
{
  "install_button": "立即安装",
  "version_info": "版本号 1.11.3.10000"
}
```

XML 使用 `@key`：

```xml
<Button text="@install_button" ... />
<Label text="@version_info" ... />
```

运行时读取 `resources.locales_dir/<locale>.json`。默认 locale 来自
`localization.default_locale`，缺省为 `zh-CN`。

自动截图和开发验证可在启动前设置：

```powershell
$env:NANO_INSTALLER_TEST_LOCALE = "en-US"
.\examples\TapTap\dist\TapTap_Setup.exe
```

Select 会显示 value 等于当前 locale 的 Option 文案。用户点击切换语言、运行时重载页面、
缺失 key 检查以及不同语言 key 集合一致性校验尚未实现。

所有路径和文字使用 Unicode；不提供 ANSI 语言包或 ANSI stub。
