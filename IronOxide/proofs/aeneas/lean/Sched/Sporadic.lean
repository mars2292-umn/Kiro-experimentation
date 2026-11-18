import Sched.SpIntake
open Aeneas Aeneas.Std Result

/-! # Sporadic arrivals at the instants the Kernel records

The run of `Sched/KRun.lean` takes "two arrivals of a Task are at least one
period apart" as a condition of each arrival (`KStep.jarrive`).
`Sched/Arrive.lean` derives it for a Periodic_Task. This file derives it
for a Sporadic_Task, whose period is its minimum inter-arrival time (MIT):

- `SpRel`: what one transition of the run does to a Task's sporadic timing
  fields and Release state. Either nothing that matters (`KeptAt`), or the
  Kernel accepted an event (the due instant moved by at least one MIT), or
  it deferred one (the deferral instant is the last due instant plus one
  MIT), or it made a deferred Release (the due instant is the deferral
  instant). Every code-modelled transition satisfies it (`kstep_sprel`,
  from the extracted `intake_release` and `deferred_release` and the frame
  lemmas of `Sched/SpFrame.lean`); since 2026-11-17 no transition is left
  for which it is a condition (`serve_slot_sprel`).
- `SStep`, `SReach`: the run in which the Job of a Sporadic_Task arrives
  either with the transition that accepts its event, at the instant the
  Kernel records as the Release's due instant (`saccept`), or, for a
  deferred event, at the instant the Kernel deferred it to (`darrive`).
  Neither carries a spacing condition.
- `sreach_kreach`: every such run is a run of `KRun.lean`.
  `property18_sporadic`: Property 18 over it.

What stays a condition: that the instant the Kernel records for an
accepted event is the instant of the run (the Kernel's reading when its
handler runs, not the instant of the event itself; the handler's latency
is measured, R10.6). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- Whether a Release of Task `u` was ever accepted. -/
def hlOf (k : logic.kernel.Kernel NT NR NP NL NS NE) (u : Nat) : Bool :=
  k.timing.has_last.val[u]!
/-- The instant the last Release of Task `u` became due, in ticks. -/
def ldOf (k : logic.kernel.Kernel NT NR NP NL NS NE) (u : Nat) : Nat :=
  (k.timing.last_due.val[u]!).val
/-- The instant a held event of Task `u` is deferred to, in ticks. -/
def duOf (k : logic.kernel.Kernel NT NR NP NL NS NE) (u : Nat) : Nat :=
  (k.timing.deferred_until.val[u]!).val

