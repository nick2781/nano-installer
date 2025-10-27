# ⚠️ Rust 版本要求

## 关键信息

**必须使用 Rust 1.75.0**

这是 **唯一正确** 的版本选择！

## 为什么是 1.75？

### Windows 7 支持的关键

根据 Rust 官方公告：

| Rust 版本 | Windows 7 支持 | 说明 |
|-----------|---------------|------|
| **Rust 1.75** | ✅ **支持** | **最后一个支持 Windows 7 的版本** |
| Rust 1.76+ | ❌ **不支持** | 最低要求 Windows 10 |

### 官方时间线

- **2023年12月28日**：Rust 1.75 发布，这是最后一个支持 Windows 7/8/8.1 的版本
- **2024年2月8日**：Rust 1.76 发布，最低要求提升到 Windows 10

## ⚠️ 重要警告

### 不要升级 Rust 版本！

```bash
# ❌ 危险！会导致无法在 Windows 7 上运行
rustup update

# ✅ 正确！固定使用 1.75
rustup install 1.75.0
rustup default 1.75.0
```

### 如果不小心升级了

```bash
# 回退到 1.75
rustup install 1.75.0
rustup override set 1.75.0

# 验证版本
rustc --version
# 应该显示: rustc 1.75.0 (...)
```

## 安装 Rust 1.75

### 新安装

```bash
# 安装 rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 1.75
rustup install 1.75.0
rustup default 1.75.0
```

### Windows 安装

```powershell
# 下载 rustup-init.exe
# https://rustup.rs/

# 安装后
rustup install 1.75.0
rustup default 1.75.0
```

### 验证安装

```bash
rustc --version
# 输出: rustc 1.75.0 (82e1608df 2023-12-21)

cargo --version
# 输出: cargo 1.75.0 (1d8b05cdd 2023-11-20)
```

## Cargo.toml 配置

项目已配置 `rust-version`：

```toml
[package]
rust-version = "1.75.0"
```

这会确保：
- ✅ 如果 Rust 版本 < 1.75，构建会失败并提示
- ✅ 如果 Rust 版本 >= 1.76，会警告版本不匹配
- ✅ 团队成员使用统一版本

## 依赖兼容性

所有依赖都兼容 Rust 1.75：

| 依赖 | 版本 | 兼容性 |
|------|------|--------|
| gpui | latest | ✅ 兼容 1.75 |
| tokio | 1.x | ✅ 兼容 1.75 |
| serde | 1.x | ✅ 兼容 1.75 |
| windows | 0.52 | ✅ 兼容 1.75 |

## Windows 7 测试环境

### 系统要求

- Windows 7 SP1 64-bit
- .NET Framework 4.5+（某些依赖需要）
- Visual C++ Redistributable

### 测试流程

1. **编译**（在开发机上，使用 Rust 1.75）
   ```bash
   cargo build --release --target x86_64-pc-windows-msvc
   ```

2. **复制到 Windows 7 测试机**
   ```bash
   # 复制 target/release/installer.exe 到测试机
   ```

3. **测试**
   ```powershell
   # 在 Windows 7 上运行
   .\installer.exe /S /D=C:\TestApp
   ```

## 常见问题

### Q: 为什么不能用更新的 Rust 版本？

A: Rust 1.76+ 使用了 Windows 10+ 的新 API，生成的可执行文件无法在 Windows 7 上运行。即使编译通过，运行时会立即崩溃。

### Q: Rust 1.75 安全吗？

A: 是的。Rust 1.75 是 2023年12月发布的稳定版本，包含所有必要的安全修复。虽然不是最新版本，但对于支持 Windows 7 是唯一选择。

### Q: 未来怎么办？

A: 如果要放弃 Windows 7 支持，可以：
1. 更新 `rust-version = "1.76"`（或更新版本）
2. 更新文档，移除 Windows 7 支持
3. 通知用户升级到 Windows 10+

### Q: 可以同时支持 Windows 7 和使用新 Rust 吗？

A: 不行。这是不可能的。必须选择：
- **Rust 1.75 + Windows 7 支持**
- **Rust 1.76+ + 放弃 Windows 7**

### Q: GPUI 在 Rust 1.75 上工作正常吗？

A: GPUI 本身支持 Rust 1.75，但 GPUI 在 Windows 7 上的兼容性是另一个问题（参见 `GPUI_COMPATIBILITY.md`）。即使使用 Rust 1.75，GPUI 可能仍然需要 Windows 10 的某些 GPU/DirectX 特性。

## CI/CD 配置

### GitHub Actions 示例

```yaml
name: Build

on: [push, pull_request]

jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust 1.75
        uses: actions-rs/toolchain@v1
        with:
          toolchain: 1.75.0
          override: true
          
      - name: Build
        run: cargo build --release
        
      - name: Test
        run: cargo test
```

### 本地 rust-toolchain.toml

可以创建 `rust-toolchain.toml`：

```toml
[toolchain]
channel = "1.75.0"
```

这会自动使用 Rust 1.75，无需手动切换。

## 版本锁定策略

### 开发环境

```bash
# 在项目根目录
rustup override set 1.75.0

# 验证
rustup show
# 应该显示: active toolchain: 1.75.0-...
```

### 团队协作

在项目 README 中明确说明：

```markdown
## 开发环境要求

- **Rust 1.75.0**（必须！不要使用其他版本）
- Visual Studio Build Tools（Windows）

安装：
\`\`\`bash
rustup install 1.75.0
cd /path/to/nano-installer
rustup override set 1.75.0
\`\`\`
```

## 总结

✅ **DO（必须做）**：
- 使用 Rust 1.75.0
- 在 Windows 7 上测试
- 锁定工具链版本
- 文档中明确说明

❌ **DON'T（禁止做）**：
- 升级到 Rust 1.76+
- 使用 `rustup update`
- 假设新版本兼容
- 在 Windows 10 测试后认为够了

---

**关键点**：Rust 1.75 是支持 Windows 7 的唯一正确选择，这是不可妥协的技术限制。

