param(
    [Parameter(Mandatory = $true)]
    [string[]]$File
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$dumpbin = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
$objdump = Get-Command objdump.exe -ErrorAction SilentlyContinue
if (-not $dumpbin -and -not $objdump) {
    throw "PE import audit requires dumpbin.exe or objdump.exe"
}

$forbidden = @(
    "api-ms-win-core-synch-l1-2-0.dll",
    "api-ms-win-crt-",
    "bcryptprimitives.dll",
    "combase.dll",
    "vcruntime140.dll",
    "WaitOnAddress",
    "WakeByAddress",
    "GetSystemTimePreciseAsFileTime",
    "GetDpiForSystem",
    "GetDpiForWindow",
    "GetThreadDpiAwarenessContext",
    "SetThreadDpiAwarenessContext",
    "AreDpiAwarenessContextsEqual"
)

$failures = @()
foreach ($path in $File) {
    $resolved = (Resolve-Path -LiteralPath $path -ErrorAction Stop).Path
    $imports = if ($dumpbin) {
        & $dumpbin.Source /nologo /imports $resolved 2>&1
    } else {
        & $objdump.Source -p $resolved 2>&1
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to inspect PE imports: $resolved"
    }

    $text = $imports -join "`n"
    foreach ($symbol in $forbidden) {
        if ($text -match [regex]::Escape($symbol)) {
            $failures += "$resolved imports forbidden Win7 dependency: $symbol"
        }
    }

    Write-Output "Win7 import audit inspected: $resolved"
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    throw "Win7 PE import audit failed"
}

Write-Output "Win7 PE import audit passed"
