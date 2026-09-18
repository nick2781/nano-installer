<#
    Runs the whole workspace test suite and writes what it did to a report.

    The suite is every case the workspace holds: the core library's unit cases,
    the setup-level cases that build and run a real installer, the project
    inspection, the visual builder, and the runtimes. The report keeps the
    commit, the toolchain, the command, the whole output, the result line of
    every target and the totals in target/test-report.txt, so a result can still
    be read after the terminal that produced it is gone. A CI job keeps the same
    file as an artifact.

    The suite runs through cmd, which points the build's own handles at a plain
    file. A build step is only finished once its output has closed, and the
    linker starts a telemetry process that inherits whatever the build writes to
    and outlives the linker that started it. On a console, or through a pipe
    that is read to its end, that process holds the step's pipe open after every
    case has passed, so the step never ends, leaves no log, and cannot be
    cancelled. A file has no such reader, and cmd hands the build a file handle
    instead of a pipe.

    The suite is waited on with a deadline rather than through Start-Process
    -Wait, which waits for every process the build started as well, and it is
    not started with a redirected standard output either, which would hand the
    build a .NET pipe to hold. If the suite is still running when the deadline
    passes, the report says so and names what is still alive, so a suite that
    wedges becomes a reported failure instead of a job that never ends.

    The exit code is cargo's own, so a caller can gate on it.
#>
param(
    [string]$Report = "target/test-report.txt",
    [string]$SuiteOutput = "target/tests.txt",
    [string]$SuiteCommand = "cargo test --locked --workspace",
    [int]$TimeoutMinutes = 30
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot

function Resolve-UnderRepo {
    param([string]$Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return Join-Path $repoRoot $Path
}

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

# What a stopped suite had spawned, read from the runner down. Listing the whole
# machine would say nothing about what wedged, and the runner is the only root
# this script knows.
function Get-SuiteProcessList {
    param([int]$RootId)

    $all = @(Get-CimInstance Win32_Process | Select-Object Name, ProcessId, ParentProcessId)
    $tree = @($RootId)
    $frontier = @($RootId)
    while ($frontier.Count -gt 0) {
        $next = @()
        foreach ($candidate in $all) {
            $parent = [int]$candidate.ParentProcessId
            $id = [int]$candidate.ProcessId
            if (($frontier -contains $parent) -and ($tree -notcontains $id)) {
                $tree += $id
                $next += $id
            }
        }
        $frontier = $next
    }
    return @($all |
        Where-Object { $tree -contains [int]$_.ProcessId } |
        Sort-Object Name |
        ForEach-Object { "$($_.Name) (pid $($_.ProcessId), parent $($_.ParentProcessId))" })
}

$reportPath = Resolve-UnderRepo $Report
$outputPath = Resolve-UnderRepo $SuiteOutput
$targetDir = Split-Path -Parent $reportPath
New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outputPath) | Out-Null

$runnerPath = Join-Path $targetDir "run-suite.cmd"
$exitPath = Join-Path $targetDir "run-suite.exit.txt"
foreach ($stale in @($outputPath, $exitPath)) {
    if (Test-Path -LiteralPath $stale) {
        Remove-Item -LiteralPath $stale -Force
    }
}

# cmd redirects the command's own handles, so every process the build starts
# gets a file handle rather than this script's console or a pipe of ours. The
# exit code goes to a file as well, because a code read off a process object
# this script did not wait on comes back empty on PowerShell 5.1.
$runnerLines = @(
    "@echo off",
    "cd /d `"$repoRoot`"",
    "$SuiteCommand > `"$outputPath`" 2>&1",
    "set NANO_INSTALLER_SUITE_EXIT=%ERRORLEVEL%",
    "> `"$exitPath`" echo %NANO_INSTALLER_SUITE_EXIT%",
    "exit /b %NANO_INSTALLER_SUITE_EXIT%"
)
[System.IO.File]::WriteAllLines($runnerPath, $runnerLines, (New-Object System.Text.ASCIIEncoding))

$toolchain = (Invoke-NativeStep { cargo --version }).Output -join "; "
$rustcVersion = (Invoke-NativeStep { rustc --version }).Output -join "; "

Write-Output "Running the workspace suite: $SuiteCommand"
$process = Start-Process -FilePath "cmd.exe" -ArgumentList @("/c", "`"$runnerPath`"") `
    -WindowStyle Hidden -PassThru

$deadline = (Get-Date).AddMinutes($TimeoutMinutes)
while (-not $process.HasExited -and (Get-Date) -lt $deadline) {
    Start-Sleep -Seconds 5
}
$timedOut = -not $process.HasExited
$leftover = @()
if ($timedOut) {
    Write-Output "The suite was still running after $TimeoutMinutes minute(s); stopping what is left of it"
    $leftover = @(Get-SuiteProcessList -RootId $process.Id)
    & taskkill /PID $process.Id /T /F | Out-Null
    Start-Sleep -Seconds 2
}

$code = 1
$exitNote = $null
if ($timedOut) {
    $exitNote = "the suite was stopped, so the run is a failure"
}
elseif (Test-Path -LiteralPath $exitPath) {
    $code = [int]([System.IO.File]::ReadAllText($exitPath).Trim())
}
else {
    $exitNote = "the runner left no exit code, so the run is reported as a failure"
}

$printed = @()
if (Test-Path -LiteralPath $outputPath) {
    $printed = @([System.IO.File]::ReadAllLines($outputPath))
}

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

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("nano-installer workspace test suite")
$lines.Add("run at      $((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'))")
$lines.Add("commit      $(Get-CommitDescription)")
$lines.Add("toolchain   $toolchain")
$lines.Add("rustc       $rustcVersion")
$lines.Add("")
$lines.Add("$ $SuiteCommand")
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
if ($timedOut) {
    $lines.Add("The suite was still running after $TimeoutMinutes minute(s), and was stopped. What was")
    $lines.Add("still alive under the runner when it was stopped:")
    if ($leftover.Count -gt 0) {
        $lines.AddRange([string[]]$leftover)
    }
    else {
        $lines.Add("nothing that was still alive could be traced back to the runner")
    }
}
else {
    $lines.Add("The setup-level cases build and run a real installer, so they need the runtime")
    $lines.Add("executables the builder embeds and skip without them. scripts/run_e2e_setup.ps1")
    $lines.Add("builds those and keeps its own report.")
}
if ($exitNote) {
    $lines.Add("Note: $exitNote.")
}
$lines.Add("exit code $code")
[System.IO.File]::WriteAllLines($reportPath, $lines, (New-Object System.Text.UTF8Encoding($false)))

$printed | ForEach-Object { Write-Output $_ }
Write-Output "exit code $code"
Write-Output "report written to $reportPath"
exit $code
