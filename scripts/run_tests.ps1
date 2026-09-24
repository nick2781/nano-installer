<#
    Runs the whole workspace test suite and writes what it did to two reports.

    The suite is every case the workspace holds: the core library's unit cases,
    the setup-level cases that build and run a real installer, the project
    inspection, the visual builder, and the runtimes. Both reports keep the
    commit, the toolchain, the command, the whole output, one row per target, one
    row per case -- and the page also points a target at the cases that ran in
    it, and lists every behaviour the coverage document promises with what this
    run ran for it -- and the totals in target/test-report.txt and
    target/test-report.html, so a result can still be read after the terminal
    that produced it is gone. The text file is what a log, a diff or a grep
    reads; the page is what a person reads: it shows each case with what it
    holds, and names the behaviours nothing ran for. A CI job keeps both as an
    artifact.

    The words both reports say come from report_text.json, chosen by -Language,
    which defaults to zh-CN. The case names, the doc comments above them and the
    suite's own output are what the sources and the tools wrote, and are shown
    as they are.

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
    cannot be cancelled; a pipe this script reads ends with the command. What the
    step's own output holds is whatever the step leaves behind -- a wizard a case
    did not close, that same telemetry process -- and the redirect does not cover
    that, so everything this script starts joins a job that ends with it, which
    scripts/step_job.ps1 sets up.

    The exit code is cargo's own, so a caller can gate on it. Every phase sends
    the build agent a notice with the time it was reached, so a step that never
    ends, and is therefore archived with no log, still says where it stopped.
