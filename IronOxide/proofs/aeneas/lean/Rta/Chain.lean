/-
  The Analyzer's verdict for one Task, end to end, on the code as compiled
  (Verification_Plan 7.2, 2026-11-14): `terms::terms_for` returns the terms
  of the Task's recurrence, and `terms::response_of` (the terms, the fixed
  point from `C + B + O`, the verdict) returns what R29.1 and R29.5 to R29.7
  say. Handwritten; `Rta/Rta.lean` is the generated translation.
-/
import Aeneas
import Rta.Rta
import Rta.Proofs
open Aeneas Aeneas.Std Result

namespace rsk_analyzer

/-- The interference of a Task's Jobs: `⌈(w + J_j) / T_j⌉ · E_j` (R29.1). -/
def termJobs (t : terms.Interferer) : rta.Term :=
  { jitter := t.jitter, period := t.period, bound := t.bound }

/-- The Kernel-level work of a lower Task's arrivals:
`⌈(w + J_j) / T_j⌉ · intake` (R19.2; the jitter since 2026-11-14). -/
def termIntake (t : terms.Interferer) : rta.Term :=
  { jitter := t.jitter, period := t.period, bound := t.intake }

/-- The other timed events of a lower Task's Jobs:
`⌈(w + J_j) / T_j⌉ · events` (R19.2, 2026-11-14). -/
def termEvents (t : terms.Interferer) : rta.Term :=
  { jitter := t.jitter, period := t.period, bound := t.events }

/-- The terms a lower-Priority Task adds: its arrivals' and its other timed
events', each when it has a cost. -/
def pushLower (prev : List rta.Term) (t : terms.Interferer) : List rta.Term :=
  let a := if t.intake.val > 0 then prev ++ [termIntake t] else prev
  if t.events.val > 0 then a ++ [termEvents t] else a

/-- The terms of the recurrence of the Task `ti` at index `i`, over the
first `n` Tasks: every other Task of equal or higher Priority by its Jobs,
every lower one by its Kernel-level work. -/
def specTerms (tasks : List terms.Interferer) (ti : terms.Interferer) (i : Nat) :
    Nat → List rta.Term
  | 0 => []
  | n + 1 =>
    let prev := specTerms tasks ti i n
    match tasks[n]? with
    | none => prev
    | some t =>
      if n = i then prev
      else if t.priority.val ≥ ti.priority.val then prev ++ [termJobs t]
      else pushLower prev t

/-- All the terms of Task `i`'s recurrence: those of the other Tasks, then
the housekeeping event's. -/
def specAll (tasks : List terms.Interferer) (ti : terms.Interferer) (i : Nat) (hk : rta.Term) :
    List rta.Term :=
  specTerms tasks ti i tasks.length ++ [hk]

