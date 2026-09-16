<#
    Runs the release-notes generator the way the release job does and proves the
    result is readable.

    Published bodies are built from CHANGELOG.md by scripts/changelog_notes.ps1.
    That script holds a Chinese footer, and Windows PowerShell decodes a BOM-less
    script with the ANSI code page, so v2026.9.18 shipped a footer that read
    correctly on a UTF-8 development machine and arrived as mojibake. The
    generator's own output is what has to be checked, and the release job only
    does that on a tag push, so it is checked here on every change instead.

    The expected footer is read out of the generator source, decoded the way
    PowerShell is supposed to decode it. If the file loses its BOM, executing it
    decodes the literal with the ANSI code page and the two no longer match.

    This check can only fail where the ANSI code page is not UTF-8, which is why
    it belongs on the CI runner rather than on a machine set to UTF-8. The
    host-independent guard for the same regression is
    scripts/audit_script_encoding.ps1, which inspects bytes and needs no
    particular code page.
#>
param(
    [string]$Tag,
    [string]$Repository = "nick2781/nano-installer"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$generator = Join-Path $PSScriptRoot "changelog_notes.ps1"
$output = Join-Path ([System.IO.Path]::GetTempPath()) ("nano-release-notes-" + [Guid]::NewGuid().ToString("N") + ".md")

if (-not $Tag) {
    $versionLine = Get-Content -LiteralPath (Join-Path $repoRoot "Cargo.toml") -Encoding UTF8 |
        Where-Object { $_ -match '^version\s*=\s*"' } |
        Select-Object -First 1
    if (-not $versionLine) {
        throw "Cargo.toml has no workspace version; cannot verify release notes"
    }
    $Tag = "v" + ([regex]::Match($versionLine, '"([^"]+)"').Groups[1].Value)
}

# The release runner resolves the repository from the environment, which also
# turns relative links in the section into absolute ones.
$expectedLink = "https://github.com/$Repository/blob/main/CHANGELOG.md"
$previousRepository = $env:GITHUB_REPOSITORY
$env:GITHUB_REPOSITORY = $Repository
& $generator -Tag $Tag -OutputPath $output | Out-Null
$env:GITHUB_REPOSITORY = $previousRepository

if (-not (Test-Path -LiteralPath $output -PathType Leaf)) {
    throw "The release-notes generator wrote no file for $Tag"
}
$notes = [System.IO.File]::ReadAllText($output, [System.Text.UTF8Encoding]::new($false))
[System.IO.File]::Delete($output)

$lines = @($notes -split "`r?`n" | Where-Object { $_.Trim() })
if ($lines.Count -eq 0) {
    throw "The generated release notes for $Tag are empty"
}
$footer = $lines[-1]

# Read the generator the way the module loader should: a BOM selects UTF-8.
$source = [System.IO.File]::ReadAllText($generator, [System.Text.UTF8Encoding]::new($true))
$appends = @($source -split "`r?`n" | Where-Object { $_ -match '\$body \+=' -and $_ -match 'CHANGELOG\.md\]\(' })
if ($appends.Count -ne 1) {
    throw "scripts/changelog_notes.ps1 must have exactly one footer append; found $($appends.Count)"
}
$literal = $appends[0].Substring($appends[0].IndexOf('"') + 1)
$literal = $literal.Substring(0, $literal.LastIndexOf('"'))
$literal = $literal.Replace("``n", "").Replace('$changelogLink', $expectedLink)

if (-not $footer.Equals($literal)) {
    throw "The generated footer is not the text the generator holds.`n  generated: $footer`n  expected:  $literal"
}
if ($footer.Contains([char]0xFFFD)) {
    throw "The generated footer contains a replacement character, so a byte was lost: $footer"
}

Write-Output "Release notes verified for ${Tag}: $($notes.Length) characters, footer intact"
