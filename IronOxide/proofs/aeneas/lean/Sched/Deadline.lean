import Sched.Slots
open Aeneas Aeneas.Std Result

/-! # An armed deadline slot belongs to a Job that is pending or in progress

The fact behind the run's condition "a deadline event has an entry only on
a miss", proven on the Kernel's functions as extracted. In Verus it is a
pair of postconditions (`job_return`, `end_top_release`); there is no
invariant over the run there. Here it is one:

  outside the safe state, a timer slot armed with the deadline kind names a
  Task of the configuration, is that Task's deadline slot, and the Task has
  a Release pending or a Job in progress.

- `DG cfg E b sc sl`: the invariant for the safe-state flag `b`, scheduler
  state `sc` and slot table `sl`, with the Tasks of `E` exempt. The
  exemption is what lets the statement pass through the places where it is
  false for a moment: a Job ended (`Sched::finish_top`) just before its
  deadline slot is disarmed, the Jobs of a Partition killed just before
  their slots are, a Release dropped just before its slot is.
- `*_dx`: each Kernel function of the run keeps it, with no Task exempt
  before or after.
- `kstep_di`, `kreachs_di`: so does every transition, and every state the
  run reaches from a boot state with no deadline slot armed.
- `deadline_slot_incomplete`: the consequence in the run's accounts: the
  Task of an armed deadline slot has a Job with work left.

The safe state is excluded because `Kernel::enter_safe_state` drops every
Release and disarms no slot. The Tasks number at most 256 (`Fact`), as the
Kernel's `u8` Task index requires. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- No Task exempt. -/
def NoEx : Nat → Prop := fun _ => False
/-- The Tasks of `E`, and Task `t`. -/
def Ex (E : Nat → Prop) (t : Nat) : Nat → Prop := fun τ => E τ ∨ τ = t

/-- Outside the safe state, an armed slot of the deadline kind names a Task
below `NT`, is that Task's deadline slot, and the Task has a Release
pending or a Job in progress, or is exempt. -/
def DG (cfg : logic.Config NT NR NP) (E : Nat → Prop) (b : Bool)
    (sc : logic.sched.Sched NT NR NL) (sl : logic.time.Slots NS) : Prop :=
  b = false → ∀ i, i < NS.val → (slotAt sl i).armed = true →
    (slotAt sl i).kind = logic.time.SLOT_DEADLINE →
    (slotAt sl i).task.val < NT.val ∧
    (cfg.slot_deadline.val[(slotAt sl i).task.val]!).val = i ∧
    (rel sc (slotAt sl i).task.val = 1 ∨ job sc (slotAt sl i).task.val = 1 ∨
      E (slotAt sl i).task.val)

/-- The chain through the calls of a function. -/
@[reducible] def DX (cfg : logic.Config NT NR NP)
    (E : Nat → Prop) (b : Bool) (sc : logic.sched.Sched NT NR NL) (sl : logic.time.Slots NS)
    (E' : Nat → Prop) (b' : Bool) (sc' : logic.sched.Sched NT NR NL) (sl' : logic.time.Slots NS) :
    Prop :=
  DG cfg E b sc sl → DG cfg E' b' sc' sl'

