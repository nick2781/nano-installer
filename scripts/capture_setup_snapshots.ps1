<#
    Captures the pages the example setup draws.

    The subject is examples/TapTap itself: the script builds a real setup from
    the project, then saves the window it opens as a PNG. It photographs every
    page the project declares -- the wizard's and the uninstaller's -- because a
    setup opens the first page of the wizard and nothing else: for each page the
    build runs with that page placed first, and the project's configuration is
    written back byte for byte once the captures are done. A page captured this
    way is drawn, not reached, and the report says so: the uninstaller's pages
    appear here without an uninstall ever having run.

    The wizard's first page is the one a reader meets, so it is captured once
    per supported locale and once at each scaling the -Dpi list names. The
    default list captures 100%, 150% and 200%, so a page a reader sees at 200%
    on their own display is in the report at 200% too: the picture of a scaled
    window is the scaled number of pixels, not the 100% one stretched. Every
    other page is captured once, in the default locale at 100%, since what it
    adds is the page itself rather than another scaling of it. The script never
    presses the install button, so nothing is deployed: the window is closed as
    soon as the picture is taken, and the project is left exactly as it was
    found. A project whose declared payload archive is not there, which is the
    case for a clone that never unpacked the example, is built against an empty
    archive of the same format; a page is drawn before anything is unpacked, so
    nothing has to be inside it.

    The page size, the elements that must hold artwork or text, and the words
    the labels show are all read from each page's own layout, so a layout that
    moves is measured where it moved to. What is checked is arithmetic and runs
    on any machine: the client area matches the page, every image and label the
    layout places by absolute coordinates drew something rather than a flat
    patch, no page holds so few colours that it cannot have drawn, and no two
    pages came out as the same picture -- two identical pages would mean the
    page a snapshot asked for never reached the setup. The wizard's first page
    is the one a reader studies, so its logo, tagline, install button and
    version line are held to a bar that catches a half-drawn page rather than
    only a blank one.

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
    [int[]]$Dpi = @(96, 144, 192),
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

# What a page is checked on, read from the page's own layout rather than from a
# list of ids: an element the layout places by absolute coordinates is measured
# where it was placed, and the element's id names it in the report. A layout that
# positions by container -- a page built out of VBox and HBox -- leaves nothing
# for an element check to measure, and such a page keeps the checks that need no
# element: its size, its colours, its shape, and its difference from the others.
#
# The bar is low on purpose. A snapshot is worth reporting when a region that
# should hold something holds nothing, while telling "drew slightly less than
# expected" apart from "did not draw" needs a person, and this script has none.
# The four parts of the first page are the ones a reader studies, so those are
# held high.
$minimumArtworkColors = 2
$minimumTextPixels = 16
$scrutinised = @{
    "logo"       = @{ Kind = "artwork"; MinimumColors = 16 }
    "tagline"    = @{ Kind = "artwork"; MinimumColors = 16 }
    "btnInstall" = @{ Kind = "text"; MinimumPixels = 100 }
    "lblVersion" = @{ Kind = "text"; MinimumPixels = 100 }
}
# An image that never decoded, or a label that never painted, leaves one colour
# where the layout asked for content; anything actually drawn clears these by a
# wide margin. A page of nothing but a fill holds one colour, not sixty-four.
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

# Where an element sits on the page: the rectangle it declares, plus the offset
# of every absolute ancestor between it and the page, which is how the runtime
# places it. An element the layout sizes or positions in percentages, or one a
# container lays out, has no rectangle here and is left out of the checks rather
# than failed on arithmetic the runtime does differently.
function Get-AbsoluteElementRect {
    param([System.Xml.XmlElement]$Node, [double]$Scale)

    $left = ConvertTo-Pixels -Node $Node -Attribute "left"
    $top = ConvertTo-Pixels -Node $Node -Attribute "top"
    $width = ConvertTo-Pixels -Node $Node -Attribute "width"
    $height = ConvertTo-Pixels -Node $Node -Attribute "height"
    if ($null -eq $left -or $null -eq $top -or $null -eq $width -or $null -eq $height) {
        return $null
    }
    $ancestor = $Node.ParentNode
    while ($null -ne $ancestor -and $ancestor -is [System.Xml.XmlElement]) {
        if ($ancestor.LocalName -eq "Page") {
            break
        }
        if ($ancestor.GetAttribute("position") -eq "absolute") {
            $offsetLeft = ConvertTo-Pixels -Node $ancestor -Attribute "left"
            $offsetTop = ConvertTo-Pixels -Node $ancestor -Attribute "top"
            if ($null -ne $offsetLeft) { $left += $offsetLeft }
            if ($null -ne $offsetTop) { $top += $offsetTop }
        }
        $ancestor = $ancestor.ParentNode
    }
    return @{
        Left   = [int][Math]::Round($left * $Scale)
        Top    = [int][Math]::Round($top * $Scale)
        Right  = [int][Math]::Round(($left + $width) * $Scale)
        Bottom = [int][Math]::Round(($top + $height) * $Scale)
    }
}

