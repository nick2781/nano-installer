<#
    Proves scripts/release_version.ps1 accepts real CalVer dates and rejects the
    ways a version can lie about its date.

    The rule it guards is the one that let v2026.9.19 and v2026.9.20 be published
    on 2026-09-17: nothing tied a release number to the day it was cut. A checker
    that only runs on the release path would let that class of bug back in if it
    were ever weakened, so the cases are pinned here and run on every change.

    "Today" is pinned so the expected answers do not depend on the day the tests
    happen to run.
#>
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$checker = Join-Path $PSScriptRoot "release_version.ps1"
# A fixed instant in the project's calendar day, in the same shape the release
# job runs with.
$now = "2026-09-17T09:00:00+08:00"

function Invoke-Checker {
    param([string[]]$Arguments)

    # Windows PowerShell turns a native command's stderr into an error record,
    # and the checker reports a rejected version that way, so the preference is
    # relaxed around the call and the exit code is what is read.
    $previous = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $output = & powershell -NoProfile -ExecutionPolicy Bypass -File $checker @Arguments 2>&1 | Out-String
        $code = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previous
    }
    return [pscustomobject]@{
        Failed = $code -ne 0
        Output = $output
    }
}

$failures = New-Object System.Collections.Generic.List[string]

function Assert-Accepted {
    param([string[]]$Arguments, [string]$Because)

    # Nothing is returned on purpose: the callers are statements, so a returned
    # object would be written to the output stream and formatted as a table of
    # every case, burying the one line that matters.
    $result = Invoke-Checker $Arguments
    if ($result.Failed) {
        $failures.Add("expected $($Arguments -join ' ') to be accepted ($Because), but it failed: $($result.Output.Trim())")
    }
}

function Assert-Rejected {
    param([string[]]$Arguments, [string]$Because)

    $result = Invoke-Checker $Arguments
    if (-not $result.Failed) {
        $failures.Add("expected $($Arguments -join ' ') to be rejected ($Because), but it was accepted")
    }
}

# Real dates in the shapes the project uses, including a same-day repeat.
Assert-Accepted @("-Tag", "v2026.9.16", "-Now", $now) "a full year with short month and day"
Assert-Accepted @("-Tag", "v2026.9.16-r2", "-Now", $now) "a second release on one day takes a modifier"
Assert-Accepted @("-Version", "2026.9.16-r12", "-Now", $now) "a two-digit modifier is still a modifier"
Assert-Accepted @("-Version", "2026.12.31", "-Now", "2027-1-1T00:00:00+08:00") "a two-digit month or day needs no padding"
Assert-Accepted @("-Tag", "2026.9.17", "-Now", $now) "the leading v is optional"

# Ways a number can claim a day it is not.
Assert-Rejected @("-Tag", "v2026.09.16") "CalVer spells the month unpadded"
Assert-Rejected @("-Tag", "v2026.9.06") "CalVer spells the day unpadded"
Assert-Rejected @("-Tag", "v2026.9.16.1") "a fourth numeric segment is discouraged and unsupported"
Assert-Rejected @("-Tag", "v2026.13.1") "month 13 is not a date"
Assert-Rejected @("-Tag", "v2026.2.30") "February never has 30 days"
Assert-Rejected @("-Tag", "v26.9.16") "the year segment is the full year"
Assert-Rejected @("-Tag", "2026-9-16") "dashes are not the CalVer separator"
Assert-Rejected @("-Tag", "v2026.9") "a release needs a day, not just a month"

# The date has to be possible for the commit being released. These two are the
# tags that were actually published ahead of their date.
Assert-Rejected @("-Tag", "v2026.9.20", "-Commit", "2633482", "-Now", $now) "2026-09-20 had not happened yet"
Assert-Rejected @("-Tag", "v2026.9.19", "-Commit", "cc1a626", "-Now", $now) "2026-09-19 had not happened yet"
Assert-Rejected @("-Version", "2026.9.18", "-Now", $now) "the workspace version cannot be dated ahead either"

# A tag on or after its commit, but not in the future, is fine.
Assert-Accepted @("-Tag", "v2026.9.15", "-Commit", "0bd81a2", "-Now", $now) "the tag is dated the day the commit was made"
Assert-Accepted @("-Tag", "v2026.9.16", "-Commit", "0363f20", "-Now", $now) "an earlier commit may be released later"
Assert-Accepted @("-Tag", "v2026.9.17", "-Commit", "cc1a626", "-Now", $now) "a release may be tagged the day after its commit"
Assert-Accepted @("-Tag", "v2026.9.17-r2", "-Commit", "cc1a626", "-Now", $now) "a modifier does not change the date"
Assert-Rejected @("-Tag", "v2026.9.14", "-Commit", "0bd81a2", "-Now", $now) "a release cannot predate its own code"

# The tag and the workspace version have to name the same release, otherwise the
# built binaries carry a version resource that does not match the release they
# are attached to.
Assert-Accepted @("-Tag", "v2026.9.17", "-Version", "2026.9.17", "-Now", $now) "a tag and the workspace version may name the same release"
Assert-Accepted @("-Tag", "v2026.9.17-r2", "-Version", "2026.9.17-r2", "-Now", $now) "a same-day repeat release agrees on its modifier"
Assert-Rejected @("-Tag", "v2026.9.16", "-Version", "2026.9.17", "-Now", $now) "a tag cannot publish a different date than Cargo.toml declares"
Assert-Rejected @("-Tag", "v2026.9.17-r2", "-Version", "2026.9.17", "-Now", $now) "a same-day repeat needs the modifier on both"
Assert-Rejected @("-Tag", "v2026.9.17", "-Version", "2026.9.17-r2", "-Now", $now) "the modifier cannot exist on only one of them"

# The calendar day is the project's stated UTC+08:00, so a late-evening Beijing
# release keeps that day.
$late = "2026-09-16T23:30:00+08:00"
Assert-Accepted @("-Tag", "v2026.9.16", "-Now", $late) "23:30 in UTC+08:00 is still the 16th"
Assert-Rejected @("-Tag", "v2026.9.17", "-Now", $late) "23:30 in UTC+08:00 is not yet the 17th"

if ($failures.Count -gt 0) {
    throw "release_version.ps1 behaved unexpectedly:`n  - $($failures -join "`n  - ")"
}

Write-Output "CalVer version rules verified: 28 accepted/rejected cases"
