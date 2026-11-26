import Sched.ArchHw
open Aeneas Aeneas.Std Result

/-! # BASEPRI after every entry core (A1; 2026-11-25)

`Sched/ArchHw.lean` has A1 after `apply_core` and the six cores that end
in it. This file has the others, as Verus states them (`entry.rs`):

- the dispatch of a Job and the unwind end with BASEPRI rewritten from the
  Kernel's state, or at 0 in the safe state (`dispatch_job_core_a1`,
  `epilogue_core_a1`);
- a dispatch vector keeps A1: if BASEPRI was the value the Kernel's state
  called for at the call, it is at the return, unless the entry ends in
  the safe state, where it is 0 (`dispatch_vector_core_a1`). An entry that
  does not rewrite BASEPRI leaves the Kernel's state as well.

`Sched/ArchHwA1Svc.lean` has the same of a supervisor call. Not proven
here: the same of an interrupt entry (`irq_entry_core`, which runs one of
the dispatch vector's, the intake's, the time base's and the unbound
line's cores; two attempts did not check), and `svc_check_core`, which is
not in the translation. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {hv : A → HV} {bp : K → Nat}

/-- BASEPRI set to 0 and kept since. -/
theorem a1_zero [HSpec ai hv] {h0 h1 hw' : A}
    (hap : ai.apply h0 (arch.cm4.entry.Action.SetBasepri 0#u8) = ok h1) (hb : HB hv hw' h1) :
    (hv hw').basepri = 0 := by
  unfold HB at hb
  rw [hb, HSpec.apply (hv := hv) hap]
  rfl

/-- A1 unconditionally gives A1 kept. -/
theorem a1_weak {k k' : K} {hw hw' : A} {sf : Bool}
    (h : (sf = false → (hv hw').basepri = bp k') ∧ (sf = true → (hv hw').basepri = 0)) :
    (sf = true → (hv hw').basepri = 0) ∧
    (sf = false → (hv hw).basepri = bp k → (hv hw').basepri = bp k') :=
  ⟨h.2, fun hs _ => h.1 hs⟩

/-- An entry that keeps BASEPRI and the Kernel's state keeps A1. -/
theorem a1_keep {k : K} {hw hw' : A} (hb : HB hv hw' hw) :
    (false = true → (hv hw').basepri = 0) ∧
    (false = false → (hv hw).basepri = bp k → (hv hw').basepri = bp k) :=
  by
  unfold HB at hb
  refine ⟨fun h => (by cases h), fun _ hold => ?_⟩
  rw [hb]
  exact hold

/-- A1 kept by a call, with BASEPRI kept before and after it. -/
theorem a1_wrap {k k' : K} {hw h1 h2 hw' : A} {sf : Bool}
    (h : (sf = true → (hv h2).basepri = 0) ∧
      (sf = false → (hv h1).basepri = bp k → (hv h2).basepri = bp k'))
    (hb1 : HB hv h1 hw) (hb2 : HB hv hw' h2) :
    (sf = true → (hv hw').basepri = 0) ∧
    (sf = false → (hv hw).basepri = bp k → (hv hw').basepri = bp k') := by
  unfold HB at hb1 hb2
  rw [hb2, ← hb1]
  exact h

/-- A call already shown to establish A1. -/
syntax "a1_call" : tactic
macro_rules | `(tactic| a1_call) => `(tactic| fail "no call")

macro "a1_reg " l:ident : command =>
  `(macro_rules | `(tactic| a1_call) => `(tactic| refine a1_after ($l (by assumption)) ?_))

a1_reg apply_core_a1
a1_reg timer_core_a1
a1_reg intake_core_a1
a1_reg unbound_core_a1
a1_reg stack_violation_core_a1
a1_reg fault_core_a1

/-- One case of a core that establishes A1. -/
syntax "a1u_goal" : tactic
macro_rules
  | `(tactic| a1u_goal) => `(tactic| first
      | (a1_call; hb_close; done)
      | (refine ⟨fun _ => a1_core (by assumption) (by assumption) ?_ rfl, fun hs => ?_⟩
         · hb_close; done
         · cases hs)
      | (refine ⟨fun hs => ?_, fun _ => a1_zero (by assumption) ?_⟩
         · cases hs
         · hb_close; done))

macro "a1u_core " h:ident : tactic => `(tactic| (all_goals first
    | a1u_goal
    | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
        | a1u_goal
        | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
            | a1u_goal
            | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;>
                a1u_goal)))))

set_option maxHeartbeats 16000000 in
/-- **A1 after the dispatch of a Job** (`dispatch_job_core`). -/
theorem dispatch_job_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (h : arch.cm4.entry.dispatch_job_core ki ai ti k hw tables s n t psp control exc_return
      = ok ((plan, safe), k', hw', s')) :
    (safe = false → (hv hw').basepri = bp k') ∧ (safe = true → (hv hw').basepri = 0) := by
  unfold arch.cm4.entry.dispatch_job_core at h
  peel h
  a1u_core h

a1_reg dispatch_job_core_a1

set_option maxHeartbeats 16000000 in
/-- **A1 after an unwind** (`epilogue_core`). -/
theorem epilogue_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U32} {plan : U32 × U32 × U32 × U32} {safe : Bool}
    (h : arch.cm4.entry.epilogue_core ki ai ti k hw tables s n = ok ((plan, safe), k', hw', s')) :
    (safe = false → (hv hw').basepri = bp k') ∧ (safe = true → (hv hw').basepri = 0) := by
  unfold arch.cm4.entry.epilogue_core at h
  peel h
  a1u_core h


/-! ## The entries that keep A1 -/

/-- A call already shown to keep A1. -/
syntax "a1c_call" : tactic
macro_rules | `(tactic| a1c_call) => `(tactic| fail "no call")

macro "a1c_reg " l:ident : command =>
  `(macro_rules | `(tactic| a1c_call) => `(tactic| refine a1_wrap ($l (by assumption)) ?_ ?_))

/-- One case of a core that keeps A1. -/
syntax "a1c_goal" : tactic
macro_rules
  | `(tactic| a1c_goal) => `(tactic| first
      | (refine a1_weak ?_; a1u_goal)
      | (refine a1_keep ?_; hb_close; done)
      | (a1c_call <;> (hb_close; done)))

macro "a1c_core " h:ident : tactic => `(tactic| (all_goals first
    | a1c_goal
    | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
        | a1c_goal
        | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
            | a1c_goal
            | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;>
                a1c_goal)))))

set_option maxHeartbeats 16000000 in
/-- **A dispatch vector keeps A1** (`dispatch_vector_core`). -/
theorem dispatch_vector_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (h : arch.cm4.entry.dispatch_vector_core ki ai ti k hw tables s n t psp control exc_return
      = ok ((plan, safe), k', hw', s')) :
    (safe = true → (hv hw').basepri = 0) ∧
    (safe = false → (hv hw).basepri = bp k → (hv hw').basepri = bp k') := by
  unfold arch.cm4.entry.dispatch_vector_core at h
  peel h
  a1c_core h

a1c_reg dispatch_vector_core_a1


a1_reg epilogue_core_a1

end P18
