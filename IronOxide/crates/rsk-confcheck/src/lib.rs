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
//! Not yet compared with the binary: the `LAYOUT` (no image of it is
//! stored; the MPU values are checked against the Task_Model only) and the
//! stack region sizes against the Link_Checker (task 12). The Kernel's
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

/// Checks `model_text` (the Task_Model JSON) and, when given, the linked
/// binary. Errors are unusable inputs; discrepancies are in the outcome.
pub fn check(model_text: &str, elf: Option<&[u8]>) -> Result<Outcome, String> {
    let json: Value = serde_json::from_str(model_text).map_err(|e| format!("Task_Model is not JSON: {e}"))?;
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
        }
        None => out.notes.push("no binary given: the Task_Model's derived values were checked, not the linked binary (R26.4)".to_string()),
    }
    out.notes.push(format!("{} values compared", out.checked));
    Ok(out)
}
