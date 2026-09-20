<#
    Fails when a case has no description in the language a report is read in.

    A report says what each case checks in the language it is read in, and on
    the Chinese page those words come from docs/zh-CN/TEST_CASES.md: the doc
    comments above the cases are the sources' own English. A case added to the
    sources without a row there would quietly read as English in a Chinese
    report, which is what a reader of that page cannot tell at a glance, so the
    gap fails here instead.

    The cases are read out of the sources the way a report reads them, so this
    needs no build and no run: a function carrying a #[test] attribute is a
    case, and every case has to have a row. A row for something that is no
    longer a case is refused as well, since a table that keeps stale rows stops
    being worth reading.
#>
param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$Language = "zh-CN"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "report_data.ps1")

$documentLanguage = Get-ReportDocLanguage -Language $Language
$documentPath = "docs/$documentLanguage/TEST_CASES.md"
$document = Join-Path $RepositoryRoot $documentPath
if (-not (Test-Path -LiteralPath $document -PathType Leaf)) {
    Write-Error "$documentPath is missing, so a report in $Language has no words of its own for its cases"
    throw "Case description audit failed"
}

$catalog = Get-TestCatalog -RepoRoot $RepositoryRoot -Language $Language
$cases = 0
$described = 0
$missing = New-Object System.Collections.Generic.List[string]
$stale = New-Object System.Collections.Generic.List[string]
foreach ($name in ($catalog.Keys | Sort-Object)) {
    $entry = $catalog[$name]
    if ($entry.Source) {
        $cases++
        if ($entry.Described) { $described++ } else { $missing.Add($name) }
        continue
    }
    # A name the coverage document mentions is not a case; only a row of the
    # case table that matches nothing can be stale.
    if ($entry.Described) { $stale.Add($name) }
}

foreach ($name in $missing) {
    Write-Error "$documentPath has no row for the case $name"
}
foreach ($name in $stale) {
    Write-Error "$documentPath describes $name, which is not a case in the sources any more"
}
if ($missing.Count -gt 0 -or $stale.Count -gt 0) {
    throw "Case description audit failed"
}

Write-Output "Case description audit passed: $described of $cases case(s) described in $documentPath"
