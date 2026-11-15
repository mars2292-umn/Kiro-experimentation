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

/-- The timed events of a lower-Priority Task's Job, beside its arrival,
that take a timer entry from a higher Job: the unmask of a Sporadic_Task,
its deferral under `defer`, and the deadline event when it can fire. -/
def lowerEvents (t : terms.TaskIn) (fires : Bool) : Nat :=
  (if t.periodic then 0 else if t.defer then 2 else 1) + (if fires then 1 else 0)

theorem lowerEvents_le (t : terms.TaskIn) (fires : Bool) : lowerEvents t fires ≤ timedEvents t := by
  unfold lowerEvents timedEvents
  cases t.periodic <;> cases t.defer <;> cases fires <;> simp

theorem lower_event_count_spec (t : terms.TaskIn) (q : Bool) :
    terms.lower_event_count t q ⦃ k => k.val = lowerEvents t (!q) ⦄ := by
  unfold terms.lower_event_count lowerEvents
  cases hp : t.periodic <;> cases hd : t.defer <;> cases q
  all_goals simp
  all_goals step*

/-- `it` is Task `j` as an interferer of the others' recurrences: its
Priority, `J_rel`, its period (at least 1), `E_j` as R29.4 states it from
the Task set and its sites, the Kernel-level cost of one arrival, and that
of its other timed events, the deadline event left out when `quiet` marks
the Task. -/
def interfererOk (tasks : List terms.TaskIn) (sites : List terms.Site) (p : terms.Params)
    (any_fpu : Bool) (windows : List terms.Window) (quiet : List Bool) (j : Nat)
    (it : terms.Interferer) : Prop :=
  ∃ t w q, tasks[j]? = some t ∧ windows[j]? = some w ∧ quiet[j]? = some q
    ∧ it.priority = t.priority ∧ it.jitter = p.j_rel
    ∧ it.period.val = (if w.period.val = 0 then 1 else w.period.val)
    ∧ it.bound.val = specE t p (longestCs sites j sites.length)
        (specO t p (siteCount sites j sites.length) any_fpu)
    ∧ it.intake.val = (if t.periodic then p.timer.val else p.irq.val)
    ∧ it.events.val = p.timer.val * lowerEvents t (!q)

def allInterferers (tasks : List terms.TaskIn) (sites : List terms.Site) (p : terms.Params)
    (any_fpu : Bool) (windows : List terms.Window) (quiet : List Bool)
    (its : List terms.Interferer) : Prop :=
  ∀ j it, its[j]? = some it → interfererOk tasks sites p any_fpu windows quiet j it

