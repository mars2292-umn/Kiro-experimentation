import Sched.SpIntake
open Aeneas Aeneas.Std Result

/-! # A source's mask is lifted only once its window has passed (2026-11-25)

`Timing` keeps, per Sporadic_Task, whether its Release_Source is masked
(`masked`) and the instant its MIT window ends (`mask_until`). Verus
carries the consequence as a clause of the Kernel's invariant
(`intakes_linked`, Property 9, R19.1); here it is shown of the Kernel's
functions as Aeneas extracts them, each statement of the form "if the
function returns":

- `KM a b`, for the pairs (instant, timing state) before and after: the
  instant is left, the windows' ends are left, and a source that was masked
  is still masked unless the instant has reached the end of its window.
- Every function of the run other than `Kernel::observe` and
  `Kernel::intake_release` satisfies it (`*_km`). The two places that lift
  a mask are `Timing::try_unmask` (from `end_mit_window`, at a Job's end)
  and the unmask event of `serve_slot`, each behind the comparison of the
  Kernel's instant with `mask_until`.

The proofs of the functions that only pass the timing state on are those
of `Sched/SpFrame.lean` with the relation in place of the equalities
(generated from them). `Sched/Intakes.lean` has `intake_release` and the
run. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- Whether the source of Task `u` is masked. -/
def mk (tm : logic.time.Timing NT) (u : Nat) : Bool := tm.masked.val[u]!
/-- The instant the MIT window of Task `u` ends, in ticks. -/
def mu (tm : logic.time.Timing NT) (u : Nat) : Nat := (tm.mask_until.val[u]!).val

/-- The Kernel's instant with its timing state. -/
@[reducible] def kv (k : logic.kernel.Kernel NT NR NP NL NS NE) : U64 × logic.time.Timing NT :=
  (k.now, k.timing)

/-- From `a` to `b` the instant and the windows' ends are left, and a
source masked in `a` is masked in `b` unless the instant has reached the
end of its window. -/
def KM (a b : U64 × logic.time.Timing NT) : Prop :=
  b.1 = a.1 ∧ b.2.mask_until = a.2.mask_until ∧
  ∀ u, mk a.2 u = true → mk b.2 u = true ∨ mu a.2 u ≤ a.1.val

theorem KM.refl (a : U64 × logic.time.Timing NT) : KM a a :=
  ⟨rfl, rfl, fun _ h => Or.inl h⟩

theorem KM.trans {a b c : U64 × logic.time.Timing NT} (h1 : KM a b) (h2 : KM b c) : KM a c := by
  obtain ⟨n1, m1, r1⟩ := h1
  obtain ⟨n2, m2, r2⟩ := h2
  refine ⟨n2.trans n1, m2.trans m1, fun u hu => ?_⟩
  rcases r1 u hu with h | h
  · rcases r2 u h with h' | h'
    · exact Or.inl h'
    · right
      have e : mu b.2 u = mu a.2 u := by unfold mu; rw [m1]
      rw [e, n1] at h'
      exact h'
  · exact Or.inr h

/-- The timing state with the masks and the windows' ends left. -/
theorem KM.same {n : U64} {tm tm' : logic.time.Timing NT}
    (hm : tm'.masked = tm.masked) (hu : tm'.mask_until = tm.mask_until) : KM (n, tm) (n, tm') :=
  ⟨rfl, hu, fun u h => Or.inl (by unfold mk at h ⊢; rw [hm]; exact h)⟩

/-- The mask of Task `t` lifted with the instant at or past the end of its
window. -/
theorem KM.lift {n : U64} {tm : logic.time.Timing NT} {i : Usize} (hi : i.val < NT.val)
    (hn : (tm.mask_until.val[i.val]!).val ≤ n.val) :
    KM (n, tm) (n, { tm with masked := tm.masked.set i false }) := by
  refine ⟨rfl, rfl, fun u h => ?_⟩
  by_cases hu : u = i.val
  · right
    subst hu
    exact hn
  · left
    unfold mk at h ⊢
    dsimp only
    rw [get_set _ _ _ _ hi]
    simp only [hu, ite_false]
    exact h

syntax "km_step" : tactic
macro_rules | `(tactic| km_step) => `(tactic| exact KM.refl _)

syntax "km_close" : tactic
macro_rules
  | `(tactic| km_close) => `(tactic| (repeat (first | km_step | dsimp only [kv])))

/-! ## The functions of `Timing` -/

theorem advance_periodic_km {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {o : Option U64} {n : U64}
    (h : logic.time.Timing.advance_periodic tm cfg t = ok (o, tm')) : KM (n, tm) (n, tm') := by
  unfold logic.time.Timing.advance_periodic at h
  peel h
  all_goals exact KM.same rfl rfl

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (advance_periodic_km (by assumption)))

theorem periodic_due_km {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    {«at» : U64} {n : U64}
    (h : logic.time.Timing.periodic_due tm cfg t «at» = ok tm') : KM (n, tm) (n, tm') := by
  unfold logic.time.Timing.periodic_due at h
  peel h
  all_goals exact KM.same rfl rfl

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (periodic_due_km (by assumption)))

theorem job_started_km {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    {n : U64}
    (h : logic.time.Timing.job_started tm cfg t = ok tm') : KM (n, tm) (n, tm') := by
  unfold logic.time.Timing.job_started at h
  peel h
  all_goals exact KM.same rfl rfl

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (job_started_km (by assumption)))

theorem deferred_due_clamped_km {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {«at» : U64} {n : U64}
    (h : logic.time.Timing.deferred_due_clamped tm cfg t «at» = ok tm') : KM (n, tm) (n, tm') := by
  unfold logic.time.Timing.deferred_due_clamped at h
  peel h
  all_goals exact KM.same rfl rfl

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (deferred_due_clamped_km (by assumption)))

/-- `Timing::try_unmask` lifts the mask only with the instant it is given
at or past the end of the window. -/
theorem try_unmask_km {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {now : U64} {je b : Bool}
    (h : logic.time.Timing.try_unmask tm cfg t now je = ok (b, tm')) :
    KM (now, tm) (now, tm') := by
  unfold logic.time.Timing.try_unmask at h
  simp only [lift, bind_ok, bind_eq_ok] at h
  obtain ⟨i1, hi1, h⟩ := h
  obtain ⟨hlt, rfl⟩ := index_ok hi1
  split at h
  · split at h
    · rename_i hge
      simp only [bind_eq_ok] at h
      obtain ⟨a, ha, h⟩ := h
      have h := ok_inj h
      rw [Prod.mk.injEq] at h
      obtain ⟨_, rfl⟩ := h
      obtain ⟨_, rfl⟩ := update_ok ha
      exact KM.lift hlt (by scalar_tac)
    · have h := ok_inj h
      rw [Prod.mk.injEq] at h
      obtain ⟨_, rfl⟩ := h
      exact KM.refl _
  · have h := ok_inj h
    rw [Prod.mk.injEq] at h
    obtain ⟨_, rfl⟩ := h
    exact KM.refl _

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (try_unmask_km (by assumption)))

/-- `Timing::unmask`, called with the instant at or past the end of the
window (the unmask event of `serve_slot`). -/
theorem unmask_km {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    {n m : U64}
    (h : logic.time.Timing.unmask tm cfg t = ok tm')
    (hm : tm.mask_until.index_usize (UScalar.cast .Usize t) = ok m) (hn : n >= m) :
    KM (n, tm) (n, tm') := by
  unfold logic.time.Timing.unmask at h
  simp only [lift, bind_ok, bind_eq_ok] at h
  obtain ⟨a, ha, h⟩ := h
  have h := ok_inj h
  subst h
  obtain ⟨hlt, rfl⟩ := index_ok hm
  obtain ⟨_, rfl⟩ := update_ok ha
  exact KM.lift hlt (by scalar_tac)

/-! ## The Kernel functions without a loop -/

theorem push_event_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.push_event at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (push_event_km (by assumption)))

theorem arm_budget_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.arm_budget at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (arm_budget_km (by assumption)))

theorem end_mit_window_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.end_mit_window at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (end_mit_window_km (by assumption)))

theorem enter_safe_state_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.enter_safe_state k cfg = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (enter_safe_state_km (by assumption)))

theorem kill_at_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.kill_at k cfg i = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.kill_at at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (kill_at_km (by assumption)))

theorem end_top_release_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top_release k cfg = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.end_top_release at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (end_top_release_km (by assumption)))

theorem end_top_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top k cfg = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.end_top at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (end_top_km (by assumption)))

/-! ## The loops of a Partition's stop and restart -/

/-- `kill_partition_jobs`' loop. -/
theorem kill_loop_km {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) res,
      k.sched.depth.val - i.val = n →
      logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i = ok res →
      ∃ k2 : logic.kernel.Kernel NT NR NP NL NS NE, KM (kv k) (kv k2) ∧
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
    exact ⟨k, KM.refl _, h.symm⟩
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
    have hcase : self1.sched.depth = k.sched.depth ∧ KM (kv k) (kv self1) := by
      peel hself1
      · exact ⟨(kill_at_r hiv (by assumption)).2, kill_at_km (by assumption)⟩
      · exact ⟨rfl, KM.refl _⟩
      · exact ⟨rfl, KM.refl _⟩
    obtain ⟨hd, htk⟩ := hcase
    obtain ⟨k2, h2, e⟩ := ih self1 i2 res (by rw [hd]; omega) hrec
    exact ⟨k2, KM.trans htk h2, e⟩

theorem kill_partition_jobs_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨res, hres, h⟩ := h
  obtain ⟨k2, h2, e⟩ := kill_loop_km _ _ _ res rfl hres
  subst e
  simp only [Aeneas.Std.uncurry] at h
  have h := ok_inj h
  subst h
  exact h2

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (kill_partition_jobs_km (by assumption)))

/-- `clear_partition_releases`' loop. -/
theorem clear_loop_km {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NT.val - t.val = n →
      logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k' →
      KM (kv k) (kv k') := by
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
    exact KM.refl _
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
    have hcase : KM (kv k) (kv self1) := by
      peel hself1
      all_goals km_close
    have h2 := ih self1 t1 k' (by omega) hrec
    exact KM.trans hcase h2

theorem clear_partition_releases_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') :
    KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  rename_i k1 hk1 a ha
  have h2 := clear_loop_km _ _ _ _ rfl hk1
  exact h2

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (clear_partition_releases_km (by assumption)))

/-! ## The responses and the handlers of a slot -/

theorem end_partition_jobs_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (end_partition_jobs_km (by assumption)))

theorem stop_partition_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (stop_partition_km (by assumption)))

theorem restart_partition_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (restart_partition_km (by assumption)))

