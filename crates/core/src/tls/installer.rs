use std::sync::Mutex;

use anyhow::{Context, Result, ensure};

use crate::app_paths::AppPaths;

const INSTALL_ARGUMENT: &str = "--azerothkit-install-ca";
const MAX_CERTIFICATE_SIZE: usize = 8192;

static INSTALL_LOCK: Mutex<()> = Mutex::new(());

/// Blocking operation. Run on a worker thread when called from Tauri.
///
/// Returns true after installation, false if authorization was cancelled.
pub fn install_ca() -> Result<bool> {
    let _guard = INSTALL_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("CA installation lock is poisoned"))?;

    // Initialize under the original user account, before elevation.
    let paths = AppPaths::discover()?;
    let public_file = super::ca::initialize_ca(&paths)?;

    let blocks = pem::parse_many(std::fs::read(&public_file)?)?;

    ensure!(
        blocks.len() == 1 && blocks[0].tag() == "CERTIFICATE",
        "Expected one public CA certificate",
    );

    let der = blocks[0].contents();
    validate_ca(der)?;

    // Only public certificate bytes cross the elevation boundary.
    platform::install(der)
}

/// Must run at the beginning of main(), before Tauri and its plugins.
///
/// Some(code) means this process handled an installation request
/// and must exit with that code.
pub fn handle_ca_install_request() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);

    if args.next().as_deref()
        != Some(std::ffi::OsStr::new(INSTALL_ARGUMENT))
    {
        return None;
    }

    let result = (|| -> Result<()> {
        let encoded = args
            .next()
            .context("Public CA argument is missing")?;

        ensure!(
            args.next().is_none(),
            "Unexpected CA installer arguments",
        );

        let encoded = encoded
            .to_str()
            .context("Invalid public CA argument")?;

        let der = decode_hex(encoded)?;
        validate_ca(&der)?;

        #[cfg(any(
            windows,
            target_os = "linux",
            target_os = "macos",
        ))]
        return platform::install_privileged(&der);

        #[cfg(not(any(
            windows,
            target_os = "linux",
            target_os = "macos",
        )))]
        anyhow::bail!(
            "This platform does not use the elevated installer"
        );
    })();

    Some(match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("CA installation failed: {error:#}");
            1
        }
    })
}

fn validate_ca(der: &[u8]) -> Result<()> {
    ensure!(
        !der.is_empty() && der.len() <= MAX_CERTIFICATE_SIZE,
        "Invalid CA certificate size",
    );

    let (remaining, certificate) =
        x509_parser::parse_x509_certificate(der)
            .map_err(|error| {
                anyhow::anyhow!("Invalid CA certificate: {error}")
            })?;

    ensure!(
        remaining.is_empty(),
        "Unexpected data after certificate",
    );

    ensure!(
        certificate.subject() == certificate.issuer(),
        "Expected a self-issued root CA",
    );

    ensure!(
        certificate.validity().is_valid(),
        "CA certificate is not currently valid",
    );

    let constraints = certificate
        .basic_constraints()?
        .context("CA basic constraints are missing")?;

    ensure!(constraints.value.ca, "Certificate is not a CA");

    let usage = certificate
        .key_usage()?
        .context("CA key usage is missing")?;

    ensure!(
        usage.value.key_cert_sign(),
        "CA does not permit certificate signing",
    );

    Ok(())
}

fn encode_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);

    for &byte in bytes {
        output.push(DIGITS[usize::from(byte >> 4)] as char);
        output.push(DIGITS[usize::from(byte & 15)] as char);
    }

    output
}

fn decode_hex(encoded: &str) -> Result<Vec<u8>> {
    ensure!(
        !encoded.is_empty()
            && encoded.len() <= MAX_CERTIFICATE_SIZE * 2
            && encoded.len() % 2 == 0,
        "Invalid encoded CA size",
    );

    encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)?;
            Ok(u8::from_str_radix(text, 16)?)
        })
        .collect()
}

