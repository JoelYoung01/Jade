# Jade CLI cheat sheet

Global: `--json`, `--db <path>`, `-v` / `--version`, `jade help`, `jade <topic…> help`.

Prefer a **local** DB. Do not use `--db /mnt/c/...` to share the Windows Jade file from WSL — use `jade sync` instead.

## Update

```text
jade update --check
jade update --check --json
jade update [-y|--yes]
```

Detects AUR / `.deb` / AppImage / Windows and updates that channel when possible.

## Tasks

```text
jade tasks list --json
jade tasks add "<title>" [--due tomorrow|next-monday|RFC3339|…] [-t tag] [--repeat "0 9 * * 1-5"] [--json]
jade tasks update --id <uuid> --status inactive|active|complete [--json]
jade tasks update --id <uuid> --title "…" | --due … | -d "…" [--json]
jade tasks delete --id <uuid> [--json]
jade tasks history [--id <uuid>] [--limit 50] [--json]
```

New tasks start as `inactive`. Delete soft-deletes (tombstone).

## Peer sync

```text
jade sync init [--name "laptop"]
jade sync status [--json]
jade sync pair http://192.168.1.10:7421 --token <secret>
jade sync now [--json]
jade sync serve [--bind 0.0.0.0:7421] [--token <secret>]
```

LAN or Tailscale. Desktop: **⋯ → Peer sync**. See `docs/sync.md`.

## Wiki

```text
jade wiki roots [--format json]
jade wiki roots add <path> [--label name]
jade wiki roots remove --id <uuid>
jade wiki list [--root …]
jade wiki search <query>
```

## Dev shortcuts (this repo)

```text
just cli help
just cli tasks list --json
cargo run -p jade-cli -- tasks list --json
```
