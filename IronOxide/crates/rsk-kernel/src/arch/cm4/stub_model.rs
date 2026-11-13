//! The assembly entry and exit stubs of DD-01, proven against a machine
//! model (Verification_Plan 5.17; 2026-11-13). Ghost code only: the module
//! exists under `cfg(verus_keep_ghost)` and no build compiles it.
//!
//! What is here:
//!
//! - A model of the processor state a stub touches (`M`): R0-R12, the main
//!   stack pointer, LR, PSP, CONTROL, IPSR, PRIMASK, FAULTMASK, the Z flag,
//!   S16-S31, and memory as a function of the address; and one function per
//!   instruction form the stubs use (`i_*`), which is that instruction's
//!   effect on the state.
//! - Each stub of `unsafe_module/stubs.rs` as the sequence of those
//!   functions, one per assembly line, in the order of the source
//!   (`irq_*`, `unwind_*`, `svc_*`, `mm_*`, `fault_*`, `timer_*`). The static
//!   check `cargo xtask check` (`checks::stub_model`) compares the two texts line by
//!   line, so the model cannot drift from the assembly unnoticed.
//! - A call of a Rust half (`call*`): what AAPCS leaves to the caller (R4
//!   to R11, SP, the memory at and above SP, S16 to S31), everything else
//!   arbitrary, and the result as the verified core's contract states it
//!   (`entry::irq_rel`, `entry::epi_rel`, `entry::kentry_rel`).
//! - The theorems: each stub passes its Rust half the arguments the core's
//!   contract is stated for; pushes and pops balance; a dispatch leaves the
//!   level's register record on the main stack where `rsk_unwind` later
//!   finds it; `rsk_unwind` pops exactly the records of the levels
//!   `epilogue_core` unwinds, calls it again with each plan it is handed
//!   until the plan is 0, and performs the exception return with the
//!   EXC_RETURN value, PSP and CONTROL of the context that level preempted
//!   and with R4 to R11 and S16 to S31 as that context left them; every
//!   return is made with PRIMASK clear and, from a dispatch-level stub,
//!   FAULTMASK set (5.8a); and the invariant `entry::arch_inv` with the
//!   main-stack layout (`world`) holds again at every exception return.
//!
//! What is assumed (the trust base of this proof, by inspection):
//!
//! - the instruction semantics of the `i_*` functions (Armv7-M ARM, the
//!   forms listed) and the assembler;
//! - the exception model: an exception's entry sets LR to the EXC_RETURN
//!   value of the interrupted context and stacks that context's frame on
//!   its own stack (`hw_entry`: HW-06), an exception return with a value
//!   resumes the context it names and unstacks its frame (HW-07), and
//!   handlers of one priority do not preempt each other (HW-01);
//! - that the compiled Rust halves follow AAPCS and write neither PRIMASK,
//!   FAULTMASK, PSP nor CONTROL, nor main-stack memory outside their own
//!   frames and their out-pointer (the SVC handler: the caller's frame);
//! - that the shells of `arch/cm4/mod.rs` and the `extern "C"` functions of
//!   `rsk-entry` call the verified core once and return its plan unchanged
//!   (they are safe Rust, outside `verus!`, and instrumented by features);
//! - 32-bit arithmetic does not wrap on the main stack pointer.

#![forbid(unsafe_code)]

use vstd::prelude::*;

use super::entry::{DispatchPlan, CONTROL_JOB, EXC_RETURN_THREAD_PSP, WINDOW_DEFER};
use super::plan::ArchState;
use crate::arch::bound::KernelOps;

