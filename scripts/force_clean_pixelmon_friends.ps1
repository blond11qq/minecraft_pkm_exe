$ErrorActionPreference = "Stop"

Write-Host "Closing Minecraft Launcher / Java..."
taskkill /IM MinecraftLauncher.exe /F 2>$null
TASKKILL /IM Minecraft.exe /F 2>$null
taskkill /IM javaw.exe /F 2>$null
taskkill /IM java.exe /F 2>$null

$mc = Join-Path $env:APPDATA ".minecraft"
$game = Join-Path $env:APPDATA ".minecraft-pixelmon-friends"
$cache = Join-Path $env:APPDATA ".pixelmon-friends-client-cache"

$targets = @(
  $game,
  $cache,
  (Join-Path $mc "versions\neoforge-21.1.200"),
  (Join-Path $mc "versions\neoforge-21.1.219"),
  (Join-Path $mc "libraries\net\neoforged\neoforge\21.1.200"),
  (Join-Path $mc "libraries\net\neoforged\neoforge\21.1.219")
)

foreach ($target in $targets) {
  if (Test-Path $target) {
    Write-Host "Deleting $target"
    Remove-Item $target -Recurse -Force
  }
}

$profileFiles = @(
  (Join-Path $mc "launcher_profiles.json"),
  (Join-Path $mc "launcher_profiles_microsoft_store.json")
)

foreach ($file in $profileFiles) {
  if (Test-Path $file) {
    Copy-Item $file "$file.bak-$(Get-Date -Format yyyyMMdd-HHmmss)" -Force
    $json = Get-Content $file -Raw | ConvertFrom-Json
    if (-not $json.profiles) { $json | Add-Member -NotePropertyName profiles -NotePropertyValue ([pscustomobject]@{}) }

    $remove = @()
    foreach ($p in $json.profiles.PSObject.Properties) {
      $name = [string]$p.Value.name
      $gameDir = [string]$p.Value.gameDir
      $lastVersionId = [string]$p.Value.lastVersionId
      $isPixelmonFriends = $p.Name -eq "pixelmon-friends" -or $p.Name -eq "pixelmon_friends" -or $name -eq "Pixelmon Friends" -or $gameDir.Contains(".minecraft-pixelmon-friends")
      $isManagedNeoForgeProfile = $name -eq "NeoForge" -and ($lastVersionId -eq "neoforge-21.1.200" -or $lastVersionId -eq "neoforge-21.1.219")
      if ($isPixelmonFriends -or $isManagedNeoForgeProfile) {
        $remove += $p.Name
      }
    }

    foreach ($key in $remove) {
      $json.profiles.PSObject.Properties.Remove($key)
    }

    $json | ConvertTo-Json -Depth 100 | Set-Content $file -Encoding UTF8
  }
}

Write-Host "Clean complete. Now run PixelmonFriendsClient.exe."
