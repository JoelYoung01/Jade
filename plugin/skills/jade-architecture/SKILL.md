---
name: jade-architecture
description: >-
  Jade local-first architecture, LAN peer sync, concurrency, and failure
  modes. Use when diagnosing Jade errors, sync/peer questions, DB conflicts,
  wiki indexing, or when tempted to treat Jade like a remote server API.
---

# Jade architecture

Jade is **local-first**, not a hosted SaaS. Desktop UI and CLI each open a **local** SQLite database. Machines sync **tasks** over LAN/Tailscale HTTP peer sync — not by sharing one DB file.

## Mental model

```text
Windows ── jade.db ── jade sync serve / desktop listener ──┐
                                                          ├── HTTP pull/push (tasks)
WSL     ── jade.db ── jade sync serve / now ──────────────┘

Wiki markdown on disk ── Syncthing (optional) ── indexed into each local SQLite
```

- **No central Jade cloud.** Pairing token + peer URL = access to that machine’s task events.
- **One DB per machine** (GUI + CLI on that OS share the default path unless `--db` overrides). Concurrent local writers use SQLite WAL.
- **Do not** point WSL `--db` at `/mnt/c/.../jade.db`. WAL over 9p/`/mnt/c` fails; use **peer sync** instead.
- **Event log** for task mutations: per-DB `seq` (outbound cursor only), `origin` = device id for new writes, payload snapshots for LWW apply.
- **Soft deletes** tombstone rows (`deleted_at`) so sync can propagate deletions.

## Peer sync (tasks)

| Piece | Role |
| --- | --- |
| `jade sync init` / first open | Stable `device_id` |
| `jade sync serve` or desktop **Peer sync** enabled | Listen + periodic pull/push while running |
| `jade sync pair <url> --token …` | Store peer + hello |
| `jade sync now` | One-shot sync all peers |
| Conflicts | Last-writer-wins by event `created_at` + `event.id` |

Protocol notes: [`docs/sync.md`](../../../docs/sync.md). Wiki content is **not** in this protocol — use Syncthing (or similar) for markdown folders.

Desktop: sync is **off by default**. Once enabled, the in-process listener runs whenever the app is open; quitting stops it unless `jade sync serve` is also running.

## Concurrency

- GUI and CLI on the **same** machine may open the same DB at once.
- Prefer short CLI commands; avoid long-held write transactions.
- The desktop UI notices changes via SQLite `data_version` — remote apply updates the board without a restart.

## Wiki specifics

- Files + YAML front matter on disk are authoritative.
- SQLite stores roots, page locations, and FTS/search cache.
- Syncthing (if present) syncs files; Jade may show Syncthing status for roots — optional.

## Failure modes (agent checklist)

| Symptom | Likely cause | What to do |
| --- | --- | --- |
| `jade` not found | CLI not on PATH / not built | Use `just cli` in this repo, or build `jade-cli` |
| Empty / unexpected data | Wrong database | Confirm default path vs `--db` (local path only) |
| WAL / `xShmMap` on `/mnt/c` | Shared Windows DB from WSL | Use local DB + `jade sync` |
| Peers not converging | Listener down / bad URL / token | `jade sync status`; enable desktop sync or `serve` |
| `task not found` | Bad id or already deleted | `jade tasks list --json` / history |
| Invalid cron | Bad `--repeat` | See `jade tasks add help` (5-field POSIX) |
| Wiki search empty | Root not added / files outside root | `jade wiki roots`, then re-check paths |
| “Server down” intuition | Wrong model | Local process + optional peer HTTP — not a hosted API |

For CLI flag details: `jade help` and topic help (`jade sync help`, `jade tasks delete help`, etc.).
