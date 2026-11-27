import Sched.ContainMore
import Sched.SlotsApart
open Aeneas Aeneas.Std Result

/-! # What a response leaves alone: the timer slots and the timing fields (2026-11-26)

`Sched/Contain.lean` and `Sched/ContainMore.lean` have, of
`Kernel::others_unchanged` (R14.9), the scheduler's clauses, the flags, the
System_Ceiling and the Budget accounts. This file has the rest of
`Kernel::task_unchanged`, on the Kernel's functions as Aeneas extracts
them, each statement of the form "if the function returns":

- `apply_response_others` (the statement in Verus' terms, from
  `Config::validate` returning `true`): a Health_Monitor response for a
  Task of its Partition leaves, for every Task `u` of another Partition,
  `u`'s deadline slot and its release slot (when it has one) as they were
  (armed or not, the instant, the kind and the Task named), its Budget slot
  as it was unless the response ended the executing Job and `u` is the
  Task of the Job that resumed (`arm_budget` re-arms that slot for the
  remaining Budget), and every timing field of `u` as it was.
- `fault_others`: the same of `Kernel::fault`, where the code meets the
  premise on the response's Task (`fault_first`).
- `apply_response_slots`, `apply_response_timing`: the same without the
  premise that the response's Task is of the response's Partition, for the
  Tasks outside that Partition other than that Task; the whole timing
  state is as it was but for the mask of the response's Task
  (`MaskOnly`: END_JOB ends the MIT window of the Job it ends).
- `apply_response_sr`: the frame itself, with no premise on the
  configuration. `SK W`: the slot table is as it was outside the slot
  indices in `W`. A response writes only slots that a Task of its
  Partition (or its Task) owns by the configuration's tables (`WP`, `Own`),
  and, when it ended the executing Job, the Budget slot of the Task now on
  top (`SR`).
- `sr_frame`: with the slots apart (`SlotsApart`, which
  `Config::validate` establishes: `Sched/SlotsApart.lean`), no slot of a
  Task outside is among those written, but the Budget slot of the Task
  resumed.

The steps: `arm_budget_sk`, `end_top_release_sk`, `end_top_sk`,
`end_mit_window_sk`, `kill_partition_jobs_qs`,
`clear_partition_releases_qs`, `end_partition_jobs_sr`,
`stop_partition_sr`, `restart_partition_sr`, `enter_safe_state_qs`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! ## The slot table outside a set of indices -/

