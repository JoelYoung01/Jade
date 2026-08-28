# Jade peer sync protocol (v1)

LAN-first task sync. Each node keeps its own SQLite database and exchanges
append-only `task_events` over HTTP. Wiki and Nostr are out of scope.

## Why not one shared DB?

Pointing WSL `--db` at `/mnt/c/.../jade.db` fails under SQLite WAL (`xShmMap` /
I/O errors on 9p). Prefer **two databases + peer sync**.

## Topology

Peers dial each other by URL (LAN or Tailscale). No public relay.

**Threat model:** the pairing token grants full read/write of task events on
the raw LAN. Prefer Tailscale (or localhost + tunnel) over open Wi‑Fi.

## CLI

```text
jade sync init [--name …]
jade sync status
jade sync pair <url> --token <secret>
jade sync now
jade sync serve [--bind 0.0.0.0:7421] [--token …]
```

While `serve` runs: periodic pull → apply → push. Desktop: menu **⋯ → Peer sync**
(off by default; when enabled, listens whenever the app is open).

### Windows ↔ WSL sketch

1. On Windows: enable Peer sync (or `jade sync serve`), note bind port and token.
2. On WSL: local DB + `jade sync pair http://<windows-host>:7421 --token …`.
3. Run `jade sync serve` or `jade sync now` on WSL; keep Windows listener up
   (desktop open or CLI serve).

WSL2 host IP can change — update the stored pair URL (`pair` again) or use a
stable Tailscale IP / mirrored networking hostname.

## Identity

- Each node has a stable `device_id` (UUID) in `sync_device`.
- New local writes stamp `task_events.origin = device_id`.
- Historical rows may still say `origin = local`.

## Cursors

- Local `task_events.seq` is **per-database** AUTOINCREMENT — never compare across peers.
- Cross-peer identity of an event is `task_events.id` (UUID).
- Each peer row stores `last_pulled_seq` for that remote device's outbound log.

## Wire format

### `GET /v1/hello`

Response:

```json
{
  "protocol_version": 1,
  "device_id": "<uuid>",
  "capabilities": ["tasks"]
}
```

Auth: `Authorization: Bearer <token>` (same shared secret both directions).

### `GET /v1/tasks/events?after_seq=N`

Returns events from **this** node with `seq > N`, oldest first:

```json
{
  "events": [
    {
      "id": "<uuid>",
      "task_id": "<uuid>",
      "event_type": "created|updated|deleted",
      "payload": { },
      "origin": "<device_id>",
      "created_at": "<rfc3339>",
      "seq": 12
    }
  ]
}
```

`seq` is the sender's local sequence (used only as a pull cursor for that peer).

### `POST /v1/tasks/events`

Body:

```json
{ "events": [ /* same event objects; seq ignored on apply */ ] }
```

Response: `{ "accepted": <count>, "skipped": <count> }`

## Apply / LWW

1. Sort batch by `(created_at, id)`.
2. Skip if `event.id` already exists locally.
3. Insert into `task_events` with remote `origin` and a **new local** `seq`.
4. If this event wins LWW for `task_id` (strictly later `created_at`, or equal time and greater `id`), upsert `tasks` / tags from `payload.task`.
5. Soft-delete when `event_type = deleted`.

## Non-goals

- Wiki sync, settings sync, conflict UI, Nostr transport, OS daemon.
