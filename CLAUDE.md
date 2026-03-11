# Claude Code 项目指令

开发规范和架构详情见 [agent.md](./agent.md)。

## 关键规则

1. **零硬编码** — 所有 UI 尺寸、颜色、文案从 XML/配置/locale 读取，代码中禁止出现魔法数字
2. **NSIS 对齐** — 修改 UI 前必须先读 NSIS 参考配置（`D:\taptap-pc\taptap-pc\setup\NSIS_SetupSkin\SetupScripts\TapTap_CN\`）
3. **原生 DPI** — 不设 `pixels_per_point`，窗口用逻辑尺寸，egui 自动缩放
4. **dest UV 用逻辑尺寸** — `compute_uv_rect` 使用 `get_render_size()`，不是 `texture.size()`
5. **双格式同步** — 动态修改元素属性时同时更新 `attributes.*` 和 `visual_style.*`
6. **测试验证传递关系** — 断言值来自配置，不是断言硬编码默认值
7. **Action 驱动** — 按钮行为由 XML `action` 属性声明，代码通过 `dispatch_action()` 分发
8. **配置驱动页面** — 页面使用 String ID，顺序从 `config.wizard.pages[]` 读取
9. **Links 通用映射** — `config.links` 是 `HashMap<String, String>`
10. **Locale 覆盖** — 所有用户可见文案必须在 `locales/*.json` 中定义
11. **三层架构** — config.json (声明式) > TaskRunner (默认) > scripts/*.rhai (脚本覆盖)
12. **原子化 API** — 脚本 API 提供原子能力（文件/注册表/进程等），业务逻辑在脚本中组合
13. **配置是数据，脚本是逻辑** — 配置描述"是什么"，脚本描述"做什么"，不在配置中表达逻辑流程
14. **Stub 必须重编译** — 修改 Rust 代码后必须 `touch` 源文件再 `cargo build --release --bin lzma-x64-unicode`（确认有 `Compiling` 输出），然后重新 build installer。仅修改 XML/locale/config 不需要重编译 stub，但需要重新 build installer
