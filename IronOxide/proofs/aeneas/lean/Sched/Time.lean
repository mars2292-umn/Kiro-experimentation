import Sched.Timer
open Aeneas Aeneas.Std Result

/-! # The time base on the code: the reading and the nominal instants

`Sched/Timer.lean` shows that `Kernel::timer_event` serves a slot only when
it is armed and its instant is at or before `Kernel::now`. This file says
what `now` and a Periodic_Task's nominal instant are in the extracted code
(`Sched/Kernel.lean`), each statement of the form "if the function
returns":

- `TimeKept`: the Kernel's reading of the time base (`now`) and the nominal
  instants of the Periodic_Tasks (`timing.nominal`) are left as they were by
  every function a served slot reaches except the two that are meant to
  change them: `Kernel::observe` (the reading) and
  `Timing::advance_periodic` (the nominal instant).
- `serve_slot_now`, `served_chain_now`: serving slots does not change the
  reading.
- `timer_event_reading`: a timer entry given the reading `now` serves its
  slots under one reading, the larger of `now` and the reading before, so
  every slot it serves has its instant at or before that reading.
- `periodic_release_nominal`: `Kernel::periodic_release` of a
  Periodic_Task leaves the Task's nominal instant where it was with the
  release slot disarmed (no further instant within the operating duration),
  or moves it by exactly the Task's period and arms the release slot for
  the new instant, for this Task.