# A layout hides an element with visible="false", and hiding a container hides
# everything the container holds, so a hidden element is not measured.
function Test-ElementVisible {
    param([System.Xml.XmlElement]$Node)

    $current = $Node
    while ($null -ne $current -and $current -is [System.Xml.XmlElement]) {
        if ($current.GetAttribute("visible") -eq "false") {
            return $false
        }
        $current = $current.ParentNode
    }
    return $true
}

# Every element of a page the checks can measure: it carries an id, it is one of
# the tags that draws, it is visible as the layout stands, and it sits at an
# absolute rectangle. An image is measured for artwork and a label for text in
# the colour the layout declares; anything else is left to the reader, since a
# progress bar with nothing running has nothing filled.
function Get-PageElements {
    param([xml]$Layout, [double]$Scale)

    $elements = New-Object System.Collections.Generic.List[object]
    $seen = @{}
    foreach ($node in $Layout.SelectNodes("//*[@id]")) {
        $id = [string]$node.GetAttribute("id")
        if (-not $id -or $seen.ContainsKey($id)) {
            continue
        }
        $seen[$id] = $true
        if (-not (Test-ElementVisible -Node $node)) {
            continue
        }
        $tag = $node.LocalName
        if ($tag -notin @("Image", "Icon", "Button", "Label", "Checkbox", "TextInput")) {
            continue
        }
        $rect = Get-AbsoluteElementRect -Node $node -Scale $Scale
        if ($null -eq $rect) {
            continue
        }
        $text = [string]$node.GetAttribute("text")
        $hasArtwork = $node.HasAttribute("src") -or $node.HasAttribute("normal-image") -or
            $node.HasAttribute("background-image")
        if ($tag -eq "Image" -or $tag -eq "Icon" -or ($hasArtwork -and -not $text)) {
            $elements.Add(@{ Id = $id; Kind = "artwork"; Rect = $rect })
            continue
        }
        $declared = ConvertFrom-ArgbHex -Value $node.GetAttribute("color")
        if ($null -eq $declared -or -not $text) {
            continue
        }
        # A label whose text comes from the run rather than from the layout --
        # a progress line, say -- shows nothing while nothing is running, so it
        # is not measured. A value read from the configuration is there.
        $source = [string]$node.GetAttribute("value-source")
        if ($source -and -not $source.StartsWith("config:")) {
            continue
        }
        $elements.Add(@{
            Id         = $id
            Kind       = "text"
            Rect       = $rect
            Colour     = $declared
            ColourText = $node.GetAttribute("color")
        })
    }
    return $elements.ToArray()
}

# The pages the project declares, in the order it declares them, each named by
# the list it came from and numbered as a person counts it. The title is the
# project's own name for the page, which the locale files do not translate.
function Get-DeclaredPages {
    param($Config)

    $pages = New-Object System.Collections.Generic.List[object]
    foreach ($declaration in @(
            @{ Field = "pages"; Role = "install" },
            @{ Field = "uninstall_pages"; Role = "uninstall" })) {
        $list = $Config.wizard.PSObject.Properties[$declaration.Field]
        if ($null -eq $list -or $null -eq $list.Value) {
            continue
        }
        $index = 0
        foreach ($entry in $list.Value) {
            $layout = [string]$entry.layout
            if (-not $layout) {
                continue
            }
            $index++
            $title = Split-Path -Leaf $layout
            $named = $entry.PSObject.Properties["title"]
            if ($null -ne $named -and [string]$named.Value) {
                $title = [string]$named.Value
            }
            $pages.Add(@{
                Id     = "$($declaration.Role)-$index"
                Role   = $declaration.Role
                Index  = $index
                Layout = $layout
                Title  = $title
            })
        }
    }
    return $pages.ToArray()
}

