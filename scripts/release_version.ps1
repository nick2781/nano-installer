<#
    Prints the CalVer version a release should carry, or checks one.

    A release number is the day it is cut, using a full year and a short
    (unpadded) month and day, which is the scheme at https://calver.org/. Three
    numeric segments is as many as the convention recommends, so a second
    release on one day adds a modifier (2026.9.17-r2) rather than a fourth
    number. As the convention also allows, this project states its own calendar
    day: dates are counted in UTC+08:00, so a release cut late in a Beijing
    evening keeps that day's number instead of slipping back one.

    No arguments prints the version for today. That is the value to put in
    Cargo.toml and as the CHANGELOG.md heading.

    -Tag checks a tag name. With -Commit it also checks that the date is
    possible: not before the commit being released, and not after today. A
    release can therefore no longer be numbered for a day that has not happened,
    which is how v2026.9.19 and v2026.9.20 were published on 2026-09-17.

    scripts/build.ps1 checks the workspace version this way, and the release job
    runs the tag check before it builds, so a number that lies about its date
    fails instead of reaching users. The release-notes generator separately
    requires a CHANGELOG.md section for the tag, which is what keeps the tag, the
    version, and the documented release telling the same story.

    -Version checks a bare version string, such as the one in Cargo.toml. Given
    together with -Tag it also requires the two to name the same release, so a
    tag cannot publish binaries whose version resource says something else.

    -UtcOffsetHours changes the calendar day; -Now pins "today" so the checks are
    reproducible in tests.
#>
[CmdletBinding()]
param(
    [string]$Tag,
    [string]$Version,
    [string]$Commit,
    [int]$UtcOffsetHours = 8,
    [datetime]$Now,
    # The repository to read commit dates from. Overridable so the checks can
    # run against a fixture with known dates instead of this repository's
    # history, which a shallow CI checkout does not contain.
    [string]$RepoRoot
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if (-not $RepoRoot) {
    $RepoRoot = Split-Path -Parent $PSScriptRoot
}

# v, a full four-digit year, unpadded month and day, and an optional modifier.
# A padded month or day, or a fourth number, is rejected: the scheme is what
# keeps two spellings of one date from existing.
$scheme = '^(?<year>[0-9]{4})\.(?<month>[0-9]{1,2})\.(?<day>[0-9]{1,2})(?<modifier>-[0-9A-Za-z][0-9A-Za-z.]*)?$'

function Get-CalendarDay {
    param([datetime]$Instant)

    return $Instant.ToUniversalTime().AddHours($UtcOffsetHours)
}

# Resolved once, at script scope: $PSBoundParameters inside a function describes
# that function's own parameters, so reading it there would silently ignore -Now.
$nowInstant = if ($PSBoundParameters.ContainsKey("Now")) { $Now } else { [datetime]::UtcNow }

function Format-CalendarDay {
    param([datetime]$Value)

    return "{0}.{1}.{2}" -f $Value.Year, $Value.Month, $Value.Day
}

function Split-CalVer {
    param([string]$Value, [string]$Original)

    $match = [regex]::Match(($Value -replace '^v', ''), $scheme)
    if (-not $match.Success) {
        throw "'$Original' is not CalVer. Expected <full year>.<short month>.<short day> with an optional '-modifier', such as 2026.9.17."
    }

    $year = $match.Groups["year"].Value
    $month = $match.Groups["month"].Value
    $day = $match.Groups["day"].Value
    # Two spellings of one date is exactly the drift this check exists to stop,
    # and CalVer writes the month and day unpadded.
    if ($month.StartsWith("0")) {
        throw "'$Original' zero-pads its month. CalVer uses the short month, so write $year.$([int]$month).$([int]$day)."
    }
    if ($day.StartsWith("0")) {
        throw "'$Original' zero-pads its day. CalVer uses the short day, so write $year.$([int]$month).$([int]$day)."
    }
    # A month of 13 or a day of 32 would otherwise be normalised into a different
    # date instead of being reported.
    try {
        $null = New-Object datetime([int]$year, [int]$month, [int]$day)
    }
    catch {
        throw "'$Original' is not a real date: $($_.Exception.Message)"
    }

    return [pscustomobject]@{
        Day = New-Object datetime([int]$year, [int]$month, [int]$day)
        Text = "$year.$([int]$month).$([int]$day)"
        Modifier = $match.Groups["modifier"].Value
    }
}

$now = Get-CalendarDay $nowInstant
$today = Format-CalendarDay $now

if ($PSBoundParameters.Count -eq 0) {
    Write-Output $today
    return
}

if (-not $Tag -and -not $Version) {
    throw "Pass -Tag, -Version, or no arguments at all."
}

# A release publishes one version: the tag names it and the workspace declares
# it. If they disagree the built binaries carry a version resource that does not
# match the release they are attached to, so both spellings have to be the same.
if ($Tag -and $Version) {
    $tagged = Split-CalVer -Value $Tag -Original $Tag
    $declared = Split-CalVer -Value $Version -Original $Version
    if ($declared.Day -ne $tagged.Day) {
        throw "Tag '$Tag' is dated $($tagged.Text), but Cargo.toml declares $($declared.Text). The tag has to name the version being built."
    }
    if ($declared.Modifier -ne $tagged.Modifier) {
        throw "Tag '$Tag' carries the modifier '$($tagged.Modifier)', but Cargo.toml declares '$($declared.Modifier)'. A same-day repeat release needs both to agree, such as v2026.9.17-r2 and 2026.9.17-r2."
    }
}

$target = if ($Version) { $Version } else { $Tag }
$parsed = Split-CalVer -Value $target -Original $target
$normalised = "v$($parsed.Text)$($parsed.Modifier)"

# A CalVer number names the day it was cut, so a number from the future names a
# release that has not happened. That is the check the published v2026.9.19 and
# v2026.9.20 both failed.
if ($parsed.Day -gt $now.Date) {
    throw "$target is dated $($parsed.Text), but today is $today. A CalVer version is the day it is cut, so use $today (add a modifier such as '-r2' if that day already has a release)."
}

if ($Commit) {
    # A release also cannot be dated before the code it releases.
    $commitDate = & git -C $RepoRoot show --no-patch --format=%cI $Commit 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Cannot read the date of commit '$Commit': $commitDate"
    }
    $made = Get-CalendarDay ([datetimeoffset]::Parse($commitDate.Trim()).UtcDateTime)
    if ($parsed.Day -lt $made.Date) {
        throw "Tag '$Tag' is dated $($parsed.Text), but commit '$Commit' was made on $(Format-CalendarDay $made). A release cannot be dated before the code it releases."
    }
}

if ($Version) {
    # With both given, the interesting result is that they agree; the tag's own
    # date has already been checked above, so there is nothing left to say.
    if ($Tag) {
        Write-Output "Tag $Tag and version $Version name the same release (today is $today)"
        return
    }
    Write-Output "Version $Version follows CalVer (today is $today)"
    return
}

if (-not $Commit) {
    Write-Output "CalVer tag $normalised is well formed (today is $today)"
    return
}

Write-Output "CalVer tag $normalised is possible for commit $Commit (today is $today)"
