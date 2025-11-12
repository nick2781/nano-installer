# 检查 Windows 系统 DPI 设置
# 使用方法：在 PowerShell 中运行：.\check_dpi.ps1

Write-Host "========================================"
Write-Host "Windows DPI 检测工具"
Write-Host "========================================"
Write-Host ""

# 方法1: 使用 WMI 获取显示器信息
Write-Host "方法1: WMI 检测"
Write-Host "----------------------------------------"
try {
    $monitors = Get-WmiObject -Class Win32_DesktopMonitor -ErrorAction SilentlyContinue
    if ($monitors) {
        foreach ($monitor in $monitors) {
            Write-Host "显示器: $($monitor.Name)"
            Write-Host "  PixelsPerXLogicalInch: $($monitor.PixelsPerXLogicalInch)"
            Write-Host "  PixelsPerYLogicalInch: $($monitor.PixelsPerYLogicalInch)"
        }
    } else {
        Write-Host "无法通过 WMI 获取显示器信息"
    }
} catch {
    Write-Host "WMI 检测失败: $_"
}
Write-Host ""

# 方法2: 使用注册表获取系统 DPI
Write-Host "方法2: 注册表检测"
Write-Host "----------------------------------------"
try {
    $dpiValue = Get-ItemProperty -Path "HKCU:\Control Panel\Desktop" -Name "LogPixels" -ErrorAction SilentlyContinue
    if ($dpiValue) {
        $dpi = $dpiValue.LogPixels
        Write-Host "注册表 LogPixels: $dpi"
        Write-Host "  对应缩放比例: $([math]::Round($dpi / 96, 2))x"
        
        # 常见的 DPI 值
        $dpiMap = @{
            96 = "100% (96 DPI)"
            120 = "125% (120 DPI)"
            144 = "150% (144 DPI)"
            192 = "200% (192 DPI)"
            240 = "250% (240 DPI)"
            288 = "300% (288 DPI)"
        }
        
        if ($dpiMap.ContainsKey($dpi)) {
            Write-Host "  系统显示: $($dpiMap[$dpi])"
        }
    } else {
        Write-Host "无法从注册表获取 DPI 信息"
    }
} catch {
    Write-Host "注册表检测失败: $_"
}
Write-Host ""

# 方法3: 使用 .NET API (类似我们的代码)
Write-Host "方法3: .NET Graphics API 检测"
Write-Host "----------------------------------------"
try {
    Add-Type -TypeDefinition @"
        using System;
        using System.Runtime.InteropServices;
        public class DpiHelper {
            [DllImport("user32.dll")]
            public static extern IntPtr GetDC(IntPtr hWnd);
            
            [DllImport("gdi32.dll")]
            public static extern int GetDeviceCaps(IntPtr hdc, int nIndex);
            
            [DllImport("user32.dll")]
            public static extern bool ReleaseDC(IntPtr hWnd, IntPtr hDC);
            
            public static int GetDpi() {
                IntPtr hdc = GetDC(IntPtr.Zero);
                int dpi = GetDeviceCaps(hdc, 88); // LOGPIXELSX = 88
                ReleaseDC(IntPtr.Zero, hdc);
                return dpi;
            }
        }
"@
    $dpi = [DpiHelper]::GetDpi()
    Write-Host "GetDeviceCaps LOGPIXELSX: $dpi"
    Write-Host "  对应缩放比例: $([math]::Round($dpi / 96, 2))x"
} catch {
    Write-Host ".NET API 检测失败: $_"
}
Write-Host ""

# 方法4: 使用 Windows API (GetDpiForSystem，类似我们的代码)
Write-Host "方法4: GetDpiForSystem API 检测 (与我们的代码一致)"
Write-Host "----------------------------------------"
try {
    Add-Type -TypeDefinition @"
        using System;
        using System.Runtime.InteropServices;
        public class DpiSystemHelper {
            [DllImport("user32.dll")]
            public static extern uint GetDpiForSystem();
            
            public static uint GetSystemDpi() {
                return GetDpiForSystem();
            }
        }
"@
    $dpi = [DpiSystemHelper]::GetSystemDpi()
    Write-Host "GetDpiForSystem: $dpi"
    Write-Host "  对应缩放比例: $([math]::Round($dpi / 96, 2))x"
    Write-Host ""
    Write-Host "✓ 这是我们的代码使用的 API，应该与安装器输出一致"
} catch {
    Write-Host "GetDpiForSystem API 检测失败: $_"
}
Write-Host ""

# 方法5: 显示系统设置中的缩放比例
Write-Host "方法5: 系统显示设置"
Write-Host "----------------------------------------"
Write-Host "请在以下位置查看系统缩放设置："
Write-Host "  设置 -> 系统 -> 显示 -> 缩放与布局"
Write-Host "  或"
Write-Host "  控制面板 -> 显示 -> 更改文本、应用和其他项目的大小"
Write-Host ""
Write-Host "常见对应关系："
Write-Host "  100% = 96 DPI"
Write-Host "  125% = 120 DPI"
Write-Host "  150% = 144 DPI  (我们的阈值)"
Write-Host "  200% = 192 DPI"
Write-Host "  250% = 240 DPI"
Write-Host "  300% = 288 DPI"
Write-Host ""

Write-Host "========================================"
Write-Host "验证方法："
Write-Host "1. 运行安装程序，查看输出的 DPI 值"
Write-Host "2. 运行此脚本，查看 GetDpiForSystem 的值"
Write-Host "3. 两者应该完全一致"
Write-Host "4. 如果系统缩放 >= 150%，应该使用 2x 布局"
Write-Host "========================================"

