# Pixelmon Friends

`PixelmonFriends.exe` is a Windows-only bootstrapper for configuring the official Minecraft Launcher for a private Pixelmon server.

It targets:

- Minecraft Java Edition `1.21.1`
- Pixelmon `9.3.16`
- NeoForge `21.1.200`
- Java `21` or newer
- Dedicated game directory `%APPDATA%\.minecraft-pixelmon-friends`
- Launcher installation `Pixelmon Friends`
- Server list entry `뭐해 포켓몬 모드 서버` at `34.64.32.34:25565`

## What It Does

- Checks that the official Minecraft Launcher has been opened at least once.
- Refuses to continue while Minecraft Launcher is running.
- Checks that Java 21 or newer is available for the NeoForge installer.
- Downloads the NeoForge installer from NeoForged Maven when NeoForge is not already installed.
- Opens the NeoForge GUI installer and verifies the client installation afterward.
- Finds Pixelmon on Modrinth by version, Minecraft version, and loader.
- Downloads Pixelmon to a temp file, verifies SHA-512 when Modrinth provides it, then moves it into place.
- Downloads compatible optimization mods for Minecraft `1.21.1` + NeoForge from Modrinth: Sodium, ModernFix, FerriteCore, Lithium, Entity Culling, ImmediatelyFast, and Clumps.
- Removes old bundled add-on jars from previous builds, while keeping Pixelmon and the new optimization set.
- Shows a modern GUI window with progress, current status, install logs, and a finish button.
- Creates or updates only the `pixelmon-friends` launcher profile.
- Backs up the launcher profile JSON before editing it.
- Adds or updates the Minecraft multiplayer server list entry for `뭐해 포켓몬 모드 서버` at `34.64.32.34:25565`.
- Optionally copies `assets/options.txt` if it exists and the destination file does not already exist.

## What It Does Not Do

- It does not redistribute Minecraft, Mojang assets, Pixelmon, NeoForge, or a modified client.
- It does not install Pixelmon into `%APPDATA%\.minecraft\mods`.
- It does not install optimization mods into `%APPDATA%\.minecraft\mods`.
- It does not install older convenience/world mods from previous builds such as Embeddium, JEI, Jade, Xaero maps, Mouse Tweaks, Balm, Waystones, or Lootr.
- It does not require administrator rights.
- It does not read, copy, or edit account/session files, launcher accounts, selected users, tokens, UUIDs, skins, nicknames, or Microsoft account data.
- It does not auto-install Java in this MVP.

## Configure Server Details

Edit `src/manifest.rs`:

```rust
server_name: "뭐해 포켓몬 모드 서버".to_string(),
server_address: "34.64.32.34:25565".to_string(),
```

The installer writes this server into the dedicated game directory server list at `%APPDATA%\.minecraft-pixelmon-friends\servers.dat`.

## Build Locally

Install stable Rust, then from this directory run:

```powershell
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

The release executable is:

```text
target\release\pixelmon-friends-setup.exe
```

Rename it for friends if desired:

```powershell
Copy-Item target\release\pixelmon-friends-setup.exe PixelmonFriends.exe
```

## GitHub Actions Build

The repository-root `.github/workflows/build-windows.yml` runs on pushes to `main` or `master`, pull requests, and manual dispatch. It formats, tests, runs clippy, builds on `windows-latest`, copies the release binary to `dist/PixelmonFriends.exe`, and uploads it as the `PixelmonFriends-windows` artifact.

## Release With A Tag

Create and push a semver-style tag:

```powershell
git tag v0.1.0
git push origin v0.1.0
```

The repository-root `.github/workflows/release.yml` builds on `windows-latest`, creates `dist/PixelmonFriends.exe`, zips it as `dist/PixelmonFriends-windows.zip`, and publishes both files to a GitHub Release.

You can also run the release workflow manually from GitHub Actions and provide a tag such as `v0.1.0`.

## Friend Usage

1. Install the official Minecraft Launcher.
2. Open the Launcher once, then close it fully.
3. Install Java 21 or newer.
4. Run `PixelmonFriends.exe`.
5. Follow the GUI progress. If the NeoForge installer opens, choose **Install client** and finish it.
6. When setup finishes, open Minecraft Java Edition from the official Minecraft Launcher, select **Pixelmon Friends**, and press **Play**.
7. Open Multiplayer and choose **뭐해 포켓몬 모드 서버**.

## Troubleshooting

**Minecraft Launcher is running**

Close the Launcher completely and run setup again. The setup edits only launcher profile JSON and will not do that while the Launcher may overwrite the same file.

**Java 21 is missing or too old**

Install Java 21 or newer, then run setup again. Java is only used to run the NeoForge installer in this MVP.

**NeoForge was not detected after installer exit**

Run setup again. When the NeoForge installer opens, choose **Install client**. The setup checks for `%APPDATA%\.minecraft\versions\neoforge-21.1.200\neoforge-21.1.200.json`.

**Launcher profile file is missing**

Open the official Minecraft Launcher once, close it, and run setup again.
