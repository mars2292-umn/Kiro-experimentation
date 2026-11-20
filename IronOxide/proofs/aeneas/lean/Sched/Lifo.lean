import Sched.Blocking
open Aeneas Aeneas.Std Result

/-! # LIFO resumption, the scheduler's half, on the Kernel's code (2026-11-19)

Property 2 of the design: when a Job ends, the most recently preempted Job
resumes (or a Job Property 1 selects starts), and the resumed Job finds its
context as it was at its preemption: its registers, CONTROL, PSP, the MPU
view, and the System_Ceiling.

This file is the part the scheduler state decides. Over a stretch of a run
that begins with the stack at depth `d` and never ends a Job at a position
below `d` (`Nested`), the Jobs at the positions below `d` stay where they
are, the lock entries they hold stay as they are, and when the stack is
back at depth `d` the lock stack and the System_Ceiling are exactly those
of the beginning (`lifo_resume`). The registers are `Sched/StubModel.lean`;
the level record is `Sched/Record.lean`.

Verus states this as postconditions of `Sched::start`, `complete` and
`end_job` (the stack below the top and the entries of the Jobs below are
unchanged, `lemma_ceiling_restored`); here it is one statement over a run
whose scheduler state changes only through the extracted functions. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP : Usize}

/-- The first `n` entries of the lock stack are those held by the Jobs at
the stack positions below `d`. -/
def Low (s : logic.sched.Sched NT NR NL) (d n : Nat) : Prop :=
  n ≤ s.lock_len.val ∧ (∀ p, p < n → ldep s p < d) ∧
  (∀ p, n ≤ p → p < s.lock_len.val → d ≤ ldep s p)

/-- From `s` to `s'` the stack keeps at least depth `d`, its positions
below `d` hold the same Tasks, and the first `n` lock entries are still
those of the Jobs below `d`. -/
def Framed (d n : Nat) (s s' : logic.sched.Sched NT NR NL) : Prop :=
  d ≤ s'.depth.val ∧ (∀ p, p < d → stk s' p = stk s p) ∧ Low s' d n

