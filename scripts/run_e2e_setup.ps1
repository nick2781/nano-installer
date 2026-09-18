<#
    Runs the setup-level end-to-end suite and writes what it did to a report.

    The suite in crates/nano-installer-core/tests/e2e_setup.rs builds a real
    setup from a project it writes itself, installs it, and then runs the
    uninstaller it deployed, so it needs the three runtime executables the
    builder embeds. This builds them first, then runs the suite, and leaves
    target/e2e-report.txt holding the commit it ran against, the commands, the
    whole output and the summary line, so a result can still be read after the
    terminal that produced it is gone. A CI job keeps the same file as an
    artifact.

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

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("nano-installer setup end-to-end suite")
$lines.Add("run at      $((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'))")
$lines.Add("commit      $(Get-CommitDescription)")
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

if ($null -ne $suite) {
    $suite.Output | ForEach-Object { Write-Output $_ }
}
Write-Output "exit code $code"
Write-Output "report written to $reportPath"
exit $code