verus! {

/// The processor state the stubs read and write.
pub ghost struct M {
    /// R0 to R12.
    pub r: Seq<int>,
    /// The main stack pointer (the stubs run in Handler mode).
    pub sp: int,
    pub lr: int,
    pub psp: int,
    pub control: int,
    pub ipsr: int,
    pub primask: bool,
    pub faultmask: bool,
    /// The Z flag.
    pub z: bool,
    /// S16 to S31.
    pub fp: Seq<int>,
    /// Memory, by byte address of a word.
    pub mem: spec_fn(int) -> int,
}

pub open spec fn wf(s: M) -> bool {
    s.r.len() == 13 && s.fp.len() == 16
}

/// Bit test on a 32-bit register value.
pub open spec fn has_bit(v: int, mask: u32) -> bool {
    (v as u32) & mask != 0
}

pub open spec fn setr(s: M, i: int, v: int) -> M {
    M { r: s.r.update(i, v), ..s }
}

pub open spec fn wr(s: M, a: int, v: int) -> M {
    M { mem: |x: int| if x == a { v } else { (s.mem)(x) }, ..s }
}

// ---- The instruction forms of the stubs.

pub open spec fn i_cpsid_i(s: M) -> M { M { primask: true, ..s } }
pub open spec fn i_cpsie_i(s: M) -> M { M { primask: false, ..s } }
pub open spec fn i_cpsid_f(s: M) -> M { M { faultmask: true, ..s } }
pub open spec fn i_dsb(s: M) -> M { s }
pub open spec fn i_isb(s: M) -> M { s }

/// `push {r3-r11, lr}`: the lowest register at the lowest address.
pub open spec fn i_push_r3_r11_lr(s: M) -> M {
    let b = s.sp - 40;
    M {
        sp: b,
        mem: |x: int|
            if x == b { s.r[3] } else if x == b + 4 { s.r[4] } else if x == b + 8 { s.r[5] }
            else if x == b + 12 { s.r[6] } else if x == b + 16 { s.r[7] } else if x == b + 20 { s.r[8] }
            else if x == b + 24 { s.r[9] } else if x == b + 28 { s.r[10] } else if x == b + 32 { s.r[11] }
            else if x == b + 36 { s.lr } else { (s.mem)(x) },
        ..s
    }
}

/// `pop {r3-r11, lr}`.
pub open spec fn i_pop_r3_r11_lr(s: M) -> M {
    let b = s.sp;
    M {
        sp: b + 40,
        lr: (s.mem)(b + 36),
        r: Seq::new(13, |i: int| if 3 <= i <= 11 { (s.mem)(b + 4 * (i - 3)) } else { s.r[i] }),
        ..s
    }
}

/// `push {r4, lr}`.
pub open spec fn i_push_r4_lr(s: M) -> M {
    let b = s.sp - 8;
    M { sp: b, mem: |x: int| if x == b { s.r[4] } else if x == b + 4 { s.lr } else { (s.mem)(x) }, ..s }
}

/// `pop {r4, lr}`.
pub open spec fn i_pop_r4_lr(s: M) -> M {
    M { sp: s.sp + 8, lr: (s.mem)(s.sp + 4), r: s.r.update(4, (s.mem)(s.sp)), ..s }
}

/// `tst lr, #mask`: Z is set when no tested bit is.
pub open spec fn i_tst_lr(s: M, mask: u32) -> M { M { z: !has_bit(s.lr, mask), ..s } }
/// `tst rN, #mask`.
pub open spec fn i_tst_r(s: M, n: int, mask: u32) -> M { M { z: !has_bit(s.r[n], mask), ..s } }

/// `vpush {s16-s31}`: sixteen words, S16 at the lowest address.
pub open spec fn vpush(s: M) -> M {
    let b = s.sp - 64;
    M { sp: b, mem: |x: int| if b <= x < b + 64 && (x - b) % 4 == 0 { s.fp[(x - b) / 4] } else { (s.mem)(x) }, ..s }
}

/// `vpop {s16-s31}`.
pub open spec fn vpop(s: M) -> M {
    M { sp: s.sp + 64, fp: Seq::new(16, |i: int| (s.mem)(s.sp + 4 * i)), ..s }
}

/// `it eq` / `vpusheq {s16-s31}`.
pub open spec fn i_vpush_eq(s: M) -> M { if s.z { vpush(s) } else { s } }
/// `it eq` / `vpopeq {s16-s31}`.
pub open spec fn i_vpop_eq(s: M) -> M { if s.z { vpop(s) } else { s } }

pub open spec fn i_mrs_ipsr(s: M, rd: int) -> M { setr(s, rd, s.ipsr) }
pub open spec fn i_mrs_psp(s: M, rd: int) -> M { setr(s, rd, s.psp) }
pub open spec fn i_mrs_control(s: M, rd: int) -> M { setr(s, rd, s.control) }
pub open spec fn i_mrs_primask(s: M, rd: int) -> M { setr(s, rd, if s.primask { 1 } else { 0 }) }
/// `mrseq rd, msp` under `ite eq`.
pub open spec fn i_mrs_msp_eq(s: M, rd: int) -> M { if s.z { setr(s, rd, s.sp) } else { s } }
/// `mrsne rd, psp`.
pub open spec fn i_mrs_psp_ne(s: M, rd: int) -> M { if !s.z { setr(s, rd, s.psp) } else { s } }
/// `moveq rd, #v`.
pub open spec fn i_mov_imm_eq(s: M, rd: int, v: int) -> M { if s.z { setr(s, rd, v) } else { s } }
/// `addeq rd, sp, #off`.
pub open spec fn i_add_r_sp_eq(s: M, rd: int, off: int) -> M { if s.z { setr(s, rd, s.sp + off) } else { s } }
/// `mov rd, lr`.
pub open spec fn i_mov_r_lr(s: M, rd: int) -> M { setr(s, rd, s.lr) }
/// `mov rd, rs`.
pub open spec fn i_mov(s: M, rd: int, rs: int) -> M { setr(s, rd, s.r[rs]) }
/// `mov rd, sp`.
pub open spec fn i_mov_r_sp(s: M, rd: int) -> M { setr(s, rd, s.sp) }
/// `sub sp, sp, #n`.
pub open spec fn i_sub_sp(s: M, n: int) -> M { M { sp: s.sp - n, ..s } }
/// `add sp, sp, #n`.
pub open spec fn i_add_sp(s: M, n: int) -> M { M { sp: s.sp + n, ..s } }
/// `add rd, sp, #off`.
pub open spec fn i_add_r_sp(s: M, rd: int, off: int) -> M { setr(s, rd, s.sp + off) }
/// `str rs, [sp, #off]`.
pub open spec fn i_str_sp(s: M, rs: int, off: int) -> M { wr(s, s.sp + off, s.r[rs]) }
/// `ldr rd, [sp, #off]`.
pub open spec fn i_ldr_sp(s: M, rd: int, off: int) -> M { setr(s, rd, (s.mem)(s.sp + off)) }
/// `msr psp, rs`.
pub open spec fn i_msr_psp(s: M, rs: int) -> M { M { psp: s.r[rs], ..s } }
/// `msr control, rs`.
pub open spec fn i_msr_control(s: M, rs: int) -> M { M { control: s.r[rs], ..s } }
/// `cmp rn, #v`.
pub open spec fn i_cmp_imm(s: M, rn: int, v: int) -> M { M { z: s.r[rn] == v, ..s } }

// ---- A call of a Rust half (`bl`).

/// AAPCS: the callee returns with R4 to R11, SP, S16 to S31 and the memory
/// at and above SP as they were; R0 to R3, R12, LR, the flags and the memory
/// below SP are arbitrary (`h`). The Rust halves write neither mask, PSP
/// nor CONTROL.
pub open spec fn call(s: M, h: M) -> M {
    M {
        r: Seq::new(13, |i: int| if 4 <= i <= 11 { s.r[i] } else { h.r[i] }),
        lr: h.lr,
        z: h.z,
        mem: |x: int| if x >= s.sp { (s.mem)(x) } else { (h.mem)(x) },
        ..s
    }
}

/// A call whose callee writes four result words through the pointer `out`.
pub open spec fn call_out4(s: M, h: M, out: int, v0: int, v1: int, v2: int, v3: int) -> M {
    wr(wr(wr(wr(call(s, h), out, v0), out + 4, v1), out + 8, v2), out + 12, v3)
}

/// A call whose callee returns `v` in R0.
pub open spec fn call_ret(s: M, h: M, v: int) -> M {
    setr(call(s, h), 0, v)
}

/// The SVC handler's call: the callee returns `v` in R0 and may write the
/// caller's exception frame (the results of the service), the eight words at
/// `frame`, when that frame is on the main stack (`on_msp`).
pub open spec fn call_svc(s: M, h: M, frame: int, on_msp: bool, v: int) -> M {
    let c = call(s, h);
    setr(M { mem: |x: int| if on_msp && frame <= x < frame + 32 { (h.mem)(x) } else { (c.mem)(x) }, ..c }, 0, v)
}

// ---- rsk_irq_entry.

/// From the entry instruction to the call of the Rust half.
pub open spec fn irq_to_call(s: M) -> M {
    let s = i_cpsid_i(s);
    let s = i_push_r3_r11_lr(s);
    let s = i_tst_lr(s, 0x10);
    let s = i_vpush_eq(s);
    let s = i_mrs_ipsr(s, 0);
    let s = i_mrs_psp(s, 1);
    let s = i_mrs_control(s, 2);
    let s = i_mov_r_lr(s, 3);
    let s = i_mov_r_lr(s, 8);
    let s = i_sub_sp(s, 24);
    let s = i_add_r_sp(s, 4, 8);
    let s = i_str_sp(s, 4, 0);
    // ctl: bl {entry}
    s
}

/// From the return of the Rust half to the test of the plan's frame.
pub open spec fn irq_after_call(s: M) -> M {
    let s = i_ldr_sp(s, 0, 8);
    let s = i_ldr_sp(s, 1, 12);
    let s = i_ldr_sp(s, 2, 16);
    let s = i_ldr_sp(s, 3, 20);
    let s = i_add_sp(s, 24);
    // ctl: cbz r0, 1f
    s
}

/// The dispatch path, to its exception return with R2.
pub open spec fn irq_dispatch(s: M) -> M {
    let s = i_msr_psp(s, 0);
    let s = i_msr_control(s, 1);
    let s = i_isb(s);
    let s = i_dsb(s);
    let s = i_cpsid_f(s);
    let s = i_cpsie_i(s);
    // ctl: bx r2
    s
}

/// The path without a dispatch, to the test of the unwind plan.
pub open spec fn irq_restore(s: M) -> M {
    // ctl: 1:
    let s = i_mov(s, 12, 3);
    let s = i_tst_r(s, 8, 0x10);
    let s = i_vpop_eq(s);
    let s = i_pop_r3_r11_lr(s);
    let s = i_cmp_imm(s, 12, 0);
    // ctl: bne 2f
    s
}

/// The return as entered, to its exception return with LR.
pub open spec fn irq_return(s: M) -> M {
    let s = i_dsb(s);
    let s = i_cpsid_f(s);
    let s = i_cpsie_i(s);
    // ctl: bx lr
    s
}

/// The branch to `rsk_unwind` with the plan in R0.
pub open spec fn irq_to_unwind(s: M) -> M {
    // ctl: 2:
    let s = i_mov(s, 0, 12);
    // ctl: b {unwind}
    s
}

// ---- rsk_unwind.

/// The entry instruction of `rsk_unwind`.
pub open spec fn unwind_entry(s: M) -> M {
    let s = i_cpsid_i(s);
    s
}

/// One round, from the loop's head to the call of the Rust half.
pub open spec fn unwind_to_call(s: M) -> M {
    // ctl: 1:
    let s = i_sub_sp(s, 16);
    let s = i_mov_r_sp(s, 1);
    // ctl: bl {epilogue}
    s
}

/// One round, from the return of the Rust half to the test of the next plan.
pub open spec fn unwind_after_call(s: M) -> M {
    let s = i_ldr_sp(s, 0, 0);
    let s = i_ldr_sp(s, 1, 4);
    let s = i_ldr_sp(s, 2, 8);
    let s = i_ldr_sp(s, 12, 12);
    let s = i_add_sp(s, 16);
    let s = i_tst_r(s, 2, 0x10);
    let s = i_vpop_eq(s);
    let s = i_pop_r3_r11_lr(s);
    let s = i_msr_psp(s, 0);
    let s = i_msr_control(s, 1);
    let s = i_isb(s);
    let s = i_cmp_imm(s, 12, 0);
    // ctl: bne 2f
    s
}

/// The exit, to its exception return with R2.
pub open spec fn unwind_exit(s: M) -> M {
    let s = i_dsb(s);
    let s = i_cpsid_f(s);
    let s = i_cpsie_i(s);
    // ctl: bx r2
    s
}

/// The branch back to the loop's head with the next plan in R0.
pub open spec fn unwind_again(s: M) -> M {
    // ctl: 2:
    let s = i_mov(s, 0, 12);
    // ctl: b 1b
    s
}

// ---- rsk_svc_entry.

pub open spec fn svc_to_call(s: M) -> M {
    let s = i_tst_lr(s, 4);
    let s = i_mrs_msp_eq(s, 0);
    let s = i_mrs_psp_ne(s, 0);
    let s = i_mov_r_lr(s, 1);
    let s = i_push_r4_lr(s);
    // ctl: bl {svc}
    s
}

/// The tail every Kernel-level handler shares: from the return of the Rust
/// half to the test of the plan.
pub open spec fn kentry_after_call(s: M) -> M {
    let s = i_pop_r4_lr(s);
    // ctl: cbz r0, 1f
    // ctl: b {unwind}
    s
}

/// The return as entered, to its exception return with LR.
pub open spec fn kentry_return(s: M) -> M {
    // ctl: 1:
    let s = i_dsb(s);
    // ctl: bx lr
    s
}

// ---- rsk_timer_entry.

pub open spec fn timer_to_call(s: M) -> M {
    let s = i_push_r4_lr(s);
    // ctl: bl {timer}
    s
}

// ---- rsk_memmanage_entry.

pub open spec fn mm_to_call(s: M) -> M {
    let s = i_push_r4_lr(s);
    let s = i_mrs_primask(s, 0);
    let s = i_tst_lr(s, 8);
    let s = i_mov_imm_eq(s, 0, 1);
    let s = i_tst_lr(s, 4);
    let s = i_mrs_psp_ne(s, 1);
    let s = i_add_r_sp_eq(s, 1, 8);
    let s = i_mov_r_lr(s, 2);
    // ctl: bl {f}
    s
}

// ---- rsk_busfault_entry and rsk_usagefault_entry.

pub open spec fn fault_to_call(s: M) -> M {
    let s = i_push_r4_lr(s);
    let s = i_mrs_primask(s, 0);
    let s = i_tst_lr(s, 8);
    let s = i_mov_imm_eq(s, 0, 1);
    // ctl: bl {f}
    s
}

// ---- The main stack: the register records of the levels.

/// What a dispatch stub leaves on the main stack for its level: R3 to R11
/// and LR, and below them S16 to S31 when the preempted context had a
/// floating-point context; `gap` is the room between this record and the
/// next older one (the frame the processor stacked on the main stack when
/// the level preempted the idle activity; 0 above a Job).
pub ghost struct Frame {
    pub has_fp: bool,
    pub regs: Seq<int>,
    pub fpv: Seq<int>,
    pub gap: int,
}

pub open spec fn frame_size(f: Frame) -> int {
    if f.has_fp { 104 } else { 40 }
}

/// The record `f` is the one at the top of the main stack `sp`.
pub open spec fn frame_top(mem: spec_fn(int) -> int, sp: int, f: Frame) -> bool {
    let b = if f.has_fp { sp + 64 } else { sp };
    &&& f.regs.len() == 10
    &&& forall|i: int| 0 <= i < 10 ==> mem(b + 4 * i) == #[trigger] f.regs[i]
    &&& f.has_fp ==> f.fpv.len() == 16 && forall|i: int| 0 <= i < 16 ==> mem(sp + 4 * i) == #[trigger] f.fpv[i]
    &&& f.gap >= 0
}

/// The records `fs` lie on the main stack from `sp` upward, the last one
/// at the top.
pub open spec fn frames_at(mem: spec_fn(int) -> int, sp: int, fs: Seq<Frame>) -> bool
    decreases fs.len(),
{
    if fs.len() == 0 {
        true
    } else {
        frame_top(mem, sp, fs.last()) && frames_at(mem, sp + frame_size(fs.last()) + fs.last().gap, fs.drop_last())
    }
}

/// The register record `i` belongs to chain entry `i`: its shape follows
/// the EXC_RETURN value the level record keeps, which is the LR it saved;
/// and every level but the first preempted a Job, in Thread mode on the
/// process stack, with nothing stacked on the main stack between the two
/// records.
pub open spec fn frame_rec(state: ArchState, fs: Seq<Frame>, i: int) -> bool {
    let rec = state.levels[state.chain_levels@[i]];
    &&& fs[i].has_fp == !has_bit(rec.exc_return as int, 0x10)
    &&& fs[i].regs.len() == 10
    &&& fs[i].regs[9] == rec.exc_return as int
    &&& i > 0 ==> fs[i].gap == 0 && has_bit(rec.exc_return as int, 0x8) && has_bit(rec.exc_return as int, 0x4)
}

pub open spec fn frames_match(state: ArchState, fs: Seq<Frame>) -> bool {
    &&& fs.len() == state.chain_levels@.len()
    &&& forall|i: int| 0 <= i < fs.len() ==> #[trigger] frame_rec(state, fs, i)
}

/// The main stack and the level records agree: the register records of
/// the chain's entries lie from `base` upward, and while there is one the
/// Thread-mode context is a Job's, on the process stack (`tpsp`).
pub open spec fn world(state: ArchState, mem: spec_fn(int) -> int, base: int, fs: Seq<Frame>, tpsp: bool) -> bool {
    &&& frames_at(mem, base, fs)
    &&& frames_match(state, fs)
    &&& fs.len() > 0 ==> tpsp
}

/// The exception model at a handler's entry (HW-06): LR is the EXC_RETURN
/// value of the interrupted context. Taken from Thread mode (bit 3), bit 2
/// tells whether that context is on the process stack; its frame was then
/// stacked there and nothing on the main stack (`g`, the bytes the entry
/// stacked on the main stack, is 0); a frame stacked on the main stack is
/// at least the eight words of a basic frame.
pub open spec fn hw_entry(lr: int, g: int, tpsp: bool) -> bool {
    &&& g >= 0
    &&& has_bit(lr, 0x8) ==> (has_bit(lr, 0x4) <==> tpsp)
    &&& has_bit(lr, 0x8) && has_bit(lr, 0x4) ==> g == 0
    &&& !has_bit(lr, 0x4) ==> g >= 32
}

/// The register record a dispatch stub entered in state `s` pushes.
pub open spec fn entry_frame(s: M, g: int) -> Frame {
    Frame {
        has_fp: !has_bit(s.lr, 0x10),
        regs: seq![s.r[3], s.r[4], s.r[5], s.r[6], s.r[7], s.r[8], s.r[9], s.r[10], s.r[11], s.lr],
        fpv: s.fp,
        gap: g,
    }
}

/// Memory that agrees at and above `sp` holds the same records there.
pub proof fn lemma_frames_ext(m1: spec_fn(int) -> int, m2: spec_fn(int) -> int, sp: int, fs: Seq<Frame>)
    requires
        forall|x: int| x >= sp ==> m1(x) == #[trigger] m2(x),
        frames_at(m1, sp, fs),
    ensures frames_at(m2, sp, fs),
    decreases fs.len(),
{
    if fs.len() > 0 {
        let f = fs.last();
        let b = if f.has_fp { sp + 64 } else { sp };
        assert forall|i: int| 0 <= i < 10 implies m2(b + 4 * i) == #[trigger] f.regs[i] by {
            assert(m1(b + 4 * i) == m2(b + 4 * i));
        }
        if f.has_fp {
            assert forall|i: int| 0 <= i < 16 implies m2(sp + 4 * i) == #[trigger] f.fpv[i] by {
                assert(m1(sp + 4 * i) == m2(sp + 4 * i));
            }
        }
        lemma_frames_ext(m1, m2, sp + frame_size(f) + f.gap, fs.drop_last());
    }
}

