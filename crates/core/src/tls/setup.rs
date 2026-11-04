use std::path::Path;

pub fn installation_instructions(certificate: &Path) -> String {
    #[cfg(target_os = "macos")]
    {
        let path = shell_quote(certificate);

        format!(
            "macOS:\n\
             sudo security add-trusted-cert -d -r trustRoot \
             -k /Library/Keychains/System.keychain {path}"
        )
    }

    #[cfg(windows)]
    {
        let path = certificate.to_string_lossy().replace('\'', "''");

        format!(
            "Windows PowerShell, run as administrator:\n\
         certutil -addstore Root '{path}'"
        )
    }

    #[cfg(target_os = "linux")]
    {
        let path = shell_quote(certificate);

        format!(
            "Debian/Ubuntu:\n\
             sudo install -m 0644 {path} \
             /usr/local/share/ca-certificates/azerothkit-ca.crt\n\
             sudo update-ca-certificates\n\n\
             Fedora/RHEL:\n\
             sudo install -m 0644 {path} \
             /etc/pki/ca-trust/source/anchors/azerothkit-ca.crt\n\
             sudo update-ca-trust extract"
        )
    }

    #[cfg(not(any(target_os = "macos", windows, target_os = "linux",)))]
    {
        format!(
            "Add this CA certificate to your trusted root store: {}",
            certificate.display(),
        )
    }
}

#[cfg(unix)]
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"),)
}
