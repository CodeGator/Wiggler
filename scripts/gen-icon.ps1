$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root 'assets\wiggler.ico'
Add-Type -AssemblyName System.Drawing

function New-OrbPng([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)
    $fill = [System.Drawing.Color]::FromArgb(255, 0x9B, 0xE1, 0x5D)
    $brush = New-Object System.Drawing.SolidBrush $fill
    $pad = [Math]::Max(1, [int][Math]::Round($size * 0.12))
    $g.FillEllipse($brush, $pad, $pad, $size - 2 * $pad, $size - 2 * $pad)
    $brush.Dispose()
    $g.Dispose()
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    $bytes = $ms.ToArray()
    $ms.Dispose()
    return ,$bytes
}

$sizes = @(16, 32, 48, 256)
$images = foreach ($size in $sizes) {
    @{ Size = $size; Bytes = New-OrbPng $size }
}

$headerSize = 6 + (16 * $images.Count)
$offset = $headerSize
$stream = New-Object System.IO.MemoryStream
$writer = New-Object System.IO.BinaryWriter $stream
$writer.Write([uint16]0)
$writer.Write([uint16]1)
$writer.Write([uint16]$images.Count)
foreach ($image in $images) {
    $dim = if ($image.Size -ge 256) { 0 } else { [byte]$image.Size }
    $writer.Write([byte]$dim)
    $writer.Write([byte]$dim)
    $writer.Write([byte]0)
    $writer.Write([byte]0)
    $writer.Write([uint16]1)
    $writer.Write([uint16]32)
    $writer.Write([uint32]$image.Bytes.Length)
    $writer.Write([uint32]$offset)
    $offset += $image.Bytes.Length
}
foreach ($image in $images) {
    $writer.Write($image.Bytes)
}
$writer.Flush()
[System.IO.File]::WriteAllBytes($dest, $stream.ToArray())
$writer.Dispose()
$stream.Dispose()
Write-Host "Wrote $dest"
