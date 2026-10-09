//! The workspace as `cargo metadata --frozen` reports it.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::cmd::{stdout_of, Cmd};
use crate::diag::{Report, Rule};
use crate::policy::Policy;

#[derive(Clone, Debug)]
pub struct Target {
    pub name: String,
    pub kinds: Vec<String>,
    pub src_path: PathBuf,
}

impl Target {
    pub fn is_lib(&self) -> bool {
        self.kinds.iter().any(|k| {
            matches!(
                k.as_str(),
                "lib" | "rlib" | "staticlib" | "cdylib" | "dylib" | "proc-macro"
            )
        })
    }
}

#[derive(Clone, Debug)]
pub struct Dependency {
    pub name: String,
    pub req: String,
    /// `None` for normal dependencies, else `dev` or `build`.
    pub kind: Option<String>,
    pub source: Option<String>,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
    pub dir: PathBuf,
    pub source: Option<String>,
    pub edition: String,
    pub targets: Vec<Target>,
    pub dependencies: Vec<Dependency>,
    pub member: bool,
}

impl Package {
    pub fn lib(&self) -> Option<&Target> {
        self.targets.iter().find(|t| t.is_lib())
    }

    pub fn is_proc_macro(&self) -> bool {
        self.targets
            .iter()
            .any(|t| t.kinds.iter().any(|k| k == "proc-macro"))
    }

    /// The `.rs` files of the library target: every file under the
    /// directory of the library root, except other targets' roots and their
    /// directories (such as `src/main.rs` and `src/bin/`). Modules placed
    /// elsewhere with `#[path]` are not included.
    pub fn lib_files(&self) -> Vec<PathBuf> {
        let Some(lib) = self.lib() else {
            return Vec::new();
        };
        let Some(dir) = lib.src_path.parent() else {
            return vec![lib.src_path.clone()];
        };
        let others: Vec<&Path> = self
            .targets
            .iter()
            .filter(|t| !t.is_lib())
            .map(|t| t.src_path.as_path())
            .collect();
        rust_files(dir)
            .into_iter()
            .filter(|f| {
                !others.iter().any(|o| {
                    // Another target's root, or a file in a directory of
                    // its own below the library directory (`src/bin/x.rs`).
                    *f == *o
                        || o.parent()
                            .is_some_and(|od| od != dir && od.starts_with(dir) && f.starts_with(od))
                })
            })
            .collect()
    }
}

/// One edge of the resolved dependency graph.
#[derive(Clone, Debug)]
pub struct Edge {
    pub to: String,
    /// The dependency kinds of the edge: `None` (normal), `dev`, `build`.
    pub kinds: Vec<Option<String>>,
}

#[derive(Clone, Debug)]
pub struct Workspace {
    pub root: PathBuf,
    pub packages: BTreeMap<String, Package>,
    /// Package ids of the workspace members.
    pub members: Vec<String>,
    pub policy: Policy,
    /// The dependency graph resolved for the Target platform
    /// (`cargo metadata --filter-platform <target>`).
    pub target_graph: BTreeMap<String, Vec<Edge>>,
}

impl Workspace {
    /// Loads the workspace. Diagnostics go to `report`; `None` means the
    /// remaining checks cannot run.
    pub fn load(root: &Path, report: &mut Report) -> Option<Workspace> {
        let manifest = root.join("Cargo.toml");
        if !root.join("Cargo.lock").is_file() {
            report.error(
                Rule::Locked,
                Some("Cargo.lock".to_string()),
                "the workspace has no committed lock file; builds must resolve only from Cargo.lock",
            );
            return None;
        }
        let full = metadata(&manifest, None, report)?;
        let policy = match full.get("metadata").map(Policy::from_workspace_metadata) {
            Some(Ok(policy)) => policy,
            Some(Err(e)) => {
                report.error(Rule::Policy, Some("Cargo.toml".to_string()), e);
                return None;
            }
            None => {
                report.error(
                    Rule::Policy,
                    Some("Cargo.toml".to_string()),
                    "the root Cargo.toml has no [workspace.metadata.rsk] table",
                );
                return None;
            }
        };
        let filtered = metadata(&manifest, Some(&policy.target), report)?;

        let members: BTreeSet<String> = strings(full.get("workspace_members"));
        let mut packages = BTreeMap::new();
        for p in full
            .get("packages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let package = parse_package(p, &members);
            packages.insert(package.id.clone(), package);
        }
        let target_graph = parse_graph(&filtered);
        Some(Workspace {
            root: root.to_path_buf(),
            packages,
            members: members.into_iter().collect(),
            policy,
            target_graph,
        })
    }

    pub fn member_packages(&self) -> impl Iterator<Item = &Package> {
        self.members.iter().filter_map(|id| self.packages.get(id))
    }

    pub fn package_named(&self, name: &str) -> Option<&Package> {
        self.member_packages().find(|p| p.name == name)
    }

    /// `path` relative to the workspace root, for messages.
    pub fn rel(&self, path: &Path) -> String {
        rel(&self.root, path)
    }

    /// The crates linked into the Target build of `roots`: the closure over
    /// normal dependencies in the Target-filtered graph. Proc-macro crates
    /// run on the host and are not linked, so the walk does not enter them.
    pub fn target_closure<'a>(&self, roots: impl Iterator<Item = &'a str>) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut queue: VecDeque<String> = roots
            .filter_map(|name| self.package_named(name).map(|p| p.id.clone()))
            .collect();
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id.clone()) {
                continue;
            }
            for edge in self.target_graph.get(&id).into_iter().flatten() {
                let normal = edge.kinds.iter().any(Option::is_none);
                let proc_macro = self
                    .packages
                    .get(&edge.to)
                    .is_some_and(Package::is_proc_macro);
                if normal && !proc_macro {
                    queue.push_back(edge.to.clone());
                }
            }
        }
        seen
    }
}

