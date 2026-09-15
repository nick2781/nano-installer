# TapTap Dual Example Split Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将当前仓库整理为两个正式样板 `examples/TapTap`（国内）和 `examples/TapTap-Global`（海外），删除旧样板与 `TapTap-v2`，并补齐 7 类页面。

**Architecture:** 以当前 `TapTap-v2` 作为国内版基线，将其迁入新的 `examples/TapTap`；再复制一套为 `examples/TapTap-Global`，分别独立维护 `layouts/assets/locales/scripts/config`。example 层不共享布局逻辑，只共享 installer runtime。

**Tech Stack:** Rust, egui/eframe, XML layout DSL, Rhai scripts, Figma MCP, docsify docs, GitHub Actions

---

### Task 1: 盘点并冻结迁移范围

**Files:**
- Modify: `docs/superpowers/specs/2026-03-20-taptap-dual-example-design.md`
- Create: `docs/superpowers/plans/2026-03-20-taptap-dual-example-split.md`

- [ ] **Step 1: 记录现有 example 目录与页面清单**

Run: `Get-ChildItem examples`
Expected: 同时存在 `TapTap` 与 `TapTap-v2`

- [ ] **Step 2: 记录所有引用 `TapTap-v2` 和旧 `TapTap` 的路径**

Run: `rg -n "TapTap-v2|examples/TapTap-v2|examples/TapTap" README.md docs installer examples .github`
Expected: 输出 docs/tests/CI/example 中的引用清单

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-03-20-taptap-dual-example-design.md docs/superpowers/plans/2026-03-20-taptap-dual-example-split.md
git commit -m "docs(plan): define TapTap domestic/global split"
```

### Task 2: 将 `TapTap-v2` 迁入正式 `TapTap`

**Files:**
- Delete: `examples/TapTap/**`
- Delete: `examples/TapTap-v2/**`
- Create: `examples/TapTap/**`

- [ ] **Step 1: 删除旧 `examples/TapTap`**

Run: remove old directory contents using safe patch or shell removal after verification.
Expected: 旧样板不再保留。

- [ ] **Step 2: 将当前 `examples/TapTap-v2` 复制为新的 `examples/TapTap`**

Expected: 国内版目录成为唯一正式 `TapTap`。

- [ ] **Step 3: 调整国内版配置命名**

Files:
- `examples/TapTap/installer_config.json`
- `examples/TapTap/README.md`

Expected:
- `project.name = "TapTap"`
- 输出文件名从 `TapTapV2_Setup.exe` 变回正式国内版命名

- [ ] **Step 4: 删除 `examples/TapTap-v2`**

Expected: 仓库中不再保留 v2 目录。

- [ ] **Step 5: Commit**

```bash
git add examples/TapTap
git rm -r examples/TapTap-v2
git commit -m "refactor(examples): promote TapTap v2 as domestic example"
```

### Task 3: 新建 `TapTap-Global`

**Files:**
- Create: `examples/TapTap-Global/**`

- [ ] **Step 1: 复制 `examples/TapTap` 到 `examples/TapTap-Global`**

Expected: 海外版拥有完整独立目录。

- [ ] **Step 2: 修改海外版 `installer_config.json`**

Expected:
- `project.name`、`output_name`、`installer_name` 使用全球版命名
- `default_locale = "en-US"`
- `supported_locales` 至少包括 `en-US`、`ru`、`zh-CN`

- [ ] **Step 3: 按 Figma 对齐海外版 7 类页面**

Files:
- `examples/TapTap-Global/layouts/configpage.xml`
- `examples/TapTap-Global/layouts/installingpage.xml`
- `examples/TapTap-Global/layouts/finishpage.xml`
- `examples/TapTap-Global/layouts/uninstallpage.xml`
- `examples/TapTap-Global/layouts/uninstallingpage.xml`
- `examples/TapTap-Global/layouts/uninstallfinishpage.xml`
- `examples/TapTap-Global/layouts/msgBox.xml`

Expected:
- slogan、背景、卸载页、完成页文案与资源按海外版节点替换

- [ ] **Step 4: 补齐海外版资源与 locales**

Files:
- `examples/TapTap-Global/assets/**`
- `examples/TapTap-Global/locales/**`

- [ ] **Step 5: Commit**

```bash
git add examples/TapTap-Global
git commit -m "feat(examples): add TapTap global installer example"
```

### Task 4: 更新 docs / tests / CI / release workflow

**Files:**
- Modify: `README.md`
- Modify: `docs/**`
- Modify: `.github/workflows/*.yml`
- Modify: `installer/**` tests or examples references

- [ ] **Step 1: 将所有 `TapTap-v2` 引用改为新的国内/海外命名**

Expected:
- 国内用 `examples/TapTap`
- 海外用 `examples/TapTap-Global`

- [ ] **Step 2: 将 docs 中“双样板”表述更新为国内/海外样板**

- [ ] **Step 3: 调整 CI / release workflow**

Expected:
- 构建国内版 `examples/TapTap`
- 构建海外版 `examples/TapTap-Global`

- [ ] **Step 4: 调整测试引用**

Expected: 不再引用已删除的旧 `TapTap-v2`。

- [ ] **Step 5: Commit**

```bash
git add README.md docs .github installer
git commit -m "docs(ci): switch to TapTap domestic/global examples"
```

### Task 5: 验证与打包

**Files:**
- Verify only; no source file expected unless fixes needed

- [ ] **Step 1: 如果 `installer/**` 有改动，先重编 release stubs**

Run:
`cargo build --release -p nano-installer-lzma -p uninst -p nano-installer-cli`

- [ ] **Step 2: 构建国内版**

Run:
`cargo run --release -p nano-installer-cli -- build --project examples\\TapTap`

- [ ] **Step 3: 构建海外版**

Run:
`cargo run --release -p nano-installer-cli -- build --project examples\\TapTap-Global`

- [ ] **Step 4: 资源 lint**

Run:
`cargo run -p nano-installer-cli -- harness lint-resources --project examples\\TapTap --format text`

Run:
`cargo run -p nano-installer-cli -- harness lint-resources --project examples\\TapTap-Global --format text`

- [ ] **Step 5: 提交最终验证修正**

```bash
git add .
git commit -m "test(examples): validate TapTap domestic/global builds"
```

