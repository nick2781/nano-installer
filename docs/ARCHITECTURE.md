# nano-installer 架构设计

## 概述

nano-installer 是一个现代化的 Windows 安装器框架，使用 Rust 构建，采用 CLI + runtime stub + 资源分段打包 的架构。

## 总体架构

```
nano-installer.exe        # CLI 工具
  ├─ 读取 installer_config.json
  ├─ 选择 stub 文件
  ├─ 打包资源
  └─ 生成 YourApp_Setup.exe

Stubs/
  ├─ lzma-x64-unicode.exe # 安装器 stub
  └─ uninst.exe           # 卸载器 stub
```

## 项目结构

```
nano-installer/
├── Cargo.toml                          # Workspace 配置
│
├── assets/                             # 共享资源
│   ├── nano-installer.ico              # CLI 工具图标
│   ├── stub.ico                        # 安装器 stub 默认图标
│   └── uninstall.ico                   # 卸载器 stub 默认图标
│
├── nano-installer/                     # 子项目 1：CLI 工具
│   ├── Cargo.toml
│   ├── build.rs                        # 设置 nano-installer.ico
│   └── src/
│       ├── lib.rs                      # 共享库（导出所有模块）
│       ├── main.rs                     # CLI 入口（纯 CLI）
│       ├── common/                     # 通用工具
│       ├── config/                     # 配置管理
│       ├── installer/                  # 安装引擎
│       ├── installer_runtime/          # 安装器运行时编排
│       ├── uninstaller/                # 卸载引擎
│       ├── ui/                         # Egui UI
│       ├── layout/                     # XML 布局解析
│       ├── i18n/                       # 国际化
│       ├── resources/                  # 资源管理
│       └── logger/                     # 日志系统
│
├── lzma-x64-unicode/                   # 子项目 2：安装器 stub
│   ├── Cargo.toml
│   │   [dependencies]
│   │   nano-installer = { path = "../nano-installer" }
│   ├── build.rs                        # 设置 stub.ico
│   └── src/
│       └── main.rs                     # 调用 installer_runtime
│
└── uninst/                             # 子项目 3：卸载器 stub
    ├── Cargo.toml
    ├── build.rs                        # 设置 uninstall.ico
    └── src/
        └── main.rs                     # 纯 Win32 卸载逻辑
```

## 编译输出

```
target/release/
├── nano-installer.exe                  # CLI 工具
├── lzma-x64-unicode.exe                # 安装器 stub
└── uninst.exe                          # 卸载器 stub
```

## Harness 边界

为了避免所有 UI 问题都依赖真实窗口和手工截图，工程现在按资源源头拆分：

- `RuntimeUiResourceProvider`
  - 运行时从嵌入 bundle 读取布局和语言包
- `FilesystemUiResourceProvider`
  - harness 从项目目录直接读取 fixture

这样 `InstallerApp`、`LayoutRenderer`、XML DSL 和语言切换逻辑可以在库内被直接驱动，真实 EXE smoke 只负责最后的链路确认。

## 工作流程

### 1. 编译时（nano-installer build）

```
用户运行：nano-installer build
                 ↓
┌─────────────────────────────────────┐
│ 读取配置                             │
│ installer_config.json                │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│ 创建临时目录                         │
│ .build/                              │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│ 构建卸载器                           │
│ ① 复制 uninst.exe → .build/          │
│ ② 打包资源（config, layouts, assets) │
│ ③ 替换图标（可选）                   │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│ 构建安装器                           │
│ ① 复制 lzma-x64-unicode.exe          │
│ ② 打包所有资源                       │
│    - config, layouts, assets         │
│    - locales (编译为 .pak)           │
│    - payload (app.7z)                │
│    - uninst.exe (嵌入)               │
│ ③ 替换图标                           │
│ ④ 输出到 dist/YourApp_Setup.exe      │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│ 清理临时目录                         │
│ rm -rf .build/                       │
└─────────────────────────────────────┘
```

### 2. 安装时（用户运行 YourApp_Setup.exe）

```
YourApp_Setup.exe 启动
（lzma-x64-unicode stub）
                 ↓
┌─────────────────────────────────────┐
│ 初始化                               │
│ ① 初始化日志系统                     │
│ ② 从 PE 末尾提取资源包               │
│ ③ 加载配置和多语言                   │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│ 权限检查                             │
│ ① 检查管理员权限（如需要）           │
│ ② 初始化互斥锁（防止多实例）         │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│ 启动 Egui GUI                        │
│ ① 语言选择页                         │
│ ② 许可协议页                         │
│ ③ 安装路径页                         │
│ ④ 安装进度页                         │
│ ⑤ 完成页                             │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│ 执行安装                             │
│ ① 解压 payload (app.7z)              │
│ ② 复制 uninst.exe 到安装目录         │
│ ③ 创建快捷方式                       │
│ ④ 写注册表                           │
└─────────────────────────────────────┘
```

