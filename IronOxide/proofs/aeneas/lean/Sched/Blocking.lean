import Sched.Stage
open Aeneas Aeneas.Std Result

/-! # Single-section blocking, on the Kernel's code (2026-11-19)

Property 5 of the design: the time a Job spends released but not eligible
only because of the System_Ceiling is at most one interval per Release,
inside one Critical_Section (with its nested sections) of one Job of lower
Priority on a Resource whose Ceiling is at least the Job's Priority.

Verus proves it of the scheduler model in two halves: three state lemmas
(`Sched::lemma_blocked_by_one_holder`, `lemma_one_low_blocker`,
`lemma_low_locker_nested`) and two lemmas over the ghost history
(`lemma_no_new_blocking_section`, `lemma_no_lower_dispatch_while_waiting`)
under the premise that an eligible Job is dispatched at once.

Here both halves are stated of a run (`Step`, `Sched/Run.lean`) whose
scheduler state changes only through the Kernel's extracted functions. The
promptness premise is the run's own rule (a Job executes, locks and unlocks
only when the eligibility test accepts no Task), so the trace half needs no
premise of its own, and it is joined into one statement over any stretch
of a run during which the Task waits (`single_section`, `one_interval`).

What this file does not state is the length of that Critical_Section in
time: that bound is `Blk` in `Sched/Stage.lean`, which the response-time
theorem uses. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP : Usize}

/-- Task `t` has a Release pending and no Job in progress. -/
def Waiting (s : logic.sched.Sched NT NR NL) (t : Nat) : Prop :=
  t < NT.val ∧ rel s t = 1 ∧ job s t = 0

theorem blocking_lt {cfg : logic.Config NT NR NP} {s : logic.sched.Sched NT NR NL} {t p : Nat}
    (h : blocking cfg t s p) : p < s.lock_len.val := by
  obtain ⟨h1, _, _⟩ := h
  exact h1

/-- A Job on the stack has at most the Priority of the Job on top. -/
theorem Wf.below_top {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) {p : Nat} (hp : p < st.s.depth.val) :
    prio cfg (stk st.s p) ≤ prio cfg (stk st.s (st.s.depth.val - 1)) := by
  by_cases hpt : p = st.s.depth.val - 1
  · rw [hpt]
  · exact Nat.le_of_lt (w.sorted p (st.s.depth.val - 1) (by omega) (by omega))

/-! ## The state half -/

/-- `lemma_blocked_by_one_holder`: a waiting Task the eligibility test
refuses, though no Job of its Priority or above is executing, is refused
because of the System_Ceiling: some lock entry has a Ceiling at least its
Priority and a holder of lower Priority. -/
theorem waiting_blocked {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) {t : Nat} (hw : Waiting st.s t) (hp : 0 < prio cfg t)
    (hne : ¬ Elig cfg st.s t)
    (htop : st.s.depth.val = 0 ∨ prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg t) :
    ∃ p, blocking cfg t st.s p := by
  obtain ⟨htn, hrel, hjob⟩ := hw
  have hceil : prio cfg t ≤ st.s.ceiling.val := by
    by_contra hc
    exact hne (elig_views.mpr ⟨htn, hrel, hjob, by omega, htop⟩)
  obtain ⟨p, hpl, hpc⟩ : ∃ p, p < st.s.lock_len.val ∧ st.s.ceiling.val = ceil cfg (lres st.s p) := by
    rcases w.ceil_is.2 with h | h
    · omega
    · exact h
  have hld := w.ldep_lt p hpl
  have hb := w.below_top hld
  refine ⟨p, hpl, by omega, ?_⟩
  rcases htop with h0 | h1 <;> omega

/-- `lemma_one_low_blocker`: the entries that block a Task are all held by
one Job. -/
theorem one_low_blocker {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) {t e f : Nat}
    (he : blocking cfg t st.s e) (hf : blocking cfg t st.s f) : ldep st.s e = ldep st.s f := by
  obtain ⟨hel, hec, hep⟩ := he
  obtain ⟨hfl, hfc, hfp⟩ := hf
  by_contra hne
  rcases Nat.lt_or_gt_of_ne hne with h | h
  · have := w.srp e (ldep st.s f) hel (w.ldep_lt f hfl) h
    omega
  · have := w.srp f (ldep st.s e) hfl (w.ldep_lt e hel) h
    omega