/// Records that are the same for the chain's entries match the same frames.
pub proof fn lemma_match_same(s0: ArchState, s1: ArchState, fs: Seq<Frame>)
    requires frames_match(s0, fs), super::entry::records_same(s1, s0),
    ensures frames_match(s1, fs),
{
    assert forall|i: int| 0 <= i < fs.len() implies #[trigger] frame_rec(s1, fs, i) by {
        assert(frame_rec(s0, fs, i));
    }
}

// ---- rsk_irq_entry: the machine.

/// The state after the Rust half returned `p` (frame address, CONTROL,
/// EXC_RETURN, unwind plan) through the out-pointer the stub passed.
pub open spec fn irq_called(s: M, h: M, p: (int, int, int, int)) -> M {
    let c = irq_to_call(s);
    irq_after_call(call_out4(c, h, (c.mem)(c.sp), p.0, p.1, p.2, p.3))
}

/// The stub's call: the arguments of `irq_entry_core`, PRIMASK held, and
/// the out-pointer inside the stub's own stack room.
pub proof fn lemma_irq_call(s: M)
    requires wf(s),
    ensures ({
        let c = irq_to_call(s);
        &&& wf(c)
        &&& c.primask
        &&& c.r[0] == s.ipsr && c.r[1] == s.psp && c.r[2] == s.control && c.r[3] == s.lr
        &&& (c.mem)(c.sp) == c.sp + 8
        &&& c.sp == s.sp - frame_size(entry_frame(s, 0)) - 24
    }),
{
}

