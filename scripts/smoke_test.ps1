Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Convert-ToRegistryProviderPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RegistryPath
    )

    if ($RegistryPath.StartsWith("HKCU\")) {
        return "Registry::HKEY_CURRENT_USER\" + $RegistryPath.Substring(5)
    }
    if ($RegistryPath.StartsWith("HKLM\")) {
        return "Registry::HKEY_LOCAL_MACHINE\" + $RegistryPath.Substring(5)
    }

    throw "Unsupported registry hive in path: $RegistryPath"
}

function Assert-PathExists {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PathValue,
        [Parameter(Mandatory = $true)]
        [string]$Label
    )

    if (-not (Test-Path -LiteralPath $PathValue)) {
        throw "$Label missing: $PathValue"
    }
}

function Assert-PathMissing {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PathValue,
        [Parameter(Mandatory = $true)]
        [string]$Label
    )

    if (Test-Path -LiteralPath $PathValue) {
        throw "$Label still exists: $PathValue"
    }
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$workspace = Join-Path $repoRoot ("tmp\smoke_" + $timestamp)
$projectDir = Join-Path $workspace "TapTapManifestSmoke"
$installDir = Join-Path $workspace "install-root"

New-Item -ItemType Directory -Force -Path $workspace | Out-Null
Copy-Item -Recurse -Force (Join-Path $repoRoot "examples\TapTap") $projectDir
if (Test-Path (Join-Path $projectDir "scripts")) {
    Remove-Item -Recurse -Force (Join-Path $projectDir "scripts")
}

$configPath = Join-Path $projectDir "installer_config.json"
$config = Get-Content -Raw $configPath | ConvertFrom-Json
$config.project.name = "TapTapManifestSmoke"
$config.project.version = "1.0.1"
$config.project.publisher = "nano-installer smoke"
$config.output.installer_name = "TapTapManifestSmoke_Setup.exe"
$config.output.uninstaller_name = "uninst.exe"
$config.install.require_admin = $false
$config.install.default_path = $installDir
$config.install.append_to_path = ""
$config.install.mutex_name = "TapTapManifestSmoke_Installer"
$config.registry.install_path_key = "HKCU\Software\TapTapManifestSmoke"
$config.registry.uninstall_key = "HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\TapTapManifestSmoke"
$config.shortcuts.start_menu_folder = "TapTapManifestSmoke"
$config.autostart.enabled = $true
$config.autostart.default = $true
$config.autostart.registry_key = "HKCU\Software\Microsoft\Windows\CurrentVersion\Run"
$config.autostart.registry_value_name = "TapTapManifestSmoke"
$config.advanced.launch_app_after_install = $false
$config.advanced.silent_mode_support = $true
$config.advanced.uninstall_mode_support = $true
$configJson = $config | ConvertTo-Json -Depth 100
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($configPath, $configJson, $utf8NoBom)

$installerRegPath = Convert-ToRegistryProviderPath $config.registry.install_path_key
$uninstallRegPath = Convert-ToRegistryProviderPath $config.registry.uninstall_key
$autostartRegPath = Convert-ToRegistryProviderPath $config.autostart.registry_key
$desktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "TapTapManifestSmoke.lnk"
$startMenuShortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\TapTapManifestSmoke\TapTapManifestSmoke.lnk"

Write-Host "[smoke] Building release binaries..."
cargo build --release -p nano-installer-cli -p nano-installer-lzma -p uninst | Out-Host

Write-Host "[smoke] Building installer package..."
cargo run --bin nano-installer -- build --project $projectDir | Out-Host

$installerPath = Join-Path $projectDir "dist\TapTapManifestSmoke_Setup.exe"
Assert-PathExists -PathValue $installerPath -Label "Installer package"

Write-Host "[smoke] Running silent install..."
$env:RUST_LOG = "info"
$installProc = Start-Process -FilePath $installerPath -ArgumentList @("--silent", "--path", $installDir) -PassThru -Wait
if ($installProc.ExitCode -ne 0) {
    throw "Silent install failed with exit code $($installProc.ExitCode)"
}

$mainExe = Join-Path $installDir $config.install.exe_name
$uninstExe = Join-Path $installDir $config.output.uninstaller_name
$manifestPath = Join-Path $installDir "uninstall.json"

