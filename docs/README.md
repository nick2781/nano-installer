<h1 align="center"><img src="../assets/nano-technology.png" width="48" height="48" alt="nano-installer 工具图标"> nano-installer native 文档</h1>

<p align="center"><a href="../README.md">English README</a> | <a href="../README.zh-CN.md">中文 README</a></p>

本分支的 installer runtime 是原生 Win32 实现，最低支持 Windows 7 SP1 x64。可视化构建
工具是隔离的 Windows 10+ eframe crate，不进入 setup。旧 eframe runtime 保留在 `main`。

## 使用顺序

1. [快速开始](QUICK_START.md)
2. [项目结构](PROJECT_STRUCTURE.md)
3. [配置参考](CONFIG_REFERENCE.md)
4. [XML 布局指南](XML_LAYOUT_GUIDE.md)
5. [本地化](LOCALIZATION.md)
6. [构建与发布](BUILD_AND_RELEASE.md)
7. [GUI 构建工具](GUI.md)
8. [测试计划](TEST_PLAN.md)
9. [当前生产状态](PRODUCTION_STATUS.md)

## 设计与兼容性

- [Native 架构](NATIVE_ARCHITECTURE.md)
- [Windows 兼容性](WINDOWS_COMPATIBILITY.md)

当前能够完成项目收集、payload 格式识别、独立 backend、CLI/GUI 构建、单 EXE 打包和
原生页面渲染、页面流转与进度显示，以及安装/卸载/升级/回滚流程。项目 Rhai 脚本执行、
代码签名和 Win7 SP1 虚拟机验收尚未完成，生成的 setup 仍只能在隔离测试环境验证，
不能用于生产发布。
