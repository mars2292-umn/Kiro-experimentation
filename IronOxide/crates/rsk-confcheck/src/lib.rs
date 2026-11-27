//! The Config_Checker (Requirement 26, task 13.1): an implementation
//! independent of the Generator (R26.1) that reads the Task_Model with its
//! own parser, recomputes every derived value of R26.2 from the declared
//! values, extracts the `CONFIG` tables from the linked binary (the
//! `RSK_CONFIG_IMAGE` and `RSK_CONFIG_CHECKSUM` symbols, R26.4), and reports
//! every item whose recomputed, declared, or extracted values differ, with
//! the expected and the found value (R26.3).
//!
//! What is checked:
//!
//! | R26.2 value class | Recomputed from | Compared with |
//! |---|---|---|
//! | Ceilings | the Priorities of each Resource's accessors | Task_Model `ceiling`; binary `ResourceCfg.ceiling` |
//! | Priority-to-hardware mapping | DD-02: `(8 - p) << 5`, Kernel level 0 | Task_Model `priority_map`, `kernel_levels` |
//! | Nesting depths | the number of Resources per Task (PR-07) | Task_Model `nesting_depth` |
//! | Capacities (R23.4, R9.4) | lock stack = Σ depths; timer slots = 3·NT + 2; log as declared | Task_Model `capacities`; binary `slot_count` and slot tables |
//! | Budget and MIT tables | the declared ticks and cycles | binary `TaskCfg` fields, `operating_duration` |
//! | Kernel-owned peripherals (R15.6) | this crate's Target table | Task_Model `peripherals` |
//! | MPU permissions | power-of-two aligned regions, read-only EasyDMA windows (DD-08), stack within RAM, disjointness | Task_Model `mpu` |
//! | Checksum (R25.2, R26.4) | CRC-32 (IEEE 802.3) of the extracted image | binary `RSK_CONFIG_CHECKSUM`; Task_Model `config_checksum` |
//!
//! | `LAYOUT` (R16.7) | the Task_Model's MPU values | binary `RSK_LAYOUT_IMAGE` and `RSK_LAYOUT_CHECKSUM` (task 13.2) |
//!
//! | Stack region sizes (R26.2, R37.4) | the Link_Checker's `rsk-stack-bounds/1` export | Task_Model `regions.stack` (declared) and the computed worst case | The Kernel's
//! table encodings (`TaskCfg`, `PartitionCfg`, the kinds, policies, and
//! responses) are restated in `image` from the Kernel's documentation.
#![forbid(unsafe_code)]
#![cfg_attr(not(verus_keep_ghost), allow(unused_imports))]

use serde_json::Value;

pub mod derive_v;
pub mod image;
pub mod image_v;
pub mod model;
pub mod recompute;
pub mod tables_v;

/// One discrepancy (R26.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discrepancy {
    pub item: String,
    pub expected: String,
    pub found: String,
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub discrepancies: Vec<Discrepancy>,
    pub notes: Vec<String>,
    /// Values compared.
    pub checked: usize,
}

impl Outcome {
    pub fn mismatch(&mut self, item: impl Into<String>, expected: impl ToString, found: impl ToString) {
        self.checked += 1;
        let (expected, found) = (expected.to_string(), found.to_string());
        if expected != found {
            self.discrepancies.push(Discrepancy { item: item.into(), expected, found });
        }
    }

    pub fn require(&mut self, item: impl Into<String>, ok: bool, expected: impl ToString, found: impl ToString) {
        self.checked += 1;
        if !ok {
            self.discrepancies.push(Discrepancy { item: item.into(), expected: expected.to_string(), found: found.to_string() });
        }
    }
}

/// `check` plus the Link_Checker's stack bounds (R26.2: each declared stack
/// region against its computed worst case, R37.4).
pub fn check_with_bounds(model_text: &str, elf: Option<&[u8]>, bounds: Option<&str>) -> Result<Outcome, String> {
    let mut out = check(model_text, elf)?;
    if let Some(text) = bounds {
        let json: Value = serde_json::from_str(text).map_err(|e| format!("stack bounds are not JSON: {e}"))?;
        let model_json: Value = serde_json::from_str(model_text).map_err(|e| format!("Task_Model is not JSON: {e}"))?;
        let model = model::Model::parse(&model_json)?;
        recompute::check_stack_bounds(&model, &json, &mut out)?;
    } else {
        out.notes.push("no stack bounds given: the stack region sizes were not checked against the Link_Checker (R26.2)".to_string());
    }
    out.notes.push(format!("{} values compared", out.checked));
    Ok(out)
}

