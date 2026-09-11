# 项目结构

```text
nano-installer/
├── installer/
│   ├── cli/                 # 构建工具
│   ├── lib/                 # 共享核心库
│   └── stubs/
│       ├── lzma/            # 7z/LZMA 安装器 stub
│       ├── zlib/            # ZIP/Deflate 安装器 stub
│       └── uninst/          # 卸载器 stub
├── examples/
│   └── TapTap/              # 唯一完整示例
├── templates/               # init 命令内嵌模板
├── assets/                  # 工具链图标
├── tools/                   # 7za.exe 等辅助工具
├── scripts/                 # 标准/Win7 构建、签名、PE 审计和 smoke 脚本
├── tests/                   # workspace 集成测试
└── docs/                    # docsify 文档
```

## 产品项目结构

产品项目可以位于本仓库外，只要把路径传给 `--project`：

```text
product-installer/
├── installer_config.json
├── assets/
├── layouts/
├── locales/
├── scripts/                 # 可选
├── payload/app.7z
├── .build/                  # 生成，不提交
└── dist/                    # 发布候选产物
```

`installer_config.json` 中的资源路径均相对于产品项目目录。不要从 `.build/` 反向维护
配置或语言包。

## 产物

- `target/release/nano-installer.exe`
- `target/release/lzma-x64.exe`
- `target/release/zlib-x64.exe`
- `target/release/uninst-x64.exe`
- `<project>/dist/<Product>_Setup.exe`

CLI、所选压缩算法的 x64 installer stub 和 x64 uninstaller stub 构成构建工具包。
