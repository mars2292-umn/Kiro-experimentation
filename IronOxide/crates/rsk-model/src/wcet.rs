//! WCET_Records (`rsk-wcet/1`, R33.1) and Kernel_Timing_Parameters
//! (`rsk-kernel-timing/1`, R13.1, R13.2; a measured parameter carries its
//! runs, maximum and margin like a measured WCET_Record, task 19.3) as
//! serde types, plus the symbol
//! table of Table 13-1 (with the Δ_svc, Δ_dma_write and Δ_view additions
//! of design.md and the Partition stop/restart work of R14.10).

use serde::{Deserialize, Serialize};

pub const WCET_SCHEMA: &str = "rsk-wcet/1";
pub const KERNEL_TIMING_SCHEMA: &str = "rsk-kernel-timing/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WcetSet {
    pub schema: String,
    pub records: Vec<WcetRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measured {
    pub runs: u64,
    pub max_observed: u64,
    pub margin_percent: u64,
    pub coverage_branch_percent: u64,
    pub scenarios: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WcetRecord {
    pub item: String,
    pub wcet_cycles: u64,
    pub method: String,
    pub tool: String,
    pub binary_hash: String,
    pub target: String,
    pub clock_hz: u64,
    pub icache: String,
    pub conditions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measured: Option<Measured>,
}

impl WcetRecord {
    /// Provenance for the report (R30.3).
    pub fn provenance(&self) -> String {
        let mut s = format!("{}: {} cycles ({}, {}", self.item, self.wcet_cycles, self.method, self.tool);
        if let Some(m) = &self.measured {
            s.push_str(&format!(", {} runs, max {} +{}%, branch coverage {}%", m.runs, m.max_observed, m.margin_percent, m.coverage_branch_percent));
        }
        if !self.conditions.is_empty() {
            s.push_str(&format!(", conditions: {}", self.conditions.join("/")));
        }
        s.push(')');
        s
    }
}

/// The Kernel_Timing_Parameter symbols (Table 13-1 and design.md R13.6).
pub const KERNEL_TIMING_SYMBOLS: &[&str] = &[
    "delta_lock_in",
    "delta_lock_out",
    "delta_release",
    "delta_dispatch",
    "delta_complete",
    "delta_timer",
    "delta_irq",
    "delta_stamp",
    "l_kernel",
    "delta_fp_save",
    "delta_fp_restore",
    "delta_detect",
    "delta_enforce",
    "delta_hm",
    "delta_wake",
    "j_rel",
    "delta_svc",
    "delta_dma_write",
    "delta_view",
    "delta_restart",
    "delta_stop",
    "delta_entry",
    "delta_send",
    "delta_recv",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelTiming {
    pub schema: String,
    pub binary_hash: String,
    pub target: String,
    pub clock_hz: u64,
    pub icache: String,
    pub parameters: Vec<KernelTimingParameter>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelTimingParameter {
    pub symbol: String,
    pub cycles: u64,
    pub method: String,
    pub tool: String,
    pub conditions: Vec<String>,
    /// R33.1: for a measured parameter, the runs, the maximum observed
    /// value, the margin applied (R34.6), and the scenarios (task 19.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measured: Option<Measured>,
}

impl KernelTimingParameter {
    /// Provenance for the report (R30.3).
    pub fn provenance(&self) -> String {
        let mut s = format!("{}: {} cycles ({}, {}", self.symbol, self.cycles, self.method, self.tool);
        if let Some(m) = &self.measured {
            s.push_str(&format!(", {} runs, max {} +{}%", m.runs, m.max_observed, m.margin_percent));
            if !m.scenarios.is_empty() {
                s.push_str(&format!(", scenarios: {}", m.scenarios.join("/")));
            }
        }
        if !self.conditions.is_empty() {
            s.push_str(&format!(", conditions: {}", self.conditions.join("/")));
        }
        s.push(')');
        s
    }
}

impl KernelTiming {
    pub fn get(&self, symbol: &str) -> Option<u64> {
        self.parameters.iter().find(|p| p.symbol == symbol).map(|p| p.cycles)
    }

    pub fn parameter(&self, symbol: &str) -> Option<&KernelTimingParameter> {
        self.parameters.iter().find(|p| p.symbol == symbol)
    }

    /// Overlays `other`'s parameters on this set (task 19.3): a symbol
    /// `other` carries replaces this set's entry for it when `other`'s
    /// bound is at least as large, or when this set's entry is a
    /// placeholder (a measurement replaces a nominal value whatever its
    /// size); the set's binary hash and build settings stay this set's.
    /// Returns the symbols taken from `other`.
    pub fn overlay(&mut self, other: &KernelTiming) -> Vec<String> {
        let mut taken = Vec::new();
        for p in &other.parameters {
            match self.parameters.iter_mut().find(|q| q.symbol == p.symbol) {
                Some(q) => {
                    if q.method == "placeholder" || p.cycles >= q.cycles {
                        *q = p.clone();
                        taken.push(p.symbol.clone());
                    }
                }
                None => {
                    self.parameters.push(p.clone());
                    taken.push(p.symbol.clone());
                }
            }
        }
        taken
    }

    /// The symbols of Table 13-1 that the set lacks (R13.3).
    pub fn missing(&self) -> Vec<&'static str> {
        KERNEL_TIMING_SYMBOLS.iter().copied().filter(|s| self.get(s).is_none()).collect()
    }
}
