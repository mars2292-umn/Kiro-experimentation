//! Partition MPU views (Property 12; R16.1 to R16.3, R16.5, R16.6, R20.4;
//! DD-07).
//!
//! A `Layout` is what the Generator computes and the Config_Checker
//! re-derives (task 10.4, R26.2): for each Partition one code interval, one
//! RAM interval with the stack at its lowest addresses, and up to five
//! peripheral windows; plus the shared code interval and the Kernel's flash
//! and RAM. Every interval has a power-of-two size of at least 32 bytes and
//! a base aligned to its size, the Armv7-M region rules (Armv7-M ARM
//! B3.5.8, MPU_RBAR; B3.5.9, MPU_RASR). `view` turns a Partition's
//! intervals into its eight MPU regions; subregion-disable bits only remove
//! eighths of a region (B3.5.5), so a region never grants more than its
//! interval.
//!
//! The hardware decides an unprivileged access by the highest-numbered
//! enabled region that contains the address (B3.5.3); `hw_model` proves
//! that whatever the hardware grants, some enabled region of the view
//! covers the address with that permission. Property 12 is then the three
//! lemmas at the end: an access granted by Partition p's view lies in one
//! of p's intervals; two Partitions' views never grant conflicting access
//! to one address, and no view grants access to Kernel memory
//! (`layout.wf()` makes the intervals disjoint); and the word below a
//! Partition's stack is granted to no writable region, so an overflow
//! faults before any memory outside the stack region changes (R16.5).
#![forbid(unsafe_code)]

use vstd::prelude::*;

