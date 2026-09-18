<#
    Captures the first page the example setup draws.

    The subject is examples/TapTap itself: the script builds a real setup from
    the project into target/, then saves the window it opens as a PNG, once per
    supported locale and once at 150% scaling. It never presses the install
    button, so nothing is deployed - the window is closed as soon as the picture
    is taken, and the project is left exactly as it was found. A project whose
    declared payload archive is not there, which is the case for a clone that
    never unpacked the example, is built against an empty archive of the same
    format; only the first page is photographed, so nothing has to be inside it.

    The page size, the regions that must hold artwork, and the text the labels
    must show are all read from the project itself, so a layout that moves is
    measured where it moved to. What is checked is arithmetic and runs on any
    machine: the client area matches the page, the images drew artwork rather
    than a flat rectangle, the button and the version line drew text, the page
    holds far more colours than an empty one, and the button reads differently
    in each language.

    Whether the glyphs read correctly is a judgement rather than an equation, so
    the script also writes manifest.json holding what each snapshot is meant to
    show. review_setup_snapshots.ps1 hands those pairs to a vision model on a
    machine that runs one.
#>
param(
    [string]$Project = "examples/TapTap",
    [string]$OutputDirectory,
    [string]$Builder,
    [string]$StubDirectory,
    [string[]]$Locales,
    [int[]]$Dpi = @(96, 144),
    # Builds the project exactly as it stands, elevation request included. The
    # consent prompt then has to be answered by hand before the window appears.
    [switch]$KeepElevation
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public class RuntimeWindow {
    public IntPtr Handle;
    public int Width;
    public int Height;
    public string ClassName;
    public string Title;
    public bool Visible;

    public override string ToString() {
        return string.Format("class='{0}' title='{1}' visible={2} {3}x{4}", ClassName, Title, Visible, Width, Height);
    }
}

public static class SnapshotWindow {
    public delegate bool EnumProc(IntPtr handle, IntPtr parameter);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumProc callback, IntPtr parameter);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr handle);

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr handle, out RECT rect);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr handle, out uint processId);

    [DllImport("user32.dll")]
    public static extern bool PrintWindow(IntPtr handle, IntPtr deviceContext, uint flags);

    [DllImport("user32.dll")]
    public static extern bool SetProcessDPIAware();

    [DllImport("user32.dll")]
    public static extern int GetWindowRgn(IntPtr handle, IntPtr region);

    [DllImport("gdi32.dll")]
    public static extern IntPtr CreateRectRgn(int left, int top, int right, int bottom);

    [DllImport("gdi32.dll")]
    public static extern bool PtInRegion(IntPtr region, int x, int y);

    [DllImport("gdi32.dll")]
    public static extern bool DeleteObject(IntPtr handle);

    // The window region is what makes the corners rounded. Asking it directly
    // beats reading pixels: a square corner and a cut one differ by a few
    // anti-aliased pixels, and this does not.
    public static int[] WindowShape(IntPtr handle, int width, int height) {
        IntPtr region = CreateRectRgn(0, 0, 1, 1);
        if (region == IntPtr.Zero) {
            return null;
        }
        try {
            if (GetWindowRgn(handle, region) == 0) {
                return null;
            }
            int corner = 0;
            if (PtInRegion(region, 1, 1)) corner++;
            if (PtInRegion(region, width - 2, 1)) corner++;
            if (PtInRegion(region, 1, height - 2)) corner++;
            if (PtInRegion(region, width - 2, height - 2)) corner++;
            int middle = PtInRegion(region, width / 2, height / 2) ? 1 : 0;
            return new int[] { corner, middle };
        }
        finally {
            DeleteObject(region);
        }
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetClassName(IntPtr handle, StringBuilder name, int count);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr handle, StringBuilder text, int count);

    // Every top-level window the process owns, for both the lookup and the
    // message a run that found none has to explain itself with.
    public static List<RuntimeWindow> Describe(uint wanted) {
        List<RuntimeWindow> found = new List<RuntimeWindow>();
        EnumWindows(delegate(IntPtr handle, IntPtr parameter) {
            uint owner = 0;
            GetWindowThreadProcessId(handle, out owner);
            if (owner == wanted) {
                found.Add(DescribeOne(handle));
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static RuntimeWindow Find(uint wanted, string className) {
        foreach (RuntimeWindow window in Describe(wanted)) {
            if (window.ClassName == className && window.Visible && window.Width > 0 && window.Height > 0) {
                return window;
            }
        }
        return null;
    }

    private static RuntimeWindow DescribeOne(IntPtr handle) {
        RECT rect;
        GetWindowRect(handle, out rect);
        StringBuilder name = new StringBuilder(256);
        GetClassName(handle, name, 256);
        StringBuilder title = new StringBuilder(256);
        GetWindowText(handle, title, 256);
        RuntimeWindow window = new RuntimeWindow();
        window.Handle = handle;
        window.Width = rect.Right - rect.Left;
        window.Height = rect.Bottom - rect.Top;
        window.ClassName = name.ToString();
        window.Title = title.ToString();
        window.Visible = IsWindowVisible(handle);
        return window;
    }
}
'@

# The window is measured in real pixels, so the reader has to be DPI aware too:
# a virtualized reader would report the scaled size of a window it is not looking at.
[void][SnapshotWindow]::SetProcessDPIAware()

$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $repoRoot "target/setup-snapshots" }
if (-not $Builder) { $Builder = Join-Path $repoRoot "target/debug/nano-installer-native-x64.exe" }
if (-not $StubDirectory) { $StubDirectory = Join-Path $repoRoot "target/debug" }
$projectRoot = $Project
if (-not [System.IO.Path]::IsPathRooted($projectRoot)) {
    $projectRoot = Join-Path $repoRoot $projectRoot
}

$buildHint = "cargo build -p nano-installer-native-cli -p nano-installer-stub-lzma -p nano-installer-stub-zlib -p nano-installer-uninstaller"
if (-not (Test-Path -LiteralPath $Builder -PathType Leaf)) {
    throw "Builder not found: $Builder. Build it first: $buildHint"
}
foreach ($stub in @("lzma-stub-native.exe", "zlib-stub-native.exe", "uninst-stub-native.exe")) {
    $path = Join-Path $StubDirectory $stub
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Runtime stub not found: $path. Build it first: $buildHint"
    }
}
if (-not (Test-Path -LiteralPath (Join-Path $projectRoot "installer_config.json") -PathType Leaf)) {
    throw "Project not found: $projectRoot"
}

# The regions a drawn page is checked in, taken from the layout the runtime
# opens first. The ids are the ones the manual checklist names: the logo and the
# tagline are artwork, the install button and the version line carry text.
$artwork = @(
    @{ Id = "logo"; Description = "logo" },
    @{ Id = "tagline"; Description = "tagline" }
)
$text = @(
    @{ Id = "btnInstall"; Description = "install button"; MinimumPixels = 100 },
    @{ Id = "lblVersion"; Description = "version line"; MinimumPixels = 100 }
)
# An image that never decoded, or a label that never painted, leaves one colour
# where the layout asked for content; anything actually drawn clears these by a
# wide margin. A page of nothing but a fill holds one colour, not sixty-four.
$minimumArtworkColors = 16
$minimumPageColors = 64
# Text is smoothed, so only the middle of a stroke keeps the declared colour and
# its edges lean towards the background. The margin is wide enough to count a
# whole glyph and still far short of the dark page behind it.
$textColorTolerance = 64

# A native tool that reports progress on standard error would otherwise trip
# $ErrorActionPreference = "Stop" on a message that is not a failure.
function Invoke-NativeCapture {
    param([scriptblock]$Command)

    $previous = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $Command 2>&1
        $code = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previous
    }
    return @{ Output = $output; ExitCode = $code }
}

