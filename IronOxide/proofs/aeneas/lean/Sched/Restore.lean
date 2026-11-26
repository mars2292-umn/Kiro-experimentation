import Sched.Mutex
open Aeneas Aeneas.Std Result

/-! # The scheduler over a run: ceiling restore and deadlock freedom (2026-11-25)

Two statements Verus proves of `rsk_logic` over its ghost history
(`history.rs`, `lemma_ceiling_restored`; `sched.rs`, `lemma_deadlock_free`;
Verification_Plan arguments 5.3 and 5.5), here over the run of the
scheduler's functions as Aeneas extracts them. The ghost history is erased
from the compiled crate, so the run is the one of `Sched/Run.lean`, whose
transitions are those functions (`Sched::start`, `lock`, `unlock`,
`finish_top`, `kill`, `make_pending`, `defer`, `clear_release`): the
history's steps, one for one.

- `step_prefix`: no transition changes an entry of the lock stack that is
  there before it and after it.
- `Above`, `above_prefix`: along a stretch of a run in which the lock stack
  never has fewer than `n` entries, its first `n` entries stay as they were
  (`lemma_prefix_kept`).
- `ceiling_restored`: a lock is granted in a reached state; the run goes on,
  the lock stack never shorter than the lock left it, whatever other Jobs
  lock, unlock, start or end in between; in a state where it is again as
  long as the lock left it, an unlock is granted. That unlock is of the
  Resource locked, and it leaves the System_Ceiling at its value before the
  lock (R8.2, Property 3 over a run).
- `BlockedBy`, `deadlock_free`: in every reached state, (2) a pending Task
  that is not eligible and has no Job in progress is kept from starting by
  a Job on the stack, the executing one or the holder of a Resource whose
  Ceiling is at least its Priority; and (3) that Job is in progress and is
  itself kept from starting by nothing. So every wait is for a Job in
  progress, a wait-for path has length one, and there is no cycle (R8.5).
  Part (1) of the Verus lemma, that a lock never waits for another Job, is
  `lock_entry_finds_free` of `Sched/Mutex.lean`.
- `kreach_deadlock_free`: the same of every state the run over the Kernel's
  functions reaches (`Sched/KRun.lean`).

What is not here: `ceiling_restored` for a stretch of the run over the
Kernel's functions. That run's states are states of this one (`ksim`), but
one of its transitions can be several of this one's, and the correspondence
is proven for the states reached, not for stretches. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! ## The entries of the lock stack along a run -/

