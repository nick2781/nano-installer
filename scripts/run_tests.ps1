<#
    Runs the whole workspace test suite and writes what it did to two reports.

    The suite is every case the workspace holds: the core library's unit cases,
    the setup-level cases that build and run a real installer, the project
    inspection, the visual builder, and the runtimes. Both reports keep the
    commit, the toolchain, the command, the whole output, one row per target, one
    row per case -- and the page also points a target at the cases that ran in
    it -- and the totals in target/test-report.txt and
    target/test-report.html, so a result can still be read after the terminal
    that produced it is gone. The text file is what a log, a diff or a grep
    reads; the page is what a person reads, and it shows each case with what it
    holds. A CI job keeps both as an artifact.

    The page shows the pages a real setup draws too, when
    scripts/capture_setup_snapshots.ps1 has left snapshots in
    target/setup-snapshots: that script needs a desktop session, so a build
    agent has none and its report simply leaves that section out.

    The suite's output is captured rather than left on the console this script
    runs on, the same way scripts/run_e2e_setup.ps1 runs its commands. A build
    step is only finished once its output has closed, and the linker starts a
    telemetry process that inherits whatever the build writes to and outlives
    the linker that started it. On a console that process holds the step's pipe
    open after every case has passed, so the step never ends, leaves no log, and
    cannot be cancelled; a pipe this script reads ends with the command.

    The exit code is cargo's own, so a caller can gate on it. Every phase sends
    the build agent a notice with the time it was reached, so a step that never
    ends, and is therefore archived with no log, still says where it stopped.
