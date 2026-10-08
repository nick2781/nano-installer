<#
    Removes what this repository's probe installations left on a machine.

    The end-to-end suite builds throwaway products called E2eProbe, installs them
    and uninstalls them again. A case clears its own installation as it ends, and
    the suite reaps what an earlier run left in the user's own hive before it
    starts. Neither reaches two things:

      * A run that was killed leaves an entry in Programs and Features whose
        uninstaller lived in a temporary directory that is gone.
      * A run that had administrator rights installs its probe through Windows
        Installer, which registers the product in HKLM. Taking that back needs
        administrator rights, so an ordinary suite run cannot do it, and the
        entry it leaves behind says `MsiExec.exe /I{...}` -- which fails, because
        the package it names was in a temporary directory too.

    What is left is a product nobody can uninstall through the interface. This
    script removes it: the registry entries are all that is left of it, and they
    are this repository's own, which is why matching them by name is enough.

    Only entries that are provably dead are touched: one whose install directory
    is gone, or whose Windows Installer package is missing and whose install
    location is empty. A product that is still installed is left alone.

    Usage:
        .\scripts\clean_probe_installs.ps1 -DryRun     # list what it would remove
        .\scripts\clean_probe_installs.ps1             # remove the user's own leftovers
        # From an elevated prompt, for the machine-wide ones Windows Installer wrote:
        .\scripts\clean_probe_installs.ps1
