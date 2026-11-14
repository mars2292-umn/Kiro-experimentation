import Sched.Views
open Aeneas Aeneas.Std Result

namespace P18
open rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! ## The run -/

/-- A state of a run: the Kernel's scheduler state, the instant, and per
Task the nominal instant of its last arrival (`last`), the execution its
pending Job will ask for, and the execution its Job in progress still
needs; per entry of the lock stack, the execution its Critical_Section may
still take; per Task the Kernel's work still to do for its arrivals
(`kw`), and whether an arrival awaits its Release (`wait`). -/
structure St (NT NR NL : Usize) where
  s : logic.sched.Sched NT NR NL
  now : Nat
  last : Nat → Option Nat
  pend : Nat → Nat
  run : Nat → Nat
  allow : Nat → Nat
  kw : Nat → Nat
  wait : Nat → Bool

/-- The timing parameters of a run: per Task its period or MIT, the bound
on the execution of one Job (its Budget), its release jitter, and the
Kernel's work for one arrival; per Task and Resource the bound on the
execution of one Critical_Section. -/
structure Params where
  T : Nat → Nat
  C : Nat → Nat
  L : Nat → Nat → Nat
  J : Nat → Nat
  I : Nat → Nat

/-- No arrival that awaits its Release is past its jitter one unit on. -/
def JitterOk (P : Params) (st : St NT NR NL) : Prop :=
  ∀ k a, st.wait k = true → st.last k = some a → st.now + 1 ≤ a + P.J k

abbrev eligibleFn {NT NR NP NL NS NE : Usize} :=
  @_root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.eligible NT NR NP NL NS NE

/-- The lock entries of the Job on top of the stack. -/
def topEntry (s : logic.sched.Sched NT NR NL) (p : Nat) : Prop :=
  p < s.lock_len.val ∧ ldep s p = s.depth.val - 1

instance (s : logic.sched.Sched NT NR NL) (p : Nat) : Decidable (topEntry s p) := by
  unfold topEntry; infer_instance

/-- One transition of a run. The scheduler state changes only through the
Kernel's functions as extracted, and a dispatch only of a Task for which
the Kernel's eligibility test, as extracted, answers true.

- `arrive`: the nominal instant of a Release of Task `t`, at least `T t`
  after its last one. The Kernel's work for it, at most `I t`, is done
  before any Job executes.
- `release`: the Release of the Task whose arrival awaits it
  (`make_pending`), of a Job that will execute for `d`, `1 ≤ d ≤ C t`. It
  comes at most `J t` after the arrival: time does not pass beyond that
  with the arrival still waiting.
- `ktick`: one unit of time spent on the Kernel's work.
- `dispatch`: the start of a pending Task the test accepts (`start`).
- `lock`, `unlock`: the Job on top of the stack takes or returns a
  Resource (`lock`, `unlock`, when they grant it). The Critical_Section
  may execute for `a`, at most `L` of the Task and the Resource.
- `idle`, `exec`, `finish`: one unit of time. `idle`: no Job is in
  progress. `exec`: the Job on top of the stack executes for one unit and
  needs more. `finish`: it executes its last unit and ends (`finish_top`).
  A unit of execution is taken from every Critical_Section the Job is in,
  and none of them may be exhausted.

