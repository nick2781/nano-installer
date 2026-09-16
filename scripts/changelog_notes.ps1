param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [string]$ChangelogPath = "CHANGELOG.md"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$version = $Tag -replace '^v', ''
$heading = "## [$version]"

$lines = Get-Content -LiteralPath $ChangelogPath
$start = -1
for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i].Trim() -eq $heading) {
        $start = $i + 1
        break
    }
}

if ($start -lt 0) {
    throw "CHANGELOG.md has no '$heading' section; cannot build release notes for '$Tag'."
}

$end = $lines.Count
for ($i = $start; $i -lt $lines.Count; $i++) {
    if ($lines[$i].StartsWith("## ")) {
        $end = $i
        break
    }
}

if ($end -le $start) {
    throw "The '$heading' section is empty; refusing to publish empty release notes."
}

$body = ($lines[$start..($end - 1)] -join "`n").TrimEnd()
$repo = $env:GITHUB_REPOSITORY
$changelogLink = if ([string]::IsNullOrEmpty($repo)) { "CHANGELOG.md" } else { "https://github.com/$repo/blob/main/CHANGELOG.md" }
$body += "`n`n完整更新日志见 [CHANGELOG.md]($changelogLink)。`n"

$full = [System.IO.Path]::GetFullPath($OutputPath)
$dir = [System.IO.Path]::GetDirectoryName($full)
if (-not [string]::IsNullOrEmpty($dir)) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
}

[System.IO.File]::WriteAllText($full, $body, (New-Object System.Text.UTF8Encoding($false)))
Write-Host "Wrote release notes for $Tag to $full"