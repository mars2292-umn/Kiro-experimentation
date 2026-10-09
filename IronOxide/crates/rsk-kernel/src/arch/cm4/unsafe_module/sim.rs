//! The simulated register file (R45.4; task 6.1): the Kani stubs and the
//! host-test doubles for the memory-mapped registers and the core
//! registers that `hw` accesses. Each simulated register behaves as the
//! Hardware_Model step says; `docs/kani_stubs.toml` lists them, and the HIL
//! differential tests of R49.3 validate each against the Target.
//!
//! The file is a fixed table of (address, value) cells, single threaded
//! (host tests and Kani run sequentially), with the last frame written by
//! `build_frame` and the console output of semihosting recorded for
//! assertions.

use core::cell::Cell;

const SLOTS: usize = 96;

struct Store {
    addrs: [Cell<u32>; SLOTS],
    values: [Cell<u32>; SLOTS],
    used: Cell<usize>,
    basepri: Cell<u8>,
    primask: Cell<bool>,
    frame_sp: Cell<u32>,
    frame: [Cell<u32>; 8],
    writes: Cell<u32>,
    exited: Cell<Option<u32>>,
}

// SAFETY (UJ-040): the simulated register file exists only in host and
// Kani builds, which are single threaded; no Flight_Build compiles it.
unsafe impl Sync for Store {}

const C: Cell<u32> = Cell::new(0);

static STORE: Store = Store {
    addrs: [C; SLOTS],
    values: [C; SLOTS],
    used: Cell::new(0),
    basepri: Cell::new(0),
    primask: Cell::new(false),
    frame_sp: Cell::new(0),
    frame: [C; 8],
    writes: Cell::new(0),
    exited: Cell::new(None),
};

fn slot(a: u32) -> Option<usize> {
    let n = STORE.used.get();
    let mut i = 0;
    while i < n {
        if STORE.addrs[i].get() == a {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Resets every simulated register (host tests).
pub fn reset() {
    STORE.used.set(0);
    STORE.basepri.set(0);
    STORE.primask.set(false);
    STORE.frame_sp.set(0);
    STORE.writes.set(0);
    STORE.exited.set(None);
    // A plausible nRF52840 identity for the boot checks.
    write(super::hw::addr::CPUID, 0x410F_C241);
    write(super::hw::addr::MPU_TYPE, 8 << 8);
    write(super::hw::addr::DWT_CTRL, 0);
}

pub fn read(a: u32) -> u32 {
    match slot(a) {
        Some(i) => STORE.values[i].get(),
        None => 0,
    }
}

pub fn write(a: u32, v: u32) {
    STORE.writes.set(STORE.writes.get().wrapping_add(1));
    match slot(a) {
        Some(i) => STORE.values[i].set(v),
        None => {
            let n = STORE.used.get();
            if n < SLOTS {
                STORE.addrs[n].set(a);
                STORE.values[n].set(v);
                STORE.used.set(n + 1);
            }
        }
    }
    // NVIC_IPR read-back models 3 implemented bits (ASM-03).
    if a == super::hw::addr::NVIC_IPR {
        if let Some(i) = slot(a) {
            STORE.values[i].set(v & 0xE0E0_E0E0);
        }
    }
}

pub fn write_u8(a: u32, v: u8) {
    let word = a & !3;
    let shift = (a & 3) * 8;
    let old = read(word);
    write(word, (old & !(0xFF << shift)) | ((v as u32) << shift));
}

pub fn set_basepri(v: u8) {
    STORE.basepri.set(v);
}

pub fn basepri() -> u8 {
    STORE.basepri.get()
}

pub fn set_primask(on: bool) {
    STORE.primask.set(on);
}

pub fn primask() -> bool {
    STORE.primask.get()
}

pub fn record_frame(sp: u32, words: [u32; 8]) {
    STORE.frame_sp.set(sp);
    let mut i = 0;
    while i < 8 {
        STORE.frame[i].set(words[i]);
        i += 1;
    }
}

pub fn last_frame() -> (u32, [u32; 8]) {
    let mut words = [0u32; 8];
    let mut i = 0;
    while i < 8 {
        words[i] = STORE.frame[i].get();
        i += 1;
    }
    (STORE.frame_sp.get(), words)
}

/// The simulated SVC instruction before `pc`: the low byte of the address
/// (host tests place the SVC number there).
pub fn svc_number_at(pc: u32) -> u8 {
    (pc & 0xFF) as u8
}

pub fn console(_s: &[u8]) {}

pub fn exit(code: u32) -> ! {
    STORE.exited.set(Some(code));
    #[cfg(test)]
    {
        panic!("semihosting exit {code}");
    }
    #[cfg(not(test))]
    {
        loop {}
    }
}

pub fn write_count() -> u32 {
    STORE.writes.get()
}
