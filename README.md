# nano-installer

> 一个现代化的 Windows 安装器生成工具，类似 NSIS，但更简单、更强大

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## 📖 简介

**nano-installer** 是一个用 Rust 编写的安装器制作工具。它可以让你通过简单的配置文件和 XML 布局，快速创建专业的 Windows 安装程序。

### ✨ 主要特点

- 🎯 **配置驱动** - 使用 JSON 配置文件，无需编程
- 🎨 **自定义界面** - 基于 XML 的灵活布局系统
- 🌍 **多语言支持** - 内置 11 种语言，轻松扩展
- 📦 **7z 压缩** - 高效的文件打包
- 🔧 **Windows 集成** - 自动处理注册表、快捷方式、开始菜单
- 💻 **DPI 自适应** - 完美支持高分辨率屏幕

## 🚀 快速开始

### 1. 安装 nano-installer

```bash
git clone https://github.com/yourusername/nano-installer.git
cd nano-installer
cargo build --release
```

### 2. 查看示例项目

```bash
cd examples/TapTap
```

项目结构：
```
TapTap/
├── installer_config.json    # 配置文件
├── assets/                  # 图片资源
│   ├── logo.png            # 应用图标
│   ├── logo@2x.png         # 高 DPI 版本
│   └── ...
├── layouts/                 # XML 布局文件
│   ├── welcome.xml         # 欢迎页面
│   ├── config.xml          # 配置页面
│   └── ...
├── locales/                 # 语言文件
│   ├── zh-CN.json
│   ├── en-US.json
│   └── ...
├── files/                   # 要安装的文件
│   └── (你的应用程序文件)
└── dist/                    # 构建输出
    └── TapTap_Setup.exe     # 生成的安装程序
```

### 3. 配置你的项目

编辑 `installer_config.json`：

```json
{
  "project": {
    "name": "你的应用名称",
    "version": "1.0.0",
    "publisher": "你的公司",
    "website": "https://example.com"
  },
  "install": {
    "exe_name": "YourApp.exe",
    "default_path": "{pf64}\\YourCompany\\YourApp",
    "required_space_mb": 100
  }
}
```

### 4. 构建安装程序

```bash
# Windows
.\build.ps1

# Linux/Mac
./build.sh
```

生成的安装程序位于 `dist/` 目录。

## 📚 文档

### 用户文档

- [完整配置参考](docs/CONFIG_REFERENCE.md) - 所有配置项的详细说明
- [XML 布局指南](docs/XML_LAYOUT_GUIDE.md) - 如何自定义安装界面
- [多语言支持](docs/LOCALIZATION.md) - 如何添加和自定义语言
- [示例项目](examples/TapTap/README.md) - TapTap 示例项目说明

### 规范文档

- [XML 布局规范](docs/XML_SCHEMA.md) - XML 文件的完整 Schema 定义
- [JSON 配置规范](docs/JSON_SCHEMA.md) - JSON 配置的完整 Schema 定义

### 开发者文档

- [开发指南](docs/DEVELOPMENT.md) - 如何参与 nano-installer 开发

## 🎨 XML 布局系统

nano-installer 使用 XML 来定义安装界面，非常直观：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="Welcome" version="1.0.0">
  <Page>
    <VBox>
      <!-- 显示 Logo -->
      <Image icon="assets/logo.png" width="200" height="60" />
      <Spacer height="20" />
      
      <!-- 标题 -->
      <Label text="欢迎使用 {product_name}" font_size="24" />
      <Label text="版本 {version}" font_size="14" />
      <Spacer height="30" />
      
      <!-- 按钮 -->
      <HBox>
        <Button id="next" text="下一步" width="120" height="40" />
        <Spacer width="10" />
        <Button id="cancel" text="取消" width="120" height="40" />
      </HBox>
    </VBox>
  </Page>
</Layout>
```

支持的元素：
- `<VBox>` / `<HBox>` - 垂直/水平布局
- `<Label>` - 文本标签
- `<Button>` - 按钮
- `<Image>` - 图片
- `<Checkbox>` - 复选框
- `<TextInput>` - 文本输入框
- `<ProgressBar>` - 进度条
- `<Spacer>` - 间距

详见 [XML 布局指南](docs/XML_LAYOUT_GUIDE.md)

## 🌍 多语言支持

在 `locales/` 目录下添加 JSON 文件：

```json
{
  "language_name": "简体中文",
  "strings": {
    "welcome_title": "欢迎使用",
    "install_button": "安装",
    "cancel_button": "取消"
  }
}
```

在 XML 中使用 `{key}` 引用：

```xml
<Label text="{welcome_title}" />
```

## 🔧 配置系统

配置文件 `installer_config.json` 包含以下部分：

### 基本信息
```json
{
  "project": {
    "name": "应用名称",
    "version": "1.0.0",
    "publisher": "发布者",
    "website": "https://example.com"
  }
}
```

### 安装设置
```json
{
  "install": {
    "exe_name": "app.exe",           // 主程序名
    "default_path": "{pf64}\\App",   // 默认安装路径
    "required_space_mb": 100,        // 所需空间（MB）
    "require_admin": true            // 是否需要管理员权限
  }
}
```

### 快捷方式
```json
{
  "shortcuts": {
    "desktop": true,                 // 桌面快捷方式
    "start_menu": true,              // 开始菜单
    "quick_launch": false            // 快速启动栏
  }
}
```

### 自启动
```json
{
  "autostart": {
    "enabled": true,                 // 是否开机自启
    "args": "--background"           // 启动参数
  }
}
```

详见 [完整配置参考](docs/CONFIG_REFERENCE.md)

## 📦 打包流程

1. **准备文件** - 将应用程序文件放入 `files/` 目录
2. **配置项目** - 编辑 `installer_config.json`
3. **自定义界面** - 修改 `layouts/*.xml` 文件（可选）
4. **添加资源** - 放置图标、背景图到 `assets/` 目录
5. **构建** - 运行 `build.ps1` 或 `build.sh`
6. **测试** - 运行生成的安装程序

## 🎯 系统要求

### 运行安装程序
- Windows 7 SP1 或更高版本
- 无需额外依赖

### 构建 nano-installer
- Rust 1.75 或更高版本
- Windows 10+ (用于开发)

## 📝 示例项目

### TapTap 游戏平台安装器

完整示例位于 `examples/TapTap/`，展示了：

- ✅ 多页面安装向导
- ✅ 自定义品牌界面
- ✅ 11 种语言支持
- ✅ 高 DPI 支持（1x 和 2x 资源）
- ✅ 注册表集成
- ✅ 快捷方式创建
- ✅ 开机自启动

查看 [TapTap 示例说明](examples/TapTap/README.md)

## 🤝 参与贡献

欢迎贡献代码、报告问题或提出建议！

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

- [egui](https://github.com/emilk/egui) - 优秀的即时模式 GUI 框架
- [7-Zip](https://www.7-zip.org/) - 高效的压缩工具
- Rust 社区的所有贡献者

---

**开始使用**：查看 [examples/TapTap](examples/TapTap/) 学习如何创建你的第一个安装程序！
