import Sched.Timer
open Aeneas Aeneas.Std Result

/-! # Overrun detection whatever the ceiling, on the Kernel's code (2026-11-19)

Property 10 of the design: a Job whose consumed cycles exceed its Budget is
reported, counted while it is the executing Job, even while the
System_Ceiling is at its maximum; and time spent in other Jobs and in
Kernel work for other Tasks is never charged to it.

Verus proves it in three parts: the Budget accounts
(`rsk_logic::logic::budget`), the Kernel functions that charge, reset and
judge them (`Kernel::dispatch`, `job_return`, `budget_event` and the
responses, `only_reset_from`), and the level argument of the Hardware_Model
(`hw_model::lemma_kernel_level_never_masked`,
`lemma_kernel_level_taken_in_every_state`, `lemma_job_cannot_set_mask`).

Here the first two parts are stated of the functions as Aeneas extracts
them. The third is stated of the BASEPRI value the extracted
`basepri_for` computes, against the execution-priority rule of HW-02
written out in Lean (`execPrio`): as in Verus, that rule is the
Hardware_Model's, an assumption about the processor, not code. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

deriving instance Inhabited for rsk_kernel.rsk_logic.logic.TaskCfg

/-! ## The accounts -/

/-- The cycles charged to the current Job of Task `k`. -/
def used (b : logic.budget.Budget NT) (k : Nat) : Nat := (b.used.val[k]!).val
/-- Whether the Overrun of the current Job of Task `k` has been reported. -/
def rep (b : logic.budget.Budget NT) (k : Nat) : Bool := b.reported.val[k]!
/-- The Budget of Task `k`. -/
def budgetOf (cfg : logic.Config NT NR NP) (k : Nat) : Nat := (cfg.tasks.val[k]!).budget.val

theorem sat_val (x y : U32) :
    (core.num.U32.saturating_add x y).val = min 4294967295 (x.val + y.val) := by
  unfold core.num.U32.saturating_add UScalar.saturating_add
  simp only [UScalar.val]
  have hm : U32.max = 4294967295 := by scalar_tac
  have hx : x.val ≤ 4294967295 := by scalar_tac
  have hy : y.val ≤ 4294967295 := by scalar_tac
  simp only [UScalar.val] at hx hy
  simp
  rw [hm]
  omega

