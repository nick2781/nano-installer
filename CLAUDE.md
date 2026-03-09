# Claude Code 项目指令

开发规范和架构详情见 [agent.md](./agent.md)。

## 关键规则

1. **零硬编码** — 所有 UI 尺寸、颜色、文案从 XML/配置/locale 读取，代码中禁止出现魔法数字
2. **NSIS 对齐** — 修改 UI 前必须先读 NSIS 参考配置（`D:\taptap-pc\taptap-pc\setup\NSIS_SetupSkin\SetupScripts\TapTap_CN\`）
3. **原生 DPI** — 不设 `pixels_per_point`，窗口用逻辑尺寸，egui 自动缩放
4. **dest UV 用逻辑尺寸** — `compute_uv_rect` 使用 `get_render_size()`，不是 `texture.size()`
5. **双格式同步** — 动态修改元素属性时同时更新 `attributes.*` 和 `visual_style.*`
6. **测试验证传递关系** — 断言值来自配置，不是断言硬编码默认值
7. **Action 驱动** — 按钮行为由 XML `action` 属性声明，代码通过 `dispatch_action()` 分发，禁止硬编码 button_id→行为映射
8. **配置驱动页面** — 页面使用 String ID（非枚举），顺序从 `config.wizard.pages[]` 读取，新增页面不需要修改代码
9. **Links 通用映射** — `config.links` 是 `HashMap<String, String>`，`action="open_url:KEY"` 查 map
10. **Locale 覆盖** — 所有用户可见文案必须在 `locales/*.json` 中定义，Rust 代码中不出现用户可见的中文字符串