### 3. 卸载时（用户运行 uninst.exe）

```
C:\Program Files\YourApp\uninst.exe
（uninst stub - 纯 Win32 API）
                 ↓
┌─────────────────────────────────────┐
│ 读取卸载信息                         │
│ ① 从注册表读取                       │
│ ② 或从 uninstall.json 读取           │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│ 显示确认对话框                       │
│ Win32 MessageBox                     │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│ 执行卸载                             │
│ ① 删除文件                           │
│ ② 删除快捷方式                       │
│ ③ 删除注册表项                       │
│ ④ 创建自删除批处理脚本               │
└─────────────────────────────────────┘
```

## 组件详解

### 1. nano-installer.exe（CLI 工具）

**职责**：
- 项目初始化（`init` 命令）
- 编译安装器（`build` 命令）
- 验证配置（`validate` 命令）
- 编译语言包（`langpack` 命令）

**依赖**：
- `clap` - CLI 参数解析
- `serde_json` - 配置解析
- `anyhow` - 错误处理
- 内部库：`config`, `resources::bundle`, `i18n::langpack`

**特点**：
- ❌ 不包含任何 GUI 代码
- ❌ 不包含安装/卸载运行时逻辑
- ✅ 纯粹的构建工具

### 2. lzma-x64-unicode.exe（安装器 stub）

**职责**：
- 作为安装器的"壳"程序
- 提取和管理嵌入资源
- 启动 Egui GUI
- 执行安装逻辑

**依赖**：
- `nano-installer` 库（完整依赖）
- `eframe`, `egui` - GUI 框架
- `windows` - Windows API
- 所有共享模块

**特点**：
- ✅ 预编译的可执行文件
- ✅ 在构建时被复制和修改
- ✅ 包含完整的安装器功能

**大小优化目标**：
- 当前：~4.16 MB
- 目标：< 1 MB（移除不必要依赖）

### 3. uninst.exe（卸载器 stub）

**职责**：
- 纯粹的卸载功能
- 使用 Win32 API（无 GUI 框架）
- 最小化体积

**依赖**：
- `windows` - Win32 API
- `serde_json` - 读取配置
- `anyhow` - 错误处理

**特点**：
- ✅ 极小体积（~200KB）
- ✅ 无 Egui 依赖
- ✅ 纯 Win32 MessageBox
- ❌ 几乎不依赖 `nano-installer` 库

## 资源管理

### 资源打包格式

```
资源包格式（追加到 PE 文件末尾）：
┌────────────────────────────────────┐
│ Magic: "NAIR" (4 bytes)            │
├────────────────────────────────────┤
│ Version: u16                       │
├────────────────────────────────────┤
│ Item Count: u32                    │
├────────────────────────────────────┤
│ Resource Item 1                    │
│   - Type: u8 (Config/Layout/Asset)  │
│   - Name Length: u16               │
│   - Name: String                   │
│   - Data Length: u32               │
│   - Data: Vec<u8>                  │
├────────────────────────────────────┤
│ Resource Item 2                    │
│   ...                              │
├────────────────────────────────────┤
│ CRC32: u32                         │
└────────────────────────────────────┘
```

### 资源类型

```rust
pub enum ResourceType {
    Config = 1,        // installer_config.json
    Layout = 2,        // layouts/*.xml
    Asset = 3,         // assets/*
    Locale = 4,        // locales/*.pak (编译后)
    Payload = 5,       // app.7z
    Uninstaller = 6,   // uninst.exe (仅在安装器中)
}
```

## 配置文件

### installer_config.json

```json
{
  "project": {
    "name": "YourApp",
    "version": "1.0.0",
    "publisher": "Your Company"
  },
  "output": {
    "installer_name": "YourApp_Setup.exe",
    "uninstaller_name": "uninst.exe"
  },
  "resources": {
    "installer_icon": "assets/icon.ico",
    "uninstaller_icon": "assets/uninstall.ico",
    "payload_file": "payload/app.7z"
  },
  "install": {
    "default_dir": "C:\\Program Files\\YourApp",
    "require_admin": true,
    "mutex_name": "YourAppInstaller"
  }
}
```

## 图标配置

### 编译时图标（stub 默认图标）

| 文件 | 图标路径 | 说明 |
|------|----------|------|
| `nano-installer.exe` | `assets/nano-installer.ico` | CLI 工具图标 |
| `lzma-x64-unicode.exe` | `assets/stub.ico` | 安装器 stub 默认图标 |
| `uninst.exe` | `assets/uninstall.ico` | 卸载器 stub 默认图标 |

