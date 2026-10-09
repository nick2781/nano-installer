# Rebuilds and republishes the body of every published release from CHANGELOG.md,
# the same way the release job builds it. Run it after the changelog's wording
# changes: the published body is a copy, so it does not follow on its own.
param(
    [string]$Repository = 'nick2781/nano-installer',
    [switch]$DryRun
)

Set-Location 'D:\taptap-pc\nano-installer'
$env:GITHUB_REPOSITORY = $Repository

$releases = gh release list --repo $Repository --limit 50 --json tagName,publishedAt | ConvertFrom-Json
if (-not $releases) { Write-Output 'no releases found'; exit 1 }

foreach ($release in ($releases | Sort-Object publishedAt)) {
    $tag = $release.tagName
    $out = Join-Path ([System.IO.Path]::GetTempPath()) "nano-notes-$tag.md"
    # The generator throws when the changelog has no section for the tag, which is
    # the right answer for a release that predates the changelog format.
    try {
        & "$PSScriptRoot\changelog_notes.ps1" -Tag $tag -OutputPath $out | Out-Null
    } catch {
        Write-Output "skip ${tag}: $($_.Exception.Message)"
        continue
    }
    if (-not (Test-Path -LiteralPath $out)) {
        Write-Output "skip ${tag}: the generator wrote no body"
        continue
    }
    $body = [System.IO.File]::ReadAllText($out)
    if ($DryRun) {
        Write-Output ("{0}: would publish {1} character(s)" -f $tag, $body.Length)
        Remove-Item $out -Force
        continue
    }
    gh release edit $tag --repo $Repository --notes-file $out | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-Output ("{0}: republished {1} character(s)" -f $tag, $body.Length)
    } else {
        Write-Output ("{0}: gh release edit failed" -f $tag)
    }
    Remove-Item $out -Force -ErrorAction SilentlyContinue
}
