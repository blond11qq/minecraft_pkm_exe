# Pixelmon Friends Client single-profile rebuild source

This source builds `PixelmonFriendsClient.exe`.

## What this version does

- Uses Minecraft `1.21.1`
- Uses NeoForge `21.1.219`
- Uses Pixelmon `9.3.16`
- Deletes the old Pixelmon Friends game folder before installing again
- Deletes old NeoForge `21.1.200` and `21.1.219` version/library cache before reinstalling
- Removes old `Pixelmon Friends` launcher profiles and the generic auto-created `NeoForge` launcher profile from both:
  - `launcher_profiles.json`
  - `launcher_profiles_microsoft_store.json`
- Creates a fresh `Pixelmon Friends` profile with `lastVersionId = neoforge-21.1.219`
- Leaves only `Pixelmon Friends` visible in the launcher; it removes the duplicate `NeoForge` entry created by the NeoForge installer
- Downloads mods into a fresh `.minecraft-pixelmon-friends\mods` folder
- Keeps the console open until the user types `q` and presses Enter
- Embeds a Windows manifest using `asInvoker`
- Avoids `installer` / `setup` in the executable name

## Build on Windows

Install Rust, then run:

```powershell
.\scripts\build_windows.ps1
```

The output file will be:

```text
dist\PixelmonFriendsClient.exe
```

Do not rename the file to anything containing `installer`, `setup`, `update`, or `patch`.

## GitHub Actions release build

This repository includes `.github/workflows/build-windows-release.yml`.

When you push to `main` or `master`, GitHub Actions will:

1. Build `PixelmonFriendsClient.exe` on `windows-latest`
2. Upload it as a workflow artifact
3. Create a release tag like `client-<run_number>-<run_attempt>`
4. Publish a GitHub Release containing `PixelmonFriendsClient.exe`

You can also create a release by pushing a tag starting with `v`, for example:

```powershell
git tag v0.3.2
git push origin v0.3.2
```

If release creation fails with a permission error, open GitHub repo settings and enable:

```text
Settings → Actions → General → Workflow permissions → Read and write permissions
```

## Manual force-clean script

If a PC is already messed up with an old `neoforge-21.1.200` profile, run:

```powershell
.\scripts\force_clean_pixelmon_friends.ps1
```

Then run `PixelmonFriendsClient.exe`.

## Expected success check

After launching Minecraft, the log must show:

```text
--version, neoforge-21.1.219
--fml.neoForgeVersion, 21.1.219
```

If it still shows `21.1.200`, the old executable was run or the launcher profile was not overwritten.
