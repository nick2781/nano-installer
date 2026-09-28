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
├── index.html                  # 站点由这个 docsify 外壳渲染
├── llms.txt                    # 每一页的清单，写给 agent 读
├── robots.txt, sitemap.xml     # 同一份页面清单，给爬虫看
├── en/                         # 英文文档
└── zh-CN/                      # 中文文档
examples/TapTap/                # 校验项目
examples/nsis-migration/        # 从 NSIS 迁过来的示例与它的对照脚本
examples/plugin-c/              # 插件 ABI 的 C 实现，只用 MSVC 就能编
scripts/                        # 构建、冒烟测试、PE 审计、文档索引生成
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

## 文档站

对外发布的站点是一个 [docsify](https://docsify.js.org/) 外壳：`docs/index.html` 在浏览器里渲染
Markdown，GitHub Pages 上只有这一个 HTML。每一页同时就是它自己那个 Markdown 文件，所以这个站点
不用浏览器也能读——编码 agent 手里就只有这个。站点渲染的内容就是那些源文件，没有为阅读另外生成
任何东西。

让「跑不了网页的东西」也能找到这些页面的，是三个文件，由 `scripts/build_docs_index.ps1` 依据
`scripts/docs_index.json` 写出来：

| 文件 | 里面是什么 |
| --- | --- |
| `docs/llms.txt` | 每一页一行，链接直接指向那一页自己的 Markdown 文件，后面跟一句说明；站点之外的仓库文件另列一节 |
| `docs/robots.txt` | 全部公开，以及 sitemap 在哪 |
| `docs/sitemap.xml` | 同一份清单的 XML 版。里面列的是 Markdown 文件而不是网页的 `#/` 地址，因为片段地址爬虫取不到 |

`.\scripts\build_docs_index.ps1 -Verify` 就是检查，而且它和生成用的是同一份代码，所以两者不可能对不上。
页面在 `scripts/docs_index.json` 里没有条目、条目指向一个不存在的文件、标题或说明为空、标题与页面自己的
`# 标题` 不一致、仓库文件搬走了、以及某一页不属于任何分组，全都是失败。因此新增一页，就必须在同一个改动里
把它的那一行加进那份数据文件。

侧边栏那两个文件（`_sidebar.md`、`_navbar.md`）是导航而不是页面，所有以下划线开头的文件都因为这个原因
不进索引。
