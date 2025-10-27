# GPUI 与 Windows 7 兼容性说明

## ⚠️ 重要提示

在继续使用 GPUI 实现 UI 之前，需要了解以下兼容性问题：

## GPUI 简介

GPUI 是 Zed 编辑器开发的现代 GPU 加速 UI 框架，特点：
- 基于 GPU 渲染（Metal/DirectX/Vulkan）
- 高性能、响应式
- 数据驱动的声明式 API
- **主要针对现代系统**

## gpui-component 组件库

✅ **好消息**：项目将使用 [gpui-component](https://github.com/longbridge/gpui-component) 组件库！

**优势**：
- ✅ 提供现成的 UI 组件（Button、Input、Checkbox、ProgressBar 等）
- ✅ 避免手动绘制复杂 UI
- ✅ 开发速度快 3-5 倍
- ✅ 组件经过测试和优化
- ✅ 统一的样式系统

**但是**：gpui-component 基于 GPUI，所以 Windows 7 兼容性问题依然存在。

## Windows 7 SP1 的限制

### 1. 图形 API 支持

Windows 7 默认支持：
- ✅ DirectX 11
- ✅ OpenGL 4.x（取决于驱动）
- ❌ DirectX 12（不支持）
- ⚠️ Vulkan（需要额外驱动，老硬件可能不支持）

### 2. GPUI 的要求

GPUI 在 Windows 上使用：
- DirectX 11+ 或 Vulkan
- 现代 GPU 驱动
- Windows 8+ 优化

**潜在问题**：
- GPUI 主要在 macOS (Metal) 和现代 Windows 上测试
- Windows 7 支持可能不完整
- 老旧硬件可能无法运行

### 3. 实际风险

如果用户系统：
- ✅ Windows 7 + 现代 GPU + 最新驱动 → 可能可以运行
- ⚠️ Windows 7 + 老旧 GPU → 可能黑屏/崩溃
- ⚠️ Windows 7 未安装 Platform Update → DirectX 11.1 特性缺失

## 建议的解决方案

### 方案 A：GPUI + gpui-component（当前选择）

✅ **已选择此方案**：使用 gpui-component 组件库

**优点**：
- ✅ 现代化的架构
- ✅ GPU 加速，性能好
- ✅ 现成组件，开发快
- ✅ 避免手动绘制 UI
- ✅ 与 Zed 生态一致

**缺点**：
- ⚠️ Windows 7 兼容性未知（需要测试）
- ⚠️ 可能需要降级方案
- ⚠️ 老旧硬件可能不支持

**风险缓解**：
1. 优先在 Windows 10+ 上开发测试
2. 准备 Windows 7 测试环境
3. 如果不兼容，快速切换到 egui（1-2 天）

**适用场景**：
- 主要用户在 Windows 10+
- 可以要求用户更新显卡驱动
- 愿意承担一定风险

### 方案 B：使用更兼容的 Rust GUI 框架（推荐）

#### 1. **egui**（推荐度：⭐⭐⭐⭐⭐）

```toml
[dependencies]
eframe = "0.24"  # egui 的应用框架
```

**优点**：
- ✅ 轻量级（~100KB）
- ✅ 纯 Rust，无 C++ 依赖
- ✅ 支持 Windows 7
- ✅ 即时模式 GUI，简单易用
- ✅ 成熟稳定

**缺点**：
- 样式自定义相对简单
- 不如 GPUI 现代

**代码示例**：
```rust
eframe::run_native(
    "Installer",
    eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(574.0, 358.0)),
        resizable: false,
        ..Default::default()
    },
    Box::new(|_cc| Box::new(MyApp::default())),
);
```

#### 2. **slint**（推荐度：⭐⭐⭐⭐）

```toml
[dependencies]
slint = "1.3"
```

**优点**：
- ✅ 声明式 UI（类似 QML）
- ✅ 支持 Windows 7
- ✅ 设计师友好
- ✅ 性能好

**缺点**：
- 学习曲线
- 需要 .slint 文件

#### 3. **iced**（推荐度：⭐⭐⭐）

```toml
[dependencies]
iced = "0.10"
```

**优点**：
- ✅ Elm 架构，清晰
- ✅ 纯 Rust
- ✅ 响应式

**缺点**：
- 相对新，生态小
- 样式系统还在完善

#### 4. **native-windows-gui**（推荐度：⭐⭐⭐）

```toml
[dependencies]
native-windows-gui = "1.0"
```

**优点**：
- ✅ 原生 Windows 控件
- ✅ 100% Windows 7 兼容
- ✅ 小巧

**缺点**：
- 只支持 Windows
- 外观老旧

### 方案 C：混合方案

保留 GPUI 架构，但：
- 检测系统是否支持 GPUI
- 不支持时回退到 egui 或原生控件
- 提供两套 UI 实现

## 推荐行动计划

### 立即行动（第 1 天）

1. **快速测试 GPUI**
   ```bash
   # 在 Windows 7 虚拟机上测试
   cargo run --bin installer
   ```

2. **如果失败，立即切换到 egui**
   - egui 最简单，学习成本低
   - 1-2 天即可实现完整 UI
   - 兼容性最好

### 中期（第 2-3 天）

实现基础 UI（无论用哪个框架）：
- 配置页
- 进度页
- 完成页

### 长期

如果 GPUI 确认可用：
- 继续完善
- 提供详细的系统要求说明

如果 GPUI 不可用：
- 使用 egui 完成项目
- 保留架构清晰，未来可切换

## 我的建议

**强烈建议先测试 GPUI 是否能在 Windows 7 上运行。**

如果不能，我推荐：
1. **首选 egui** - 最简单、最稳定
2. **备选 slint** - 如果需要更好的设计支持

原因：
- ✅ 兼容性保证
- ✅ 开发速度快
- ✅ 文档齐全
- ✅ 社区活跃

GPUI 虽然现代，但：
- ❌ Windows 7 支持未知
- ❌ 学习成本高
- ❌ 可能需要大量适配

## 决策矩阵

| 框架               | Windows 7 | 开发速度 | 外观现代化 | 稳定性   | 推荐度 |
| ------------------ | --------- | -------- | ---------- | -------- | ------ |
| GPUI               | ❓未知     | 慢       | ⭐⭐⭐⭐⭐      | ⚠️ 不确定 | ⭐⭐     |
| egui               | ✅ 支持    | 快       | ⭐⭐⭐        | ⭐⭐⭐⭐⭐    | ⭐⭐⭐⭐⭐  |
| slint              | ✅ 支持    | 中等     | ⭐⭐⭐⭐       | ⭐⭐⭐⭐     | ⭐⭐⭐⭐   |
| iced               | ✅ 支持    | 中等     | ⭐⭐⭐⭐       | ⭐⭐⭐      | ⭐⭐⭐    |
| native-windows-gui | ✅ 完美    | 快       | ⭐          | ⭐⭐⭐⭐⭐    | ⭐⭐⭐    |

## 当前状态

✅ **已选择方案 A**：使用 GPUI + gpui-component

**已完成**：
1. ✅ 添加 gpui-component 依赖到 Cargo.toml
2. ✅ 从 NSIS 提取设计规格
3. ✅ 复制图片资源到 assets/
4. ✅ 创建样式定义（颜色、尺寸等）
5. ✅ 创建实现指南文档

**下一步**：
1. 实现页面组件（使用 gpui-component）
   - 配置页（带展开选项）
   - 安装进度页
   - 完成页
2. 集成安装逻辑
3. 测试（Windows 10+ 优先）
4. Windows 7 兼容性测试

**备用方案**：
如果 Windows 7 测试失败，可以在 1-2 天内切换到 egui。

## 参考文档

- **实现指南**：`docs/GPUI_COMPONENTS_GUIDE.md` ⭐
- **设计规格**：`docs/UI_DESIGN.md`
- **样式定义**：`src/ui/styles/mod.rs`

**让我们开始实现吧！** 🚀

