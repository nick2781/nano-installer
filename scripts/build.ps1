param(
    [string]$Project = "examples/TapTap",
    [string]$Toolchain = "nightly-2025-11-08",
    [string]$Output
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$targetTriple = "x86_64-win7-windows-msvc"
$cargoRelease = Join-Path $repoRoot "target/$targetTriple/release"
$publishRelease = Join-Path $repoRoot "target/release"
$projectPath = if ([System.IO.Path]::IsPathRooted($Project)) {
    $Project
} else {
    Join-Path $repoRoot $Project
}
$configPath = Join-Path $projectPath "installer_config.json"
if (-not (Test-Path -LiteralPath $configPath)) {
    throw "Installer project not found: $projectPath"
}

$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
$outputPath = if ($Output) {
    if ([System.IO.Path]::IsPathRooted($Output)) { $Output } else { Join-Path $repoRoot $Output }
} else {
    Join-Path $publishRelease $config.output.installer_name
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
            -p nano-installer-native
    } "Win7+ native toolchain build"

    New-Item -ItemType Directory -Force -Path $publishRelease | Out-Null
    foreach ($name in @(
        "nano-installer-native-x64.exe",
        "native-lzma-x64.exe",
        "native-zlib-x64.exe",
        "native-uninst-x64.exe"
    )) {
        Copy-Item -LiteralPath (Join-Path $cargoRelease $name) `
            -Destination (Join-Path $publishRelease $name) -Force
    }

    $builder = Join-Path $publishRelease "nano-installer-native-x64.exe"
    Invoke-Checked {
        & $builder build --project $projectPath --output $outputPath
    } "Native setup build"

    & (Join-Path $PSScriptRoot "audit_win7_imports.ps1") -File @(
        $builder,
        (Join-Path $publishRelease "native-lzma-x64.exe"),
        (Join-Path $publishRelease "native-zlib-x64.exe"),
        (Join-Path $publishRelease "native-uninst-x64.exe"),
        $outputPath
    )

    Write-Output "Native Win7+ setup: $outputPath"
} finally {
    Pop-Location
}