function Read-JsonFile {
    param([string]$Path)

    return (Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json)
}

# A layout colour as `#AARRGGBB`. Only an opaque colour can be looked for on the
# page, since anything else arrives blended with whatever is behind it.
function ConvertFrom-ArgbHex {
    param([string]$Value)

    if (-not ($Value -match '^#(?<alpha>[0-9a-fA-F]{2})(?<red>[0-9a-fA-F]{2})(?<green>[0-9a-fA-F]{2})(?<blue>[0-9a-fA-F]{2})$')) {
        return $null
    }
    if ($Matches["alpha"] -ne "FF") {
        return $null
    }
    return @{
        R = [Convert]::ToInt32($Matches["red"], 16)
        G = [Convert]::ToInt32($Matches["green"], 16)
        B = [Convert]::ToInt32($Matches["blue"], 16)
    }
}

# A layout attribute is a number of pixels at 96 DPI, or nothing the checks can
# use: a percentage or a missing attribute skips the element instead of failing
# on arithmetic the runtime does differently.
function ConvertTo-Pixels {
    param([System.Xml.XmlElement]$Node, [string]$Attribute)

    if (-not $Node.HasAttribute($Attribute)) {
        return $null
    }
    $value = 0.0
    if (-not [double]::TryParse($Node.GetAttribute($Attribute), [ref]$value)) {
        return $null
    }
    return $value
}

