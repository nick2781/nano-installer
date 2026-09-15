# TapTap 示例安装器

这是 native 分支唯一的端到端输入项目，用于直接验证 XML 布局、多语言、图片资源、
payload 打包和 Win7 SP1+ 原生运行时。

## 资源权属

`examples/TapTap` 下的 TapTap 名称、商标、图片、文案及相关素材归易玩（上海）网络
科技有限公司及相关权利人所有，仅用于 `nano-installer` 工具的开发、测试和兼容性验证。
这些资源不属于本项目开源许可范围，不得视为对外授权的产品素材。

## 目录

```text
TapTap/
├── installer_config.json
├── assets/
├── layouts/
├── locales/
├── scripts/
└── payload/app.7z
```

## 构建

从仓库根目录执行：

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

工具链输出位于 `target/release/`，example setup 输出位于
`examples/TapTap/dist/TapTap_Setup.exe`。后者只用于本地验证，不属于发布文件。

## 用作产品起点

复制目录后至少替换：

- `installer_config.json` 中的产品信息、安装路径、EXE、注册表键和输出名
- `assets/` 中的图标与品牌资源
- `layouts/` 中的页面文案键、链接和产品交互
- `locales/` 中的全部用户可见文案
- `scripts/` 中的产品专用安装与卸载行为
- `payload/app.7z` 中的应用文件

当前 native runtime 尚未实现 payload 解压、安装任务、完整页面流和卸载流程，不能用于
生产发布。
