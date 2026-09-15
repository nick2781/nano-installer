# 项目结构

## 工具源码

```text
crates/
├── nano-installer-core/       # bundle、XML、WIC/GDI 和共享 runtime
├── nano-installer-cli/        # builder
├── nano-installer-gui/        # Windows 10+ eframe/egui 工具
├── nano-installer-stub-lzma/  # 独立 package；只依赖 sevenz-rust
├── nano-installer-stub-zlib/  # 独立 package；只依赖 zip/deflate
└── nano-installer-uninstaller/ # 独立 package；不依赖 archive backend
```

生成的产品安装包和卸载器使用项目自己的资源，原始 stubs 不包含产品资源。

## 产品项目

```text
product-installer/
├── installer_config.json
├── layouts/
├── assets/
├── locales/
├── scripts/                   # 可选，当前只打包不执行
└── payload/app.7z             # 也可以是 ZIP
```

所有配置路径都相对于产品项目目录。Native builder 当前收集整个 layouts、assets、locales
目录、可选 scripts 目录以及配置指定的 payload。

## 发布目录

```text
target/release/
├── nano-installer-native-x64.exe
├── nano-installer-gui-x64.exe
└── stubs/
    ├── lzma-stub-native.exe
    ├── zlib-stub-native.exe
    └── uninst-stub-native.exe
```

`target/x86_64-win7-windows-msvc/` 是 Cargo cross-target 缓存，不是另一套发布物。
Example setup 默认位于 `examples/TapTap/dist/`，不会进入工具链发布包。
