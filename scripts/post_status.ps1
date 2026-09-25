<#
    Posts one commit status, in a process of its own.

    The suite records where it is by posting a commit status: a status is kept by
    the commit whether the run ends or not, so a run that stopped answering still
    says which command it stopped on, while its log -- archived only once a step
    ends -- says nothing at all. The record is worth having only if making it
    cannot cost the run it describes, and a network call inside the suite is
    exactly the kind of thing that can stop answering without a deadline:

    Windows PowerShell implements `Invoke-RestMethod` on `HttpWebRequest`, whose
    `Timeout` covers getting the response and not reading its body, so a response
    that starts and never finishes is a call that never returns however short the
    timeout is. Nothing inside the suite can bound that, and a suite that waits on
    it stops reporting and stops ending -- which is the wedge this record exists
    to describe. The post is therefore made here, where the caller can take the
    whole process down when it does not answer, and this script keeps its timeout
    as the first line of defence rather than the only one.

    It is started with scripts/step_job.ps1's launcher, so it inherits none of the
    caller's handles: a poster still alive when the caller gives up on it cannot
    hold the caller's output open and keep its step from ending.

    The body is passed as a file rather than as text, because a description is
    whatever a test happened to print and quoting it into a command line is a
    thing that breaks on the one description that holds a quote.
#>
param(
    [Parameter(Mandatory = $true)][string]$Url,
    [Parameter(Mandatory = $true)][string]$BodyPath
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$token = $env:GH_TOKEN
if (-not $token) {
    Write-Output "the status could not be posted: no token is in the environment"
    exit 2
}

try {
    $body = [System.IO.File]::ReadAllText($BodyPath)
    $headers = @{
        Authorization          = "Bearer $token"
        Accept                 = "application/vnd.github+json"
        "X-GitHub-Api-Version" = "2022-11-28"
    }
    Invoke-RestMethod -Method Post -TimeoutSec 10 -Uri $Url -Headers $headers -Body $body -ContentType "application/json" | Out-Null
    exit 0
}
catch {
    Write-Output ("the status could not be posted: {0}" -f $_.Exception.Message)
    exit 1
}
