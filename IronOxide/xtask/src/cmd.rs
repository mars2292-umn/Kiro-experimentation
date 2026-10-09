//! Commands run by the verification job, with a controlled environment.
//!
//! Every Cargo and rustc invocation goes through [`Cmd::cargo`] or
//! [`Cmd::rustc`], which remove variables that would change what is compiled
//! or how (`RUSTC_BOOTSTRAP`, `RUSTFLAGS`, `RUSTC`, `CARGO_BUILD_TARGET`, and
//! similar) and set `CARGO_ENCODED_RUSTFLAGS` and `CARGO_ENCODED_RUSTDOCFLAGS`
//! to the empty string. Cargo gives those two variables precedence over
//! `RUSTFLAGS` and over `rustflags` in any Cargo configuration file, so no
//! extra `--cfg` or `-Z` flag reaches rustc. The Target build alone adds
//! `--remap-path-prefix` (see `verify::target_build`) so that checkout paths
//! never reach a Flight_Build binary (R59.1). `RUSTC_WRAPPER` and
//! `RUSTC_WORKSPACE_WRAPPER` are set empty, which disables any wrapper
//! configured elsewhere. `cargo_frozen` adds `--frozen` (`--locked` plus
//! `--offline`) to every Cargo command that resolves dependencies (R58.1).

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use crate::sandbox::Sandbox;

/// Variables removed from the environment of every Cargo and rustc command.
pub const REMOVED_ENV: &[&str] = &[
    "RUSTC_BOOTSTRAP",
    "RUSTFLAGS",
    "RUSTDOCFLAGS",
    "RUSTC",
    "RUSTDOC",
    "CARGO_BUILD_RUSTFLAGS",
    "CARGO_BUILD_RUSTDOCFLAGS",
    "CARGO_BUILD_TARGET",
    "CARGO_BUILD_TARGET_DIR",
    "CARGO_TARGET_DIR",
    "CARGO_NET_OFFLINE",
];

/// Variables set for every Cargo and rustc command.
pub const PINNED_ENV: &[(&str, &str)] = &[
    ("CARGO_ENCODED_RUSTFLAGS", ""),
    ("CARGO_ENCODED_RUSTDOCFLAGS", ""),
    ("RUSTC_WRAPPER", ""),
    ("RUSTC_WORKSPACE_WRAPPER", ""),
];

/// Incoming variables that the job overrides, reported as warnings so that a
/// developer sees that the environment was ignored.
pub fn overridden_incoming_env() -> Vec<String> {
    REMOVED_ENV
        .iter()
        .copied()
        .chain(PINNED_ENV.iter().map(|(k, _)| *k))
        .filter(|k| *k != "RUSTC_BOOTSTRAP" && std::env::var_os(k).is_some_and(|v| !v.is_empty()))
        .map(str::to_string)
        .collect()
}

/// A command line with its working directory and environment changes.
#[derive(Clone, Debug)]
pub struct Cmd {
    program: OsString,
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
    env: Vec<(OsString, Option<OsString>)>,
}

impl Cmd {
    pub fn new(program: impl AsRef<OsStr>) -> Cmd {
        Cmd {
            program: program.as_ref().to_os_string(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
        }
    }

    /// The Cargo of the running toolchain (`$CARGO` when run through Cargo).
    pub fn cargo() -> Cmd {
        let program = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
        Cmd::new(program).controlled_env()
    }

    /// A Cargo command that resolves dependencies: always `--frozen`.
    pub fn cargo_frozen(subcommand: &str) -> Cmd {
        Cmd::cargo().arg(subcommand).arg("--frozen")
    }

    /// rustc as Cargo would find it: the rustup proxy on `PATH`, which
    /// selects the toolchain from `rust-toolchain.toml` in the working
    /// directory or from `RUSTUP_TOOLCHAIN`.
    pub fn rustc() -> Cmd {
        Cmd::new("rustc").controlled_env()
    }

    fn controlled_env(mut self) -> Cmd {
        for key in REMOVED_ENV {
            self.env.push((OsString::from(key), None));
        }
        for (key, value) in PINNED_ENV {
            self.env
                .push((OsString::from(key), Some(OsString::from(value))));
        }
        self
    }

    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Cmd {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Cmd
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|a| a.as_ref().to_os_string()));
        self
    }

    pub fn cwd(mut self, dir: &Path) -> Cmd {
        self.cwd = Some(dir.to_path_buf());
        self
    }

    pub fn env(mut self, key: &str, value: impl AsRef<OsStr>) -> Cmd {
        self.env
            .push((OsString::from(key), Some(value.as_ref().to_os_string())));
        self
    }

    /// The command line for logs.
    pub fn display(&self) -> String {
        let program = Path::new(&self.program)
            .file_name()
            .map_or_else(|| self.program.to_string_lossy(), OsStr::to_string_lossy)
            .into_owned();
        std::iter::once(program)
            .chain(self.args.iter().map(|a| a.to_string_lossy().into_owned()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Builds the process, inside the sandbox when one is given.
    pub fn command(&self, sandbox: Option<&Sandbox>) -> Command {
        let mut command = match sandbox.and_then(Sandbox::prefix) {
            Some(prefix) => {
                let mut c = Command::new(&prefix[0]);
                c.args(&prefix[1..]).arg(&self.program);
                c
            }
            None => Command::new(&self.program),
        };
        command.args(&self.args);
        if let Some(dir) = &self.cwd {
            command.current_dir(dir);
        }
        for (key, value) in &self.env {
            match value {
                Some(v) => command.env(key, v),
                None => command.env_remove(key),
            };
        }
        command
    }

    /// Runs to completion, capturing stdout and stderr.
    pub fn output(&self, sandbox: Option<&Sandbox>) -> io::Result<Output> {
        self.command(sandbox)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    }

    /// Runs to completion, capturing stdout and passing stderr through.
    pub fn output_stdout(&self, sandbox: Option<&Sandbox>) -> io::Result<Output> {
        self.command(sandbox)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
    }

    /// Runs to completion with output passed through; returns success.
    pub fn status(&self, sandbox: Option<&Sandbox>) -> io::Result<bool> {
        Ok(self
            .command(sandbox)
            .stdin(Stdio::null())
            .status()?
            .success())
    }
}

/// Runs a command and returns its stdout as text, or an error message that
/// includes stderr.
pub fn stdout_of(cmd: &Cmd) -> Result<String, String> {
    let out = cmd
        .output(None)
        .map_err(|e| format!("cannot run `{}`: {e}", cmd.display()))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(format!(
            "`{}` failed ({}): {}",
            cmd.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}
