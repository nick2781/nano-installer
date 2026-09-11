param(
    [string]$Project = "examples/TapTap",
    [ValidateSet("lzma", "zlib")]
    [string]$Compression = "lzma",
    [string]$Toolchain = "nightly-2025-11-08",
    [string]$SignScript
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$projectPath = if ([System.IO.Path]::IsPathRooted($Project)) {
    $Project
} else {
    Join-Path $repoRoot $Project
}
$targetTriple = "x86_64-win7-windows-msvc"
$targetRelease = Join-Path $repoRoot "target/$targetTriple/release"
$installerStub = "$Compression-x64.exe"
$uninstallerStub = "uninst-x64.exe"
$previousStubDir = $env:NANO_INSTALLER_STUB_DIR
$previousInstallerStub = $env:NANO_INSTALLER_INSTALLER_STUB
$previousUninstallerStub = $env:NANO_INSTALLER_UNINSTALLER_STUB

if (-not (Test-Path -LiteralPath (Join-Path $projectPath "installer_config.json"))) {
    throw "Installer project not found: $projectPath"
}

function Invoke-Checked {
    param([scriptblock]$Command, [string]$Description)

    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Description failed with exit code $LASTEXITCODE"
    }
}

Push-Location $repoRoot
try {
    Invoke-Checked {
        & rustup run $Toolchain cargo build --locked --release `
            -Z build-std=std,panic_abort `
            --target $targetTriple `
            -p nano-installer-lzma
    } "Win7 LZMA runtime build"
    Invoke-Checked {
        & rustup run $Toolchain cargo build --locked --release `
            -Z build-std=std,panic_abort `
            --target $targetTriple `
            -p nano-installer-zlib
    } "Win7 zlib runtime build"
    Invoke-Checked {
        & rustup run $Toolchain cargo build --locked --release `
            -Z build-std=std,panic_abort `
            --target $targetTriple `
            -p uninst
    } "Win7 uninstaller stub build"

    & (Join-Path $PSScriptRoot "retarget_win7_com_import.ps1") -File @(
        (Join-Path $targetRelease "lzma-x64.exe"),
        (Join-Path $targetRelease "zlib-x64.exe"),
        (Join-Path $targetRelease "uninst-x64.exe")
    )

    Invoke-Checked {
        cargo build --locked --release -p nano-installer-cli
    } "Host CLI build"

    $cli = Join-Path $repoRoot "target/release/nano-installer.exe"
    $env:NANO_INSTALLER_STUB_DIR = $targetRelease
    $env:NANO_INSTALLER_INSTALLER_STUB = $installerStub
    $env:NANO_INSTALLER_UNINSTALLER_STUB = $uninstallerStub

    Invoke-Checked {
        & $cli validate --config (Join-Path $projectPath "installer_config.json")
    } "Configuration validation"
    Invoke-Checked {
        & $cli harness lint-resources --project $projectPath --format text
    } "Resource lint"

    $buildArgs = @("build", "--project", $projectPath, "--release")
    if ($SignScript) {
        $buildArgs += @("--sign-script", $SignScript)
    }
    Invoke-Checked { & $cli @buildArgs } "Win7 setup build"

    $config = Get-Content -LiteralPath (Join-Path $projectPath "installer_config.json") -Raw |
        ConvertFrom-Json
    $setupPath = Join-Path (Join-Path $projectPath "dist") $config.output.installer_name
    $uninstallerPath = Join-Path (Join-Path $projectPath ".build") $config.output.uninstaller_name
    & (Join-Path $PSScriptRoot "audit_win7_imports.ps1") -File @(
        (Join-Path $targetRelease "lzma-x64.exe"),
        (Join-Path $targetRelease "zlib-x64.exe"),
        (Join-Path $targetRelease $uninstallerStub),
        $uninstallerPath,
        $setupPath
    )
} finally {
    $env:NANO_INSTALLER_STUB_DIR = $previousStubDir
    $env:NANO_INSTALLER_INSTALLER_STUB = $previousInstallerStub
    $env:NANO_INSTALLER_UNINSTALLER_STUB = $previousUninstallerStub
    Pop-Location
}