function Get-ElementRect {
    param([System.Xml.XmlElement]$Node, [double]$Scale)

    $left = ConvertTo-Pixels -Node $Node -Attribute "left"
    $top = ConvertTo-Pixels -Node $Node -Attribute "top"
    $width = ConvertTo-Pixels -Node $Node -Attribute "width"
    $height = ConvertTo-Pixels -Node $Node -Attribute "height"
    if ($null -eq $left -or $null -eq $top -or $null -eq $width -or $null -eq $height) {
        return $null
    }
    return @{
        Left   = [int][Math]::Round($left * $Scale)
        Top    = [int][Math]::Round($top * $Scale)
        Right  = [int][Math]::Round(($left + $width) * $Scale)
        Bottom = [int][Math]::Round(($top + $height) * $Scale)
    }
}

function Get-SnapshotPixels {
    param([string]$Path)

    $bitmap = [System.Drawing.Bitmap]::FromFile($Path)
    try {
        $width = $bitmap.Width
        $height = $bitmap.Height
        $rectangle = New-Object System.Drawing.Rectangle 0, 0, $width, $height
        $data = $bitmap.LockBits(
            $rectangle,
            [System.Drawing.Imaging.ImageLockMode]::ReadOnly,
            [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        try {
            $bytes = New-Object byte[] ($data.Stride * $height)
            [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)
        }
        finally {
            $bitmap.UnlockBits($data)
        }
    }
    finally {
        $bitmap.Dispose()
    }

    # Format32bppArgb keeps its bytes in BGRA order.
    return @{ Width = $width; Height = $height; Stride = $data.Stride; Bytes = $bytes }
}

# How many colours a region holds, and how many of its pixels carry the colour
# the layout declares for the text in it. An image that never decoded leaves a
# single colour; text that never painted leaves the second count at nothing,
# whatever artwork it was meant to sit on.
function Get-RegionStats {
    param($Image, [hashtable]$Rect, $Target)

    $counts = @{}
    $matches = 0
    for ($y = $Rect.Top; $y -lt $Rect.Bottom -and $y -lt $Image.Height; $y++) {
        $row = $y * $Image.Stride
        for ($x = $Rect.Left; $x -lt $Rect.Right -and $x -lt $Image.Width; $x++) {
            $pixel = $row + $x * 4
            $blue = $Image.Bytes[$pixel]
            $green = $Image.Bytes[$pixel + 1]
            $red = $Image.Bytes[$pixel + 2]
            $color = ($red -shl 16) -bor ($green -shl 8) -bor $blue
            if ($counts.ContainsKey($color)) { $counts[$color] = $counts[$color] + 1 } else { $counts[$color] = 1 }
            if ($null -ne $Target -and
                [Math]::Abs($red - $Target.R) -le $textColorTolerance -and
                [Math]::Abs($green - $Target.G) -le $textColorTolerance -and
                [Math]::Abs($blue - $Target.B) -le $textColorTolerance) {
                $matches++
            }
        }
    }
    return @{ Colors = $counts.Count; TargetPixels = $matches }
}

# One region's rows, so the same region can be compared across two snapshots.
function Get-RegionFingerprint {
    param($Image, [hashtable]$Rect)

    $rows = New-Object System.Collections.Generic.List[byte]
    for ($y = $Rect.Top; $y -lt $Rect.Bottom -and $y -lt $Image.Height; $y++) {
        $offset = $y * $Image.Stride + $Rect.Left * 4
        for ($index = 0; $index -lt ($Rect.Right - $Rect.Left) * 4 -and ($offset + $index) -lt $Image.Bytes.Length; $index++) {
            $rows.Add($Image.Bytes[$offset + $index])
        }
    }
    $hash = [System.Security.Cryptography.MD5]::Create()
    try {
        return [BitConverter]::ToString($hash.ComputeHash($rows.ToArray()))
    }
    finally {
        $hash.Dispose()
    }
}

function Save-Snapshot {
    param(
        [string]$Setup,
        [string]$Locale,
        [int]$Dpi,
        [string]$Path
    )

    # The child inherits these, so the setup opens the page this snapshot wants
    # without a click. Both are read once at start-up and ignored when unset.
    $env:NANO_INSTALLER_TEST_LOCALE = $Locale
    $env:NANO_INSTALLER_TEST_DPI = "$Dpi"
    try {
        $process = Start-Process -FilePath $Setup -PassThru
    }
    finally {
        Remove-Item Env:NANO_INSTALLER_TEST_LOCALE -ErrorAction SilentlyContinue
        Remove-Item Env:NANO_INSTALLER_TEST_DPI -ErrorAction SilentlyContinue
    }

    try {
        $deadline = (Get-Date).AddSeconds(60)
        $handle = [IntPtr]::Zero
        $observed = ""
        $gone = ""
        while ((Get-Date) -lt $deadline -and $handle -eq [IntPtr]::Zero -and -not $gone) {
            $process.Refresh()
            if ($process.HasExited) {
                $gone = "the setup exited with code $($process.ExitCode)"
                break
            }
            # The size the window came out is what the snapshot asserts, so it is
            # measured here rather than assumed from the layout.
            $window = [SnapshotWindow]::Find([uint32]$process.Id, "NanoInstallerNativeRuntime")
            if ($null -ne $window) {
                $handle = $window.Handle
                $observed = "$($window.Width)x$($window.Height)"
            }
            else {
                Start-Sleep -Milliseconds 100
            }
        }
        if ($handle -eq [IntPtr]::Zero) {
            $seen = @([SnapshotWindow]::Describe([uint32]$process.Id) | ForEach-Object { $_.ToString() })
            $detail = "no window of any kind was seen"
            if ($gone) { $detail = $gone }
            elseif ($seen.Count -gt 0) { $detail = "windows seen: $($seen -join '; ')" }
            throw "The runtime window of $Setup did not appear within 60 seconds ($detail)"
        }
        # The window paints before it is shown, so this only covers a slow first
        # frame; a snapshot taken mid-paint shows up as a failed check.
        Start-Sleep -Milliseconds 700
        $parts = $observed -split "x"
        $windowWidth = [int]$parts[0]
        $windowHeight = [int]$parts[1]
        $shape = [SnapshotWindow]::WindowShape($handle, $windowWidth, $windowHeight)
        $cornerPoints = $null
        $middleInside = $null
        if ($null -ne $shape) {
            $cornerPoints = $shape[0]
            $middleInside = $shape[1]
        }
        $bitmap = New-Object System.Drawing.Bitmap $windowWidth, $windowHeight, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $deviceContext = $graphics.GetHdc()
        try {
            $captured = [SnapshotWindow]::PrintWindow($handle, $deviceContext, 0)
        }
        finally {
            $graphics.ReleaseHdc($deviceContext)
            $graphics.Dispose()
        }
        if (-not $captured) {
            $bitmap.Dispose()
            throw "PrintWindow returned nothing for $Setup"
        }
        $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
        $bitmap.Dispose()
        return @{ Window = $observed; CornerPoints = $cornerPoints; MiddleInside = $middleInside }
    }
    finally {
        if ($process -and -not $process.HasExited) {
            Stop-Process -Id $process.Id -Force
        }
    }
}

function Test-Snapshot {
    param([string]$Path, [int]$Dpi, [hashtable]$Page, $Layout, [hashtable]$Window)

    $problems = New-Object System.Collections.Generic.List[string]
    $asserted = New-Object System.Collections.Generic.List[string]
    $regions = @{}
    $scale = $Dpi / 96.0
    $image = Get-SnapshotPixels -Path $Path

    $expectedWidth = [int][Math]::Round($Page.Width * $scale)
    $expectedHeight = [int][Math]::Round($Page.Height * $scale)
    if ($image.Width -ne $expectedWidth -or $image.Height -ne $expectedHeight) {
        $problems.Add("the client area is $($image.Width)x$($image.Height), the page declares ${expectedWidth}x${expectedHeight}")
    }
    else {
        $asserted.Add("client area ${expectedWidth}x${expectedHeight}")
    }

    if ($null -eq $Window.CornerPoints) {
        $asserted.Add("the window region was unreadable, so its shape was not checked")
    }
    elseif ($Window.CornerPoints -eq 0 -and $Window.MiddleInside -eq 1) {
        $asserted.Add("every corner falls outside the window region, so the corners are cut")
    }
    else {
        $problems.Add("$($Window.CornerPoints) of 4 corner points fall inside the window region, so its corners are not rounded")
    }

    foreach ($check in $artwork) {
        $node = $Layout.SelectSingleNode("//*[@id='$($check.Id)']")
        if ($null -eq $node) {
            $problems.Add("the layout no longer declares <$($check.Id)>")
            continue
        }
        $rect = Get-ElementRect -Node $node -Scale $scale
        if ($null -eq $rect) {
            $problems.Add("<$($check.Id)> has no absolute rectangle to check")
            continue
        }
        $stats = Get-RegionStats -Image $image -Rect $rect
        if ($stats.Colors -lt $minimumArtworkColors) {
            $problems.Add("the $($check.Description) drew $($stats.Colors) colours, so it is a flat rectangle rather than artwork")
        }
        else {
            $asserted.Add("the $($check.Description) drew artwork ($($stats.Colors) colours)")
        }
    }

    foreach ($check in $text) {
        $node = $Layout.SelectSingleNode("//*[@id='$($check.Id)']")
        if ($null -eq $node) {
            $problems.Add("the layout no longer declares <$($check.Id)>")
            continue
        }
        $rect = Get-ElementRect -Node $node -Scale $scale
        if ($null -eq $rect) {
            $problems.Add("<$($check.Id)> has no absolute rectangle to check")
            continue
        }
        $declared = ConvertFrom-ArgbHex -Value $node.GetAttribute("color")
        if ($null -eq $declared) {
            $problems.Add("<$($check.Id)> declares no opaque text colour to look for")
            continue
        }
        $stats = Get-RegionStats -Image $image -Rect $rect -Target $declared
        if ($stats.TargetPixels -lt $check.MinimumPixels) {
            $problems.Add("the $($check.Description) drew $($stats.TargetPixels) pixels in its declared colour $($node.GetAttribute('color')), so its text is missing")
        }
        else {
            $asserted.Add("the $($check.Description) drew text ($($stats.TargetPixels) pixels in $($node.GetAttribute('color')))")
        }
        $regions[$check.Id] = Get-RegionFingerprint -Image $image -Rect $rect
    }

    $colors = New-Object System.Collections.Generic.HashSet[int]
    for ($offset = 0; $offset -lt $image.Bytes.Length; $offset += 4) {
        [void]$colors.Add(
            ($image.Bytes[$offset + 2] -shl 16) -bor
            ($image.Bytes[$offset + 1] -shl 8) -bor
            $image.Bytes[$offset])
    }
    if ($colors.Count -lt $minimumPageColors) {
        $problems.Add("the page holds $($colors.Count) distinct colours, so it did not draw")
    }
    else {
        $asserted.Add("$($colors.Count) distinct colours on the page")
    }

    return @{ Problems = $problems; Asserted = $asserted; Regions = $regions }
}

function Get-Expectation {
    param([string]$Locale, [int]$Dpi, [hashtable]$Page, $Config, $LocaleText)

    $scale = $Dpi / 96.0
    $width = [int][Math]::Round($Page.Width * $scale)
    $height = [int][Math]::Round($Page.Height * $scale)
    $scaling = [int][Math]::Round($scale * 100)
    $installButton = $LocaleText.install_button
    $versionPrefix = $LocaleText.version_info
    $agreement = $LocaleText.agree_full
    $version = $Config.project.version
    return @"
The first page of the TapTap setup, captured in $Locale at $scaling% display scaling ($width x $height client area).
It is a borderless dark window with a background image, rounded corners, a language selector in the top right, and a minimize button and a close button beside it.
In the middle of the page a logo and, just under it, a tagline image must both be visible, not blank and not a white box.
Under them a blue install button must read exactly: "$installButton".
Near the bottom a version line, centred, must read exactly: "$versionPrefix$version".
Below that a checkbox line must show the agreement text: "$agreement".
Report anything wrong: missing glyphs shown as boxes, mojibake, clipped or overlapping text, a missing or blank image, a wrong language, or a region that did not draw at all.
"@
}

# The project decides what is captured: its supported locales and the layout of
# the page it opens first.
$config = Read-JsonFile -Path (Join-Path $projectRoot "installer_config.json")
$pageLayoutPath = Join-Path $projectRoot ([string]$config.wizard.pages[0].layout)
[xml]$layout = Get-Content -Raw -Encoding UTF8 -LiteralPath $pageLayoutPath
$page = @{
    Width  = [double]$layout.Page.width
    Height = [double]$layout.Page.height
    Layout = [string]$config.wizard.pages[0].layout
}
$defaultLocale = [string]$config.localization.default_locale
if (-not $Locales -or $Locales.Count -eq 0) {
    $Locales = @($config.localization.supported_locales)
}
$localeText = @{}
foreach ($locale in $Locales) {
    $file = Join-Path $projectRoot "locales/$locale.json"
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Locale file not found: $file"
    }
    $localeText[$locale] = Read-JsonFile -Path $file
}

# The first scaling captures every locale; the later ones only the default, so a
# scaling problem is caught without multiplying the whole run by the scaling.
$targets = New-Object System.Collections.Generic.List[object]
for ($index = 0; $index -lt $Dpi.Count; $index++) {
    $scaling = $Dpi[$index]
    $wanted = $Locales
    if ($index -gt 0) { $wanted = @($defaultLocale) }
    foreach ($locale in $wanted) {
        $targets.Add(@{ Locale = $locale; Dpi = $scaling })
    }
}

$problems = New-Object System.Collections.Generic.List[string]
$snapshots = New-Object System.Collections.Generic.List[object]
$fingerprints = @{}

# The example asks for elevation, and that request is written into the setup's
# requestedExecutionLevel: Windows shows its consent prompt before the window
# exists, so an unattended run would stop on the prompt. Clearing the flag for
# the build is the documented way to get a setup that never prompts, so this
# clears it, builds, and puts the file back byte for byte afterwards.
# -KeepElevation leaves the project alone and waits for a hand on the prompt.
$configPath = Join-Path $projectRoot "installer_config.json"
$configBytes = [System.IO.File]::ReadAllBytes($configPath)
$clearedElevation = $false

# The example's payload is a 150 MB archive the repository does not track, so a
# machine that has never unpacked the example has no payload, and the build
# refuses to run without one. The build reads the declared archive's format and
# embeds it untouched, the first page does not depend on what is inside it, and a
# run that photographs the page never installs anything. A project that has no
# payload therefore gets an empty archive of the declared format for this build,
# and gets it removed again afterwards.
$payloadDeclared = $null
$resources = $config.PSObject.Properties["resources"]
if ($null -ne $resources) {
    $declaredPayload = $resources.Value.PSObject.Properties["payload_file"]
    if ($null -ne $declaredPayload) { $payloadDeclared = [string]$declaredPayload.Value }
}
$placeholderPath = $null
$wrotePlaceholder = $false

try {
    New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null

    if ($payloadDeclared) {
        $payloadPath = Join-Path $projectRoot $payloadDeclared
        $present = (Test-Path -LiteralPath $payloadPath -PathType Leaf) -and
            (Get-Item -LiteralPath $payloadPath).Length -ge 6
        if (-not $present) {
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $payloadPath) | Out-Null
            # An empty ZIP: the signature the build reads, and nothing to unpack.
            [System.IO.File]::WriteAllBytes($payloadPath, [byte[]]@(0x50, 0x4B, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0))
            $placeholderPath = $payloadPath
            $wrotePlaceholder = $true
            Write-Output "Wrote a placeholder payload at $payloadPath because the project declares none"
        }
    }

    if ($KeepElevation) {
        Write-Output "Keeping install.require_admin as declared; answer the consent prompt when it appears"
    }
    elseif ([System.Text.Encoding]::UTF8.GetString($configBytes) -match '"require_admin"\s*:\s*true') {
        $patched = [regex]::Replace(
            [System.Text.Encoding]::UTF8.GetString($configBytes),
            '("require_admin"\s*:\s*)true',
            '${1}false')
        [System.IO.File]::WriteAllBytes($configPath, (New-Object System.Text.UTF8Encoding($false)).GetBytes($patched))
        $clearedElevation = $true
        Write-Output "Cleared install.require_admin for this build"
    }

    $setup = Join-Path $OutputDirectory "TapTap_Setup.exe"
    Write-Output "Building $($config.project.name) with $Builder"
    $build = Invoke-NativeCapture { & $Builder build --project $projectRoot --output $setup --stubs $StubDirectory }
    if ($build.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $setup -PathType Leaf)) {
        throw "The setup did not build: $($build.Output -join [Environment]::NewLine)"
    }

    foreach ($target in $targets) {
        $name = "$($target.Locale)-$($target.Dpi)dpi"
        $path = Join-Path $OutputDirectory "$name.png"
        Write-Output "Capturing $name"
        $capture = Save-Snapshot -Setup $setup -Locale $target.Locale -Dpi $target.Dpi -Path $path
        Write-Output "  window at $($capture.Window)"
        $result = Test-Snapshot -Path $path -Dpi $target.Dpi -Page $page -Layout $layout -Window $capture
        foreach ($problem in $result.Problems) {
            $problems.Add("${name}: $problem")
        }
        foreach ($id in $result.Regions.Keys) {
            $fingerprints["$name/$id"] = $result.Regions[$id]
        }
        $snapshots.Add([ordered]@{
            file        = "$name.png"
            locale      = $target.Locale
            dpi         = $target.Dpi
            scaling     = [int][Math]::Round(($target.Dpi / 96.0) * 100)
            width       = [int][Math]::Round($page.Width * ($target.Dpi / 96.0))
            height      = [int][Math]::Round($page.Height * ($target.Dpi / 96.0))
            asserted    = @($result.Asserted)
            expectation = Get-Expectation -Locale $target.Locale -Dpi $target.Dpi -Page $page -Config $config -LocaleText $localeText[$target.Locale]
        })
        Write-Output "  $($result.Asserted -join '; ')"
    }

    # The same button region in two languages has to differ: identical pixels
    # would mean the locale never reached the page.
    $first = $targets[0]
    foreach ($target in $targets) {
        if ($target.Dpi -ne $first.Dpi -or $target.Locale -eq $first.Locale) { continue }
        foreach ($id in $text | ForEach-Object { $_.Id }) {
            $left = $fingerprints["$($first.Locale)-$($first.Dpi)dpi/$id"]
            $right = $fingerprints["$($target.Locale)-$($target.Dpi)dpi/$id"]
            if ($null -eq $left -or $null -eq $right) { continue }
            if ($left -eq $right) {
                $problems.Add("$($first.Locale) and $($target.Locale) drew <$id> pixel for pixel the same, so the locale never reached it")
            }
            else {
                Write-Output "  <$id> differs between $($first.Locale) and $($target.Locale)"
            }
        }
    }

    $manifest = [ordered]@{
        setup     = "TapTap_Setup.exe"
        project   = $config.project.name
        page      = [ordered]@{ layout = $page.Layout; width = $page.Width; height = $page.Height }
        snapshots = $snapshots
    }
    [System.IO.File]::WriteAllText(
        (Join-Path $OutputDirectory "manifest.json"),
        ($manifest | ConvertTo-Json -Depth 8),
        (New-Object System.Text.UTF8Encoding($false)))
}
finally {
    if ($clearedElevation) {
        [System.IO.File]::WriteAllBytes($configPath, $configBytes)
        Write-Output "Restored $configPath"
    }
    if ($wrotePlaceholder) {
        Remove-Item -LiteralPath $placeholderPath -Force -ErrorAction SilentlyContinue
        Write-Output "Removed the placeholder payload $placeholderPath"
    }
}

if ($problems.Count -gt 0) {
    $problems | ForEach-Object { Write-Error $_ }
    throw "$($problems.Count) snapshot check(s) failed"
}

Write-Output "Captured and checked $($snapshots.Count) snapshot(s) in $OutputDirectory"
