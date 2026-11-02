//! The rig description `hil-rig.toml` (schema `rsk-hil-rig/1`): the boards
//! by probe serial and role, the analyzer and the channels it watches, the
//! wiring (documentation the report repeats), and what the self-test's
//! loopback expects. Unknown keys are errors, as in the build policy, so
//! a misspelt key cannot silently change what the rig checks.

use std::path::Path;

use toml::{Table, Value};

pub const SCHEMA: &str = "rsk-hil-rig/1";
/// The file's name at the workspace root.
pub const FILE: &str = "hil-rig.toml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    /// The role (`target`, `stimulus`, `spare`).
    pub role: String,
    /// probe-rs's probe selector, `VID:PID:SERIAL`.
    pub serial: String,
    pub board: String,
    /// The Application binary the rig flashes on it (the stimulus).
    pub bin: Option<String>,
    /// Wired into the rig (a spare is listed but not wired).
    pub wired: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Analyzer {
    /// The sigrok driver (`fx2lafw`).
    pub driver: String,
    pub samplerate_hz: u32,
    /// The channel (D0 to D7) carrying the stimulus board's mirror of its line.
    pub stimulus_channel: u8,
    /// The channel carrying the target's response line.
    pub response_channel: u8,
    /// The target's trace pins (rsk-kernel `trace-pins`): the executing
    /// Task's index, bit 0 first (7 = none), and the Kernel-active flag.
    pub task_id_channels: Vec<u8>,
    pub kernel_channel: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expect {
    pub period_ms: u64,
    pub pulse_us: u64,
    pub response_deadline_ms: u64,
    pub min_matched: usize,
    /// R9.10: the bound on a periodic Release's lag, in CPU cycles: the
    /// exported bound of the Kernel_Timing_Parameter J_rel (R13.4).
    pub j_rel_cycles: u32,
    /// R19.3: the bound on one run of the intake handler, in CPU cycles:
    /// the exported bound of Δ_irq (R13.4).
    pub delta_irq_cycles: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rig {
    pub chip: String,
    pub target: Board,
    pub stimulus: Board,
    pub spares: Vec<Board>,
    pub analyzer: Analyzer,
    /// The wiring table as written: `(key, text)` lines for the report.
    pub wiring: Vec<(String, String)>,
    pub expect: Expect,
}

impl Rig {
    /// The analyzer configuration at another sample rate.
    pub fn analyzer_at(&self, samplerate_hz: u32) -> Analyzer {
        Analyzer { samplerate_hz, ..self.analyzer.clone() }
    }

    pub fn load(path: &Path) -> Result<Rig, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Rig::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Rig, String> {
        let table: Table = text.parse().map_err(|e| format!("not valid TOML: {e}"))?;
        for key in table.keys() {
            if !["schema", "chip", "boards", "analyzer", "wiring", "expect"].contains(&key.as_str()) {
                return Err(format!("unknown key `{key}`"));
            }
        }
        let schema = str_of(&table, "schema")?;
        if schema != SCHEMA {
            return Err(format!("schema `{schema}` is not `{SCHEMA}`"));
        }
        let chip = str_of(&table, "chip")?.to_string();
        let boards = table.get("boards").and_then(Value::as_table).ok_or("no [boards] table")?;
        let mut target = None;
        let mut stimulus = None;
        let mut spares = Vec::new();
        for (role, entry) in boards {
            let entry = entry.as_table().ok_or_else(|| format!("[boards.{role}] is not a table"))?;
            for key in entry.keys() {
                if !["serial", "board", "bin", "wired"].contains(&key.as_str()) {
                    return Err(format!("[boards.{role}]: unknown key `{key}`"));
                }
            }
            let serial = str_of(entry, "serial").map_err(|e| format!("[boards.{role}]: {e}"))?.to_string();
            if serial.split(':').count() != 3 {
                return Err(format!("[boards.{role}]: serial `{serial}` is not `VID:PID:SERIAL`"));
            }
            let board = Board {
                role: role.clone(),
                serial,
                board: entry.get("board").and_then(Value::as_str).unwrap_or("").to_string(),
                bin: entry.get("bin").and_then(Value::as_str).map(str::to_string),
                wired: entry.get("wired").and_then(Value::as_bool).unwrap_or(true),
            };
            match role.as_str() {
                "target" => target = Some(board),
                "stimulus" => stimulus = Some(board),
                _ => spares.push(board),
            }
        }
        let target = target.ok_or("no [boards.target]")?;
        let stimulus = stimulus.ok_or("no [boards.stimulus]")?;
        if stimulus.bin.is_none() {
            return Err("[boards.stimulus] names no `bin`".into());
        }
        let mut serials: Vec<&str> = std::iter::once(&target).chain(std::iter::once(&stimulus)).chain(spares.iter()).map(|b| b.serial.as_str()).collect();
        serials.sort_unstable();
        if serials.windows(2).any(|w| w[0] == w[1]) {
            return Err("two boards share a serial".into());
        }
        let an = table.get("analyzer").and_then(Value::as_table).ok_or("no [analyzer] table")?;
        for key in an.keys() {
            if !["driver", "samplerate_hz", "stimulus_channel", "response_channel", "task_id_channels", "kernel_channel"].contains(&key.as_str()) {
                return Err(format!("[analyzer]: unknown key `{key}`"));
            }
        }
        let analyzer = Analyzer {
            driver: str_of(an, "driver")?.to_string(),
            samplerate_hz: int_of(an, "samplerate_hz")? as u32,
            stimulus_channel: channel_of(an, "stimulus_channel")?,
            response_channel: channel_of(an, "response_channel")?,
            task_id_channels: match an.get("task_id_channels") {
                None => Vec::new(),
                Some(v) => v
                    .as_array()
                    .ok_or("[analyzer]: task_id_channels is not an array")?
                    .iter()
                    .map(|c| c.as_integer().filter(|c| (0..=7).contains(c)).map(|c| c as u8).ok_or("[analyzer]: task_id_channels holds a value outside 0 to 7".to_string()))
                    .collect::<Result<Vec<u8>, String>>()?,
            },
            kernel_channel: match an.get("kernel_channel") {
                None => None,
                Some(_) => Some(channel_of(an, "kernel_channel")?),
            },
        };
        if analyzer.stimulus_channel == analyzer.response_channel {
            return Err("[analyzer]: the stimulus and response channels are the same".into());
        }
        if analyzer.samplerate_hz == 0 {
            return Err("[analyzer]: samplerate_hz is 0".into());
        }
        let mut wiring = Vec::new();
        if let Some(w) = table.get("wiring").and_then(Value::as_table) {
            for (k, v) in w {
                let text = match v {
                    Value::String(s) => s.clone(),
                    Value::Array(a) => a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "),
                    other => format!("{other:?}"),
                };
                wiring.push((k.clone(), text));
            }
        }
        let ex = table.get("expect").and_then(Value::as_table).ok_or("no [expect] table")?;
        for key in ex.keys() {
            if !["period_ms", "pulse_us", "response_deadline_ms", "min_matched", "j_rel_cycles", "delta_irq_cycles"].contains(&key.as_str()) {
                return Err(format!("[expect]: unknown key `{key}`"));
            }
        }
        let expect = Expect {
            period_ms: int_of(ex, "period_ms")?,
            pulse_us: int_of(ex, "pulse_us")?,
            response_deadline_ms: int_of(ex, "response_deadline_ms")?,
            min_matched: int_of(ex, "min_matched")? as usize,
            j_rel_cycles: ex.get("j_rel_cycles").map(|_| int_of(ex, "j_rel_cycles")).transpose()?.unwrap_or(u32::MAX as u64) as u32,
            delta_irq_cycles: ex.get("delta_irq_cycles").map(|_| int_of(ex, "delta_irq_cycles")).transpose()?.unwrap_or(u32::MAX as u64) as u32,
        };
        if expect.period_ms == 0 || expect.response_deadline_ms == 0 {
            return Err("[expect]: period_ms and response_deadline_ms must be positive".into());
        }
        Ok(Rig { chip, target, stimulus, spares, analyzer, wiring, expect })
    }

    /// Every board of the rig, the target first.
    pub fn boards(&self) -> impl Iterator<Item = &Board> {
        std::iter::once(&self.target).chain(std::iter::once(&self.stimulus)).chain(self.spares.iter())
    }
}

fn str_of<'a>(t: &'a Table, key: &str) -> Result<&'a str, String> {
    t.get(key).and_then(Value::as_str).ok_or_else(|| format!("`{key}` is missing or not a string"))
}

