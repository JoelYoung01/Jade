//! `jade update` — detect install channel and apply updates.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context};
use jade_core::{
    appimage_download_url, cli_script_marker_path, command_exists, deb_download_url,
    detect_install_context, download_to_temp, fetch_aur_package_info, fetch_latest_release,
    is_newer_version, prefix_from_jade_exe, windows_setup_download_url, write_cli_script_marker,
    InstallKind, AUR_PACKAGE_NAME, GITHUB_RELEASES_URL,
};
use serde::Serialize;

use crate::output::print_json;

const INSTALL_SCRIPT_URL: &str =
    "https://raw.githubusercontent.com/JoelYoung01/Jade/main/scripts/install-cli.sh";

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

    if check_only {
        if json {
            print_json(&status)?;
        }
        return Ok(());
    }

    if !update_available {
        if json {
            print_json(&status)?;
        }
        // Unknown installs: still offer to adopt the CLI install script.
        if matches!(ctx.kind, InstallKind::Unknown) {
            return offer_install_script(yes, json, latest.as_deref());
        }
        return Ok(());
    }

    if json {
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
        InstallKind::CliScript => {
            let gh = fetch_latest_release().map_err(map_err)?;
            Ok((
                Some(gh.version.clone()),
                "cliScript".into(),
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
                gh.as_ref().map(|g| deb_download_url(&g.version)),
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
        InstallKind::CliScript => {
            let version = latest.context("missing latest version")?;
            let url = download_url.context("missing deb url")?;
            apply_cli_script(version, url, yes)
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
        InstallKind::Unknown => offer_install_script(yes, false, latest),
    }
}

fn confirm(prompt: &str, yes: bool) -> anyhow::Result<bool> {
    if yes {
        return Ok(true);
    }
    if !atty_stdin() {
        bail!("refusing to prompt without -y/--yes (stdin is not a TTY)");
    }
    eprint!("{prompt} [y/N] ");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn atty_stdin() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}

fn offer_install_script(yes: bool, json: bool, latest: Option<&str>) -> anyhow::Result<()> {
    let version = latest.unwrap_or("latest");
    if !json {
        eprintln!(
            "Install channel is unknown (dev build, cargo install, or manual copy).\n\
             You can install/replace the CLI using the official install script\n\
             (writes a marker so future `jade update` works):"
        );
        eprintln!("  curl -fsSL {INSTALL_SCRIPT_URL} | bash -s -- -y");
        if latest.is_some() {
            eprintln!("  # or pin: … | bash -s -- --version {version} -y");
        }
    }
    if !confirm("Run the install script now?", yes)? {
        if json {
            print_json(&serde_json::json!({
                "action": "skipped",
                "hint": INSTALL_SCRIPT_URL,
            }))?;
        } else {
            eprintln!("Skipped. You can run the script later, or keep using this binary.");
        }
        return Ok(());
    }
    run_install_script(latest, true)?;
    Ok(())
}

fn run_install_script(version: Option<&str>, yes: bool) -> anyhow::Result<()> {
    if !command_exists("curl") {
        bail!("curl is required to fetch the install script");
    }
    if !command_exists("bash") {
        bail!("bash is required to run the install script");
    }

    let mut script_args: Vec<String> = Vec::new();
    if let Some(v) = version {
        script_args.push("--version".into());
        script_args.push(v.trim_start_matches('v').to_string());
    }
    if yes {
        script_args.push("-y".into());
    }

    // Prefer inferring prefix from current binary when it looks like prefix/bin/jade.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(prefix) = prefix_from_jade_exe(&exe) {
            script_args.push("--prefix".into());
            script_args.push(prefix.display().to_string());
        }
    }

    let pipe = format!(
        "curl -fsSL {INSTALL_SCRIPT_URL} | bash -s -- {}",
        script_args
            .iter()
            .map(|a| shell_quote(a))
            .collect::<Vec<_>>()
            .join(" ")
    );
    eprintln!("Running: {pipe}");
    let status = Command::new("bash")
        .args(["-lc", &pipe])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("failed to run install script")?;
    if !status.success() {
        bail!("install script exited with {status}");
    }
    println!("Install script finished. Try: jade -v && jade update --check");
    Ok(())
}

fn shell_quote(s: &str) -> String {
    if s.is_empty() {
        return "''".into();
    }
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '~'))
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
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
    if !confirm(&format!("Download and install via dpkg?\n  {url}"), yes)? {
        bail!("aborted");
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

fn apply_cli_script(version: &str, url: &str, yes: bool) -> anyhow::Result<()> {
    let exe = std::env::current_exe().context("current_exe")?;
    let dest = std::fs::canonicalize(&exe).unwrap_or(exe);
    let prefix = prefix_from_jade_exe(&dest).unwrap_or_else(|| {
        dest.parent()
            .and_then(|p| p.parent())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("/usr/local"))
    });

    if !confirm(
        &format!(
            "Update CLI script install in place?\n  {} → {}\n  marker: {}",
            url,
            dest.display(),
            cli_script_marker_path(&prefix).display()
        ),
        yes,
    )? {
        bail!("aborted");
    }

    let file_name = format!("Jade_{version}_amd64.deb");
    eprintln!("Downloading {url} …");
    let deb = download_to_temp(url, &file_name).map_err(map_err)?;
    let extract_root = deb
        .parent()
        .map(|p| p.join("jade-cli-extract"))
        .unwrap_or_else(|| PathBuf::from("jade-cli-extract"));
    let _ = std::fs::remove_dir_all(&extract_root);
    std::fs::create_dir_all(&extract_root)?;
    extract_deb(&deb, &extract_root)?;
    let new_bin = extract_root.join("usr/bin/jade");
    if !new_bin.is_file() {
        bail!("jade binary missing from extracted .deb");
    }

    install_binary(&new_bin, &dest)?;
    write_marker_best_effort(&prefix, version)?;
    let _ = std::fs::remove_dir_all(&extract_root);
    println!(
        "CLI script install updated to {version}.\nTry: jade -v && jade update --check"
    );
    Ok(())
}

