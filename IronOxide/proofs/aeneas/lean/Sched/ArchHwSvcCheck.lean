import Sched.SvcCheck
import Sched.ArchHwA1Svc
import Sched.CoreTotal
open Aeneas Aeneas.Std Result

/-! # The supervisor call's stack check (`svc_check_core`; 2026-11-26)

`svc_check_core` is the function the SVC handler runs before it reads
anything through the process stack pointer. The Kernel's translation did
not reach it; `Sched/SvcCheck.lean` has its definition as Aeneas extracts
it, over the definitions of `Sched/Kernel.lean`. This file proves of that
definition what the Verus contract states about the hardware, and that it
returns:

- `svc_check_core_none`: **a check that passes changes nothing**: the
  Kernel's state, the hardware and the level records are the ones given.
- `svc_check_core_stack`: **a check that passes for a frame on the process
  stack with a Job executing found the frame inside the stack region of
  the executing Job's Partition** (`psp_in_stack` answered `true`; R16.5).
- `svc_check_core_some`: **a check that reports is the stack violation's
  core followed by the discard of the pending stacking fault**, taken for
  a frame on the process stack, with a Job executing, that `psp_in_stack`
  refused. Every statement of `stack_violation_core` carries over by it.
- `svc_check_core_a1`: after a report BASEPRI is the value the Kernel's
  state calls for, or 0 in the safe state; `svc_check_core_a1_step`:
  **the check keeps A1** (Verus: `a1_step`).
- `svc_check_core_hq`: it does not touch PRIMASK and does not feed the
  watchdog.
- `svc_check_core_keeps`: it keeps the invariant of the entry cores, with
  the plan it hands the stub.
- `svc_check_core_t`: **it returns** for every input, over the classes of
  `Sched/CoreTotal.lean` and `HwDiscardT` (the discard returns).

Not here: that the MPU view is left (the view is not a field of `HV` or
`HV2`), the Verus flag `stacking_discarded` as such (it is ghost state of
the trait; `svc_check_core_some` has the call instead), and the slots'
and deadline statements of the contract (`slots_step`,
`spec_deadline_owned`), which are the Kernel's, not the hardware's. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {hv : A → HV} {bp : K → Nat}

