<#
    Holds the two documentation trees and the language switch to each other.

    docs/en and docs/zh-CN are the same pages in two languages, and a reader
    crosses between them with the switch in the site's top bar. Every page has a
    twin: a page inside a tree is paired by name, and a page at the top of docs/
    is paired the same way inside the Chinese tree. The switch knows both lists,
    and this audit compares each with the files beside it.

    A page added to one tree without its twin, or a twin added without a line in
    the switch's list, would send a reader to a file that is there in one language
    and missing in the other. Both fail here, in the docs workflow, rather than in
    a browser.

    The one file that is not a document is the site's landing page: it is a chooser
    between the two languages, so it has no twin to pair and is named as such below.

    Usage:
      .\scripts\audit_docs_languages.ps1
#>
param(
    [string]$DocumentationDirectory
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$docsDirectory = if ($DocumentationDirectory) {
    $DocumentationDirectory
} else {
    Join-Path $repoRoot "docs"
}

if (-not (Test-Path -LiteralPath $docsDirectory)) {
    throw "Not found: $docsDirectory"
}

# The site's landing page is a chooser, not a document, so it is the one page that
# has nothing to be a translation of.
$chooser = "README.md"

$failures = @()

function Get-PageNames([string]$Directory) {
    if (-not (Test-Path -LiteralPath $Directory)) {
        throw "Not found: $Directory"
    }
    $names = @()
    foreach ($file in Get-ChildItem -LiteralPath $Directory -Filter "*.md" -File) {
        # An underscore names the site's own navigation fragment, not a page.
        if ($file.Name.StartsWith("_")) {
            continue
        }
        $names += $file.Name
    }
    return @($names | Sort-Object)
}

$english = @(Get-PageNames (Join-Path $docsDirectory "en"))
$chinese = @(Get-PageNames (Join-Path $docsDirectory "zh-CN"))
$root = @(Get-PageNames $docsDirectory)
if ($english.Count -eq 0 -or $chinese.Count -eq 0) {
    throw "One of the two documentation trees holds no page"
}

# Every page in one tree has its twin in the other, with no exceptions: a page
# that only one language carries is the thing this audit exists to refuse.
$paired = @()
foreach ($name in $english) {
    if ($chinese -notcontains $name) {
        $failures += "docs/en/$name has no page in docs/zh-CN"
        continue
    }
    $paired += $name
}
foreach ($name in $chinese) {
    if ($english -contains $name) {
        continue
    }
    # A Chinese page whose English side sits at the top of docs/ is paired as well.
    if ($root -contains $name) {
        continue
    }
    $failures += "docs/zh-CN/$name has no page in docs/en"
}
$paired = @($paired | Sort-Object)

# The pages at the top of docs/ are the chooser and the pages whose translation
# sits in the Chinese tree under the same name.
$rootPages = @()
foreach ($name in $root) {
    if ($name -eq $chooser) {
        continue
    }
    if ($chinese -notcontains $name) {
        $failures += "docs/$name sits at the top of docs/ and docs/zh-CN/$name is not there"
        continue
    }
    $rootPages += $name
}
$rootPages = @($rootPages | Sort-Object)

# The switch's own lists, read out of the shell they live in.
$shellPath = Join-Path $docsDirectory "index.html"
if (-not (Test-Path -LiteralPath $shellPath -PathType Leaf)) {
    throw "Not found: $shellPath"
}
$shell = Get-Content -Raw -Encoding UTF8 -LiteralPath $shellPath

function Get-SwitchList([string]$Name) {
    $startMarker = "/* nano-language-${Name}:start */"
    $endMarker = "/* nano-language-${Name}:end */"
    $startIndex = $shell.IndexOf($startMarker)
    $endIndex = $shell.IndexOf($endMarker)
    if ($startIndex -lt 0 -or $endIndex -lt $startIndex) {
        throw "$shellPath has no list between $startMarker and $endMarker"
    }
    $names = @()
    foreach ($match in [regex]::Matches($shell.Substring($startIndex, $endIndex - $startIndex), "'([^']+\.md)'")) {
        $names += $match.Groups[1].Value
    }
    return @($names | Sort-Object -Unique)
}

$listed = @(Get-SwitchList "pages")
$listedRoot = @(Get-SwitchList "root-pages")

foreach ($name in $paired) {
    if ($listed -notcontains $name) {
        $failures += "$name is a page in both trees but the language switch does not know it"
    }
}
foreach ($name in $listed) {
    if ($paired -notcontains $name) {
        $failures += "the language switch pairs $name, which is not a page in both trees"
    }
}
foreach ($name in $rootPages) {
    if ($listedRoot -notcontains $name) {
        $failures += "$name is a page at the top of docs/ but the language switch does not know it"
    }
}
foreach ($name in $listedRoot) {
    if ($rootPages -notcontains $name) {
        $failures += "the language switch calls $name a page at the top of docs/, which it is not"
    }
}

# A reader crosses trees through the switch, and a reader of the raw Markdown
# through what each landing page publishes; both need their files to be there.
foreach ($tree in @("en", "zh-CN")) {
    foreach ($name in @("README.md", "_navbar.md", "_sidebar.md")) {
        $path = Join-Path $docsDirectory "$tree/$name"
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            $failures += "docs/$tree/$name is not there, so that tree cannot be read or switched from"
        }
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    throw "The documentation trees and the language switch disagree"
}

$message = "Language audit passed: {0} paired page(s), {1} page(s) at the top of docs/, and the switch lists exactly those" -f $paired.Count, $rootPages.Count
Write-Output $message
