<#
    Fails when a Rust test file sits somewhere Cargo never compiles.

    A root-level tests/ directory next to a virtual manifest is the trap: it
    looks like an integration suite, and the files in it reference crates that
    may not even exist, but Cargo only discovers tests/ inside a package. One
    such file lived here for months, referencing a `nano_installer` crate that
    was really `nano_installer_core` and types that had been removed; every test
    in it would have failed to compile, and none of them ever ran.

    The check reads the workspace from Cargo itself rather than guessing, so a
    new crate or a renamed package cannot make it quietly vacuous.
#>
param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

Push-Location $RepositoryRoot
try {
    $metadata = & cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) {
        throw "cargo metadata failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}
$packageRoots = @{}
foreach ($package in $metadata.packages) {
    $root = Split-Path -Parent $package.manifest_path
    $packageRoots[$root] = $package.name
}

$failures = @()
$inspected = @()
$queue = New-Object System.Collections.Generic.Queue[string]
$queue.Enqueue($RepositoryRoot)
while ($queue.Count -gt 0) {
    $directory = $queue.Dequeue()
    foreach ($child in Get-ChildItem -LiteralPath $directory -Directory -Force) {
        if ($child.Name -in @("target", ".git", "node_modules", "tmp")) {
            continue
        }
        if ($child.Name -eq "tests") {
            foreach ($file in Get-ChildItem -LiteralPath $child.FullName -Filter "*.rs" -File -Recurse) {
                $inspected += $file.FullName
                $owner = $null
                foreach ($root in $packageRoots.Keys) {
                    if ($file.FullName.StartsWith("$root\", [System.StringComparison]::OrdinalIgnoreCase)) {
                        if ($null -eq $owner -or $root.Length -gt $owner.Length) {
                            $owner = $root
                        }
                    }
                }
                if ($null -eq $owner) {
                    $relative = $file.FullName.Substring($RepositoryRoot.Length).TrimStart("\")
                    $failures += "$relative is not inside a package, so Cargo never compiles it"
                }
            }
            continue
        }
        $queue.Enqueue($child.FullName)
    }
}

# A package that turns discovery off would hide its tests/ files just as
# effectively, so the setting is refused outright.
foreach ($root in $packageRoots.Keys) {
    $manifest = Join-Path $root "Cargo.toml"
    if (Select-String -LiteralPath $manifest -Pattern '^\s*autotests\s*=\s*false' -Quiet) {
        $failures += "$manifest disables test discovery with autotests = false"
    }
}

if ($inspected.Count -eq 0) {
    Write-Output "Test target audit found no integration test files"
}

foreach ($file in $inspected) {
    Write-Output "Test target audit inspected: $file"
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    throw "Test target audit failed"
}

Write-Output "Test target audit passed: $($inspected.Count) integration test file(s) belong to a package"
