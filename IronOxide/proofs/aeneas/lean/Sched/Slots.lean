import Sched.Endpoint
open Aeneas Aeneas.Std Result

/-! # An armed Budget slot is the executing Job's, on the code

The Kernel's Verus invariant `budget_slots_owned` (`Kernel::inv`,
2026-11-15) proven a second time, on the functions as extracted: in every
state the run of `Sched/KRun.lean` reaches from a boot with no Budget slot
armed, a timer slot that is armed with the Budget kind names the Task on
top of the stack and is that Task's Budget slot.

- `BO`: the invariant, stated by the slot's kind (Verus states it by the
  slot's index, which needs the configuration's slots distinct; stated by
  kind it needs nothing of the configuration, since only `arm_budget`
  writes the Budget kind).
- `*_box`: each Kernel function of the run keeps it, with no precondition.
- `kstep_bo`, `kreachb_bo`: so does every transition, and every state
  reached.
- `budget_slot_top`: the consequence the run's Budget transitions take as
  premises (`topBudgetArmed`, the slot's Task on top).

No assumption is made on the configuration or on the arguments of a
transition. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- Slot `i` of a slot table. -/
abbrev slotAt (sl : logic.time.Slots NS) (i : Nat) : logic.time.Slot := sl.slots.val[i]!

/-- An armed slot of the Budget kind names the Task on top of the stack and
is that Task's Budget slot. -/
def BO (cfg : logic.Config NT NR NP) (sc : logic.sched.Sched NT NR NL)
    (sl : logic.time.Slots NS) : Prop :=
  ∀ i, i < NS.val → (slotAt sl i).armed = true → (slotAt sl i).kind = logic.time.SLOT_BUDGET →
    0 < sc.depth.val ∧ (slotAt sl i).task.val = stk sc (sc.depth.val - 1) ∧
    (cfg.slot_budget.val[(slotAt sl i).task.val]!).val = i

/-- No slot is armed with the Budget kind. -/
def NoB (sl : logic.time.Slots NS) : Prop :=
  ∀ i, i < NS.val → (slotAt sl i).armed = true → (slotAt sl i).kind ≠ logic.time.SLOT_BUDGET

theorem BO.of_nob {cfg : logic.Config NT NR NP} {sc : logic.sched.Sched NT NR NL}
    {sl : logic.time.Slots NS} (h : NoB sl) : BO cfg sc sl :=
  fun i hi ha hk => absurd hk (h i hi ha)

/-- With no Job in progress the invariant says no Budget slot is armed. -/
theorem BO.nob_of_idle {cfg : logic.Config NT NR NP} {sc : logic.sched.Sched NT NR NL}
    {sl : logic.time.Slots NS} (h : BO cfg sc sl) (hd : sc.depth.val = 0) : NoB sl := by
  intro i hi ha hk
  have := (h i hi ha hk).1
  omega

/-! ## Arming and disarming a slot -/

theorem disarm_at {sl sl' : logic.time.Slots NS} {j : Usize}
    (h : logic.time.Slots.disarm sl j = ok sl') (i : Nat) :
    j.val < NS.val ∧
    slotAt sl' i = if i = j.val then
      ({ armed := false, «at» := 0#u64, kind := logic.time.SLOT_FREE, task := 0#u8 } :
        logic.time.Slot)
      else slotAt sl i := by
  unfold logic.time.Slots.disarm at h
  simp only [bind_eq_ok] at h
  obtain ⟨a, ha, h⟩ := h
  have h := ok_inj h
  subst h
  obtain ⟨hb, e⟩ := update_ok ha
  refine ⟨hb, ?_⟩
  show a.val[i]! = _
  rw [e, get_set _ _ _ _ hb]

theorem arm_at {sl sl' : logic.time.Slots NS} {j : Usize} {a : U64} {kind task : U8}
    (h : logic.time.Slots.arm sl j a kind task = ok sl') (i : Nat) :
    j.val < NS.val ∧
    slotAt sl' i = if i = j.val then
      ({ armed := true, «at» := a, kind := kind, task := task } : logic.time.Slot)
      else slotAt sl i := by
  unfold logic.time.Slots.arm at h
  simp only [bind_eq_ok] at h
  obtain ⟨a', ha, h⟩ := h
  have h := ok_inj h
  subst h
  obtain ⟨hb, e⟩ := update_ok ha
  refine ⟨hb, ?_⟩
  show a'.val[i]! = _
  rw [e, get_set _ _ _ _ hb]

theorem nob_disarm {sl sl' : logic.time.Slots NS} {j : Usize}
    (h : logic.time.Slots.disarm sl j = ok sl') (hn : NoB sl) : NoB sl' := by
  intro i hi ha
  rw [(disarm_at h i).2] at ha ⊢
  split at ha
  · simp at ha
  · rename_i hne; rw [if_neg hne]; exact hn i hi ha

theorem nob_arm {sl sl' : logic.time.Slots NS} {j : Usize} {a : U64} {kind task : U8}
    (h : logic.time.Slots.arm sl j a kind task = ok sl') (hk : kind ≠ logic.time.SLOT_BUDGET)
    (hn : NoB sl) : NoB sl' := by
  intro i hi ha
  rw [(arm_at h i).2] at ha ⊢
  split
  · exact hk
  · rename_i hne; rw [if_neg hne] at ha; exact hn i hi ha

/-! ## The chain through the calls of a function

`BOX cfg sc sl sc' sl'`: if the invariant holds of the scheduler state and
slot table `(sc, sl)`, it holds of `(sc', sl')`. `bo_step` reduces a goal
of this form through the call that returned `sl'` or `sc'`. -/

@[reducible] def BOX (cfg : logic.Config NT NR NP)
    (sc : logic.sched.Sched NT NR NL) (sl : logic.time.Slots NS)
    (sc' : logic.sched.Sched NT NR NL) (sl' : logic.time.Slots NS) : Prop :=
  BO cfg sc sl → BO cfg sc' sl'

/-- A Kernel function keeps the invariant. -/
@[reducible] def BOK (cfg : logic.Config NT NR NP)
    (k k' : logic.kernel.Kernel NT NR NP NL NS NE) : Prop :=
  BOX cfg k.sched k.slots k'.sched k'.slots

theorem box_trans {cfg : logic.Config NT NR NP}
    {sc scm sc' : logic.sched.Sched NT NR NL} {sl slm sl' : logic.time.Slots NS}
    (g1 : BOX cfg sc sl scm slm) (g2 : BOX cfg scm slm sc' sl') : BOX cfg sc sl sc' sl' :=
  fun h => g2 (g1 h)

theorem box_disarm {cfg : logic.Config NT NR NP}
    {sc sc' : logic.sched.Sched NT NR NL} {sl slm sl' : logic.time.Slots NS} {j : Usize}
    (h : logic.time.Slots.disarm slm j = ok sl') (g : BOX cfg sc sl sc' slm) :
    BOX cfg sc sl sc' sl' := by
  intro hb i hi ha hk
  rw [(disarm_at h i).2] at ha hk ⊢
  split at ha
  · simp at ha
  · rename_i hne
    rw [if_neg hne] at hk ⊢
    exact g hb i hi ha hk

theorem box_arm {cfg : logic.Config NT NR NP}
    {sc sc' : logic.sched.Sched NT NR NL} {sl slm sl' : logic.time.Slots NS} {j : Usize}
    {a : U64} {kind task : U8}
    (h : logic.time.Slots.arm slm j a kind task = ok sl') (hkind : kind ≠ logic.time.SLOT_BUDGET)
    (g : BOX cfg sc sl sc' slm) : BOX cfg sc sl sc' sl' := by
  intro hb i hi ha hk
  rw [(arm_at h i).2] at ha hk ⊢
  split at hk
  · exact absurd hk hkind
  · rename_i hne
    rw [if_neg hne] at ha ⊢
    exact g hb i hi ha hk

/-- The stack and its depth are those of `s`. -/
def StkKept (s s' : logic.sched.Sched NT NR NL) : Prop :=
  (∀ p, stk s' p = stk s p) ∧ s'.depth.val = s.depth.val

theorem box_sched {cfg : logic.Config NT NR NP}
    {sc scm sc' : logic.sched.Sched NT NR NL} {sl sl' : logic.time.Slots NS}
    (hs : StkKept scm sc') (g : BOX cfg sc sl scm sl') : BOX cfg sc sl sc' sl' := by
  intro hb i hi ha hk
  obtain ⟨h1, h2, h3⟩ := g hb i hi ha hk
  rw [hs.2, hs.1]
  exact ⟨h1, h2, h3⟩

theorem stk_make_pending {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : U8} (h : logic.sched.Sched.make_pending s cfg t = ok s') : StkKept s s' := by
  obtain ⟨_, _, _, hstk, hdep, _⟩ := make_pending_views h
  exact ⟨hstk, by rw [hdep]⟩

theorem stk_defer {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : U8} (h : logic.sched.Sched.defer s cfg t = ok s') : StkKept s s' := by
  obtain ⟨_, _, _, hstk, hdep, _⟩ := defer_views h
  exact ⟨hstk, by rw [hdep]⟩

theorem stk_clear_release {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : U8} (h : logic.sched.Sched.clear_release s cfg t = ok s') : StkKept s s' := by
  obtain ⟨_, _, _, hstk, hdep, _⟩ := clear_release_views h
  exact ⟨hstk, by rw [hdep]⟩

theorem stk_kill {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {i : Usize} (h : logic.sched.Sched.kill s cfg i = ok s') : StkKept s s' := by
  obtain ⟨_, _, _, hstk, hdep, _⟩ := kill_views h
  exact ⟨hstk, by rw [hdep]⟩

syntax "bo_step" : tactic
macro_rules | `(tactic| bo_step) => `(tactic| with_reducible exact fun h => h)
macro_rules | `(tactic| bo_step) => `(tactic| refine box_disarm (by assumption) ?_)
macro_rules
  | `(tactic| bo_step) =>
    `(tactic| refine box_arm (by assumption)
        (by simp [logic.time.SLOT_UNMASK, logic.time.SLOT_BUDGET, logic.time.SLOT_RELEASE,
          logic.time.SLOT_DEFER, logic.time.SLOT_DEADLINE, logic.time.SLOT_END,
          logic.time.SLOT_HOUSEKEEPING]) ?_)
macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_sched (stk_make_pending (by assumption)) ?_)
macro_rules | `(tactic| bo_step) => `(tactic| refine box_sched (stk_defer (by assumption)) ?_)
macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_sched (stk_clear_release (by assumption)) ?_)
macro_rules | `(tactic| bo_step) => `(tactic| refine box_sched (stk_kill (by assumption)) ?_)

/-- Closes `BOK cfg k k'` after `peel`. -/
syntax "bo_close" : tactic
macro_rules
  | `(tactic| bo_close) =>
    `(tactic| (show BOX _ _ _ _ _; repeat (first | bo_step | dsimp only)))

/-! ## The Kernel functions that leave the stack -/

theorem push_event_slots {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    k'.slots = k.slots := by
  unfold logic.kernel.Kernel.push_event at h
  frame_ok h

theorem push_event_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    BOK cfg k k' := by
  show BO _ _ _ → BO _ _ _
  rw [push_event_slots h, push_event_sched h]
  exact fun h => h

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (push_event_box (by assumption)))

theorem slot_budget_idx {cfg : logic.Config NT NR NP} {t b : U8}
    (h : cfg.slot_budget.index_usize (UScalar.cast .Usize t) = ok b) :
    (cfg.slot_budget.val[t.val]!).val = (UScalar.cast .Usize b : Usize).val := by
  obtain ⟨_, e⟩ := index_ok h
  rw [U8.cast_Usize_val_eq] at e
  rw [U8.cast_Usize_val_eq, e]

theorem box_arm_budget {cfg : logic.Config NT NR NP}
    {sc sc' : logic.sched.Sched NT NR NL} {sl slm sl' : logic.time.Slots NS} {j : Usize}
    {a : U64} {t : U8}
    (h : logic.time.Slots.arm slm j a logic.time.SLOT_BUDGET t = ok sl')
    (hd : 0 < sc'.depth.val) (ht : stk sc' (sc'.depth.val - 1) = t.val)
    (hj : (cfg.slot_budget.val[t.val]!).val = j.val)
    (g : BOX cfg sc sl sc' slm) : BOX cfg sc sl sc' sl' := by
  intro hb i hi ha hk
  rw [(arm_at h i).2] at ha hk ⊢
  split at hk
  · rename_i he
    rw [if_pos he]
    exact ⟨hd, ht.symm, by rw [he]; exact hj⟩
  · rename_i hne
    rw [if_neg hne] at ha ⊢
    exact g hb i hi ha hk

/-- `arm_budget` for the Task on top keeps the invariant. -/
theorem arm_budget_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.arm_budget k cfg t = ok k')
    (hd : 0 < k.sched.depth.val) (ht : stk k.sched (k.sched.depth.val - 1) = t.val) :
    BOK cfg k k' := by
  unfold logic.kernel.Kernel.arm_budget at h
  peel h
  all_goals
    show BOX _ _ _ _ _
    dsimp only
    exact box_arm_budget (by assumption) hd ht (slot_budget_idx (by assumption)) (fun h => h)

theorem end_mit_window_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.end_mit_window at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (end_mit_window_box (by assumption)))

theorem kill_at_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.kill_at k cfg i = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.kill_at at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (kill_at_box (by assumption)))

theorem enter_safe_state_loop_stk {cfg : logic.Config NT NR NP} :
    ∀ (n : Nat) (s s' : logic.sched.Sched NT NR NL) (t : Usize), NT.val - t.val = n →
      logic.kernel.Kernel.enter_safe_state_loop s cfg t = ok s' → StkKept s s' := by
  intro n
  induction n with
  | zero =>
    intro s s' t hn h
    unfold logic.kernel.Kernel.enter_safe_state_loop loop at h
    simp only [logic.kernel.Kernel.enter_safe_state_loop.body] at h
    have hlt : ¬ t < NT := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    exact ⟨fun _ => rfl, rfl⟩
  | succ n ih =>
    intro s s' t hn h
    unfold logic.kernel.Kernel.enter_safe_state_loop loop at h
    simp only [logic.kernel.Kernel.enter_safe_state_loop.body] at h
    have hlt : t < NT := by scalar_tac
    simp only [hlt, ite_true, lift, bind_ok, bind_eq_ok] at h
    obtain ⟨cf, ⟨s1, hs1, t1, ht1, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    have ht1 := usize_add_ok ht1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at ht1
    have hrec : logic.kernel.Kernel.enter_safe_state_loop s1 cfg t1 = ok s' := by
      unfold logic.kernel.Kernel.enter_safe_state_loop
      simp only [logic.kernel.Kernel.enter_safe_state_loop.body, lift, bind_ok]
      exact hloop
    have a := stk_clear_release hs1
    have b := ih s1 s' t1 (by omega) hrec
    exact ⟨fun p => (b.1 p).trans (a.1 p), b.2.trans a.2⟩

macro_rules
  | `(tactic| bo_step) =>
    `(tactic| refine box_sched (enter_safe_state_loop_stk _ _ _ _ rfl (by assumption)) ?_)

theorem enter_safe_state_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.enter_safe_state k cfg = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (enter_safe_state_box (by assumption)))

/-! ## The functions that change the stack

Each disarms the Budget slot of the Job on top before the stack changes,
so that no Budget slot is armed while it does, and arms the Budget slot of
the Job that is on top after. -/

theorem top_of {sc : logic.sched.Sched NT NR NL} {i1 : Usize} {top : U8}
    (hi : sc.depth - 1#usize = ok i1) (hp : sc.stack.index_usize i1 = ok top) :
    0 < sc.depth.val ∧ stk sc (sc.depth.val - 1) = top.val := by
  obtain ⟨hle, hv⟩ := usize_sub_ok hi
  obtain ⟨_, e⟩ := index_ok hp
  have h1 : (1#usize).val = 1 := rfl
  rw [h1] at hle hv
  refine ⟨by omega, ?_⟩
  unfold stk
  rw [← hv, e]

theorem nob_top_disarm {cfg : logic.Config NT NR NP} {sc : logic.sched.Sched NT NR NL}
    {sl sl' : logic.time.Slots NS} (hb : BO cfg sc sl) {i1 : Usize} {top b : U8}
    (hi : sc.depth - 1#usize = ok i1) (hp : sc.stack.index_usize i1 = ok top)
    (hsb : cfg.slot_budget.index_usize (UScalar.cast .Usize top) = ok b)
    (hd : logic.time.Slots.disarm sl (UScalar.cast .Usize b) = ok sl') : NoB sl' := by
  intro i hi' ha hk
  rw [(disarm_at hd i).2] at ha hk
  split at ha
  · simp at ha
  · rename_i hne
    rw [if_neg hne] at hk
    obtain ⟨_, htk, hix⟩ := hb i hi' ha hk
    rw [(top_of hi hp).2] at htk
    rw [htk, slot_budget_idx hsb] at hix
    exact hne hix.symm

theorem start_top {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.sched.Sched.start s cfg t = ok s') :
    0 < s'.depth.val ∧ stk s' (s'.depth.val - 1) = t.val := by
  obtain ⟨_, _, _, _, hstk, hdep, _⟩ := start_views h
  refine ⟨by omega, ?_⟩
  rw [hstk, hdep]
  simp

syntax "nb_step" : tactic
macro_rules | `(tactic| nb_step) => `(tactic| assumption)
macro_rules | `(tactic| nb_step) => `(tactic| refine nob_disarm (by assumption) ?_)
macro_rules
  | `(tactic| nb_step) =>
    `(tactic| refine nob_arm (by assumption)
        (by simp [logic.time.SLOT_UNMASK, logic.time.SLOT_BUDGET, logic.time.SLOT_RELEASE,
          logic.time.SLOT_DEFER, logic.time.SLOT_DEADLINE, logic.time.SLOT_END,
          logic.time.SLOT_HOUSEKEEPING]) ?_)
macro_rules
  | `(tactic| nb_step) => `(tactic| rw [push_event_slots (by assumption)])
macro_rules
  | `(tactic| nb_step) =>
    `(tactic| exact nob_top_disarm (by assumption) (by assumption) (by assumption)
        (by assumption) (by assumption))
macro_rules
  | `(tactic| nb_step) =>
    `(tactic| exact BO.nob_of_idle (by assumption) (by scalar_tac))

/-- Closes `NoB sl'` after `peel`, from the invariant in the context. -/
syntax "nb_close" : tactic
macro_rules
  | `(tactic| nb_close) => `(tactic| (repeat (first | nb_step | dsimp only)))

theorem end_mit_window_nob {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') (hn : NoB k.slots) :
    NoB k'.slots := by
  unfold logic.kernel.Kernel.end_mit_window at h
  peel h
  all_goals nb_close

macro_rules
  | `(tactic| nb_step) => `(tactic| refine end_mit_window_nob (by assumption) ?_)

theorem end_top_release_nob {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top_release k cfg = ok k') (hb : BO cfg k.sched k.slots) :
    NoB k'.slots := by
  unfold logic.kernel.Kernel.end_top_release at h
  peel h
  all_goals nb_close

theorem end_top_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top k cfg = ok k') : BOK cfg k k' := by
  intro hb
  unfold logic.kernel.Kernel.end_top at h
  peel h
  · rename_i k1 hk1 _ i1 hi1 top htop
    exact arm_budget_box h (top_of hi1 htop).1 (top_of hi1 htop).2
      (BO.of_nob (end_top_release_nob hk1 hb))
  · exact BO.of_nob (end_top_release_nob (by assumption) hb)

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (end_top_box (by assumption)))

set_option maxHeartbeats 8000000 in
theorem job_return_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {ec : U32}
    (h : logic.kernel.Kernel.job_return k cfg ec = ok k') : BOK cfg k k' := by
  intro hb
  unfold logic.kernel.Kernel.job_return at h
  peel h
  all_goals first
    | (refine arm_budget_box h (top_of (by assumption) (by assumption)).1
        (top_of (by assumption) (by assumption)).2 (BO.of_nob ?_)
       nb_close)
    | (refine BO.of_nob ?_
       nb_close)

set_option maxHeartbeats 8000000 in
theorem dispatch_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {pc : U32}
    (h : logic.kernel.Kernel.dispatch k cfg t pc = ok k') : BOK cfg k k' := by
  intro hb
  unfold logic.kernel.Kernel.dispatch at h
  peel h
  all_goals
    refine arm_budget_box h (start_top (by assumption)).1 (start_top (by assumption)).2
      (BO.of_nob ?_)
    nb_close

/-! ## The loops of a Partition's stop and restart -/

theorem kill_loop_box {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) res,
      k.sched.depth.val - i.val = n →
      logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i = ok res →
      ∃ k2 : logic.kernel.Kernel NT NR NP NL NS NE, BOK cfg k k2 ∧
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
    have hcase : self1.sched.depth = k.sched.depth ∧ BOK cfg k self1 := by
      peel hself1
      · exact ⟨(kill_at_r hiv (by assumption)).2, kill_at_box (by assumption)⟩
      · exact ⟨rfl, fun h => h⟩
      · exact ⟨rfl, fun h => h⟩
    obtain ⟨hd, hbk⟩ := hcase
    obtain ⟨k2, h2, e⟩ := ih self1 i2 res (by rw [hd]; omega) hrec
    exact ⟨k2, fun h => h2 (hbk h), e⟩

theorem kill_partition_jobs_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨res, hres, h⟩ := h
  obtain ⟨k2, h2, e⟩ := kill_loop_box _ _ _ res rfl hres
  subst e
  simp only [Aeneas.Std.uncurry] at h
  have h := ok_inj h
  subst h
  exact h2

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (kill_partition_jobs_box (by assumption)))

set_option maxHeartbeats 8000000 in
theorem clear_loop_box {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NT.val - t.val = n →
      logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k' →
      BOK cfg k k' := by
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
    exact fun h => h
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
    have hcase : BOK cfg k self1 := by
      peel hself1
      all_goals bo_close
    have h2 := ih self1 t1 k' (by omega) hrec
    exact fun h => h2 (hcase h)

theorem clear_partition_releases_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') :
    BOK cfg k k' := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  rename_i k1 hk1 a ha
  refine box_trans (clear_loop_box _ _ _ _ rfl hk1) ?_
  bo_close

macro_rules
  | `(tactic| bo_step) =>
    `(tactic| refine box_trans ?_ (clear_partition_releases_box (by assumption)))

/-! ## The responses and the handlers of a slot -/

theorem end_partition_jobs_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (end_partition_jobs_box (by assumption)))

theorem stop_partition_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (stop_partition_box (by assumption)))

theorem restart_partition_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (restart_partition_box (by assumption)))

set_option maxHeartbeats 16000000 in
theorem apply_response_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.apply_response at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (apply_response_box (by assumption)))

theorem deadline_miss_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deadline_miss k cfg t i = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.deadline_miss at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (deadline_miss_box (by assumption)))

theorem mit_violation_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t partition offender : U8}
    (h : logic.kernel.Kernel.mit_violation k cfg t partition offender = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.mit_violation at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (mit_violation_box (by assumption)))

theorem fault_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind : U8} {kn : Bool}
    (h : logic.kernel.Kernel.fault k cfg kind kn = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.fault at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (fault_box (by assumption)))

theorem fault_of_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind t : U8}
    (h : logic.kernel.Kernel.fault_of k cfg kind t = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.fault_of at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (fault_of_box (by assumption)))

theorem record_event_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind t : U8}
    (h : logic.kernel.Kernel.record_event k cfg kind t = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.record_event at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (record_event_box (by assumption)))

theorem observe_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64}
    (h : logic.kernel.Kernel.observe k cfg now = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.observe at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (observe_box (by assumption)))

set_option maxHeartbeats 16000000 in
/-- `budget_event` arms the Budget slot again only for the Task it found on
top of the stack. -/
theorem budget_event_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.budget_event at h
  peel h
  case isFalse.isFalse.isFalse.isFalse =>
    rename_i s hs hd0 i1 hi1 top htop hne b hb hnb ov hov hnov
    have htp := top_of hi1 htop
    have e : top = t := by
      by_contra hc
      exact hne (bne_iff_ne.mpr hc)
    refine box_trans ?_ (arm_budget_box h htp.1
      (by show stk k.sched (k.sched.depth.val - 1) = t.val; rw [htp.2, e]))
    bo_close
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (budget_event_box (by assumption)))

set_option maxHeartbeats 16000000 in
theorem deferred_release_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deferred_release k cfg t i = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.deferred_release at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (deferred_release_box (by assumption)))

set_option maxHeartbeats 16000000 in
theorem periodic_release_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.periodic_release k cfg t i = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.periodic_release at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (periodic_release_box (by assumption)))

set_option maxHeartbeats 16000000 in
theorem intake_release_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t sp off : U8} {now : U64}
    (h : logic.kernel.Kernel.intake_release k cfg t sp now off = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.intake_release at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (intake_release_box (by assumption)))

set_option maxHeartbeats 16000000 in
theorem serve_slot_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k') : BOK cfg k k' := by
  unfold logic.kernel.Kernel.serve_slot at h
  peel h
  all_goals bo_close

macro_rules
  | `(tactic| bo_step) => `(tactic| refine box_trans ?_ (serve_slot_box (by assumption)))

/-- `settle` arms a Budget slot only for the Task it read from the top of
the stack, and otherwise disarms one or leaves the Kernel as it is. -/
theorem settle_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : Option U8 × Bool}
    (h : logic.kernel.Kernel.settle k cfg = ok (r, k')) : BOK cfg k k' := by
  unfold logic.kernel.Kernel.settle at h
  peel h
  all_goals first
    | exact arm_budget_box (by assumption) (top_of (by assumption) (by assumption)).1
        (top_of (by assumption) (by assumption)).2
    | bo_close

theorem svc_lock_slots {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_lock k cfg r = ok (b, k')) : k'.slots = k.slots := by
  unfold logic.kernel.Kernel.svc_lock at h
  peel h
  all_goals first
    | rfl
    | (refine (push_event_slots (by assumption)).trans ?_
       rfl)

theorem svc_unlock_slots {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (b, k')) : k'.slots = k.slots := by
  unfold logic.kernel.Kernel.svc_unlock at h
  peel h
  all_goals first
    | rfl
    | (refine (push_event_slots (by assumption)).trans ?_
       rfl)

theorem svc_lock_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.kernel.Kernel.svc_lock k cfg r = ok (true, k')) : BOK cfg k k' := by
  obtain ⟨_, _, hstk, hdep⟩ := (lock_views (svc_lock_eff h)).1
  show BOX _ _ _ _ _
  rw [svc_lock_slots h]
  exact box_sched ⟨hstk, by rw [hdep]⟩ (fun h => h)

theorem svc_unlock_box {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (true, k')) : BOK cfg k k' := by
  obtain ⟨_, _, hstk, hdep⟩ := (unlock_views (svc_unlock_eff h)).1
  show BOX _ _ _ _ _
  rw [svc_unlock_slots h]
  exact box_sched ⟨hstk, by rw [hdep]⟩ (fun h => h)

/-! ## The run -/

/-- Every transition of the run keeps the invariant. No condition is put on
the transition: the slot index and the Task a timer transition names are
free, and the invariant is kept all the same. -/
theorem kstep_bo {cfg : logic.Config NT NR NP} {P : Params}
    {x y : KSt NT NR NP NL NS NE} (h : KStep cfg P x y)
    (hb : BO cfg x.k.sched x.k.slots) : BO cfg y.k.sched y.k.slots := by
  cases h with
  | jarrive => exact hb
  | earrive => exact hb
  | periodic q t i d c k' hq hk hw hrel hnd hc hs hmade =>
    have := periodic_release_box hs hb
    split <;> exact this
  | intake q t sp off now d c k' hq hk hw hrel hnd hc hs hmade =>
    have := intake_release_box hs hb
    split <;> exact this
  | dispatch t pc c k' hk hsel hc hs => exact dispatch_box hs hb
  | lock hsel hk r a c k' hrun hrl hc ha hal hs => exact svc_lock_box hs hb
  | unlock hsel hk r c k' hrun hru hc hal hs => exact svc_unlock_box hs hb
  | finish hsel hk hj ec c k' hd hr0 hr hc hal hs => exact job_return_box hs hb
  | unwind ec k' hs => exact job_return_box hs hb
  | resp partition response cause t k' hs => exact apply_response_box hs hb
  | faulted kind kn k' hs => exact fault_box hs hb
  | faulted_of kind t k' hs => exact fault_of_box hs hb
  | bserved t i k' hs => exact budget_event_box hs hb
  | opend i k1 k' hcode h1 hs => exact serve_slot_box hcode hb
  | settle r k' hs => exact settle_box hs hb
  | charged t rc b hs => exact hb
  | acked_reinit p k' hs =>
    obtain ⟨a, rfl⟩ := ack_reinit_upd hs
    exact hb
  | acked_stop p k' hs =>
    obtain ⟨a, rfl⟩ := ack_stop_dma_upd hs
    exact hb
  | logged kind task partition prs detail k' hs => exact push_event_box hs hb
  | observed now k' hs => exact observe_box hs hb
  | deferred q t i d c k' hq hk hw hrel hnd hc hs hmade =>
    have := deferred_release_box hs hb
    split <;> exact this
  | deadline q c t i k' hq hk hw hc hdl hs => exact deadline_miss_box hs hb
  | plain_h q c k' hq hk hw hc hnd i hcode hs => exact serve_slot_box hcode hb
  | svc_h hsel hk c k' hd hrun hre hal hcode =>
    rcases hcode with e | ⟨kind, t, h⟩
    · rw [e]; exact hb
    · exact record_event_box h hb
  | bevent_h hq harmed hk c k' hd hrun hrb hc t i hcode hs =>
    exact budget_event_box hcode hb
  | overrun_h hq harmed hk hj c r k' hd hr0 hr hstop hrb hc hal t i hcode hs =>
    exact budget_event_box hcode hb
  | overrun_p hq harmed hk hj c r s1 k' hd hr0 hr hstop hrb hc hal t i hcode hs hrs =>
    exact budget_event_box hcode hb
  | bevent_r hq harmed hk c k' hd hrun hrb hc t i hcode =>
    exact budget_event_box hcode hb
  | ktick => exact hb
  | idle => exact hb
  | exec => exact hb

/-- The states the run reaches from a boot state in which no Budget slot is
armed. -/
inductive KReachB (cfg : logic.Config NT NR NP) (P : Params) : KSt NT NR NP NL NS NE → Prop
  | boot (k0 : logic.kernel.Kernel NT NR NP NL NS NE)
      (h : logic.sched.Sched.new NL cfg = ok k0.sched) (hb : NoB k0.slots) :
      KReachB cfg P (KSt.boot k0)
  | step (x y : KSt NT NR NP NL NS NE) (h : KReachB cfg P x) (hs : KStep cfg P x y) :
      KReachB cfg P y

theorem kreachb_kreach {cfg : logic.Config NT NR NP} {P : Params}
    {x : KSt NT NR NP NL NS NE} (h : KReachB cfg P x) : KReach cfg P x := by
  induction h with
  | boot k0 h0 _ => exact KReach.boot k0 h0
  | step x y _ hs ih => exact KReach.step x y ih hs

/-- **An armed Budget slot is the executing Job's**, in every state the run
reaches (Verus: `budget_slots_owned`). -/
theorem kreachb_bo {cfg : logic.Config NT NR NP} {P : Params}
    {x : KSt NT NR NP NL NS NE} (h : KReachB cfg P x) : BO cfg x.k.sched x.k.slots := by
  induction h with
  | boot k0 _ hb => exact BO.of_nob hb
  | step x y _ hs ih => exact kstep_bo hs ih

/-- What the run's Budget transitions take as premises, from the
invariant: when slot `i` is armed with the Budget kind, a Job is in
progress, the slot's Task is the one on top of the stack, and the Budget
slot of the Job on top is armed (`topBudgetArmed`). -/
theorem budget_slot_top {cfg : logic.Config NT NR NP} {P : Params}
    {x : KSt NT NR NP NL NS NE} (h : KReachB cfg P x) {i : Usize} (hi : i.val < NS.val)
    (ha : (slotOf x.k i).armed = true) (hk : (slotOf x.k i).kind = logic.time.SLOT_BUDGET) :
    0 < x.k.sched.depth.val ∧
    (slotOf x.k i).task.val = stk x.k.sched (x.k.sched.depth.val - 1) ∧
    topBudgetArmed x.k cfg := by
  obtain ⟨hd, ht, hix⟩ := kreachb_bo h i.val hi ha hk
  refine ⟨hd, ht, hd, ?_⟩
  have e : (x.k.sched.stack.val[x.k.sched.depth.val - 1]!).val = (slotOf x.k i).task.val := ht.symm
  rw [e, hix]
  exact ha

/-- A Budget slot that `timer_event` serves as an entry of its own, in a
state the run reaches: the run's `bevent_r` transition, with the two
premises about the slot ("a Job is in progress", "the Budget slot of the
Job on top is armed") derived from the invariant and no longer asked. -/
theorem KStep.served_bevent_reach {cfg : logic.Config NT NR NP} {P : Params}
    {x : KSt NT NR NP NL NS NE} (hx : KReachB cfg P x) (i : Usize)
    (k' : logic.kernel.Kernel NT NR NP NL NS NE) (c : Nat)
    (hs : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
    (hdue : SlotDue x.k i)
    (hkind : (slotOf x.k i).kind = logic.time.SLOT_BUDGET)
    (ht : (slotOf x.k i).task.val < NT.val)
    (hq : Quieted cfg x.k)
    (hk : x.ks = 0) (hrun : 0 < x.run x.top) (hrb : 0 < x.rb x.top) (hc : c ≤ P.Kt) :
    ∃ y, KStep cfg P x y ∧ y.k = k' := by
  obtain ⟨hd, _, harmed⟩ := budget_slot_top hx hdue.1 hdue.2.1 hkind
  exact KStep.served_bevent_any x i k' c hs hkind ht hq harmed hk hd hrun hrb hc

/-! ## The boot state -/

theorem slots_new_nob {sl : logic.time.Slots NS} (h : logic.time.Slots.new NS = ok sl) :
    NoB sl := by
  unfold logic.time.Slots.new at h
  have h := ok_inj h
  subst h
  intro i hi ha
  simp [slotAt, Array.repeat, hi] at ha

/-- `Kernel::init` leaves no slot armed, whatever the state before. -/
theorem new_nob {cfg : logic.Config NT NR NP} {k0 : logic.kernel.Kernel NT NR NP NL NS NE}
    {kold : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.init kold cfg = ok k0) : NoB k0.slots := by
  unfold logic.kernel.Kernel.init at h
  peel h
  dsimp only
  exact slots_new_nob (by unfold logic.time.Slots.new; rfl)

/-- `Kernel::start`'s loop arms release slots only. -/
theorem start_loop_nob {cfg : logic.Config NT NR NP} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize) res,
      NT.val - t.val = n →
      logic.kernel.Kernel.start_loop k cfg t = ok res → NoB k.slots → NoB res.1.slots := by
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
    have hcase : NoB self1.slots := by
      peel hself1
      all_goals nb_close
    exact ih self1 t1 res (by omega) hrec hcase

/-- `Kernel::start` arms release slots, the slot of the end of the
operating duration and the housekeeping slot: no Budget slot. -/
theorem start_nob {cfg : logic.Config NT NR NP} {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.start k cfg = ok k') (hb : NoB k.slots) : NoB k'.slots := by
  unfold logic.kernel.Kernel.start at h
  peel h
  all_goals
    have hl := start_loop_nob _ _ _ _ rfl (by assumption) hb
    dsimp only at hl
    nb_close

end P18
