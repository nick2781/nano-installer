param(
    [string]$SetupExe,
    [string]$OutputDir,
    [ValidateSet("close_confirm_locale", "legacy_click_flow")]
    [string]$Scenario = "close_confirm_locale",
    [string]$TestLocale = "ru"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class Win32Gui {
    [StructLayout(LayoutKind.Sequential)]
    public struct POINT {
        public int X;
        public int Y;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

    [DllImport("user32.dll")]
    public static extern bool GetClientRect(IntPtr hWnd, out RECT rect);

    [DllImport("user32.dll")]
    public static extern bool ClientToScreen(IntPtr hWnd, ref POINT point);

    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int X, int Y);

    [DllImport("user32.dll")]
    public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);

    [DllImport("user32.dll")]
    public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, UIntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);
}
"@

$MouseLeftDown = 0x0002
$MouseLeftUp = 0x0004
$WM_MOUSEMOVE = 0x0200
$WM_LBUTTONDOWN = 0x0201
$WM_LBUTTONUP = 0x0202
$script:WindowActivator = New-Object -ComObject WScript.Shell
$script:ForegroundProcessId = $null

function Get-LatestGuiInstaller {
    $tmpRoot = Join-Path $PSScriptRoot "..\tmp"
    $manifest = Get-ChildItem -Path $tmpRoot -Recurse -File -Filter "TapTapManifestSmoke_Setup.exe" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if ($manifest) {
        return $manifest.FullName
    }

    $latest = Get-ChildItem -Path $tmpRoot -Recurse -File -Filter "*_Setup.exe" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $latest) {
        throw "No setup executable found under tmp/"
    }
    return $latest.FullName
}

function Wait-ForWindow {
    param(
        [System.Diagnostics.Process]$Process,
        [int]$TimeoutSeconds = 20
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $Process.Refresh()
        if ($Process.MainWindowHandle -ne 0 -and -not [string]::IsNullOrWhiteSpace($Process.MainWindowTitle)) {
            return $Process.MainWindowHandle
        }
        Start-Sleep -Milliseconds 250
    }

    throw "Timed out waiting for a window from process $($Process.Id)"
}

function Restore-Window {
    param(
        [IntPtr]$Handle,
        [int]$TimeoutSeconds = 10
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        [void][Win32Gui]::ShowWindow($Handle, 1)
        [void][Win32Gui]::ShowWindow($Handle, 9)
        [void][Win32Gui]::SetForegroundWindow($Handle)
        Start-Sleep -Milliseconds 200
        try {
            $rect = Get-WindowRect -Handle $Handle
            if ($rect.Width -ge 500 -and $rect.Height -ge 300 -and $rect.Left -gt -5000) {
                return $rect
            }
        } catch {
        }
    }

    throw "Failed to restore a usable installer window"
}

function Get-WindowRect {
    param([IntPtr]$Handle)

    $rect = New-Object Win32Gui+RECT
    if (-not [Win32Gui]::GetWindowRect($Handle, [ref]$rect)) {
        throw "Failed to read window rect"
    }

    [pscustomobject]@{
        Left   = $rect.Left
        Top    = $rect.Top
        Right  = $rect.Right
        Bottom = $rect.Bottom
        Width  = $rect.Right - $rect.Left
        Height = $rect.Bottom - $rect.Top
    }
}

function Get-ClientArea {
    param([IntPtr]$Handle)

    $rect = New-Object Win32Gui+RECT
    if (-not [Win32Gui]::GetClientRect($Handle, [ref]$rect)) {
        throw "Failed to read client rect"
    }

    $point = New-Object Win32Gui+POINT
    $point.X = 0
    $point.Y = 0
    if (-not [Win32Gui]::ClientToScreen($Handle, [ref]$point)) {
        throw "Failed to translate client origin to screen coordinates"
    }

    [pscustomobject]@{
        Left   = $point.X
        Top    = $point.Y
        Width  = $rect.Right - $rect.Left
        Height = $rect.Bottom - $rect.Top
    }
}

