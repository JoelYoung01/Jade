---
name: shipping-changes
description: Ship Jade changes end-to-end — clean work tree, push to main, verify CI build, bump version, publish a GitHub Release, and confirm the updater manifest works. Use when the user says "ship these changes", "ship it", "release this", "publish a new version", or asks to push and release Jade.
---

# Shipping Changes (Jade)

End-to-end release workflow for this Tauri monorepo. **Build** and **Publish** are separate GitHub Actions workflows — pushing to `main` does not create a release.

## Quick checklist

```
- [ ] 1. Work tree clean (committed + pushed)
- [ ] 2. Build workflow green on main
- [ ] 3. Version bumped, tagged, pushed
- [ ] 4. Publish workflow green (all jobs)
- [ ] 5. Release assets + latest.json verified
```

---

## Phase 1 — Clean work tree

1. Inspect state:
   ```powershell
   git status
   git diff
   git log -5 --oneline
   ```

2. If there are uncommitted changes, organize and commit them before shipping. Prefer focused commits following repo style (`feat`, `fix`, `chore`, etc.). Load the `clean-work-tree` skill if many unrelated changes need splitting.

3. Run the local quality gate when feasible:
   ```powershell
   just check
   ```

4. Push to `main` if not already up to date:
   ```powershell
   git push origin main
   ```

**Stop** if the user has not asked to commit, unless this skill was explicitly invoked (shipping implies committing is OK).

---

## Phase 2 — Verify Build on main

The **Build** workflow (`.github/workflows/build.yml`) runs on every push to `main`. It compiles the Windows NSIS installer and uploads a 30-day Actions artifact — **not** a GitHub Release.

1. Find the latest run:
   ```powershell
   gh run list --workflow=build.yml --branch=main --limit 3
   ```

2. Watch until it completes:
   ```powershell
   gh run watch <run-id> --exit-status
   ```

3. On failure, pull logs and fix before releasing:
   ```powershell
   gh run view <run-id> --log-failed
   ```

Common desktop build failures:
- **TS6133** — unused imports (strict `tsc -b`)
- **TS2322** — test helpers out of sync with `WikiPage` / other types in `apps/desktop/src/lib/types.ts`

Local repro:
```powershell
cd apps/desktop
pnpm exec tsc -b
```

---

## Phase 3 — Choose version and release

### Pick the next version

1. Read current version from `package.json` (root).
2. Find the last release tag:
   ```powershell
   git tag --sort=-v:refname | Select-Object -First 5
   ```
3. Review commits since last tag:
   ```powershell
   git log v<current>..HEAD --oneline
   ```

| Changes since last tag | Bump |
|---|---|
| Bug fixes, CI-only, small patches | patch (`0.2.0` → `0.2.1`) |
| New features, notable UI/workflow changes | minor (`0.2.0` → `0.3.0`) |

If unclear, ask the user once; default to **patch** for fix-only, **minor** for `feat` commits.

### Bump, commit, tag, push

```powershell
just bump <X.Y.Z>

git add package.json apps/desktop/package.json apps/desktop/src-tauri/Cargo.toml `
  apps/desktop/src-tauri/tauri.conf.json crates/jade-core/Cargo.toml crates/jade-cli/Cargo.toml

git commit -m "chore: release v<X.Y.Z>"

git tag v<X.Y.Z>
git push origin main
git push origin v<X.Y.Z>
```

`just bump` runs `scripts/bump-version.mjs` and syncs version across all package manifests.

---

## Phase 4 — Verify Publish workflow

The **Publish** workflow (`.github/workflows/publish.yml`) triggers on `v*` tag push. It builds:
- **Windows** — NSIS `.exe` + signature
- **Linux** — `.AppImage` + `.deb` + signatures
- **publish-updater-json** job — generates and uploads `latest.json`

1. Watch the publish run:
   ```powershell
   gh run list --workflow=publish.yml --limit 3
   gh run watch <run-id> --exit-status
   ```

2. Confirm **all three jobs** succeeded:
   - `windows-latest`
   - `ubuntu-22.04`
   - `Publish latest.json`

If any job failed, inspect logs before declaring the release good:
```powershell
gh run view <run-id> --json jobs -q ".jobs[] | {name: .name, conclusion: .conclusion}"
gh run view <run-id> --log-failed
```

---

## Phase 5 — Verify release and updater

In-app updates fetch:
```
https://github.com/JoelYoung01/Jade/releases/latest/download/latest.json
```

### Required release assets

```powershell
gh release view v<X.Y.Z> --json assets -q ".assets[].name"
```

Expect at minimum:
- `Jade_<version>_x64-setup.exe` + `.sig`
- `Jade_<version>_amd64.AppImage` + `.sig`
- `Jade_<version>_amd64.deb` + `.sig`
- **`latest.json`** ← required for in-app updates; without it users see *"Could not fetch a valid release JSON from the remote"*

### Validate latest.json

```powershell
gh release download v<X.Y.Z> --pattern "latest.json" --dir $env:TEMP\jade-release-check
Get-Content $env:TEMP\jade-release-check\latest.json | ConvertFrom-Json | Select-Object version, pub_date
```

Confirm `version` matches the tag and `platforms` includes `windows-x86_64` (and `windows-x86_64-nsis`) with non-empty `signature` and `url` fields.

---

## Recovery — missing latest.json

If Publish succeeded for platform builds but `latest.json` is absent (historically caused by `tauri-action` upload failures):

```powershell
node scripts/generate-latest-json.mjs `
  --tag v<X.Y.Z> `
  --repo JoelYoung01/Jade `
  --output latest.json

gh release upload v<X.Y.Z> latest.json --clobber
```

Requires `GH_TOKEN` or `GITHUB_TOKEN` when fetching signatures from the GitHub API (no `--assets-dir`).

---

## Post-release notes

- **Arch/AUR** — not automated. After each release, manually update `packaging/aur/jade-desktop-bin` (see that README) before yay users can update.
- **Build vs Release** — CI Build artifacts expire in 30 days; durable installs come from GitHub Releases.
- Report the release URL and version to the user when done:
  `https://github.com/JoelYoung01/Jade/releases/tag/v<X.Y.Z>`

## Summary report template

```markdown
Shipped **v<X.Y.Z>**

- Build: [run link] — success
- Publish: [run link] — success
- Release: https://github.com/JoelYoung01/Jade/releases/tag/v<X.Y.Z>
- Updater: latest.json present, version X.Y.Z
- AUR: manual PKGBUILD bump still needed (if applicable)
```
