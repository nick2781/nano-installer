# Session: nano-installer Phase1-4 全配置驱动重构

## 完成状态

### Phase 1: 配置驱动架构 ✅
- 1.1 WizardPage 枚举 → String page ID
- 1.2 Button action 属性 (XML 声明行为)
- 1.3 dispatch_action() 替代 handle_button_click()
- 1.4 硬编码中文移入 locale
- 1.5 消息框 XML 渲染 (已实现，但定位有问题，暂用代码回退)
- 1.6 Links → HashMap<String, String>

### Phase 2: Update Mode + Silent Mode ✅
- InstallerMode::Update / Silent
- config.wizard.update_pages[]
- 静默安装使用 TaskRunner

### Phase 3: 运行时语言切换 ✅
- switch_language() 方法
- action="switch_language:zh-CN"

### Phase 4: 可配置任务流水线 ✅
- TaskRunner 从 config.install_tasks 读取
- GUI 和静默模式共用

### 卸载器改造 ✅
- uninst stub 改为完整 egui 引擎 (和 lzma stub 相同架构)
- build 时打包全部 UI 资源 + locale 到 uninst.exe
- 安装时写入 installer_config.json 供 uninst 读取 (write_uninstall_config task)
- 通过 exe 文件名 "uninst" 自动检测为 Uninstall 模式

### Bug Fixes (测试中发现)
- payload.rs: 支持 ZIP + 7z 双格式自动检测解压
- path_validation.rs: 权限检查降为 warning 不阻止安装
- installer_runtime: 去掉自动 update 检测（避免跳过配置页）
- platform.rs: request_elevation 修复参数传递 (跳过 args[0])
- release profile: opt-level "z" → "s", 去掉 panic="abort" (避免闪退)
- cli/main.rs: stub 搜索优先 release, uninst 打包全部 assets

## 测试结果
- [x] TapTap_Setup.exe 完整安装流程 (UAC提权 → 配置页 → 安装 → 完成)
- [x] 关闭确认对话框居中显示
- [x] 展开/收起配置面板
- [x] 安装路径选择
- [x] 协议勾选 → 按钮启用/禁用
- [x] 卸载流程 (egui 自定义界面)
- [ ] 卸载后安装目录空文件夹残留 (批处理自删除可能未完全执行)

## 测试配置
- 安装路径: C:\Program Files\TapTapTest
- 注册表: HKCU\Software\TapTapTest (不影响真实TapTap)
- 快捷方式: TapTapTest
- require_admin: true (会弹UAC)

## 已知问题 / 待办
- 消息框 XML 渲染定位不正确 (egui Area 内 Taffy 布局偏移, 暂用代码回退)
- TaskRunner rollback 只有框架，未完整实现
- Debug 模式下 lzma stub 会显示控制台窗口
- release opt-level="z" 导致闪退, 当前用 "s"
- 安装包大小优化 (当前 release ~28MB, 目标 <15MB)
  - uninst.exe 和 lzma stub 是相同引擎, 各 ~6MB
  - 可考虑: 共享引擎 DLL, UPX 压缩, 条件编译裁剪 egui features
  - 可考虑: uninst 走轻量路径 (不含 egui, 用 Win32 API + 嵌入资源做自定义 UI)

## 如何恢复
```bash
cd D:\taptap-pc\nano-installer
claude --resume
```

## 如何测试
```bash
# release build
cargo build --release --bin lzma-x64-unicode --bin uninst

# 打包
cargo run --bin nano-installer -- build --project examples/TapTap

# 测试
cd examples/TapTap/dist
./TapTap_Setup.exe
```
