<#
    Writes the digests of the published release assets, beside them.

    A release is a set of files somebody downloads and installs on a machine they
    care about, and the only thing that says a download arrived whole is a digest
    published next to it. The product already works that way for what it fetches
    itself: a dependency's installer is only run when its bytes match the digest
    the project wrote down. What this repository publishes deserves the same, so
    `SHA256SUMS.txt` is written here and uploaded with the assets.

    The format is the one `sha256sum -c` and `shasum -c` read: one digest, two
    spaces, and the file's name as it is published (the assets from `stubs/` are
    published at the top level, so that is how they are named here). On Windows
    the same check is:

        Get-Content SHA256SUMS.txt | ForEach-Object {
            $digest, $name = $_ -split '\s+', 2
            if ((Get-FileHash $name -Algorithm SHA256).Hash -ne $digest) { throw "$name does not match" }
        }

    Usage:
        .\scripts\release_manifest.ps1
        .\scripts\release_manifest.ps1 -ReleaseDirectory target/release -Output tmp/SHA256SUMS.txt
#>
param(
    [string]$ReleaseDirectory = "target/release",
    [string[]]$Asset = @(
        "nano-installer-native-x64.exe",
        "nano-installer-gui-x64.exe",
        "stubs/lzma-stub-native.exe",
        "stubs/zlib-stub-native.exe",
        "stubs/uninst-stub-native.exe"
    ),
    [string]$Output = "tmp/SHA256SUMS.txt"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
function Resolve-RepoPath([string]$Path) {
    if ([System.IO.Path]::IsPathRooted($Path)) { return $Path }
    return (Join-Path $repoRoot $Path)
}

$directory = Resolve-RepoPath $ReleaseDirectory
if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
    throw "there is no release directory at $directory; build one first"
}

$lines = New-Object System.Collections.Generic.List[string]
foreach ($relative in $Asset) {
    $path = Join-Path $directory ($relative -replace '/', '\')
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "the release is missing $relative; a manifest of what is there is not a manifest of the release"
    }
    $digest = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    $published = Split-Path -Leaf $relative
    $lines.Add("$digest  $published")
}

$outputPath = Resolve-RepoPath $Output
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outputPath) | Out-Null
[System.IO.File]::WriteAllLines($outputPath, [string[]]$lines, (New-Object System.Text.UTF8Encoding($false)))

foreach ($line in $lines) {
    Write-Output $line
}
Write-Output "Digests written to $outputPath"
