param(
    [string]$Project = "examples/TapTap",
    [ValidateSet("lzma", "zlib")]
    [string]$Compression = "lzma",
    [switch]$SkipStubs,
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
$targetRelease = Join-Path $repoRoot "target/release"
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
    if (-not $SkipStubs) {
        Invoke-Checked {
            cargo build --locked --release -p nano-installer-lzma
        } "x64 LZMA runtime stub build"
        Invoke-Checked {
            cargo build --locked --release -p nano-installer-zlib
        } "x64 zlib runtime stub build"
        Invoke-Checked {
            cargo build --locked --release -p uninst
        } "x64 uninstaller stub build"
        Invoke-Checked {
            cargo build --locked --release -p nano-installer-cli
        } "Host CLI build"
    }

    $cli = Join-Path $repoRoot "target/release/nano-installer.exe"
    if (-not (Test-Path -LiteralPath $cli)) {
        throw "CLI not found: $cli. Run without -SkipStubs first."
    }

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
    Invoke-Checked { & $cli @buildArgs } "Setup build"
} finally {
    $env:NANO_INSTALLER_STUB_DIR = $previousStubDir
    $env:NANO_INSTALLER_INSTALLER_STUB = $previousInstallerStub
    $env:NANO_INSTALLER_UNINSTALLER_STUB = $previousUninstallerStub
    Pop-Location
}
