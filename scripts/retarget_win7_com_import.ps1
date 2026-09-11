param(
    [Parameter(Mandatory = $true)]
    [string[]]$File
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$dumpbin = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
if (-not $dumpbin) {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path -LiteralPath $vswhere) {
        $dumpbinPath = & $vswhere -latest -products * `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
            -find "VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe" |
            Select-Object -First 1
        if ($dumpbinPath) {
            $dumpbin = Get-Item -LiteralPath $dumpbinPath
        }
    }
}
if (-not $dumpbin) {
    throw "Win7 COM import retargeting requires Visual Studio dumpbin.exe"
}

function Find-BytePattern {
    param([byte[]]$Data, [byte[]]$Pattern)

    $offsets = [System.Collections.Generic.List[int]]::new()
    for ($offset = 0; $offset -le $Data.Length - $Pattern.Length; $offset++) {
        $matches = $true
        for ($index = 0; $index -lt $Pattern.Length; $index++) {
            if ($Data[$offset + $index] -ne $Pattern[$index]) {
                $matches = $false
                break
            }
        }
        if ($matches) {
            $offsets.Add($offset)
        }
    }
    return $offsets.ToArray()
}

$sourceName = [System.Text.Encoding]::ASCII.GetBytes("combase.dll`0")
$targetName = [System.Text.Encoding]::ASCII.GetBytes("ole32.dll`0")

foreach ($path in $File) {
    $resolved = (Resolve-Path -LiteralPath $path -ErrorAction Stop).Path
    $dumpbinExe = if ($dumpbin.PSObject.Properties.Name -contains "Source") {
        $dumpbin.Source
    } else {
        $dumpbin.FullName
    }
    $imports = @(& $dumpbinExe /nologo /imports $resolved 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to inspect PE imports: $resolved"
    }

    $combaseLine = -1
    for ($index = 0; $index -lt $imports.Count; $index++) {
        if ($imports[$index].ToString().Trim() -ieq "combase.dll") {
            $combaseLine = $index
            break
        }
    }

    if ($combaseLine -lt 0) {
        Write-Output "Win7 COM import already compatible: $resolved"
        continue
    }

    $symbols = [System.Collections.Generic.List[string]]::new()
    for ($index = $combaseLine + 1; $index -lt $imports.Count; $index++) {
        $line = $imports[$index].ToString()
        if ($line.Trim() -match '^[A-Za-z0-9_.-]+\.dll$') {
            break
        }
        if ($line -match '^\s+[0-9A-Fa-f]+\s+([A-Za-z_][A-Za-z0-9_@$?]*)\s*$') {
            $symbols.Add($Matches[1])
        }
    }

    if ($symbols.Count -ne 1 -or $symbols[0] -ne "CoTaskMemFree") {
        throw "Refusing to retarget $resolved because combase.dll imports are not exactly CoTaskMemFree: $($symbols -join ', ')"
    }

    $bytes = [System.IO.File]::ReadAllBytes($resolved)
    $offsets = @(Find-BytePattern -Data $bytes -Pattern $sourceName)
    if ($offsets.Count -ne 1) {
        throw "Expected exactly one combase.dll import name in $resolved, found $($offsets.Count)"
    }

    $replacement = [byte[]]::new($sourceName.Length)
    [Array]::Copy($targetName, $replacement, $targetName.Length)
    [Array]::Copy($replacement, 0, $bytes, $offsets[0], $replacement.Length)
    [System.IO.File]::WriteAllBytes($resolved, $bytes)
    Write-Output "Retargeted CoTaskMemFree from combase.dll to ole32.dll: $resolved"
}
