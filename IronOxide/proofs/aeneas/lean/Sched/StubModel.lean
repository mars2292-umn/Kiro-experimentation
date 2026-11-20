import Mathlib.Tactic

/-! # The assembly stubs against a machine model (2026-11-19)

Property 2 of the design, the registers' half: the Job that resumes gets
back R4 to R11, S16 to S31, its stack pointer, PSP and CONTROL as they were
when it was preempted.

This is the Lean counterpart of `rsk-kernel/src/arch/cm4/stub_model.rs`
(Verus, Verification_Plan 5.17). The assembly cannot be translated by
Aeneas, so, as there, the stubs are written out by hand: a model of the
processor state a stub touches (`M`), one function per instruction form
(`i_*`), and each stub of `unsafe_module/stubs.rs` as the sequence of
those functions, one per assembly line, under the same names as in the
Verus model. `cargo xtask check` compares the two models line by line, and
the Verus model with the assembly, so neither can drift unnoticed.

Proven here, of the machine alone: what a dispatch stub pushes
(`irq_called`), that it restores it when it does not dispatch
(`irq_restore`), that one round of `rsk_unwind` pops exactly the record at
the top of the main stack (`unwind_round_spec`), that a dispatch followed
by the unwind of its level gives the preempted context back its registers
(`dispatch_unwind_restores`), the same over any number of levels
(`unwind_loop_frames`), that the Kernel-level handlers return with the
registers they were entered with (`kentry_tail`, and the four stubs), and
that a handler preempted by an interrupt vector that neither dispatches nor
unwinds finds its state as it left it (`nested_irq_resumed`).

Not ported from Verus: the agreement of the main stack with the level
records across every entry (`world`, `arch_inv`), which rests on ghost
state of `ArchState` that the extraction erases. What links a record to its
stub here is `Sched/Record.lean`.

Assumed, as in Verus: the instruction semantics of the `i_*` functions, the
exception model, and AAPCS for the compiled Rust halves (`call`). -/

namespace Stub

/-- The processor state the stubs read and write: R0 to R12, the main stack
pointer, LR, PSP, CONTROL, IPSR, PRIMASK, FAULTMASK, the Z flag, S16 to S31
and memory by the address of a word. -/
structure M where
  r : Nat → Int
  sp : Int
  lr : Int
  psp : Int
  control : Int
  ipsr : Int
  primask : Bool
  faultmask : Bool
  z : Bool
  fp : Nat → Int
  mem : Int → Int

/-- Bit test on a 32-bit register value. -/
def hasBit (v : Int) (mask : Nat) : Bool := (v.toNat % 4294967296) &&& mask != 0

def setr (s : M) (i : Nat) (v : Int) : M := { s with r := fun j => if j = i then v else s.r j }

def wr (s : M) (a v : Int) : M := { s with mem := fun x => if x = a then v else s.mem x }

/-! ## The instruction forms of the stubs -/

def i_cpsid_i (s : M) : M := { s with primask := true }
def i_cpsie_i (s : M) : M := { s with primask := false }
def i_cpsid_f (s : M) : M := { s with faultmask := true }
def i_dsb (s : M) : M := s
def i_isb (s : M) : M := s

/-- `push {r3-r11, lr}`: the lowest register at the lowest address. -/
def i_push_r3_r11_lr (s : M) : M :=
  { s with
    sp := s.sp - 40,
    mem := fun x =>
      if x = s.sp - 40 then s.r 3 else if x = s.sp - 40 + 4 then s.r 4
      else if x = s.sp - 40 + 8 then s.r 5 else if x = s.sp - 40 + 12 then s.r 6
      else if x = s.sp - 40 + 16 then s.r 7 else if x = s.sp - 40 + 20 then s.r 8
      else if x = s.sp - 40 + 24 then s.r 9 else if x = s.sp - 40 + 28 then s.r 10
      else if x = s.sp - 40 + 32 then s.r 11 else if x = s.sp - 40 + 36 then s.lr
      else s.mem x }

/-- `pop {r3-r11, lr}`. -/
def i_pop_r3_r11_lr (s : M) : M :=
  { s with
    sp := s.sp + 40,
    lr := s.mem (s.sp + 36),
    r := fun i => if 3 ≤ i ∧ i ≤ 11 then s.mem (s.sp + 4 * ((i : Int) - 3)) else s.r i }

/-- `push {r4, lr}`. -/
def i_push_r4_lr (s : M) : M :=
  { s with
    sp := s.sp - 8,
    mem := fun x => if x = s.sp - 8 then s.r 4 else if x = s.sp - 8 + 4 then s.lr else s.mem x }

/-- `pop {r4, lr}`. -/
def i_pop_r4_lr (s : M) : M :=
  { s with
    sp := s.sp + 8, lr := s.mem (s.sp + 4),
    r := fun j => if j = 4 then s.mem s.sp else s.r j }

/-- `tst lr, #mask`: Z is set when no tested bit is. -/
def i_tst_lr (s : M) (mask : Nat) : M := { s with z := !hasBit s.lr mask }
/-- `tst rN, #mask`. -/
def i_tst_r (s : M) (n : Nat) (mask : Nat) : M := { s with z := !hasBit (s.r n) mask }

/-- `vpush {s16-s31}`: sixteen words, S16 at the lowest address. -/
def vpush (s : M) : M :=
  { s with
    sp := s.sp - 64,
    mem := fun x =>
      if s.sp - 64 ≤ x ∧ x < s.sp ∧ (x - (s.sp - 64)) % 4 = 0
      then s.fp ((x - (s.sp - 64)) / 4).toNat else s.mem x }

/-- `vpop {s16-s31}`. -/
def vpop (s : M) : M :=
  { s with sp := s.sp + 64, fp := fun i => if i < 16 then s.mem (s.sp + 4 * (i : Int)) else s.fp i }

/-- `it eq` / `vpusheq {s16-s31}`. -/
def i_vpush_eq (s : M) : M := if s.z then vpush s else s
/-- `it eq` / `vpopeq {s16-s31}`. -/
def i_vpop_eq (s : M) : M := if s.z then vpop s else s

def i_mrs_ipsr (s : M) (rd : Nat) : M := setr s rd s.ipsr
def i_mrs_psp (s : M) (rd : Nat) : M := setr s rd s.psp
def i_mrs_control (s : M) (rd : Nat) : M := setr s rd s.control
def i_mrs_primask (s : M) (rd : Nat) : M := setr s rd (if s.primask then 1 else 0)
/-- `mrseq rd, msp` under `ite eq`. -/
def i_mrs_msp_eq (s : M) (rd : Nat) : M := if s.z then setr s rd s.sp else s
/-- `mrsne rd, psp`. -/
def i_mrs_psp_ne (s : M) (rd : Nat) : M := if !s.z then setr s rd s.psp else s
/-- `moveq rd, #v`. -/
def i_mov_imm_eq (s : M) (rd : Nat) (v : Int) : M := if s.z then setr s rd v else s
/-- `addeq rd, sp, #off`. -/
def i_add_r_sp_eq (s : M) (rd : Nat) (off : Int) : M := if s.z then setr s rd (s.sp + off) else s
/-- `mov rd, lr`. -/
def i_mov_r_lr (s : M) (rd : Nat) : M := setr s rd s.lr
/-- `mov rd, rs`. -/
def i_mov (s : M) (rd rs : Nat) : M := setr s rd (s.r rs)
/-- `mov rd, sp`. -/
def i_mov_r_sp (s : M) (rd : Nat) : M := setr s rd s.sp
/-- `sub sp, sp, #n`. -/
def i_sub_sp (s : M) (n : Int) : M := { s with sp := s.sp - n }
/-- `add sp, sp, #n`. -/
def i_add_sp (s : M) (n : Int) : M := { s with sp := s.sp + n }
/-- `add rd, sp, #off`. -/
def i_add_r_sp (s : M) (rd : Nat) (off : Int) : M := setr s rd (s.sp + off)
/-- `str rs, [sp, #off]`. -/
def i_str_sp (s : M) (rs : Nat) (off : Int) : M := wr s (s.sp + off) (s.r rs)
/-- `ldr rd, [sp, #off]`. -/
def i_ldr_sp (s : M) (rd : Nat) (off : Int) : M := setr s rd (s.mem (s.sp + off))
/-- `msr psp, rs`. -/
def i_msr_psp (s : M) (rs : Nat) : M := { s with psp := s.r rs }
/-- `msr control, rs`. -/
def i_msr_control (s : M) (rs : Nat) : M := { s with control := s.r rs }
/-- `cmp rn, #v`. -/
def i_cmp_imm (s : M) (rn : Nat) (v : Int) : M := { s with z := decide (s.r rn = v) }

/-! ## A call of a Rust half (`bl`) -/

/-- AAPCS: the callee returns with R4 to R11, SP, S16 to S31 and the memory
at and above SP as they were; R0 to R3, R12, LR, the flags and the memory
below SP are arbitrary (`h`). The Rust halves write neither mask, PSP nor
CONTROL. -/
def call (s h : M) : M :=
  { s with
    r := fun i => if 4 ≤ i ∧ i ≤ 11 then s.r i else h.r i,
    lr := h.lr, z := h.z,
    mem := fun x => if s.sp ≤ x then s.mem x else h.mem x }

/-- A call whose callee writes four result words through the pointer `out`. -/
def call_out4 (s h : M) (out v0 v1 v2 v3 : Int) : M :=
  wr (wr (wr (wr (call s h) out v0) (out + 4) v1) (out + 8) v2) (out + 12) v3

/-- A call whose callee returns `v` in R0. -/
def call_ret (s h : M) (v : Int) : M := setr (call s h) 0 v

