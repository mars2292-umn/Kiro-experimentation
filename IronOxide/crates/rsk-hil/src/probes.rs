//! The debug probes the host sees (`probe-rs list`). Boards are selected
//! by serial: `probe-rs list` orders probes arbitrarily, so a position is
//! never a board.

use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Probe {
    /// probe-rs's selector, `VID:PID:SERIAL`.
    pub id: String,
    /// The probe's name (`J-Link`).
    pub name: String,
}

/// The first line of `probe-rs --version`.
pub fn version() -> Result<String, String> {
    let out = Command::new("probe-rs").arg("--version").output().map_err(|e| format!("cannot run probe-rs: {e}"))?;
    if !out.status.success() {
        return Err("probe-rs --version failed".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or("").trim().to_string())
}

/// The attached probes.
pub fn list() -> Result<Vec<Probe>, String> {
    let out = Command::new("probe-rs").arg("list").output().map_err(|e| format!("cannot run probe-rs: {e}"))?;
    if !out.status.success() {
        return Err(format!("probe-rs list failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(parse_list(&String::from_utf8_lossy(&out.stdout)))
}

/// Parses `probe-rs list` (`[0]: J-Link -- 1366:1060:001050217737 (J-Link)`).
pub fn parse_list(text: &str) -> Vec<Probe> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix('[').and_then(|l| l.split_once("]:")).map(|(_, r)| r.trim()) else { continue };
        let Some((name, rest)) = rest.split_once(" -- ") else { continue };
        let id = rest.split_whitespace().next().unwrap_or("").to_string();
        if id.split(':').count() == 3 {
            out.push(Probe { id, name: name.trim().to_string() });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_probe_rs_list() {
        let text = "The following debug probes were found:\n[0]: J-Link -- 1366:1060:001050217737 (J-Link)\n[1]: J-Link -- 1366:1061:001050279833 (J-Link)\n";
        let probes = parse_list(text);
        assert_eq!(probes.len(), 2);
        assert_eq!(probes[1].id, "1366:1061:001050279833");
        assert_eq!(probes[0].name, "J-Link");
        assert!(parse_list("No debug probes were found.\n").is_empty());
    }
}