/// The state after the call: the plan in R0 to R3, the level's register
/// record on top of the main stack, everything above the entry's stack
/// pointer untouched.
pub proof fn lemma_irq_called(s: M, h: M, p: (int, int, int, int), g: int)
    requires wf(s), g >= 0,
    ensures ({
        let t = irq_called(s, h, p);
        &&& wf(t)
        &&& t.r[0] == p.0 && t.r[1] == p.1 && t.r[2] == p.2 && t.r[3] == p.3
        &&& t.r[8] == s.lr
        &&& t.sp == s.sp - frame_size(entry_frame(s, g))
        &&& frame_top(t.mem, t.sp, entry_frame(s, g))
        &&& forall|x: int| x >= s.sp ==> (s.mem)(x) == #[trigger] (t.mem)(x)
        &&& t.fp == s.fp && t.primask && t.psp == s.psp && t.control == s.control && t.faultmask == s.faultmask
    }),
{
    let c = irq_to_call(s);
    let t = irq_called(s, h, p);
    let f = entry_frame(s, g);
    let b = if f.has_fp { t.sp + 64 } else { t.sp };
    assert(b == s.sp - 40);
    assert forall|i: int| 0 <= i < 10 implies (t.mem)(b + 4 * i) == #[trigger] f.regs[i] by {
        assert(i == 0 || i == 1 || i == 2 || i == 3 || i == 4 || i == 5 || i == 6 || i == 7 || i == 8 || i == 9);
    }
    if f.has_fp {
        assert forall|i: int| 0 <= i < 16 implies (t.mem)(t.sp + 4 * i) == #[trigger] f.fpv[i] by {
            assert((4 * i) % 4 == 0 && (4 * i) / 4 == i);
        }
    }
}

/// The path without a dispatch restores what the stub pushed.
pub proof fn lemma_irq_restore(s: M, h: M, p: (int, int, int, int))
    requires wf(s),
    ensures ({
        let u = irq_restore(irq_called(s, h, p));
        &&& wf(u)
        &&& u.sp == s.sp && u.lr == s.lr && u.fp =~= s.fp && u.psp == s.psp && u.control == s.control
        &&& forall|i: int| 3 <= i <= 11 ==> u.r[i] == s.r[i]
        &&& u.r[12] == p.3 && u.z == (p.3 == 0) && u.primask && u.faultmask == s.faultmask
        &&& forall|x: int| x >= s.sp ==> (s.mem)(x) == #[trigger] (u.mem)(x)
    }),
{
    lemma_irq_called(s, h, p, 0);
    let t = irq_called(s, h, p);
    let f = entry_frame(s, 0);
    let u = irq_restore(t);
    let b = if f.has_fp { t.sp + 64 } else { t.sp };
    assert forall|i: int| 3 <= i <= 11 implies u.r[i] == s.r[i] by {
        assert((t.mem)(b + 4 * (i - 3)) == f.regs[i - 3]);
    }
    assert((t.mem)(b + 4 * 9) == f.regs[9]);
    if f.has_fp {
        assert forall|i: int| 0 <= i < 16 implies u.fp[i] == s.fp[i] by {
            assert((t.mem)(t.sp + 4 * i) == f.fpv[i]);
        }
    }
}

// ---- rsk_irq_entry: the theorem.

/// The dispatch stub. Entered (at its `cpsid i`) in state `s` with the
/// main stack and the records in agreement, its Rust half answering as
/// `irq_entry_core`'s contract states (`irq_rel`; a safe state does not
/// return): the stub calls with IPSR, PSP, CONTROL and its LR under
/// PRIMASK; on a dispatch it installs the plan's PSP and CONTROL and
/// returns to unprivileged Thread mode on the process stack with the
/// level's register record left on the main stack; without one it restores
/// every register it pushed and either returns as entered or reaches
/// `rsk_unwind` with the plan in R0 and the topmost record at the stack's
/// top; and the agreement (`world`) holds again in each case.
pub proof fn theorem_irq_stub<K: KernelOps>(s: M, h: M, k1: &K, s0: ArchState, s1: ArchState, plan: DispatchPlan, fs: Seq<Frame>, g: int, tpsp: bool)
    requires
        wf(s), 0 <= s.lr <= 0xFFFF_FFFF,
        world(s0, s.mem, s.sp + g, fs, tpsp),
        hw_entry(s.lr, g, tpsp),
        k1.inv(),
        super::entry::irq_rel(k1, s0, s1, s.lr as u32, plan),
        WINDOW_DEFER,
    ensures ({
        let c = irq_to_call(s);
        let t = irq_called(s, h, (plan.psp as int, plan.control as int, plan.exc_return as int, plan.hop as int));
        let u = irq_restore(t);
        &&& c.primask && c.r[0] == s.ipsr && c.r[1] == s.psp && c.r[2] == s.control && c.r[3] == s.lr
        &&& (t.r[0] == 0) == (plan.psp == 0)
        &&& plan.psp != 0 ==> ({
            let d = irq_dispatch(t);
            &&& d.psp == plan.psp && d.control == CONTROL_JOB && d.r[2] == EXC_RETURN_THREAD_PSP
            &&& !d.primask && d.faultmask
            &&& world(s1, d.mem, d.sp, fs.push(entry_frame(s, g)), true)
            &&& super::entry::arch_inv(k1, s1, 0)
        })
        &&& plan.psp == 0 ==> ({
            &&& u.sp == s.sp && u.lr == s.lr && u.fp =~= s.fp && u.psp == s.psp && u.control == s.control
            &&& forall|i: int| 3 <= i <= 11 ==> u.r[i] == s.r[i]
            &&& u.z == (plan.hop == 0)
        })
        &&& plan.psp == 0 && plan.hop == 0 ==> ({
            let f = irq_return(u);
            &&& !f.primask && f.faultmask && f.sp == s.sp && f.lr == s.lr
            &&& world(s1, f.mem, f.sp + g, fs, tpsp)
            &&& super::entry::arch_inv(k1, s1, 0)
        })
        &&& plan.psp == 0 && plan.hop != 0 ==> ({
            let w = irq_to_unwind(u);
            &&& wf(w) && w.r[0] == plan.hop && w.sp == s.sp && w.primask
            &&& world(s1, w.mem, w.sp, fs, tpsp)
            &&& super::entry::arch_inv(k1, s1, plan.hop as int)
        })
    }),
{
    let p = (plan.psp as int, plan.control as int, plan.exc_return as int, plan.hop as int);
    lemma_irq_call(s);
    lemma_irq_called(s, h, p, g);
    lemma_irq_restore(s, h, p);
    let t = irq_called(s, h, p);
    let u = irq_restore(t);
    k1.lemma_stack_rising();
    if plan.psp != 0 {
        let d = irq_dispatch(t);
        let f = entry_frame(s, g);
        let fs1 = fs.push(f);
        let m = fs.len() as int;
        // The entry was taken from Thread mode (a Handler-mode entry does not dispatch).
        assert(has_bit(s.lr, 0x8));
        reveal(super::entry::chain_ok);
        assert(s1.chain_levels@.len() == m + 1);
        assert(fs1.drop_last() =~= fs);
        lemma_frames_ext(s.mem, d.mem, s.sp + g, fs);
        assert forall|i: int| 0 <= i < fs1.len() implies #[trigger] frame_rec(s1, fs1, i) by {
            super::entry::lemma_link(k1, s1, i);
            if i < m {
                assert(frame_rec(s0, fs, i));
                assert(fs1[i] == fs[i]);
                assert(s1.chain_levels@.drop_last()[i] == s1.chain_levels@[i]);
                assert(s1.chain_levels@[i] < s1.chain_levels@[m]);
                assert(s1.levels[s1.chain_levels@[i]] == s0.levels[s0.chain_levels@[i]]);
            } else {
                assert(fs1[i] == f);
                assert(s1.chain_levels@[i] == s1.chain_levels@.last());
                assert(s1.levels[s1.chain_levels@[i]].exc_return as int == s.lr);
                assert(f.regs[9] == s.lr);
            }
        }
    } else {
        lemma_match_same(s0, s1, fs);
        if plan.hop == 0 {
            let f = irq_return(u);
            lemma_frames_ext(s.mem, f.mem, s.sp + g, fs);
        } else {
            let w = irq_to_unwind(u);
            super::entry::lemma_plan_number(k1, s1, plan.hop as int);
            super::entry::lemma_chain_len(k1, s1);
            assert(fs.len() > 0);
            assert(has_bit(s.lr, 0x8));
            assert(g == 0);
            lemma_frames_ext(s.mem, w.mem, s.sp, fs);
        }
    }
}

