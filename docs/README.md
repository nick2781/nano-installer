# nano-installer 文档中心

欢迎来到 nano-installer 文档中心！这里提供完整的文档导航和快速入口。

## 📚 核心文档

### 新手入门

<table>
<tr>
<td width="50%">

#### [快速开始](../README.md#-快速开始)
5 分钟上手，创建第一个安装器

**适合人群**：初次使用者

**内容**：
- 安装 nano-installer
- 创建新项目
- 构建安装器
- 测试运行

</td>
<td width="50%">

#### [项目结构详解](../PROJECT_STRUCTURE.md)
深入了解项目组织架构

**适合人群**：开发者、贡献者

**内容**：
- 完整目录结构
- 组件详细说明
- 构建流程
- 扩展指南

</td>
</tr>
</table>

### 核心文档

<table>
<tr>
<td width="50%">

#### [核心代码说明](../installer/README.md)
installer/ 目录详解

**适合人群**：开发者

**内容**：
- 各包的详细说明
- 代码结构
- 开发指南
- 性能指标

</td>
<td width="50%">

#### [重构总结](../REFACTORING_SUMMARY.md)
架构演进历史

**适合人群**：架构师、维护者

**内容**：
- 重构动机
- 实施步骤
- 技术细节
- 经验总结

</td>
</tr>
</table>

---

## 🎯 按角色导航

### 🆕 我是新用户

**推荐阅读顺序**：

