# Jade — common agent/human commands

set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]
set dotenv-load := false

# PowerShell prefers pnpm.ps1 over pnpm.cmd; .ps1 is blocked when
# ExecutionPolicy disallows scripts. Force the .cmd shim on Windows.
pnpm := if os_family() == "windows" { "pnpm.cmd" } else { "pnpm" }

default:
    @just --list

# Full quality gate (TS + Rust)
check:
    {{pnpm}} check

# Auto-fix format/lint where safe
fix:
    {{pnpm}} fix

# Frontend Vitest + Rust tests
test:
    {{pnpm}} test
    {{pnpm}} test:rust

# Full Tauri stack (Windows WebView2)
dev:
    {{pnpm}} dev

# Vite-only UI (browser mocks when not in Tauri)
ui:
    {{pnpm}} ui

# Production Tauri build
build:
    {{pnpm}} build

# Run the jade CLI. Prefer: just cli help | just cli tasks list
# Use an extra -- only when an arg looks like a just flag: just cli -- --json tasks list
cli *args:
    cargo run -p jade-cli -- {{args}}
