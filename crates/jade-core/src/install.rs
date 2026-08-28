//! Install channel detection and update helpers (shared by CLI + desktop).

use std::env;
use std::path::{Path, PathBuf};

#[cfg(not(windows))]
use std::process::Command;

use serde::{Deserialize, Serialize};

pub const AUR_PACKAGE_NAME: &str = "jade-desktop-bin";
pub const GITHUB_REPO: &str = "JoelYoung01/Jade";
pub const GITHUB_RELEASES_URL: &str = "https://github.com/JoelYoung01/Jade/releases/latest";
pub const LATEST_JSON_URL: &str =
    "https://github.com/JoelYoung01/Jade/releases/latest/download/latest.json";

const AUR_INFO_URL: &str = "https://aur.archlinux.org/rpc/v5/info/jade-desktop-bin";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallKind {
    Windows,
    AppImage,
    Aur,
    Deb,
    Unknown,
}

impl InstallKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::AppImage => "appImage",
            Self::Aur => "aur",
            Self::Deb => "deb",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AurPackageInfo {
    pub name: String,
    pub version: String,
    /// pkgver without Arch pkgrel suffix (e.g. `0.1.1` from `0.1.1-1`).
    pub upstream_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestReleaseInfo {
    pub version: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
}

#[derive(Deserialize)]
struct LatestJson {
    version: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    pub_date: Option<String>,
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

/// Detect how the **current process** binary was installed.
pub fn detect_install_context() -> InstallContext {
    detect_install_context_for(env::current_exe().ok().as_deref())
}

/// Detect install channel for a binary path (tests / tooling).
pub fn detect_install_context_for(exe: Option<&Path>) -> InstallContext {
    #[cfg(windows)]
    {
        let _ = exe;
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
        detect_linux_install_context(exe)
    }
}

#[cfg(not(windows))]
fn detect_linux_install_context(exe: Option<&Path>) -> InstallContext {
    let distro = read_os_release();
    let app_image_env = env::var_os("APPIMAGE").and_then(|v| v.into_string().ok());

    let (kind, package_name) = if app_image_env.is_some() {
        (InstallKind::AppImage, None)
    } else if let Some(exe) = exe {
        classify_linux_package_owned(exe)
    } else {
        (InstallKind::Unknown, None)
    };

    InstallContext {
        kind,
        platform: "linux".into(),
        distro_id: distro.id,
        arch_based: distro.arch_based,
        package_name,
        konsole_available: command_exists("konsole"),
        yay_available: command_exists("yay"),
        app_image_env,
        releases_url: GITHUB_RELEASES_URL.into(),
    }
}

#[cfg(not(windows))]
fn classify_linux_package_owned(exe: &Path) -> (InstallKind, Option<String>) {
    // Resolve symlinks so pacman/dpkg see the real packaged path.
    let resolved = std::fs::canonicalize(exe).unwrap_or_else(|_| exe.to_path_buf());
    if let Some(pkg) = pacman_owner(&resolved) {
        let kind = if pkg == AUR_PACKAGE_NAME || pkg == "jade-desktop" {
            InstallKind::Aur
        } else {
            InstallKind::Unknown
        };
        return (kind, Some(pkg));
    }
    if let Some(pkg) = dpkg_owner(&resolved) {
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

pub fn command_exists(name: &str) -> bool {
    #[cfg(windows)]
    {
        let Ok(path) = env::var("PATH") else {
            return false;
        };
        path.split(';').any(|dir| {
            let base = Path::new(dir).join(name);
            base.is_file() || base.with_extension("exe").is_file()
        })
    }
    #[cfg(not(windows))]
    {
        let Ok(path) = env::var("PATH") else {
            return false;
        };
        path.split(':')
            .any(|dir| Path::new(dir).join(name).is_file())
    }
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

/// Fetch `latest.json` from the GitHub Release.
pub fn fetch_latest_release() -> Result<LatestReleaseInfo, String> {
    let response = ureq::get(LATEST_JSON_URL)
        .call()
        .map_err(|e| format!("failed to fetch latest.json: {e}"))?;
    let text = response
        .into_body()
        .read_to_string()
        .map_err(|e| format!("failed to read latest.json: {e}"))?;
    let body: LatestJson =
        serde_json::from_str(&text).map_err(|e| format!("invalid latest.json: {e}"))?;
    Ok(LatestReleaseInfo {
        version: body.version,
        notes: body.notes,
        pub_date: body.pub_date,
    })
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

/// Best-effort AppImage asset URL for a release version.
pub fn appimage_download_url(version: &str) -> String {
    let v = version.trim_start_matches('v');
    format!(
        "https://github.com/{GITHUB_REPO}/releases/download/v{v}/Jade_{v}_amd64.AppImage"
    )
}

/// Debian package asset URL for a release version.
pub fn deb_download_url(version: &str) -> String {
    let v = version.trim_start_matches('v');
    format!("https://github.com/{GITHUB_REPO}/releases/download/v{v}/Jade_{v}_amd64.deb")
}

/// Windows NSIS installer URL for a release version.
pub fn windows_setup_download_url(version: &str) -> String {
    let v = version.trim_start_matches('v');
    format!("https://github.com/{GITHUB_REPO}/releases/download/v{v}/Jade_{v}_x64-setup.exe")
}

pub fn latest_appimage_download_url() -> String {
    GITHUB_RELEASES_URL.into()
}

/// Download a URL to a temp file; returns the path.
pub fn download_to_temp(url: &str, file_name: &str) -> Result<PathBuf, String> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("download failed ({url}): {e}"))?;
    let mut reader = response.into_body().into_reader();
    let dir = env::temp_dir().join("jade-update");
    std::fs::create_dir_all(&dir).map_err(|e| format!("temp dir: {e}"))?;
    let path = dir.join(file_name);
    let mut file = std::fs::File::create(&path).map_err(|e| format!("create {path:?}: {e}"))?;
    std::io::copy(&mut reader, &mut file).map_err(|e| format!("write {path:?}: {e}"))?;
    Ok(path)
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

    #[test]
    fn asset_urls() {
        assert!(deb_download_url("0.2.2").ends_with("Jade_0.2.2_amd64.deb"));
        assert!(appimage_download_url("v0.2.2").contains("Jade_0.2.2_amd64.AppImage"));
    }
}
