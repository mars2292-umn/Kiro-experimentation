import Sched.Slots
import Sched.LoopInv
open Aeneas Aeneas.Std Result

/-! # The System_Ceiling stays within the Priority levels (2026-11-20)

`Bound::basepri_now` encodes the greater of the top Job's Priority and the
System_Ceiling as a BASEPRI value (`encode_level`, `8 - level`), so that it
returns only when the System_Ceiling is at most 8. The ceiling is written
in three places of the Kernel's code: `Sched::lock` (a Resource's Ceiling),
`Sched::unlock` and `Sched::end_job` (`recompute_ceiling`, the greatest
Ceiling of the Resources still held, or `IDLE_CEILING`). With every
Resource's Ceiling within the levels (`RC`), every Kernel function that
returns leaves the System_Ceiling within them if it was (`CLK`).

The statements are of the "if it returns" form; the chain tactic `cl_close`
follows a function's calls, as `bo_close` does in `Sched/Slots.lean`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- Every Resource's Ceiling is within the Priority levels. -/
def RC (cfg : logic.Config NT NR NP) : Prop :=
  ∀ (i : Nat) (rc : logic.ResourceCfg), cfg.resources.val[i]? = some rc → rc.ceiling.val ≤ 8

/-- If the System_Ceiling of `s` is within the levels, that of `s'` is. -/
@[reducible] def CLs (s s' : logic.sched.Sched NT NR NL) : Prop :=
  s.ceiling.val ≤ 8 → s'.ceiling.val ≤ 8

@[reducible] def CLK (k k' : logic.kernel.Kernel NT NR NP NL NS NE) : Prop :=
  CLs k.sched k'.sched

theorem cl_trans {s sm s' : logic.sched.Sched NT NR NL} (g1 : CLs s sm) (g2 : CLs sm s') :
    CLs s s' := fun h => g2 (g1 h)

theorem cl_of_eq {s sm s' : logic.sched.Sched NT NR NL} (e : s' = sm) (g : CLs s sm) :
    CLs s s' := by rw [e]; exact g

/-- One step of the chain: the call that returned the state the goal ends at. -/
syntax "cl_step" : tactic
macro_rules | `(tactic| cl_step) => `(tactic| exact fun h => h)

/-- Closes `CLK k k'` after `peel`. -/
syntax "cl_close" : tactic
macro_rules
  | `(tactic| cl_close) => `(tactic| (show CLs _ _; repeat (first | cl_step | dsimp only)))

/-! ## The scheduler state's functions -/

theorem recompute_le {s : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {n : Usize}
    {c : U8} (hrc : RC cfg) (h : logic.sched.Sched.recompute_ceiling s cfg n = ok c) :
    c.val ≤ 8 := by
  unfold logic.sched.Sched.recompute_ceiling logic.sched.Sched.recompute_ceiling_loop at h
  refine loop_inv _ (fun x => x.1.val ≤ 8) (fun r => r.val ≤ 8) ?_ ?_ _ c ?_ h
  · rintro ⟨c1, e⟩ ⟨c2, e2⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.sched.Sched.recompute_ceiling_loop.body at hb
    peel hb
    all_goals first
      | exact hinv
      | exact hrc _ _ (index_ok' (by assumption)).2
  · rintro ⟨c1, e⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.sched.Sched.recompute_ceiling_loop.body at hb
    peel hb
    exact hinv
  · show logic.IDLE_CEILING.val ≤ 8
    rw [idle_ceiling]
    decide

theorem sstart_cl {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.sched.Sched.start s cfg t = ok s') : CLs s s' := by
  unfold logic.sched.Sched.start at h
  peel h
  exact fun h => h

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (sstart_cl (by assumption)))

theorem scomplete_cl {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} 
    (h : logic.sched.Sched.complete s cfg = ok s') : CLs s s' := by
  unfold logic.sched.Sched.complete at h
  peel h
  exact fun h => h

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (scomplete_cl (by assumption)))

theorem sclear_release_cl {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.sched.Sched.clear_release s cfg t = ok s') : CLs s s' := by
  unfold logic.sched.Sched.clear_release at h
  peel h
  exact fun h => h

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (sclear_release_cl (by assumption)))

theorem skill_cl {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.sched.Sched.kill s cfg i = ok s') : CLs s s' := by
  unfold logic.sched.Sched.kill at h
  peel h
  exact fun h => h

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (skill_cl (by assumption)))

theorem sdefer_cl {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.sched.Sched.defer s cfg t = ok s') : CLs s s' := by
  unfold logic.sched.Sched.defer at h
  peel h
  exact fun h => h

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (sdefer_cl (by assumption)))

theorem smake_pending_cl {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.sched.Sched.make_pending s cfg t = ok s') : CLs s s' := by
  unfold logic.sched.Sched.make_pending at h
  peel h
  exact fun h => h

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (smake_pending_cl (by assumption)))

theorem send_job_cl {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {n : Usize}
    (hrc : RC cfg) (h : logic.sched.Sched.end_job s cfg = ok (n, s')) : CLs s s' := by
  unfold logic.sched.Sched.end_job at h
  peel h
  intro _
  have hle := recompute_le hrc (by assumption)
  exact scomplete_cl (by assumption) hle

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (send_job_cl (by assumption) (by assumption)))

theorem sfinish_top_cl {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {n : Usize}
    (hrc : RC cfg) (h : logic.sched.Sched.finish_top s cfg = ok (n, s')) : CLs s s' := by
  unfold logic.sched.Sched.finish_top at h
  exact send_job_cl hrc h

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (sfinish_top_cl (by assumption) (by assumption)))

theorem slock_cl {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    {b : Bool} (hrc : RC cfg) (h : logic.sched.Sched.lock s cfg r = ok (b, s')) : CLs s s' := by
  unfold logic.sched.Sched.lock at h
  peel h
  all_goals first
    | exact fun h => h
    | (intro _; exact hrc _ _ (index_ok' (by assumption)).2)

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (slock_cl (by assumption) (by assumption)))

theorem sunlock_cl {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    {b : Bool} (hrc : RC cfg) (h : logic.sched.Sched.unlock s cfg r = ok (b, s')) : CLs s s' := by
  unfold logic.sched.Sched.unlock at h
  peel h
  all_goals first
    | exact fun h => h
    | (intro _; exact recompute_le hrc (by assumption))

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (sunlock_cl (by assumption) (by assumption)))

theorem ess_loop_cl {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {t : Usize}
    (h : logic.kernel.Kernel.enter_safe_state_loop s cfg t = ok s') : CLs s s' := by
  unfold logic.kernel.Kernel.enter_safe_state_loop at h
  refine loop_inv _ (fun x => CLs s x.1) (fun r => CLs s r) ?_ ?_ _ s' (fun h => h) h
  · rintro ⟨s1, t1⟩ ⟨s2, t2⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.enter_safe_state_loop.body at hb
    peel hb
    exact cl_trans hinv (sclear_release_cl (by assumption))
  · rintro ⟨s1, t1⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.enter_safe_state_loop.body at hb
    peel hb
    exact hinv

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (ess_loop_cl (by assumption)))

/-! ## The Kernel's functions, in the order of the file -/

theorem push_event_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {kind : U8} {task : U8} {partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') : CLK k k' := by
  show _ → _
  rw [push_event_sched h]
  exact fun h => h

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (push_event_cl (by assumption)))

theorem record_event_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {kind : U8} {t : U8}
    (h : logic.kernel.Kernel.record_event k cfg kind t = ok k') : CLK k k' := by
  show _ → _
  rw [record_event_sched h]
  exact fun h => h

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (record_event_cl (by assumption)))

theorem observe_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {now : U64}
    (h : logic.kernel.Kernel.observe k cfg now = ok k') : CLK k k' := by
  show _ → _
  rw [observe_sched h]
  exact fun h => h

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (observe_cl (by assumption)))

theorem arm_budget_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') : CLK k k' := by
  show _ → _
  rw [arm_budget_sched h]
  exact fun h => h

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (arm_budget_cl (by assumption)))

set_option maxHeartbeats 16000000 in
theorem dispatch_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {t : U8} {preempted_cycles : U32}
    (hrc : RC cfg) (h : logic.kernel.Kernel.dispatch k cfg t preempted_cycles = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.dispatch at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (dispatch_cl (by assumption) (by assumption)))

theorem end_mit_window_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') : CLK k k' := by
  show _ → _
  rw [end_mit_window_sched h]
  exact fun h => h

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (end_mit_window_cl (by assumption)))

set_option maxHeartbeats 16000000 in
theorem job_return_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {elapsed_cycles : U32}
    (hrc : RC cfg) (h : logic.kernel.Kernel.job_return k cfg elapsed_cycles = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.job_return at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (job_return_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem svc_lock_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (hrc : RC cfg) (h : logic.kernel.Kernel.svc_lock k cfg r = ok (b, k')) : CLK k k' := by
  unfold logic.kernel.Kernel.svc_lock at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (svc_lock_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem svc_unlock_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (hrc : RC cfg) (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (b, k')) : CLK k k' := by
  unfold logic.kernel.Kernel.svc_unlock at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (svc_unlock_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem enter_safe_state_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} 
    (hrc : RC cfg) (h : logic.kernel.Kernel.enter_safe_state k cfg  = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (enter_safe_state_cl (by assumption) (by assumption)))

theorem clear_partition_releases_loop_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {partition : U8} {t : Usize}
    (hrc : RC cfg) (h : logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.clear_partition_releases_loop at h
  refine loop_inv _ (fun x => CLK k x.1) (fun r => CLK k r) ?_ ?_ _ k' (fun h => h) h
  · rintro ⟨k1, t1⟩ ⟨k2, t2⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.clear_partition_releases_loop.body at hb
    peel hb
    all_goals (refine cl_trans hinv ?_; cl_close)
  · rintro ⟨k1, t1⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.clear_partition_releases_loop.body at hb
    peel hb
    exact hinv

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (clear_partition_releases_loop_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem clear_partition_releases_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {partition : U8}
    (hrc : RC cfg) (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (clear_partition_releases_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem kill_at_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {i : Usize}
    (hrc : RC cfg) (h : logic.kernel.Kernel.kill_at k cfg i = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.kill_at at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (kill_at_cl (by assumption) (by assumption)))

theorem kill_loop_cl {k : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {partition : U8} {i : Usize}
    {res : _} (hrc : RC cfg)
    (h : logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i = ok res) :
    k.sched.ceiling.val ≤ 8 → res.2.2.2.2.2.1.val ≤ 8 := by
  unfold logic.kernel.Kernel.kill_partition_jobs_loop at h
  refine loop_inv _ (fun x => CLK k x.1) (fun r => k.sched.ceiling.val ≤ 8 → r.2.2.2.2.2.1.val ≤ 8)
    ?_ ?_ _ res (fun h => h) h
  · rintro ⟨k1, t1⟩ ⟨k2, t2⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.kill_partition_jobs_loop.body at hb
    peel hb
    all_goals (refine cl_trans hinv ?_; cl_close)
  · rintro ⟨k1, t1⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.kill_partition_jobs_loop.body at hb
    peel hb
    exact hinv

theorem kill_partition_jobs_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {partition : U8}
    (hrc : RC cfg) (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') :
    CLK k k' := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨res, hres, h⟩ := h
  have hp := kill_loop_cl hrc hres
  obtain ⟨a, a1, a2, i, a3, i1, a4, a5, i2, t, s, b, l, a6, a7, a8, a9, b1, b2, i3, g⟩ := res
  have h := ok_inj h
  subst h
  exact hp

macro_rules
  | `(tactic| cl_step) =>
    `(tactic| refine cl_trans ?_ (kill_partition_jobs_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem end_top_release_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} 
    (hrc : RC cfg) (h : logic.kernel.Kernel.end_top_release k cfg  = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.end_top_release at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (end_top_release_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem end_top_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} 
    (hrc : RC cfg) (h : logic.kernel.Kernel.end_top k cfg  = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.end_top at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (end_top_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem end_partition_jobs_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {partition : U8}
    (hrc : RC cfg) (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (end_partition_jobs_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem stop_partition_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {partition : U8} {cause : U8}
    (hrc : RC cfg) (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (stop_partition_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem restart_partition_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {partition : U8} {cause : U8}
    (hrc : RC cfg) (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (restart_partition_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem apply_response_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {partition : U8} {response : U8} {cause : U8} {t : U8}
    (hrc : RC cfg) (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.apply_response at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (apply_response_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem deadline_miss_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (hrc : RC cfg) (h : logic.kernel.Kernel.deadline_miss k cfg t i = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.deadline_miss at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (deadline_miss_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem mit_violation_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {t : U8} {partition : U8} {offender : U8}
    (hrc : RC cfg) (h : logic.kernel.Kernel.mit_violation k cfg t partition offender = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.mit_violation at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (mit_violation_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem intake_release_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {t : U8} {source_partition : U8} {now : U64} {offender : U8}
    (hrc : RC cfg) (h : logic.kernel.Kernel.intake_release k cfg t source_partition now offender = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.intake_release at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (intake_release_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem intake_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {t : U8} {source_partition : U8} {now : U64} {offender : U8}
    (hrc : RC cfg) (h : logic.kernel.Kernel.intake k cfg t source_partition now offender = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.intake at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (intake_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem budget_event_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (hrc : RC cfg) (h : logic.kernel.Kernel.budget_event k cfg t i = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.budget_event at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (budget_event_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem deferred_release_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (hrc : RC cfg) (h : logic.kernel.Kernel.deferred_release k cfg t i = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.deferred_release at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (deferred_release_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem periodic_release_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (hrc : RC cfg) (h : logic.kernel.Kernel.periodic_release k cfg t i = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.periodic_release at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (periodic_release_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem serve_slot_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {i : Usize}
    (hrc : RC cfg) (h : logic.kernel.Kernel.serve_slot k cfg i = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.serve_slot at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (serve_slot_cl (by assumption) (by assumption)))

theorem timer_event_loop0_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {i : Usize}
    (hrc : RC cfg) (h : logic.kernel.Kernel.timer_event_loop0 k cfg i = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.timer_event_loop0 at h
  refine loop_inv _ (fun x => CLK k x.1) (fun r => CLK k r) ?_ ?_ _ k' (fun h => h) h
  · rintro ⟨k1, t1⟩ ⟨k2, t2⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.timer_event_loop0.body at hb
    peel hb
    all_goals (refine cl_trans hinv ?_; cl_close)
  · rintro ⟨k1, t1⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.timer_event_loop0.body at hb
    peel hb
    exact hinv

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (timer_event_loop0_cl (by assumption) (by assumption)))

theorem timer_event_loop1_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {i : Usize}
    (hrc : RC cfg) (h : logic.kernel.Kernel.timer_event_loop1 k cfg i = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.timer_event_loop1 at h
  refine loop_inv _ (fun x => CLK k x.1) (fun r => CLK k r) ?_ ?_ _ k' (fun h => h) h
  · rintro ⟨k1, t1⟩ ⟨k2, t2⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.timer_event_loop1.body at hb
    peel hb
    all_goals (refine cl_trans hinv ?_; cl_close)
  · rintro ⟨k1, t1⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.timer_event_loop1.body at hb
    peel hb
    exact hinv

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (timer_event_loop1_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem timer_event_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {now : U64} {running_cycles : U32}
    (hrc : RC cfg) (h : logic.kernel.Kernel.timer_event k cfg now running_cycles = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.timer_event at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (timer_event_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem fault_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {kind : U8} {kernel : Bool}
    (hrc : RC cfg) (h : logic.kernel.Kernel.fault k cfg kind kernel = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.fault at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (fault_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem fault_of_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {kind : U8} {t : U8}
    (hrc : RC cfg) (h : logic.kernel.Kernel.fault_of k cfg kind t = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.fault_of at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (fault_of_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem boot_check_failed_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {failed : U32}
    (hrc : RC cfg) (h : logic.kernel.Kernel.boot_check_failed k cfg failed = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.boot_check_failed at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (boot_check_failed_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem ack_reinit_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {partition : U8}
    (hrc : RC cfg) (h : logic.kernel.Kernel.ack_reinit k cfg partition = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.ack_reinit at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (ack_reinit_cl (by assumption) (by assumption)))

set_option maxHeartbeats 16000000 in
theorem ack_stop_dma_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {partition : U8}
    (hrc : RC cfg) (h : logic.kernel.Kernel.ack_stop_dma k cfg partition = ok k') : CLK k k' := by
  unfold logic.kernel.Kernel.ack_stop_dma at h
  peel h
  all_goals cl_close

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (ack_stop_dma_cl (by assumption) (by assumption)))

theorem settle_cl {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP} {r : Option U8 × Bool}
    (h : logic.kernel.Kernel.settle k cfg = ok (r, k')) : CLK k k' := by
  show _ → _
  rw [settle_sched h]
  exact fun h => h

macro_rules | `(tactic| cl_step) => `(tactic| refine cl_trans ?_ (settle_cl (by assumption)))

end P18
