<#
    Measures what one build costs, for a payload of a given size.

    The builder appends the payload to the setup, and it does that by streaming:
    the file is copied in 1 MiB blocks, and each block feeds the SHA-256 the
    bundle records for it. This script is how that claim is checked rather than
    asserted -- it writes a project whose payload archive holds one stored (that
    is, uncompressed) entry of the requested size, builds it with the CLI, and
    reports the wall time and the most memory the builder was seen holding.

    A stored entry is deliberate: what is being measured is the copy, not an
    archiver, and a stored entry makes the setup grow by the payload's size and
    by nothing else. The bytes are zeros for the same reason -- nothing here
    compresses them, so what they are does not matter, and `fsutil file
    createnew` makes a file of any size in no time at all.

    Usage (after `cargo build --release -p nano-installer-native-cli`):
        .\scripts\measure_build.ps1 -PayloadMiB 512
        .\scripts\measure_build.ps1 -PayloadMiB 8,128,512 -Report target/build-cost.txt

    The report is one line per payload size, and the script exits non-zero if a
    build fails or an archive does not come out the size that was asked for, so
    a caller can gate on it.
#>
param(
    # A list, "8,128,512". It is text rather than an array so that cmd.exe can
    # pass it too: `-PayloadMiB 8,128` reaches a typed array as one word.
    [string]$PayloadMiB = "512",
    [string]$Stubs = "target/release",
    [string]$Builder = "target/release/nano-installer-native-x64.exe",
    [string]$WorkDirectory,
    [string]$Report = "target/build-cost.txt"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
function Resolve-RepoPath([string]$Path) {
    if ([IO.Path]::IsPathRooted($Path)) { return $Path }
    return (Join-Path $repoRoot $Path)
}

$builderPath = Resolve-RepoPath $Builder
$stubDirectory = Resolve-RepoPath $Stubs
$reportPath = Resolve-RepoPath $Report
if (-not (Test-Path -LiteralPath $builderPath -PathType Leaf)) {
    throw "Build the CLI first (cargo build --release -p nano-installer-native-cli): $builderPath is missing"
}
foreach ($stub in @("lzma-stub-native.exe", "zlib-stub-native.exe", "uninst-stub-native.exe")) {
    if (-not (Test-Path -LiteralPath (Join-Path $stubDirectory $stub) -PathType Leaf)) {
        throw "The runtime the builder embeds is missing: $(Join-Path $stubDirectory $stub)"
    }
}

if (-not $WorkDirectory) {
    $WorkDirectory = Join-Path ([IO.Path]::GetTempPath()) ("nano-build-cost-" + [Guid]::NewGuid().ToString("N"))
}
$null = New-Item -ItemType Directory -Force -Path $WorkDirectory
$project = Join-Path $WorkDirectory "project"
foreach ($directory in @("layouts", "assets", "locales", "payload")) {
    $null = New-Item -ItemType Directory -Force -Path (Join-Path $project $directory)
}

# The payload has to hold the executable the project declares, so the archive
# carries a small one beside the large entry.
$exePath = Join-Path $project "assets/app.exe"
[IO.File]::WriteAllBytes($exePath, [Text.Encoding]::ASCII.GetBytes("MZ payload marker"))

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
function New-PayloadArchive([string]$Path, [string]$Executable, [string]$BigFile, [long]$Bytes) {
    if (Test-Path -LiteralPath $Path) { Remove-Item -LiteralPath $Path -Force }
    $archive = [IO.Compression.ZipFile]::Open($Path, [IO.Compression.ZipArchiveMode]::Create)
    try {
        $null = [IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
            $archive, $Executable, "app.exe", [IO.Compression.CompressionLevel]::NoCompression)
        $null = [IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
            $archive, $BigFile, "app.bin", [IO.Compression.CompressionLevel]::NoCompression)
    } finally { $archive.Dispose() }
    # An archive that is not the size asked for would make every number below it
    # meaningless, and a payload archive that grows without bound is exactly the
    # kind of thing a measurement script must not do quietly.
    $size = (Get-Item -LiteralPath $Path).Length
    if ($size -lt $Bytes -or $size -gt ($Bytes + 1MB)) {
        throw "The payload archive is $size bytes, not the ${Bytes} bytes that were asked for"
    }
    return $size
}

$config = @'
{
  "project": { "name": "BuildCost", "version": "1.0.0", "file_version": "1.0.0.0", "publisher": "nano-installer" },
  "output": { "installer_name": "BuildCost_Setup.exe", "uninstaller_name": "uninst.exe" },
  "install": { "exe_name": "app.exe", "require_admin": false },
  "shortcuts": { "desktop_shortcut": false, "start_menu": false },
  "autostart": { "enabled": false, "default": false },
  "localization": { "default_locale": "en-US", "supported_locales": ["en-US"] },
  "resources": { "layouts_dir": "layouts", "assets_dir": "assets", "locales_dir": "locales", "payload_file": "payload/app.zip" },
  "ui": { "dialog_layout": "layouts/msgBox.xml" },
  "wizard": { "pages": [ { "id": "config", "layout": "layouts/configpage.xml", "title": "Options" } ] },
  "advanced": { "silent_mode_support": true, "uninstall_mode_support": true }
}
'@
[IO.File]::WriteAllText((Join-Path $project "installer_config.json"), $config, [Text.UTF8Encoding]::new($false))
Set-Content -LiteralPath (Join-Path $project "layouts/configpage.xml") -Value '<Page width="720" height="450"><Label text="Build cost" /></Page>'
Set-Content -LiteralPath (Join-Path $project "layouts/msgBox.xml") -Value '<Page width="480" height="180" />'
[IO.File]::WriteAllText((Join-Path $project "locales/en-US.json"), '{}', [Text.UTF8Encoding]::new($false))

$sizes = @()
foreach ($word in $PayloadMiB.Split(",")) {
    $value = 0
    if (-not [int]::TryParse($word.Trim(), [ref]$value) -or $value -le 0) {
        throw "PayloadMiB wants sizes in MiB, like 8 or 8,128,512; got '$word'"
    }
    $sizes += $value
}

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("payload MiB  setup MiB  seconds  peak working set MiB (sampled)")
foreach ($size in $sizes) {
    $bytes = [long]$size * 1MB
    $archivePath = Join-Path $project "payload/app.zip"
    $bigFile = Join-Path $WorkDirectory "app.bin"
    if (Test-Path -LiteralPath $bigFile) { Remove-Item -LiteralPath $bigFile -Force }
    # A zero-filled file of the requested size, made by the filesystem rather
    # than written out here.
    & fsutil file createnew $bigFile $bytes | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "fsutil could not create $bigFile" }
    Write-Output "payload ${size} MiB: writing the archive"
    $archiveBytes = New-PayloadArchive $archivePath $exePath $bigFile $bytes

    $setup = Join-Path $WorkDirectory ("BuildCost_Setup_{0}MiB.exe" -f $size)
    if (Test-Path -LiteralPath $setup) { Remove-Item -LiteralPath $setup -Force }

    # The builder is a console program, so waiting on the process is the whole
    # measurement: how long it took, and the most memory it was seen holding.
    # The working set is sampled rather than read from PeakWorkingSet64, because
    # that counter is gone once the process has exited.
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $builderPath
    $startInfo.Arguments = "build --project `"$project`" --output `"$setup`" --stubs `"$stubDirectory`""
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.CreateNoWindow = $true
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $process = [System.Diagnostics.Process]::Start($startInfo)
    # Both pipes are read as they fill, so a build that writes more than a pipe
    # holds cannot be stopped by that while this loop waits for it.
    $stdoutText = New-Object System.Text.StringBuilder
    $stderrText = New-Object System.Text.StringBuilder
    $null = Register-ObjectEvent -InputObject $process -EventName OutputDataReceived `
        -Action { $null = $Event.MessageData.AppendLine($EventArgs.Data) } -MessageData $stdoutText
    $null = Register-ObjectEvent -InputObject $process -EventName ErrorDataReceived `
        -Action { $null = $Event.MessageData.AppendLine($EventArgs.Data) } -MessageData $stderrText
    $process.BeginOutputReadLine()
    $process.BeginErrorReadLine()
    $peak = [long]0
    while (-not $process.HasExited) {
        try { $current = $process.WorkingSet64 } catch { $current = 0 }
        if ($current -gt $peak) { $peak = $current }
        Start-Sleep -Milliseconds 20
    }
    $process.WaitForExit()
    $watch.Stop()
    $code = $process.ExitCode
    Get-EventSubscriber | Unregister-Event
    $stdout = $stdoutText.ToString()
    $stderr = $stderrText.ToString()
    $process.Dispose()
    if ($code -ne 0) {
        Write-Output $stdout
        Write-Output $stderr
        throw "The build of a ${size} MiB payload failed with exit code $code"
    }
    $setupMiB = [Math]::Round((Get-Item -LiteralPath $setup).Length / 1MB, 1)
    $line = "{0,11}  {1,9}  {2,7:N1}  {3,21:N1}" -f $size, $setupMiB, $watch.Elapsed.TotalSeconds, ($peak / 1MB)
    $lines.Add($line)
    Write-Output $line
    Remove-Item -LiteralPath $setup -Force
    Remove-Item -LiteralPath $archivePath -Force
    Remove-Item -LiteralPath $bigFile -Force
}

$lines | Out-File -Encoding ascii $reportPath
Write-Output "report written to $reportPath"