function Capture-Window {
    param(
        [IntPtr]$Handle,
        [string]$Path
    )

    if ($script:ForegroundProcessId) {
        try {
            [void]$script:WindowActivator.AppActivate([int]$script:ForegroundProcessId)
        } catch {
        }
    }
    [void][Win32Gui]::ShowWindow($Handle, 9)
    [void][Win32Gui]::SetForegroundWindow($Handle)
    Start-Sleep -Milliseconds 250

    $rect = Get-WindowRect -Handle $Handle
    $bitmap = New-Object System.Drawing.Bitmap $rect.Width, $rect.Height
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen(
            $rect.Left,
            $rect.Top,
            0,
            0,
            (New-Object System.Drawing.Size $rect.Width, $rect.Height)
        )
    } finally {
    }
    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
    return $rect
}

function Click-Window {
    param(
        [IntPtr]$Handle,
        [int]$RelativeX,
        [int]$RelativeY
    )

    $lParam = [IntPtr](($RelativeY -shl 16) -bor ($RelativeX -band 0xFFFF))
    [void][Win32Gui]::SendMessage($Handle, $WM_MOUSEMOVE, [UIntPtr]::Zero, $lParam)
    [void][Win32Gui]::SendMessage($Handle, $WM_LBUTTONDOWN, [UIntPtr]::op_Explicit(1), $lParam)
    Start-Sleep -Milliseconds 50
    [void][Win32Gui]::SendMessage($Handle, $WM_LBUTTONUP, [UIntPtr]::Zero, $lParam)
    Start-Sleep -Milliseconds 350
}

function Wait-ForPath {
    param(
        [string]$Path,
        [int]$TimeoutSeconds = 30
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        if (Test-Path $Path) {
            return $true
        }
        Start-Sleep -Milliseconds 250
    }
    return $false
}

if (-not $SetupExe) {
    $SetupExe = Get-LatestGuiInstaller
}

$setupItem = Get-Item $SetupExe
$existingSetup = Get-Process | Where-Object {
    try {
        $_.Path -eq $setupItem.FullName
    } catch {
        $false
    }
}
if ($existingSetup) {
    $existingSetup | Stop-Process -Force
    Start-Sleep -Milliseconds 500
}

$projectRoot = Split-Path (Split-Path $setupItem.FullName -Parent) -Parent
$config = Get-Content (Join-Path $projectRoot "installer_config.json") -Raw | ConvertFrom-Json
$installRoot = $config.install.default_path

