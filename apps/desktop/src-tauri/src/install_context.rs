//! Detect how Jade was installed and expose Linux update helpers.
//!
//! On Linux, Tauri can only self-update AppImages. Pacman/AUR installs must be
//! updated through the package manager; we launch Konsole + yay for that.

#[cfg(not(windows))]
use std::env;
#[cfg(not(windows))]
use std::path::Path;
#[cfg(not(windows))]
use std::process::Command;

use serde::{Deserialize, Serialize};

#[cfg_attr(windows, allow(dead_code))]
pub const AUR_PACKAGE_NAME: &str = "jade-desktop-bin";
const AUR_INFO_URL: &str = "https://aur.archlinux.org/rpc/v5/info/jade-desktop-bin";
const GITHUB_RELEASES_URL: &str = "https://github.com/JoelYoung01/Jade/releases/latest";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(windows, allow(dead_code))]
pub enum InstallKind {
    Windows,
    AppImage,
    Aur,
    Deb,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallContext {
    pub kind: InstallKind,
    pub platform: String,
    pub distro_id: Option<String>,
    pub arch_based: bool,
    pub package_name: Option<String>,
    pub konsole_available: bool,
    pub yay_available: bool,
    pub app_image_env: Option<String>,
    pub releases_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AurPackageInfo {
    pub name: String,
    pub version: String,
    /// pkgver without Arch pkgrel suffix (e.g. `0.1.1` from `0.1.1-1`).
    pub upstream_version: String,
}

#[derive(Deserialize)]
struct AurRpcResponse {
    results: Vec<AurRpcPackage>,
}

#[derive(Deserialize)]
struct AurRpcPackage {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Version")]
    version: String,
}

pub fn detect_install_context() -> InstallContext {
    #[cfg(windows)]
    {
        return InstallContext {
            kind: InstallKind::Windows,
            platform: "windows".into(),
            distro_id: None,
            arch_based: false,
            package_name: None,
            konsole_available: false,
            yay_available: false,
            app_image_env: None,
            releases_url: GITHUB_RELEASES_URL.into(),
        };
    }

    #[cfg(not(windows))]
    {
        detect_linux_install_context()
    }
}

#[cfg(not(windows))]
fn detect_linux_install_context() -> InstallContext {
    let distro = read_os_release();
    let arch_based = distro.arch_based;
    let app_image_env = env::var_os("APPIMAGE").and_then(|v| v.into_string().ok());

    let exe = env::current_exe().ok();
    let (kind, package_name) = if app_image_env.is_some() {
        (InstallKind::AppImage, None)
    } else if let Some(exe) = exe.as_deref() {
        classify_linux_package_owned(exe)
    } else {
        (InstallKind::Unknown, None)
    };

    InstallContext {
        kind,
        platform: "linux".into(),
        distro_id: distro.id,
        arch_based,
        package_name,
        konsole_available: command_exists("konsole"),
        yay_available: command_exists("yay"),
        app_image_env,
        releases_url: GITHUB_RELEASES_URL.into(),
    }
}

#[cfg(not(windows))]
fn classify_linux_package_owned(exe: &Path) -> (InstallKind, Option<String>) {
    if let Some(pkg) = pacman_owner(exe) {
        let kind = if pkg == AUR_PACKAGE_NAME || pkg == "jade-desktop" {
            InstallKind::Aur
        } else {
            InstallKind::Unknown
        };
        return (kind, Some(pkg));
    }
    if let Some(pkg) = dpkg_owner(exe) {
        return (InstallKind::Deb, Some(pkg));
    }
    (InstallKind::Unknown, None)
}

#[cfg(not(windows))]
fn pacman_owner(exe: &Path) -> Option<String> {
    let output = Command::new("pacman")
        .args(["-Qoq", &exe.to_string_lossy()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

#[cfg(not(windows))]
fn dpkg_owner(exe: &Path) -> Option<String> {
    let output = Command::new("dpkg-query")
        .args(["-S", &exe.to_string_lossy()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    // Format: "package: /path/to/file"
    let line = String::from_utf8_lossy(&output.stdout);
    let pkg = line.split(':').next()?.trim();
    if pkg.is_empty() {
        None
    } else {
        Some(pkg.to_string())
    }
}

#[cfg(not(windows))]
struct OsRelease {
    id: Option<String>,
    arch_based: bool,
}

#[cfg(not(windows))]
fn read_os_release() -> OsRelease {
    let Ok(raw) = std::fs::read_to_string("/etc/os-release") else {
        return OsRelease {
            id: None,
            arch_based: false,
        };
    };
    let mut id = None;
    let mut id_like = None;
    for line in raw.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            id = Some(unquote(value));
        } else if let Some(value) = line.strip_prefix("ID_LIKE=") {
            id_like = Some(unquote(value));
        }
    }
    let arch_based = id.as_deref().is_some_and(is_arch_family_id)
        || id_like
            .as_deref()
            .is_some_and(|s| s.split_whitespace().any(is_arch_family_id));
    OsRelease { id, arch_based }
}

#[cfg_attr(windows, allow(dead_code))]
fn is_arch_family_id(id: &str) -> bool {
    matches!(
        id,
        "arch" | "archlinux" | "endeavouros" | "manjaro" | "garuda" | "cachyos" | "artix"
    )
}

#[cfg(not(windows))]
fn unquote(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

#[cfg(not(windows))]
fn command_exists(name: &str) -> bool {
    let Ok(path) = env::var("PATH") else {
        return false;
    };
    path.split(':')
        .any(|dir| Path::new(dir).join(name).is_file())
}

#[cfg(windows)]
#[allow(dead_code)]
fn command_exists(_name: &str) -> bool {
    false
}

/// Fetch the published AUR package version for `jade-desktop-bin`.
pub fn fetch_aur_package_info() -> Result<Option<AurPackageInfo>, String> {
    let response = ureq::get(AUR_INFO_URL)
        .call()
        .map_err(|e| format!("AUR RPC request failed: {e}"))?;
    let text = response
        .into_body()
        .read_to_string()
        .map_err(|e| format!("failed to read AUR RPC body: {e}"))?;
    let body: AurRpcResponse =
        serde_json::from_str(&text).map_err(|e| format!("invalid AUR RPC response: {e}"))?;
    let Some(pkg) = body.results.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(AurPackageInfo {
        upstream_version: strip_pkgrel(&pkg.version),
        name: pkg.name,
        version: pkg.version,
    }))
}

pub fn strip_pkgrel(version: &str) -> String {
    match version.rsplit_once('-') {
        Some((upstream, pkgrel)) if pkgrel.chars().all(|c| c.is_ascii_digit()) => {
            upstream.to_string()
        }
        _ => version.to_string(),
    }
}

/// True when `remote` is a newer SemVer than `current` (ignores leading `v`).
#[cfg_attr(not(test), allow(dead_code))]
pub fn is_newer_version(current: &str, remote: &str) -> bool {
    match (parse_semver(current), parse_semver(remote)) {
        (Some(c), Some(r)) => r > c,
        _ => {
            let c = current.trim_start_matches('v');
            let r = remote.trim_start_matches('v');
            r != c && r > c
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let s = raw.trim().trim_start_matches('v');
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch_raw = parts.next().unwrap_or("0");
    let patch = patch_raw
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    Some((major, minor, patch))
}

/// Open Konsole and run a targeted yay update for the AUR package.
pub fn open_aur_update_in_konsole() -> Result<(), String> {
    #[cfg(windows)]
    {
        return Err("AUR updates are only available on Linux".into());
    }

    #[cfg(not(windows))]
    {
        if !command_exists("konsole") {
            return Err("Konsole is not installed".into());
        }
        if !command_exists("yay") {
            return Err("yay is not installed".into());
        }
        Command::new("konsole")
            .args(["--hold", "-e", "yay", "-S", "--needed", AUR_PACKAGE_NAME])
            .spawn()
            .map_err(|e| format!("failed to launch Konsole: {e}"))?;
        Ok(())
    }
}

/// Best-effort AppImage asset URL for the latest GitHub release.
pub fn latest_appimage_download_url() -> String {
    // Stable latest redirect; browser/GitHub UI resolves the asset list.
    // Prefer a direct asset when known; Releases page is a safe fallback.
    GITHUB_RELEASES_URL.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_pkgrel_removes_numeric_suffix() {
        assert_eq!(strip_pkgrel("0.1.1-1"), "0.1.1");
        assert_eq!(strip_pkgrel("0.2.0-3"), "0.2.0");
        assert_eq!(strip_pkgrel("0.1.1"), "0.1.1");
        assert_eq!(strip_pkgrel("1.0.0-beta.1"), "1.0.0-beta.1");
    }

    #[test]
    fn newer_version_compares_semver() {
        assert!(is_newer_version("0.1.0", "0.1.1"));
        assert!(is_newer_version("v0.1.0", "0.2.0"));
        assert!(!is_newer_version("0.1.1", "0.1.1"));
        assert!(!is_newer_version("0.2.0", "0.1.9"));
    }

    #[test]
    fn arch_family_detection() {
        assert!(is_arch_family_id("endeavouros"));
        assert!(is_arch_family_id("arch"));
        assert!(!is_arch_family_id("ubuntu"));
    }
}