/-- The SVC handler's call: the callee returns `v` in R0 and may write the
caller's exception frame, the eight words at `frame`, when that frame is on
the main stack (`on_msp`). -/
def call_svc (s h : M) (frame : Int) (on_msp : Bool) (v : Int) : M :=
  setr { call s h with
    mem := fun x => if on_msp = true ∧ frame ≤ x ∧ x < frame + 32 then h.mem x
      else (call s h).mem x } 0 v

/-! ## The stubs, line by line (`unsafe_module/stubs.rs`) -/

/-- `rsk_irq_entry`, from the entry instruction to the call of the Rust half. -/
def irq_to_call (s : M) : M :=
  let s := i_cpsid_i s
  let s := i_push_r3_r11_lr s
  let s := i_tst_lr s 0x10
  let s := i_vpush_eq s
  let s := i_mrs_ipsr s 0
  let s := i_mrs_psp s 1
  let s := i_mrs_control s 2
  let s := i_mov_r_lr s 3
  let s := i_mov_r_lr s 8
  let s := i_sub_sp s 24
  let s := i_add_r_sp s 4 8
  let s := i_str_sp s 4 0
  -- ctl: bl {entry}
  s

/-- From the return of the Rust half to the test of the plan's frame. -/
def irq_after_call (s : M) : M :=
  let s := i_ldr_sp s 0 8
  let s := i_ldr_sp s 1 12
  let s := i_ldr_sp s 2 16
  let s := i_ldr_sp s 3 20
  let s := i_add_sp s 24
  -- ctl: cbz r0, 1f
  s

/-- The dispatch path, to its exception return with R2. -/
def irq_dispatch (s : M) : M :=
  let s := i_msr_psp s 0
  let s := i_msr_control s 1
  let s := i_isb s
  let s := i_dsb s
  let s := i_cpsid_f s
  let s := i_cpsie_i s
  -- ctl: bx r2
  s

/-- The path without a dispatch, to the test of the unwind plan. -/
def irq_restore (s : M) : M :=
  -- ctl: 1:
  let s := i_mov s 12 3
  let s := i_tst_r s 8 0x10
  let s := i_vpop_eq s
  let s := i_pop_r3_r11_lr s
  let s := i_cmp_imm s 12 0
  -- ctl: bne 2f
  s

/-- The return as entered, to its exception return with LR. -/
def irq_return (s : M) : M :=
  let s := i_dsb s
  let s := i_cpsid_f s
  let s := i_cpsie_i s
  -- ctl: bx lr
  s

/-- The branch to `rsk_unwind` with the plan in R0. -/
def irq_to_unwind (s : M) : M :=
  -- ctl: 2:
  let s := i_mov s 0 12
  -- ctl: b {unwind}
  s

/-- The entry instruction of `rsk_unwind`. -/
def unwind_entry (s : M) : M :=
  let s := i_cpsid_i s
  s

/-- One round, from the loop's head to the call of the Rust half. -/
def unwind_to_call (s : M) : M :=
  -- ctl: 1:
  let s := i_sub_sp s 16
  let s := i_mov_r_sp s 1
  -- ctl: bl {epilogue}
  s

/-- One round, from the return of the Rust half to the test of the next plan. -/
def unwind_after_call (s : M) : M :=
  let s := i_ldr_sp s 0 0
  let s := i_ldr_sp s 1 4
  let s := i_ldr_sp s 2 8
  let s := i_ldr_sp s 12 12
  let s := i_add_sp s 16
  let s := i_tst_r s 2 0x10
  let s := i_vpop_eq s
  let s := i_pop_r3_r11_lr s
  let s := i_msr_psp s 0
  let s := i_msr_control s 1
  let s := i_isb s
  let s := i_cmp_imm s 12 0
  -- ctl: bne 2f
  s

/-- The exit, to its exception return with R2. -/
def unwind_exit (s : M) : M :=
  let s := i_dsb s
  let s := i_cpsid_f s
  let s := i_cpsie_i s
  -- ctl: bx r2
  s

/-- The branch back to the loop's head with the next plan in R0. -/
def unwind_again (s : M) : M :=
  -- ctl: 2:
  let s := i_mov s 0 12
  -- ctl: b 1b
  s

/-- `rsk_svc_entry`, to the call of the Rust half. -/
def svc_to_call (s : M) : M :=
  let s := i_tst_lr s 4
  let s := i_mrs_msp_eq s 0
  let s := i_mrs_psp_ne s 0
  let s := i_mov_r_lr s 1
  let s := i_push_r4_lr s
  -- ctl: bl {svc}
  s

/-- The tail every Kernel-level handler shares: from the return of the Rust
half to the test of the plan. -/
def kentry_after_call (s : M) : M :=
  let s := i_pop_r4_lr s
  -- ctl: cbz r0, 1f
  -- ctl: b {unwind}
  s

/-- The return as entered, to its exception return with LR. -/
def kentry_return (s : M) : M :=
  -- ctl: 1:
  let s := i_dsb s
  -- ctl: bx lr
  s

/-- `rsk_timer_entry`. -/
def timer_to_call (s : M) : M :=
  let s := i_push_r4_lr s
  -- ctl: bl {timer}
  s

/-- `rsk_memmanage_entry`. -/
def mm_to_call (s : M) : M :=
  let s := i_push_r4_lr s
  let s := i_mrs_primask s 0
  let s := i_tst_lr s 8
  let s := i_mov_imm_eq s 0 1
  let s := i_tst_lr s 4
  let s := i_mrs_psp_ne s 1
  let s := i_add_r_sp_eq s 1 8
  let s := i_mov_r_lr s 2
  -- ctl: bl {f}
  s

/-- `rsk_busfault_entry` and `rsk_usagefault_entry`. -/
def fault_to_call (s : M) : M :=
  let s := i_push_r4_lr s
  let s := i_mrs_primask s 0
  let s := i_tst_lr s 8
  let s := i_mov_imm_eq s 0 1
  -- ctl: bl {f}
  s

/-! ## The main stack: the register records of the levels -/

/-- What a dispatch stub leaves on the main stack for its level: R3 to R11
and LR, and below them S16 to S31 when the preempted context had a
floating-point context; `gap` is the room between this record and the next
older one. -/
structure Frame where
  hasFp : Bool
  regs : Nat → Int
  fpv : Nat → Int
  gap : Int

def frameSize (f : Frame) : Int := if f.hasFp then 104 else 40

/-- The record `f` is the one at the top of the main stack `sp`. -/
def frameTop (mem : Int → Int) (sp : Int) (f : Frame) : Prop :=
  (∀ i : Nat, i < 10 → mem ((if f.hasFp then sp + 64 else sp) + 4 * (i : Int)) = f.regs i) ∧
  (f.hasFp = true → ∀ i : Nat, i < 16 → mem (sp + 4 * (i : Int)) = f.fpv i) ∧ 0 ≤ f.gap

/-- The records `fs` lie on the main stack from `sp` upward, the first one
at the top. -/
def framesAt (mem : Int → Int) : Int → List Frame → Prop
  | _, [] => True
  | sp, f :: fs => frameTop mem sp f ∧ framesAt mem (sp + frameSize f + f.gap) fs

/-- The register record a dispatch stub entered in state `s` pushes. -/
def entryFrame (s : M) (g : Int) : Frame :=
  { hasFp := !hasBit s.lr 0x10,
    regs := fun i => if i < 9 then s.r (i + 3) else s.lr,
    fpv := s.fp, gap := g }

/-- The state after the Rust half returned `p` (frame address, CONTROL,
EXC_RETURN, unwind plan) through the out-pointer the stub passed. -/
def irq_called (s h : M) (p0 p1 p2 p3 : Int) : M :=
  irq_after_call (call_out4 (irq_to_call s) h ((irq_to_call s).mem (irq_to_call s).sp) p0 p1 p2 p3)

/-- The stub's call: the arguments of `irq_entry_core`, PRIMASK held, and
the out-pointer inside the stub's own stack room. -/
theorem irq_call (s : M) :
    (irq_to_call s).primask = true ∧ (irq_to_call s).r 0 = s.ipsr ∧
    (irq_to_call s).r 1 = s.psp ∧ (irq_to_call s).r 2 = s.control ∧
    (irq_to_call s).r 3 = s.lr ∧
    (irq_to_call s).mem (irq_to_call s).sp = (irq_to_call s).sp + 8 ∧
    (irq_to_call s).sp = s.sp - frameSize (entryFrame s 0) - 24 := by
  cases hb : hasBit s.lr 16 <;>
    simp [irq_to_call, i_cpsid_i, i_push_r3_r11_lr, i_tst_lr, i_vpush_eq, vpush, i_mrs_ipsr,
      i_mrs_psp, i_mrs_control, i_mov_r_lr, i_sub_sp, i_add_r_sp, i_str_sp, setr, wr, entryFrame,
      frameSize, hb] <;> omega

/-! ## The instructions that move the main stack -/

theorem push_spec (s : M) :
    (i_push_r3_r11_lr s).sp = s.sp - 40 ∧
    (∀ k : Nat, k < 10 →
      (i_push_r3_r11_lr s).mem (s.sp - 40 + 4 * (k : Int)) = if k < 9 then s.r (k + 3) else s.lr) ∧
    (∀ x, s.sp ≤ x → (i_push_r3_r11_lr s).mem x = s.mem x) := by
  refine ⟨rfl, ?_, ?_⟩
  · intro k hk
    interval_cases k <;> simp [i_push_r3_r11_lr]
  · intro x hx
    simp only [i_push_r3_r11_lr]
    repeat' split
    all_goals first | rfl | omega

