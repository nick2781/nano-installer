param(
    [string]$StubsDirectory = "target/release/stubs"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$targetRoot = Join-Path $repoRoot "target"
$stubs = if ([System.IO.Path]::IsPathRooted($StubsDirectory)) {
    $StubsDirectory
} else {
    Join-Path $repoRoot $StubsDirectory
}
$lzmaStub = Join-Path $stubs "lzma-stub-native.exe"
$zlibStub = Join-Path $stubs "zlib-stub-native.exe"
$sevenZip = Join-Path $repoRoot "tools/7za.exe"
foreach ($file in @($lzmaStub, $zlibStub, $sevenZip)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Backend smoke dependency is missing: $file"
    }
}

$smokeRoot = Join-Path $targetRoot ("backend-smoke-" + [guid]::NewGuid().ToString("N"))
$resolvedTarget = [System.IO.Path]::GetFullPath($targetRoot).TrimEnd('\') + '\'
$resolvedSmoke = [System.IO.Path]::GetFullPath($smokeRoot).TrimEnd('\') + '\'
if (-not $resolvedSmoke.StartsWith($resolvedTarget, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Backend smoke directory escaped target/: $resolvedSmoke"
}

try {
    $input = Join-Path $smokeRoot "input"
    $nested = Join-Path $input "nested"
    $lzmaOutput = Join-Path $smokeRoot "lzma-output"
    $zlibOutput = Join-Path $smokeRoot "zlib-output"
    $archive7z = Join-Path $smokeRoot "payload.7z"
    $archiveZip = Join-Path $smokeRoot "payload.zip"
    New-Item -ItemType Directory -Path $nested -Force | Out-Null
    [System.IO.File]::WriteAllText(
        (Join-Path $input "root.txt"),
        "nano installer backend smoke",
        [System.Text.UTF8Encoding]::new($false)
    )
    [System.IO.File]::WriteAllBytes((Join-Path $nested "data.bin"), [byte[]](0..255))

    & $sevenZip a -t7z $archive7z (Join-Path $input "*") -mx=5 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "7z smoke archive creation failed with exit code $LASTEXITCODE"
    }
    Compress-Archive -Path (Join-Path $input "*") -DestinationPath $archiveZip

    $lzma = Start-Process -FilePath $lzmaStub `
        -ArgumentList "--extract `"$archive7z`" `"$lzmaOutput`"" -Wait -PassThru
    if ($lzma.ExitCode -ne 0) {
        throw "LZMA stub extraction failed with exit code $($lzma.ExitCode)"
    }
    $zlib = Start-Process -FilePath $zlibStub `
        -ArgumentList "--extract `"$archiveZip`" `"$zlibOutput`"" -Wait -PassThru
    if ($zlib.ExitCode -ne 0) {
        throw "zlib stub extraction failed with exit code $($zlib.ExitCode)"
    }

    foreach ($relative in @("root.txt", "nested\data.bin")) {
        $sourceHash = (Get-FileHash (Join-Path $input $relative) -Algorithm SHA256).Hash
        $lzmaHash = (Get-FileHash (Join-Path $lzmaOutput $relative) -Algorithm SHA256).Hash
        $zlibHash = (Get-FileHash (Join-Path $zlibOutput $relative) -Algorithm SHA256).Hash
        if ($sourceHash -ne $lzmaHash -or $sourceHash -ne $zlibHash) {
            throw "Backend extraction mismatch: $relative"
        }
    }
    Write-Output "Real ZIP and 7z backend smoke test passed"
} finally {
    if (Test-Path -LiteralPath $smokeRoot) {
        Remove-Item -LiteralPath $smokeRoot -Recurse -Force
    }
}
