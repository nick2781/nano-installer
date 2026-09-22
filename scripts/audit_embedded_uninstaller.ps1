param(
    [Parameter(Mandatory = $true)][string]$Setup,
    [Parameter(Mandatory = $true)][string]$ExpectedName,
    [Parameter(Mandatory = $true)][string]$ExpectedVersion
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if (-not $ExpectedName -or [IO.Path]::GetFileName($ExpectedName) -ne $ExpectedName -or $ExpectedName.Contains("/")) {
    throw "ExpectedName must be an executable filename without directory components"
}
$setupPath = (Resolve-Path -LiteralPath $Setup).Path
$auditDirectory = Join-Path ([IO.Path]::GetTempPath()) ("nano-installer-pe-audit-" + [Guid]::NewGuid().ToString("N"))
$null = New-Item -ItemType Directory -Path $auditDirectory
$uninstallerPath = Join-Path $auditDirectory $ExpectedName
if (-not [IO.Path]::GetFullPath($uninstallerPath).StartsWith($auditDirectory + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Embedded uninstaller target is outside the temporary audit directory"
}

try {
    $stream = [IO.File]::OpenRead($setupPath)
    try {
        if ($stream.Length -lt 16) { throw "Installer has no bundle footer" }
        $reader = [IO.BinaryReader]::new($stream, [Text.Encoding]::UTF8, $true)
        # The footer is searched for near the end of the file rather than taken
        # to be the last thing in it, the way the runtime reads it: a project's
        # own finalize command may have appended a signature behind the bundle.
        $window = [Math]::Min([long]$stream.Length, 1MB)
        $null = $stream.Seek(-$window, [IO.SeekOrigin]::End)
        $tail = $reader.ReadBytes([int]$window)
        $magicAt = [Text.Encoding]::ASCII.GetString($tail).LastIndexOf("NATVEND1", [StringComparison]::Ordinal)
        if ($magicAt -lt 8) { throw "Installer has no bundle footer" }
        $bundleSize = [BitConverter]::ToUInt64($tail, $magicAt - 8)
        $footerAt = ($stream.Length - $window) + $magicAt
        $bundleStart = $footerAt - 8 - [long]$bundleSize
        if ($bundleStart -lt 0) { throw "Installer bundle size is invalid" }
        $null = $stream.Seek($bundleStart, [IO.SeekOrigin]::Begin)
        if ([Text.Encoding]::ASCII.GetString($reader.ReadBytes(8)) -ne "NATVRS01") {
            throw "Installer bundle header is invalid"
        }
        if ($reader.ReadUInt16() -ne 2) { throw "Unsupported installer bundle version" }
        $count = $reader.ReadUInt32()
        $embeddedName = "runtime/$ExpectedName"
        $found = $false
        for ($index = 0; $index -lt $count; $index++) {
            $nameLength = $reader.ReadUInt16()
            $name = [Text.Encoding]::UTF8.GetString($reader.ReadBytes($nameLength))
            $size = $reader.ReadUInt64()
            $digest = $reader.ReadBytes(32)
            if ($digest.Length -ne 32) { throw "Installer bundle entry has a truncated digest: $name" }
            if ($size -gt [UInt64]($stream.Length - $stream.Position)) {
                throw "Installer bundle entry is out of bounds: $name"
            }
            if ($name -eq $embeddedName) {
                if ($size -lt 16 -or $size -gt 32MB) { throw "Embedded uninstaller size is invalid" }
                $output = [IO.File]::Create($uninstallerPath)
                try {
                    $remaining = [long]$size
                    $buffer = [byte[]]::new(65536)
                    while ($remaining -gt 0) {
                        $read = $stream.Read($buffer, 0, [int][Math]::Min($remaining, $buffer.Length))
                        if ($read -le 0) { throw "Embedded uninstaller was truncated" }
                        $output.Write($buffer, 0, $read)
                        $remaining -= $read
                    }
                } finally {
                    $output.Dispose()
                }
                # The bundle records what the entry hashes to; a setup whose
                # embedded uninstaller no longer matches it is not the one the
                # build wrote, and auditing its bytes would say nothing.
                $actual = (Get-FileHash -LiteralPath $uninstallerPath -Algorithm SHA256).Hash.ToLowerInvariant()
                $expected = ($digest | ForEach-Object { $_.ToString("x2") }) -join ""
                if ($actual -ne $expected) {
                    throw "Embedded uninstaller is damaged: recorded $expected, found $actual"
                }
                $found = $true
                break
            }
            $null = $stream.Seek([long]$size, [IO.SeekOrigin]::Current)
        }
        if (-not $found) { throw "Installer is missing $embeddedName" }
    } finally {
        $stream.Dispose()
    }

    $metadata = (Get-Item -LiteralPath $uninstallerPath).VersionInfo
    if ($metadata.FileVersion -ne $ExpectedVersion -or $metadata.OriginalFilename -ne $ExpectedName) {
        throw "Embedded uninstaller VERSIONINFO does not match the project"
    }
    & (Join-Path $PSScriptRoot "audit_win7_imports.ps1") -File $uninstallerPath
    # The uninstaller must carry the same elevation the setup asked for, or the
    # uninstall entry could not undo an elevated installation.
    & (Join-Path $PSScriptRoot "audit_application_manifest.ps1") -File $uninstallerPath
    Write-Output "Embedded uninstaller verified: $embeddedName ($($metadata.FileVersion))"
} finally {
    if (Test-Path -LiteralPath $uninstallerPath) {
        Remove-Item -LiteralPath $uninstallerPath -Force
    }
    Remove-Item -LiteralPath $auditDirectory -Force
}