if (-not $OutputDir) {
    $stamp = Get-Date -Format "yyyyMMdd_HHmmss"
    $OutputDir = Join-Path $PSScriptRoot "..\tmp\gui_real_$stamp"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if (Test-Path $installRoot) {
    Remove-Item -Recurse -Force $installRoot
}

if ($Scenario -eq "close_confirm_locale") {
    $env:NANO_INSTALLER_TEST_LOCALE = $TestLocale
    $env:NANO_INSTALLER_TEST_ACTION = "close_confirm"
}

$installer = Start-Process -FilePath $setupItem.FullName -PassThru

if ($Scenario -eq "close_confirm_locale") {
    Remove-Item Env:NANO_INSTALLER_TEST_LOCALE -ErrorAction SilentlyContinue
    Remove-Item Env:NANO_INSTALLER_TEST_ACTION -ErrorAction SilentlyContinue
}

$script:ForegroundProcessId = $installer.Id
$installerHandle = Wait-ForWindow -Process $installer
Start-Sleep -Seconds 1
$installerRect = Restore-Window -Handle $installerHandle
Write-Host "[gui] Installer window: $($installerRect.Width)x$($installerRect.Height) at ($($installerRect.Left), $($installerRect.Top))"

if ($Scenario -eq "close_confirm_locale") {
    Start-Sleep -Seconds 2
    $msgboxShot = Join-Path $OutputDir "01_close_confirm_$TestLocale.png"
    Capture-Window -Handle $installerHandle -Path $msgboxShot | Out-Null
    if (-not $installer.HasExited) {
        Stop-Process -Id $installer.Id -Force
    }

    [pscustomobject]@{
        setup_exe = $setupItem.FullName
        scenario = $Scenario
        locale = $TestLocale
        output_dir = (Resolve-Path $OutputDir).Path
        screenshot = $msgboxShot
    } | ConvertTo-Json -Depth 4
    return
}

$initialShot = Join-Path $OutputDir "01_config_initial.png"
Capture-Window -Handle $installerHandle -Path $initialShot | Out-Null

# Switch to Russian from the language selector.
Click-Window -Handle $installerHandle -RelativeX 462 -RelativeY 20
$dropdownShot = Join-Path $OutputDir "02_language_dropdown.png"
Capture-Window -Handle $installerHandle -Path $dropdownShot | Out-Null
Click-Window -Handle $installerHandle -RelativeX 462 -RelativeY 283
Start-Sleep -Milliseconds 500
$russianShot = Join-Path $OutputDir "03_config_russian.png"
Capture-Window -Handle $installerHandle -Path $russianShot | Out-Null

# Open close-confirm dialog after language switch.
Click-Window -Handle $installerHandle -RelativeX 552 -RelativeY 20
Start-Sleep -Milliseconds 500
$msgboxShot = Join-Path $OutputDir "04_close_confirm_russian.png"
Capture-Window -Handle $installerHandle -Path $msgboxShot | Out-Null

# Dismiss dialog and continue installation.
Click-Window -Handle $installerHandle -RelativeX 200 -RelativeY 285
Click-Window -Handle $installerHandle -RelativeX 34 -RelativeY 308
Click-Window -Handle $installerHandle -RelativeX 287 -RelativeY 246

$installOk = Wait-ForPath -Path (Join-Path $installRoot "uninst.exe") -TimeoutSeconds 90
if (-not $installOk) {
    throw "GUI install did not produce uninst.exe under $installRoot"
}

Start-Sleep -Seconds 2
$finishShot = Join-Path $OutputDir "05_install_finish.png"
Capture-Window -Handle $installerHandle -Path $finishShot | Out-Null

if (-not $installer.HasExited) {
    Stop-Process -Id $installer.Id -Force
}

$uninstallerExe = Join-Path $installRoot "uninst.exe"
if (-not (Test-Path $uninstallerExe)) {
    throw "Installed uninstaller not found at $uninstallerExe"
}

$uninstaller = Start-Process -FilePath $uninstallerExe -PassThru
$script:ForegroundProcessId = $uninstaller.Id
$uninstallerHandle = Wait-ForWindow -Process $uninstaller
Start-Sleep -Seconds 1
$uninstallerRect = Restore-Window -Handle $uninstallerHandle
Write-Host "[gui] Uninstaller window: $($uninstallerRect.Width)x$($uninstallerRect.Height) at ($($uninstallerRect.Left), $($uninstallerRect.Top))"

$uninstallConfirmShot = Join-Path $OutputDir "06_uninstall_confirm.png"
Capture-Window -Handle $uninstallerHandle -Path $uninstallConfirmShot | Out-Null

Click-Window -Handle $uninstallerHandle -RelativeX 386 -RelativeY 217
Start-Sleep -Seconds 3
$uninstallFinishShot = Join-Path $OutputDir "07_uninstall_finish.png"
Capture-Window -Handle $uninstallerHandle -Path $uninstallFinishShot | Out-Null

Click-Window -Handle $uninstallerHandle -RelativeX 287 -RelativeY 262
$removed = -not (Wait-ForPath -Path $installRoot -TimeoutSeconds 15)

if (-not $uninstaller.HasExited) {
    Stop-Process -Id $uninstaller.Id -Force
}

[pscustomobject]@{
    setup_exe = $setupItem.FullName
    output_dir = (Resolve-Path $OutputDir).Path
    install_root = $installRoot
    install_completed = $installOk
    uninstall_removed_root = $removed
    screenshots = @(
        $initialShot,
        $dropdownShot,
        $russianShot,
        $msgboxShot,
        $finishShot,
        $uninstallConfirmShot,
        $uninstallFinishShot
    )
} | ConvertTo-Json -Depth 4
