import Sched.Stage1
import Rta.Chain
open Aeneas Aeneas.Std Result

namespace P18
open rsk_logic
open rsk_analyzer (ceilDiv interf stepF specTerms termJobs termIntake)

variable {NT NR NL NP : Usize}

/-- The run's timing parameters read from the Analyzer's inputs: Task
`k`'s period or MIT, and the bound `E_k` on one Job's execution. -/
def paramsOf (tasks : List rsk_analyzer.terms.Interferer) : Params :=
  { T := fun k => match tasks[k]? with
      | some t => t.period.val
      | none => 0,
    C := fun k => match tasks[k]? with
      | some t => t.bound.val
      | none => 0 }

/-- Task `k`'s Priority in the Analyzer's inputs. -/
def prioOf (tasks : List rsk_analyzer.terms.Interferer) (k : Nat) : Nat :=
  match tasks[k]? with
  | some t => t.priority.val
  | none => 0

theorem interf_append (a : List rsk_analyzer.rta.Term) (x : rsk_analyzer.rta.Term) (w : Nat) :
    interf (a ++ [x]) w = interf a w + ceilDiv (w + x.jitter.val) x.period.val * x.bound.val := by
  induction a with
  | nil => simp [interf]
  | cons y ys ih => simp [interf, ih]; omega

/-- With no jitter and no intake cost, the Analyzer's interference terms
are the Jobs of the other Tasks of equal or higher Priority. -/
theorem interf_spec (tasks : List rsk_analyzer.terms.Interferer)
    (ti : rsk_analyzer.terms.Interferer) (i : Nat)
    (hplain : ∀ t ∈ tasks, t.jitter.val = 0 ∧ t.intake.val = 0) (R : Nat) :
    ∀ n, n ≤ tasks.length →
    interf (specTerms tasks ti i n) R =
      ∑ k ∈ Finset.range n,
        if k ≠ i ∧ ti.priority.val ≤ prioOf tasks k
        then (paramsOf tasks).C k * ceilDiv R ((paramsOf tasks).T k) else 0 := by
  intro n
  induction n with
  | zero => intro _; simp [specTerms, interf]
  | succ n ih =>
    intro hn
    have hlt : n < tasks.length := by omega
    have hget : tasks[n]? = some tasks[n] := List.getElem?_eq_getElem hlt
    obtain ⟨hj, hin⟩ := hplain tasks[n] (List.getElem_mem hlt)
    rw [Finset.sum_range_succ, ← ih (by omega)]
    simp only [specTerms, hget]
    have hp : prioOf tasks n = (tasks[n]).priority.val := by simp [prioOf, hget]
    have hC : (paramsOf tasks).C n = (tasks[n]).bound.val := by simp [paramsOf, hget]
    have hT : (paramsOf tasks).T n = (tasks[n]).period.val := by simp [paramsOf, hget]
    by_cases h1 : n = i
    · simp [h1]
    · by_cases h2 : (tasks[n]).priority.val ≥ ti.priority.val
      · simp only [h1, ite_false, h2, ite_true]
        rw [interf_append]
        simp only [termJobs, hj, Nat.add_zero, hp, hC, hT, ne_eq, h1, not_false_eq_true, true_and]
        rw [ite_eq_left_of_eq_true _ _ (eq_true h2), Nat.mul_comm]
      · simp only [h1, ite_false, h2, hin]
        simp [hp, h2]

theorem ceilDiv_one (R T : Nat) (hR : 0 < R) (h : R ≤ T) : ceilDiv R T = 1 := by
  unfold ceilDiv
  exact Nat.div_eq_of_lt_le (by omega) (by omega)

