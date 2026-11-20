import Sched.Refine
import Sched.Overrun
open Aeneas Aeneas.Std Result

/-! # The Kernel's logic returns: no overflow, no index out of bounds, no panic (2026-11-19)

Property 21 of the design, for the part Verus proves it of (`rsk-logic`):
called with inputs that satisfy its precondition, a Kernel function does
not overflow, index out of bounds, or panic.

In the translation Aeneas makes, each of those is the function not
returning `ok`: an arithmetic operation, an array access and a panic are
monadic steps that fail. So the property is, function by function, "for
the inputs the precondition admits, the function returns" (`⦃ _ ⦄` is
total: the function returns a value and the value satisfies the
postcondition). The other files of this library prove "if the function
returns, then ..."; this one proves that it does.

The preconditions are the structural part of Verus's invariants: indices
within their arrays, the stack and the lock stack within their bounds
(`SInv`), the log well formed, the instant within the operating duration. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- A total specification gives a result. -/
theorem spec_returns {α} {x : Result α} {p : α → Prop} (h : x ⦃ p ⦄) :
    ∃ r, x = ok r ∧ p r := by
  cases x with
  | ret v => exact ⟨v, rfl, (WP.spec_ok v).mp h⟩
  | vis e k => exact absurd h (by simp [WP.spec_vis])
  | div => exact absurd h (by simp [WP.spec_div])

/-! ## The Budget accounts -/

theorem reset_total (b : logic.budget.Budget NT) (t : U8) (ht : t.val < NT.val) :
    logic.budget.Budget.reset b t ⦃ _ => True ⦄ := by
  unfold logic.budget.Budget.reset
  simp only [lift, bind_ok]
  step*

theorem charge_total (b : logic.budget.Budget NT) (t : U8) (c : U32) (ht : t.val < NT.val) :
    logic.budget.Budget.charge b t c ⦃ _ => True ⦄ := by
  unfold logic.budget.Budget.charge
  simp only [lift, bind_ok]
  step*

theorem report_total (b : logic.budget.Budget NT) (cfg : logic.Config NT NR NP) (t : U8)
    (ht : t.val < NT.val) : logic.budget.Budget.report b cfg t ⦃ _ => True ⦄ := by
  unfold logic.budget.Budget.report
  simp only [lift, bind_ok]
  step*

theorem overrun_total (b : logic.budget.Budget NT) (cfg : logic.Config NT NR NP) (t : U8)
    (ht : t.val < NT.val) : logic.budget.Budget.overrun b cfg t ⦃ _ => True ⦄ := by
  unfold logic.budget.Budget.overrun
  simp only [lift, bind_ok]
  step*

theorem remaining_total (b : logic.budget.Budget NT) (cfg : logic.Config NT NR NP) (t : U8)
    (ht : t.val < NT.val) : logic.budget.Budget.remaining b cfg t ⦃ _ => True ⦄ := by
  unfold logic.budget.Budget.remaining
  simp only [lift, bind_ok]
  step*

/-! ## The level encoding -/

theorem encode_level_total (l : U8) (hl : l.val ≤ 8) :
    arch.bound.encode_level l ⦃ _ => True ⦄ := by
  unfold arch.bound.encode_level
  step*

theorem basepri_for_total (top c : U8) (ht : top.val ≤ 8) (hc : c.val ≤ 8) :
    arch.bound.basepri_for top c ⦃ _ => True ⦄ := by
  unfold arch.bound.basepri_for arch.bound.basepri_for_ceiling
  split
  · simp only [bind_ok]
    split
    · exact (WP.spec_ok _).mpr trivial
    · exact encode_level_total _ ht
  · simp only [bind_ok]
    split
    · exact (WP.spec_ok _).mpr trivial
    · exact encode_level_total _ hc

/-! ## The scheduler state -/

/-- The bounds of the scheduler state: the stack within the Task count and
holding Task identifiers, the lock stack within its capacity and holding
Resource identifiers. -/
def SInv (s : logic.sched.Sched NT NR NL) : Prop :=
  s.depth.val ≤ NT.val ∧ (∀ p, p < s.depth.val → stk s p < NT.val) ∧
  s.lock_len.val ≤ NL.val ∧ (∀ p, p < s.lock_len.val → lres s p < NR.val)

theorem make_pending_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (t : U8) (ht : t.val < NT.val) :
    logic.sched.Sched.make_pending s cfg t ⦃ _ => True ⦄ := by
  unfold logic.sched.Sched.make_pending
  simp only [lift, bind_ok]
  step*

theorem defer_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (t : U8) (ht : t.val < NT.val) : logic.sched.Sched.defer s cfg t ⦃ _ => True ⦄ := by
  unfold logic.sched.Sched.defer
  simp only [lift, bind_ok]
  step*

theorem clear_release_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (t : U8) (ht : t.val < NT.val) :
    logic.sched.Sched.clear_release s cfg t ⦃ _ => True ⦄ := by
  unfold logic.sched.Sched.clear_release
  simp only [lift, bind_ok]
  step*

theorem start_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (t : U8) (ht : t.val < NT.val) (hd : s.depth.val < NT.val) :
    logic.sched.Sched.start s cfg t ⦃ _ => True ⦄ := by
  unfold logic.sched.Sched.start
  simp only [lift, bind_ok]
  step*

