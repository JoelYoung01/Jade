#!/usr/bin/env bash
# Install the Jade CLI from GitHub Releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/JoelYoung01/Jade/main/scripts/install-cli.sh | bash
#   # or from a clone:
#   ./scripts/install-cli.sh
#   ./scripts/install-cli.sh --version 0.2.4
#   ./scripts/install-cli.sh --full          # Linux: install .deb (desktop + CLI) via dpkg
#   ./scripts/install-cli.sh --prefix ~/.local
#
# Default (Linux): download the amd64 .deb, extract usr/bin/jade, install to PREFIX/bin,
# and write PREFIX/share/jade/install-method so `jade update` can find this channel.

set -euo pipefail

REPO="JoelYoung01/Jade"
LATEST_JSON_URL="https://github.com/${REPO}/releases/latest/download/latest.json"
DEFAULT_PREFIX="/usr/local"
PREFIX="${JADE_PREFIX:-$DEFAULT_PREFIX}"
VERSION=""
FULL_PACKAGE=0
YES=0

usage() {
  cat <<'EOF'
Install the Jade CLI from GitHub Releases.

Usage:
  install-cli.sh [options]

Options:
  --version <X.Y.Z>   Install a specific release (default: latest.json)
  --prefix <dir>      Install prefix (default: /usr/local → …/bin/jade)
  --full              On Debian/Ubuntu-like systems, sudo dpkg -i the .deb
                      (desktop + CLI). Default is CLI-only extract.
  -y, --yes           Non-interactive (skip confirmations)
  -h, --help          Show this help

Environment:
  JADE_PREFIX         Same as --prefix
EOF
}

log() { printf '==> %s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

detect_os() {
  local uname_s
  uname_s="$(uname -s)"
  case "$uname_s" in
    Linux*)
      if grep -qiE 'microsoft|wsl' /proc/version 2>/dev/null; then
        echo "linux-wsl"
      else
        echo "linux"
      fi
      ;;
    Darwin*) echo "macos" ;;
    MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
    *) echo "unknown" ;;
  esac
}

detect_arch() {
  local uname_m
  uname_m="$(uname -m)"
  case "$uname_m" in
    x86_64|amd64) echo "amd64" ;;
    aarch64|arm64) echo "arm64" ;;
    *) echo "$uname_m" ;;
  esac
}

is_arch_like() {
  [[ -f /etc/os-release ]] || return 1
  # shellcheck disable=SC1091
  . /etc/os-release
  local id="${ID:-}"
  local like="${ID_LIKE:-}"
  [[ "$id" == "arch" || "$id" == "endeavouros" || "$id" == "manjaro" \
    || "$id" == "garuda" || "$id" == "cachyos" || "$id" == "artix" ]] \
    || [[ " $like " == *" arch "* ]]
}

is_debian_like() {
  command -v dpkg >/dev/null 2>&1
}

fetch_latest_version() {
  need_cmd curl
  local json
  json="$(curl -fsSL "$LATEST_JSON_URL")" || die "failed to fetch latest.json"
  local ver
  ver="$(printf '%s' "$json" | sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"
  [[ -n "$ver" ]] || die "could not parse version from latest.json"
  printf '%s' "$ver"
}

deb_url() {
  local ver="$1"
  printf 'https://github.com/%s/releases/download/v%s/Jade_%s_amd64.deb' "$REPO" "$ver" "$ver"
}

download_deb() {
  local ver="$1"
  local out="$2"
  local url
  url="$(deb_url "$ver")"
  log "Downloading $url"
  curl -fL --progress-bar -o "$out" "$url" || die "download failed (check version/tag match)"
}

extract_jade_from_deb() {
  local deb="$1"
  local dest_dir="$2"
  mkdir -p "$dest_dir"

  if command -v dpkg-deb >/dev/null 2>&1; then
    dpkg-deb -x "$deb" "$dest_dir"
  else
    need_cmd ar
    need_cmd tar
    local work
    work="$(mktemp -d)"
    (
      cd "$work"
      ar x "$deb"
      local data
      data="$(echo data.tar.*)"
      tar -xf "$data"
      cp -a usr "$dest_dir/"
    )
    rm -rf "$work"
  fi

  [[ -f "$dest_dir/usr/bin/jade" ]] || die "jade binary not found inside .deb"
}