### 构建时图标替换（项目自定义图标）

在 `nano-installer build` 时：
- 读取项目配置中的 `installer_icon`
- 使用 Win32 API 替换 PE 资源中的图标
- 最终用户看到的是项目自定义图标

## 多语言支持

### 语言包编译

```
JSON 源文件 → .pak 二进制文件

en-US.json → en-US.pak
zh-CN.json → zh-CN.pak
...

.pak 格式：
┌────────────────────────────────┐
│ Magic: "LNGP" (4 bytes)        │
├────────────────────────────────┤
│ Version: u16                   │
├────────────────────────────────┤
│ Locale Length: u16             │
│ Locale: String                 │
├────────────────────────────────┤
│ CRC32: u32                     │
├────────────────────────────────┤
│ Entry Count: u32               │
├────────────────────────────────┤
│ Entries:                       │
│   - Key Length: u16            │
│   - Key: String                │
│   - Value Length: u32          │
│   - Value: String              │
└────────────────────────────────┘
```

### 运行时加载

```rust
// 运行时自动选择语言
let locale = detect_system_locale(); // "zh-CN"
let bundle = RuntimeResources::get_locale("zh-CN")?;
let text = bundle.get("welcome.title")?;
```

## XML 布局系统

### 设计边界

`nano-installer` 的 UI 有三层职责：

1. XML DSL 负责声明界面结构和盒模型语义。
2. Taffy 负责根据这些语义计算布局结果。
3. Renderer 只负责按计算后的盒子绘制，不允许再内置“某个按钮应该怎么排”的隐藏规则。

这意味着：

- `width` / `min-width` / `max-width` / `padding` / `margin` / `wrap` 这类约束必须由 XML 表达。
- `Button` 内部“文字 + 图标”的排列也必须由 XML 内容模型表达，而不是靠 `dest='68,2,80,15'` 这类绘制坐标硬编码。
- NSIS 风格属性仍然保留兼容，但只作为迁移层，不再作为新增布局能力的首选表达方式。

### DSL 方向

布局文件继续使用 XML 作为 DSL，但命名会向“人能读懂的盒模型”收敛，而不是直接暴露 CSS 术语或渲染器实现细节。

推荐的新内容模型：

```xml
<Button min-width="80" max-width="164">
  <Content
    layout="horizontal"
    horizontal-align="right"
    vertical-align="center"
    item-spacing="4">
    <Text value="@show_more" wrap="true" />
    <Icon src="assets/arrow-down.png" width="12" height="12" />
  </Content>
</Button>
```

语义约定：

- `Content` 表示控件内部的内容容器。
- `layout="horizontal|vertical"` 表示子项的排列方向。
- `horizontal-align` / `vertical-align` 分开定义，不使用复合值。
- `item-spacing` 表示子项之间的距离。
- `wrap` 表示文本是否允许自动换行。

内部实现上，这些属性会被规范化后映射到 Taffy 的 flex 语义；但这种映射属于引擎细节，不暴露给布局作者。

### 布局文件示例

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout id="welcome">
  <VBox padding="20" spacing="10">
    <Label text_i18n="welcome.title" font_size="24" color="#333333" />
    <Label text_i18n="welcome.subtitle" wrap="true" max_lines="3" />
    
    <HBox spacing="10" align="center">
      <Button id="back" text_i18n="button.back" min_width="100" />
      <Spacer flex="1" />
      <Button id="next" text_i18n="button.next" min_width="100" />
    </HBox>
  </VBox>
</Layout>
```

### 布局属性

- `text_i18n` - 国际化文本键
- `wrap` - 文本换行
- `max_lines` - 最大行数
- `min_width` / `max_width` - 宽度限制
- `flex` - 弹性布局
- `align` - 对齐方式（`start`, `center`, `end`）
- `color` - 文本颜色
- `font_size` - 字体大小

## 未来扩展

### 计划支持

1. **多种压缩算法**
   - `lzma-x64-unicode.exe` (当前)
   - `zlib-x64-unicode.exe` (未来)
   - `bzip2-x64-unicode.exe` (未来)

2. **32 位支持**
   - `lzma-x86-unicode.exe`
   - `uninst-x86.exe`

3. **macOS 支持**
   - DMG 打包
   - PKG 安装器

4. **Linux 支持**
   - DEB/RPM 打包
   - AppImage

## 参考

- [Rust 官方文档](https://doc.rust-lang.org/)
- [egui 文档](https://docs.rs/egui/)
- [Egui 文档](https://docs.rs/egui/)
- [Windows Installer](https://learn.microsoft.com/en-us/windows/win32/msi/windows-installer-portal)

