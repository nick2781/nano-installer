# GPUI Components 使用指南

## 介绍

使用 [gpui-component](https://github.com/longbridge/gpui-component) 来实现安装器 UI，避免手动绘制复杂组件。

## gpui-component 提供的组件

### 基础组件
- **Button** - 按钮（支持多种样式）
- **Input** - 文本输入框
- **Checkbox** - 复选框
- **Radio** - 单选框
- **Switch** - 开关
- **Label** - 文本标签

### 布局组件
- **List** - 列表
- **Stack** - 堆栈布局
- **Scroll** - 滚动容器
- **Dropdown** - 下拉菜单

### 进度组件
- **ProgressBar** - 进度条
- **Indicator** - 加载指示器

### 其他组件
- **Modal** - 模态对话框
- **Tooltip** - 提示框
- **Icon** - 图标

## 安装器组件映射

基于 NSIS 设计，我们需要：

| NSIS 元素  | gpui-component   | 说明                   |
| ---------- | ---------------- | ---------------------- |
| 主按钮     | `Button`         | 使用 primary 样式      |
| 关闭按钮   | `Button` + Icon  | 自定义样式             |
| 复选框     | `Checkbox`       | 同意协议、创建快捷方式 |
| 路径输入框 | `Input`          | 只读，带浏览按钮       |
| 进度条     | `ProgressBar`    | 安装进度               |
| Logo       | `Image` 或自定义 | 居中显示               |

## 实现示例

### 1. 主按钮

```rust
use gpui::*;
use gpui_component::button::Button;

fn render_install_button(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
    Button::new("install_btn")
        .label("开始安装")
        .primary()
        .width(px(240.0))
        .height(px(40.0))
        .on_click(cx.listener(|view, _event, cx| {
            // 开始安装
            view.start_installation(cx);
        }))
}
```

### 2. 复选框

```rust
use gpui_component::checkbox::Checkbox;

fn render_agree_checkbox(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
    Checkbox::new("agree")
        .label("我同意用户协议和隐私政策")
        .checked(self.agreed)
        .on_click(cx.listener(|view, checked, cx| {
            view.agreed = *checked;
            cx.notify();
        }))
}
```

### 3. 进度条

```rust
use gpui_component::progress::ProgressBar;

fn render_progress(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
    ProgressBar::new()
        .value(self.progress_percentage)
        .width(percent(100.0))
        .height(px(6.0))
}
```

### 4. 输入框

```rust
use gpui_component::input::Input;

fn render_path_input(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
    h_flex()
        .gap_2()
        .child(
            Input::new("install_path")
                .value(self.install_path.clone())
                .readonly(true)
                .width(px(410.0))
        )
        .child(
            Button::new("browse")
                .label("浏览...")
                .on_click(cx.listener(|view, _, cx| {
                    view.browse_directory(cx);
                }))
        )
}
```

## 样式定制

### 使用自定义颜色

```rust
use gpui_component::theme::Theme;

// 创建自定义主题
let theme = Theme {
    primary: rgb(0x00C4B2),        // #00C4B2 青色
    primary_hover: rgb(0x00FFE8),  // #00FFE8 浅青
    background: rgb(0x181B22),     // #181B22 深色背景
    text: rgb(0xFFFFFF),           // 白色文字
    ..Default::default()
};

// 应用主题
cx.set_theme(theme);
```

### 自定义按钮样式

```rust
Button::new("custom")
    .label("自定义按钮")
    .style(ButtonStyle {
        background: Some(rgb(0x00C4B2)),
        background_hover: Some(rgb(0x00FFE8)),
        text_color: rgb(0xFFFFFF),
        border_radius: px(8.0),
        ..Default::default()
    })
```

## 页面实现

### 配置页面

```rust
use gpui::*;
use gpui_component::*;

pub struct ConfigPage {
    agreed: bool,
    expanded: bool,
    install_path: String,
    create_shortcut: bool,
    auto_run: bool,
}

impl ConfigPage {
    pub fn new() -> Self {
        Self {
            agreed: false,
            expanded: false,
            install_path: r"C:\Program Files\MyApp".to_string(),
            create_shortcut: true,
            auto_run: true,
        }
    }
}

impl Render for ConfigPage {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(rgb(0x181B22))  // 背景色
            .child(
                // Logo 区域
                v_flex()
                    .h(px(96.0))  // 顶部空白
            )
            .child(
                // Logo 图片
                img("assets/logo.png")
                    .w(px(148.0))
                    .h(px(80.0))
                    .mx_auto()
            )
            .child(
                // 主按钮
                Button::new("install")
                    .label("开始安装")
                    .primary()
                    .w(px(240.0))
                    .h(px(40.0))
                    .mx_auto()
                    .mt(px(55.0))
                    .disabled(!self.agreed)
                    .on_click(cx.listener(|view, _, cx| {
                        // 开始安装
                    }))
            )
            .child(
                // 底部协议区
                self.render_bottom_section(cx)
            )
            .when(self.expanded, |this| {
                this.child(self.render_expanded_section(cx))
            })
    }
}

impl ConfigPage {
    fn render_bottom_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        h_flex()
            .h(px(87.0))
            .p_4()
            .gap_2()
            .child(
                Checkbox::new("agree")
                    .label("我同意")
                    .checked(self.agreed)
                    .on_click(cx.listener(|view, checked, cx| {
                        view.agreed = *checked;
                        cx.notify();
                    }))
            )
            .child(
                Button::new("agreement")
                    .label("用户协议")
                    .link()  // 链接样式
            )
            .child(Label::new("和"))
            .child(
                Button::new("policy")
                    .label("隐私政策")
                    .link()
            )
            .child(div().flex_1())  // 占位
            .child(
                Button::new("toggle_more")
                    .label(if self.expanded { "收起" } else { "展开更多" })
                    .on_click(cx.listener(|view, _, cx| {
                        view.expanded = !view.expanded;
                        cx.notify();
                    }))
            )
    }
    
    fn render_expanded_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .p_4()
            .gap_2()
            .bg(rgb(0x1A1D24))  // 稍浅的背景
            .child(
                // 安装路径
                h_flex()
                    .gap_2()
                    .child(
                        Input::new("path")
                            .value(self.install_path.clone())
                            .readonly(true)
                            .w(px(410.0))
                    )
                    .child(
                        Button::new("browse")
                            .label("浏览...")
                    )
            )
            .child(
                // 空间显示
                h_flex()
                    .gap_4()
                    .child(Label::new("所需空间：100 MB"))
                    .child(Label::new("可用空间：50 GB"))
            )
            .child(
                // 选项
                h_flex()
                    .gap_4()
                    .child(
                        Checkbox::new("shortcut")
                            .label("创建桌面快捷方式")
                            .checked(self.create_shortcut)
                    )
                    .child(
                        Checkbox::new("autorun")
                            .label("开机自启动")
                            .checked(self.auto_run)
                    )
            )
    }
}
```

### 安装进度页

```rust
pub struct InstallingPage {
    progress: f32,
    status_text: String,
}

impl Render for InstallingPage {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(rgb(0x181B22))
            .child(
                // 顶部空白
                div().h(px(260.0))
            )
            .child(
                // 进度条
                ProgressBar::new()
                    .value(self.progress)
                    .w_full()
                    .h(px(6.0))
            )
            .child(
                // 状态文字
                Label::new(self.status_text.clone())
                    .text_color(rgb(0xFFFFFF))
                    .mt(px(25.0))
                    .mx_auto()
            )
    }
}
```

### 完成页

```rust
pub struct FinishPage {
    can_run: bool,
}

impl Render for FinishPage {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(rgb(0x181B22))
            .child(div().h(px(80.0)))
            .child(
                img("assets/logo.png")
                    .w(px(148.0))
                    .h(px(80.0))
                    .mx_auto()
            )
            .child(
                Label::new("安装完成")
                    .text_size(px(14.0))
                    .text_color(rgb(0xFFFFFF))
                    .mt(px(16.0))
                    .mx_auto()
            )
            .child(
                Button::new("run")
                    .label("立即使用")
                    .primary()
                    .w(px(240.0))
                    .h(px(40.0))
                    .mx_auto()
                    .mt(px(40.0))
                    .on_click(cx.listener(|view, _, cx| {
                        // 启动应用
                    }))
            )
    }
}
```

## 向导容器

```rust
pub struct Wizard {
    current_page: WizardPage,
    config_page: View<ConfigPage>,
    installing_page: View<InstallingPage>,
    finish_page: View<FinishPage>,
}

impl Render for Wizard {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .size(px(574.0), px(358.0))
            .child(
                match self.current_page {
                    WizardPage::Config => self.config_page.clone(),
                    WizardPage::Installing => self.installing_page.clone(),
                    WizardPage::Finish => self.finish_page.clone(),
                }
            )
            .child(
                // 关闭按钮（浮动在右上角）
                Button::new("close")
                    .icon("close")
                    .absolute()
                    .top(px(16.0))
                    .right(px(16.0))
                    .on_click(cx.listener(|_, _, cx| {
                        cx.quit();
                    }))
            )
    }
}
```

## 主题配置

创建符合 NSIS 设计的主题：

```rust
use gpui_component::theme::*;

pub fn create_installer_theme() -> Theme {
    Theme {
        // 主色调
        primary: rgb(0x00C4B2),           // #00C4B2
        primary_hover: rgb(0x00FFE8),     // #00FFE8
        primary_active: rgb(0x00D9C5),    // #00D9C5
        
        // 背景
        background: rgb(0x181B22),        // #181B22
        surface: rgb(0x1A1D24),           // 稍浅
        
        // 文字
        text: rgb(0xFFFFFF),              // #FFFFFF
        text_muted: rgb(0xE4E8EC),        // #E4E8EC
        text_disabled: rgb(0x96A1A9),     // #96A1A9
        
        // 边框
        border: rgb(0x00C4B2).with_alpha(0.2),
        
        // 其他
        shadow: rgb(0x000000).with_alpha(0.3),
        
        ..Default::default()
    }
}
```

## 注意事项

### 1. Windows 7 兼容性

⚠️ **重要**：gpui-component 基于 GPUI，同样面临 Windows 7 兼容性问题。

建议：
- 先在 Windows 10+ 上开发和测试
- 再在 Windows 7 虚拟机上测试
- 如果有问题，准备降级方案

### 2. 图片加载

使用 `include_bytes!` 嵌入图片：

```rust
const LOGO: &[u8] = include_bytes!("../../assets/logo.png");

// 在组件中使用
img(LOGO)
    .w(px(148.0))
    .h(px(80.0))
```

### 3. 多语言集成

```rust
use crate::i18n;

Button::new("install")
    .label(i18n::tr("button_install").to_string())
    .primary()
```

### 4. 状态管理

使用 GPUI 的响应式状态：

```rust
pub struct AppState {
    progress: f32,
    status: String,
}

// 在 View 中使用
view.update(cx, |view, cx| {
    view.state.progress = 50.0;
    cx.notify();  // 触发重新渲染
});
```

## 下一步

1. ✅ 已添加 gpui-component 依赖
2. 实现基础页面组件
3. 测试组件库功能
4. 集成安装逻辑
5. 在 Windows 7 上测试

## 参考资源

- [gpui-component GitHub](https://github.com/longbridge/gpui-component)
- [gpui-component 文档](https://github.com/longbridge/gpui-component/tree/main/docs)
- [GPUI 官方文档](https://github.com/zed-industries/zed)

