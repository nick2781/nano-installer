# nano-installer 文档

欢迎使用 nano-installer！这里是完整的文档索引。

## 🚀 快速开始

- **[主 README](../README.md)** - 项目介绍和快速开始
- **[示例项目](../examples/TapTap/README.md)** - 通过 TapTap 示例学习

## 📖 用户文档

### 核心文档

| 文档 | 说明 |
|------|------|
| **[配置参考](CONFIG_REFERENCE.md)** | 完整的配置文件说明，包含所有选项 |
| **[XML 布局指南](XML_LAYOUT_GUIDE.md)** | 如何使用 XML 自定义安装界面 |
| **[多语言支持](LOCALIZATION.md)** | 如何添加和管理多语言 |
| **[XML 布局规范](XML_SCHEMA.md)** | XML 文件的完整 Schema 定义 |
| **[JSON 配置规范](JSON_SCHEMA.md)** | JSON 配置文件的完整 Schema 定义 |

### 开发者文档

| 文档 | 说明 |
|------|------|
| [开发指南](DEVELOPMENT.md) | 如何参与 nano-installer 开发（修改源代码） |

## 📂 按主题浏览

### 配置和设置

- [项目信息配置](CONFIG_REFERENCE.md#project---项目信息) - 设置应用名称、版本等
- [安装设置](CONFIG_REFERENCE.md#install---安装设置) - 配置安装路径、权限等
- [快捷方式](CONFIG_REFERENCE.md#shortcuts---快捷方式) - 桌面和开始菜单快捷方式
- [开机自启](CONFIG_REFERENCE.md#autostart---开机自启) - 配置自动启动
- [注册表集成](CONFIG_REFERENCE.md#registry---注册表) - Windows 注册表设置

### 界面定制

- [XML 布局系统](XML_LAYOUT_GUIDE.md) - 完整的 XML 布局指南
- [布局元素](XML_LAYOUT_GUIDE.md#布局元素) - VBox, HBox, Button, Label 等
- [元素属性](XML_LAYOUT_GUIDE.md#元素属性) - 尺寸、样式、对齐等
- [变量替换](XML_LAYOUT_GUIDE.md#变量替换) - 在文本中使用动态变量
- [DPI 支持](XML_LAYOUT_GUIDE.md#dpi-支持) - 高分辨率屏幕适配

### 多语言

- [添加新语言](LOCALIZATION.md#添加新语言) - 创建语言文件
- [语言文件结构](LOCALIZATION.md#语言文件结构) - JSON 格式说明
- [在 XML 中使用](LOCALIZATION.md#在-xml-中使用) - 如何引用语言字符串
- [语言代码规范](LOCALIZATION.md#语言代码规范) - BCP 47 标准
- [最佳实践](LOCALIZATION.md#最佳实践) - 命名、组织、测试

### 高级功能

- [渠道标识](CONFIG_REFERENCE.md#channel---渠道标识) - 区分不同分发渠道
- [卸载配置](CONFIG_REFERENCE.md#uninstall---卸载配置) - 自定义卸载行为
- [路径校验](CONFIG_REFERENCE.md#validation---路径校验) - 安装路径验证规则
- [调试选项](CONFIG_REFERENCE.md#advanced---高级选项) - 日志和调试

## 🎓 教程和示例

### 从零开始

1. **[查看示例项目](../examples/TapTap/README.md)**
   - 完整的 TapTap 安装器示例
   - 展示所有功能的实际应用

2. **[配置你的第一个项目](CONFIG_REFERENCE.md)**
   - 创建 `installer_config.json`
   - 设置基本信息

3. **[自定义界面](XML_LAYOUT_GUIDE.md)**
   - 修改 XML 布局
   - 添加自己的品牌元素

4. **[添加多语言](LOCALIZATION.md)**
   - 创建语言文件
   - 支持国际用户

### 常见任务

- **修改窗口图标** → [resources 配置](CONFIG_REFERENCE.md#resources---资源路径)
- **更改默认安装路径** → [install 配置](CONFIG_REFERENCE.md#install---安装设置)
- **添加许可协议** → [wizard 配置](CONFIG_REFERENCE.md#wizard---安装向导)
- **创建桌面快捷方式** → [shortcuts 配置](CONFIG_REFERENCE.md#shortcuts---快捷方式)
- **区分不同渠道** → [channel 配置](CONFIG_REFERENCE.md#channel---渠道标识)
- **自定义卸载行为** → [uninstall 配置](CONFIG_REFERENCE.md#uninstall---卸载配置)

## 💡 提示和技巧

### 调试

启用调试模式查看详细日志：

```json
{
  "advanced": {
    "debug_mode": true,
    "log_level": "debug"
  }
}
```

日志文件位置：`%TEMP%\nano-installer-XXXX\install.log`

### 高 DPI 支持

为所有图片提供 2x 版本：

```
assets/
├── logo.png        # 100x30
└── logo@2x.png     # 200x60
```

系统会自动根据 DPI 选择合适的版本。

### 快速测试

使用命令行参数快速测试：

```powershell
# 静默安装到指定路径
.\MyApp_Setup.exe --silent --install-path "D:\Test"

# 指定语言
.\MyApp_Setup.exe --config installer_config.json
```

## 🔍 快速查找

### 我想...

- **更改窗口大小** → [ui.window_width/height](CONFIG_REFERENCE.md#ui---界面配置)
- **添加新按钮** → [XML Button 元素](XML_LAYOUT_GUIDE.md#button---按钮)
- **显示进度条** → [XML ProgressBar 元素](XML_LAYOUT_GUIDE.md#progressbar---进度条)
- **添加复选框** → [XML Checkbox 元素](XML_LAYOUT_GUIDE.md#checkbox---复选框)
- **翻译界面文本** → [多语言支持](LOCALIZATION.md)
- **需要管理员权限** → [install.require_admin](CONFIG_REFERENCE.md#install---安装设置)
- **检测已安装版本** → [wizard 配置](CONFIG_REFERENCE.md#wizard---安装向导)
- **在完成页面启动应用** → 在 finish.xml 中添加启动按钮

### 错误排查

| 问题 | 解决方案 |
|------|----------|
| 窗口显示不正常 | 检查 [ui 配置](CONFIG_REFERENCE.md#ui---界面配置) |
| 语言切换无效 | 检查 [localization 配置](CONFIG_REFERENCE.md#localization---多语言) |
| 布局解析失败 | 查看 [XML 布局指南](XML_LAYOUT_GUIDE.md#基本结构) |
| 图片加载失败 | 检查 [resources 配置](CONFIG_REFERENCE.md#resources---资源路径) |
| 安装失败 | 启用 [调试模式](CONFIG_REFERENCE.md#advanced---高级选项) |

## 📚 参考

### 配置选项总览

完整配置结构：

```json
{
  "project": { ... },         // 项目信息
  "install": { ... },         // 安装设置
  "registry": { ... },        // 注册表
  "shortcuts": { ... },       // 快捷方式
  "autostart": { ... },       // 开机自启
  "localization": { ... },    // 多语言
  "links": { ... },           // 外部链接
  "resources": { ... },       // 资源路径
  "ui": { ... },              // 界面配置
  "wizard": { ... },          // 安装向导
  "channel": { ... },         // 渠道标识
  "uninstall": { ... },       // 卸载配置
  "validation": { ... },      // 路径校验
  "advanced": { ... }         // 高级选项
}
```

详见 [完整配置参考](CONFIG_REFERENCE.md)

### XML 元素总览

- **容器**：VBox, HBox, Page
- **控件**：Button, Label, Image, Checkbox, TextInput, ProgressBar
- **布局**：Spacer

详见 [XML 布局指南](XML_LAYOUT_GUIDE.md)

## 🤝 贡献

想要贡献文档？

1. Fork 项目
2. 编辑或添加文档
3. 提交 Pull Request

文档使用 Markdown 格式编写。

## 📝 许可证

文档采用 MIT 许可证，与项目相同。

---

**找不到需要的信息？** 提交 [Issue](https://github.com/yourusername/nano-installer/issues) 让我们知道！