#>
param(
    [string]$Report = "target/test-report.txt",
    [string]$Snapshots = "target/setup-snapshots",
    [string]$Language = "zh-CN",
    [string]$SuiteCommand = "cargo test --locked --workspace",
    [int]$SuiteDeadlineMinutes = 15
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
# Everything this script starts ends when it does, so a process the suite leaves
# behind cannot hold this step's own output open and keep the step from ending.
. (Join-Path $PSScriptRoot "step_job.ps1")

$text = Get-ReportText -Language $Language

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

# The command's output goes into a file the operating system opens for it, and
# not through a pipe this script reads. A pipe stays open for as long as any
# process that inherited its writing end is alive, and the linker leaves a
# telemetry process behind that can outlive the build by hours: read through a
# pipe, a suite that has already finished never returns, no report is written,
# and neither a step timeout nor a cancel ends the step it runs in. cmd.exe sets
# the file handle itself, so what this script reads is not held by the build's
# own children.
#
# A command that does not end at all would hold the step for as long as the
# build agent waits, and a step that never ends is archived with no log, which
# leaves nothing that says where it stopped. A command given a deadline is
# therefore waited on with one: when the deadline passes, the last lines it
# wrote are printed, the script leaves with a code of its own, and the job it
# joined takes down everything it started -- so the step ends, keeps its log,
# and says where it hung.
#
# The command is started as a process this script holds, rather than waited on
# through cmd.exe's own exit status, because waiting with a deadline needs the
# process; its output still goes to a file the operating system opened for it.
#
# That process is given a console of its own, because one created with this
# step's own handles hands this step's output to every process the command
# starts, and a single one of those that outlives the command -- the linker's
# telemetry helper, a wizard a failing case did not close -- then holds the
# step's output open. A step whose output is still open never ends: the build
# agent waits for it however long the job is given, archives no log, and
# neither a step timeout nor a cancel reaches it, which is how a suite that had
# already finished once cost a build agent thirty minutes and said nothing
# about it. A command with a console of its own shares none of this step's
# output, so what it leaves behind cannot hold the step open, and the deadline
# below can always end it.
function Invoke-NativeStep {
    param([string]$Command, [int]$DeadlineMinutes = 0)

    $log = Join-Path ([System.IO.Path]::GetTempPath()) ("nano-step-{0}.log" -f [guid]::NewGuid().ToString("n"))
    $previous = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        # No inherited handles and no console: a build agent gives its step a pipe
        # for output and calls the step finished when that pipe closes, so a
        # process that inherits the pipe and outlives the command keeps the step
        # open however the agent is asked to end it. scripts/step_job.ps1's
        # launcher is what creates the process that way -- `Start-Process` and
        # `Process.Start` both hand the child every inheritable handle this process
        # holds, and the console a command of its own would need is one more thing
        # the step can stop inside.
        $process = [NanoStepCommand]::Start(
            "cmd.exe /c `"$Command > `"$log`" 2>&1`"",
            (Get-Location).ProviderPath)
        try {
            $finished = [NanoStepCommand]::Wait($process, $(if ($DeadlineMinutes -gt 0) { $DeadlineMinutes * 60 * 1000 } else { 0 }))
        }
        catch {
            [NanoStepCommand]::Release($process)
            throw
        }
        if (-not $finished) {
            Write-Output "::error title=command deadline::$Command did not end within $DeadlineMinutes minute(s); what it wrote so far follows"
            foreach ($line in (Get-CapturedTail -Path $log -Count 40)) {
                Write-Output "  | $line"
            }
            # The command is taken down here rather than left to the job the step
            # joined, which is best effort -- scripts/step_job.ps1 says so and prints
            # which of the two happened. /T takes what the command started with it,
            # which is where a hung build keeps the process that hangs it.
            # What is still alive is usually what the command was waiting on, so
            # it is written down before the tree is taken down: a killed tree that
            # left no record of the process it hung on is a hang nobody can read.
            #
            # The list comes from the process table rather than from WMI. A
            # `Get-CimInstance Win32_Process` here can block for as long as the
            # machine is unwell, and a diagnosis that hangs the step it is
            # diagnosing leaves no log at all: this branch is what ends a step the
            # suite wedged, so it must not be able to wedge itself, and the
            # command lines WMI would add are not worth that. A pid, a name, when
            # it started and any window it owns is enough to name a holder.
            Write-Output "the processes still alive while it waited:"
            foreach ($row in (Get-Process | Sort-Object Id)) {
                $detail = ""
                try {
                    $detail = "  started {0}  cpu {1:N1}s" -f $row.StartTime.ToUniversalTime().ToString("HH:mm:ssZ"), $row.CPU
                    if ($row.MainWindowTitle) {
                        $detail += "  ::  " + $row.MainWindowTitle
                    }
                }
                catch {
                    $detail = ""
                }
                Write-Output ("  | pid {0}  {1}{2}" -f $row.Id, $row.ProcessName, $detail)
            }
            # The tree is taken down with a deadline of its own: this branch
            # exists to end a step, so nothing in it may wait without one.
            Stop-ProcessTree -ProcessId ([NanoStepCommand]::Id($process))
            Write-Output "everything this step started ends with it now, so the step can end and keep this log"
            exit 124
        }
        $code = [NanoStepCommand]::ExitCode($process)
        [NanoStepCommand]::Release($process)
    }
    finally {
        $ErrorActionPreference = $previous
    }
    $output = @()
    if (Test-Path -LiteralPath $log) {
        $output = [System.IO.File]::ReadAllLines($log)
        Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue
    }
    return @{ Output = @($output | ForEach-Object { ConvertTo-ReportLine "$_" }); ExitCode = $code }
}

# A command that is still running holds its output file open for writing, so
# the tail is read with sharing allowed: what a command that never ends has
# said so far is the only record of where it stopped.
function Get-CapturedTail {
    param([string]$Path, [int]$Count)

    if (-not (Test-Path -LiteralPath $Path)) {
        return @()
    }
    $lines = New-Object System.Collections.Generic.List[string]
    $stream = $null
    $reader = $null
    try {
        $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
        $reader = New-Object System.IO.StreamReader($stream)
        while (-not $reader.EndOfStream) {
            $lines.Add($reader.ReadLine())
        }
    }
    finally {
        if ($null -ne $reader) { $reader.Dispose() }
        elseif ($null -ne $stream) { $stream.Dispose() }
    }
    if ($lines.Count -le $Count) {
        return @($lines)
    }
    return @($lines[($lines.Count - $Count)..($lines.Count - 1)])
}

# The reports say what they were run against, so a green result cannot be read
# as a statement about a tree other than the one they name.
function Get-CommitDescription {
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        return "unknown, git is not on the path"
    }
    $head = (Invoke-NativeStep "git rev-parse --short HEAD" -DeadlineMinutes 2).Output
    if (-not $head) {
        return "unknown, not a git checkout"
    }
    $changes = (Invoke-NativeStep "git status --porcelain" -DeadlineMinutes 2).Output
    if ($changes) {
        return "$head with uncommitted changes"
    }
    return $head
}

