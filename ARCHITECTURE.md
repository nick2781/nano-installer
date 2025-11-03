# nano-installer 架构文档

## 概述

`nano-installer` 是一个类似 NSIS 的安装器生成工具，用于创建专业的 Windows 安装程序。

## 核心架构

### 1. CLI 工具：`nano-installer.exe`

唯一的命令行工具，提供以下命令：

```bash
nano-installer init <project>        # 创建新项目
nano-installer build [--release]     # 编译项目
nano-installer validate <config>     # 验证配置
nano-installer langpack <input.json> # 构建语言包
```

### 2. 项目结构

用户项目（如 `MyApp/`）：

```
MyApp/
├── installer_config.json    # 主配置文件
├── assets/                  # 图片、图标等资源
│   ├── logo.png
│   ├── logo.ico
│   └── ...
├── layouts/                 # XML 布局文件
│   ├── welcome.xml
│   ├── config.xml
│   ├── installing.xml
│   └── finish.xml
├── locales/                 # 语言文件（JSON）
│   ├── en-US.json
│   └── zh-CN.json
├── files/                   # 待安装的文件
│   └── myapp.exe
├── dist/                    # 构建输出
│   ├── MyApp_Setup.exe      # 生成的安装器
│   └── uninst.exe           # 生成的卸载器
├── payload.7z               # files/ 的压缩包（自动生成）
└── build.ps1                # 构建脚本

```

### 3. 构建流程

#### `nano-installer init ProjectName`

1. 创建项目目录结构
2. 从 `templates/` 复制基础布局和语言文件
3. 生成默认的 `installer_config.json`
4. 生成 README 和构建脚本

#### `nano-installer build`

1. 验证项目配置和资源完整性
2. 将 `files/` 目录压缩为 `payload.7z`
3. 编译安装器引擎 (`src/bin/installer.rs`)
4. **嵌入所有资源到安装器 exe 中**：
   - `installer_config.json`
   - `layouts/*.xml`
   - `assets/*`（图片、图标）
   - `locales/*.json`
   - `payload.7z`
5. 生成 `dist/ProjectName_Setup.exe`
6. 编译卸载器引擎（使用 `installer.rs` 的卸载模式）
7. **嵌入必要资源到卸载器 exe 中**
8. 生成 `dist/uninst.exe`

### 4. 安装器运行流程

生成的 `ProjectName_Setup.exe` 是一个**独立的**可执行文件：

1. **启动时**：
   - 从嵌入的资源中读取配置
   - 初始化 GUI（egui）
   - 加载语言包（从嵌入的 `.pak` 文件）
   - 解析 XML 布局文件
   
2. **安装过程**：
   - 显示安装向导（XML 驱动的 UI）
   - 提取嵌入的 `payload.7z` 到临时目录
   - 解压到用户选择的安装路径
   - 创建快捷方式、注册表项
   - **复制自身到安装目录作为 `uninst.exe`**
   
3. **命令行模式**：
   ```bash
   ProjectName_Setup.exe --silent --install-path "C:\MyApp"
   ```

### 5. 卸载器运行流程

`uninst.exe` 本质上是安装器的副本，通过参数或名称判断运行模式：

1. **启动时**：
   - 检测到运行模式为卸载
   - 从嵌入的资源中读取卸载配置
   - 显示卸载界面（可选）
   
2. **卸载过程**：
   - 删除安装的文件
   - 清理注册表项
   - 删除快捷方式
   - 可选保留用户数据

### 6. 资源嵌入机制

**问题**：如何将用户项目的资源嵌入到生成的 exe 中？

**方案 A**（推荐）：使用 `rust-embed` crate
- 在编译时将资源目录嵌入到二进制
- 运行时通过 `EmbeddedResources::get("file.xml")` 访问
- 优点：零配置，性能好
- 缺点：需要重新编译才能更新资源

**方案 B**：自定义资源打包
- 将所有资源打包为一个 `.bin` 文件
- 使用自定义 PE 资源节或直接追加到 exe 末尾
- 优点：更灵活
- 缺点：实现复杂，需要解析 PE 格式

**当前实现**：暂时使用方案 A（rust-embed），待完善。

### 7. 代码组织

```
nano-installer/
├── src/
│   ├── bin/
│   │   └── nano-installer.rs       # CLI 工具（唯一的 bin）
│   ├── installer/                  # 安装器引擎库
│   │   ├── engine.rs
│   │   ├── tasks.rs
│   │   └── ...
│   ├── uninstaller/                # 卸载器引擎库
│   │   ├── engine.rs
│   │   └── ...
│   ├── ui/                         # GUI 相关
│   │   ├── egui_app_xml.rs        # XML 驱动的 UI
│   │   ├── layout_renderer.rs
│   │   └── ...
│   ├── layout/                     # XML 解析
│   ├── i18n/                       # 国际化
│   ├── config/                     # 配置管理
│   └── lib.rs                      # 库入口
├── templates/                      # 模板文件（嵌入到 nano-installer.exe）
│   ├── layouts/
│   │   ├── welcome.xml
│   │   ├── config.xml
│   │   ├── installing.xml
│   │   └── finish.xml
│   └── locales/
│       ├── en-US.json
│       └── zh-CN.json
├── assets/
│   └── nano-installer.ico          # CLI 工具图标
└── build.rs                        # 构建脚本（仅设置图标）
```

## 待实现功能

1. ✅ CLI 工具基本框架
2. ✅ 项目初始化（init）
3. ⚠️ 资源嵌入机制（build）
4. ⚠️ 安装器从嵌入资源运行
5. ⚠️ 卸载器生成
6. ⚠️ 图标设置（从用户项目配置）

## 设计原则

1. **零依赖运行**：生成的安装器是独立的 exe，不依赖外部文件
2. **配置驱动**：所有行为由 `installer_config.json` 控制
3. **XML 布局**：UI 完全由 XML 定义，不硬编码
4. **多语言支持**：语言包独立，易于扩展
5. **简单易用**：用户只需关心项目配置，不需要了解编译过程

## 与 NSIS 的对比

| 特性 | NSIS | nano-installer |
|------|------|----------------|
| 配置方式 | NSI 脚本 | JSON 配置 |
| UI 定义 | 代码 | XML 布局 |
| 语言 | 内置 | JSON 文件 |
| 图标 | 命令行指定 | 配置文件 |
| 现代化 | ❌ | ✅ |
| DPI 感知 | ❌ | ✅ |

