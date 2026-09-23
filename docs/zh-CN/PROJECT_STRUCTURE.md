# 项目结构

## 仓库

```text
crates/
├── nano-installer-core/        # 打包、XML 布局、WIC/GDI 渲染与共享运行时
├── nano-installer-cli/         # 命令行构建器
├── nano-installer-gui/         # Windows 10+ 可视化构建器
├── nano-installer-stub-lzma/   # 7z 运行时，只链接 sevenz-rust
├── nano-installer-stub-zlib/   # ZIP 运行时，只链接 zip/deflate
└── nano-installer-uninstaller/ # 卸载运行时，不链接解包后端
docs/
├── en/                         # 英文文档
└── zh-CN/                      # 中文文档
examples/TapTap/                # 校验项目
examples/nsis-migration/        # 从 NSIS 迁过来的示例与它的对照脚本
scripts/                        # 构建、冒烟测试、PE 审计
```

原始运行时不带任何产品资源：图标、版本信息和品牌素材都靠构建器按你的项目配置注入，
所以同一套运行时可以为任意产品生成安装包。

从 NSIS 搬过来的项目看[从 NSIS 迁移](MIGRATION_FROM_NSIS.md)：那里逐条写着每个构造在这里变成什么，
`examples/nsis-migration/migrated/` 就是照它写出来的一份可以构建的工程。

## 产品项目

```text
product-installer/
├── installer_config.json
├── layouts/                    # XML 页面
├── assets/                     # 背景、按钮、图标
├── locales/                    # 每种语言一个 JSON 文件
├── scripts/                    # 可选的安装/卸载逻辑
└── payload/app.7z              # 也可以是 ZIP 归档
```

配置里的每个路径都相对项目目录解析。构建器会把整个 `layouts`、`assets`、`locales` 目录、可选的
`scripts` 目录，以及配置里指定的那一个应用文件（payload）一起打包进去。

## 构建产物

```text
target/release/
├── nano-installer-native-x64.exe
├── nano-installer-gui-x64.exe
└── stubs/
    ├── lzma-stub-native.exe
    ├── zlib-stub-native.exe
    └── uninst-stub-native.exe
```

构建时加上 `--msi`，会在安装包旁边多写出一个企业按 MSI 分发的包。

`target/x86_64-win7-windows-msvc/` 只是 Cargo 的交叉编译缓存，不是另一套发布文件。示例安装包输出
到 `examples/TapTap/dist/`，不算发布内容。
