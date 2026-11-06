//! The Kernel's fault record (rsk-kernel feature `trace-ram`; task 7.4):
//! `FAULT_RING` holds, for every MemManage, BusFault, and UsageFault the
//! Kernel took, the CFSR and the faulting address (MMFAR or BFAR when
//! valid, else the stacked PC), in order, so that each fault event of the
//! mirror can be matched with the access that caused it and its class
//! judged (precise or imprecise, HW-11).

pub const MAGIC: u32 = 0x5253_4b46;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    pub cfsr: u32,
    pub address: u32,
}

impl Fault {
    /// MemManage status (CFSR bits 7:0): IACCVIOL 0, DACCVIOL 1, MUNSTKERR 3, MSTKERR 4, MLSPERR 5, MMARVALID 7.
    pub fn mmfsr(&self) -> u8 {
        (self.cfsr & 0xFF) as u8
    }
    /// BusFault status (CFSR bits 15:8): IBUSERR 0, PRECISERR 1, IMPRECISERR 2, UNSTKERR 3, STKERR 4, LSPERR 5, BFARVALID 7.
    pub fn bfsr(&self) -> u8 {
        ((self.cfsr >> 8) & 0xFF) as u8
    }
    pub fn iaccviol(&self) -> bool {
        self.mmfsr() & 1 != 0
    }
    pub fn daccviol(&self) -> bool {
        self.mmfsr() & 2 != 0
    }
    pub fn mmarvalid(&self) -> bool {
        self.mmfsr() & 0x80 != 0
    }
    pub fn precise_bus(&self) -> bool {
        self.bfsr() & 2 != 0
    }
    pub fn imprecise_bus(&self) -> bool {
        self.bfsr() & 4 != 0
    }
    pub fn bfarvalid(&self) -> bool {
        self.bfsr() & 0x80 != 0
    }
    pub fn is_memmanage(&self) -> bool {
        self.mmfsr() & 0x3B != 0
    }
    pub fn is_bus(&self) -> bool {
        self.bfsr() & 0x3F != 0
    }

    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        if self.iaccviol() { parts.push("IACCVIOL"); }
        if self.daccviol() { parts.push("DACCVIOL"); }
        if self.mmfsr() & 0x08 != 0 { parts.push("MUNSTKERR"); }
        if self.mmfsr() & 0x10 != 0 { parts.push("MSTKERR"); }
        if self.mmarvalid() { parts.push("MMARVALID"); }
        if self.bfsr() & 1 != 0 { parts.push("IBUSERR"); }
        if self.precise_bus() { parts.push("PRECISERR"); }
        if self.imprecise_bus() { parts.push("IMPRECISERR"); }
        if self.bfsr() & 0x08 != 0 { parts.push("UNSTKERR"); }
        if self.bfsr() & 0x10 != 0 { parts.push("STKERR"); }
        if self.bfarvalid() { parts.push("BFARVALID"); }
        // UsageFault status (CFSR bits 31:16): UNDEFINSTR 0, INVSTATE 1, INVPC 2, NOCP 3, UNALIGNED 8, DIVBYZERO 9.
        let ufsr = (self.cfsr >> 16) & 0xFFFF;
        if ufsr & 0x001 != 0 { parts.push("UNDEFINSTR"); }
        if ufsr & 0x002 != 0 { parts.push("INVSTATE"); }
        if ufsr & 0x004 != 0 { parts.push("INVPC"); }
        if ufsr & 0x008 != 0 { parts.push("NOCP"); }
        if ufsr & 0x100 != 0 { parts.push("UNALIGNED"); }
        if ufsr & 0x200 != 0 { parts.push("DIVBYZERO"); }
        if ufsr & !0x30F != 0 { parts.push("UFSR"); }
        format!("cfsr {:#010x} ({}) at {:#010x}", self.cfsr, parts.join("|"), self.address)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Faults {
    /// Faults ever recorded (the ring holds the last `entries.len()`).
    pub written: u32,
    /// Oldest first.
    pub entries: Vec<Fault>,
}

impl Faults {
    pub fn decode(words: &[u32]) -> Result<Faults, String> {
        if words.len() < 2 || words[0] != MAGIC {
            return Err("FAULT_RING: no record (the magic word is absent: the Kernel never ran this build)".into());
        }
        let cap = (words.len() - 2) / 2;
        let written = words[1];
        let mut entries = Vec::new();
        if cap > 0 {
            let held = (written as usize).min(cap);
            let first = written as usize - held;
            for k in first..written as usize {
                let s = 2 + 2 * (k % cap);
                entries.push(Fault { cfsr: words[s], address: words[s + 1] });
            }
        }
        Ok(Faults { written, entries })
    }

    pub fn render(&self) -> String {
        let mut s = format!("{} faults recorded ({} held)\n", self.written, self.entries.len());
        for (i, f) in self.entries.iter().enumerate() {
            s.push_str(&format!("{i:4}: {}\n", f.describe()));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_ring_oldest_first() {
        let mut words = vec![MAGIC, 3, 0, 0, 0, 0];
        // Capacity 2: entries 1 and 2 are held, at slots 1 and 0.
        words[2] = 0x0000_0082; words[3] = 0x2000_0100; // entry 2 (slot 0)
        words[4] = 0x0000_0200; words[5] = 0xE000_E100; // entry 1 (slot 1)
        let f = Faults::decode(&words).unwrap();
        assert_eq!(f.written, 3);
        assert_eq!(f.entries.len(), 2);
        assert!(f.entries[0].precise_bus() && f.entries[0].address == 0xE000_E100);
        assert!(f.entries[1].daccviol() && f.entries[1].mmarvalid());
        assert!(Faults::decode(&[0, 0]).is_err());
    }
}
