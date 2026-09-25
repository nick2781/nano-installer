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

    Everything this script starts joins a job that ends with it, so a process the
    suite leaves behind cannot hold the step's own output open. scripts/step_job.ps1
    says why that matters and what a host that refuses the job does instead.

    The exit code is the suite's own, so a caller can gate on it.
#>
param(
    [string]$Report = "target/e2e-report.txt",
    [switch]$RequireDesktop,
    [string]$Language = "zh-CN",
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

$stubCommand = "cargo build --locked -p nano-installer-stub-lzma -p nano-installer-stub-zlib -p nano-installer-uninstaller"
$suiteCommand = "cargo test --locked -p nano-installer-core --test e2e_setup -- --nocapture"

# A native tool that reports progress on standard error would otherwise trip
# $ErrorActionPreference = "Stop" on a message that is not a failure.
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
        # The shell opens this file for the command, and the command's own
        # children inherit that handle: one that outlives the command still holds
        # the file open, so it is read with sharing allowed -- and a file that
        # cannot be read at all says so instead of throwing the run away.
        try {
            $output = @(Get-CapturedTail -Path $log -Count ([int]::MaxValue))
        }
        catch {
            $output = @("the command's output could not be read: $($_.Exception.Message)")
        }
        Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue
    }
    return @{ Output = @($output | ForEach-Object { ConvertTo-ReportLine "$_" }); ExitCode = $code }
}

# A command that is still running holds its output file open for writing, so
# the tail is read with sharing allowed: what a command that never ends has
# said so far is the only record of where it stopped.
#
# What is read is what the file holds at this moment, and not a byte more. A
# reader that waits for the end of the file waits for the command: a suite with a
# test that never returns prints a line a minute for as long as it runs, so the
# file never ends, and the branch that exists to end a wedged step sits in the
# wedge with it. That is what a step still in progress nineteen minutes after its
# own fifteen-minute deadline is.
function Get-CapturedTail {
    param([string]$Path, [int]$Count)

    if (-not (Test-Path -LiteralPath $Path)) {
        return @()
    }
    $text = ""
    $stream = $null
    try {
        $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
        $length = $stream.Length
        if ($length -gt 0) {
            $buffer = New-Object byte[] $length
            $read = 0
            while ($read -lt $length) {
                $got = $stream.Read($buffer, $read, [int]($length - $read))
                if ($got -le 0) {
                    break
                }
                $read += $got
            }
            $text = [System.Text.Encoding]::UTF8.GetString($buffer, 0, $read)
        }
    }
    finally {
        if ($null -ne $stream) { $stream.Dispose() }
    }
    $lines = @($text -split "\r?\n")
    if ($lines.Count -le $Count) {
        return $lines
    }
    return @($lines[($lines.Count - $Count)..($lines.Count - 1)])
}

# The report says what it was run against, so a green result cannot be read as a
# statement about a tree other than the one it names.
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

$requirements = "NANO_INSTALLER_E2E_REQUIRE_STUBS=1"
$env:NANO_INSTALLER_E2E_REQUIRE_STUBS = "1"
if ($RequireDesktop) {
    $env:NANO_INSTALLER_E2E_REQUIRE_DESKTOP = "1"
    $requirements = "$requirements NANO_INSTALLER_E2E_REQUIRE_DESKTOP=1"
}
$elevated = Get-ReportPhrase -Text $text -Key $(if (Test-Elevated) { "elevated.yes" } else { "elevated.no" })

# A step that stops answering ends anyway: the watchdog is outside this process
# and ends it once its deadline has passed. A step is finished when its output
# closes, and a step that never answers never closes it, so nothing inside the
# step can end it -- not the deadline below, not the runner's own timeouts.
Start-StepWatchdog -Minutes ($SuiteDeadlineMinutes + 4)

Write-Output (Get-ReportPhrase -Text $text -Key "console.buildingstubs")
$stubs = Invoke-NativeStep $stubCommand -DeadlineMinutes $SuiteDeadlineMinutes

$suite = $null
if ($stubs.ExitCode -eq 0) {
    Write-Output (Get-ReportPhrase -Text $text -Key "console.runninge2e")
    $suite = Invoke-NativeStep $suiteCommand -DeadlineMinutes $SuiteDeadlineMinutes
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
Add-ReportField -Lines $lines -Label (Get-ReportPhrase -Text $text -Key "text.elevated") -Value $elevated
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
    @{ Label = (Get-ReportPhrase -Text $text -Key "text.elevated"); Value = $elevated },
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
        Add-CaseResult -Results $caseResults -Path $Matches[1] -Result $Matches[2] -Layer "e2e"
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

$images = @(Get-SnapshotGallery -Directory (Join-Path $repoRoot "target/setup-snapshots") -Text $text -Language $Language)
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
$undescribed = Get-UndescribedCaseNote -Text $text -RepoRoot $repoRoot -Language $Language -Cases $caseRows.ToArray()
if ($undescribed) {
    $notes.Add($undescribed)
}
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
