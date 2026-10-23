//! The Core_Subset (`profile/core_subset.toml`) as the lint matches it:
//! each item's module, its listed names (none: the whole module), and its
//! scopes (R57.2, PR-21).

#[derive(Debug, Clone)]
pub struct Item {
    pub raw: String,
    pub module: String,
    pub names: Vec<String>,
    pub scope: Vec<String>,
}

pub fn parse(text: &str) -> Result<Vec<Item>, String> {
    let table: toml::Table = text.parse().map_err(|e| format!("core_subset.toml: {e}"))?;
    let mut out = Vec::new();
    let Some(items) = table.get("item").and_then(|v| v.as_array()) else { return Ok(out) };
    for it in items {
        let raw = it.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let scope: Vec<String> = it.get("scope").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default();
        let (module, names) = match raw.find("::{") {
            Some(k) => {
                let inner = raw[k + 3..].trim_end_matches('}');
                let names = inner.split(',').map(|n| n.trim().split(['(', '!', ' ']).next().unwrap_or("").to_string()).filter(|n| !n.is_empty()).collect();
                (raw[..k].to_string(), names)
            }
            None => match raw.rsplit_once("::") {
                // `core::option::Option`: module `core::option`, name `Option`;
                // a lowercase last segment that is a module (`core::primitive`,
                // `core::fmt`, `core::panicking`) is a whole-module item.
                Some((m, n)) if n.chars().next().is_some_and(|c| c.is_ascii_uppercase()) || matches!(n, "asm" | "naked_asm" | "black_box") => (m.to_string(), vec![n.to_string()]),
                _ => (raw.clone(), vec![]),
            },
        };
        out.push(Item { raw, module, names, scope });
    }
    Ok(out)
}

/// The item a `core::...` path maps to: the item's module is a prefix of
/// the path and, when the item lists names, one of them is a segment.
pub fn lookup<'a>(path: &str, items: &'a [Item]) -> Option<&'a Item> {
    let segs: Vec<&str> = path.split("::").collect();
    items.iter().find(|it| {
        let prefix_ok = path == it.module || path.starts_with(&format!("{}::", it.module));
        prefix_ok && (it.names.is_empty() || it.names.iter().any(|n| segs.contains(&n.as_str())))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_and_lookup() {
        let items = parse("[[item]]\npath = \"core::ptr::{read_volatile, write_volatile}\"\nscope = [\"kernel-unsafe\"]\n[[item]]\npath = \"core::option::Option\"\nscope = [\"kernel\", \"application\"]\n[[item]]\npath = \"core::fmt\"\nscope = [\"unreachable\"]\n[[item]]\npath = \"core::arch::asm\"\nscope = [\"kernel-unsafe\"]\n").unwrap();
        assert_eq!(lookup("core::ptr::write_volatile", &items).unwrap().raw, "core::ptr::{read_volatile, write_volatile}");
        assert!(lookup("core::ptr::null", &items).is_none());
        assert_eq!(lookup("core::option::Option", &items).unwrap().scope, vec!["kernel", "application"]);
        assert_eq!(lookup("core::fmt::Write", &items).unwrap().raw, "core::fmt");
        assert!(lookup("core::arch::asm", &items).is_some());
        assert!(lookup("core::arch::global_asm", &items).is_none());
    }
}