// ---- rsk_unwind: the machine.

/// One round of the loop with the Rust half answering `e` (PSP, CONTROL,
/// EXC_RETURN, next plan).
pub open spec fn unwind_round(m: M, h: M, e: (u32, u32, u32, u32)) -> M {
    let a = unwind_to_call(m);
    unwind_after_call(call_out4(a, h, a.r[1], e.0 as int, e.1 as int, e.2 as int, e.3 as int))
}

/// The loop: a round per answer, until a round's next plan is 0.
pub open spec fn unwind_loop(m: M, hs: Seq<M>, es: Seq<(u32, u32, u32, u32)>) -> M
    decreases es.len(),
{
    if es.len() == 0 {
        m
    } else {
        let t = unwind_round(m, hs[0], es[0]);
        if t.z { unwind_exit(t) } else { unwind_loop(unwind_again(t), hs.drop_first(), es.drop_first()) }
    }
}

/// One round pops exactly the record at the top of the main stack when the
/// EXC_RETURN value the Rust half returns is the one that record was
/// pushed under.
pub proof fn lemma_unwind_round(m: M, h: M, e: (u32, u32, u32, u32), f: Frame)
    requires wf(m), frame_top(m.mem, m.sp, f), f.has_fp == !has_bit(e.2 as int, 0x10),
    ensures ({
        let a = unwind_to_call(m);
        let t = unwind_round(m, h, e);
        &&& a.r[0] == m.r[0] && a.r[1] == a.sp && a.primask == m.primask
        &&& wf(t)
        &&& t.sp == m.sp + frame_size(f)
        &&& forall|i: int| 3 <= i <= 11 ==> t.r[i] == f.regs[i - 3]
        &&& t.lr == f.regs[9]
        &&& f.has_fp ==> t.fp =~= f.fpv
        &&& t.psp == e.0 && t.control == e.1 && t.r[2] == e.2 && t.r[12] == e.3
        &&& t.z == (e.3 == 0)
        &&& t.primask == m.primask && t.faultmask == m.faultmask
        &&& forall|x: int| x >= m.sp ==> (m.mem)(x) == #[trigger] (t.mem)(x)
    }),
{
    let t = unwind_round(m, h, e);
    let b = if f.has_fp { m.sp + 64 } else { m.sp };
    assert forall|i: int| 3 <= i <= 11 implies t.r[i] == f.regs[i - 3] by {
        assert((m.mem)(b + 4 * (i - 3)) == f.regs[i - 3]);
    }
    assert((m.mem)(b + 4 * 9) == f.regs[9]);
    if f.has_fp {
        assert forall|i: int| 0 <= i < 16 implies t.fp[i] == f.fpv[i] by {
            assert((m.mem)(m.sp + 4 * i) == f.fpv[i]);
        }
    }
}

// ---- rsk_unwind: the theorem.

/// Round `i` of an unwind: called with the plan the stub holds (`n` in the
/// first round, the previous answer's next plan after it) in a state that
/// satisfies `epilogue_core`'s `requires`, the Rust half returns without
/// the safe state and its answer satisfies `epilogue_core`'s `ensures`.
pub open spec fn epi_step<K: KernelOps>(ks: Seq<K>, ss: Seq<ArchState>, es: Seq<(u32, u32, u32, u32)>, n: int, i: int) -> bool {
    let arg = if i == 0 { n } else { es[i - 1].3 as int };
    ks[i].inv() && ((arg as u32) & 0x1FF) != 0 && super::entry::arch_inv(&ks[i], ss[i], ((arg as u32) & 0x1FF) as int)
        ==> ks[i + 1].inv() && super::entry::epi_rel(&ks[i + 1], ss[i], ss[i + 1], es[i])
}