fn int_of(t: &Table, key: &str) -> Result<u64, String> {
    t.get(key).and_then(Value::as_integer).filter(|v| *v >= 0).map(|v| v as u64).ok_or_else(|| format!("`{key}` is missing or not a non-negative integer"))
}

fn channel_of(t: &Table, key: &str) -> Result<u8, String> {
    let v = int_of(t, key)?;
    if v > 7 {
        return Err(format!("`{key}` is {v}; the analyzer's channels are 0 to 7"));
    }
    Ok(v as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RIG: &str = r#"
schema = "rsk-hil-rig/1"
chip = "nRF52840_xxAA"
[boards.target]
serial = "1366:1060:001"
[boards.stimulus]
serial = "1366:1061:002"
bin = "hil-stimulus"
[boards.spare]
serial = "1366:1061:003"
wired = false
[analyzer]
driver = "fx2lafw"
samplerate_hz = 2000000
stimulus_channel = 6
response_channel = 0
[wiring]
stimulus_line = "P1.01"
board_spares = ["P1.02", "P1.03"]
[expect]
period_ms = 18
pulse_us = 100
response_deadline_ms = 15
min_matched = 9
"#;

    #[test]
    fn parses_the_rig() {
        let rig = Rig::parse(RIG).unwrap();
        assert_eq!(rig.target.serial, "1366:1060:001");
        assert_eq!(rig.stimulus.bin.as_deref(), Some("hil-stimulus"));
        assert_eq!(rig.spares.len(), 1);
        assert!(!rig.spares[0].wired);
        assert_eq!(rig.analyzer.stimulus_channel, 6);
        assert!(rig.analyzer.task_id_channels.is_empty() && rig.analyzer.kernel_channel.is_none());
        assert_eq!(rig.expect.j_rel_cycles, u32::MAX);
        assert_eq!(rig.expect.delta_irq_cycles, u32::MAX);
        let with_pins = RIG.replace("response_channel = 0", "response_channel = 0\ntask_id_channels = [1, 2, 3]\nkernel_channel = 4").replace("min_matched = 9", "min_matched = 9\nj_rel_cycles = 4000");
        let rig = Rig::parse(&with_pins).unwrap();
        assert_eq!(rig.analyzer.task_id_channels, vec![1, 2, 3]);
        assert_eq!(rig.analyzer.kernel_channel, Some(4));
        assert_eq!(rig.expect.j_rel_cycles, 4000);
        // Keys come out sorted (a TOML table).
        assert_eq!(rig.wiring[0], ("board_spares".to_string(), "P1.02, P1.03".to_string()));
        assert_eq!(rig.wiring[1], ("stimulus_line".to_string(), "P1.01".to_string()));
        assert_eq!(rig.expect.min_matched, 9);
        assert_eq!(rig.boards().count(), 3);
    }

    #[test]
    fn rejects_unknown_keys_and_bad_values() {
        assert!(Rig::parse(&RIG.replace("chip =", "chips =")).unwrap_err().contains("unknown key"));
        assert!(Rig::parse(&RIG.replace("response_channel = 0", "response_channel = 6")).unwrap_err().contains("same"));
        assert!(Rig::parse(&RIG.replace("response_channel = 0", "response_channel = 9")).unwrap_err().contains("0 to 7"));
        assert!(Rig::parse(&RIG.replace("1366:1061:003", "1366:1060:001")).unwrap_err().contains("share"));
        assert!(Rig::parse(&RIG.replace("bin = \"hil-stimulus\"\n", "")).unwrap_err().contains("bin"));
        assert!(Rig::parse(&RIG.replace("rsk-hil-rig/1", "rsk-hil-rig/2")).unwrap_err().contains("schema"));
    }
}
