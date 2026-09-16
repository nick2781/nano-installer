param(
    [Parameter(Mandatory = $true)]
    [string[]]$File,
    [string]$ExpectLevel = "",
    [ValidateSet("", "true", "false")]
    [string]$ExpectDpiAware = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# The application manifest is what makes Windows raise the consent prompt and
# what tells the shell the window scales its own pixels. Windows reads it as the
# RT_MANIFEST resource before the process starts, so an image that lost the
# resource starts quietly with the wrong rights and nothing else reports it.
# Reading the resource back is the only way to catch that from a build script.
Add-Type -Namespace NanoInstaller -Name ManifestReader -MemberDefinition @"
[System.Runtime.InteropServices.DllImport("kernel32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode, SetLastError = true)]
public static extern System.IntPtr LoadLibraryExW(string fileName, System.IntPtr file, uint flags);

[System.Runtime.InteropServices.DllImport("kernel32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode, SetLastError = true)]
public static extern System.IntPtr FindResourceW(System.IntPtr module, System.IntPtr name, System.IntPtr type);

[System.Runtime.InteropServices.DllImport("kernel32.dll", SetLastError = true)]
public static extern uint SizeofResource(System.IntPtr module, System.IntPtr resource);

[System.Runtime.InteropServices.DllImport("kernel32.dll", SetLastError = true)]
public static extern System.IntPtr LoadResource(System.IntPtr module, System.IntPtr resource);

[System.Runtime.InteropServices.DllImport("kernel32.dll", SetLastError = true)]
public static extern System.IntPtr LockResource(System.IntPtr resource);

[System.Runtime.InteropServices.DllImport("kernel32.dll", SetLastError = true)]
public static extern bool FreeLibrary(System.IntPtr module);
"@

$LoadLibraryAsDatafile = 0x00000002
# Both a manifest and a manifest type are addressed by their low-word integer
# id, so the string names are turned into the pointers Windows expects.
$ManifestTypeId = [System.IntPtr]24
$ManifestNameId = [System.IntPtr]1

function Read-ApplicationManifest {
    param([string]$Path)

    $module = [NanoInstaller.ManifestReader]::LoadLibraryExW($Path, [System.IntPtr]::Zero, $LoadLibraryAsDatafile)
    if ($module -eq [System.IntPtr]::Zero) {
        throw "Cannot open PE image for the manifest audit: $Path"
    }
    try {
        $resource = [NanoInstaller.ManifestReader]::FindResourceW($module, $ManifestNameId, $ManifestTypeId)
        if ($resource -eq [System.IntPtr]::Zero) {
            return $null
        }
        $size = [NanoInstaller.ManifestReader]::SizeofResource($module, $resource)
        $handle = [NanoInstaller.ManifestReader]::LoadResource($module, $resource)
        if ($handle -eq [System.IntPtr]::Zero -or $size -eq 0) {
            throw "Manifest resource is empty: $Path"
        }
        $pointer = [NanoInstaller.ManifestReader]::LockResource($handle)
        if ($pointer -eq [System.IntPtr]::Zero) {
            throw "Manifest resource cannot be read: $Path"
        }
        $bytes = [byte[]]::new($size)
        [System.Runtime.InteropServices.Marshal]::Copy($pointer, $bytes, 0, $size)
        return [Text.Encoding]::UTF8.GetString($bytes)
    }
    finally {
        $null = [NanoInstaller.ManifestReader]::FreeLibrary($module)
    }
}

$failures = @()
foreach ($path in $File) {
    $resolved = (Resolve-Path -LiteralPath $path -ErrorAction Stop).Path
    $manifest = Read-ApplicationManifest -Path $resolved
    if (-not $manifest) {
        $failures += "$resolved has no application manifest; Windows cannot know its rights or DPI behaviour"
        continue
    }

    try {
        $document = [xml]$manifest
    }
    catch {
        $failures += "$resolved has a manifest Windows would reject as malformed: $($_.Exception.Message)"
        continue
    }

    $level = $document.SelectSingleNode("//*[local-name()='requestedExecutionLevel']")
    if (-not $level) {
        $failures += "$resolved does not declare requestedExecutionLevel"
    }
    else {
        $actual = $level.GetAttribute("level")
        if ($actual -notin @("asInvoker", "highestAvailable", "requireAdministrator")) {
            $failures += "$resolved declares an unknown execution level: $actual"
        }
        if ($ExpectLevel -and $actual -ne $ExpectLevel) {
            $failures += "$resolved requests '$actual' but the project resolves to '$ExpectLevel'"
        }
    }

    $dpi = $document.SelectSingleNode("//*[local-name()='dpiAware']")
    if (-not $dpi) {
        $failures += "$resolved does not declare dpiAware"
    }
    elseif ($ExpectDpiAware -and $dpi.InnerText.Trim() -ne $ExpectDpiAware) {
        $failures += "$resolved declares dpiAware '$($dpi.InnerText.Trim())' but the project resolves to '$ExpectDpiAware'"
    }

    $described = if ($level) { $level.GetAttribute("level") } else { "?" }
    $aware = if ($dpi) { $dpi.InnerText.Trim() } else { "?" }
    Write-Output "Application manifest audit inspected: $resolved (level=$described, dpiAware=$aware)"
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    throw "Application manifest audit failed"
}

Write-Output "Application manifest audit passed"