theorem vpush_spec (s : M) :
    (vpush s).sp = s.sp - 64 ∧
    (∀ k : Nat, k < 16 → (vpush s).mem (s.sp - 64 + 4 * (k : Int)) = s.fp k) ∧
    (∀ x, s.sp ≤ x → (vpush s).mem x = s.mem x) := by
  refine ⟨rfl, ?_, ?_⟩
  · intro k hk
    simp only [vpush]
    rw [if_pos (by omega)]
    congr 1
    omega
  · intro x hx
    simp only [vpush]
    rw [if_neg (by omega)]

/-- The entry of `rsk_irq_entry` up to the pushes. -/
def irq_pro (s : M) : M := i_vpush_eq (i_tst_lr (i_push_r3_r11_lr (i_cpsid_i s)) 0x10)

/-- What a dispatch stub has pushed: the level's register record at the top
of the main stack, nothing above the entry's stack pointer touched, no
register changed. -/
theorem irq_pro_spec (s : M) (g : Int) (hg : 0 ≤ g) :
    (irq_pro s).sp = s.sp - frameSize (entryFrame s g) ∧
    frameTop (irq_pro s).mem (irq_pro s).sp (entryFrame s g) ∧
    (∀ x, s.sp ≤ x → (irq_pro s).mem x = s.mem x) ∧
    (irq_pro s).r = s.r ∧ (irq_pro s).lr = s.lr ∧ (irq_pro s).fp = s.fp ∧
    (irq_pro s).psp = s.psp ∧ (irq_pro s).control = s.control ∧ (irq_pro s).ipsr = s.ipsr ∧
    (irq_pro s).primask = true ∧ (irq_pro s).faultmask = s.faultmask := by
  obtain ⟨hp1, hp2, hp3⟩ := push_spec (i_cpsid_i s)
  have e0 : (i_cpsid_i s).sp = s.sp := rfl
  have e1 : (i_cpsid_i s).r = s.r := rfl
  have e2 : (i_cpsid_i s).lr = s.lr := rfl
  rw [e0] at hp1 hp2 hp3
  rw [e1, e2] at hp2
  cases hb : hasBit s.lr 16
  · have hq : irq_pro s = vpush (i_tst_lr (i_push_r3_r11_lr (i_cpsid_i s)) 0x10) := by
      simp only [irq_pro, i_vpush_eq, i_tst_lr, i_push_r3_r11_lr, i_cpsid_i, hb, Bool.not_false,
        if_true]
    obtain ⟨hv1, hv2, hv3⟩ := vpush_spec (i_tst_lr (i_push_r3_r11_lr (i_cpsid_i s)) 0x10)
    have e3 : (i_tst_lr (i_push_r3_r11_lr (i_cpsid_i s)) 0x10).sp = s.sp - 40 := hp1
    have e4 : (i_tst_lr (i_push_r3_r11_lr (i_cpsid_i s)) 0x10).mem
        = (i_push_r3_r11_lr (i_cpsid_i s)).mem := rfl
    have e5 : (i_tst_lr (i_push_r3_r11_lr (i_cpsid_i s)) 0x10).fp = s.fp := rfl
    rw [e3] at hv1 hv2 hv3
    rw [e5] at hv2
    rw [e4] at hv3
    rw [hq]
    have hfs : frameSize (entryFrame s g) = 104 := by simp [frameSize, entryFrame, hb]
    have hfp : (entryFrame s g).hasFp = true := by simp [entryFrame, hb]
    refine ⟨by rw [hv1, hfs]; omega, ⟨?_, ?_, hg⟩, ?_, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩
    · intro i hi
      rw [hfp, if_pos rfl, hv1]
      rw [show s.sp - 40 - 64 + 64 + 4 * (i : Int) = s.sp - 40 + 4 * (i : Int) by omega]
      rw [hv3 _ (by omega), hp2 i hi]
      rfl
    · intro _ i hi
      rw [hv1, hv2 i hi]
      rfl
    · intro x hx
      rw [hv3 x (by omega), hp3 x hx]
      rfl
  · have hq : irq_pro s = i_tst_lr (i_push_r3_r11_lr (i_cpsid_i s)) 0x10 := by
      simp only [irq_pro, i_vpush_eq, i_tst_lr, i_push_r3_r11_lr, i_cpsid_i, hb, Bool.not_true,
        Bool.false_eq_true, ite_false]
    rw [hq]
    have hfs : frameSize (entryFrame s g) = 40 := by simp [frameSize, entryFrame, hb]
    have hfp : (entryFrame s g).hasFp = false := by simp [entryFrame, hb]
    have e3 : (i_tst_lr (i_push_r3_r11_lr (i_cpsid_i s)) 0x10).sp = s.sp - 40 := hp1
    have e4 : (i_tst_lr (i_push_r3_r11_lr (i_cpsid_i s)) 0x10).mem
        = (i_push_r3_r11_lr (i_cpsid_i s)).mem := rfl
    refine ⟨by rw [e3, hfs], ⟨?_, ?_, hg⟩, ?_, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩
    · intro i hi
      rw [hfp, e3, e4]
      simp only [Bool.false_eq_true, ite_false]
      rw [hp2 i hi]
      rfl
    · intro hf
      rw [hfp] at hf
      cases hf
    · intro x hx
      rw [e4, hp3 x hx]
      rfl

/-- Memory that agrees at and above `sp` holds the same record there. -/
theorem frameTop_ext {m1 m2 : Int → Int} {sp : Int} {f : Frame}
    (h : ∀ x, sp ≤ x → m2 x = m1 x) (hf : frameTop m1 sp f) : frameTop m2 sp f := by
  obtain ⟨a, b, c⟩ := hf
  refine ⟨fun i hi => ?_, fun hfp i hi => ?_, c⟩
  · rw [h _ (by split <;> omega)]; exact a i hi
  · rw [h _ (by omega)]; exact b hfp i hi

theorem frameSize_pos (f : Frame) : 0 < frameSize f := by
  unfold frameSize; split <;> omega

/-- Memory that agrees at and above `sp` holds the same records there. -/
theorem framesAt_ext {m1 m2 : Int → Int} :
    ∀ (fs : List Frame) (sp : Int), (∀ x, sp ≤ x → m2 x = m1 x) → framesAt m1 sp fs →
      framesAt m2 sp fs := by
  intro fs
  induction fs with
  | nil => intro sp _ _; trivial
  | cons f fs ih =>
    intro sp h hf
    obtain ⟨h1, h2⟩ := hf
    have hg := h1.2.2
    have := frameSize_pos f
    exact ⟨frameTop_ext h h1, ih _ (fun x hx => h x (by omega)) h2⟩

/-- Closes a goal about a register or a stacked word once the instruction
functions are unfolded. -/
syntax "stub_close" : tactic
macro_rules
  | `(tactic| stub_close) => `(tactic| first
      | rfl
      | omega
      | (split_ifs <;> first | rfl | omega)
      | (simp; done))

/-- `rsk_irq_entry` from the pushes to the call. -/
def irq_tail (q : M) : M :=
  i_str_sp (i_add_r_sp (i_sub_sp (i_mov_r_lr (i_mov_r_lr (i_mrs_control (i_mrs_psp
    (i_mrs_ipsr q 0) 1) 2) 3) 8) 24) 4 8) 4 0

theorem irq_to_call_eq (s : M) : irq_to_call s = irq_tail (irq_pro s) := rfl

theorem irq_tail_out (q : M) : (irq_tail q).mem (irq_tail q).sp = (irq_tail q).sp + 8 := by
  simp only [irq_tail, i_str_sp, i_add_r_sp, i_sub_sp, i_mov_r_lr, i_mrs_control, i_mrs_psp,
    i_mrs_ipsr, setr, wr]
  stub_close

/-- The call and the reload of the plan, from the state the pushes left. -/
def irq_ret (q h : M) (p0 p1 p2 p3 : Int) : M :=
  irq_after_call (call_out4 (irq_tail q) h ((irq_tail q).sp + 8) p0 p1 p2 p3)

theorem irq_called_eq (s h : M) (p0 p1 p2 p3 : Int) :
    irq_called s h p0 p1 p2 p3 = irq_ret (irq_pro s) h p0 p1 p2 p3 := by
  unfold irq_called irq_ret
  rw [irq_to_call_eq, irq_tail_out]

set_option maxHeartbeats 4000000 in
theorem irq_ret_spec (q h : M) (p0 p1 p2 p3 : Int) :
    (irq_ret q h p0 p1 p2 p3).r 0 = p0 ∧ (irq_ret q h p0 p1 p2 p3).r 1 = p1 ∧
    (irq_ret q h p0 p1 p2 p3).r 2 = p2 ∧ (irq_ret q h p0 p1 p2 p3).r 3 = p3 ∧
    (irq_ret q h p0 p1 p2 p3).r 8 = q.lr ∧
    (irq_ret q h p0 p1 p2 p3).sp = q.sp ∧
    (∀ x, q.sp ≤ x → (irq_ret q h p0 p1 p2 p3).mem x = q.mem x) ∧
    (irq_ret q h p0 p1 p2 p3).fp = q.fp ∧ (irq_ret q h p0 p1 p2 p3).primask = q.primask ∧
    (irq_ret q h p0 p1 p2 p3).psp = q.psp ∧ (irq_ret q h p0 p1 p2 p3).control = q.control ∧
    (irq_ret q h p0 p1 p2 p3).faultmask = q.faultmask := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, rfl, rfl, rfl, rfl, rfl⟩
  all_goals
    simp only [irq_ret, irq_tail, irq_after_call, call_out4, call, i_str_sp, i_add_r_sp, i_sub_sp,
      i_mov_r_lr, i_mrs_control, i_mrs_psp, i_mrs_ipsr, i_ldr_sp, i_add_sp, setr, wr]
  · stub_close
  · stub_close
  · stub_close
  · stub_close
  · stub_close
  · omega
  · intro x hx
    stub_close