fn extract_deb(deb: &Path, dest_dir: &Path) -> anyhow::Result<()> {
    if command_exists("dpkg-deb") {
        let status = Command::new("dpkg-deb")
            .args(["-x", &deb.to_string_lossy(), &dest_dir.to_string_lossy()])
            .status()
            .context("dpkg-deb")?;
        if status.success() {
            return Ok(());
        }
        bail!("dpkg-deb failed with {status}");
    }
    // Fallback: ar + tar
    if !command_exists("ar") || !command_exists("tar") {
        bail!("need dpkg-deb or ar+tar to extract the .deb");
    }
    let work = dest_dir.join("_ar");
    std::fs::create_dir_all(&work)?;
    let status = Command::new("ar")
        .current_dir(&work)
        .args(["x", &deb.to_string_lossy()])
        .status()
        .context("ar x")?;
    if !status.success() {
        bail!("ar failed with {status}");
    }
    let data = std::fs::read_dir(&work)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("data.tar"))
        })
        .context("data.tar.* missing from .deb")?;
    let status = Command::new("tar")
        .args([
            "-xf",
            &data.to_string_lossy(),
            "-C",
            &dest_dir.to_string_lossy(),
        ])
        .status()
        .context("tar")?;
    if !status.success() {
        bail!("tar failed with {status}");
    }
    let _ = std::fs::remove_dir_all(&work);
    Ok(())
}

fn install_binary(src: &Path, dest: &Path) -> anyhow::Result<()> {
    let parent = dest.parent().context("dest has no parent")?;
    let staging = parent.join(format!(
        ".jade-update-{}",
        std::process::id()
    ));
    if parent
        .metadata()
        .map(|m| !m.permissions().readonly())
        .unwrap_or(false)
        && std::fs::copy(src, &staging).is_ok()
    {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&staging)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&staging, perms)?;
        }
        std::fs::rename(&staging, dest).with_context(|| {
            format!("replace {}", dest.display())
        })?;
        return Ok(());
    }
    // Fallback: sudo install
    let _ = std::fs::remove_file(&staging);
    if !command_exists("sudo") {
        bail!(
            "cannot write {} (permission denied) and sudo is unavailable",
            dest.display()
        );
    }
    let status = Command::new("sudo")
        .args([
            "install",
            "-m",
            "755",
            &src.to_string_lossy(),
            &dest.to_string_lossy(),
        ])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("sudo install")?;
    if !status.success() {
        bail!("sudo install failed with {status}");
    }
    Ok(())
}

fn write_marker_best_effort(prefix: &Path, version: &str) -> anyhow::Result<()> {
    match write_cli_script_marker(prefix, version) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Try via sudo tee
            if !command_exists("sudo") {
                bail!(
                    "updated binary but could not write marker at {}",
                    cli_script_marker_path(prefix).display()
                );
            }
            let marker_path = cli_script_marker_path(prefix);
            let marker_dir = marker_path.parent().context("marker parent")?;
            let status = Command::new("sudo")
                .args(["mkdir", "-p", &marker_dir.to_string_lossy()])
                .status()?;
            if !status.success() {
                bail!("sudo mkdir failed");
            }
            let body = serde_json::json!({
                "channel": "cli-script",
                "version": version.trim_start_matches('v'),
                "prefix": prefix.display().to_string(),
            });
            let mut child = Command::new("sudo")
                .args(["tee", &marker_path.to_string_lossy()])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .context("sudo tee")?;
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .context("tee stdin")?
                .write_all(format!("{body}\n").as_bytes())?;
            let status = child.wait()?;
            if !status.success() {
                bail!("sudo tee failed with {status}");
            }
            Ok(())
        }
    }
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

    if !confirm(
        &format!("Download AppImage to {}?", dest.display()),
        yes,
    )? {
        bail!("aborted");
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
