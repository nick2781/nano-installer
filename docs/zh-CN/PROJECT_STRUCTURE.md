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

原始运行时里不带产品资源。图标、版本信息和品牌素材都来自你的项目配置，由构建器注入。这样一套运行时就能给任意产品生成安装包。

从 NSIS 搬过来的项目，先看[从 NSIS 迁移](MIGRATION_FROM_NSIS.md)。那一页逐条说明每个构造在这里变成什么。`examples/nsis-migration/migrated/` 是照它写出来的一份工程，可以直接构建。

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

配置里的每个路径都相对项目目录解析。构建器会打包三个目录：`layouts`、`assets`、`locales`。可选的 `scripts` 目录也一起打包。配置里指定的那一个应用文件，同样打包进去。

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

构建时加上 `--msi`，安装包旁边会多出一个包，给企业按 MSI 分发。

`target/x86_64-win7-windows-msvc/` 只是 Cargo 的交叉编译缓存。它不是另一套发布文件。示例安装包输出到 `examples/TapTap/dist/`，不算发布内容。

## 文档站

对外发布的站点是一个 [docsify](https://docsify.js.org/) 外壳。`docs/index.html` 在浏览器里渲染 Markdown。GitHub Pages 上只有这一个 HTML。每一页同时就是它自己的 Markdown 文件。不开浏览器也能读这个站点，agent 手里就是这些文件。站点渲染的就是源文件，没有另外生成什么给读者。

有四个文件让不运行网页的东西也能找到这些页面。它们都由 `scripts/build_docs_index.ps1` 依据 `scripts/docs_index.json` 写出来：

| 文件 | 里面是什么 |
| --- | --- |
| `docs/llms.txt` | 每一页一行，直接链接到那一页的 Markdown 文件，后面跟一句说明。站点之外的仓库文件另列一节 |
| `docs/llms-full.txt` | 把声明要收进来的分组合成一份，也就是英文文档。每页前面写上它自己的地址。测试覆盖表没收进来，那是用来查的，不是用来通读的 |
| `docs/robots.txt` | 全部公开。sitemap 在哪也写在这里 |
| `docs/sitemap.xml` | 同一份清单的 XML 版。列的是 Markdown 文件，不是网页的 `#/` 地址。片段地址爬虫取不到 |

`.\scripts\build_docs_index.ps1 -Verify` 就是检查。写和查用的是同一份代码，两者不可能对不上。下面这些都会让检查失败。某一页在 `scripts/docs_index.json` 里没有条目。条目指向不存在的文件。标题或说明是空的。标题和页面自己的 `# 标题` 对不上。仓库文件搬走了。某一页不属于任何分组。把不是页面的名字写进「不收进合成文件」的名单。没有任何页面会进那份文件。新增一页，就得在同一个改动里把它的那一行加进数据文件。收不收进 `llms-full.txt` 由它所在的分组决定，单页也能覆盖这个决定。站点首页只是两个入口，中文、英文各一个，不是用来读的页面，靠这一点把它挡在英文文档之外。

侧边栏那两个文件是导航，不是页面：`_sidebar.md`、`_navbar.md`。所有以下划线开头的文件都不进索引。

读者靠站点顶栏的下拉菜单在两棵树之间切换。它是 `docs/index.html` 里的一段脚本，不是写在每一页里的链接。菜单显示当前页面用的是哪种语言。选另一种，就落到**同一页**。能配对的页名就写在这个文件里。有两个检查守着它和目录一致。`scripts/audit_docs_languages.ps1` 拿页名跟 `docs/en`、`docs/zh-CN` 里的文件逐个核对。`scripts/check_language_switch.js` 在没有浏览器的情况下把这段脚本跑一遍。两个检查都在文档工作流里，和索引检查并排。

`docs/AGENTS.md` 是这套文档自己的地图。它说清哪个问题看哪一页、两种语言怎么对应、机器可读的入口在哪。它在 `docs/` 根目录下，不在两棵树里。中文版是 `docs/zh-CN/AGENTS.md`。别的页面都在另一棵树里有对照的一份。
