import Sched.Script
open Aeneas Aeneas.Std Result

/-! # The Task an interrupt entry dispatches (2026-11-27)

`Sched/Script.lean` gives the paths of the interrupt entry; an item
`dispatch` on one of them is `KernelOps::dispatch` of some Task. This file
reads from the cores, for any implementor of the interface, which Task:
the one the tables bind to the exception number the entry read.

- `Disp t k k'`: `dispatch` of the Task `t` on `k`, alone or followed by a
  fault on its result and `apply_core`.
- `dispatch_job_core_task`, `dispatch_vector_core_task`: the start of a
  Job and a dispatch vector's core, given the Task `t`, dispatch `t` or no
  Task.
- `irq_entry_core_task`: the interrupt entry called with the exception
  status word `ipsr` either takes a path with no dispatch, or the tables
  bind the exception number `vecOf ipsr` (the word's low nine bits) to
  the dispatch of a Task `t` and the entry is `Disp t`: on the Kernel
  state the entry was called on.

Handwritten; no `sorry`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

/-- The exception number the interrupt entry reads from the exception
status word. -/
def vecOf (ipsr : U32) : U16 := UScalar.cast .U16 (ipsr &&& 511#u32)

section task
variable {K A T : Type _}
  (ki : _root_.rsk_kernel.arch.bound.KernelOps K)
  (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A)
  (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T)

/-- The dispatch of the Task `t` on `k`, alone or followed by a fault on
its result and `apply_core`. -/
def Disp (t : U8) (k k' : K) : Prop :=
  ∃ (pc : U32) (k1 : K), ki.dispatch k t pc = ok k1 ∧ En ki [[], [.fault, .apply]] k1 k'

set_option maxHeartbeats 8000000 in
/-- The start of a Job dispatches the Task it was given, or none. -/
theorem dispatch_job_core_task {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control er : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.dispatch_job_core ki ai ti k hw tables state n t psp control er = ok (r, k', hw', st')) :
    En ki [[.fault, .apply]] k k' ∨ Disp ki t k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.dispatch_job_core at h
  peel h
  all_goals first
    | exact Or.inl (en_wa (by decide) (.fault _ _ _ _ (by assumption)) (by assumption))
    | exact Or.inr ⟨_, _, by assumption, en_plain (by decide)⟩
    | exact Or.inr ⟨_, _, by assumption,
        en_wa (by decide) (.fault _ _ _ _ (by assumption)) (by assumption)⟩

set_option maxHeartbeats 8000000 in
/-- A dispatch vector's core dispatches the Task it was given, or none; and
it dispatches only when the exception return word says the vector was
taken from thread mode (bit 3 set). -/
theorem dispatch_vector_core_task {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control er : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.dispatch_vector_core ki ai ti k hw tables state n t psp control er = ok (r, k', hw', st')) :
    En ki [[], [.fault, .apply]] k k' ∨ ((er &&& 8#u32) ≠ 0#u32 ∧ Disp ki t k k') := by
  unfold _root_.rsk_kernel.arch.cm4.entry.dispatch_vector_core at h
  simp only [_root_.rsk_kernel.arch.cm4.entry.WINDOW_DEFER, ↓reduceIte] at h
  peel h
  all_goals first
    | exact Or.inl (en_plain (by decide))
    | exact Or.inl (en_wa (by decide) (.fault _ _ _ _ (by assumption)) (by assumption))
    | exact (dispatch_job_core_task ki ai ti (by assumption)).imp (fun e => e.mono (by decide))
        (fun d => ⟨by assumption, d⟩)
    | exact Or.inl ((stack_violation_core_sh ki ai ti (by assumption)).mono (by decide))

set_option maxHeartbeats 8000000 in
/-- **The Task an interrupt entry dispatches is the one the tables bind to
the exception number it read**, and it dispatches only from thread mode. -/
theorem irq_entry_core_task {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {ipsr psp control er : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.irq_entry_core ki ai ti k hw tables state ipsr psp control er = ok (r, k', hw', st')) :
    En ki [[], [.fault, .apply], [.intake, .apply], [.fault, .intake, .apply], [.timer, .apply]] k k' ∨
    ∃ t : U8, ti.binding tables (vecOf ipsr) = ok (_root_.rsk_kernel.arch.cm4.entry.Binding.Dispatch t) ∧
      (er &&& 8#u32) ≠ 0#u32 ∧ Disp ki t k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.irq_entry_core at h
  unfold vecOf
  peel h
  all_goals first
    | exact Or.inl (en_plain (by decide))
    | exact Or.inl ((timer_core_sh ki ai ti (by assumption)).mono (by decide))
    | exact Or.inl ((intake_core_sh ki ai ti (by assumption)).mono (by decide))
    | exact Or.inl ((unbound_core_sh ki ai ti (by assumption)).mono (by decide))
    | exact (dispatch_vector_core_task ki ai ti (by assumption)).imp (fun e => e.mono (by decide))
        (fun d => ⟨_, by assumption, d.1, d.2⟩)

end task
end P18
