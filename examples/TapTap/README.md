# TapTap 示例安装器

这是仓库中保留的完整示例，用于验证多页面安装/更新/卸载流程、多语言、XML 布局、
Rhai 脚本以及 `1x/@2x` 图片资源。

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
├── payload/app.7z
├── .build/                 # 中间产物
└── dist/TapTap_Setup.exe   # 最终产物
```

## 构建

从仓库根目录执行：

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

若只修改本示例的配置、资源、布局、语言或脚本，并且 release stubs 已经是最新版本：

```powershell
.\scripts\build.ps1 -Project examples\TapTap -SkipStubs
```

## 用作产品起点

复制目录后至少替换：

- `installer_config.json` 中的产品信息、安装路径、EXE、注册表键和输出名
- `assets/` 中的图标与品牌资源
- `layouts/` 中的页面文案键、链接和产品交互
- `locales/` 中的全部用户可见文案
- `scripts/` 中的产品专用安装与卸载行为
- `payload/app.7z` 中的应用文件

正式接入生产项目之前，请按[生产接入指南](../../docs/PRODUCTION_INTEGRATION.md)
完成签名、升级/卸载、静默模式和回滚验证。
