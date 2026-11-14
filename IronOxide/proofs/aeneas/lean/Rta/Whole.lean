/-
  The whole numeric analysis on the code as compiled (Verification_Plan
  7.2, 2026-11-14): `terms::analyse_all`, from the Task set, its sites, the
  Kernel_Timing_Parameters and each Task's period and deadline to every
  Task's verdict. The derivation (`derive_all_spec`), the Tasks as each
  other's interferers, and each Task's recurrence (`response_of_spec`),
  joined. Handwritten; `Rta/Rta.lean` is the generated translation.
-/
import Aeneas
import Rta.Rta
import Rta.Proofs
import Rta.Chain
import Rta.Derive
open Aeneas Aeneas.Std Result

namespace rsk_analyzer

/-- `it` is Task `j` as an interferer of the others' recurrences: its
Priority, `J_rel`, its period (at least 1), `E_j` as R29.4 states it from
the Task set and its sites, and the intake cost of its Release_Source. -/
def interfererOk (tasks : List terms.TaskIn) (sites : List terms.Site) (p : terms.Params)
    (any_fpu : Bool) (windows : List terms.Window) (j : Nat) (it : terms.Interferer) : Prop :=
  ∃ t w, tasks[j]? = some t ∧ windows[j]? = some w
    ∧ it.priority = t.priority ∧ it.jitter = p.j_rel
    ∧ it.period.val = (if w.period.val = 0 then 1 else w.period.val)
    ∧ it.bound.val = specE t p (longestCs sites j sites.length)
        (specO t p (siteCount sites j sites.length) any_fpu)
    ∧ it.intake.val = (if t.periodic then 0 else p.irq.val)

def allInterferers (tasks : List terms.TaskIn) (sites : List terms.Site) (p : terms.Params)
    (any_fpu : Bool) (windows : List terms.Window) (its : List terms.Interferer) : Prop :=
  ∀ j it, its[j]? = some it → interfererOk tasks sites p any_fpu windows j it

theorem interferers_body_spec (tasks : alloc.vec.Vec terms.TaskIn) (sites : alloc.vec.Vec terms.Site)
    (p : terms.Params) (any_fpu : Bool) (windows : alloc.vec.Vec terms.Window)
    (derived : alloc.vec.Vec terms.Derived) (n : Usize)
    (its : alloc.vec.Vec terms.Interferer) (j : Usize)
    (hn : n.val = tasks.val.length) (hwl : windows.val.length = tasks.val.length)
    (hdl : derived.val.length = tasks.val.length)
    (hder : allDerived tasks.val sites.val p any_fpu derived.val)
    (hj : j.val ≤ n.val) (hlen : its.val.length = j.val)
    (hall : allInterferers tasks.val sites.val p any_fpu windows.val its.val) :
    terms.analyse_all_loop0.body tasks p windows derived n its j ⦃ r =>
      match r with
      | .done o => o.val.length = tasks.val.length
          ∧ allInterferers tasks.val sites.val p any_fpu windows.val o.val
      | .cont (its', j') => j'.val = j.val + 1 ∧ j'.val ≤ n.val ∧ its'.val.length = j'.val
          ∧ allInterferers tasks.val sites.val p any_fpu windows.val its'.val ⦄ := by
  unfold terms.analyse_all_loop0.body
  split
  · rename_i hlt
    have hlt' : j.val < tasks.val.length := by scalar_tac
    step as ⟨ t, ht ⟩
    step as ⟨ d, hd ⟩
    step as ⟨ w, hw ⟩
    have hgt : tasks.val[j.val]? = some t := by rw [ht]; exact List.getElem?_eq_getElem hlt'
    have hgd : derived.val[j.val]? = some d := by
      rw [hd]; exact List.getElem?_eq_getElem (by omega)
    have hgw : windows.val[j.val]? = some w := by
      rw [hw]; exact List.getElem?_eq_getElem (by omega)
    obtain ⟨d1, d2, d3, d4, d5, d6⟩ := hder j.val d t hgd hgt
    have hbound : d.e.val = specE t p (longestCs sites.val j.val sites.val.length)
        (specO t p (siteCount sites.val j.val sites.val.length) any_fpu) := by
      rw [d4, d3, d1, d2]
    have key : ∀ (it : terms.Interferer) (its1 : alloc.vec.Vec terms.Interferer),
        its1.val = its.val ++ [it] →
        it.priority = t.priority → it.jitter = d.j → it.bound = d.e →
        it.period.val = (if w.period.val = 0 then 1 else w.period.val) →
        it.intake.val = (if t.periodic then 0 else p.irq.val) →
        allInterferers tasks.val sites.val p any_fpu windows.val its1.val := by
      intro it its1 h1 hp hjit hb hper hin k x hx
      rw [h1] at hx
      by_cases hk : k < its.val.length
      · rw [List.getElem?_append_left hk] at hx
        exact hall k x hx
      · have hkl : k = its.val.length := by
          have := (List.getElem?_eq_some_iff.mp hx).1
          simp at this; omega
        subst hkl
        simp at hx
        subst hx
        rw [hlen]
        exact ⟨t, w, hgt, hgw, hp, by rw [hjit, d6], hper, by rw [hb, hbound], hin⟩
    by_cases hw0 : w.period = 0#u64 <;> cases hper : t.periodic
    all_goals simp [hw0, hper]
    all_goals step as ⟨ its1, hits1 ⟩
    all_goals step as ⟨ j1, hj1 ⟩
    all_goals refine ⟨hj1, by scalar_tac, by rw [hits1]; simp; omega, ?_⟩
    all_goals apply key _ its1 hits1 rfl rfl rfl
    all_goals simp_all
  · rename_i hge
    have heq : j.val = tasks.val.length := by scalar_tac
    simp
    exact ⟨by omega, hall⟩