install_file() {
  local src="$1"
  local dest="$2"
  mkdir -p "$(dirname "$dest")" 2>/dev/null || true
  if [[ -w "$(dirname "$dest")" ]] || [[ -w "$dest" ]]; then
    install -m 755 "$src" "$dest"
  else
    need_cmd sudo
    sudo mkdir -p "$(dirname "$dest")"
    sudo install -m 755 "$src" "$dest"
  fi
}

write_install_marker() {
  local ver="$1"
  local marker_dir="$PREFIX/share/jade"
  local marker="$marker_dir/install-method"
  local body
  body="$(printf '{\n  "channel": "cli-script",\n  "version": "%s",\n  "prefix": "%s"\n}\n' "$ver" "$PREFIX")"
  if mkdir -p "$marker_dir" 2>/dev/null && [[ -w "$marker_dir" ]]; then
    printf '%s' "$body" >"$marker"
  else
    need_cmd sudo
    sudo mkdir -p "$marker_dir"
    printf '%s' "$body" | sudo tee "$marker" >/dev/null
  fi
  log "Wrote install marker $marker (jade update uses this)"
}

confirm() {
  if [[ "$YES" -eq 1 ]]; then
    return 0
  fi
  if [[ ! -t 0 ]]; then
    die "refusing to continue without -y/--yes (stdin is not a TTY)"
  fi
  printf '%s [y/N] ' "$*"
  local ans
  read -r ans
  [[ "$ans" == "y" || "$ans" == "Y" || "$ans" == "yes" ]]
}

install_linux_cli_only() {
  local ver="$1"
  local arch
  arch="$(detect_arch)"
  [[ "$arch" == "amd64" ]] || die "only amd64 Linux packages are published today (got $arch)"

  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN

  local deb="$tmp/Jade_${ver}_amd64.deb"
  download_deb "$ver" "$deb"
  extract_jade_from_deb "$deb" "$tmp/root"

  local target="$PREFIX/bin/jade"
  log "Install $tmp/root/usr/bin/jade → $target"
  confirm "Continue?" || die "aborted"
  install_file "$tmp/root/usr/bin/jade" "$target"
  write_install_marker "$ver"
  log "Installed $($target -v 2>/dev/null || echo "jade $ver")"
  log "Try: jade help && jade update --check"
}

install_linux_full_deb() {
  local ver="$1"
  is_debian_like || die "--full requires dpkg (Debian/Ubuntu). Use default CLI-only install on Arch/WSL."

  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN

  local deb="$tmp/Jade_${ver}_amd64.deb"
  download_deb "$ver" "$deb"
  log "Will run: sudo dpkg -i $deb"
  confirm "Continue?" || die "aborted"
  need_cmd sudo
  sudo dpkg -i "$deb"
  log "Package installed. Try: jade -v && jade help"
}

maybe_suggest_aur() {
  if is_arch_like && command -v yay >/dev/null 2>&1; then
    log "Tip: when jade-desktop-bin is on the AUR, prefer: yay -S --needed jade-desktop-bin"
  fi
}

install_macos() {
  die "macOS builds are not published yet. Build from source: cargo install --path crates/jade-cli"
}

install_windows() {
  cat >&2 <<EOF
Native Windows does not ship a standalone jade CLI zip yet.
- Desktop: download Jade_*_x64-setup.exe from GitHub Releases
- CLI in WSL: run this script inside WSL (bash)
- CLI from source: cargo install --path crates/jade-cli
Releases: https://github.com/${REPO}/releases
EOF
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      [[ $# -ge 2 ]] || die "--version needs a value"
      VERSION="$2"
      shift 2
      ;;
    --prefix)
      [[ $# -ge 2 ]] || die "--prefix needs a value"
      PREFIX="$2"
      shift 2
      ;;
    --full)
      FULL_PACKAGE=1
      shift
      ;;
    -y|--yes)
      YES=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1 (try --help)"
      ;;
  esac
done

OS="$(detect_os)"
log "Detected OS: $OS ($(uname -m))"

case "$OS" in
  linux|linux-wsl)
    need_cmd curl
    if [[ -z "$VERSION" ]]; then
      log "Resolving latest version…"
      VERSION="$(fetch_latest_version)"
    fi
    VERSION="${VERSION#v}"
    log "Target version: $VERSION"
    maybe_suggest_aur
    if [[ "$FULL_PACKAGE" -eq 1 ]]; then
      install_linux_full_deb "$VERSION"
    else
      install_linux_cli_only "$VERSION"
    fi
    ;;
  macos)
    install_macos
    ;;
  windows)
    install_windows
    ;;
  *)
    die "unsupported OS: $(uname -s)"
    ;;
esac
