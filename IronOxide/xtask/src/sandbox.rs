//! Network denial for build scripts and proc-macros (R58.4).
//!
//! `.cargo/config.toml` sets `net.offline = true` and every Cargo command runs
//! with `--frozen`, so Cargo itself does not use the network. Build scripts
//! and proc-macros are arbitrary code, though, so the verification job runs
//! every Cargo command that compiles or runs code inside an operating-system
//! sandbox that denies network access to the whole process tree:
//!
//! | Platform | Mechanism | Enforced | Not enforced or not verified |
//! |---|---|---|---|
//! | macOS | `sandbox-exec` with `(deny network*)` | the profile denies every network operation of the process tree; the probe confirms that `connect` fails with `EPERM` | `sandbox-exec` is deprecated by Apple; only `connect` is exercised by the probe; filesystem and IPC other than sockets are not restricted |
//! | Linux | `unshare --net --map-root-user` | the process tree runs in a new, empty network namespace (only a down loopback interface); the probe confirms `ENETUNREACH` | needs unprivileged user namespaces (on Ubuntu 24.04, `kernel.apparmor_restrict_unprivileged_userns=0`); filesystem and Unix sockets on the filesystem are not restricted |
//! | other | none | nothing; the job fails unless `--network-sandbox off` is given | |
//!
//! [`Sandbox::establish`] proves the denial before the builds by running
//! `xtask probe-network` inside the sandbox: it attempts a TCP connection to
//! 192.0.2.1 (TEST-NET-1, RFC 5737, never routed) and passes only if the
//! attempt fails with the mechanism's denial error, not with a timeout.

use std::ffi::OsString;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

use crate::cmd::Cmd;

/// Address used by the probe: TEST-NET-1, discard port.
pub const PROBE_ADDR: &str = "192.0.2.1:9";

const MACOS_PROFILE: &str = "(version 1)(allow default)(deny network*)";

/// Whether the job must run with the network denied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Required,
    Off,
}

/// How network access is denied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mechanism {
    SandboxExec,
    Unshare,
    None,
}

#[derive(Clone, Debug)]
pub struct Sandbox {
    pub mechanism: Mechanism,
}

/// Outcome of one connection attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeResult {
    Denied(String),
    NotDenied(String),
}

impl Sandbox {
    /// The mechanism for this platform, if one is available.
    pub fn for_platform() -> Sandbox {
        let mechanism = if cfg!(target_os = "macos") && Path::new("/usr/bin/sandbox-exec").exists()
        {
            Mechanism::SandboxExec
        } else if cfg!(target_os = "linux") {
            Mechanism::Unshare
        } else {
            Mechanism::None
        };
        Sandbox { mechanism }
    }

    pub fn none() -> Sandbox {
        Sandbox {
            mechanism: Mechanism::None,
        }
    }

    /// The command prefix that runs a program inside the sandbox.
    pub fn prefix(&self) -> Option<Vec<OsString>> {
        let words: &[&str] = match self.mechanism {
            Mechanism::SandboxExec => &["/usr/bin/sandbox-exec", "-p", MACOS_PROFILE],
            Mechanism::Unshare => &["unshare", "--net", "--map-root-user", "--"],
            Mechanism::None => return None,
        };
        Some(words.iter().map(OsString::from).collect())
    }

    pub fn describe(&self) -> &'static str {
        match self.mechanism {
            Mechanism::SandboxExec => "macOS sandbox-exec, profile (deny network*)",
            Mechanism::Unshare => "Linux unshare --net --map-root-user (empty network namespace)",
            Mechanism::None => "none",
        }
    }

    /// Sets up the sandbox for `mode` and proves that it denies network
    /// access by running `probe_exe probe-network` inside it.
    pub fn establish(mode: Mode, probe_exe: &Path) -> Result<Sandbox, String> {
        if mode == Mode::Off {
            return Ok(Sandbox::none());
        }
        let sandbox = Sandbox::for_platform();
        if sandbox.mechanism == Mechanism::None {
            return Err(
                "no network sandbox is available on this platform; rerun with `--network-sandbox off` \
                 to build with the offline Cargo configuration only"
                    .to_string(),
            );
        }
        let probe = Cmd::new(probe_exe).arg("probe-network");
        let out = probe.output(Some(&sandbox)).map_err(|e| {
            format!(
                "cannot start the probe in the sandbox ({}): {e}",
                sandbox.describe()
            )
        })?;
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if out.status.success() && stdout.starts_with("denied") {
            Ok(sandbox)
        } else {
            Err(format!(
                "the network probe was not denied inside the sandbox ({}): {} {}",
                sandbox.describe(),
                stdout,
                stderr
            ))
        }
    }
}

/// Attempts a TCP connection to [`PROBE_ADDR`] and classifies the result.
/// Only errors that a sandbox produces count as denial: `EPERM`/`EACCES`
/// (macOS sandbox, seccomp) and `ENETUNREACH` (empty network namespace). A
/// timeout, a refusal, or a connection means the network was reachable.
pub fn probe_network() -> ProbeResult {
    let addr: SocketAddr = match PROBE_ADDR.parse() {
        Ok(a) => a,
        Err(e) => return ProbeResult::NotDenied(format!("bad probe address: {e}")),
    };
    match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
        Ok(_) => ProbeResult::NotDenied(format!("connected to {PROBE_ADDR}")),
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied | ErrorKind::NetworkUnreachable => {
                ProbeResult::Denied(format!("connect to {PROBE_ADDR} failed: {e}"))
            }
            _ => ProbeResult::NotDenied(format!(
                "connect to {PROBE_ADDR} failed without denial: {e}"
            )),
        },
    }
}