/-- The entries of the lock stack that are there in both states are the
same. -/
def PrefixKept (s s' : logic.sched.Sched NT NR NL) : Prop :=
  ∀ p, p < s.lock_len.val → p < s'.lock_len.val → lres s' p = lres s p

theorem prefix_refl (s : logic.sched.Sched NT NR NL) : PrefixKept s s := fun _ _ _ => rfl

theorem prefix_same {s s' : logic.sched.Sched NT NR NL} (h : LockSame s' s) : PrefixKept s s' :=
  fun p _ _ => h.1 p

theorem prefix_lock {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.sched.Sched.lock s cfg r = ok (true, s')) : PrefixKept s s' := by
  obtain ⟨_, _, _, _, hl, _, _⟩ := lock_views h
  intro p hp _
  rw [hl p]
  have : ¬ p = s.lock_len.val := by omega
  simp only [this, ite_false]

theorem prefix_unlock {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.sched.Sched.unlock s cfg r = ok (true, s')) : PrefixKept s s' := by
  obtain ⟨_, _, _, _, _, _, hl, _, _⟩ := unlock_views h
  exact fun p _ _ => hl p

theorem prefix_finish_top {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {r : Usize} (h : logic.sched.Sched.finish_top s cfg = ok (r, s')) : PrefixKept s s' := by
  obtain ⟨_, _, _, _, _, _, _, hl, _, _, _, _, _⟩ := finish_top_views h
  exact fun p _ _ => hl p

/-- No transition of the run changes an entry of the lock stack that is
there before it and after it. -/
theorem step_prefix {cfg : logic.Config NT NR NP} {P : Params} {st st' : St NT NR NL}
    (h : Step cfg P st st') : PrefixKept st.s st'.s := by
  cases h <;> dsimp only <;> first
    | exact prefix_refl _
    | exact prefix_same (make_pending_views (by assumption)).2.2.2.2.2
    | exact prefix_same (defer_views (by assumption)).2.2.2.2.2
    | exact prefix_same (start_views (by assumption)).2.2.2.2.2.2
    | exact prefix_same (kill_views (by assumption)).2.2.2.2.2
    | exact prefix_same (clear_release_views (by assumption)).2.2.2.2.2
    | exact prefix_lock (by assumption)
    | exact prefix_unlock (by assumption)
    | exact prefix_finish_top (by assumption)

/-- A stretch of a run from `a` to `b` in which, after `a`, the lock stack
never has fewer than `n` entries. -/
inductive Above (cfg : logic.Config NT NR NP) (P : Params) (n : Nat) :
    St NT NR NL → St NT NR NL → Prop
  | refl (a : St NT NR NL) : Above cfg P n a a
  | tail (a b c : St NT NR NL) (h : Above cfg P n a b) (hs : Step cfg P b c)
      (hn : n ≤ c.s.lock_len.val) : Above cfg P n a c

/-- Along such a stretch the first `n` entries of the lock stack stay as
they were (`lemma_prefix_kept` of the Verus history). -/
theorem above_prefix {cfg : logic.Config NT NR NP} {P : Params} {n : Nat} {a b : St NT NR NL}
    (h : Above cfg P n a b) (ha : n ≤ a.s.lock_len.val) :
    n ≤ b.s.lock_len.val ∧ ∀ p, p < n → lres b.s p = lres a.s p := by
  induction h with
  | refl => exact ⟨ha, fun _ _ => rfl⟩
  | tail b c _ hs hn ih =>
    obtain ⟨hb, hp⟩ := ih
    refine ⟨hn, fun p hpn => ?_⟩
    rw [step_prefix hs p (by omega) (by omega)]
    exact hp p hpn

/-- A state reached from a reached state is reached. -/
theorem above_reach {cfg : logic.Config NT NR NP} {P : Params} {n : Nat} {a b : St NT NR NL}
    (h : Above cfg P n a b) (ha : Reach cfg P a) : Reach cfg P b := by
  induction h with
  | refl => exact ha
  | tail b c _ hs _ ih => exact Reach.step _ _ ih hs

/-! ## Property 3 over a run: the ceiling is restored across preemptions -/

/-- **The System_Ceiling is restored by the matching unlock, whatever
happened in between (R8.2; `lemma_ceiling_restored`).** In a reached state
`a` a lock of Resource `r` is granted, leaving the scheduler state of `b`.
From `b` the run goes on to `c`, the lock stack never shorter than in `b`:
other Jobs may start, lock, unlock and end. In `c` the lock stack is as
long as in `b`, and an unlock is granted there. Then the unlock is of `r`,
and it leaves the System_Ceiling of `a`. -/
theorem ceiling_restored {cfg : logic.Config NT NR NP} {P : Params} {a b c : St NT NR NL}
    {r r' : U8} {s3 : logic.sched.Sched NT NR NL}
    (ha : Reach cfg P a)
    (hlock : logic.sched.Sched.lock a.s cfg r = ok (true, b.s))
    (hrun : Above cfg P b.s.lock_len.val b c)
    (hlen : c.s.lock_len.val = b.s.lock_len.val)
    (hunlock : logic.sched.Sched.unlock c.s cfg r' = ok (true, s3)) :
    s3.ceiling.val = a.s.ceiling.val ∧ r'.val = r.val := by
  obtain ⟨_, hp⟩ := above_prefix hrun (Nat.le_refl _)
  exact unlock_restores (reach_wf ha).ceil_is hlock hlen hp hunlock

/-! ## Property 6: deadlock freedom -/

/-- The pending Job of Task `t` (no Job of `t` in progress) is kept from
starting by the Job at stack position `i`: the executing Job, whose
Priority is at least `t`'s, or the holder of a Resource whose Ceiling is at
least `t`'s Priority. The only waiting relation of the Kernel
(`Sched::blocked_by`). -/
def BlockedBy (cfg : logic.Config NT NR NP) (s : logic.sched.Sched NT NR NL) (t i : Nat) : Prop :=
  t < NT.val ∧ rel s t = 1 ∧ job s t = 0 ∧ ¬ Elig cfg s t ∧ i < s.depth.val ∧
  ((i = s.depth.val - 1 ∧ prio cfg t ≤ prio cfg (stk s (s.depth.val - 1))) ∨
   ∃ e, e < s.lock_len.val ∧ ldep s e = i ∧ prio cfg t ≤ ceil cfg (lres s e))

/-- **Deadlock freedom (R8.5; `lemma_deadlock_free`), of a state with the
run's structure.** Every pending Task that is not eligible has a Job of
its own in progress or is kept from starting by a Job on the stack; and
such a Job is in progress and is kept from starting by nothing: every
wait-for path has length one. -/
theorem deadlock_free {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) (hprio : ∀ k, k < NT.val → 0 < prio cfg k) :
    (∀ t, t < NT.val → rel st.s t = 1 → ¬ Elig cfg st.s t →
      job st.s t ≠ 0 ∨ ∃ i, BlockedBy cfg st.s t i) ∧
    (∀ t i, BlockedBy cfg st.s t i →
      job st.s (stk st.s i) ≠ 0 ∧ ∀ j, ¬ BlockedBy cfg st.s (stk st.s i) j) := by
  refine ⟨fun t ht hr hne => ?_, fun t i hb => ?_⟩
  · by_cases hj : job st.s t = 0
    · right
      by_cases hc : st.s.ceiling.val < prio cfg t
      · -- Not the ceiling: the executing Job's Priority.
        have hd : ¬ (st.s.depth.val = 0 ∨ prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg t) := by
          intro hd
          exact hne ⟨ht, hr, hj, hc, hd⟩
        have hd0 : 0 < st.s.depth.val := by omega
        exact ⟨st.s.depth.val - 1, ht, hr, hj, hne, by omega, Or.inl ⟨rfl, by omega⟩⟩
      · -- The ceiling: the entry that attains it.
        obtain ⟨_, h2⟩ := w.ceil_is
        have hp := hprio t ht
        rcases h2 with h0 | ⟨e, he, hce⟩
        · omega
        · exact ⟨ldep st.s e, ht, hr, hj, hne, w.ldep_lt e he, Or.inr ⟨e, he, rfl, by omega⟩⟩
    · exact Or.inl hj
  · obtain ⟨_, _, _, _, hi, _⟩ := hb
    have hin : job st.s (stk st.s i) ≠ 0 :=
      (w.on_stack _ (w.stk_lt i hi)).mpr ⟨i, hi, rfl⟩
    exact ⟨hin, fun j hbj => hin hbj.2.2.1⟩

/-- Deadlock freedom in every state a run of the scheduler's functions
reaches. -/
theorem reach_deadlock_free {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (h : Reach cfg P st) (hprio : ∀ k, k < NT.val → 0 < prio cfg k) :
    (∀ t, t < NT.val → rel st.s t = 1 → ¬ Elig cfg st.s t →
      job st.s t ≠ 0 ∨ ∃ i, BlockedBy cfg st.s t i) ∧
    (∀ t i, BlockedBy cfg st.s t i →
      job st.s (stk st.s i) ≠ 0 ∧ ∀ j, ¬ BlockedBy cfg st.s (stk st.s i) j) :=
  deadlock_free (reach_wf h) hprio

/-- Deadlock freedom in every state a run of the Kernel's functions
reaches. -/
theorem kreach_deadlock_free {cfg : logic.Config NT NR NP} {P : Params} (hNT : NT.val ≤ 256)
    {x : KSt NT NR NP NL NS NE} (h : KReach cfg P x)
    (hprio : ∀ k, k < NT.val → 0 < prio cfg k) :
    (∀ t, t < NT.val → rel x.k.sched t = 1 → ¬ Elig cfg x.k.sched t →
      job x.k.sched t ≠ 0 ∨ ∃ i, BlockedBy cfg x.k.sched t i) ∧
    (∀ t i, BlockedBy cfg x.k.sched t i →
      job x.k.sched (stk x.k.sched i) ≠ 0 ∧ ∀ j, ¬ BlockedBy cfg x.k.sched (stk x.k.sched i) j) :=
  reach_deadlock_free (ksim hNT h) hprio

end P18