# Whether this process runs elevated decides which branches the suite takes:
# a case that installs a service, or writes a key the whole machine shares,
# holds the refusal of a plain run and the round trip of an elevated one. The
# report names which it was, because a green run cannot otherwise be read for
# those cases.
function Test-Elevated {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

$toolchain = (Invoke-NativeStep "cargo --version" -DeadlineMinutes 2).Output -join "; "
$rustcVersion = (Invoke-NativeStep "rustc --version" -DeadlineMinutes 2).Output -join "; "
$elevated = Get-ReportPhrase -Text $text -Key $(if (Test-Elevated) { "elevated.yes" } else { "elevated.no" })
$commit = Get-CommitDescription
$runAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

Write-Phase "script started"
Write-Output (Get-ReportPhrase -Text $text -Key "console.runningsuite" -Values @($suiteCommand))
$suite = Invoke-NativeStep $suiteCommand -DeadlineMinutes $SuiteDeadlineMinutes

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
$lines.Add((Get-ReportPhrase -Text $text -Key "text.title"))
Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.runat") -Value $runAt
Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.commit") -Value $commit
Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.toolchain") -Value $toolchain
Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.rustc") -Value $rustcVersion
Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.elevated") -Value $elevated
$lines.Add("")
$lines.Add("$ $suiteCommand")
$lines.AddRange([string[]]$printed)
$lines.Add("")
$lines.Add((Get-ReportPhrase -Text $text -Key "text.results"))
if ($summary.Count -gt 0) {
    $lines.AddRange([string[]]$summary)
}
else {
    $lines.Add((Get-ReportPhrase -Text $text -Key "text.nosummary"))
}
$lines.Add("")
$totals = Get-ReportCounts -Text $text -Passed $passed -Failed $failed -Ignored $ignored -All
Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.total") -Value $totals
if ($skipping.Count -gt 0) {
    $skippedCount = Get-ReportPhrase -Text $text -Key "text.skippedcases" -Values @($skipping.Count)
    Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.skipped") -Value $skippedCount
    $lines.AddRange([string[]]$skipping)
}
$lines.Add("")
$lines.AddRange([string[]]((Get-ReportPhrase -Text $text -Key "text.setupnote") -split "\r?\n"))
Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.exitcode") -Value $code
[System.IO.File]::WriteAllLines($reportPath, $lines, (New-Object System.Text.UTF8Encoding($false)))

Write-Phase "result files written"
$catalog = Get-TestCatalog -RepoRoot $repoRoot -Language $Language
Write-Phase "case catalog read: $($catalog.Count) entries"

# Every target below belongs to one layer of the suite, and the coverage
# document says what that layer's cases are for and what passing them does not
# prove, so a target is not left as a binary path and a count. Which layers this
# run has is known once its targets are read, so the table is built after them.
$documentLanguage = Get-ReportDocLanguage -Language $Language
$documentPath = "docs/$documentLanguage/TEST_COVERAGE.md"
$layerDocument = Get-TestLayerCatalog -RepoRoot $repoRoot -Language $Language

# One pass over the output builds both tables: a target's row comes from the
# "Running" line that introduces it and the "test result" line that closes it,
# and every case line between them belongs to it.
$rows = New-Object System.Collections.Generic.List[object]
$rowTargets = New-Object System.Collections.Generic.List[string]
$rowLayers = New-Object System.Collections.Generic.List[string]
$caseGroups = New-Object System.Collections.Generic.List[object]
# Every case this run reported, by name, for the behaviour card: the coverage
# document names a case by its function or by the whole path it sits at.
$caseResults = @{}
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
        Add-CaseResult -Results $caseResults -Path $Matches[1] -Result $Matches[2] -Layer (Get-TargetLayerKey -Target $target)
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
        $stateLabel = Get-ReportResultLabel -Text $text -Result $state
        $rowTargets.Add($target)
        $rowLayers.Add((Get-TargetLayerKey -Target $target))
        $rows.Add(@(
            (New-ReportCell ""),
            (New-ReportCell $name "" $binary "" $binary),
            (New-ReportCell $stateLabel $stateClass),
            (New-ReportCell $passedHere),
            (New-ReportCell $failedHere),
            (New-ReportCell $ignoredHere),
            (New-ReportCell $time)
        ))
    }
}

