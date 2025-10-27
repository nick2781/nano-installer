# 文件清单

本文档列出项目中所有重要文件及其说明。

## 📋 文档文件

| 文件                | 说明                             |
| ------------------- | -------------------------------- |
| `README.md`         | 项目主文档，介绍、特性、使用方法 |
| `SUMMARY.md`        | **项目总结**，快速了解项目全貌   |
| `PROJECT_STATUS.md` | 详细的开发状态和完成情况         |
| `QUICKSTART.md`     | 5 分钟快速开始指南               |
| `TODO.md`           | 待办事项和未来计划               |
| `CHANGELOG.md`      | 版本更新记录                     |
| `FILES.md`          | 本文件，文件清单                 |

## 📚 技术文档

| 文件                             | 说明                         |
| -------------------------------- | ---------------------------- |
| `docs/DEVELOPMENT.md`            | 开发指南，环境配置、开发流程 |
| `docs/API.md`                    | API 文档，各模块使用方法     |
| `.spec/tech-spec.md`             | 技术规格说明                 |
| `.spec/language-package-spec.md` | 语言包规范                   |
| `.spec/uninstall-spec.md`        | 卸载程序规范                 |

## ⚙️ 配置文件

| 文件                  | 说明                  |
| --------------------- | --------------------- |
| `Cargo.toml`          | Rust 项目配置，依赖项 |
| `build.rs`            | 构建脚本，编译时处理  |
| `config.example.json` | 示例配置文件          |
| `.gitignore`          | Git 忽略规则          |
| `.cursorignore`       | Cursor 忽略规则       |

## 🔨 构建脚本

| 文件                 | 说明                 |
| -------------------- | -------------------- |
| `scripts/build.sh`   | Linux/macOS 构建脚本 |
| `scripts/build.ps1`  | Windows 构建脚本     |
| `scripts/package.sh` | Payload 打包脚本     |
| `scripts/sign.sh`    | 代码签名脚本         |

## 🌍 语言文件

| 文件                 | 语言     |
| -------------------- | -------- |
| `locales/en-US.json` | 英语     |
| `locales/zh-CN.json` | 简体中文 |
| `locales/zh-TW.json` | 繁体中文 |
| `locales/ja.json`    | 日语     |
| `locales/vi.json`    | 越南语   |

## 💻 源代码

### 可执行文件入口

| 文件                          | 说明           |
| ----------------------------- | -------------- |
| `src/bin/installer.rs`        | 安装器主程序   |
| `src/bin/uninstaller.rs`      | 卸载器主程序   |
| `src/bin/langpack_builder.rs` | 语言包构建工具 |

### 库入口

| 文件         | 说明                     |
| ------------ | ------------------------ |
| `src/lib.rs` | 库的主入口，导出所有模块 |

### 通用模块 (`src/common/`)

| 文件          | 说明                             |
| ------------- | -------------------------------- |
| `mod.rs`      | 模块入口                         |
| `error.rs`    | 错误类型定义                     |
| `result.rs`   | Result 类型别名                  |
| `config.rs`   | 配置结构和默认值                 |
| `platform.rs` | 平台相关功能（语言检测、权限等） |
| `cli.rs`      | 命令行参数解析                   |

### 多语言模块 (`src/i18n/`)

| 文件          | 说明                  |
| ------------- | --------------------- |
| `mod.rs`      | 多语言 API            |
| `langpack.rs` | .pak 格式定义和序列化 |
| `bundle.rs`   | 语言包集合管理        |
| `loader.rs`   | 语言加载器，嵌入资源  |

### 资源管理 (`src/resources/`)

| 文件          | 说明                     |
| ------------- | ------------------------ |
| `mod.rs`      | 模块入口                 |
| `payload.rs`  | Payload 提取和 7z 解压   |
| `manifest.rs` | 安装清单，记录已安装内容 |

### 日志系统 (`src/logger/`)

| 文件     | 说明                 |
| -------- | -------------------- |
| `mod.rs` | 日志初始化和步骤追踪 |

### 安装逻辑 (`src/installer/`)

| 文件                   | 说明                                   |
| ---------------------- | -------------------------------------- |
| `mod.rs`               | 模块入口                               |
| `engine.rs`            | 安装引擎，协调任务执行                 |
| `state.rs`             | 安装状态管理                           |
| `tasks.rs`             | 安装任务定义（提取、快捷方式、注册表） |
| `windows/mod.rs`       | Windows 特定功能入口                   |
| `windows/registry.rs`  | 注册表操作                             |
| `windows/shortcuts.rs` | 快捷方式创建                           |
| `windows/elevation.rs` | 权限提升                               |

### 卸载逻辑 (`src/uninstaller/`)

| 文件             | 说明             |
| ---------------- | ---------------- |
| `mod.rs`         | 模块入口         |
| `engine.rs`      | 卸载引擎         |
| `tasks.rs`       | 卸载任务（预留） |
| `windows/mod.rs` | Windows 特定功能 |

### UI 模块 (`src/ui/`)

⚠️ **注意**：UI 模块目前只有骨架，等待设计稿后实现。

| 文件                         | 说明                     |
| ---------------------------- | ------------------------ |
| `mod.rs`                     | UI 模块入口              |
| `wizard.rs`                  | 向导容器，页面导航       |
| `styles/mod.rs`              | 样式定义（颜色、尺寸）   |
| `pages/mod.rs`               | 页面模块入口             |
| `pages/language.rs`          | 语言选择页（待实现）     |
| `pages/welcome.rs`           | 欢迎页（待实现）         |
| `pages/license.rs`           | 许可协议页（待实现）     |
| `pages/install_path.rs`      | 安装路径选择页（待实现） |
| `pages/installing.rs`        | 安装进度页（待实现）     |
| `pages/finish.rs`            | 完成页（待实现）         |
| `components/mod.rs`          | 组件模块入口             |
| `components/button.rs`       | 按钮组件（待实现）       |
| `components/progress_bar.rs` | 进度条组件（待实现）     |
| `components/text_input.rs`   | 文本输入组件（待实现）   |
| `components/checkbox.rs`     | 复选框组件（待实现）     |

## 🧪 测试

| 文件                        | 说明                     |
| --------------------------- | ------------------------ |
| `tests/integration_test.rs` | 集成测试，测试多语言系统 |

## 📊 统计

- **总文件数**：70+ 个
- **Rust 源文件**：47 个
- **文档文件**：13 个
- **配置文件**：4 个
- **脚本文件**：4 个
- **语言文件**：5 个

## 🎯 关键文件（快速导航）

如果您想：

### 了解项目
👉 先读 `SUMMARY.md`，然后 `README.md`

### 开始开发
👉 读 `QUICKSTART.md`，然后 `docs/DEVELOPMENT.md`

### 修改配置
👉 编辑 `src/common/config.rs`

### 添加翻译
👉 在 `locales/` 添加 JSON 文件

### 实现 UI
👉 编辑 `src/ui/pages/*.rs` 和 `src/ui/components/*.rs`

### 添加安装步骤
👉 在 `src/installer/tasks.rs` 添加新任务

### 查看 API
👉 读 `docs/API.md`

### 查看规范
👉 读 `.spec/` 目录下的文件

