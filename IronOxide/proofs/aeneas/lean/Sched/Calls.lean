import Sched.Entry
open Aeneas Aeneas.Std Result

/-! # The Kernel's transitions, read on the scheduler state

`Kernel::dispatch`, `job_return`, `svc_lock`, `svc_unlock`,
`periodic_release` and `Sched::select` as extracted: what each leaves in
`sched` whenever it returns. With `intake_release_eff` (`Sched/Entry.lean`)
these are the transitions of the run of `Sched/KRun.lean`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- `Kernel::dispatch` starts the Job (`Sched::start`). -/
theorem dispatch_eff {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {pc : U32}
    (h : logic.kernel.Kernel.dispatch k cfg t pc = ok k') :
    logic.sched.Sched.start k.sched cfg t = ok k'.sched := by
  unfold logic.kernel.Kernel.dispatch at h
  peel h
  all_goals
    rw [arm_budget_sched h]
    dsimp only
    assumption

/-- `Kernel::job_return` ends the Job on top (`Sched::finish_top`). -/
theorem job_return_eff {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {ec : U32}
    (h : logic.kernel.Kernel.job_return k cfg ec = ok k') :
    ∃ r, logic.sched.Sched.finish_top k.sched cfg = ok (r, k'.sched) := by
  unfold logic.kernel.Kernel.job_return at h
  peel h
  all_goals
    apply Exists.intro
    try rw [arm_budget_sched h]
    try (have he := end_mit_window_sched (by assumption); dsimp only at he; rw [he])
    try dsimp only
    try (have hp := push_event_sched (by assumption); dsimp only at hp; rw [hp])
    try dsimp only
    assumption

/-- `Kernel::svc_lock` is `Sched::lock`. -/
theorem svc_lock_eff {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_lock k cfg r = ok (b, k')) :
    logic.sched.Sched.lock k.sched cfg r = ok (b, k'.sched) := by
  unfold logic.kernel.Kernel.svc_lock at h
  peel h
  all_goals
    try dsimp only
    try (have hp := push_event_sched (by assumption); rw [hp])
    try dsimp only
    assumption

/-- `Kernel::svc_unlock` is `Sched::unlock`. -/
theorem svc_unlock_eff {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (b, k')) :
    logic.sched.Sched.unlock k.sched cfg r = ok (b, k'.sched) := by
  unfold logic.kernel.Kernel.svc_unlock at h
  peel h
  all_goals
    try dsimp only
    try (have hp := push_event_sched (by assumption); rw [hp])
    try dsimp only
    assumption

set_option maxHeartbeats 2000000 in
/-- A periodic release event (`periodic_release`): the Release is made
(`make_pending`, only with no Release of the Task held), or the scheduler
state changes by a response only (an overlap's MIT_Violation; a stopped
Partition or the end of the operating duration: no change). -/
theorem periodic_release_eff {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.periodic_release k cfg t i = ok k') :
    (rel k.sched t.val = 0 ∧ logic.sched.Sched.make_pending k.sched cfg t = ok k'.sched) ∨
    RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.periodic_release at h
  peel h
  all_goals first
    | (refine Or.inr ?_; try dsimp only
       exact Relation.ReflTransGen.refl)
    | (refine Or.inr ?_; dsimp only; exact mit_violation_r (by assumption))
    | (have hr := rel_of_index (by assumption)
       simp only [*, rel_none] at hr
       refine Or.inl ⟨hr, ?_⟩; dsimp only; assumption)
    | (have hp := push_event_sched (by assumption)
       have hr := rel_of_index (by assumption)
       simp only [*, rel_none] at hr
       refine Or.inl ⟨hr, ?_⟩; dsimp only; rw [← hp]; assumption)

/-- A deferral event (`deferred_release`): the held Release is made
(`make_pending`, only when one is held), or the scheduler state changes by
a dropped Release only (the deferred instant too late in the operating
duration; a stopped Partition or no Release held: no change). -/
theorem deferred_release_eff {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deferred_release k cfg t i = ok k') :
    (rel k.sched t.val = 2 ∧ logic.sched.Sched.make_pending k.sched cfg t = ok k'.sched) ∨
    RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.deferred_release at h
  peel h
  all_goals first
    | (refine Or.inr ?_; dsimp only
       exact Relation.ReflTransGen.refl)
    | (refine Or.inr ?_; dsimp only
       exact Relation.ReflTransGen.single (RStep.kclear _ _ _ (by assumption)))
    | (have hr := rel_of_index (by assumption)
       refine Or.inl ⟨?_, ?_⟩
       · rw [hr]; simp_all [rel_deferred]
       · dsimp only; assumption)

/-- A deadline event's entry (`deadline_miss`): the Deadline_Miss response. -/
theorem deadline_miss_eff {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deadline_miss k cfg t i = ok k') :
    RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.deadline_miss at h
  peel h
  have hr := apply_response_r h
  have hp := push_event_sched (by assumption)
  dsimp only at hp
  rw [hp] at hr
  exact hr

/-- One Overrun report per Job, on the code as extracted (R18.2,
`Budget::reported`): a Budget event of a Task whose Overrun is already
reported changes nothing but its slot: no second report, no second
response. (`Budget::report` sets the flag and `Budget::reset`, at the
dispatch of the Task's next Job, clears it.) -/
theorem budget_event_reported {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k')
    (hr : k.budget.reported.val[t.val]! = true) :
    k'.sched = k.sched ∧ k'.log = k.log ∧ k'.budget = k.budget := by
  unfold logic.kernel.Kernel.budget_event at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨s, hs, h⟩ := h
  split at h
  · have h := ok_inj h; subst h; exact ⟨rfl, rfl, rfl⟩
  · simp only [bind_eq_ok] at h
    obtain ⟨i1, hi1, i2, hi2, h⟩ := h
    split at h
    · have h := ok_inj h; subst h; exact ⟨rfl, rfl, rfl⟩
    · simp only [bind_eq_ok] at h
      obtain ⟨b, hb, h⟩ := h
      obtain ⟨_, rfl⟩ := index_ok hb
      rw [U8.cast_Usize_val_eq] at h
      simp only [hr, ite_true] at h
      have h := ok_inj h; subst h; exact ⟨rfl, rfl, rfl⟩

/-- A Budget event, whatever it finds, changes the scheduler state only
through its Overrun_Response (`apply_response`): not at all when the Task's
Job is not on top, when the Overrun is already reported, or when there is
no Overrun (the slot is armed again). -/
theorem budget_event_eff {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k') :
    RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.budget_event at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨s, hs, h⟩ := h
  split at h
  · have h := ok_inj h; subst h; exact Relation.ReflTransGen.refl
  · simp only [bind_eq_ok] at h
    obtain ⟨i1, hi1, i2, hi2, h⟩ := h
    split at h
    · have h := ok_inj h; subst h; exact Relation.ReflTransGen.refl
    · simp only [bind_eq_ok] at h
      obtain ⟨b, hb, h⟩ := h
      split at h
      · have h := ok_inj h; subst h; exact Relation.ReflTransGen.refl
      · simp only [bind_eq_ok] at h
        obtain ⟨b1, hb1, h⟩ := h
        split at h
        · simp only [bind_eq_ok] at h
          obtain ⟨b2, hb2, tc, htc, i6, hi6, i7, hi7, self1, hpe, pc, hpc, h⟩ := h
          have hr := apply_response_r h
          have hp := push_event_sched hpe
          dsimp only at hp
          rw [hp] at hr
          exact hr
        · have ha := arm_budget_sched h
          dsimp only at ha
          rw [ha]

/-! ## The last step of every Kernel entry (`Kernel::settle`, 2026-11-15) -/

deriving instance Inhabited for rsk_kernel.rsk_logic.logic.time.Slot

/-- The Budget slot of the Job on top of the stack is armed. -/
def topBudgetArmed (k : logic.kernel.Kernel NT NR NP NL NS NE) (cfg : logic.Config NT NR NP) :
    Prop :=
  0 < k.sched.depth.val ∧
  (k.slots.slots.val[(cfg.slot_budget.val[(k.sched.stack.val[k.sched.depth.val - 1]!).val]!).val]!).armed
    = true

/-- `settle` leaves the scheduler state as it is. -/
theorem settle_sched {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : Option U8 × Bool}
    (h : logic.kernel.Kernel.settle k cfg = ok (r, k')) : k'.sched = k.sched := by
  unfold logic.kernel.Kernel.settle at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨adm, _, sel, _, h⟩ := h
  split at h
  · have h := ok_inj h; cases h; rfl
  · simp only [bind_eq_ok] at h
    obtain ⟨i, _, t, _, i2, _, h⟩ := h
    cases adm with
    | none =>
      dsimp only at h
      split at h
      · split at h
        · have h := ok_inj h; cases h; rfl
        · simp only [bind_eq_ok] at h
          obtain ⟨s, _, h⟩ := h
          split at h
          · have h := ok_inj h; cases h; rfl
          · simp only [bind_eq_ok] at h
            obtain ⟨i5, _, h⟩ := h
            split at h
            · simp only [bind_eq_ok] at h
              obtain ⟨b, _, h⟩ := h
              split at h
              · have h := ok_inj h; cases h; rfl
              · simp only [bind_eq_ok] at h
                obtain ⟨k1, hk1, h⟩ := h
                have h := ok_inj h; cases h
                exact arm_budget_sched hk1
            · have h := ok_inj h; cases h; rfl
      · have h := ok_inj h; cases h; rfl
    | some u =>
      dsimp only at h
      simp only [bind_eq_ok] at h
      obtain ⟨s, _, h⟩ := h
      split at h
      · simp only [bind_eq_ok] at h
        obtain ⟨s1, _, h⟩ := h
        have h := ok_inj h; cases h; rfl
      · have h := ok_inj h; cases h; rfl

/-- When R7.1 admits a Task (`Sched::select` returns one), `settle` leaves
the Budget slot of the Job on top disarmed: the entry that pends a dispatch
leaves no Budget event of the executing Job armed. -/
theorem settle_admitted_quiet {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : Option U8 × Bool} {u : U8}
    (h : logic.kernel.Kernel.settle k cfg = ok (r, k'))
    (hadm : logic.sched.Sched.select k.sched cfg = ok (some u)) : ¬ topBudgetArmed k' cfg := by
  rintro ⟨hd, ha⟩
  have hs := settle_sched h
  rw [hs] at hd ha
  unfold logic.kernel.Kernel.settle at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨adm, hadm', sel, _, h⟩ := h
  rw [hadm] at hadm'
  have hadm' := ok_inj hadm'
  subst hadm'
  split at h
  · rename_i h0
    rw [h0] at hd
    simp at hd
  · simp only [bind_eq_ok] at h
    obtain ⟨i, hi, t, ht, i2, hi2, h⟩ := h
    obtain ⟨s, hsl, h⟩ := h
    obtain ⟨_, hiv⟩ := usize_sub_ok hi
    obtain ⟨_, rfl⟩ := index_ok ht
    obtain ⟨_, rfl⟩ := index_ok hi2
    obtain ⟨hlt, rfl⟩ := index_ok hsl
    simp only [U8.cast_Usize_val_eq, hiv] at ha hlt hsl h ⊢
    have h1 : (1#usize).val = 1 := rfl
    simp only [h1] at hlt hsl h
    split at h
    · simp only [bind_eq_ok] at h
      obtain ⟨s1, hs1, h⟩ := h
      have h := ok_inj h; cases h
      unfold logic.time.Slots.disarm at hs1
      simp only [bind_eq_ok] at hs1
      obtain ⟨a, ha', hs1⟩ := hs1
      have hs1 := ok_inj hs1
      subst hs1
      obtain ⟨_, rfl⟩ := update_ok ha'
      dsimp only at ha
      rw [get_set _ _ _ _ (by simpa [U8.cast_Usize_val_eq] using hlt)] at ha
      simp [U8.cast_Usize_val_eq] at ha
    · rename_i hna
      have h := ok_inj h; cases h
      exact hna ha

/-- A Kernel state that `settle` left with the Budget slot of the Job on
top armed is one in which R7.1 admits no Task. This is what the run's
condition on a Budget event's own timer entry was (no dispatch pending):
a consequence of the Kernel's code since 2026-11-15. -/
theorem settled_armed_none {k0 k : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : Option U8 × Bool}
    (h : logic.kernel.Kernel.settle k0 cfg = ok (r, k)) (ha : topBudgetArmed k cfg) :
    logic.sched.Sched.select k.sched cfg = ok none := by
  have hs := settle_sched h
  have hsel : ∃ adm, logic.sched.Sched.select k0.sched cfg = ok adm := by
    unfold logic.kernel.Kernel.settle at h
    simp only [bind_eq_ok, lift, bind_ok] at h
    obtain ⟨adm, hadm, _⟩ := h
    exact ⟨adm, hadm⟩
  obtain ⟨adm, hadm⟩ := hsel
  rw [hs]
  cases adm with
  | none => exact hadm
  | some u => exact absurd ha (settle_admitted_quiet h hadm)

/-- The two cases that the Analyzer's `B_i` did not cover before the
Kernel change (Verification_Plan 7.3) do not arise. After the unlock that
ends a blocking Critical_Section, when it makes a dispatch pending, -/
theorem unlock_entry_leaves_no_budget_event {k k1 k2 : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {res u : U8} {b : Bool} {r : Option U8 × Bool}
    (_hu : logic.kernel.Kernel.svc_unlock k cfg res = ok (b, k1))
    (hs : logic.kernel.Kernel.settle k1 cfg = ok (r, k2))
    (hadm : logic.sched.Sched.select k1.sched cfg = ok (some u)) : ¬ topBudgetArmed k2 cfg :=
  settle_admitted_quiet hs hadm

/-- and after an entry that ends the Job on top (its return, or a response
of a Budget event), when a dispatch is pending: the Job resumed underneath
has no Budget event armed, whatever `end_top` or `job_return` armed for
it. -/
theorem job_end_entry_leaves_no_budget_event {k k1 k2 : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {ec : U32} {u : U8} {r : Option U8 × Bool}
    (_hj : logic.kernel.Kernel.job_return k cfg ec = ok k1)
    (hs : logic.kernel.Kernel.settle k1 cfg = ok (r, k2))
    (hadm : logic.sched.Sched.select k1.sched cfg = ok (some u)) : ¬ topBudgetArmed k2 cfg :=
  settle_admitted_quiet hs hadm

theorem budget_event_entry_leaves_no_budget_event
    {k k1 k2 : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t u : U8} {i : Usize} {r : Option U8 × Bool}
    (_hb : logic.kernel.Kernel.budget_event k cfg t i = ok k1)
    (hs : logic.kernel.Kernel.settle k1 cfg = ok (r, k2))
    (hadm : logic.sched.Sched.select k1.sched cfg = ok (some u)) : ¬ topBudgetArmed k2 cfg :=
  settle_admitted_quiet hs hadm

end P18
