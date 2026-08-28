//! `jade update` — detect install channel and apply updates.

use std::process::{Command, Stdio};

use anyhow::{bail, Context};
use jade_core::{
    appimage_download_url, command_exists, deb_download_url, detect_install_context,
    download_to_temp, fetch_aur_package_info, fetch_latest_release, is_newer_version,
    windows_setup_download_url, InstallKind, AUR_PACKAGE_NAME, GITHUB_RELEASES_URL,
};
use serde::Serialize;

use crate::output::print_json;

#[derive(Debug, Clone, Serialize)]
struct UpdateStatus {
    current_version: String,
    latest_version: Option<String>,
    install_kind: String,
    package_name: Option<String>,
    update_available: bool,
    channel: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    download_url: Option<String>,
}

pub fn run(check_only: bool, yes: bool, json: bool) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let ctx = detect_install_context();

    let (latest, channel, download_url) = resolve_channel(&ctx.kind)?;
    let update_available = latest
        .as_ref()
        .is_some_and(|v| is_newer_version(&current, v));

    let message = if !update_available {
        match &latest {
            Some(v) => format!("Already up to date ({current}; channel latest {v})."),
            None => format!("Already up to date ({current}); could not resolve channel latest."),
        }
    } else {
        let v = latest.as_deref().unwrap_or("?");
        format!("Update available: {current} → {v} via {channel}.")
    };

    let status = UpdateStatus {
        current_version: current.clone(),
        latest_version: latest.clone(),
        install_kind: ctx.kind.as_str().to_string(),
        package_name: ctx.package_name.clone(),
        update_available,
        channel: channel.clone(),
        message: message.clone(),
        download_url: download_url.clone(),
    };

    if json && (check_only || !update_available) {
        print_json(&status)?;
        return Ok(());
    }

    if !json {
        println!("install: {}", ctx.kind.as_str());
        if let Some(pkg) = &ctx.package_name {
            println!("package: {pkg}");
        }
        println!("current: {current}");
        if let Some(v) = &latest {
            println!("latest:  {v}");
        }
        println!("{message}");
    }

    if check_only || !update_available {
        if json {
            print_json(&status)?;
        }
        return Ok(());
    }

    if json {
        // Still print plan before installing so agents can see intent.
        print_json(&serde_json::json!({
            "status": status,
            "installing": true,
        }))?;
    }

    apply_update(&ctx.kind, latest.as_deref(), download_url.as_deref(), yes)?;
    Ok(())
}

fn map_err(err: String) -> anyhow::Error {
    anyhow::anyhow!(err)
}

fn resolve_channel(
    kind: &InstallKind,
) -> anyhow::Result<(Option<String>, String, Option<String>)> {
    match kind {
        InstallKind::Aur => {
            let aur = fetch_aur_package_info().map_err(map_err)?;
            if let Some(info) = aur {
                return Ok((
                    Some(info.upstream_version),
                    format!("aur:{AUR_PACKAGE_NAME}"),
                    None,
                ));
            }
            // AUR package missing — fall back to GitHub release version.
            let gh = fetch_latest_release().map_err(map_err)?;
            Ok((
                Some(gh.version.clone()),
                "github".into(),
                Some(deb_download_url(&gh.version)),
            ))
        }
        InstallKind::Deb => {
            let gh = fetch_latest_release().map_err(map_err)?;
            Ok((
                Some(gh.version.clone()),
                "deb".into(),
                Some(deb_download_url(&gh.version)),
            ))
        }
        InstallKind::AppImage => {
            let gh = fetch_latest_release().map_err(map_err)?;
            Ok((
                Some(gh.version.clone()),
                "appImage".into(),
                Some(appimage_download_url(&gh.version)),
            ))
        }
        InstallKind::Windows => {
            let gh = fetch_latest_release().map_err(map_err)?;
            Ok((
                Some(gh.version.clone()),
                "windows".into(),
                Some(windows_setup_download_url(&gh.version)),
            ))
        }
        InstallKind::Unknown => {
            let gh = fetch_latest_release().ok();
            Ok((
                gh.as_ref().map(|g| g.version.clone()),
                "unknown".into(),
                None,
            ))
        }
    }
}

