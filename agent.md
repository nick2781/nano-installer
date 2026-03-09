# nano-installer 开发规范

## 核心原则

### 零硬编码
- **所有尺寸、颜色、文案、间距**必须从 XML 布局或 `installer_config.json` 读取
- 代码中不允许出现 `574`, `358`, `16.0`, `#FF00FFE8` 等硬编码值
- 合理的回退值（如 `unwrap_or(0.0)` 表示间距默认 0）可以接受
- 图片加载失败时的兜底渲染（灰色方框）可以接受，但不应是正常路径

### 配置驱动
- 窗口尺寸: `config.ui.window_width / window_height / expanded_height`
- 对话框尺寸: `config.ui.dialog_width / dialog_height`
- DPI 阈值: `config.ui.dpi_threshold`
- 安装路径/产品名等: `config.install.*` / `config.project.*`
- UI 文案: `locales/*.json` 通过 `text="@key"` 引用
- 布局/控件: `layouts/*.xml` 定义所有 UI 元素

### NSIS 对齐
- 本项目是 NSIS 安装器的 Rust 替代品，必须与 NSIS 行为保持一致
- 参考目录: `D:\taptap-pc\taptap-pc\setup\NSIS_SetupSkin\SetupScripts\TapTap_CN\`
- 关键参考文件:
  - `skin/configpage.xml` — 1x 布局定义（尺寸、位置、颜色、图片路径）
  - `skin/configpage2x.xml` — 2x 布局定义
  - `lang_strings.nsh` — 所有 UI 文案的中文版
- 每次修改 UI 前，必须先读 NSIS 对应配置确认正确值

## DPI 处理

### 方案：拥抱 egui 原生 DPI
- 窗口逻辑尺寸始终等于配置中的 1x 设计尺寸（如 574x358）
- **不设** `pixels_per_point = 1.0`，让 egui 自动检测系统 DPI
- XML 坐标和 Taffy 输出在 1x 逻辑空间，直接匹配 egui 逻辑坐标
- `use_2x` 仅驱动资源选择（加载 @2x 图片）
- `get_render_size()` 将 @2x 纹理尺寸除 2 得到逻辑尺寸

### dest 裁剪坐标
- XML 中的 `dest='x1,y1,x2,y2'` 始终是 1x 逻辑像素坐标
- UV 计算必须用逻辑纹理尺寸（`get_render_size`），不是物理 `texture.size()`
- 使用 `Self::compute_uv_rect(dest, texture, &dpi_config)` 统一处理

## XML 布局格式

### 新格式（<Page> 根元素）
```xml
<Page width="574" height="358" background-image="assets/bg.png">
  <VBox width="100%" height="100%">
    <Button id="btn" width="240" height="40" align-self="center"
            normal-image="assets/btn.png" text="@key" />
  </VBox>