# One row per layer this run has a target in, and the layer every target row
# names, so a reader can start from what a layer proves rather than from a
# binary path.
# `@()` around a function's return: PowerShell unrolls a one-element array and answers $null
# for an empty one, and a suite that never reached a target -- a build that failed to compile
# is the ordinary way there -- would otherwise kill the script here, before it writes the page
# a reader opens to see what went wrong.
$layers = @(Get-ReportLayerTable -Text $text -Document $layerDocument -Keys @($rowLayers | Select-Object -Unique))
$layerAnchor = Get-ReportLayerAnchor -Layers $layers
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
for ($index = 0; $index -lt $rows.Count; $index++) {
    $layerName = Get-ReportPhrase -Text $text -Key "layer.$($rowLayers[$index])"
    $rows[$index][0] = (New-ReportCell $layerName "" "" $layerAnchor[$layerName])
}
Write-Phase "layer table read: $($layers.Count) layer(s)"

# The page says what was tested, not only what passed: every behaviour the
# coverage document promises, the cases it names for that behaviour, and how
# those cases ran this time -- including the behaviours none of whose cases ran
# at all, and the part of the document where it admits nobody checks a thing.
$coverageDocument = Get-TestCoverage -RepoRoot $repoRoot -Language $Language
$behaviourRows = @(Get-BehaviourRows -Coverage $coverageDocument.Rows -Results $caseResults)
Write-Phase "coverage read: $($behaviourRows.Count) behaviour(s), $($coverageDocument.Uncovered.Count) uncovered section(s)"

$groups = New-Object System.Collections.Generic.List[object]
foreach ($group in $caseGroups) {
    if ($group.Rows.Count -eq 0) {
        continue
    }
    $groupPassed = @($group.Rows | Where-Object { $_.Result -eq "ok" }).Count
    $groupFailed = @($group.Rows | Where-Object { $_.Result -eq "FAILED" }).Count
    $groupIgnored = @($group.Rows | Where-Object { $_.Result -eq "ignored" }).Count
    $counts = Get-ReportCounts -Text $text -Passed $groupPassed -Failed $groupFailed -Ignored $groupIgnored
    $groupLayer = Get-ReportPhrase -Text $text -Key "layer.$(Get-TargetLayerKey -Target $group.Target)"
    $heading = Get-ReportPhrase -Text $text -Key "cases.group" -Values @($groupLayer, $group.Target)
    $groups.Add(@{ Target = $heading; Source = $group.Target; Counts = $counts; Rows = $group.Rows })
}

# A target row in the target table points at the cases that ran in it, so the
# count a result line reports and the rows below it read as one story: the same
# run, said twice, from two different readings of it.
$anchorByTarget = @{}
for ($index = 0; $index -lt $groups.Count; $index++) {
    $anchorByTarget[$groups[$index].Source] = "#cases-$index"
}
$linked = 0
while ($linked -lt $rows.Count) {
    if ($anchorByTarget.ContainsKey($rowTargets[$linked])) {
        $rows[$linked][1]["Link"] = $anchorByTarget[$rowTargets[$linked]]
    }
    $linked++
}

Write-Phase "target and case tables built"
$snapshotDirectory = $Snapshots
if (-not [System.IO.Path]::IsPathRooted($snapshotDirectory)) {
    $snapshotDirectory = Join-Path $repoRoot $snapshotDirectory
}
$images = @(Get-SnapshotGallery -Directory $snapshotDirectory -Text $text -Language $Language)