/// `rsk_unwind`. Entered at its loop's head with the plan `n` in R0, the
/// invariant in the form that plan left, and the main stack in agreement
/// with the records: in every round the stub calls `epilogue_core` with the
/// plan it holds, which satisfies that core's `requires`; it pops exactly
/// the register record of the level the core unwound; it goes round again
/// exactly while the core names a next level; and it leaves by an exception
/// return with the EXC_RETURN value the last unwound level saved at its
/// dispatch, the PSP and CONTROL of the context that level preempted, R3 to
/// R11 (and S16 to S31) as that dispatch pushed them, PRIMASK clear and
/// FAULTMASK set. Then nothing is owed, the level records are those of the
/// Kernel's preemption stack (`arch_matches`), and the main stack holds the
/// register records of exactly the remaining levels.
pub proof fn theorem_unwind<K: KernelOps>(m: M, n: int, ks: Seq<K>, ss: Seq<ArchState>, es: Seq<(u32, u32, u32, u32)>, hs: Seq<M>, fs: Seq<Frame>, tpsp: bool)
    requires
        wf(m), es.len() >= 1, ks.len() == es.len() + 1, ss.len() == es.len() + 1, hs.len() == es.len(),
        m.r[0] == n, n != 0, m.primask,
        ks[0].inv(), super::entry::arch_inv(&ks[0], ss[0], n),
        world(ss[0], m.mem, m.sp, fs, tpsp),
        forall|i: int| 0 <= i < es.len() ==> #[trigger] epi_step(ks, ss, es, n, i),
        forall|i: int| 0 <= i < es.len() - 1 ==> (#[trigger] es[i]).3 != 0,
        es.last().3 == 0,
    ensures ({
        let j = es.len() as int;
        let f = unwind_loop(m, hs, es);
        &&& fs.len() >= j
        &&& !f.primask && f.faultmask
        &&& f.psp == es.last().0 && f.control == es.last().1 && f.r[2] == es.last().2
        &&& f.r[2] == fs[fs.len() - j].regs[9] && f.lr == fs[fs.len() - j].regs[9]
        &&& forall|i: int| 3 <= i <= 11 ==> f.r[i] == fs[fs.len() - j].regs[i - 3]
        &&& fs[fs.len() - j].has_fp ==> f.fp =~= fs[fs.len() - j].fpv
        &&& world(ss[j], f.mem, f.sp + fs[fs.len() - j].gap, fs.subrange(0, fs.len() - j),
                has_bit(es.last().2 as int, 0x8) && has_bit(es.last().2 as int, 0x4))
        &&& super::entry::arch_inv(&ks[j], ss[j], 0) && ss[j].pending_unwind == 0
        &&& super::entry::arch_matches(&ks[j], ss[j])
    }),
    decreases es.len(),
{
    let e = es[0];
    let len = fs.len() as int;
    // The plan satisfies the core's `requires`.
    super::entry::lemma_plan_number(&ks[0], ss[0], n);
    assert(epi_step(ks, ss, es, n, 0));
    assert(super::entry::epi_rel(&ks[1], ss[0], ss[1], e));
    reveal(super::entry::chain_ok);
    super::entry::lemma_chain_len(&ks[0], ss[0]);
    assert(len >= 1);
    let top = fs.last();
    assert(frame_rec(ss[0], fs, len - 1));
    lemma_unwind_round(m, hs[0], e, top);
    let t = unwind_round(m, hs[0], e);
    let rest = fs.drop_last();
    // The remaining records still match: no other level record changed.
    assert forall|i: int| 0 <= i < rest.len() implies #[trigger] frame_rec(ss[1], rest, i) by {
        assert(frame_rec(ss[0], fs, i));
        super::entry::lemma_link(&ks[0], ss[0], i);
        assert(rest[i] == fs[i]);
        assert(ss[0].chain_levels@[i] < ss[0].chain_levels@[len - 1]);
        assert(ss[1].chain_levels@[i] == ss[0].chain_levels@.drop_last()[i]);
        assert(ss[1].levels[ss[1].chain_levels@[i]] == ss[0].levels[ss[0].chain_levels@[i]]);
    }
    lemma_frames_ext(m.mem, t.mem, m.sp + frame_size(top) + top.gap, rest);
    let tp = has_bit(e.2 as int, 0x8) && has_bit(e.2 as int, 0x4);
    assert(world(ss[1], t.mem, t.sp + top.gap, rest, tp));
    assert(rest =~= fs.subrange(0, len - 1));
    if es.len() == 1 {
        assert(es.last() == e);
        assert(t.z);
    } else {
        assert(e.3 != 0);
        let m2 = unwind_again(t);
        let ks2 = ks.drop_first();
        let ss2 = ss.drop_first();
        let es2 = es.drop_first();
        let hs2 = hs.drop_first();
        // A further level is named only above another record: no gap.
        super::entry::lemma_plan_number(&ks[1], ss[1], e.3 as int);
        super::entry::lemma_chain_len(&ks[1], ss[1]);
        assert(len - 1 >= 1);
        assert(top.gap == 0);
        assert forall|i: int| 0 <= i < es2.len() implies #[trigger] epi_step(ks2, ss2, es2, e.3 as int, i) by {
            assert(epi_step(ks, ss, es, n, i + 1));
        }
        assert forall|i: int| 0 <= i < es2.len() - 1 implies (#[trigger] es2[i]).3 != 0 by {
            assert(es[i + 1].3 != 0);
        }
        assert(es2.last() == es.last());
        theorem_unwind(m2, e.3 as int, ks2, ss2, es2, hs2, rest, tp);
        let j = es.len() as int;
        assert(rest.subrange(0, rest.len() - (j - 1)) =~= fs.subrange(0, len - j));
        assert(rest[rest.len() - (j - 1)] == fs[len - j]);
        assert(ks2[j - 1] == ks[j]);
        assert(ss2[j - 1] == ss[j]);
    }
}

// ---- The Kernel-level handlers: SVC, faults, the timer.

/// What every Kernel-level handler's stub does after its Rust half returned
/// the plan `hop` (`kentry_rel`): it restores R4 and LR; with no plan it
/// returns as entered, the agreement intact; with one it reaches
/// `rsk_unwind` with the plan in R0 and the topmost record at the stack's
/// top. `c` is the state at the call, `t` the state the call returned,
/// `s` the handler's entry state.
pub proof fn lemma_kentry_tail<K: KernelOps>(s: M, c: M, t: M, k1: &K, s0: ArchState, s1: ArchState, hop: int, fs: Seq<Frame>, g: int, tpsp: bool)
    requires
        wf(s), wf(t),
        world(s0, s.mem, s.sp + g, fs, tpsp),
        hw_entry(s.lr, g, tpsp),
        k1.inv(),
        super::entry::kentry_rel(k1, s0, s1, hop),
        // A plan is handed only to a handler entered from Thread mode.
        hop != 0 ==> has_bit(s.lr, 0x8),
        // The call: R4 and LR pushed at the entry's stack pointer, the plan
        // in R0, memory at and above the records untouched.
        c.sp == s.sp - 8 && (c.mem)(c.sp) == s.r[4] && (c.mem)(c.sp + 4) == s.lr,
        t.sp == c.sp && t.r[0] == hop && t.fp == s.fp && t.psp == s.psp && t.control == s.control,
        (t.mem)(c.sp) == (c.mem)(c.sp) && (t.mem)(c.sp + 4) == (c.mem)(c.sp + 4),
        forall|i: int| 5 <= i <= 11 ==> t.r[i] == s.r[i],
        forall|x: int| x >= s.sp + g ==> (s.mem)(x) == #[trigger] (t.mem)(x),
    ensures ({
        let u = kentry_after_call(t);
        &&& wf(u)
        &&& u.sp == s.sp && u.lr == s.lr && u.fp == s.fp && u.psp == s.psp && u.control == s.control
        &&& forall|i: int| 4 <= i <= 11 ==> u.r[i] == s.r[i]
        &&& u.r[0] == hop
        &&& hop == 0 ==> ({
            let f = kentry_return(u);
            &&& f.sp == s.sp && f.lr == s.lr
            &&& world(s1, f.mem, f.sp + g, fs, tpsp)
            &&& super::entry::arch_inv(k1, s1, 0)
        })
        &&& hop != 0 ==> ({
            let w = unwind_entry(u);
            &&& wf(w) && w.r[0] == hop && w.primask && w.sp == s.sp
            &&& world(s1, w.mem, w.sp, fs, tpsp)
            &&& super::entry::arch_inv(k1, s1, hop)
        })
    }),
{
    let u = kentry_after_call(t);
    lemma_match_same(s0, s1, fs);
    k1.lemma_stack_rising();
    lemma_frames_ext(s.mem, u.mem, s.sp + g, fs);
    if hop != 0 {
        super::entry::lemma_plan_number(k1, s1, hop);
        super::entry::lemma_chain_len(k1, s1);
        assert(fs.len() > 0);
        assert(g == 0);
    }
}

/// The SVC handler's stub: it passes the address of the caller's exception
/// frame (the PSP for a Job, the main stack pointer otherwise) and its LR,
/// and ends as `lemma_kentry_tail` states. An SVC is taken from Thread mode
/// (in Handler mode at the Kernel level it escalates to a HardFault).
pub proof fn theorem_svc_stub<K: KernelOps>(s: M, h: M, k1: &K, s0: ArchState, s1: ArchState, hop: int, fs: Seq<Frame>, g: int, tpsp: bool)
    requires
        wf(s), world(s0, s.mem, s.sp + g, fs, tpsp), hw_entry(s.lr, g, tpsp), k1.inv(),
        has_bit(s.lr, 0x8),
        super::entry::kentry_rel(k1, s0, s1, hop),
    ensures ({
        let c = svc_to_call(s);
        let t = call_svc(c, h, c.r[0], !has_bit(s.lr, 0x4), hop);
        let u = kentry_after_call(t);
        &&& c.r[0] == (if has_bit(s.lr, 0x4) { s.psp } else { s.sp }) && c.r[1] == s.lr
        &&& u.sp == s.sp && u.lr == s.lr && u.fp == s.fp && u.r[0] == hop
        &&& forall|i: int| 4 <= i <= 11 ==> u.r[i] == s.r[i]
        &&& hop == 0 ==> world(s1, kentry_return(u).mem, s.sp + g, fs, tpsp) && super::entry::arch_inv(k1, s1, 0)
        &&& hop != 0 ==> wf(unwind_entry(u)) && unwind_entry(u).r[0] == hop && unwind_entry(u).primask
                && world(s1, unwind_entry(u).mem, unwind_entry(u).sp, fs, tpsp) && super::entry::arch_inv(k1, s1, hop)
    }),
{
    let c = svc_to_call(s);
    let t = call_svc(c, h, c.r[0], !has_bit(s.lr, 0x4), hop);
    lemma_kentry_tail(s, c, t, k1, s0, s1, hop, fs, g, tpsp);
}

