import Sched.KRun
open Aeneas Aeneas.Std Result

/-! # The Kernel's hold in the run (2026-11-22)

What keeps a pending Kernel-level event from the Kernel is the Kernel
section in progress: one handler, with the unwind it runs before it
returns. This file states that of the run of `Sched/Run.lean`, where it was
a consequence of the transitions' shape and not a statement.

- `holdOf`: the longest Kernel section the run's parameters allow, the
  largest of a dispatch (`Kd`), a lock (`Kin`), an unlock (`Kout`), a
  Budget event's timer entry (`Kt`), an event source's entry (`EC`), a
  Task's Endpoint time (`EP`), and `endHold`: the two entries that end a
  Job and unwind it, the Job end (`Kc`) and the entry that ends an
  overrunning Job (`Kov`).
- `reach_hold`: in every state a run reaches, the Kernel section in
  progress has at most `holdOf` left.
- `step_in_section`, `in_section`: while a Kernel section is in progress,
  time passes only by its own units. An arrival, and the unwinding of the
  Jobs the handler ended (`kend`, `kkill`, `kclear`, which have no Kernel
  section of their own), leave the instant and the section as they are, and
  no other entry begins. So from a state with `ks` left the Kernel is free
  exactly `ks` later, and at most `holdOf` later.
- `kreach_hold`: the same for the run over the Kernel's functions.
- `runParams_hold`: with the Analyzer's inputs, `holdOf` is at most the
  largest of Δ_dispatch, Δ_complete, Δ_lock_in, Δ_lock_out, Δ_timer,
  Δ_detect + Δ_enforce, Δ_irq and a Task's Endpoint cost.

