import Sched.Contain
import Sched.Overrun
open Aeneas Aeneas.Std Result

/-! # What a response leaves alone, continued (2026-11-25)

`Sched/Contain.lean` has the scheduler's clauses and the flags of
`Kernel::others_unchanged`. This file has two more of its clauses, on the
Kernel's functions as extracted:

- `csteps_ceil`: **a response does not raise the System_Ceiling**: its
  steps leave it the ceiling of the lock stack they leave, which has only
  lost entries.
- `BU`, `apply_response_bu`: **a response leaves the Budget account of
  every Task outside its Partition as it was**, the used cycles and the
  Overrun report (only `Budget::reset`, for the Tasks of a Partition being
  ended, writes an account under a response).

Not here (Verus only): that the timer slots of the other Partitions'
Tasks are left. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! ## The System_Ceiling -/

theorem cstep_ceil {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    {s s' : logic.sched.Sched NT NR NL} (h : CStep cfg inP s s')
    (hc : IsCeil cfg s s.lock_len.val s.ceiling.val) :
    IsCeil cfg s' s'.lock_len.val s'.ceiling.val ∧ s'.ceiling.val ≤ s.ceiling.val := by
  cases h with
  | kend r hin h =>
    obtain ⟨_, _, _, _, _, _, _, hlr, _, hlen, _, _, hci⟩ := finish_top_views h
    refine ⟨IsCeil.same hlr hci, ?_⟩
    obtain ⟨_, h2⟩ := hci
    rcases h2 with h0 | ⟨p, hp, hcp⟩
    · omega
    · rw [hcp]; exact hc.1 p (by omega)
  | kkill p hin h =>
    obtain ⟨_, _, _, _, _, hl, _, hn, hce⟩ := kill_views h
    rw [hn, hce]
    exact ⟨IsCeil.same hl hc, Nat.le_refl _⟩
  | kclear t hin h =>
    obtain ⟨_, _, _, _, _, hl, _, hn, hce⟩ := clear_release_views h
    rw [hn, hce]
    exact ⟨IsCeil.same hl hc, Nat.le_refl _⟩

/-- **A response does not raise the System_Ceiling** (`others_unchanged`,
its last clause), from a state whose ceiling is its lock stack's (every
state of a run, `kreach_ceiling`). -/
theorem csteps_ceil {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    {s s' : logic.sched.Sched NT NR NL} (h : CSteps cfg inP s s')
    (hc : IsCeil cfg s s.lock_len.val s.ceiling.val) :
    IsCeil cfg s' s'.lock_len.val s'.ceiling.val ∧ s'.ceiling.val ≤ s.ceiling.val := by
  induction h with
  | refl => exact ⟨hc, Nat.le_refl _⟩
  | tail _ hs ih =>
    obtain ⟨h1, h2⟩ := cstep_ceil hs ih.1
    exact ⟨h1, Nat.le_trans h2 ih.2⟩

/-! ## The Budget accounts -/

/-- From `b` to `b'` the account of every Task outside `inP` is as it was:
the cycles used and the Overrun report. -/
def BU (inP : Nat → Prop) (b b' : logic.budget.Budget NT) : Prop :=
  ∀ u, ¬ inP u → used b' u = used b u ∧ rep b' u = rep b u

theorem BU.refl (inP : Nat → Prop) (b : logic.budget.Budget NT) : BU inP b b :=
  fun _ _ => ⟨rfl, rfl⟩

theorem BU.trans {inP : Nat → Prop} {a b c : logic.budget.Budget NT} (h1 : BU inP a b)
    (h2 : BU inP b c) : BU inP a c := fun u hu =>
  ⟨(h2 u hu).1.trans (h1 u hu).1, (h2 u hu).2.trans (h1 u hu).2⟩

variable [hNT : Fact (NT.val ≤ 256)]

include hNT in
/-- The account of a Task of the Partition emptied. -/
theorem reset_bu_c {b b' : logic.budget.Budget NT} {cfg : logic.Config NT NR NP} {t : Usize}
    {tc : logic.TaskCfg} {partition : U8}
    (hr : logic.budget.Budget.reset b (UScalar.cast .U8 t) = ok b') (ht : t.val < NT.val)
    (htc : cfg.tasks.index_usize t = ok tc) (hp : tc.partition = partition) :
    BU (fun u => ptOf cfg u = partition.val) b b' := by
  have hN := hNT.out
  have e : (UScalar.cast .U8 t : U8).val = t.val := by
    simp only [UScalar.cast_val_eq, UScalarTy.numBits]
    omega
  obtain ⟨_, hu, hre⟩ := reset_ok hr
  intro u hin
  have hne : ¬ u = (UScalar.cast .U8 t : U8).val := by
    intro eq
    apply hin
    show ptOf cfg u = partition.val
    rw [eq, e, pt_of_index htc, hp]
  rw [hu, hre, if_neg hne, if_neg hne]
  exact ⟨rfl, rfl⟩

omit hNT

syntax "bu_step" : tactic
macro_rules | `(tactic| bu_step) => `(tactic| exact BU.refl _ _)

syntax "bu_close" : tactic
macro_rules
  | `(tactic| bu_close) => `(tactic| (repeat (first | bu_step | dsimp only)))



theorem push_event_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    BU inP k.budget k'.budget := by
  unfold logic.kernel.Kernel.push_event at h
  peel h
  all_goals bu_close

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (push_event_bu (by assumption)))

theorem arm_budget_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {t : U8}
    (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') : BU inP k.budget k'.budget := by
  unfold logic.kernel.Kernel.arm_budget at h
  peel h
  all_goals bu_close

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (arm_budget_bu (by assumption)))

theorem end_mit_window_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') : BU inP k.budget k'.budget := by
  unfold logic.kernel.Kernel.end_mit_window at h
  peel h
  all_goals bu_close

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (end_mit_window_bu (by assumption)))

theorem enter_safe_state_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    (h : logic.kernel.Kernel.enter_safe_state k cfg = ok k') : BU inP k.budget k'.budget := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  peel h
  all_goals bu_close

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (enter_safe_state_bu (by assumption)))

theorem kill_at_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {i : Usize}
    (h : logic.kernel.Kernel.kill_at k cfg i = ok k') : BU inP k.budget k'.budget := by
  unfold logic.kernel.Kernel.kill_at at h
  peel h
  all_goals bu_close

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (kill_at_bu (by assumption)))

theorem end_top_release_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    (h : logic.kernel.Kernel.end_top_release k cfg = ok k') : BU inP k.budget k'.budget := by
  unfold logic.kernel.Kernel.end_top_release at h
  peel h
  all_goals bu_close

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (end_top_release_bu (by assumption)))

theorem end_top_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    (h : logic.kernel.Kernel.end_top k cfg = ok k') : BU inP k.budget k'.budget := by
  unfold logic.kernel.Kernel.end_top at h
  peel h
  all_goals bu_close

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (end_top_bu (by assumption)))



/-- `kill_partition_jobs`' loop. -/
theorem kill_loop_bu {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) res,
      k.sched.depth.val - i.val = n →
      logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i = ok res →
      ∃ k2 : logic.kernel.Kernel NT NR NP NL NS NE, BU inP k.budget k2.budget ∧
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
    exact ⟨k, BU.refl _ _, h.symm⟩
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
    have hcase : self1.sched.depth = k.sched.depth ∧ BU inP k.budget self1.budget := by
      peel hself1
      · exact ⟨(kill_at_r hiv (by assumption)).2, kill_at_bu (by assumption)⟩
      · exact ⟨rfl, BU.refl _ _⟩
      · exact ⟨rfl, BU.refl _ _⟩
    obtain ⟨hd, htk⟩ := hcase
    obtain ⟨k2, h2, e⟩ := ih self1 i2 res (by rw [hd]; omega) hrec
    exact ⟨k2, BU.trans htk h2, e⟩

theorem kill_partition_jobs_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {inP : Nat → Prop} {partition : U8}
    (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') : BU inP k.budget k'.budget := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨res, hres, h⟩ := h
  obtain ⟨k2, h2, e⟩ := kill_loop_bu _ _ _ res rfl hres
  subst e
  simp only [Aeneas.Std.uncurry] at h
  have h := ok_inj h
  subst h
  exact h2

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (kill_partition_jobs_bu (by assumption)))

include hNT in
/-- `clear_partition_releases`' loop. -/
theorem clear_loop_bu {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NT.val - t.val = n →
      logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k' →
      BU (fun u => ptOf cfg u = partition.val) k.budget k'.budget := by
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
    exact BU.refl _ _
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
    have hcase : BU (fun u => ptOf cfg u = partition.val) k.budget self1.budget := by
      have htv : t.val < NT.val := by scalar_tac
      peel hself1
      all_goals first
        | (dsimp only
           exact reset_bu_c (by assumption) htv htc (by assumption))
        | exact BU.refl _ _
    have h2 := ih self1 t1 k' (by omega) hrec
    exact BU.trans hcase h2

include hNT in
theorem clear_partition_releases_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') :
    BU (fun u => ptOf cfg u = partition.val) k.budget k'.budget := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  rename_i k1 hk1 a ha
  exact clear_loop_bu (cfg := cfg) (partition := partition) _ k 0#usize k1 rfl hk1

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (clear_partition_releases_bu (by assumption)))



include hNT in
theorem end_partition_jobs_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') : BU (fun u => ptOf cfg u = partition.val) k.budget k'.budget := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals bu_close

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (end_partition_jobs_bu (by assumption)))

include hNT in
theorem stop_partition_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') : BU (fun u => ptOf cfg u = partition.val) k.budget k'.budget := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  all_goals bu_close

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (stop_partition_bu (by assumption)))

include hNT in
theorem restart_partition_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') : BU (fun u => ptOf cfg u = partition.val) k.budget k'.budget := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  all_goals bu_close

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (restart_partition_bu (by assumption)))

set_option maxHeartbeats 16000000 in
include hNT in
theorem apply_response_bu {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') : BU (fun u => ptOf cfg u = partition.val) k.budget k'.budget := by
  unfold logic.kernel.Kernel.apply_response at h
  peel h
  all_goals bu_close

macro_rules
  | `(tactic| bu_step) => `(tactic| refine BU.trans ?_ (apply_response_bu (by assumption)))


end P18