/-- `Budget::reset`: the account of `t` is empty and unreported, the others
as they were. -/
theorem reset_ok {b b' : logic.budget.Budget NT} {t : U8}
    (h : logic.budget.Budget.reset b t = ok b') :
    t.val < NT.val ∧ (∀ k, used b' k = if k = t.val then 0 else used b k) ∧
    (∀ k, rep b' k = if k = t.val then false else rep b k) := by
  simp only [logic.budget.Budget.reset, lift, bind_ok, bind_eq_ok] at h
  obtain ⟨a, ha, a1, ha1, h⟩ := h
  obtain ⟨hi, rfl⟩ := update_ok ha
  obtain ⟨_, rfl⟩ := update_ok ha1
  have h := ok_inj h; subst h
  rw [U8.cast_Usize_val_eq] at hi
  refine ⟨hi, fun k => ?_, fun k => ?_⟩
  · simp only [used]
    rw [get_set _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact hi), U8.cast_Usize_val_eq]
    split <;> rfl
  · simp only [rep]
    rw [get_set _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact hi), U8.cast_Usize_val_eq]

/-- `Budget::charge` (attribution, R18.1): the cycles are added to the
account of `t`, saturating, and to no other; no report changes. -/
theorem charge_ok {b b' : logic.budget.Budget NT} {t : U8} {c : U32}
    (h : logic.budget.Budget.charge b t c = ok b') :
    t.val < NT.val ∧
    (∀ k, used b' k = if k = t.val then min 4294967295 (used b t.val + c.val) else used b k) ∧
    b'.reported = b.reported := by
  simp only [logic.budget.Budget.charge, lift, bind_ok, bind_eq_ok] at h
  obtain ⟨v, hv, a, ha, h⟩ := h
  obtain ⟨hi, rfl⟩ := index_ok hv
  obtain ⟨_, rfl⟩ := update_ok ha
  have h := ok_inj h; subst h
  rw [U8.cast_Usize_val_eq] at hi
  refine ⟨hi, fun k => ?_, rfl⟩
  simp only [used]
  rw [get_set _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact hi), U8.cast_Usize_val_eq]
  split
  · rw [sat_val]
  · rfl

/-- A charge never lowers an account: an Overrun cannot wrap into
compliance. -/
theorem charge_mono {b b' : logic.budget.Budget NT} {t : U8} {c : U32}
    (h : logic.budget.Budget.charge b t c = ok b') (k : Nat) : used b k ≤ used b' k := by
  obtain ⟨_, hu, _⟩ := charge_ok h
  rw [hu]
  split
  · rename_i hk
    subst hk
    have : used b t.val ≤ 4294967295 := by
      simp only [used]; scalar_tac
    omega
  · exact Nat.le_refl _

/-- `Budget::report`: the Overrun of `t` is marked reported; no account
changes. -/
theorem report_ok {b b' : logic.budget.Budget NT} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.budget.Budget.report b cfg t = ok b') :
    t.val < NT.val ∧ b'.used = b.used ∧
    (∀ k, rep b' k = if k = t.val then true else rep b k) := by
  simp only [logic.budget.Budget.report, lift, bind_ok, bind_eq_ok] at h
  obtain ⟨a, ha, h⟩ := h
  obtain ⟨hi, rfl⟩ := update_ok ha
  have h := ok_inj h; subst h
  rw [U8.cast_Usize_val_eq] at hi
  refine ⟨hi, rfl, fun k => ?_⟩
  simp only [rep]
  rw [get_set _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact hi), U8.cast_Usize_val_eq]

/-- `Budget::overrun` (R18.2): the account is above the Budget. -/
theorem overrun_ok {b : logic.budget.Budget NT} {cfg : logic.Config NT NR NP} {t : U8} {o : Bool}
    (h : logic.budget.Budget.overrun b cfg t = ok o) :
    t.val < NT.val ∧ (o = true ↔ budgetOf cfg t.val < used b t.val) := by
  simp only [logic.budget.Budget.overrun, lift, bind_ok, bind_eq_ok] at h
  obtain ⟨u, hu, tc, htc, h⟩ := h
  obtain ⟨hi, rfl⟩ := index_ok hu
  obtain ⟨_, rfl⟩ := index_ok htc
  have h := ok_inj h; subst h
  rw [U8.cast_Usize_val_eq] at hi ⊢
  refine ⟨hi, ?_⟩
  simp only [budgetOf, used, decide_eq_true_eq]
  scalar_tac

/-- `Budget::remaining`: the Budget less the account, 0 from the Budget on. -/
theorem remaining_ok {b : logic.budget.Budget NT} {cfg : logic.Config NT NR NP} {t : U8} {r : U32}
    (h : logic.budget.Budget.remaining b cfg t = ok r) :
    t.val < NT.val ∧ r.val = budgetOf cfg t.val - used b t.val := by
  simp only [logic.budget.Budget.remaining, lift, bind_ok, bind_eq_ok] at h
  obtain ⟨tc, htc, u, hu, h⟩ := h
  obtain ⟨hi, rfl⟩ := index_ok htc
  obtain ⟨_, rfl⟩ := index_ok hu
  rw [U8.cast_Usize_val_eq] at hi h
  refine ⟨hi, ?_⟩
  simp only [budgetOf, used]
  split at h
  · have h := ok_inj h; subst h
    scalar_tac
  · have := UScalar.sub_equiv (cfg.tasks.val[t.val]!).budget (b.used.val[t.val]!)
    simp_all

/-! ## The Kernel's charge points -/

theorem push_event_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    k'.budget = k.budget := by
  unfold logic.kernel.Kernel.push_event at h
  frame_ok h

theorem arm_budget_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') : k'.budget = k.budget := by
  unfold logic.kernel.Kernel.arm_budget at h
  frame_ok h

theorem end_mit_window_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') : k'.budget = k.budget := by
  unfold logic.kernel.Kernel.end_mit_window at h
  frame_ok h

/-- `Kernel::dispatch` (R18.1): the preempted Job, if there is one, is
charged the cycles it ran; the account of the Task that starts is then
emptied. Nothing else touches an account. -/
theorem dispatch_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {pc : U32}
    (h : logic.kernel.Kernel.dispatch k cfg t pc = ok k') :
    ∃ b0 : logic.budget.Budget NT,
      ((k.sched.depth.val = 0 ∧ b0 = k.budget) ∨
        ∃ (i : Usize) (prev : U8), k.sched.depth - 1#usize = ok i ∧
          k.sched.stack.index_usize i = ok prev ∧
          logic.budget.Budget.charge k.budget prev pc = ok b0) ∧
      logic.budget.Budget.reset b0 t = ok k'.budget := by
  unfold logic.kernel.Kernel.dispatch at h
  peel h
  · rw [arm_budget_budget h]
    exact ⟨_, Or.inr ⟨_, _, ‹_›, ‹_›, ‹_›⟩, ‹_›⟩
  · rw [arm_budget_budget h]
    exact ⟨_, Or.inl ⟨by scalar_tac, rfl⟩, ‹_›⟩

/-- The same, account by account: the new Job starts from 0, unreported;
the preempted Job's account grows by the cycles it ran; every other
account is as it was. -/
theorem dispatch_charges_only_preempted {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {pc : U32}
    (h : logic.kernel.Kernel.dispatch k cfg t pc = ok k') :
    used k'.budget t.val = 0 ∧ rep k'.budget t.val = false ∧
    (∀ u, u ≠ t.val → (k.sched.depth.val = 0 ∨ u ≠ stk k.sched (k.sched.depth.val - 1)) →
      used k'.budget u = used k.budget u) ∧
    (0 < k.sched.depth.val → stk k.sched (k.sched.depth.val - 1) ≠ t.val →
      used k'.budget (stk k.sched (k.sched.depth.val - 1)) =
        min 4294967295 (used k.budget (stk k.sched (k.sched.depth.val - 1)) + pc.val)) := by
  obtain ⟨b0, hb0, hr⟩ := dispatch_budget h
  obtain ⟨_, hu, hrp⟩ := reset_ok hr
  have h1 : (1#usize).val = 1 := rfl
  refine ⟨by rw [hu]; simp, by rw [hrp]; simp, ?_, ?_⟩
  · intro u hut hutop
    rw [hu, if_neg hut]
    rcases hb0 with ⟨_, rfl⟩ | ⟨i, prev, hi, hprev, hc⟩
    · rfl
    · obtain ⟨hd, hiv⟩ := usize_sub_ok hi
      obtain ⟨_, rfl⟩ := index_ok hprev
      obtain ⟨_, hcu, _⟩ := charge_ok hc
      rw [h1] at hd hiv
      rw [hcu, if_neg]
      rw [hiv]
      rcases hutop with h0 | hne
      · omega
      · exact hne
  · intro hd hne
    rcases hb0 with ⟨h0, _⟩ | ⟨i, prev, hi, hprev, hc⟩
    · omega
    · obtain ⟨_, hiv⟩ := usize_sub_ok hi
      obtain ⟨_, rfl⟩ := index_ok hprev
      obtain ⟨_, hcu, _⟩ := charge_ok hc
      rw [h1] at hiv
      rw [hu, if_neg hne, hcu, hiv]
      simp only [stk, if_true]

/-- `Kernel::job_return` (R18.1): only the ended Job is charged, its last
segment. -/
theorem job_return_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {ec : U32}
    (h : logic.kernel.Kernel.job_return k cfg ec = ok k') :
    ∃ (i : Usize) (t : U8) (b : logic.budget.Budget NT), k.sched.depth - 1#usize = ok i ∧
      k.sched.stack.index_usize i = ok t ∧
      logic.budget.Budget.charge k.budget t ec = ok b ∧ k'.budget = b := by
  unfold logic.kernel.Kernel.job_return at h
  peel h
  all_goals
    refine ⟨_, _, _, ‹_›, ‹_›, ‹_›, ?_⟩
    try rw [arm_budget_budget h]
    try dsimp only
    try (have he := end_mit_window_budget (by assumption); dsimp only at he; rw [he])
    try dsimp only
    try (have hp := push_event_budget (by assumption); dsimp only at hp; rw [hp])

/-- The Budget event of the executing Job, found above its Budget and not
yet reported (R18.2): the Overrun is marked reported, the event is logged,
and the Partition's Overrun_Response is applied. -/
theorem budget_event_detects {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k')
    (hd : 0 < k.sched.depth.val) (htop : k.sched.stack.val[k.sched.depth.val - 1]! = t)
    (hr : rep k.budget t.val = false) (ho : budgetOf cfg t.val < used k.budget t.val) :
    ∃ (b2 : logic.budget.Budget NT) (k1 : logic.kernel.Kernel NT NR NP NL NS NE)
      (s : logic.time.Slots NS) (p r : U8) (prs : U64) (d : U32),
      logic.budget.Budget.report k.budget cfg t = ok b2 ∧
      logic.kernel.Kernel.push_event { k with slots := s, budget := b2 } cfg
        logic.log.EV_OVERRUN t p prs d = ok k1 ∧
      logic.kernel.Kernel.apply_response k1 cfg p r logic.log.EV_OVERRUN t = ok k' := by
  unfold logic.kernel.Kernel.budget_event at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨s, hs, h⟩ := h
  have h1 : (1#usize).val = 1 := rfl
  split at h
  · rename_i h0
    exfalso
    scalar_tac
  · simp only [bind_eq_ok] at h
    obtain ⟨i1, hi1, i2, hi2, h⟩ := h
    obtain ⟨_, hi1v⟩ := usize_sub_ok hi1
    obtain ⟨_, rfl⟩ := index_ok hi2
    rw [h1] at hi1v
    rw [hi1v, htop] at h
    simp only [bne_self_eq_false, Bool.false_eq_true, ite_false, bind_eq_ok] at h
    obtain ⟨b, hb, h⟩ := h
    obtain ⟨_, rfl⟩ := index_ok hb
    rw [U8.cast_Usize_val_eq] at h
    have hr' : k.budget.reported.val[t.val]! = false := hr
    simp only [hr', Bool.false_eq_true, ite_false, bind_eq_ok] at h
    obtain ⟨b1, hb1, h⟩ := h
    have ho' := (overrun_ok hb1).2.mpr ho
    subst ho'
    simp only [ite_true, bind_eq_ok] at h
    obtain ⟨b2, hb2, tc, htc, i6, hi6, i7, hi7, self1, hpe, pc, hpc, h⟩ := h
    exact ⟨b2, self1, s, _, _, _, _, hb2, hpe, h⟩

/-! ## Nothing but the charge points charges (R18.1, R18.6)

Verus's `only_reset_from` and `reported_kept`, in one relation: across a
response or a served slot every account is as it was, with a report kept,
or it has been emptied (the Task's Partition ended). -/

def Kept (b' b : logic.budget.Budget NT) : Prop :=
  ∀ u, (used b' u = used b u ∧ (rep b u = true → rep b' u = true)) ∨ used b' u = 0

theorem Kept.refl (b : logic.budget.Budget NT) : Kept b b := fun _ => Or.inl ⟨rfl, id⟩

theorem Kept.of_eq {b' b : logic.budget.Budget NT} (h : b' = b) : Kept b' b := by
  subst h; exact Kept.refl _

theorem Kept.trans {a b c : logic.budget.Budget NT} (h1 : Kept b a) (h2 : Kept c b) :
    Kept c a := by
  intro u
  rcases h2 u with ⟨e2, r2⟩ | z2
  · rcases h1 u with ⟨e1, r1⟩ | z1
    · exact Or.inl ⟨e2.trans e1, fun h => r2 (r1 h)⟩
    · exact Or.inr (e2.trans z1)
  · exact Or.inr z2

theorem kept_of_reset {b b' : logic.budget.Budget NT} {t : U8}
    (h : logic.budget.Budget.reset b t = ok b') : Kept b' b := by
  obtain ⟨_, hu, hr⟩ := reset_ok h
  intro u
  by_cases hut : u = t.val
  · right; rw [hu, if_pos hut]
  · left; rw [hu, hr, if_neg hut, if_neg hut]; exact ⟨rfl, id⟩

theorem kept_of_report {b b' : logic.budget.Budget NT} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.budget.Budget.report b cfg t = ok b') : Kept b' b := by
  obtain ⟨_, hu, hr⟩ := report_ok h
  intro u
  left
  refine ⟨by simp only [used, hu], fun hb => ?_⟩
  rw [hr]; split
  · rfl
  · exact hb

theorem observe_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64}
    (h : logic.kernel.Kernel.observe k cfg now = ok k') : k'.budget = k.budget := by
  unfold logic.kernel.Kernel.observe at h
  frame_ok h

theorem end_top_release_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top_release k cfg = ok k') : k'.budget = k.budget := by
  unfold logic.kernel.Kernel.end_top_release at h
  peel h
  all_goals
    try dsimp only
    try (have hp := push_event_budget (by assumption); rw [hp])

theorem end_top_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top k cfg = ok k') : k'.budget = k.budget := by
  unfold logic.kernel.Kernel.end_top at h
  peel h
  · rw [arm_budget_budget h]; exact end_top_release_budget ‹_›
  · exact end_top_release_budget ‹_›

theorem kill_at_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.kill_at k cfg i = ok k') : k'.budget = k.budget := by
  unfold logic.kernel.Kernel.kill_at at h
  frame_ok h

theorem kill_loop_budget {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) res,
      k.sched.depth.val - i.val = n →
      logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i = ok res →
      ∃ k2 : logic.kernel.Kernel NT NR NP NL NS NE, k2.budget = k.budget ∧
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
    exact ⟨k, rfl, h.symm⟩
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
    have hcase : self1.budget = k.budget ∧ self1.sched.depth = k.sched.depth := by
      peel hself1
      · exact ⟨kill_at_budget (by assumption), (kill_at_r hiv (by assumption)).2⟩
      · exact ⟨rfl, rfl⟩
      · exact ⟨rfl, rfl⟩
    obtain ⟨hb, hd⟩ := hcase
    obtain ⟨k2, h2, e⟩ := ih self1 i2 res (by rw [hd]; omega) hrec
    exact ⟨k2, h2.trans hb, e⟩

theorem kill_partition_jobs_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') :
    k'.budget = k.budget := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨res, hres, h⟩ := h
  obtain ⟨k2, h2, e⟩ := kill_loop_budget _ _ _ res rfl hres
  subst e
  simp only [Aeneas.Std.uncurry] at h
  have h := ok_inj h
  subst h
  exact h2

/-- `clear_partition_releases`' loop: the only place an account is emptied
outside a dispatch (R14.5: the Partition's Jobs are over). -/
theorem clear_loop_kept {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NT.val - t.val = n →
      logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k' →
      Kept k'.budget k.budget := by
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
    exact Kept.refl _
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
    have hcase : Kept self1.budget k.budget := by
      peel hself1
      all_goals first
        | exact Kept.refl _
        | exact kept_of_reset (by assumption)
    exact hcase.trans (ih self1 t1 k' (by omega) hrec)

theorem clear_partition_releases_kept {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') :
    Kept k'.budget k.budget := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  rename_i k1 hk1 a ha
  dsimp only
  exact clear_loop_kept _ _ _ _ rfl hk1

theorem enter_safe_state_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.enter_safe_state k cfg = ok k') : k'.budget = k.budget := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  peel h
  · rfl
  · rename_i k1 hs hp s hl
    dsimp only
    exact push_event_budget hp

theorem end_partition_jobs_kept {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') :
    Kept k'.budget k.budget := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals
    have hc := clear_partition_releases_kept h
    have hk := kill_partition_jobs_budget (by assumption)
  · have he := end_top_budget (k := k) (by assumption)
    rw [hk, he] at hc; exact hc
  · rw [hk] at hc; exact hc
  · rw [hk] at hc; exact hc

theorem stop_partition_kept {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') :
    Kept k'.budget k.budget := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  rename_i k1 hp k2 he a ha
  dsimp only
  have := end_partition_jobs_kept he
  rw [push_event_budget hp] at this
  exact this

theorem restart_partition_kept {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') :
    Kept k'.budget k.budget := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  rename_i k1 hp k2 he a ha a1 ha1
  dsimp only
  have := end_partition_jobs_kept he
  rw [push_event_budget hp] at this
  exact this

set_option maxHeartbeats 2000000 in
/-- A Health_Monitor response charges nothing. -/
theorem apply_response_kept {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') :
    Kept k'.budget k.budget := by
  unfold logic.kernel.Kernel.apply_response at h
  peel h
  all_goals first
    | exact Kept.refl _
    | exact restart_partition_kept h
    | exact stop_partition_kept h
    | exact Kept.of_eq (enter_safe_state_budget h)
    | skip
  · rename_i hd i hi top htop htt hn hnt k1 hp k2 he tc htc hk
    rw [end_mit_window_budget h, end_top_budget he, push_event_budget hp]
    exact Kept.refl _
  · rename_i hd i hi top htop htt hn hnt k1 hp k2 he tc htc hk
    rw [end_top_budget he, push_event_budget hp]
    exact Kept.refl _

theorem mit_violation_kept {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t partition offender : U8}
    (h : logic.kernel.Kernel.mit_violation k cfg t partition offender = ok k') :
    Kept k'.budget k.budget := by
  unfold logic.kernel.Kernel.mit_violation at h
  peel h
  · rename_i prs hprs k1 hp c hc c2 back hb pc hpc hgt pc1 hpc1
    have := apply_response_kept h
    dsimp only at this
    rw [push_event_budget hp] at this
    exact this
  · rename_i prs hprs k1 hp c hc c2 back hb pc hpc hgt
    dsimp only
    rw [push_event_budget hp]
    exact Kept.refl _

theorem deadline_miss_kept {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deadline_miss k cfg t i = ok k') :
    Kept k'.budget k.budget := by
  unfold logic.kernel.Kernel.deadline_miss at h
  peel h
  have hr := apply_response_kept h
  have hp := push_event_budget (by assumption)
  dsimp only at hp
  rw [hp] at hr
  exact hr

theorem periodic_release_kept {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.periodic_release k cfg t i = ok k') :
    Kept k'.budget k.budget := by
  unfold logic.kernel.Kernel.periodic_release at h
  peel h
  all_goals first
    | exact Kept.refl _
    | (dsimp only; exact Kept.refl _)
    | (dsimp only; exact mit_violation_kept (by assumption))
    | exact mit_violation_kept (by assumption)
    | (try dsimp only; rw [push_event_budget (by assumption)]; exact Kept.refl _)

theorem deferred_release_kept {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deferred_release k cfg t i = ok k') :
    Kept k'.budget k.budget := by
  unfold logic.kernel.Kernel.deferred_release at h
  peel h
  all_goals first
    | exact Kept.refl _
    | (dsimp only; exact Kept.refl _)
    | (try dsimp only; rw [push_event_budget (by assumption)]; exact Kept.refl _)

/-- The Budget event judges and responds; it charges nothing (R18.1), and a
report it finds stays (R18.6). -/
theorem budget_event_kept {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k') :
    Kept k'.budget k.budget := by
  unfold logic.kernel.Kernel.budget_event at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨s, hs, h⟩ := h
  split at h
  · have h := ok_inj h; subst h; exact Kept.refl _
  · simp only [bind_eq_ok] at h
    obtain ⟨i1, hi1, i2, hi2, h⟩ := h
    split at h
    · have h := ok_inj h; subst h; exact Kept.refl _
    · simp only [bind_eq_ok] at h
      obtain ⟨b, hb, h⟩ := h
      split at h
      · have h := ok_inj h; subst h; exact Kept.refl _
      · simp only [bind_eq_ok] at h
        obtain ⟨b1, hb1, h⟩ := h
        split at h
        · simp only [bind_eq_ok] at h
          obtain ⟨b2, hb2, tc, htc, i6, hi6, i7, hi7, self1, hpe, pc, hpc, h⟩ := h
          have hr := apply_response_kept h
          have hp := push_event_budget hpe
          dsimp only at hp
          rw [hp] at hr
          exact (kept_of_report hb2).trans hr
        · have ha := arm_budget_budget h
          dsimp only at ha
          rw [ha]
          exact Kept.refl _

set_option maxHeartbeats 2000000 in
/-- A served slot, whatever its kind, charges nothing. -/
theorem serve_slot_kept_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k') : Kept k'.budget k.budget := by
  unfold logic.kernel.Kernel.serve_slot at h
  peel h
  all_goals first
    | exact Kept.refl _
    | exact periodic_release_kept h
    | exact deferred_release_kept h
    | exact deadline_miss_kept h
    | exact budget_event_kept h
    | (have he := enter_safe_state_budget h
       have hp := push_event_budget (by assumption)
       dsimp only at hp
       rw [he, hp]; exact Kept.refl _)

theorem ServedChain.kept {cfg : logic.Config NT NR NP}
    {a b : logic.kernel.Kernel NT NR NP NL NS NE} (h : ServedChain cfg a b) :
    Kept b.budget a.budget := by
  induction h with
  | done => exact Kept.refl _
  | serve k k1 k' i hdue hs _ ih => exact (serve_slot_kept_budget hs).trans ih

/-- `Kernel::timer_event` (R18.1): the Job on top, if there is one, is
charged the cycles it ran since the last charge point; the events served
after that charge nothing. -/
theorem timer_event_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64} {rc : U32}
    (h : logic.kernel.Kernel.timer_event k cfg now rc = ok k') :
    ∃ b : logic.budget.Budget NT,
      (b = k.budget ∨ ∃ (i : Usize) (t : U8) (k0 : logic.kernel.Kernel NT NR NP NL NS NE),
        k0.sched = k.sched ∧ k0.sched.depth - 1#usize = ok i ∧
        k0.sched.stack.index_usize i = ok t ∧
        logic.budget.Budget.charge k.budget t rc = ok b) ∧
      Kept k'.budget b := by
  unfold logic.kernel.Kernel.timer_event at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨k0, hobs, h⟩ := h
  have hsch := observe_sched hobs
  have hbud := observe_budget hobs
  split at h
  · have h := ok_inj h; subst h
    exact ⟨k.budget, Or.inl rfl, Kept.of_eq hbud⟩
  · simp only [bind_eq_ok] at h
    obtain ⟨b, hb, k2, hl0, hl1⟩ := h
    have hch := ((timer_loop0_served _ _ _ _ rfl hl0).trans
      (timer_loop1_served _ _ _ _ rfl hl1)).kept
    dsimp only at hch
    refine ⟨b, ?_, hch⟩
    split at hb
    · simp only [bind_eq_ok] at hb
      obtain ⟨i, hi, t, ht, hc⟩ := hb
      rw [hbud] at hc
      exact Or.inr ⟨i, t, k0, hsch, hi, ht, hc⟩
    · have hb := ok_inj hb
      rw [hbud] at hb
      exact Or.inl hb.symm

theorem u64_sub_ok {a b c : U64} (h : a - b = ok c) : b.val ≤ a.val ∧ c.val = a.val - b.val := by
  have := UScalar.sub_equiv a b
  simp_all

/-- `Kernel::arm_budget` (R18.2): the Budget event of the Task is armed for
the instant its remaining Budget runs out, in ticks rounded up, and no
later than the end of the operating duration. With an account at or above
the Budget that instant is the present one. -/
theorem arm_budget_at {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') :
    (k'.slots.slots.val[(cfg.slot_budget.val[t.val]!).val]!).armed = true ∧
    (k'.slots.slots.val[(cfg.slot_budget.val[t.val]!).val]!).kind = logic.time.SLOT_BUDGET ∧
    (k'.slots.slots.val[(cfg.slot_budget.val[t.val]!).val]!).task = t ∧
    (k'.slots.slots.val[(cfg.slot_budget.val[t.val]!).val]!).«at».val =
      min (k.now.val + (budgetOf cfg t.val - used k.budget t.val + 3) / 4)
        cfg.operating_duration.val := by
  unfold logic.kernel.Kernel.arm_budget at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨r, hr, i1, hi1, i2, hi2, ticks, hticks, i3, hi3, at', hat, i5, hi5, s, hs, h⟩ := h
  have hk := ok_inj h
  subst hk
  obtain ⟨_, hrv⟩ := remaining_ok hr
  obtain ⟨_, rfl⟩ := index_ok hi5
  unfold logic.time.Slots.arm at hs
  simp only [bind_eq_ok] at hs
  obtain ⟨a, ha, hs⟩ := hs
  have hs := ok_inj hs
  subst hs
  obtain ⟨hlt, rfl⟩ := update_ok ha
  have hc4 : logic.kernel.CYCLES_PER_TICK = 4#u64 := by unfold logic.kernel.CYCLES_PER_TICK; rfl
  rw [hc4] at hi1 hticks
  have e1 := u64_add_ok hi1
  obtain ⟨_, e2⟩ := u64_sub_ok hi2
  have e3 : ticks.val = i2.val / 4 := by
    obtain ⟨z, hz, hzv⟩ := UScalar.div_spec i2 (y := 4#u64) (by decide)
    rw [hz] at hticks
    have := ok_inj hticks; subst this
    exact hzv
  have e4 := u64_add_ok hi3
  have hcast : (UScalar.cast .U64 r).val = r.val := by scalar_tac
  have h4 : (4#u64).val = 4 := rfl
  have h1 : (1#u64).val = 1 := rfl
  rw [hcast, h4] at e1
  rw [h1] at e2
  dsimp only
  rw [get_set _ _ _ _ hlt]
  simp only [U8.cast_Usize_val_eq, if_true]
  refine ⟨trivial, trivial, trivial, ?_⟩
  split at hat
  · rename_i hgt
    have hat := ok_inj hat; subst hat
    have : cfg.operating_duration.val < i3.val := by scalar_tac
    omega
  · rename_i hgt
    have hat := ok_inj hat; subst hat
    have : ¬ cfg.operating_duration.val < i3.val := by scalar_tac
    omega

/-! ## The level part (DD-02) -/

/-- The NVIC priority encoding of a level (HW-03): 0 for the Kernel level,
a multiple of 32 falling with the level for the Task levels. -/
def encodeLevel (l : Nat) : Nat := if l = 0 then 0 else (8 - l) * 32

theorem encode_level_ok {l v : U8} (h : arch.bound.encode_level l = ok v) :
    v.val = encodeLevel l.val := by
  unfold arch.bound.encode_level at h
  unfold encodeLevel
  split at h
  · rename_i h0
    have h := ok_inj h; subst h
    have : l.val = 0 := by scalar_tac
    simp [this]
  · rename_i h0
    simp only [bind_eq_ok] at h
    obtain ⟨i, hi, h⟩ := h
    have hs := UScalar.sub_equiv 8#u8 l
    have hm := UScalar.mul_equiv i 32#u8
    have hl0 : l.val ≠ 0 := by scalar_tac
    simp only [hl0, ite_false]
    have hmm : UScalar.mul i 32#u8 = ok v := h
    simp_all
    omega

/-- The BASEPRI value the Kernel writes: the encoding of the greater of the
executing Job's level and the System_Ceiling. -/
theorem basepri_for_ok {top c v : U8} (h : arch.bound.basepri_for top c = ok v) :
    v.val = encodeLevel (max top.val c.val) := by
  unfold arch.bound.basepri_for arch.bound.basepri_for_ceiling at h
  simp only [bind_eq_ok] at h
  obtain ⟨m, hm, h⟩ := h
  have hmv : m.val = max top.val c.val := by
    split at hm
    · have hm := ok_inj hm; subst hm; scalar_tac
    · have hm := ok_inj hm; subst hm; scalar_tac
  rw [← hmv]
  split at h
  · rename_i h0
    have h := ok_inj h; subst h
    have : m.val = 0 := by scalar_tac
    simp [encodeLevel, this]
  · exact encode_level_ok h

/-- HW-02: the execution priority is the most urgent (least) of the active
exceptions' group priorities (`active`: their minimum, `none` when no
exception is active), of BASEPRI when it is not 0, and of 0 when PRIMASK is
set; 256 when nothing boosts it. -/
def execPrio (active : Option Nat) (basepri : Nat) (primask : Bool) : Nat :=
  min (active.getD 256) (min (if basepri = 0 then 256 else basepri) (if primask then 0 else 256))

/-- HW-05: a pending exception of group priority `g` is taken only when it
is more urgent than the execution priority. -/
def nvicTakes (g e : Nat) : Prop := g < e

/-- HW-04 with HW-08: a write of PRIMASK or FAULTMASK takes effect only
when executed privileged. -/
def maskWrite (privileged current set : Bool) : Bool := if privileged then set else current

/-- `lemma_job_cannot_set_mask`: a Job (Thread mode, unprivileged) cannot
set PRIMASK or FAULTMASK. -/
theorem job_cannot_set_mask (current set : Bool) :
    maskWrite (false || !true) current set = current := rfl

/-- `lemma_kernel_level_never_masked`: a pending exception at the Kernel
level is taken whatever BASEPRI holds and whatever Task-level exceptions
are active; only PRIMASK or an active Kernel-level exception delays it. -/
theorem kernel_level_never_masked (active : Option Nat) (basepri : Nat)
    (ha : ∀ a, active = some a → encodeLevel 0 < a) :
    nvicTakes (encodeLevel 0) (execPrio active basepri false) := by
  unfold nvicTakes execPrio encodeLevel
  cases active with
  | none => simp; split <;> omega
  | some a =>
    have := ha a rfl
    simp [encodeLevel] at this
    simp
    refine ⟨this, ?_⟩
    split <;> omega

/-- `lemma_kernel_level_taken_in_every_state`: with the BASEPRI value the
Kernel computes for any executing Job and any System_Ceiling, its maximum
included, and no exception active, a pending Kernel-level exception (the
time base that serves the Budget events among them) is taken. -/
theorem kernel_level_taken {top c v : U8} (_h : arch.bound.basepri_for top c = ok v) :
    nvicTakes (encodeLevel 0) (execPrio none v.val false) :=
  kernel_level_never_masked none v.val (by intro a ha; cases ha)

/-- The BASEPRI value never names the Kernel level's priority: it is 0 (no
masking) or the encoding of a Task level, at least 32. -/
theorem basepri_for_task_level {top c v : U8} (h : arch.bound.basepri_for top c = ok v)
    (hl : max top.val c.val ≤ 7) : v.val = 0 ∨ 32 ≤ v.val := by
  rw [basepri_for_ok h]
  unfold encodeLevel
  split
  · left; rfl
  · right; omega

end P18
