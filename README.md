# Jade

Local-first personal software. Windows-first Tauri desktop app + shared Rust domain (`jade-core`) and optional CLI.

## Install

### Windows

1. Download the latest `*-setup.exe` from [GitHub Releases](https://github.com/JoelYoung01/Jade/releases).
2. Run the installer.

The installer is **not Authenticode-signed** yet, so Windows SmartScreen may warn on first launch — use “More info” → “Run anyway” if you trust this build.

After install, Jade checks for updates on startup (Windows only). You can also use the app menu (**⋯** → **Check for updates**).

### Arch / EndeavourOS (AUR)

Prefer the binary AUR package (recipe lives in [`packaging/aur/jade-desktop-bin`](./packaging/aur/jade-desktop-bin)):

```bash
yay -S jade-desktop-bin
# or
paru -S jade-desktop-bin
```

Updates on Arch come from your AUR helper / pacman — not from the in-app updater.

Until the package is published on the AUR, you can install a release `.deb` manually or build the PKGBUILD locally (see that folder’s README).

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
4. The **Publish** workflow builds Windows (NSIS + updater) and Linux (`.deb`), then uploads them to a GitHub Release with `latest.json` for the Windows updater.
5. For Arch: update [`packaging/aur/jade-desktop-bin`](./packaging/aur/jade-desktop-bin) and push to the AUR (manual — see that README).

CI on `main` still uploads a Windows installer Actions artifact (30-day retention) as a smoke build; durable installs should use Releases.