</Page>
```

### 属性命名
- 新格式用 kebab-case: `align-self`, `flex-grow`, `font-size`, `background-image`
- 兼容 NSIS 旧属性: `normalimage`, `textpadding`, `borderround`, `bkcolor`
- 解析器自动识别两种格式

### 控件属性 → custom 映射
新格式解析时，未明确处理的属性通过 `widget.custom` 传递到 `attrs.custom`，渲染器通过 `element.attributes.get_custom("key")` 读取。

## Action 系统

### 按钮行为声明
按钮在 XML 中通过 `action` 属性声明行为，代码通过 `dispatch_action()` 统一分发：

```xml
<Button id="btnInstall" action="install" text="@install_button" ... />
<Button id="btnRun" action="launch_app" text="@launch_button" ... />
<Button id="btnShowMore" action="toggle_panel:moreconfiginfo:show" ... />
<Button id="btnAgreement" action="open_url:terms_of_service" ... />
<Button id="close" action="close_confirm" ... />
```

### 可用 Action 列表

| action | 参数 | 行为 |
|--------|------|------|
| `next_page` | 无 | wizard.next() |
| `prev_page` | 无 | wizard.previous() |
| `install` | 无 | start_installation() |
| `uninstall` | 无 | start_uninstall_task() |
| `launch_app` | 无 | 从 config 读 exe 路径并启动 |
| `close` | 无 | 关闭窗口 |
| `close_confirm` | 无 | 显示关闭确认对话框 |
| `open_url:KEY` | config.links 的 key | 浏览器打开 URL |
| `toggle_panel:ID:show/hide` | 元素 ID + 方向 | 切换面板 + 调窗口大小 |
| `browse_folder` | 无 | 打开文件夹选择器 |
| `cancel` | 无 | 取消安装 |
| `dialog_ok` | 无 | 对话框确认 |
| `dialog_cancel` | 无 | 对话框取消 |

### Page ID 规则
- 页面使用 String ID 而非枚举，从 `config.wizard.pages[]` 读取
- 安装流程: `["config", "installing", "finish"]`
- 卸载流程: `["uninstall_confirm", "uninstall_progress", "uninstall_finish"]`
- 新增页面只需: 编辑 `installer_config.json` + 创建 XML 文件

### Links 配置
`config.links` 是 `HashMap<String, String>` 通用映射:
```json
{ "links": { "terms_of_service": "https://...", "privacy_policy": "https://..." } }
```
`action="open_url:terms_of_service"` 直接查 links map。

### 运行模式
- `Install` — 正常安装流程 (config → installing → finish)
- `Update` — 跳过配置页 (installing → finish)，CLI `--mode update`
- `Silent` — 无 GUI 直接安装，CLI `--silent` 或 `/S`
- `Uninstall` — 卸载流程，自动通过 exe 文件名 `uninst` 检测

### 卸载器架构
- uninst.exe = lzma stub 相同引擎 (egui UI + 嵌入资源 bundle)
- build 时打包全部 layouts/assets/locales 到 uninst.exe
- 通过 exe 文件名自动检测为 Uninstall 模式
- 卸载路径从 `current_exe()` 父目录获取
- 自删除: 准备批处理脚本 → 用户点完成按钮时启动 → 进程退出后批处理重试删除

### TaskRunner (可配置任务流水线)
- 从 `config.install_tasks` 读取任务列表，不配置则用默认流水线
- 默认: extract → copy_uninstaller → create_shortcuts → write_registry
- GUI 安装和静默安装共用同一 TaskRunner
- 失败时反向 rollback (框架已就位)

### Locale Key 命名规范
- UI 按钮/标签: `install_button`, `launch_button`, `cancel`
- 安装状态: `status.preparing`, `status.install_complete`
- 卸载状态: `uninstall.status.closing_app`, `uninstall.status.complete`
- 错误信息: `error.path_illegal_char`, `error.no_write_permission`
- 警告信息: `warning.removable_disk`, `warning.system_dir`

## 状态同步

### 运行时修改元素属性
当代码动态修改布局元素时（如 visible、enabled、text、color），必须同时更新：
- `element.attributes.*` — 旧格式字段
- `element.visual_style.*` — 新格式字段（如果存在）

```rust
// 正确做法
element.attributes.visible = Some(visible);
if let Some(vs) = &mut element.visual_style {
    vs.visible = visible;
}
```

## 测试要求

- 测试必须覆盖实际改动的核心逻辑，不是只测构造函数
- 配置值读取必须有测试验证（自定义值能正确传递）
- UV 计算、布局计算等关键路径必须有测试
- 不要写 `assert_eq!(dpi_config.window_width, 574.0)` 这种断言硬编码默认值的测试
- 应该写 `assert_eq!(dpi_config.window_width, config.ui.window_width as f32)` 验证传递关系

## 构建与测试

```bash
# 编译 (debug)
cargo build

# 编译 (release, 用于打包)
cargo build --release --bin lzma-x64-unicode --bin uninst

# 运行测试
cargo test

# 打包 TapTap 安装器
cargo run --bin nano-installer -- build --project examples/TapTap

# 测试安装器
cd examples/TapTap/dist && ./TapTap_Setup.exe

# 测试卸载器 (从安装目录运行)
# "C:\Program Files\TapTapTest\uninst.exe"

# 查看运行日志
cat examples/TapTap/dist/install_*.log | grep -E "WARN|ERROR"
```

## Release 配置
- `opt-level = "s"` (不用 `"z"`，会导致闪退)
- `lto = true`, `codegen-units = 1`, `strip = true`
- 不用 `panic = "abort"` (需要 backtrace 调试)
- release stub ~6MB, 安装包 ~17MB
