<#
    Downloads a published release and checks that it is the release it claims to be.

    The job that publishes a release checks a lot before it does: the tag names the version in
    Cargo.toml, the suite behind that commit was green, the binaries passed the Windows 7 import
    audit, and SHA256SUMS.txt was written from the files it uploads. What nothing checks afterwards
    is the download itself -- whether the bytes a user receives are those files, whether the
    executables in them carry the version the release names, and whether they run. This checks
    exactly that, from the release and nothing else:

      1. every asset of the tag is downloaded into a workspace
      2. each digest is recomputed and compared with SHA256SUMS.txt
      3. the version resource of the two product-facing executables has to name the tag's version,
         because a release whose binaries report another version is one nobody can identify after
         it is installed
      4. unless -SkipUi: the downloaded builder and the downloaded stubs build the TapTap example,
         and every page it declares is run and checked against the project's own layout -- the
         published bytes drawing a real installer, rather than a build from the sources

    It reads the release through the GitHub API, so `gh` has to be authenticated. Step 4 needs a
    desktop session, since it photographs real windows; a machine without one stops before it with
    -SkipUi. -AllowNoDigests is for the releases published before the digest list existed, which are
    still worth checking for the version they carry and the installer they can build.

    Usage:
      .\scripts\smoke_release.ps1 -Tag v2026.9.29
      .\scripts\smoke_release.ps1 -Tag v2026.9.29 -SkipUi
      .\scripts\smoke_release.ps1 -Tag v2026.9.17 -AllowNoDigests -SkipUi
#>
param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [string]$Workspace,
    [string]$Repository = "nick2781/nano-installer",
    [switch]$SkipUi,
    [switch]$AllowNoDigests
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "gh is not on the PATH, and this reads the release through the GitHub API"
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$version = $Tag -replace '^v', ''
if (-not $Workspace) {
    $Workspace = Join-Path $repoRoot "target/smoke-$Tag"
}
New-Item -ItemType Directory -Force -Path $Workspace | Out-Null

# The five executables a release publishes. The first two carry product resources and therefore a
# version; the stubs are raw by design and carry none, which is why they are not asked for one. The
# digest list beside them is checked where it is read, because only that knows about -AllowNoDigests.
$productExecutables = @("nano-installer-native-x64.exe", "nano-installer-gui-x64.exe")
$expectedAssets = @(
    "nano-installer-native-x64.exe"
    "nano-installer-gui-x64.exe"
    "lzma-stub-native.exe"
    "zlib-stub-native.exe"
    "uninst-stub-native.exe"
)

Write-Output "Downloading $Tag into $Workspace"
try {
    & gh release download $Tag --repo $Repository --dir $Workspace --clobber
    if ($LASTEXITCODE -ne 0) {
        throw "gh exited with $LASTEXITCODE"
    }
}
catch {
    throw "could not download $Tag from ${Repository}: $($_.Exception.Message)"
}

$failures = @()

foreach ($asset in $expectedAssets) {
    $path = Join-Path $Workspace $asset
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        $failures += "$Tag publishes no $asset"
    }
}

# 2. The digests, recomputed from the bytes that were downloaded. The list arrived with v2026.9.28;
#    a release older than that publishes none, which is a fact about the release rather than a
#    defect in the download, so it is tolerated only when it was asked for.
$sumsPath = Join-Path $Workspace "SHA256SUMS.txt"
$digestsChecked = 0
if (-not (Test-Path -LiteralPath $sumsPath -PathType Leaf)) {
    if ($AllowNoDigests) {
        Write-Output "  this release publishes no digest list; nothing to check against (-AllowNoDigests)"
    }
    else {
        $failures += "$Tag publishes no SHA256SUMS.txt; releases before v2026.9.28 publish none, so pass -AllowNoDigests to check what it does publish"
    }
}
else {
    foreach ($line in (Get-Content -LiteralPath $sumsPath -Encoding UTF8)) {
        if (-not $line.Trim()) {
            continue
        }
        $parts = $line -split '\s+', 2
        $expected = $parts[0].Trim().ToLower()
        $name = $parts[1].Trim()
        $file = Join-Path $Workspace $name
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
            $failures += "SHA256SUMS.txt names $name, which is not among the assets"
            continue
        }
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLower()
        if ($actual -ne $expected) {
            $failures += "$name does not match its published digest"
            continue
        }
        $digestsChecked++
        Write-Output "  digest matches: $name"
    }
    if ($digestsChecked -eq 0) {
        $failures += "SHA256SUMS.txt is empty, so nothing was checked against it"
    }
}

# 3. The version the two product-facing executables report has to be the one the tag names.
foreach ($name in $productExecutables) {
    $path = Join-Path $Workspace $name
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        continue
    }
    $info = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($path)
    if ($info.FileVersion -ne $version -or $info.ProductVersion -ne $version) {
        $failures += "$name reports version '$($info.FileVersion)'/'$($info.ProductVersion)' where $Tag names $version"
        continue
    }
    Write-Output "  version matches: $name reports $version"
}

# 4. The published bytes have to build and run a real installer. This is the same script that draws
#    the example's pages for the documentation, pointed at the download instead of a source build.
if (-not $SkipUi -and $failures.Count -eq 0) {
    $builder = Join-Path $Workspace "nano-installer-native-x64.exe"
    $snapshots = Join-Path $Workspace "snapshots"
    Write-Output "Building the TapTap example with the downloaded builder and stubs"
    & (Join-Path $PSScriptRoot "capture_setup_snapshots.ps1") `
        -Builder $builder -StubDirectory $Workspace -OutputDirectory $snapshots -Dpi 96, 192
    if ($LASTEXITCODE -ne 0) {
        $failures += "the downloaded builder could not build and run the example"
    }
    else {
        $drawn = @(Get-ChildItem -LiteralPath $snapshots -Filter "*.png" -File -ErrorAction SilentlyContinue).Count
        if ($drawn -eq 0) {
            $failures += "the run drew no page at all"
        }
        else {
            Write-Output "  the downloaded bytes built a setup and drew $drawn page(s)"
        }
    }
}
elseif ($SkipUi) {
    Write-Output "Skipping the run of the built setup (-SkipUi)"
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    throw "The release $Tag is not the release it claims to be"
}

$summary = "Release smoke test passed: $Tag, $digestsChecked digest(s) verified"
if ($AllowNoDigests -and $digestsChecked -eq 0) {
    $summary = "Release smoke test passed: $Tag, no digest list to verify"
}
if ($SkipUi) {
    $summary += ", assets only"
}
else {
    $summary += ", and the published bytes ran an installer"
}
Write-Output $summary