#>
param(
    [string]$Report = "target/test-report.txt",
    [string]$Snapshots = "target/setup-snapshots"
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

$suiteCommand = "cargo test --locked --workspace"

# A native tool that reports progress on standard error would otherwise trip
# $ErrorActionPreference = "Stop" on a message that is not a failure.

# What the script is doing goes to the build agent as a notice as well as into
# the reports: a step that never ends is archived with no log at all, and the
# last notice it sent still says how far the script got.
function Write-Phase {
    param([string]$Message)

    $stamp = (Get-Date).ToUniversalTime().ToString("HH:mm:ss")
    Write-Output "::notice title=suite phase::$stamp $Message"
}

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

# The reports say what they were run against, so a green result cannot be read
# as a statement about a tree other than the one they name.
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

function New-ReportCell {
    param([string]$Text, [string]$Class = "", [string]$Title = "", [string]$Link = "", [string]$Sub = "")

    $cell = @{ Text = $Text }
    if ($Class) { $cell["Class"] = $Class }
    if ($Title) { $cell["Title"] = $Title }
    if ($Link) { $cell["Link"] = $Link }
    if ($Sub) { $cell["Sub"] = $Sub }
    return $cell
}

$toolchain = (Invoke-NativeStep { cargo --version }).Output -join "; "
$rustcVersion = (Invoke-NativeStep { rustc --version }).Output -join "; "
$commit = Get-CommitDescription
$runAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

Write-Phase "script started"
Write-Output "Running the workspace suite: $suiteCommand"
$suite = Invoke-NativeStep { cargo test --locked --workspace }

Write-Phase "suite finished with exit code $($suite.ExitCode) and $(@($suite.Output).Count) output line(s)"
$printed = @($suite.Output)
$summary = @($printed | Where-Object { $_ -like "test result:*" })
$skipping = @($printed | Where-Object { $_ -like "skipping:*" })

$passed = 0
$failed = 0
$ignored = 0
foreach ($line in $summary) {
    if ($line -match "(\d+) passed") { $passed += [int]$Matches[1] }
    if ($line -match "(\d+) failed") { $failed += [int]$Matches[1] }
    if ($line -match "(\d+) ignored") { $ignored += [int]$Matches[1] }
}

$code = $suite.ExitCode

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("nano-installer workspace test suite")
$lines.Add("run at      $runAt")
$lines.Add("commit      $commit")
$lines.Add("toolchain   $toolchain")
$lines.Add("rustc       $rustcVersion")
$lines.Add("")
$lines.Add("$ $suiteCommand")
$lines.AddRange([string[]]$printed)
$lines.Add("")
$lines.Add("results, target by target:")
if ($summary.Count -gt 0) {
    $lines.AddRange([string[]]$summary)
}
else {
    $lines.Add("no test result line was printed")
}
$lines.Add("")
$lines.Add("total       $passed passed, $failed failed, $ignored ignored")
if ($skipping.Count -gt 0) {
    $lines.Add("skipped     $($skipping.Count) case(s) reported skipping")
    $lines.AddRange([string[]]$skipping)
}
$lines.Add("")
$lines.Add("The setup-level cases build and run a real installer, so they need the runtime")
$lines.Add("executables the builder embeds and skip without them. scripts/run_e2e_setup.ps1")
$lines.Add("builds those and keeps its own report.")
$lines.Add("exit code $code")
[System.IO.File]::WriteAllLines($reportPath, $lines, (New-Object System.Text.UTF8Encoding($false)))

Write-Phase "result files written"
$catalog = Get-TestCatalog -RepoRoot $repoRoot
Write-Phase "case catalog read: $($catalog.Count) entries"

# One pass over the output builds both tables: a target's row comes from the
# "Running" line that introduces it and the "test result" line that closes it,
# and every case line between them belongs to it.
$rows = New-Object System.Collections.Generic.List[object]
$rowTargets = New-Object System.Collections.Generic.List[string]
$caseGroups = New-Object System.Collections.Generic.List[object]
$cases = $null
$target = "the suite"
foreach ($line in $printed) {
    if ($line -match "^\s+Running (.+)$") {
        $target = $Matches[1]
        $cases = New-Object System.Collections.Generic.List[object]
        $caseGroups.Add(@{ Target = $target; Rows = $cases })
        continue
    }
    if ($line -match "^\s+Doc-tests (.+)$") {
        $target = "doc-tests $($Matches[1])"
        $cases = New-Object System.Collections.Generic.List[object]
        $caseGroups.Add(@{ Target = $target; Rows = $cases })
        continue
    }
    if ($line -match "^test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:,\s*(.*))?$") {
        if ($null -ne $cases) {
            $note = ""
            if ($Matches.ContainsKey(3)) { $note = $Matches[3] }
            $cases.Add((Get-CaseRow -Catalog $catalog -Path $Matches[1] -Result $Matches[2] -Note $note))
        }
        continue
    }
    if ($line -match "^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*finished in (.+?)\s*$") {
        $state = $Matches[1]
        $passedHere = $Matches[2]
        $failedHere = $Matches[3]
        $ignoredHere = $Matches[4]
        $time = $Matches[5]
        $binary = ""
        $name = $target
        if ($name -match "^(.*) \((.*)\)$") {
            $name = $Matches[1]
            $binary = $Matches[2]
        }
        $stateClass = "t-ok"
        if ($state -eq "FAILED") { $stateClass = "t-bad" }
        $rowTargets.Add($target)
        $rows.Add(@(
            (New-ReportCell $name "" $binary "" $binary),
            (New-ReportCell $state $stateClass),
            (New-ReportCell $passedHere),
            (New-ReportCell $failedHere),
            (New-ReportCell $ignoredHere),
            (New-ReportCell $time)
        ))
    }
}

$groups = New-Object System.Collections.Generic.List[object]
foreach ($group in $caseGroups) {
    if ($group.Rows.Count -eq 0) {
        continue
    }
    $groupPassed = @($group.Rows | Where-Object { $_.Result -eq "ok" }).Count
    $groupFailed = @($group.Rows | Where-Object { $_.Result -eq "FAILED" }).Count
    $groupIgnored = @($group.Rows | Where-Object { $_.Result -eq "ignored" }).Count
    $counts = "$groupPassed passed"
    if ($groupFailed -gt 0) { $counts = "$counts, $groupFailed failed" }
    if ($groupIgnored -gt 0) { $counts = "$counts, $groupIgnored ignored" }
    $groups.Add(@{ Target = $group.Target; Counts = $counts; Rows = $group.Rows })
}

# A target row in the target table points at the cases that ran in it, so the
# count a result line reports and the rows below it read as one story: the same
# run, said twice, from two different readings of it.
$anchorByTarget = @{}
for ($index = 0; $index -lt $groups.Count; $index++) {
    $anchorByTarget[$groups[$index].Target] = "#cases-$index"
}
$linked = 0
while ($linked -lt $rows.Count) {
    if ($anchorByTarget.ContainsKey($rowTargets[$linked])) {
        $rows[$linked][0]["Link"] = $anchorByTarget[$rowTargets[$linked]]
    }
    $linked++
}

Write-Phase "target and case tables built"
$snapshotDirectory = $Snapshots
if (-not [System.IO.Path]::IsPathRooted($snapshotDirectory)) {
    $snapshotDirectory = Join-Path $repoRoot $snapshotDirectory
}
$images = @(Get-SnapshotGallery -Directory $snapshotDirectory)

$notes = New-Object System.Collections.Generic.List[string]
if ($skipping.Count -gt 0) {
    $notes.Add("$($skipping.Count) case(s) reported skipping, and are listed above.")
}
# What the cases say happened and what the result lines say happened are two
# readings of one output, so they are compared here: a case line this script
# failed to read would otherwise leave both tables quietly short.
$casesPassed = 0
$casesFailed = 0
$casesIgnored = 0
foreach ($group in $groups) {
    $casesPassed += @($group.Rows | Where-Object { $_.Result -eq "ok" }).Count
    $casesFailed += @($group.Rows | Where-Object { $_.Result -eq "FAILED" }).Count
    $casesIgnored += @($group.Rows | Where-Object { $_.Result -eq "ignored" }).Count
}
$caseTotal = $casesPassed + $casesFailed + $casesIgnored
if ($casesPassed -eq $passed -and $casesFailed -eq $failed -and $casesIgnored -eq $ignored) {
    $notes.Add("Every case the suite printed is listed under Cases: $caseTotal case(s), $casesPassed passed, $casesFailed failed, $casesIgnored ignored, which is what the run's own result lines report.")
}
else {
    $notes.Add("The cases listed under Cases count $casesPassed passed, $casesFailed failed, $casesIgnored ignored, while the run's own result lines count $passed passed, $failed failed, $ignored ignored: a case line could not be read.")
}
if ($images.Count -gt 0) {
    $notes.Add("The page snapshots come from scripts/capture_setup_snapshots.ps1, which photographs a real setup; the checks beside each one are what that capture measured on it.")
}
else {
    $notes.Add("No page snapshots were found in target/setup-snapshots, so this report has none. scripts/capture_setup_snapshots.ps1 photographs a real setup and checks it, and needs a desktop session.")
}
$notes.Add("The setup-level cases build and run a real installer, so they need the runtime executables the builder embeds and skip without them; scripts/run_e2e_setup.ps1 builds those and keeps its own report.")
$notes.Add("target/test-report.txt next to this page holds the same run as plain text.")

$stats = @(
    @{ Value = $passed; Label = "passed"; Tone = "ok" },
    @{ Value = $failed; Label = "failed"; Tone = $(if ($failed -gt 0) { "bad" } else { "" }) },
    @{ Value = $ignored; Label = "ignored"; Tone = "muted" },
    @{ Value = $code; Label = "exit code"; Tone = $(if ($code -ne 0) { "bad" } else { "" }) }
)
$fields = @(
    @{ Label = "commit"; Value = $commit },
    @{ Label = "toolchain"; Value = $toolchain },
    @{ Label = "rustc"; Value = $rustcVersion },
    @{ Label = "command"; Value = $suiteCommand }
)
$verdict = "passed"
if ($failed -gt 0 -or $code -ne 0) { $verdict = "failed" }

Write-ReportHtml -Path $htmlPath -Title "Workspace test suite" -Subtitle "run at $runAt" `
    -Verdict $verdict -Fields $fields -Stats $stats `
    -Tables @(@{
        Heading = "Targets"
        Headers = @("Target", "Result", "Passed", "Failed", "Ignored", "Time")
        Rows = $rows
        NumericFrom = 2
    }) `
    -Cases $groups.ToArray() `
    -Images $images `
    -Sections @(@{ Heading = "Output"; Summary = "$ $suiteCommand"; Lines = $printed; Open = ($verdict -eq "failed") }) `
    -Notes $notes.ToArray()

Write-Phase "page written"
$printed | ForEach-Object { Write-Output $_ }
Write-Output "exit code $code"
Write-Output "report written to $reportPath"
Write-Output "page written to $htmlPath"
exit $code
