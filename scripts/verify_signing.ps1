<#
    Checks that a setup is signed the way a release needs it signed.

    An Authenticode signature made with a short-lived certificate is worth
    something only because of its timestamp: Windows validates the chain as it
    stood when the signature was made, so a signature without one stops
    validating the moment the certificate expires, and a certificate that lives
    for days rather than years makes that certain rather than likely.

    This reads a setup and the uninstaller embedded in it, and reports for each
    whether a signature is there, whether it carries a timestamp, who signed it
    and what Windows makes of the chain. Signing belongs to the pipeline that
    ships a product rather than to the builder, so an unsigned setup is only a
    failure when the caller says one is required; a signature that carries no
    timestamp is always a failure, because that is the shape of a signature that
    quietly stops working.

    Run: .\scripts\verify_signing.ps1 -Setup dist\MyProduct.exe
         .\scripts\verify_signing.ps1 -Setup dist\MyProduct.exe -RequireSignature -ExpectPublisher "My Company"
#>
param(
    [Parameter(Mandatory = $true)][string]$Setup,
    [switch]$RequireSignature,
    [string]$ExpectPublisher = "",
    # A chain that ends in a root this machine does not trust is what a test
    # certificate looks like. Checking a test signature is a real thing to want,
    # so the caller says so rather than this script guessing.
    [switch]$AllowUntrustedRoot
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $Setup -PathType Leaf)) {
    throw "no setup at $Setup"
}
$setupPath = (Resolve-Path -LiteralPath $Setup).Path
$problems = New-Object System.Collections.Generic.List[string]
$reported = New-Object System.Collections.Generic.List[string]

function Get-SignatureReport {
    param([string]$Path, [string]$What)
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -eq [System.Management.Automation.SignatureStatus]::NotSigned) {
        if ($RequireSignature) {
            $problems.Add("$What carries no signature, and one was required")
        }
        $reported.Add("$What is not signed")
        return
    }
    $signer = if ($signature.SignerCertificate) { $signature.SignerCertificate.Subject } else { "(no signer certificate)" }
    $stamped = $null -ne $signature.TimeStamperCertificate
    $reported.Add("$What is signed by $signer, $($signature.Status)")
    $reported.Add("  timestamp: $(if ($stamped) { "yes, from $($signature.TimeStamperCertificate.Subject)" } else { 'none' })")
    if (-not $stamped) {
        $problems.Add("$What is signed but carries no timestamp, so the signature expires with the certificate")
    }
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
        $line = "$What does not verify on this machine: $($signature.Status) -- $($signature.StatusMessage)"
        if ($AllowUntrustedRoot) { $reported.Add("  $line") } else { $problems.Add($line) }
    }
    if ($ExpectPublisher -and $signature.SignerCertificate) {
        if ($signature.SignerCertificate.Subject -notlike "*$ExpectPublisher*") {
            $problems.Add("$What is signed by '$($signature.SignerCertificate.Subject)', which does not match '$ExpectPublisher'")
        }
    }
}

# The uninstaller travels inside the setup, and the finalize.uninstaller hook is
# what signs it while it is still a file of its own, so a setup whose own
# signature is fine can still ship an unsigned uninstaller.
function Get-EmbeddedUninstaller {
    param([string]$SetupPath, [string]$Destination)
    $bytes = [System.IO.File]::ReadAllBytes($SetupPath)
    $text = [System.Text.Encoding]::ASCII.GetString($bytes)
    $footerAt = $text.LastIndexOf("NATVEND1", [StringComparison]::Ordinal)
    if ($footerAt -lt 8) { throw "the setup has no bundle footer, so its uninstaller cannot be read" }
    $size = [System.BitConverter]::ToUInt64($bytes, $footerAt - 8)
    # The magic can also sit inside a program's own code, so a footer whose size
    # does not fit the file is not a footer, and saying so beats a cast error.
    if ($size -lt 14 -or $size -gt [uint64]($footerAt - 8)) {
        throw "this file is not a setup: its footer does not describe a bundle"
    }
    $start = [int]($footerAt - 8 - [long]$size)
    if ($start -lt 0) { throw "the setup's bundle size is invalid" }
    $count = [System.BitConverter]::ToUInt32($bytes, $start + 10)
    $cursor = $start + 14
    for ($index = 0; $index -lt $count; $index++) {
        $nameLength = [System.BitConverter]::ToUInt16($bytes, $cursor); $cursor += 2
        $name = [System.Text.Encoding]::UTF8.GetString($bytes, $cursor, $nameLength); $cursor += $nameLength
        $entrySize = [System.BitConverter]::ToUInt64($bytes, $cursor); $cursor += 8 + 32
        if ($name -like "runtime/*.exe") {
            $slice = New-Object byte[] ([int]$entrySize)
            [Array]::Copy($bytes, $cursor, $slice, 0, [int]$entrySize)
            [System.IO.File]::WriteAllBytes($Destination, $slice)
            return $name
        }
        $cursor += $entrySize
    }
    throw "the setup carries no uninstaller"
}

$scratch = Join-Path ([System.IO.Path]::GetTempPath()) ("nano-signing-" + [Guid]::NewGuid().ToString("N"))
$null = New-Item -ItemType Directory -Path $scratch
try {
    Get-SignatureReport -Path $setupPath -What "the setup"
    $uninstaller = Join-Path $scratch "uninst.exe"
    $entry = Get-EmbeddedUninstaller -SetupPath $setupPath -Destination $uninstaller
    Get-SignatureReport -Path $uninstaller -What "the embedded uninstaller ($entry)"
} finally {
    Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction SilentlyContinue
}

foreach ($line in $reported) { Write-Output $line }
if ($problems.Count -gt 0) {
    Write-Output "Signing check failed: $($problems.Count) problem(s)."
    foreach ($problem in $problems) { Write-Output "  $problem" }
    throw "the setup is not signed the way this check requires"
}
Write-Output "Signing check passed: $setupPath"
