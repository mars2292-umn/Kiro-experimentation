import Sched.ArchIrq
open Aeneas Aeneas.Std Result

/-! # The entry cores and what they write to the hardware (2026-11-25)

Verus proves the entry cores of Kernel_Arch against a ghost model of the
hardware (`entry::HwModel`, stepped by `entry::step` at each
`KernelArch::apply`). Ghost code is erased from the compiled crate, so
here the model is restated for the fields these statements read (`HV`,
`hstep`) and the trait's contract is a class over any implementation of
the extracted `KernelArch` (`HSpec`: an action steps the model, the other
methods leave it), as `KSpec` is for `KernelOps`. The statements are then
proven of the cores as Aeneas extracts them, each of the form "if the core
returns":

- `HQ`: from one hardware state to another PRIMASK is as it was, no action
  has cleared it, and the watchdog has not been fed. Every function on the
  supervisor call's path keeps it (`svc_core_hq`: **a supervisor call does
  not touch PRIMASK and does not feed the watchdog**, R40.6), and so does
  `apply_core`, the last steps of every entry.
- `apply_core_a1`: **after the last steps of an entry BASEPRI is the value
  the Kernel's state calls for** (A1, DD-01, HW-03), read through the
  Kernel's own `basepri_now` on the state `apply_core` returns, or 0 when
  the Kernel is in its safe state.

Elsewhere since 2026-11-26: A1 after the dispatch, the unwind, a dispatch
vector and a supervisor call (`Sched/ArchHwA1.lean`,
`Sched/ArchHwA1Svc.lean`); the vector lines `boot_vectors` sets, the
start's frame and the floating-point access (`Sched/ArchHwBoot.lean`).
What is not in Lean (Verus only): A1 over an interrupt entry, and the
lines still so at `boot_core`'s return. That an
implementation of `KernelArch` satisfies `HSpec` is the trait's contract,
as in Verus; for the Target's implementation it rests on the register
drivers (`Hw/Model.lean`) and the trusted leaves. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T}

/-- What the Kernel has written to the hardware, as far as these
statements read it (`entry::HwModel`): BASEPRI, PRIMASK, whether an action
has cleared PRIMASK, and whether the watchdog has been fed. -/
structure HV where
  basepri : Nat
  primask : Bool
  pcl : Bool
  wfed : Bool

/-- The model's step for one action (`entry::step`, these fields). -/
def hstep (h : HV) : arch.cm4.entry.Action → HV
  | .SetBasepri v => { h with basepri := v.val }
  | .PrimaskSet => { h with primask := true }
  | .PrimaskClear => { h with primask := false, pcl := true }
  | .WatchdogFeed => { h with wfed := true }
  | _ => h