theorem terms_for_body_spec (tasks : alloc.vec.Vec terms.Interferer) (i : Usize)
    (out : alloc.vec.Vec terms.TermOf) (ti : terms.Interferer) (j : Usize)
    (hsz : 2 * tasks.val.length + 1 ≤ Usize.max)
    (hj : j.val ≤ tasks.length) (hlen : out.length ≤ 2 * j.val)
    (hout : out.val.map (·.term) = specTerms tasks.val ti i.val j.val)
    (hper : ∀ t ∈ tasks.val, 0 < t.period.val)
    (hop : ∀ t ∈ out.val, 0 < t.term.period.val) :
    terms.terms_for_loop.body tasks i out ti j ⦃ r =>
      match r with
      | .done o => o.length ≤ 2 * tasks.length
          ∧ o.val.map (·.term) = specTerms tasks.val ti i.val tasks.length
          ∧ ∀ t ∈ o.val, 0 < t.term.period.val
      | .cont (o, ti', j') => ti' = ti ∧ j'.val = j.val + 1 ∧ j'.val ≤ tasks.length
          ∧ o.length ≤ 2 * j'.val
          ∧ o.val.map (·.term) = specTerms tasks.val ti i.val j'.val
          ∧ (∀ t ∈ o.val, 0 < t.term.period.val) ⦄ := by
  unfold terms.terms_for_loop.body
  simp only []
  split
  · rename_i hlt
    have hlt' : j.val < tasks.length := by scalar_tac
    step as ⟨ t, ht ⟩
    have hget : tasks.val[j.val]? = some t := by
      rw [ht]; exact List.getElem?_eq_getElem hlt'
    have htper : 0 < t.period.val := by
      apply hper; rw [ht]; exact List.getElem_mem _
    have hnext : specTerms tasks.val ti i.val (j.val + 1) =
        (if j.val = i.val then specTerms tasks.val ti i.val j.val
         else if t.priority.val ≥ ti.priority.val then specTerms tasks.val ti i.val j.val ++ [termJobs t]
         else pushLower (specTerms tasks.val ti i.val j.val) t) := by
      simp [specTerms, hget]
    by_cases hji : j = i
    · have hji' : j.val = i.val := by rw [hji]
      simp [hji]
      step as ⟨ j1, hj1 ⟩
      have hj1' : j1.val = j.val + 1 := by omega
      refine ⟨hj1, by scalar_tac, by scalar_tac, ?_, hop⟩
      rw [hj1', hnext]; simp [hji', hout]
    · have hji' : j.val ≠ i.val := by
        intro h; exact hji (UScalar.eq_of_val_eq h)
      simp [hji]
      by_cases hp : ti.priority.val ≤ t.priority.val
      · simp [hp]
        step as ⟨ o1, ho1 ⟩
        step as ⟨ j1, hj1 ⟩
        refine ⟨hj1, by scalar_tac, by simp [ho1]; scalar_tac, ?_, ?_⟩
        · rw [hj1, hnext, ho1]; simp [hji', hp, hout, termJobs]
        · intro x hx
          rw [ho1] at hx
          simp at hx
          rcases hx with h | h
          · exact hop x h
          · subst h; simpa using htper
      · simp [hp]
        by_cases hin : 0 < t.intake.val <;> by_cases hev : 0 < t.events.val
        · simp [hin, hev]
          step as ⟨ o1, ho1 ⟩
          step as ⟨ o2, ho2 ⟩
          · simp [ho1]; scalar_tac
          step as ⟨ j1, hj1 ⟩
          refine ⟨hj1, by scalar_tac, by simp [ho2, ho1]; scalar_tac, ?_, ?_⟩
          · rw [hj1, hnext, ho2, ho1]; simp [hji', hp, hin, hev, hout, termIntake, termEvents, pushLower]
          · intro x hx
            rw [ho2, ho1] at hx
            simp at hx
            rcases hx with h | h | h
            · exact hop x h
            · subst h; simpa using htper
            · subst h; simpa using htper
        · simp [hin, hev]
          step as ⟨ o1, ho1 ⟩
          step as ⟨ j1, hj1 ⟩
          refine ⟨hj1, by scalar_tac, by simp [ho1]; scalar_tac, ?_, ?_⟩
          · rw [hj1, hnext, ho1]; simp [hji', hp, hin, hev, hout, termIntake, pushLower]
          · intro x hx
            rw [ho1] at hx
            simp at hx
            rcases hx with h | h
            · exact hop x h
            · subst h; simpa using htper
        · simp [hin, hev]
          step as ⟨ o1, ho1 ⟩
          step as ⟨ j1, hj1 ⟩
          refine ⟨hj1, by scalar_tac, by simp [ho1]; scalar_tac, ?_, ?_⟩
          · rw [hj1, hnext, ho1]; simp [hji', hp, hin, hev, hout, termEvents, pushLower]
          · intro x hx
            rw [ho1] at hx
            simp at hx
            rcases hx with h | h
            · exact hop x h
            · subst h; simpa using htper
        · simp [hin, hev]
          step as ⟨ j1, hj1 ⟩
          refine ⟨hj1, by scalar_tac, by scalar_tac, ?_, hop⟩
          rw [hj1, hnext]; simp [hji', hp, hin, hev, hout, pushLower]
  · rename_i hge
    have heq : j.val = tasks.val.length := by scalar_tac
    simp
    rw [← heq]
    exact ⟨by scalar_tac, hout, hop⟩

/-- `terms_for`, on the code as compiled: the plain terms it returns are the
terms of Task `i`'s recurrence, the housekeeping term last, and every
period is positive. (`hsz`: twice the Task count fits a `usize`, which the
list the function builds needs in Aeneas's model of `Vec::push`.) -/
theorem terms_for_spec (tasks : alloc.vec.Vec terms.Interferer) (i : Usize)
    (ti : terms.Interferer) (hti : tasks.val[i.val]? = some ti)
    (hk : rta.Term) (hkp : 0 < hk.period.val)
    (hsz : 2 * tasks.val.length + 1 ≤ Usize.max)
    (hper : ∀ t ∈ tasks.val, 0 < t.period.val) :
    terms.terms_for tasks i hk ⦃ out =>
      out.val.map (·.term) = specAll tasks.val ti i.val hk
        ∧ ∀ t ∈ out.val, 0 < t.term.period.val ⦄ := by
  have hi : i.val < tasks.val.length := by
    rcases List.getElem?_eq_some_iff.mp hti with ⟨h, _⟩; exact h
  unfold terms.terms_for
  step as ⟨ ti0, hti0 ⟩
  have hti1 : ti0 = ti := by
    have := List.getElem?_eq_getElem hi
    rw [this] at hti; simp at hti; rw [hti0]; exact hti
  subst hti1
  have hloop : terms.terms_for_loop tasks i (alloc.vec.Vec.new terms.TermOf) ti0 0#usize ⦃ o =>
      o.length ≤ 2 * tasks.length
      ∧ o.val.map (·.term) = specTerms tasks.val ti0 i.val tasks.val.length
      ∧ ∀ t ∈ o.val, 0 < t.term.period.val ⦄ := by
    unfold terms.terms_for_loop
    apply loop.spec_decr_nat
      (fun (s : alloc.vec.Vec terms.TermOf × terms.Interferer × Usize) => tasks.val.length - s.2.2.val)
      (fun s => s.2.1 = ti0 ∧ s.2.2.val ≤ tasks.val.length ∧ s.1.val.length ≤ 2 * s.2.2.val
        ∧ s.1.val.map (·.term) = specTerms tasks.val ti0 i.val s.2.2.val
        ∧ ∀ t ∈ s.1.val, 0 < t.term.period.val)
    · intro s hs
      obtain ⟨out, ti', j⟩ := s
      obtain ⟨h1, h2, h3, h4, h5⟩ := hs
      simp only [] at h1 h2 h3 h4 h5
      subst h1
      apply WP.spec_mono (terms_for_body_spec tasks i out ti' j hsz (by scalar_tac) (by scalar_tac) h4 hper h5)
      intro r hr
      cases r with
      | done o => simpa using hr
      | cont s' =>
        obtain ⟨o, ti'', j'⟩ := s'
        simp only [] at hr ⊢
        obtain ⟨e1, e2, e3, e4, e5, e6⟩ := hr
        refine ⟨⟨e1, by scalar_tac, by scalar_tac, e5, e6⟩, ?_⟩
        scalar_tac
    · simp [specTerms]
  step with hloop as ⟨ o, ho1, ho2, ho3 ⟩
  step as ⟨ o2, ho2' ⟩
  constructor
  · rw [ho2']; simp [specAll, ho2]
  · intro x hx
    rw [ho2'] at hx
    simp at hx
    rcases hx with h | h
    · exact ho3 x h
    · subst h; simpa using hkp

theorem add_spec (a b : U64) :
    terms.add a b ⦃ r => ∀ v, r = some v → v.val = a.val + b.val ⦄ := by
  unfold terms.add
  step as ⟨ i, hi ⟩
  · sc
  split
  · simp
  · step as ⟨ i1, hi1 ⟩
    · sc
    intro v hv
    simp at hv
    subst hv
    exact hi1

theorem plain_body_spec (ts : alloc.vec.Vec terms.TermOf) (pl : alloc.vec.Vec rta.Term) (k : Usize)
    (hk : k.val ≤ ts.val.length) (hpl : pl.val = (ts.val.take k.val).map (·.term)) :
    terms.response_of_loop.body ts pl k ⦃ r =>
      match r with
      | .done p => p.val = ts.val.map (·.term)
      | .cont (p, k') => k'.val = k.val + 1 ∧ k'.val ≤ ts.val.length
          ∧ p.val = (ts.val.take k'.val).map (·.term) ⦄ := by
  unfold terms.response_of_loop.body
  simp only []
  split
  · rename_i hlt
    have hlt' : k.val < ts.val.length := by scalar_tac
    step as ⟨ t, ht ⟩
    step as ⟨ p1, hp1 ⟩
    · have : pl.val.length = k.val := by rw [hpl]; simp; omega
      scalar_tac
    step as ⟨ k1, hk1 ⟩
    refine ⟨hk1, by scalar_tac, ?_⟩
    rw [hp1, hpl, hk1, ht, List.take_succ_eq_append_getElem hlt', List.map_append]
    rfl
  · rename_i hge
    have heq : k.val = ts.val.length := by scalar_tac
    simp
    rw [hpl, heq]; simp

/-- The analysis of one Task, end to end, on the code as compiled
(`terms::response_of`: the terms, the fixed point from `C + B + O`, the
verdict). For every Task set with positive periods and every Task `i` of
it the function returns, and:

- the terms it used are the terms of Task `i`'s recurrence (`specAll`);
- a Task reported schedulable has `R = J + w` for the least fixed point `w`
  of `w = (C + B + O) + Σ ⌈(w + J_j) / T_j⌉ · E_j`, and `R ≤ D`;
- a Task reported not schedulable without an overflow has no fixed point
  `x` with `J + x ≤ D`;
- an overflow is never reported schedulable. -/
theorem response_of_spec (tasks : alloc.vec.Vec terms.Interferer) (i : Usize)
    (ti : terms.Interferer) (hti : tasks.val[i.val]? = some ti)
    (c b o deadline : U64) (hk : rta.Term) (hkp : 0 < hk.period.val)
    (hsz : 2 * tasks.val.length + 1 ≤ Usize.max)
    (hper : ∀ t ∈ tasks.val, 0 < t.period.val) :
    terms.response_of tasks i c b o deadline hk ⦃ r =>
      r.2.val.map (·.term) = specAll tasks.val ti i.val hk
      ∧ (r.1.overflow = true → r.1.schedulable = false)
      ∧ (r.1.schedulable = true →
          stepF (c.val + b.val + o.val) (specAll tasks.val ti i.val hk) r.1.w.val = r.1.w.val
          ∧ (∀ x, stepF (c.val + b.val + o.val) (specAll tasks.val ti i.val hk) x = x → r.1.w.val ≤ x)
          ∧ r.1.r.val = ti.jitter.val + r.1.w.val
          ∧ r.1.r.val ≤ deadline.val)
      ∧ (r.1.schedulable = false → r.1.overflow = false →
          ∀ x, stepF (c.val + b.val + o.val) (specAll tasks.val ti i.val hk) x = x →
            deadline.val < ti.jitter.val + x) ⦄ := by
  have hi : i.val < tasks.val.length := by
    rcases List.getElem?_eq_some_iff.mp hti with ⟨h, _⟩; exact h
  unfold terms.response_of
  step with terms_for_spec tasks i ti hti hk hkp hsz hper as ⟨ own, hown, hownp ⟩
  step as ⟨ i1, hi1 ⟩
  have hi1' : i1 = ti := by
    have := List.getElem?_eq_getElem hi
    rw [this] at hti; simp at hti; rw [hi1]; exact hti
  subst hi1'
  have hloop : terms.response_of_loop own (alloc.vec.Vec.new rta.Term) 0#usize
      ⦃ p => p.val = own.val.map (·.term) ⦄ := by
    unfold terms.response_of_loop
    apply loop.spec_decr_nat
      (fun (s : alloc.vec.Vec rta.Term × Usize) => own.val.length - s.2.val)
      (fun s => s.2.val ≤ own.val.length ∧ s.1.val = (own.val.take s.2.val).map (·.term))
    · intro s hs
      obtain ⟨pl, k⟩ := s
      obtain ⟨h1, h2⟩ := hs
      simp only [] at h1 h2
      apply WP.spec_mono (plain_body_spec own pl k h1 h2)
      intro r hr
      cases r with
      | done p => simpa using hr
      | cont s' =>
        obtain ⟨p, k'⟩ := s'
        simp only [] at hr ⊢
        obtain ⟨e1, e2, e3⟩ := hr
        exact ⟨⟨e2, e3⟩, by omega⟩
    · simp
  step with hloop as ⟨ pl, hpl ⟩
  have htok : termsOk pl.val := by
    intro t ht
    rw [hpl] at ht
    simp at ht
    obtain ⟨x, hx, rfl⟩ := ht
    exact hownp x hx
  have hts : pl.val = specAll tasks.val i1 i.val hk := by rw [hpl, hown]
  step with add_spec as ⟨ o1, ho1 ⟩
  cases o1 with
  | none => simp [hown]
  | some x =>
    simp only []
    have hx := ho1 x rfl
    step with add_spec as ⟨ o2, ho2 ⟩
    cases o2 with
    | none => simp [hown]
    | some x1 =>
      simp only []
      have hx1 := ho2 x1 rfl
      step with fixpoint_spec x1 pl i1.jitter deadline htok as ⟨ o3, ho3 ⟩
      have hb : x1.val = c.val + b.val + o.val := by omega
      rw [hts, hb] at ho3
      cases o3 with
      | Fixpoint w =>
        simp only [post] at ho3
        obtain ⟨hf, hl, hd⟩ := ho3
        simp only []
        step as ⟨ i2, hi2 ⟩
        simp
        and_intros
        all_goals first | exact hown | exact hf | exact hl | omega
      | NotSchedulable w =>
        simp only [post] at ho3
        obtain ⟨hlate, hl⟩ := ho3
        simp only []
        step as ⟨ i2, hi2 ⟩
        · sc
        split
        · simp
          and_intros
          all_goals first | exact hown | (intro x hx; have := hl x hx; omega)
        · step as ⟨ r, hr ⟩
          · sc
          simp
          and_intros
          all_goals first | exact hown | (intro x hx; have := hl x hx; omega)
      | Overflow => simp [hown]

end rsk_analyzer