#[cfg(target_os = "macos")]
mod platform {
    use std::process::Command;

    use anyhow::{Context, Result, ensure};
    use security_framework::{
        certificate::SecCertificate,
        os::macos::keychain::SecKeychain,
        trust_settings::{Domain, TrustSettings},
    };

    use super::{INSTALL_ARGUMENT, encode_hex};

    const AUTHORIZATION_SCRIPT: &str = r#"
on run argv
    set executablePath to item 1 of argv
    set installerArgument to item 2 of argv
    set certificateHex to item 3 of argv

    set commandText to (quoted form of executablePath) & " " & ¬
        (quoted form of installerArgument) & " " & ¬
        (quoted form of certificateHex)

    try
        do shell script commandText with administrator privileges
        return "installed"
    on error errorMessage number errorNumber
        if errorNumber is -128 then
            return "cancelled"
        end if

        error errorMessage number errorNumber
    end try
end run
"#;

    pub(super) fn install(der: &[u8]) -> Result<bool> {
        if !add_with_authorization(der)? {
            return Ok(false);
        }

        // Trust changes run in the original GUI process.
        let certificate = SecCertificate::from_der(der)
            .context("Failed to decode the public CA")?;

        match TrustSettings::new(Domain::Admin)
            .set_trust_settings_always(&certificate)
        {
            Ok(()) => Ok(true),

            Err(error)
            if matches!(error.code(), -128 | -60006) =>
                {
                    Ok(false)
                }

            Err(error) => Err(error)
                .context("Failed to authorize system CA trust"),
        }
    }

    fn add_with_authorization(der: &[u8]) -> Result<bool> {
        let executable = std::env::current_exe()
            .context("Failed to locate the application executable")?;

        // Arguments are passed separately, then shell-quoted by AppleScript.
        let output = Command::new("/usr/bin/osascript")
            .arg("-e")
            .arg(AUTHORIZATION_SCRIPT)
            .arg(executable)
            .arg(INSTALL_ARGUMENT)
            .arg(encode_hex(der))
            .output()
            .context("Failed to request administrator authorization")?;

        ensure!(
            output.status.success(),
            "System CA installation failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim(),
        );

        match String::from_utf8_lossy(&output.stdout).trim() {
            "installed" => Ok(true),
            "cancelled" => Ok(false),
            response => anyhow::bail!(
                "Unexpected CA installer response: {response}"
            ),
        }
    }

    pub(super) fn install_privileged(der: &[u8]) -> Result<()> {
        ensure!(
            unsafe { libc::geteuid() } == 0,
            "System keychain installation requires root privileges",
        );

        let certificate = SecCertificate::from_der(der)
            .context("Failed to decode the public CA")?;

        let keychain =
            SecKeychain::open("/Library/Keychains/System.keychain")
                .context("Failed to open the system keychain")?;

        match certificate.add_to_keychain(Some(keychain)) {
            Ok(()) => Ok(()),

            // errSecDuplicateItem
            Err(error) if error.code() == -25299 => Ok(()),

            Err(error) => Err(error)
                .context("Failed to add CA to the system keychain"),
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::os::windows::ffi::OsStrExt;

    use anyhow::{Context, Result, ensure};
    use windows::{
        core::{HRESULT, PCWSTR, w},
        Win32::{
            Foundation::{
                CloseHandle, ERROR_CANCELLED, WAIT_OBJECT_0,
            },
            Security::Cryptography::*,
            System::{
                Com::{
                    CoInitializeEx, CoUninitialize,
                    COINIT_APARTMENTTHREADED,
                },
                Threading::{
                    GetExitCodeProcess, WaitForSingleObject, INFINITE,
                },
            },
            UI::{
                Shell::{
                    ShellExecuteExW, SHELLEXECUTEINFOW,
                    SEE_MASK_NOCLOSEPROCESS, SEE_MASK_NOASYNC,
                },
                WindowsAndMessaging::SW_HIDE,
            },
        },
    };

    use super::{INSTALL_ARGUMENT, encode_hex};

    pub(super) fn install(der: &[u8]) -> Result<bool> {
        let encoded = encode_hex(der);

        // Use a dedicated thread with a known COM apartment.
        std::thread::spawn(move || request_elevation(&encoded))
            .join()
            .map_err(|_| {
                anyhow::anyhow!("CA authorization thread panicked")
            })?
    }

    fn request_elevation(encoded: &str) -> Result<bool> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .ok()
                .context("Failed to initialize COM")?;
        }

        struct ComGuard;

        impl Drop for ComGuard {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }

        let _com = ComGuard;

        let executable = std::env::current_exe()?;

        let executable: Vec<u16> = executable
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();

        let parameters: Vec<u16> =
            format!("{INSTALL_ARGUMENT} {encoded}")
                .encode_utf16()
                .chain(Some(0))
                .collect();

        ensure!(
            executable.len() + parameters.len() < 30_000,
            "CA installer command line is too long",
        );

        let mut request = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            lpVerb: w!("runas"),
            lpFile: PCWSTR(executable.as_ptr()),
            lpParameters: PCWSTR(parameters.as_ptr()),
            nShow: SW_HIDE.0,
            ..Default::default()
        };

