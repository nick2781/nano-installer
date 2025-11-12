# DPI 检测不一致问题说明

## 问题现象

从日志可以看到：
- **第一次检测**（窗口创建前）：系统 DPI = 96，use_2x = false，窗口大小 = 574x358
- **第二次检测**（App 创建时）：系统 DPI = 192，use_2x = true，窗口大小 = 1148x716

**结果**：窗口大小和布局/资源选择不匹配，导致闪烁。

## 根本原因

### Windows DPI 感知上下文的影响

`GetDpiForSystem()` 的返回值**不是固定的**，它受到以下因素影响：

1. **线程的 DPI 感知上下文**
   - Windows 允许在同一进程中为不同的线程设置不同的 DPI 感知上下文
   - 使用 `SetThreadDpiAwarenessContext` 可以在单个进程中设置不同的 DPI 感知模式
   - 如果线程的 DPI 感知上下文不同，`GetDpiForSystem()` 可能返回不同的值

2. **进程的 DPI 感知模式**
   - 进程级别的 DPI 感知模式（`SetProcessDpiAwareness`）
   - 在窗口创建前后，进程的 DPI 感知模式可能发生变化

3. **eframe/egui 的初始化**
   - 当 `eframe::run_native` 被调用时，它可能会：
     - 设置线程的 DPI 感知上下文
     - 创建窗口，触发 Windows 的 DPI 感知模式设置
     - 这些操作可能改变后续 `GetDpiForSystem()` 的返回值

### 具体场景

```
时间线：
1. run_gui_install() 调用
   └─ DpiConfig::new() → GetDpiForSystem() → 返回 96 DPI
      └─ 此时线程可能处于 UNAWARE 模式

2. eframe::run_native() 调用
   └─ eframe 内部可能设置 DPI 感知模式
   └─ 窗口创建，Windows 可能改变线程的 DPI 感知上下文

3. InstallerApp::new() 调用（在 eframe 的回调中）
   └─ DpiConfig::new() → GetDpiForSystem() → 返回 192 DPI
      └─ 此时线程可能处于 AWARE 模式
```

## 解决方案

### ✅ 已实施的方案：统一检测一次

**修改内容**：
1. 在 `run_gui_install` 中只检测一次 DPI
2. 将检测结果传递给 `InstallerApp::new_with_dpi`
3. App 不再重复检测，直接使用传入的配置

**优点**：
- 确保窗口大小和 App 的 DPI 配置一致
- 避免重复检测导致的不一致
- 简单可靠

### 其他可选方案（未实施）

#### 方案 2：设置统一的 DPI 感知上下文

在应用启动时，显式设置线程的 DPI 感知上下文：

```rust
use windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext;
use windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2;

unsafe {
    SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
}
```

**缺点**：
- 可能与 eframe 的内部设置冲突
- 需要确保在所有检测之前设置

#### 方案 3：使用窗口的 DPI 而不是系统 DPI

在窗口创建后，使用窗口的 DPI：

```rust
use windows::Win32::UI::HiDpi::GetDpiForWindow;

let hwnd = /* 获取窗口句柄 */;
let dpi = unsafe { GetDpiForWindow(hwnd) };
```

**缺点**：
- 需要在窗口创建后才能获取
- 窗口大小已经在创建时设置了，无法更改

## 验证方法

运行安装程序后，查看日志：

```
[DPI] GetDpiForSystem() 返回: 96
[DPI] 当前线程 DPI 感知上下文: UNAWARE (UNAWARE=true)
[DPI] 说明: 如果窗口创建前后 DPI 感知上下文不同，GetDpiForSystem() 可能返回不同值
```

如果两次检测的 DPI 感知上下文不同，就能确认问题原因。

## 参考文档

- [Windows High DPI Desktop Application Development](https://learn.microsoft.com/en-us/windows/win32/hidpi/high-dpi-desktop-application-development-on-windows)
- [High DPI Improvements for Desktop Applications](https://learn.microsoft.com/en-us/windows/win32/hidpi/high-dpi-improvements-for-desktop-applications)
- [GetDpiForSystem function](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdpiforsystem)

