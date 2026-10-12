<#
    Holds the documentation's two language trees to each other.

    The published site is an Astro project under site/, built with Cloudflare
    Nimbus, and its pages live in two content trees: site/src/content/docs holds
    the English pages, served at the site root, and site/src/content/zh-CN holds
    the same pages in Chinese, served under /zh-CN. Nimbus mounts each tree as its
    own collection, the sidebar lists one language's pages at a time, and the site
    root is the chooser between them. So a page that only one tree carries is a
    page a reader can reach in one language and not in the other.

    This audit compares the two trees by name: every page in one tree has to have
    its twin in the other, and a page with no twin fails here, in the docs
    workflow and on the pull request, rather than in a browser.

    The repository's map of the documentation is a pair as well, kept under docs/
    by the repository's own conventions: docs/AGENTS.md and docs/zh-CN/AGENTS.md.
    The map is published as pages of the site too, and those two pages are paired
    by the tree comparison above; the two files under docs/ are paired here,
    because nothing else reads them.

    This is what the audit used to be about, and why it changed. It compared
    docs/en and docs/zh-CN with the switch's own list of page names inside
    docs/index.html, so a page added to one tree, or a name added to the switch,
    failed. The site is no longer a docsify shell over those files, the switch has
    no list of its own, and docs/index.html is gone: the content trees are the
    pages, and Nimbus writes the navigation, the index and every Markdown version
    from them. Two trees that match each other are therefore the whole of what
    there is to hold, and there is no third list left to drift.

    Usage:
      .\scripts\audit_docs_languages.ps1
      .\scripts\audit_docs_languages.ps1 -RepositoryRoot C:\src\nano-installer
#>
param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$contentDirectory = Join-Path $RepositoryRoot "site/src/content"
$englishDirectory = Join-Path $contentDirectory "docs"
$chineseDirectory = Join-Path $contentDirectory "zh-CN"

$failures = @()

function Get-PageNames([string]$Directory) {
    if (-not (Test-Path -LiteralPath $Directory -PathType Container)) {
        throw "Not found: $Directory"
    }
    # Every Markdown file in a content tree is a page of the site. The tree holds
    # no navigation fragments: Nimbus builds the sidebar and the table of contents
    # from the pages and the config, so there is no underscore-prefixed file here
    # to leave out.
    $names = @()
    foreach ($file in Get-ChildItem -LiteralPath $Directory -Filter "*.md" -File) {
        $names += $file.Name
    }
    return @($names | Sort-Object)
}

$english = @(Get-PageNames $englishDirectory)
$chinese = @(Get-PageNames $chineseDirectory)
if ($english.Count -eq 0 -or $chinese.Count -eq 0) {
    throw "One of the two documentation trees holds no page"
}

# Every page in one tree has its twin in the other, with no exceptions: a page
# that only one language carries is the thing this audit exists to refuse.
$paired = @()
foreach ($name in $english) {
    if ($chinese -notcontains $name) {
        $failures += "site/src/content/docs/$name has no page in site/src/content/zh-CN"
        continue
    }
    $paired += $name
}
foreach ($name in $chinese) {
    if ($english -contains $name) {
        continue
    }
    $failures += "site/src/content/zh-CN/$name has no page in site/src/content/docs"
}
$paired = @($paired | Sort-Object)

# The map of the documentation, as the repository keeps it: two files at the top
# of docs/, one per language, which is where the repository's conventions ask for
# them. Neither is a page of the site, so the tree comparison above says nothing
# about them; they answer for each other here.
$mapFiles = @("docs/AGENTS.md", "docs/zh-CN/AGENTS.md")
foreach ($path in $mapFiles) {
    if (-not (Test-Path -LiteralPath (Join-Path $RepositoryRoot $path) -PathType Leaf)) {
        $failures += "$path is not there, so the documentation map is written in one language only"
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    throw "The two documentation trees disagree"
}

$message = "Language audit passed: {0} paired page(s) in site/src/content, and docs/AGENTS.md is paired with docs/zh-CN/AGENTS.md" -f $paired.Count
Write-Output $message
