import Sched.Stage
import Rta.Chain
open Aeneas Aeneas.Std Result

namespace P18
open rsk_logic
open rsk_analyzer (ceilDiv interf stepF specTerms termJobs termIntake)

variable {NT NR NL NP : Usize}

/-- The run's timing parameters: per Task the period or MIT and the jitter
of the Analyzer's inputs, and the given bounds on a Job's execution (`C`),
on the Kernel's work for an arrival (`I`) and on a Critical_Section (`L`). -/
def paramsOf (tasks : List rsk_analyzer.terms.Interferer) (C I : Nat → Nat)
    (L : Nat → Nat → Nat) : Params :=
  { T := fun k => match tasks[k]? with
      | some t => t.period.val
      | none => 0,
    J := fun k => match tasks[k]? with
      | some t => t.jitter.val
      | none => 0,
    C := C, I := I, L := L }

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

/-- Task `k`'s term in Task `i`'s recurrence: its Jobs for a Task of equal
or higher Priority, the Kernel's work for its arrivals for a lower one. -/
def tfun (tasks : List rsk_analyzer.terms.Interferer) (ti : rsk_analyzer.terms.Interferer)
    (i w k : Nat) : Nat :=
  match tasks[k]? with
  | some t =>
    if k = i then 0
    else if ti.priority.val ≤ t.priority.val
      then ceilDiv (w + t.jitter.val) t.period.val * t.bound.val
      else ceilDiv w t.period.val * t.intake.val
  | none => 0

