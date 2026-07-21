# Jade

Local-first personal software. Windows-first Tauri desktop app + shared Rust domain (`jade-core`) and optional CLI.

## Setup (Windows)

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
| CLI | `just cli help` / `just cli tasks list` |

VS Code / Cursor: **Terminal → Run Task…** — tasks call the same `just` recipes.

First `just dev` compiles Rust and opens the Jade window. Hot reload covers the React UI; Rust/Tauri changes rebuild the native side.
