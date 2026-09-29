<#
    Proves the report a run writes survives a run that did nothing.

    The page a failed run leaves behind is the evidence for that run, and it is
    the least exercised path in these scripts: a build that never ran passes null
    where a build that worked passes a collection, and a null dereferenced under
    Set-StrictMode ends the script before it has written the report that says
    what happened. That is not hypothetical. When a dependency bump failed the
    runtime build, run_e2e_setup.ps1 printed "the runtime build failed, so the
    suite was not run" and then died with "The property 'Output' cannot be found
    on this object" -- at report_html.ps1, from a coverage table it was given no
    rows for -- and left no report at all.

    So the renderer is run twice here: once with every collection null, which is
    what a run that never got started passes, and once with one of each thing it
    can be given, so the tables render too. Both pages have to be written and to
    carry their own words. This needs no suite, no build and no desktop, so it
    costs about a second in CI, where the path it guards is otherwise only
    reached on a day something else has already gone wrong.
#>
param(
    [string]$OutputDirectory
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

. (Join-Path $PSScriptRoot "report_html.ps1")
. (Join-Path $PSScriptRoot "report_data.ps1")

$repoRoot = Split-Path -Parent $PSScriptRoot
$directory = if ($OutputDirectory) {
    $OutputDirectory
} else {
    Join-Path ([System.IO.Path]::GetTempPath()) ("nano-reports-" + [Guid]::NewGuid().ToString("N"))
}
New-Item -ItemType Directory -Force -Path $directory | Out-Null

$text = Get-ReportText -Language "en-US"
$failures = @()

# 1. A run that never got started: every collection is null rather than empty,
#    which is what the callers pass when a build did not happen.
$emptyPath = Join-Path $directory "empty-run.html"
Write-ReportHtml -Path $emptyPath -Title "Nothing ran" -Subtitle "a run that did not start" `
    -Verdict "failed" -Language "en-US" -Text $text `
    -Fields $null -Stats $null -Tables $null -Cases $null -Coverage $null `
    -Uncovered $null -Images $null -Sections $null -Notes $null -Guide $null

if (-not (Test-Path -LiteralPath $emptyPath -PathType Leaf)) {
    $failures += "the renderer wrote no page for a run that did nothing"
}
else {
    $empty = [System.IO.File]::ReadAllText($emptyPath, [System.Text.UTF8Encoding]::new($false))
    if ($empty.Length -eq 0) {
        $failures += "the page for a run that did nothing is empty"
    }
    if (-not $empty.Contains("Nothing ran")) {
        $failures += "the page for a run that did nothing does not carry its own title"
    }
}

# 2. A run with one of everything, built by the same readers the suites use, so
#    the shapes are the shapes a real report carries rather than ones invented
#    here: one case from the catalog, and a behaviour that names it.
$catalog = Get-TestCatalog -RepoRoot $repoRoot -Language "en-US"
$caseNames = @($catalog.Keys | Sort-Object)
if ($caseNames.Count -eq 0) {
    throw "No cases were read out of the sources, so this check cannot build a report with one"
}
$caseName = $caseNames[0]
$caseRow = Get-CaseRow -Catalog $catalog -Path "report_check::$caseName" -Result "FAILED" -Note ""

$fullPath = Join-Path $directory "full-run.html"
Write-ReportHtml -Path $fullPath -Title "A run with rows" -Subtitle "every part filled in" `
    -Verdict "failed" -Language "en-US" -Text $text `
    -Fields @(@{ Label = "commit"; Value = "0000000" }) `
    -Stats @(@{ Value = 1; Label = "failed"; Tone = "bad" }) `
    -Tables @(@{
        Heading     = "Targets"
        Headers     = @("target", "result")
        Rows        = @(@(@{ Text = "a_target" }, @{ Text = "failed" }))
        NumericFrom = 1
    }) `
    -Cases @(@{
        Target = "a_target"
        Counts = "1 case(s)"
        Rows   = @($caseRow)
    }) `
    -Coverage @(@{
        Section   = "A section"
        Behaviour = "a behaviour that a case holds"
        Verdict   = "FAILED"
        Cases     = @(@{ Name = $caseName; Result = "FAILED"; Layer = "core" })
    }) `
    -Uncovered @(@{ Section = "A section nothing covers"; Lines = @("a line from the document") }) `
    -Sections @(@{ Heading = "Runtime build"; Summary = "$ a command"; Lines = @("output of the build"); Open = $true })

if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
    $failures += "the renderer wrote no page for a run with rows"
}
else {
    $full = [System.IO.File]::ReadAllText($fullPath, [System.Text.UTF8Encoding]::new($false))
    foreach ($expected in @($caseName, "a behaviour that a case holds", "output of the build", "a_target")) {
        if (-not $full.Contains($expected)) {
            $failures += "the page for a run with rows does not mention $expected"
        }
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    throw "The report renderer did not survive the runs it was given"
}

$emptySize = (Get-Item -LiteralPath $emptyPath).Length
$fullSize = (Get-Item -LiteralPath $fullPath).Length
if (-not $OutputDirectory) {
    Remove-Item -LiteralPath $directory -Recurse -Force
}
Write-Output "Report renderer verified: a run that did nothing wrote $emptySize bytes, a run with rows wrote $fullSize bytes"
