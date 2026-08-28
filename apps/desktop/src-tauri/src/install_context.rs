//! Re-export install helpers from jade-core (kept as a module for desktop IPC).

pub use jade_core::{
    detect_install_context, fetch_aur_package_info, latest_appimage_download_url,
    open_aur_update_in_konsole, AurPackageInfo, InstallContext,
};
