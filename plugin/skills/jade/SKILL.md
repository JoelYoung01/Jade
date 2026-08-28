---
name: jade
description: >-
  Use Jade — local-first personal tasks and wiki — via the jade CLI.
  Use when the user asks to manage Jade tasks or wiki, list/add/update/delete
  tasks, search notes, or persist work into Jade instead of ad-hoc files.
---

# Jade

Local-first personal toolkit: **tasks** (kanban-style board) and a **wiki** (indexed markdown folders). Desktop app and CLI share one SQLite database. There is no Jade HTTP server.

Prefer the architecture skill (`jade-architecture`) when diagnosing sync, concurrency, or “is the server down?” style failures.

## When to use

- User wants to create, list, update, complete, or delete Jade tasks
- User wants wiki roots, page list, or search
- Agent should store durable personal work in Jade rather than scratch notes

## Prerequisite

`jade` must be runnable:

| Context | Command |
| --- | --- |
| This repo (dev) | `just cli <args>` or `cargo run -p jade-cli -- <args>` |
| Linux desktop / AUR / `.deb` install | `jade <args>` (`/usr/bin/jade` from the package) |
| Binary on PATH (other) | `jade <args>` |

Linux GitHub Releases (`.deb` / AppImage) and AUR `jade-desktop-bin` ship the CLI alongside the desktop app. The Windows installer does not yet include `jade`; use a local build there (`just cli` / `cargo run -p jade-cli`).

## Agent conventions

1. Prefer **`--json`** (or `--format json` where supported) for machine-readable output.
2. Pass **`--db <path>`** only when the user specifies a non-default database.
3. Default DB is the same file as the GUI: `app.jade.desktop/jade.db` under the OS user data directory. The GUI does not need to be running.
4. For flag details, run `jade help` or `jade <topic> help` — do not invent flags.

See also [references/cli-cheatsheet.md](../../references/cli-cheatsheet.md).

## Surface map

### Tasks

| Goal | Command |
| --- | --- |
| List | `jade tasks list --json` |
| Add | `jade tasks add "<title>" [--due …] [-t tag] [--repeat cron] [--json]` |
| Update | `jade tasks update --id <uuid> [--title\|--status\|--due\|--description\|-d …] [--json]` |
| Soft-delete | `jade tasks delete --id <uuid> [--json]` |
| Event history | `jade tasks history [--id <uuid>] [--limit n] [--json]` |

Notes:

- New tasks always start as **`inactive`**.
- Status values: `inactive` | `active` | `complete` (see `jade tasks update status help`).
- Delete is a **tombstone** (`deleted_at`); the row remains for future sync.

### Wiki

| Goal | Command |
| --- | --- |
| List roots | `jade wiki roots --json` |
| Add root | `jade wiki roots add <path> [--label name]` |
| Remove root | `jade wiki roots remove --id <uuid>` |
| List pages | `jade wiki list [--root …]` |
| Search | `jade wiki search <query>` |

Markdown files on disk are the source of truth; SQLite holds location metadata and search index.

## Quick examples

```text
jade tasks list --json
jade tasks add "Buy milk" --due tomorrow --tag errands --json
jade tasks update --id <uuid> --status active --json
jade wiki search recipes
```
