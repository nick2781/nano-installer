# Session: nano-installer 全配置驱动重构 + Rhai 脚本引擎

## 完成状态

### Phase 1-4: 配置驱动架构 ✅
- String page ID, Action 驱动, Locale 覆盖, Links HashMap
- Update/Silent 模式, 运行时语言切换, TaskRunner

### Phase 5: Rhai 脚本引擎 ✅
- 三层架构: config.json > TaskRunner > scripts/*.rhai
- 50+ 原子 API 函数 (文件/注册表/进程/快捷方式/UI/系统)
- extract_payload_with_progress 带平滑进度动画
- NSIS 脚本 1:1 翻译 (install.rhai + uninstall.rhai)

### 卸载器 ✅
- uninst.exe = 完整 egui 引擎 + 嵌入资源
- 自删除: 批处理 cd %TEMP% + CREATE_NO_WINDOW + del/rmdir

### 测试结果
- [x] 安装: UAC→配置页→安装→完成 (含真实 TapTap payload)
- [x] 桌面快捷方式创建
- [x] 开始菜单快捷方式 + 卸载快捷方式
- [x] 注册表写入 (全部 8 个值 + NoModify/NoRepair)
- [x] channel.conf 写入
- [x] 卸载: 进程杀 + 快捷方式删 + 注册表清 + 文件删 + 自删除
- [x] 进度条平滑动画
- [ ] 窗口置顶 (加了 with_active, 待验证)

## 如何测试
```bash
cargo build --release --bin lzma-x64-unicode --bin uninst
cargo run --bin nano-installer -- build --project examples/TapTap
cd examples/TapTap/dist && ./TapTap_Setup.exe
```

## 待完成
- 安装包大小优化
- 消息框 XML 渲染修复
- Web 编辑器