fn metadata(manifest: &Path, platform: Option<&str>, report: &mut Report) -> Option<Value> {
    let mut cmd = Cmd::cargo_frozen("metadata")
        .args(["--format-version", "1", "--manifest-path"])
        .arg(manifest);
    if let Some(platform) = platform {
        cmd = cmd.args(["--filter-platform", platform]);
    }
    match stdout_of(&cmd) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(value) => Some(value),
            Err(e) => {
                report.error(
                    Rule::Step,
                    None,
                    format!("cannot parse cargo metadata output: {e}"),
                );
                None
            }
        },
        Err(e) if e.contains("lock file") || e.contains("Cargo.lock") => {
            let missing = unlocked_dependencies(manifest);
            let named = if missing.is_empty() {
                String::new()
            } else {
                format!(" ({})", missing.join(", "))
            };
            report.error(
                Rule::Locked,
                Some("Cargo.lock".to_string()),
                format!(
                    "the dependency graph needs packages that the committed lock file does not list{named}, \
                     and --frozen forbids updating it: {}",
                    first_error_line(&e)
                ),
            );
            None
        }
        Err(e) => {
            report.error(Rule::Step, None, e);
            None
        }
    }
}

/// Best effort, for the message only: the dependencies (and members) that
/// the workspace declares but `Cargo.lock` does not name.
/// `cargo metadata --no-deps` reads the manifests without resolving.
fn unlocked_dependencies(manifest: &Path) -> Vec<String> {
    let cmd = Cmd::cargo_frozen("metadata")
        .args(["--no-deps", "--format-version", "1", "--manifest-path"])
        .arg(manifest);
    let Ok(text) = stdout_of(&cmd) else {
        return Vec::new();
    };
    let Ok(meta) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    let lock_path = manifest.with_file_name("Cargo.lock");
    let locked: BTreeSet<String> = std::fs::read_to_string(lock_path)
        .ok()
        .and_then(|t| t.parse::<toml::Table>().ok())
        .and_then(|t| t.get("package").and_then(toml::Value::as_array).cloned())
        .into_iter()
        .flatten()
        .filter_map(|p| {
            p.get("name")
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
        .collect();
    let mut missing = BTreeSet::new();
    for p in meta
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let deps = p
            .get("dependencies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten();
        let names = std::iter::once(str_of(p, "name")).chain(deps.map(|d| str_of(d, "name")));
        missing.extend(names.filter(|n| !n.is_empty() && !locked.contains(n)));
    }
    missing.into_iter().collect()
}

fn first_error_line(text: &str) -> &str {
    text.lines()
        .find(|l| l.contains("error"))
        .map_or(text, str::trim)
}

fn strings(value: Option<&Value>) -> BTreeSet<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn str_of(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn parse_package(p: &Value, members: &BTreeSet<String>) -> Package {
    let id = str_of(p, "id");
    let manifest_path = PathBuf::from(str_of(p, "manifest_path"));
    let dir = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let targets = p
        .get("targets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|t| Target {
            name: str_of(t, "name"),
            kinds: strings(t.get("kind")).into_iter().collect(),
            src_path: canonical(Path::new(&str_of(t, "src_path"))),
        })
        .collect();
    let dependencies = p
        .get("dependencies")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|d| Dependency {
            name: str_of(d, "name"),
            req: str_of(d, "req"),
            kind: d.get("kind").and_then(Value::as_str).map(str::to_string),
            source: d.get("source").and_then(Value::as_str).map(str::to_string),
            path: d.get("path").and_then(Value::as_str).map(PathBuf::from),
        })
        .collect();
    Package {
        member: members.contains(&id),
        id,
        name: str_of(p, "name"),
        version: str_of(p, "version"),
        dir: canonical(&dir),
        source: p.get("source").and_then(Value::as_str).map(str::to_string),
        edition: str_of(p, "edition"),
        targets,
        dependencies,
    }
}

fn parse_graph(metadata: &Value) -> BTreeMap<String, Vec<Edge>> {
    let mut graph = BTreeMap::new();
    let nodes = metadata
        .get("resolve")
        .and_then(|r| r.get("nodes"))
        .and_then(Value::as_array);
    for node in nodes.into_iter().flatten() {
        let edges = node
            .get("deps")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|d| Edge {
                to: str_of(d, "pkg"),
                kinds: d
                    .get("dep_kinds")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|k| k.get("kind").and_then(Value::as_str).map(str::to_string))
                    .collect(),
            })
            .collect();
        graph.insert(str_of(node, "id"), edges);
    }
    graph
}

/// The canonical form of `path`, or `path` itself if it cannot be resolved.
pub fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `path` relative to `root` with `/` separators.
pub fn rel(root: &Path, path: &Path) -> String {
    let root = canonical(root);
    let path = canonical(path);
    path.strip_prefix(&root)
        .unwrap_or(&path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Every `.rs` file under `dir`, sorted. Hidden directories, `target`
/// directories, and directories that hold their own `Cargo.toml` (nested
/// packages and fixture workspaces) are skipped.
pub fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(&canonical(dir), true, &mut out);
    out.sort();
    out
}

fn collect(dir: &Path, top: bool, out: &mut Vec<PathBuf>) {
    if !top && dir.join("Cargo.toml").exists() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if !name.starts_with('.') && name != "target" {
                collect(&path, false, out);
            }
        } else if kind.is_file() && name.ends_with(".rs") {
            out.push(path);
        }
    }
}
