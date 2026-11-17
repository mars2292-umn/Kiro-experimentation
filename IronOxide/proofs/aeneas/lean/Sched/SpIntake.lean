import Sched.SpFrame
open Aeneas Aeneas.Std Result

/-! # The two functions that write the sporadic timing fields

What `Kernel::intake_release` and `Kernel::deferred_release` do to
`has_last`, `last_due` and `deferred_until`, each statement of the form "if
the function returns", joined to what the same call does to the scheduler
state (`intake_release_eff`, `deferred_release_eff`):

- an accepted event (`Timing::accept` after `Timing::intake_decision`
  answered accept) makes the Release pending;
- a deferred event (`Timing::defer` after the decision answered defer)
  holds the Release, and the MIT_Violation's response follows;
- anything else leaves the three fields and changes the scheduler state by
  a response only;
- a deferred Release that is made takes `deferred_until` as its due
  instant (`Timing::deferred_due_clamped`). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- `Timing::accept`: the Task has a last Release, due at `now`; the
deferral instants are left. -/
theorem accept_elem {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {now : U64}
    (h : logic.time.Timing.accept tm cfg t now = ok tm') :
    t.val < NT.val ∧
    tm'.has_last = tm.has_last.set (UScalar.cast .Usize t) true ∧
    tm'.last_due = tm.last_due.set (UScalar.cast .Usize t) now ∧
    tm'.deferred_until = tm.deferred_until := by
  unfold logic.time.Timing.accept at h
  simp only [lift, bind_ok, bind_eq_ok] at h
  obtain ⟨a, ha, a1, ha1, a2, ha2, a3, ha3, tc, htc, i5, hi5, a4, ha4, h⟩ := h
  have h := ok_inj h
  subst h
  obtain ⟨hb, rfl⟩ := update_ok ha
  obtain ⟨_, rfl⟩ := update_ok ha1
  rw [U8.cast_Usize_val_eq] at hb
  exact ⟨hb, rfl, rfl, rfl⟩

/-- `Timing::defer`: the deferral instant of the Task is its last due
instant plus its MIT; `has_last` and `last_due` are left. -/
theorem defer_elem {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {now a : U64}
    (h : logic.time.Timing.defer tm cfg t now = ok (a, tm')) :
    t.val < NT.val ∧ tm'.has_last = tm.has_last ∧ tm'.last_due = tm.last_due ∧
    tm'.deferred_until = tm.deferred_until.set (UScalar.cast .Usize t) a ∧
    ∃ tc, cfg.tasks.val[t.val]? = some tc ∧
      a.val = (tm.last_due.val[t.val]!).val + tc.period.val := by
  unfold logic.time.Timing.defer at h
  simp only [lift, bind_ok, bind_eq_ok] at h
  obtain ⟨i1, hi1, tc, htc, at0, hat, a0, ha0, a1, ha1, tc1, htc1, i6, hi6, a2, ha2, h⟩ := h
  have h := ok_inj h
  rw [Prod.mk.injEq] at h
  obtain ⟨rfl, rfl⟩ := h
  obtain ⟨hb, rfl⟩ := update_ok ha0
  obtain ⟨_, rfl⟩ := index_ok hi1
  obtain ⟨_, htc'⟩ := index_ok' htc
  rw [U8.cast_Usize_val_eq] at hb htc'
  refine ⟨hb, rfl, rfl, rfl, tc, htc', ?_⟩
  have := u64_add_ok hat
  rw [U8.cast_Usize_val_eq] at this
  exact this

/-- `Timing::deferred_due_clamped`: the Task has a last Release, due at
the instant given; the deferral instants are left. -/
theorem deferred_due_clamped_elem {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {a : U64}
    (h : logic.time.Timing.deferred_due_clamped tm cfg t a = ok tm') :
    t.val < NT.val ∧
    tm'.has_last = tm.has_last.set (UScalar.cast .Usize t) true ∧
    tm'.last_due = tm.last_due.set (UScalar.cast .Usize t) a ∧
    tm'.deferred_until = tm.deferred_until := by
  unfold logic.time.Timing.deferred_due_clamped at h
  simp only [lift, bind_ok, bind_eq_ok] at h
  obtain ⟨a0, ha0, a1, ha1, a2, ha2, h⟩ := h
  have h := ok_inj h
  subst h
  obtain ⟨hb, rfl⟩ := update_ok ha0
  obtain ⟨_, rfl⟩ := update_ok ha2
  rw [U8.cast_Usize_val_eq] at hb
  exact ⟨hb, rfl, rfl, rfl⟩

/-- The MIT decision answers "defer" only with no Release of the Task held
and only for a Task that has had an accepted Release. -/
theorem intake_decision_defer {tm : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t d : U8} {rn : Bool} {now : U64}
    (h : logic.time.Timing.intake_decision tm cfg t rn now = ok d)
    (hd : d = logic.time.INTAKE_DEFER) :
    rn = true ∧ tm.has_last.val[t.val]! = true := by
  subst hd
  unfold logic.time.Timing.intake_decision at h
  split at h
  · rename_i hrn
    refine ⟨hrn, ?_⟩
    simp only [lift, bind_ok, bind_eq_ok] at h
    obtain ⟨tc, htc, b, hb, h⟩ := h
    obtain ⟨_, rfl⟩ := index_ok hb
    rw [U8.cast_Usize_val_eq] at h
    split at h
    · rename_i hb
      exact hb
    · have h := ok_inj h
      rw [intake_accept, intake_defer] at h
      cases h
  · have h := ok_inj h
    rw [intake_discard, intake_defer] at h
    cases h

set_option maxHeartbeats 4000000 in
/-- `Kernel::intake_release`, the sporadic timing fields with the scheduler
state. -/
theorem intake_release_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t sp off : U8} {now : U64}
    (h : logic.kernel.Kernel.intake_release k cfg t sp now off = ok k') :
    (rel k.sched t.val = 0 ∧ logic.sched.Sched.make_pending k.sched cfg t = ok k'.sched ∧
      ∃ (rn : Bool) (now1 : U64) (d : U8),
        logic.time.Timing.intake_decision k.timing cfg t rn now1 = ok d ∧
        d = logic.time.INTAKE_ACCEPT ∧
        logic.time.Timing.accept k.timing cfg t now1 = ok k'.timing) ∨
    (∃ (s1 : logic.sched.Sched NT NR NL) (rn : Bool) (now1 : U64) (d : U8) (a : U64)
        (tm2 : logic.time.Timing NT),
      rel k.sched t.val = 0 ∧ logic.sched.Sched.defer k.sched cfg t = ok s1 ∧
      RSteps cfg s1 k'.sched ∧
      logic.time.Timing.intake_decision k.timing cfg t rn now1 = ok d ∧
      d = logic.time.INTAKE_DEFER ∧
      logic.time.Timing.defer k.timing cfg t now1 = ok (a, tm2) ∧
      k'.timing.has_last = tm2.has_last ∧ spw k'.timing = spw tm2) ∨
    (RSteps cfg k.sched k'.sched ∧ SpKept k k') := by
  unfold logic.kernel.Kernel.intake_release at h
  peel h
  all_goals
    have ho := observe_sched (by assumption)
    have ht := observe_timing (by assumption)
    first
      | (refine Or.inr (Or.inr ⟨?_, ?_, ?_⟩)
         · try dsimp only
           rw [ho]
         · try dsimp only
           rw [ht]
         · try dsimp only
           rw [ht])
      | (refine Or.inr (Or.inr ⟨?_, ?_, ?_⟩)
         · try dsimp only
           rw [ho]
         · dsimp only
           exact (mask_hl (by assumption)).trans (by rw [ht])
         · dsimp only
           exact (mask_sw (by assumption)).trans (by rw [ht]))
      | (have hacc := (intake_decision_accept (by assumption) (by assumption)).1
         have hr := rel_of_index (by assumption)
         rw [of_decide_eq_true hacc, rel_none, ho] at hr
         simp only [ho, ht] at *
         exact Or.inl ⟨hr, by assumption, _, _, _, by assumption, by assumption, by assumption⟩)
      | (have hm := mit_violation_r h
         have hsp := mit_violation_sp h
         dsimp only at hm hsp
         refine Or.inr (Or.inr ⟨?_, ?_, ?_⟩)
         · rw [ho] at hm; exact hm
         · exact hsp.1.trans ((mask_hl (by assumption)).trans (by rw [ht]))
         · exact hsp.2.trans ((mask_sw (by assumption)).trans (by rw [ht])))
      | (have hm := mit_violation_r h
         have hsp := mit_violation_sp h
         dsimp only at hm hsp
         have hr := rel_of_index (by assumption)
         simp only [*, rel_none] at hr
         simp only [ho] at *
         simp only [ht] at *
         exact Or.inr (Or.inl ⟨_, _, _, _, _, _, hr, by assumption, hm, by assumption, by assumption,
           by assumption, hsp.1, hsp.2⟩))

set_option maxHeartbeats 2000000 in
/-- `Kernel::deferred_release`, the sporadic timing fields with the
scheduler state: the held Release is made with `deferred_until` as its due
instant, or the fields are left. -/
theorem deferred_release_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deferred_release k cfg t i = ok k') :
    (rel k.sched t.val = 2 ∧ logic.sched.Sched.make_pending k.sched cfg t = ok k'.sched ∧
      ∃ a : U64, k.timing.deferred_until.index_usize (UScalar.cast .Usize t) = ok a ∧
        logic.time.Timing.deferred_due_clamped k.timing cfg t a = ok k'.timing) ∨
    (RSteps cfg k.sched k'.sched ∧ SpKept k k') := by
  unfold logic.kernel.Kernel.deferred_release at h
  peel h
  all_goals first
    | (refine Or.inr ⟨?_, rfl, rfl⟩; dsimp only
       exact Relation.ReflTransGen.refl)
    | (refine Or.inr ⟨?_, rfl, rfl⟩; dsimp only
       exact Relation.ReflTransGen.single (RStep.kclear _ _ _ (by assumption)))
    | (have hr := rel_of_index (by assumption)
       refine Or.inl ⟨?_, ?_, ?_⟩
       · rw [hr]; simp_all [rel_deferred]
       · dsimp only; assumption
       · exact ⟨_, by assumption, by dsimp only; assumption⟩)

/-- Whether Task `u` is not a Periodic_Task in the configuration. -/
def npk (cfg : logic.Config NT NR NP) (u : Nat) : Prop :=
  ∀ tc, cfg.tasks.val[u]? = some tc → tc.kind ≠ logic.KIND_PERIODIC

/-- `Kernel::periodic_release` of a Task that is not periodic leaves the
scheduler state (it disarms the slot and returns). -/
theorem periodic_release_np {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (hnp : npk cfg t.val)
    (h : logic.kernel.Kernel.periodic_release k cfg t i = ok k') : k'.sched = k.sched := by
  unfold logic.kernel.Kernel.periodic_release at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨tc, htc, nominal, hnom, tc1, htc1, h1⟩ := h
  rw [htc] at htc1
  have e := ok_inj htc1
  subst e
  obtain ⟨_, htc'⟩ := index_ok' htc
  rw [U8.cast_Usize_val_eq] at htc'
  have hk := hnp tc htc'
  have hne : (tc.kind != logic.KIND_PERIODIC) = true := by simp [hk]
  simp only [hne, ite_true, bind_eq_ok] at h1
  obtain ⟨sl, _, h1⟩ := h1
  have h1 := ok_inj h1
  subst h1
  rfl

end P18
