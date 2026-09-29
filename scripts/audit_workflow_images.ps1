<#
    Holds the workflows to the two promises their comments make about images.

    Every job here names the image it runs on instead of using `-latest`, because
    what builds the five released executables has to change in a commit rather
    than on GitHub's schedule. A comment is not a promise a machine can check, so
    this is the machine: a job whose image ends in `-latest`, a job whose image is
    decided by an expression this cannot read, and the Windows jobs of ci.yml and
    release.yml disagreeing about their image are all failures.

    The last one is the point of the pair. The suite that has to be green behind a
    tag is what says a release was tested, and it says that about binaries built
    on one image; if the release job builds on another, the audit covers something
    that is not being shipped. Naming both `windows-2025-vs2026` is only correct
    while it is true in both files at once, so moving one without the other fails
    here.

    Usage:
      .\scripts\audit_workflow_images.ps1
      .\scripts\audit_workflow_images.ps1 -WorkflowDirectory .github/workflows
#>
param(
    [string]$WorkflowDirectory
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$directory = if ($WorkflowDirectory) {
    $WorkflowDirectory
} else {
    Join-Path $repoRoot ".github/workflows"
}
if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
    throw "Workflow directory not found: $directory"
}

$failures = @()
$inspected = 0
# Which image each Windows job names, by the file it is in, so the two that have
# to agree can be compared afterwards.
$windowsImages = @{}

foreach ($file in (Get-ChildItem -LiteralPath $directory -Filter "*.yml" -File | Sort-Object Name)) {
    $lines = [System.IO.File]::ReadAllLines($file.FullName)
    $job = ""
    for ($index = 0; $index -lt $lines.Count; $index++) {
        $line = $lines[$index]
        $jobMatch = [regex]::Match($line, '^  ([A-Za-z0-9_-]+):\s*$')
        if ($jobMatch.Success) {
            $job = $jobMatch.Groups[1].Value
        }
        $imageMatch = [regex]::Match($line, '^\s*runs-on:\s*(.+?)\s*$')
        if (-not $imageMatch.Success) {
            continue
        }
        $inspected++
        $image = $imageMatch.Groups[1].Value.Trim().Trim("'").Trim('"')
        $where = "$($file.Name):$($index + 1) ($job)"

        if ($image -match '\$\{\{') {
            $failures += "$where names its image with an expression this cannot read: $image"
            continue
        }
        if ($image -match 'latest') {
            $failures += "$where runs on '$image'; name the image a release is built on"
            continue
        }
        Write-Output "Workflow image audit inspected: $where -> $image"
        if ($image -like "windows-*") {
            $windowsImages[$file.Name] = $image
        }
    }
}

if ($inspected -eq 0) {
    throw "No job in $directory names an image, so this audit checked nothing"
}

# The suite that backs a release and the job that builds it have to run on the
# same Windows image, or the audited binaries are not the shipped ones.
$releaseImage = ""
if ($windowsImages.ContainsKey("release.yml")) {
    $releaseImage = $windowsImages["release.yml"]
    foreach ($file in $windowsImages.Keys) {
        if ($file -eq "release.yml") {
            continue
        }
        if ($windowsImages[$file] -ne $releaseImage) {
            $failures += "$file runs on $($windowsImages[$file]) while release.yml builds on $releaseImage; the audit behind a tag would cover a different binary"
        }
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    throw "Workflow image audit failed"
}

Write-Output "Workflow image audit passed: $inspected job(s) name their image; a release is built and audited on $releaseImage"