Assert-PathExists -PathValue $mainExe -Label "Installed main executable"
Assert-PathExists -PathValue $uninstExe -Label "Installed uninstaller"
Assert-PathExists -PathValue $manifestPath -Label "Uninstall manifest"
if (Test-Path -LiteralPath $desktopShortcut) {
    Write-Host "[smoke] Desktop shortcut created"
} else {
    Write-Host "[smoke] Desktop shortcut not present; continuing"
}
if (Test-Path -LiteralPath $startMenuShortcut) {
    Write-Host "[smoke] Start menu shortcut created"
} else {
    Write-Host "[smoke] Start menu shortcut not present; continuing"
}

$installProps = Get-ItemProperty -Path $installerRegPath
if ($installProps.InstallPath -ne $installDir) {
    throw "InstallPath registry value mismatch: $($installProps.InstallPath)"
}

$uninstallProps = Get-ItemProperty -Path $uninstallRegPath
if ($uninstallProps.DisplayName -ne "TapTapManifestSmoke") {
    throw "Uninstall registry DisplayName mismatch: $($uninstallProps.DisplayName)"
}

$autostartProps = Get-ItemProperty -Path $autostartRegPath -Name $config.autostart.registry_value_name
$autostartValue = $autostartProps.($config.autostart.registry_value_name)
if ([string]::IsNullOrWhiteSpace($autostartValue)) {
    throw "Autostart registry value missing"
}

$manifest = Get-Content -Raw $manifestPath | ConvertFrom-Json
if ($manifest.registry_values_to_remove.Count -lt 1) {
    throw "Uninstall manifest did not record registry values"
}
if (-not ($manifest.shortcuts_to_remove -contains $desktopShortcut)) {
    throw "Uninstall manifest did not record desktop shortcut"
}

$legacyMarker = Join-Path $installDir "legacy-marker.txt"
[System.IO.File]::WriteAllText($legacyMarker, "legacy", $utf8NoBom)
$manifest.files_to_remove += $legacyMarker
[System.IO.File]::WriteAllText($manifestPath, ($manifest | ConvertTo-Json -Depth 100), $utf8NoBom)

Write-Host "[smoke] Running second silent install over existing install..."
$reinstallProc = Start-Process -FilePath $installerPath -ArgumentList @("--silent", "--path", $installDir) -PassThru -Wait
if ($reinstallProc.ExitCode -ne 0) {
    throw "Second silent install failed with exit code $($reinstallProc.ExitCode)"
}

$mergedManifest = Get-Content -Raw $manifestPath | ConvertFrom-Json
if (-not ($mergedManifest.files_to_remove -contains $legacyMarker)) {
    throw "Reinstall did not preserve legacy uninstall manifest entry"
}

Write-Host "[smoke] Running silent uninstall..."
$uninstallProc = Start-Process -FilePath $uninstExe -ArgumentList @("--silent") -PassThru -Wait
if ($uninstallProc.ExitCode -ne 0) {
    throw "Silent uninstall failed with exit code $($uninstallProc.ExitCode)"
}

$deadline = (Get-Date).AddSeconds(20)
while ((Get-Date) -lt $deadline) {
    if (-not (Test-Path -LiteralPath $installDir)) {
        break
    }
    Start-Sleep -Milliseconds 500
}

Assert-PathMissing -PathValue $installDir -Label "Install directory"
Assert-PathMissing -PathValue $legacyMarker -Label "Legacy manifest marker"
Assert-PathMissing -PathValue $desktopShortcut -Label "Desktop shortcut"
Assert-PathMissing -PathValue $startMenuShortcut -Label "Start menu shortcut"
if (Test-Path $installerRegPath) {
    throw "Install path registry key still exists"
}
if (Test-Path $uninstallRegPath) {
    throw "Uninstall registry key still exists"
}
try {
    $null = Get-ItemProperty -Path $autostartRegPath -Name $config.autostart.registry_value_name
    throw "Autostart registry value still exists"
} catch {
    if ($_.Exception.Message -notmatch "does not exist") {
        throw
    }
}

Write-Host "[smoke] PASS"
Write-Host "[smoke] Workspace: $workspace"
