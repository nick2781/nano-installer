# 实现步骤指南

## 当前状态

✅ **架构完成**：所有核心模块和 UI 框架已就绪  
⏳ **等待实现**：GPUI 实际渲染代码

## 下一步：实现 GPUI 渲染

### 步骤 1：验证依赖

```bash
cd /Users/nick/work/nano-installer
cargo check
```

这会下载所有依赖（包括 GPUI 和 gpui-component），第一次可能需要较长时间。

### 步骤 2：参考 gpui-component 示例

查看 gpui-component 的实际 API：
```bash
# 克隆 gpui-component 查看示例
git clone https://github.com/longbridge/gpui-component /tmp/gpui-component
cd /tmp/gpui-component
cargo run --example button  # 查看按钮示例
cargo run --example input   # 查看输入框示例
```

### 步骤 3：实现配置页渲染

在 `src/ui/app.rs` 中，参考 `src/ui/gpui_impl.rs` 的示例代码，实现实际的 Render trait。

关键代码结构：
```rust
impl Render for InstallerApp {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // 根据 gpui-component 的实际 API 实现
    }
}
```

### 步骤 4：更新安装器入口

修改 `src/bin/installer.rs`，添加 GUI 模式：

```rust
if args.silent {
    // 静默安装（已实现）
    run_silent_install(&install_path).await
} else {
    // GUI 安装（新增）
    run_gui_install(config).await
}
```

### 步骤 5：集成安装逻辑

在 GUI 中调用安装引擎：

```rust
async fn run_gui_install(config: InstallerConfig) -> Result<()> {
    use gpui::*;
    
    App::new().run(|cx| {
        let app = cx.new_view(|cx| InstallerApp::new(config));
        
        cx.open_window(
            WindowOptions {
                bounds: Bounds {
                    size: Size {
                        width: px(574.0),
                        height: px(358.0),
                    },
                    ..Default::default()
                },
                titlebar: None,
                resizable: false,
                ..Default::default()
            },
            |cx| app,
        );
    });
    
    Ok(())
}
```

### 步骤 6：测试

#### Windows 10+ 测试
```bash
cargo build --release --bin installer
./target/release/installer.exe
```

#### Windows 7 测试（关键）
在 Windows 7 SP1 64位虚拟机或实体机上：
```bash
installer.exe
```

如果出现错误（黑屏、崩溃、无法启动），立即切换到 egui 方案。

## 如果需要切换到 egui

### 步骤 1：更新 Cargo.toml

```toml
[dependencies]
# 替换 GPUI
# gpui = { git = "https://github.com/zed-industries/zed", rev = "main" }
# gpui-component = { git = "https://github.com/longbridge/gpui-component", branch = "main" }

# 使用 egui
eframe = "0.24"
egui = "0.24"
```

### 步骤 2：重新实现 UI（1-2天）

egui 的实现要简单得多：

```rust
impl eframe::App for InstallerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_page() {
                WizardPage::Config => self.render_config_page(ui),
                WizardPage::Installing => self.render_installing_page(ui),
                WizardPage::Finish => self.render_finish_page(ui),
            }
        });
    }
}
```

egui 的好处：
- ✅ 100% Windows 7 兼容
- ✅ API 简单，文档齐全
- ✅ 开发速度快
- ✅ 稳定可靠

## 完整的开发流程

### 第 1 天：GPUI 实现
- [ ] 安装依赖
- [ ] 查看 gpui-component 示例
- [ ] 实现配置页
- [ ] 实现进度页
- [ ] 实现完成页

### 第 2 天：集成和测试
- [ ] 集成安装逻辑
- [ ] Windows 10+ 测试
- [ ] 修复 bug
- [ ] 优化样式

### 第 3 天：Windows 7 测试
- [ ] 在 Windows 7 上测试
- [ ] 如果成功：完成！
- [ ] 如果失败：切换到 egui

### 第 4-5 天：egui 实现（如果需要）
- [ ] 更新依赖
- [ ] 重新实现 UI
- [ ] 测试
- [ ] 完成

## 需要的资源

### 代码参考
- `docs/GPUI_COMPONENTS_GUIDE.md` - 组件使用示例
- `src/ui/gpui_impl.rs` - 实现框架
- `docs/UI_DESIGN.md` - 设计规格

### 测试环境
- Windows 10+ 开发机
- Windows 7 SP1 64位测试机（虚拟机或实体机）

### 应用信息
- 应用名称（替换代码中的 "MyApp"）
- 应用版本
- 发布者名称
- 测试用的 app.7z 文件

## 关键文件位置

| 文件                   | 说明                              |
| ---------------------- | --------------------------------- |
| `src/ui/app.rs`        | 主应用窗口，需要实现 Render trait |
| `src/ui/gpui_impl.rs`  | 实现示例代码                      |
| `src/bin/installer.rs` | 安装器入口，需要添加 GUI 模式     |
| `src/ui/wizard.rs`     | 页面导航逻辑（已完成）            |
| `src/ui/pages/*.rs`    | 各页面的数据模型（已完成）        |
| `src/ui/styles/mod.rs` | 样式定义（已完成）                |

## 预期时间

- **GPUI 路线**：3-5 天（包括测试）
- **如需切换 egui**：额外 1-2 天

## 成功标准

✅ **最低要求**：
- 能在 Windows 10+ 上运行
- GUI 显示正常
- 能完成安装流程
- 静默安装正常工作

🎯 **理想状态**：
- Windows 7 SP1 兼容
- 界面美观，符合设计
- 流畅无卡顿
- 错误处理完善

## 寻求帮助

如果遇到问题：
1. 查看 GPUI 官方文档和示例
2. 查看 gpui-component 的示例代码
3. 在 GPUI Discord 或 GitHub Issues 寻求帮助
4. 考虑切换到 egui（更简单、更稳定）

## 备注

当前项目已完成：
- ✅ 完整的架构设计
- ✅ 所有后端逻辑
- ✅ 多语言系统
- ✅ 日志系统
- ✅ 静默安装
- ✅ UI 框架和数据模型
- ✅ 样式定义
- ✅ 图片资源

**只差最后一步：GPUI 的实际渲染代码！**

加油，你已经完成了 95% 的工作！🚀

