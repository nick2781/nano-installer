<#
    Asks Windows which cursor it is showing, at a grid of screen points.

    The acceptance run records what a user sees, and one item on the checklist is the
    cursor a page asks for with `cursor="hand"`. A screenshot cannot answer that: the
    hardware cursor is not part of what is captured. What can answer it is the machine
    itself, so run this in the guest with the wizard in front -- it walks the screen,
    calls `GetCursorInfo` at each point and reports where the cursor is the hand one.

    Run it in the guest, not here: powershell -ep bypass -f F:\cursor_probe.ps1
#>
param(
    [int]$Step = 60
)

$ErrorActionPreference = 'Continue'

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class CursorProbe {
    [StructLayout(LayoutKind.Sequential)]
    public struct CURSORINFO { public int cbSize; public int flags; public IntPtr hCursor; public int x; public int y; }
    [DllImport("user32.dll")] public static extern bool GetCursorInfo(ref CURSORINFO pci);
    [DllImport("user32.dll")] public static extern IntPtr LoadCursor(IntPtr hInstance, int lpCursorName);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
}
"@

# IDC_HAND and IDC_ARROW, from the resource ids every Windows ships.
$hand = [CursorProbe]::LoadCursor([IntPtr]::Zero, 32649)
$arrow = [CursorProbe]::LoadCursor([IntPtr]::Zero, 32512)
$width = [CursorProbe]::GetSystemMetrics(0)
$height = [CursorProbe]::GetSystemMetrics(1)
Write-Output "screen: $width x $height, step $Step"

$handPoints = New-Object System.Collections.ArrayList
$otherPoints = @{}
for ($y = 20; $y -lt ($height - 20); $y += $Step) {
    for ($x = 20; $x -lt ($width - 20); $x += $Step) {
        [void][CursorProbe]::SetCursorPos($x, $y)
        # The window under the cursor has to notice the move before it sets a shape.
        Start-Sleep -Milliseconds 35
        $info = New-Object CursorProbe+CURSORINFO
        $info.cbSize = [Runtime.InteropServices.Marshal]::SizeOf($info)
        if ([CursorProbe]::GetCursorInfo([ref]$info)) {
            if ($info.hCursor -eq $hand) { [void]$handPoints.Add("$x,$y") }
            elseif ($info.hCursor -ne $arrow) { $otherPoints["$($info.hCursor)"] = "$x,$y" }
        }
    }
}

Write-Output "hand-cursor points: $($handPoints.Count)"
if ($handPoints.Count -gt 0) {
    # The points come in runs, one run per control that asks for the hand.
    Write-Output "  first few: $(($handPoints | Select-Object -First 15) -join ' ')"
}
Write-Output "other cursors seen: $($otherPoints.Count)"
foreach ($handle in $otherPoints.Keys) { Write-Output "  handle $handle first at $($otherPoints[$handle])" }
if ($handPoints.Count -eq 0) {
    Write-Output 'no control asked for the hand cursor while this ran'
    exit 1
}