# Jade CLI cheat sheet

Global: `--json`, `--db <path>`, `jade help`, `jade <topic…> help`.

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