#>
param(
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# The names this repository's probes install under. A product called anything
# else is not ours to remove.
$probeNames = @("E2eProbe", "MsiProbe", "ChineseProbe", "ZhProbe")

$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall"
$machineUninstallKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall"

$removed = 0
$needsElevation = New-Object System.Collections.Generic.List[string]

function Remove-ProbeKey {
    param([string]$Path, [string]$Why)

    if ($DryRun) {
        Write-Output "would remove $Path ($Why)"
        return
    }
    Remove-Item -LiteralPath $Path -Recurse -Force -ErrorAction SilentlyContinue
    Write-Output "removed $Path ($Why)"
}

function Test-InstallGone {
    param([string]$Location)

    if ([string]::IsNullOrWhiteSpace($Location)) { return $true }
    $directory = $Location.TrimEnd('\')
    return -not (Test-Path -LiteralPath (Join-Path $directory "uninst.exe"))
}

# Under StrictMode a value an entry does not carry is an error rather than
# `$null`, and most entries carry only some of what this script asks about.
function Get-EntryValue {
    param($Values, [string]$Name)

    if ($null -eq $Values) { return $null }
    $property = $Values.PSObject.Properties[$Name]
    if ($null -eq $property) { return $null }
    return $property.Value
}

# The key name Windows Installer files a product under.
#
# It is the product code with every byte's two hex digits swapped, and with the
# byte order of the first three fields reversed as well, because those three are
# little-endian integers. Checked against the eighty-odd products this machine
# has installed: {03DF2DED-3E42-A901-82E3-70FF2C4199A7} is filed as
# DED2FD3024E3109A283E07FFC214997A.
function Get-PackedGuid {
    param([string]$Guid)

    $swap = {
        param([string]$Hex)
        $swapped = ""
        for ($i = 0; $i -lt $Hex.Length; $i += 2) {
            $swapped += $Hex.Substring($i + 1, 1) + $Hex.Substring($i, 1)
        }
        $swapped
    }

    $parts = $Guid.Trim('{', '}').Split('-')
    if ($parts.Count -ne 5) { return $null }
    $head = ""
    foreach ($field in $parts[0..2]) {
        $bytes = @()
        for ($i = 0; $i -lt $field.Length; $i += 2) {
            $bytes += (& $swap $field.Substring($i, 2))
        }
        [array]::Reverse($bytes)
        $head += ($bytes -join '')
    }
    return ($head + (& $swap $parts[3]) + (& $swap $parts[4])).ToUpperInvariant()
}

Write-Output "Probe installations this script knows how to remove: $($probeNames -join ', ')"

# The user's own hive: entries the suite's own reaper takes when it runs, and
# which are also here so a machine whose suite never runs again can be cleaned.
foreach ($entry in (Get-ChildItem -LiteralPath $uninstallKey -ErrorAction SilentlyContinue)) {
    if ($entry.PSChildName -notlike 'nano-installer*') { continue }
    $values = Get-ItemProperty -LiteralPath $entry.PSPath -ErrorAction SilentlyContinue
    if (-not (Test-InstallGone -Location (Get-EntryValue $values "InstallLocation"))) { continue }
    Remove-ProbeKey -Path $entry.PSPath -Why "its installation is gone"
    Remove-ProbeKey -Path "HKCU:\Software\$($entry.PSChildName)" -Why "the case's own key"
    $removed++
}

foreach ($parent in (Get-ChildItem -LiteralPath "HKCU:\Software" -ErrorAction SilentlyContinue)) {
    if ($parent.PSChildName -notlike 'nano-installer*') { continue }
    foreach ($child in (Get-ChildItem -LiteralPath $parent.PSPath -ErrorAction SilentlyContinue)) {
        $values = Get-ItemProperty -LiteralPath $child.PSPath -ErrorAction SilentlyContinue
        if (-not (Test-InstallGone -Location (Get-EntryValue $values "InstallLocation"))) { continue }
        Remove-ProbeKey -Path $child.PSPath -Why "its installation is gone"
        $removed++
    }
    if (-not $DryRun -and (Get-ChildItem -LiteralPath $parent.PSPath -ErrorAction SilentlyContinue).Count -eq 0) {
        Remove-ProbeKey -Path $parent.PSPath -Why "nothing of ours is left under it"
    }
}

# What Windows Installer registered, in whichever hive it used.
foreach ($root in @($machineUninstallKey, $uninstallKey)) {
    foreach ($entry in (Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue)) {
        if ($entry.PSChildName -notlike '{*-*-*-*-*}') { continue }
        $values = Get-ItemProperty -LiteralPath $entry.PSPath -ErrorAction SilentlyContinue
        $name = Get-EntryValue $values "DisplayName"
        if (-not $name) { continue }
        $ours = $false
        foreach ($probe in $probeNames) {
            if ($name -like "$probe*") { $ours = $true; break }
        }
        if (-not $ours) { continue }
        # Only a product whose package and installation are both gone: anything
        # else still has a way to be uninstalled properly.
        $package = Get-EntryValue $values "LocalPackage"
        $packageGone = [string]::IsNullOrWhiteSpace($package) -or -not (Test-Path -LiteralPath $package)
        if (-not $packageGone -or -not (Test-InstallGone -Location (Get-EntryValue $values "InstallLocation"))) { continue }

        $productCode = $entry.PSChildName
        $packed = Get-PackedGuid -Guid $productCode
        # Written as an HKLM:/HKCU: path rather than the provider path PowerShell
        # hands back, because which hive it is decides whether this run may touch
        # it at all.
        $targets = @(
            "$root\$productCode",
            "HKLM:\Software\Classes\Installer\Products\$packed",
            "HKLM:\Software\Microsoft\Windows\CurrentVersion\Installer\UserData\S-1-5-18\Products\$packed",
            "HKLM:\Software\Classes\Installer\Features\$packed",
            "HKCU:\Software\Classes\Installer\Products\$packed",
            "HKCU:\Software\Microsoft\Installer\Products\$packed"
        )
        foreach ($target in $targets) {
            if (-not (Test-Path -LiteralPath $target)) { continue }
            $isMachine = $target -like 'HKLM:*'
            $elevated = ([Security.Principal.WindowsPrincipal] `
                    [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
                    [Security.Principal.WindowsBuiltInRole]::Administrator)
            if ($isMachine -and -not $elevated) {
                $needsElevation.Add($target)
                continue
            }
            Remove-ProbeKey -Path $target -Why "a probe product that cannot be uninstalled"
            $removed++
        }
    }
}

Write-Output ""
if ($needsElevation.Count -gt 0) {
    Write-Output "$($needsElevation.Count) machine-wide entr(ies) need an elevated prompt:"
    foreach ($target in $needsElevation) { Write-Output "  $target" }
    Write-Output "Run this script again from an elevated prompt to remove them."
} else {
    Write-Output "Nothing left that needs an elevated prompt."
}
if ($DryRun) {
    Write-Output "Dry run: nothing was removed."
} else {
    Write-Output "Probe entries removed: $removed"
}