verus! {

/// PAR-07: MPU regions on the Target.
pub const MPU_REGIONS: usize = 8;
/// Peripheral windows per Partition (DD-07).
pub const MAX_WINDOWS: usize = 5;

/// 2^k as a mathematical integer.
pub open spec fn pow2(k: nat) -> nat
    decreases k,
{
    if k == 0 { 1 } else { 2 * pow2((k - 1) as nat) }
}

/// pow2 is monotone.
pub proof fn lemma_pow2_mono(a: nat, b: nat)
    requires a <= b,
    ensures pow2(a) <= pow2(b),
    decreases b,
{
    if a < b {
        lemma_pow2_mono(a, (b - 1) as nat);
    }
}

/// pow2(k) <= 2^30 for k <= 30, so doubling stays within u32.
pub proof fn lemma_pow2_bound(k: nat)
    requires k <= 30,
    ensures pow2(k) <= 0x4000_0000,
{
    lemma_pow2_mono(k, 30);
    assert(pow2(30) == 0x4000_0000) by (compute);
}

pub proof fn lemma_pow2_pos(k: nat)
    ensures pow2(k) >= 1,
    decreases k,
{
    if k > 0 {
        lemma_pow2_pos((k - 1) as nat);
    }
}

/// 2^k as a `u32`, by a bounded loop (PR-16).
pub fn pow2_u32(k: u8) -> (r: u32)
    requires k <= 31,
    ensures r == pow2(k as nat),
{
    let mut r: u32 = 1;
    let mut i: u8 = 0;
    while i < k
        invariant i <= k, k <= 31, r == pow2(i as nat),
        decreases k - i,
    {
        proof {
            lemma_pow2_bound(i as nat);
        }
        r = r * 2;
        i = i + 1;
    }
    r
}

/// An aligned power-of-two interval: `[base, base + 2^size_log2)`.
#[derive(Clone, Copy)]
pub struct Interval {
    pub base: u32,
    /// 5 (32 bytes) to 31 (2 GiB).
    pub size_log2: u8,
}

impl Interval {
    pub open spec fn size(&self) -> int {
        pow2(self.size_log2 as nat) as int
    }

    pub open spec fn end(&self) -> int {
        self.base as int + self.size()
    }

    /// Armv7-M region rules: size at least 32 bytes, base aligned to the size.
    pub open spec fn rules_ok(&self) -> bool {
        &&& 5 <= self.size_log2 <= 31
        &&& (self.base as int) % self.size() == 0
        &&& self.end() <= 0x1_0000_0000
    }

    pub open spec fn contains(&self, a: int) -> bool {
        self.base as int <= a < self.end()
    }

    pub open spec fn disjoint(&self, other: &Interval) -> bool {
        self.end() <= other.base as int || other.end() <= self.base as int
    }

    /// Executable check of `rules_ok` (boot validation, R12.3).
    pub fn check_rules(&self) -> (ok: bool)
        ensures ok == self.rules_ok(),
    {
        if !(5 <= self.size_log2 && self.size_log2 <= 31) {
            return false;
        }
        let size = pow2_u32(self.size_log2);
        proof {
            lemma_pow2_pos(self.size_log2 as nat);
        }
        let aligned = self.base % size == 0;
        let fits = (self.base as u64) + (size as u64) <= 0x1_0000_0000u64;
        aligned && fits
    }

    /// Executable check of disjointness.
    pub fn check_disjoint(&self, other: &Interval) -> (d: bool)
        requires self.size_log2 <= 31, other.size_log2 <= 31,
        ensures d == self.disjoint(other),
    {
        let s = pow2_u32(self.size_log2) as u64;
        let o = pow2_u32(other.size_log2) as u64;
        (self.base as u64) + s <= other.base as u64 || (other.base as u64) + o <= self.base as u64
    }
}

/// One MPU region as the logic sees it; Kernel_Arch encodes it into
/// MPU_RBAR and MPU_RASR.
#[derive(Clone, Copy)]
pub struct Region {
    pub enabled: bool,
    pub interval: Interval,
    /// Subregion-disable bits, bit s for subregion s (meaningful only when
    /// the size is at least 256 bytes, B3.5.5).
    pub srd: u8,
    /// Unprivileged write permitted (AP = 3, full access) or not (AP = 2, read only).
    pub write: bool,
    /// Execute never.
    pub xn: bool,
    /// Strongly-ordered device memory (peripheral windows, DD-12).
    pub device: bool,
}

impl Region {
    pub open spec fn subregion_of(&self, a: int) -> int {
        (a - self.interval.base as int) / (self.interval.size() / 8)
    }

    pub open spec fn subregion_disabled(&self, s: int) -> bool {
        (self.srd >> (s as u8)) & 1 == 1
    }

    /// The region grants the address: enabled, in range, and (for regions
    /// of 256 bytes or more) its subregion is not disabled (B3.5.5).
    pub open spec fn covers(&self, a: int) -> bool {
        &&& self.enabled
        &&& self.interval.contains(a)
        &&& (self.interval.size_log2 < 8 || {
            let s = self.subregion_of(a);
            0 <= s < 8 && !self.subregion_disabled(s)
        })
    }

    pub open spec fn disabled_spec() -> Region {
        Region {
            enabled: false,
            interval: Interval { base: 0, size_log2: 5 },
            srd: 0,
            write: false,
            xn: true,
            device: false,
        }
    }

    pub fn disabled() -> (r: Region)
        ensures r == Region::disabled_spec(),
    {
        Region {
            enabled: false,
            interval: Interval { base: 0, size_log2: 5 },
            srd: 0,
            write: false,
            xn: true,
            device: false,
        }
    }
}

/// A Partition's eight MPU regions.
pub struct View {
    pub regions: [Region; MPU_REGIONS],
}

impl View {
    /// Over-approximation of the hardware grant: some enabled region
    /// covers the address with the permission (`hw_model::lemma_mpu_grant_covered`).
    pub open spec fn readable(&self, a: int) -> bool {
        exists|r: int| 0 <= r < MPU_REGIONS && self.regions[r].covers(a)
    }

    pub open spec fn writable(&self, a: int) -> bool {
        exists|r: int| 0 <= r < MPU_REGIONS && self.regions[r].covers(a) && self.regions[r].write
    }

    pub open spec fn executable(&self, a: int) -> bool {
        exists|r: int| 0 <= r < MPU_REGIONS && self.regions[r].covers(a) && !self.regions[r].xn
    }
}

/// The memory assignment of one Partition (DD-07).
#[derive(Clone, Copy)]
pub struct PartitionLayout {
    pub code: Interval,
    pub code_srd: u8,
    pub ram: Interval,
    pub ram_srd: u8,
    /// Stack bytes at the bottom of `ram` (R16.5).
    pub stack_size: u32,
    pub windows: [Interval; MAX_WINDOWS],
    pub window_count: u8,
}

impl PartitionLayout {
    pub open spec fn in_window(&self, a: int) -> bool {
        exists|w: int| 0 <= w < self.window_count && self.windows[w].contains(a)
    }

    pub open spec fn wf(&self) -> bool {
        &&& self.code.rules_ok()
        &&& self.ram.rules_ok()
        &&& self.ram.size_log2 >= 8
        &&& 0 < self.stack_size <= self.ram.size()
        &&& (self.window_count as int) <= MAX_WINDOWS
        &&& forall|w: int| 0 <= w < self.window_count ==> self.windows[w].rules_ok()
        &&& self.code.disjoint(&self.ram)
        &&& forall|w: int| 0 <= w < self.window_count ==> self.windows[w].disjoint(&self.ram) && self.windows[w].disjoint(&self.code)
        // R16.5: the word below the stack belongs to no writable interval of the Partition.
        &&& forall|w: int| 0 <= w < self.window_count ==> !self.windows[w].contains(self.ram.base as int - 1)
    }
}

/// The whole memory assignment.
pub struct Layout<const NP: usize> {
    pub shared_code: Interval,
    pub kernel_flash: Interval,
    pub kernel_ram: Interval,
    pub partitions: [PartitionLayout; NP],
}

impl<const NP: usize> Layout<NP> {
    pub open spec fn partition_ok(&self, p: int) -> bool {
        let pl = self.partitions[p];
        &&& pl.wf()
        &&& pl.code.disjoint(&self.kernel_flash) && pl.code.disjoint(&self.kernel_ram) && pl.code.disjoint(&self.shared_code)
        &&& pl.ram.disjoint(&self.kernel_flash) && pl.ram.disjoint(&self.kernel_ram) && pl.ram.disjoint(&self.shared_code)
        &&& forall|w: int| 0 <= w < pl.window_count ==> pl.windows[w].disjoint(&self.kernel_flash)
                && pl.windows[w].disjoint(&self.kernel_ram) && pl.windows[w].disjoint(&self.shared_code)
    }

    /// Two Partitions share nothing but the shared code (PR-28, PR-29).
    pub open spec fn pair_ok(&self, p: int, q: int) -> bool {
        let a = self.partitions[p];
        let b = self.partitions[q];
        &&& a.code.disjoint(&b.code) && a.code.disjoint(&b.ram) && a.ram.disjoint(&b.code) && a.ram.disjoint(&b.ram)
        &&& forall|w: int| 0 <= w < a.window_count ==> a.windows[w].disjoint(&b.ram) && a.windows[w].disjoint(&b.code)
        &&& forall|w: int| 0 <= w < b.window_count ==> b.windows[w].disjoint(&a.ram) && b.windows[w].disjoint(&a.code)
        &&& forall|v: int, w: int| 0 <= v < a.window_count && 0 <= w < b.window_count ==> a.windows[v].disjoint(&b.windows[w])
    }

    pub open spec fn wf(&self) -> bool {
        &&& self.shared_code.rules_ok() && self.kernel_flash.rules_ok() && self.kernel_ram.rules_ok()
        &&& self.shared_code.disjoint(&self.kernel_flash) && self.shared_code.disjoint(&self.kernel_ram)
        &&& self.kernel_flash.disjoint(&self.kernel_ram)
        &&& forall|p: int| 0 <= p < NP ==> self.partition_ok(p)
        &&& forall|p: int, q: int| 0 <= p < NP && 0 <= q < NP && p != q ==> self.pair_ok(p, q)
    }

    /// Executable check of `wf` (boot validation of the layout, R12.3).
    pub fn validate(&self) -> (ok: bool)
        ensures ok ==> self.wf(),
    {
        if !(self.shared_code.check_rules() && self.kernel_flash.check_rules() && self.kernel_ram.check_rules()) {
            return false;
        }
        if !(self.shared_code.check_disjoint(&self.kernel_flash) && self.shared_code.check_disjoint(&self.kernel_ram)
            && self.kernel_flash.check_disjoint(&self.kernel_ram))
        {
            return false;
        }
        let mut p: usize = 0;
        while p < NP
            invariant p <= NP,
                self.shared_code.rules_ok(), self.kernel_flash.rules_ok(), self.kernel_ram.rules_ok(),
                self.shared_code.disjoint(&self.kernel_flash), self.shared_code.disjoint(&self.kernel_ram),
                self.kernel_flash.disjoint(&self.kernel_ram),
                forall|i: int| 0 <= i < p ==> self.partition_ok(i),
                forall|i: int, j: int| 0 <= i < p && 0 <= j < p && i != j ==> self.pair_ok(i, j),
            decreases NP - p,
        {
            if !self.validate_partition(p) {
                return false;
            }
            let mut q: usize = 0;
            while q < p
                invariant q <= p, p < NP, self.partition_ok(p as int),
                    forall|i: int| 0 <= i < p ==> self.partition_ok(i),
                    forall|i: int, j: int| 0 <= i < p && 0 <= j < p && i != j ==> self.pair_ok(i, j),
                    forall|j: int| 0 <= j < q ==> self.pair_ok(p as int, j) && self.pair_ok(j, p as int),
                decreases p - q,
            {
                if !(self.validate_pair(p, q) && self.validate_pair(q, p)) {
                    return false;
                }
                q = q + 1;
            }
            p = p + 1;
        }
        true
    }

    fn validate_partition(&self, p: usize) -> (ok: bool)
        requires p < NP, self.shared_code.rules_ok(), self.kernel_flash.rules_ok(), self.kernel_ram.rules_ok(),
        ensures ok ==> self.partition_ok(p as int),
    {
        let pl = self.partitions[p];
        if !(pl.code.check_rules() && pl.ram.check_rules() && pl.ram.size_log2 >= 8) {
            return false;
        }
        let ram_size = pow2_u32(pl.ram.size_log2);
        if !(0 < pl.stack_size && pl.stack_size <= ram_size) {
            return false;
        }
        if (pl.window_count as usize) > MAX_WINDOWS {
            return false;
        }
        if !pl.code.check_disjoint(&pl.ram) {
            return false;
        }
        if !(pl.code.check_disjoint(&self.kernel_flash) && pl.code.check_disjoint(&self.kernel_ram) && pl.code.check_disjoint(&self.shared_code)
            && pl.ram.check_disjoint(&self.kernel_flash) && pl.ram.check_disjoint(&self.kernel_ram) && pl.ram.check_disjoint(&self.shared_code))
        {
            return false;
        }
        let count = pl.window_count as usize;
        let mut w: usize = 0;
        while w < count
            invariant w <= count, count <= MAX_WINDOWS, count == pl.window_count as int, pl == self.partitions[p as int], p < NP,
                pl.ram.rules_ok(), pl.ram.size_log2 >= 8, pl.code.rules_ok(),
                self.shared_code.rules_ok(), self.kernel_flash.rules_ok(), self.kernel_ram.rules_ok(),
                forall|i: int| 0 <= i < w ==> pl.windows[i].rules_ok() && pl.windows[i].disjoint(&pl.ram) && pl.windows[i].disjoint(&pl.code)
                    && !pl.windows[i].contains(pl.ram.base as int - 1)
                    && pl.windows[i].disjoint(&self.kernel_flash) && pl.windows[i].disjoint(&self.kernel_ram) && pl.windows[i].disjoint(&self.shared_code),
            decreases count - w,
        {
            let win = pl.windows[w];
            if !win.check_rules() {
                return false;
            }
            if !(win.check_disjoint(&pl.ram) && win.check_disjoint(&pl.code) && win.check_disjoint(&self.kernel_flash)
                && win.check_disjoint(&self.kernel_ram) && win.check_disjoint(&self.shared_code))
            {
                return false;
            }
            // The word below the stack: with the window disjoint from RAM, it is
            // inside the window only if the window ends exactly at the RAM base.
            let wsize = pow2_u32(win.size_log2) as u64;
            if (win.base as u64) + wsize == pl.ram.base as u64 {
                return false;
            }
            proof {
                assert(!win.contains(pl.ram.base as int - 1));
            }
            w = w + 1;
        }
        true
    }

    fn validate_pair(&self, p: usize, q: usize) -> (ok: bool)
        requires p < NP, q < NP,
        ensures ok ==> self.pair_ok(p as int, q as int),
    {
        let a = self.partitions[p];
        let b = self.partitions[q];
        if !(a.code.size_log2 <= 31 && a.ram.size_log2 <= 31 && b.code.size_log2 <= 31 && b.ram.size_log2 <= 31) {
            return false;
        }
        if !(a.code.check_disjoint(&b.code) && a.code.check_disjoint(&b.ram) && a.ram.check_disjoint(&b.code) && a.ram.check_disjoint(&b.ram)) {
            return false;
        }
        let ca = a.window_count as usize;
        let cb = b.window_count as usize;
        if ca > MAX_WINDOWS || cb > MAX_WINDOWS {
            return false;
        }
        let mut v: usize = 0;
        while v < ca
            invariant v <= ca, ca <= MAX_WINDOWS, cb <= MAX_WINDOWS, ca == a.window_count as int, cb == b.window_count as int,
                a == self.partitions[p as int], b == self.partitions[q as int], p < NP, q < NP,
                a.code.size_log2 <= 31, a.ram.size_log2 <= 31, b.code.size_log2 <= 31, b.ram.size_log2 <= 31,
                a.code.disjoint(&b.code), a.code.disjoint(&b.ram), a.ram.disjoint(&b.code), a.ram.disjoint(&b.ram),
                forall|i: int| 0 <= i < v ==> a.windows[i].disjoint(&b.ram) && a.windows[i].disjoint(&b.code)
                    && forall|j: int| 0 <= j < cb ==> a.windows[i].disjoint(&b.windows[j]),
            decreases ca - v,
        {
            let win = a.windows[v];
            if win.size_log2 > 31 {
                return false;
            }
            if !(win.check_disjoint(&b.ram) && win.check_disjoint(&b.code)) {
                return false;
            }
            let mut j: usize = 0;
            while j < cb
                invariant j <= cb, cb <= MAX_WINDOWS, cb == b.window_count as int, b == self.partitions[q as int], q < NP,
                    win == a.windows[v as int], win.size_log2 <= 31,
                    forall|k: int| 0 <= k < j ==> win.disjoint(&b.windows[k]),
                decreases cb - j,
            {
                let other = b.windows[j];
                if other.size_log2 > 31 {
                    return false;
                }
                if !win.check_disjoint(&other) {
                    return false;
                }
                j = j + 1;
            }
            v = v + 1;
        }
        // b's windows against a's RAM and code.
        let mut w: usize = 0;
        while w < cb
            invariant w <= cb, cb <= MAX_WINDOWS, ca <= MAX_WINDOWS, cb == b.window_count as int, ca == a.window_count as int,
                b == self.partitions[q as int], q < NP,
                a == self.partitions[p as int], p < NP, a.code.size_log2 <= 31, a.ram.size_log2 <= 31,
                a.code.disjoint(&b.code), a.code.disjoint(&b.ram), a.ram.disjoint(&b.code), a.ram.disjoint(&b.ram),
                forall|i: int| 0 <= i < ca ==> a.windows[i].disjoint(&b.ram) && a.windows[i].disjoint(&b.code)
                    && forall|j: int| 0 <= j < cb ==> a.windows[i].disjoint(&b.windows[j]),
                forall|j: int| 0 <= j < w ==> b.windows[j].disjoint(&a.ram) && b.windows[j].disjoint(&a.code),
            decreases cb - w,
        {
            let win = b.windows[w];
            if win.size_log2 > 31 {
                return false;
            }
            if !(win.check_disjoint(&a.ram) && win.check_disjoint(&a.code)) {
                return false;
            }
            w = w + 1;
        }
        true
    }

    /// Region `i` of Partition `p`'s view: region 0 shared code (R, X),
    /// region 1 its code (R, X), region 2 its RAM (RW, XN), regions 3 to 7
    /// its peripheral windows (RW, XN, device), unused regions disabled.
    pub open spec fn region_spec(&self, p: int, i: int) -> Region {
        let pl = self.partitions[p];
        if i == 0 {
            Region { enabled: true, interval: self.shared_code, srd: 0, write: false, xn: false, device: false }
        } else if i == 1 {
            Region { enabled: true, interval: pl.code, srd: pl.code_srd, write: false, xn: false, device: false }
        } else if i == 2 {
            Region { enabled: true, interval: pl.ram, srd: pl.ram_srd, write: true, xn: true, device: false }
        } else if i < 3 + pl.window_count {
            Region { enabled: true, interval: pl.windows[i - 3], srd: 0, write: true, xn: true, device: true }
        } else {
            Region::disabled_spec()
        }
    }

    /// Whether `v` is the view of Partition `p`.
    pub open spec fn is_view_of(&self, p: int, v: &View) -> bool {
        forall|i: int| 0 <= i < MPU_REGIONS ==> v.regions[i] == self.region_spec(p, i)
    }

    /// Computes the MPU view of Partition `p` (R16.1 to R16.3).
    pub fn view(&self, p: usize) -> (v: View)
        requires p < NP, self.wf(),
        ensures self.is_view_of(p as int, &v),
    {
        let pl = self.partitions[p];
        let mut regions = [Region::disabled(); MPU_REGIONS];
        regions[0] = Region { enabled: true, interval: self.shared_code, srd: 0, write: false, xn: false, device: false };
        regions[1] = Region { enabled: true, interval: pl.code, srd: pl.code_srd, write: false, xn: false, device: false };
        regions[2] = Region { enabled: true, interval: pl.ram, srd: pl.ram_srd, write: true, xn: true, device: false };
        proof {
            assert(self.partition_ok(p as int));
        }
        let count = pl.window_count as usize;
        let mut w: usize = 0;
        while w < count
            invariant p < NP, self.wf(), w <= count, count <= MAX_WINDOWS,
                pl == self.partitions[p as int], count == pl.window_count as int,
                forall|i: int| 0 <= i < 3 ==> regions[i] == self.region_spec(p as int, i),
                forall|i: int| 3 <= i < 3 + w ==> regions[i] == self.region_spec(p as int, i),
                forall|i: int| 3 + w <= i < MPU_REGIONS ==> regions[i] == Region::disabled_spec(),
            decreases count - w,
        {
            regions[3 + w] = Region { enabled: true, interval: pl.windows[w], srd: 0, write: true, xn: true, device: true };
            w = w + 1;
        }
        View { regions }
    }

    /// Property 12 (a): an address granted by Partition `p`'s view lies in
    /// one of `p`'s intervals or the shared code; a writable one lies in its
    /// RAM or a peripheral window; an executable one in its code or the
    /// shared code.
    pub proof fn lemma_view_within_layout(&self, p: int, v: &View, a: int)
        requires self.wf(), 0 <= p < NP, self.is_view_of(p, v),
        ensures
            v.readable(a) ==> self.shared_code.contains(a) || self.partitions[p].code.contains(a)
                || self.partitions[p].ram.contains(a) || self.partitions[p].in_window(a),
            v.writable(a) ==> self.partitions[p].ram.contains(a) || self.partitions[p].in_window(a),
            v.executable(a) ==> self.shared_code.contains(a) || self.partitions[p].code.contains(a),
    {
        let pl = self.partitions[p];
        if v.readable(a) {
            let r = choose|r: int| 0 <= r < MPU_REGIONS && v.regions[r].covers(a);
            assert(v.regions[r] == self.region_spec(p, r));
            if r >= 3 && r < 3 + pl.window_count {
                assert(pl.windows[r - 3].contains(a));
            }
        }
        if v.writable(a) {
            let r = choose|r: int| 0 <= r < MPU_REGIONS && v.regions[r].covers(a) && v.regions[r].write;
            assert(v.regions[r] == self.region_spec(p, r));
            if r >= 3 && r < 3 + pl.window_count {
                assert(pl.windows[r - 3].contains(a));
            }
        }
        if v.executable(a) {
            let r = choose|r: int| 0 <= r < MPU_REGIONS && v.regions[r].covers(a) && !v.regions[r].xn;
            assert(v.regions[r] == self.region_spec(p, r));
        }
    }

    /// Property 12 (b): no address is writable in one Partition's view and
    /// accessible in another's, and no view grants Kernel memory (R16.1,
    /// R16.2, R16.6).
    pub proof fn lemma_views_disjoint(&self, p: int, q: int, vp: &View, vq: &View, a: int)
        requires self.wf(), 0 <= p < NP, 0 <= q < NP, p != q, self.is_view_of(p, vp), self.is_view_of(q, vq),
        ensures
            !(vp.writable(a) && vq.readable(a)),
            !(vp.readable(a) && (self.kernel_flash.contains(a) || self.kernel_ram.contains(a))),
    {
        self.lemma_view_within_layout(p, vp, a);
        self.lemma_view_within_layout(q, vq, a);
        let pl = self.partitions[p];
        let ql = self.partitions[q];
        assert(self.pair_ok(p, q));
        assert(self.partition_ok(p));
        if vp.writable(a) && vq.readable(a) {
            // Where a lies on q's side.
            let in_q_window = ql.in_window(a);
            if in_q_window {
                let w = choose|w: int| 0 <= w < ql.window_count && ql.windows[w].contains(a);
                assert(ql.windows[w].disjoint(&pl.ram));
                if !pl.ram.contains(a) {
                    let v = choose|v: int| 0 <= v < pl.window_count && pl.windows[v].contains(a);
                    assert(pl.windows[v].disjoint(&ql.windows[w]));
                }
            } else {
                // a is in the shared code, q's code, or q's RAM.
                if pl.ram.contains(a) {
                    assert(pl.ram.disjoint(&self.shared_code));
                    assert(pl.ram.disjoint(&ql.code));
                    assert(pl.ram.disjoint(&ql.ram));
                } else {
                    let v = choose|v: int| 0 <= v < pl.window_count && pl.windows[v].contains(a);
                    assert(pl.windows[v].disjoint(&self.shared_code));
                    assert(pl.windows[v].disjoint(&ql.code));
                    assert(pl.windows[v].disjoint(&ql.ram));
                }
            }
            assert(false);
        }
        if vp.readable(a) && (self.kernel_flash.contains(a) || self.kernel_ram.contains(a)) {
            assert(self.shared_code.disjoint(&self.kernel_flash) && self.shared_code.disjoint(&self.kernel_ram));
            assert(pl.code.disjoint(&self.kernel_flash) && pl.code.disjoint(&self.kernel_ram));
            assert(pl.ram.disjoint(&self.kernel_flash) && pl.ram.disjoint(&self.kernel_ram));
            if pl.in_window(a) {
                let w = choose|w: int| 0 <= w < pl.window_count && pl.windows[w].contains(a);
                assert(pl.windows[w].disjoint(&self.kernel_flash) && pl.windows[w].disjoint(&self.kernel_ram));
            }
            assert(false);
        }
    }

    /// Property 12 (c), R16.5: the word below a Partition's stack is granted
    /// to no writable region of its view, so a stack overflow faults before
    /// it modifies memory outside the stack region.
    pub proof fn lemma_stack_overflow_faults(&self, p: int, v: &View)
        requires self.wf(), 0 <= p < NP, self.is_view_of(p, v),
        ensures !v.writable(self.partitions[p].ram.base as int - 1),
    {
        let a = self.partitions[p].ram.base as int - 1;
        self.lemma_view_within_layout(p, v, a);
        assert(self.partition_ok(p));
        if v.writable(a) {
            assert(!self.partitions[p].ram.contains(a));
            let w = choose|w: int| 0 <= w < self.partitions[p].window_count && self.partitions[p].windows[w].contains(a);
            assert(false);
        }
    }
}

} // verus!
