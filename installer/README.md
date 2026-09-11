# installer

本目录是 `nano-installer` 的可执行代码，按构建工具、共享运行时和 stub 划分：

```text
installer/
├── cli/          # nano-installer.exe
├── lib/          # 配置、资源、布局、UI、安装/卸载核心
└── stubs/
    ├── lzma/     # lzma-x64.exe
    ├── zlib/     # zlib-x64.exe
    └── uninst/   # uninst-x64.exe
```

## 依赖方向

```text
cli ---------> lib
lzma stub ---> lib
zlib stub ---> lib
uninst ------> lib
```

CLI 只负责构建期操作。三个预编译 stub 都链接 `lib`，再由 CLI 注入产品资源。产品差异应放在
项目目录的配置、XML、语言包和脚本中，不应写进这里。

## 修改后如何验证

```powershell
cargo fmt --all -- --check
cargo test -p nano-installer-lib --lib
cargo test -p nano-installer-cli
cargo build --locked --release -p nano-installer-lzma
cargo build --locked --release -p nano-installer-zlib
cargo build --locked --release -p uninst
cargo build --locked --release -p nano-installer-cli
.\target\release\nano-installer.exe `
  build --project .\examples\TapTap --release
```

架构和模块职责见 [架构文档](../docs/ARCHITECTURE.md)，构建边界见
[开发指南](../docs/DEVELOPMENT.md)。
