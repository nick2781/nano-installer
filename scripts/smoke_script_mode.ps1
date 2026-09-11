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
$workspace = Join-Path $repoRoot ("tmp\smoke_script_" + $timestamp)
$projectDir = Join-Path $workspace "TapTapScriptSmoke"
$installDir = Join-Path $workspace "install-root"

New-Item -ItemType Directory -Force -Path $workspace | Out-Null
Copy-Item -Recurse -Force (Join-Path $repoRoot "examples\TapTap") $projectDir

$configPath = Join-Path $projectDir "installer_config.json"
$config = Get-Content -Raw $configPath | ConvertFrom-Json
$config.project.name = "TapTapScriptSmoke"
$config.project.version = "1.0.2"
$config.project.publisher = "nano-installer script smoke"
$config.output.installer_name = "TapTapScriptSmoke_Setup.exe"
$config.output.uninstaller_name = "uninst.exe"
$config.install.require_admin = $false
$config.install.default_path = $installDir
$config.install.append_to_path = ""
$config.install.mutex_name = "TapTapScriptSmoke_Installer"
$config.registry.install_path_key = "HKCU\Software\TapTapScriptSmoke"
$config.registry.uninstall_key = "HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\TapTapScriptSmoke"
$config.shortcuts.start_menu_folder = "TapTapScriptSmoke"
$config.autostart.enabled = $true
$config.autostart.default = $true
$config.autostart.registry_key = "HKCU\Software\Microsoft\Windows\CurrentVersion\Run"
$config.autostart.registry_value_name = "TapTapScriptSmoke"
$config.advanced.launch_app_after_install = $false
$config.advanced.silent_mode_support = $true
$config.advanced.uninstall_mode_support = $true
$configJson = $config | ConvertTo-Json -Depth 100
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($configPath, $configJson, $utf8NoBom)

$installScriptPath = Join-Path $projectDir "scripts\install.rhai"
$installScript = Get-Content -Raw $installScriptPath
$installScript = $installScript.Replace('reg_write_string("HKLM\\Software\\', 'reg_write_string("HKCU\\Software\\')
$installScript = $installScript.Replace('let uninst_key = "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\" + app_name;', 'let uninst_key = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\" + app_name;')
[System.IO.File]::WriteAllText($installScriptPath, $installScript, $utf8NoBom)

$legacyUninstallScript = @'
set_status("Running legacy uninstall script...");
set_progress(25.0);
shell_notify();
'@
[System.IO.File]::WriteAllText((Join-Path $projectDir "scripts\uninstall.rhai"), $legacyUninstallScript, $utf8NoBom)

$installerRegPath = Convert-ToRegistryProviderPath $config.registry.install_path_key
$uninstallRegPath = Convert-ToRegistryProviderPath $config.registry.uninstall_key
$autostartRegPath = Convert-ToRegistryProviderPath $config.autostart.registry_key
$desktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "TapTapScriptSmoke.lnk"
$startMenuShortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\TapTapScriptSmoke\TapTapScriptSmoke.lnk"

Write-Host "[script-smoke] Building release binaries..."
cargo build --release -p nano-installer-lzma | Out-Host
cargo build --release -p nano-installer-zlib | Out-Host
cargo build --release -p uninst | Out-Host
cargo build --release -p nano-installer-cli | Out-Host

Write-Host "[script-smoke] Building installer package..."
cargo run --bin nano-installer -- build --project $projectDir | Out-Host

$installerPath = Join-Path $projectDir "dist\TapTapScriptSmoke_Setup.exe"
Assert-PathExists -PathValue $installerPath -Label "Installer package"

Write-Host "[script-smoke] Running silent install..."
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
if (-not (Test-Path -LiteralPath $desktopShortcut)) {
    throw "Desktop shortcut missing"
}
if (-not (Test-Path -LiteralPath $startMenuShortcut)) {
    throw "Start menu shortcut missing"
}

$installProps = Get-ItemProperty -Path $installerRegPath
if ($installProps.InstallPath -ne $installDir) {
    throw "InstallPath registry value mismatch: $($installProps.InstallPath)"
}

$manifest = Get-Content -Raw $manifestPath | ConvertFrom-Json
if ($manifest.shortcuts_to_remove.Count -lt 2) {
    throw "Uninstall manifest did not record script-created shortcuts"
}
if ($manifest.registry_keys_to_remove.Count -lt 2) {
    throw "Uninstall manifest did not record script-created registry keys"
}

$legacyMarker = Join-Path $installDir "legacy-script-marker.txt"
[System.IO.File]::WriteAllText($legacyMarker, "legacy", $utf8NoBom)
$manifest.files_to_remove += $legacyMarker
[System.IO.File]::WriteAllText($manifestPath, ($manifest | ConvertTo-Json -Depth 100), $utf8NoBom)

Write-Host "[script-smoke] Running second silent install over existing install..."
$reinstallProc = Start-Process -FilePath $installerPath -ArgumentList @("--silent", "--path", $installDir) -PassThru -Wait
if ($reinstallProc.ExitCode -ne 0) {
    throw "Second silent install failed with exit code $($reinstallProc.ExitCode)"
}

$mergedManifest = Get-Content -Raw $manifestPath | ConvertFrom-Json
if (-not ($mergedManifest.files_to_remove -contains $legacyMarker)) {
    throw "Reinstall did not preserve legacy script uninstall manifest entry"
}

Write-Host "[script-smoke] Running silent uninstall..."
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

Write-Host "[script-smoke] PASS"
Write-Host "[script-smoke] Workspace: $workspace"