/-- The state after the call: the plan in R0 to R3, the level's register
record on top of the main stack, everything above the entry's stack
pointer untouched. -/
theorem irq_called_spec (s h : M) (p0 p1 p2 p3 g : Int) (hg : 0 ≤ g) :
    (irq_called s h p0 p1 p2 p3).r 0 = p0 ∧ (irq_called s h p0 p1 p2 p3).r 1 = p1 ∧
    (irq_called s h p0 p1 p2 p3).r 2 = p2 ∧ (irq_called s h p0 p1 p2 p3).r 3 = p3 ∧
    (irq_called s h p0 p1 p2 p3).r 8 = s.lr ∧
    (irq_called s h p0 p1 p2 p3).sp = s.sp - frameSize (entryFrame s g) ∧
    frameTop (irq_called s h p0 p1 p2 p3).mem (irq_called s h p0 p1 p2 p3).sp (entryFrame s g) ∧
    (∀ x, s.sp ≤ x → (irq_called s h p0 p1 p2 p3).mem x = s.mem x) ∧
    (irq_called s h p0 p1 p2 p3).fp = s.fp ∧ (irq_called s h p0 p1 p2 p3).primask = true ∧
    (irq_called s h p0 p1 p2 p3).psp = s.psp ∧ (irq_called s h p0 p1 p2 p3).control = s.control ∧
    (irq_called s h p0 p1 p2 p3).faultmask = s.faultmask := by
  rw [irq_called_eq]
  obtain ⟨a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11⟩ := irq_pro_spec s g hg
  obtain ⟨b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12⟩ :=
    irq_ret_spec (irq_pro s) h p0 p1 p2 p3
  have hfs := frameSize_pos (entryFrame s g)
  refine ⟨b1, b2, b3, b4, by rw [b5, a5], by rw [b6, a1], ?_, ?_, by rw [b8, a6],
    by rw [b9, a10], by rw [b10, a7], by rw [b11, a8], by rw [b12, a11]⟩
  · rw [b6]; exact frameTop_ext b7 a2
  · intro x hx
    rw [b7 x (by omega), a3 x hx]

/-- `tst rz, #0x10` / `vpopeq {s16-s31}` / `pop {r3-r11, lr}`: the three
instructions that take a register record off the main stack. -/
def pops (t : M) (rz : Nat) : M := i_pop_r3_r11_lr (i_vpop_eq (i_tst_r t rz 0x10))

/-- They pop exactly the record at the top of the main stack when the
tested value is the EXC_RETURN the record was pushed under. -/
theorem pops_spec (t : M) (rz : Nat) (f : Frame) (hft : frameTop t.mem t.sp f)
    (hz : f.hasFp = !hasBit (t.r rz) 16) :
    (pops t rz).sp = t.sp + frameSize f ∧
    (∀ i, 3 ≤ i → i ≤ 11 → (pops t rz).r i = f.regs (i - 3)) ∧
    (∀ i, (i < 3 ∨ 11 < i) → (pops t rz).r i = t.r i) ∧
    (pops t rz).lr = f.regs 9 ∧
    (∀ i, (pops t rz).fp i = if f.hasFp = true ∧ i < 16 then f.fpv i else t.fp i) ∧
    (pops t rz).mem = t.mem ∧ (pops t rz).psp = t.psp ∧ (pops t rz).control = t.control ∧
    (pops t rz).primask = t.primask ∧ (pops t rz).faultmask = t.faultmask ∧
    (pops t rz).ipsr = t.ipsr := by
  obtain ⟨hr, hfv, _⟩ := hft
  cases hf : f.hasFp
  · -- no floating-point context
    rw [hf] at hz hr
    have hbit : hasBit (t.r rz) 16 = true := by
      cases hh : hasBit (t.r rz) 16
      · rw [hh] at hz; cases hz
      · rfl
    have hq : pops t rz = i_pop_r3_r11_lr (i_tst_r t rz 0x10) := by
      simp only [pops, i_vpop_eq, i_tst_r, hbit, Bool.not_true, Bool.false_eq_true, ite_false]
    rw [hq]
    simp only [Bool.false_eq_true, ite_false] at hr
    refine ⟨by simp [i_pop_r3_r11_lr, i_tst_r, frameSize, hf], ?_, ?_, ?_, ?_, rfl, rfl, rfl, rfl,
      rfl, rfl⟩
    · intro i h3 h11
      simp only [i_pop_r3_r11_lr, i_tst_r]
      rw [ite_eq_left_of_eq_true _ _ (eq_true ⟨h3, h11⟩)]
      have := hr (i - 3) (by omega)
      rw [← this]
      congr 1
      omega
    · intro i hi
      simp only [i_pop_r3_r11_lr, i_tst_r]
      rw [if_neg (by omega)]
    · simp only [i_pop_r3_r11_lr, i_tst_r]
      have := hr 9 (by omega)
      rw [← this]
      congr 1
    · intro i
      simp [i_pop_r3_r11_lr, i_tst_r]
  · rw [hf] at hz hr
    have hbit : hasBit (t.r rz) 16 = false := by
      cases hh : hasBit (t.r rz) 16
      · rfl
      · rw [hh] at hz; cases hz
    have hq : pops t rz = i_pop_r3_r11_lr (vpop (i_tst_r t rz 0x10)) := by
      simp only [pops, i_vpop_eq, i_tst_r, hbit, Bool.not_false, ite_true]
    rw [hq]
    simp only [ite_true] at hr
    have hfv := hfv hf
    refine ⟨by simp [i_pop_r3_r11_lr, vpop, i_tst_r, frameSize, hf]; omega, ?_, ?_, ?_, ?_, rfl,
      rfl, rfl, rfl, rfl, rfl⟩
    · intro i h3 h11
      simp only [i_pop_r3_r11_lr, vpop, i_tst_r]
      rw [ite_eq_left_of_eq_true _ _ (eq_true ⟨h3, h11⟩)]
      have := hr (i - 3) (by omega)
      rw [← this]
      congr 1
      omega
    · intro i hi
      simp only [i_pop_r3_r11_lr, vpop, i_tst_r]
      rw [if_neg (by omega)]
    · simp only [i_pop_r3_r11_lr, vpop, i_tst_r]
      have := hr 9 (by omega)
      rw [← this]
      congr 1
    · intro i
      simp only [i_pop_r3_r11_lr, vpop, i_tst_r]
      by_cases hi : i < 16
      · rw [if_pos hi, if_pos ⟨trivial, hi⟩]
        exact hfv i hi
      · rw [if_neg hi, if_neg (by omega)]

/-! ## `rsk_irq_entry` -/

theorem irq_restore_eq (t : M) : irq_restore t = i_cmp_imm (pops (i_mov t 12 3) 8) 12 0 := rfl

/-- The path without a dispatch restores what the stub pushed: the stack
pointer, LR, R3 to R11 and S16 to S31 are those of the entry, and nothing
at or above the entry's stack pointer has changed. -/
theorem irq_restore_spec (s h : M) (p0 p1 p2 p3 : Int) :
    (irq_restore (irq_called s h p0 p1 p2 p3)).sp = s.sp ∧
    (irq_restore (irq_called s h p0 p1 p2 p3)).lr = s.lr ∧
    (∀ i, (irq_restore (irq_called s h p0 p1 p2 p3)).fp i = s.fp i) ∧
    (irq_restore (irq_called s h p0 p1 p2 p3)).psp = s.psp ∧
    (irq_restore (irq_called s h p0 p1 p2 p3)).control = s.control ∧
    (∀ i, 3 ≤ i → i ≤ 11 → (irq_restore (irq_called s h p0 p1 p2 p3)).r i = s.r i) ∧
    (irq_restore (irq_called s h p0 p1 p2 p3)).r 12 = p3 ∧
    (irq_restore (irq_called s h p0 p1 p2 p3)).z = decide (p3 = 0) ∧
    (irq_restore (irq_called s h p0 p1 p2 p3)).primask = true ∧
    (irq_restore (irq_called s h p0 p1 p2 p3)).faultmask = s.faultmask ∧
    (∀ x, s.sp ≤ x → (irq_restore (irq_called s h p0 p1 p2 p3)).mem x = s.mem x) := by
  obtain ⟨c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, c11, c12, c13⟩ :=
    irq_called_spec s h p0 p1 p2 p3 0 (Int.le_refl 0)
  rw [irq_restore_eq]
  generalize irq_called s h p0 p1 p2 p3 = t at *
  have t8 : (i_mov t 12 3).r 8 = t.r 8 := by simp [i_mov, setr]
  have t12 : (i_mov t 12 3).r 12 = t.r 3 := by simp [i_mov, setr]
  have hft : frameTop (i_mov t 12 3).mem (i_mov t 12 3).sp (entryFrame s 0) := c7
  have hz : (entryFrame s 0).hasFp = !hasBit ((i_mov t 12 3).r 8) 16 := by rw [t8, c5]; rfl
  obtain ⟨q1, q2, q3, q4, q5, q6, q7, q8, q9, q10, q11⟩ := pops_spec _ 8 _ hft hz
  have hsp : (i_mov t 12 3).sp = t.sp := rfl
  have h12 : (pops (i_mov t 12 3) 8).r 12 = p3 := by rw [q3 12 (Or.inr (by omega)), t12, c4]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, h12, ?_, ?_, ?_, ?_⟩
  · show (pops (i_mov t 12 3) 8).sp = s.sp
    rw [q1, hsp, c6]; omega
  · show (pops (i_mov t 12 3) 8).lr = s.lr
    rw [q4]; simp [entryFrame]
  · intro i
    show (pops (i_mov t 12 3) 8).fp i = s.fp i
    rw [q5]
    split
    · rfl
    · show t.fp i = s.fp i
      rw [c9]
  · show (pops (i_mov t 12 3) 8).psp = s.psp
    rw [q7]; exact c11
  · show (pops (i_mov t 12 3) 8).control = s.control
    rw [q8]; exact c12
  · intro i h3 h11
    show (pops (i_mov t 12 3) 8).r i = s.r i
    rw [q2 i h3 h11]
    simp only [entryFrame]
    rw [if_pos (by omega)]
    congr 1
    omega
  · show decide ((pops (i_mov t 12 3) 8).r 12 = 0) = decide (p3 = 0)
    rw [h12]
  · show (pops (i_mov t 12 3) 8).primask = true
    rw [q9]; exact c10
  · show (pops (i_mov t 12 3) 8).faultmask = s.faultmask
    rw [q10]; exact c13
  · intro x hx
    show (pops (i_mov t 12 3) 8).mem x = s.mem x
    rw [q6]; exact c8 x hx