/// The MemManage handler's stub: `in_kernel` (R0) is 1 for a fault taken
/// from Handler mode or with PRIMASK set, the frame's address (R1) is the
/// PSP or the main stack pointer of the entry, R2 is its LR. The Rust half
/// plans no unwind for a fault it is told is the Kernel's
/// (`fault_core`: `in_kernel ==> r.0 == 0`).
pub proof fn theorem_memmanage_stub<K: KernelOps>(s: M, h: M, k1: &K, s0: ArchState, s1: ArchState, hop: int, fs: Seq<Frame>, g: int, tpsp: bool)
    requires
        wf(s), world(s0, s.mem, s.sp + g, fs, tpsp), hw_entry(s.lr, g, tpsp), k1.inv(),
        super::entry::kentry_rel(k1, s0, s1, hop),
        mm_to_call(s).r[0] != 0 ==> hop == 0,
    ensures ({
        let c = mm_to_call(s);
        let u = kentry_after_call(call_ret(c, h, hop));
        &&& (c.r[0] != 0) == (s.primask || !has_bit(s.lr, 0x8))
        &&& c.r[1] == (if has_bit(s.lr, 0x4) { s.psp } else { s.sp }) && c.r[2] == s.lr
        &&& u.sp == s.sp && u.lr == s.lr && u.fp == s.fp && u.r[0] == hop
        &&& forall|i: int| 4 <= i <= 11 ==> u.r[i] == s.r[i]
        &&& hop == 0 ==> world(s1, kentry_return(u).mem, s.sp + g, fs, tpsp) && super::entry::arch_inv(k1, s1, 0)
        &&& hop != 0 ==> wf(unwind_entry(u)) && unwind_entry(u).r[0] == hop && unwind_entry(u).primask
                && world(s1, unwind_entry(u).mem, unwind_entry(u).sp, fs, tpsp) && super::entry::arch_inv(k1, s1, hop)
    }),
{
    let c = mm_to_call(s);
    lemma_kentry_tail(s, c, call_ret(c, h, hop), k1, s0, s1, hop, fs, g, tpsp);
}

/// The BusFault and UsageFault handlers' stub, as the MemManage one without
/// the frame's address.
pub proof fn theorem_fault_stub<K: KernelOps>(s: M, h: M, k1: &K, s0: ArchState, s1: ArchState, hop: int, fs: Seq<Frame>, g: int, tpsp: bool)
    requires
        wf(s), world(s0, s.mem, s.sp + g, fs, tpsp), hw_entry(s.lr, g, tpsp), k1.inv(),
        super::entry::kentry_rel(k1, s0, s1, hop),
        fault_to_call(s).r[0] != 0 ==> hop == 0,
    ensures ({
        let c = fault_to_call(s);
        let u = kentry_after_call(call_ret(c, h, hop));
        &&& (c.r[0] != 0) == (s.primask || !has_bit(s.lr, 0x8))
        &&& u.sp == s.sp && u.lr == s.lr && u.fp == s.fp && u.r[0] == hop
        &&& forall|i: int| 4 <= i <= 11 ==> u.r[i] == s.r[i]
        &&& hop == 0 ==> world(s1, kentry_return(u).mem, s.sp + g, fs, tpsp) && super::entry::arch_inv(k1, s1, 0)
        &&& hop != 0 ==> wf(unwind_entry(u)) && unwind_entry(u).r[0] == hop && unwind_entry(u).primask
                && world(s1, unwind_entry(u).mem, unwind_entry(u).sp, fs, tpsp) && super::entry::arch_inv(k1, s1, hop)
    }),
{
    let c = fault_to_call(s);
    lemma_kentry_tail(s, c, call_ret(c, h, hop), k1, s0, s1, hop, fs, g, tpsp);
}

/// The timer handler's stub (`rsk_timer_entry`; no vector of the nRF52840
/// table uses it, the time base is an interrupt line served by
/// `rsk_irq_entry`). It has no Handler-mode guard of its own, hence the
/// premise that a plan is handed only to an entry from Thread mode.
pub proof fn theorem_timer_stub<K: KernelOps>(s: M, h: M, k1: &K, s0: ArchState, s1: ArchState, hop: int, fs: Seq<Frame>, g: int, tpsp: bool)
    requires
        wf(s), world(s0, s.mem, s.sp + g, fs, tpsp), hw_entry(s.lr, g, tpsp), k1.inv(),
        super::entry::kentry_rel(k1, s0, s1, hop),
        hop != 0 ==> has_bit(s.lr, 0x8),
    ensures ({
        let c = timer_to_call(s);
        let u = kentry_after_call(call_ret(c, h, hop));
        &&& u.sp == s.sp && u.lr == s.lr && u.fp == s.fp && u.r[0] == hop
        &&& forall|i: int| 4 <= i <= 11 ==> u.r[i] == s.r[i]
        &&& hop == 0 ==> world(s1, kentry_return(u).mem, s.sp + g, fs, tpsp) && super::entry::arch_inv(k1, s1, 0)
        &&& hop != 0 ==> wf(unwind_entry(u)) && unwind_entry(u).r[0] == hop && unwind_entry(u).primask
                && world(s1, unwind_entry(u).mem, unwind_entry(u).sp, fs, tpsp) && super::entry::arch_inv(k1, s1, hop)
    }),
{
    let c = timer_to_call(s);
    lemma_kentry_tail(s, c, call_ret(c, h, hop), k1, s0, s1, hop, fs, g, tpsp);
}

// ---- The exception model at a handler, and the entry-instruction window (5.8a).

/// HW-06: exception `n` taken while a handler executes (Handler mode, on the
/// main stack) in state `s`. The processor stacks the interrupted handler's
/// frame on the main stack (`g` bytes, at least the eight words of a basic
/// frame: R0 to R3, R12 and LR in its first six words; the rest of the room,
/// the return address, xPSR, an extended frame and padding among it, is not
/// modelled, `hx`), sets LR to the Handler-mode EXC_RETURN value and IPSR to
/// `n`, and leaves every other register as it is.
pub open spec fn exc_enter_handler(s: M, n: int, g: int, hx: M) -> M {
    let b = s.sp - g;
    M {
        sp: b,
        lr: 0xFFFF_FFF1,
        ipsr: n,
        mem: |x: int|
            if x == b { s.r[0] } else if x == b + 4 { s.r[1] } else if x == b + 8 { s.r[2] }
            else if x == b + 12 { s.r[3] } else if x == b + 16 { s.r[12] } else if x == b + 20 { s.lr }
            else if b <= x < s.sp { (hx.mem)(x) } else { (s.mem)(x) },
        ..s
    }
}

/// HW-07: the exception return to a handler from state `f`, whose main
/// stack pointer is at the frame the entry stacked: R0 to R3, R12 and LR
/// come back from the frame, the stack pointer rises over its `g` bytes,
/// IPSR is the interrupted handler's again, and FAULTMASK is cleared.
pub open spec fn exc_return_handler(f: M, g: int, ipsr: int) -> M {
    M {
        sp: f.sp + g,
        lr: (f.mem)(f.sp + 20),
        ipsr: ipsr,
        faultmask: false,
        r: Seq::new(13, |i: int|
            if i == 0 { (f.mem)(f.sp) } else if i == 1 { (f.mem)(f.sp + 4) } else if i == 2 { (f.mem)(f.sp + 8) }
            else if i == 3 { (f.mem)(f.sp + 12) } else if i == 12 { (f.mem)(f.sp + 16) } else { f.r[i] }),
        ..f
    }
}