/-- The verdict `v` for the recurrence from `base` over `ts`, with jitter
`J` and deadline `D` (R29.5 to R29.7). -/
def verdictOk (base : Nat) (ts : List rta.Term) (J D : Nat) (v : terms.Verdict) : Prop :=
  (v.overflow = true → v.schedulable = false)
  ∧ (v.schedulable = true →
      stepF base ts v.w.val = v.w.val
      ∧ (∀ x, stepF base ts x = x → v.w.val ≤ x)
      ∧ v.r.val = J + v.w.val
      ∧ v.r.val ≤ D)
  ∧ (v.schedulable = false → v.overflow = false → ∀ x, stepF base ts x = x → D < J + x)

/-- Every Task's verdict and terms are those of its recurrence: from
`C_i + B_i + O_i`, over the terms of the interferers `its`. -/
def allVerdicts (tasks : List terms.TaskIn) (sites : List terms.Site) (p : terms.Params)
    (any_fpu : Bool) (windows : List terms.Window) (its : List terms.Interferer)
    (vs : List terms.Verdict) (tss : List (alloc.vec.Vec terms.TermOf)) : Prop :=
  ∀ i v t w it ts, vs[i]? = some v → tss[i]? = some ts → tasks[i]? = some t
    → windows[i]? = some w → its[i]? = some it →
    ts.val.map (·.term) = specTerms its it i its.length
    ∧ verdictOk
        (t.budget.val + maxCandidate tasks sites t p sites.length
          + specO t p (siteCount sites i sites.length) any_fpu)
        (specTerms its it i its.length) p.j_rel.val w.deadline.val v

