<#
    Guards what a release note section is allowed to publish.

    A section of CHANGELOG.md is cut in two by `<!-- release-notes:end -->`.
    Everything above the marker becomes the release body on GitHub, which is the
    only part a user reads; everything below stays in the repository changelog.
    scripts/changelog_notes.ps1 performs that cut and can only notice that the
    marker is absent -- it warns and publishes the whole section. It cannot tell
    a sentence about the product from a sentence about the pipeline, and the
    pipeline is what a section tends to fill up with: script paths, workflow
    files, branch protection, dependency bots, cache keys.

    So this audit reads every section that can still change and fails when the
    published half carries repository machinery. The published half is what the
    changelog beside a download says, and a reader deciding whether to install
    the product has no use for how the release job is wired.

    Sections released before $firstAuditedVersion were published before this
    rule existed; their bodies are already public and are left as they are
    rather than rewritten to match a rule that did not apply to them.

    Run: .\scripts\audit_release_notes.ps1
#>
param(
    [string]$ChangelogPath = "CHANGELOG.md"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# The first version whose section was written under this rule. A section is
# audited when it is the working section or a release at or after this one.
$firstAuditedVersion = [version]"2026.9.30"
$marker = "<!-- release-notes:end -->"

# Everything here describes this repository rather than the product. Each entry
# is a regular expression and the reason it is refused, because the failure
# message has to say what to move rather than that something matched.
#
# The script pattern names the repository's own tooling by file type on purpose.
# A project folder keeps its install logic in `scripts/`, which is the product's
# own vocabulary and belongs in a release note; scripts/build.ps1 does not.
$repositoryVocabulary = @(
    @{ Pattern = 'scripts/[\w.\-]+\.(ps1|js|mjs|cjs)\b'; Reason = "a repository script" },
    @{ Pattern = '\.github/'; Reason = "a repository or workflow path" },
    @{ Pattern = '\.ya?ml\b'; Reason = "a workflow or issue-form file" },
    @{ Pattern = 'crates/'; Reason = "a source-tree path" },
    @{ Pattern = 'Cargo\.(toml|lock)'; Reason = "the Rust manifest" },
    @{ Pattern = '\bcargo\s'; Reason = "a Cargo command" },
    @{ Pattern = 'clippy|rustfmt'; Reason = "a lint or format command" },
    @{ Pattern = 'CODE_OF_CONDUCT|CONTRIBUTING|AGENTS\.md'; Reason = "a repository policy file" },
    @{ Pattern = 'llms|docs_index|sitemap|robots\.txt'; Reason = "a documentation-index artifact" },
    @{ Pattern = '分支保护|ruleset|必需检查'; Reason = "a branch-protection rule" },
    @{ Pattern = 'Dependabot|secret scanning'; Reason = "a repository security setting" },
    @{ Pattern = '工作流|\bworkflows?\b|ci\.yml|release\.yml'; Reason = "the build pipeline" },
    @{ Pattern = 'pull request|\bPR\b'; Reason = "the contribution route" },
    @{ Pattern = 'target/'; Reason = "a build output path" }
)

# Sub-sections that hold the repository's own record. None of them describes
# what a download does, so none of them belongs in a release body.
$repositoryHeadings = @("技术细节", "已验证", "未完成", "实现细节")

$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not [System.IO.Path]::IsPathRooted($ChangelogPath)) {
    $ChangelogPath = Join-Path $repoRoot $ChangelogPath
}
if (-not (Test-Path -LiteralPath $ChangelogPath -PathType Leaf)) {
    throw "No changelog at $ChangelogPath"
}

$lines = [System.IO.File]::ReadAllLines($ChangelogPath, [System.Text.UTF8Encoding]::new($false))

# Cut the file into sections: a heading, then everything up to the next one.
$starts = @()
for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^## \[(.+)\]') { $starts += [pscustomobject]@{ Name = $Matches[1]; Index = $i } }
}
if ($starts.Count -eq 0) {
    throw "CHANGELOG.md has no '## [version]' section to audit"
}

$audited = 0
$publishedLines = 0
$problems = @()

for ($s = 0; $s -lt $starts.Count; $s++) {
    $section = $starts[$s]
    $isWorking = $section.Name -eq "未发布"
    if (-not $isWorking) {
        # A suffix such as -r2 names a second release on one calendar day; the
        # date is what decides whether the rule covers the section.
        $version = [version]($section.Name -replace '-r\d+$', '')
        if ($version -lt $firstAuditedVersion) { continue }
    }

    $from = $section.Index + 1
    $to = if ($s + 1 -lt $starts.Count) { $starts[$s + 1].Index - 1 } else { $lines.Count - 1 }
    $body = @($lines[$from..$to])

    $markerIndex = -1
    for ($i = 0; $i -lt $body.Count; $i++) {
        if ($body[$i].Trim() -eq $marker) { $markerIndex = $i; break }
    }

    if ($markerIndex -lt 0) {
        $problems += "## [$($section.Name)] (line $($section.Index + 1)): no '$marker'. Declare the split: the product-facing summary above it, the repository's own record below."
        continue
    }

    $published = @($body[0..($markerIndex - 1)])
    $text = @($published | Where-Object { $_.Trim() })
    if ($text.Count -eq 0) {
        $problems += "## [$($section.Name)] (line $($section.Index + 1)): nothing above '$marker', so the release would publish an empty body."
        continue
    }

    $audited++
    $publishedLines += $text.Count

    for ($i = 0; $i -lt $published.Count; $i++) {
        $line = $published[$i]
        $lineNumber = $from + $i + 1
        $refused = $false
        foreach ($entry in $repositoryVocabulary) {
            $match = [regex]::Match($line, $entry.Pattern)
            if ($match.Success) {
                $problems += "## [$($section.Name)] line ${lineNumber}: '$($line.Trim())' mentions $($entry.Reason) ('$($match.Value)'), which stays below the marker."
                $refused = $true
                break
            }
        }
        if (-not $refused -and $line -match '^###\s+(.+?)\s*$') {
            $heading = $Matches[1]
            if ($repositoryHeadings -contains $heading) {
                $problems += "## [$($section.Name)] line ${lineNumber}: the '### $heading' sub-section holds the repository's own record and belongs below the marker."
            }
        }
    }
}

if ($problems.Count -gt 0) {
    Write-Output "Release notes scope audit failed: $($problems.Count) problem(s) above the marker."
    foreach ($problem in $problems) { Write-Output "  $problem" }
    throw "A published release note section carries repository detail. Move it below '$marker'."
}

Write-Output "Release notes scope audit passed: $audited section(s) in scope, $publishedLines published line(s), no repository detail above the marker"
