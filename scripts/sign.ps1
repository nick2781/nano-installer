param(
    [Parameter(Mandatory = $true)]
    [string]$File
)

$ErrorActionPreference = "Stop"
$thumbprint = $env:NANO_INSTALLER_CERT_THUMBPRINT
$timestampUrl = if ($env:NANO_INSTALLER_TIMESTAMP_URL) {
    $env:NANO_INSTALLER_TIMESTAMP_URL
} else {
    "http://timestamp.digicert.com"
}

if (-not $thumbprint) {
    throw "NANO_INSTALLER_CERT_THUMBPRINT is required"
}
if (-not (Test-Path -LiteralPath $File -PathType Leaf)) {
    throw "File to sign not found: $File"
}

$signTool = Get-Command signtool.exe -ErrorAction Stop
& $signTool.Source sign /sha1 $thumbprint /fd SHA256 /tr $timestampUrl /td SHA256 $File
if ($LASTEXITCODE -ne 0) {
    throw "signtool sign failed with exit code $LASTEXITCODE"
}

& $signTool.Source verify /pa /v $File
if ($LASTEXITCODE -ne 0) {
    throw "signtool verify failed with exit code $LASTEXITCODE"
}
