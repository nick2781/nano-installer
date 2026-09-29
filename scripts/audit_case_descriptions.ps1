<#
    Fails when a case has no description in the language a report is read in.

    A report says what each case checks in the language it is read in, and those
    words come from docs/<language>/TEST_CASES.md: one row per case, kept in
    English and in Chinese. The doc comments above the cases are the sources' own
    English, and they are what a report falls back to for a case its table does
    not name. A case added to the sources without a row in every table would
    quietly read as a doc comment in a report that expects its own words, so the
    gap fails here instead.

    Every language that keeps a table is checked; -Language narrows that to one.

    The cases are read out of the sources the way a report reads them, so this
    needs no build and no run: a function carrying a #[test] attribute is a
    case, and every case has to have a row. A row for something that is no
    longer a case is refused as well, since a table that keeps stale rows stops
    being worth reading.
#>
param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$Language = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "report_data.ps1")

# The languages a report can be read in. Each one keeps its own table beside the
# other documents in its tree, and both are checked unless one is named.
$languages = if ($Language) { @($Language) } else { @("en-US", "zh-CN") }

$failures = New-Object System.Collections.Generic.List[string]
$checked = New-Object System.Collections.Generic.List[string]

foreach ($current in $languages) {
    $documentLanguage = Get-ReportDocLanguage -Language $current
    $documentPath = "docs/$documentLanguage/TEST_CASES.md"
    $document = Join-Path $RepositoryRoot $documentPath
    if (-not (Test-Path -LiteralPath $document -PathType Leaf)) {
        $failures.Add("$documentPath is missing, so a report in $current has no words of its own for its cases")
        continue
    }

    $catalog = Get-TestCatalog -RepoRoot $RepositoryRoot -Language $current
    $cases = 0
    $described = 0
    foreach ($name in ($catalog.Keys | Sort-Object)) {
        $entry = $catalog[$name]
        if ($entry.Source) {
            $cases++
            if ($entry.Described) {
                $described++
            }
            else {
                $failures.Add("$documentPath has no row for the case $name")
            }
            continue
        }
        # A name the coverage document mentions is not a case; only a row of the
        # case table that matches nothing can be stale.
        if ($entry.Described) {
            $failures.Add("$documentPath describes $name, which is not a case in the sources any more")
        }
    }
    $checked.Add("$described of $cases case(s) described in $documentPath")
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    throw "Case description audit failed"
}

Write-Output "Case description audit passed: $($checked -join '; ')"
