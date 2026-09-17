param(
    [string]$Project,
    [string]$Toolchain = "nightly-2025-11-08",
    [string]$Output
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$targetTriple = "x86_64-win7-windows-msvc"
$cargoRelease = Join-Path $repoRoot "target/$targetTriple/release"
$guiTargetRoot = Join-Path $repoRoot "target/gui-build"
$guiCargoRelease = Join-Path $guiTargetRoot "release"
$publishRelease = Join-Path $repoRoot "target/release"
$publishStubs = Join-Path $publishRelease "stubs"
if ($Output -and -not $Project) {
    throw "-Output requires -Project"
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
    # Read before the encoding audit below, so the CalVer check and the build
    # agree on which version is being released.
    $versionLine = Get-Content -LiteralPath (Join-Path $repoRoot "Cargo.toml") -Encoding UTF8 |
        Where-Object { $_ -match '^version\s*=\s*"' } |
        Select-Object -First 1
    if (-not $versionLine) {
        throw "Cargo.toml has no workspace version"
    }
    $workspaceVersion = [regex]::Match($versionLine, '"([^"]+)"').Groups[1].Value
    # A BOM-less script holding non-ASCII text is decoded with the ANSI code page
    # on Windows PowerShell, which silently corrupted the release-note footer once
    # already. Check it before anything expensive runs.
    & (Join-Path $PSScriptRoot "audit_script_encoding.ps1")

    # A release number is a date, so it can be wrong: the workspace version has
    # to follow CalVer, which keeps the tag, the changelog heading, and the
    # version resources in the built binaries telling the same story.
    & (Join-Path $PSScriptRoot "release_version.ps1") -Version $workspaceVersion

    Invoke-Checked {
        & rustup run $Toolchain cargo build --locked --release `
            -Z build-std=std,panic_abort `
            --target $targetTriple `
            -p nano-installer-native-cli `
            -p nano-installer-stub-lzma `
            -p nano-installer-stub-zlib `
            -p nano-installer-uninstaller
    } "Win7+ native toolchain build"

    $previousRustFlags = $env:RUSTFLAGS
    try {
        $env:RUSTFLAGS = "-C target-feature=+crt-static"
        Invoke-Checked {
            & rustup run $Toolchain cargo build --locked --release `
                --target-dir $guiTargetRoot `
                -p nano-installer-gui
        } "Windows 10+ GUI build"
    } finally {
        $env:RUSTFLAGS = $previousRustFlags
    }

    if (Test-Path -LiteralPath $publishRelease) {
        Remove-Item -LiteralPath $publishRelease -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $publishStubs | Out-Null
    Copy-Item -LiteralPath (Join-Path $cargoRelease "nano-installer-native-x64.exe") `
        -Destination (Join-Path $publishRelease "nano-installer-native-x64.exe") -Force
    Copy-Item -LiteralPath (Join-Path $guiCargoRelease "nano-installer-gui-x64.exe") `
        -Destination (Join-Path $publishRelease "nano-installer-gui-x64.exe") -Force
    foreach ($name in @(
        "lzma-stub-native.exe",
        "zlib-stub-native.exe",
        "uninst-stub-native.exe"
    )) {
        Copy-Item -LiteralPath (Join-Path $cargoRelease $name) `
            -Destination (Join-Path $publishStubs $name) -Force
    }

    $builder = Join-Path $publishRelease "nano-installer-native-x64.exe"
    $auditFiles = @(
        $builder,
        (Join-Path $publishStubs "lzma-stub-native.exe"),
        (Join-Path $publishStubs "zlib-stub-native.exe"),
        (Join-Path $publishStubs "uninst-stub-native.exe")
    )

    & (Join-Path $PSScriptRoot "smoke_backends.ps1") -StubsDirectory $publishStubs

    if ($Project) {
        $projectPath = if ([System.IO.Path]::IsPathRooted($Project)) {
            $Project
        } else {
            Join-Path $repoRoot $Project
        }
        $configPath = Join-Path $projectPath "installer_config.json"
        if (-not (Test-Path -LiteralPath $configPath)) {
            throw "Installer project not found: $projectPath"
        }
        # A project file is UTF-8 and may hold non-ASCII product names; without
        # the named encoding Windows PowerShell reads it with the ANSI code page.
        $config = Get-Content -LiteralPath $configPath -Raw -Encoding UTF8 | ConvertFrom-Json
        $outputPath = if ($Output) {
            if ([System.IO.Path]::IsPathRooted($Output)) {
                $Output
            } else {
                Join-Path $repoRoot $Output
            }
        } else {
            Join-Path (Join-Path $projectPath "dist") $config.output.installer_name
        }
        Invoke-Checked {
            & $builder build --project $projectPath --output $outputPath
        } "Native setup build"
        & (Join-Path $PSScriptRoot "audit_embedded_uninstaller.ps1") `
            -Setup $outputPath `
            -ExpectedName $(if ($config.output.uninstaller_name) { $config.output.uninstaller_name } else { "uninst.exe" }) `
            -ExpectedVersion $(if ($config.project.file_version) { $config.project.file_version } else { $config.project.version })
        # The builder injects the manifest the project asked for, so the setup has
        # to declare the level and DPI behaviour this configuration resolves to.
        # Raw stubs carry no manifest on purpose; the builder adds it per project.
        & (Join-Path $PSScriptRoot "audit_application_manifest.ps1") `
            -File $outputPath `
            -ExpectLevel $(if ($config.install.require_admin) { "requireAdministrator" } else { "asInvoker" }) `
            -ExpectDpiAware $(if ($config.ui.dpi_aware -eq $false) { "false" } else { "true" })
        $auditFiles += $outputPath
        Write-Output "Native Win7+ setup: $outputPath"
    }

    & (Join-Path $PSScriptRoot "audit_win7_imports.ps1") -File $auditFiles
    Write-Output "Native Win7+ toolchain: $publishRelease"
    Write-Output "Windows 10+ GUI: $(Join-Path $publishRelease 'nano-installer-gui-x64.exe')"
} finally {
    Pop-Location
}
