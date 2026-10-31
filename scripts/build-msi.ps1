$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

cargo build --release
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$exe = Join-Path $root 'target\release\wiggler.exe'
if (-not (Test-Path $exe)) {
    throw "Release exe was not built: $exe"
}

$meta = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
$version = ($meta.packages | Where-Object { $_.name -eq 'wiggler' }).version
if ([string]::IsNullOrWhiteSpace($version)) {
    throw 'Could not read the Wiggler version from cargo metadata.'
}

$wixproj = Join-Path $root 'wix\Wiggler.wixproj'
dotnet build $wixproj -c Release -p:ProductVersion=$version
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$msi = Get-ChildItem (Join-Path $root 'target\wix') -Filter 'Wiggler-*.msi' -Recurse |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if (-not $msi) {
    throw 'WiX finished but no MSI was found under target\wix.'
}

$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$dest = Join-Path $dist $msi.Name
Copy-Item $msi.FullName $dest -Force
Write-Host "Built $dest"