theorem interferers_body_spec (tasks : alloc.vec.Vec terms.TaskIn) (sites : alloc.vec.Vec terms.Site)
    (p : terms.Params) (any_fpu : Bool) (windows : alloc.vec.Vec terms.Window)
    (derived : alloc.vec.Vec terms.Derived) (quiet : alloc.vec.Vec Bool) (n : Usize)
    (its : alloc.vec.Vec terms.Interferer) (j : Usize)
    (hn : n.val = tasks.val.length) (hwl : windows.val.length = tasks.val.length)
    (hdl : derived.val.length = tasks.val.length) (hql : quiet.val.length = tasks.val.length)
    (hder : allDerived tasks.val sites.val p any_fpu derived.val)
    (hj : j.val ≤ n.val) (hlen : its.val.length = j.val)
    (hall : allInterferers tasks.val sites.val p any_fpu windows.val quiet.val its.val) :
    terms.analyse_pass_loop0.body tasks p windows derived quiet n its j ⦃ r =>
      match r with
      | .done (p', o) => p' = p ∧ o.val.length = tasks.val.length
          ∧ allInterferers tasks.val sites.val p any_fpu windows.val quiet.val o.val
      | .cont (its', j') => j'.val = j.val + 1 ∧ j'.val ≤ n.val ∧ its'.val.length = j'.val
          ∧ allInterferers tasks.val sites.val p any_fpu windows.val quiet.val its'.val ⦄ := by
  unfold terms.analyse_pass_loop0.body
  split
  · rename_i hlt
    have hlt' : j.val < tasks.val.length := by scalar_tac
    step as ⟨ t, ht ⟩
    step as ⟨ d, hd ⟩
    step as ⟨ w, hw ⟩
    step as ⟨ q, hq ⟩
    have hgt : tasks.val[j.val]? = some t := by rw [ht]; exact List.getElem?_eq_getElem hlt'
    have hgd : derived.val[j.val]? = some d := by
      rw [hd]; exact List.getElem?_eq_getElem (by omega)
    have hgw : windows.val[j.val]? = some w := by
      rw [hw]; exact List.getElem?_eq_getElem (by omega)
    have hgq : quiet.val[j.val]? = some q := by
      rw [hq]; exact List.getElem?_eq_getElem (by omega)
    obtain ⟨d1, d2, d3, d4, d5, d6⟩ := hder j.val d t hgd hgt
    have hbound : d.e.val = specE t p (longestCs sites.val j.val sites.val.length)
        (specO t p (siteCount sites.val j.val sites.val.length) any_fpu) := by
      rw [d4, d3, d1, d2]
    have key : ∀ (it : terms.Interferer) (its1 : alloc.vec.Vec terms.Interferer),
        its1.val = its.val ++ [it] →
        it.priority = t.priority → it.jitter = d.j → it.bound = d.e →
        it.period.val = (if w.period.val = 0 then 1 else w.period.val) →
        it.intake.val = (if t.periodic then p.timer.val else p.irq.val) →
        it.events.val = p.timer.val * lowerEvents t (!q) →
        allInterferers tasks.val sites.val p any_fpu windows.val quiet.val its1.val := by
      intro it its1 h1 hp hjit hb hper hin hev k x hx
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
        exact ⟨t, w, q, hgt, hgw, hgq, hp, by rw [hjit, d6], hper, by rw [hb, hbound], hin, hev⟩
    have hmul : ∀ k : U64, k.val = lowerEvents t (!q) → p.timer.val * k.val ≤ d.o.val := by
      intro k hk
      have h1 := lowerEvents_le t (!q)
      have h2 : p.timer.val * k.val ≤ p.timer.val * timedEvents t := by
        rw [hk]; exact Nat.mul_le_mul_left _ h1
      rw [d3]
      unfold specO
      omega
    by_cases hw0 : w.period = 0#u64 <;> cases hper : t.periodic
    all_goals simp [hw0, hper]
    all_goals step with lower_event_count_spec as ⟨ k, hk ⟩
    all_goals (have hm := hmul k hk)
    all_goals step as ⟨ ev, hev ⟩
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

/-- The housekeeping event's term: `⌈(w + J_rel) / T_hk⌉ · Δ_timer`. -/
def hkTerm (p : terms.Params) : rta.Term :=
  { jitter := p.j_rel, period := if p.hk_period = 0#u64 then 1#u64 else p.hk_period, bound := p.timer }

/-- Every Task's verdict and terms are those of its recurrence: from
`C_i + B_i + O_i`, over the terms of the interferers `its` and the
housekeeping event. -/
def allVerdicts (tasks : List terms.TaskIn) (sites : List terms.Site) (p : terms.Params)
    (any_fpu : Bool) (windows : List terms.Window) (its : List terms.Interferer)
    (vs : List terms.Verdict) (tss : List (alloc.vec.Vec terms.TermOf)) : Prop :=
  ∀ i v t w it ts, vs[i]? = some v → tss[i]? = some ts → tasks[i]? = some t
    → windows[i]? = some w → its[i]? = some it →
    ts.val.map (·.term) = specAll its it i (hkTerm p)
    ∧ verdictOk
        (t.budget.val + maxCandidate tasks sites t p sites.length
          + specO t p (siteCount sites i sites.length) any_fpu)
        (specAll its it i (hkTerm p)) p.j_rel.val w.deadline.val v

theorem verdicts_body_spec (tasks : alloc.vec.Vec terms.TaskIn) (sites : alloc.vec.Vec terms.Site)
    (p : terms.Params) (any_fpu : Bool) (windows : alloc.vec.Vec terms.Window)
    (derived : alloc.vec.Vec terms.Derived) (quiet : List Bool) (n : Usize)
    (its : alloc.vec.Vec terms.Interferer)
    (vs : alloc.vec.Vec terms.Verdict) (tss : alloc.vec.Vec (alloc.vec.Vec terms.TermOf)) (i : Usize)
    (hsz : 2 * tasks.val.length + 1 ≤ Usize.max)
    (hn : n.val = tasks.val.length) (hwl : windows.val.length = tasks.val.length)
    (hdl : derived.val.length = tasks.val.length) (hil : its.val.length = tasks.val.length)
    (hder : allDerived tasks.val sites.val p any_fpu derived.val)
    (hits : allInterferers tasks.val sites.val p any_fpu windows.val quiet its.val)
    (hi : i.val ≤ n.val) (hvl : vs.val.length = i.val) (htl : tss.val.length = i.val)
    (hall : allVerdicts tasks.val sites.val p any_fpu windows.val its.val vs.val tss.val) :
    terms.analyse_pass_loop1.body tasks p windows derived n its vs tss i ⦃ r =>
      match r with
      | .done (vs', tss') => vs'.val.length = tasks.val.length ∧ tss'.val.length = tasks.val.length
          ∧ allVerdicts tasks.val sites.val p any_fpu windows.val its.val vs'.val tss'.val
      | .cont (vs', tss', i') => i'.val = i.val + 1 ∧ i'.val ≤ n.val
          ∧ vs'.val.length = i'.val ∧ tss'.val.length = i'.val
          ∧ allVerdicts tasks.val sites.val p any_fpu windows.val its.val vs'.val tss'.val ⦄ := by
  unfold terms.analyse_pass_loop1.body
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
    obtain ⟨t', w', q', ht', hw', hq', q1, q2, q3, q4, q5, q6⟩ := hits i.val _ hgi
    have hper : ∀ x ∈ its.val, 0 < x.period.val := by
      intro x hx
      obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hx
      obtain ⟨_, w2, _, _, _, _, _, _, r3, _, _, _⟩ := hits k _ (List.getElem?_eq_getElem hk)
      rw [r3]; split <;> omega
    have hsz' : 2 * its.val.length + 1 ≤ Usize.max := by rw [hil]; exact hsz
    have hkp : 0 < (hkTerm p).period.val := by
      unfold hkTerm
      by_cases h0 : p.hk_period = 0#u64
      · simp [h0]
      · simp [h0]; scalar_tac
    have hite : (if p.hk_period = 0#u64 then (ok 1#u64 : Result U64) else ok p.hk_period)
        = ok (hkTerm p).period := by
      unfold hkTerm; split <;> rfl
    rw [hite]
    simp only [Std.bind_ok]
    have hlit : ∀ (x : U64), x = (hkTerm p).period →
        ({ jitter := p.j_rel, period := x, bound := p.timer } : rta.Term) = hkTerm p := by
      intro x hx; subst hx; rfl
    rw [hlit _ rfl]
    all_goals step with response_of_spec its i _ hgi t.budget d.b d.o w.deadline (hkTerm p) hkp hsz' hper as ⟨ verdict, own, e1, e2, e3, e4 ⟩
    all_goals step as ⟨ vs1, hvs1 ⟩
    all_goals step as ⟨ tss1, htss1 ⟩
    all_goals step as ⟨ i1, hi1 ⟩
    all_goals refine ⟨hi1, by scalar_tac, by rw [hvs1]; simp; omega, by rw [htss1]; simp; omega, ?_⟩
    all_goals (
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
        exact ⟨e1, e2, e3, e4⟩)
  · rename_i hge
    have heq : i.val = tasks.val.length := by scalar_tac
    simp
    exact ⟨by omega, by omega, hall⟩

/-- One pass: every Task's terms and verdict over the interferers that
`quiet` gives. -/
def PassOk (tasks : List terms.TaskIn) (sites : List terms.Site) (p : terms.Params)
    (any_fpu : Bool) (windows : List terms.Window) (quiet : List Bool)
    (vs : List terms.Verdict) (tss : List (alloc.vec.Vec terms.TermOf)) : Prop :=
  vs.length = tasks.length ∧ tss.length = tasks.length
  ∧ ∃ its : List terms.Interferer, its.length = tasks.length
      ∧ allInterferers tasks sites p any_fpu windows quiet its
      ∧ allVerdicts tasks sites p any_fpu windows its vs tss

theorem analyse_pass_spec (tasks : alloc.vec.Vec terms.TaskIn) (sites : alloc.vec.Vec terms.Site)
    (p : terms.Params) (any_fpu : Bool) (windows : alloc.vec.Vec terms.Window)
    (derived : alloc.vec.Vec terms.Derived) (quiet : alloc.vec.Vec Bool)
    (hsz : 2 * tasks.val.length + 1 ≤ Usize.max)
    (hwl : windows.val.length = tasks.val.length)
    (hdl : derived.val.length = tasks.val.length) (hql : quiet.val.length = tasks.val.length)
    (hder : allDerived tasks.val sites.val p any_fpu derived.val) :
    terms.analyse_pass tasks sites p any_fpu windows derived quiet ⦃ r =>
      PassOk tasks.val sites.val p any_fpu windows.val quiet.val r.1.val r.2.val ⦄ := by
  unfold terms.analyse_pass
  have hn : (alloc.vec.Vec.len tasks).val = tasks.val.length := by scalar_tac
  have hloop0 : terms.analyse_pass_loop0 tasks p windows derived quiet (alloc.vec.Vec.len tasks)
      (alloc.vec.Vec.new terms.Interferer) 0#usize ⦃ r =>
        r.1 = p ∧ r.2.val.length = tasks.val.length
          ∧ allInterferers tasks.val sites.val p any_fpu windows.val quiet.val r.2.val ⦄ := by
    unfold terms.analyse_pass_loop0
    apply loop.spec_decr_nat
      (fun (s : alloc.vec.Vec terms.Interferer × Usize) => tasks.val.length - s.2.val)
      (fun s => s.2.val ≤ tasks.val.length ∧ s.1.val.length = s.2.val
        ∧ allInterferers tasks.val sites.val p any_fpu windows.val quiet.val s.1.val)
    · intro s hs
      obtain ⟨its, j⟩ := s
      obtain ⟨h1, h2, h3⟩ := hs
      simp only [] at h1 h2 h3
      apply WP.spec_mono (interferers_body_spec tasks sites p any_fpu windows derived quiet _ its j hn hwl hdl hql hder (by omega) h2 h3)
      intro r hr
      cases r with
      | done o =>
        obtain ⟨p', o'⟩ := o
        simpa using hr
      | cont s' =>
        obtain ⟨its', j'⟩ := s'
        simp only [] at hr ⊢
        obtain ⟨e1, e2, e3, e4⟩ := hr
        exact ⟨⟨by omega, e3, e4⟩, by omega⟩
    · simp [allInterferers]
  step with hloop0 as ⟨ p1, its, hp1, hil, hits ⟩
  subst hp1
  have hloop1 : terms.analyse_pass_loop1 tasks p1 windows derived (alloc.vec.Vec.len tasks) its
      (alloc.vec.Vec.new terms.Verdict) (alloc.vec.Vec.new (alloc.vec.Vec terms.TermOf)) 0#usize
      ⦃ r => r.1.val.length = tasks.val.length ∧ r.2.val.length = tasks.val.length
        ∧ allVerdicts tasks.val sites.val p1 any_fpu windows.val its.val r.1.val r.2.val ⦄ := by
    unfold terms.analyse_pass_loop1
    apply loop.spec_decr_nat
      (fun (s : alloc.vec.Vec terms.Verdict × alloc.vec.Vec (alloc.vec.Vec terms.TermOf) × Usize) =>
        tasks.val.length - s.2.2.val)
      (fun s => s.2.2.val ≤ tasks.val.length ∧ s.1.val.length = s.2.2.val
        ∧ s.2.1.val.length = s.2.2.val
        ∧ allVerdicts tasks.val sites.val p1 any_fpu windows.val its.val s.1.val s.2.1.val)
    · intro s hs
      obtain ⟨vs, tss, i⟩ := s
      obtain ⟨h1, h2, h3, h4⟩ := hs
      simp only [] at h1 h2 h3 h4
      apply WP.spec_mono (verdicts_body_spec tasks sites p1 any_fpu windows derived quiet.val _ its vs tss i hsz hn hwl hdl hil hder hits (by omega) h2 h3 h4)
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
  step with hloop1 as ⟨ res, hvl, htl, hvs ⟩
  exact ⟨hvl, htl, its.val, hil, hits, hvs⟩

/-- Every Task that `quiet` marks is schedulable in `vs`. -/
def quietOk (quiet : List Bool) (vs : List terms.Verdict) : Prop :=
  ∀ (i : Nat) (v : terms.Verdict), quiet[i]? = some true → vs[i]? = some v → v.schedulable = true

theorem quiet_init_spec (n : Usize) :
    terms.analyse_all_loop0 n (alloc.vec.Vec.new Bool) 0#usize ⦃ q =>
      q.val.length = n.val ∧ ∀ (i : Nat), q.val[i]? ≠ some true ⦄ := by
  unfold terms.analyse_all_loop0
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec Bool × Usize) => n.val - s.2.val)
    (fun s => s.2.val ≤ n.val ∧ s.1.val.length = s.2.val ∧ ∀ (i : Nat), s.1.val[i]? ≠ some true)
  · intro s hs
    obtain ⟨q, j⟩ := s
    obtain ⟨h1, h2, h3⟩ := hs
    simp only [] at h1 h2 h3
    try simp only []
    unfold terms.analyse_all_loop0.body
    split
    · rename_i hlt
      step as ⟨ q1, hq1 ⟩
      step as ⟨ j1, hj1 ⟩
      try simp only []
      refine ⟨by scalar_tac, by rw [hq1]; simp; omega, ?_, by scalar_tac⟩
      intro i hi
      rw [hq1] at hi
      by_cases hk : i < q.val.length
      · rw [List.getElem?_append_left hk] at hi
        exact h3 i hi
      · have := (List.getElem?_eq_some_iff.mp hi).1
        simp at this
        have hil : i = q.val.length := by omega
        subst hil
        simp at hi
    · rename_i hge
      simp
      exact ⟨by scalar_tac, h3⟩
  · simp

theorem next_flags_spec (n : Usize) (quiet : alloc.vec.Vec Bool)
    (verdicts : alloc.vec.Vec terms.Verdict)
    (hn : n.val = verdicts.val.length) (hq : quiet.val.length = verdicts.val.length) :
    terms.analyse_all_loop1_loop0 n quiet verdicts (alloc.vec.Vec.new Bool) true 0#usize ⦃ r =>
      r.1.val = verdicts.val.map (·.schedulable) ⦄ := by
  unfold terms.analyse_all_loop1_loop0
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec Bool × Bool × Usize) => n.val - s.2.2.val)
    (fun s => s.2.2.val ≤ n.val ∧ s.1.val = (verdicts.val.take s.2.2.val).map (·.schedulable))
  · intro s hs
    obtain ⟨next, same, k⟩ := s
    obtain ⟨h1, h2⟩ := hs
    simp only [] at h1 h2
    try simp only []
    unfold terms.analyse_all_loop1_loop0.body
    split
    · rename_i hlt
      have hlt' : k.val < verdicts.val.length := by scalar_tac
      step as ⟨ v, hv ⟩
      step as ⟨ b, hb ⟩
      have hlen : next.val.length = k.val := by rw [h2]; simp; omega
      by_cases hne : (v.schedulable != b) = true
      all_goals simp only [hne, ite_true, ite_false, Std.bind_ok, Bool.false_eq_true]
      all_goals step as ⟨ next1, hnext1 ⟩
      all_goals step as ⟨ k1, hk1 ⟩
      all_goals refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      all_goals rw [hnext1, h2, hk1, hv, List.take_succ_eq_append_getElem hlt', List.map_append]
      all_goals rfl
    · rename_i hge
      have heq : k.val = verdicts.val.length := by scalar_tac
      simp
      rw [h2, heq]; simp
  · simp

theorem check_flags_spec (n : Usize) (next : alloc.vec.Vec Bool)
    (v2 : alloc.vec.Vec terms.Verdict)
    (hn : n.val = v2.val.length) (hq : next.val.length = v2.val.length) :
    terms.analyse_all_loop1_loop1 n next v2 true 0#usize ⦃ ok1 =>
      ok1 = true → quietOk next.val v2.val ⦄ := by
  unfold terms.analyse_all_loop1_loop1
  apply loop.spec_decr_nat
    (fun (s : Bool × Usize) => n.val - s.2.val)
    (fun s => s.2.val ≤ n.val ∧ (s.1 = true → ∀ (i : Nat) (v : terms.Verdict), i < s.2.val →
      next.val[i]? = some true → v2.val[i]? = some v → v.schedulable = true))
  · intro s hs
    obtain ⟨ok1, m⟩ := s
    obtain ⟨h1, h2⟩ := hs
    simp only [] at h1 h2
    try simp only []
    unfold terms.analyse_all_loop1_loop1.body
    split
    · rename_i hlt
      have hlt' : m.val < v2.val.length := by scalar_tac
      step as ⟨ b, hb ⟩
      have hgb : next.val[m.val]? = some b := by
        rw [hb]; exact List.getElem?_eq_getElem (by omega)
      cases hbv : b
      · simp only [Bool.false_eq_true, ite_false, Std.bind_ok]
        step as ⟨ m1, hm1 ⟩
        try simp only []
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro hok i v hi hni hvi
        by_cases him : i < m.val
        · exact h2 hok i v him hni hvi
        · have : i = m.val := by omega
          subst this
          rw [hgb, hbv] at hni
          simp at hni
      · simp only [ite_true]
        step as ⟨ v, hv ⟩
        have hgv : v2.val[m.val]? = some v := by
          rw [hv]; exact List.getElem?_eq_getElem hlt'
        cases hsv : v.schedulable
        · simp only [Bool.false_eq_true, ite_false, Std.bind_ok]
          step as ⟨ m1, hm1 ⟩
          try simp only []
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          intro hok
          simp at hok
        · simp only [ite_true, Std.bind_ok]
          step as ⟨ m1, hm1 ⟩
          try simp only []
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          intro hok i v' hi hni hvi
          by_cases him : i < m.val
          · exact h2 hok i v' him hni hvi
          · have : i = m.val := by omega
            subst this
            rw [hgv] at hvi
            simp at hvi
            subst hvi
            exact hsv
    · rename_i hge
      simp
      intro hok i v hni hvi
      have hi : i < v2.val.length := (List.getElem?_eq_some_iff.mp hvi).1
      exact h2 hok i v (by scalar_tac) hni hvi
  · simp

/-- The whole numeric analysis, on the code as compiled. For every Task
set whose sites name Tasks of the set, with a period and a deadline per
Task, `analyse_all` returns (no panic, no overflow, every loop ends), and
when it returns results:

- there is one derived entry per Task, holding its terms (R29.2 to R29.4);
- `quiet` marks Tasks, and every Task it marks is reported schedulable;
- there is a list of interferers, one per Task, each built from that
  Task's Priority, `J_rel`, period, `E_j`, the Kernel-level cost of its
  arrivals and of its other timed events, the deadline event left out for
  a Task `quiet` marks;
- every Task's terms are those of its recurrence over those interferers
  and the housekeeping event, and its verdict is that of the recurrence
  from `C_i + B_i + O_i`: a Task reported schedulable has `R = J_rel + w`
  for the least fixed point `w` and `R ≤ D_i`; a Task reported not
  schedulable without an overflow has no fixed point within its deadline;
  an overflow is never reported schedulable.

(`hsz`: twice the Task count fits a `usize`; see `terms_for_spec`.) -/
theorem analyse_all_spec (tasks : alloc.vec.Vec terms.TaskIn) (sites : alloc.vec.Vec terms.Site)
    (p : terms.Params) (any_fpu : Bool) (windows : alloc.vec.Vec terms.Window)
    (hsz : 2 * tasks.val.length + 1 ≤ Usize.max)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length)
    (hwl : windows.val.length = tasks.val.length) :
    terms.analyse_all tasks sites p any_fpu windows ⦃ r =>
      ∀ ds vs tss quiet, r = some (ds, vs, tss, quiet) →
        ds.val.length = tasks.val.length
        ∧ allDerived tasks.val sites.val p any_fpu ds.val
        ∧ quiet.val.length = tasks.val.length
        ∧ PassOk tasks.val sites.val p any_fpu windows.val quiet.val vs.val tss.val
        ∧ quietOk quiet.val vs.val ⦄ := by
  unfold terms.analyse_all
  step with derive_all_spec tasks sites p any_fpu hsok as ⟨ o, ho ⟩
  cases o with
  | none => simp
  | some d =>
    try simp only []
    obtain ⟨hdl, hder⟩ := ho d rfl
    have hn : (alloc.vec.Vec.len tasks).val = tasks.val.length := by scalar_tac
    step with quiet_init_spec as ⟨ q0, hq0l, hq0 ⟩
    step with analyse_pass_spec tasks sites p any_fpu windows d q0 hsz hwl hdl (by omega) hder as ⟨ vs0, tss0, hr0 ⟩
    have hloop : terms.analyse_all_loop1 tasks sites p any_fpu windows d (alloc.vec.Vec.len tasks)
        q0 vs0 tss0 0#usize ⦃ r =>
          r.1.val.length = tasks.val.length
          ∧ PassOk tasks.val sites.val p any_fpu windows.val r.1.val r.2.1.val r.2.2.val
          ∧ quietOk r.1.val r.2.1.val ⦄ := by
      unfold terms.analyse_all_loop1
      apply loop.spec_decr_nat
        (fun (s : alloc.vec.Vec Bool × alloc.vec.Vec terms.Verdict
            × alloc.vec.Vec (alloc.vec.Vec terms.TermOf) × Usize) => tasks.val.length - s.2.2.2.val)
        (fun s => s.1.val.length = tasks.val.length
          ∧ PassOk tasks.val sites.val p any_fpu windows.val s.1.val s.2.1.val s.2.2.1.val
          ∧ quietOk s.1.val s.2.1.val)
      · intro s hs
        obtain ⟨quiet, vs, tss, pass⟩ := s
        obtain ⟨h1, h2, h3⟩ := hs
        simp only [] at h1 h2 h3
        try simp only []
        unfold terms.analyse_all_loop1.body
        split
        · rename_i hlt
          have hvl : vs.val.length = tasks.val.length := h2.1
          step with next_flags_spec (alloc.vec.Vec.len tasks) quiet vs (by omega) (by omega) as ⟨ nx, same, hnx ⟩
          cases hsame : same
          · simp only [Bool.false_eq_true, ite_false]
            have hnl : nx.val.length = tasks.val.length := by rw [hnx]; simp; omega
            step with analyse_pass_spec tasks sites p any_fpu windows d nx hsz hwl hdl hnl hder as ⟨ v2, t2, hr2 ⟩
            step with check_flags_spec (alloc.vec.Vec.len tasks) nx v2 (by have := hr2.1; omega) (by have := hr2.1; omega) as ⟨ ok1, hok1 ⟩
            cases hok : ok1
            · simp only [Bool.false_eq_true, ite_false]
              simp
              exact ⟨h1, h2, h3⟩
            · simp only [ite_true]
              step as ⟨ pass1, hpass1 ⟩
              try simp only []
              exact ⟨hnl, hr2, hok1 hok, by scalar_tac⟩
          · simp only [ite_true]
            simp
            exact ⟨h1, h2, h3⟩
        · simp
          exact ⟨h1, h2, h3⟩
      · simp only []
        refine ⟨by omega, hr0, ?_⟩
        intro i v hi _
        exact absurd hi (hq0 i)
    step with hloop as ⟨ qf, vsf, tssf, hl1, hl2, hl3 ⟩
    intro ds vs tss quiet heq
    simp at heq
    obtain ⟨rfl, rfl, rfl, rfl⟩ := heq
    exact ⟨hdl, hder, hl1, hl2, hl3⟩

end rsk_analyzer