theorem framed_same {s s' : logic.sched.Sched NT NR NL} {d n : Nat}
    (hdv : s'.depth.val = s.depth.val) (hst : ∀ p, stk s' p = stk s p)
    (hdp : ∀ p, ldep s' p = ldep s p) (hlv : s'.lock_len.val = s.lock_len.val)
    (hd : d ≤ s.depth.val) (hlow : Low s d n) : Framed d n s s' := by
  obtain ⟨hn, hlt, hge⟩ := hlow
  refine ⟨by omega, fun p _ => hst p, by omega, fun p hp => ?_, fun p h1 h2 => ?_⟩
  · rw [hdp]; exact hlt p hp
  · rw [hdp]; exact hge p h1 (by omega)

/-- One step that ends no Job at a position below `d`: it is taken with
the stack deeper than `d`, or it is the dispatch that makes it so. -/
theorem Step.framed {cfg : logic.Config NT NR NP} {P : Params} {st st' : St NT NR NL}
    (w : Wf cfg P st) (h : Step cfg P st st') {d n : Nat}
    (hd : d < st.s.depth.val ∨ (d ≤ st.s.depth.val ∧ st'.s.depth.val = st.s.depth.val + 1))
    (hlow : Low st.s d n) : Framed d n st.s st'.s := by
  have hle : d ≤ st.s.depth.val := by
    rcases hd with h1 | ⟨h1, _⟩
    · omega
    · exact h1
  cases h with
  | erel q t d' c s' hq hk hw hrel hj hd1 hd2 hc hs =>
    obtain ⟨_, _, _, hst, hdeq, hl, hdp, hlen, _⟩ := make_pending_views hs
    exact framed_same (s' := s') (congrArg (·.val) hdeq) hst hdp (congrArg (·.val) hlen) hle hlow
  | edefer q t c s' hq hk hw hrel hc hr0 hs =>
    obtain ⟨_, _, _, hst, hdeq, hl, hdp, hlen, _⟩ := defer_views hs
    exact framed_same (s' := s') (congrArg (·.val) hdeq) hst hdp (congrArg (·.val) hlen) hle hlow
  | kkill p s' hp hs =>
    obtain ⟨_, _, _, hst, hdeq, hl, hdp, hlen, _⟩ := kill_views hs
    exact framed_same (s' := s') (congrArg (·.val) hdeq) hst hdp (congrArg (·.val) hlen) hle hlow
  | kclear t s' hs =>
    obtain ⟨_, _, _, hst, hdeq, hl, hdp, hlen, _⟩ := clear_release_views hs
    exact framed_same (s' := s') (congrArg (·.val) hdeq) hst hdp (congrArg (·.val) hlen) hle hlow
  | dispatch t c s' hk he hmax hc hs =>
    obtain ⟨_, _, _, _, hst, hdv, hl, hdp, hlen, _⟩ := start_views hs
    have hlv : s'.lock_len.val = st.s.lock_len.val := congrArg (·.val) hlen
    obtain ⟨hn, hlt, hge⟩ := hlow
    refine (⟨by omega, fun p hp => ?_, by omega, fun p hp => ?_, fun p h1 h2 => ?_⟩ :
      Framed d n st.s s')
    · rw [hst]; split
      · omega
      · rfl
    · rw [hdp]; exact hlt p hp
    · rw [hdp]; exact hge p h1 (by omega)
  | lock hne hk r' a c s' hrun hrl hc ha hal hs =>
    obtain ⟨⟨_, _, hst, hdeq⟩, hd1, _, hlen, hl, hdp, _⟩ := lock_views hs
    have hdv : s'.depth.val = st.s.depth.val := congrArg (·.val) hdeq
    have hd' : d < st.s.depth.val ∨ (d ≤ st.s.depth.val ∧ s'.depth.val = st.s.depth.val + 1) := hd
    have hdd : d < st.s.depth.val := by
      rcases hd' with h1 | ⟨_, h1⟩ <;> omega
    obtain ⟨hn, hlt, hge⟩ := hlow
    refine (⟨by omega, fun p _ => hst p, by omega, fun p hp => ?_, fun p h1 h2 => ?_⟩ :
      Framed d n st.s s')
    · rw [hdp]; split
      · omega
      · exact hlt p hp
    · rw [hdp]; split
      · omega
      · exact hge p h1 (by omega)
  | unlock hne hk r' c s' hrun hru hc hal hs =>
    obtain ⟨⟨_, _, hst, hdeq⟩, hl1, hd1, _, htopd, hlen, hl, hdp, _⟩ := unlock_views hs
    have hdv : s'.depth.val = st.s.depth.val := congrArg (·.val) hdeq
    have hd' : d < st.s.depth.val ∨ (d ≤ st.s.depth.val ∧ s'.depth.val = st.s.depth.val + 1) := hd
    have hdd : d < st.s.depth.val := by
      rcases hd' with h1 | ⟨_, h1⟩ <;> omega
    obtain ⟨hn, hlt, hge⟩ := hlow
    have hnl : n ≤ st.s.lock_len.val - 1 := by
      by_contra hcn
      have := hlt (st.s.lock_len.val - 1) (by omega)
      omega
    refine (⟨by omega, fun p _ => hst p, by omega, fun p hp => ?_, fun p h1 h2 => ?_⟩ :
      Framed d n st.s s')
    · rw [hdp]; exact hlt p hp
    · rw [hdp]; exact hge p h1 (by omega)
  | finish hne hk hj ov c r' s' hd0 hr0 hr hov hc hal hs =>
    obtain ⟨hd1, _, _, _, _, hst, hdv, hl, hdp, hlen, hpop, _, _⟩ := finish_top_views hs
    have hd' : d < st.s.depth.val ∨ (d ≤ st.s.depth.val ∧ s'.depth.val = st.s.depth.val + 1) := hd
    have hdd : d < st.s.depth.val := by
      rcases hd' with h1 | ⟨_, h1⟩ <;> omega
    obtain ⟨hn, hlt, hge⟩ := hlow
    have hnl : n ≤ s'.lock_len.val := by
      by_contra hcn
      have h1 := hlt s'.lock_len.val (by omega)
      have h2 := hpop s'.lock_len.val (Nat.le_refl _) (by omega)
      omega
    refine (⟨by omega, fun p _ => hst p, hnl, fun p hp => ?_, fun p h1 h2 => ?_⟩ :
      Framed d n st.s s')
    · rw [hdp]; exact hlt p hp
    · rw [hdp]; exact hge p h1 (by omega)
  | kend r' s' hs =>
    obtain ⟨hd1, _, _, _, _, hst, hdv, hl, hdp, hlen, hpop, _, _⟩ := finish_top_views hs
    have hd' : d < st.s.depth.val ∨ (d ≤ st.s.depth.val ∧ s'.depth.val = st.s.depth.val + 1) := hd
    have hdd : d < st.s.depth.val := by
      rcases hd' with h1 | ⟨_, h1⟩ <;> omega
    obtain ⟨hn, hlt, hge⟩ := hlow
    have hnl : n ≤ s'.lock_len.val := by
      by_contra hcn
      have h1 := hlt s'.lock_len.val (by omega)
      have h2 := hpop s'.lock_len.val (Nat.le_refl _) (by omega)
      omega
    refine (⟨by omega, fun p _ => hst p, hnl, fun p hp => ?_, fun p h1 h2 => ?_⟩ :
      Framed d n st.s s')
    · rw [hdp]; exact hlt p hp
    · rw [hdp]; exact hge p h1 (by omega)
  | _ => exact framed_same (s' := st.s) rfl (fun _ => rfl) (fun _ => rfl) rfl hle hlow

/-- A stretch of a run from `a` during which no Job at a stack position
below `d` ends: every step is taken with the stack deeper than `d`, or is
a dispatch at depth `d` or above. -/
inductive Nested (cfg : logic.Config NT NR NP) (P : Params) (d : Nat) (a : St NT NR NL) :
    St NT NR NL → Prop
  | refl : Nested cfg P d a a
  | step (b c : St NT NR NL) (h : Nested cfg P d a b)
      (hd : d < b.s.depth.val ∨ (d ≤ b.s.depth.val ∧ c.s.depth.val = b.s.depth.val + 1))
      (hs : Step cfg P b c) : Nested cfg P d a c

theorem Nested.wf {cfg : logic.Config NT NR NP} {P : Params} {d : Nat} {a c : St NT NR NL}
    (wa : Wf cfg P a) (h : Nested cfg P d a c) : Wf cfg P c := by
  induction h with
  | refl => exact wa
  | step b c _ _ hs ih => exact ih.step hs

/-- Over such a stretch the Jobs below `d` and their lock entries are
untouched. -/
theorem Nested.framed {cfg : logic.Config NT NR NP} {P : Params} {d n : Nat}
    {a c : St NT NR NL} (wa : Wf cfg P a) (h : Nested cfg P d a c)
    (hda : d ≤ a.s.depth.val) (hlow : Low a.s d n) :
    Framed d n a.s c.s ∧ ∀ r, r < n → Same a.s c.s r := by
  induction h with
  | refl =>
    have hl2 := hlow
    obtain ⟨hn, _, _⟩ := hl2
    exact ⟨⟨hda, fun _ _ => rfl, hlow⟩, fun r hr => ⟨by omega, by omega, rfl, rfl, rfl⟩⟩
  | step b c hab hd hbc ih =>
    obtain ⟨⟨_, hstk, hlowb⟩, hsame⟩ := ih
    have wb := hab.wf wa
    obtain ⟨f1, f2, f3⟩ := Step.framed wb hbc hd hlowb
    have hlb2 := hlowb
    obtain ⟨hnb, _, _⟩ := hlb2
    have f32 := f3
    obtain ⟨hnc, _, _⟩ := f32
    exact ⟨⟨f1, fun p hp => (f2 p hp).trans (hstk p hp), f3⟩,
      fun r hr => (hsame r hr).trans (Step.entry_kept wb hbc (by omega) (by omega))⟩

/-- **Property 2, the scheduler's half.** From a state `a` through a
stretch that ends no Job already on the stack in `a`, to a state `c` whose
stack is as deep as `a`'s: the stack holds the same Tasks (the Job that
was executing in `a` is the Job on top again: the most recently preempted
Job resumes), the lock stack is the same, entry by entry, and the
System_Ceiling is the one of `a`. -/
theorem lifo_resume {cfg : logic.Config NT NR NP} {P : Params} {a c : St NT NR NL}
    (wa : Wf cfg P a) (h : Nested cfg P a.s.depth.val a c)
    (hc : c.s.depth.val = a.s.depth.val) :
    (∀ p, p < a.s.depth.val → stk c.s p = stk a.s p) ∧
    c.s.lock_len.val = a.s.lock_len.val ∧
    (∀ r, r < a.s.lock_len.val → lres c.s r = lres a.s r ∧ ldep c.s r = ldep a.s r) ∧
    c.s.ceiling.val = a.s.ceiling.val := by
  have wc := h.wf wa
  have hlow : Low a.s a.s.depth.val a.s.lock_len.val :=
    ⟨Nat.le_refl _, fun p hp => wa.ldep_lt p hp, fun p h1 h2 => by omega⟩
  obtain ⟨⟨_, hstk, hn, _, hge⟩, hsame⟩ := h.framed wa (Nat.le_refl _) hlow
  have hlen : c.s.lock_len.val = a.s.lock_len.val := by
    by_contra hne
    have h1 := hge a.s.lock_len.val (Nat.le_refl _) (by omega)
    have h2 := wc.ldep_lt a.s.lock_len.val (by omega)
    omega
  have hent : ∀ r, r < a.s.lock_len.val → lres c.s r = lres a.s r ∧ ldep c.s r = ldep a.s r := by
    intro r hr
    obtain ⟨_, _, e1, e2, _⟩ := hsame r hr
    exact ⟨e1, e2⟩
  refine ⟨hstk, hlen, hent, ?_⟩
  obtain ⟨ca1, ca2⟩ := wa.ceil_is
  obtain ⟨cc1, cc2⟩ := wc.ceil_is
  have hle1 : c.s.ceiling.val ≤ a.s.ceiling.val := by
    rcases cc2 with h0 | ⟨p, hp, he⟩
    · omega
    · have := ca1 p (by omega)
      rw [(hent p (by omega)).1] at he
      omega
  have hle2 : a.s.ceiling.val ≤ c.s.ceiling.val := by
    rcases ca2 with h0 | ⟨p, hp, he⟩
    · omega
    · have := cc1 p (by omega)
      rw [(hent p hp).1] at this
      omega
  omega

/-- The same of a state a run reaches from boot. -/
theorem reach_lifo_resume {cfg : logic.Config NT NR NP} {P : Params} {a c : St NT NR NL}
    (hr : Reach cfg P a) (h : Nested cfg P a.s.depth.val a c)
    (hc : c.s.depth.val = a.s.depth.val) :
    (∀ p, p < a.s.depth.val → stk c.s p = stk a.s p) ∧
    c.s.lock_len.val = a.s.lock_len.val ∧
    (∀ r, r < a.s.lock_len.val → lres c.s r = lres a.s r ∧ ldep c.s r = ldep a.s r) ∧
    c.s.ceiling.val = a.s.ceiling.val :=
  lifo_resume (reach_wf hr) h hc

end P18
