import Sched.Model
open Aeneas Aeneas.Std Result

namespace P18
open rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! Views of the scheduler state as functions over the naturals. -/

def rel (s : logic.sched.Sched NT NR NL) (k : Nat) : Nat := (s.release.val[k]!).val
def job (s : logic.sched.Sched NT NR NL) (k : Nat) : Nat := (s.job.val[k]!).val
def stk (s : logic.sched.Sched NT NR NL) (p : Nat) : Nat := (s.stack.val[p]!).val

theorem get_set_u8 {n : Usize} (v : Array U8 n) (i : Usize) (x : U8) (k : Nat)
    (hi : i.val < n.val) :
    ((v.set i x).val[k]!) = if k = i.val then x else v.val[k]! := by
  have := v.property
  simp only [Array.set_val_eq]
  split
  · rename_i h; subst h; simp [*]
  · rename_i h; simp [*]

theorem rel_pending : logic.sched.REL_PENDING = 1#u8 := by
  unfold logic.sched.REL_PENDING; rfl
theorem rel_none : logic.sched.REL_NONE = 0#u8 := by
  unfold logic.sched.REL_NONE; rfl
theorem job_none : logic.sched.JOB_NONE = 0#u8 := by
  unfold logic.sched.JOB_NONE; rfl
theorem job_running : logic.sched.JOB_RUNNING = 1#u8 := by
  unfold logic.sched.JOB_RUNNING; rfl
theorem idle_ceiling : logic.IDLE_CEILING = 0#u8 := by
  unfold logic.IDLE_CEILING; rfl

