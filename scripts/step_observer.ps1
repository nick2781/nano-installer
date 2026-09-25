<#
    Watches a build step from outside it, reports what the step says, and ends it
    when its time is up.

    A step is finished when its output closes, and a step that stops answering
    never closes it: nothing on the runner's side reaches such a step -- not the
    step's own timeout, not the job's, and not `gh run cancel` either -- while the
    log it would have left is archived only once the step ends. Two runs of this
    repository stopped in the same place, at the first report the suite posted
    while a command was running, and left nothing but the commit statuses they had
    already posted: the suite's own process never reached its own deadline, so
    even the branch that exists to say where it stopped said nothing.

    That is the reason this is a separate process rather than more code inside the
    step. It is started with scripts/step_job.ps1's launcher, so it inherits none
    of the step's handles and cannot be the thing holding the step open; it reads
    what the step writes into a crumb file, which is a local append the step makes
    before each thing that could block, and it posts that as commit statuses --
    a commit status is kept by the commit whether the run ends or not. A step
    blocked inside a call still says where it was, because the crumb was written
    before the call; a step whose machine is gone says nothing, and that silence
    is itself the finding.

    It also reads the output file of the command the step is running, so the last
    line a command wrote -- which for a test harness is the name of the test that
    never answered -- reaches the commit statuses even when the step's own read
    never returns.

    Three things go out on separate contexts, so a wedged run leaves all three
    behind: `suite phase` (what the step said), `suite output` (the last line of
    the command it is running) and `suite step` (whether the step's process is
    alive, how much processor time it has used, and how long it has been silent).

    When its deadline passes it ends the step's **whole tree**: the step's own
    process is not the only thing the agent waits for, because a descendant that
    inherited a write of the step's output keeps that output open after the step
    is gone. Measured on a developer machine: with the step's own process exiting
    at once and a descendant living 25 s, the step's output closed 25.2 s later,
    and `Stop-Process` on the step alone left the descendant alive where
    `taskkill /T /F` ended it. The kill is bounded, because `taskkill /T` waits on
    the very tree that may be holding everything up.
#>
param(
    [Parameter(Mandatory = $true)][int]$StepProcessId,
    [Parameter(Mandatory = $true)][int]$Minutes,
    [Parameter(Mandatory = $true)][string]$CrumbPath,
    [int]$KillDeadlineSeconds = 120
)

$ErrorActionPreference = "Continue"
Set-StrictMode -Version Latest

Add-Type -AssemblyName System.Net.Http

$token = $env:GH_TOKEN
$repository = $env:GITHUB_REPOSITORY
$sha = $env:GITHUB_SHA
$api = if ($env:GITHUB_API_URL) { $env:GITHUB_API_URL } else { "https://api.github.com" }

$statusUrl = ""
$runUrl = ""
if ($token -and $repository -and $sha) {
    $statusUrl = "$api/repos/$repository/statuses/$sha"
    $runUrl = "$env:GITHUB_SERVER_URL/$repository/actions/runs/$env:GITHUB_RUN_ID"
}

# One status per call, and every way it can fail is swallowed: this process has
# one job, and a report that cannot be delivered is not a reason to stop
# reporting.
#
# The wait is bounded here rather than left to the client, because a client's own
# timeout is not a bound this code can rely on: a request whose connection is
# black-holed can sit in a call that never returns however that timeout is set,
# which is the defect that used to take the whole record down with the step. A
# post that has not answered in its ten seconds is left where it is -- its socket
# leaks, this process lives on to report the next one -- and counted, so the next
# report says how many were lost, and a machine whose network has gone says so
# instead of going quiet.
$script:LostPosts = 0

function Send-Report {
    param([string]$Context, [string]$Description)

    if (-not $statusUrl) {
        return
    }
    $body = @{
        state       = "success"
        context     = $Context
        description = $Description.Substring(0, [Math]::Min(140, $Description.Length))
        target_url  = $runUrl
    } | ConvertTo-Json -Compress
    $client = New-Object System.Net.Http.HttpClient
    $content = $null
    try {
        $client.Timeout = [TimeSpan]::FromSeconds(10)
        $client.DefaultRequestHeaders.Add("User-Agent", "nano-installer-step-observer")
        $client.DefaultRequestHeaders.Add("Accept", "application/vnd.github+json")
        $client.DefaultRequestHeaders.Add("X-GitHub-Api-Version", "2022-11-28")
        $client.DefaultRequestHeaders.Authorization =
            New-Object System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", $token)
        $content = New-Object System.Net.Http.StringContent($body, [System.Text.Encoding]::UTF8, "application/json")
        $posting = $client.PostAsync($statusUrl, $content)
        if (-not $posting.Wait(10000)) {
            $script:LostPosts = $script:LostPosts + 1
            return
        }
        $response = $posting.Result
        $response.Dispose()
    }
    catch {
        $script:LostPosts = $script:LostPosts + 1
    }
    finally {
        foreach ($disposable in @($content, $client)) {
            if ($null -ne $disposable) {
                try {
                    $disposable.Dispose()
                }
                catch {
                }
            }
        }
    }
}

