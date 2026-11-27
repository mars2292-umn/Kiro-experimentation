import Sched.ArchHwA1Svc
open Aeneas Aeneas.Std Result

/-! # BASEPRI over an interrupt entry (A1; 2026-11-26)

An interrupt entry keeps A1 (`irq_entry_core_a1`): if BASEPRI was the
value the Kernel's state called for at the call, it is at the return, and
it is 0 if the entry ends in the safe state. Verus states the same in
`irq_entry_core`'s `ensures` (`r.1 ==> basepri == 0`, and
`!r.1 && basepri_matches(old) ==> basepri_matches(final)`).

The entry is not split into its cases at once (two attempts at that did
not check). Its body is taken apart bind by bind (`bstep`) into the trace
action, the loop over the deferred vectors, the core its vector's binding
selects, and the deferral of a planned unwind at the end; each part has a
lemma of its own:

- the selected core: `irq_dispatch_a1` (a dispatch vector),
  `irq_intake_a1` (a sporadic Task's line), `irq_timer_a1` (the time
  base), `irq_unbound_a1` (a line bound to nothing), `irq_other_a1` (a
  Kernel line other than the time base's: nothing runs);
- the end (`irq_tail`): the Kernel's state and the safe-state answer are
  the selected core's, and BASEPRI is kept (the one action is a trace);
- `irq_join` puts the three together.

An entry that dispatches a Job (a frame address other than 0 in the plan)
establishes A1 whatever BASEPRI was at the call
(`irq_entry_core_a1_job`, from `dispatch_vector_core_a1_job`; Verus:
`r.0.psp != 0 ==> basepri_matches`), by the same decomposition.

Every statement is of the extracted function and has the form "if it
returns". -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {hv : A → HV} {bp : K → Nat}

theorem bind_ok_inv {α β} {x : Result α} {f : α → Result β} {y : β}
    (h : (x >>= f) = ok y) : ∃ a, x = ok a ∧ f a = ok y := by
  cases x <;> simp_all

/-- The first bind of `h : (do let a ← x; f a) = ok y`. -/
macro "bstep " h:ident " with " a:rcasesPat ", " ha:ident : tactic => `(tactic| (
  have hbs := bind_ok_inv $h:ident
  clear $h:ident
  obtain ⟨$a, $ha:ident, $h:ident⟩ := hbs
  try dsimp only at $h:ident))

/-! ## One lemma per branch of the interrupt entry -/

/-- The vector is a dispatch vector. -/
theorem irq_dispatch_a1 [HSpec ai hv] [KBp ki bp] {k k1 : K} {hw2 hw3 : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control exc_return : U32}
    {r : arch.cm4.entry.DispatchPlan × Bool}
    (h : arch.cm4.entry.dispatch_vector_core ki ai ti k hw2 tables s n t psp control exc_return
      = ok (r, k1, hw3, s')) :
    (r.2 = true → (hv hw3).basepri = 0) ∧
    (r.2 = false → (hv hw2).basepri = bp k → (hv hw3).basepri = bp k1) :=
  dispatch_vector_core_a1 (plan := r.1) (safe := r.2) h

/-- The vector is a sporadic Task's (the intake). -/
theorem irq_intake_a1 [HSpec ai hv] [KBp ki bp] {k k1 : K} {hw2 hw3 : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U16} {task partition : U8} {psp exc_return hop : U32}
    {safe : Bool}
    (h : arch.cm4.entry.intake_core ki ai ti k hw2 tables s n task partition psp exc_return
      = ok ((hop, safe), k1, hw3, s')) :
    (safe = true → (hv hw3).basepri = 0) ∧
    (safe = false → (hv hw2).basepri = bp k → (hv hw3).basepri = bp k1) :=
  a1_weak (intake_core_a1 h)

/-- The vector is the time base's. -/
theorem irq_timer_a1 [HSpec ai hv] [KBp ki bp] {k k1 : K} {hw2 hw3 : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool}
    (h : arch.cm4.entry.timer_core ki ai ti k hw2 tables s = ok ((hop, safe), k1, hw3, s')) :
    (safe = true → (hv hw3).basepri = 0) ∧
    (safe = false → (hv hw2).basepri = bp k → (hv hw3).basepri = bp k1) :=
  a1_weak (timer_core_a1 h)

/-- The vector is bound to nothing. -/
theorem irq_unbound_a1 [HSpec ai hv] [KBp ki bp] {k k1 : K} {hw2 hw3 : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U16} {hop : U32} {safe : Bool}
    (h : arch.cm4.entry.unbound_core ki ai ti k hw2 tables s n = ok ((hop, safe), k1, hw3, s')) :
    (safe = true → (hv hw3).basepri = 0) ∧
    (safe = false → (hv hw2).basepri = bp k → (hv hw3).basepri = bp k1) :=
  a1_weak (unbound_core_a1 h)

/-- A Kernel vector that is not the time base's: nothing runs. -/
theorem irq_other_a1 {k : K} {hw2 : A} :
    (false = true → (hv hw2).basepri = 0) ∧
    (false = false → (hv hw2).basepri = bp k → (hv hw2).basepri = bp k) :=
  a1_keep (HB.refl _ _)

/-- The core the vector's binding selects, after `peel`. -/
macro "irq_mid" : tactic => `(tactic| first
  | exact irq_dispatch_a1 (by assumption)
  | exact irq_intake_a1 (by assumption)
  | exact irq_timer_a1 (by assumption)
  | exact irq_unbound_a1 (by assumption)
  | exact irq_other_a1)

/-- The deferral at the end, after `peel`: the Kernel's state and the
safe-state answer are the selected core's, and BASEPRI is kept. -/
macro "irq_tail" : tactic => `(tactic| (
  refine ⟨rfl, ?_, ?_⟩
  · first
      | rfl
      | (symm; simpa using ‹¬ _ = true›)
  · hb_close))

/-- The entry from the selected core's result to the return. -/
theorem irq_join {k k1 : K} {hw h2 h3 hw' : A} {b1 safe : Bool} {k' : K}
    (hmid : (b1 = true → (hv h3).basepri = 0) ∧
      (b1 = false → (hv h2).basepri = bp k → (hv h3).basepri = bp k1))
    (htail : k' = k1 ∧ safe = b1 ∧ HB hv hw' h3) (hpre : HB hv h2 hw) :
    (safe = true → (hv hw').basepri = 0) ∧
    (safe = false → (hv hw).basepri = bp k → (hv hw').basepri = bp k') := by
  obtain ⟨rfl, rfl, hb⟩ := htail
  exact a1_wrap hmid hpre hb

set_option maxHeartbeats 4000000 in
/-- **An interrupt entry keeps A1** (`irq_entry_core`). -/
theorem irq_entry_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {ipsr psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (h : arch.cm4.entry.irq_entry_core ki ai ti k hw tables s ipsr psp control exc_return
      = ok ((plan, safe), k', hw', s')) :
    (safe = true → (hv hw').basepri = 0) ∧
    (safe = false → (hv hw).basepri = bp k → (hv hw').basepri = bp k') := by
  unfold arch.cm4.entry.irq_entry_core at h
  simp only [arch.cm4.entry.WINDOW_DEFER, ↓reduceIte] at h
  bstep h with i, hi
  bstep h with n, hn
  bstep h with i1, hi1
  bstep h with hw1, hhw1
  bstep h with i2, hi2
  rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩
  · bstep h with ⟨hw2, st1⟩, hl
    bstep h with b, hb
    have hbs := bind_ok_inv h
    clear h
    obtain ⟨⟨k1, hw3, st2, r⟩, hm, ht⟩ := hbs
    have hmid : (r.2 = true → (hv hw3).basepri = 0) ∧
        (r.2 = false → (hv hw2).basepri = bp k → (hv hw3).basepri = bp k1) := by
      clear ht
      cases b <;> try dsimp only at hm
      all_goals peel hm
      all_goals irq_mid
    obtain ⟨dp, b1⟩ := r
    have htail : k' = k1 ∧ safe = b1 ∧ HB hv hw' hw3 := by
      clear hmid hm
      peel ht
      all_goals irq_tail
    refine irq_join hmid htail ?_
    hb_close
  · bstep h with b, hb
    have hbs := bind_ok_inv h
    clear h
    obtain ⟨⟨k1, hw3, st2, r⟩, hm, ht⟩ := hbs
    have hmid : (r.2 = true → (hv hw3).basepri = 0) ∧
        (r.2 = false → (hv hw1).basepri = bp k → (hv hw3).basepri = bp k1) := by
      clear ht
      cases b <;> try dsimp only at hm
      all_goals peel hm
      all_goals irq_mid
    obtain ⟨dp, b1⟩ := r
    have htail : k' = k1 ∧ safe = b1 ∧ HB hv hw' hw3 := by
      clear hmid hm
      peel ht
      all_goals irq_tail
    refine irq_join hmid htail ?_
    hb_close

/-! ## An entry that dispatches a Job -/

/-- One case of a dispatch vector's core: no Job is dispatched (the frame
address is 0), or the dispatch of a Job established A1. -/
syntax "a1j_goal" : tactic
macro_rules
  | `(tactic| a1j_goal) => `(tactic| first
      | (intro hp; exact absurd rfl hp)
      | (intro _; a1_call; hb_close; done))

macro "a1j_core " h:ident : tactic => `(tactic| (all_goals first
    | a1j_goal
    | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
        | a1j_goal
        | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
            | a1j_goal
            | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;>
                a1j_goal)))))

