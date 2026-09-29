<#
    Holds the two documentation trees and the language switch to each other.

    docs/en and docs/zh-CN are the same pages in two languages, and a reader
    crosses between them with the switch in the site's top bar. That switch moves
    a reader from a page to the same page in the other tree, which works only
    while both trees carry the same file names, so the names it knows are listed
    in docs/index.html between the markers this audit reads.

    A page added to one tree without its twin, or a twin added without a line in
    that list, sends a reader to a file that is there in one language and missing
    in the other. Both fail here, in the docs workflow, rather than in a browser.

    A document that carries one language on purpose is not something the switch
    can pair, so it is named below with the reason; the switch sends a reader who
    asks for it to that language's landing page instead.

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

# Documents that live in one tree only, and why. A page here is one the switch
# cannot pair, so it is a decision rather than an oversight.
$singleTree = @{
    "TEST_CASES.md" = "the case descriptions the test report reads are written in Chinese only"
}

$failures = @()

function Get-PageNames([string]$Tree) {
    $directory = Join-Path $docsDirectory $Tree
    if (-not (Test-Path -LiteralPath $directory)) {
        throw "Not found: $directory"
    }
    $names = @()
    foreach ($file in Get-ChildItem -LiteralPath $directory -Filter "*.md" -File) {
        # An underscore names the site's own navigation fragment, not a page.
        if ($file.Name.StartsWith("_")) {
            continue
        }
        $names += $file.Name
    }
    return @($names | Sort-Object)
}

$english = @(Get-PageNames "en")
$chinese = @(Get-PageNames "zh-CN")
if ($english.Count -eq 0 -or $chinese.Count -eq 0) {
    throw "One of the two documentation trees holds no page"
}

$paired = @()
foreach ($name in $english) {
    if ($chinese -contains $name) {
        $paired += $name
        continue
    }
    if (-not $singleTree.ContainsKey($name)) {
        $failures += "docs/en/$name has no page in docs/zh-CN and is not a declared single-language document"
    }
}
foreach ($name in $chinese) {
    if ($english -contains $name) {
        continue
    }
    if (-not $singleTree.ContainsKey($name)) {
        $failures += "docs/zh-CN/$name has no page in docs/en and is not a declared single-language document"
    }
}
$paired = @($paired | Sort-Object)

# The switch's own list, read out of the shell it lives in.
$shellPath = Join-Path $docsDirectory "index.html"
if (-not (Test-Path -LiteralPath $shellPath -PathType Leaf)) {
    throw "Not found: $shellPath"
}
$shell = Get-Content -Raw -Encoding UTF8 -LiteralPath $shellPath
$startMarker = "/* nano-language-pages:start */"
$endMarker = "/* nano-language-pages:end */"
$startIndex = $shell.IndexOf($startMarker)
$endIndex = $shell.IndexOf($endMarker)
if ($startIndex -lt 0 -or $endIndex -lt $startIndex) {
    throw "$shellPath has no page list between $startMarker and $endMarker"
}
$listed = @()
foreach ($match in [regex]::Matches($shell.Substring($startIndex, $endIndex - $startIndex), "'([^']+\.md)'")) {
    $listed += $match.Groups[1].Value
}
$listed = @($listed | Sort-Object -Unique)

foreach ($name in $paired) {
    if ($listed -notcontains $name) {
        $failures += "$name is a page in both trees but the language switch does not know it"
    }
}
foreach ($name in $listed) {
    if ($paired -notcontains $name) {
        $failures += "the language switch lists $name, which is not a page in both trees"
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

$message = "Language audit passed: {0} paired page(s), {1} single-language document(s), and the switch lists exactly the paired pages" -f $paired.Count, $singleTree.Count
Write-Output $message