/-- **A check that passes changes nothing.** -/
theorem svc_check_core_none {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {psp exc_return : U32}
    (h : arch.cm4.entry.svc_check_core ki ai ti k hw tables s psp exc_return
      = ok (none, k', hw', s')) : k' = k ∧ hw' = hw ∧ s' = s := by
  unfold arch.cm4.entry.svc_check_core at h
  peel h
  all_goals exact ⟨rfl, rfl, rfl⟩

/-- **A check that passes for a frame on the process stack, with a Job
executing, found the frame inside the stack region of the executing Job's
Partition** (R16.5). -/
theorem svc_check_core_stack {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {psp exc_return : U32}
    (h : arch.cm4.entry.svc_check_core ki ai ti k hw tables s psp exc_return
      = ok (none, k', hw', s'))
    (he : exc_return &&& 4#u32 ≠ 0#u32) (hp : s.current_partition ≠ 255#u8) :
    arch.cm4.entry.psp_in_stack ti tables s psp = ok true := by
  unfold arch.cm4.entry.svc_check_core at h
  peel h
  all_goals first
    | (exfalso; exact he (by assumption))
    | simp_all

/-- **A check that reports is the stack violation's core followed by the
discard of the pending stacking fault**; it is taken only for a frame on
the process stack, with a Job executing, that `psp_in_stack` refused. -/
theorem svc_check_core_some {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {psp exc_return : U32} {r : U32 × Bool}
    (h : arch.cm4.entry.svc_check_core ki ai ti k hw tables s psp exc_return
      = ok (some r, k', hw', s')) :
    exc_return &&& 4#u32 ≠ 0#u32 ∧ s.current_partition ≠ 255#u8 ∧
    arch.cm4.entry.psp_in_stack ti tables s psp = ok false ∧
    ∃ hw1, arch.cm4.entry.stack_violation_core ki ai ti k hw tables s = ok (r, k', hw1, s') ∧
      ai.stacking_fault_discard hw1 = ok hw' := by
  unfold arch.cm4.entry.svc_check_core at h
  peel h
  refine ⟨by assumption, by assumption, ?_, _, by assumption, by assumption⟩
  simp_all

/-- After a report BASEPRI is the value the Kernel's state calls for, or
0 in the safe state. -/
theorem svc_check_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {psp exc_return hop : U32} {safe : Bool}
    (h : arch.cm4.entry.svc_check_core ki ai ti k hw tables s psp exc_return
      = ok (some (hop, safe), k', hw', s')) :
    (safe = false → (hv hw').basepri = bp k') ∧ (safe = true → (hv hw').basepri = 0) := by
  obtain ⟨_, _, _, hw1, hsv, hd⟩ := svc_check_core_some h
  refine a1_after (stack_violation_core_a1 hsv) ?_
  exact congrArg HV.basepri (HSpec.stacking_fault_discard hd)

/-- **The stack check keeps A1** (Verus: `a1_step`): unless it ends in the
safe state, BASEPRI the value the Kernel's state called for at the call
is that value at the return. -/
theorem svc_check_core_a1_step [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {psp exc_return : U32} {r : Option (U32 × Bool)}
    (h : arch.cm4.entry.svc_check_core ki ai ti k hw tables s psp exc_return
      = ok (r, k', hw', s'))
    (hs : ∀ hop, r ≠ some (hop, true)) (hold : (hv hw).basepri = bp k) :
    (hv hw').basepri = bp k' := by
  cases r with
  | none =>
    obtain ⟨rfl, rfl, _⟩ := svc_check_core_none h
    exact hold
  | some x =>
    obtain ⟨hop, safe⟩ := x
    cases safe with
    | false => exact (svc_check_core_a1 h).1 rfl
    | true => exact absurd rfl (hs hop)

/-- **The stack check does not touch PRIMASK and does not feed the
watchdog.** -/
theorem svc_check_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {psp exc_return : U32} {r : Option (U32 × Bool)}
    (h : arch.cm4.entry.svc_check_core ki ai ti k hw tables s psp exc_return
      = ok (r, k', hw', s')) : HQ hv hw' hw := by
  cases r with
  | none =>
    obtain ⟨_, rfl, _⟩ := svc_check_core_none h
    exact HQ.refl _ _
  | some x =>
    obtain ⟨_, _, _, hw1, hsv, hd⟩ := svc_check_core_some h
    exact HQ.trans (stack_violation_core_hq hsv) (HQ.of_eq (HSpec.stacking_fault_discard hd))

/-- **The stack check keeps the invariant of the entry cores**, with the
plan it hands the stub (none: nothing is owed). -/
theorem svc_check_core_keeps {kv : K → Arch.KV} {okK : K → Prop} [KSpec ki kv okK] {k k' : K}
    {hw hw' : A} {tables : T} {s s' : arch.cm4.plan.ArchState} {psp exc_return : U32}
    {r : Option (U32 × Bool)} (hok : okK k) (hai : AI kv k s 0)
    (h : arch.cm4.entry.svc_check_core ki ai ti k hw tables s psp exc_return
      = ok (r, k', hw', s')) :
    okK k' ∧ AI kv k' s' (match r with | none => 0 | some x => x.1.val) := by
  cases r with
  | none =>
    obtain ⟨rfl, _, rfl⟩ := svc_check_core_none h
    exact ⟨hok, hai⟩
  | some x =>
    obtain ⟨_, _, _, hw1, hsv, hd⟩ := svc_check_core_some h
    exact stack_violation_core_keeps (hop := x.1) (safe := x.2) hok hai hsv

/-! ## It returns -/

/-- The discard of a pending stacking fault returns (the one method of
the hardware trait `HwT` does not list: no other core calls it). -/
class HwDiscardT (ai : arch.cm4.entry.KernelArch A) : Prop where
  stacking_fault_discard : ∀ hw, ai.stacking_fault_discard hw ⦃ _ => True ⦄

@[step] theorem hw_stacking_fault_discard [HwDiscardT ai] (hw : A) :
    ai.stacking_fault_discard hw ⦃ _ => True ⦄ := HwDiscardT.stacking_fault_discard hw

variable {okT : K → Prop} {TN TP : Nat}

/-- **`svc_check_core` returns**, and the Kernel's state it returns
satisfies what the one given did. -/
theorem svc_check_core_t [HwT ai] [HwDiscardT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT]
    (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (psp exc_return : U32)
    (hk : okT k) (hcp : state.current_partition = 255#u8 ∨ state.current_partition.val < TP) :
    arch.cm4.entry.svc_check_core ki ai ti k hw tables state psp exc_return
      ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.svc_check_core
  ptac

end P18
