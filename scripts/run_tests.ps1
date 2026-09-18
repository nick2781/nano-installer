<#
    Runs the whole workspace test suite and writes what it did to a report.

    The suite is every case the workspace holds: the core library's unit cases,
    the setup-level cases that build and run a real installer, the project
    inspection, the visual builder, and the runtimes. The report keeps the
    commit, the toolchain, the command, the whole output, the result line of
    every target and the totals in target/test-report.txt, so a result can still
    be read after the terminal that produced it is gone. A CI job keeps the same
    file as an artifact.

    The suite's output is captured rather than left on the console this script
    runs on, the same way scripts/run_e2e_setup.ps1 runs its commands. A build
    step is only finished once its output has closed, and the linker starts a
    telemetry process that inherits whatever the build writes to and outlives
    the linker that started it. On a console that process holds the step's pipe
    open after every case has passed, so the step never ends, leaves no log, and
    cannot be cancelled; a pipe this script reads ends with the command.

    The exit code is cargo's own, so a caller can gate on it.
#>
param(
    [string]$Report = "target/test-report.txt"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$reportPath = $Report
if (-not [System.IO.Path]::IsPathRooted($reportPath)) {
    $reportPath = Join-Path $repoRoot $reportPath
}
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $reportPath) | Out-Null

$suiteCommand = "cargo test --locked --workspace"

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

$toolchain = (Invoke-NativeStep { cargo --version }).Output -join "; "
$rustcVersion = (Invoke-NativeStep { rustc --version }).Output -join "; "

Write-Output "Running the workspace suite: $suiteCommand"
$suite = Invoke-NativeStep { cargo test --locked --workspace }

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

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("nano-installer workspace test suite")
$lines.Add("run at      $((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'))")
$lines.Add("commit      $(Get-CommitDescription)")
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
$code = $suite.ExitCode
$lines.Add("exit code $code")
[System.IO.File]::WriteAllLines($reportPath, $lines, (New-Object System.Text.UTF8Encoding($false)))

$printed | ForEach-Object { Write-Output $_ }
Write-Output "exit code $code"
Write-Output "report written to $reportPath"
exit $code