theorem make_pending_views {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : U8} (h : logic.sched.Sched.make_pending s cfg t = ok s') :
    t.val < NT.val ∧ (∀ k, rel s' k = if k = t.val then 1 else rel s k) ∧
    (∀ k, job s' k = job s k) ∧ (∀ p, stk s' p = stk s p) ∧ s'.depth = s.depth ∧
    s'.ceiling = s.ceiling ∧ s'.lock_len = s.lock_len := by
  obtain ⟨ht, rfl⟩ := make_pending_ok h
  refine ⟨ht, ?_, fun _ => rfl, fun _ => rfl, rfl, rfl, rfl⟩
  intro k
  simp only [rel]
  rw [get_set_u8 _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact ht), U8.cast_Usize_val_eq, rel_pending]
  split <;> rfl

theorem start_views {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : U8} (h : logic.sched.Sched.start s cfg t = ok s') :
    t.val < NT.val ∧ s.depth.val < NT.val ∧
    (∀ k, rel s' k = if k = t.val then 0 else rel s k) ∧
    (∀ k, job s' k = if k = t.val then 1 else job s k) ∧
    (∀ p, stk s' p = if p = s.depth.val then t.val else stk s p) ∧
    s'.depth.val = s.depth.val + 1 ∧ s'.ceiling = s.ceiling ∧ s'.lock_len = s.lock_len := by
  obtain ⟨ht, hd, d, hd', rfl⟩ := start_ok h
  refine ⟨ht, hd, ?_, ?_, ?_, hd', rfl, rfl⟩
  · intro k
    simp only [rel]
    rw [get_set_u8 _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact ht), U8.cast_Usize_val_eq, rel_none]
    split <;> rfl
  · intro k
    simp only [job]
    rw [get_set_u8 _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact ht), U8.cast_Usize_val_eq, job_running]
    split <;> rfl
  · intro p
    simp only [stk]
    rw [get_set_u8 _ _ _ _ hd]
    split <;> rfl

theorem finish_top_views {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {r : Usize} (hl : s.lock_len = 0#usize)
    (h : logic.sched.Sched.finish_top s cfg = ok (r, s')) :
    1 ≤ s.depth.val ∧ s.depth.val ≤ NT.val ∧ stk s (s.depth.val - 1) < NT.val ∧
    (∀ k, rel s' k = rel s k) ∧
    (∀ k, job s' k = if k = stk s (s.depth.val - 1) then 0 else job s k) ∧
    (∀ p, stk s' p = stk s p) ∧
    s'.depth.val = s.depth.val - 1 ∧ s'.ceiling.val = 0 ∧ s'.lock_len = s.lock_len := by
  obtain ⟨h1, h2, h3, d, hd, rfl⟩ := finish_top_ok0 hl h
  refine ⟨h1, h2, h3, fun _ => rfl, ?_, fun _ => rfl, hd, ?_, rfl⟩
  · intro k
    simp only [job, stk]
    rw [get_set_u8 _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact h3), U8.cast_Usize_val_eq, job_none]
    by_cases hk : k = (s.stack.val[s.depth.val - 1]!).val
    · simp only [hk, ite_true]; rfl
    · simp only [hk, ite_false]
  · show (logic.IDLE_CEILING).val = 0
    rw [idle_ceiling]; rfl

/-! ## The run -/

/-- A state of a run: the Kernel's scheduler state, the instant, and per
Task the instant of its last Release, the execution its pending Job will
ask for, and the execution its Job in progress still needs. -/
structure St (NT NR NL : Usize) where
  s : logic.sched.Sched NT NR NL
  now : Nat
  last : Nat → Option Nat
  pend : Nat → Nat
  run : Nat → Nat

/-- The timing parameters of a run: per Task its period or MIT, and the
bound on the execution of one Job (its Budget). -/
structure Params where
  T : Nat → Nat
  C : Nat → Nat

abbrev eligibleFn {NT NR NP NL NS NE : Usize} :=
  @_root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.eligible NT NR NP NL NS NE

/-- One transition of a run. The scheduler state changes only through the
Kernel's functions as extracted, and a dispatch only of a Task for which
the Kernel's eligibility test, as extracted, answers true.

- `release`: a Release of Task `t` (`make_pending`), at least `T t` after
  its last one, of a Job that will execute for `d`, `1 ≤ d ≤ C t`.
- `dispatch`: the start of a pending Task the test accepts (`start`).
- `idle`, `exec`, `finish`: one unit of time, taken only when the test
  accepts no Task (a pending eligible Task is dispatched at once, which is
  the Hardware_Model's rule: the dispatch exception is taken before the
  preempted code continues). `idle`: no Job is in progress. `exec`: the Job
  on top of the stack executes for one unit and needs more. `finish`: it
  executes its last unit and ends (`finish_top`). -/
inductive Step (cfg : logic.Config NT NR NP) (P : Params) : St NT NR NL → St NT NR NL → Prop
  | release (st : St NT NR NL) (t : U8) (d : Nat) (s' : logic.sched.Sched NT NR NL)
      (hd1 : 1 ≤ d) (hd2 : d ≤ P.C t.val)
      (hsp : ∀ r, st.last t.val = some r → r + P.T t.val ≤ st.now)
      (hs : logic.sched.Sched.make_pending st.s cfg t = ok s') :
      Step cfg P st { st with
        s := s', last := Function.update st.last t.val (some st.now),
        pend := Function.update st.pend t.val (st.pend t.val + d) }
  | dispatch (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE) (t : U8)
      (s' : logic.sched.Sched NT NR NL)
      (hm : Matches b cfg st.s) (he : eligibleFn b t = ok true)
      (hs : logic.sched.Sched.start st.s cfg t = ok s') :
      Step cfg P st { st with
        s := s', run := Function.update st.run t.val (st.pend t.val),
        pend := Function.update st.pend t.val 0 }
  | idle (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hd : st.s.depth.val = 0) :
      Step cfg P st { st with now := st.now + 1 }
  | exec (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hd : 0 < st.s.depth.val) (hr : 1 < st.run (stk st.s (st.s.depth.val - 1))) :
      Step cfg P st { st with
        now := st.now + 1,
        run := Function.update st.run (stk st.s (st.s.depth.val - 1))
          (st.run (stk st.s (st.s.depth.val - 1)) - 1) }
  | finish (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (r : Usize) (s' : logic.sched.Sched NT NR NL)
      (hd : 0 < st.s.depth.val) (hr : st.run (stk st.s (st.s.depth.val - 1)) ≤ 1)
      (hs : logic.sched.Sched.finish_top st.s cfg = ok (r, s')) :
      Step cfg P st { st with
        s := s', now := st.now + 1,
        run := Function.update st.run (stk st.s (st.s.depth.val - 1)) 0 }

/-- The states a run reaches from boot (`Sched::new`, instant 0, no Job). -/
inductive Reach (cfg : logic.Config NT NR NP) (P : Params) : St NT NR NL → Prop
  | boot (s0 : logic.sched.Sched NT NR NL) (h : logic.sched.Sched.new NL cfg = ok s0) :
      Reach cfg P { s := s0, now := 0, last := fun _ => none, pend := fun _ => 0, run := fun _ => 0 }
  | step (st st' : St NT NR NL) (h : Reach cfg P st) (hs : Step cfg P st st') : Reach cfg P st'

/-! ## The structure of the scheduler state in a run without locks -/

structure Wf (cfg : logic.Config NT NR NP) (st : St NT NR NL) : Prop where
  lock0 : st.s.lock_len = 0#usize
  ceil0 : st.s.ceiling.val = 0
  depth_le : st.s.depth.val ≤ NT.val
  stk_lt : ∀ p, p < st.s.depth.val → stk st.s p < NT.val
  sorted : ∀ p q, p < q → q < st.s.depth.val → prio cfg (stk st.s p) < prio cfg (stk st.s q)
  job01 : ∀ k, k < NT.val → job st.s k = 0 ∨ job st.s k = 1
  on_stack : ∀ k, k < NT.val → (job st.s k = 1 ↔ ∃ p, p < st.s.depth.val ∧ stk st.s p = k)
  run_iff : ∀ k, k < NT.val → (0 < st.run k ↔ job st.s k = 1)
  pend_iff : ∀ k, k < NT.val → (0 < st.pend k ↔ rel st.s k = 1)

theorem elig_views {cfg : logic.Config NT NR NP} {s : logic.sched.Sched NT NR NL} {t : Nat} :
    Elig cfg s t ↔ (t < NT.val ∧ rel s t = 1 ∧ job s t = 0 ∧ s.ceiling.val < prio cfg t ∧
      (s.depth.val = 0 ∨ prio cfg (stk s (s.depth.val - 1)) < prio cfg t)) := Iff.rfl

theorem Wf.step {cfg : logic.Config NT NR NP} {P : Params} {st st' : St NT NR NL}
    (w : Wf cfg st) (h : Step cfg P st st') : Wf cfg st' := by
  cases h with
  | release t d s' hd1 hd2 hsp hs =>
    obtain ⟨ht, hrel, hjob, hstk, hdep, hceil, hlock⟩ := make_pending_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> dsimp only
    · rw [hlock]; exact w.lock0
    · rw [hceil]; exact w.ceil0
    · rw [hdep]; exact w.depth_le
    · intro p hp; rw [hstk]; rw [hdep] at hp; exact w.stk_lt p hp
    · intro p q hpq hq; rw [hstk, hstk]; rw [hdep] at hq; exact w.sorted p q hpq hq
    · intro k hk; rw [hjob]; exact w.job01 k hk
    · intro k hk; rw [hjob, hdep]; simp only [hstk]; exact w.on_stack k hk
    · intro k hk; rw [hjob]; exact w.run_iff k hk
    · intro k hk; rw [hrel]
      by_cases h : k = t.val
      · subst h; simp [Function.update]; omega
      · rw [Function.update_of_ne h]; simp only [h, ite_false]; exact w.pend_iff k hk
  | dispatch b t s' hm he hs =>
    obtain ⟨_, hr1, hj0, hc, htop⟩ := elig_views.mp ((eligible_ok hm he).mp rfl)
    obtain ⟨ht, hdl, hrel, hjob, hstk, hdep, hceil, hlock⟩ := start_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> dsimp only
    · rw [hlock]; exact w.lock0
    · rw [hceil]; exact w.ceil0
    · rw [hdep]; omega
    · intro p hp; rw [hstk]; rw [hdep] at hp
      by_cases h : p = st.s.depth.val
      · simp only [h, ite_true]; exact ht
      · simp only [h, ite_false]; exact w.stk_lt p (by omega)
    · intro p q hpq hq; rw [hstk, hstk]; rw [hdep] at hq
      have hp : p ≠ st.s.depth.val := by omega
      simp only [hp, ite_false]
      by_cases h : q = st.s.depth.val
      · simp only [h, ite_true]
        rcases htop with h0 | h1
        · omega
        · by_cases hp1 : p = st.s.depth.val - 1
          · rw [hp1]; exact h1
          · exact Nat.lt_trans (w.sorted p (st.s.depth.val - 1) (by omega) (by omega)) h1
      · simp only [h, ite_false]; exact w.sorted p q hpq (by omega)
    · intro k hk; rw [hjob]
      by_cases h : k = t.val
      · simp only [h, ite_true]; right; trivial
      · simp only [h, ite_false]; exact w.job01 k hk
    · intro k hk; rw [hjob, hdep]
      by_cases h : k = t.val
      · simp only [h, ite_true, true_iff]
        exact ⟨st.s.depth.val, by omega, by rw [hstk]; simp⟩
      · simp only [h, ite_false]
        rw [w.on_stack k hk]
        constructor
        · rintro ⟨p, hp, hpk⟩
          refine ⟨p, by omega, ?_⟩
          rw [hstk]; simp only [show p ≠ st.s.depth.val by omega, ite_false]; exact hpk
        · rintro ⟨p, hp, hpk⟩
          rw [hstk] at hpk
          by_cases hpd : p = st.s.depth.val
          · simp only [hpd, ite_true] at hpk; exact absurd hpk.symm h
          · simp only [hpd, ite_false] at hpk; exact ⟨p, by omega, hpk⟩
    · intro k hk; rw [hjob]
      by_cases h : k = t.val
      · subst h
        have := (w.pend_iff t.val ht).mpr hr1
        simp [Function.update]; exact this
      · rw [Function.update_of_ne h]; simp only [h, ite_false]; exact w.run_iff k hk
    · intro k hk; rw [hrel]
      by_cases h : k = t.val
      · subst h; simp [Function.update]
      · rw [Function.update_of_ne h]; simp only [h, ite_false]; exact w.pend_iff k hk
  | idle b hm hne hd =>
    exact ⟨w.lock0, w.ceil0, w.depth_le, w.stk_lt, w.sorted, w.job01, w.on_stack, w.run_iff, w.pend_iff⟩
  | exec b hm hne hd hr =>
    refine ⟨w.lock0, w.ceil0, w.depth_le, w.stk_lt, w.sorted, w.job01, w.on_stack, ?_, w.pend_iff⟩
    dsimp only
    intro k hk
    by_cases h : k = stk st.s (st.s.depth.val - 1)
    · subst h
      have := (w.run_iff _ hk).mp (by omega)
      simp [Function.update]; exact ⟨fun _ => this, fun _ => hr⟩
    · rw [Function.update_of_ne h]; exact w.run_iff k hk
  | finish b hm hne r s' hd hr hs =>
    obtain ⟨h1, h2, h3, hrel, hjob, hstk, hdep, hceil, hlock⟩ := finish_top_views w.lock0 hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> dsimp only
    · rw [hlock]; exact w.lock0
    · exact hceil
    · rw [hdep]; omega
    · intro p hp; rw [hstk]; rw [hdep] at hp; exact w.stk_lt p (by omega)
    · intro p q hpq hq; rw [hstk, hstk]; rw [hdep] at hq; exact w.sorted p q hpq (by omega)
    · intro k hk; rw [hjob]
      by_cases h : k = stk st.s (st.s.depth.val - 1)
      · simp only [h, ite_true]; left; trivial
      · simp only [h, ite_false]; exact w.job01 k hk
    · intro k hk; rw [hjob, hdep]; simp only [hstk]
      by_cases h : k = stk st.s (st.s.depth.val - 1)
      · simp only [h, ite_true]
        constructor
        · intro hx; omega
        · rintro ⟨p, hp, hpk⟩
          have := w.sorted p (st.s.depth.val - 1) (by omega) (by omega)
          rw [hpk] at this; omega
      · simp only [h, ite_false]
        rw [w.on_stack k hk]
        constructor
        · rintro ⟨p, hp, hpk⟩
          refine ⟨p, ?_, hpk⟩
          by_contra hc
          have : p = st.s.depth.val - 1 := by omega
          rw [this] at hpk; exact h hpk.symm
        · rintro ⟨p, hp, hpk⟩; exact ⟨p, by omega, hpk⟩
    · intro k hk; rw [hjob]
      by_cases h : k = stk st.s (st.s.depth.val - 1)
      · subst h; simp [Function.update]
      · rw [Function.update_of_ne h]; simp only [h, ite_false]; exact w.run_iff k hk
    · intro k hk; rw [hrel]; exact w.pend_iff k hk

end P18