# The project's configuration with one page placed first, and with the elevation
# request cleared unless -KeepElevation asked to keep it. A setup opens
# wizard.pages[0], so that is the entry a page is captured through; nothing else
# in the file is touched, and the file itself is written back byte for byte when
# the captures are done.
function Get-PatchedConfig {
    param([byte[]]$Bytes, [string]$Layout, [switch]$KeepElevation)

    $text = [System.Text.Encoding]::UTF8.GetString($Bytes)
    $pattern = '("pages"\s*:\s*\[\s*\{[^}]*?"layout"\s*:\s*")[^"]+(")'
    $match = [regex]::Match($text, $pattern)
    if (-not $match.Success) {
        throw "No wizard.pages[0].layout to place a page in"
    }
    $patched = $text.Substring(0, $match.Groups[1].Index + $match.Groups[1].Length) +
        $Layout + $text.Substring($match.Groups[2].Index)
    if (-not $KeepElevation -and $patched -match '"require_admin"\s*:\s*true') {
        $patched = [regex]::Replace($patched, '("require_admin"\s*:\s*)true', '${1}false')
    }
    return (New-Object System.Text.UTF8Encoding($false)).GetBytes($patched)
}

# The whole picture as one hash, so two pages can be told apart without
# comparing them pixel by pixel.
function Get-ImageFingerprint {
    param($Image)

    $hash = [System.Security.Cryptography.MD5]::Create()
    try {
        return [BitConverter]::ToString($hash.ComputeHash($Image.Bytes))
    }
    finally {
        $hash.Dispose()
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
    param([string]$Path, [int]$Dpi, [hashtable]$Page, [hashtable]$Size, [xml]$Layout, [hashtable]$Window)

    $problems = New-Object System.Collections.Generic.List[string]
    $asserted = New-Object System.Collections.Generic.List[object]
    $regions = @{}
    $scale = $Dpi / 96.0
    $image = Get-SnapshotPixels -Path $Path

    $expectedWidth = [int][Math]::Round($Size.Width * $scale)
    $expectedHeight = [int][Math]::Round($Size.Height * $scale)
    if ($image.Width -ne $expectedWidth -or $image.Height -ne $expectedHeight) {
        $problems.Add("the client area is $($image.Width)x$($image.Height), the page declares ${expectedWidth}x${expectedHeight}")
    }
    else {
        $asserted.Add(@{ k = "size"; v = @("$expectedWidth", "$expectedHeight") })
    }

    # A page that declares a rounded corner has to come out rounded; a page that
    # declares none is free to come out either way, and only says which it did.
    $radius = ConvertTo-Pixels -Node $Layout.Page -Attribute "border-radius"
    $rounded = ($null -ne $radius -and $radius -gt 0)
    if ($null -eq $Window.CornerPoints) {
        $asserted.Add(@{ k = "region" })
    }
    elseif ($Window.CornerPoints -eq 0 -and $Window.MiddleInside -eq 1) {
        $asserted.Add(@{ k = "corners" })
    }
    elseif ($rounded) {
        $problems.Add("$($Window.CornerPoints) of 4 corner points fall inside the window region, and the page declares a border-radius of $radius, so its corners are not cut")
    }
    else {
        $asserted.Add(@{ k = "square" })
    }

    $measured = @{}
    foreach ($element in (Get-PageElements -Layout $Layout -Scale $scale)) {
        $rule = @{ MinimumColors = $minimumArtworkColors; MinimumPixels = $minimumTextPixels }
        $scrutiny = $scrutinised[$element.Id]
        if ($null -ne $scrutiny -and $scrutiny.Kind -eq $element.Kind) {
            $rule = $scrutiny
        }
        $measured[$element.Id] = $true
        if ($element.Kind -eq "artwork") {
            $stats = Get-RegionStats -Image $image -Rect $element.Rect
            if ($stats.Colors -lt $rule.MinimumColors) {
                $problems.Add("the <$($element.Id)> drew $($stats.Colors) colours, so it did not draw")
            }
            else {
                $asserted.Add(@{ k = "artwork"; v = @($element.Id, "$($stats.Colors)") })
            }
            continue
        }
        $stats = Get-RegionStats -Image $image -Rect $element.Rect -Target $element.Colour
        if ($stats.TargetPixels -lt $rule.MinimumPixels) {
            $problems.Add("the <$($element.Id)> drew $($stats.TargetPixels) pixels in $($element.ColourText), so its text is missing")
        }
        else {
            $asserted.Add(@{ k = "text"; v = @($element.Id, "$($stats.TargetPixels)", $element.ColourText) })
        }
        $regions[$element.Id] = Get-RegionFingerprint -Image $image -Rect $element.Rect
    }

    # The four parts a reader studies are the first page's own, so a layout that
    # no longer declares one of them is reported rather than quietly left out.
    if ($Page.Role -eq "install" -and $Page.Index -eq 1) {
        foreach ($id in ($scrutinised.Keys | Sort-Object)) {
            if (-not $measured.ContainsKey($id)) {
                $problems.Add("the layout of the first page no longer declares <$id>")
            }
        }
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
        $asserted.Add(@{ k = "colours"; v = @("$($colors.Count)") })
    }

    return @{
        Problems    = $problems
        Asserted    = $asserted.ToArray()
        Regions     = $regions
        Fingerprint = (Get-ImageFingerprint -Image $image)
    }
}

# What a page is meant to show, for the vision review. Whether glyphs read as
# words is a judgement, so every page is described in words, and the strings the
# locale supplies are quoted from the locale file its picture was taken with.
function Get-PageExpectation {
    param([hashtable]$Page, [hashtable]$Size, [string]$Locale, [int]$Dpi, $Config, $LocaleText)

    $scale = $Dpi / 96.0
    $width = [int][Math]::Round($Size.Width * $scale)
    $height = [int][Math]::Round($Size.Height * $scale)
    $scaling = [int][Math]::Round($scale * 100)
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("The $($Page.Title) page of the $($Config.project.name) setup, the layout $($Page.Layout), captured in $Locale at $scaling% display scaling ($width x $height client area).")
    switch (Split-Path -Leaf $Page.Layout) {
        "configpage.xml" {
            $lines.Add("It is a borderless dark window with a background image and rounded corners: a language selector, a minimise button and a close button in the top right, a logo and, under it, a tagline image in the middle, and below them the install button, the install folder field, three checkboxes, a version line and the agreement line.")
            $lines.Add("The install button must read exactly: ""$($LocaleText.install_button)"".")
            $lines.Add("The version line must read exactly: ""$($LocaleText.version_info)$($Config.project.version)"".")
            $lines.Add("The agreement line must show: ""$($LocaleText.agree_full)"".")
        }
        "installingpage.xml" {
            $lines.Add("It is the same window with a progress bar across the middle and a status line under it. No install is running while this picture is taken, so the bar may still be empty and the status line blank, but both must be drawn rather than missing.")
        }
        "finishpage.xml" {
            $lines.Add("It is the same window with a logo and, under it, a tagline image in the middle, a blue button below them, and a completion line near the bottom.")
            $lines.Add("The button must read exactly: ""$($LocaleText.launch_button)"".")
            $lines.Add("The completion line must read exactly: ""$($LocaleText.install_complete)"".")
        }
        "uninstallpage.xml" {
            $lines.Add("It is a smaller dark window: a logo at the top, a confirmation question in the middle, a checkbox under it, and two buttons at the bottom.")
            $lines.Add("The question must read exactly: ""$($LocaleText.uninstall_confirm)"".")
            $lines.Add("The checkbox must read: ""$($LocaleText.reserve_data)"".")
            $lines.Add("The two buttons must read exactly: ""$($LocaleText.not_now)"" and ""$($LocaleText.uninstall_button)"".")
        }
        "uninstallingpage.xml" {
            $lines.Add("It is a smaller dark window with a progress bar across it and a status line under the bar. No uninstall is running while this picture is taken, so the bar may still be empty and the status line blank, but both must be drawn rather than missing.")
        }
        "uninstallfinishpage.xml" {
            $lines.Add("It is a smaller dark window with a logo at the top, a completion line under it, and one button below.")
            $lines.Add("The completion line must read exactly: ""$($LocaleText.uninstall_complete)"".")
            $lines.Add("The button must read exactly: ""$($LocaleText.uninstall_done)"".")
        }
        default {
            $lines.Add("Its layout places the elements below; check that each of them drew.")
        }
    }
    $lines.Add("Report anything wrong: missing glyphs shown as boxes, mojibake, clipped or overlapping text, a missing or blank image, a wrong language, or a region that did not draw at all.")
    return ($lines -join "`r`n")
}

# The project decides what is captured: its supported locales and the pages it
# declares. A setup opens the first page of the wizard and nothing else, so a
# page is captured by building the setup with that page placed first, and the
# project's configuration is written back byte for byte afterwards.
$config = Read-JsonFile -Path (Join-Path $projectRoot "installer_config.json")
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
$declaredPages = @(Get-DeclaredPages -Config $config)
if ($declaredPages.Count -eq 0) {
    throw "The project declares no page to capture: $projectRoot"
}

$problems = New-Object System.Collections.Generic.List[string]
$snapshots = New-Object System.Collections.Generic.List[object]
$pageSizes = @{}
$pageFingerprints = @{}
$fingerprints = @{}

# The example asks for elevation, and that request is written into the setup's
# requestedExecutionLevel: Windows shows its consent prompt before the window
# exists, so an unattended run would stop on the prompt. Clearing the flag for
# the builds is the documented way to get a setup that never prompts, so this
# clears it, builds, and puts the file back byte for byte afterwards.
# -KeepElevation leaves the project alone and waits for a hand on the prompt.
$configPath = Join-Path $projectRoot "installer_config.json"
$configBytes = [System.IO.File]::ReadAllBytes($configPath)
$patchedConfig = $false

# The example's payload is a 150 MB archive the repository does not track, so a
# machine that has never unpacked the example has no payload, and the build
# refuses to run without one. The build reads the declared archive's format and
# embeds it untouched, a page is drawn before anything is unpacked, and a run
# that photographs a page never installs anything. A project that has no payload
# therefore gets an empty archive of the declared format for these builds, and
# gets it removed again afterwards.
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
    else {
        Write-Output "Cleared install.require_admin for these builds"
    }

    $setup = Join-Path $OutputDirectory "TapTap_Setup.exe"
    foreach ($page in $declaredPages) {
        # The wizard's first page is the one a reader meets: every locale, and
        # every scaling -Dpi names. Every other page is captured once, in the
        # default locale at 100%, because what it adds is the page itself.
        $targets = New-Object System.Collections.Generic.List[object]
        if ($page.Role -eq "install" -and $page.Index -eq 1) {
            for ($index = 0; $index -lt $Dpi.Count; $index++) {
                $wanted = $Locales
                if ($index -gt 0) { $wanted = @($defaultLocale) }
                foreach ($locale in $wanted) {
                    $targets.Add(@{ Locale = $locale; Dpi = $Dpi[$index] })
                }
            }
        }
        else {
            $targets.Add(@{ Locale = $defaultLocale; Dpi = 96 })
        }

        [System.IO.File]::WriteAllBytes($configPath, (Get-PatchedConfig -Bytes $configBytes -Layout $page.Layout -KeepElevation:$KeepElevation))
        $patchedConfig = $true
        Write-Output "Building $($config.project.name) with $($page.Layout) first, using $Builder"
        $build = Invoke-NativeCapture { & $Builder build --project $projectRoot --output $setup --stubs $StubDirectory }
        if ($build.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $setup -PathType Leaf)) {
            throw "The setup did not build: $($build.Output -join [Environment]::NewLine)"
        }

        $layoutPath = Join-Path $projectRoot $page.Layout
        if (-not (Test-Path -LiteralPath $layoutPath -PathType Leaf)) {
            throw "Layout not found: $layoutPath"
        }
        [xml]$layout = Get-Content -Raw -Encoding UTF8 -LiteralPath $layoutPath
        $size = @{ Width = [double]$layout.Page.width; Height = [double]$layout.Page.height }
        $pageSizes[$page.Id] = $size

        foreach ($target in $targets) {
            $name = "$($page.Id)-$($target.Locale)-$($target.Dpi)dpi"
            $path = Join-Path $OutputDirectory "$name.png"
            Write-Output "Capturing $name"
            $capture = Save-Snapshot -Setup $setup -Locale $target.Locale -Dpi $target.Dpi -Path $path
            Write-Output "  window at $($capture.Window)"
            $result = Test-Snapshot -Path $path -Dpi $target.Dpi -Page $page -Size $size -Layout $layout -Window $capture
            foreach ($problem in $result.Problems) {
                $problems.Add("${name}: $problem")
            }
            foreach ($id in $result.Regions.Keys) {
                $fingerprints["$name/$id"] = $result.Regions[$id]
            }
            if ($target.Locale -eq $defaultLocale -and $target.Dpi -eq 96) {
                $pageFingerprints[$page.Id] = $result.Fingerprint
            }
            $snapshots.Add([ordered]@{
                file        = "$name.png"
                page        = $page.Id
                locale      = $target.Locale
                dpi         = $target.Dpi
                scaling     = [int][Math]::Round(($target.Dpi / 96.0) * 100)
                width       = [int][Math]::Round($size.Width * ($target.Dpi / 96.0))
                height      = [int][Math]::Round($size.Height * ($target.Dpi / 96.0))
                asserted    = $result.Asserted
                expectation = Get-PageExpectation -Page $page -Size $size -Locale $target.Locale -Dpi $target.Dpi -Config $config -LocaleText $localeText[$target.Locale]
            })
            Write-Output "  $($result.Asserted.Count) check(s) measured"
        }

        # The same element in two languages has to differ: identical pixels would
        # mean the locale never reached the page. Only the first page is captured
        # in more than one language.
        if ($page.Role -eq "install" -and $page.Index -eq 1) {
            $first = $targets[0]
            foreach ($target in $targets) {
                if ($target.Dpi -ne $first.Dpi -or $target.Locale -eq $first.Locale) { continue }
                foreach ($id in @($scrutinised.Keys)) {
                    $left = $fingerprints["$($page.Id)-$($first.Locale)-$($first.Dpi)dpi/$id"]
                    $right = $fingerprints["$($page.Id)-$($target.Locale)-$($target.Dpi)dpi/$id"]
                    if ($null -eq $left -or $null -eq $right) { continue }
                    if ($left -eq $right) {
                        $problems.Add("$($first.Locale) and $($target.Locale) drew <$id> pixel for pixel the same, so the locale never reached it")
                    }
                    else {
                        Write-Output "  <$id> differs between $($first.Locale) and $($target.Locale)"
                    }
                }
            }
        }
    }

    # Two pages that came out pixel for pixel the same would mean the page a
    # snapshot asked for never reached the setup: every picture would be the
    # wizard's first page.
    $firstPage = $declaredPages[0]
    foreach ($page in $declaredPages) {
        if ($page.Id -eq $firstPage.Id) { continue }
        $fingerprint = $pageFingerprints[$page.Id]
        if (-not $fingerprint) { continue }
        if ($fingerprint -eq $pageFingerprints[$firstPage.Id]) {
            $problems.Add("$($page.Id) came out pixel for pixel the same as $($firstPage.Id), so the page never reached the setup")
            continue
        }
        foreach ($snapshot in $snapshots) {
            if ($snapshot["page"] -eq $page.Id -and $snapshot["locale"] -eq $defaultLocale -and $snapshot["dpi"] -eq 96) {
                $snapshot["asserted"] = $snapshot["asserted"] + @(@{ k = "differs" })
            }
        }
        Write-Output "  $($page.Id) differs from $($firstPage.Id)"
    }

    $manifest = [ordered]@{
        setup     = "TapTap_Setup.exe"
        project   = $config.project.name
        pages     = @(foreach ($page in $declaredPages) {
                [ordered]@{
                    id     = $page.Id
                    role   = $page.Role
                    index  = $page.Index
                    title  = $page.Title
                    layout = $page.Layout
                    width  = [int][Math]::Round($pageSizes[$page.Id].Width)
                    height = [int][Math]::Round($pageSizes[$page.Id].Height)
                }
            })
        snapshots = $snapshots
    }
    [System.IO.File]::WriteAllText(
        (Join-Path $OutputDirectory "manifest.json"),
        ($manifest | ConvertTo-Json -Depth 8),
        (New-Object System.Text.UTF8Encoding($false)))
}
finally {
    if ($patchedConfig) {
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

Write-Output "Captured and checked $($snapshots.Count) snapshot(s) of $(@($declaredPages).Count) page(s) in $OutputDirectory"
