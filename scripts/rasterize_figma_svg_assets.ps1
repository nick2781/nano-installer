Add-Type -AssemblyName PresentationCore
Add-Type -AssemblyName WindowsBase

function Get-BrushFromSvgPath {
    param($PathNode)

    $fill = $PathNode.fill
    $fillOpacity = 1.0
    if ($PathNode.'fill-opacity') {
        $fillOpacity = [double]$PathNode.'fill-opacity'
    }

    if ($fill -match 'white') {
        $alpha = [math]::Round(255 * $fillOpacity)
        return New-Object System.Windows.Media.SolidColorBrush(
            [System.Windows.Media.Color]::FromArgb([byte]$alpha, 255, 255, 255)
        )
    }

    if ($fill -match '^#([0-9A-Fa-f]{6})$') {
        $hex = $Matches[1]
        $alpha = [math]::Round(255 * $fillOpacity)
        $r = [Convert]::ToByte($hex.Substring(0, 2), 16)
        $g = [Convert]::ToByte($hex.Substring(2, 2), 16)
        $b = [Convert]::ToByte($hex.Substring(4, 2), 16)
        return New-Object System.Windows.Media.SolidColorBrush(
            [System.Windows.Media.Color]::FromArgb([byte]$alpha, $r, $g, $b)
        )
    }

    return New-Object System.Windows.Media.SolidColorBrush(
        [System.Windows.Media.Color]::FromArgb(255, 255, 255, 255)
    )
}

function Export-SvgToPng {
    param(
        [string]$SvgPath,
        [string]$OutputPath,
        [int]$TargetWidth,
        [int]$TargetHeight,
        [double]$DrawX = 0,
        [double]$DrawY = 0,
        [double]$DrawWidth = 0,
        [double]$DrawHeight = 0,
        [switch]$FlipVertical
    )

    [xml]$svg = Get-Content $SvgPath -Raw
    $viewBox = ($svg.svg.viewBox -split '\s+') | Where-Object { $_ -ne '' }
    $vbWidth = [double]$viewBox[2]
    $vbHeight = [double]$viewBox[3]
    if ($DrawWidth -le 0) {
        $DrawWidth = $TargetWidth
    }
    if ($DrawHeight -le 0) {
        $DrawHeight = $TargetHeight
    }

    $visual = New-Object System.Windows.Media.DrawingVisual
    $dc = $visual.RenderOpen()

    foreach ($pathNode in $svg.SelectNodes('//*[local-name()="path"]')) {
        $geometry = ([System.Windows.Media.Geometry]::Parse($pathNode.d)).Clone()

        $transformGroup = New-Object System.Windows.Media.TransformGroup
        if ($FlipVertical) {
            $transformGroup.Children.Add(
                [System.Windows.Media.ScaleTransform]::new(1.0, -1.0, $vbWidth / 2.0, $vbHeight / 2.0)
            )
        }
        $transformGroup.Children.Add(
            [System.Windows.Media.ScaleTransform]::new($DrawWidth / $vbWidth, $DrawHeight / $vbHeight)
        )
        $transformGroup.Children.Add(
            [System.Windows.Media.TranslateTransform]::new($DrawX, $DrawY)
        )
        $geometry.Transform = $transformGroup

        $brush = Get-BrushFromSvgPath $pathNode
        $brush.Freeze()
        $dc.DrawGeometry($brush, $null, $geometry)
    }

    $dc.Close()

    $bitmap = New-Object System.Windows.Media.Imaging.RenderTargetBitmap(
        $TargetWidth,
        $TargetHeight,
        96,
        96,
        [System.Windows.Media.PixelFormats]::Pbgra32
    )
    $bitmap.Render($visual)

    $encoder = New-Object System.Windows.Media.Imaging.PngBitmapEncoder
    $encoder.Frames.Add([System.Windows.Media.Imaging.BitmapFrame]::Create($bitmap))

    $stream = [System.IO.File]::Open($OutputPath, [System.IO.FileMode]::Create)
    try {
        $encoder.Save($stream)
    } finally {
        $stream.Dispose()
    }
}

$root = Split-Path -Parent $PSScriptRoot
$assets = Join-Path $root 'examples\TapTap-v2\assets'

Export-SvgToPng -SvgPath (Join-Path $assets 'folder-icon.svg') -OutputPath (Join-Path $assets 'folder-icon.png') -TargetWidth 16 -TargetHeight 16 -DrawX 1.4 -DrawY 1.066 -DrawWidth 13.6907 -DrawHeight 13.2006
Export-SvgToPng -SvgPath (Join-Path $assets 'folder-icon.svg') -OutputPath (Join-Path $assets 'folder-icon@2x.png') -TargetWidth 32 -TargetHeight 32 -DrawX 2.8 -DrawY 2.132 -DrawWidth 27.3814 -DrawHeight 26.4012
Export-SvgToPng -SvgPath (Join-Path $assets 'select-arrow.svg') -OutputPath (Join-Path $assets 'select-arrow.png') -TargetWidth 10 -TargetHeight 10 -DrawX 1.364 -DrawY 2.142 -DrawWidth 7.2718 -DrawHeight 4.90176
Export-SvgToPng -SvgPath (Join-Path $assets 'select-arrow.svg') -OutputPath (Join-Path $assets 'select-arrow@2x.png') -TargetWidth 20 -TargetHeight 20 -DrawX 2.728 -DrawY 4.284 -DrawWidth 14.5436 -DrawHeight 9.80352
Export-SvgToPng -SvgPath (Join-Path $assets 'select-arrow.svg') -OutputPath (Join-Path $assets 'select-arrow-up.png') -TargetWidth 10 -TargetHeight 10 -DrawX 1.364 -DrawY 2.142 -DrawWidth 7.2718 -DrawHeight 4.90176 -FlipVertical
Export-SvgToPng -SvgPath (Join-Path $assets 'select-arrow.svg') -OutputPath (Join-Path $assets 'select-arrow-up@2x.png') -TargetWidth 20 -TargetHeight 20 -DrawX 2.728 -DrawY 4.284 -DrawWidth 14.5436 -DrawHeight 9.80352 -FlipVertical