/-- **Property 18, stage 1** (no Resource is locked; no Kernel overhead,
no jitter, no intake cost). `cfg` is the Kernel's configuration and
`tasks` the Analyzer's inputs for the same Tasks, with the same
Priorities. If `response_of`, as compiled, reports Task `i` schedulable
with response time `v.r`, then in every run of the Kernel's scheduler
whose Releases of each Task `k` are at least `tasks[k].period` apart and
whose Jobs of Task `k` each execute for at most `tasks[k].bound`, in
every state in which Task `i` has unfinished work its last Release was
less than `v.r` ago: every Job of Task `i` completes within `v.r` of its
Release. -/
theorem property18_stage1 {cfg : logic.Config NT NR NP}
    (tasks : alloc.vec.Vec rsk_analyzer.terms.Interferer) (i : Usize)
    (ti : rsk_analyzer.terms.Interferer) (hti : tasks.val[i.val]? = some ti)
    (c deadline : U64)
    (v : rsk_analyzer.terms.Verdict) (own : alloc.vec.Vec rsk_analyzer.terms.TermOf)
    (hres : rsk_analyzer.terms.response_of tasks i c 0#u64 0#u64 deadline = ok (v, own))
    (hsched : v.schedulable = true)
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hprio : ∀ k, k < NT.val → 0 < prio cfg k ∧ prio cfg k = prioOf tasks.val k)
    (hper : ∀ t ∈ tasks.val, 0 < t.period.val)
    (hplain : ∀ t ∈ tasks.val, t.jitter.val = 0 ∧ t.intake.val = 0)
    (hc : 0 < c.val ∧ ti.bound.val ≤ c.val) (hdl : deadline.val ≤ ti.period.val)
    {st : St NT NR NL} (hreach : Reach cfg (paramsOf tasks.val) st)
    (hpos : 0 < st.pend i.val + st.run i.val) :
    ∃ r, st.last i.val = some r ∧ st.now < r + v.r.val := by
  have hspec := rsk_analyzer.response_of_spec tasks i ti hti c 0#u64 0#u64 deadline hper
  rw [hres] at hspec
  simp only [WP.spec_ok] at hspec
  obtain ⟨_, _, hs, _⟩ := hspec
  obtain ⟨hfix, _, hr, hrd⟩ := hs hsched
  have hi : i.val < NT.val := by
    have := (List.getElem?_eq_some_iff.mp hti).1
    omega
  have hj0 := (hplain ti (List.mem_of_getElem? hti)).1
  have hz : (0#u64).val = 0 := by rfl
  simp only [hz, Nat.add_zero] at hfix
  rw [hj0, Nat.zero_add] at hr
  rw [hr]
  have hRpos : 0 < v.w.val := by
    unfold stepF at hfix; omega
  have hpi : prioOf tasks.val i.val = ti.priority.val := by simp [prioOf, hti]
  have hCi : (paramsOf tasks.val).C i.val = ti.bound.val := by simp [paramsOf, hti]
  have hTi : (paramsOf tasks.val).T i.val = ti.period.val := by simp [paramsOf, hti]
  apply stage1 hNT (fun k hk => (hprio k hk).1) hi hRpos _ _ hreach hpos
  · intro k hk
    obtain ⟨hkn, _⟩ := mem_lvl.mp hk
    have hlt : k < tasks.val.length := by omega
    have hget : tasks.val[k]? = some tasks.val[k] := List.getElem?_eq_getElem hlt
    simp only [paramsOf, hget]
    exact hper _ (List.getElem_mem hlt)
  · have hsum := interf_spec tasks.val ti i.val hplain v.w.val tasks.val.length (Nat.le_refl _)
    unfold stepF at hfix
    rw [hsum, hlen] at hfix
    unfold lvl
    rw [Finset.sum_filter]
    have hsplit : ∀ k ∈ Finset.range NT.val,
        (if prio cfg i.val ≤ prio cfg k
          then (paramsOf tasks.val).C k * ceilDiv v.w.val ((paramsOf tasks.val).T k) else 0)
        = (if k = i.val then (paramsOf tasks.val).C i.val * ceilDiv v.w.val ((paramsOf tasks.val).T i.val) else 0)
          + (if k ≠ i.val ∧ ti.priority.val ≤ prioOf tasks.val k
              then (paramsOf tasks.val).C k * ceilDiv v.w.val ((paramsOf tasks.val).T k) else 0) := by
      intro k hk
      have hkn : k < NT.val := Finset.mem_range.mp hk
      rw [(hprio k hkn).2, (hprio i.val hi).2, hpi]
      by_cases hki : k = i.val
      · subst hki; simp [hpi]
      · simp [hki]
    rw [Finset.sum_congr rfl hsplit, Finset.sum_add_distrib, Finset.sum_ite_eq' (Finset.range NT.val) i.val]
    simp only [Finset.mem_range, hi, ite_true]
    rw [hCi, hTi, ceilDiv_one _ _ hRpos (by omega), Nat.mul_one]
    omega

end P18
