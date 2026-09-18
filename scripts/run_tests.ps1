<#
    Runs the whole workspace test suite and writes what it did to a report.

    The suite is every case the workspace holds: the core library's unit cases,
    the setup-level cases that build and run a real installer, the project
    inspection, the visual builder, and the runtimes. The report keeps the
    commit, the toolchain, the command, the whole output, the result line of
    every target and the totals in target/test-report.txt, so a result can still
    be read after the terminal that produced it is gone. A CI job keeps the same
    file as an artifact.

    The suite writes to files instead of to this script's console. A build step
    is only finished once its output has closed, and the linker starts a
    telemetry process that inherits whatever the build writes to and outlives
    the linker that started it. On a console that process holds the step's pipe
    open after every case has passed, so the step never ends, leaves no log, and
    cannot be cancelled. A file has no such reader.

    The exit code is cargo's own, so a caller can gate on it.
#>
param(
    [string]$Report = "target/test-report.txt",
    [string]$SuiteOutput = "target/tests.txt",
    [string]$SuiteError = "target/tests.err.txt"
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

$reportPath = Resolve-UnderRepo $Report
$outputPath = Resolve-UnderRepo $SuiteOutput
$errorPath = Resolve-UnderRepo $SuiteError
foreach ($path in @($reportPath, $outputPath, $errorPath)) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
}

$suiteCommand = "cargo test --locked --workspace"
$toolchain = (Invoke-NativeStep { cargo --version }).Output -join "; "
$rustcVersion = (Invoke-NativeStep { rustc --version }).Output -join "; "

Write-Output "Running the workspace suite: $suiteCommand"
$process = Start-Process -FilePath "cargo" -ArgumentList @("test", "--locked", "--workspace") `
    -RedirectStandardOutput $outputPath `
    -RedirectStandardError $errorPath `
    -WindowStyle Hidden -PassThru -Wait
$code = $process.ExitCode

$printed = New-Object System.Collections.Generic.List[string]
foreach ($path in @($outputPath, $errorPath)) {
    if (Test-Path -LiteralPath $path) {
        foreach ($line in [System.IO.File]::ReadAllLines($path)) {
            $printed.Add($line)
        }
    }
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
$lines.Add("$ $suiteCommand")
$lines.AddRange($printed.ToArray())
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

$printed | ForEach-Object { Write-Output $_ }
Write-Output "exit code $code"
Write-Output "report written to $reportPath"
exit $code