/// R27.2: a Task_Model that assigns any item to a core other than 0, or
/// marks a Resource global, is rejected naming the item and citing NG-01
/// (v1 is single-core; the fields exist for the partitioned multi-core
/// extension, R27.1).
fn reject_other_cores(json: &Value) -> Result<(), String> {
    let items = |key: &str| json.get(key).and_then(Value::as_array).into_iter().flatten();
    let id = |v: &Value| v.get("id").and_then(Value::as_str).unwrap_or("?").to_string();
    for t in items("tasks") {
        if t.get("core").and_then(Value::as_u64) != Some(0) {
            return Err(format!("NG-01: Task `{}` is assigned to core {} (R27.2: v1 is single-core)", id(t), t.get("core").cloned().unwrap_or(Value::Null)));
        }
    }
    for r in items("resources") {
        match r.get("core") {
            Some(Value::Number(n)) if n.as_u64() == Some(0) => {}
            other => {
                return Err(format!("NG-01: Resource `{}` is not on core 0 but `{}` (R27.2: v1 is single-core)", id(r), other.cloned().unwrap_or(Value::Null)));
            }
        }
    }
    for e in items("endpoints") {
        let cores: Vec<u64> = e.get("cores").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_u64).collect();
        if cores.iter().any(|&c| c != 0) {
            return Err(format!("NG-01: Endpoint `{}` spans cores {:?} (R27.2: v1 is single-core)", id(e), cores));
        }
    }
    Ok(())
}

/// R6.7: one version symbol of the Kernel in the binary against the value
/// the Task_Model records. A symbol that is absent, and one that differs,
/// are each a discrepancy.
pub fn check_version(out: &mut Outcome, symbol: &str, model_field: &str, in_model: Option<&str>, in_binary: Option<&str>) {
    let item = format!("{symbol} (R6.7; Task_Model `{model_field}`)");
    match (in_model, in_binary) {
        (Some(m), Some(b)) => out.mismatch(item, m, b),
        (Some(m), None) => out.require(item, false, m, "absent from the binary"),
        (None, b) => out.require(item, false, format!("`{model_field}` in the Task_Model"), format!("missing from the Task_Model (binary: {})", b.unwrap_or("absent"))),
    }
}

/// Checks `model_text` (the Task_Model JSON) and, when given, the linked
/// binary. Errors are unusable inputs; discrepancies are in the outcome.
pub fn check(model_text: &str, elf: Option<&[u8]>) -> Result<Outcome, String> {
    let json: Value = serde_json::from_str(model_text).map_err(|e| format!("Task_Model is not JSON: {e}"))?;
    reject_other_cores(&json)?;
    let model = model::Model::parse(&json)?;
    let mut out = Outcome::default();
    // R23.5: the priority-to-hardware mapping and the Kernel-reserved
    // levels, which the checker recomputes, are marked as derived.
    let marked: Vec<&str> = json.get("derived").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect();
    for name in ["kernel_levels", "priority_map"] {
        out.require(format!("derived marker of `{name}` (R23.5)"), marked.contains(&name), "listed in `derived`", "not listed");
    }
    recompute::check_model(&model, &mut out);
    match elf {
        Some(bytes) => {
            let img = image::extract(bytes)?;
            out.notes.push(format!(
                "binary: RSK_CONFIG_IMAGE {} bytes at 0x{:08x}, RSK_CONFIG_CHECKSUM 0x{:08x}",
                img.bytes.len(),
                img.address,
                img.stored_checksum
            ));
            // docs/pacemaker PM33.4: the image's tail is the PPI channels'
            // image and `RSK_PPI` the table the Kernel writes from, both
            // against the table recomputed from the Task_Model; the tables
            // are decoded from what precedes the tail. Without channels the
            // tail is empty and the image is the tables, as before.
            let mut ppi_rules = Outcome::default();
            let ppi = recompute::expected_ppi(&model, &mut ppi_rules);
            let ppi_symbol = image::ppi_symbol(bytes)?;
            let tables_len = recompute::check_ppi_binary(&ppi, &img.bytes, ppi_symbol.as_deref(), &mut out).unwrap_or(img.bytes.len());
            let tables = image::decode(&img.bytes[..tables_len])?;
            recompute::check_binary(&model, &img, tables_len, &tables, &mut out);
            // R6.7: the Kernel linked is the one the model was generated
            // for, of the model's Profile version and Profile_Variant.
            for (symbol, field, value) in [
                ("RSK_KERNEL_VERSION", "kernel", json.get("kernel").and_then(Value::as_str)),
                ("RSK_PROFILE_VERSION", "profile.version", json.get("profile").and_then(|p| p.get("version")).and_then(Value::as_str)),
                ("RSK_PROFILE_VARIANT", "profile.variant", json.get("profile").and_then(|p| p.get("variant")).and_then(Value::as_str)),
            ] {
                let found = image::version_symbol(bytes, symbol)?;
                check_version(&mut out, symbol, field, value, found.as_deref());
            }
            let limg = image::extract_layout(bytes)?;
            out.notes.push(format!(
                "binary: RSK_LAYOUT_IMAGE {} bytes at 0x{:08x}, RSK_LAYOUT_CHECKSUM 0x{:08x}",
                limg.bytes.len(),
                limg.address,
                limg.stored_checksum
            ));
            let layout = image::decode_layout(&limg.bytes)?;
            recompute::check_layout_binary(&model, &limg, &layout, &mut out);
            // The Endpoint table (task 21.2): compared when the binary carries it.
            match image::extract_endpoints(bytes)? {
                Some(eimg) => {
                    out.notes.push(format!("binary: RSK_ENDPOINT_IMAGE {} bytes at 0x{:08x}, RSK_ENDPOINT_CHECKSUM 0x{:08x}", eimg.bytes.len(), eimg.address, eimg.stored_checksum));
                    let (table, storage_bytes) = image::decode_endpoints(&eimg.bytes)?;
                    recompute::check_endpoints(&model, &eimg, &table, storage_bytes, &mut out);
                    // R16.1 (2026-11-21): where the Endpoint storage lies,
                    // the fact the Kernel's message statements rest on.
                    recompute::check_storage_placement(&layout, &table, storage_bytes, image::endpoint_storage_symbol(bytes)?, &mut out);
                }
                None => out.require("RSK_ENDPOINT_IMAGE", model.endpoints.is_empty(), "present when the Task_Model declares Endpoints", "absent from the binary"),
            }
        }
        None => out.notes.push("no binary given: the Task_Model's derived values were checked, not the linked binary (R26.4)".to_string()),
    }
    Ok(out)
}