/-- The Analyzer's interference terms as a sum over the Tasks. -/
theorem interf_spec (tasks : List rsk_analyzer.terms.Interferer)
    (ti : rsk_analyzer.terms.Interferer) (i : Nat) (w : Nat) :
    ∀ n, n ≤ tasks.length →
    interf (specTerms tasks ti i n) w = ∑ k ∈ Finset.range n, tfun tasks ti i w k := by
  intro n
  induction n with
  | zero => intro _; simp [specTerms, interf]
  | succ n ih =>
    intro hn
    have hlt : n < tasks.length := by omega
    have hget : tasks[n]? = some tasks[n] := List.getElem?_eq_getElem hlt
    rw [Finset.sum_range_succ, ← ih (by omega)]
    simp only [specTerms, hget, tfun]
    by_cases h1 : n = i
    · simp [h1]
    · by_cases h2 : (tasks[n]).priority.val ≥ ti.priority.val
      · simp only [h1, ite_false, h2, ite_true]
        rw [interf_append]
        simp only [termJobs]
      · simp only [h1, ite_false, h2]
        by_cases h3 : (tasks[n]).intake.val > 0
        · simp only [h3, ite_true]
          rw [interf_append]
          simp only [termIntake]
          have hz : (0#u64).val = 0 := by rfl
          rw [hz, Nat.add_zero]
        · simp only [h3, ite_false]
          have : (tasks[n]).intake.val = 0 := by omega
          rw [this, Nat.mul_zero, Nat.add_zero]

theorem ceilDiv_one (R T : Nat) (hR : 0 < R) (h : R ≤ T) : ceilDiv R T = 1 := by
  unfold ceilDiv
  exact Nat.div_eq_of_lt_le (by omega) (by omega)

/-- **Property 18 over the run** (interference, SRP blocking, the
overhead term, release jitter and intake cost). `cfg` is the Kernel's
configuration and `tasks` the Analyzer's inputs for the same Tasks, with
the same Priorities. If `response_of`, as compiled, reports Task `i`
schedulable with response time `v.r` for the execution bound `c`, the
blocking term `b` and the overhead term `o`, then in every run of the
Kernel's scheduler

- whose arrivals of each Task `k` are at least `tasks[k].period` apart and
  each released at most `tasks[k].jitter` later,
- in which the Kernel's work for an arrival of Task `k` is at most `I k`
  and a Job of Task `k` executes for at most `C k`, where `I k + C k` is at
  most `tasks[k].bound` for another Task of equal or higher Priority,
  `I k` at most `tasks[k].intake` for a Task of lower Priority, and
  `I i + C i` at most `c + o` for Task `i` itself, and
- whose Critical_Sections execute for at most `L` of their Task and
  Resource, `L` being at most `b` for a Task of lower Priority than `i`
  on a Resource whose Ceiling is at least `i`'s Priority,

in every state in which a released Job of Task `i` has unfinished work,
the nominal instant `a` of its arrival has `now < a + v.r`: every Job of
Task `i` completes within `v.r` of its arrival. -/
theorem property18 {cfg : logic.Config NT NR NP}
    (tasks : alloc.vec.Vec rsk_analyzer.terms.Interferer) (i : Usize)
    (ti : rsk_analyzer.terms.Interferer) (hti : tasks.val[i.val]? = some ti)
    (c b o deadline : U64) (C I : Nat → Nat) (L : Nat → Nat → Nat)
    (v : rsk_analyzer.terms.Verdict) (own : alloc.vec.Vec rsk_analyzer.terms.TermOf)
    (hres : rsk_analyzer.terms.response_of tasks i c b o deadline = ok (v, own))
    (hsched : v.schedulable = true)
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hprio : ∀ k, k < NT.val → 0 < prio cfg k ∧ prio cfg k = prioOf tasks.val k)
    (hper : ∀ t ∈ tasks.val, 0 < t.period.val)
    (hc : 0 < c.val) (hdl : deadline.val ≤ ti.period.val)
    (hOwn : I i.val + C i.val ≤ c.val + o.val)
    (hE : ∀ k t, tasks.val[k]? = some t → k ≠ i.val → ti.priority.val ≤ t.priority.val →
      I k + C k ≤ t.bound.val)
    (hI : ∀ k t, tasks.val[k]? = some t → t.priority.val < ti.priority.val → I k ≤ t.intake.val)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i.val → prio cfg i.val ≤ ceil cfg r →
      L k r ≤ b.val)
    {st : St NT NR NL} (hreach : Reach cfg (paramsOf tasks.val C I L) st)
    (hpos : 0 < st.pend i.val + st.run i.val) :
    ∃ a, st.last i.val = some a ∧ st.now < a + v.r.val := by
  have hspec := rsk_analyzer.response_of_spec tasks i ti hti c b o deadline hper
  rw [hres] at hspec
  simp only [WP.spec_ok] at hspec
  obtain ⟨_, _, hs, _⟩ := hspec
  obtain ⟨hfix, _, hr, hrd⟩ := hs hsched
  have hi : i.val < NT.val := by
    have := (List.getElem?_eq_some_iff.mp hti).1
    omega
  have hRpos : 0 < v.w.val := by
    unfold stepF at hfix; omega
  have hpi : prioOf tasks.val i.val = ti.priority.val := by simp [prioOf, hti]
  have hJi : (paramsOf tasks.val C I L).J i.val = ti.jitter.val := by simp [paramsOf, hti]
  have hTi : (paramsOf tasks.val C I L).T i.val = ti.period.val := by simp [paramsOf, hti]
  have hmain := stage3 (P := paramsOf tasks.val C I L) (R := v.w.val) (B := b.val) hNT
    (fun k hk => (hprio k hk).1) hi hRpos ?_ hB ?_ hreach hpos
  · obtain ⟨a, ha, hlt⟩ := hmain
    rw [hJi] at hlt
    exact ⟨a, ha, by omega⟩
  · intro k hkn
    have hlt : k < tasks.val.length := by omega
    have hget : tasks.val[k]? = some tasks.val[k] := List.getElem?_eq_getElem hlt
    simp only [paramsOf, hget]
    exact hper _ (List.getElem_mem hlt)
  · have hsum := interf_spec tasks.val ti i.val v.w.val tasks.val.length (Nat.le_refl _)
    unfold stepF at hfix
    rw [hsum, hlen] at hfix
    have hterm : ∀ k ∈ Finset.range NT.val,
        costOf cfg (paramsOf tasks.val C I L) i.val k
          * ceilDiv (v.w.val + jeff cfg (paramsOf tasks.val C I L) i.val k)
              ((paramsOf tasks.val C I L).T k)
        ≤ (if k = i.val then c.val + o.val else 0) + tfun tasks.val ti i.val v.w.val k := by
      intro k hk
      have hkn : k < NT.val := Finset.mem_range.mp hk
      have hlt : k < tasks.val.length := by omega
      have hget : tasks.val[k]? = some tasks.val[k] := List.getElem?_eq_getElem hlt
      have hpk : prio cfg k = (tasks.val[k]).priority.val := by
        rw [(hprio k hkn).2]; simp [prioOf, hget]
      have hpi' : prio cfg i.val = ti.priority.val := by rw [(hprio i.val hi).2, hpi]
      have hTk : (paramsOf tasks.val C I L).T k = (tasks.val[k]).period.val := by
        simp [paramsOf, hget]
      have hJk : (paramsOf tasks.val C I L).J k = (tasks.val[k]).jitter.val := by
        simp [paramsOf, hget]
      unfold costOf jeff tfun
      rw [hpk, hpi', hTk, hJk]
      simp only [hget]
      by_cases hki : k = i.val
      · subst hki
        have hti' : tasks.val[i.val] = ti := by
          have := hget.symm.trans hti
          exact Option.some.inj this
        rw [hti']
        simp only [Nat.le_refl, ite_true]
        rw [ceilDiv_one _ _ (by omega) (by omega), Nat.mul_one]
        exact hOwn
      · simp only [hki, ite_false, Nat.zero_add]
        by_cases hhi : ti.priority.val ≤ (tasks.val[k]).priority.val
        · simp only [hhi, ite_true]
          rw [Nat.mul_comm]
          exact Nat.mul_le_mul_left _ (hE k _ hget hki hhi)
        · simp only [hhi, ite_false, Nat.add_zero]
          rw [Nat.mul_comm]
          exact Nat.mul_le_mul_left _ (hI k _ hget (by omega))
    have hs2 := Finset.sum_le_sum hterm
    rw [Finset.sum_add_distrib, Finset.sum_ite_eq' (Finset.range NT.val) i.val] at hs2
    simp only [Finset.mem_range, hi, ite_true] at hs2
    omega

/-- **Stage 2** (interference and SRP blocking): `property18` with no
overhead term and a run in which the Kernel's work for an arrival takes
no time. -/
theorem property18_stage2 {cfg : logic.Config NT NR NP}
    (tasks : alloc.vec.Vec rsk_analyzer.terms.Interferer) (i : Usize)
    (ti : rsk_analyzer.terms.Interferer) (hti : tasks.val[i.val]? = some ti)
    (c b deadline : U64) (C : Nat → Nat) (L : Nat → Nat → Nat)
    (v : rsk_analyzer.terms.Verdict) (own : alloc.vec.Vec rsk_analyzer.terms.TermOf)
    (hres : rsk_analyzer.terms.response_of tasks i c b 0#u64 deadline = ok (v, own))
    (hsched : v.schedulable = true)
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hprio : ∀ k, k < NT.val → 0 < prio cfg k ∧ prio cfg k = prioOf tasks.val k)
    (hper : ∀ t ∈ tasks.val, 0 < t.period.val)
    (hc : 0 < c.val ∧ C i.val ≤ c.val) (hdl : deadline.val ≤ ti.period.val)
    (hE : ∀ k t, tasks.val[k]? = some t → k ≠ i.val → ti.priority.val ≤ t.priority.val →
      C k ≤ t.bound.val)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i.val → prio cfg i.val ≤ ceil cfg r →
      L k r ≤ b.val)
    {st : St NT NR NL} (hreach : Reach cfg (paramsOf tasks.val C (fun _ => 0) L) st)
    (hpos : 0 < st.pend i.val + st.run i.val) :
    ∃ a, st.last i.val = some a ∧ st.now < a + v.r.val :=
  property18 tasks i ti hti c b 0#u64 deadline C (fun _ => 0) L v own hres hsched hlen hNT hprio
    hper hc.1 hdl (by have h0 : (0#u64).val = 0 := rfl; rw [h0]; have := hc.2; omega)
    (fun k t a b' c' => by have := hE k t a b' c'; omega) (fun _ _ _ _ => Nat.zero_le _) hB
    hreach hpos

/-- **Stage 1** (interference only): stage 2 with no blocking term, for
runs in which no Critical_Section of a lower Task on a Resource of
Ceiling at least Task `i`'s Priority executes. -/
theorem property18_stage1 {cfg : logic.Config NT NR NP}
    (tasks : alloc.vec.Vec rsk_analyzer.terms.Interferer) (i : Usize)
    (ti : rsk_analyzer.terms.Interferer) (hti : tasks.val[i.val]? = some ti)
    (c deadline : U64) (C : Nat → Nat) (L : Nat → Nat → Nat)
    (v : rsk_analyzer.terms.Verdict) (own : alloc.vec.Vec rsk_analyzer.terms.TermOf)
    (hres : rsk_analyzer.terms.response_of tasks i c 0#u64 0#u64 deadline = ok (v, own))
    (hsched : v.schedulable = true)
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hprio : ∀ k, k < NT.val → 0 < prio cfg k ∧ prio cfg k = prioOf tasks.val k)
    (hper : ∀ t ∈ tasks.val, 0 < t.period.val)
    (hc : 0 < c.val ∧ C i.val ≤ c.val) (hdl : deadline.val ≤ ti.period.val)
    (hE : ∀ k t, tasks.val[k]? = some t → k ≠ i.val → ti.priority.val ≤ t.priority.val →
      C k ≤ t.bound.val)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i.val → prio cfg i.val ≤ ceil cfg r →
      L k r = 0)
    {st : St NT NR NL} (hreach : Reach cfg (paramsOf tasks.val C (fun _ => 0) L) st)
    (hpos : 0 < st.pend i.val + st.run i.val) :
    ∃ a, st.last i.val = some a ∧ st.now < a + v.r.val :=
  property18_stage2 tasks i ti hti c 0#u64 deadline C L v own hres hsched hlen hNT hprio hper hc
    hdl hE (fun k r a b c => by rw [hB k r a b c]; exact Nat.zero_le _) hreach hpos

end P18