/-- The dispatch path: the plan's PSP and CONTROL are installed, the
exception return is made with the plan's EXC_RETURN under FAULTMASK with
PRIMASK clear, and the level's register record stays at the top of the main
stack. -/
theorem irq_dispatch_spec (s h : M) (p0 p1 p2 p3 g : Int) (hg : 0 ≤ g) :
    (irq_dispatch (irq_called s h p0 p1 p2 p3)).psp = p0 ∧
    (irq_dispatch (irq_called s h p0 p1 p2 p3)).control = p1 ∧
    (irq_dispatch (irq_called s h p0 p1 p2 p3)).r 2 = p2 ∧
    (irq_dispatch (irq_called s h p0 p1 p2 p3)).primask = false ∧
    (irq_dispatch (irq_called s h p0 p1 p2 p3)).faultmask = true ∧
    (irq_dispatch (irq_called s h p0 p1 p2 p3)).sp = s.sp - frameSize (entryFrame s g) ∧
    frameTop (irq_dispatch (irq_called s h p0 p1 p2 p3)).mem
      (irq_dispatch (irq_called s h p0 p1 p2 p3)).sp (entryFrame s g) ∧
    (∀ x, s.sp ≤ x → (irq_dispatch (irq_called s h p0 p1 p2 p3)).mem x = s.mem x) := by
  obtain ⟨c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, c11, c12, c13⟩ :=
    irq_called_spec s h p0 p1 p2 p3 g hg
  exact ⟨c1, c2, c3, rfl, rfl, c6, c7, c8⟩

/-! ## `rsk_unwind` -/

/-- One round of the loop with the Rust half answering `e` (PSP, CONTROL,
EXC_RETURN, next plan). -/
def unwind_round (m h : M) (e0 e1 e2 e3 : Int) : M :=
  unwind_after_call (call_out4 (unwind_to_call m) h ((unwind_to_call m).r 1) e0 e1 e2 e3)

/-- The round up to the reload of the answer. -/
def unwind_loaded (m h : M) (e0 e1 e2 e3 : Int) : M :=
  i_add_sp (i_ldr_sp (i_ldr_sp (i_ldr_sp (i_ldr_sp
    (call_out4 (unwind_to_call m) h ((unwind_to_call m).r 1) e0 e1 e2 e3) 0 0) 1 4) 2 8) 12 12) 16

theorem unwind_round_eq (m h : M) (e0 e1 e2 e3 : Int) :
    unwind_round m h e0 e1 e2 e3 =
      i_cmp_imm (i_isb (i_msr_control (i_msr_psp (pops (unwind_loaded m h e0 e1 e2 e3) 2) 0) 1))
        12 0 := rfl

/-- The stub's call of `epilogue_core`: the plan it holds in R0, the
out-pointer in its own stack room. -/
theorem unwind_call (m : M) :
    (unwind_to_call m).r 0 = m.r 0 ∧ (unwind_to_call m).r 1 = (unwind_to_call m).sp ∧
    (unwind_to_call m).primask = m.primask := by
  refine ⟨?_, ?_, rfl⟩ <;> simp [unwind_to_call, i_sub_sp, i_mov_r_sp, setr]

set_option maxHeartbeats 4000000 in
theorem unwind_loaded_spec (m h : M) (e0 e1 e2 e3 : Int) :
    (unwind_loaded m h e0 e1 e2 e3).sp = m.sp ∧ (unwind_loaded m h e0 e1 e2 e3).r 0 = e0 ∧
    (unwind_loaded m h e0 e1 e2 e3).r 1 = e1 ∧ (unwind_loaded m h e0 e1 e2 e3).r 2 = e2 ∧
    (unwind_loaded m h e0 e1 e2 e3).r 12 = e3 ∧
    (∀ x, m.sp ≤ x → (unwind_loaded m h e0 e1 e2 e3).mem x = m.mem x) ∧
    (unwind_loaded m h e0 e1 e2 e3).fp = m.fp ∧
    (unwind_loaded m h e0 e1 e2 e3).primask = m.primask ∧
    (unwind_loaded m h e0 e1 e2 e3).faultmask = m.faultmask := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, rfl, rfl, rfl⟩
  all_goals
    simp only [unwind_loaded, unwind_to_call, call_out4, call, i_sub_sp, i_mov_r_sp, i_ldr_sp,
      i_add_sp, setr, wr]
  · omega
  · stub_close
  · stub_close
  · stub_close
  · stub_close
  · intro x hx
    stub_close

/-- One round pops exactly the record at the top of the main stack when the
EXC_RETURN value the Rust half returns is the one that record was pushed
under: R3 to R11, LR and S16 to S31 come back from it, PSP and CONTROL are
the answer's. -/
theorem unwind_round_spec (m h : M) (e0 e1 e2 e3 : Int) (f : Frame)
    (hft : frameTop m.mem m.sp f) (hz : f.hasFp = !hasBit e2 16) :
    (unwind_round m h e0 e1 e2 e3).sp = m.sp + frameSize f ∧
    (∀ i, 3 ≤ i → i ≤ 11 → (unwind_round m h e0 e1 e2 e3).r i = f.regs (i - 3)) ∧
    (unwind_round m h e0 e1 e2 e3).lr = f.regs 9 ∧
    (∀ i, (unwind_round m h e0 e1 e2 e3).fp i
      = if f.hasFp = true ∧ i < 16 then f.fpv i else m.fp i) ∧
    (unwind_round m h e0 e1 e2 e3).psp = e0 ∧ (unwind_round m h e0 e1 e2 e3).control = e1 ∧
    (unwind_round m h e0 e1 e2 e3).r 2 = e2 ∧ (unwind_round m h e0 e1 e2 e3).r 12 = e3 ∧
    (unwind_round m h e0 e1 e2 e3).z = decide (e3 = 0) ∧
    (unwind_round m h e0 e1 e2 e3).primask = m.primask ∧
    (unwind_round m h e0 e1 e2 e3).faultmask = m.faultmask ∧
    (∀ x, m.sp ≤ x → (unwind_round m h e0 e1 e2 e3).mem x = m.mem x) := by
  obtain ⟨a1, a2, a3, a4, a5, a6, a7, a8, a9⟩ := unwind_loaded_spec m h e0 e1 e2 e3
  rw [unwind_round_eq]
  generalize unwind_loaded m h e0 e1 e2 e3 = a at *
  have hft' : frameTop a.mem a.sp f := by rw [a1]; exact frameTop_ext a6 hft
  have hz' : f.hasFp = !hasBit (a.r 2) 16 := by rw [a4]; exact hz
  obtain ⟨q1, q2, q3, q4, q5, q6, q7, q8, q9, q10, q11⟩ := pops_spec a 2 f hft' hz'
  have h12 : (pops a 2).r 12 = e3 := by rw [q3 12 (Or.inr (by omega)), a5]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, h12, ?_, ?_, ?_, ?_⟩
  · show (pops a 2).sp = m.sp + frameSize f
    rw [q1, a1]
  · intro i h3 h11
    exact q2 i h3 h11
  · exact q4
  · intro i
    show (pops a 2).fp i = _
    rw [q5, a7]
  · show (pops a 2).r 0 = e0
    rw [q3 0 (Or.inl (by omega)), a2]
  · show (pops a 2).r 1 = e1
    rw [q3 1 (Or.inl (by omega)), a3]
  · show (pops a 2).r 2 = e2
    rw [q3 2 (Or.inl (by omega)), a4]
  · show decide ((pops a 2).r 12 = 0) = decide (e3 = 0)
    rw [h12]
  · show (pops a 2).primask = m.primask
    rw [q9, a8]
  · show (pops a 2).faultmask = m.faultmask
    rw [q10, a9]
  · intro x hx
    show (pops a 2).mem x = m.mem x
    rw [q6]; exact a6 x hx