#[cfg(test)]
mod version_tests {
    use super::*;

    /// R6.7: a Kernel version symbol that is absent from the binary, and
    /// one that differs from the Task_Model's, are each reported; an equal
    /// one is counted and not reported.
    #[test]
    fn r6_7_an_absent_or_differing_kernel_version_symbol_is_reported() {
        let mut out = Outcome::default();
        check_version(&mut out, "RSK_KERNEL_VERSION", "kernel", Some("1.2.0"), Some("1.2.0"));
        assert!(out.discrepancies.is_empty() && out.checked == 1);
        check_version(&mut out, "RSK_KERNEL_VERSION", "kernel", Some("1.2.0"), Some("1.1.0"));
        check_version(&mut out, "RSK_KERNEL_VERSION", "kernel", Some("1.2.0"), None);
        check_version(&mut out, "RSK_KERNEL_VERSION", "kernel", None, Some("1.2.0"));
        assert_eq!(out.checked, 4);
        let found: Vec<(&str, &str)> = out.discrepancies.iter().map(|d| (d.expected.as_str(), d.found.as_str())).collect();
        assert_eq!(found[0], ("1.2.0", "1.1.0"));
        assert_eq!(found[1], ("1.2.0", "absent from the binary"));
        assert!(found[2].1.contains("missing from the Task_Model"));
        assert!(out.discrepancies.iter().all(|d| d.item.contains("R6.7")));
    }
}

#[cfg(test)]
mod floor_tests {
    use super::*;

    /// R23.4, R26.3: an Application whose Tasks access no Resource has a
    /// lock stack of one slot, the Generator's floor, and the checker
    /// recomputes the same (until 2026-11-17 it recomputed 0 and reported
    /// the HIL stimulus Application's model).
    #[test]
    fn r23_4_an_application_without_resources_has_a_lock_stack_of_one() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/hil-stimulus/gen/task_model.json");
        let text = std::fs::read_to_string(&path).unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();
        assert!(json["resources"].as_array().unwrap().is_empty(), "the stimulus Application declares no Resource");
        assert_eq!(json["capacities"]["lock_stack"], Value::from(1));
        let out = check(&text, None).unwrap();
        assert!(out.discrepancies.is_empty(), "{:?}", out.discrepancies.iter().map(|d| format!("{}: expected {}, found {}", d.item, d.expected, d.found)).collect::<Vec<_>>());
    }
}