/// The interrupted handler finds its state as it left it: every register,
/// the stack pointers, the floating-point registers and the memory at and
/// above its stack pointer; the masks are clear.
pub open spec fn resumed_as_left(r: M, s: M) -> bool {
    &&& r.r =~= s.r && r.sp == s.sp && r.lr == s.lr && r.psp == s.psp && r.control == s.control
    &&& r.fp =~= s.fp && r.ipsr == s.ipsr
    &&& !r.primask && !r.faultmask
    &&& forall|x: int| x >= s.sp ==> (s.mem)(x) == #[trigger] (r.mem)(x)
}

/// The Handler-mode EXC_RETURN value: neither Thread mode nor the process stack.
pub proof fn lemma_handler_exc_return()
    ensures !has_bit(0xFFFF_FFF1, 0x8), !has_bit(0xFFFF_FFF1, 0x4), 0xFFFF_FFF1u32 & 0x8 == 0,
{
    assert(0xFFFF_FFF1u32 & 0x8 == 0) by (bit_vector);
    assert(0xFFFF_FFF1u32 & 0x4 == 0) by (bit_vector);
}

/// The entry-instruction window (5.8a), an interrupt vector's case. A
/// handler in state `s`, PRIMASK clear (a dispatch stub at its entry
/// instruction, with the main stack and the records in agreement for its
/// own entry, `g0`), is preempted by an interrupt vector: any binding, the
/// time base, an intake, another dispatch vector. The preempting stub's
/// Rust half answers as `irq_entry_core`'s contract states for an entry
/// from Handler mode: no dispatch and no unwind plan (a dispatch vector is
/// deferred, an unwind is owed). Then that stub returns as entered and the
/// exception return gives the preempted handler back every register, its
/// stack pointer, the memory at and above it, and the agreement of the main
/// stack and the records for the Kernel state the preempting entry left;
/// which is the precondition of the preempted stub's own theorem
/// (`theorem_irq_stub`), now for that Kernel state.
pub proof fn theorem_nested_irq<K: KernelOps>(s: M, n: int, g: int, hx: M, h: M, k1: &K, s0: ArchState, s1: ArchState, plan: DispatchPlan, fs: Seq<Frame>, g0: int, tpsp: bool)
    requires
        wf(s), !s.primask, g >= 32, g0 >= 0,
        world(s0, s.mem, s.sp + g0, fs, tpsp),
        k1.inv(), WINDOW_DEFER,
        super::entry::irq_rel(k1, s0, s1, 0xFFFF_FFF1u32, plan),
    ensures ({
        let e = exc_enter_handler(s, n, g, hx);
        let p = (plan.psp as int, plan.control as int, plan.exc_return as int, plan.hop as int);
        let f = irq_return(irq_restore(irq_called(e, h, p)));
        let r = exc_return_handler(f, g, s.ipsr);
        &&& plan.psp == 0 && plan.hop == 0
        &&& resumed_as_left(r, s)
        &&& world(s1, r.mem, r.sp + g0, fs, tpsp)
        &&& super::entry::arch_inv(k1, s1, 0)
    }),
{
    lemma_handler_exc_return();
    let e = exc_enter_handler(s, n, g, hx);
    let p = (plan.psp as int, plan.control as int, plan.exc_return as int, plan.hop as int);
    assert(wf(e));
    lemma_frames_ext(s.mem, e.mem, s.sp + g0, fs);
    assert(hw_entry(e.lr, g + g0, tpsp));
    theorem_irq_stub(e, h, k1, s0, s1, plan, fs, g + g0, tpsp);
    lemma_irq_restore(e, h, p);
    let u = irq_restore(irq_called(e, h, p));
    let f = irq_return(u);
    let r = exc_return_handler(f, g, s.ipsr);
    assert forall|i: int| 0 <= i < 13 implies r.r[i] == s.r[i] by {
        assert((e.mem)(e.sp) == s.r[0] && (e.mem)(e.sp + 4) == s.r[1] && (e.mem)(e.sp + 8) == s.r[2]
            && (e.mem)(e.sp + 12) == s.r[3] && (e.mem)(e.sp + 16) == s.r[12]);
        assert((e.mem)(e.sp) == (u.mem)(e.sp) && (e.mem)(e.sp + 4) == (u.mem)(e.sp + 4) && (e.mem)(e.sp + 8) == (u.mem)(e.sp + 8)
            && (e.mem)(e.sp + 12) == (u.mem)(e.sp + 12) && (e.mem)(e.sp + 16) == (u.mem)(e.sp + 16));
        if 3 <= i <= 11 {
            assert(u.r[i] == e.r[i]);
        }
    }
    assert((e.mem)(e.sp + 20) == (u.mem)(e.sp + 20));
    assert forall|x: int| x >= s.sp implies (s.mem)(x) == #[trigger] (r.mem)(x) by {
        assert((e.mem)(x) == (u.mem)(x));
    }
}

/// The entry-instruction window, a fault's case (BusFault, UsageFault; the
/// MemManage stub differs in its arguments only): a fault taken from
/// Handler mode is passed to the Rust half as the Kernel's (`in_kernel`),
/// for which `fault_core` plans no unwind; an entry that returns at all
/// (the Kernel's fault response is the safe state) gives the interrupted
/// handler back its state as `theorem_nested_irq` states.
pub proof fn theorem_nested_fault<K: KernelOps>(s: M, n: int, g: int, hx: M, h: M, k1: &K, s0: ArchState, s1: ArchState, hop: int, fs: Seq<Frame>, g0: int, tpsp: bool)
    requires
        wf(s), !s.primask, !s.faultmask, g >= 32, g0 >= 0,
        world(s0, s.mem, s.sp + g0, fs, tpsp),
        k1.inv(),
        super::entry::kentry_rel(k1, s0, s1, hop),
        // `fault_core`: `in_kernel ==> r.0 == 0`, with the stub's argument.
        fault_to_call(exc_enter_handler(s, n, g, hx)).r[0] != 0 ==> hop == 0,
    ensures ({
        let e = exc_enter_handler(s, n, g, hx);
        let f = kentry_return(kentry_after_call(call_ret(fault_to_call(e), h, hop)));
        let r = exc_return_handler(f, g, s.ipsr);
        &&& fault_to_call(e).r[0] != 0 && hop == 0
        &&& resumed_as_left(r, s)
        &&& world(s1, r.mem, r.sp + g0, fs, tpsp)
        &&& super::entry::arch_inv(k1, s1, 0)
    }),
{
    lemma_handler_exc_return();
    let e = exc_enter_handler(s, n, g, hx);
    assert(wf(e));
    lemma_frames_ext(s.mem, e.mem, s.sp + g0, fs);
    assert(hw_entry(e.lr, g + g0, tpsp));
    theorem_fault_stub(e, h, k1, s0, s1, hop, fs, g + g0, tpsp);
    let c = fault_to_call(e);
    let t = call_ret(c, h, hop);
    let f = kentry_return(kentry_after_call(t));
    let r = exc_return_handler(f, g, s.ipsr);
    assert forall|x: int| x >= e.sp implies (e.mem)(x) == #[trigger] (f.mem)(x) by {}
    assert forall|i: int| 0 <= i < 13 implies r.r[i] == s.r[i] by {
        assert((e.mem)(e.sp) == s.r[0] && (e.mem)(e.sp + 4) == s.r[1] && (e.mem)(e.sp + 8) == s.r[2]
            && (e.mem)(e.sp + 12) == s.r[3] && (e.mem)(e.sp + 16) == s.r[12]);
        assert((f.mem)(e.sp) == (e.mem)(e.sp) && (f.mem)(e.sp + 4) == (e.mem)(e.sp + 4) && (f.mem)(e.sp + 8) == (e.mem)(e.sp + 8)
            && (f.mem)(e.sp + 12) == (e.mem)(e.sp + 12) && (f.mem)(e.sp + 16) == (e.mem)(e.sp + 16));
    }
    assert((f.mem)(e.sp + 20) == (e.mem)(e.sp + 20));
    assert forall|x: int| x >= s.sp implies (s.mem)(x) == #[trigger] (r.mem)(x) by {
        assert((e.mem)(x) == (f.mem)(x));
    }
}

} // verus!
