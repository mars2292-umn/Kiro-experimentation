//! The Profile source (`profile/profile.toml`): the version identifier that
//! the Build_Manifest, every Evidence_Item, and the generated
//! `profile_version.rs` files carry (R1.4, R3.5). The full Profile checker
//! and document generator live in `profile_gen`.

use std::path::Path;

use toml::{Table, Value};

pub const SOURCE: &str = "profile/profile.toml";

/// `(version, variant)` of the Profile source, or placeholders when the
/// source is absent or unreadable.
pub fn version_of(root: &Path) -> (String, String) {
    let fallback = ("unversioned".to_string(), "Ravenscar".to_string());
    let Ok(text) = std::fs::read_to_string(root.join(SOURCE)) else {
        return fallback;
    };
    let Ok(table) = text.parse::<Table>() else {
        return fallback;
    };
    let profile = table.get("profile").and_then(Value::as_table);
    let get = |key: &str| {
        profile
            .and_then(|p| p.get(key))
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    (
        get("version").unwrap_or(fallback.0),
        get("default_variant").unwrap_or(fallback.1),
    )
}
