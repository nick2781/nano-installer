# DPI 检测不一致问题 - 通俗解释

## 问题：为什么同一个函数返回不同的值？

你可能会想：`GetDpiForSystem()` 应该是"获取系统DPI"，系统DPI是固定的，为什么两次调用会返回不同的值？

**答案是：这个函数的名字有误导性！它实际上返回的是"当前上下文下的DPI"，而不是"系统DPI"。**

## 什么是 DPI 感知模式？

想象一下，Windows 有两种"看世界"的方式：

### 方式1：DPI 未感知（UNAWARE）
- 程序认为：所有显示器都是 96 DPI（标准）
- 系统说：你的显示器是 192 DPI（高DPI）
- 程序说：我不信，我就按 96 DPI 来
- **结果**：`GetDpiForSystem()` 返回 96（程序认为的值）

### 方式2：DPI 感知（AWARE）
- 程序认为：显示器可能是 96、144、192 等不同 DPI
- 系统说：你的显示器是 192 DPI
- 程序说：好的，我按 192 DPI 来
- **结果**：`GetDpiForSystem()` 返回 192（系统实际的值）

## 为什么窗口创建前后会变化？

### 场景重现：

```rust
// 时刻1：窗口创建前
fn run_gui_install() {
    // 此时程序可能还是"未感知"模式
    let dpi1 = GetDpiForSystem();  // 返回 96（程序认为的值）
    
    // 创建窗口
    eframe::run_native(...);  // ← 这里发生了什么？
    
    // 时刻2：窗口创建后（在回调中）
    fn callback() {
        // 此时程序可能已经变成"感知"模式了
        let dpi2 = GetDpiForSystem();  // 返回 192（系统实际的值）
    }
}
```

### eframe 在创建窗口时做了什么？

当 `eframe::run_native` 被调用时，它内部可能：

1. **设置 DPI 感知模式**：
   ```rust
   // eframe 内部可能做了类似这样的事：
   SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE);
   ```

2. **创建窗口**：
   - Windows 在创建窗口时，可能会根据窗口的属性自动设置 DPI 感知模式
   - 如果窗口声明支持高DPI，Windows 会自动切换到"感知"模式

3. **结果**：
   - 窗口创建前：程序是"未感知"模式 → `GetDpiForSystem()` 返回 96
   - 窗口创建后：程序变成"感知"模式 → `GetDpiForSystem()` 返回 192

## 用代码验证

让我们添加代码来验证这个假设：

```rust
fn detect_system_dpi() -> u32 {
    unsafe {
        let dpi = GetDpiForSystem() as u32;
        
        // 检查当前线程的 DPI 感知上下文
        let context = GetThreadDpiAwarenessContext();
        let is_unaware = AreDpiAwarenessContextsEqual(
            context,
            DPI_AWARENESS_CONTEXT_UNAWARE
        );
        
        eprintln!("GetDpiForSystem() = {}, 是否未感知 = {}", dpi, is_unaware);
        
        dpi
    }
}
```

运行后，你会看到：
- 第一次：`GetDpiForSystem() = 96, 是否未感知 = true`
- 第二次：`GetDpiForSystem() = 192, 是否未感知 = false`

## 为什么 NSIS 不会有这个问题？

NSIS 在脚本开始时就统一设置了 DPI 感知模式：

```nsis
; NSIS 脚本开始时就设置
!ifdef NSIS_UNICODE
    System::Call 'user32::SetProcessDpiAwareness(i 2)' ; PROCESS_PER_MONITOR_DPI_AWARE
!endif
```

所以整个安装程序运行期间，DPI 感知模式都是一致的，`GetDpiForSystem()` 始终返回相同的值。

## 解决方案

既然知道了原因，解决方案就很简单：

**在窗口创建前统一检测一次，然后传递结果，避免重复检测。**

这样无论窗口创建后 DPI 感知模式如何变化，我们使用的都是窗口创建前的检测结果，确保窗口大小和 App 配置一致。

## 总结

- `GetDpiForSystem()` 的名字有误导性，它返回的是"当前上下文下的DPI"
- Windows 的 DPI 感知模式会影响这个函数的返回值
- 窗口创建前后，程序的 DPI 感知模式可能发生变化
- 解决方案：统一检测一次，避免重复检测