/-- A Kernel function keeps the invariant, no Task exempt. -/
@[reducible] def DXK (cfg : logic.Config NT NR NP)
    (k k' : logic.kernel.Kernel NT NR NP NL NS NE) : Prop :=
  DX cfg NoEx k.safe_state k.sched k.slots NoEx k'.safe_state k'.sched k'.slots

theorem dg_mono {cfg : logic.Config NT NR NP} {E E' : Nat → Prop} {b : Bool}
    {sc : logic.sched.Sched NT NR NL} {sl : logic.time.Slots NS}
    (hE : ∀ τ, E τ → E' τ) : DX cfg E b sc sl E' b sc sl := by
  intro h hb i hi ha hk
  obtain ⟨h1, h2, h3⟩ := h hb i hi ha hk
  refine ⟨h1, h2, ?_⟩
  rcases h3 with h | h | h
  · exact Or.inl h
  · exact Or.inr (Or.inl h)
  · exact Or.inr (Or.inr (hE _ h))

theorem dx_trans {cfg : logic.Config NT NR NP} {E Em E' : Nat → Prop} {b bm b' : Bool}
    {sc scm sc' : logic.sched.Sched NT NR NL} {sl slm sl' : logic.time.Slots NS}
    (g1 : DX cfg E b sc sl Em bm scm slm) (g2 : DX cfg Em bm scm slm E' b' sc' sl') :
    DX cfg E b sc sl E' b' sc' sl' :=
  fun h => g2 (g1 h)

/-- In the safe state nothing is claimed. -/
theorem dx_safe {cfg : logic.Config NT NR NP} {E E' : Nat → Prop} {b : Bool}
    {sc sc' : logic.sched.Sched NT NR NL} {sl sl' : logic.time.Slots NS} :
    DX cfg E b sc sl E' true sc' sl' :=
  fun _ hb => by cases hb

/-! ## Slots -/

theorem dx_disarm {cfg : logic.Config NT NR NP} {E E' : Nat → Prop} {b b' : Bool}
    {sc sc' : logic.sched.Sched NT NR NL} {sl slm sl' : logic.time.Slots NS} {j : Usize}
    (h : logic.time.Slots.disarm slm j = ok sl') (g : DX cfg E b sc sl E' b' sc' slm) :
    DX cfg E b sc sl E' b' sc' sl' := by
  intro hd hb i hi ha hk
  rw [(disarm_at h i).2] at ha hk ⊢
  split at ha
  · simp at ha
  · rename_i hne
    rw [if_neg hne] at hk ⊢
    exact g hd hb i hi ha hk

theorem dx_arm {cfg : logic.Config NT NR NP} {E E' : Nat → Prop} {b b' : Bool}
    {sc sc' : logic.sched.Sched NT NR NL} {sl slm sl' : logic.time.Slots NS} {j : Usize}
    {a : U64} {kind task : U8}
    (h : logic.time.Slots.arm slm j a kind task = ok sl')
    (hkind : kind ≠ logic.time.SLOT_DEADLINE)
    (g : DX cfg E b sc sl E' b' sc' slm) : DX cfg E b sc sl E' b' sc' sl' := by
  intro hd hb i hi ha hk
  rw [(arm_at h i).2] at ha hk ⊢
  split at hk
  · exact absurd hk hkind
  · rename_i hne
    rw [if_neg hne] at ha ⊢
    exact g hd hb i hi ha hk

theorem slot_deadline_idx {cfg : logic.Config NT NR NP} {t b : U8}
    (h : cfg.slot_deadline.index_usize (UScalar.cast .Usize t) = ok b) :
    (cfg.slot_deadline.val[t.val]!).val = (UScalar.cast .Usize b : Usize).val := by
  obtain ⟨_, e⟩ := index_ok h
  rw [U8.cast_Usize_val_eq] at e
  rw [U8.cast_Usize_val_eq, e]

/-- Whatever is written to the deadline slot of Task `t` removes `t` from
the exempt Tasks: a slot that names `t` with the deadline kind is that
slot. -/
theorem dx_over {cfg : logic.Config NT NR NP} {E E' : Nat → Prop} {b b' : Bool}
    {sc sc' : logic.sched.Sched NT NR NL} {sl slm sl' : logic.time.Slots NS} {j : Usize}
    {t : Nat} {new : logic.time.Slot}
    (hat : ∀ i, j.val < NS.val ∧ slotAt sl' i = if i = j.val then new else slotAt slm i)
    (hnew : ¬ (new.armed = true ∧ new.kind = logic.time.SLOT_DEADLINE))
    (hj : (cfg.slot_deadline.val[t]!).val = j.val)
    (g : DX cfg E b sc sl (Ex E' t) b' sc' slm) : DX cfg E b sc sl E' b' sc' sl' := by
  intro hd hb i hi ha hk
  rw [(hat i).2] at ha hk ⊢
  split at ha
  · exact absurd ⟨ha, by rename_i he; rw [if_pos he] at hk; exact hk⟩ hnew
  · rename_i hne
    rw [if_neg hne] at hk ⊢
    obtain ⟨h1, h2, h3⟩ := g hd hb i hi ha hk
    refine ⟨h1, h2, ?_⟩
    rcases h3 with h | h | h
    · exact Or.inl h
    · exact Or.inr (Or.inl h)
    · rcases h with h | h
      · exact Or.inr (Or.inr h)
      · rw [h, hj] at h2
        exact absurd h2.symm hne

theorem dx_disarm_ex {cfg : logic.Config NT NR NP} {E E' : Nat → Prop} {b b' : Bool}
    {sc sc' : logic.sched.Sched NT NR NL} {sl slm sl' : logic.time.Slots NS} {t d : U8}
    (h : logic.time.Slots.disarm slm (UScalar.cast .Usize d) = ok sl')
    (hsd : cfg.slot_deadline.index_usize (UScalar.cast .Usize t) = ok d)
    (g : DX cfg E b sc sl (Ex E' t.val) b' sc' slm) : DX cfg E b sc sl E' b' sc' sl' :=
  dx_over (disarm_at h) (by simp) (slot_deadline_idx hsd) g

/-- The same with the Task given as an index (the loop of
`clear_partition_releases`). -/
theorem dx_disarm_ex_u {cfg : logic.Config NT NR NP} {E E' : Nat → Prop} {b b' : Bool}
    {sc sc' : logic.sched.Sched NT NR NL} {sl slm sl' : logic.time.Slots NS} {t : Usize} {d : U8}
    (h : logic.time.Slots.disarm slm (UScalar.cast .Usize d) = ok sl')
    (hsd : cfg.slot_deadline.index_usize t = ok d)
    (g : DX cfg E b sc sl (Ex E' t.val) b' sc' slm) : DX cfg E b sc sl E' b' sc' sl' :=
  dx_over (disarm_at h) (by simp)
    (by obtain ⟨_, e⟩ := index_ok hsd; rw [U8.cast_Usize_val_eq, e]) g

theorem dx_arm_ex_u {cfg : logic.Config NT NR NP} {E E' : Nat → Prop} {b b' : Bool}
    {sc sc' : logic.sched.Sched NT NR NL} {sl slm sl' : logic.time.Slots NS} {t : Usize} {d : U8}
    {a : U64} {kind task : U8}
    (h : logic.time.Slots.arm slm (UScalar.cast .Usize d) a kind task = ok sl')
    (hkind : kind ≠ logic.time.SLOT_DEADLINE)
    (hsd : cfg.slot_deadline.index_usize t = ok d)
    (g : DX cfg E b sc sl (Ex E' t.val) b' sc' slm) : DX cfg E b sc sl E' b' sc' sl' :=
  dx_over (arm_at h) (fun hh => hkind hh.2)
    (by obtain ⟨_, e⟩ := index_ok hsd; rw [U8.cast_Usize_val_eq, e]) g

/-- Arming the deadline slot of Task `t` for `t`, a Release of `t` pending. -/
theorem dx_arm_deadline {cfg : logic.Config NT NR NP} {E E' : Nat → Prop} {b b' : Bool}
    {sc sc' : logic.sched.Sched NT NR NL} {sl slm sl' : logic.time.Slots NS} {t d : U8}
    {a : U64}
    (h : logic.time.Slots.arm slm (UScalar.cast .Usize d) a logic.time.SLOT_DEADLINE t = ok sl')
    (hsd : cfg.slot_deadline.index_usize (UScalar.cast .Usize t) = ok d)
    (hrel : rel sc' t.val = 1 ∧ t.val < NT.val)
    (g : DX cfg E b sc sl E' b' sc' slm) : DX cfg E b sc sl E' b' sc' sl' := by
  intro hd hb i hi ha hk
  rw [(arm_at h i).2] at ha hk ⊢
  split at hk
  · rename_i he
    rw [if_pos he]
    exact ⟨hrel.2, by rw [he]; exact slot_deadline_idx hsd, Or.inl hrel.1⟩
  · rename_i hne
    rw [if_neg hne] at ha ⊢
    exact g hd hb i hi ha hk

/-! ## The scheduler state -/

/-- A Task with a Release pending or a Job in progress still has one. -/
def SMono (s s' : logic.sched.Sched NT NR NL) : Prop :=
  ∀ τ, (rel s τ = 1 ∨ job s τ = 1) → (rel s' τ = 1 ∨ job s' τ = 1)

theorem dx_sched {cfg : logic.Config NT NR NP} {E E' : Nat → Prop} {b b' : Bool}
    {sc scm sc' : logic.sched.Sched NT NR NL} {sl sl' : logic.time.Slots NS}
    (hs : SMono scm sc') (g : DX cfg E b sc sl E' b' scm sl') :
    DX cfg E b sc sl E' b' sc' sl' := by
  intro hd hb i hi ha hk
  obtain ⟨h1, h2, h3⟩ := g hd hb i hi ha hk
  refine ⟨h1, h2, ?_⟩
  rcases h3 with h | h | h
  · rcases hs _ (Or.inl h) with h | h
    · exact Or.inl h
    · exact Or.inr (Or.inl h)
  · rcases hs _ (Or.inr h) with h | h
    · exact Or.inl h
    · exact Or.inr (Or.inl h)
  · exact Or.inr (Or.inr h)

/-- A change of the scheduler state that touches Task `t` only, `t` exempt. -/
theorem dx_sched_ex {cfg : logic.Config NT NR NP} {E E' : Nat → Prop} {b b' : Bool}
    {sc scm sc' : logic.sched.Sched NT NR NL} {sl sl' : logic.time.Slots NS} {t : Nat}
    (hs : ∀ τ, τ ≠ t → rel sc' τ = rel scm τ ∧ job sc' τ = job scm τ)
    (g : DX cfg E b sc sl (Ex E' t) b' scm sl') : DX cfg E b sc sl (Ex E' t) b' sc' sl' := by
  intro hd hb i hi ha hk
  obtain ⟨h1, h2, h3⟩ := g hd hb i hi ha hk
  refine ⟨h1, h2, ?_⟩
  by_cases e : (slotAt sl' i).task.val = t
  · exact Or.inr (Or.inr (Or.inr e))
  · rw [(hs _ e).1, (hs _ e).2]
    exact h3

theorem mono_make_pending {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : U8} (h : logic.sched.Sched.make_pending s cfg t = ok s') : SMono s s' := by
  obtain ⟨_, hrel, hjob, _⟩ := make_pending_views h
  intro τ hτ
  rw [hrel, hjob]
  split
  · exact Or.inl rfl
  · exact hτ

theorem mp_rel {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : U8} (h : logic.sched.Sched.make_pending s cfg t = ok s') :
    rel s' t.val = 1 ∧ t.val < NT.val := by
  obtain ⟨ht, hrel, _⟩ := make_pending_views h
  exact ⟨by rw [hrel]; simp, ht⟩

theorem mono_start {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : U8} (h : logic.sched.Sched.start s cfg t = ok s') : SMono s s' := by
  obtain ⟨_, _, hrel, hjob, _⟩ := start_views h
  intro τ hτ
  rw [hrel, hjob]
  split
  · exact Or.inr rfl
  · exact hτ

theorem rel_at_index {s : logic.sched.Sched NT NR NL} {t r : U8}
    (h : s.release.index_usize (UScalar.cast .Usize t) = ok r) : rel s t.val = r.val := by
  obtain ⟨_, e⟩ := index_ok h
  rw [U8.cast_Usize_val_eq] at e
  unfold rel
  rw [e]

/-- `Sched::defer`, called with no Release of the Task held. -/
theorem mono_defer {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t r : U8} (h : logic.sched.Sched.defer s cfg t = ok s')
    (hidx : s.release.index_usize (UScalar.cast .Usize t) = ok r)
    (hr : r = logic.sched.REL_NONE) : SMono s s' := by
  obtain ⟨_, hrel, hjob, _⟩ := defer_views h
  have h0 : rel s t.val = 0 := by rw [rel_at_index hidx, hr, rel_none]; rfl
  intro τ hτ
  rw [hrel, hjob]
  split
  · rename_i e
    rw [e] at hτ
    rcases hτ with h | h
    · omega
    · exact Or.inr (by rw [e]; exact h)
  · exact hτ

/-- `Sched::clear_release` of a Release that is deferred, not pending. -/
theorem mono_clear_deferred {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t r : U8} (h : logic.sched.Sched.clear_release s cfg t = ok s')
    (hidx : s.release.index_usize (UScalar.cast .Usize t) = ok r)
    (hr : ¬ (r != logic.sched.REL_DEFERRED) = true) : SMono s s' := by
  obtain ⟨_, hrel, hjob, _⟩ := clear_release_views h
  have e : r = logic.sched.REL_DEFERRED := by
    by_contra hc
    exact hr (bne_iff_ne.mpr hc)
  have h2 : rel s t.val = 2 := by rw [rel_at_index hidx, e, rel_deferred]; rfl
  intro τ hτ
  rw [hrel, hjob]
  split
  · rename_i e
    rw [e] at hτ
    rcases hτ with h | h
    · omega
    · exact Or.inr (by rw [e]; exact h)
  · exact hτ

theorem finish_ex {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {r i1 : Usize} {t : U8}
    (h : logic.sched.Sched.finish_top s cfg = ok (r, s'))
    (hi : s.depth - 1#usize = ok i1) (hp : s.stack.index_usize i1 = ok t) :
    ∀ τ, τ ≠ t.val → rel s' τ = rel s τ ∧ job s' τ = job s τ := by
  obtain ⟨_, _, _, hrel, hjob, _⟩ := finish_top_views h
  intro τ hne
  rw [hrel, hjob, (top_of hi hp).2, if_neg hne]
  exact ⟨rfl, rfl⟩

theorem end_job_ex {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {r i1 : Usize} {t : U8}
    (h : logic.sched.Sched.end_job s cfg = ok (r, s'))
    (hi : s.depth - 1#usize = ok i1) (hp : s.stack.index_usize i1 = ok t) :
    ∀ τ, τ ≠ t.val → rel s' τ = rel s τ ∧ job s' τ = job s τ :=
  finish_ex (by rw [finish_top_eq]; exact h) hi hp

/-- A Release of Task `t` is pending in `sc'`: `t` need not be exempt. -/
theorem dx_pending {cfg : logic.Config NT NR NP} {E : Nat → Prop} {b b' : Bool}
    {sc sc' : logic.sched.Sched NT NR NL} {sl sl' : logic.time.Slots NS} {t r : U8}
    (hidx : sc'.release.index_usize (UScalar.cast .Usize t) = ok r)
    (hr : ¬ (r != logic.sched.REL_PENDING) = true)
    (g : DX cfg E b sc sl (Ex NoEx t.val) b' sc' sl') : DX cfg E b sc sl NoEx b' sc' sl' := by
  have e : r = logic.sched.REL_PENDING := by
    by_contra hc
    exact hr (bne_iff_ne.mpr hc)
  have h1 : rel sc' t.val = 1 := by rw [rel_at_index hidx, e, rel_pending]; rfl
  intro hd hb i hi ha hk
  obtain ⟨g1, g2, g3⟩ := g hd hb i hi ha hk
  refine ⟨g1, g2, ?_⟩
  rcases g3 with h | h | h
  · exact Or.inl h
  · exact Or.inr (Or.inl h)
  · rcases h with h | h
    · exact absurd h id
    · rw [h]; exact Or.inl h1

/-! ## The chain tactic -/

/-- A nested call of a Kernel function already shown to keep the invariant. -/
syntax "dx_call" : tactic
macro_rules | `(tactic| dx_call) => `(tactic| fail "no call")

/-- One step back through the function's calls. -/
syntax "dx_step" : tactic
macro_rules
  | `(tactic| dx_step) => `(tactic| first
    | exact dx_safe
    | with_reducible exact fun h => h
    | exact dg_mono (fun _ h => Or.inl h)
    | refine dx_pending (by assumption) (by assumption) ?_
    | refine dx_disarm_ex (by assumption) (by assumption) ?_
    | refine dx_disarm (by assumption) ?_
    | refine dx_arm_deadline (by assumption) (by assumption) (mp_rel (by assumption)) ?_
    | refine dx_arm (by assumption)
        (by simp [logic.time.SLOT_UNMASK, logic.time.SLOT_BUDGET, logic.time.SLOT_RELEASE,
          logic.time.SLOT_DEFER, logic.time.SLOT_DEADLINE, logic.time.SLOT_END,
          logic.time.SLOT_HOUSEKEEPING]) ?_
    | refine dx_sched (mono_make_pending (by assumption)) ?_
    | refine dx_sched (mono_start (by assumption)) ?_
    | refine dx_sched (mono_defer (by assumption) (by assumption) (by assumption)) ?_
    | refine dx_sched (mono_clear_deferred (by assumption) (by assumption) (by assumption)) ?_
    | refine dx_sched_ex (finish_ex (by assumption) (by assumption) (by assumption)) ?_
    | refine dx_sched_ex (end_job_ex (by assumption) (by assumption) (by assumption)) ?_
    | dx_call)

/-- Closes `DXK cfg k k'` after `peel`. -/
syntax "dx_close" : tactic
macro_rules
  | `(tactic| dx_close) =>
    `(tactic| (show DX _ _ _ _ _ _ _ _ _; repeat (first | dx_step | dsimp only)))

/-! ## The Kernel functions -/

theorem push_event_safe {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    k'.safe_state = k.safe_state := by
  unfold logic.kernel.Kernel.push_event at h
  frame_ok h

/-- `push_event` changes none of the three, whatever Tasks are exempt. -/
theorem push_event_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind task partition : U8} {prs : U64} {detail : U32}
    {E : Nat → Prop}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    DX cfg E k.safe_state k.sched k.slots E k'.safe_state k'.sched k'.slots := by
  show DG _ _ _ _ _ → DG _ _ _ _ _
  rw [push_event_slots h, push_event_sched h, push_event_safe h]
  exact fun h => h

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (push_event_dx (by assumption)))

theorem arm_budget_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.arm_budget at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (arm_budget_dx (by assumption)))

theorem end_mit_window_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.end_mit_window at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (end_mit_window_dx (by assumption)))

theorem enter_safe_state_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} 
    (h : logic.kernel.Kernel.enter_safe_state k cfg = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (enter_safe_state_dx (by assumption)))

theorem end_top_release_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} 
    (h : logic.kernel.Kernel.end_top_release k cfg = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.end_top_release at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (end_top_release_dx (by assumption)))

theorem end_top_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} 
    (h : logic.kernel.Kernel.end_top k cfg = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.end_top at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (end_top_dx (by assumption)))

set_option maxHeartbeats 8000000 in
theorem job_return_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {ec : U32}
    (h : logic.kernel.Kernel.job_return k cfg ec = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.job_return at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (job_return_dx (by assumption)))

set_option maxHeartbeats 8000000 in
theorem dispatch_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {pc : U32}
    (h : logic.kernel.Kernel.dispatch k cfg t pc = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.dispatch at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (dispatch_dx (by assumption)))

/-! ## A Partition stopped or restarted

`end_partition_jobs` kills the Partition's Jobs and then, Task by Task,
drops the Release and disarms or overwrites the deadline slot. Between the
two the invariant holds with the Partition's Tasks exempt. -/

/-- Task `τ` is of Partition `p`. -/
def InPart (cfg : logic.Config NT NR NP) (p : U8) (τ : Nat) : Prop :=
  ∃ tc, cfg.tasks.val[τ]? = some tc ∧ tc.partition = p

theorem kill_at_dg {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize} {E : Nat → Prop}
    (h : logic.kernel.Kernel.kill_at k cfg i = ok k') (hE : E (stk k.sched i.val)) :
    DX cfg E k.safe_state k.sched k.slots E k'.safe_state k'.sched k'.slots := by
  unfold logic.kernel.Kernel.kill_at at h
  peel h
  rename_i u hu s1 hk
  show DX _ _ _ _ _ _ _ _ _
  dsimp only
  intro hd hb j hj ha hkd
  obtain ⟨h1, h2, h3⟩ := hd hb j hj ha hkd
  obtain ⟨_, hrel, hjob, _⟩ := kill_views hk
  refine ⟨h1, h2, ?_⟩
  rw [hrel, hjob]
  by_cases e : (slotAt k.slots j).task.val = stk k.sched i.val
  · exact Or.inr (Or.inr (by rw [e]; exact hE))
  · rw [if_neg e]; exact h3

theorem kill_loop_dg {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) res,
      k.sched.depth.val - i.val = n →
      logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i = ok res →
      ∃ k2 : logic.kernel.Kernel NT NR NP NL NS NE,
        DX cfg (InPart cfg partition) k.safe_state k.sched k.slots
          (InPart cfg partition) k2.safe_state k2.sched k2.slots ∧
        res = (k2.sched.release, k2.sched.job, k2.sched.stack, k2.sched.depth,
          k2.sched.start_ceiling, k2.sched.ceiling, k2.sched.lock_res, k2.sched.lock_depth,
          k2.sched.lock_len, k2.timing, k2.slots, k2.budget, k2.log, k2.stopped,
          k2.mit_violations, k2.reinit_partition, k2.stop_dma, k2.safe_state, k2.started,
          k2.now, k2.history) := by
  intro n
  induction n with
  | zero =>
    intro k i res hn h
    unfold logic.kernel.Kernel.kill_partition_jobs_loop loop at h
    simp only [logic.kernel.Kernel.kill_partition_jobs_loop.body] at h
    have hlt : ¬ i < k.sched.depth := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    exact ⟨k, fun h => h, h.symm⟩
  | succ n ih =>
    intro k i res hn h
    unfold logic.kernel.Kernel.kill_partition_jobs_loop loop at h
    simp only [logic.kernel.Kernel.kill_partition_jobs_loop.body] at h
    have hiv : i.val < k.sched.depth.val := by omega
    have hlt : i < k.sched.depth := by scalar_tac
    simp only [hlt, ite_true, lift, bind_ok, bind_eq_ok] at h
    obtain ⟨cf, ⟨u, hu, tc, htc, self1, hself1, i2, hi2, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    have hi2 := usize_add_ok hi2
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at hi2
    have hrec : logic.kernel.Kernel.kill_partition_jobs_loop self1 cfg partition i2 = ok res := by
      unfold logic.kernel.Kernel.kill_partition_jobs_loop
      simp only [logic.kernel.Kernel.kill_partition_jobs_loop.body, lift, bind_ok]
      exact hloop
    have hst : stk k.sched i.val = u.val := by
      obtain ⟨_, e⟩ := index_ok hu
      unfold stk
      rw [e]
    have htc' : cfg.tasks.val[u.val]? = some tc := by
      have := (index_ok' htc).2
      rwa [U8.cast_Usize_val_eq] at this
    have hcase : self1.sched.depth = k.sched.depth ∧
        DX cfg (InPart cfg partition) k.safe_state k.sched k.slots
          (InPart cfg partition) self1.safe_state self1.sched self1.slots := by
      peel hself1
      · exact ⟨(kill_at_r hiv (by assumption)).2,
          kill_at_dg (by assumption) (by rw [hst]; exact ⟨tc, htc', by assumption⟩)⟩
      · exact ⟨rfl, fun h => h⟩
      · exact ⟨rfl, fun h => h⟩
    obtain ⟨hd, hbk⟩ := hcase
    obtain ⟨k2, h2, e⟩ := ih self1 i2 res (by rw [hd]; omega) hrec
    exact ⟨k2, fun h => h2 (hbk h), e⟩

theorem kill_partition_jobs_dg {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') :
    DX cfg (InPart cfg partition) k.safe_state k.sched k.slots
      (InPart cfg partition) k'.safe_state k'.sched k'.slots := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨res, hres, h⟩ := h
  obtain ⟨k2, h2, e⟩ := kill_loop_dg _ _ _ res rfl hres
  subst e
  simp only [Aeneas.Std.uncurry] at h
  have h := ok_inj h
  subst h
  exact h2

section
variable [hNT : Fact (NT.val ≤ 256)]
include hNT

theorem clear_ex_u {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : Usize} (h : logic.sched.Sched.clear_release s cfg (UScalar.cast .U8 t) = ok s')
    (ht : t.val < NT.val) :
    ∀ τ, τ ≠ t.val → rel s' τ = rel s τ ∧ job s' τ = job s τ := by
  obtain ⟨_, hrel, hjob, _⟩ := clear_release_views h
  have hN := hNT.out
  have e : (UScalar.cast .U8 t : U8).val = t.val := by
    simp only [UScalar.cast_val_eq, UScalarTy.numBits]
    omega
  intro τ hne
  rw [hrel, hjob, e, if_neg hne]
  exact ⟨rfl, rfl⟩

set_option maxHeartbeats 8000000 in
theorem clear_loop_dg {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NT.val - t.val = n →
      logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k' →
      DX cfg (fun τ => InPart cfg partition τ ∧ t.val ≤ τ) k.safe_state k.sched k.slots
        NoEx k'.safe_state k'.sched k'.slots := by
  intro n
  induction n with
  | zero =>
    intro k t k' hn h
    unfold logic.kernel.Kernel.clear_partition_releases_loop loop at h
    simp only [logic.kernel.Kernel.clear_partition_releases_loop.body] at h
    have hlt : ¬ t < NT := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    intro hd hb i hi ha hk
    obtain ⟨h1, h2, h3⟩ := hd hb i hi ha hk
    refine ⟨h1, h2, ?_⟩
    rcases h3 with h | h | h
    · exact Or.inl h
    · exact Or.inr (Or.inl h)
    · have := h.2
      omega
  | succ n ih =>
    intro k t k' hn h
    unfold logic.kernel.Kernel.clear_partition_releases_loop loop at h
    simp only [logic.kernel.Kernel.clear_partition_releases_loop.body] at h
    have hlt : t < NT := by scalar_tac
    have htv : t.val < NT.val := by omega
    simp only [hlt, ite_true, lift, bind_ok, bind_eq_ok] at h
    obtain ⟨cf, ⟨tc, htc, self1, hself1, t1, ht1, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    have ht1 := usize_add_ok ht1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at ht1
    have hrec : logic.kernel.Kernel.clear_partition_releases_loop self1 cfg partition t1 = ok k' := by
      unfold logic.kernel.Kernel.clear_partition_releases_loop
      simp only [logic.kernel.Kernel.clear_partition_releases_loop.body, lift, bind_ok]
      exact hloop
    have htc' : cfg.tasks.val[t.val]? = some tc := (index_ok' htc).2
    have hcase : DX cfg (fun τ => InPart cfg partition τ ∧ t.val ≤ τ) k.safe_state k.sched k.slots
        (fun τ => InPart cfg partition τ ∧ t1.val ≤ τ) self1.safe_state self1.sched self1.slots := by
      peel hself1
      all_goals first
        | (rename_i hnp
           refine dg_mono (fun τ hτ => ⟨hτ.1, ?_⟩)
           have hne : τ ≠ t.val := by
             intro e
             obtain ⟨tc', htc2, hp⟩ := hτ.1
             rw [e, htc'] at htc2
             have := Option.some.inj htc2
             subst this
             exact hnp hp
           have := hτ.2
           omega)
        | (show DX _ _ _ _ _ _ _ _ _
           dsimp only
           repeat (first
             | refine dx_disarm_ex_u (by assumption) (by assumption) ?_
             | refine dx_arm_ex_u (by assumption)
                 (by simp [logic.time.SLOT_UNMASK, logic.time.SLOT_DEADLINE]) (by assumption) ?_
             | refine dx_disarm (by assumption) ?_
             | dsimp only)
           refine dx_sched_ex (clear_ex_u (by assumption) htv) ?_
           refine dg_mono (fun τ hτ => ?_)
           by_cases e : τ = t.val
           · exact Or.inr e
           · exact Or.inl ⟨hτ.1, by have := hτ.2; omega⟩)
    have h2 := ih self1 t1 k' (by omega) hrec
    exact fun h => h2 (hcase h)

theorem clear_partition_releases_dg {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') :
    DX cfg (InPart cfg partition) k.safe_state k.sched k.slots
      NoEx k'.safe_state k'.sched k'.slots := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  rename_i k1 hk1 a ha
  refine dx_trans (dx_trans (dg_mono (fun τ hτ => ⟨hτ, Nat.zero_le _⟩))
    (clear_loop_dg _ _ _ _ rfl hk1)) ?_
  dx_close

theorem end_partition_jobs_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals first
    | exact dx_trans (dx_trans (dx_trans (end_top_dx (by assumption))
        (dg_mono (fun _ hτ => False.elim hτ))) (kill_partition_jobs_dg (by assumption)))
        (clear_partition_releases_dg h)
    | exact dx_trans (dx_trans (dg_mono (fun _ hτ => False.elim hτ))
        (kill_partition_jobs_dg (by assumption))) (clear_partition_releases_dg h)

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (end_partition_jobs_dx (by assumption)))

theorem stop_partition_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (stop_partition_dx (by assumption)))

theorem restart_partition_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (restart_partition_dx (by assumption)))

set_option maxHeartbeats 16000000 in
theorem apply_response_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.apply_response at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (apply_response_dx (by assumption)))

theorem deadline_miss_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deadline_miss k cfg t i = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.deadline_miss at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (deadline_miss_dx (by assumption)))

theorem mit_violation_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t partition offender : U8}
    (h : logic.kernel.Kernel.mit_violation k cfg t partition offender = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.mit_violation at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (mit_violation_dx (by assumption)))

theorem fault_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind : U8} {kn : Bool}
    (h : logic.kernel.Kernel.fault k cfg kind kn = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.fault at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (fault_dx (by assumption)))

theorem fault_of_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind t : U8}
    (h : logic.kernel.Kernel.fault_of k cfg kind t = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.fault_of at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (fault_of_dx (by assumption)))

theorem record_event_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind t : U8}
    (h : logic.kernel.Kernel.record_event k cfg kind t = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.record_event at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (record_event_dx (by assumption)))

theorem observe_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64}
    (h : logic.kernel.Kernel.observe k cfg now = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.observe at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (observe_dx (by assumption)))

set_option maxHeartbeats 16000000 in
theorem budget_event_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.budget_event at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (budget_event_dx (by assumption)))

set_option maxHeartbeats 16000000 in
theorem deferred_release_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deferred_release k cfg t i = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.deferred_release at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (deferred_release_dx (by assumption)))

set_option maxHeartbeats 16000000 in
theorem periodic_release_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.periodic_release k cfg t i = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.periodic_release at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (periodic_release_dx (by assumption)))

set_option maxHeartbeats 16000000 in
theorem intake_release_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t sp off : U8} {now : U64}
    (h : logic.kernel.Kernel.intake_release k cfg t sp now off = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.intake_release at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (intake_release_dx (by assumption)))

set_option maxHeartbeats 16000000 in
theorem serve_slot_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k') : DXK cfg k k' := by
  unfold logic.kernel.Kernel.serve_slot at h
  peel h
  all_goals dx_close

macro_rules
  | `(tactic| dx_call) => `(tactic| refine dx_trans ?_ (serve_slot_dx (by assumption)))

end

theorem settle_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : Option U8 × Bool}
    (h : logic.kernel.Kernel.settle k cfg = ok (r, k')) : DXK cfg k k' := by
  unfold logic.kernel.Kernel.settle at h
  peel h
  all_goals first
    | exact arm_budget_dx (by assumption)
    | dx_close

theorem svc_lock_safe {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_lock k cfg r = ok (b, k')) : k'.safe_state = k.safe_state := by
  unfold logic.kernel.Kernel.svc_lock at h
  peel h
  all_goals first
    | rfl
    | (refine (push_event_safe (by assumption)).trans ?_
       rfl)

theorem svc_unlock_safe {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (b, k')) : k'.safe_state = k.safe_state := by
  unfold logic.kernel.Kernel.svc_unlock at h
  peel h
  all_goals first
    | rfl
    | (refine (push_event_safe (by assumption)).trans ?_
       rfl)

theorem svc_lock_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.kernel.Kernel.svc_lock k cfg r = ok (true, k')) : DXK cfg k k' := by
  obtain ⟨hrel, hjob, _, _⟩ := (lock_views (svc_lock_eff h)).1
  show DX _ _ _ _ _ _ _ _ _
  rw [svc_lock_slots h, svc_lock_safe h]
  exact dx_sched (fun τ hτ => by rw [hrel, hjob]; exact hτ) (fun h => h)

theorem svc_unlock_dx {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (true, k')) : DXK cfg k k' := by
  obtain ⟨hrel, hjob, _, _⟩ := (unlock_views (svc_unlock_eff h)).1
  show DX _ _ _ _ _ _ _ _ _
  rw [svc_unlock_slots h, svc_unlock_safe h]
  exact dx_sched (fun τ hτ => by rw [hrel, hjob]; exact hτ) (fun h => h)

/-! ## The run -/

section
variable [hNT : Fact (NT.val ≤ 256)]
include hNT

/-- Every transition of the run keeps the invariant, with no condition on
what the transition names. -/
theorem kstep_di {cfg : logic.Config NT NR NP} {P : Params}
    {x y : KSt NT NR NP NL NS NE} (h : KStep cfg P x y)
    (hb : DG cfg NoEx x.k.safe_state x.k.sched x.k.slots) :
    DG cfg NoEx y.k.safe_state y.k.sched y.k.slots := by
  cases h with
  | jarrive => exact hb
  | earrive => exact hb
  | periodic q t i d c k' hq hk hw hrel hnd hc hs hmade =>
    have := periodic_release_dx hs hb
    split <;> exact this
  | intake q t sp off now d c k' hq hk hw hrel hnd hc hs hmade =>
    have := intake_release_dx hs hb
    split <;> exact this
  | dispatch t pc c k' hk hsel hc hs => exact dispatch_dx hs hb
  | lock hsel hk r a c k' hrun hrl hc ha hal hs => exact svc_lock_dx hs hb
  | unlock hsel hk r c k' hrun hru hc hal hs => exact svc_unlock_dx hs hb
  | finish hsel hk hj ec c k' hd hr0 hr hc hal hs => exact job_return_dx hs hb
  | unwind ec k' hs => exact job_return_dx hs hb
  | resp partition response cause t k' hs => exact apply_response_dx hs hb
  | faulted kind kn k' hs => exact fault_dx hs hb
  | faulted_of kind t k' hs => exact fault_of_dx hs hb
  | bserved t i k' hs => exact budget_event_dx hs hb
  | opend i k1 k' hcode h1 hs => exact serve_slot_dx hcode hb
  | settle r k' hs => exact settle_dx hs hb
  | charged t rc b hs => exact hb
  | acked_reinit p k' hs =>
    obtain ⟨a, rfl⟩ := ack_reinit_upd hs
    exact hb
  | acked_stop p k' hs =>
    obtain ⟨a, rfl⟩ := ack_stop_dma_upd hs
    exact hb
  | logged kind task partition prs detail k' hs => exact push_event_dx hs hb
  | observed now k' hs => exact observe_dx hs hb
  | deferred q t i d c k' hq hk hw hrel hnd hc hs hmade =>
    have := deferred_release_dx hs hb
    split <;> exact this
  | deadline q c t i k' hq hk hw hc hdl hs => exact deadline_miss_dx hs hb
  | plain_h q c k' hq hk hw hc hnd i hcode hs => exact serve_slot_dx hcode hb
  | svc_h hsel hk c k' hd hrun hre hal hcode =>
    rcases hcode with e | ⟨kind, t, h⟩
    · rw [e]; exact hb
    · exact record_event_dx h hb
  | bevent_h hq harmed hk c k' hd hrun hrb hc t i hcode hs =>
    exact budget_event_dx hcode hb
  | overrun_h hq harmed hk hj c r k' hd hr0 hr hstop hrb hc hal t i hcode hs =>
    exact budget_event_dx hcode hb
  | overrun_p hq harmed hk hj c r s1 k' hd hr0 hr hstop hrb hc hal t i hcode hs hrs =>
    exact budget_event_dx hcode hb
  | bevent_r hq harmed hk c k' hd hrun hrb hc t i hcode =>
    exact budget_event_dx hcode hb
  | ktick => exact hb
  | idle => exact hb
  | exec => exact hb

omit hNT in
/-- No slot is armed with the deadline kind. -/
def NoD (sl : logic.time.Slots NS) : Prop :=
  ∀ i, i < NS.val → (slotAt sl i).armed = true → (slotAt sl i).kind ≠ logic.time.SLOT_DEADLINE

/-- The states the run reaches from a boot state in which no Budget slot
and no deadline slot is armed. -/
inductive KReachS (cfg : logic.Config NT NR NP) (P : Params) : KSt NT NR NP NL NS NE → Prop
  | boot (k0 : logic.kernel.Kernel NT NR NP NL NS NE)
      (h : logic.sched.Sched.new NL cfg = ok k0.sched) (hb : NoB k0.slots) (hd : NoD k0.slots) :
      KReachS cfg P (KSt.boot k0)
  | step (x y : KSt NT NR NP NL NS NE) (h : KReachS cfg P x) (hs : KStep cfg P x y) :
      KReachS cfg P y

omit hNT in
theorem kreachs_kreachb {cfg : logic.Config NT NR NP} {P : Params}
    {x : KSt NT NR NP NL NS NE} (h : KReachS cfg P x) : KReachB cfg P x := by
  induction h with
  | boot k0 h0 hb _ => exact KReachB.boot k0 h0 hb
  | step x y _ hs ih => exact KReachB.step x y ih hs

/-- **Outside the safe state, an armed deadline slot belongs to a Task with
a Release pending or a Job in progress**, in every state the run reaches. -/
theorem kreachs_di {cfg : logic.Config NT NR NP} {P : Params}
    {x : KSt NT NR NP NL NS NE} (h : KReachS cfg P x) :
    DG cfg NoEx x.k.safe_state x.k.sched x.k.slots := by
  induction h with
  | boot k0 _ _ hd => exact fun _ i hi ha hk => absurd hk (hd i hi ha)
  | step x y _ hs ih => exact kstep_di hs ih

/-- In the run's accounts: outside the safe state, the Task of an armed
deadline slot has a Job with work left (pending or in progress). This is
the third part of what the run's `deadline` transition asks of a deadline
event ("the Job is not complete at its deadline"). -/
theorem deadline_slot_incomplete {cfg : logic.Config NT NR NP} {P : Params}
    {x : KSt NT NR NP NL NS NE} (hx : KReachS cfg P x) (hsafe : x.k.safe_state = false)
    {i : Usize} (hi : i.val < NS.val)
    (ha : (slotOf x.k i).armed = true) (hk : (slotOf x.k i).kind = logic.time.SLOT_DEADLINE) :
    (slotOf x.k i).task.val < NT.val ∧
    0 < x.pend (slotOf x.k i).task.val + x.run (slotOf x.k i).task.val := by
  obtain ⟨h1, _, h3⟩ := kreachs_di hx hsafe i.val hi ha hk
  have w := reach_wf (ksim hNT.out (kreachb_kreach (kreachs_kreachb hx)))
  show (slotAt x.k.slots i.val).task.val < NT.val ∧
    0 < x.pend (slotAt x.k.slots i.val).task.val + x.run (slotAt x.k.slots i.val).task.val
  refine ⟨h1, ?_⟩
  rcases h3 with h | h | h
  · have := (w.pend_iff _ h1).mpr h
    dsimp only [KSt.st] at this
    omega
  · have := (w.run_iff _ h1).mpr h
    dsimp only [KSt.st] at this
    omega
  · exact absurd h id

/-- A deadline slot that `timer_event` serves outside the safe state, in a
state the run reaches, is the run's `deadline` transition; that the Job is
not complete is derived, and what is still asked of the model is which
Task the event's source belongs to and that its deadline has passed. -/
theorem KStep.served_deadline_reach {cfg : logic.Config NT NR NP} {P : Params}
    {x : KSt NT NR NP NL NS NE} (hx : KReachS cfg P x) (hsafe : x.k.safe_state = false)
    (i : Usize) (q c : Nat) (k' : logic.kernel.Kernel NT NR NP NL NS NE)
    (hs : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
    (hdue : SlotDue x.k i)
    (hkind : (slotOf x.k i).kind = logic.time.SLOT_DEADLINE)
    (hq : q < P.NQ) (hk : x.ks = 0) (hw : x.ewait q = true) (hc : c ≤ P.EC q)
    (hdl : ∀ t', P.dlOf q = some t' → t' = (slotOf x.k i).task.val ∧
      ∃ a, x.jl t' = some a ∧ a + P.D t' ≤ x.now) :
    ∃ y, KStep cfg P x y ∧ y.k = k' := by
  obtain ⟨ht, hpos⟩ := deadline_slot_incomplete hx hsafe hdue.1 hdue.2.1 hkind
  refine KStep.served_deadline x i q c k' hs hkind ht hq hk hw hc ?_
  intro t' h
  obtain ⟨e, a, ha, hle⟩ := hdl t' h
  exact ⟨a, ha, hle, Or.inl (by rw [e]; exact hpos)⟩

end

/-! ## The boot state -/

theorem nod_arm {sl sl' : logic.time.Slots NS} {j : Usize} {a : U64} {kind task : U8}
    (h : logic.time.Slots.arm sl j a kind task = ok sl') (hk : kind ≠ logic.time.SLOT_DEADLINE)
    (hn : NoD sl) : NoD sl' := by
  intro i hi ha
  rw [(arm_at h i).2] at ha ⊢
  split
  · exact hk
  · rename_i hne; rw [if_neg hne] at ha; exact hn i hi ha

/-- `Kernel::init` leaves no slot armed, whatever the state before. -/
theorem new_nod {cfg : logic.Config NT NR NP} {k0 : logic.kernel.Kernel NT NR NP NL NS NE}
    {kold : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.init kold cfg = ok k0) : NoD k0.slots := by
  unfold logic.kernel.Kernel.init at h
  peel h
  dsimp only
  intro i hi ha
  simp [slotAt, Array.repeat, hi] at ha

theorem start_loop_nod {cfg : logic.Config NT NR NP} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize) res,
      NT.val - t.val = n →
      logic.kernel.Kernel.start_loop k cfg t = ok res → NoD k.slots → NoD res.1.slots := by
  intro n
  induction n with
  | zero =>
    intro k t res hn h hb
    unfold logic.kernel.Kernel.start_loop loop at h
    simp only [logic.kernel.Kernel.start_loop.body] at h
    have hlt : ¬ t < NT := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    exact hb
  | succ n ih =>
    intro k t res hn h hb
    unfold logic.kernel.Kernel.start_loop loop at h
    simp only [logic.kernel.Kernel.start_loop.body] at h
    have hlt : t < NT := by scalar_tac
    simp only [hlt, ite_true, lift, bind_ok, bind_eq_ok] at h
    obtain ⟨cf, ⟨tc, htc, self1, hself1, t1, ht1, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    have ht1 := usize_add_ok ht1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at ht1
    have hrec : logic.kernel.Kernel.start_loop self1 cfg t1 = ok res := by
      unfold logic.kernel.Kernel.start_loop
      simp only [logic.kernel.Kernel.start_loop.body, lift, bind_ok]
      exact hloop
    have hcase : NoD self1.slots := by
      peel hself1
      · dsimp only
        exact nod_arm (by assumption)
          (by simp [logic.time.SLOT_RELEASE, logic.time.SLOT_DEADLINE]) hb
      · exact hb
    exact ih self1 t1 res (by omega) hrec hcase

/-- `Kernel::start` arms release slots, the slot of the end of the
operating duration and the housekeeping slot: no deadline slot. -/
theorem start_nod {cfg : logic.Config NT NR NP} {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.start k cfg = ok k') (hb : NoD k.slots) : NoD k'.slots := by
  unfold logic.kernel.Kernel.start at h
  peel h
  all_goals
    have hl := start_loop_nod _ _ _ _ rfl (by assumption) hb
    dsimp only at hl ⊢
    refine nod_arm (by assumption)
      (by simp [logic.time.SLOT_HOUSEKEEPING, logic.time.SLOT_DEADLINE]) ?_
    exact nod_arm (by assumption) (by simp [logic.time.SLOT_END, logic.time.SLOT_DEADLINE]) hl

end P18