        if let Err(error) = unsafe {
            ShellExecuteExW(&mut request)
        } {
            if error.code()
                == HRESULT::from_win32(ERROR_CANCELLED.0)
            {
                return Ok(false);
            }

            return Err(error)
                .context("Failed to launch the elevated CA installer");
        }

        ensure!(
            !request.hProcess.is_invalid(),
            "Windows did not return the CA installer process",
        );

        let process = request.hProcess;

        let result = (|| -> Result<()> {
            let wait = unsafe {
                WaitForSingleObject(process, INFINITE)
            };

            if wait != WAIT_OBJECT_0 {
                return Err(windows::core::Error::from_win32())
                    .context("Failed to wait for the CA installer");
            }

            let mut exit_code = 0;

            unsafe {
                GetExitCodeProcess(process, &mut exit_code)?;
            }

            ensure!(
                exit_code == 0,
                "Elevated CA installation failed: exit code {exit_code}",
            );

            Ok(())
        })();

        let close_result = unsafe { CloseHandle(process) };

        result?;
        close_result?;

        Ok(true)
    }

    pub(super) fn install_privileged(der: &[u8]) -> Result<()> {
        let store = unsafe {
            CertOpenStore(
                CERT_STORE_PROV_SYSTEM_W,
                CERT_QUERY_ENCODING_TYPE(0),
                None,
                CERT_OPEN_STORE_FLAGS(CERT_SYSTEM_STORE_LOCAL_MACHINE)
                    | CERT_STORE_OPEN_EXISTING_FLAG,
                Some(w!("ROOT").as_ptr().cast()),
            )
        }
            .context("Failed to open LocalMachine ROOT")?;

        let result = unsafe {
            CertAddEncodedCertificateToStore(
                Some(store),
                X509_ASN_ENCODING,
                der,
                CERT_STORE_ADD_USE_EXISTING,
                None,
            )
        };

        let close_result = unsafe {
            CertCloseStore(Some(store), 0)
        };

        result.context("Failed to install the system CA")?;
        close_result?;

        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::{
        fs::{self, OpenOptions},
        io::{ErrorKind, Write},
        os::unix::fs::OpenOptionsExt,
        path::Path,
        process::Command,
    };

    use anyhow::{Context, Result, ensure};
    use sha2::{Digest, Sha256};

    use super::{INSTALL_ARGUMENT, encode_hex};

    pub(super) fn install(der: &[u8]) -> Result<bool> {
        // Check support before asking the user to authorize.
        configuration()?;

        let executable = fs::canonicalize(std::env::current_exe()?)?;

        let output = Command::new("/usr/bin/pkexec")
            .arg("--disable-internal-agent")
            .arg(executable)
            .arg(INSTALL_ARGUMENT)
            .arg(encode_hex(der))
            .output()
            .context(
                "Failed to start polkit authorization; \
                 pkexec must be installed",
            )?;

        if output.status.code() == Some(126) {
            return Ok(false);
        }

        ensure!(
            output.status.success(),
            "CA installation failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim(),
        );

        Ok(true)
    }

    fn configuration()
        -> Result<(&'static str, &'static str, &'static [&'static str])>
    {
        let configurations: &[
            (&str, &str, &[&str])
        ] = &[
            (
                "/usr/local/share/ca-certificates",
                "/usr/sbin/update-ca-certificates",
                &[],
            ),
            (
                "/etc/pki/ca-trust/source/anchors",
                "/usr/bin/update-ca-trust",
                &["extract"],
            ),
        ];

        for &(directory, program, arguments) in configurations {
            if Path::new(directory).is_dir()
                && Path::new(program).is_file()
            {
                return Ok((directory, program, arguments));
            }
        }

        anyhow::bail!(
            "Automatic CA installation requires a supported \
             Debian/Ubuntu or Fedora/RHEL trust-store layout"
        )
    }

    pub(super) fn install_privileged(der: &[u8]) -> Result<()> {
        ensure!(
            unsafe { libc::geteuid() } == 0,
            "CA installation requires root authorization",
        );

        let (directory, program, arguments) = configuration()?;

        let fingerprint = encode_hex(&Sha256::digest(der));

        let destination = Path::new(directory)
            .join(format!("azerothkit-{fingerprint}.crt"));

        let certificate = pem::encode(
            &pem::Pem::new("CERTIFICATE", der.to_vec()),
        );

        let mut options = OpenOptions::new();

        options
            .write(true)
            .create_new(true)
            .mode(0o644)
            .custom_flags(libc::O_NOFOLLOW);

        match options.open(&destination) {
            Ok(mut file) => {
                let result = file
                    .write_all(certificate.as_bytes())
                    .and_then(|_| file.sync_all());

                if let Err(error) = result {
                    drop(file);
                    let _ = fs::remove_file(&destination);

                    return Err(error)
                        .context("Failed to write the system CA");
                }
            }

            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&destination)?;

                ensure!(
                    metadata.file_type().is_file(),
                    "Existing CA destination is not a regular file",
                );

                ensure!(
                    fs::read(&destination)? == certificate.as_bytes(),
                    "Existing CA file has unexpected contents",
                );
            }

            Err(error) => {
                return Err(error)
                    .context("Failed to create the system CA file");
            }
        }

        let output = Command::new(program)
            .args(arguments)
            .env_clear()
            .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
            .env("LANG", "C")
            .output()
            .context("Failed to refresh the system trust store")?;

        ensure!(
            output.status.success(),
            "System trust-store refresh failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim(),
        );

        Ok(())
    }
}

#[cfg(not(any(
    target_os = "macos",
    windows,
    target_os = "linux",
)))]
mod platform {
    pub(super) fn install(_: &[u8]) -> anyhow::Result<bool> {
        anyhow::bail!(
            "Automatic CA installation is unsupported on this platform"
        )
    }
}

/// Blocking operation. Initializes the local CA if necessary.
pub fn is_ca_installed() -> Result<bool> {
    let _guard = INSTALL_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("CA installation lock is poisoned"))?;

    let paths = AppPaths::discover()?;
    let public_file = super::ca::initialize_ca(&paths)?;

    let certificate = pem::parse(std::fs::read(&public_file)?)?;
    validate_ca(certificate.contents())?;

    let roots = rustls_native_certs::load_native_certs();

    let found = roots
        .certs
        .iter()
        .any(|root| root.as_ref() == certificate.contents());

    if found {
        return Ok(true);
    }

    ensure!(
        roots.errors.is_empty(),
        "Could not determine CA installation status: {}",
        roots
            .errors
            .iter()
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("; "),
    );

    Ok(false)
}