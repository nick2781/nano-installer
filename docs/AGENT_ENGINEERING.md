# Agent Engineering

## Goal

这里的目标不是“多写几个测试命令”，而是让 AI agent 能更稳定地理解、修改、验证这个项目。

面向 agent 的工程约束分三层：

1. `agent interfaces`
   - 提供稳定、可组合、可机读的入口
   - 让 agent 不需要靠猜测或人工操作驱动项目
2. `build/runtime contracts`
   - 明确哪些改动需要重编 stub，哪些只需要重打 example
   - 减少“代码改了但产物没更新”的隐式状态
3. `verification tooling`
   - 提供 snapshot、lint、smoke 等验证工具
   - 让 agent 可以先收敛结构化证据，再做真实 EXE 验证

## Agent Interfaces

当前已经具备的 agent-facing 入口：

- `nano-installer build --project <dir>`
  - 构建项目安装包
- `nano-installer validate <config>`
  - 校验配置
- `nano-installer harness snapshot --project <dir> ...`
  - 导出结构化 UI 布局快照
- `nano-installer harness lint-resources --project <dir>`
  - 校验布局引用的资源
- PowerShell smoke 脚本
  - [smoke_test.ps1](/D:/taptap-pc/nano-installer/scripts/smoke_test.ps1)
  - [smoke_script_mode.ps1](/D:/taptap-pc/nano-installer/scripts/smoke_script_mode.ps1)

这些入口属于 agent interface 的一部分，但它们本身不等于 harness engineering。

## Build / Runtime Contracts

项目当前最重要的 contract 已经明确写入 [AGENTS.md](/D:/taptap-pc/nano-installer/AGENTS.md)：

- 修改 `installer/**` 的运行时或 stub 源码
  - 必须先重编 `nano-installer-lzma`、`nano-installer-zlib`、`uninst`、`nano-installer-cli`
  - 然后再重打 setup
- 只修改 `examples/**`
  - 只需要重新 `build --project ...`

这类 contract 才是 harness engineering 的核心组成部分，因为它直接影响 agent 能否稳定交付正确产物。

## Verification Tooling

当前已经落地的验证工具：

- `library snapshot`
  - [test_harness.rs](/D:/taptap-pc/nano-installer/installer/lib/src/ui/test_harness.rs)
- `CLI snapshot`
  - `nano-installer harness snapshot`
- `resource lint`
  - `nano-installer harness lint-resources`
- `script smoke`
  - PowerShell 主链路回归脚本

这些工具的定位是“verification tooling”，不是整个 harness engineering。

## Recommended Terminology

后续文档和沟通统一按下面这套叫法：

- `agent engineering`
  - 面向 AI agent 的整体工程改造
- `agent interfaces`
  - agent 可调用的项目入口
- `build/runtime contracts`
  - 构建与运行时边界规则
- `verification tooling`
  - snapshot、lint、smoke 等验证工具

不要再把 `harness snapshot` 或 `harness lint-resources` 直接叫成 “harness engineering”。