1. [README.md](../README.md) - 了解项目概况
2. [快速开始](#) - 动手实践
3. [配置参考](API.md) - 自定义配置
4. [示例项目](../examples/TapTap/) - 学习最佳实践

### 💻 我是开发者

**推荐阅读顺序**：

1. [项目结构](PROJECT_STRUCTURE.md) - 理解架构
2. [核心代码](../installer/README.md) - 深入代码
3. [开发指南](DEVELOPMENT.md) - 开始贡献
4. [架构设计](ARCHITECTURE.md) - 设计理念

### 🏗️ 我是架构师

**推荐阅读顺序**：

1. [架构设计](ARCHITECTURE.md) - 详细设计
2. [项目结构](PROJECT_STRUCTURE.md) - 组织方式
3. [核心代码](../installer/README.md) - 代码实现
4. [扩展指南](PROJECT_STRUCTURE.md#-扩展指南) - 如何扩展

---

## 📖 完整文档列表

### 根目录文档

| 文档 | 说明 | 目标读者 |
|------|------|----------|
| [README.md](../README.md) | 项目主文档，快速开始 | 所有人 |
| [CHANGELOG.md](../CHANGELOG.md) | 版本更新日志 | 所有人 |

### installer/ 目录

| 文档 | 说明 | 目标读者 |
|------|------|----------|
| [installer/README.md](../installer/README.md) | 核心代码详解 | 开发者 |

### docs/ 目录

| 文档 | 说明 | 目标读者 |
|------|------|----------|
| [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) | 项目结构详解 | 开发者 |
| [ARCHITECTURE.md](ARCHITECTURE.md) | 架构设计文档 | 架构师 |
| [API.md](API.md) | 配置 API 参考 | 用户、开发者 |
| [DEVELOPMENT.md](DEVELOPMENT.md) | 开发指南 | 贡献者 |
| [UI_DESIGN.md](UI_DESIGN.md) | UI 设计指南 | UI 设计师 |
| [XML_LAYOUT_REFERENCE.md](XML_LAYOUT_REFERENCE.md) | XML 布局参考手册 | 开发者 |
| [LOCALIZATION_UPDATE.md](LOCALIZATION_UPDATE.md) | 国际化指南 | 翻译者 |
| [TEST_PLAN.md](TEST_PLAN.md) | 测试计划 | 测试人员 |

### guides/ 目录

| 文档 | 说明 | 目标读者 |
|------|------|----------|
| [guides/QUICKSTART.md](guides/QUICKSTART.md) | 5 分钟快速开始 | 新用户 |
| [guides/START_HERE.md](guides/START_HERE.md) | 从这里开始 | 所有人 |
| [guides/FINAL_STEP.md](guides/FINAL_STEP.md) | 最后步骤 | 用户 |

---

## 🔍 按主题导航

### 安装和配置

- [安装 nano-installer](../README.md#安装)
- [创建新项目](../README.md#创建第一个安装器)
- [配置文件格式](API.md#配置文件)
- [布局系统](UI_DESIGN.md)

### 构建和发布

- [构建安装器](../README.md#构建指南)
- [命令行参考](../README.md#命令行参考)
- [优化大小](../installer/README.md#优化二进制大小)
- [多架构支持](../PROJECT_STRUCTURE.md#支持多架构)

### 开发和贡献

- [环境搭建](DEVELOPMENT.md#环境要求)
- [代码结构](../installer/README.md#目录结构)
- [添加新功能](../installer/README.md#添加新功能到-lib)
- [调试技巧](../installer/README.md#调试技巧)

### 架构和设计

- [整体架构](../PROJECT_STRUCTURE.md#总体架构)
- [工作流程](../PROJECT_STRUCTURE.md#工作流程)
- [设计原则](ARCHITECTURE.md)
- [与 NSIS 对比](../README.md#与-nsis-的对比)

---

## 🎓 教程和示例

### 入门教程

1. **Hello World** - 创建最简单的安装器
   ```bash
   nano-installer init HelloWorld
   cd HelloWorld
   nano-installer build
   ```

2. **自定义 UI** - 修改布局和样式
   - 编辑 `layouts/*.xml`
   - 自定义 `assets/`
   - 查看 [UI 设计指南](UI_DESIGN.md)

3. **多语言支持** - 添加新语言
   - 创建 `locales/xx-XX.json`
   - 查看 [国际化指南](LOCALIZATION_UPDATE.md)

### 完整示例

#### [TapTap 客户端安装器](../examples/TapTap/)

完整的生产级示例，包含：
- 自定义 UI 布局
- 11 种语言支持
- 复杂的安装逻辑
- 高级配置

**学习要点**：
- 如何组织大型项目
- 如何优化安装体验
- 如何处理复杂场景

---

## 🛠️ 工具和资源

### 开发工具

- **VS Code 扩展**：
  - Rust Analyzer
  - Even Better TOML
  - XML Tools

- **调试工具**：
  - `cargo-bloat` - 分析二进制大小
  - `cargo-watch` - 自动重新编译
  - `cargo-edit` - 管理依赖

### 外部资源

- [Rust 官方文档](https://doc.rust-lang.org/)
- [Egui 文档](https://docs.rs/egui/)
- [NSIS 文档](https://nsis.sourceforge.io/Docs/) (参考)
- [7-Zip 文档](https://www.7-zip.org/sdk.html)

---

## 📝 文档贡献

### 改进文档

欢迎帮助改进文档！

1. **发现错误**：提交 Issue
2. **建议改进**：提交 Pull Request
3. **添加示例**：贡献新的示例项目
4. **翻译文档**：帮助翻译成其他语言

### 文档规范

- 使用 Markdown 格式
- 遵循现有的风格和结构
- 提供代码示例
- 添加适当的链接

---

## ❓ 常见问题

### 快速解答

**Q: 如何开始使用 nano-installer？**  
A: 查看 [快速开始](../README.md#快速开始) 指南

**Q: 如何自定义安装器 UI？**  
A: 编辑 `layouts/*.xml` 文件，参考 [UI 设计指南](UI_DESIGN.md)

**Q: 如何添加新语言？**  
A: 创建新的 `locales/xx-XX.json` 文件，参考 [国际化指南](LOCALIZATION_UPDATE.md)

**Q: 如何优化安装器大小？**  
A: 查看 [优化指南](../installer/README.md#优化二进制大小)

**Q: 如何贡献代码？**  
A: 阅读 [开发指南](DEVELOPMENT.md) 和 [架构文档](ARCHITECTURE.md)

---

## 📞 获取帮助

### 联系方式

- **Issues**: [GitHub Issues](https://github.com/your-org/nano-installer/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/nano-installer/discussions)
- **Email**: your-email@example.com

### 社区

- **Discord**: (待创建)
- **Telegram**: (待创建)
- **QQ群**: (待创建)

---

## 🗺️ 文档路线图

### 计划中的文档

- [ ] 视频教程系列
- [ ] 交互式在线文档
- [ ] API 文档（rustdoc）
- [ ] 更多示例项目
- [ ] 多语言文档（中文优先）

### 正在进行

- [x] 核心文档完善
- [ ] 代码注释补充
- [ ] 性能优化指南
- [ ] 故障排查指南

---

<div align="center">

**[⬆ 回到顶部](#nano-installer-文档中心)**

感谢阅读！希望这些文档对你有帮助。

最后更新：2025-11-05

</div>