theorem kill_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (i : Usize) (hi : i.val < s.depth.val) (hs : SInv s) :
    logic.sched.Sched.kill s cfg i ⦃ _ => True ⦄ := by
  obtain ⟨h1, h2, _, _⟩ := hs
  have hst := h2 i.val hi
  simp only [stk] at hst
  unfold logic.sched.Sched.kill
  have hlen : i.val < s.stack.val.length := by have := s.stack.property; scalar_tac
  step as ⟨ t, ht ⟩
  have ht' : s.stack.val[i.val]! = t := by rw [ht]; exact getElem!_pos s.stack.val i.val hlen
  rw [ht'] at hst
  simp only [lift, bind_ok]
  step*

theorem complete_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (hd : 0 < s.depth.val) (hdn : s.depth.val ≤ NT.val)
    (hst : stk s (s.depth.val - 1) < NT.val) :
    logic.sched.Sched.complete s cfg ⦃ _ => True ⦄ := by
  unfold logic.sched.Sched.complete
  step as ⟨ d, hdv ⟩
  have hlen : d.val < s.stack.val.length := by have := s.stack.property; scalar_tac
  step as ⟨ t, ht ⟩
  have ht' : s.stack.val[d.val]! = t := by rw [ht]; exact getElem!_pos s.stack.val d.val hlen
  have : d.val = s.depth.val - 1 := by scalar_tac
  simp only [stk] at hst
  rw [← this, ht'] at hst
  simp only [lift, bind_ok]
  step*

theorem lock_loop_total (a : Array U8 NL) (i : Usize) (r : U8) (hi : i.val ≤ NL.val) :
    ∀ (e : Usize) (b : Bool), e.val ≤ i.val →
      logic.sched.Sched.lock_loop a i r e b ⦃ _ => True ⦄ := by
  intro e b he
  unfold logic.sched.Sched.lock_loop
  apply loop.spec_decr_nat (fun (x : Usize × Bool) => i.val - x.1.val)
    (fun x => x.1.val ≤ i.val)
  · intro x hx
    obtain ⟨e, b⟩ := x
    try dsimp only at hx ⊢
    unfold logic.sched.Sched.lock_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : e.val < i.val := by scalar_tac
      step as ⟨ x, hx' ⟩
      split
      · simp only [bind_ok]
        step as ⟨ e1, he1 ⟩
        exact ⟨by scalar_tac, by scalar_tac⟩
      · simp only [bind_ok]
        step as ⟨ e1, he1 ⟩
        exact ⟨by scalar_tac, by scalar_tac⟩
    · exact (WP.spec_ok _).mpr trivial
  · exact he

theorem recompute_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (n : Usize) (hn : n.val ≤ NL.val) (hr : ∀ p, p < n.val → lres s p < NR.val) :
    logic.sched.Sched.recompute_ceiling s cfg n ⦃ _ => True ⦄ := by
  unfold logic.sched.Sched.recompute_ceiling logic.sched.Sched.recompute_ceiling_loop
  apply loop.spec_decr_nat (fun (x : U8 × Usize) => n.val - x.2.val)
    (fun x => x.2.val ≤ n.val)
  · intro x hx
    obtain ⟨c, e⟩ := x
    try dsimp only at hx ⊢
    unfold logic.sched.Sched.recompute_ceiling_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : e.val < n.val := by scalar_tac
      have hlen : e.val < s.lock_res.val.length := by have := s.lock_res.property; scalar_tac
      step as ⟨ i, hi ⟩
      have hi' : s.lock_res.val[e.val]! = i := by
        rw [hi]; exact getElem!_pos s.lock_res.val e.val hlen
      have hres := hr e.val hlt'
      simp only [lres] at hres
      rw [hi'] at hres
      simp only [lift, bind_ok]
      step as ⟨ rc, hrc ⟩
      split
      · simp only [bind_ok]
        step as ⟨ e1, he1 ⟩
        exact ⟨by scalar_tac, by scalar_tac⟩
      · simp only [bind_ok]
        step as ⟨ e1, he1 ⟩
        exact ⟨by scalar_tac, by scalar_tac⟩
    · exact (WP.spec_ok _).mpr trivial
  · scalar_tac

theorem end_job_loop_total (a : Array Usize NL) (d : Usize) (n0 : Usize)
    (hn : n0.val ≤ NL.val) :
    logic.sched.Sched.end_job_loop a d n0 ⦃ n => n.val ≤ n0.val ⦄ := by
  unfold logic.sched.Sched.end_job_loop
  apply loop.spec_decr_nat (fun (x : Usize) => x.val) (fun x => x.val ≤ n0.val)
  · intro n hx
    unfold logic.sched.Sched.end_job_loop.body
    split
    · rename_i hgt
      step as ⟨ i, hi ⟩
      step as ⟨ i1, hi1 ⟩
      split
      · exact (WP.spec_ok _).mpr ⟨by scalar_tac, by scalar_tac⟩
      · exact (WP.spec_ok _).mpr hx
    · exact (WP.spec_ok _).mpr hx
  · exact Nat.le_refl _

theorem lock_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP) (r : U8)
    (hs : SInv s) (hd : 0 < s.depth.val) (hr : r.val < NR.val) :
    logic.sched.Sched.lock s cfg r ⦃ _ => True ⦄ := by
  obtain ⟨h1, h2, h3, h4⟩ := hs
  unfold logic.sched.Sched.lock
  step as ⟨ d, hdv ⟩
  have hdd : d.val = s.depth.val - 1 := by scalar_tac
  have hlen : d.val < s.stack.val.length := by have := s.stack.property; scalar_tac
  step as ⟨ t, ht ⟩
  have ht' : s.stack.val[d.val]! = t := by rw [ht]; exact getElem!_pos s.stack.val d.val hlen
  have hst := h2 d.val (by omega)
  simp only [stk] at hst
  rw [ht'] at hst
  simp only [lift, bind_ok]
  step as ⟨ a, ha ⟩
  step as ⟨ b, hb ⟩
  split
  · refine WP.spec_bind (lock_loop_total s.lock_res s.lock_len r h3 0#usize false (by scalar_tac))
      (fun reentry _ => ?_)
    split
    · exact (WP.spec_ok _).mpr trivial
    · split
      · exact (WP.spec_ok _).mpr trivial
      · rename_i hnl
        have hll : s.lock_len.val < NL.val := by scalar_tac
        step*
  · exact (WP.spec_ok _).mpr trivial

theorem unlock_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP) (r : U8)
    (hs : SInv s) (hd : 0 < s.lock_len.val → 0 < s.depth.val) :
    logic.sched.Sched.unlock s cfg r ⦃ _ => True ⦄ := by
  obtain ⟨h1, h2, h3, h4⟩ := hs
  unfold logic.sched.Sched.unlock
  split
  · exact (WP.spec_ok _).mpr trivial
  · rename_i hne
    have hpos : 0 < s.lock_len.val := by scalar_tac
    have hdp := hd hpos
    step as ⟨ n, hn ⟩
    have hnn : n.val = s.lock_len.val - 1 := by scalar_tac
    step as ⟨ i, hi ⟩
    split
    · exact (WP.spec_ok _).mpr trivial
    · step as ⟨ i1, hi1 ⟩
      step as ⟨ i2, hi2 ⟩
      split
      · exact (WP.spec_ok _).mpr trivial
      · refine WP.spec_bind (recompute_total { s with lock_len := n } cfg n (by omega)
          (fun p hp => h4 p (by omega))) (fun c _ => ?_)
        exact (WP.spec_ok _).mpr trivial

theorem end_job_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (hs : SInv s) (hd : 0 < s.depth.val) :
    logic.sched.Sched.end_job s cfg ⦃ _ => True ⦄ := by
  obtain ⟨h1, h2, h3, h4⟩ := hs
  unfold logic.sched.Sched.end_job
  step as ⟨ d, hdv ⟩
  refine WP.spec_bind (end_job_loop_total s.lock_depth d s.lock_len h3) (fun n hn => ?_)
  step as ⟨ released, hrel ⟩
  refine WP.spec_bind (recompute_total { s with lock_len := n } cfg n (by omega)
    (fun p hp => h4 p (by omega))) (fun c _ => ?_)
  refine WP.spec_bind (complete_total { s with ceiling := c, lock_len := n } cfg hd h1
    (h2 (s.depth.val - 1) (by omega))) (fun s1 _ => ?_)
  exact (WP.spec_ok _).mpr trivial

/-- Job end (`Kernel::job_return`, the responses): the Job on top leaves. -/
theorem finish_top_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (hs : SInv s) (hd : 0 < s.depth.val) :
    logic.sched.Sched.finish_top s cfg ⦃ _ => True ⦄ := by
  unfold logic.sched.Sched.finish_top
  exact end_job_total s cfg hs hd

/-- Closes the branches of a function's tail that only return. -/
syntax "ok_tac" : tactic
macro_rules
  | `(tactic| ok_tac) => `(tactic|
      repeat' (first | exact (WP.spec_ok _).mpr trivial | split | simp only [bind_ok]))

theorem select_loop_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (bt t : Usize) (hb : bt.val ≤ NT.val) (ht : t.val ≤ NT.val) :
    logic.sched.Sched.select_loop s cfg bt t ⦃ r =>
      r.1 = s ∧ r.2.1 = cfg ∧ r.2.2.val ≤ NT.val ⦄ := by
  unfold logic.sched.Sched.select_loop
  apply loop.spec_decr_nat
    (fun (x : logic.Config NT NR NP × Usize × Usize) => NT.val - x.2.2.val)
    (fun x => x.1 = cfg ∧ x.2.1.val ≤ NT.val ∧ x.2.2.val ≤ NT.val)
  · intro x hx
    obtain ⟨cfg1, bt, t⟩ := x
    obtain ⟨hc, hb, ht⟩ := hx
    try dsimp only at hc hb ht ⊢
    subst hc
    unfold logic.sched.Sched.select_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : t.val < NT.val := by scalar_tac
      step as ⟨ i, hi ⟩
      split
      · split
        · simp only [bind_ok]
          step as ⟨ t1, ht1 ⟩
          exact ⟨by scalar_tac, by scalar_tac, by scalar_tac⟩
        · rename_i hne
          have hbl : bt.val < NT.val := by scalar_tac
          step as ⟨ tc, htc ⟩
          step as ⟨ tc1, htc1 ⟩
          split
          · simp only [bind_ok]
            step as ⟨ t1, ht1 ⟩
            exact ⟨by scalar_tac, by scalar_tac, by scalar_tac⟩
          · simp only [bind_ok]
            step as ⟨ t1, ht1 ⟩
            exact ⟨by scalar_tac, by scalar_tac, by scalar_tac⟩
      · simp only [bind_ok]
        step as ⟨ t1, ht1 ⟩
        exact ⟨by scalar_tac, by scalar_tac, by scalar_tac⟩
    · exact (WP.spec_ok _).mpr ⟨rfl, rfl, hb⟩
  · exact ⟨rfl, hb, ht⟩

/-- `Sched::select`: the eligibility test, over every Task. -/
theorem select_total (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (hs : SInv s) (hNT : NT.val ≤ 256) :
    logic.sched.Sched.select s cfg ⦃ _ => True ⦄ := by
  obtain ⟨h1, h2, h3, h4⟩ := hs
  unfold logic.sched.Sched.select
  refine WP.spec_bind (select_loop_total s cfg NT 0#usize (Nat.le_refl _) (by scalar_tac))
    (fun x hx => ?_)
  obtain ⟨s1, cfg1, bt⟩ := x
  obtain ⟨e1, e2, hbt⟩ := hx
  try dsimp only at e1
  try dsimp only at e2
  try dsimp only at hbt
  subst e1; subst e2
  simp only [Aeneas.Std.uncurry, lift, bind_ok]
  by_cases hlt : bt < NT
  · simp only [hlt, ite_true, bind_ok]
    have hlt' : bt.val < NT.val := by scalar_tac
    have hc : (UScalar.cast .Usize (UScalar.cast .U8 bt)).val = bt.val := by scalar_tac
    try simp only [lift, bind_ok]
    step as ⟨ tc, htc ⟩
    split
    · try simp only [bind_ok]
      step as ⟨ i2, hi2 ⟩
      ok_tac
    · rename_i hd
      have hdp : 0 < s1.depth.val := by scalar_tac
      step as ⟨ i1, hi1 ⟩
      have hi1' : i1.val = s1.depth.val - 1 := by scalar_tac
      have hlen : i1.val < s1.stack.val.length := by have := s1.stack.property; scalar_tac
      step as ⟨ i2, hi2 ⟩
      have hi2' : s1.stack.val[i1.val]! = i2 := by
        rw [hi2]; exact getElem!_pos s1.stack.val i1.val hlen
      have hst := h2 i1.val (by omega)
      simp only [stk] at hst
      rw [hi2'] at hst
      try simp only [lift, bind_ok]
      step as ⟨ tc1, htc1 ⟩
      split
      · try simp only [bind_ok]
        step as ⟨ i3, hi3 ⟩
        ok_tac
      · try simp only [bind_ok]
        step as ⟨ i3, hi3 ⟩
        ok_tac
  · simp only [hlt, ite_false, bind_ok]
    exact (WP.spec_ok _).mpr trivial

/-! ## The Kernel's entries for a Job: dispatch, lock, unlock -/

/-- A total function with a statement about its result, from the two
halves: it returns, and what it returns satisfies the statement. -/
theorem spec_of_total {α} {x : Result α} {P : α → Prop} (ht : x ⦃ _ => True ⦄)
    (h : ∀ r, x = ok r → P r) : x ⦃ P ⦄ := by
  obtain ⟨r, hr, _⟩ := spec_returns ht
  rw [hr]
  exact (WP.spec_ok r).mpr (h r hr)

/-- The configuration names slots of the table: a Budget and a deadline
slot for every Task, a release slot for every Task that has one (a
Periodic_Task, or a Sporadic_Task whose MIT policy defers; a discarding
Sporadic_Task has none, `NO_SLOT`). 2026-11-20: the release clause was
unconditional, which no configuration with a discarding Sporadic_Task
satisfies (`Config::slot_ok`). -/
def CfgOk (cfg : logic.Config NT NR NP) (NS : Usize) : Prop :=
  ∀ t, t < NT.val → (cfg.slot_budget.val[t]!).val < NS.val ∧
    (cfg.slot_deadline.val[t]!).val < NS.val ∧
    ∀ tc, cfg.tasks.val[t]? = some tc →
      (tc.kind = logic.KIND_PERIODIC ∨ tc.mit_policy = logic.MIT_DEFER) →
      (cfg.slot_release.val[t]!).val < NS.val

/-- `Kernel::push_event`: the log takes the event. -/
theorem push_event_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (kind task partition : U8) (prs : U64) (detail : U32)
    (hl : Lwf k.log) :
    logic.kernel.Kernel.push_event k cfg kind task partition prs detail ⦃ k' =>
      Lwf k'.log ∧ k'.sched = k.sched ∧ k'.now = k.now ∧ k'.slots = k.slots ∧
      k'.timing = k.timing ∧ k'.budget = k.budget ⦄ := by
  unfold logic.kernel.Kernel.push_event
  refine WP.spec_bind (log_push_spec _ _ hl) (fun l hl' => ?_)
  exact (WP.spec_ok _).mpr ⟨hl'.1, rfl, rfl, rfl, rfl, rfl⟩

theorem cycles_per_tick : logic.kernel.CYCLES_PER_TICK = 4#u64 := by
  unfold logic.kernel.CYCLES_PER_TICK; rfl

/-- `Kernel::arm_budget`: the remaining Budget in ticks added to the
instant does not overflow, and the slot is the Task's. -/
theorem arm_budget_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (t : U8) (ht : t.val < NT.val) (hc : CfgOk cfg NS)
    (hn : k.now.val ≤ 4611686018427387904) :
    logic.kernel.Kernel.arm_budget k cfg t ⦃ k' =>
      k'.sched = k.sched ∧ k'.log = k.log ∧ k'.now = k.now ∧ k'.timing = k.timing ∧
      k'.budget = k.budget ⦄ := by
  unfold logic.kernel.Kernel.arm_budget
  refine WP.spec_bind (remaining_total k.budget cfg t ht) (fun r _ => ?_)
  simp only [lift, bind_ok, cycles_per_tick]
  have hcast : (UScalar.cast .U64 r).val = r.val := by scalar_tac
  step as ⟨ i1, hi1 ⟩
  step as ⟨ i2, hi2 ⟩
  step as ⟨ ticks, hticks ⟩
  step as ⟨ i3, hi3 ⟩
  have hlen : t.val < cfg.slot_budget.val.length := by
    have := cfg.slot_budget.property; scalar_tac
  have hsb := (hc t.val ht).1
  split
  · simp only [bind_ok]
    step as ⟨ i5, hi5 ⟩
    have hi5' : cfg.slot_budget.val[t.val]! = i5 := by
      simp only [U8.cast_Usize_val_eq] at hi5
      rw [hi5]; exact getElem!_pos cfg.slot_budget.val t.val hlen
    rw [hi5'] at hsb
    refine WP.spec_bind (arm_spec k.slots _ _ _ _ (by scalar_tac)) (fun s' _ => ?_)
    exact (WP.spec_ok _).mpr ⟨rfl, rfl, rfl, rfl, rfl⟩
  · simp only [bind_ok]
    step as ⟨ i5, hi5 ⟩
    have hi5' : cfg.slot_budget.val[t.val]! = i5 := by
      simp only [U8.cast_Usize_val_eq] at hi5
      rw [hi5]; exact getElem!_pos cfg.slot_budget.val t.val hlen
    rw [hi5'] at hsb
    refine WP.spec_bind (arm_spec k.slots _ _ _ _ (by scalar_tac)) (fun s' _ => ?_)
    exact (WP.spec_ok _).mpr ⟨rfl, rfl, rfl, rfl, rfl⟩

theorem job_started_total (tm : logic.time.Timing NT) (cfg : logic.Config NT NR NP) (t : U8)
    (ht : t.val < NT.val) : logic.time.Timing.job_started tm cfg t ⦃ _ => True ⦄ := by
  unfold logic.time.Timing.job_started
  simp only [lift, bind_ok]
  step*

/-- `Kernel::dispatch`: the entry of a dispatch stub. -/
theorem dispatch_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (t : U8) (pc : U32) (hs : SInv k.sched)
    (ht : t.val < NT.val) (hd : k.sched.depth.val < NT.val) (hc : CfgOk cfg NS)
    (hn : k.now.val ≤ 4611686018427387904) :
    logic.kernel.Kernel.dispatch k cfg t pc ⦃ _ => True ⦄ := by
  obtain ⟨h1, h2, h3, h4⟩ := hs
  unfold logic.kernel.Kernel.dispatch
  by_cases hpos : k.sched.depth > 0#usize
  · have hpos' : 0 < k.sched.depth.val := by scalar_tac
    simp only [hpos, ite_true]
    step as ⟨ i, hi ⟩
    have hlen : i.val < k.sched.stack.val.length := by
      have := k.sched.stack.property; scalar_tac
    step as ⟨ prev, hprev ⟩
    have hprev' : k.sched.stack.val[i.val]! = prev := by
      rw [hprev]; exact getElem!_pos k.sched.stack.val i.val hlen
    have hst := h2 i.val (by omega)
    simp only [stk] at hst
    rw [hprev'] at hst
    refine WP.spec_bind (charge_total k.budget prev pc hst) (fun b0 _ => ?_)
    simp only [lift, bind_ok]
    have hlen2 : prev.val < cfg.slot_budget.val.length := by
      have := cfg.slot_budget.property; scalar_tac
    step as ⟨ sb, hsb ⟩
    have hsb' : cfg.slot_budget.val[prev.val]! = sb := by
      simp only [U8.cast_Usize_val_eq] at hsb
      rw [hsb]; exact getElem!_pos cfg.slot_budget.val prev.val hlen2
    have hsl := (hc prev.val hst).1
    rw [hsb'] at hsl
    refine WP.spec_bind (disarm_spec k.slots _ (by scalar_tac)) (fun sl _ => ?_)
    refine WP.spec_bind (start_total k.sched cfg t ht hd) (fun s1 _ => ?_)
    refine WP.spec_bind (job_started_total k.timing cfg t ht) (fun t1 _ => ?_)
    refine WP.spec_bind (reset_total b0 t ht) (fun b1 _ => ?_)
    exact WP.spec_mono (arm_budget_total _ cfg t ht hc hn) (fun _ _ => trivial)
  · simp only [hpos, ite_false, bind_ok, Aeneas.Std.uncurry]
    refine WP.spec_bind (start_total k.sched cfg t ht hd) (fun s1 _ => ?_)
    refine WP.spec_bind (job_started_total k.timing cfg t ht) (fun t1 _ => ?_)
    refine WP.spec_bind (reset_total k.budget t ht) (fun b1 _ => ?_)
    exact WP.spec_mono (arm_budget_total _ cfg t ht hc hn) (fun _ _ => trivial)

/-- A refused `lock` leaves the scheduler state as it is. -/
theorem lock_refused {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.sched.Sched.lock s cfg r = ok (false, s')) : s' = s := by
  unfold logic.sched.Sched.lock at h
  peel h
  all_goals first
    | rfl
    | (rename_i hh; cases hh)
    | simp_all

/-- A refused `unlock` leaves the scheduler state as it is. -/
theorem unlock_refused {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {r : U8} (h : logic.sched.Sched.unlock s cfg r = ok (false, s')) : s' = s := by
  unfold logic.sched.Sched.unlock at h
  peel h
  all_goals first
    | rfl
    | (rename_i hh; cases hh)
    | simp_all

/-- `Kernel::svc_lock`: the service, and the record of a refused call. -/
theorem svc_lock_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (r : U8) (hs : SInv k.sched) (hd : 0 < k.sched.depth.val)
    (hr : r.val < NR.val) (hl : Lwf k.log) :
    logic.kernel.Kernel.svc_lock k cfg r ⦃ _ => True ⦄ := by
  have hs' := hs
  obtain ⟨h1, h2, h3, h4⟩ := hs'
  unfold logic.kernel.Kernel.svc_lock
  obtain ⟨res, hres, _⟩ := spec_returns (lock_total k.sched cfg r hs hd hr)
  rw [hres]
  obtain ⟨b, s'⟩ := res
  simp only [bind_ok, Aeneas.Std.uncurry]
  cases b
  · have hsame := lock_refused hres
    subst hsame
    simp only [Bool.false_eq_true, ite_false, bind_ok]
    step as ⟨ i, hi ⟩
    have hlen : i.val < k.sched.stack.val.length := by have := k.sched.stack.property; scalar_tac
    step as ⟨ t, ht ⟩
    have ht' : k.sched.stack.val[i.val]! = t := by rw [ht]; exact getElem!_pos k.sched.stack.val i.val hlen
    have hst := h2 i.val (by omega)
    simp only [stk] at hst
    rw [ht'] at hst
    simp only [lift, bind_ok]
    step as ⟨ tc, htc ⟩
    step as ⟨ i2, hi2 ⟩
    refine WP.spec_bind (push_event_total _ cfg _ _ _ _ _ hl) (fun k1 _ => ?_)
    exact (WP.spec_ok _).mpr trivial
  · simp only [ite_true, bind_ok]
    exact (WP.spec_ok _).mpr trivial

/-- `Kernel::svc_unlock`: the service, and the record of a refused call. -/
theorem svc_unlock_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (r : U8) (hs : SInv k.sched) (hd : 0 < k.sched.depth.val)
    (hl : Lwf k.log) :
    logic.kernel.Kernel.svc_unlock k cfg r ⦃ _ => True ⦄ := by
  have hs' := hs
  obtain ⟨h1, h2, h3, h4⟩ := hs'
  unfold logic.kernel.Kernel.svc_unlock
  obtain ⟨res, hres, _⟩ := spec_returns (unlock_total k.sched cfg r hs (fun _ => hd))
  rw [hres]
  obtain ⟨b, s'⟩ := res
  simp only [bind_ok, Aeneas.Std.uncurry]
  cases b
  · have hsame := unlock_refused hres
    subst hsame
    simp only [Bool.false_eq_true, ite_false, bind_ok]
    step as ⟨ i, hi ⟩
    have hlen : i.val < k.sched.stack.val.length := by have := k.sched.stack.property; scalar_tac
    step as ⟨ t, ht ⟩
    have ht' : k.sched.stack.val[i.val]! = t := by rw [ht]; exact getElem!_pos k.sched.stack.val i.val hlen
    have hst := h2 i.val (by omega)
    simp only [stk] at hst
    rw [ht'] at hst
    simp only [lift, bind_ok]
    step as ⟨ tc, htc ⟩
    step as ⟨ i2, hi2 ⟩
    refine WP.spec_bind (push_event_total _ cfg _ _ _ _ _ hl) (fun k1 _ => ?_)
    exact (WP.spec_ok _).mpr trivial
  · simp only [ite_true, bind_ok]
    exact (WP.spec_ok _).mpr trivial

/-! ## Job return -/

theorem try_unmask_total (tm : logic.time.Timing NT) (cfg : logic.Config NT NR NP) (t : U8)
    (now : U64) (je : Bool) (ht : t.val < NT.val) :
    logic.time.Timing.try_unmask tm cfg t now je ⦃ _ => True ⦄ := by
  unfold logic.time.Timing.try_unmask
  simp only [lift, bind_ok]
  step*

theorem unmask_instant_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (t : U8) (ht : t.val < NT.val) :
    logic.kernel.Kernel.unmask_instant k cfg t ⦃ _ => True ⦄ := by
  unfold logic.kernel.Kernel.unmask_instant
  simp only [lift, bind_ok]
  step*

/-- `Kernel::end_mit_window`: the unmask of an ended Job's source, or the
event that will lift it. -/
theorem end_mit_window_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (t : U8) (ht : t.val < NT.val) (hc : CfgOk cfg NS) :
    logic.kernel.Kernel.end_mit_window k cfg t ⦃ k' =>
      k'.sched = k.sched ∧ k'.now = k.now ⦄ := by
  unfold logic.kernel.Kernel.end_mit_window
  refine WP.spec_bind (try_unmask_total k.timing cfg t k.now true ht) (fun x _ => ?_)
  obtain ⟨unmasked, t1⟩ := x
  simp only [Aeneas.Std.uncurry]
  split
  · exact (WP.spec_ok _).mpr ⟨rfl, rfl⟩
  · simp only [lift, bind_ok]
    step as ⟨ i1, hi1 ⟩
    split
    · refine WP.spec_bind (unmask_instant_total _ cfg t ht) (fun a _ => ?_)
      have hlen : t.val < cfg.slot_deadline.val.length := by
        have := cfg.slot_deadline.property; scalar_tac
      step as ⟨ i3, hi3 ⟩
      have hi3' : cfg.slot_deadline.val[t.val]! = i3 := by
        simp only [U8.cast_Usize_val_eq] at hi3
        rw [hi3]; exact getElem!_pos cfg.slot_deadline.val t.val hlen
      have hsl := (hc t.val ht).2.1
      rw [hi3'] at hsl
      refine WP.spec_bind (arm_spec k.slots _ _ _ _ (by scalar_tac)) (fun s' _ => ?_)
      exact (WP.spec_ok _).mpr ⟨rfl, rfl⟩
    · exact (WP.spec_ok _).mpr ⟨rfl, rfl⟩

/-- `Kernel::job_return`: the exit of a dispatch stub. -/
theorem job_return_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (ec : U32) (hs : SInv k.sched) (hd : 0 < k.sched.depth.val)
    (hc : CfgOk cfg NS) (hl : Lwf k.log) (hn : k.now.val ≤ 4611686018427387904) :
    logic.kernel.Kernel.job_return k cfg ec ⦃ _ => True ⦄ := by
  have hs' := hs
  obtain ⟨h1, h2, h3, h4⟩ := hs'
  unfold logic.kernel.Kernel.job_return
  step as ⟨ i, hi ⟩
  have hlen : i.val < k.sched.stack.val.length := by
    have := k.sched.stack.property; scalar_tac
  step as ⟨ t, ht ⟩
  have ht' : k.sched.stack.val[i.val]! = t := by
    rw [ht]; exact getElem!_pos k.sched.stack.val i.val hlen
  have hst := h2 i.val (by omega)
  simp only [stk] at hst
  rw [ht'] at hst
  refine WP.spec_bind (charge_total k.budget t ec hst) (fun b _ => ?_)
  simp only [lift, bind_ok]
  have hlen2 : t.val < cfg.slot_budget.val.length := by
    have := cfg.slot_budget.property; scalar_tac
  step as ⟨ sb, hsb ⟩
  have hsb' : cfg.slot_budget.val[t.val]! = sb := by
    simp only [U8.cast_Usize_val_eq] at hsb
    rw [hsb]; exact getElem!_pos cfg.slot_budget.val t.val hlen2
  have hsl := (hc t.val hst).1
  rw [hsb'] at hsl
  refine WP.spec_bind (disarm_spec k.slots _ (by scalar_tac)) (fun sl _ => ?_)
  step as ⟨ jb, hjb ⟩
  refine WP.spec_bind (spec_of_total (finish_top_total k.sched cfg hs hd)
    (fun r h => finish_top_views (r := r.1) (s' := r.2) h)) (fun x hx => ?_)
  obtain ⟨released, s1⟩ := x
  dsimp only at hx
  obtain ⟨_, _, _, _, _, v_stk, v_dep, v_res, _, v_len, _, _, _⟩ := hx
  simp only [Aeneas.Std.uncurry]
  have hs1 : SInv s1 := by
    refine ⟨by omega, fun p hp => ?_, by omega, fun p hp => ?_⟩
    · rw [v_stk]; exact h2 p (by omega)
    · rw [v_res]; exact h4 p (by omega)
  refine WP.spec_bind
    (Pₘ := fun self1 : logic.kernel.Kernel NT NR NP NL NS NE =>
      self1.sched = s1 ∧ self1.now = k.now) ?_ (fun self1 hself1 => ?_)
  · split
    · step as ⟨ tc, htc ⟩
      split
      · simp only [bind_ok]
        step as ⟨ i7, hi7 ⟩
        exact WP.spec_mono (push_event_total _ cfg _ _ _ _ _ hl)
          (fun k1 hk1 => ⟨hk1.2.1, hk1.2.2.1⟩)
      · simp only [bind_ok]
        step as ⟨ i7, hi7 ⟩
        exact WP.spec_mono (push_event_total _ cfg _ _ _ _ _ hl)
          (fun k1 hk1 => ⟨hk1.2.1, hk1.2.2.1⟩)
    · exact (WP.spec_ok _).mpr ⟨rfl, rfl⟩
  · obtain ⟨e1, e2⟩ := hself1
    step as ⟨ i7, hi7 ⟩
    refine WP.spec_bind (Pₘ := fun _ : logic.time.Slots NS => True) ?_ (fun s2 _ => ?_)
    · split
      · have hlen3 : t.val < cfg.slot_deadline.val.length := by
          have := cfg.slot_deadline.property; scalar_tac
        step as ⟨ i9, hi9 ⟩
        have hi9' : cfg.slot_deadline.val[t.val]! = i9 := by
          simp only [U8.cast_Usize_val_eq] at hi9
          rw [hi9]; exact getElem!_pos cfg.slot_deadline.val t.val hlen3
        have hsl2 := (hc t.val hst).2.1
        rw [hi9'] at hsl2
        exact WP.spec_mono (disarm_spec self1.slots _ (by scalar_tac)) (fun _ _ => trivial)
      · exact (WP.spec_ok _).mpr trivial
    · step as ⟨ tc, htc ⟩
      refine WP.spec_bind
        (Pₘ := fun self2 : logic.kernel.Kernel NT NR NP NL NS NE =>
          self2.sched = s1 ∧ self2.now = k.now) ?_ (fun self2 hself2 => ?_)
      · split
        · exact WP.spec_mono (end_mit_window_total _ cfg t hst hc)
            (fun k' hk' => ⟨hk'.1.trans e1, hk'.2.trans e2⟩)
        · exact (WP.spec_ok _).mpr ⟨e1, e2⟩
      · obtain ⟨f1, f2⟩ := hself2
        split
        · rename_i hpos
          step as ⟨ i9, hi9 ⟩
          have hlen4 : i9.val < self2.sched.stack.val.length := by
            have := self2.sched.stack.property; scalar_tac
          step as ⟨ resumed, hres ⟩
          have hres' : self2.sched.stack.val[i9.val]! = resumed := by
            rw [hres]; exact getElem!_pos self2.sched.stack.val i9.val hlen4
          have hrt : resumed.val < NT.val := by
            have := hs1.2.1 i9.val (by rw [← f1]; scalar_tac)
            simp only [stk] at this
            rw [← f1, hres'] at this
            exact this
          exact WP.spec_mono (arm_budget_total self2 cfg resumed hrt hc (by rw [f2]; exact hn))
            (fun _ _ => trivial)
        · exact (WP.spec_ok _).mpr trivial

/-! ## The steps every Kernel entry shares -/

/-- `Kernel::observe`: the reading of the time base. -/
theorem observe_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (now : U64) :
    logic.kernel.Kernel.observe k cfg now ⦃ _ => True ⦄ := by
  unfold logic.kernel.Kernel.observe
  ok_tac

/-- `Kernel::select`: the Task the entry pends. -/
theorem kernel_select_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (hs : SInv k.sched) (hNT : NT.val ≤ 256) :
    logic.kernel.Kernel.select k cfg ⦃ _ => True ⦄ := by
  unfold logic.kernel.Kernel.select
  split
  · exact (WP.spec_ok _).mpr trivial
  · split
    · exact select_total k.sched cfg hs hNT
    · exact (WP.spec_ok _).mpr trivial

/-- `Kernel::record_event`: an event of Kernel_Arch's on the log. -/
theorem record_event_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (kind t : U8) (ht : t.val < NT.val) (hl : Lwf k.log) :
    logic.kernel.Kernel.record_event k cfg kind t ⦃ _ => True ⦄ := by
  unfold logic.kernel.Kernel.record_event
  simp only [lift, bind_ok]
  step as ⟨ tc, htc ⟩
  step as ⟨ i1, hi1 ⟩
  exact WP.spec_mono (push_event_total _ cfg _ _ _ _ _ hl) (fun _ _ => trivial)

/-- `Kernel::kill_at`: a Job on the stack is marked killed. -/
theorem kill_at_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (i : Usize) (hi : i.val < k.sched.depth.val)
    (hs : SInv k.sched) : logic.kernel.Kernel.kill_at k cfg i ⦃ _ => True ⦄ := by
  unfold logic.kernel.Kernel.kill_at
  have hlen : i.val < k.sched.stack.val.length := by
    have := k.sched.stack.property; have := hs.1; scalar_tac
  step as ⟨ t, ht ⟩
  refine WP.spec_bind (kill_total k.sched cfg i hi hs) (fun s _ => ?_)
  exact (WP.spec_ok _).mpr trivial

/-- `Kernel::settle`, the last step of every Kernel entry: the eligibility
test, and the Budget event of the Job on top armed or quieted. -/
theorem settle_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (hs : SInv k.sched) (hNT : NT.val ≤ 256)
    (hc : CfgOk cfg NS) (hn : k.now.val ≤ 4611686018427387904) :
    logic.kernel.Kernel.settle k cfg ⦃ _ => True ⦄ := by
  have hs' := hs
  obtain ⟨h1, h2, h3, h4⟩ := hs'
  unfold logic.kernel.Kernel.settle
  refine WP.spec_bind (select_total k.sched cfg hs hNT) (fun admitted _ => ?_)
  refine WP.spec_bind (Pₘ := fun _ : Option U8 => True) (by ok_tac) (fun sel _ => ?_)
  split
  · exact (WP.spec_ok _).mpr trivial
  · rename_i hd0
    have hd : 0 < k.sched.depth.val := by scalar_tac
    step as ⟨ i, hi ⟩
    have hlen : i.val < k.sched.stack.val.length := by
      have := k.sched.stack.property; scalar_tac
    step as ⟨ t, ht ⟩
    have ht' : k.sched.stack.val[i.val]! = t := by
      rw [ht]; exact getElem!_pos k.sched.stack.val i.val hlen
    have hst := h2 i.val (by omega)
    simp only [stk] at hst
    rw [ht'] at hst
    simp only [lift, bind_ok]
    have hlen2 : t.val < cfg.slot_budget.val.length := by
      have := cfg.slot_budget.property; scalar_tac
    step as ⟨ sb, hsb ⟩
    have hsb' : cfg.slot_budget.val[t.val]! = sb := by
      simp only [U8.cast_Usize_val_eq] at hsb
      rw [hsb]; exact getElem!_pos cfg.slot_budget.val t.val hlen2
    have hsl := (hc t.val hst).1
    rw [hsb'] at hsl
    have hcast : (UScalar.cast .Usize sb).val < NS.val := by scalar_tac
    cases admitted with
    | none =>
      simp only []
      split
      · split
        · exact (WP.spec_ok _).mpr trivial
        · step as ⟨ sl, hslot ⟩
          split
          · exact (WP.spec_ok _).mpr trivial
          · step as ⟨ jb, hjb ⟩
            split
            · step as ⟨ rp, hrp ⟩
              split
              · exact (WP.spec_ok _).mpr trivial
              · refine WP.spec_bind (arm_budget_total k cfg t hst hc hn) (fun k1 _ => ?_)
                exact (WP.spec_ok _).mpr trivial
            · exact (WP.spec_ok _).mpr trivial
      · exact (WP.spec_ok _).mpr trivial
    | some b =>
      simp only []
      step as ⟨ sl, hslot ⟩
      split
      · refine WP.spec_bind (disarm_spec k.slots _ hcast) (fun s1 _ => ?_)
        exact (WP.spec_ok _).mpr trivial
      · exact (WP.spec_ok _).mpr trivial

end P18