/-- `lemma_low_locker_nested`: when the Job on top has a Priority below
Task `t`'s and the System_Ceiling is at least Task `t`'s Priority, the Job
on top itself holds an entry that blocks Task `t`. -/
theorem low_locker_nested {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) {t : Nat} (hd : 0 < st.s.depth.val)
    (hlow : prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg t)
    (hc : prio cfg t ≤ st.s.ceiling.val) :
    ∃ e, topEntry st.s e ∧ blocking cfg t st.s e := by
  obtain ⟨p, hpl, hpc⟩ : ∃ p, p < st.s.lock_len.val ∧ st.s.ceiling.val = ceil cfg (lres st.s p) := by
    rcases w.ceil_is.2 with h | h
    · omega
    · exact h
  have hld := w.ldep_lt p hpl
  have htop : ldep st.s p = st.s.depth.val - 1 := by
    by_contra hn
    have := w.srp p (st.s.depth.val - 1) hpl (by omega) (by omega)
    omega
  exact ⟨p, ⟨hpl, htop⟩, hpl, by omega, by rw [htop]; exact hlow⟩

/-- `lemma_no_lower_dispatch_while_waiting`: the Task a dispatch starts has
at least the Priority of every waiting Task. -/
theorem dispatch_not_lower {cfg : logic.Config NT NR NP} {s : logic.sched.Sched NT NR NL}
    {t u : Nat} (hw : Waiting s t) (he : Elig cfg s u)
    (hmax : ∀ v, Elig cfg s v → prio cfg v ≤ prio cfg u) : prio cfg t ≤ prio cfg u := by
  obtain ⟨htn, hrel, hjob⟩ := hw
  by_cases het : Elig cfg s t
  · exact hmax t het
  · obtain ⟨_, _, _, hc, htop⟩ := elig_views.mp he
    by_contra hlt
    apply het
    refine elig_views.mpr ⟨htn, hrel, hjob, by omega, ?_⟩
    rcases htop with h | h
    · exact Or.inl h
    · exact Or.inr (by omega)

/-! ## One step of a run -/

/-- Entry `r` of the lock stack is the same in two states: the same
Resource, held at the same stack position by the same Task. -/
def Same (a c : logic.sched.Sched NT NR NL) (r : Nat) : Prop :=
  r < a.lock_len.val ∧ r < c.lock_len.val ∧ lres c r = lres a r ∧ ldep c r = ldep a r ∧
  stk c (ldep a r) = stk a (ldep a r)

theorem Same.lt_right {a c : logic.sched.Sched NT NR NL} {r : Nat} (h : Same a c r) :
    r < c.lock_len.val := by
  obtain ⟨_, h2, _⟩ := h
  exact h2

theorem Same.trans {a b c : logic.sched.Sched NT NR NL} {r : Nat}
    (h1 : Same a b r) (h2 : Same b c r) : Same a c r := by
  obtain ⟨a1, a2, a3, a4, a5⟩ := h1
  obtain ⟨b1, b2, b3, b4, b5⟩ := h2
  refine ⟨a1, b2, by rw [b3, a3], by rw [b4, a4], ?_⟩
  rw [a4] at b5
  rw [b5, a5]

theorem Same.blk_iff {cfg : logic.Config NT NR NP} {a c : logic.sched.Sched NT NR NL}
    {t r : Nat} (h : Same a c r) : blocking cfg t c r ↔ blocking cfg t a r := by
  obtain ⟨a1, a2, a3, a4, a5⟩ := h
  unfold blocking
  rw [a3, a4, a5]
  constructor
  · rintro ⟨_, h2, h3⟩; exact ⟨a1, h2, h3⟩
  · rintro ⟨_, h2, h3⟩; exact ⟨a2, h2, h3⟩