/-- **Property 2, the registers.** A dispatch stub entered in state `s`
dispatches (state `d`). Later `rsk_unwind` runs a round from a state `m`
whose main stack is where the dispatch left it (`m.sp = d.sp`, the memory
at and above it unchanged: every record pushed since has been popped), and
the Rust half answers with the EXC_RETURN value the level's record keeps,
which is the LR the stub was entered with. Then the round leaves the
preempted context's R3 to R11 and S16 to S31 as `s` had them, the main
stack pointer and the memory above it as at the entry, LR and R2 the
EXC_RETURN of that context, and PSP and CONTROL the record's. -/
theorem dispatch_unwind_restores (s h m h' : M) (p0 p1 p2 p3 g e0 e1 e3 : Int) (hg : 0 ≤ g)
    (hsp : m.sp = (irq_dispatch (irq_called s h p0 p1 p2 p3)).sp)
    (hmem : ∀ x, m.sp ≤ x → m.mem x = (irq_dispatch (irq_called s h p0 p1 p2 p3)).mem x) :
    (unwind_round m h' e0 e1 s.lr e3).sp = s.sp ∧
    (∀ i, 3 ≤ i → i ≤ 11 → (unwind_round m h' e0 e1 s.lr e3).r i = s.r i) ∧
    (unwind_round m h' e0 e1 s.lr e3).lr = s.lr ∧
    (unwind_round m h' e0 e1 s.lr e3).r 2 = s.lr ∧
    (hasBit s.lr 16 = false → ∀ i, i < 16 → (unwind_round m h' e0 e1 s.lr e3).fp i = s.fp i) ∧
    (hasBit s.lr 16 = true → ∀ i, (unwind_round m h' e0 e1 s.lr e3).fp i = m.fp i) ∧
    (unwind_round m h' e0 e1 s.lr e3).psp = e0 ∧
    (unwind_round m h' e0 e1 s.lr e3).control = e1 ∧
    (∀ x, s.sp ≤ x → (unwind_round m h' e0 e1 s.lr e3).mem x = s.mem x) := by
  obtain ⟨_, _, _, _, _, d6, d7, d8⟩ := irq_dispatch_spec s h p0 p1 p2 p3 g hg
  have hfs := frameSize_pos (entryFrame s g)
  have hft : frameTop m.mem m.sp (entryFrame s g) := by
    rw [hsp]
    exact frameTop_ext (fun x hx => hmem x (by rw [hsp]; exact hx)) d7
  have hz : (entryFrame s g).hasFp = !hasBit s.lr 16 := rfl
  obtain ⟨u1, u2, u3, u4, u5, u6, u7, u8, u9, u10, u11, u12⟩ :=
    unwind_round_spec m h' e0 e1 s.lr e3 _ hft hz
  refine ⟨by rw [u1, hsp, d6]; omega, ?_, ?_, u7, ?_, ?_, u5, u6, ?_⟩
  · intro i h3 h11
    rw [u2 i h3 h11]
    simp only [entryFrame]
    rw [if_pos (by omega)]
    congr 1
    omega
  · rw [u3]; simp [entryFrame]
  · intro hb i hi
    rw [u4]
    simp [entryFrame, hb, hi]
  · intro hb i
    rw [u4]
    simp [entryFrame, hb]
  · intro x hx
    rw [u12 x (by rw [hsp, d6]; omega), hmem x (by rw [hsp, d6]; omega), d8 x hx]

/-- The loop: a round per answer, until a round's next plan is 0. Each
answer is listed with the record its round is to pop. -/
def unwind_loop : M → List (Frame × M × Int × Int × Int × Int) → M
  | m, [] => m
  | m, (_, h, e0, e1, e2, e3) :: rest =>
    if (unwind_round m h e0 e1 e2 e3).z then unwind_exit (unwind_round m h e0 e1 e2 e3)
    else unwind_loop (unwind_again (unwind_round m h e0 e1 e2 e3)) rest

/-- The answers fit the records: each round's EXC_RETURN is the one its
record was pushed under; every round but the last names a next level,
which lies directly above the next record; the last names none. -/
def RoundsOk : List (Frame × M × Int × Int × Int × Int) → Prop
  | [] => False
  | [(f, _, _, _, e2, e3)] => f.hasFp = !hasBit e2 16 ∧ e3 = 0
  | (f, _, _, _, e2, e3) :: r :: rs =>
    f.hasFp = !hasBit e2 16 ∧ e3 ≠ 0 ∧ f.gap = 0 ∧ RoundsOk (r :: rs)

/-- `rsk_unwind` over any number of levels: with the records of the levels
it is to unwind at the top of the main stack, it pops exactly those, and
leaves by an exception return (PRIMASK clear, FAULTMASK set) with the PSP,
CONTROL and EXC_RETURN of the last answer and R3 to R11 and LR of the last
record popped; the older records are still on the stack above it. -/
theorem unwind_loop_frames :
    ∀ (rounds : List (Frame × M × Int × Int × Int × Int)) (m : M) (rest : List Frame),
      RoundsOk rounds → framesAt m.mem m.sp (rounds.map Prod.fst ++ rest) →
      ∃ f h e0 e1 e2 e3, rounds.getLast? = some (f, h, e0, e1, e2, e3) ∧
        (unwind_loop m rounds).psp = e0 ∧ (unwind_loop m rounds).control = e1 ∧
        (unwind_loop m rounds).r 2 = e2 ∧
        (∀ i, 3 ≤ i → i ≤ 11 → (unwind_loop m rounds).r i = f.regs (i - 3)) ∧
        (unwind_loop m rounds).lr = f.regs 9 ∧
        (unwind_loop m rounds).primask = false ∧ (unwind_loop m rounds).faultmask = true ∧
        framesAt (unwind_loop m rounds).mem ((unwind_loop m rounds).sp + f.gap) rest := by
  intro rounds
  induction rounds with
  | nil => intro m rest hok _; exact absurd hok (by simp [RoundsOk])
  | cons x xs ih =>
    intro m rest hok hfr
    obtain ⟨f, h, e0, e1, e2, e3⟩ := x
    have hfr0 : framesAt m.mem m.sp (f :: (xs.map Prod.fst ++ rest)) := hfr
    obtain ⟨hft, hfr'⟩ := hfr0
    cases xs with
    | nil =>
      obtain ⟨hz, he3⟩ := hok
      obtain ⟨u1, u2, u3, u4, u5, u6, u7, u8, u9, u10, u11, u12⟩ :=
        unwind_round_spec m h e0 e1 e2 e3 f hft hz
      have hzt : (unwind_round m h e0 e1 e2 e3).z = true := by rw [u9]; simp [he3]
      have hl : unwind_loop m [(f, h, e0, e1, e2, e3)] = unwind_exit (unwind_round m h e0 e1 e2 e3) := by
        simp [unwind_loop, hzt]
      rw [hl]
      refine ⟨f, h, e0, e1, e2, e3, rfl, u5, u6, u7, u2, u3, rfl, rfl, ?_⟩
      show framesAt (unwind_round m h e0 e1 e2 e3).mem ((unwind_round m h e0 e1 e2 e3).sp + f.gap) rest
      rw [u1]
      have hg := hft.2.2
      have hfs := frameSize_pos f
      exact framesAt_ext rest _ (fun x hx => u12 x (by omega)) hfr'
    | cons y ys =>
      obtain ⟨hz, he3, hgap, hrest⟩ := hok
      obtain ⟨u1, u2, u3, u4, u5, u6, u7, u8, u9, u10, u11, u12⟩ :=
        unwind_round_spec m h e0 e1 e2 e3 f hft hz
      have hzt : (unwind_round m h e0 e1 e2 e3).z = false := by rw [u9]; simp [he3]
      have hl : unwind_loop m ((f, h, e0, e1, e2, e3) :: y :: ys)
          = unwind_loop (unwind_again (unwind_round m h e0 e1 e2 e3)) (y :: ys) := by
        simp [unwind_loop, hzt]
      rw [hl]
      have hfs := frameSize_pos f
      have hfr2 : framesAt (unwind_again (unwind_round m h e0 e1 e2 e3)).mem
          (unwind_again (unwind_round m h e0 e1 e2 e3)).sp ((y :: ys).map Prod.fst ++ rest) := by
        show framesAt (unwind_round m h e0 e1 e2 e3).mem (unwind_round m h e0 e1 e2 e3).sp _
        rw [u1]
        have : m.sp + frameSize f = m.sp + frameSize f + f.gap := by omega
        rw [this]
        exact framesAt_ext _ _ (fun x hx => u12 x (by omega)) hfr'
      obtain ⟨f', h', a0, a1, a2, a3, hlast, r⟩ := ih _ rest hrest hfr2
      exact ⟨f', h', a0, a1, a2, a3, by rw [List.getLast?_cons_cons]; exact hlast, r⟩

/-! ## The Kernel-level handlers: SVC, faults, the timer -/

/-- What every Kernel-level handler's stub does after its Rust half
returned: `pop {r4, lr}` gives back R4 and LR, and the stack pointer is the
entry's. `c` is the state at the call, `t` the state the call returned. -/
theorem kentry_tail (s c t : M) (h1 : c.sp = s.sp - 8) (h2 : c.mem c.sp = s.r 4)
    (h3 : c.mem (c.sp + 4) = s.lr) (h4 : t.sp = c.sp) (h5 : t.mem c.sp = c.mem c.sp)
    (h6 : t.mem (c.sp + 4) = c.mem (c.sp + 4)) (h7 : ∀ i, 5 ≤ i → i ≤ 11 → t.r i = s.r i) :
    (kentry_after_call t).sp = s.sp ∧ (kentry_after_call t).lr = s.lr ∧
    (∀ i, 4 ≤ i → i ≤ 11 → (kentry_after_call t).r i = s.r i) ∧
    (kentry_after_call t).r 0 = t.r 0 ∧ (kentry_after_call t).fp = t.fp ∧
    (kentry_after_call t).psp = t.psp ∧ (kentry_after_call t).control = t.control ∧
    (kentry_after_call t).mem = t.mem := by
  refine ⟨?_, ?_, ?_, rfl, rfl, rfl, rfl, rfl⟩
  · show t.sp + 8 = s.sp
    omega
  · show t.mem (t.sp + 4) = s.lr
    rw [h4, h6, h3]
  · intro i h4' h11
    show (if i = 4 then t.mem t.sp else t.r i) = s.r i
    split
    · rename_i hi; subst hi; rw [h4, h5, h2]
    · exact h7 i (by omega) h11

/-- The same for a handler whose Rust half returns its plan in R0. -/
theorem kentry_ret_tail (s c h : M) (v : Int) (h1 : c.sp = s.sp - 8) (h2 : c.mem c.sp = s.r 4)
    (h3 : c.mem (c.sp + 4) = s.lr) (h7 : ∀ i, 5 ≤ i → i ≤ 11 → c.r i = s.r i) :
    (kentry_after_call (call_ret c h v)).sp = s.sp ∧
    (kentry_after_call (call_ret c h v)).lr = s.lr ∧
    (∀ i, 4 ≤ i → i ≤ 11 → (kentry_after_call (call_ret c h v)).r i = s.r i) ∧
    (kentry_after_call (call_ret c h v)).r 0 = v ∧
    (kentry_after_call (call_ret c h v)).fp = c.fp ∧
    (kentry_after_call (call_ret c h v)).psp = c.psp ∧
    (kentry_after_call (call_ret c h v)).control = c.control := by
  have t4 : (call_ret c h v).sp = c.sp := rfl
  have t5 : (call_ret c h v).mem c.sp = c.mem c.sp := by simp [call_ret, call, setr]
  have t6 : (call_ret c h v).mem (c.sp + 4) = c.mem (c.sp + 4) := by simp [call_ret, call, setr]
  have t7 : ∀ i, 5 ≤ i → i ≤ 11 → (call_ret c h v).r i = s.r i := by
    intro i h5 h11
    have : (call_ret c h v).r i = c.r i := by
      simp only [call_ret, call, setr]
      rw [if_neg (by omega), if_pos ⟨by omega, h11⟩]
    rw [this]; exact h7 i h5 h11
  obtain ⟨k1, k2, k3, k4, k5, k6, k7, _⟩ := kentry_tail s c (call_ret c h v) h1 h2 h3 t4 t5 t6 t7
  refine ⟨k1, k2, k3, ?_, k5, k6, k7⟩
  rw [k4]; simp [call_ret, setr]

/-- `rsk_timer_entry`: the registers a handler must preserve are the
entry's when it returns or branches to `rsk_unwind`, and R0 is the plan. -/
theorem timer_stub (s h : M) (hop : Int) :
    (kentry_after_call (call_ret (timer_to_call s) h hop)).sp = s.sp ∧
    (kentry_after_call (call_ret (timer_to_call s) h hop)).lr = s.lr ∧
    (∀ i, 4 ≤ i → i ≤ 11 → (kentry_after_call (call_ret (timer_to_call s) h hop)).r i = s.r i) ∧
    (kentry_after_call (call_ret (timer_to_call s) h hop)).r 0 = hop ∧
    (kentry_after_call (call_ret (timer_to_call s) h hop)).fp = s.fp ∧
    (kentry_after_call (call_ret (timer_to_call s) h hop)).psp = s.psp ∧
    (kentry_after_call (call_ret (timer_to_call s) h hop)).control = s.control :=
  kentry_ret_tail s (timer_to_call s) h hop rfl (by simp [timer_to_call, i_push_r4_lr])
    (by simp [timer_to_call, i_push_r4_lr]) (fun _ _ _ => rfl)

/-- `rsk_busfault_entry` and `rsk_usagefault_entry`: R0 at the call says
whether the fault is the Kernel's (taken from Handler mode or with PRIMASK
set); the registers are preserved as by the timer's stub. -/
theorem fault_stub (s h : M) (hop : Int) :
    ((fault_to_call s).r 0 ≠ 0 ↔ (s.primask = true ∨ hasBit s.lr 8 = false)) ∧
    (kentry_after_call (call_ret (fault_to_call s) h hop)).sp = s.sp ∧
    (kentry_after_call (call_ret (fault_to_call s) h hop)).lr = s.lr ∧
    (∀ i, 4 ≤ i → i ≤ 11 → (kentry_after_call (call_ret (fault_to_call s) h hop)).r i = s.r i) ∧
    (kentry_after_call (call_ret (fault_to_call s) h hop)).r 0 = hop ∧
    (kentry_after_call (call_ret (fault_to_call s) h hop)).fp = s.fp ∧
    (kentry_after_call (call_ret (fault_to_call s) h hop)).psp = s.psp ∧
    (kentry_after_call (call_ret (fault_to_call s) h hop)).control = s.control := by
  have hc : (fault_to_call s).sp = s.sp - 8 ∧ (fault_to_call s).mem (fault_to_call s).sp = s.r 4 ∧
      (fault_to_call s).mem ((fault_to_call s).sp + 4) = s.lr ∧
      (∀ i, 5 ≤ i → i ≤ 11 → (fault_to_call s).r i = s.r i) ∧
      (fault_to_call s).fp = s.fp ∧ (fault_to_call s).psp = s.psp ∧
      (fault_to_call s).control = s.control ∧
      ((fault_to_call s).r 0 ≠ 0 ↔ (s.primask = true ∨ hasBit s.lr 8 = false)) := by
    cases hb : hasBit s.lr 8 <;> cases hp : s.primask <;>
      simp [fault_to_call, i_push_r4_lr, i_mrs_primask, i_tst_lr, i_mov_imm_eq, setr, hb, hp] <;>
      (intro i h5 _; first | (split_ifs <;> first | rfl | omega) | (intro h0; omega) | omega)
  obtain ⟨c1, c2, c3, c4, c5, c6, c7, c8⟩ := hc
  obtain ⟨k1, k2, k3, k4, k5, k6, k7⟩ := kentry_ret_tail s (fault_to_call s) h hop c1 c2 c3 c4
  exact ⟨c8, k1, k2, k3, k4, by rw [k5, c5], by rw [k6, c6], by rw [k7, c7]⟩

/-- `rsk_memmanage_entry`: as the other fault stubs, with the address of
the faulting context's frame in R1 (its PSP, or the main stack pointer of
the entry) and the entry's LR in R2. -/
theorem memmanage_stub (s h : M) (hop : Int) :
    ((mm_to_call s).r 0 ≠ 0 ↔ (s.primask = true ∨ hasBit s.lr 8 = false)) ∧
    (mm_to_call s).r 1 = (if hasBit s.lr 4 = true then s.psp else s.sp) ∧
    (mm_to_call s).r 2 = s.lr ∧
    (kentry_after_call (call_ret (mm_to_call s) h hop)).sp = s.sp ∧
    (kentry_after_call (call_ret (mm_to_call s) h hop)).lr = s.lr ∧
    (∀ i, 4 ≤ i → i ≤ 11 → (kentry_after_call (call_ret (mm_to_call s) h hop)).r i = s.r i) ∧
    (kentry_after_call (call_ret (mm_to_call s) h hop)).r 0 = hop ∧
    (kentry_after_call (call_ret (mm_to_call s) h hop)).fp = s.fp ∧
    (kentry_after_call (call_ret (mm_to_call s) h hop)).psp = s.psp ∧
    (kentry_after_call (call_ret (mm_to_call s) h hop)).control = s.control := by
  have hc : (mm_to_call s).sp = s.sp - 8 ∧ (mm_to_call s).mem (mm_to_call s).sp = s.r 4 ∧
      (mm_to_call s).mem ((mm_to_call s).sp + 4) = s.lr ∧
      (∀ i, 5 ≤ i → i ≤ 11 → (mm_to_call s).r i = s.r i) ∧
      (mm_to_call s).fp = s.fp ∧ (mm_to_call s).psp = s.psp ∧
      (mm_to_call s).control = s.control ∧
      ((mm_to_call s).r 0 ≠ 0 ↔ (s.primask = true ∨ hasBit s.lr 8 = false)) ∧
      (mm_to_call s).r 1 = (if hasBit s.lr 4 = true then s.psp else s.sp) ∧
      (mm_to_call s).r 2 = s.lr := by
    cases hb : hasBit s.lr 8 <;> cases hb4 : hasBit s.lr 4 <;> cases hp : s.primask <;>
      simp [mm_to_call, i_push_r4_lr, i_mrs_primask, i_tst_lr, i_mov_imm_eq, i_mrs_psp_ne,
        i_add_r_sp_eq, i_mov_r_lr, setr, hb, hb4, hp] <;>
      (intro i h5 _; first | (split_ifs <;> first | rfl | omega) | (intro h0; omega) | omega)
  obtain ⟨c1, c2, c3, c4, c5, c6, c7, c8, c9, c10⟩ := hc
  obtain ⟨k1, k2, k3, k4, k5, k6, k7⟩ := kentry_ret_tail s (mm_to_call s) h hop c1 c2 c3 c4
  exact ⟨c8, c9, c10, k1, k2, k3, k4, by rw [k5, c5], by rw [k6, c6], by rw [k7, c7]⟩

/-- `rsk_svc_entry`: the Rust half is passed the address of the caller's
exception frame (the PSP for a Job, the main stack pointer otherwise) and
the entry's LR; the registers are preserved as by the other Kernel-level
stubs, whatever the service writes into the caller's frame. -/
theorem svc_stub (s h : M) (hop : Int) :
    (svc_to_call s).r 0 = (if hasBit s.lr 4 = true then s.psp else s.sp) ∧
    (svc_to_call s).r 1 = s.lr ∧
    (kentry_after_call (call_svc (svc_to_call s) h ((svc_to_call s).r 0) (!hasBit s.lr 4) hop)).sp
      = s.sp ∧
    (kentry_after_call (call_svc (svc_to_call s) h ((svc_to_call s).r 0) (!hasBit s.lr 4) hop)).lr
      = s.lr ∧
    (∀ i, 4 ≤ i → i ≤ 11 →
      (kentry_after_call (call_svc (svc_to_call s) h ((svc_to_call s).r 0) (!hasBit s.lr 4) hop)).r i
        = s.r i) ∧
    (kentry_after_call (call_svc (svc_to_call s) h ((svc_to_call s).r 0) (!hasBit s.lr 4) hop)).r 0
      = hop ∧
    (kentry_after_call (call_svc (svc_to_call s) h ((svc_to_call s).r 0) (!hasBit s.lr 4) hop)).fp
      = s.fp := by
  have hc : (svc_to_call s).sp = s.sp - 8 ∧ (svc_to_call s).mem (svc_to_call s).sp = s.r 4 ∧
      (svc_to_call s).mem ((svc_to_call s).sp + 4) = s.lr ∧
      (∀ i, 4 ≤ i → i ≤ 11 → (svc_to_call s).r i = s.r i) ∧
      (svc_to_call s).fp = s.fp ∧
      (svc_to_call s).r 0 = (if hasBit s.lr 4 = true then s.psp else s.sp) ∧
      (svc_to_call s).r 1 = s.lr := by
    cases hb4 : hasBit s.lr 4 <;>
      simp [svc_to_call, i_tst_lr, i_mrs_msp_eq, i_mrs_psp_ne, i_mov_r_lr, i_push_r4_lr, setr, hb4] <;>
      (intro i h4 _; first | (split_ifs <;> first | rfl | omega) | (intro h0; omega) | omega)
  obtain ⟨c1, c2, c3, c4, c5, c6, c7⟩ := hc
  generalize hc0 : svc_to_call s = c at *
  have t4 : (call_svc c h (c.r 0) (!hasBit s.lr 4) hop).sp = c.sp := rfl
  have t5 : (call_svc c h (c.r 0) (!hasBit s.lr 4) hop).mem c.sp = c.mem c.sp := by
    have hne : ¬ ((!hasBit s.lr 4) = true ∧ c.r 0 ≤ c.sp ∧ c.sp < c.r 0 + 32) := by
      rw [c6]; cases hb4 : hasBit s.lr 4 <;> simp <;> first | omega | (intros; omega)
    simp only [call_svc, call, setr]
    rw [if_neg hne, if_pos (le_refl _)]
  have t6 : (call_svc c h (c.r 0) (!hasBit s.lr 4) hop).mem (c.sp + 4) = c.mem (c.sp + 4) := by
    have hne : ¬ ((!hasBit s.lr 4) = true ∧ c.r 0 ≤ c.sp + 4 ∧ c.sp + 4 < c.r 0 + 32) := by
      rw [c6]; cases hb4 : hasBit s.lr 4 <;> simp <;> first | omega | (intros; omega)
    simp only [call_svc, call, setr]
    rw [if_neg hne, if_pos (by omega)]
  have t7 : ∀ i, 5 ≤ i → i ≤ 11 → (call_svc c h (c.r 0) (!hasBit s.lr 4) hop).r i = s.r i := by
    intro i h5 h11
    have : (call_svc c h (c.r 0) (!hasBit s.lr 4) hop).r i = c.r i := by
      simp only [call_svc, call, setr]
      rw [if_neg (by omega), if_pos ⟨by omega, h11⟩]
    rw [this]; exact c4 i (by omega) h11
  obtain ⟨k1, k2, k3, k4, k5, _, _, _⟩ :=
    kentry_tail s c (call_svc c h (c.r 0) (!hasBit s.lr 4) hop) c1 c2 c3 t4 t5 t6 t7
  refine ⟨c6, c7, k1, k2, k3, ?_, ?_⟩
  · rw [k4]; simp [call_svc, setr]
  · rw [k5]; exact c5

/-! ## The exception model at a handler, and the entry-instruction window (5.8a) -/

/-- HW-06: exception `n` taken while a handler executes (Handler mode, on
the main stack) in state `s`. The processor stacks the interrupted
handler's frame on the main stack (`g` bytes: R0 to R3, R12 and LR in its
first six words; the rest is not modelled, `hx`), sets LR to the
Handler-mode EXC_RETURN value and IPSR to `n`, and leaves every other
register as it is. -/
def exc_enter_handler (s : M) (n g : Int) (hx : M) : M :=
  { s with
    sp := s.sp - g, lr := 0xFFFFFFF1, ipsr := n,
    mem := fun x =>
      if x = s.sp - g then s.r 0 else if x = s.sp - g + 4 then s.r 1
      else if x = s.sp - g + 8 then s.r 2 else if x = s.sp - g + 12 then s.r 3
      else if x = s.sp - g + 16 then s.r 12 else if x = s.sp - g + 20 then s.lr
      else if s.sp - g ≤ x ∧ x < s.sp then hx.mem x else s.mem x }

/-- HW-07: the exception return to a handler from state `f`, whose main
stack pointer is at the frame the entry stacked. -/
def exc_return_handler (f : M) (g ipsr : Int) : M :=
  { f with
    sp := f.sp + g, lr := f.mem (f.sp + 20), ipsr := ipsr, faultmask := false,
    r := fun i =>
      if i = 0 then f.mem f.sp else if i = 1 then f.mem (f.sp + 4)
      else if i = 2 then f.mem (f.sp + 8) else if i = 3 then f.mem (f.sp + 12)
      else if i = 12 then f.mem (f.sp + 16) else f.r i }

/-- The entry-instruction window, an interrupt vector's case. A handler in
state `s` is preempted by an interrupt vector whose Rust half plans neither
a dispatch nor an unwind (what `irq_entry_core` answers for an entry from
Handler mode). That stub returns as entered, and the exception return gives
the preempted handler back every register, its stack pointer, PSP, CONTROL,
S16 to S31, and the memory at and above its stack pointer, with both masks
clear. -/
theorem nested_irq_resumed (s hx h : M) (n g p1 p2 : Int) (hg : 32 ≤ g) :
    (∀ i, i ≤ 12 → (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 p1 p2 0))) g s.ipsr).r i = s.r i) ∧
    (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 p1 p2 0))) g s.ipsr).sp = s.sp ∧
    (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 p1 p2 0))) g s.ipsr).lr = s.lr ∧
    (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 p1 p2 0))) g s.ipsr).psp = s.psp ∧
    (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 p1 p2 0))) g s.ipsr).control = s.control ∧
    (∀ i, (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 p1 p2 0))) g s.ipsr).fp i = s.fp i) ∧
    (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 p1 p2 0))) g s.ipsr).ipsr = s.ipsr ∧
    (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 p1 p2 0))) g s.ipsr).primask = false ∧
    (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 p1 p2 0))) g s.ipsr).faultmask = false ∧
    (∀ x, s.sp ≤ x → (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 p1 p2 0))) g s.ipsr).mem x = s.mem x) := by
  obtain ⟨u1, u2, u3, u4, u5, u6, u7, u8, u9, u10, u11⟩ :=
    irq_restore_spec (exc_enter_handler s n g hx) h 0 p1 p2 0
  have esp : (exc_enter_handler s n g hx).sp = s.sp - g := rfl
  have m0 : (exc_enter_handler s n g hx).mem (s.sp - g) = s.r 0 := by simp [exc_enter_handler]
  have m1 : (exc_enter_handler s n g hx).mem (s.sp - g + 4) = s.r 1 := by simp [exc_enter_handler]
  have m2 : (exc_enter_handler s n g hx).mem (s.sp - g + 8) = s.r 2 := by simp [exc_enter_handler]
  have m3 : (exc_enter_handler s n g hx).mem (s.sp - g + 12) = s.r 3 := by simp [exc_enter_handler]
  have m4 : (exc_enter_handler s n g hx).mem (s.sp - g + 16) = s.r 12 := by simp [exc_enter_handler]
  have m5 : (exc_enter_handler s n g hx).mem (s.sp - g + 20) = s.lr := by simp [exc_enter_handler]
  have mab : ∀ x, s.sp ≤ x → (exc_enter_handler s n g hx).mem x = s.mem x := by
    intro x hx'
    simp only [exc_enter_handler]
    split_ifs <;> first | rfl | omega
  rw [esp] at u1 u11
  generalize irq_restore (irq_called (exc_enter_handler s n g hx) h 0 p1 p2 0) = u at *
  refine ⟨?_, ?_, ?_, u4, u5, u3, rfl, rfl, rfl, ?_⟩
  · intro i hi
    show (if i = 0 then u.mem u.sp else if i = 1 then u.mem (u.sp + 4)
      else if i = 2 then u.mem (u.sp + 8) else if i = 3 then u.mem (u.sp + 12)
      else if i = 12 then u.mem (u.sp + 16) else u.r i) = s.r i
    split_ifs with h0 h1 h2 h3 h12
    · subst h0; rw [u1, u11 _ (by omega), m0]
    · subst h1; rw [u1, u11 _ (by omega), m1]
    · subst h2; rw [u1, u11 _ (by omega), m2]
    · subst h3; rw [u1, u11 _ (by omega), m3]
    · subst h12; rw [u1, u11 _ (by omega), m4]
    · exact u6 i (by omega) (by omega)
  · show u.sp + g = s.sp
    rw [u1]; omega
  · show u.mem (u.sp + 20) = s.lr
    rw [u1, u11 _ (by omega), m5]
  · intro x hx'
    show u.mem x = s.mem x
    rw [u11 x (by omega), mab x hx']

/-- The Handler-mode EXC_RETURN value names neither Thread mode nor the
process stack. -/
theorem handler_exc_return : hasBit 0xFFFFFFF1 0x8 = false ∧ hasBit 0xFFFFFFF1 0x4 = false := by
  constructor <;> decide

end Stub
