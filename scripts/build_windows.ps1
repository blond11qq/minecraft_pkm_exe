$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

Write-Host "Building PixelmonFriendsClient.exe..."
cargo build --release

$OutDir = Join-Path $Root "dist"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$SourceExe = Join-Path $Root "target\release\pixelmon_friends_client.exe"
$TargetExe = Join-Path $OutDir "PixelmonFriendsClient.exe"

Copy-Item $SourceExe $TargetExe -Force

Write-Host "Done: $TargetExe"
Write-Host "Do not rename it with installer/setup/update/patch in the filename."
