param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [string]$ChangelogPath = "CHANGELOG.md",
    [switch]$Full
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

$body = ($lines[$start..($end - 1)] -join "`n").Trim()

# The published notes stay product-facing: everything from the marker on is
# implementation detail kept in the repository changelog only.
$marker = "<!-- release-notes:end -->"
if (-not $Full) {
    $markerIndex = $body.IndexOf($marker)
    if ($markerIndex -ge 0) {
        $body = $body.Substring(0, $markerIndex).TrimEnd()
    }
    else {
        Write-Warning "The '$heading' section has no '$marker'; the whole section, including technical detail, will be published."
    }
}
else {
    $body = $body.Replace($marker, "").TrimEnd()
}

if ([string]::IsNullOrWhiteSpace($body)) {
    throw "The '$heading' section has no product-facing content before '$marker'."
}

$repo = $env:GITHUB_REPOSITORY

# Release bodies are not rendered next to the checkout, so relative links such as
# docs/PRODUCTION_STATUS.md must be promoted to absolute repository links.
if (-not [string]::IsNullOrEmpty($repo)) {
    $body = [regex]::Replace($body, '\]\((?![a-zA-Z][a-zA-Z0-9+.-]*:|#)([^)]+)\)', {
        param($match)
        "]($('https://github.com/' + $repo + '/blob/main/' + $match.Groups[1].Value))"
    })
}

$changelogLink = if ([string]::IsNullOrEmpty($repo)) { "CHANGELOG.md" } else { "https://github.com/$repo/blob/main/CHANGELOG.md" }
$body += "`n`n完整更新日志见 [CHANGELOG.md]($changelogLink)。`n"

$fullPath = [System.IO.Path]::GetFullPath($OutputPath)
$dir = [System.IO.Path]::GetDirectoryName($fullPath)
if (-not [string]::IsNullOrEmpty($dir)) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
}

[System.IO.File]::WriteAllText($fullPath, $body, (New-Object System.Text.UTF8Encoding($false)))
Write-Host "Wrote release notes for $Tag to $fullPath"