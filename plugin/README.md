# Jade agent plugin

Portable agent plugin for **using** Jade (tasks + wiki) via the `jade` CLI. Skills-only for now — no MCP.

This package follows [Agent Plugins 1.0](https://agent-plugins.org/specification) (`plugin.json` + `skills/`), with thin Cursor and Claude Code manifests so the same skills load in those harnesses.

## Skills

| Skill | When it loads |
| --- | --- |
| `jade` | Manage Jade tasks/wiki, or persist work into Jade instead of ad-hoc notes |
| `jade-architecture` | Local-first / sync-ready mental model, concurrency, failure modes |

## Prerequisite: CLI

Agents shell out to `jade … --json`. The desktop app and CLI share one SQLite file; the GUI does not need to be running.

| How you got Jade | How to run the CLI |
| --- | --- |
| Arch / EndeavourOS AUR (`jade-desktop-bin`) | `jade` on `PATH` |
| Linux `.deb` from Releases | `jade` on `PATH` |
| Developing this repo | `just cli <args>` or `cargo run -p jade-cli -- <args>` |
| Windows desktop installer | CLI not bundled yet — build from source |

Default database: `app.jade.desktop/jade.db` under the OS user data directory. Override with `--db <path>` (e.g. from WSL to a Windows install: `/mnt/c/Users/<you>/AppData/Roaming/app.jade.desktop/jade.db`).

## Install

### Cursor

This repo’s root [`.cursor-plugin/marketplace.json`](../.cursor-plugin/marketplace.json) lists this plugin (`source: "plugin"`).

- **This workspace:** enable the `jade` plugin from Cursor’s plugin / marketplace UI for the repo, or point Cursor at the `plugin/` directory as an Agent Plugin / Cursor Plugin.
- **Marketplace submit (later):** [cursor.com/marketplace/publish](https://cursor.com/marketplace/publish) with this package (or a dedicated public face).

### Claude Code

```bash
claude --plugin-dir /path/to/Jade/plugin
```

Or add this repo (or the `plugin/` directory) as a local marketplace / plugin per [Claude Code plugins](https://code.claude.com/docs/en/plugins).

### Any Agent Plugins client

Point the client at the `plugin/` directory (the folder that contains `plugin.json`). Compatible clients discover `skills/` from that root.

## Maintainer note

In-repo skill [`.agents/skills/shipping-changes`](../.agents/skills/shipping-changes/SKILL.md) is for **shipping** Jade releases. It is not part of this consumer plugin.