fn apply_update(
    kind: &InstallKind,
    latest: Option<&str>,
    download_url: Option<&str>,
    yes: bool,
) -> anyhow::Result<()> {
    match kind {
        InstallKind::Aur => apply_aur(yes),
        InstallKind::Deb => {
            let version = latest.context("missing latest version")?;
            let url = download_url.context("missing deb url")?;
            apply_deb(version, url, yes)
        }
        InstallKind::AppImage => {
            let version = latest.context("missing latest version")?;
            let url = download_url.context("missing AppImage url")?;
            apply_appimage(version, url, yes)
        }
        InstallKind::Windows => {
            bail!(
                "Windows does not ship the `jade` CLI in the installer yet.\n\
                 Update the desktop app from ⋯ → Check for updates, or download:\n  {}\n\
                 Build the CLI from source with `cargo install --path crates/jade-cli`.",
                download_url.unwrap_or(GITHUB_RELEASES_URL)
            );
        }
        InstallKind::Unknown => {
            bail!(
                "Could not detect a supported install channel for this `jade` binary.\n\
                 Supported: AUR (`jade-desktop-bin`), Debian `.deb`, or AppImage.\n\
                 Releases: {GITHUB_RELEASES_URL}"
            );
        }
    }
}

fn apply_aur(yes: bool) -> anyhow::Result<()> {
    if !command_exists("yay") {
        bail!(
            "yay is not installed. Install updates with:\n  yay -S --needed {AUR_PACKAGE_NAME}"
        );
    }
    let mut cmd = Command::new("yay");
    cmd.args(["-S", "--needed", AUR_PACKAGE_NAME]);
    if yes {
        cmd.arg("--noconfirm");
    }
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = cmd.status().context("failed to run yay")?;
    if !status.success() {
        bail!("yay exited with {status}");
    }
    println!("AUR package update finished.");
    Ok(())
}

fn apply_deb(version: &str, url: &str, yes: bool) -> anyhow::Result<()> {
    if !yes {
        eprintln!("Will download and install:\n  {url}");
        eprintln!("Continue? [y/N]");
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            bail!("aborted");
        }
    }

    let file_name = format!("Jade_{version}_amd64.deb");
    eprintln!("Downloading {url} …");
    let path = download_to_temp(url, &file_name).map_err(map_err)?;
    eprintln!("Installing {} (needs sudo) …", path.display());

    let status = Command::new("sudo")
        .args(["dpkg", "-i", &path.to_string_lossy()])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("failed to run sudo dpkg")?;
    if !status.success() {
        bail!("dpkg exited with {status}");
    }
    println!("Debian package update finished.");
    Ok(())
}

fn apply_appimage(version: &str, url: &str, yes: bool) -> anyhow::Result<()> {
    let Some(current) = std::env::var_os("APPIMAGE") else {
        bail!(
            "AppImage detected but $APPIMAGE is unset. Download manually:\n  {url}\n\
             Or use Check for updates in the Jade desktop app."
        );
    };
    let current_path = std::path::PathBuf::from(current);
    let dest = current_path
        .parent()
        .map(|p| p.join(format!("Jade_{version}_amd64.AppImage")))
        .unwrap_or_else(|| std::path::PathBuf::from(format!("Jade_{version}_amd64.AppImage")));

    if !yes {
        eprintln!("Will download AppImage to:\n  {}", dest.display());
        eprintln!("Continue? [y/N]");
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            bail!("aborted");
        }
    }

    eprintln!("Downloading {url} …");
    let tmp =
        download_to_temp(url, &format!("Jade_{version}_amd64.AppImage")).map_err(map_err)?;
    std::fs::copy(&tmp, &dest).with_context(|| format!("copy to {}", dest.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)?;
    }
    println!(
        "Downloaded new AppImage to {}.\nQuit the running app and launch that file to finish updating.",
        dest.display()
    );
    Ok(())
}
