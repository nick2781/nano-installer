<#
    Guards the one encoding rule that silently corrupts release notes.

    Windows PowerShell decodes a BOM-less script using the ANSI code page of the
    machine. A script that is pure ASCII is unaffected, but one holding a
    non-ASCII literal looks correct on a UTF-8 development machine and produces
    mojibake on an English runner. That is exactly how the Chinese footer in
    scripts/changelog_notes.ps1 reached v2026.9.18 as "ae*...".

    A UTF-8 BOM makes the encoding explicit, so it is required here for any
    script that is not pure ASCII.
#>
param(
    [string]$ScriptDirectory
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$directory = if ($ScriptDirectory) {
    $ScriptDirectory
} else {
    Join-Path $repoRoot "scripts"
}
if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
    throw "Script directory not found: $directory"
}

$failures = @()
foreach ($file in Get-ChildItem -LiteralPath $directory -Filter "*.ps1" -File) {
    $bytes = [System.IO.File]::ReadAllBytes($file.FullName)
    $hasBom = $bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF
    if ($hasBom) {
        Write-Output "Script encoding audit inspected: $($file.FullName) (UTF-8 BOM)"
        continue
    }
    $nonAscii = $false
    foreach ($byte in $bytes) {
        if ($byte -gt 0x7F) {
            $nonAscii = $true
            break
        }
    }
    if ($nonAscii) {
        $failures += "$($file.FullName) holds non-ASCII text without a UTF-8 BOM; Windows PowerShell would decode it with the ANSI code page"
    }
    else {
        Write-Output "Script encoding audit inspected: $($file.FullName) (pure ASCII)"
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    throw "Script encoding audit failed"
}

Write-Output "Script encoding audit passed"