What is not here: the run of `Sched/KRun.lean` still has its own instants.
Joining them to `now` needs the release slot's instant tied to the nominal
instant across every other entry (Verus: `release_slots_consistent`), and
the model's arrival of a Periodic_Task's Job defined as that instant. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The reading of the time base and the nominal instants are those of `k`. -/
@[reducible] def TimeKept (k k' : logic.kernel.Kernel NT NR NP NL NS NE) : Prop :=
  k'.now = k.now ∧ k'.timing.nominal = k.timing.nominal

/-- One step of a chain of equations through the calls of a function: the
goal `a.now = b.now` (or the same for the nominal instants) is reduced
through the call that returned `a`. Each lemma below adds its rule. -/
syntax "tk_step" : tactic
macro_rules | `(tactic| tk_step) => `(tactic| with_reducible rfl)

/-- Closes `TimeKept k k'` after `peel`. -/
syntax "tk_close" : tactic
macro_rules
  | `(tactic| tk_close) =>
    `(tactic| (refine ⟨?_, ?_⟩ <;> (repeat (first | tk_step | dsimp only))))

/-! ## The functions of `Timing` reached by a response -/

theorem try_unmask_nom {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {now : U64} {je b : Bool}
    (h : logic.time.Timing.try_unmask tm cfg t now je = ok (b, tm')) :
    tm'.nominal = tm.nominal := by
  unfold logic.time.Timing.try_unmask at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (try_unmask_nom (by assumption)).trans ?_)

theorem deferred_due_clamped_nom {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8} {«at» : U64}
    (h : logic.time.Timing.deferred_due_clamped tm cfg t «at» = ok tm') : tm'.nominal = tm.nominal := by
  unfold logic.time.Timing.deferred_due_clamped at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (deferred_due_clamped_nom (by assumption)).trans ?_)

theorem periodic_due_nom {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8} {«at» : U64}
    (h : logic.time.Timing.periodic_due tm cfg t «at» = ok tm') : tm'.nominal = tm.nominal := by
  unfold logic.time.Timing.periodic_due at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (periodic_due_nom (by assumption)).trans ?_)

theorem unmask_nom {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.time.Timing.unmask tm cfg t = ok tm') : tm'.nominal = tm.nominal := by
  unfold logic.time.Timing.unmask at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (unmask_nom (by assumption)).trans ?_)

theorem mask_nom {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8} {now : U64}
    (h : logic.time.Timing.mask tm cfg t now = ok tm') : tm'.nominal = tm.nominal := by
  unfold logic.time.Timing.mask at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (mask_nom (by assumption)).trans ?_)

theorem accept_nom {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8} {now : U64}
    (h : logic.time.Timing.accept tm cfg t now = ok tm') : tm'.nominal = tm.nominal := by
  unfold logic.time.Timing.accept at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (accept_nom (by assumption)).trans ?_)

theorem job_started_nom {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.time.Timing.job_started tm cfg t = ok tm') : tm'.nominal = tm.nominal := by
  unfold logic.time.Timing.job_started at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (job_started_nom (by assumption)).trans ?_)

/-! ## The Kernel functions without a loop -/

theorem push_event_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    TimeKept k k' := by
  unfold logic.kernel.Kernel.push_event at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (push_event_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (push_event_tk (by assumption)).2.trans ?_)

theorem arm_budget_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.arm_budget at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (arm_budget_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (arm_budget_tk (by assumption)).2.trans ?_)

theorem end_mit_window_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.end_mit_window at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (end_mit_window_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (end_mit_window_tk (by assumption)).2.trans ?_)

theorem enter_safe_state_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.enter_safe_state k cfg = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (enter_safe_state_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (enter_safe_state_tk (by assumption)).2.trans ?_)

theorem kill_at_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.kill_at k cfg i = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.kill_at at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (kill_at_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (kill_at_tk (by assumption)).2.trans ?_)

theorem end_top_release_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top_release k cfg = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.end_top_release at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (end_top_release_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (end_top_release_tk (by assumption)).2.trans ?_)

theorem end_top_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top k cfg = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.end_top at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (end_top_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (end_top_tk (by assumption)).2.trans ?_)

/-! ## The loops of a Partition's stop and restart -/

/-- `kill_partition_jobs`' loop. -/
theorem kill_loop_tk {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) res,
      k.sched.depth.val - i.val = n →
      logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i = ok res →
      ∃ k2 : logic.kernel.Kernel NT NR NP NL NS NE, TimeKept k k2 ∧
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
    exact ⟨k, ⟨rfl, rfl⟩, h.symm⟩
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
    have hcase : self1.sched.depth = k.sched.depth ∧ TimeKept k self1 := by
      peel hself1
      · exact ⟨(kill_at_r hiv (by assumption)).2, kill_at_tk (by assumption)⟩
      · exact ⟨rfl, rfl, rfl⟩
      · exact ⟨rfl, rfl, rfl⟩
    obtain ⟨hd, htk⟩ := hcase
    obtain ⟨k2, h2, e⟩ := ih self1 i2 res (by rw [hd]; omega) hrec
    exact ⟨k2, ⟨h2.1.trans htk.1, h2.2.trans htk.2⟩, e⟩

theorem kill_partition_jobs_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨res, hres, h⟩ := h
  obtain ⟨k2, h2, e⟩ := kill_loop_tk _ _ _ res rfl hres
  subst e
  simp only [Aeneas.Std.uncurry] at h
  have h := ok_inj h
  subst h
  exact h2

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (kill_partition_jobs_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (kill_partition_jobs_tk (by assumption)).2.trans ?_)

/-- `clear_partition_releases`' loop. -/
theorem clear_loop_tk {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NT.val - t.val = n →
      logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k' →
      TimeKept k k' := by
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
    exact ⟨rfl, rfl⟩
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
    have hcase : TimeKept k self1 := by
      peel hself1
      all_goals tk_close
    have h2 := ih self1 t1 k' (by omega) hrec
    exact ⟨h2.1.trans hcase.1, h2.2.trans hcase.2⟩

theorem clear_partition_releases_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') :
    TimeKept k k' := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  rename_i k1 hk1 a ha
  have h2 := clear_loop_tk _ _ _ _ rfl hk1
  exact ⟨h2.1, h2.2⟩

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (clear_partition_releases_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (clear_partition_releases_tk (by assumption)).2.trans ?_)

/-! ## The responses and the handlers of a slot -/

theorem end_partition_jobs_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (end_partition_jobs_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (end_partition_jobs_tk (by assumption)).2.trans ?_)

theorem stop_partition_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (stop_partition_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (stop_partition_tk (by assumption)).2.trans ?_)

theorem restart_partition_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (restart_partition_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (restart_partition_tk (by assumption)).2.trans ?_)

set_option maxHeartbeats 16000000 in
theorem apply_response_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.apply_response at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (apply_response_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (apply_response_tk (by assumption)).2.trans ?_)

theorem deadline_miss_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deadline_miss k cfg t i = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.deadline_miss at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (deadline_miss_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (deadline_miss_tk (by assumption)).2.trans ?_)

theorem mit_violation_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t partition offender : U8}
    (h : logic.kernel.Kernel.mit_violation k cfg t partition offender = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.mit_violation at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (mit_violation_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (mit_violation_tk (by assumption)).2.trans ?_)

set_option maxHeartbeats 16000000 in
theorem budget_event_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.budget_event at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (budget_event_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (budget_event_tk (by assumption)).2.trans ?_)

theorem deferred_release_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deferred_release k cfg t i = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.deferred_release at h
  peel h
  all_goals tk_close

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (deferred_release_tk (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| tk_step) => `(tactic| refine (deferred_release_tk (by assumption)).2.trans ?_)

/-! ## The reading: one per timer entry -/

/-- `Kernel::periodic_release` leaves the reading. -/
theorem periodic_release_now {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.periodic_release k cfg t i = ok k') : k'.now = k.now := by
  unfold logic.kernel.Kernel.periodic_release at h
  peel h
  all_goals (repeat (first | tk_step | dsimp only))

/-- Serving a slot leaves the reading of the time base as it was. -/
theorem serve_slot_now {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k') : k'.now = k.now := by
  unfold logic.kernel.Kernel.serve_slot at h
  peel h
  all_goals first
    | exact periodic_release_now h
    | exact (deferred_release_tk h).1
    | exact (deadline_miss_tk h).1
    | exact (budget_event_tk h).1
    | (repeat (first | tk_step | dsimp only))

/-- So does a chain of served slots. -/
theorem served_chain_now {cfg : logic.Config NT NR NP}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE} (h : ServedChain cfg k k') :
    k'.now = k.now := by
  induction h with
  | done => rfl
  | serve k k1 k' i _ hs _ ih => exact ih.trans (serve_slot_now hs)

/-- `Kernel::observe`: the reading never goes back, and is at least the
value the entry was given. -/
theorem observe_now {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64}
    (h : logic.kernel.Kernel.observe k cfg now = ok k') :
    k'.now.val = max k.now.val now.val := by
  unfold logic.kernel.Kernel.observe at h
  split at h
  · rename_i hgt
    have h := ok_inj h; subst h
    have : k.now.val < now.val := by scalar_tac
    dsimp only
    omega
  · rename_i hgt
    have h := ok_inj h; subst h
    have : ¬ k.now.val < now.val := by scalar_tac
    omega

/-- Every slot of a chain was due at the reading `r`. -/
inductive ServedAt (cfg : logic.Config NT NR NP) (r : Nat) :
    logic.kernel.Kernel NT NR NP NL NS NE → logic.kernel.Kernel NT NR NP NL NS NE → Prop
  | done (k : logic.kernel.Kernel NT NR NP NL NS NE) : ServedAt cfg r k k
  | serve (k k1 k' : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize)
      (hin : i.val < NS.val) (harm : (slotOf k i).armed = true)
      (hat : (slotOf k i).at.val ≤ r)
      (hs : logic.kernel.Kernel.serve_slot k cfg i = ok k1)
      (rest : ServedAt cfg r k1 k') : ServedAt cfg r k k'

theorem served_chain_at {cfg : logic.Config NT NR NP}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE} (h : ServedChain cfg k k') :
    ServedAt cfg k.now.val k k' := by
  induction h with
  | done => exact ServedAt.done _
  | serve k k1 k' i hdue hs _ ih =>
    rw [serve_slot_now hs] at ih
    exact ServedAt.serve k k1 k' i hdue.1 hdue.2.1 hdue.2.2 hs ih

/-- A timer entry given the reading `now` serves its slots under one
reading: the larger of `now` and the reading the Kernel had. Each slot it
serves is armed, and its instant is at or before that reading; the reading
is the Kernel's when the entry returns. -/
theorem timer_event_reading {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64} {rc : U32}
    (h : logic.kernel.Kernel.timer_event k cfg now rc = ok k') :
    k'.now.val = max k.now.val now.val ∧
    ∃ k1 : logic.kernel.Kernel NT NR NP NL NS NE,
      k1.sched = k.sched ∧ ServedAt cfg (max k.now.val now.val) k1 k' := by
  unfold logic.kernel.Kernel.timer_event at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨k0, hobs, h⟩ := h
  have hsch := observe_sched hobs
  have hnow := observe_now hobs
  split at h
  · have h := ok_inj h; subst h
    exact ⟨hnow, k0, hsch, ServedAt.done _⟩
  · simp only [bind_eq_ok] at h
    obtain ⟨b, _, k2, hl0, hl1⟩ := h
    have hch := timer_loops_served hl0 hl1
    have hat := served_chain_at hch
    have hk := served_chain_now hch
    dsimp only at hat hk
    rw [hnow] at hat
    exact ⟨by rw [hk]; exact hnow, { k0 with budget := b }, hsch, hat⟩

/-! ## The nominal instant of a Periodic_Task -/

/-- `Timing::advance_periodic`: no further instant and nothing changed, or
the nominal instant moved by exactly the Task's period. -/
theorem advance_periodic_cases {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {o : Option U64}
    (h : logic.time.Timing.advance_periodic tm cfg t = ok (o, tm')) :
    (o = none ∧ tm' = tm) ∨
    ∃ (tc : logic.TaskCfg) (n next : U64),
      cfg.tasks.index_usize (UScalar.cast .Usize t) = ok tc ∧
      tm.nominal.index_usize (UScalar.cast .Usize t) = ok n ∧
      next.val = n.val + tc.period.val ∧ next.val ≤ cfg.operating_duration.val ∧
      o = some next ∧
      tm.nominal.update (UScalar.cast .Usize t) next = ok tm'.nominal := by
  unfold logic.time.Timing.advance_periodic at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨tc, htc, n, hn, h⟩ := h
  cases hadd : U64.checked_add n tc.period with
  | none =>
    rw [hadd] at h
    simp only at h
    have h := ok_inj h
    injection h with h1 h2
    exact Or.inl ⟨h1.symm, h2.symm⟩
  | some next =>
    rw [hadd] at h
    simp only at h
    split at h
    · rename_i hle
      simp only [bind_eq_ok] at h
      obtain ⟨i3, hi3, h⟩ := h
      split at h
      · have h := ok_inj h
        injection h with h1 h2
        exact Or.inl ⟨h1.symm, h2.symm⟩
      · simp only [bind_eq_ok] at h
        obtain ⟨i5, hi5, i6, hi6, a, ha, a1, ha1, h⟩ := h
        have h := ok_inj h
        injection h with h1 h2
        subst h2
        refine Or.inr ⟨tc, n, next, htc, hn, ?_, by scalar_tac, h1.symm, ha1⟩
        have := U64.checked_add_bv_spec n tc.period
        rw [hadd] at this
        simp only at this
        scalar_tac
    · have h := ok_inj h
      injection h with h1 h2
      exact Or.inl ⟨h1.symm, h2.symm⟩

/-- `Kernel::periodic_release` of a Periodic_Task: whatever it did about
the Release (made it, found an overlap and applied the response, found the
Partition stopped), the reading and every nominal instant are as they were
up to the one call of `advance_periodic`, whose result is the Kernel's
`timing`; the release slot is then disarmed (no further instant) or armed
for the new nominal instant, as a release slot of this Task. -/
theorem periodic_release_nominal {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize} {tc : logic.TaskCfg}
    (h : logic.kernel.Kernel.periodic_release k cfg t i = ok k')
    (htc : cfg.tasks.index_usize (UScalar.cast .Usize t) = ok tc)
    (hper : tc.kind = logic.KIND_PERIODIC) :
    ∃ (k1 : logic.kernel.Kernel NT NR NP NL NS NE) (o : Option U64),
      TimeKept k k1 ∧
      logic.time.Timing.advance_periodic k1.timing cfg t = ok (o, k'.timing) ∧
      match o with
      | none => logic.time.Slots.disarm k1.slots i = ok k'.slots
      | some next =>
        logic.time.Slots.arm k1.slots i next logic.time.SLOT_RELEASE t = ok k'.slots := by
  unfold logic.kernel.Kernel.periodic_release at h
  simp only [bind_eq_ok, lift, bind_ok, htc] at h
  obtain ⟨nominal, hnom, h⟩ := h
  have hk : (tc.kind != logic.KIND_PERIODIC) = false := by rw [hper]; simp
  simp only [hk, Bool.false_eq_true, ite_false, bind_eq_ok] at h
  obtain ⟨i5, hi5, b, hb, self1, hself1, h⟩ := h
  have htk : TimeKept k self1 := by
    peel hself1
    all_goals tk_close
  simp only [Prod.exists] at h
  obtain ⟨o, t1, hadv, h⟩ := h
  peel h
  all_goals
    refine ⟨self1, _, htk, hadv, ?_⟩
    first | assumption | (dsimp only; assumption)

end P18
