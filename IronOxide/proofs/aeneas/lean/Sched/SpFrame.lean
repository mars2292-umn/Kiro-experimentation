import Sched.Arrive
open Aeneas Aeneas.Std Result

/-! # The sporadic timing fields are left alone

`Timing` keeps three things per Sporadic_Task: whether a Release of it was
ever accepted (`has_last`), the instant the last one became due
(`last_due`), and the instant a held event is deferred to
(`deferred_until`). Two Kernel functions are meant to write them:
`Kernel::intake_release` (through `Timing::accept` and `Timing::defer`) and
`Kernel::deferred_release` (through `Timing::deferred_due_clamped`). This
file shows that every other function of the run leaves all three as they
were, each statement of the form "if the function returns". The proofs are
those of `TimeKept` in `Sched/Time.lean` with the fields exchanged
(generated from them; `deferred_release` and the two `Timing` writers left
out). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The last due instants and the deferral instants. -/
def spw (tm : logic.time.Timing NT) : Array U64 NT × Array U64 NT := (tm.last_due, tm.deferred_until)

/-- The three sporadic timing fields are those of `k`. -/
@[reducible] def SpKept (k k' : logic.kernel.Kernel NT NR NP NL NS NE) : Prop :=
  k'.timing.has_last = k.timing.has_last ∧ spw k'.timing = spw k.timing

syntax "sp_step" : tactic
macro_rules | `(tactic| sp_step) => `(tactic| with_reducible rfl)

syntax "sp_close" : tactic
macro_rules
  | `(tactic| sp_close) =>
    `(tactic| (refine ⟨?_, ?_⟩ <;> (repeat (first | sp_step | dsimp only))))

theorem advance_periodic_hl {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {o : Option U64}
    (h : logic.time.Timing.advance_periodic tm cfg t = ok (o, tm')) : tm'.has_last = tm.has_last := by
  unfold logic.time.Timing.advance_periodic at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (advance_periodic_hl (by assumption)).trans ?_)

theorem advance_periodic_sw {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {o : Option U64}
    (h : logic.time.Timing.advance_periodic tm cfg t = ok (o, tm')) : spw tm' = spw tm := by
  unfold logic.time.Timing.advance_periodic at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (advance_periodic_sw (by assumption)).trans ?_)

theorem observe_timing {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64}
    (h : logic.kernel.Kernel.observe k cfg now = ok k') : k'.timing = k.timing := by
  unfold logic.kernel.Kernel.observe at h
  split at h <;> (have h := ok_inj h; subst h; rfl)

theorem try_unmask_hl {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {now : U64} {je b : Bool}
    (h : logic.time.Timing.try_unmask tm cfg t now je = ok (b, tm')) :
    tm'.has_last = tm.has_last := by
  unfold logic.time.Timing.try_unmask at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (try_unmask_hl (by assumption)).trans ?_)

theorem try_unmask_sw {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {now : U64} {je b : Bool}
    (h : logic.time.Timing.try_unmask tm cfg t now je = ok (b, tm')) :
    spw tm' = spw tm := by
  unfold logic.time.Timing.try_unmask at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (try_unmask_sw (by assumption)).trans ?_)


theorem periodic_due_hl {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8} {«at» : U64}
    (h : logic.time.Timing.periodic_due tm cfg t «at» = ok tm') : tm'.has_last = tm.has_last := by
  unfold logic.time.Timing.periodic_due at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (periodic_due_hl (by assumption)).trans ?_)

theorem periodic_due_sw {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8} {«at» : U64}
    (h : logic.time.Timing.periodic_due tm cfg t «at» = ok tm') : spw tm' = spw tm := by
  unfold logic.time.Timing.periodic_due at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (periodic_due_sw (by assumption)).trans ?_)

theorem unmask_hl {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.time.Timing.unmask tm cfg t = ok tm') : tm'.has_last = tm.has_last := by
  unfold logic.time.Timing.unmask at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (unmask_hl (by assumption)).trans ?_)

theorem unmask_sw {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.time.Timing.unmask tm cfg t = ok tm') : spw tm' = spw tm := by
  unfold logic.time.Timing.unmask at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (unmask_sw (by assumption)).trans ?_)

theorem mask_hl {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8} {now : U64}
    (h : logic.time.Timing.mask tm cfg t now = ok tm') : tm'.has_last = tm.has_last := by
  unfold logic.time.Timing.mask at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (mask_hl (by assumption)).trans ?_)

theorem mask_sw {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8} {now : U64}
    (h : logic.time.Timing.mask tm cfg t now = ok tm') : spw tm' = spw tm := by
  unfold logic.time.Timing.mask at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (mask_sw (by assumption)).trans ?_)


theorem job_started_hl {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.time.Timing.job_started tm cfg t = ok tm') : tm'.has_last = tm.has_last := by
  unfold logic.time.Timing.job_started at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (job_started_hl (by assumption)).trans ?_)

theorem job_started_sw {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.time.Timing.job_started tm cfg t = ok tm') : spw tm' = spw tm := by
  unfold logic.time.Timing.job_started at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (job_started_sw (by assumption)).trans ?_)

/-! ## The Kernel functions without a loop -/

theorem push_event_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    SpKept k k' := by
  unfold logic.kernel.Kernel.push_event at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (push_event_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (push_event_sp (by assumption)).2.trans ?_)

theorem arm_budget_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.arm_budget at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (arm_budget_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (arm_budget_sp (by assumption)).2.trans ?_)

theorem end_mit_window_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.end_mit_window at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (end_mit_window_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (end_mit_window_sp (by assumption)).2.trans ?_)

theorem enter_safe_state_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.enter_safe_state k cfg = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (enter_safe_state_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (enter_safe_state_sp (by assumption)).2.trans ?_)

theorem kill_at_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.kill_at k cfg i = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.kill_at at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (kill_at_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (kill_at_sp (by assumption)).2.trans ?_)

theorem end_top_release_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top_release k cfg = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.end_top_release at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (end_top_release_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (end_top_release_sp (by assumption)).2.trans ?_)

theorem end_top_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top k cfg = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.end_top at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (end_top_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (end_top_sp (by assumption)).2.trans ?_)

/-! ## The loops of a Partition's stop and restart -/

/-- `kill_partition_jobs`' loop. -/
theorem kill_loop_sp {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) res,
      k.sched.depth.val - i.val = n →
      logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i = ok res →
      ∃ k2 : logic.kernel.Kernel NT NR NP NL NS NE, SpKept k k2 ∧
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
    have hcase : self1.sched.depth = k.sched.depth ∧ SpKept k self1 := by
      peel hself1
      · exact ⟨(kill_at_r hiv (by assumption)).2, kill_at_sp (by assumption)⟩
      · exact ⟨rfl, rfl, rfl⟩
      · exact ⟨rfl, rfl, rfl⟩
    obtain ⟨hd, htk⟩ := hcase
    obtain ⟨k2, h2, e⟩ := ih self1 i2 res (by rw [hd]; omega) hrec
    exact ⟨k2, ⟨h2.1.trans htk.1, h2.2.trans htk.2⟩, e⟩

theorem kill_partition_jobs_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨res, hres, h⟩ := h
  obtain ⟨k2, h2, e⟩ := kill_loop_sp _ _ _ res rfl hres
  subst e
  simp only [Aeneas.Std.uncurry] at h
  have h := ok_inj h
  subst h
  exact h2

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (kill_partition_jobs_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (kill_partition_jobs_sp (by assumption)).2.trans ?_)

/-- `clear_partition_releases`' loop. -/
theorem clear_loop_sp {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NT.val - t.val = n →
      logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k' →
      SpKept k k' := by
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
    have hcase : SpKept k self1 := by
      peel hself1
      all_goals sp_close
    have h2 := ih self1 t1 k' (by omega) hrec
    exact ⟨h2.1.trans hcase.1, h2.2.trans hcase.2⟩

theorem clear_partition_releases_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') :
    SpKept k k' := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  rename_i k1 hk1 a ha
  have h2 := clear_loop_sp _ _ _ _ rfl hk1
  exact ⟨h2.1, h2.2⟩

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (clear_partition_releases_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (clear_partition_releases_sp (by assumption)).2.trans ?_)

/-! ## The responses and the handlers of a slot -/

theorem end_partition_jobs_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (end_partition_jobs_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (end_partition_jobs_sp (by assumption)).2.trans ?_)

theorem stop_partition_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (stop_partition_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (stop_partition_sp (by assumption)).2.trans ?_)

theorem restart_partition_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (restart_partition_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (restart_partition_sp (by assumption)).2.trans ?_)

set_option maxHeartbeats 16000000 in
theorem apply_response_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.apply_response at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (apply_response_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (apply_response_sp (by assumption)).2.trans ?_)

theorem deadline_miss_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deadline_miss k cfg t i = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.deadline_miss at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (deadline_miss_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (deadline_miss_sp (by assumption)).2.trans ?_)

theorem mit_violation_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t partition offender : U8}
    (h : logic.kernel.Kernel.mit_violation k cfg t partition offender = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.mit_violation at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (mit_violation_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (mit_violation_sp (by assumption)).2.trans ?_)

set_option maxHeartbeats 16000000 in
theorem budget_event_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.budget_event at h
  peel h
  all_goals sp_close

macro_rules
  | `(tactic| sp_step) => `(tactic| refine (budget_event_sp (by assumption)).1.trans ?_)
macro_rules
  | `(tactic| sp_step) => `(tactic| refine (budget_event_sp (by assumption)).2.trans ?_)

set_option maxHeartbeats 8000000 in
theorem dispatch_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {pc : U32}
    (h : logic.kernel.Kernel.dispatch k cfg t pc = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.dispatch at h
  peel h
  all_goals sp_close

set_option maxHeartbeats 8000000 in
theorem job_return_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {ec : U32}
    (h : logic.kernel.Kernel.job_return k cfg ec = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.job_return at h
  peel h
  all_goals sp_close

set_option maxHeartbeats 8000000 in
theorem svc_lock_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_lock k cfg r = ok (b, k')) : SpKept k k' := by
  unfold logic.kernel.Kernel.svc_lock at h
  peel h
  all_goals sp_close

set_option maxHeartbeats 8000000 in
theorem svc_unlock_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (b, k')) : SpKept k k' := by
  unfold logic.kernel.Kernel.svc_unlock at h
  peel h
  all_goals sp_close

set_option maxHeartbeats 8000000 in
theorem settle_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : Option U8 × Bool}
    (h : logic.kernel.Kernel.settle k cfg = ok (r, k')) : SpKept k k' := by
  unfold logic.kernel.Kernel.settle at h
  peel h
  all_goals sp_close

set_option maxHeartbeats 8000000 in
theorem fault_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind : U8} {kn : Bool}
    (h : logic.kernel.Kernel.fault k cfg kind kn = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.fault at h
  peel h
  all_goals sp_close

set_option maxHeartbeats 8000000 in
theorem fault_of_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind t : U8}
    (h : logic.kernel.Kernel.fault_of k cfg kind t = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.fault_of at h
  peel h
  all_goals sp_close

set_option maxHeartbeats 8000000 in
theorem record_event_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind t : U8}
    (h : logic.kernel.Kernel.record_event k cfg kind t = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.record_event at h
  peel h
  all_goals sp_close

set_option maxHeartbeats 16000000 in
theorem periodic_release_sp {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.periodic_release k cfg t i = ok k') : SpKept k k' := by
  unfold logic.kernel.Kernel.periodic_release at h
  peel h
  all_goals sp_close

end P18