$notes = New-Object System.Collections.Generic.List[string]
if ($skipping.Count -gt 0) {
    $notes.Add((Get-ReportPhrase -Text $text -Key "notes.skipping" -Values @($skipping.Count)))
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
    $notes.Add((Get-ReportPhrase -Text $text -Key "notes.cases.match" -Values @($caseTotal, $casesPassed, $casesFailed, $casesIgnored)))
}
else {
    $notes.Add((Get-ReportPhrase -Text $text -Key "notes.cases.mismatch" -Values @($casesPassed, $casesFailed, $casesIgnored, $passed, $failed, $ignored)))
}
$undescribed = Get-UndescribedCaseNote -Text $text -RepoRoot $repoRoot -Language $Language -Cases @($groups | ForEach-Object { $_.Rows })
if ($undescribed) {
    $notes.Add($undescribed)
}
if ($images.Count -gt 0) {
    $notes.Add((Get-ReportPhrase -Text $text -Key "notes.snapshots"))
}
else {
    $notes.Add((Get-ReportPhrase -Text $text -Key "notes.nosnapshots"))
}
$notes.Add((Get-ReportPhrase -Text $text -Key "notes.setupcases"))
$notes.Add((Get-ReportPhrase -Text $text -Key "notes.textfile" -Values @($reportName)))
$notes.Add((Get-ReportPhrase -Text $text -Key "notes.layers" -Values @($documentPath)))
$guide = Get-ReportGuide -Text $text -DocumentPath $documentPath

$stats = @(
    @{ Value = $passed; Label = (Get-ReportPhrase -Text $text -Key "stats.passed"); Tone = "ok" },
    @{ Value = $failed; Label = (Get-ReportPhrase -Text $text -Key "stats.failed"); Tone = $(if ($failed -gt 0) { "bad" } else { "" }) },
    @{ Value = $ignored; Label = (Get-ReportPhrase -Text $text -Key "stats.ignored"); Tone = "muted" },
    @{ Value = $code; Label = (Get-ReportPhrase -Text $text -Key "stats.exitcode"); Tone = $(if ($code -ne 0) { "bad" } else { "" }) }
)
$fields = @(
    @{ Label = (Get-ReportPhrase -Text $text -Key "text.commit"); Value = $commit },
    @{ Label = (Get-ReportPhrase -Text $text -Key "text.toolchain"); Value = $toolchain },
    @{ Label = (Get-ReportPhrase -Text $text -Key "text.rustc"); Value = $rustcVersion },
    @{ Label = (Get-ReportPhrase -Text $text -Key "text.elevated"); Value = $elevated },
    @{ Label = (Get-ReportPhrase -Text $text -Key "fields.command"); Value = $suiteCommand }
)
$verdict = "passed"
if ($failed -gt 0 -or $code -ne 0) { $verdict = "failed" }

Write-ReportHtml -Path $htmlPath -Title (Get-ReportPhrase -Text $text -Key "title.tests") `
    -Subtitle (Get-ReportPhrase -Text $text -Key "subtitle.runat" -Values @($runAt)) `
    -Language $Language -Text $text -Guide $guide `
    -Verdict $verdict -Fields $fields -Stats $stats `
    -Tables @(
        @{
            Heading = (Get-ReportPhrase -Text $text -Key "targets.heading")
            Headers = @(
                (Get-ReportPhrase -Text $text -Key "targets.layer"),
                (Get-ReportPhrase -Text $text -Key "targets.target"),
                (Get-ReportPhrase -Text $text -Key "targets.result"),
                (Get-ReportPhrase -Text $text -Key "targets.passed"),
                (Get-ReportPhrase -Text $text -Key "targets.failed"),
                (Get-ReportPhrase -Text $text -Key "targets.ignored"),
                (Get-ReportPhrase -Text $text -Key "targets.time")
            )
            Rows = $rows
            NumericFrom = 3
        },
        @{
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
        }
    ) `
    -Cases $groups.ToArray() `
    -Coverage $behaviourRows `
    -Uncovered $coverageDocument.Uncovered `
    -CoverageIntro (Get-ReportPhrase -Text $text -Key "coverage.intro" -Values @($documentPath)) `
    -Images $images `
    -Sections @(@{ Heading = (Get-ReportPhrase -Text $text -Key "output.heading"); Summary = "$ $suiteCommand"; Lines = $printed; Open = ($verdict -eq "failed") }) `
    -Notes $notes.ToArray()

Write-Phase "page written"
$printed | ForEach-Object { Write-Output $_ }
Write-Output (Get-ReportPhrase -Text $text -Key "console.exitcode" -Values @($code))
Write-Output (Get-ReportPhrase -Text $text -Key "console.reportwritten" -Values @($reportPath))
Write-Output (Get-ReportPhrase -Text $text -Key "console.pagewritten" -Values @($htmlPath))
exit $code