/-- A step keeps every lock entry that is on the stack before and after
it. -/
theorem Step.entry_kept {cfg : logic.Config NT NR NP} {P : Params} {st st' : St NT NR NL}
    (w : Wf cfg P st) (h : Step cfg P st st') {r : Nat}
    (h1 : r < st.s.lock_len.val) (h2 : r < st'.s.lock_len.val) : Same st.s st'.s r := by
  have hld := w.ldep_lt r h1
  cases h with
  | erel q t d c s' hq hk hw hrel hj hd1 hd2 hc hs =>
    obtain ⟨_, _, _, hst, _, hl, hdp, _, _⟩ := make_pending_views hs
    exact ⟨h1, h2, hl r, hdp r, hst _⟩
  | edefer q t c s' hq hk hw hrel hc hr0 hs =>
    obtain ⟨_, _, _, hst, _, hl, hdp, _, _⟩ := defer_views hs
    exact ⟨h1, h2, hl r, hdp r, hst _⟩
  | dispatch t c s' hk he hmax hc hs =>
    obtain ⟨_, _, _, _, hst, _, hl, hdp, _, _⟩ := start_views hs
    refine ⟨h1, h2, hl r, hdp r, ?_⟩
    show stk s' (ldep st.s r) = stk st.s (ldep st.s r)
    rw [hst]; split <;> omega
  | lock hne hk r' a c s' hrun hrl hc ha hal hs =>
    obtain ⟨⟨_, _, hst, _⟩, _, _, _, hl, hdp, _⟩ := lock_views hs
    refine ⟨h1, h2, ?_, ?_, hst _⟩
    · show lres s' r = lres st.s r
      rw [hl]; split <;> omega
    · show ldep s' r = ldep st.s r
      rw [hdp]; split <;> omega
  | unlock hne hk r' c s' hrun hru hc hal hs =>
    obtain ⟨⟨_, _, hst, _⟩, _, _, _, _, _, hl, hdp, _⟩ := unlock_views hs
    exact ⟨h1, h2, hl r, hdp r, hst _⟩
  | finish hne hk hj ov c r' s' hd hr0 hr hov hc hal hs =>
    obtain ⟨_, _, _, _, _, hst, _, hl, hdp, _⟩ := finish_top_views hs
    exact ⟨h1, h2, hl r, hdp r, hst _⟩
  | kend r' s' hs =>
    obtain ⟨_, _, _, _, _, hst, _, hl, hdp, _⟩ := finish_top_views hs
    exact ⟨h1, h2, hl r, hdp r, hst _⟩
  | kkill p s' hp hs =>
    obtain ⟨_, _, _, hst, _, hl, hdp, _, _⟩ := kill_views hs
    exact ⟨h1, h2, hl r, hdp r, hst _⟩
  | kclear t s' hs =>
    obtain ⟨_, _, _, hst, _, hl, hdp, _, _⟩ := clear_release_views hs
    exact ⟨h1, h2, hl r, hdp r, hst _⟩
  | _ => exact ⟨h1, h2, rfl, rfl, rfl⟩

/-- Only a lock lengthens the lock stack: by one entry, held by the Job on
top, when the eligibility test accepts no Task. -/
theorem Step.len {cfg : logic.Config NT NR NP} {P : Params} {st st' : St NT NR NL}
    (h : Step cfg P st st') :
    st'.s.lock_len.val ≤ st.s.lock_len.val ∨
    ((∀ u, ¬ Elig cfg st.s u) ∧ 1 ≤ st.s.depth.val ∧
      st'.s.lock_len.val = st.s.lock_len.val + 1 ∧
      ldep st'.s st.s.lock_len.val = st.s.depth.val - 1 ∧
      stk st'.s (st.s.depth.val - 1) = stk st.s (st.s.depth.val - 1)) := by
  cases h with
  | erel q t d c s' hq hk hw hrel hj hd1 hd2 hc hs =>
    obtain ⟨_, _, _, _, _, _, _, hlen, _⟩ := make_pending_views hs
    left; exact Nat.le_of_eq (congrArg (·.val) hlen)
  | edefer q t c s' hq hk hw hrel hc hr0 hs =>
    obtain ⟨_, _, _, _, _, _, _, hlen, _⟩ := defer_views hs
    left; exact Nat.le_of_eq (congrArg (·.val) hlen)
  | dispatch t c s' hk he hmax hc hs =>
    obtain ⟨_, _, _, _, _, _, _, _, hlen, _⟩ := start_views hs
    left; exact Nat.le_of_eq (congrArg (·.val) hlen)
  | lock hne hk r' a c s' hrun hrl hc ha hal hs =>
    obtain ⟨⟨_, _, hst, _⟩, hd1, _, hlen, _, hdp, _⟩ := lock_views hs
    right
    refine ⟨hne, hd1, hlen, ?_, hst _⟩
    show ldep s' st.s.lock_len.val = st.s.depth.val - 1
    rw [hdp]; simp
  | unlock hne hk r' c s' hrun hru hc hal hs =>
    obtain ⟨_, _, _, _, _, hlen, _⟩ := unlock_views hs
    left
    show s'.lock_len.val ≤ st.s.lock_len.val
    omega
  | finish hne hk hj ov c r' s' hd hr0 hr hov hc hal hs =>
    obtain ⟨_, _, _, _, _, _, _, _, _, hlen, _⟩ := finish_top_views hs
    left; exact hlen
  | kend r' s' hs =>
    obtain ⟨_, _, _, _, _, _, _, _, _, hlen, _⟩ := finish_top_views hs
    left; exact hlen
  | kkill p s' hp hs =>
    obtain ⟨_, _, _, _, _, _, _, hlen, _⟩ := kill_views hs
    left; exact Nat.le_of_eq (congrArg (·.val) hlen)
  | kclear t s' hs =>
    obtain ⟨_, _, _, _, _, _, _, hlen, _⟩ := clear_release_views hs
    left; exact Nat.le_of_eq (congrArg (·.val) hlen)
  | _ => left; exact Nat.le_refl _

/-- `lemma_no_new_blocking_section`, of every step: while Task `t` waits,
an entry that blocks it after a step either blocked it before the step, or
is a lock of the Job that already held an entry blocking it (the new
Critical_Section is nested in the blocking one). -/
theorem blocking_step {cfg : logic.Config NT NR NP} {P : Params} {st st' : St NT NR NL}
    (w : Wf cfg P st) (h : Step cfg P st st') {t q : Nat}
    (hw : Waiting st.s t) (hb : blocking cfg t st'.s q) :
    q < st.s.lock_len.val ∨
    ∃ p, p < q ∧ blocking cfg t st.s p ∧ ldep st'.s q = ldep st.s p := by
  have hql := blocking_lt hb
  rcases h.len with hle | ⟨hne, hd, hlen, hdq, hstk⟩
  · left; omega
  · by_cases hq : q < st.s.lock_len.val
    · left; exact hq
    · right
      have hqe : q = st.s.lock_len.val := by omega
      obtain ⟨htn, hrel, hjob⟩ := hw
      obtain ⟨_, hqc, hqp⟩ := hb
      rw [hqe, hdq, hstk] at hqp
      have hceil : prio cfg t ≤ st.s.ceiling.val := by
        by_contra hc
        exact hne t (elig_views.mpr ⟨htn, hrel, hjob, by omega, Or.inr hqp⟩)
      obtain ⟨e, hte, hbe⟩ := low_locker_nested w (by omega) hqp hceil
      obtain ⟨hel, hed⟩ := hte
      exact ⟨e, by omega, hbe, by rw [hqe, hdq, hed]⟩

/-- A Release leaves the blocking entries as they are. -/
theorem release_keeps_blocking {cfg : logic.Config NT NR NP} {s s' : logic.sched.Sched NT NR NL}
    {u : U8} (h : logic.sched.Sched.make_pending s cfg u = ok s') (t p : Nat) :
    blocking cfg t s' p ↔ blocking cfg t s p := by
  obtain ⟨_, _, _, hst, _, hl, hdp, hlen, _⟩ := make_pending_views h
  unfold blocking
  rw [hl, hdp, hst, hlen]

/-! ## A stretch of a run during which the Task waits -/

/-- A run from `a` in which Task `t` waits in every state a step is taken
from. -/
inductive WaitRun (cfg : logic.Config NT NR NP) (P : Params) (t : Nat) (a : St NT NR NL) :
    St NT NR NL → Prop
  | refl : WaitRun cfg P t a a
  | step (b c : St NT NR NL) (h : WaitRun cfg P t a b) (hw : Waiting b.s t)
      (hs : Step cfg P b c) : WaitRun cfg P t a c

theorem WaitRun.wf {cfg : logic.Config NT NR NP} {P : Params} {t : Nat} {a c : St NT NR NL}
    (wa : Wf cfg P a) (h : WaitRun cfg P t a c) : Wf cfg P c := by
  induction h with
  | refl => exact wa
  | step b c _ _ hs ih => exact ih.step hs

/-- An entry that blocks the Task at the end of the stretch is at or above
an entry that blocked it at the beginning, and the lock stack up to that
entry is as it was. -/
theorem WaitRun.blocking_from {cfg : logic.Config NT NR NP} {P : Params} {t : Nat}
    {a c : St NT NR NL} (wa : Wf cfg P a) (h : WaitRun cfg P t a c) :
    ∀ q, blocking cfg t c.s q →
      ∃ p, p ≤ q ∧ blocking cfg t a.s p ∧ ∀ r, r ≤ p → Same a.s c.s r := by
  induction h with
  | refl =>
    intro q hb
    have hql := blocking_lt hb
    exact ⟨q, Nat.le_refl _, hb, fun r hr => ⟨by omega, by omega, rfl, rfl, rfl⟩⟩
  | step b c hab hwb hbc ih =>
    intro q hb
    have hql := blocking_lt hb
    have wb := hab.wf wa
    rcases blocking_step wb hbc hwb hb with hq | ⟨p', hp'q, hbp', _⟩
    · have sq := Step.entry_kept wb hbc hq hql
      obtain ⟨p, hpq, hbp, hsame⟩ := ih q (sq.blk_iff.mp hb)
      refine ⟨p, hpq, hbp, fun r hr => ?_⟩
      have s1 := hsame r hr
      exact s1.trans (Step.entry_kept wb hbc s1.lt_right (by omega))
    · obtain ⟨p, hpp', hbp, hsame⟩ := ih p' hbp'
      refine ⟨p, by omega, hbp, fun r hr => ?_⟩
      have s1 := hsame r hr
      exact s1.trans (Step.entry_kept wb hbc s1.lt_right (by omega))

/-- **Property 5.** Over a stretch of a run during which Task `t` waits:
whenever an entry `q` blocks it, the outermost entry `p0` that blocked it
at the beginning of the stretch is still on the lock stack, the same
Resource held by the same Task; `q` is that entry or one above it (a
Critical_Section nested in `p0`'s), held by the same Job. -/
theorem single_section {cfg : logic.Config NT NR NP} {P : Params} {t : Nat}
    {a c : St NT NR NL} (wa : Wf cfg P a) (h : WaitRun cfg P t a c) {q p0 : Nat}
    (hb : blocking cfg t c.s q) (h0 : blocking cfg t a.s p0)
    (hmin : ∀ r, blocking cfg t a.s r → p0 ≤ r) :
    p0 ≤ q ∧ Same a.s c.s p0 ∧ blocking cfg t c.s p0 ∧ ldep c.s q = ldep c.s p0 := by
  obtain ⟨p, hpq, hbp, hsame⟩ := h.blocking_from wa q hb
  have hp0 := hmin p hbp
  have s0 := hsame p0 hp0
  have hb0 := s0.blk_iff.mpr h0
  exact ⟨by omega, s0, hb0, one_low_blocker (h.wf wa) hb hb0⟩

/-- **Property 5, one interval per Release.** Once no entry blocks the
waiting Task, none blocks it for as long as it waits. -/
theorem one_interval {cfg : logic.Config NT NR NP} {P : Params} {t : Nat}
    {a c : St NT NR NL} (wa : Wf cfg P a) (h : WaitRun cfg P t a c)
    (hnone : ∀ p, ¬ blocking cfg t a.s p) (q : Nat) : ¬ blocking cfg t c.s q := by
  intro hb
  obtain ⟨p, _, hbp, _⟩ := h.blocking_from wa q hb
  exact hnone p hbp

/-- The same of a state a run reaches from boot. -/
theorem reach_single_section {cfg : logic.Config NT NR NP} {P : Params} {t : Nat}
    {a c : St NT NR NL} (hr : Reach cfg P a) (h : WaitRun cfg P t a c) {q p0 : Nat}
    (hb : blocking cfg t c.s q) (h0 : blocking cfg t a.s p0)
    (hmin : ∀ r, blocking cfg t a.s r → p0 ≤ r) :
    p0 ≤ q ∧ Same a.s c.s p0 ∧ blocking cfg t c.s p0 ∧ ldep c.s q = ldep c.s p0 :=
  single_section (reach_wf hr) h hb h0 hmin

theorem reach_one_interval {cfg : logic.Config NT NR NP} {P : Params} {t : Nat}
    {a c : St NT NR NL} (hr : Reach cfg P a) (h : WaitRun cfg P t a c)
    (hnone : ∀ p, ¬ blocking cfg t a.s p) (q : Nat) : ¬ blocking cfg t c.s q :=
  one_interval (reach_wf hr) h hnone q

end P18

