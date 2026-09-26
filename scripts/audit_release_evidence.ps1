<#
    Refuses to publish a release whose commit has no green CI run behind it.

    A tag is pushed by hand, and the release workflow builds the assets and
    publishes them without running the suite itself: what stands behind a
    published binary is the CI run on the commit the tag names. That is an
    assumption until it is checked, and it is a bad one here -- a run of this
    repository is ended routinely when a newer push supersedes it, so a commit
    can easily have a cancelled run as its last word, and a release is the one
    artifact that cannot be replaced quietly.

    This asks GitHub for the runs of the commit and fails unless the newest run
    of the suite on it finished successfully. Two things about that choice are
    deliberate:

      * Only `push` runs count. A dispatched run may have been given targets of
        its own (that is what the `suite_command` input in `ci.yml` is for), so
        a green dispatch is not the whole suite, and the release needs the whole
        suite.
      * A missing token is a failure rather than a pass. A gate that opens when
        it cannot see is not a gate.

    Usage:
        .\scripts\audit_release_evidence.ps1 -Commit <sha>
        .\scripts\audit_release_evidence.ps1 -Commit <sha> -Workflow ci.yml -RunsToInspect 50
#>
param(
    [Parameter(Mandatory = $true)][string]$Commit,
    [string]$Repository = $env:GITHUB_REPOSITORY,
    [string]$Workflow = "ci.yml",
    [int]$RunsToInspect = 30
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if (-not $Repository) {
    throw "no repository to ask about; set GITHUB_REPOSITORY or pass -Repository"
}
if (-not $env:GH_TOKEN -and -not $env:GITHUB_TOKEN) {
    throw "no token to ask with; the release gate needs one (GH_TOKEN)"
}
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "gh is not on the path, so this commit's runs cannot be asked about"
}

# The token gh uses is the one the workflow already exports; passing it through
# explicitly keeps this working whichever of the two names a caller set.
$token = if ($env:GH_TOKEN) { $env:GH_TOKEN } else { $env:GITHUB_TOKEN }
$previous = $env:GH_TOKEN
$env:GH_TOKEN = $token
try {
    # The runs endpoint filters on the whole object id and a tag is usually named
    # by hand, so an abbreviated commit is resolved first rather than silently
    # finding no runs at all -- which would read as "this commit was never tested"
    # when what happened is that the question was asked about the wrong name.
    $resolved = & gh api "repos/$Repository/commits/${Commit}" --jq ".sha" 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "the commit $Commit could not be read: $resolved"
    }
    $sha = ($resolved | Out-String).Trim()
    $raw = & gh api "repos/$Repository/actions/workflows/$Workflow/runs?head_sha=$sha&per_page=$RunsToInspect" 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "the runs of $sha could not be read: $raw"
    }
    $runs = @(($raw | Out-String | ConvertFrom-Json).workflow_runs)
}
finally {
    $env:GH_TOKEN = $previous
}

$pushed = @($runs | Where-Object { $_.event -eq "push" } | Sort-Object { [datetime]$_.created_at } -Descending)
if ($pushed.Count -eq 0) {
    throw "no push run of $Workflow on ${sha}: this commit has no suite behind it to publish"
}
$newest = $pushed[0]
if ($newest.status -ne "completed") {
    throw "the newest push run of $Workflow on $sha is $($newest.status); publish after it finishes"
}
if ($newest.conclusion -ne "success") {
    throw "the newest push run of $Workflow on $sha is $($newest.conclusion); a release is published from a commit whose suite passed"
}

Write-Output "Release evidence: $Workflow on $sha passed ($($newest.html_url))"