# The end of a file that is still being written, read with sharing allowed and
# without waiting for it to end: a command that never returns keeps writing.
function Get-OutputTail {
    param([string]$Path)

    if (-not $Path -or -not (Test-Path -LiteralPath $Path)) {
        return ""
    }
    $stream = $null
    try {
        $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
        $length = $stream.Length
        $limit = 64 * 1024
        $start = [Math]::Max(0, $length - $limit)
        if ($start -gt 0) {
            $stream.Position = $start
        }
        $count = [int]($length - $start)
        if ($count -le 0) {
            return ""
        }
        $buffer = New-Object byte[] $count
        $read = 0
        while ($read -lt $count) {
            $got = $stream.Read($buffer, $read, $count - $read)
            if ($got -le 0) {
                break
            }
            $read += $got
        }
        $text = [System.Text.Encoding]::UTF8.GetString($buffer, 0, $read)
        $lines = @($text -split "\r?\n" | Where-Object { $_.Trim() })
        if ($lines.Count -eq 0) {
            return ""
        }
        return $lines[$lines.Count - 1].Trim()
    }
    catch {
        return ""
    }
    finally {
        if ($null -ne $stream) {
            $stream.Dispose()
        }
    }
}

# Ends the step's tree with a deadline of its own, and says what came of it.
function Stop-StepTree {
    param([int]$ProcessId, [int]$DeadlineSeconds)

    $started = New-Object System.Diagnostics.ProcessStartInfo
    $started.FileName = "taskkill.exe"
    $started.Arguments = "/PID $ProcessId /T /F"
    $started.UseShellExecute = $false
    $started.CreateNoWindow = $true
    $started.RedirectStandardOutput = $true
    $started.RedirectStandardError = $true
    try {
        $killer = [System.Diagnostics.Process]::Start($started)
    }
    catch {
        return "the tree could not be taken down: $($_.Exception.Message)"
    }
    if (-not $killer.WaitForExit($DeadlineSeconds * 1000)) {
        try {
            $killer.Kill()
        }
        catch {
        }
        return "taskkill did not end within $DeadlineSeconds second(s)"
    }
    return "the step's tree was ended"
}

$deadline = (Get-Date).AddMinutes([Math]::Max(1, $Minutes))
$consumed = 0
$logPath = ""
$lastLine = ""
$silentSince = Get-Date
$nextHealth = (Get-Date).AddSeconds(30)
$nextOutput = (Get-Date).AddSeconds(30)

while ($true) {
    $alive = $null -ne (Get-Process -Id $StepProcessId -ErrorAction SilentlyContinue)
    if (-not $alive) {
        Send-Report "suite step" "the step's process is gone"
        break
    }

    if (Test-Path -LiteralPath $CrumbPath) {
        $lines = @(Get-Content -LiteralPath $CrumbPath -Encoding UTF8 -ErrorAction SilentlyContinue)
        while ($consumed -lt $lines.Count) {
            $line = $lines[$consumed].Trim()
            $consumed = $consumed + 1
            if (-not $line) {
                continue
            }
            $silentSince = Get-Date
            if ($line.StartsWith("log ")) {
                $logPath = $line.Substring(4).Trim()
                $lastLine = ""
                continue
            }
            if ($line.StartsWith("exit ")) {
                $logPath = ""
            }
            Send-Report "suite phase" $line
        }
    }

    $now = Get-Date
    if ($now -ge $nextOutput) {
        if ($logPath) {
            $line = Get-OutputTail -Path $logPath
            if ($line -and $line -ne $lastLine) {
                $lastLine = $line
                Send-Report "suite output" $line
            }
        }
        $nextOutput = $now.AddSeconds(30)
    }
    if ($now -ge $nextHealth) {
        $process = Get-Process -Id $StepProcessId -ErrorAction SilentlyContinue
        if ($process) {
            $cpu = ""
            try {
                $cpu = "{0:N1}s" -f $process.CPU
            }
            catch {
                $cpu = "unknown"
            }
            $quiet = [int]($now - $silentSince).TotalSeconds
            $lost = ""
            if ($script:LostPosts -gt 0) {
                $lost = ", $script:LostPosts post(s) unanswered"
            }
            Send-Report "suite step" ("pid {0} alive, cpu {1}, threads {2}, quiet {3}s$lost" -f $process.Id, $cpu, $process.Threads.Count, $quiet)
        }
        $nextHealth = $now.AddSeconds(60)
    }

    if ($now -ge $deadline) {
        $said = Stop-StepTree -ProcessId $StepProcessId -DeadlineSeconds $KillDeadlineSeconds
        Send-Report "suite step" ("the step's time is up after {0} minute(s): {1}" -f $Minutes, $said)
        break
    }

    Start-Sleep -Seconds 5
}