/-- The contract of the `KernelArch` trait (Verus: the `ensures` of its
methods over `hw()`): an action steps the model, and the other methods
that return the hardware leave it. -/
class HSpec (ai : arch.cm4.entry.KernelArch A) (hv : A → HV) : Prop where
  apply : ∀ {hw act hw'}, ai.apply hw act = ok hw' → hv hw' = hstep (hv hw) act
  svc_clear_pending : ∀ {hw hw'}, ai.svc_clear_pending hw = ok hw' → hv hw' = hv hw
  stacking_fault_discard : ∀ {hw hw'}, ai.stacking_fault_discard hw = ok hw' → hv hw' = hv hw
  copy_in : ∀ {hw a b c x hw'}, ai.copy_in hw a b c = ok (x, hw') → hv hw' = hv hw
  copy_out : ∀ {hw a b c x hw'}, ai.copy_out hw a b c = ok (x, hw') → hv hw' = hv hw

/-- An action that neither sets nor clears PRIMASK and does not feed the
watchdog. -/
def QuietAct : arch.cm4.entry.Action → Prop
  | .PrimaskSet => False
  | .PrimaskClear => False
  | .WatchdogFeed => False
  | _ => True

/-- An action that does not write BASEPRI. -/
def NoBp : arch.cm4.entry.Action → Prop
  | .SetBasepri _ => False
  | _ => True

variable {hv : A → HV}

/-- From `hw` to `hw'` PRIMASK is as it was, no action has cleared it, and
the watchdog has not been fed. -/
def HQ (hv : A → HV) (hw' hw : A) : Prop :=
  (hv hw').primask = (hv hw).primask ∧ (hv hw').pcl = (hv hw).pcl ∧ (hv hw').wfed = (hv hw).wfed

theorem HQ.refl (hv : A → HV) (hw : A) : HQ hv hw hw := ⟨rfl, rfl, rfl⟩

theorem HQ.trans {a b c : A} (h1 : HQ hv b a) (h2 : HQ hv c b) : HQ hv c a :=
  ⟨h2.1.trans h1.1, h2.2.1.trans h1.2.1, h2.2.2.trans h1.2.2⟩

theorem HQ.of_eq {hw hw' : A} (h : hv hw' = hv hw) : HQ hv hw' hw := by
  unfold HQ; rw [h]; exact ⟨rfl, rfl, rfl⟩

theorem HQ.apply [HSpec ai hv] {hw hw' : A} {act : arch.cm4.entry.Action}
    (h : ai.apply hw act = ok hw') (hq : QuietAct act) : HQ hv hw' hw := by
  have e := HSpec.apply (hv := hv) h
  unfold HQ
  rw [e]
  cases act <;> first
    | exact ⟨rfl, rfl, rfl⟩
    | exact absurd hq (by simp [QuietAct])

/-- From `hw` to `hw'` BASEPRI is as it was. -/
def HB (hv : A → HV) (hw' hw : A) : Prop := (hv hw').basepri = (hv hw).basepri

theorem HB.refl (hv : A → HV) (hw : A) : HB hv hw hw := rfl

theorem HB.trans {a b c : A} (h1 : HB hv b a) (h2 : HB hv c b) : HB hv c a :=
  Eq.trans h2 h1

theorem HB.apply [HSpec ai hv] {hw hw' : A} {act : arch.cm4.entry.Action}
    (h : ai.apply hw act = ok hw') (hq : NoBp act) : HB hv hw' hw := by
  have e := HSpec.apply (hv := hv) h
  unfold HB
  rw [e]
  cases act <;> first
    | rfl
    | exact absurd hq (by simp [NoBp])

/-- A call of a function already shown to keep `HQ`. -/
syntax "hq_call" : tactic
macro_rules | `(tactic| hq_call) => `(tactic| fail "no call")

macro "hq_reg " l:ident : command =>
  `(macro_rules | `(tactic| hq_call) => `(tactic| refine HQ.trans ?_ ($l (by assumption))))

macro_rules
  | `(tactic| hq_call) => `(tactic| refine HQ.trans ?_ (HQ.apply (by assumption) (by exact True.intro)))
macro_rules
  | `(tactic| hq_call) => `(tactic| refine HQ.trans ?_ (HQ.of_eq (HSpec.svc_clear_pending (by assumption))))
macro_rules
  | `(tactic| hq_call) => `(tactic| refine HQ.trans ?_ (HQ.of_eq (HSpec.stacking_fault_discard (by assumption))))
macro_rules
  | `(tactic| hq_call) => `(tactic| refine HQ.trans ?_ (HQ.of_eq (HSpec.copy_in (by assumption))))
macro_rules
  | `(tactic| hq_call) => `(tactic| refine HQ.trans ?_ (HQ.of_eq (HSpec.copy_out (by assumption))))

/-- Closes `HQ hv hw' hw` after `peel`. -/
syntax "hq_close" : tactic
macro_rules
  | `(tactic| hq_close) => `(tactic| repeat (first
      | exact HQ.refl _ _
      | assumption
      | hq_call
      | dsimp only))

/-- The cases of a core after `peel`, as in the frame proofs of
`Sched/ArchEntry.lean`. -/
macro "hq_core " h:ident : tactic => `(tactic| (all_goals first
    | (hq_close; done)
    | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
        | (hq_close; done)
        | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
            | (hq_close; done)
            | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;>
                (hq_close; done))))))

/-! ## The compare and the EasyDMA stop -/

theorem arm_compare_hq [HSpec ai hv] {hw hw' : A} {at_ : U64}
    (h : arch.cm4.entry.arm_compare ai hw at_ = ok hw') : HQ hv hw' hw := by
  unfold arch.cm4.entry.arm_compare at h
  peel h
  hq_core h

hq_reg arm_compare_hq

theorem rearm_compare_hq [HSpec ai hv] {hw hw' : A} {at_ : U64}
    (h : arch.cm4.entry.rearm_compare ai hw at_ = ok hw') : HQ hv hw' hw := by
  unfold arch.cm4.entry.rearm_compare at h
  peel h
  hq_core h

hq_reg rearm_compare_hq

theorem dma_stop_partition_loop_hq [HSpec ai hv] {hw hw' : A} {tables : T} {p : U8} {n i : Usize}
    (h : arch.cm4.entry.dma_stop_partition_loop ai ti hw tables p n i = ok hw') : HQ hv hw' hw := by
  unfold arch.cm4.entry.dma_stop_partition_loop at h
  refine loop_inv _ (fun x => HQ hv x.1 hw) (fun r => HQ hv r hw) ?_ ?_ _ hw' (HQ.refl _ _) h
  · rintro ⟨hw1, c⟩ ⟨hw2, c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.dma_stop_partition_loop.body at hb
    peel hb
    hq_core hb
  · rintro ⟨hw1, c⟩ hw2 hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.dma_stop_partition_loop.body at hb
    peel hb
    hq_core hb

hq_reg dma_stop_partition_loop_hq

theorem dma_stop_partition_hq [HSpec ai hv] {hw hw' : A} {tables : T} {p : U8}
    (h : arch.cm4.entry.dma_stop_partition ai ti hw tables p = ok hw') : HQ hv hw' hw := by
  unfold arch.cm4.entry.dma_stop_partition at h
  peel h
  hq_core h

hq_reg dma_stop_partition_hq

/-! ## `apply_core`: the last steps of every entry -/

theorem apply_core_loop0_hq [HSpec ai hv] {k : K} {hw hw' : A} {tables : T} {n t : Usize}
    (h : arch.cm4.entry.apply_core_loop0 ki ai ti k hw tables n t = ok hw') : HQ hv hw' hw := by
  unfold arch.cm4.entry.apply_core_loop0 at h
  refine loop_inv _ (fun x => HQ hv x.1 hw) (fun r => HQ hv r hw) ?_ ?_ _ hw' (HQ.refl _ _) h
  · rintro ⟨hw1, c⟩ ⟨hw2, c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.apply_core_loop0.body at hb
    peel hb
    hq_core hb
  · rintro ⟨hw1, c⟩ hw2 hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.apply_core_loop0.body at hb
    peel hb
    hq_core hb

hq_reg apply_core_loop0_hq

theorem apply_core_loop1_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {np p : Usize}
    (h : arch.cm4.entry.apply_core_loop1 ki ai ti k hw tables s np p = ok (k', hw', s')) :
    HQ hv hw' hw := by
  unfold arch.cm4.entry.apply_core_loop1 at h
  refine loop_inv _ (fun x => HQ hv x.2.1 hw) (fun r => HQ hv r.2.1 hw) ?_ ?_ _ (k', hw', s')
    (HQ.refl _ _) h
  · rintro ⟨k1, hw1, st, c⟩ ⟨k2, hw2, st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.apply_core_loop1.body at hb
    peel hb
    hq_core hb
  · rintro ⟨k1, hw1, st, c⟩ ⟨k2, hw2, st'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.apply_core_loop1.body at hb
    peel hb
    hq_core hb

hq_reg apply_core_loop1_hq

/-- **`apply_core` neither touches PRIMASK nor feeds the watchdog.** -/
theorem apply_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {safe : Bool}
    (h : arch.cm4.entry.apply_core ki ai ti k hw tables s = ok (safe, k', hw', s')) :
    HQ hv hw' hw := by
  unfold arch.cm4.entry.apply_core at h
  peel h
  hq_core h

hq_reg apply_core_hq

/-! ## The supervisor call's path -/

theorem dma_request_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {ptr len : U32} {b : Bool}
    (h : arch.cm4.entry.dma_request_core ki ai ti k hw tables i ptr len = ok (b, k', hw')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.dma_request_core at h
  peel h
  hq_core h

hq_reg dma_request_core_hq

theorem dma_write_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {offset value : U32} {b : Bool}
    (h : arch.cm4.entry.dma_write_core ki ai ti k hw tables i offset value = ok (b, k', hw')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.dma_write_core at h
  peel h
  hq_core h

hq_reg dma_write_core_hq

theorem raise_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {b : Bool}
    (h : arch.cm4.entry.raise_core ki ai ti k hw tables i = ok (b, k', hw')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.raise_core at h
  peel h
  hq_core h

hq_reg raise_core_hq

theorem copy_in_checked_hq [HSpec ai hv] {k : K} {hw hw' : A} {tables : T}
    {g : verus_builtin.Ghost verus_builtin.int} {dst src len x : U32}
    (h : arch.cm4.entry.copy_in_checked ki ai ti k hw tables g dst src len = ok (x, hw')) :
    HQ hv hw' hw := by
  unfold arch.cm4.entry.copy_in_checked at h
  peel h
  hq_core h

hq_reg copy_in_checked_hq

theorem copy_out_checked_hq [HSpec ai hv] {k : K} {hw hw' : A} {tables : T}
    {g : verus_builtin.Ghost verus_builtin.int} {dst src len x : U32}
    (h : arch.cm4.entry.copy_out_checked ki ai ti k hw tables g dst src len = ok (x, hw')) :
    HQ hv hw' hw := by
  unfold arch.cm4.entry.copy_out_checked at h
  peel h
  hq_core h

hq_reg copy_out_checked_hq

set_option maxHeartbeats 8000000 in
theorem endpoint_send_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {arg ptr len x : U32}
    (h : arch.cm4.entry.endpoint_send_core ki ai ti k hw tables s arg ptr len = ok (x, k', hw', s')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.endpoint_send_core at h
  peel h
  hq_core h

hq_reg endpoint_send_core_hq

set_option maxHeartbeats 8000000 in
theorem endpoint_recv_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {arg ptr len : U32} {x : U32 × verus_builtin.Ghost verus_builtin.int}
    (h : arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables s arg ptr len = ok (x, k', hw', s')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.endpoint_recv_core at h
  peel h
  hq_core h

hq_reg endpoint_recv_core_hq

set_option maxHeartbeats 8000000 in
theorem svc_service_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {frame frame' : Array U32 8#usize} {number : U8} {x : U32 × Bool}
    (h : arch.cm4.entry.svc_service_core ki ai ti k hw tables s frame number = ok (x, k', hw', s', frame')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.svc_service_core at h
  peel h
  hq_core h

hq_reg svc_service_core_hq

set_option maxHeartbeats 8000000 in
/-- **A supervisor call does not touch PRIMASK and does not feed the
watchdog** (R40.6: the call runs at the Kernel level, not under
PRIMASK): from its core's call to its return PRIMASK is as it was and no
action has cleared it. -/
theorem svc_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState}
    {frame frame' : Array U32 8#usize} {number : U8} {x : U32 × Bool}
    (h : arch.cm4.entry.svc_core ki ai ti k hw tables s frame number = ok (x, k', hw', s', frame')) :
    HQ hv hw' hw := by
  unfold arch.cm4.entry.svc_core at h
  peel h
  hq_core h

hq_reg svc_core_hq

/-! ## The other cores -/

theorem svc_of_ended_job_core_hq [HSpec ai hv] {k : K} {hw hw' : A} {d : Usize}
    (h : arch.cm4.entry.svc_of_ended_job_core ki ai k hw d = ok hw') : HQ hv hw' hw := by
  unfold arch.cm4.entry.svc_of_ended_job_core at h
  peel h
  hq_core h

hq_reg svc_of_ended_job_core_hq

set_option maxHeartbeats 8000000 in
theorem stack_violation_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {x : U32 × Bool}
    (h : arch.cm4.entry.stack_violation_core ki ai ti k hw tables s = ok (x, k', hw', s')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.stack_violation_core at h
  peel h
  hq_core h

hq_reg stack_violation_core_hq

set_option maxHeartbeats 16000000 in
theorem dispatch_job_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control exc_return : U32} {x : arch.cm4.entry.DispatchPlan × Bool}
    (h : arch.cm4.entry.dispatch_job_core ki ai ti k hw tables s n t psp control exc_return = ok (x, k', hw', s')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.dispatch_job_core at h
  peel h
  hq_core h

hq_reg dispatch_job_core_hq

set_option maxHeartbeats 16000000 in
theorem dispatch_vector_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control exc_return : U32} {x : arch.cm4.entry.DispatchPlan × Bool}
    (h : arch.cm4.entry.dispatch_vector_core ki ai ti k hw tables s n t psp control exc_return = ok (x, k', hw', s')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.dispatch_vector_core at h
  peel h
  hq_core h

hq_reg dispatch_vector_core_hq

set_option maxHeartbeats 8000000 in
theorem intake_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {n : U16} {task partition : U8} {psp exc_return : U32} {x : U32 × Bool}
    (h : arch.cm4.entry.intake_core ki ai ti k hw tables s n task partition psp exc_return = ok (x, k', hw', s')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.intake_core at h
  peel h
  hq_core h

hq_reg intake_core_hq

set_option maxHeartbeats 8000000 in
theorem unbound_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {n : U16} {x : U32 × Bool}
    (h : arch.cm4.entry.unbound_core ki ai ti k hw tables s n = ok (x, k', hw', s')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.unbound_core at h
  peel h
  hq_core h

hq_reg unbound_core_hq

theorem irq_entry_core_loop_hq [HSpec ai hv] {hw hw' : A} {s s' : arch.cm4.plan.ArchState} {i : Usize}
    (h : arch.cm4.entry.irq_entry_core_loop ai hw s i = ok (hw', s')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.irq_entry_core_loop at h
  refine loop_inv _ (fun x => HQ hv x.1 hw) (fun r => HQ hv r.1 hw) ?_ ?_ _ (hw', s')
    (HQ.refl _ _) h
  · rintro ⟨hw1, st, c⟩ ⟨hw2, st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.irq_entry_core_loop.body at hb
    peel hb
    hq_core hb
  · rintro ⟨hw1, st, c⟩ ⟨hw2, st'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.irq_entry_core_loop.body at hb
    peel hb
    hq_core hb

hq_reg irq_entry_core_loop_hq

set_option maxHeartbeats 16000000 in
theorem epilogue_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {n : U32} {x : (U32 × U32 × U32 × U32) × Bool}
    (h : arch.cm4.entry.epilogue_core ki ai ti k hw tables s n = ok (x, k', hw', s')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.epilogue_core at h
  peel h
  hq_core h

hq_reg epilogue_core_hq

set_option maxHeartbeats 8000000 in
theorem fault_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {kind : U8} {in_kernel : Bool} {x : U32 × Bool}
    (h : arch.cm4.entry.fault_core ki ai ti k hw tables s kind in_kernel = ok (x, k', hw', s')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.fault_core at h
  peel h
  hq_core h

hq_reg fault_core_hq

/-! ## One step of the frame, for the weaker frames below -/

/-- One call of a function shown to keep `HQ`, or one quiet action. -/
syntax "hq_one" : tactic
macro_rules | `(tactic| hq_one) => `(tactic| fail "no call")
macro_rules | `(tactic| hq_one) => `(tactic| exact HQ.apply (by assumption) (by exact True.intro))
macro_rules | `(tactic| hq_one) => `(tactic| exact HQ.of_eq (HSpec.svc_clear_pending (by assumption)))
macro_rules | `(tactic| hq_one) => `(tactic| exact HQ.of_eq (HSpec.stacking_fault_discard (by assumption)))
macro_rules | `(tactic| hq_one) => `(tactic| exact HQ.of_eq (HSpec.copy_in (by assumption)))
macro_rules | `(tactic| hq_one) => `(tactic| exact HQ.of_eq (HSpec.copy_out (by assumption)))
macro_rules | `(tactic| hq_one) => `(tactic| exact arm_compare_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact rearm_compare_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact dma_stop_partition_loop_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact dma_stop_partition_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact apply_core_loop0_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact apply_core_loop1_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact apply_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact dma_request_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact dma_write_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact raise_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact copy_in_checked_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact copy_out_checked_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact endpoint_send_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact endpoint_recv_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact svc_service_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact svc_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact svc_of_ended_job_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact stack_violation_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact dispatch_job_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact dispatch_vector_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact intake_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact unbound_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact irq_entry_core_loop_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact epilogue_core_hq (by assumption))
macro_rules | `(tactic| hq_one) => `(tactic| exact fault_core_hq (by assumption))

/-- From `hw` to `hw'` PRIMASK is as it was and no action has cleared it. -/
def HP (hv : A → HV) (hw' hw : A) : Prop :=
  (hv hw').primask = (hv hw).primask ∧ (hv hw').pcl = (hv hw).pcl

theorem HP.refl (hv : A → HV) (hw : A) : HP hv hw hw := ⟨rfl, rfl⟩

theorem HP.trans {a b c : A} (h1 : HP hv b a) (h2 : HP hv c b) : HP hv c a :=
  ⟨h2.1.trans h1.1, h2.2.trans h1.2⟩

theorem HQ.hp {hw hw' : A} (h : HQ hv hw' hw) : HP hv hw' hw := ⟨h.1, h.2.1⟩

/-- An action that neither sets nor clears PRIMASK. -/
def PQuiet : arch.cm4.entry.Action → Prop
  | .PrimaskSet => False
  | .PrimaskClear => False
  | _ => True

theorem HP.apply [HSpec ai hv] {hw hw' : A} {act : arch.cm4.entry.Action}
    (h : ai.apply hw act = ok hw') (hq : PQuiet act) : HP hv hw' hw := by
  have e := HSpec.apply (hv := hv) h
  unfold HP
  rw [e]
  cases act <;> first
    | exact ⟨rfl, rfl⟩
    | exact absurd hq (by simp [PQuiet])

/-- From `hw` to `hw'` the watchdog has not been fed. -/
def HWf (hv : A → HV) (hw' hw : A) : Prop := (hv hw').wfed = (hv hw).wfed

theorem HWf.refl (hv : A → HV) (hw : A) : HWf hv hw hw := rfl

theorem HWf.trans {a b c : A} (h1 : HWf hv b a) (h2 : HWf hv c b) : HWf hv c a := Eq.trans h2 h1

theorem HQ.wf {hw hw' : A} (h : HQ hv hw' hw) : HWf hv hw' hw := h.2.2

/-- An action that does not feed the watchdog. -/
def NoFeed : arch.cm4.entry.Action → Prop
  | .WatchdogFeed => False
  | _ => True

theorem HWf.apply [HSpec ai hv] {hw hw' : A} {act : arch.cm4.entry.Action}
    (h : ai.apply hw act = ok hw') (hq : NoFeed act) : HWf hv hw' hw := by
  have e := HSpec.apply (hv := hv) h
  unfold HWf
  rw [e]
  cases act <;> first
    | rfl
    | exact absurd hq (by simp [NoFeed])

/-! ## The time base's core feeds the watchdog; the start sets and clears PRIMASK -/

/-- The watchdog fed between two quiet stretches. -/
theorem feed_then [HSpec ai hv] {hw h0 h1 hw' : A}
    (hf : ai.apply h0 arch.cm4.entry.Action.WatchdogFeed = ok h1)
    (hq : HQ hv hw' h1) (hq0 : HQ hv h0 hw) : (hv hw').wfed = true ∧ HP hv hw' hw := by
  have e1 : (hv h1).wfed = true := by rw [HSpec.apply (hv := hv) hf]; rfl
  have hp0 : HP hv h1 h0 := HP.apply hf (by exact True.intro)
  exact ⟨by rw [hq.2.2]; exact e1, HP.trans (HP.trans hq0.hp hp0) hq.hp⟩

set_option maxHeartbeats 8000000 in
/-- **The time base's core feeds the watchdog** and does not touch
PRIMASK. -/
theorem timer_core_fed [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {x : U32 × Bool}
    (h : arch.cm4.entry.timer_core ki ai ti k hw tables s = ok (x, k', hw', s')) :
    (hv hw').wfed = true ∧ HP hv hw' hw := by
  unfold arch.cm4.entry.timer_core at h
  peel h
  all_goals
    refine feed_then (hv := hv) ‹ai.apply _ arch.cm4.entry.Action.WatchdogFeed = ok _› ?_ ?_
    all_goals (hq_close; done)

set_option maxHeartbeats 16000000 in
/-- **An interrupt entry does not touch PRIMASK, and feeds the watchdog
only when its vector is the time base's.** -/
theorem irq_entry_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {ipsr psp control exc_return : U32}
    {x : arch.cm4.entry.DispatchPlan × Bool}
    (h : arch.cm4.entry.irq_entry_core ki ai ti k hw tables s ipsr psp control exc_return
      = ok (x, k', hw', s')) :
    HQ hv hw' hw ∨
    (HP hv hw' hw ∧ ∃ v, ti.timer_vector tables = ok v ∧ UScalar.cast .U16 (ipsr &&& 511#u32) = v) := by
  unfold arch.cm4.entry.irq_entry_core at h
  peel h
  all_goals first
    | (left; hq_close; done)
    | (right
       refine ⟨?_, _, by assumption, by assumption⟩
       have ht := timer_core_fed (hv := hv) (by assumption)
       refine HP.trans (HP.trans (HQ.hp ?_) ht.2) (HQ.hp ?_)
       all_goals (hq_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (left; hq_close; done)
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (left; hq_close; done)
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (left; hq_close; done))))

set_option maxHeartbeats 8000000 in
/-- **The start does not feed the watchdog, and leaves PRIMASK clear
unless it ends in the safe state.** -/
theorem start_core_wf [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {safe : Bool}
    (h : arch.cm4.entry.start_core ki ai ti k hw tables s = ok (safe, k', hw', s')) :
    HWf hv hw' hw ∧ (safe = false → (hv hw').primask = false) := by
  unfold arch.cm4.entry.start_core at h
  peel h
  all_goals refine ⟨?_, fun hs => ?_⟩
  all_goals first
    | (repeat (first
        | exact HWf.refl _ _
        | refine HWf.trans ?_ (HQ.wf (by hq_one))
        | refine HWf.trans ?_ (HWf.apply (by assumption) (by exact True.intro))
        | dsimp only)
       done)
    | (rw [HSpec.apply (hv := hv) ‹ai.apply _ arch.cm4.entry.Action.PrimaskClear = ok _›]; rfl)
    | (exfalso; simp_all)

/-! ## BASEPRI after the last steps of an entry (A1) -/

syntax "hb_call" : tactic
macro_rules | `(tactic| hb_call) => `(tactic| fail "no call")

macro "hb_reg " l:ident : command =>
  `(macro_rules | `(tactic| hb_call) => `(tactic| refine HB.trans ?_ ($l (by assumption))))

macro_rules
  | `(tactic| hb_call) => `(tactic| refine HB.trans ?_ (HB.apply (by assumption) (by exact True.intro)))

syntax "hb_close" : tactic
macro_rules
  | `(tactic| hb_close) => `(tactic| repeat (first
      | exact HB.refl _ _
      | assumption
      | hb_call
      | dsimp only))

theorem arm_compare_hb [HSpec ai hv] {hw hw' : A} {at_ : U64}
    (h : arch.cm4.entry.arm_compare ai hw at_ = ok hw') : HB hv hw' hw := by
  unfold arch.cm4.entry.arm_compare at h
  peel h
  all_goals (hb_close; done)

hb_reg arm_compare_hb

theorem rearm_compare_hb [HSpec ai hv] {hw hw' : A} {at_ : U64}
    (h : arch.cm4.entry.rearm_compare ai hw at_ = ok hw') : HB hv hw' hw := by
  unfold arch.cm4.entry.rearm_compare at h
  peel h
  all_goals (hb_close; done)

hb_reg rearm_compare_hb

theorem dma_stop_partition_loop_hb [HSpec ai hv] {hw hw' : A} {tables : T} {p : U8} {n i : Usize}
    (h : arch.cm4.entry.dma_stop_partition_loop ai ti hw tables p n i = ok hw') : HB hv hw' hw := by
  unfold arch.cm4.entry.dma_stop_partition_loop at h
  refine loop_inv _ (fun x => HB hv x.1 hw) (fun r => HB hv r hw) ?_ ?_ _ hw' (HB.refl _ _) h
  · rintro ⟨hw1, c⟩ ⟨hw2, c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.dma_stop_partition_loop.body at hb
    peel hb
    all_goals (hb_close; done)
  · rintro ⟨hw1, c⟩ hw2 hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.dma_stop_partition_loop.body at hb
    peel hb
    all_goals (hb_close; done)

hb_reg dma_stop_partition_loop_hb

theorem dma_stop_partition_hb [HSpec ai hv] {hw hw' : A} {tables : T} {p : U8}
    (h : arch.cm4.entry.dma_stop_partition ai ti hw tables p = ok hw') : HB hv hw' hw := by
  unfold arch.cm4.entry.dma_stop_partition at h
  peel h
  all_goals (hb_close; done)

hb_reg dma_stop_partition_hb

theorem apply_core_loop0_hb [HSpec ai hv] {k : K} {hw hw' : A} {tables : T} {n t : Usize}
    (h : arch.cm4.entry.apply_core_loop0 ki ai ti k hw tables n t = ok hw') : HB hv hw' hw := by
  unfold arch.cm4.entry.apply_core_loop0 at h
  refine loop_inv _ (fun x => HB hv x.1 hw) (fun r => HB hv r hw) ?_ ?_ _ hw' (HB.refl _ _) h
  · rintro ⟨hw1, c⟩ ⟨hw2, c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.apply_core_loop0.body at hb
    peel hb
    all_goals (hb_close; done)
  · rintro ⟨hw1, c⟩ hw2 hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.apply_core_loop0.body at hb
    peel hb
    all_goals (hb_close; done)

hb_reg apply_core_loop0_hb

/-- The contract of `KernelOps` as far as BASEPRI depends on it (Verus:
`basepri_now`'s `ensures`, and `top_level` and `ceiling_level` in those of
`take_reinit` and `take_stop_dma`). `bp k` is the BASEPRI the Kernel's
state calls for: `hw_model::basepri_for` of the executing Job's level and
the System_Ceiling (`basepriFor` of `Sched/Nvic.lean`). -/
class KBp (ki : arch.bound.KernelOps K) (bp : K → Nat) : Prop where
  basepri_now : ∀ {k b}, ki.basepri_now k = ok b → b.val = bp k
  take_reinit : ∀ {k p b k'}, ki.take_reinit k p = ok (b, k') → bp k' = bp k
  take_stop_dma : ∀ {k p b k'}, ki.take_stop_dma k p = ok (b, k') → bp k' = bp k

variable {bp : K → Nat}

/-- The loop that acknowledges the Partitions' requests keeps BASEPRI and
the value the Kernel's state calls for. -/
theorem apply_core_loop1_bp [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {np p : Usize}
    (h : arch.cm4.entry.apply_core_loop1 ki ai ti k hw tables s np p = ok (k', hw', s')) :
    HB hv hw' hw ∧ bp k' = bp k := by
  unfold arch.cm4.entry.apply_core_loop1 at h
  refine loop_inv _ (fun x => HB hv x.2.1 hw ∧ bp x.1 = bp k)
    (fun r => HB hv r.2.1 hw ∧ bp r.1 = bp k) ?_ ?_ _ (k', hw', s') ⟨HB.refl _ _, rfl⟩ h
  · rintro ⟨k1, hw1, st, c⟩ ⟨k2, hw2, st', c'⟩ ⟨hinv, hk⟩ hb
    dsimp only at hinv hk hb ⊢
    unfold arch.cm4.entry.apply_core_loop1.body at hb
    peel hb
    all_goals refine ⟨by hb_close, ?_⟩
    all_goals
      repeat (first
        | exact hk
        | rw [KBp.take_stop_dma (bp := bp) (by assumption)]
        | rw [KBp.take_reinit (bp := bp) (by assumption)])
  · rintro ⟨k1, hw1, st, c⟩ ⟨k2, hw2, st'⟩ ⟨hinv, hk⟩ hb
    dsimp only at hinv hk hb ⊢
    unfold arch.cm4.entry.apply_core_loop1.body at hb
    peel hb
    all_goals refine ⟨by hb_close, ?_⟩
    all_goals
      repeat (first
        | exact hk
        | rw [KBp.take_stop_dma (bp := bp) (by assumption)]
        | rw [KBp.take_reinit (bp := bp) (by assumption)])

/-- BASEPRI written from the Kernel's `basepri_now` and kept since. -/
theorem a1_core [HSpec ai hv] [KBp ki bp] {k1 k' : K} {i : U8} {h0 h1 h5 : A}
    (hbn : ki.basepri_now k1 = ok i)
    (hap : ai.apply h0 (arch.cm4.entry.Action.SetBasepri i) = ok h1)
    (hb : HB hv h5 h1) (hk : bp k' = bp k1) : (hv h5).basepri = bp k' := by
  have e1 : (hv h1).basepri = i.val := by rw [HSpec.apply (hv := hv) hap]; rfl
  have e2 := KBp.basepri_now (bp := bp) hbn
  unfold HB at hb
  rw [hb, e1, e2, hk]

set_option maxHeartbeats 8000000 in
/-- **A1 after the last steps of an entry** (DD-01, HW-03;
`apply_core`'s `basepri_matches`): unless the Kernel is in its safe state,
BASEPRI is the value the state `apply_core` returns calls for; in the safe
state it is 0. -/
theorem apply_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {safe : Bool}
    (h : arch.cm4.entry.apply_core ki ai ti k hw tables s = ok (safe, k', hw', s')) :
    (safe = false → (hv hw').basepri = bp k') ∧ (safe = true → (hv hw').basepri = 0) := by
  unfold arch.cm4.entry.apply_core at h
  peel h
  all_goals refine ⟨fun hs => ?_, fun hs => ?_⟩
  all_goals first
    | (rw [HSpec.apply (hv := hv) ‹ai.apply _ (arch.cm4.entry.Action.SetBasepri 0#u8) = ok _›]; rfl)
    | (have hl := apply_core_loop1_bp (hv := hv) (bp := bp) (by assumption)
       refine a1_core (hv := hv) (bp := bp) ‹ki.basepri_now _ = ok _›
         ‹ai.apply _ (arch.cm4.entry.Action.SetBasepri _) = ok _› (HB.trans ?_ hl.1) hl.2
       hb_close
       done)
    | (exfalso; simp_all)

/-! ## A1 over the cores that end in `apply_core` -/

macro_rules
  | `(tactic| hb_call) => `(tactic| refine HB.trans ?_ (congrArg HV.basepri (HSpec.svc_clear_pending (by assumption))))
macro_rules
  | `(tactic| hb_call) => `(tactic| refine HB.trans ?_ (congrArg HV.basepri (HSpec.stacking_fault_discard (by assumption))))

theorem svc_of_ended_job_core_hb [HSpec ai hv] {k : K} {hw hw' : A} {d : Usize}
    (h : arch.cm4.entry.svc_of_ended_job_core ki ai k hw d = ok hw') : HB hv hw' hw := by
  unfold arch.cm4.entry.svc_of_ended_job_core at h
  peel h
  all_goals (hb_close; done)

hb_reg svc_of_ended_job_core_hb

/-- A1 established and BASEPRI kept since. -/
theorem a1_after {k' : K} {h2 hw' : A} {sf : Bool}
    (ha : (sf = false → (hv h2).basepri = bp k') ∧ (sf = true → (hv h2).basepri = 0))
    (hb : HB hv hw' h2) :
    (sf = false → (hv hw').basepri = bp k') ∧ (sf = true → (hv hw').basepri = 0) := by
  unfold HB at hb
  rw [hb]
  exact ha

set_option maxHeartbeats 8000000 in
theorem timer_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool}
    (h : arch.cm4.entry.timer_core ki ai ti k hw tables s = ok ((hop, safe), k', hw', s')) :
    (safe = false → (hv hw').basepri = bp k') ∧ (safe = true → (hv hw').basepri = 0) := by
  unfold arch.cm4.entry.timer_core at h
  peel h
  all_goals
    refine a1_after (apply_core_a1 (hv := hv) (bp := bp) (by assumption)) ?_
    hb_close
    done

set_option maxHeartbeats 8000000 in
theorem intake_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {n : U16} {task partition : U8} {psp exc_return : U32} {hop : U32} {safe : Bool}
    (h : arch.cm4.entry.intake_core ki ai ti k hw tables s n task partition psp exc_return = ok ((hop, safe), k', hw', s')) :
    (safe = false → (hv hw').basepri = bp k') ∧ (safe = true → (hv hw').basepri = 0) := by
  unfold arch.cm4.entry.intake_core at h
  peel h
  all_goals
    refine a1_after (apply_core_a1 (hv := hv) (bp := bp) (by assumption)) ?_
    hb_close
    done

set_option maxHeartbeats 8000000 in
theorem unbound_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {n : U16} {hop : U32} {safe : Bool}
    (h : arch.cm4.entry.unbound_core ki ai ti k hw tables s n = ok ((hop, safe), k', hw', s')) :
    (safe = false → (hv hw').basepri = bp k') ∧ (safe = true → (hv hw').basepri = 0) := by
  unfold arch.cm4.entry.unbound_core at h
  peel h
  all_goals
    refine a1_after (apply_core_a1 (hv := hv) (bp := bp) (by assumption)) ?_
    hb_close
    done

set_option maxHeartbeats 8000000 in
theorem stack_violation_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool}
    (h : arch.cm4.entry.stack_violation_core ki ai ti k hw tables s = ok ((hop, safe), k', hw', s')) :
    (safe = false → (hv hw').basepri = bp k') ∧ (safe = true → (hv hw').basepri = 0) := by
  unfold arch.cm4.entry.stack_violation_core at h
  peel h
  all_goals
    refine a1_after (apply_core_a1 (hv := hv) (bp := bp) (by assumption)) ?_
    hb_close
    done

set_option maxHeartbeats 8000000 in
theorem fault_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {kind : U8} {in_kernel : Bool} {hop : U32} {safe : Bool}
    (h : arch.cm4.entry.fault_core ki ai ti k hw tables s kind in_kernel = ok ((hop, safe), k', hw', s')) :
    (safe = false → (hv hw').basepri = bp k') ∧ (safe = true → (hv hw').basepri = 0) := by
  unfold arch.cm4.entry.fault_core at h
  peel h
  all_goals
    refine a1_after (apply_core_a1 (hv := hv) (bp := bp) (by assumption)) ?_
    hb_close
    done

set_option maxHeartbeats 8000000 in
theorem start_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {safe : Bool}
    (h : arch.cm4.entry.start_core ki ai ti k hw tables s = ok (safe, k', hw', s')) :
    (safe = false → (hv hw').basepri = bp k') ∧ (safe = true → (hv hw').basepri = 0) := by
  unfold arch.cm4.entry.start_core at h
  peel h
  all_goals
    refine a1_after (apply_core_a1 (hv := hv) (bp := bp) (by assumption)) ?_
    hb_close
    done

/-! ## BASEPRI kept by the services of a supervisor call -/

macro_rules
  | `(tactic| hb_call) => `(tactic| refine HB.trans ?_ (congrArg HV.basepri (HSpec.copy_in (by assumption))))
macro_rules
  | `(tactic| hb_call) => `(tactic| refine HB.trans ?_ (congrArg HV.basepri (HSpec.copy_out (by assumption))))

macro "hb_core " h:ident : tactic => `(tactic| (all_goals first
    | (hb_close; done)
    | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
        | (hb_close; done)
        | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
            | (hb_close; done)
            | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;>
                (hb_close; done))))))

theorem dma_request_core_hb [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {ptr len : U32} {b : Bool}
    (h : arch.cm4.entry.dma_request_core ki ai ti k hw tables i ptr len = ok (b, k', hw')) : HB hv hw' hw := by
  unfold arch.cm4.entry.dma_request_core at h
  peel h
  hb_core h

hb_reg dma_request_core_hb

theorem dma_write_core_hb [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {offset value : U32} {b : Bool}
    (h : arch.cm4.entry.dma_write_core ki ai ti k hw tables i offset value = ok (b, k', hw')) : HB hv hw' hw := by
  unfold arch.cm4.entry.dma_write_core at h
  peel h
  hb_core h

hb_reg dma_write_core_hb

theorem raise_core_hb [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {b : Bool}
    (h : arch.cm4.entry.raise_core ki ai ti k hw tables i = ok (b, k', hw')) : HB hv hw' hw := by
  unfold arch.cm4.entry.raise_core at h
  peel h
  hb_core h

hb_reg raise_core_hb

theorem copy_in_checked_hb [HSpec ai hv] {k : K} {hw hw' : A} {tables : T}
    {g : verus_builtin.Ghost verus_builtin.int} {dst src len x : U32}
    (h : arch.cm4.entry.copy_in_checked ki ai ti k hw tables g dst src len = ok (x, hw')) :
    HB hv hw' hw := by
  unfold arch.cm4.entry.copy_in_checked at h
  peel h
  hb_core h

hb_reg copy_in_checked_hb

theorem copy_out_checked_hb [HSpec ai hv] {k : K} {hw hw' : A} {tables : T}
    {g : verus_builtin.Ghost verus_builtin.int} {dst src len x : U32}
    (h : arch.cm4.entry.copy_out_checked ki ai ti k hw tables g dst src len = ok (x, hw')) :
    HB hv hw' hw := by
  unfold arch.cm4.entry.copy_out_checked at h
  peel h
  hb_core h

hb_reg copy_out_checked_hb

set_option maxHeartbeats 8000000 in
theorem endpoint_send_core_hb [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {arg ptr len x : U32}
    (h : arch.cm4.entry.endpoint_send_core ki ai ti k hw tables s arg ptr len = ok (x, k', hw', s')) : HB hv hw' hw := by
  unfold arch.cm4.entry.endpoint_send_core at h
  peel h
  hb_core h

hb_reg endpoint_send_core_hb

set_option maxHeartbeats 8000000 in
theorem endpoint_recv_core_hb [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {arg ptr len : U32} {x : U32 × verus_builtin.Ghost verus_builtin.int}
    (h : arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables s arg ptr len = ok (x, k', hw', s')) : HB hv hw' hw := by
  unfold arch.cm4.entry.endpoint_recv_core at h
  peel h
  hb_core h

hb_reg endpoint_recv_core_hb

theorem irq_entry_core_loop_hb [HSpec ai hv] {hw hw' : A} {s s' : arch.cm4.plan.ArchState} {i : Usize}
    (h : arch.cm4.entry.irq_entry_core_loop ai hw s i = ok (hw', s')) : HB hv hw' hw := by
  unfold arch.cm4.entry.irq_entry_core_loop at h
  refine loop_inv _ (fun x => HB hv x.1 hw) (fun r => HB hv r.1 hw) ?_ ?_ _ (hw', s')
    (HB.refl _ _) h
  · rintro ⟨hw1, st, c⟩ ⟨hw2, st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.irq_entry_core_loop.body at hb
    peel hb
    hb_core hb
  · rintro ⟨hw1, st, c⟩ ⟨hw2, st'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.irq_entry_core_loop.body at hb
    peel hb
    hb_core hb

hb_reg irq_entry_core_loop_hb

end P18