/-- Task `u`'s sporadic timing fields are left, no Release of it becomes
held, and, unless it is a Periodic_Task, none becomes pending. -/
def KeptAt (cfg : logic.Config NT NR NP) (k k' : logic.kernel.Kernel NT NR NP NL NS NE)
    (u : Nat) : Prop :=
  hlOf k' u = hlOf k u ∧ ldOf k' u = ldOf k u ∧ duOf k' u = duOf k u ∧
  (rel k'.sched u = 2 → rel k.sched u = 2) ∧
  (npk cfg u → rel k'.sched u = 1 → rel k.sched u = 1)

/-- What a transition does to Task `u`: nothing that matters; or an event
accepted; or an event deferred; or a deferred Release made. -/
def SpAtRel (cfg : logic.Config NT NR NP) (k k' : logic.kernel.Kernel NT NR NP NL NS NE)
    (u : Nat) : Prop :=
  KeptAt cfg k k' u ∨
  (hlOf k' u = true ∧ (hlOf k u = true → ldOf k u + cfgPeriod cfg u ≤ ldOf k' u) ∧
    rel k.sched u = 0 ∧ rel k'.sched u = 1) ∨
  (hlOf k u = true ∧ hlOf k' u = true ∧ ldOf k' u = ldOf k u ∧
    duOf k' u = ldOf k u + cfgPeriod cfg u ∧ rel k'.sched u ≠ 1) ∨
  (rel k.sched u = 2 ∧ hlOf k' u = true ∧ ldOf k' u = duOf k u ∧ rel k'.sched u = 1)

def SpRel (cfg : logic.Config NT NR NP) (k k' : logic.kernel.Kernel NT NR NP NL NS NE) : Prop :=
  ∀ u, u < NT.val → SpAtRel cfg k k' u

theorem spkept_views {k k' : logic.kernel.Kernel NT NR NP NL NS NE} (h : SpKept k k') (u : Nat) :
    hlOf k' u = hlOf k u ∧ ldOf k' u = ldOf k u ∧ duOf k' u = duOf k u := by
  obtain ⟨h1, h2⟩ := h
  unfold spw at h2
  rw [Prod.mk.injEq] at h2
  unfold hlOf ldOf duOf
  rw [h1, h2.1, h2.2]
  exact ⟨rfl, rfl, rfl⟩

theorem SpRel.kept {cfg : logic.Config NT NR NP} {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : SpKept k k')
    (h2 : ∀ u, rel k'.sched u = 2 → rel k.sched u = 2)
    (h1 : ∀ u, npk cfg u → rel k'.sched u = 1 → rel k.sched u = 1) : SpRel cfg k k' := by
  intro u _
  obtain ⟨a, b, c⟩ := spkept_views h u
  exact Or.inl ⟨a, b, c, h2 u, h1 u⟩

theorem SpRel.of_rsteps {cfg : logic.Config NT NR NP}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : SpKept k k') (hr : RSteps cfg k.sched k'.sched) : SpRel cfg k k' :=
  SpRel.kept h (fun u e => by rcases rsteps_rel hr u with e' | e' <;> omega)
    (fun u _ e => by rcases rsteps_rel hr u with e' | e' <;> omega)

theorem SpRel.of_releq {cfg : logic.Config NT NR NP}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : SpKept k k') (hr : ∀ u, rel k'.sched u = rel k.sched u) : SpRel cfg k k' :=
  SpRel.kept h (fun u e => by rw [← hr u]; exact e) (fun u _ e => by rw [← hr u]; exact e)

theorem SpRel.refl {cfg : logic.Config NT NR NP} {k : logic.kernel.Kernel NT NR NP NL NS NE} :
    SpRel cfg k k :=
  SpRel.of_releq ⟨rfl, rfl⟩ (fun _ => rfl)

/-! ## The Kernel functions of the run -/

theorem periodic_release_sprel {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.periodic_release k cfg t i = ok k') : SpRel cfg k k' := by
  rcases periodic_release_eff h with ⟨_, hmp⟩ | hrs
  · obtain ⟨_, hrv, _⟩ := make_pending_views hmp
    refine SpRel.kept (periodic_release_sp h) ?_ ?_
    · intro u e
      rw [hrv u] at e
      split at e
      · omega
      · exact e
    · intro u hnp e
      by_cases hu : u = t.val
      · subst hu
        rw [periodic_release_np hnp h] at e
        exact e
      · rw [hrv u, if_neg hu] at e
        exact e
  · exact SpRel.of_rsteps (periodic_release_sp h) hrs

theorem intake_release_sprel {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t sp off : U8} {now : U64}
    (h : logic.kernel.Kernel.intake_release k cfg t sp now off = ok k') : SpRel cfg k k' := by
  rcases intake_release_sp h with ⟨hr0, hmp, rn, now1, d, hdec, hd, hacc⟩ |
    ⟨s1, rn, now1, d, a, tm2, hr0, hdf, hrs, hdec, hd, htd, hhl, hsw⟩ | ⟨hrs, hk⟩
  · obtain ⟨ht, hrv, _⟩ := make_pending_views hmp
    obtain ⟨_, ehl, eld, edu⟩ := accept_elem hacc
    have hci : (UScalar.cast .Usize t : Usize).val < NT.val := by
      rw [U8.cast_Usize_val_eq]; exact ht
    intro u hu
    by_cases hut : u = t.val
    · subst hut
      refine Or.inr (Or.inl ⟨?_, ?_, hr0, ?_⟩)
      · unfold hlOf
        rw [ehl, get_set _ _ _ _ hci, U8.cast_Usize_val_eq]
        simp
      · intro hl
        obtain ⟨tc, htc, hsp⟩ := (intake_decision_accept hdec hd).2 hl
        unfold ldOf cfgPeriod
        rw [eld, get_set _ _ _ _ hci, U8.cast_Usize_val_eq, htc]
        simpa using hsp
      · rw [hrv]; simp
    · refine Or.inl ⟨?_, ?_, ?_, ?_, ?_⟩
      · unfold hlOf; rw [ehl, get_set _ _ _ _ hci, U8.cast_Usize_val_eq, if_neg hut]
      · unfold ldOf; rw [eld, get_set _ _ _ _ hci, U8.cast_Usize_val_eq, if_neg hut]
      · unfold duOf; rw [edu]
      · intro e; rw [hrv u, if_neg hut] at e; exact e
      · intro _ e; rw [hrv u, if_neg hut] at e; exact e
  · obtain ⟨ht, hdv, _⟩ := defer_views hdf
    obtain ⟨_, ehl, eld, edu, tc, htc, ha⟩ := defer_elem htd
    have hci : (UScalar.cast .Usize t : Usize).val < NT.val := by
      rw [U8.cast_Usize_val_eq]; exact ht
    unfold spw at hsw
    rw [Prod.mk.injEq] at hsw
    obtain ⟨hsl, hsd⟩ := hsw
    have hhas := (intake_decision_defer hdec hd).2
    intro u hu
    by_cases hut : u = t.val
    · subst hut
      refine Or.inr (Or.inr (Or.inl ⟨hhas, ?_, ?_, ?_, ?_⟩))
      · unfold hlOf; rw [hhl, ehl]; exact hhas
      · unfold ldOf; rw [hsl, eld]
      · unfold duOf ldOf cfgPeriod
        rw [hsd, edu, get_set _ _ _ _ hci, U8.cast_Usize_val_eq, htc]
        simpa using ha
      · have h2 := hdv t.val
        rw [if_pos rfl] at h2
        rcases rsteps_rel hrs t.val with e | e <;> omega
    · have h2 := hdv u
      rw [if_neg hut] at h2
      refine Or.inl ⟨?_, ?_, ?_, ?_, ?_⟩
      · unfold hlOf; rw [hhl, ehl]
      · unfold ldOf; rw [hsl, eld]
      · unfold duOf; rw [hsd, edu, get_set _ _ _ _ hci, U8.cast_Usize_val_eq, if_neg hut]
      · intro e
        rcases rsteps_rel hrs u with e' | e' <;> omega
      · intro _ e
        rcases rsteps_rel hrs u with e' | e' <;> omega
  · exact SpRel.of_rsteps hk hrs

theorem deferred_release_sprel {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deferred_release k cfg t i = ok k') : SpRel cfg k k' := by
  rcases deferred_release_sp h with ⟨hr2, hmp, a, hidx, hdc⟩ | ⟨hrs, hk⟩
  · obtain ⟨ht, hrv, _⟩ := make_pending_views hmp
    obtain ⟨_, ehl, eld, edu⟩ := deferred_due_clamped_elem hdc
    obtain ⟨_, ha⟩ := index_ok hidx
    have hci : (UScalar.cast .Usize t : Usize).val < NT.val := by
      rw [U8.cast_Usize_val_eq]; exact ht
    rw [U8.cast_Usize_val_eq] at ha
    intro u hu
    by_cases hut : u = t.val
    · subst hut
      refine Or.inr (Or.inr (Or.inr ⟨hr2, ?_, ?_, ?_⟩))
      · unfold hlOf
        rw [ehl, get_set _ _ _ _ hci, U8.cast_Usize_val_eq]
        simp
      · unfold ldOf duOf
        rw [eld, get_set _ _ _ _ hci, U8.cast_Usize_val_eq, if_pos rfl, ha]
      · rw [hrv]; simp
    · refine Or.inl ⟨?_, ?_, ?_, ?_, ?_⟩
      · unfold hlOf; rw [ehl, get_set _ _ _ _ hci, U8.cast_Usize_val_eq, if_neg hut]
      · unfold ldOf; rw [eld, get_set _ _ _ _ hci, U8.cast_Usize_val_eq, if_neg hut]
      · unfold duOf; rw [edu]
      · intro e; rw [hrv u, if_neg hut] at e; exact e
      · intro _ e; rw [hrv u, if_neg hut] at e; exact e
  · exact SpRel.of_rsteps hk hrs

set_option maxHeartbeats 8000000 in
/-- `Kernel::serve_slot`, of a slot of any kind. -/
theorem serve_slot_sprel {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k') : SpRel cfg k k' := by
  unfold logic.kernel.Kernel.serve_slot at h
  peel h
  all_goals first
    | exact periodic_release_sprel h
    | exact deferred_release_sprel h
    | exact SpRel.of_rsteps (deadline_miss_sp h) (deadline_miss_eff h)
    | exact SpRel.of_rsteps (budget_event_sp h) (budget_event_eff h)
    | exact SpRel.of_rsteps (by sp_close)
        (by
          have h1 := enter_safe_state_r h
          have h2 := push_event_sched (by assumption)
          dsimp only at h2
          rw [h2] at h1
          exact h1)
    | exact SpRel.of_releq (by sp_close) (fun _ => rfl)

/-- Every transition of the run satisfies `SpRel`, from the code. -/
theorem kstep_sprel {cfg : logic.Config NT NR NP} {P : Params}
    {x y : KSt NT NR NP NL NS NE} (h : KStep cfg P x y) : SpRel cfg x.k y.k := by
  cases h with
  | jarrive => exact SpRel.refl
  | earrive => exact SpRel.refl
  | periodic q t i d c k' hq hk hw hrel hnd hc hs hmade =>
    have := periodic_release_sprel hs
    split <;> exact this
  | intake q t sp off now d c k' hq hk hw hrel hnd hc hs hmade =>
    have := intake_release_sprel hs
    split <;> exact this
  | dispatch t pc c k' hk hsel hc hs =>
    obtain ⟨_, _, hrv, _⟩ := start_views (dispatch_eff hs)
    exact SpRel.kept (dispatch_sp hs)
      (fun u e => by rw [hrv u] at e; split at e <;> first | omega | exact e)
      (fun u _ e => by rw [hrv u] at e; split at e <;> first | omega | exact e)
  | lock hsel hk r a c k' hrun hrl hc ha hal hs =>
    exact SpRel.of_releq (svc_lock_sp hs) (lock_views (svc_lock_eff hs)).1.1
  | unlock hsel hk r c k' hrun hru hc hal hs =>
    exact SpRel.of_releq (svc_unlock_sp hs) (unlock_views (svc_unlock_eff hs)).1.1
  | finish hsel hk hj ec c k' hd hr0 hr hc hal hs =>
    obtain ⟨r, hf⟩ := job_return_eff hs
    exact SpRel.of_releq (job_return_sp hs) (finish_top_views hf).2.2.2.1
  | unwind ec k' hs =>
    obtain ⟨r, hf⟩ := job_return_eff hs
    exact SpRel.of_releq (job_return_sp hs) (finish_top_views hf).2.2.2.1
  | resp partition response cause t k' hs =>
    exact SpRel.of_rsteps (apply_response_sp hs) (apply_response_r hs)
  | bserved t i k' hs => exact SpRel.of_rsteps (budget_event_sp hs) (budget_event_eff hs)
  | opend i k1 k' hcode h1 hs => exact serve_slot_sprel hcode
  | settle r k' hs =>
    exact SpRel.of_releq (settle_sp hs) (fun u => by rw [settle_sched hs])
  | observed now k' hs =>
    have e := observe_timing hs
    have e2 := observe_sched hs
    exact SpRel.of_releq ⟨congrArg (·.has_last) e, congrArg spw e⟩
      (fun u => congrArg (fun s => rel s u) e2)
  | deferred q t i d c k' hq hk hw hrel hnd hc hs hmade =>
    have := deferred_release_sprel hs
    split <;> exact this
  | deadline q c t i k' hq hk hw hc hdl hs =>
    exact SpRel.of_rsteps (deadline_miss_sp hs) (deadline_miss_eff hs)
  | plain_h q c k' hq hk hw hc hnd i hcode hs => exact serve_slot_sprel hcode
  | svc_h hsel hk c k' hd hrun hre hal hcode =>
    rcases hcode with e | ⟨kind, t, h⟩
    · rw [e]; exact SpRel.refl
    · exact SpRel.of_releq (record_event_sp h) (fun u => by rw [record_event_sched h])
  | faulted kind kn k' hs => exact SpRel.of_rsteps (fault_sp hs) (fault_r hs)
  | faulted_of kind t k' hs => exact SpRel.of_rsteps (fault_of_sp hs) (fault_of_r hs)
  | bevent_h k0 r0 hset harmed hk c k' hd hrun hrb hc t i hcode hs =>
    exact SpRel.of_rsteps (budget_event_sp hcode) (budget_event_eff hcode)
  | overrun_h k0 r0 hset harmed hk hj c r k' hd hr0 hr hstop hrb hc hal t i hcode hs =>
    exact SpRel.of_rsteps (budget_event_sp hcode) (budget_event_eff hcode)
  | overrun_p k0 r0 hset harmed hk hj c r s1 k' hd hr0 hr hstop hrb hc hal t i hcode hs hrs =>
    exact SpRel.of_rsteps (budget_event_sp hcode) (budget_event_eff hcode)
  | bevent_r k0 r0 hset harmed hk c k' hd hrun hrb hc t i hcode =>
    exact SpRel.of_rsteps (budget_event_sp hcode) (budget_event_eff hcode)
  | ktick => exact SpRel.refl
  | idle => exact SpRel.refl
  | exec => exact SpRel.refl

/-- A transition leaves "an arrival of Task `u` is waiting" as it was, or
sets it (an arrival), or makes a Release of `u` pending. -/
theorem kstep_jwait {cfg : logic.Config NT NR NP} {P : Params}
    {x y : KSt NT NR NP NL NS NE} (h : KStep cfg P x y) (u : Nat) :
    y.jwait u = x.jwait u ∨ y.jwait u = true ∨
    (rel y.k.sched u = 1 ∧ rel x.k.sched u ≠ 1) := by
  cases h with
  | jarrive t ht hsp =>
    dsimp only
    simp only [Function.update_apply]
    split
    · exact Or.inr (Or.inl rfl)
    · exact Or.inl rfl
  | periodic q t i d c k' hq hk hw hrel hnd hc hs hmade =>
    split
    · rename_i hcnd
      dsimp only
      simp only [Function.update_apply]
      split
      · rename_i hu
        subst hu
        exact Or.inr (Or.inr ⟨hcnd.2, by have := hcnd.1; omega⟩)
      · exact Or.inl rfl
    · exact Or.inl rfl
  | intake q t sp off now d c k' hq hk hw hrel hnd hc hs hmade =>
    split
    · rename_i hcnd
      dsimp only
      simp only [Function.update_apply]
      split
      · rename_i hu
        subst hu
        exact Or.inr (Or.inr ⟨hcnd.2, by have := hcnd.1; omega⟩)
      · exact Or.inl rfl
    · exact Or.inl rfl
  | deferred q t i d c k' hq hk hw hrel hnd hc hs hmade =>
    split
    · rename_i hcnd
      dsimp only
      simp only [Function.update_apply]
      split
      · rename_i hu
        subst hu
        exact Or.inr (Or.inr ⟨hcnd.2, by have := hcnd.1; omega⟩)
      · exact Or.inl rfl
    · exact Or.inl rfl
  | _ => exact Or.inl rfl

/-! ## The invariant -/

/-- For Task `u` with last arrival `jl` and "an arrival is waiting" `jw`:
a held Release is deferred to the last due instant plus one MIT; and the
last arrival is at or before the last due instant, or it is waiting and is
exactly one MIT after the last due instant (a deferred event's arrival). -/
def SpAt (cfg : logic.Config NT NR NP) (tk : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (jl : Option Nat) (jw : Bool) (u : Nat) : Prop :=
  (rel k.sched u = 2 → hlOf k u = true ∧ duOf k u = ldOf k u + cfgPeriod cfg u) ∧
  (∀ r, jl = some r → hlOf k u = true ∧
    (r ≤ tk * ldOf k u ∨ (jw = true ∧ r = tk * (ldOf k u + cfgPeriod cfg u))))

theorem spat_step {cfg : logic.Config NT NR NP} {tk : Nat}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {jl : Option Nat} {jw jw' : Bool} {u : Nat}
    (hI : SpAt cfg tk k jl jw u) (hrel : SpAtRel cfg k k' u) (hnp : npk cfg u)
    (hjw : jw' = jw ∨ jw' = true ∨ (rel k'.sched u = 1 ∧ rel k.sched u ≠ 1)) :
    SpAt cfg tk k' jl jw' u := by
  obtain ⟨I1, I2⟩ := hI
  rcases hrel with ⟨ehl, eld, edu, h2, h1⟩ | ⟨hl', hspace, r0, r1⟩ | ⟨hl, hl', eld, edu, rne⟩ |
    ⟨r2, hl', eld, r1⟩
  · refine ⟨?_, ?_⟩
    · intro e
      have := I1 (h2 e)
      rw [ehl, eld, edu]
      exact this
    · intro r hr
      obtain ⟨a, b⟩ := I2 r hr
      refine ⟨by rw [ehl]; exact a, ?_⟩
      rw [eld]
      rcases b with b | ⟨bj, br⟩
      · exact Or.inl b
      · rcases hjw with e | e | ⟨e1, e2⟩
        · exact Or.inr ⟨by rw [e]; exact bj, br⟩
        · exact Or.inr ⟨e, br⟩
        · exact absurd (h1 hnp e1) e2
  · refine ⟨?_, ?_⟩
    · intro e
      omega
    · intro r hr
      obtain ⟨a, b⟩ := I2 r hr
      have hs := hspace a
      refine ⟨hl', Or.inl ?_⟩
      rcases b with b | ⟨_, br⟩
      · exact b.trans (Nat.mul_le_mul_left _ (by omega))
      · rw [br]
        exact Nat.mul_le_mul_left _ hs
  · refine ⟨?_, ?_⟩
    · intro _
      exact ⟨hl', by rw [edu, eld]⟩
    · intro r hr
      obtain ⟨a, b⟩ := I2 r hr
      refine ⟨hl', ?_⟩
      rw [eld]
      rcases b with b | ⟨bj, br⟩
      · exact Or.inl b
      · rcases hjw with e | e | ⟨e1, e2⟩
        · exact Or.inr ⟨by rw [e]; exact bj, br⟩
        · exact Or.inr ⟨e, br⟩
        · exact absurd e1 rne
  · obtain ⟨_, hdu⟩ := I1 r2
    refine ⟨?_, ?_⟩
    · intro e
      omega
    · intro r hr
      obtain ⟨a, b⟩ := I2 r hr
      refine ⟨hl', Or.inl ?_⟩
      rw [eld, hdu]
      rcases b with b | ⟨_, br⟩
      · exact b.trans (Nat.mul_le_mul_left _ (Nat.le_add_right _ _))
      · exact Nat.le_of_eq br

/-! ## The run with sporadic arrivals -/

/-- The time base of a run with the Tasks sorted: `TimeBase`, and which
Tasks are sporadic. A Sporadic_Task is not periodic in the configuration,
no Task is both, and the period the analysis uses for a Sporadic_Task is
not longer than its configured MIT. -/
structure SBase (cfg : logic.Config NT NR NP) (P : Params) extends TimeBase cfg P where
  sporadic : Nat → Bool
  hS : ∀ t, t < NT.val → sporadic t = true → P.T t ≤ tk * cfgPeriod cfg t
  hK : ∀ t, t < NT.val → sporadic t = true → npk cfg t
  hD : ∀ t, sporadic t = true → periodic t = false

/-- The state with an arrival of Task `t` now. -/
def KSt.arrive (x : KSt NT NR NP NL NS NE) (t : Nat) : KSt NT NR NP NL NS NE :=
  { x with jl := Function.update x.jl t (some x.now), jwait := Function.update x.jwait t true }

/-- One transition of the run.

- `parrive`: as in `Sched/Arrive.lean`.
- `darrive`: the Job of a Sporadic_Task whose event the Kernel deferred
  arrives, at the instant the Kernel deferred it to, once.
- `saccept`: the Job of a Sporadic_Task arrives with the transition that
  makes its Release pending from none held (the Kernel accepting the
  event), no earlier arrival of the Task waiting; the instant is the one
  the Kernel records as the Release's due instant.
- `other`: any transition of `KRun.lean` that is no arrival of a
  Periodic_Task or a Sporadic_Task.

None of the three arrivals assumes a spacing from the previous one. -/
inductive SStep (cfg : logic.Config NT NR NP) (P : Params) (B : SBase cfg P) :
    KSt NT NR NP NL NS NE → KSt NT NR NP NL NS NE → Prop
  | parrive (x : KSt NT NR NP NL NS NE) (t : Nat) (ht : t < NT.val)
      (hp : B.periodic t = true)
      (hnow : x.now = B.tk * nomOf x.k t)
      (hfresh : ∀ r, x.jl t = some r → r < x.now) : SStep cfg P B x (x.arrive t)
  | darrive (x : KSt NT NR NP NL NS NE) (t : Nat) (ht : t < NT.val)
      (hs : B.sporadic t = true)
      (hdef : rel x.k.sched t = 2)
      (hnow : x.now = B.tk * duOf x.k t)
      (hfresh : ∀ r, x.jl t = some r → r < x.now) : SStep cfg P B x (x.arrive t)
  | saccept (x y : KSt NT NR NP NL NS NE) (t : Nat) (ht : t < NT.val)
      (hs : B.sporadic t = true)
      (hw : x.jwait t = false)
      (h : KStep cfg P (x.arrive t) y)
      (hmade : rel x.k.sched t = 0 ∧ rel y.k.sched t = 1)
      (hnow : x.now = B.tk * ldOf y.k t)
      (hjl : ∀ u, u < NT.val → (B.periodic u = true ∨ B.sporadic u = true) →
        y.jl u = (x.arrive t).jl u) : SStep cfg P B x y
  | other (x y : KSt NT NR NP NL NS NE) (h : KStep cfg P x y)
      (hjl : ∀ u, u < NT.val → (B.periodic u = true ∨ B.sporadic u = true) → y.jl u = x.jl u) :
      SStep cfg P B x y

/-- The states the run reaches from boot. -/
inductive SReach (cfg : logic.Config NT NR NP) (P : Params) (B : SBase cfg P) :
    KSt NT NR NP NL NS NE → Prop
  | boot (k0 : logic.kernel.Kernel NT NR NP NL NS NE)
      (h : logic.sched.Sched.new NL cfg = ok k0.sched) : SReach cfg P B (KSt.boot k0)
  | step (x y : KSt NT NR NP NL NS NE) (h : SReach cfg P B x) (hs : SStep cfg P B x y) :
      SReach cfg P B y

/-- `SpAt` for every Sporadic_Task of the run's state. -/
def SpInv (cfg : logic.Config NT NR NP) (P : Params) (B : SBase cfg P)
    (x : KSt NT NR NP NL NS NE) : Prop :=
  ∀ u, u < NT.val → B.sporadic u = true → SpAt cfg B.tk x.k (x.jl u) (x.jwait u) u

/-- `ArrInv` is kept by a transition that leaves the Periodic_Tasks' last
arrivals and moves each nominal instant by a period or not at all. -/
theorem arrinv_step {cfg : logic.Config NT NR NP} {P : Params} {B : TimeBase cfg P}
    {x y : KSt NT NR NP NL NS NE} (hinv : ArrInv cfg P B x)
    (hjl : ∀ u, u < NT.val → B.periodic u = true → y.jl u = x.jl u)
    (hn : NomRel cfg x.k y.k) : ArrInv cfg P B y := by
  intro u hu hpu r hr
  rw [hjl u hu hpu] at hr
  obtain ⟨j, hj⟩ := hinv u hu hpu r hr
  rcases hn u hu with e | e
  · exact ⟨j, by rw [e]; exact hj⟩
  · refine ⟨j + 1, ?_⟩
    rw [e, Nat.mul_add, hj, Nat.add_mul, Nat.one_mul, Nat.add_assoc]

/-- Every run with sporadic arrivals is a run of `KRun.lean`: the spacing
of the arrivals of a Periodic_Task and of a Sporadic_Task, which that run
takes as a condition, holds. -/
theorem sreach_kreach {cfg : logic.Config NT NR NP} {P : Params} {B : SBase cfg P}
    {x : KSt NT NR NP NL NS NE} (h : SReach cfg P B x) :
    KReach cfg P x ∧ ArrInv cfg P B.toTimeBase x ∧ SpInv cfg P B x := by
  induction h with
  | boot k0 h0 =>
    refine ⟨KReach.boot k0 h0, ?_, ?_⟩
    · intro t _ _ r hr
      simp [KSt.boot] at hr
    · intro u hu _
      refine ⟨?_, ?_⟩
      · intro e
        have := (new_views h0).1 u hu
        simp only [KSt.boot] at e
        omega
      · intro r hr
        simp [KSt.boot] at hr
  | step x y _ hs ih =>
    obtain ⟨hk, hinv, hsp⟩ := ih
    cases hs with
    | parrive t ht hp hnow hfresh =>
      refine ⟨KReach.step _ _ hk (KStep.jarrive x t ht ?_), ?_, ?_⟩
      · intro r hr
        obtain ⟨j, hj⟩ := hinv t ht hp r hr
        have hlt := hfresh r hr
        have hT := B.hT t ht hp
        rw [hnow, hj] at hlt
        rw [hnow, hj]
        have hj1 : 1 ≤ j := by
          rcases Nat.eq_zero_or_pos j with h0 | h0
          · subst h0; simp at hlt
          · exact h0
        have : B.tk * cfgPeriod cfg t ≤ j * (B.tk * cfgPeriod cfg t) := Nat.le_mul_of_pos_left _ hj1
        omega
      · intro u hu hpu r hr
        simp only [KSt.arrive] at hr ⊢
        by_cases hut : u = t
        · subst hut
          simp only [Function.update_self, Option.some.injEq] at hr
          subst hr
          exact ⟨0, by rw [hnow]; simp⟩
        · rw [Function.update_of_ne hut] at hr
          exact hinv u hu hpu r hr
      · intro u hu hsu
        have hut : u ≠ t := by
          rintro rfl
          rw [B.hD _ hsu] at hp
          cases hp
        have := hsp u hu hsu
        simp only [KSt.arrive, Function.update_of_ne hut]
        exact this
    | darrive t ht hs hdef hnow hfresh =>
      obtain ⟨I1, I2⟩ := hsp t ht hs
      obtain ⟨hhl, hdu⟩ := I1 hdef
      have hS := B.hS t ht hs
      refine ⟨KReach.step _ _ hk (KStep.jarrive x t ht ?_), ?_, ?_⟩
      · intro r hr
        obtain ⟨_, b⟩ := I2 r hr
        have hlt := hfresh r hr
        rw [hnow, hdu, Nat.mul_add] at hlt ⊢
        rcases b with b | ⟨_, br⟩
        · omega
        · rw [Nat.mul_add] at br
          omega
      · refine arrinv_step hinv ?_ (NomRel.of_eq rfl)
        intro u hu hpu
        have hut : u ≠ t := by
          rintro rfl
          rw [B.hD _ hs] at hpu
          cases hpu
        simp only [KSt.arrive, Function.update_of_ne hut]
      · intro u hu hsu
        by_cases hut : u = t
        · subst hut
          simp only [KSt.arrive, Function.update_self]
          refine ⟨I1, ?_⟩
          intro r hr
          simp only [Option.some.injEq] at hr
          subst hr
          exact ⟨hhl, Or.inr ⟨rfl, by rw [hnow, hdu]⟩⟩
        · have := hsp u hu hsu
          simp only [KSt.arrive, Function.update_of_ne hut]
          exact this
    | saccept _ t ht hs hw h hmade hnow hjl =>
      have hR := kstep_sprel h
      have hN : NomRel cfg (x.arrive t).k y.k := kstep_nomrel h
      have hacc : hlOf y.k t = true ∧
          (hlOf x.k t = true → ldOf x.k t + cfgPeriod cfg t ≤ ldOf y.k t) := by
        rcases hR t ht with ⟨_, _, _, _, h1⟩ | ⟨a, b, _, _⟩ | ⟨_, _, _, _, rne⟩ | ⟨r2, _⟩
        · have := h1 (B.hK t ht hs) hmade.2
          have := hmade.1
          simp only [KSt.arrive] at *
          omega
        · exact ⟨a, b⟩
        · exact absurd hmade.2 rne
        · have := hmade.1
          simp only [KSt.arrive] at *
          omega
      have hS := B.hS t ht hs
      refine ⟨KReach.step _ _ (KReach.step _ _ hk (KStep.jarrive x t ht ?_)) h, ?_, ?_⟩
      · intro r hr
        obtain ⟨a, b⟩ := (hsp t ht hs).2 r hr
        rcases b with b | ⟨bj, _⟩
        · have hsp2 := hacc.2 a
          have : B.tk * (ldOf x.k t + cfgPeriod cfg t) ≤ B.tk * ldOf y.k t :=
            Nat.mul_le_mul_left _ hsp2
          rw [Nat.mul_add] at this
          rw [hnow]
          omega
        · rw [hw] at bj
          cases bj
      · refine arrinv_step hinv ?_ hN
        intro u hu hpu
        have hut : u ≠ t := by
          rintro rfl
          rw [B.hD _ hs] at hpu
          cases hpu
        rw [hjl u hu (Or.inl hpu)]
        simp only [KSt.arrive, Function.update_of_ne hut]
      · intro u hu hsu
        rw [hjl u hu (Or.inr hsu)]
        by_cases hut : u = t
        · subst hut
          simp only [KSt.arrive, Function.update_self]
          refine ⟨?_, ?_⟩
          · intro e
            have := hmade.2
            omega
          · intro r hr
            simp only [Option.some.injEq] at hr
            subst hr
            exact ⟨hacc.1, Or.inl (Nat.le_of_eq hnow)⟩
        · have hj := kstep_jwait h u
          simp only [KSt.arrive, Function.update_of_ne hut] at hj ⊢
          exact spat_step (hsp u hu hsu) (hR u hu) (B.hK u hu hsu) hj
    | other _ h hjl =>
      refine ⟨KReach.step _ _ hk h, ?_, ?_⟩
      · exact arrinv_step hinv (fun u hu hpu => hjl u hu (Or.inl hpu)) (kstep_nomrel h)
      · intro u hu hsu
        rw [hjl u hu (Or.inr hsu)]
        exact spat_step (hsp u hu hsu) (kstep_sprel h u hu) (B.hK u hu hsu) (kstep_jwait h u)

open rsk_analyzer (candidateOf) in
open rsk_analyzer.terms (TaskIn Site Window Verdict) in
/-- Property 18 over the run with sporadic arrivals: the bound
`analyse_all` reports holds in every run in which the Jobs of the
Periodic_Tasks arrive at the Kernel's nominal instants and the Jobs of the
Sporadic_Tasks at the instants the Kernel records for their Releases,
with no assumption on the spacing of either. -/
theorem property18_sporadic {cfg : logic.Config NT NR NP}
    (tasks : alloc.vec.Vec TaskIn) (sites : alloc.vec.Vec Site) (p : rsk_analyzer.terms.Params)
    (any_fpu : Bool) (windows : alloc.vec.Vec Window)
    (ds : alloc.vec.Vec rsk_analyzer.terms.Derived) (vs : alloc.vec.Vec Verdict)
    (tss : alloc.vec.Vec (alloc.vec.Vec rsk_analyzer.terms.TermOf)) (quiet : alloc.vec.Vec Bool)
    (L : Nat → Nat → Nat)
    (hall : rsk_analyzer.terms.analyse_all tasks sites p any_fpu windows
      = ok (some (ds, vs, tss, quiet)))
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length)
    (hwl : windows.val.length = tasks.val.length)
    (hprio : ∀ k t, tasks.val[k]? = some t → 0 < t.priority.val ∧ prio cfg k = t.priority.val)
    (hbud : ∀ t ∈ tasks.val, 0 < t.budget.val)
    (hDT : ∀ w ∈ windows.val, w.deadline.val ≤ w.period.val)
    (hL : ∀ i ti k r, tasks.val[i]? = some ti → k < NT.val → prio cfg k < prio cfg i →
      prio cfg i ≤ ceil cfg r →
      L k r = 0 ∨
      ∃ (j : Nat) (s : Site) (l : TaskIn), sites.val[j]? = some s
        ∧ tasks.val[s.task.val]? = some l
        ∧ l.priority.val < ti.priority.val ∧ ti.priority.val ≤ s.ceiling.val
        ∧ L k r ≤ candidateOf ti l s p)
    (B : SBase cfg (runParams NT.val tasks.val sites.val p windows.val L))
    (i : Nat) (v : Verdict) (hv : vs.val[i]? = some v) (hsched : v.schedulable = true)
    {x : KSt NT NR NP NL NS NE}
    (hreach : SReach cfg (runParams NT.val tasks.val sites.val p windows.val L) B x)
    (hpos : 0 < x.pend i + x.run i) :
    ∃ a, x.jl i = some a ∧ x.now < a + v.r.val :=
  property18_kernel tasks sites p any_fpu windows ds vs tss quiet L hall hlen hNT hsok hwl hprio
    hbud hDT hL i v hv hsched (sreach_kreach hreach).1 hpos

end P18
