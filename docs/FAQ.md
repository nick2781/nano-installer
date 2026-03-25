# 常见问题

## 这是一个什么项目？

`nano-installer` 是一个现代化的 Windows 安装器框架，使用 Rust 构建，提供配置驱动的打包流程、原生 GUI、国际化、XML 布局 DSL，以及安装器 / 卸载器打包能力。

## 什么时候需要重编 stub/runtime？

- 修改 `installer/**` 下的源码时，需要先重编：
  - `nano-installer-lzma`
  - `uninst`
  - `nano-installer-cli`
- 然后再重新打包目标 setup。

## 什么时候只需要重建示例工程？

如果只改了 `examples/**` 下的资源、布局、文案或配置，不需要重编 stub/runtime，只需要重新：

```bash
target/release/nano-installer.exe build --project examples/TapTap
```

## `1x/@2x` 资源是怎么工作的？

XML 布局默认只写 `1x` 资源路径。运行时会根据当前机器 DPI 自动选择 `@2x` 资源，并按逻辑尺寸回落渲染，不需要在布局里手动维护两套路径。

## 当前推荐从哪个示例开始？

推荐优先参考 [TapTap](../examples/TapTap/) 和 [TapTap-Global](../examples/TapTap-Global/)。它们分别提供国内版与海外版的最新视觉和交互基线。

## 安装 payload 是怎么解压的？

当前默认使用内置的 `7za` 解压链路，兼顾兼容性和解压速度。

## 文档站怎么本地预览？

文档站基于 `docsify` 组织。进入 `docs/` 后，可以使用任意静态文件服务器进行预览，例如：

```bash
npx docsify serve docs
```

或者使用本地 HTTP server 指向 `docs/` 目录。
