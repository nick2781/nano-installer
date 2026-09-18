<#
    Hands the captured snapshots to a vision model that runs on this machine.

    capture_setup_snapshots.ps1 proves the arithmetic - the client area, the
    artwork, the text ink, the language switch. Whether the glyphs actually read
    "Install Now" rather than a row of empty boxes is a judgement, and this is
    the half that makes it: it reads the PNGs and manifest.json the capture left
    behind, asks a local model whether each page shows what the manifest says it
    should, and writes review.md beside them.

    The model has to be local: the endpoint defaults to
    http://127.0.0.1:11434/v1, the OpenAI-compatible port Ollama serves, and
    -Endpoint points it at any other local server (LM Studio, llama.cpp, vLLM).
    -Model names the model; without it the first model the endpoint lists is
    used. Screenshots of a customer's installer never leave the machine.

    A machine with no model running is not a failure: the script says so and
    stops, leaving the snapshots and their expectations for a person to read.
#>
param(
    [string]$SnapshotDirectory,
    [string]$Endpoint = $env:NANO_INSTALLER_VISION_ENDPOINT,
    [string]$Model = $env:NANO_INSTALLER_VISION_MODEL
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $SnapshotDirectory) { $SnapshotDirectory = Join-Path $repoRoot "target/setup-snapshots" }
if (-not $Endpoint) { $Endpoint = "http://127.0.0.1:11434/v1" }
$Endpoint = $Endpoint.TrimEnd("/")

$manifestPath = Join-Path $SnapshotDirectory "manifest.json"
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "No manifest at $manifestPath. Run scripts/capture_setup_snapshots.ps1 first."
}
$manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json

# .NET asks the server for permission before sending a large body, and a local
# model server does not answer that handshake: the request would sit waiting for
# a 100-continue that never comes.
[System.Net.ServicePointManager]::Expect100Continue = $false

function Invoke-Model {
    param([string]$Uri, [string]$Body)

    $parameters = @{
        Uri         = $Uri
        Method      = "Post"
        ContentType = "application/json; charset=utf-8"
        Body        = $Body
        TimeoutSec  = 600
    }
    return Invoke-RestMethod @parameters
}

# Nothing to review with is not an error: this step is opt-in, and a machine
# without a model is the normal case in a build pipeline.
$models = $null
try {
    $models = Invoke-RestMethod -Uri "$Endpoint/models" -Method Get -TimeoutSec 5
}
catch {
    Write-Output "No vision model answered at $Endpoint, so the snapshots were not reviewed."
    Write-Output "The pages and what they are meant to show are in $SnapshotDirectory; start a local"
    Write-Output "OpenAI-compatible vision model and run this again to have them read."
    return
}

if (-not $Model) {
    if (-not $models.data -or $models.data.Count -eq 0) {
        throw "$Endpoint listed no models; pass -Model to name one"
    }
    $Model = $models.data[0].id
    Write-Output "Reviewing with $Model at $Endpoint"
}

$instructions = @"
You are checking a screenshot of a Windows installer page against a written description of what it must show.
Compare them and report only what you can see. A description is a requirement, not a hint: a missing image, a wrong language, a clipped word, text drawn as empty boxes, or mojibake all count as failures.
Answer with a single JSON object and nothing else, in this shape:
{"matches": true, "issues": [], "text_read": "the text you can actually read in the image"}
Set "matches" to false when anything in the description is missing or wrong, and list each problem in "issues".
"@

$reviews = New-Object System.Collections.Generic.List[object]
$failures = 0
foreach ($snapshot in $manifest.snapshots) {
    $imagePath = Join-Path $SnapshotDirectory $snapshot.file
    if (-not (Test-Path -LiteralPath $imagePath -PathType Leaf)) {
        throw "Snapshot missing: $imagePath"
    }
    Write-Output "Reviewing $($snapshot.file)"

    $base64 = [Convert]::ToBase64String([System.IO.File]::ReadAllBytes($imagePath))
    $body = [ordered]@{
        model       = $Model
        temperature = 0
        messages    = @(
            [ordered]@{ role = "system"; content = $instructions },
            [ordered]@{
                role    = "user"
                content = @(
                    [ordered]@{ type = "text"; text = $snapshot.expectation },
                    [ordered]@{ type = "image_url"; image_url = [ordered]@{ url = "data:image/png;base64,$base64" } }
                )
            }
        )
    }
    $response = Invoke-Model -Uri "$Endpoint/chat/completions" -Body ($body | ConvertTo-Json -Depth 10)
    if (-not $response.choices -or $response.choices.Count -eq 0) {
        throw "$Model returned no answer for $($snapshot.file)"
    }
    $answer = [string]$response.choices[0].message.content

    # Models like to wrap JSON in a fenced block even when asked not to.
    $json = $answer
    if ($json -match '(?s)\{.*\}') {
        $json = $Matches[0]
    }
    $verdict = $null
    try {
        $verdict = $json | ConvertFrom-Json
    }
    catch {
        $verdict = $null
    }

    if ($null -eq $verdict -or -not ($verdict.PSObject.Properties.Name -contains "matches")) {
        $failures++
        $reviews.Add([ordered]@{
            file   = $snapshot.file
            locale = $snapshot.locale
            dpi    = $snapshot.dpi
            matches = $false
            issues = @("the model did not answer with the requested JSON", $answer)
            text_read = ""
        })
        Write-Output "  unreadable answer"
        continue
    }

    $issues = @($verdict.issues | Where-Object { $_ })
    if (-not $verdict.matches) { $failures++ }
    $reviews.Add([ordered]@{
        file      = $snapshot.file
        locale    = $snapshot.locale
        dpi       = $snapshot.dpi
        matches   = [bool]$verdict.matches
        issues    = $issues
        text_read = [string]$verdict.text_read
    })
    if ($verdict.matches) {
        Write-Output "  matches what it should show"
    }
    else {
        foreach ($issue in $issues) { Write-Output "  $issue" }
    }
}

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("# Snapshot review")
$lines.Add("")
$lines.Add("Model: $Model at $Endpoint")
$lines.Add("")
foreach ($review in $reviews) {
    $verdict = "mismatch"
    if ($review.matches) { $verdict = "matches" }
    $lines.Add("## $($review.file) - $verdict")
    $lines.Add("")
    $lines.Add("Locale $($review.locale) at $($review.dpi) dpi.")
    $lines.Add("")
    if ($review.text_read) {
        $lines.Add("Text the model could read: $($review.text_read)")
        $lines.Add("")
    }
    if ($review.issues.Count -gt 0) {
        $lines.Add("Problems reported:")
        $lines.Add("")
        foreach ($issue in $review.issues) { $lines.Add("- $issue") }
        $lines.Add("")
    }
}
$reportPath = Join-Path $SnapshotDirectory "review.md"
[System.IO.File]::WriteAllLines($reportPath, $lines, (New-Object System.Text.UTF8Encoding($false)))
Write-Output "Wrote $reportPath"

if ($failures -gt 0) {
    throw "$failures of $($reviews.Count) snapshot(s) did not match their description"
}

Write-Output "All $($reviews.Count) snapshot(s) matched their description"