A Job executes, locks and unlocks only when the Kernel has no work and
the test accepts no Task: a pending eligible Task is dispatched at once,
which is the Hardware_Model's rule (the dispatch exception is taken before
the preempted code continues). -/
inductive Step (cfg : logic.Config NT NR NP) (P : Params) : St NT NR NL → St NT NR NL → Prop
  | arrive (st : St NT NR NL) (t : Nat) (c : Nat) (ht : t < NT.val) (hc : c ≤ P.I t)
      (hsp : ∀ r, st.last t = some r → r + P.T t ≤ st.now)
      (hw : st.wait t = false) :
      Step cfg P st { st with
        last := Function.update st.last t (some st.now),
        wait := Function.update st.wait t true,
        kw := Function.update st.kw t (st.kw t + c) }
  | release (st : St NT NR NL) (t : U8) (d : Nat) (s' : logic.sched.Sched NT NR NL)
      (hd1 : 1 ≤ d) (hd2 : d ≤ P.C t.val)
      (hw : st.wait t.val = true)
      (hs : logic.sched.Sched.make_pending st.s cfg t = ok s') :
      Step cfg P st { st with
        s := s', wait := Function.update st.wait t.val false,
        pend := Function.update st.pend t.val (st.pend t.val + d) }
  | ktick (st : St NT NR NL) (k : Nat) (hkn : k < NT.val) (hk : 0 < st.kw k) (hj : JitterOk P st) :
      Step cfg P st { st with
        now := st.now + 1, kw := Function.update st.kw k (st.kw k - 1) }
  | dispatch (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE) (t : U8)
      (s' : logic.sched.Sched NT NR NL)
      (hm : Matches b cfg st.s) (he : eligibleFn b t = ok true)
      (hs : logic.sched.Sched.start st.s cfg t = ok s') :
      Step cfg P st { st with
        s := s', run := Function.update st.run t.val (st.pend t.val),
        pend := Function.update st.pend t.val 0 }
  | lock (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hk0 : ∀ k, st.kw k = 0)
      (r : U8) (a : Nat) (s' : logic.sched.Sched NT NR NL)
      (ha : a ≤ P.L (stk st.s (st.s.depth.val - 1)) r.val)
      (hs : logic.sched.Sched.lock st.s cfg r = ok (true, s')) :
      Step cfg P st { st with
        s := s', allow := Function.update st.allow st.s.lock_len.val a }
  | unlock (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hk0 : ∀ k, st.kw k = 0)
      (r : U8) (s' : logic.sched.Sched NT NR NL)
      (hs : logic.sched.Sched.unlock st.s cfg r = ok (true, s')) :
      Step cfg P st { st with s := s' }
  | idle (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hk0 : ∀ k, st.kw k = 0) (hj : JitterOk P st)
      (hd : st.s.depth.val = 0) :
      Step cfg P st { st with now := st.now + 1 }
  | exec (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hk0 : ∀ k, st.kw k = 0) (hj : JitterOk P st)
      (hd : 0 < st.s.depth.val) (hr : 1 < st.run (stk st.s (st.s.depth.val - 1)))
      (hal : ∀ p, topEntry st.s p → 0 < st.allow p) :
      Step cfg P st { st with
        now := st.now + 1,
        run := Function.update st.run (stk st.s (st.s.depth.val - 1))
          (st.run (stk st.s (st.s.depth.val - 1)) - 1),
        allow := fun p => if topEntry st.s p then st.allow p - 1 else st.allow p }
  | finish (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hk0 : ∀ k, st.kw k = 0) (hj : JitterOk P st)
      (r : Usize) (s' : logic.sched.Sched NT NR NL)
      (hd : 0 < st.s.depth.val) (hr : st.run (stk st.s (st.s.depth.val - 1)) ≤ 1)
      (hal : ∀ p, topEntry st.s p → 0 < st.allow p)
      (hs : logic.sched.Sched.finish_top st.s cfg = ok (r, s')) :
      Step cfg P st { st with
        s := s', now := st.now + 1,
        run := Function.update st.run (stk st.s (st.s.depth.val - 1)) 0 }

/-- The states a run reaches from boot (`Sched::new`, instant 0, no Job). -/
inductive Reach (cfg : logic.Config NT NR NP) (P : Params) : St NT NR NL → Prop
  | boot (s0 : logic.sched.Sched NT NR NL) (h : logic.sched.Sched.new NL cfg = ok s0) :
      Reach cfg P ⟨s0, 0, fun _ => none, fun _ => 0, fun _ => 0, fun _ => 0, fun _ => 0, fun _ => false⟩
  | step (st st' : St NT NR NL) (h : Reach cfg P st) (hs : Step cfg P st st') : Reach cfg P st'

/-! ## The structure of the scheduler state in a run -/

structure Wf (cfg : logic.Config NT NR NP) (P : Params) (st : St NT NR NL) : Prop where
  depth_le : st.s.depth.val ≤ NT.val
  stk_lt : ∀ p, p < st.s.depth.val → stk st.s p < NT.val
  sorted : ∀ p q, p < q → q < st.s.depth.val → prio cfg (stk st.s p) < prio cfg (stk st.s q)
  job01 : ∀ k, k < NT.val → job st.s k = 0 ∨ job st.s k = 1
  on_stack : ∀ k, k < NT.val → (job st.s k = 1 ↔ ∃ p, p < st.s.depth.val ∧ stk st.s p = k)
  run_iff : ∀ k, k < NT.val → (0 < st.run k ↔ job st.s k = 1)
  pend_iff : ∀ k, k < NT.val → (0 < st.pend k ↔ rel st.s k = 1)
  /-- The System_Ceiling is that of the lock stack. -/
  ceil_is : IsCeil cfg st.s st.s.lock_len.val st.s.ceiling.val
  /-- An entry is held by a Job on the stack; -/
  ldep_lt : ∀ p, p < st.s.lock_len.val → ldep st.s p < st.s.depth.val
  /-- the entries of a Job higher on the stack are higher on the lock stack; -/
  ldep_mono : ∀ p q, p ≤ q → q < st.s.lock_len.val → ldep st.s p ≤ ldep st.s q
  /-- and a Job above an entry's holder has a Priority above the entry's
  Ceiling (the Stack Resource Policy). -/
  srp : ∀ p q, p < st.s.lock_len.val → q < st.s.depth.val → ldep st.s p < q →
    ceil cfg (lres st.s p) < prio cfg (stk st.s q)
  /-- A Critical_Section's remaining execution is within its bound. -/
  allow_le : ∀ p, p < st.s.lock_len.val →
    st.allow p ≤ P.L (stk st.s (ldep st.s p)) (lres st.s p)
  /-- An arrival that awaits its Release is within its jitter. -/
  wait_inv : ∀ k, st.wait k = true → ∃ a, st.last k = some a ∧ st.now ≤ a + P.J k

theorem elig_views {cfg : logic.Config NT NR NP} {s : logic.sched.Sched NT NR NL} {t : Nat} :
    Elig cfg s t ↔ (t < NT.val ∧ rel s t = 1 ∧ job s t = 0 ∧ s.ceiling.val < prio cfg t ∧
      (s.depth.val = 0 ∨ prio cfg (stk s (s.depth.val - 1)) < prio cfg t)) := Iff.rfl

theorem IsCeil.same {cfg : logic.Config NT NR NP} {s s' : logic.sched.Sched NT NR NL} {n c : Nat}
    (hl : ∀ p, lres s' p = lres s p) (h : IsCeil cfg s n c) : IsCeil cfg s' n c := by
  obtain ⟨h1, h2⟩ := h
  refine ⟨fun p hp => by rw [hl]; exact h1 p hp, ?_⟩
  rcases h2 with h | ⟨p, hp, hc⟩
  · left; exact h
  · right; exact ⟨p, hp, by rw [hl]; exact hc⟩

theorem Wf.step {cfg : logic.Config NT NR NP} {P : Params} {st st' : St NT NR NL}
    (w : Wf cfg P st) (h : Step cfg P st st') : Wf cfg P st' := by
  cases h with
  | arrive t c ht hc hsp hw =>
    refine ⟨w.depth_le, w.stk_lt, w.sorted, w.job01, w.on_stack, w.run_iff, w.pend_iff, w.ceil_is,
      w.ldep_lt, w.ldep_mono, w.srp, w.allow_le, ?_⟩
    dsimp only
    intro k hk
    by_cases h : k = t
    · subst h; exact ⟨st.now, by simp, by omega⟩
    · rw [Function.update_of_ne h] at hk ⊢; exact w.wait_inv k hk
  | ktick k hkn hk hj =>
    refine ⟨w.depth_le, w.stk_lt, w.sorted, w.job01, w.on_stack, w.run_iff, w.pend_iff, w.ceil_is,
      w.ldep_lt, w.ldep_mono, w.srp, w.allow_le, ?_⟩
    dsimp only
    intro k' hk'
    obtain ⟨a, ha, _⟩ := w.wait_inv k' hk'
    exact ⟨a, ha, hj k' a hk' ha⟩
  | release t d s' hd1 hd2 hw hs =>
    obtain ⟨ht, hrel, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := make_pending_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> dsimp only
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
    · rw [hll, hlc]; exact w.ceil_is.same hlr
    · intro p hp; rw [hld, hdep]; rw [hll] at hp; exact w.ldep_lt p hp
    · intro p q hpq hq; rw [hld, hld]; rw [hll] at hq; exact w.ldep_mono p q hpq hq
    · intro p q hp hq hpq; rw [hll] at hp; rw [hdep] at hq; rw [hld] at hpq
      rw [hlr, hstk]; exact w.srp p q hp hq hpq
    · intro p hp; rw [hll] at hp; rw [hld, hlr, hstk]; exact w.allow_le p hp
    · intro k hk
      by_cases h : k = t.val
      · subst h; simp at hk
      · rw [Function.update_of_ne h] at hk; exact w.wait_inv k hk
  | dispatch b t s' hm he hs =>
    obtain ⟨_, hr1, hj0, hc, htop⟩ := elig_views.mp ((eligible_ok hm he).mp rfl)
    obtain ⟨ht, hdl, hrel, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := start_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> dsimp only
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
    · rw [hll, hlc]; exact w.ceil_is.same hlr
    · intro p hp; rw [hld, hdep]; rw [hll] at hp; have := w.ldep_lt p hp; omega
    · intro p q hpq hq; rw [hld, hld]; rw [hll] at hq; exact w.ldep_mono p q hpq hq
    · intro p q hp hq hpq; rw [hll] at hp; rw [hdep] at hq; rw [hld] at hpq
      rw [hlr, hstk]
      by_cases h : q = st.s.depth.val
      · simp only [h, ite_true]
        have := w.ceil_is.1 p hp
        omega
      · simp only [h, ite_false]; exact w.srp p q hp (by omega) hpq
    · intro p hp; rw [hll] at hp; rw [hld, hlr, hstk]
      have := w.ldep_lt p hp
      simp only [show ldep st.s p ≠ st.s.depth.val by omega, ite_false]
      exact w.allow_le p hp
    · exact w.wait_inv
  | lock b hm hne hk0 r a s' ha hs =>
    obtain ⟨⟨hrel, hjob, hstk, hdep⟩, hd1, hl, hll, hlr, hld, hlc⟩ := lock_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> dsimp only
    · rw [hdep]; exact w.depth_le
    · intro p hp; rw [hstk]; rw [hdep] at hp; exact w.stk_lt p hp
    · intro p q hpq hq; rw [hstk, hstk]; rw [hdep] at hq; exact w.sorted p q hpq hq
    · intro k hk; rw [hjob]; exact w.job01 k hk
    · intro k hk; rw [hjob, hdep]; simp only [hstk]; exact w.on_stack k hk
    · intro k hk; rw [hjob]; exact w.run_iff k hk
    · intro k hk; rw [hrel]; exact w.pend_iff k hk
    · rw [hll, hlc]
      obtain ⟨c1, c2⟩ := w.ceil_is
      constructor
      · intro p hp
        rw [hlr]
        by_cases h : p = st.s.lock_len.val
        · simp only [h, ite_true]; exact Nat.le_max_right _ _
        · simp only [h, ite_false]
          exact Nat.le_trans (c1 p (by omega)) (Nat.le_max_left _ _)
      · by_cases hge : st.s.ceiling.val ≤ ceil cfg r.val
        · right
          refine ⟨st.s.lock_len.val, by omega, ?_⟩
          rw [hlr]; simp only [ite_true]; exact Nat.max_eq_right hge
        · rw [Nat.max_eq_left (by omega)]
          rcases c2 with h | ⟨p, hp, hc⟩
          · left; exact h
          · right
            refine ⟨p, by omega, ?_⟩
            rw [hlr]; simp only [show p ≠ st.s.lock_len.val by omega, ite_false]; exact hc
    · intro p hp; rw [hld, hdep]; rw [hll] at hp
      by_cases h : p = st.s.lock_len.val
      · simp only [h, ite_true]; omega
      · simp only [h, ite_false]; exact w.ldep_lt p (by omega)
    · intro p q hpq hq; rw [hld, hld]; rw [hll] at hq
      by_cases h : q = st.s.lock_len.val
      · simp only [h, ite_true]
        by_cases h' : p = st.s.lock_len.val
        · simp only [h', ite_true]; exact Nat.le_refl _
        · simp only [h', ite_false]
          have := w.ldep_lt p (by omega)
          omega
      · simp only [h, ite_false, show p ≠ st.s.lock_len.val by omega]
        exact w.ldep_mono p q hpq (by omega)
    · intro p q hp hq hpq; rw [hll] at hp; rw [hdep] at hq; rw [hld] at hpq
      rw [hlr, hstk]
      by_cases h : p = st.s.lock_len.val
      · simp only [h, ite_true] at hpq; omega
      · simp only [h, ite_false] at hpq ⊢; exact w.srp p q (by omega) hq hpq
    · intro p hp; rw [hll] at hp; rw [hld, hlr]; simp only [hstk]
      by_cases h : p = st.s.lock_len.val
      · simp only [h, ite_true, Function.update_self]; exact ha
      · simp only [h, ite_false]; rw [Function.update_of_ne h]; exact w.allow_le p (by omega)
    · exact w.wait_inv
  | unlock b hm hne hk0 r s' hs =>
    obtain ⟨⟨hrel, hjob, hstk, hdep⟩, hl1, hd1, _, _, hll, hlr, hld, hc⟩ := unlock_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> dsimp only
    · rw [hdep]; exact w.depth_le
    · intro p hp; rw [hstk]; rw [hdep] at hp; exact w.stk_lt p hp
    · intro p q hpq hq; rw [hstk, hstk]; rw [hdep] at hq; exact w.sorted p q hpq hq
    · intro k hk; rw [hjob]; exact w.job01 k hk
    · intro k hk; rw [hjob, hdep]; simp only [hstk]; exact w.on_stack k hk
    · intro k hk; rw [hjob]; exact w.run_iff k hk
    · intro k hk; rw [hrel]; exact w.pend_iff k hk
    · rw [hll]; exact hc.same hlr
    · intro p hp; rw [hld, hdep]; rw [hll] at hp; exact w.ldep_lt p (by omega)
    · intro p q hpq hq; rw [hld, hld]; rw [hll] at hq; exact w.ldep_mono p q hpq (by omega)
    · intro p q hp hq hpq; rw [hll] at hp; rw [hdep] at hq; rw [hld] at hpq
      rw [hlr, hstk]; exact w.srp p q (by omega) hq hpq
    · intro p hp; rw [hll] at hp; rw [hld, hlr, hstk]; exact w.allow_le p (by omega)
    · exact w.wait_inv
  | idle b hm hne hk0 hj hd =>
    refine ⟨w.depth_le, w.stk_lt, w.sorted, w.job01, w.on_stack, w.run_iff, w.pend_iff, w.ceil_is,
      w.ldep_lt, w.ldep_mono, w.srp, w.allow_le, ?_⟩
    dsimp only
    intro k' hk'
    obtain ⟨a, ha, _⟩ := w.wait_inv k' hk'
    exact ⟨a, ha, hj k' a hk' ha⟩
  | exec b hm hne hk0 hj hd hr hal =>
    refine ⟨w.depth_le, w.stk_lt, w.sorted, w.job01, w.on_stack, ?_, w.pend_iff, w.ceil_is,
      w.ldep_lt, w.ldep_mono, w.srp, ?_, ?_⟩ <;> dsimp only
    · intro k hk
      by_cases h : k = stk st.s (st.s.depth.val - 1)
      · subst h
        have := (w.run_iff _ hk).mp (by omega)
        simp [Function.update]; exact ⟨fun _ => this, fun _ => hr⟩
      · rw [Function.update_of_ne h]; exact w.run_iff k hk
    · intro p hp
      have := w.allow_le p hp
      split <;> omega
    · intro k' hk'
      obtain ⟨a, ha, _⟩ := w.wait_inv k' hk'
      exact ⟨a, ha, hj k' a hk' ha⟩
  | finish b hm hne hk0 hj r s' hd hr hal hs =>
    obtain ⟨h1, h2, h3, hrel, hjob, hstk, hdep, hlr, hld, hn, hpop, hkeep, hc⟩ :=
      finish_top_views hs
    have hbelow : ∀ p, p < s'.lock_len.val → ldep st.s p < st.s.depth.val - 1 := by
      intro p hp
      rcases hkeep with h0 | hne'
      · omega
      · have m := w.ldep_mono p (s'.lock_len.val - 1) (by omega) (by omega)
        have l := w.ldep_lt (s'.lock_len.val - 1) (by omega)
        omega
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> dsimp only
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
    · exact hc.same hlr
    · intro p hp; rw [hld, hdep]; exact hbelow p hp
    · intro p q hpq hq; rw [hld, hld]; exact w.ldep_mono p q hpq (by omega)
    · intro p q hp hq hpq; rw [hdep] at hq; rw [hld] at hpq
      rw [hlr, hstk]; exact w.srp p q (by omega) (by omega) hpq
    · intro p hp; rw [hld, hlr, hstk]; exact w.allow_le p (by omega)
    · intro k' hk'
      obtain ⟨a, ha, _⟩ := w.wait_inv k' hk'
      exact ⟨a, ha, hj k' a hk' ha⟩

end P18
