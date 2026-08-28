# Jade

Local-first personal software. Windows-first Tauri desktop app + shared Rust domain (`jade-core`) and optional CLI.

## Install

### Windows

1. Download the latest `*-setup.exe` from [GitHub Releases](https://github.com/JoelYoung01/Jade/releases).
2. Run the installer.

The installer is **not Authenticode-signed** yet, so Windows SmartScreen may warn on first launch — use “More info” → “Run anyway” if you trust this build.

After install, Jade checks for updates on startup. You can also use the app menu (**⋯** → **Check for updates**).

### Linux AppImage

1. Download the `.AppImage` from [GitHub Releases](https://github.com/JoelYoung01/Jade/releases).
2. `chmod +x Jade_*.AppImage` and run it.

While Jade is running as an AppImage, **Check for updates** can download and replace that AppImage in place (signed updater artifacts). The AppImage also embeds the `jade` CLI inside its filesystem (not installed onto the host `PATH` unless you use the `.deb` / AUR package).

### Arch / EndeavourOS (AUR)

Prefer the binary AUR package (recipe lives in [`packaging/aur/jade-desktop-bin`](./packaging/aur/jade-desktop-bin)):

```bash
yay -S jade-desktop-bin
```

That install includes the desktop app **and** the `jade` CLI on `PATH`. In Arch WSL you can use the same package for the CLI and pass `--db` to share a Windows Jade database if needed.

In-app **Check for updates** detects an AUR install and can open Konsole with a targeted `yay -S --needed jade-desktop-bin` command. Jade will not overwrite pacman-owned files itself.

Until `jade-desktop-bin` is published on the AUR, use the AppImage (or build the PKGBUILD locally — see that folder’s README).

### Debian / Ubuntu (`.deb`)

A `.deb` is published on each release for manual install (desktop + `jade` CLI). Updates are not applied in-app for `.deb` installs (use your package manager or switch to the AppImage).

## Setup (Windows development)

1. **Node.js** 22+ and **pnpm via Corepack** (do not `npm i -g pnpm`)  
   Node 25+ no longer ships Corepack, so install it once, then enable pnpm:
   ```powershell
   npm install -g corepack@latest
   corepack enable
   corepack prepare pnpm@10.28.2 --activate
   ```
2. **Rust** stable via `rustup` (MSVC toolchain)
3. **WebView2** (usually already on Windows 10/11)
4. **MSVC Build Tools** / VS with C++ workload
5. **just** — `cargo install just`  
   Optional: `cargo install cargo-deny` (used by `just check`)

> Edit in WSL if you like; run the desktop app on **native Windows** (WebView2).

```powershell
pnpm install
```

For local production builds with updater artifacts enabled, set:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY_PATH = "$env:USERPROFILE\.tauri\jade.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
```

(Private key is never committed; CI uses GitHub Actions secrets.)

## Agent plugin

Agents can install a portable plugin that teaches the `jade` CLI (tasks + wiki) and Jade’s local-first architecture. Package lives in [`plugin/`](./plugin/) — see [`plugin/README.md`](./plugin/README.md) for Cursor, Claude Code, and Agent Plugins 1.0 install notes.

## Local development

Commands live in the [`justfile`](./justfile) — use that as the source of truth (`just --list`).

| Goal | Command |
|---|---|
| Desktop app (Tauri + Vite) | `just dev` |
| UI only (browser, mocked IPC) | `just ui` |
| Quality gate | `just check` |
| Tests | `just test` |
| Auto-fix | `just fix` |
| Production build | `just build` |
| Bump release version | `just bump 0.2.0` |
| CLI | `just cli help` / `just cli tasks list` |

VS Code / Cursor: **Terminal → Run Task…** — tasks call the same `just` recipes.

First `just dev` compiles Rust and opens the Jade window. Hot reload covers the React UI; Rust/Tauri changes rebuild the native side.

## Releasing

1. Bump versions: `just bump 0.x.y`
2. Commit the bump.
3. Tag and push:
   ```powershell
   git tag v0.x.y
   git push origin main
   git push origin v0.x.y
   ```
4. The **Publish** workflow builds Windows (NSIS + updater), Linux (`.deb` + `.AppImage` + updater), and uploads them to a GitHub Release with `latest.json`.
5. For Arch: update [`packaging/aur/jade-desktop-bin`](./packaging/aur/jade-desktop-bin) and push to the AUR (manual — see that README) before AUR users can update via yay.

CI on `main` still uploads a Windows installer Actions artifact (30-day retention) as a smoke build; durable installs should use Releases.
