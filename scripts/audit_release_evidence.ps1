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
    of the whole suite on it finished successfully. What counts as a whole-suite
    run is the point:

      * Every `push` run does. A push run cannot be given targets of its own:
        only the `workflow_dispatch` input `suite_command` narrows a run.
      * A `workflow_dispatch` run counts when it was not narrowed. `ci.yml` puts
        that input in the name of the job it affects -- a run dispatched with
        targets of its own has a `(narrowed)` job -- so a dispatch that ran the
        suite's own list is evidence of the same thing a push run is. This is the
        way back for a commit whose push run was cancelled: dispatch the suite on
        it, and the release may be published once that run is green.
      * A dispatched run of a commit whose workflow file predates the marker
        cannot be counted at all -- there is no way to tell a whole-suite
        dispatch from a narrowed one after the fact -- so the workflow file at
        the commit is read before any dispatched run is believed. A gate that
        guesses is not a gate.
      * A missing token is a failure rather than a pass, for the same reason.

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

# The marker `ci.yml` puts in the name of the job a narrowed run narrows. A
# dispatch that ran the suite's own list has no such job, which is what makes it
# evidence.
$NarrowedMarker = "\(narrowed\)"

# Whether a dispatched run was narrowed is in its jobs, and that is one more
# question per dispatched run: asked only for the runs that could be the answer.
function Test-NarrowedRun {
    param([string]$Repository, [string]$RunId)
    $raw = & gh api "repos/$Repository/actions/runs/$RunId/jobs?per_page=100" 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "the jobs of run $RunId could not be read, so whether it ran the whole suite cannot be told: $raw"
    }
    $jobs = @(($raw | Out-String | ConvertFrom-Json).jobs)
    return @($jobs | Where-Object { "$($_.name)" -match $NarrowedMarker }).Count -gt 0
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
    $dispatched = @($runs | Where-Object { $_.event -eq "workflow_dispatch" })

    # Read only when a dispatched run is on the commit: a commit whose runs are
    # all pushes needs nothing but the runs themselves.
    $marked = $false
    if ($dispatched.Count -gt 0) {
        $file = & gh api -H "Accept: application/vnd.github.raw" `
            "repos/$Repository/contents/.github/workflows/${Workflow}?ref=$sha" 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "the workflow file at $sha could not be read, so a dispatched run of it cannot be counted: $file"
        }
        $marked = ("$($file | Out-String)" -match $NarrowedMarker)
    }

    # Newest first, and the first run that was the whole suite is the answer: the
    # question is what the commit's last whole-suite run concluded. A push run is
    # always the whole suite, so the search never has to look past one.
    $newest = $null
    foreach ($run in @($runs | Sort-Object { [datetime]$_.created_at } -Descending)) {
        if ($run.event -eq "push") {
            $newest = $run
            break
        }
        if ($run.event -ne "workflow_dispatch" -or -not $marked) {
            continue
        }
        if (-not (Test-NarrowedRun -Repository $Repository -RunId $run.id)) {
            $newest = $run
            break
        }
    }
}
finally {
    $env:GH_TOKEN = $previous
}

if (-not $newest) {
    if ($runs.Count -eq 0) {
        throw "no run of $Workflow on ${sha}: this commit has no suite behind it to publish"
    }
    if ($dispatched.Count -gt 0 -and -not $marked) {
        throw "no whole-suite run of $Workflow on ${sha}: its $($dispatched.Count) dispatched run(s) cannot be counted, because the workflow file at that commit does not mark a narrowed dispatch (no ``(narrowed)`` job). Push a commit that does, and dispatch the suite on it"
    }
    throw "no whole-suite run of $Workflow on ${sha}: every run it has was dispatched with targets of its own. Dispatch the whole suite on this commit and publish after it is green: gh workflow run $Workflow --ref <tag>"
}

$which = "$($newest.event) run $($newest.run_number)"
if ($newest.status -ne "completed") {
    throw "the newest whole-suite run of $Workflow on $sha is $($newest.status) ($which); publish after it finishes"
}
if ($newest.conclusion -ne "success") {
    throw "the newest whole-suite run of $Workflow on $sha is $($newest.conclusion) ($which, $($newest.html_url)); a release is published from a commit whose suite passed. Dispatch the whole suite on this commit and publish after it is green: gh workflow run $Workflow --ref <tag>"
}

Write-Output "Release evidence: $Workflow on $sha passed ($which, $($newest.html_url))"
