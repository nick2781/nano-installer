<#
    Runs the setup-level end-to-end suite and writes what it did to a report.

    The suite in crates/nano-installer-core/tests/e2e_setup.rs builds a real
    setup from a project it writes itself, installs it, and then runs the
    uninstaller it deployed, so it needs the three runtime executables the
    builder embeds. This builds them first, then runs the suite, and leaves
    target/e2e-report.txt holding the commit it ran against, the commands, the
    whole output and the summary line, and target/e2e-report.html with the same
    run as a page -- each case with what it holds and how it ended, every
    behaviour the coverage document promises that this suite ran a case for, and
    the pages a capture photographed -- so a result can still be read after the
    terminal that produced it is gone. A CI job keeps both files as artifacts.

    The words both reports say come from report_text.json, chosen by -Language,
    which defaults to zh-CN. The case names, the doc comments above them and the
    suite's own output are what the sources and the tools wrote, and are shown
    as they are.

    NANO_INSTALLER_E2E_REQUIRE_STUBS is set here, because a run in which every
    case skipped must not read as a pass. -RequireDesktop adds
    NANO_INSTALLER_E2E_REQUIRE_DESKTOP, which turns the skipped window case into
    a failure, and belongs on a machine that has a desktop session.

    The exit code is the suite's own, so a caller can gate on it.
#>
param(
    [string]$Report = "target/e2e-report.txt",
    [switch]$RequireDesktop,
    [string]$Language = "zh-CN"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$reportPath = $Report
if (-not [System.IO.Path]::IsPathRooted($reportPath)) {
    $reportPath = Join-Path $repoRoot $reportPath
}
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $reportPath) | Out-Null
$htmlPath = [System.IO.Path]::ChangeExtension($reportPath, ".html")
$reportName = Split-Path -Leaf $reportPath
. (Join-Path $PSScriptRoot "report_text.ps1")
. (Join-Path $PSScriptRoot "report_html.ps1")
. (Join-Path $PSScriptRoot "report_data.ps1")

$text = Get-ReportText -Language $Language

$stubCommand = "cargo build --locked -p nano-installer-stub-lzma -p nano-installer-stub-zlib -p nano-installer-uninstaller"
$suiteCommand = "cargo test --locked -p nano-installer-core --test e2e_setup"

# A native tool that reports progress on standard error would otherwise trip
# $ErrorActionPreference = "Stop" on a message that is not a failure.
function Invoke-NativeStep {
    param([scriptblock]$Command)

    $previous = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $Command 2>&1
        $code = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previous
    }
    return @{ Output = @($output | ForEach-Object { ConvertTo-ReportLine "$_" }); ExitCode = $code }
}

# The report says what it was run against, so a green result cannot be read as a
# statement about a tree other than the one it names.
function Get-CommitDescription {
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        return "unknown, git is not on the path"
    }
    $head = (Invoke-NativeStep { git rev-parse --short HEAD }).Output
    if (-not $head) {
        return "unknown, not a git checkout"
    }
    $changes = (Invoke-NativeStep { git status --porcelain }).Output
    if ($changes) {
        return "$head with uncommitted changes"
    }
    return $head
}

$requirements = "NANO_INSTALLER_E2E_REQUIRE_STUBS=1"
$env:NANO_INSTALLER_E2E_REQUIRE_STUBS = "1"
if ($RequireDesktop) {
    $env:NANO_INSTALLER_E2E_REQUIRE_DESKTOP = "1"
    $requirements = "$requirements NANO_INSTALLER_E2E_REQUIRE_DESKTOP=1"
}

Write-Output (Get-ReportPhrase -Text $text -Key "console.buildingstubs")
$stubs = Invoke-NativeStep { cargo build --locked -p nano-installer-stub-lzma -p nano-installer-stub-zlib -p nano-installer-uninstaller }

$suite = $null
if ($stubs.ExitCode -eq 0) {
    Write-Output (Get-ReportPhrase -Text $text -Key "console.runninge2e")
    $suite = Invoke-NativeStep { cargo test --locked -p nano-installer-core --test e2e_setup }
}
else {
    Write-Output (Get-ReportPhrase -Text $text -Key "console.stubsfailed")
}

$summary = @()
if ($null -ne $suite) {
    $summary = @($suite.Output | Where-Object { $_ -like "test result:*" })
}

$runAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
$commit = Get-CommitDescription

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add((Get-ReportPhrase -Text $text -Key "text.e2e.title"))
Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.runat") -Value $runAt
Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.commit") -Value $commit
Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.required") -Value $requirements
$lines.Add("")
$lines.Add("$ $stubCommand")
$lines.AddRange([string[]]$stubs.Output)
$lines.Add("")
$lines.Add("$ $suiteCommand")
if ($null -ne $suite) {
    $lines.AddRange([string[]]$suite.Output)
}
else {
    $lines.Add((Get-ReportPhrase -Text $text -Key "text.notrun"))
}
$lines.Add("")
if ($summary.Count -gt 0) {
    $lines.AddRange([string[]]$summary)
}
else {
    $lines.Add((Get-ReportPhrase -Text $text -Key "text.nosummary"))
}

$code = 1
if ($null -ne $suite) {
    $code = $suite.ExitCode
}
Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.exitcode") -Value $code
[System.IO.File]::WriteAllLines($reportPath, $lines, (New-Object System.Text.UTF8Encoding($false)))

$passed = 0
$failed = 0
$ignored = 0
foreach ($line in $summary) {
    if ($line -match "(\d+) passed") { $passed += [int]$Matches[1] }
    if ($line -match "(\d+) failed") { $failed += [int]$Matches[1] }
    if ($line -match "(\d+) ignored") { $ignored += [int]$Matches[1] }
}

$stats = @(
    @{ Value = $passed; Label = (Get-ReportPhrase -Text $text -Key "stats.passed"); Tone = "ok" },
    @{ Value = $failed; Label = (Get-ReportPhrase -Text $text -Key "stats.failed"); Tone = $(if ($failed -gt 0) { "bad" } else { "" }) },
    @{ Value = $ignored; Label = (Get-ReportPhrase -Text $text -Key "stats.ignored"); Tone = "muted" },
    @{ Value = $code; Label = (Get-ReportPhrase -Text $text -Key "stats.exitcode"); Tone = $(if ($code -ne 0) { "bad" } else { "" }) }
)
$fields = @(
    @{ Label = (Get-ReportPhrase -Text $text -Key "text.commit"); Value = $commit },
    @{ Label = (Get-ReportPhrase -Text $text -Key "text.required"); Value = $requirements },
    @{ Label = (Get-ReportPhrase -Text $text -Key "fields.runtimeBuild"); Value = $stubCommand },
    @{ Label = (Get-ReportPhrase -Text $text -Key "fields.suite"); Value = $suiteCommand }
)
$sections = New-Object System.Collections.Generic.List[object]
$sections.Add(@{ Heading = (Get-ReportPhrase -Text $text -Key "sections.runtimeBuild"); Summary = "$ $stubCommand"; Lines = $stubs.Output; Open = ($stubs.ExitCode -ne 0) })
if ($null -ne $suite) {
    $sections.Add(@{ Heading = (Get-ReportPhrase -Text $text -Key "sections.suite"); Summary = "$ $suiteCommand"; Lines = $suite.Output; Open = ($code -ne 0) })
}
$suiteLines = @()
if ($null -ne $suite) {
    $suiteLines = @($suite.Output)
}
$catalog = Get-TestCatalog -RepoRoot $repoRoot -Language $Language

# The one layer this suite has, and what the coverage document says its cases
# are for, so the report says what these 18 cases prove and what they do not.
$documentLanguage = Get-ReportDocLanguage -Language $Language
$documentPath = "docs/$documentLanguage/TEST_COVERAGE.md"
$layerDocument = Get-TestLayerCatalog -RepoRoot $repoRoot -Language $Language
$layers = Get-ReportLayerTable -Text $text -Document $layerDocument -Keys @("e2e")
$layerRows = New-Object System.Collections.Generic.List[object]
$layerIds = New-Object System.Collections.Generic.List[string]
foreach ($layer in $layers) {
    $layerIds.Add($layer.Anchor)
    $layerRows.Add(@(
        (New-ReportCell $layer.Name),
        (New-ReportCell $layer.Count),
        (New-ReportCell $layer.Proves),
        (New-ReportCell $layer.CannotProve)
    ))
}
$caseRows = New-Object System.Collections.Generic.List[object]
# Every case this run reported, by name, for the behaviour card: the coverage
# document names a case by its function or by the whole path it sits at.
$caseResults = @{}
foreach ($line in $suiteLines) {
    if ($line -match "^test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:,\s*(.*))?$") {
        Add-CaseResult -Results $caseResults -Path $Matches[1] -Result $Matches[2]
        $note = ""
        if ($Matches.ContainsKey(3)) { $note = $Matches[3] }
        $caseRows.Add((Get-CaseRow -Catalog $catalog -Path $Matches[1] -Result $Matches[2] -Note $note))
    }
}
$caseGroups = @()
$casePassed = 0
$caseFailed = 0
$caseIgnored = 0
if ($caseRows.Count -gt 0) {
    $casePassed = @($caseRows | Where-Object { $_.Result -eq "ok" }).Count
    $caseFailed = @($caseRows | Where-Object { $_.Result -eq "FAILED" }).Count
    $caseIgnored = @($caseRows | Where-Object { $_.Result -eq "ignored" }).Count
    $caseCounts = Get-ReportCounts -Text $text -Passed $casePassed -Failed $caseFailed -Ignored $caseIgnored -All
    $caseHeading = Get-ReportPhrase -Text $text -Key "cases.group" -Values @((Get-ReportPhrase -Text $text -Key "layer.e2e"), "tests\e2e_setup.rs")
    $caseGroups = @(@{ Target = $caseHeading; Counts = $caseCounts; Rows = $caseRows.ToArray() })
}

