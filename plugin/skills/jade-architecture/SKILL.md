---
name: jade-architecture
description: >-
  Jade local-first architecture, sync-ready schema, concurrency, and failure
  modes. Use when diagnosing Jade errors, sync/peer questions, DB conflicts,
  wiki indexing, or when tempted to treat Jade like a remote server API.
---

# Jade architecture

Jade is **local-first**, not client–server. Desktop UI and CLI open the same SQLite file. A future design is multi-peer sync; today it is a **single-node MVP** with a sync-shaped schema.

## Mental model

```text
Desktop (Tauri) ──┐
                  ├── jade-core ── SQLite WAL (jade.db)
CLI (jade) ───────┘
Wiki markdown on disk ── indexed into SQLite (files remain source of truth)
```

- **No remote Jade API.** Missing binary, wrong `--db`, SQLite lock/corruption, invalid cron, or unknown task id are local failures — not “API 503 / server down”.
- **One DB file** for GUI + CLI (unless `--db` overrides). Concurrent writers use SQLite WAL.
- **Event log** for task (and wiki) mutations: monotonic `seq` (replication cursor), `origin` (default `local`; future peers/agents use distinct origins), payload snapshots for apply/UI.
- **Soft deletes** tombstone rows (`deleted_at`) so sync can propagate deletions later.

## Sync today vs later

| Today | Later (planned) |
| --- | --- |
| Single node; `origin` defaults to `local` | Peers/agents write with their own `origin` |
| `seq` + event history ready for cursors | Multi-device / P2P reconciliation |
| Optional **Syncthing** for wiki folders on disk | Syncthing is still file sync — not Jade’s protocol |

Do not invent a central Jade server or assume another machine’s DB is reachable over HTTP. If data “isn’t there”, check **which `--db` path** and whether the wiki root path is indexed.

## Concurrency

- GUI and CLI (and future peers) may open the DB at once.
- Prefer short CLI commands; avoid long-held locks or bulk scripts that hold a write transaction open.
- The desktop UI notices changes via SQLite `data_version` / event polling — CLI writes should appear without needing a “restart server”.

## Wiki specifics

- Files + YAML front matter on disk are authoritative.
- SQLite stores roots, page locations, and FTS/search cache.
- Syncthing (if present) syncs files; Jade may show Syncthing status for roots — it is optional and not required for Jade to work.

## Failure modes (agent checklist)

| Symptom | Likely cause | What to do |
| --- | --- | --- |
| `jade` not found | CLI not on PATH / not built | Use `just cli` in this repo, or build `jade-cli` |
| Empty / unexpected data | Wrong database | Confirm default path vs `--db` |
| `task not found` | Bad id or already deleted | `jade tasks list --json` / history |
| Invalid cron | Bad `--repeat` | See `jade tasks add help` (5-field POSIX) |
| Wiki search empty | Root not added / files outside root | `jade wiki roots`, then re-check paths |
| “Server down” intuition | Wrong model | Treat as local process + local files only |

For CLI flag details: `jade help` and topic help (`jade tasks delete help`, etc.).
