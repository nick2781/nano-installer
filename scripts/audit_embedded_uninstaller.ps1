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
        $null = $stream.Seek(-16, [IO.SeekOrigin]::End)
        $bundleSize = $reader.ReadUInt64()
        if ([Text.Encoding]::ASCII.GetString($reader.ReadBytes(8)) -ne "NATVEND1") {
            throw "Installer has an invalid bundle footer"
        }
        $bundleStart = $stream.Length - 16 - [long]$bundleSize
        if ($bundleStart -lt 0) { throw "Installer bundle size is invalid" }
        $null = $stream.Seek($bundleStart, [IO.SeekOrigin]::Begin)
        if ([Text.Encoding]::ASCII.GetString($reader.ReadBytes(8)) -ne "NATVRS01") {
            throw "Installer bundle header is invalid"
        }
        if ($reader.ReadUInt16() -ne 1) { throw "Unsupported installer bundle version" }
        $count = $reader.ReadUInt32()
        $embeddedName = "runtime/$ExpectedName"
        $found = $false
        for ($index = 0; $index -lt $count; $index++) {
            $nameLength = $reader.ReadUInt16()
            $name = [Text.Encoding]::UTF8.GetString($reader.ReadBytes($nameLength))
            $size = $reader.ReadUInt64()
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
    Write-Output "Embedded uninstaller verified: $embeddedName ($($metadata.FileVersion))"
} finally {
    if (Test-Path -LiteralPath $uninstallerPath) {
        Remove-Item -LiteralPath $uninstallerPath -Force
    }
    Remove-Item -LiteralPath $auditDirectory -Force
}