set_option maxHeartbeats 16000000 in
theorem apply_response_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.apply_response at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (apply_response_km (by assumption)))

theorem deadline_miss_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deadline_miss k cfg t i = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.deadline_miss at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (deadline_miss_km (by assumption)))

theorem mit_violation_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t partition offender : U8}
    (h : logic.kernel.Kernel.mit_violation k cfg t partition offender = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.mit_violation at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (mit_violation_km (by assumption)))

set_option maxHeartbeats 16000000 in
theorem budget_event_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.budget_event at h
  peel h
  all_goals km_close

macro_rules
  | `(tactic| km_step) => `(tactic| refine KM.trans ?_ (budget_event_km (by assumption)))

set_option maxHeartbeats 8000000 in
theorem dispatch_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {pc : U32}
    (h : logic.kernel.Kernel.dispatch k cfg t pc = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.dispatch at h
  peel h
  all_goals km_close

set_option maxHeartbeats 8000000 in
theorem job_return_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {ec : U32}
    (h : logic.kernel.Kernel.job_return k cfg ec = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.job_return at h
  peel h
  all_goals km_close

set_option maxHeartbeats 8000000 in
theorem svc_lock_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_lock k cfg r = ok (b, k')) : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.svc_lock at h
  peel h
  all_goals km_close

set_option maxHeartbeats 8000000 in
theorem svc_unlock_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (b, k')) : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.svc_unlock at h
  peel h
  all_goals km_close

set_option maxHeartbeats 8000000 in
theorem settle_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : Option U8 × Bool}
    (h : logic.kernel.Kernel.settle k cfg = ok (r, k')) : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.settle at h
  peel h
  all_goals km_close

set_option maxHeartbeats 8000000 in
theorem fault_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind : U8} {kn : Bool}
    (h : logic.kernel.Kernel.fault k cfg kind kn = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.fault at h
  peel h
  all_goals km_close

set_option maxHeartbeats 8000000 in
theorem fault_of_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind t : U8}
    (h : logic.kernel.Kernel.fault_of k cfg kind t = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.fault_of at h
  peel h
  all_goals km_close

set_option maxHeartbeats 8000000 in
theorem record_event_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind t : U8}
    (h : logic.kernel.Kernel.record_event k cfg kind t = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.record_event at h
  peel h
  all_goals km_close

set_option maxHeartbeats 16000000 in
theorem periodic_release_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.periodic_release k cfg t i = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.periodic_release at h
  peel h
  all_goals km_close


/-! ## The functions of the run that `Sched/SpFrame.lean` has in other forms -/

set_option maxHeartbeats 4000000 in
theorem deferred_release_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deferred_release k cfg t i = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.deferred_release at h
  peel h
  all_goals km_close

theorem ack_reinit_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {p : U8}
    (h : logic.kernel.Kernel.ack_reinit k cfg p = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.ack_reinit at h
  peel h
  all_goals km_close

theorem ack_stop_dma_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {p : U8}
    (h : logic.kernel.Kernel.ack_stop_dma k cfg p = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.ack_stop_dma at h
  peel h
  all_goals km_close

set_option maxHeartbeats 16000000 in
/-- `Kernel::serve_slot`: the unmask event lifts the mask only with the
Kernel's instant at or past `mask_until`. -/
theorem serve_slot_km {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k') : KM (kv k) (kv k') := by
  unfold logic.kernel.Kernel.serve_slot at h
  peel h
  all_goals first
    | (km_close; done)
    | exact periodic_release_km h
    | exact deferred_release_km h
    | exact deadline_miss_km h
    | exact budget_event_km h
    | (try dsimp only [kv]
       exact unmask_km (by assumption) (by assumption) (by assumption))

end P18