# This suite holds one layer of what the coverage document promises, so the
# behaviour card says which of those behaviours it ran a case for and how that
# case ran; the rest of the document belongs to the suites that hold it.
$coverageDocument = Get-TestCoverage -RepoRoot $repoRoot -Language $Language
$behaviourRows = Select-BehaviourRow -Rows (Get-BehaviourRows -Coverage $coverageDocument.Rows -Results $caseResults)

$images = @(Get-SnapshotGallery -Directory (Join-Path $repoRoot "target/setup-snapshots") -Text $text)
$verdict = "passed"
if ($code -ne 0 -or $null -eq $suite) { $verdict = "failed" }
$notes = New-Object System.Collections.Generic.List[string]
$notes.Add((Get-ReportPhrase -Text $text -Key "e2e.notes.what"))
$notes.Add((Get-ReportPhrase -Text $text -Key "e2e.notes.stubs"))
if ($caseRows.Count -gt 0) {
    $notes.Add((Get-ReportPhrase -Text $text -Key "e2e.notes.cases" -Values @($caseRows.Count, $casePassed, $caseFailed, $caseIgnored)))
}
$notes.Add((Get-ReportPhrase -Text $text -Key "notes.textfile" -Values @($reportName)))
$notes.Add((Get-ReportPhrase -Text $text -Key "e2e.notes.snapshots"))
$notes.Add((Get-ReportPhrase -Text $text -Key "notes.layers" -Values @($documentPath)))
$guide = Get-ReportGuide -Text $text -DocumentPath $documentPath

Write-ReportHtml -Path $htmlPath -Title (Get-ReportPhrase -Text $text -Key "title.e2e") `
    -Subtitle (Get-ReportPhrase -Text $text -Key "subtitle.runat" -Values @($runAt)) `
    -Language $Language -Text $text -Guide $guide `
    -Verdict $verdict -Fields $fields -Stats $stats -Cases $caseGroups -Images $images `
    -Coverage $behaviourRows `
    -Uncovered $coverageDocument.Uncovered `
    -CoverageIntro (Get-ReportPhrase -Text $text -Key "coverage.intro.partial" -Values @($documentPath)) `
    -Tables @(@{
        Heading = (Get-ReportPhrase -Text $text -Key "layers.heading")
        Headers = @(
            (Get-ReportPhrase -Text $text -Key "layers.column.layer"),
            (Get-ReportPhrase -Text $text -Key "layers.column.cases"),
            (Get-ReportPhrase -Text $text -Key "layers.column.proves"),
            (Get-ReportPhrase -Text $text -Key "layers.column.cannotprove")
        )
        Rows = $layerRows
        RowIds = $layerIds
        NumericFrom = 99
    }) `
    -Sections $sections.ToArray() -Notes $notes.ToArray()

if ($null -ne $suite) {
    $suite.Output | ForEach-Object { Write-Output $_ }
}
Write-Output (Get-ReportPhrase -Text $text -Key "console.exitcode" -Values @($code))
Write-Output (Get-ReportPhrase -Text $text -Key "console.reportwritten" -Values @($reportPath))
Write-Output (Get-ReportPhrase -Text $text -Key "console.pagewritten" -Values @($htmlPath))
exit $code
