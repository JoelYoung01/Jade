#!/usr/bin/env node
/**
 * Build Tauri updater manifest (latest.json) from signed release assets.
 *
 * Usage:
 *   node scripts/generate-latest-json.mjs --tag v0.2.0 --repo JoelYoung01/Jade --output latest.json
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

function parseArgs(argv) {
  const args = {};
  for (let i = 2; i < argv.length; i++) {
    const key = argv[i];
    if (!key.startsWith("--")) continue;
    const name = key.slice(2);
    const value = argv[i + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`Missing value for ${key}`);
    }
    args[name] = value;
    i++;
  }
  return args;
}

function readSig(path) {
  return readFileSync(path, "utf8").trim();
}

const { tag, repo, output, "assets-dir": assetsDirArg } = parseArgs(process.argv);
if (!tag || !repo || !output) {
  console.error(
    "Usage: node scripts/generate-latest-json.mjs --tag vX.Y.Z --repo owner/name --output latest.json [--assets-dir path]",
  );
  process.exit(1);
}

const version = tag.startsWith("v") ? tag.slice(1) : tag;
const base = `https://github.com/${repo}/releases/download/${tag}`;
const assetsDir = assetsDirArg ? resolve(assetsDirArg) : null;

const assetMap = [
  {
    keys: ["linux-x86_64", "linux-x86_64-appimage"],
    file: `Jade_${version}_amd64.AppImage`,
  },
  {
    keys: ["linux-x86_64-deb"],
    file: `Jade_${version}_amd64.deb`,
  },
  {
    keys: ["windows-x86_64", "windows-x86_64-nsis"],
    file: `Jade_${version}_x64-setup.exe`,
  },
];

const platforms = {};
for (const { keys, file } of assetMap) {
  const sigPath = assetsDir ? resolve(assetsDir, `${file}.sig`) : null;
  if (assetsDir) {
    const signature = readSig(sigPath);
    const entry = {
      signature,
      url: `${base}/${file}`,
    };
    for (const key of keys) {
      platforms[key] = entry;
    }
  }
}

let notes = "";
let pubDate = new Date().toISOString();
if (!assetsDir) {
  const token = process.env.GH_TOKEN ?? process.env.GITHUB_TOKEN;
  if (!token) {
    throw new Error("GH_TOKEN or GITHUB_TOKEN required when --assets-dir is omitted");
  }
  const res = await fetch(`https://api.github.com/repos/${repo}/releases/tags/${tag}`, {
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
    },
  });
  if (!res.ok) {
    throw new Error(`GitHub API ${res.status}: ${await res.text()}`);
  }
  const release = await res.json();
  notes = release.body ?? "";
  pubDate = release.published_at ?? release.created_at ?? pubDate;

  for (const asset of release.assets) {
    const name = asset.name;
    if (!name.endsWith(".sig")) continue;
    const file = name.slice(0, -4);
    const match = assetMap.find((item) => item.file === file);
    if (!match) continue;
    const sigRes = await fetch(asset.browser_download_url);
    if (!sigRes.ok) {
      throw new Error(`Failed to download ${name}: ${sigRes.status}`);
    }
    const signature = (await sigRes.text()).trim();
    const entry = { signature, url: `${base}/${file}` };
    for (const key of match.keys) {
      platforms[key] = entry;
    }
  }
}

const missing = assetMap.filter(({ keys }) => !keys.some((key) => platforms[key]?.signature));
if (missing.length > 0) {
  throw new Error(`Missing signed assets for: ${missing.map((m) => m.file).join(", ")}`);
}

const manifest = {
  version,
  notes,
  pub_date: pubDate,
  platforms,
};

writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Wrote ${output} for ${tag}`);
