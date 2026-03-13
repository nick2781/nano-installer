param(
    [Parameter(Mandatory = $true)]
    [string]$SetupExe,
    [Parameter(Mandatory = $true)]
    [string]$Locale,
    [Parameter(Mandatory = $true)]
    [string]$OutputPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class CaptureWin32 {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

    [DllImport("user32.dll")]
    public static extern bool PrintWindow(IntPtr hwnd, IntPtr hDC, uint nFlags);

    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool SetWindowPos(
        IntPtr hWnd,
        IntPtr hWndInsertAfter,
        int X,
        int Y,
        int cx,
        int cy,
        uint uFlags
    );
}
"@

$HWND_TOPMOST = [IntPtr](-1)
$HWND_NOTOPMOST = [IntPtr](-2)
$SWP_NOMOVE = 0x0002
$SWP_NOSIZE = 0x0001
$SWP_SHOWWINDOW = 0x0040
$windowActivator = New-Object -ComObject WScript.Shell
$shellApp = New-Object -ComObject Shell.Application

function Wait-ForWindow {
    param(
        [System.Diagnostics.Process]$Process,
        [int]$TimeoutSeconds = 20
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $Process.Refresh()
        if ($Process.MainWindowHandle -ne 0 -and -not [string]::IsNullOrWhiteSpace($Process.MainWindowTitle)) {
            $rect = New-Object CaptureWin32+RECT
            if ([CaptureWin32]::GetWindowRect($Process.MainWindowHandle, [ref]$rect)) {
                $width = $rect.Right - $rect.Left
                $height = $rect.Bottom - $rect.Top
                if ($width -ge 500 -and $height -ge 300) {
                    return $Process.MainWindowHandle
                }
            }
        }
        Start-Sleep -Milliseconds 250
    }

    throw "Timed out waiting for installer window"
}

function Capture-WindowToFile {
    param(
        [IntPtr]$Handle,
        [string]$Path
    )

    $rect = New-Object CaptureWin32+RECT
    if (-not [CaptureWin32]::GetWindowRect($Handle, [ref]$rect)) {
        throw "GetWindowRect failed"
    }

    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -le 0 -or $height -le 0) {
        throw "Invalid window rect ${width}x${height}"
    }

    $bitmap = New-Object System.Drawing.Bitmap $width, $height
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen(
            $rect.Left,
            $rect.Top,
            0,
            0,
            (New-Object System.Drawing.Size $width, $height)
        )
    } finally {
        $graphics.Dispose()
    }

    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
}

$setupItem = Get-Item $SetupExe
New-Item -ItemType Directory -Force -Path (Split-Path $OutputPath -Parent) | Out-Null

$existing = Get-Process | Where-Object {
    try {
        $_.Path -eq $setupItem.FullName
    } catch {
        $false
    }
}
if ($existing) {
    $existing | Stop-Process -Force
    Start-Sleep -Milliseconds 500
}

$env:NANO_INSTALLER_TEST_LOCALE = $Locale
try {
    $proc = Start-Process -FilePath $setupItem.FullName -PassThru
} finally {
    Remove-Item Env:NANO_INSTALLER_TEST_LOCALE -ErrorAction SilentlyContinue
}

try {
    $handle = Wait-ForWindow -Process $proc
    try {
        $shellApp.MinimizeAll() | Out-Null
    } catch {
    }
    Start-Sleep -Milliseconds 300
    [void][CaptureWin32]::ShowWindow($handle, 9)
    [void][CaptureWin32]::SetWindowPos($handle, $HWND_TOPMOST, 0, 0, 0, 0, $SWP_NOMOVE -bor $SWP_NOSIZE -bor $SWP_SHOWWINDOW)
    [void][CaptureWin32]::SetForegroundWindow($handle)
    try {
        [void]$windowActivator.AppActivate($proc.Id)
    } catch {
    }
    Start-Sleep -Milliseconds 800
    Capture-WindowToFile -Handle $handle -Path $OutputPath
    [void][CaptureWin32]::SetWindowPos($handle, $HWND_NOTOPMOST, 0, 0, 0, 0, $SWP_NOMOVE -bor $SWP_NOSIZE -bor $SWP_SHOWWINDOW)
} finally {
    if ($proc -and -not $proc.HasExited) {
        Stop-Process -Id $proc.Id -Force
    }
}