What this is not. `L_kernel` is not a parameter of the run, and Property 18
does not use it: the Analyzer's `B_i` enters only as a bound on the
blocking Critical_Sections (`hL`, `maxCandidate_ge`). What bounds the wait
of an event for its Kernel entry in the proof is the hypothesis `JitterOk`
(every entry begins within `J_rel` of its event's instant), which covers
the section in progress and any entries taken before this one. Entries
that follow one another are separate sections here, each started with no
section in progress (`hk : st.ks = 0`) and each on the account of its own
event or Job. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The entries that end a Job and unwind it before they return: the Job
end and the entry that ends an overrunning Job. -/
def endHold (P : Params) : Nat := max P.Kc P.Kov

/-- The longest Kernel section the parameters allow among `n` Tasks: one
handler with its unwind. -/
def holdOf (n : Nat) (P : Params) : Nat :=
  max P.Kd (max P.Kin (max P.Kout (max P.Kt (max (endHold P)
    (max ((Finset.range P.NQ).sup P.EC) ((Finset.range n).sup P.EP))))))

theorem holdOf_ge (n : Nat) (P : Params) :
    P.Kd ≤ holdOf n P ∧ P.Kin ≤ holdOf n P ∧ P.Kout ≤ holdOf n P ∧ P.Kt ≤ holdOf n P ∧
    P.Kc ≤ holdOf n P ∧ P.Kov ≤ holdOf n P ∧
    (Finset.range P.NQ).sup P.EC ≤ holdOf n P ∧ (Finset.range n).sup P.EP ≤ holdOf n P := by
  unfold holdOf endHold
  omega

theorem ec_le_hold (n : Nat) (P : Params) {q : Nat} (hq : q < P.NQ) : P.EC q ≤ holdOf n P :=
  Nat.le_trans (Finset.le_sup (f := P.EC) (Finset.mem_range.mpr hq)) (holdOf_ge n P).2.2.2.2.2.2.1

/-- The invariant behind `reach_hold`: the section in progress is within
the hold, and no Task has more Endpoint time left than its allowance. -/
def HoldInv (P : Params) (st : St NT NR NL) : Prop :=
  st.ks ≤ holdOf NT.val P ∧ ∀ k, st.re k ≤ (Finset.range NT.val).sup P.EP

theorem step_hold {cfg : logic.Config NT NR NP} {P : Params} {st st' : St NT NR NL}
    (hs : Step cfg P st st') (ih : HoldInv P st) : HoldInv P st' := by
  have hg := holdOf_ge NT.val P
  obtain ⟨h1, h2⟩ := ih
  cases hs with
  | erel q t d c s' hq hk hw hrel hj hd1 hd2 hc hs =>
    exact ⟨Nat.le_trans hc (ec_le_hold _ P hq), h2⟩
  | eplain q c hq hk hw hc hdl => exact ⟨Nat.le_trans hc (ec_le_hold _ P hq), h2⟩
  | edefer q t c s' hq hk hw hrel hc hr0 hs => exact ⟨Nat.le_trans hc (ec_le_hold _ P hq), h2⟩
  | ktick hk hj => exact ⟨by dsimp only; omega, h2⟩
  | dispatch t c s' hk he hmax hc hs =>
    refine ⟨by dsimp only; omega, ?_⟩
    intro k
    dsimp only
    by_cases hkt : k = t.val
    · rw [hkt, Function.update_self]
      exact Finset.le_sup (f := P.EP) (Finset.mem_range.mpr (elig_views.mp he).1)
    · rw [Function.update_of_ne hkt]; exact h2 k
  | lock hne hk r a c s' hrun hrl hc ha hal hs => exact ⟨by dsimp only; omega, h2⟩
  | unlock hne hk r c s' hrun hru hc hal hs => exact ⟨by dsimp only; omega, h2⟩
  | svc hne hk c hd hrun hre hal =>
    refine ⟨?_, ?_⟩
    · have := h2 (stk st.s (st.s.depth.val - 1))
      dsimp only; omega
    · intro k
      dsimp only
      by_cases hkt : k = stk st.s (st.s.depth.val - 1)
      · rw [hkt, Function.update_self]
        have := h2 (stk st.s (st.s.depth.val - 1))
        omega
      · rw [Function.update_of_ne hkt]; exact h2 k
  | bevent hne hk c hd hrun hrb hc => exact ⟨by dsimp only; omega, h2⟩
  | finish hne hk hj ov c r s' hd hr0 hr hov hc hal hs =>
    refine ⟨?_, h2⟩
    dsimp only
    cases ov <;> simp only [Bool.false_eq_true, ite_false, ite_true] at hc <;> omega
  | _ => exact ⟨h1, h2⟩

/-- **The Kernel section in progress is within the hold**, in every state a
run reaches. -/
theorem reach_hold {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (h : Reach cfg P st) : st.ks ≤ holdOf NT.val P := by
  have : HoldInv P st := by
    induction h with
    | boot s0 h0 => exact ⟨Nat.zero_le _, fun _ => Nat.zero_le _⟩
    | step st st' _ hs ih => exact step_hold hs ih
  exact this.1

/-- **While a Kernel section is in progress, time passes only by its own
units, and it stays the section of the same account.** No transition
lengthens it, lets time pass beside it, or starts another entry: the
unwinding of the Jobs it ended is inside it. -/
theorem step_in_section {cfg : logic.Config NT NR NP} {P : Params} {st st' : St NT NR NL}
    (hs : Step cfg P st st') (hk : 0 < st.ks) :
    st'.now + st'.ks = st.now + st.ks ∧ st'.kp = st.kp := by
  cases hs with
  | ktick hk' hj => exact ⟨by dsimp only; omega, rfl⟩
  | jarrive t ht hsp => exact ⟨rfl, rfl⟩
  | earrive q hq hsp => exact ⟨rfl, rfl⟩
  | kend r s' hs => exact ⟨rfl, rfl⟩
  | kkill p s' hp hs => exact ⟨rfl, rfl⟩
  | kclear t s' hs => exact ⟨rfl, rfl⟩
  | _ => omega

/-- Transitions one after another with a Kernel section in progress before
each. -/
inductive InSection (cfg : logic.Config NT NR NP) (P : Params) :
    St NT NR NL → St NT NR NL → Prop
  | refl (a : St NT NR NL) : InSection cfg P a a
  | step (a b c : St NT NR NL) (h : InSection cfg P a b) (hb : 0 < b.ks)
      (hs : Step cfg P b c) : InSection cfg P a c

/-- **The hold.** From a state a run reaches, for as long as the Kernel
section in progress has not ended, the instant is at most the section's
remaining units later, and so at most `holdOf` later: this is how long the
handler in progress, with its unwind, can keep a pending Kernel-level event
from the Kernel. -/
theorem in_section {cfg : logic.Config NT NR NP} {P : Params} {a b : St NT NR NL}
    (ha : Reach cfg P a) (h : InSection cfg P a b) :
    b.now + b.ks = a.now + a.ks ∧ b.now ≤ a.now + holdOf NT.val P := by
  have h1 : b.now + b.ks = a.now + a.ks := by
    induction h with
    | refl => rfl
    | step b c _ hb hs ih => rw [← ih]; exact (step_in_section hs hb).1
  have := reach_hold ha
  exact ⟨h1, by omega⟩

/-- The same bound in the run over the Kernel's functions. -/
theorem kreach_hold {cfg : logic.Config NT NR NP} {P : Params} (hNT : NT.val ≤ 256)
    {x : KSt NT NR NP NL NS NE} (h : KReach cfg P x) : x.ks ≤ holdOf NT.val P :=
  reach_hold (ksim hNT h)

open rsk_analyzer.terms (TaskIn Site Window) in
/-- **The hold of a run with the Analyzer's inputs** is at most the largest
of Δ_dispatch, Δ_complete, Δ_lock_in, Δ_lock_out, Δ_timer,
Δ_detect + Δ_enforce, Δ_irq, and `E`, a bound on every Task's Endpoint
cost. `L_kernel` is not among them. -/
theorem runParams_hold (n : Nat) (tasks : List TaskIn) (sites : List Site)
    (p : rsk_analyzer.terms.Params) (windows : List Window) (L : Nat → Nat → Nat) (E : Nat)
    (hE : ∀ t ∈ tasks, t.endpoint_cost.val ≤ E) :
    holdOf n (runParams n tasks sites p windows L) ≤
      max p.dispatch.val (max p.complete.val (max p.lock_in.val (max p.lock_out.val
        (max p.timer.val (max (p.detect.val + p.enforce.val) (max p.irq.val E)))))) := by
  have hEC : (Finset.range (runParams n tasks sites p windows L).NQ).sup
      (runParams n tasks sites p windows L).EC ≤ max p.timer.val p.irq.val := by
    refine Finset.sup_le ?_
    intro q _
    simp only [runParams]
    split
    · exact Nat.le_max_left _ _
    · split
      · rename_i t _
        unfold evCost
        repeat' split
        all_goals omega
      · exact Nat.zero_le _
  have hEP : (Finset.range n).sup (runParams n tasks sites p windows L).EP ≤ E := by
    refine Finset.sup_le ?_
    intro k _
    simp only [runParams]
    split
    · rename_i t ht
      exact hE t (List.mem_of_getElem? ht)
    · exact Nat.zero_le _
  unfold holdOf endHold
  simp only [runParams] at hEC hEP ⊢
  omega

end P18
