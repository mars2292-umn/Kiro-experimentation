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

use serde_json::Value;

pub mod image;
pub mod model;
pub mod recompute;

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

/// Checks `model_text` (the Task_Model JSON) and, when given, the linked
/// binary. Errors are unusable inputs; discrepancies are in the outcome.
pub fn check(model_text: &str, elf: Option<&[u8]>) -> Result<Outcome, String> {
    let json: Value = serde_json::from_str(model_text).map_err(|e| format!("Task_Model is not JSON: {e}"))?;
    reject_other_cores(&json)?;
    let model = model::Model::parse(&json)?;
    let mut out = Outcome::default();
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
            let tables = image::decode(&img.bytes)?;
            recompute::check_binary(&model, &img, &tables, &mut out);
            let limg = image::extract_layout(bytes)?;
            out.notes.push(format!(
                "binary: RSK_LAYOUT_IMAGE {} bytes at 0x{:08x}, RSK_LAYOUT_CHECKSUM 0x{:08x}",
                limg.bytes.len(),
                limg.address,
                limg.stored_checksum
            ));
            let layout = image::decode_layout(&limg.bytes)?;
            recompute::check_layout_binary(&model, &limg, &layout, &mut out);
        }
        None => out.notes.push("no binary given: the Task_Model's derived values were checked, not the linked binary (R26.4)".to_string()),
    }
    Ok(out)
}
