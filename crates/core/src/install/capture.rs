use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub enum CapturePlatform {
    Macos,
    Windows,
    Linux,
    Other,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct CaptureSetupStatus {
    pub platform: CapturePlatform,
    pub ready: bool,
}

pub fn get_capture_setup_status() -> Result<CaptureSetupStatus> {
    let platform = if cfg!(target_os = "macos") {
        CapturePlatform::Macos
    } else if cfg!(windows) {
        CapturePlatform::Windows
    } else if cfg!(target_os = "linux") {
        CapturePlatform::Linux
    } else {
        CapturePlatform::Other
    };

    #[cfg(target_os = "macos")]
    let ready = macos_extension_enabled()?;

    // This check currently applies only to the macOS extension.
    #[cfg(not(target_os = "macos"))]
    let ready = true;

    Ok(CaptureSetupStatus { platform, ready })
}

#[cfg(target_os = "macos")]
fn macos_extension_enabled() -> Result<bool> {
    use std::process::Command;

    use anyhow::{Context, ensure};

    const EXTENSION_ID: &str =
        "org.mitmproxy.macos-redirector.network-extension";

    let output = Command::new("/usr/bin/systemextensionsctl")
        .arg("list")
        .env("LC_ALL", "C")
        .output()
        .context("Failed to query system extensions")?;

    ensure!(
        output.status.success(),
        "System extension check failed: {}",
        String::from_utf8_lossy(&output.stderr).trim(),
    );

    let stdout = String::from_utf8(output.stdout)
        .context("Invalid system extension response")?;

    Ok(stdout.lines().any(|line| {
        let matches_extension = line
            .split_whitespace()
            .any(|part| part == EXTENSION_ID);

        matches_extension && line.contains("[activated enabled]")
    }))
}

pub fn open_capture_settings() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        // Available on macOS 13 and later.
        unsafe {
            objc2_service_management::SMAppService::
            openSystemSettingsLoginItems();
        }

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    anyhow::bail!(
        "Capture extension settings are only available on macOS"
    );
}
