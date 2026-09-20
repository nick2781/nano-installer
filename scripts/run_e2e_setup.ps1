<#
    Runs the setup-level end-to-end suite and writes what it did to a report.

    The suite in crates/nano-installer-core/tests/e2e_setup.rs builds a real
    setup from a project it writes itself, installs it, and then runs the
    uninstaller it deployed, so it needs the three runtime executables the
    builder embeds. This builds them first, then runs the suite, and leaves
    target/e2e-report.txt holding the commit it ran against, the commands, the
    whole output and the summary line, and target/e2e-report.html with the same
    run as a page -- each case with what it holds and how it ended, and the pages
    a capture photographed -- so a result can still be read after the terminal
    that produced it is gone. A CI job keeps both files as artifacts.

    NANO_INSTALLER_E2E_REQUIRE_STUBS is set here, because a run in which every
    case skipped must not read as a pass. -RequireDesktop adds
    NANO_INSTALLER_E2E_REQUIRE_DESKTOP, which turns the skipped window case into
    a failure, and belongs on a machine that has a desktop session.

    The exit code is the suite's own, so a caller can gate on it.
#>
param(
    [string]$Report = "target/e2e-report.txt",
    [switch]$RequireDesktop
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
. (Join-Path $PSScriptRoot "report_html.ps1")
. (Join-Path $PSScriptRoot "report_data.ps1")

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
    return @{ Output = @($output | ForEach-Object { "$_" }); ExitCode = $code }
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

Write-Output "Building the runtime executables the suite embeds"
$stubs = Invoke-NativeStep { cargo build --locked -p nano-installer-stub-lzma -p nano-installer-stub-zlib -p nano-installer-uninstaller }

$suite = $null
if ($stubs.ExitCode -eq 0) {
    Write-Output "Running the setup end-to-end suite"
    $suite = Invoke-NativeStep { cargo test --locked -p nano-installer-core --test e2e_setup }
}
else {
    Write-Output "The runtime build failed, so the suite was not run"
}

$summary = @()
if ($null -ne $suite) {
    $summary = @($suite.Output | Where-Object { $_ -like "test result:*" })
}

$runAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
$commit = Get-CommitDescription

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("nano-installer setup end-to-end suite")
$lines.Add("run at      $runAt")
$lines.Add("commit      $commit")
$lines.Add("required    $requirements")
$lines.Add("")
$lines.Add("$ $stubCommand")
$lines.AddRange([string[]]$stubs.Output)
$lines.Add("")
$lines.Add("$ $suiteCommand")
if ($null -ne $suite) {
    $lines.AddRange([string[]]$suite.Output)
}
else {
    $lines.Add("not run: the runtime build failed")
}
$lines.Add("")
if ($summary.Count -gt 0) {
    $lines.AddRange([string[]]$summary)
}
else {
    $lines.Add("no test result line was printed")
}

$code = 1
if ($null -ne $suite) {
    $code = $suite.ExitCode
}
$lines.Add("exit code $code")
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
    @{ Value = $passed; Label = "passed"; Tone = "ok" },
    @{ Value = $failed; Label = "failed"; Tone = $(if ($failed -gt 0) { "bad" } else { "" }) },
    @{ Value = $ignored; Label = "ignored"; Tone = "muted" },
    @{ Value = $code; Label = "exit code"; Tone = $(if ($code -ne 0) { "bad" } else { "" }) }
)
$fields = @(
    @{ Label = "commit"; Value = $commit },
    @{ Label = "required"; Value = $requirements },
    @{ Label = "runtime build"; Value = $stubCommand },
    @{ Label = "suite"; Value = $suiteCommand }
)
$sections = New-Object System.Collections.Generic.List[object]
$sections.Add(@{ Heading = "Runtime build"; Summary = "$ $stubCommand"; Lines = $stubs.Output; Open = ($stubs.ExitCode -ne 0) })
if ($null -ne $suite) {
    $sections.Add(@{ Heading = "Suite"; Summary = "$ $suiteCommand"; Lines = $suite.Output; Open = ($code -ne 0) })
}
$suiteLines = @()
if ($null -ne $suite) {
    $suiteLines = @($suite.Output)
}
$catalog = Get-TestCatalog -RepoRoot $repoRoot
$caseRows = New-Object System.Collections.Generic.List[object]
foreach ($line in $suiteLines) {
    if ($line -match "^test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:,\s*(.*))?$") {
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
    $caseCounts = "$casePassed passed"
    if ($caseFailed -gt 0) { $caseCounts = "$caseCounts, $caseFailed failed" }
    if ($caseIgnored -gt 0) { $caseCounts = "$caseCounts, $caseIgnored ignored" }
    $caseGroups = @(@{ Target = "tests\e2e_setup.rs"; Counts = $caseCounts; Rows = $caseRows.ToArray() })
}

$images = @(Get-SnapshotGallery -Directory (Join-Path $repoRoot "target/setup-snapshots"))
$verdict = "passed"
if ($code -ne 0 -or $null -eq $suite) { $verdict = "failed" }
$notes = New-Object System.Collections.Generic.List[string]
$notes.Add("The suite writes its own project, builds a setup, runs it, and then runs the uninstaller it deployed.")
$notes.Add("NANO_INSTALLER_E2E_REQUIRE_STUBS makes a run in which every case skipped read as a failure.")
if ($caseRows.Count -gt 0) {
    $notes.Add("Every case the suite printed is listed under Cases: $($caseRows.Count) case(s), $casePassed passed, $caseFailed failed, $caseIgnored ignored.")
}
$notes.Add("target/e2e-report.txt next to this page holds the same run as plain text.")
$notes.Add("The page snapshots come from scripts/capture_setup_snapshots.ps1, which photographs a real setup and needs a desktop session; a machine without one shows none here.")

Write-ReportHtml -Path $htmlPath -Title "Setup end-to-end suite" -Subtitle "run at $runAt" `
    -Verdict $verdict -Fields $fields -Stats $stats -Cases $caseGroups -Images $images `
    -Sections $sections.ToArray() -Notes $notes.ToArray()

if ($null -ne $suite) {
    $suite.Output | ForEach-Object { Write-Output $_ }
}
Write-Output "exit code $code"
Write-Output "report written to $reportPath"
Write-Output "page written to $htmlPath"
exit $code