/-- From `sl` to `sl'` every slot outside `W` is as it was. -/
def SK (W : Nat → Prop) (sl sl' : logic.time.Slots NS) : Prop :=
  ∀ j, ¬ W j → slotAt sl' j = slotAt sl j

theorem SK.refl (W : Nat → Prop) (sl : logic.time.Slots NS) : SK W sl sl := fun _ _ => rfl

theorem SK.of_eq {W : Nat → Prop} {sl sl' : logic.time.Slots NS} (h : sl' = sl) : SK W sl sl' :=
  fun _ _ => by rw [h]

theorem SK.trans {W : Nat → Prop} {a b c : logic.time.Slots NS} (h1 : SK W a b) (h2 : SK W b c) :
    SK W a c := fun j hj => (h2 j hj).trans (h1 j hj)

theorem SK.mono {W W' : Nat → Prop} {sl sl' : logic.time.Slots NS} (hq : ∀ j, W j → W' j)
    (h : SK W sl sl') : SK W' sl sl' := fun j hj => h j (fun hw => hj (hq j hw))

theorem SK.disarm {W : Nat → Prop} {sl sl' : logic.time.Slots NS} {j : Usize}
    (h : logic.time.Slots.disarm sl j = ok sl') (hw : W j.val) : SK W sl sl' := by
  intro i hi
  rw [(disarm_at h i).2]
  split
  · rename_i e
    exact absurd (by rw [e]; exact hw) hi
  · rfl

theorem SK.arm {W : Nat → Prop} {sl sl' : logic.time.Slots NS} {j : Usize} {a : U64}
    {kind task : U8} (h : logic.time.Slots.arm sl j a kind task = ok sl') (hw : W j.val) :
    SK W sl sl' := by
  intro i hi
  rw [(arm_at h i).2]
  split
  · rename_i e
    exact absurd (by rw [e]; exact hw) hi
  · rfl

/-- Slot `j` is one that Task `v` owns by the configuration's tables: its
deadline slot, its Budget slot, or its release slot when it has one. -/
def Own (cfg : logic.Config NT NR NP) (v j : Nat) : Prop :=
  j = dslot cfg v ∨ j = bslot cfg v ∨ (needsRel cfg v ∧ j = rslot cfg v)

/-- Slot `j` is owned by a Task of `inP`. -/
def WP (cfg : logic.Config NT NR NP) (inP : Nat → Prop) (j : Nat) : Prop :=
  ∃ v, v < NT.val ∧ inP v ∧ Own cfg v j

/-! ## The writes of a response, by the reads that give their index -/

section writes
variable {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {sl sl' : logic.time.Slots NS}

/-- The Budget slot of Task `t` disarmed. -/
theorem sk_b8 {t i : U8}
    (hd : logic.time.Slots.disarm sl (UScalar.cast .Usize i) = ok sl')
    (hi : cfg.slot_budget.index_usize (UScalar.cast .Usize t) = ok i) (hin : inP t.val) :
    SK (WP cfg inP) sl sl' := by
  have hb := (index_ok hi).1
  rw [U8.cast_Usize_val_eq] at hb
  exact SK.disarm hd ⟨t.val, hb, hin, Or.inr (Or.inl (slot_budget_idx hi).symm)⟩

/-- The deadline slot of Task `t` disarmed. -/
theorem sk_d8 {t i : U8}
    (hd : logic.time.Slots.disarm sl (UScalar.cast .Usize i) = ok sl')
    (hi : cfg.slot_deadline.index_usize (UScalar.cast .Usize t) = ok i) (hin : inP t.val) :
    SK (WP cfg inP) sl sl' := by
  have hb := (index_ok hi).1
  rw [U8.cast_Usize_val_eq] at hb
  exact SK.disarm hd ⟨t.val, hb, hin, Or.inl (slot_deadline_idx hi).symm⟩

/-- The deadline slot of Task `t` armed. -/
theorem sk_ad8 {t i : U8} {a : U64} {kind task : U8}
    (hd : logic.time.Slots.arm sl (UScalar.cast .Usize i) a kind task = ok sl')
    (hi : cfg.slot_deadline.index_usize (UScalar.cast .Usize t) = ok i) (hin : inP t.val) :
    SK (WP cfg inP) sl sl' := by
  have hb := (index_ok hi).1
  rw [U8.cast_Usize_val_eq] at hb
  exact SK.arm hd ⟨t.val, hb, hin, Or.inl (slot_deadline_idx hi).symm⟩

/-- The Budget slot of Task `t` armed. -/
theorem sk_ab8 {t i : U8} {a : U64} {kind task : U8}
    (hd : logic.time.Slots.arm sl (UScalar.cast .Usize i) a kind task = ok sl')
    (hi : cfg.slot_budget.index_usize (UScalar.cast .Usize t) = ok i) :
    SK (fun j => t.val < NT.val ∧ j = bslot cfg t.val) sl sl' := by
  have hb := (index_ok hi).1
  rw [U8.cast_Usize_val_eq] at hb
  exact SK.arm hd ⟨hb, (slot_budget_idx hi).symm⟩

theorem dslot_idx {t : Usize} {i : U8} (hi : cfg.slot_deadline.index_usize t = ok i) :
    t.val < NT.val ∧ (UScalar.cast .Usize i : Usize).val = dslot cfg t.val := by
  obtain ⟨hb, e⟩ := index_ok hi
  refine ⟨hb, ?_⟩
  show (UScalar.cast .Usize i : Usize).val = (cfg.slot_deadline.val[t.val]!).val
  rw [U8.cast_Usize_val_eq, e]

theorem rslot_idx {t : Usize} {i : U8} (hi : cfg.slot_release.index_usize t = ok i) :
    t.val < NT.val ∧ (UScalar.cast .Usize i : Usize).val = rslot cfg t.val := by
  obtain ⟨hb, e⟩ := index_ok hi
  refine ⟨hb, ?_⟩
  show (UScalar.cast .Usize i : Usize).val = (cfg.slot_release.val[t.val]!).val
  rw [U8.cast_Usize_val_eq, e]

/-- The deadline slot of the Task at index `t` disarmed. -/
theorem sk_du {t : Usize} {i : U8}
    (hd : logic.time.Slots.disarm sl (UScalar.cast .Usize i) = ok sl')
    (hi : cfg.slot_deadline.index_usize t = ok i) (hin : inP t.val) :
    SK (WP cfg inP) sl sl' :=
  SK.disarm hd ⟨t.val, (dslot_idx hi).1, hin, Or.inl (dslot_idx hi).2⟩

/-- The deadline slot of the Task at index `t` armed. -/
theorem sk_adu {t : Usize} {i : U8} {a : U64} {kind task : U8}
    (hd : logic.time.Slots.arm sl (UScalar.cast .Usize i) a kind task = ok sl')
    (hi : cfg.slot_deadline.index_usize t = ok i) (hin : inP t.val) :
    SK (WP cfg inP) sl sl' :=
  SK.arm hd ⟨t.val, (dslot_idx hi).1, hin, Or.inl (dslot_idx hi).2⟩

/-- The release slot of the deferring Task at index `t` disarmed. -/
theorem sk_ru {t : Usize} {i : U8} {tc : logic.TaskCfg}
    (hd : logic.time.Slots.disarm sl (UScalar.cast .Usize i) = ok sl')
    (hi : cfg.slot_release.index_usize t = ok i) (htc : cfg.tasks.index_usize t = ok tc)
    (hk : tc.mit_policy = logic.MIT_DEFER) (hin : inP t.val) :
    SK (WP cfg inP) sl sl' :=
  SK.disarm hd ⟨t.val, (rslot_idx hi).1, hin,
    Or.inr (Or.inr ⟨⟨tc, (index_ok' htc).2, Or.inr hk⟩, (rslot_idx hi).2⟩)⟩

end writes

/-! ## The chain through the calls of a function -/

theorem push_event_tm {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    k'.timing = k.timing := by
  unfold logic.kernel.Kernel.push_event at h
  frame_ok h

theorem push_event_sk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {W : Nat → Prop} {kind task partition : U8} {prs : U64}
    {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    SK W k.slots k'.slots := SK.of_eq (push_event_slots h)

syntax "sk_step" : tactic
macro_rules | `(tactic| sk_step) => `(tactic| exact SK.refl _ _)
macro_rules
  | `(tactic| sk_step) => `(tactic| refine SK.trans ?_ (push_event_sk (by assumption)))
macro_rules
  | `(tactic| sk_step) =>
    `(tactic| refine SK.trans ?_ (sk_b8 (by assumption) (by assumption) (by assumption)))
macro_rules
  | `(tactic| sk_step) =>
    `(tactic| refine SK.trans ?_ (sk_d8 (by assumption) (by assumption) (by assumption)))
macro_rules
  | `(tactic| sk_step) =>
    `(tactic| refine SK.trans ?_ (sk_ad8 (by assumption) (by assumption) (by assumption)))
macro_rules
  | `(tactic| sk_step) =>
    `(tactic| refine SK.trans ?_ (sk_du (by assumption) (by assumption) (by assumption)))
macro_rules
  | `(tactic| sk_step) =>
    `(tactic| refine SK.trans ?_ (sk_adu (by assumption) (by assumption) (by assumption)))
macro_rules
  | `(tactic| sk_step) =>
    `(tactic| refine SK.trans ?_
      (sk_ru (by assumption) (by assumption) (by assumption) (by assumption) (by assumption)))

/-- Closes `SK (WP cfg inP) sl sl'` after `peel`. -/
syntax "sk_close" : tactic
macro_rules
  | `(tactic| sk_close) =>
    `(tactic| ((try dsimp only); repeat (first | sk_step | dsimp only)))

/-- Closes `k'.timing = k.timing` after `peel`, through one event record. -/
syntax "tm_close" : tactic
macro_rules
  | `(tactic| tm_close) =>
    `(tactic| (
        try dsimp only
        try (have hpt := push_event_tm (by assumption); rw [hpt])
        try rfl))

/-! ## Ending the Job on top -/

/-- `arm_budget` writes the Budget slot of its Task, and no timing field. -/
theorem arm_budget_sk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') :
    SK (fun j => t.val < NT.val ∧ j = bslot cfg t.val) k.slots k'.slots ∧
    k'.timing = k.timing := by
  unfold logic.kernel.Kernel.arm_budget at h
  peel h
  all_goals
    dsimp only
    exact ⟨sk_ab8 (by assumption) (by assumption), rfl⟩

set_option maxHeartbeats 4000000 in
/-- `end_top_release` writes the Budget slot and the deadline slot of the
Task on top, and no timing field. -/
theorem end_top_release_sk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    (h : logic.kernel.Kernel.end_top_release k cfg = ok k')
    (hin : inP (stk k.sched (k.sched.depth.val - 1))) :
    SK (WP cfg inP) k.slots k'.slots ∧ k'.timing = k.timing := by
  unfold logic.kernel.Kernel.end_top_release at h
  peel h
  all_goals
    have htop := (top_of (by assumption) (by assumption)).2
    rw [htop] at hin
    refine ⟨by sk_close, by tm_close⟩

/-- **`end_top` writes the slots of the Task it ends and the Budget slot of
the Task it resumes**, and no timing field; the stack loses its top. -/
theorem end_top_sk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    (h : logic.kernel.Kernel.end_top k cfg = ok k')
    (hin : inP (stk k.sched (k.sched.depth.val - 1))) :
    k'.sched.depth.val + 1 = k.sched.depth.val ∧ (∀ p, stk k'.sched p = stk k.sched p) ∧
    SK (fun j => WP cfg inP j ∨ (0 < k'.sched.depth.val ∧
      stk k'.sched (k'.sched.depth.val - 1) < NT.val ∧
      j = bslot cfg (stk k'.sched (k'.sched.depth.val - 1)))) k.slots k'.slots ∧
    k'.timing = k.timing := by
  obtain ⟨r, hf⟩ := end_top_finish h
  obtain ⟨hd1, _, _, _, _, hstk, hdep, _⟩ := finish_top_views hf
  refine ⟨by omega, hstk, ?_⟩
  clear hf hstk hdep
  unfold logic.kernel.Kernel.end_top at h
  peel h
  · rename_i k1 hk1 _ i1 hi1 top htop
    obtain ⟨hp1, hs1⟩ := top_of hi1 htop
    have es := arm_budget_sched h
    obtain ⟨e1, e1t⟩ := end_top_release_sk hk1 hin
    obtain ⟨e2, e2t⟩ := arm_budget_sk h
    refine ⟨SK.trans (SK.mono (fun j hj => Or.inl hj) e1) (SK.mono ?_ e2), e2t.trans e1t⟩
    rintro j ⟨hb, hj⟩
    rw [es]
    exact Or.inr ⟨hp1, by rw [hs1]; exact hb, by rw [hs1]; exact hj⟩
  · obtain ⟨e1, e1t⟩ := end_top_release_sk (by assumption) hin
    exact ⟨SK.mono (fun j hj => Or.inl hj) e1, e1t⟩

/-! ## The timing state but for one Task's mask -/

/-- From `tm` to `tm'` every array of the timing state is as it was, but
for the mask of Task `t`. -/
def MaskOnly (t : Nat) (tm tm' : logic.time.Timing NT) : Prop :=
  tm'.k = tm.k ∧ tm'.nominal = tm.nominal ∧ tm'.has_last = tm.has_last ∧
  tm'.last_due = tm.last_due ∧ tm'.deferred_until = tm.deferred_until ∧
  tm'.mask_until = tm.mask_until ∧ tm'.due = tm.due ∧ tm'.job_due = tm.job_due ∧
  ∀ u, u ≠ t → tm'.masked.val[u]! = tm.masked.val[u]!

theorem MaskOnly.refl (t : Nat) (tm : logic.time.Timing NT) : MaskOnly t tm tm :=
  ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, fun _ _ => rfl⟩

theorem MaskOnly.of_eq {t : Nat} {tm tm' : logic.time.Timing NT} (h : tm' = tm) :
    MaskOnly t tm tm' := by
  rw [h]; exact MaskOnly.refl _ _

theorem try_unmask_mo {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {now : U64} {je b : Bool}
    (h : logic.time.Timing.try_unmask tm cfg t now je = ok (b, tm')) :
    MaskOnly t.val tm tm' := by
  unfold logic.time.Timing.try_unmask at h
  peel h
  all_goals first
    | exact MaskOnly.refl _ _
    | exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl,
        fun u hu => flag_other (by assumption) u hu⟩

/-- `end_mit_window` writes the deadline slot of its Task and that Task's
mask. -/
theorem end_mit_window_sk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') (hin : inP t.val) :
    SK (WP cfg inP) k.slots k'.slots ∧ MaskOnly t.val k.timing k'.timing := by
  unfold logic.kernel.Kernel.end_mit_window at h
  peel h
  all_goals
    dsimp only
    refine ⟨?_, try_unmask_mo (by assumption)⟩
    sk_close

/-! ## The steps that keep the stack -/

/-- From `k` to `k'` the stack is as it was, the slots outside those of the
Tasks in `inP` are as they were, and so is the timing state. -/
def QS (cfg : logic.Config NT NR NP) (inP : Nat → Prop)
    (k k' : logic.kernel.Kernel NT NR NP NL NS NE) : Prop :=
  k'.sched.depth = k.sched.depth ∧ (∀ p, stk k'.sched p = stk k.sched p) ∧
  SK (WP cfg inP) k.slots k'.slots ∧ k'.timing = k.timing

theorem QS.refl (cfg : logic.Config NT NR NP) (inP : Nat → Prop)
    (k : logic.kernel.Kernel NT NR NP NL NS NE) : QS cfg inP k k :=
  ⟨rfl, fun _ => rfl, SK.refl _ _, rfl⟩

theorem QS.trans {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    {a b c : logic.kernel.Kernel NT NR NP NL NS NE} (h1 : QS cfg inP a b) (h2 : QS cfg inP b c) :
    QS cfg inP a c :=
  ⟨h2.1.trans h1.1, fun p => (h2.2.1 p).trans (h1.2.1 p), SK.trans h1.2.2.1 h2.2.2.1,
    h2.2.2.2.trans h1.2.2.2⟩

/-- `QS` reads the scheduler state, the slots and the timing state of its
second Kernel state only. -/
theorem QS.congr {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    {k k2 k' : logic.kernel.Kernel NT NR NP NL NS NE} (hs : k'.sched = k2.sched)
    (hsl : k'.slots = k2.slots) (ht : k'.timing = k2.timing) (h : QS cfg inP k k2) :
    QS cfg inP k k' := by
  unfold QS at *
  rw [hs, hsl, ht]
  exact h

theorem kill_at_qs {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {i : Usize}
    (h : logic.kernel.Kernel.kill_at k cfg i = ok k') : QS cfg inP k k' := by
  obtain ⟨_, _, _, hstk, hdep, _⟩ := kill_views (kill_at_c h)
  unfold logic.kernel.Kernel.kill_at at h
  peel h
  exact ⟨hdep, hstk, SK.refl _ _, rfl⟩

/-- `kill_partition_jobs`' loop. -/
theorem kill_loop_qs {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) res,
      k.sched.depth.val - i.val = n →
      logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i = ok res →
      ∃ k2 : logic.kernel.Kernel NT NR NP NL NS NE, QS cfg inP k k2 ∧
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
    exact ⟨k, QS.refl _ _ _, h.symm⟩
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
    have hcase : QS cfg inP k self1 := by
      peel hself1
      · exact kill_at_qs (by assumption)
      · exact QS.refl _ _ _
      · exact QS.refl _ _ _
    obtain ⟨k2, h2, e⟩ := ih self1 i2 res (by rw [hcase.1]; omega) hrec
    exact ⟨k2, QS.trans hcase h2, e⟩

/-- `kill_partition_jobs` writes no slot and no timing field, and keeps the
stack. -/
theorem kill_partition_jobs_qs {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {partition : U8}
    (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') :
    QS cfg inP k k' := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨res, hres, h⟩ := h
  obtain ⟨k2, h2, e⟩ := kill_loop_qs (inP := inP) _ _ _ res rfl hres
  subst e
  simp only [Aeneas.Std.uncurry] at h
  have h := ok_inj h
  subst h
  exact h2

set_option maxHeartbeats 4000000 in
/-- `clear_partition_releases`' loop: the slots written are the deadline
slot of a Task of the Partition and the release slot of a deferring one. -/
theorem clear_loop_qs {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {partition : U8}
    (hP : ∀ u, ptOf cfg u = partition.val → inP u) :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NT.val - t.val = n →
      logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k' →
      QS cfg inP k k' := by
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
    exact QS.refl _ _ _
  | succ n ih =>
    intro k t k' hn h
    unfold logic.kernel.Kernel.clear_partition_releases_loop loop at h
    simp only [logic.kernel.Kernel.clear_partition_releases_loop.body] at h
    have hlt : t < NT := by scalar_tac
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
    have hcase : QS cfg inP k self1 := by
      peel hself1
      all_goals first
        | exact QS.refl _ _ _
        | (have hpe : tc.partition = partition := by assumption
           have hin : inP t.val := hP _ (by rw [pt_of_index htc, hpe])
           have hv := clear_release_views (by assumption)
           refine ⟨hv.2.2.2.2.1, hv.2.2.2.1, ?_, rfl⟩
           sk_close)
    have h2 := ih self1 t1 k' (by omega) hrec
    exact QS.trans hcase h2

/-- `clear_partition_releases` writes slots of the Partition's Tasks only,
and no timing field, and keeps the stack. -/
theorem clear_partition_releases_qs {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {partition : U8}
    (hP : ∀ u, ptOf cfg u = partition.val → inP u)
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') :
    QS cfg inP k k' := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  rename_i k1 hk1 a ha
  exact QS.congr rfl rfl rfl (clear_loop_qs hP _ k 0#usize k1 rfl hk1)

/-- The loop of `enter_safe_state` keeps the stack. -/
theorem safe_loop_stk {s r : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : Usize} (h : logic.kernel.Kernel.enter_safe_state_loop s cfg t = ok r) :
    r.depth = s.depth ∧ ∀ p, stk r p = stk s p := by
  unfold logic.kernel.Kernel.enter_safe_state_loop at h
  refine loop_inv _ (fun x => x.1.depth = s.depth ∧ ∀ p, stk x.1 p = stk s p)
    (fun y => y.depth = s.depth ∧ ∀ p, stk y p = stk s p) ?_ ?_ (s, t) r
    ⟨rfl, fun _ => rfl⟩ h
  · rintro ⟨s1, i1⟩ ⟨s2, i2⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.enter_safe_state_loop.body at hb
    peel hb
    have hv := clear_release_views (by assumption)
    exact ⟨hv.2.2.2.2.1.trans hinv.1, fun p => (hv.2.2.2.1 p).trans (hinv.2 p)⟩
  · rintro ⟨s1, i1⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.enter_safe_state_loop.body at hb
    peel hb
    exact hinv

/-- **The system safe state writes no slot and no timing field**, and
keeps the stack. -/
theorem enter_safe_state_qs {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    (h : logic.kernel.Kernel.enter_safe_state k cfg = ok k') : QS cfg inP k k' := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  peel h
  · rename_i hs s hl
    have hst := safe_loop_stk hl
    exact ⟨hst.1, hst.2, SK.refl _ _, rfl⟩
  · rename_i k1 hs hp s hl
    have hst := safe_loop_stk hl
    rw [push_event_sched hp] at hst
    exact ⟨hst.1, hst.2, SK.of_eq (push_event_slots hp : k1.slots = k.slots),
      (push_event_tm hp : k1.timing = k.timing)⟩

/-! ## A response on the slots -/

/-- **What a response does to the slot table.** The stack keeps its
entries. Either its depth is as it was and only slots owned by Tasks of
`inP` were written; or the Job on top was ended, and beside those slots the
Budget slot of the Task now on top was written. -/
def SR (cfg : logic.Config NT NR NP) (inP : Nat → Prop)
    (k k' : logic.kernel.Kernel NT NR NP NL NS NE) : Prop :=
  (∀ p, stk k'.sched p = stk k.sched p) ∧
  ((k'.sched.depth = k.sched.depth ∧ SK (WP cfg inP) k.slots k'.slots) ∨
   (k'.sched.depth.val + 1 = k.sched.depth.val ∧
     SK (fun j => WP cfg inP j ∨ (0 < k'.sched.depth.val ∧
       stk k'.sched (k'.sched.depth.val - 1) < NT.val ∧
       j = bslot cfg (stk k'.sched (k'.sched.depth.val - 1)))) k.slots k'.slots))

theorem SR.of_qs {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE} (h : QS cfg inP k k') : SR cfg inP k k' :=
  ⟨h.2.1, Or.inl ⟨h.1, h.2.2.1⟩⟩

/-- A step that keeps the stack, after a response's steps. -/
theorem SR.step {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    {k m k' : logic.kernel.Kernel NT NR NP NL NS NE} (h : SR cfg inP k m)
    (hd : k'.sched.depth = m.sched.depth) (hs : ∀ p, stk k'.sched p = stk m.sched p)
    (hw : SK (WP cfg inP) m.slots k'.slots) : SR cfg inP k k' := by
  obtain ⟨h1, h2⟩ := h
  refine ⟨fun p => (hs p).trans (h1 p), ?_⟩
  rcases h2 with ⟨d, w⟩ | ⟨d, w⟩
  · exact Or.inl ⟨hd.trans d, SK.trans w hw⟩
  · refine Or.inr ⟨by rw [hd]; exact d,
      SK.trans (SK.mono ?_ w) (SK.mono (fun j hj => Or.inl hj) hw)⟩
    rintro j (hj | ⟨a, b, c⟩)
    · exact Or.inl hj
    · exact Or.inr ⟨by rw [hd]; exact a, by rw [hd, hs]; exact b, by rw [hd, hs]; exact c⟩

theorem SR.then_qs {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    {k m k' : logic.kernel.Kernel NT NR NP NL NS NE} (h : SR cfg inP k m)
    (h2 : QS cfg inP m k') : SR cfg inP k k' :=
  SR.step h h2.1 h2.2.1 h2.2.2.1

/-- `SR` reads the scheduler state and the slots of its Kernel states
only. -/
theorem SR.congr {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    {k0 k k2 k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (hs0 : k0.sched = k.sched) (hsl0 : k0.slots = k.slots)
    (hs : k'.sched = k2.sched) (hsl : k'.slots = k2.slots) (h : SR cfg inP k0 k2) :
    SR cfg inP k k' := by
  unfold SR at *
  rw [hs, hsl, ← hs0, ← hsl0]
  exact h

set_option maxHeartbeats 4000000 in
/-- **`end_partition_jobs` (a stop or a restart) on the slots and the
timing state**: the slots written are those of the Partition's Tasks and,
when the executing Job was the Partition's, the Budget slot of the Task
resumed; no timing field is written. -/
theorem end_partition_jobs_sr {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {partition : U8}
    (hP : ∀ u, ptOf cfg u = partition.val → inP u)
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') :
    SR cfg inP k k' ∧ k'.timing = k.timing := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals
    have hc := clear_partition_releases_qs (inP := inP) hP h
    have hk := kill_partition_jobs_qs (inP := inP) (by assumption)
  · rename_i k1 hd i hi top htop tc htc hp he k2 hkk
    obtain ⟨hsu, hpt⟩ := pt_of_stack htop htc
    have hiv : i.val = k.sched.depth.val - 1 := by
      have h1 : (1#usize).val = 1 := rfl
      have := (usize_sub_ok hi).2
      omega
    have hin : inP (stk k.sched (k.sched.depth.val - 1)) :=
      hP _ (by rw [← hiv, hpt, hp])
    obtain ⟨d1, s1, w1, t1⟩ := end_top_sk he hin
    exact ⟨SR.then_qs (SR.then_qs ⟨s1, Or.inr ⟨d1, w1⟩⟩ hk) hc,
      hc.2.2.2.trans (hk.2.2.2.trans t1)⟩
  · exact ⟨SR.then_qs (SR.of_qs hk) hc, hc.2.2.2.trans hk.2.2.2⟩
  · exact ⟨SR.then_qs (SR.of_qs hk) hc, hc.2.2.2.trans hk.2.2.2⟩

theorem stop_partition_sr {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {partition cause : U8}
    (hP : ∀ u, ptOf cfg u = partition.val → inP u)
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') :
    SR cfg inP k k' ∧ k'.timing = k.timing := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  rename_i k1 hp k2 he a ha
  obtain ⟨e1, e2⟩ := end_partition_jobs_sr (inP := inP) hP he
  exact ⟨SR.congr (push_event_sched hp) (push_event_slots hp) rfl rfl e1,
    e2.trans (push_event_tm hp)⟩

theorem restart_partition_sr {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {partition cause : U8}
    (hP : ∀ u, ptOf cfg u = partition.val → inP u)
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') :
    SR cfg inP k k' ∧ k'.timing = k.timing := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  rename_i k1 hp k2 he a ha a1 ha1
  obtain ⟨e1, e2⟩ := end_partition_jobs_sr (inP := inP) hP he
  exact ⟨SR.congr (push_event_sched hp) (push_event_slots hp) rfl rfl e1,
    e2.trans (push_event_tm hp)⟩

set_option maxHeartbeats 16000000 in
/-- **A response on the slots and the timing state.** With `inP` holding
of the Tasks of the response's Partition and of the response's Task: the
slots written are slots those Tasks own and, when the executing Job was
ended, the Budget slot of the Task resumed (`SR`); the timing state is as
it was but for the mask of the response's Task. No premise on the
configuration or the state. -/
theorem apply_response_sr {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {partition response cause t : U8}
    (hP : ∀ u, ptOf cfg u = partition.val → inP u) (hint : inP t.val)
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') :
    SR cfg inP k k' ∧ MaskOnly t.val k.timing k'.timing := by
  unfold logic.kernel.Kernel.apply_response at h
  peel h
  all_goals try first
    | exact ⟨SR.of_qs (QS.refl _ _ _), MaskOnly.refl _ _⟩
    | exact ⟨(restart_partition_sr (inP := inP) hP h).1,
        MaskOnly.of_eq (restart_partition_sr (inP := inP) hP h).2⟩
    | exact ⟨(stop_partition_sr (inP := inP) hP h).1,
        MaskOnly.of_eq (stop_partition_sr (inP := inP) hP h).2⟩
    | exact ⟨SR.of_qs (enter_safe_state_qs (inP := inP) h),
        MaskOnly.of_eq (enter_safe_state_qs (inP := inP) h).2.2.2⟩
  all_goals
    have hp1 := push_event_sched (by assumption)
    have hp2 := push_event_slots (by assumption)
    have hp3 := push_event_tm (by assumption)
    have hsub : ∃ i, k.sched.depth - 1#usize = ok i ∧ ∃ top, k.sched.stack.index_usize i = ok top ∧
        top = t := ⟨_, by assumption, _, by assumption, by assumption⟩
    obtain ⟨i, hi, top, htop, htt⟩ := hsub
    have htp := (top_of hi htop).2
    rw [htt] at htp
    have he := end_top_sk (inP := inP) (cfg := cfg) (by assumption)
      (by rw [hp1, htp]; exact hint)
    rw [hp1, hp2, hp3] at he
    obtain ⟨d1, s1, w1, t1⟩ := he
    first
      | exact ⟨⟨s1, Or.inr ⟨d1, w1⟩⟩, MaskOnly.of_eq t1⟩
      | (obtain ⟨w2, m2⟩ := end_mit_window_sk (inP := inP) h hint
         have es := end_mit_window_sched h
         refine ⟨SR.step ⟨s1, Or.inr ⟨d1, w1⟩⟩ (by rw [es]) (fun p => by rw [es]) w2, ?_⟩
         rw [← t1]
         exact m2)

/-! ## With the slots apart -/

/-- A slot owned by Task `v` is none of the slots of another Task `u`. -/
theorem own_apart {cfg : logic.Config NT NR NP} (hw : SlotsApart cfg) {u v j : Nat}
    (hu : u < NT.val) (hv : v < NT.val) (hne : u ≠ v) (ho : Own cfg v j) :
    j ≠ dslot cfg u ∧ j ≠ bslot cfg u ∧ (needsRel cfg u → j ≠ rslot cfg u) := by
  obtain ⟨a1, a2, a3, a4⟩ := hw.apart u v hu hv hne
  obtain ⟨b1, b2, b3, b4⟩ := hw.apart v u hv hu (Ne.symm hne)
  have ou := (hw.own u hu).2
  have ov := (hw.own v hv).2
  rcases ho with e | e | ⟨nv, e⟩
  · refine ⟨by omega, by omega, fun nu => ?_⟩
    have := a4 (ou nu).1
    omega
  · refine ⟨by omega, by omega, fun nu => ?_⟩
    have := a4 (ou nu).1
    omega
  · have := b4 (ov nv).1
    exact ⟨by omega, by omega, fun _ => by omega⟩

/-- The Budget slot of Task `r` is no deadline slot and no release slot,
and the Budget slot of no other Task. -/
theorem budget_apart {cfg : logic.Config NT NR NP} (hw : SlotsApart cfg) {u r : Nat}
    (hu : u < NT.val) (hr : r < NT.val) :
    bslot cfg r ≠ dslot cfg u ∧ (needsRel cfg u → bslot cfg r ≠ rslot cfg u) ∧
    (u ≠ r → bslot cfg r ≠ bslot cfg u) := by
  by_cases hne : u = r
  · subst hne
    obtain ⟨o1, o2⟩ := hw.own u hu
    refine ⟨fun e => o1 e.symm, fun nu e => (o2 nu).2.2 e.symm, fun h => absurd rfl h⟩
  · obtain ⟨a1, a2, a3, a4⟩ := hw.apart u r hu hr hne
    have ou := (hw.own u hu).2
    refine ⟨fun e => a3 e.symm, fun nu e => (a4 (ou nu).1).2.1 e.symm, fun _ e => a2 e.symm⟩

/-- **The slots of a Task outside are not among those a response writes**,
but its Budget slot when its Job is the one resumed: with the slots apart,
`SR` leaves the deadline slot and the release slot of every Task outside
`inP` as they were, and its Budget slot as it was when the depth of the
stack is as it was, or no Job is left on it, or the Task is not the one on
top (`task_unchanged`'s three clauses on the slots). -/
theorem sr_frame {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE} (hw : SlotsApart cfg)
    (h : SR cfg inP k k') (u : Nat) (hu : u < NT.val) (hn : ¬ inP u) :
    slotAt k'.slots (dslot cfg u) = slotAt k.slots (dslot cfg u) ∧
    (needsRel cfg u → slotAt k'.slots (rslot cfg u) = slotAt k.slots (rslot cfg u)) ∧
    ((k'.sched.depth = k.sched.depth ∨ k'.sched.depth.val = 0 ∨
        u ≠ stk k'.sched (k'.sched.depth.val - 1)) →
      slotAt k'.slots (bslot cfg u) = slotAt k.slots (bslot cfg u)) := by
  have nw : ∀ j, ¬ WP cfg inP j ∨ (j ≠ dslot cfg u ∧ j ≠ bslot cfg u ∧
      (needsRel cfg u → j ≠ rslot cfg u)) := by
    intro j
    by_cases hj : WP cfg inP j
    · obtain ⟨v, hv, hpv, ho⟩ := hj
      have hne : u ≠ v := fun e => hn (by rw [e]; exact hpv)
      exact Or.inr (own_apart hw hu hv hne ho)
    · exact Or.inl hj
  have nd : ¬ WP cfg inP (dslot cfg u) := fun hj =>
    (nw _).elim (fun g => g hj) (fun g => g.1 rfl)
  have nb : ¬ WP cfg inP (bslot cfg u) := fun hj =>
    (nw _).elim (fun g => g hj) (fun g => g.2.1 rfl)
  have nr : needsRel cfg u → ¬ WP cfg inP (rslot cfg u) := fun nu hj =>
    (nw _).elim (fun g => g hj) (fun g => g.2.2 nu rfl)
  obtain ⟨_, hcase⟩ := h
  rcases hcase with ⟨hd, hsk⟩ | ⟨hd, hsk⟩
  · exact ⟨hsk _ nd, fun nu => hsk _ (nr nu), fun _ => hsk _ nb⟩
  · refine ⟨hsk _ ?_, fun nu => hsk _ ?_, fun hc => hsk _ ?_⟩
    · rintro (hj | ⟨_, hr, e⟩)
      · exact nd hj
      · exact (budget_apart hw hu hr).1 e.symm
    · rintro (hj | ⟨_, hr, e⟩)
      · exact nr nu hj
      · exact (budget_apart hw hu hr).2.1 nu e.symm
    · rintro (hj | ⟨hpos, hr, e⟩)
      · exact nb hj
      · rcases hc with c | c | c
        · rw [c] at hd
          omega
        · omega
        · exact (budget_apart hw hu hr).2.2 c e.symm

/-! ## The statements -/

/-- **A response leaves the timer slots of the Tasks outside its Partition
alone** (`others_unchanged`, the three clauses of `task_unchanged` on the
slots), from the slots apart. For every Task `u` that is not of the
response's Partition and is not the response's Task: `u`'s deadline slot
is as it was, its release slot when it has one is as it was, and its Budget
slot is as it was unless the response ended the executing Job (the depth
of the stack changed), a Job is left on the stack, and `u` is the Task on
top of it: the Task resumed, whose Budget slot `arm_budget` re-arms. -/
theorem apply_response_slots {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8} (hw : SlotsApart cfg)
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k')
    (u : Nat) (hu : u < NT.val) (hpu : ptOf cfg u ≠ partition.val) (hut : u ≠ t.val) :
    slotAt k'.slots (dslot cfg u) = slotAt k.slots (dslot cfg u) ∧
    (needsRel cfg u → slotAt k'.slots (rslot cfg u) = slotAt k.slots (rslot cfg u)) ∧
    ((k'.sched.depth = k.sched.depth ∨ k'.sched.depth.val = 0 ∨
        u ≠ stk k'.sched (k'.sched.depth.val - 1)) →
      slotAt k'.slots (bslot cfg u) = slotAt k.slots (bslot cfg u)) :=
  sr_frame hw
    (apply_response_sr (inP := fun v => ptOf cfg v = partition.val ∨ v = t.val)
      (fun _ hv => Or.inl hv) (Or.inr rfl) h).1 u hu
    (fun hv => hv.elim hpu hut)

/-- **A response writes no timing field but the mask of its Task**: every
array of the timing state is as it was (the nominal instants and their
indices, the MIT history, the deferral instants, the deadline references,
the mask instants), and so is the mask of every Task but the response's.
No premise. -/
theorem apply_response_timing {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') :
    MaskOnly t.val k.timing k'.timing :=
  (apply_response_sr (inP := fun v => ptOf cfg v = partition.val ∨ v = t.val)
    (fun _ hv => Or.inl hv) (Or.inr rfl) h).2

/-- The timing fields of Task `u` (`task_unchanged`'s, and the mask
instant). -/
def timingAt (tm : logic.time.Timing NT) (u : Nat) :
    U64 × U64 × Bool × U64 × U64 × Bool × U64 × U64 × U64 :=
  (tm.k.val[u]!, tm.nominal.val[u]!, tm.has_last.val[u]!, tm.last_due.val[u]!,
    tm.deferred_until.val[u]!, tm.masked.val[u]!, tm.mask_until.val[u]!, tm.due.val[u]!,
    tm.job_due.val[u]!)

theorem MaskOnly.at {t : Nat} {tm tm' : logic.time.Timing NT} (h : MaskOnly t tm tm') (u : Nat)
    (hu : u ≠ t) : timingAt tm' u = timingAt tm u := by
  obtain ⟨h1, h2, h3, h4, h5, h6, h7, h8, h9⟩ := h
  unfold timingAt
  rw [h1, h2, h3, h4, h5, h6, h7, h8, h9 u hu]

/-- **A Health_Monitor response leaves the timer slots and the timing
fields of the other Partitions' Tasks alone** (R14.9;
`Kernel::others_unchanged`, the clauses of `task_unchanged` on the slots
and the timing state), for a configuration `Config::validate` accepts and
a response for a Task of the response's Partition. For every Task `u` of
another Partition:

- `u`'s deadline slot is as it was: armed or not, its instant, its kind
  and the Task it names;
- `u`'s release slot is as it was when `u` has one (its entry is not
  `NO_SLOT`);
- `u`'s Budget slot is as it was when the depth of the stack is as it was,
  or no Job is left on the stack, or `u` is not the Task on top of it. The
  one exception: the response ended the executing Job and `u`'s Job is the
  one that resumed, whose Budget slot `arm_budget` re-arms for the
  remaining Budget;
- every timing field of `u` is as it was. -/
theorem apply_response_others {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (hv : logic.Config.validate cfg = ok true) (hpt : ptOf cfg t.val = partition.val)
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k')
    (u : Nat) (hu : u < NT.val) (hpu : ptOf cfg u ≠ partition.val) :
    slotAt k'.slots (dslot cfg u) = slotAt k.slots (dslot cfg u) ∧
    (rslot cfg u ≠ 255 → slotAt k'.slots (rslot cfg u) = slotAt k.slots (rslot cfg u)) ∧
    ((k'.sched.depth = k.sched.depth ∨ k'.sched.depth.val = 0 ∨
        u ≠ stk k'.sched (k'.sched.depth.val - 1)) →
      slotAt k'.slots (bslot cfg u) = slotAt k.slots (bslot cfg u)) ∧
    timingAt k'.timing u = timingAt k.timing u := by
  have hut : u ≠ t.val := fun e => hpu (by rw [e]; exact hpt)
  obtain ⟨s1, s2, s3⟩ := apply_response_slots (validate_apart hv) h u hu hpu hut
  exact ⟨s1, fun hr => s2 (norel_no_slot hv u hu hr), s3, (apply_response_timing h).at u hut⟩

/-- **A fault leaves the timer slots and the timing fields of the other
Partitions' Tasks alone** (`Kernel::fault`, R14.9), for a configuration
`Config::validate` accepts: for every Task `u` whose Partition is not that
of the Task on top of the stack, the four clauses of
`apply_response_others` from the state `Kernel::fault` is given to the
state it returns. Here the premise of `apply_response_others` that the
response's Task is of the response's Partition is met by the code: the
fault's response is applied for the Task on top and that Task's
Partition (`fault_first`). When the fault is the Kernel's or no Job
executes, the safe state is entered and no slot or timing field is
written at all. -/
theorem fault_others {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind : U8} {kernel : Bool}
    (hv : logic.Config.validate cfg = ok true)
    (h : logic.kernel.Kernel.fault k cfg kind kernel = ok k')
    (u : Nat) (hu : u < NT.val)
    (hpu : ptOf cfg u ≠ ptOf cfg (stk k.sched (k.sched.depth.val - 1))) :
    slotAt k'.slots (dslot cfg u) = slotAt k.slots (dslot cfg u) ∧
    (rslot cfg u ≠ 255 → slotAt k'.slots (rslot cfg u) = slotAt k.slots (rslot cfg u)) ∧
    ((k'.sched.depth = k.sched.depth ∨ k'.sched.depth.val = 0 ∨
        u ≠ stk k'.sched (k'.sched.depth.val - 1)) →
      slotAt k'.slots (bslot cfg u) = slotAt k.slots (bslot cfg u)) ∧
    timingAt k'.timing u = timingAt k.timing u := by
  obtain ⟨prs, mid, hc⟩ := fault_first h
  rcases hc with ⟨_, hp, hs⟩ | ⟨t, tc, pc, _, _, ht, htc, _, hp, ha⟩
  · have hq := enter_safe_state_qs (inP := fun _ => False) hs
    have hsl : ∀ j, slotAt k'.slots j = slotAt k.slots j := fun j => by
      rw [← push_event_slots hp]
      refine hq.2.2.1 j (fun hw => ?_)
      obtain ⟨_, _, hf, _⟩ := hw
      exact hf
    exact ⟨hsl _, fun _ => hsl _, fun _ => hsl _, by rw [hq.2.2.2, push_event_tm hp]⟩
  · have hpt : ptOf cfg t.val = tc.partition.val := by
      unfold ptOf
      rw [htc]
    have hpu' : ptOf cfg u ≠ tc.partition.val := by
      rw [← hpt, ht]
      exact hpu
    have hr := apply_response_others hv hpt ha u hu hpu'
    rw [push_event_slots hp, push_event_sched hp, push_event_tm hp] at hr
    exact hr

end P18
