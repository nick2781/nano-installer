# nano-installer 文档

`nano-installer` 把产品配置、XML 布局、多语言资源、Rhai 脚本、payload 和卸载器打包为
单个 Windows setup。当前正式运行环境为 Windows 10/11 x64。

## 按角色阅读

准备接入产品：

1. [生产接入](PRODUCTION_INTEGRATION.md)
2. [配置参考](CONFIG_REFERENCE.md)
3. [XML 布局指南](XML_LAYOUT_GUIDE.md)
4. [本地化](LOCALIZATION.md)
5. [配置与脚本边界](CONFIG_VS_SCRIPT.md)

维护框架：

1. [架构](ARCHITECTURE.md)
2. [项目结构](PROJECT_STRUCTURE.md)
3. [开发](DEVELOPMENT.md)
4. [测试](TEST_PLAN.md)
5. [发布](RELEASE.md)

评估系统支持：

- [Windows 兼容性](WINDOWS_COMPATIBILITY.md)
- [FAQ](FAQ.md)

仓库只保留一个可构建示例：[examples/TapTap](../examples/TapTap/)。`.build/` 和
`dist/` 是生成目录，不是产品配置的来源。