set_option maxHeartbeats 16000000 in
/-- **A dispatch vector that dispatches a Job establishes A1**
(`dispatch_vector_core`; a frame address other than 0 is a dispatch). -/
theorem dispatch_vector_core_a1_job [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A}
    {tables : T} {s s' : arch.cm4.plan.ArchState} {n : U16} {t : U8}
    {psp control exc_return : U32} {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (h : arch.cm4.entry.dispatch_vector_core ki ai ti k hw tables s n t psp control exc_return
      = ok ((plan, safe), k', hw', s')) :
    plan.psp ≠ 0#u32 →
    (safe = false → (hv hw').basepri = bp k') ∧ (safe = true → (hv hw').basepri = 0) := by
  unfold arch.cm4.entry.dispatch_vector_core at h
  simp only [arch.cm4.entry.WINDOW_DEFER, ↓reduceIte] at h
  peel h
  a1j_core h

/-- The core the vector's binding selects, after `peel`: only a dispatch
vector returns a frame address other than 0. -/
macro "irq_mid_job" : tactic => `(tactic| first
  | exact dispatch_vector_core_a1_job (plan := Prod.fst _) (safe := Prod.snd _) (by assumption)
  | (intro hp; exact absurd rfl hp))

/-- The deferral at the end, after `peel`: a plan with a frame address
other than 0 is the selected core's, with its safe-state answer and its
Kernel state, and BASEPRI is kept. -/
macro "irq_tail_job" : tactic => `(tactic| first
  | (intro _; exact ⟨rfl, rfl, rfl, by hb_close⟩)
  | (intro hp; exact absurd rfl hp))

set_option maxHeartbeats 4000000 in
/-- **An interrupt entry that dispatches a Job establishes A1**
(`irq_entry_core`; Verus: `r.0.psp != 0 ==> basepri_matches`): whatever
BASEPRI was at the call, at the return to a new Job it is the value the
Kernel's state calls for. -/
theorem irq_entry_core_a1_job [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {ipsr psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (h : arch.cm4.entry.irq_entry_core ki ai ti k hw tables s ipsr psp control exc_return
      = ok ((plan, safe), k', hw', s')) :
    plan.psp ≠ 0#u32 →
    (safe = false → (hv hw').basepri = bp k') ∧ (safe = true → (hv hw').basepri = 0) := by
  unfold arch.cm4.entry.irq_entry_core at h
  simp only [arch.cm4.entry.WINDOW_DEFER, ↓reduceIte] at h
  bstep h with i, hi
  bstep h with n, hn
  bstep h with i1, hi1
  bstep h with hw1, hhw1
  bstep h with i2, hi2
  rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩
  · bstep h with ⟨hw2, st1⟩, hl
    bstep h with b, hb
    have hbs := bind_ok_inv h
    clear h
    obtain ⟨⟨k1, hw3, st2, r⟩, hm, ht⟩ := hbs
    have hmid : r.1.psp ≠ 0#u32 →
        (r.2 = false → (hv hw3).basepri = bp k1) ∧ (r.2 = true → (hv hw3).basepri = 0) := by
      clear ht
      cases b <;> try dsimp only at hm
      all_goals peel hm
      all_goals irq_mid_job
    obtain ⟨dp, b1⟩ := r
    have htail : plan.psp ≠ 0#u32 → plan = dp ∧ k' = k1 ∧ safe = b1 ∧ HB hv hw' hw3 := by
      clear hmid hm
      peel ht
      all_goals irq_tail_job
    intro hp
    obtain ⟨rfl, rfl, rfl, hbk⟩ := htail hp
    exact a1_after (hmid hp) hbk
  · bstep h with b, hb
    have hbs := bind_ok_inv h
    clear h
    obtain ⟨⟨k1, hw3, st2, r⟩, hm, ht⟩ := hbs
    have hmid : r.1.psp ≠ 0#u32 →
        (r.2 = false → (hv hw3).basepri = bp k1) ∧ (r.2 = true → (hv hw3).basepri = 0) := by
      clear ht
      cases b <;> try dsimp only at hm
      all_goals peel hm
      all_goals irq_mid_job
    obtain ⟨dp, b1⟩ := r
    have htail : plan.psp ≠ 0#u32 → plan = dp ∧ k' = k1 ∧ safe = b1 ∧ HB hv hw' hw3 := by
      clear hmid hm
      peel ht
      all_goals irq_tail_job
    intro hp
    obtain ⟨rfl, rfl, rfl, hbk⟩ := htail hp
    exact a1_after (hmid hp) hbk

end P18
