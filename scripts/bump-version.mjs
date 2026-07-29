#!/usr/bin/env node
/**
 * Sync Jade package versions across the monorepo.
 * Usage: node scripts/bump-version.mjs 0.2.0
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+(-[\w.]+)?$/.test(version)) {
  console.error("Usage: node scripts/bump-version.mjs <semver>");
  process.exit(1);
}

const root = resolve(import.meta.dirname, "..");

function replaceInFile(relPath, replacer) {
  const path = resolve(root, relPath);
  const before = readFileSync(path, "utf8");
  const after = replacer(before);
  if (after === before) {
    console.warn(`no change: ${relPath}`);
    return;
  }
  writeFileSync(path, after);
  console.log(`updated ${relPath}`);
}

function bumpPackageJson(relPath) {
  replaceInFile(relPath, (src) =>
    src.replace(/("version"\s*:\s*")[^"]+(")/, `$1${version}$2`),
  );
}

function bumpCargoToml(relPath) {
  replaceInFile(relPath, (src) => {
    // Only the package table's version line (first `version =` in file).
    let seen = false;
    return src.replace(/^version\s*=\s*"[^"]+"/m, (match) => {
      if (seen) return match;
      seen = true;
      return `version = "${version}"`;
    });
  });
}

function bumpTauriConf(relPath) {
  replaceInFile(relPath, (src) =>
    src.replace(/("version"\s*:\s*")[^"]+(")/, `$1${version}$2`),
  );
}

bumpPackageJson("package.json");
bumpPackageJson("apps/desktop/package.json");
bumpTauriConf("apps/desktop/src-tauri/tauri.conf.json");
bumpCargoToml("apps/desktop/src-tauri/Cargo.toml");
bumpCargoToml("crates/jade-core/Cargo.toml");
bumpCargoToml("crates/jade-cli/Cargo.toml");

console.log(`\nVersion set to ${version}. Next:`);
console.log(`  git commit -am "chore: release v${version}"`);
console.log(`  git tag v${version}`);
console.log(`  git push && git push origin v${version}`);