theorem verdicts_body_spec (tasks : alloc.vec.Vec terms.TaskIn) (sites : alloc.vec.Vec terms.Site)
    (p : terms.Params) (any_fpu : Bool) (windows : alloc.vec.Vec terms.Window)
    (derived : alloc.vec.Vec terms.Derived) (n : Usize) (its : alloc.vec.Vec terms.Interferer)
    (vs : alloc.vec.Vec terms.Verdict) (tss : alloc.vec.Vec (alloc.vec.Vec terms.TermOf)) (i : Usize)
    (hn : n.val = tasks.val.length) (hwl : windows.val.length = tasks.val.length)
    (hdl : derived.val.length = tasks.val.length) (hil : its.val.length = tasks.val.length)
    (hder : allDerived tasks.val sites.val p any_fpu derived.val)
    (hits : allInterferers tasks.val sites.val p any_fpu windows.val its.val)
    (hi : i.val ≤ n.val) (hvl : vs.val.length = i.val) (htl : tss.val.length = i.val)
    (hall : allVerdicts tasks.val sites.val p any_fpu windows.val its.val vs.val tss.val) :
    terms.analyse_all_loop1.body tasks windows derived n its vs tss i ⦃ r =>
      match r with
      | .done (vs', tss') => vs'.val.length = tasks.val.length ∧ tss'.val.length = tasks.val.length
          ∧ allVerdicts tasks.val sites.val p any_fpu windows.val its.val vs'.val tss'.val
      | .cont (vs', tss', i') => i'.val = i.val + 1 ∧ i'.val ≤ n.val
          ∧ vs'.val.length = i'.val ∧ tss'.val.length = i'.val
          ∧ allVerdicts tasks.val sites.val p any_fpu windows.val its.val vs'.val tss'.val ⦄ := by
  unfold terms.analyse_all_loop1.body
  split
  · rename_i hlt
    have hlt' : i.val < tasks.val.length := by scalar_tac
    step as ⟨ t, ht ⟩
    step as ⟨ d, hd ⟩
    step as ⟨ w, hw ⟩
    have hgt : tasks.val[i.val]? = some t := by rw [ht]; exact List.getElem?_eq_getElem hlt'
    have hgd : derived.val[i.val]? = some d := by
      rw [hd]; exact List.getElem?_eq_getElem (by omega)
    have hgw : windows.val[i.val]? = some w := by
      rw [hw]; exact List.getElem?_eq_getElem (by omega)
    have hili : i.val < its.val.length := by omega
    have hgi : its.val[i.val]? = some its.val[i.val] := List.getElem?_eq_getElem hili
    obtain ⟨d1, d2, d3, d4, d5, d6⟩ := hder i.val d t hgd hgt
    obtain ⟨t', w', ht', hw', q1, q2, q3, q4, q5⟩ := hits i.val _ hgi
    have hper : ∀ x ∈ its.val, 0 < x.period.val := by
      intro x hx
      obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hx
      obtain ⟨_, w2, _, _, _, _, r3, _, _⟩ := hits k _ (List.getElem?_eq_getElem hk)
      rw [r3]; split <;> omega
    step with response_of_spec its i _ hgi t.budget d.b d.o w.deadline hper as ⟨ verdict, own, e1, e2, e3, e4 ⟩
    step as ⟨ vs1, hvs1 ⟩
    step as ⟨ tss1, htss1 ⟩
    step as ⟨ i1, hi1 ⟩
    refine ⟨hi1, by scalar_tac, by rw [hvs1]; simp; omega, by rw [htss1]; simp; omega, ?_⟩
    intro k v tk wk itk ts hv hts htk hwk hitk
    rw [hvs1] at hv
    rw [htss1] at hts
    by_cases hk : k < vs.val.length
    · rw [List.getElem?_append_left hk] at hv
      rw [List.getElem?_append_left (by omega)] at hts
      exact hall k v tk wk itk ts hv hts htk hwk hitk
    · have hkl : k = i.val := by
        have := (List.getElem?_eq_some_iff.mp hv).1
        simp at this; omega
      subst hkl
      rw [← hvl] at hv
      simp at hv
      rw [← htl] at hts
      simp at hts
      subst hv
      subst hts
      rw [hgt] at htk; simp at htk; subst htk
      rw [hgw] at hwk; simp at hwk; subst hwk
      rw [hgi] at hitk; simp at hitk; subst hitk
      have hbase : t.budget.val + d.b.val + d.o.val =
          t.budget.val + maxCandidate tasks.val sites.val t p sites.val.length
            + specO t p (siteCount sites.val i.val sites.val.length) any_fpu := by
        rw [d5, d3, d1]
      rw [hbase] at e3 e4
      rw [q2] at e3 e4
      exact ⟨e1, e2, e3, e4⟩
  · rename_i hge
    have heq : i.val = tasks.val.length := by scalar_tac
    simp
    exact ⟨by omega, by omega, hall⟩

/-- The whole numeric analysis, on the code as compiled. For every Task
set whose sites name Tasks of the set, with a period and a deadline per
Task, `analyse_all` returns (no panic, no overflow, every loop ends), and
when it returns results:

- there is one derived entry per Task, holding its terms (R29.2 to R29.4);
- there is a list of interferers, one per Task, each built from that
  Task's Priority, `J_rel`, period, `E_j` and intake cost;
- every Task's terms are those of its recurrence over those interferers,
  and its verdict is that of the recurrence from `C_i + B_i + O_i`: a Task
  reported schedulable has `R = J_rel + w` for the least fixed point `w`
  and `R ≤ D_i`; a Task reported not schedulable without an overflow has no
  fixed point within its deadline; an overflow is never reported
  schedulable. -/
theorem analyse_all_spec (tasks : alloc.vec.Vec terms.TaskIn) (sites : alloc.vec.Vec terms.Site)
    (p : terms.Params) (any_fpu : Bool) (windows : alloc.vec.Vec terms.Window)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length)
    (hwl : windows.val.length = tasks.val.length) :
    terms.analyse_all tasks sites p any_fpu windows ⦃ r =>
      ∀ ds vs tss, r = some (ds, vs, tss) →
        ds.val.length = tasks.val.length
        ∧ allDerived tasks.val sites.val p any_fpu ds.val
        ∧ vs.val.length = tasks.val.length ∧ tss.val.length = tasks.val.length
        ∧ ∃ its : List terms.Interferer, its.length = tasks.val.length
            ∧ allInterferers tasks.val sites.val p any_fpu windows.val its
            ∧ allVerdicts tasks.val sites.val p any_fpu windows.val its vs.val tss.val ⦄ := by
  unfold terms.analyse_all
  step with derive_all_spec tasks sites p any_fpu hsok as ⟨ o, ho ⟩
  cases o with
  | none => simp
  | some d =>
    simp only []
    obtain ⟨hdl, hder⟩ := ho d rfl
    have hn : (alloc.vec.Vec.len tasks).val = tasks.val.length := by scalar_tac
    have hloop0 : terms.analyse_all_loop0 tasks p windows d (alloc.vec.Vec.len tasks)
        (alloc.vec.Vec.new terms.Interferer) 0#usize ⦃ its =>
          its.val.length = tasks.val.length
            ∧ allInterferers tasks.val sites.val p any_fpu windows.val its.val ⦄ := by
      unfold terms.analyse_all_loop0
      apply loop.spec_decr_nat
        (fun (s : alloc.vec.Vec terms.Interferer × Usize) => tasks.val.length - s.2.val)
        (fun s => s.2.val ≤ tasks.val.length ∧ s.1.val.length = s.2.val
          ∧ allInterferers tasks.val sites.val p any_fpu windows.val s.1.val)
      · intro s hs
        obtain ⟨its, j⟩ := s
        obtain ⟨h1, h2, h3⟩ := hs
        simp only [] at h1 h2 h3
        apply WP.spec_mono (interferers_body_spec tasks sites p any_fpu windows d _ its j hn hwl hdl hder (by omega) h2 h3)
        intro r hr
        cases r with
        | done o => simpa using hr
        | cont s' =>
          obtain ⟨its', j'⟩ := s'
          simp only [] at hr ⊢
          obtain ⟨e1, e2, e3, e4⟩ := hr
          exact ⟨⟨by omega, e3, e4⟩, by omega⟩
      · simp [allInterferers]
    step with hloop0 as ⟨ its, hil, hits ⟩
    have hloop1 : terms.analyse_all_loop1 tasks windows d (alloc.vec.Vec.len tasks) its
        (alloc.vec.Vec.new terms.Verdict) (alloc.vec.Vec.new (alloc.vec.Vec terms.TermOf)) 0#usize
        ⦃ r => r.1.val.length = tasks.val.length ∧ r.2.val.length = tasks.val.length
          ∧ allVerdicts tasks.val sites.val p any_fpu windows.val its.val r.1.val r.2.val ⦄ := by
      unfold terms.analyse_all_loop1
      apply loop.spec_decr_nat
        (fun (s : alloc.vec.Vec terms.Verdict × alloc.vec.Vec (alloc.vec.Vec terms.TermOf) × Usize) =>
          tasks.val.length - s.2.2.val)
        (fun s => s.2.2.val ≤ tasks.val.length ∧ s.1.val.length = s.2.2.val
          ∧ s.2.1.val.length = s.2.2.val
          ∧ allVerdicts tasks.val sites.val p any_fpu windows.val its.val s.1.val s.2.1.val)
      · intro s hs
        obtain ⟨vs, tss, i⟩ := s
        obtain ⟨h1, h2, h3, h4⟩ := hs
        simp only [] at h1 h2 h3 h4
        apply WP.spec_mono (verdicts_body_spec tasks sites p any_fpu windows d _ its vs tss i hn hwl hdl hil hder hits (by omega) h2 h3 h4)
        intro r hr
        cases r with
        | done q =>
          obtain ⟨vs', tss'⟩ := q
          simpa using hr
        | cont s' =>
          obtain ⟨vs', tss', i'⟩ := s'
          simp only [] at hr ⊢
          obtain ⟨e1, e2, e3, e4, e5⟩ := hr
          exact ⟨⟨by omega, e3, e4, e5⟩, by omega⟩
      · simp [allVerdicts]
    step with hloop1 as ⟨ vs, tss, hvl, htl, hvs ⟩
    intro ds vs' tss' heq
    simp at heq
    obtain ⟨rfl, rfl, rfl⟩ := heq
    exact ⟨hdl, hder, hvl, htl, its.val, hil, hits, hvs⟩

end rsk_analyzer
