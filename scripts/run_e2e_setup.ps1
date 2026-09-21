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
$suiteCommand = "cargo test --locked -p nano-installer-core --test e2e_setup"

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
function Invoke-NativeStep {
    param([string]$Command, [int]$DeadlineMinutes = 0)

    $log = Join-Path ([System.IO.Path]::GetTempPath()) ("nano-step-{0}.log" -f [guid]::NewGuid().ToString("n"))
    $previous = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $start = New-Object System.Diagnostics.ProcessStartInfo
        $start.FileName = "cmd.exe"
        $start.Arguments = "/c `"$Command > `"$log`" 2>&1`""
        $start.UseShellExecute = $false
        $process = [System.Diagnostics.Process]::Start($start)
        $finished = $true
        if ($DeadlineMinutes -gt 0) {
            $finished = $process.WaitForExit($DeadlineMinutes * 60 * 1000)
        }
        else {
            $process.WaitForExit()
        }
        if (-not $finished) {
            Write-Output "::error title=command deadline::$Command did not end within $DeadlineMinutes minute(s); what it wrote so far follows"
            foreach ($line in (Get-CapturedTail -Path $log -Count 40)) {
                Write-Output "  | $line"
            }
            # The command is taken down here rather than left to the job the step
            # joined. That job is best effort -- scripts/step_job.ps1 says so and prints
            # which of the two happened -- and a process of this command's that outlives
            # the script holds the step's own output open, which is the step that never
            # ends and is archived with no log at all. /T takes what the command started
            # with it, which is where a hung build keeps the process that hangs it.
            foreach ($line in (& taskkill /PID $process.Id /T /F 2>&1)) {
                Write-Output "  | $line"
            }
            Write-Output "everything this step started ends with it now, so the step can end and keep this log"
            exit 124
        }
        $code = $process.ExitCode
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
