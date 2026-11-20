import Sched.Total
import Sched.Views
import Sched.Calls
open Aeneas Aeneas.Std Result

/-! # The Health_Monitor's responses return (2026-11-20)

Property 21, continued from `Sched/Total.lean`: the responses
(`apply_response` and what it calls: the end of the Job on top, the stop
and the restart of a Partition, the safe state), and the Kernel entries
that are an event and its response (`fault`, `fault_of`, `deadline_miss`,
`budget_event`, `mit_violation`).

`KI` is the part of the Kernel's invariant these functions need in order
to return, and each is shown to keep it, so that they compose: the
scheduler state within its bounds (`SInv`), the log well formed, the
instant within the range of the time base. `CI` is what they need of the
configuration. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- What a Kernel function needs of the state to return. -/
structure KI (k : logic.kernel.Kernel NT NR NP NL NS NE) : Prop where
  s : SInv k.sched
  l : Lwf k.log
  n : k.now.val ≤ 4611686018427387904

/-- What a Kernel function needs of the configuration to return: the slots
it names exist, the Task count fits a Task identifier, and every Task's
Partition is one of the configuration's. -/
structure CI (cfg : logic.Config NT NR NP) (NS : Usize) : Prop where
  ok : CfgOk cfg NS
  nt : NT.val ≤ 256
  part : ∀ (i : Nat) (h : i < cfg.tasks.val.length), (cfg.tasks.val[i]).partition.val < NP.val

theorem getE {α} [Inhabited α] {n : Usize} {v : Array α n} {i : Nat} {x : α}
    (h : i < v.val.length) (hx : x = v.val[i]) : v.val[i]! = x := by
  rw [hx]; exact getElem!_pos v.val i h

theorem cast8 (t : Usize) (h : t.val < 256) : (UScalar.cast .U8 t).val = t.val := by
  scalar_tac

/-! ## The PR sets of the events -/

theorem prs_release_total : logic.kernel.PRS_RELEASE ⦃ _ => True ⦄ := by
  unfold logic.kernel.PRS_RELEASE
  step*

theorem prs_budget_total : logic.kernel.PRS_BUDGET ⦃ _ => True ⦄ := by
  unfold logic.kernel.PRS_BUDGET
  step*

theorem prs_panic_total : logic.kernel.PRS_PANIC ⦃ _ => True ⦄ := by
  unfold logic.kernel.PRS_PANIC
  step*

theorem prs_privilege_total : logic.kernel.PRS_PRIVILEGE ⦃ _ => True ⦄ := by
  unfold logic.kernel.PRS_PRIVILEGE
  step*

theorem prs_ownership_total : logic.kernel.PRS_OWNERSHIP ⦃ _ => True ⦄ := by
  unfold logic.kernel.PRS_OWNERSHIP
  step*

theorem fault_prs_total (kind : U8) : logic.kernel.fault_prs kind ⦃ _ => True ⦄ := by
  unfold logic.kernel.fault_prs
  repeat' first
    | exact prs_panic_total
    | exact prs_privilege_total
    | exact prs_ownership_total
    | split
  refine WP.spec_bind prs_panic_total (fun i _ => ?_)
  refine WP.spec_bind prs_ownership_total (fun i1 _ => ?_)
  exact (WP.spec_ok _).mpr trivial

/-! ## The log and the safe state -/

/-- `push_event` keeps `KI` and everything but the log. -/
theorem push_ki (k : logic.kernel.Kernel NT NR NP NL NS NE) (cfg : logic.Config NT NR NP)
    (kind task partition : U8) (prs : U64) (detail : U32) (hk : KI k) :
    logic.kernel.Kernel.push_event k cfg kind task partition prs detail ⦃ k' =>
      KI k' ∧ k'.sched = k.sched ∧ k'.now = k.now ∧ k'.slots = k.slots ∧
      k'.timing = k.timing ∧ k'.budget = k.budget ⦄ :=
  WP.spec_mono (push_event_total k cfg kind task partition prs detail hk.l)
    (fun k1 h => ⟨⟨by rw [h.2.1]; exact hk.s, h.1, by rw [h.2.2.1]; exact hk.n⟩, h.2⟩)

theorem clear_release_spec (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (t : U8) (ht : t.val < NT.val) (hs : SInv s) :
    logic.sched.Sched.clear_release s cfg t ⦃ s' => SInv s' ∧ s'.depth = s.depth ⦄ :=
  spec_of_total (clear_release_total s cfg t ht) (fun r h => by
    obtain ⟨_, rfl⟩ := clear_release_ok h
    exact ⟨hs, rfl⟩)

theorem safe_loop_total (cfg : logic.Config NT NR NP) (hNT : NT.val ≤ 256)
    (s : logic.sched.Sched NT NR NL) (t : Usize) (hs : SInv s) (ht : t.val ≤ NT.val) :
    logic.kernel.Kernel.enter_safe_state_loop s cfg t ⦃ s' => SInv s' ⦄ := by
  unfold logic.kernel.Kernel.enter_safe_state_loop
  apply loop.spec_decr_nat (fun (x : logic.sched.Sched NT NR NL × Usize) => NT.val - x.2.val)
    (fun x => SInv x.1 ∧ x.2.val ≤ NT.val)
  · intro x hx
    obtain ⟨s1, t1⟩ := x
    obtain ⟨h1, h2⟩ := hx
    try dsimp only at h1
    try dsimp only at h2
    try dsimp only
    unfold logic.kernel.Kernel.enter_safe_state_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : t1.val < NT.val := by scalar_tac
      simp only [lift, bind_ok]
      refine WP.spec_bind (clear_release_spec s1 cfg _ (by rw [cast8 _ (by omega)]; exact hlt') h1)
        (fun s2 hs2 => ?_)
      step as ⟨ t2, ht2 ⟩
      exact ⟨hs2.1, by scalar_tac, by scalar_tac⟩
    · exact (WP.spec_ok _).mpr h1
  · exact ⟨hs, ht⟩

/-- `enter_safe_state`: the event, once, and every Release dropped. -/
theorem enter_safe_state_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (hk : KI k) (hNT : NT.val ≤ 256) :
    logic.kernel.Kernel.enter_safe_state k cfg ⦃ k' => KI k' ⦄ := by
  unfold logic.kernel.Kernel.enter_safe_state
  refine WP.spec_bind (Pₘ := fun self1 : logic.kernel.Kernel NT NR NP NL NS NE => KI self1) ?_
    (fun self1 h1 => ?_)
  · split
    · exact (WP.spec_ok _).mpr hk
    · exact WP.spec_mono (push_ki k cfg _ _ _ _ _ hk) (fun _ h => h.1)
  · refine WP.spec_bind (safe_loop_total cfg hNT _ _ h1.s (by scalar_tac)) (fun s hs => ?_)
    exact (WP.spec_ok _).mpr ⟨hs, h1.l, h1.n⟩

/-! ## The Jobs of a Partition -/

theorem kill_at_spec (k : logic.kernel.Kernel NT NR NP NL NS NE) (cfg : logic.Config NT NR NP)
    (i : Usize) (hi : i.val < k.sched.depth.val) (hk : KI k) :
    logic.kernel.Kernel.kill_at k cfg i ⦃ k' =>
      KI k' ∧ k'.sched.depth = k.sched.depth ⦄ := by
  unfold logic.kernel.Kernel.kill_at
  have hlen : i.val < k.sched.stack.val.length := by
    have := k.sched.stack.property; have := hk.s.1; scalar_tac
  step as ⟨ t, ht ⟩
  refine WP.spec_bind (spec_of_total (kill_total k.sched cfg i hi hk.s) (fun r h => kill_ok h))
    (fun s hs => ?_)
  obtain ⟨_, _, rfl⟩ := hs
  exact (WP.spec_ok _).mpr ⟨⟨hk.s, hk.l, hk.n⟩, rfl⟩

theorem kill_loop_total (cfg : logic.Config NT NR NP) (partition : U8)
    (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) (hk : KI k) :
    logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i ⦃ r =>
      ∃ k2 : logic.kernel.Kernel NT NR NP NL NS NE, KI k2 ∧
        r = (k2.sched.release, k2.sched.job, k2.sched.stack, k2.sched.depth,
          k2.sched.start_ceiling, k2.sched.ceiling, k2.sched.lock_res, k2.sched.lock_depth,
          k2.sched.lock_len, k2.timing, k2.slots, k2.budget, k2.log, k2.stopped,
          k2.mit_violations, k2.reinit_partition, k2.stop_dma, k2.safe_state, k2.started,
          k2.now, k2.history) ⦄ := by
  unfold logic.kernel.Kernel.kill_partition_jobs_loop
  apply loop.spec_decr_nat
    (fun (x : logic.kernel.Kernel NT NR NP NL NS NE × Usize) => x.1.sched.depth.val - x.2.val)
    (fun x => KI x.1)
  · intro x hx
    obtain ⟨k1, i1⟩ := x
    try dsimp only at hx
    try dsimp only
    unfold logic.kernel.Kernel.kill_partition_jobs_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : i1.val < k1.sched.depth.val := by scalar_tac
      have hlen : i1.val < k1.sched.stack.val.length := by
        have := k1.sched.stack.property; have := hx.s.1; scalar_tac
      step as ⟨ u, hu ⟩
      have hu' := getE hlen hu
      have hst := hx.s.2.1 i1.val hlt'
      simp only [stk] at hst
      rw [hu'] at hst
      simp only [lift, bind_ok]
      step as ⟨ tc, htc ⟩
      refine WP.spec_bind
        (Pₘ := fun self1 : logic.kernel.Kernel NT NR NP NL NS NE =>
          KI self1 ∧ self1.sched.depth = k1.sched.depth) ?_ (fun self1 h1 => ?_)
      · split
        · have hlenj : u.val < k1.sched.job.val.length := by
            have := k1.sched.job.property; scalar_tac
          step as ⟨ jb, hjb ⟩
          split
          · exact kill_at_spec k1 cfg i1 hlt' hx
          · exact (WP.spec_ok _).mpr ⟨hx, rfl⟩
        · exact (WP.spec_ok _).mpr ⟨hx, rfl⟩
      · step as ⟨ i2, hi2 ⟩
        exact ⟨h1.1, by rw [h1.2]; scalar_tac⟩
    · exact (WP.spec_ok _).mpr ⟨k1, hx, rfl⟩
  · exact hk

/-- `kill_partition_jobs`: the preempted Jobs of the Partition are marked. -/
theorem kill_partition_jobs_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (partition : U8) (hk : KI k) :
    logic.kernel.Kernel.kill_partition_jobs k cfg partition ⦃ k' => KI k' ⦄ := by
  unfold logic.kernel.Kernel.kill_partition_jobs
  refine WP.spec_bind (kill_loop_total cfg partition k 0#usize hk) (fun r hr => ?_)
  obtain ⟨k2, hk2, rfl⟩ := hr
  exact (WP.spec_ok _).mpr hk2

/-- The slot bound of a Task's deadline slot, from what `step` leaves. -/
theorem dl_slot {cfg : logic.Config NT NR NP} (hc : CfgOk cfg NS) {t : Nat} (ht : t < NT.val)
    {i : U8} (hi : i = cfg.slot_deadline.val[t]'(by have := cfg.slot_deadline.property; omega)) :
    (UScalar.cast .Usize i).val < NS.val := by
  have h := (hc t ht).2.1
  rw [getE (by have := cfg.slot_deadline.property; omega) hi] at h
  scalar_tac

theorem some_of {α} {l : List α} {i : Nat} {h : i < l.length} {x : α} (hx : x = l[i]) :
    l[i]? = some x := by
  rw [hx]; exact List.getElem?_eq_getElem h

theorem rl_slot {cfg : logic.Config NT NR NP} (hc : CfgOk cfg NS) {t : Nat} (ht : t < NT.val)
    {tc : logic.TaskCfg} (htc : cfg.tasks.val[t]? = some tc)
    (hk : tc.kind = logic.KIND_PERIODIC ∨ tc.mit_policy = logic.MIT_DEFER)
    {i : U8} (hi : i = cfg.slot_release.val[t]'(by have := cfg.slot_release.property; omega)) :
    (UScalar.cast .Usize i).val < NS.val := by
  have h := (hc t ht).2.2 tc htc hk
  rw [getE (by have := cfg.slot_release.property; omega) hi] at h
  scalar_tac

theorem bg_slot {cfg : logic.Config NT NR NP} (hc : CfgOk cfg NS) {t : Nat} (ht : t < NT.val)
    {i : U8} (hi : i = cfg.slot_budget.val[t]'(by have := cfg.slot_budget.property; omega)) :
    (UScalar.cast .Usize i).val < NS.val := by
  have h := (hc t ht).1
  rw [getE (by have := cfg.slot_budget.property; omega) hi] at h
  scalar_tac

set_option maxHeartbeats 1000000 in
theorem clear_loop_total (cfg : logic.Config NT NR NP) (partition : U8) (hc : CI cfg NS)
    (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize) (hk : KI k) (ht : t.val ≤ NT.val) :
    logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t ⦃ k' => KI k' ⦄ := by
  have hNT := hc.nt
  unfold logic.kernel.Kernel.clear_partition_releases_loop
  apply loop.spec_decr_nat
    (fun (x : logic.kernel.Kernel NT NR NP NL NS NE × Usize) => NT.val - x.2.val)
    (fun x => KI x.1 ∧ x.2.val ≤ NT.val)
  · intro x hx
    obtain ⟨k1, t1⟩ := x
    obtain ⟨hk1, h2⟩ := hx
    try dsimp only at hk1
    try dsimp only at h2
    try dsimp only
    unfold logic.kernel.Kernel.clear_partition_releases_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : t1.val < NT.val := by scalar_tac
      have hc8 : (UScalar.cast .U8 t1).val = t1.val := cast8 _ (by omega)
      have hlm : t1.val < k1.timing.masked.val.length := by
        have := k1.timing.masked.property; omega
      have hld : t1.val < cfg.slot_deadline.val.length := by
        have := cfg.slot_deadline.property; omega
      have hlr : t1.val < cfg.slot_release.val.length := by
        have := cfg.slot_release.property; omega
      step as ⟨ tc, htc ⟩
      refine WP.spec_bind
        (Pₘ := fun self1 : logic.kernel.Kernel NT NR NP NL NS NE => KI self1) ?_
        (fun self1 h1 => ?_)
      · split
        · simp only [lift, bind_ok]
          refine WP.spec_bind (clear_release_spec k1.sched cfg _ (by rw [hc8]; exact hlt') hk1.s)
            (fun s hs => ?_)
          refine WP.spec_bind
            (Pₘ := fun _ : logic.time.Timing NT × logic.time.Slots NS => True) ?_ (fun p _ => ?_)
          · split
            · step as ⟨ b, hb ⟩
              refine WP.spec_bind (Pₘ := fun _ : logic.time.Slots NS => True) ?_ (fun s2 _ => ?_)
              · split
                · refine WP.spec_bind (unmask_instant_total _ cfg _ (by rw [hc8]; exact hlt'))
                    (fun a _ => ?_)
                  step as ⟨ i2, hi2 ⟩
                  exact WP.spec_mono (arm_spec k1.slots _ _ _ _ (dl_slot hc.ok hlt' hi2))
                    (fun _ _ => trivial)
                · step as ⟨ i1, hi1 ⟩
                  exact WP.spec_mono (disarm_spec k1.slots _ (dl_slot hc.ok hlt' hi1))
                    (fun _ _ => trivial)
              · exact (WP.spec_ok _).mpr trivial
            · step as ⟨ i1, hi1 ⟩
              refine WP.spec_bind (disarm_spec k1.slots _ (dl_slot hc.ok hlt' hi1)) (fun s2 _ => ?_)
              exact (WP.spec_ok _).mpr trivial
          · obtain ⟨tm1, s1⟩ := p
            simp only [Aeneas.Std.uncurry]
            refine WP.spec_bind (Pₘ := fun _ : logic.time.Slots NS => True) ?_ (fun s2 _ => ?_)
            · split
              · split
                · step as ⟨ i1, hi1 ⟩
                  exact WP.spec_mono (disarm_spec s1 _ (rl_slot hc.ok hlt' (some_of htc) (Or.inr (by assumption)) hi1)) (fun _ _ => trivial)
                · exact (WP.spec_ok _).mpr trivial
              · exact (WP.spec_ok _).mpr trivial
            · refine WP.spec_bind (reset_total k1.budget _ (by rw [hc8]; exact hlt')) (fun b _ => ?_)
              exact (WP.spec_ok _).mpr ⟨hs.1, hk1.l, hk1.n⟩
        · exact (WP.spec_ok _).mpr hk1
      · step as ⟨ t2, ht2 ⟩
        exact ⟨h1, by scalar_tac, by scalar_tac⟩
    · exact (WP.spec_ok _).mpr hk1
  · exact ⟨hk, ht⟩

/-- `clear_partition_releases`: the Releases of the Partition's Tasks are
dropped and their slots freed. -/
theorem clear_partition_releases_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (partition : U8) (hk : KI k) (hc : CI cfg NS)
    (hp : partition.val < NP.val) :
    logic.kernel.Kernel.clear_partition_releases k cfg partition ⦃ k' => KI k' ⦄ := by
  unfold logic.kernel.Kernel.clear_partition_releases
  refine WP.spec_bind (clear_loop_total cfg partition hc k 0#usize hk (by scalar_tac))
    (fun self1 h1 => ?_)
  simp only [lift, bind_ok]
  have hlen : partition.val < self1.stop_dma.val.length := by
    have := self1.stop_dma.property; scalar_tac
  step as ⟨ a, ha ⟩
  exact ⟨h1.s, h1.l, h1.n⟩

/-! ## The end of the Job on top -/

theorem end_job_spec (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (hs : SInv s) (hd : 0 < s.depth.val) :
    logic.sched.Sched.end_job s cfg ⦃ r =>
      SInv r.2 ∧ r.2.depth.val = s.depth.val - 1 ⦄ :=
  spec_of_total (end_job_total s cfg hs hd) (fun r h => by
    have hv := finish_top_views (r := r.1) (s' := r.2) (by rw [finish_top_eq]; exact h)
    obtain ⟨_, _, _, _, _, v_stk, v_dep, v_res, _, v_len, _, _, _⟩ := hv
    obtain ⟨h1, h2, h3, h4⟩ := hs
    exact ⟨⟨by omega, fun p hp => by rw [v_stk]; exact h2 p (by omega), by omega,
      fun p hp => by rw [v_res]; exact h4 p (by omega)⟩, v_dep⟩)

theorem top_holds_nothing_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (hs : SInv k.sched) (hd : 0 < k.sched.depth.val) :
    logic.kernel.Kernel.top_holds_nothing_exec k ⦃ _ => True ⦄ := by
  unfold logic.kernel.Kernel.top_holds_nothing_exec
    logic.kernel.Kernel.top_holds_nothing_exec_loop
  apply loop.spec_decr_nat (fun (x : Usize × Bool) => k.sched.lock_len.val - x.1.val)
    (fun x => x.1.val ≤ k.sched.lock_len.val)
  · intro x hx
    obtain ⟨e, all⟩ := x
    try dsimp only at hx
    try dsimp only
    unfold logic.kernel.Kernel.top_holds_nothing_exec_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlen : e.val < k.sched.lock_depth.val.length := by
        have := k.sched.lock_depth.property; have := hs.2.2.1; scalar_tac
      step as ⟨ i, hi ⟩
      step as ⟨ i1, hi1 ⟩
      split
      · simp only [bind_ok]
        step as ⟨ e1, he1 ⟩
        exact ⟨by scalar_tac, by scalar_tac⟩
      · simp only [bind_ok]
        step as ⟨ e1, he1 ⟩
        exact ⟨by scalar_tac, by scalar_tac⟩
    · exact (WP.spec_ok _).mpr trivial
  · scalar_tac

set_option maxHeartbeats 1000000 in
/-- `end_top_release`: the Job on top is ended, its Resources released. -/
theorem end_top_release_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (hk : KI k) (hc : CI cfg NS) (hd : 0 < k.sched.depth.val) :
    logic.kernel.Kernel.end_top_release k cfg ⦃ k' =>
      KI k' ∧ k'.sched.depth.val = k.sched.depth.val - 1 ⦄ := by
  have hs := hk.s
  obtain ⟨h1, h2, h3, h4⟩ := hs
  unfold logic.kernel.Kernel.end_top_release
  step as ⟨ i, hi ⟩
  have hlen : i.val < k.sched.stack.val.length := by
    have := k.sched.stack.property; scalar_tac
  step as ⟨ t, ht ⟩
  have ht' := getE hlen ht
  have hst := h2 i.val (by scalar_tac)
  simp only [stk] at hst
  rw [ht'] at hst
  simp only [lift, bind_ok]
  have hlen2 : t.val < cfg.slot_budget.val.length := by
    have := cfg.slot_budget.property; scalar_tac
  step as ⟨ sb, hsb ⟩
  simp only [U8.cast_Usize_val_eq] at hsb
  refine WP.spec_bind (disarm_spec k.slots _ (bg_slot hc.ok hst hsb)) (fun sl _ => ?_)
  refine WP.spec_bind (end_job_spec k.sched cfg hk.s hd) (fun x hx => ?_)
  obtain ⟨released, s1⟩ := x
  dsimp only at hx
  obtain ⟨hs1, hd1⟩ := hx
  simp only [Aeneas.Std.uncurry]
  refine WP.spec_bind
    (Pₘ := fun self1 : logic.kernel.Kernel NT NR NP NL NS NE => KI self1 ∧ self1.sched = s1) ?_
    (fun self1 h => ?_)
  · split
    · step as ⟨ tc, htc ⟩
      step as ⟨ i5, hi5 ⟩
      exact WP.spec_mono (push_ki _ cfg _ _ _ _ _ ⟨hs1, hk.l, hk.n⟩)
        (fun k1 hk1 => ⟨hk1.1, hk1.2.1⟩)
    · exact (WP.spec_ok _).mpr ⟨⟨hs1, hk.l, hk.n⟩, rfl⟩
  · obtain ⟨g1, g2⟩ := h
    have hlen3 : t.val < self1.sched.release.val.length := by
      have := self1.sched.release.property; scalar_tac
    step as ⟨ i5, hi5 ⟩
    split
    · have hlen4 : t.val < cfg.slot_deadline.val.length := by
        have := cfg.slot_deadline.property; scalar_tac
      step as ⟨ i7, hi7 ⟩
      simp only [U8.cast_Usize_val_eq] at hi7
      refine WP.spec_bind (disarm_spec self1.slots _ (dl_slot hc.ok hst hi7)) (fun s2 _ => ?_)
      exact (WP.spec_ok _).mpr ⟨⟨g1.s, g1.l, g1.n⟩, by rw [g2]; exact hd1⟩
    · exact (WP.spec_ok _).mpr ⟨g1, by rw [g2]; exact hd1⟩

/-- `end_top`: the Job on top is ended and the Budget event of the Job it
had preempted armed again. -/
theorem end_top_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (hk : KI k) (hc : CI cfg NS) (hd : 0 < k.sched.depth.val) :
    logic.kernel.Kernel.end_top k cfg ⦃ k' =>
      KI k' ∧ k'.sched.depth.val = k.sched.depth.val - 1 ⦄ := by
  unfold logic.kernel.Kernel.end_top
  step as ⟨ i, hi ⟩
  have hlen : i.val < k.sched.stack.val.length := by
    have := k.sched.stack.property; have := hk.s.1; scalar_tac
  step as ⟨ t, ht ⟩
  refine WP.spec_bind (end_top_release_total k cfg hk hc hd) (fun self1 h1 => ?_)
  obtain ⟨g1, g2⟩ := h1
  split
  · step as ⟨ i1, hi1 ⟩
    have hlen2 : i1.val < self1.sched.stack.val.length := by
      have := self1.sched.stack.property; have := g1.s.1; scalar_tac
    step as ⟨ resumed, hres ⟩
    have hres' := getE hlen2 hres
    have hrt : resumed.val < NT.val := by
      have := g1.s.2.1 i1.val (by scalar_tac)
      simp only [stk] at this
      rw [hres'] at this
      exact this
    exact WP.spec_mono (arm_budget_total self1 cfg resumed hrt hc.ok g1.n)
      (fun k2 h => ⟨⟨by rw [h.1]; exact g1.s, by rw [h.2.1]; exact g1.l,
        by rw [h.2.2.1]; exact g1.n⟩, by rw [h.1]; exact g2⟩)
  · exact (WP.spec_ok _).mpr ⟨g1, g2⟩

/-! ## The Partition responses -/

theorem end_partition_jobs_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (partition : U8) (hk : KI k) (hc : CI cfg NS)
    (hp : partition.val < NP.val) :
    logic.kernel.Kernel.end_partition_jobs k cfg partition ⦃ k' => KI k' ⦄ := by
  unfold logic.kernel.Kernel.end_partition_jobs
  refine WP.spec_bind
    (Pₘ := fun x : logic.kernel.Kernel NT NR NP NL NS NE × logic.Config NT NR NP =>
      KI x.1 ∧ x.2 = cfg) ?_ (fun x hx => ?_)
  · split
    · rename_i hpos
      have hd : 0 < k.sched.depth.val := by scalar_tac
      step as ⟨ i, hi ⟩
      have hlen : i.val < k.sched.stack.val.length := by
        have := k.sched.stack.property; have := hk.s.1; scalar_tac
      step as ⟨ top, htop ⟩
      have htop' := getE hlen htop
      have hst := hk.s.2.1 i.val (by scalar_tac)
      simp only [stk] at hst
      rw [htop'] at hst
      simp only [lift, bind_ok]
      step as ⟨ tc, htc ⟩
      refine WP.spec_bind
        (Pₘ := fun k1 : logic.kernel.Kernel NT NR NP NL NS NE => KI k1) ?_ (fun k1 hk1 => ?_)
      · split
        · exact WP.spec_mono (end_top_total k cfg hk hc hd) (fun _ h => h.1)
        · exact (WP.spec_ok _).mpr hk
      · exact (WP.spec_ok _).mpr ⟨hk1, rfl⟩
    · exact (WP.spec_ok _).mpr ⟨hk, rfl⟩
  · obtain ⟨self1, cfg1⟩ := x
    obtain ⟨g1, g2⟩ := hx
    dsimp only at g1 g2
    subst g2
    simp only [Aeneas.Std.uncurry]
    refine WP.spec_bind (kill_partition_jobs_total self1 cfg1 partition g1) (fun self2 h2 => ?_)
    exact clear_partition_releases_total self2 cfg1 partition h2 hc hp

/-- `stop_partition` (R14.5). -/
theorem stop_partition_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (partition cause : U8) (hk : KI k) (hc : CI cfg NS)
    (hp : partition.val < NP.val) :
    logic.kernel.Kernel.stop_partition k cfg partition cause ⦃ k' => KI k' ⦄ := by
  unfold logic.kernel.Kernel.stop_partition
  simp only [lift, bind_ok]
  refine WP.spec_bind (push_ki k cfg _ _ _ _ _ hk) (fun self1 h1 => ?_)
  refine WP.spec_bind (end_partition_jobs_total self1 cfg partition h1.1 hc hp) (fun self2 h2 => ?_)
  have hlen : partition.val < self2.stopped.val.length := by
    have := self2.stopped.property; scalar_tac
  step as ⟨ a, ha ⟩
  exact ⟨h2.s, h2.l, h2.n⟩

/-- `restart_partition` (R14.3). -/
theorem restart_partition_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (partition cause : U8) (hk : KI k) (hc : CI cfg NS)
    (hp : partition.val < NP.val) :
    logic.kernel.Kernel.restart_partition k cfg partition cause ⦃ k' => KI k' ⦄ := by
  unfold logic.kernel.Kernel.restart_partition
  simp only [lift, bind_ok]
  refine WP.spec_bind (push_ki k cfg _ _ _ _ _ hk) (fun self1 h1 => ?_)
  refine WP.spec_bind (end_partition_jobs_total self1 cfg partition h1.1 hc hp) (fun self2 h2 => ?_)
  have hlen : partition.val < self2.mit_violations.val.length := by
    have := self2.mit_violations.property; scalar_tac
  have hlen2 : partition.val < self2.reinit_partition.val.length := by
    have := self2.reinit_partition.property; scalar_tac
  step as ⟨ a, ha ⟩
  step as ⟨ a1, ha1 ⟩
  exact ⟨h2.s, h2.l, h2.n⟩

theorem end_mit_window_frame {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') :
    k'.sched = k.sched ∧ k'.log = k.log ∧ k'.now = k.now := by
  unfold logic.kernel.Kernel.end_mit_window at h
  peel h
  all_goals exact ⟨rfl, rfl, rfl⟩

theorem end_mit_window_ki (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (t : U8) (ht : t.val < NT.val) (hc : CfgOk cfg NS)
    (hk : KI k) : logic.kernel.Kernel.end_mit_window k cfg t ⦃ k' => KI k' ⦄ :=
  spec_of_total (WP.spec_mono (end_mit_window_total k cfg t ht hc) (fun _ _ => trivial))
    (fun r h => by
      obtain ⟨a, b, c⟩ := end_mit_window_frame h
      exact ⟨by rw [a]; exact hk.s, by rw [b]; exact hk.l, by rw [c]; exact hk.n⟩)

set_option maxHeartbeats 1000000 in
/-- **`apply_response` returns**, for every response: the set is closed
(R14.2), and each member keeps the Kernel's state within its bounds. -/
theorem apply_response_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (partition response cause t : U8) (hk : KI k)
    (hc : CI cfg NS) (hp : partition.val < NP.val) (ht : t.val < NT.val) :
    logic.kernel.Kernel.apply_response k cfg partition response cause t ⦃ k' => KI k' ⦄ := by
  unfold logic.kernel.Kernel.apply_response
  split
  · exact (WP.spec_ok _).mpr hk
  split
  · exact (WP.spec_ok _).mpr hk
  split
  · split
    · rename_i hpos
      have hd : 0 < k.sched.depth.val := by scalar_tac
      step as ⟨ i, hi ⟩
      have hlen : i.val < k.sched.stack.val.length := by
        have := k.sched.stack.property; have := hk.s.1; scalar_tac
      step as ⟨ top, htop ⟩
      split
      · refine WP.spec_bind (top_holds_nothing_total k hk.s hd) (fun hn _ => ?_)
        split
        · simp only [lift, bind_ok]
          refine WP.spec_bind (push_ki k cfg _ _ _ _ _ hk) (fun self1 h1 => ?_)
          refine WP.spec_bind (end_top_total self1 cfg h1.1 hc (by rw [h1.2.1]; exact hd))
            (fun self2 h2 => ?_)
          step as ⟨ tc, htc ⟩
          split
          · exact end_mit_window_ki self2 cfg t ht hc.ok h2.1
          · exact (WP.spec_ok _).mpr h2.1
        · exact restart_partition_total k cfg partition cause hk hc hp
      · exact restart_partition_total k cfg partition cause hk hc hp
    · exact restart_partition_total k cfg partition cause hk hc hp
  split
  · exact restart_partition_total k cfg partition cause hk hc hp
  split
  · exact stop_partition_total k cfg partition cause hk hc hp
  · exact enter_safe_state_total k cfg hk hc.nt

/-! ## An event and its response -/

/-- The Partition of a Task of the configuration is one of its Partitions. -/
theorem part_lt {cfg : logic.Config NT NR NP} (hc : CI cfg NS) {i : Nat} {tc : logic.TaskCfg}
    {h : i < cfg.tasks.val.length} (htc : tc = cfg.tasks.val[i]) : tc.partition.val < NP.val := by
  rw [htc]; exact hc.part i h

set_option maxHeartbeats 1000000 in
/-- **`fault` returns** (R14.1, R14.4): a fault of the executing Job gets
its Partition's response, a fault of the Kernel or with no Job the safe
state. -/
theorem fault_total (k : logic.kernel.Kernel NT NR NP NL NS NE) (cfg : logic.Config NT NR NP)
    (kind : U8) (kn : Bool) (hk : KI k) (hc : CI cfg NS) :
    logic.kernel.Kernel.fault k cfg kind kn ⦃ k' => KI k' ⦄ := by
  unfold logic.kernel.Kernel.fault
  refine WP.spec_bind (fault_prs_total kind) (fun prs _ => ?_)
  split
  · refine WP.spec_bind (push_ki k cfg _ _ _ _ _ hk) (fun self1 h1 => ?_)
    exact enter_safe_state_total self1 cfg h1.1 hc.nt
  · split
    · refine WP.spec_bind (push_ki k cfg _ _ _ _ _ hk) (fun self1 h1 => ?_)
      exact enter_safe_state_total self1 cfg h1.1 hc.nt
    · rename_i hne
      have hd : 0 < k.sched.depth.val := by scalar_tac
      step as ⟨ i, hi ⟩
      have hlen : i.val < k.sched.stack.val.length := by
        have := k.sched.stack.property; have := hk.s.1; scalar_tac
      step as ⟨ t, ht ⟩
      have ht' := getE hlen ht
      have hst := hk.s.2.1 i.val (by scalar_tac)
      simp only [stk] at hst
      rw [ht'] at hst
      simp only [lift, bind_ok]
      step as ⟨ tc, htc ⟩
      have hpart := part_lt hc htc
      refine WP.spec_bind (push_ki k cfg _ _ _ _ _ hk) (fun self1 h1 => ?_)
      have hlenp : tc.partition.val < cfg.partitions.val.length := by
        have := cfg.partitions.property; scalar_tac
      step as ⟨ pc, hpc ⟩
      exact apply_response_total self1 cfg _ _ _ _ h1.1 hc hpart hst

/-- **`fault_of` returns** (R42.4): the event against another Partition's
Task and that Partition's response. -/
theorem fault_of_total (k : logic.kernel.Kernel NT NR NP NL NS NE) (cfg : logic.Config NT NR NP)
    (kind t : U8) (hk : KI k) (hc : CI cfg NS) (ht : t.val < NT.val) :
    logic.kernel.Kernel.fault_of k cfg kind t ⦃ k' => KI k' ⦄ := by
  unfold logic.kernel.Kernel.fault_of
  refine WP.spec_bind (fault_prs_total kind) (fun prs _ => ?_)
  simp only [lift, bind_ok]
  step as ⟨ tc, htc ⟩
  have hpart := part_lt hc htc
  refine WP.spec_bind (push_ki k cfg _ _ _ _ _ hk) (fun self1 h1 => ?_)
  have hlenp : tc.partition.val < cfg.partitions.val.length := by
    have := cfg.partitions.property; scalar_tac
  step as ⟨ pc, hpc ⟩
  exact apply_response_total self1 cfg _ _ _ _ h1.1 hc hpart ht

/-- **`deadline_miss` returns** (R9.6): the event and the Partition's
Deadline response. -/
theorem deadline_miss_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (t : U8) (i : Usize) (hk : KI k) (hc : CI cfg NS)
    (ht : t.val < NT.val) (hi : i.val < NS.val) :
    logic.kernel.Kernel.deadline_miss k cfg t i ⦃ k' => KI k' ⦄ := by
  unfold logic.kernel.Kernel.deadline_miss
  refine WP.spec_bind (disarm_spec k.slots i hi) (fun s _ => ?_)
  simp only [lift, bind_ok]
  step as ⟨ tc, htc ⟩
  have hpart := part_lt hc htc
  refine WP.spec_bind prs_budget_total (fun i2 _ => ?_)
  refine WP.spec_bind (push_ki _ cfg _ _ _ _ _ ⟨hk.s, hk.l, hk.n⟩) (fun self1 h1 => ?_)
  have hlenp : tc.partition.val < cfg.partitions.val.length := by
    have := cfg.partitions.property; scalar_tac
  step as ⟨ pc, hpc ⟩
  exact apply_response_total self1 cfg _ _ _ _ h1.1 hc hpart ht

theorem arm_budget_ki (k : logic.kernel.Kernel NT NR NP NL NS NE) (cfg : logic.Config NT NR NP)
    (t : U8) (ht : t.val < NT.val) (hc : CfgOk cfg NS) (hk : KI k) :
    logic.kernel.Kernel.arm_budget k cfg t ⦃ k' => KI k' ⦄ :=
  WP.spec_mono (arm_budget_total k cfg t ht hc hk.n)
    (fun k2 h => ⟨by rw [h.1]; exact hk.s, by rw [h.2.1]; exact hk.l, by rw [h.2.2.1]; exact hk.n⟩)

set_option maxHeartbeats 1000000 in
/-- **`budget_event` returns** (R18.2): the Overrun's event and response,
or the slot armed again for what is left of the Budget. -/
theorem budget_event_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (t : U8) (i : Usize) (hk : KI k) (hc : CI cfg NS)
    (ht : t.val < NT.val) (hi : i.val < NS.val) :
    logic.kernel.Kernel.budget_event k cfg t i ⦃ k' => KI k' ⦄ := by
  unfold logic.kernel.Kernel.budget_event
  refine WP.spec_bind (disarm_spec k.slots i hi) (fun s _ => ?_)
  split
  · exact (WP.spec_ok _).mpr ⟨hk.s, hk.l, hk.n⟩
  · rename_i hne
    have hd : 0 < k.sched.depth.val := by scalar_tac
    step as ⟨ i1, hi1 ⟩
    have hlen : i1.val < k.sched.stack.val.length := by
      have := k.sched.stack.property; have := hk.s.1; scalar_tac
    step as ⟨ top, htop ⟩
    split
    · exact (WP.spec_ok _).mpr ⟨hk.s, hk.l, hk.n⟩
    · simp only [lift, bind_ok]
      have hlenr : t.val < k.budget.reported.val.length := by
        have := k.budget.reported.property; scalar_tac
      step as ⟨ b, hb ⟩
      split
      · exact (WP.spec_ok _).mpr ⟨hk.s, hk.l, hk.n⟩
      · refine WP.spec_bind (overrun_total k.budget cfg t ht) (fun b1 _ => ?_)
        split
        · refine WP.spec_bind (report_total k.budget cfg t ht) (fun b2 _ => ?_)
          step as ⟨ tc, htc ⟩
          have hpart := part_lt hc htc
          have hlenu : t.val < b2.used.val.length := by
            have := b2.used.property; scalar_tac
          step as ⟨ i6, hi6 ⟩
          refine WP.spec_bind prs_budget_total (fun i7 _ => ?_)
          refine WP.spec_bind (push_ki _ cfg _ _ _ _ _ ⟨hk.s, hk.l, hk.n⟩) (fun self1 h1 => ?_)
          have hlenp : tc.partition.val < cfg.partitions.val.length := by
            have := cfg.partitions.property; scalar_tac
          step as ⟨ pc, hpc ⟩
          exact apply_response_total self1 cfg _ _ _ _ h1.1 hc hpart ht
        · exact arm_budget_ki _ cfg t ht hc.ok ⟨hk.s, hk.l, hk.n⟩

/-- **`mit_violation` returns** (R10.4): the event, the Partition's count,
and its Fault_Response once the count passes the threshold. -/
theorem mit_violation_total (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (t partition offender : U8) (hk : KI k) (hc : CI cfg NS)
    (hp : partition.val < NP.val) (ho : offender.val < NT.val) :
    logic.kernel.Kernel.mit_violation k cfg t partition offender ⦃ k' => KI k' ⦄ := by
  unfold logic.kernel.Kernel.mit_violation
  refine WP.spec_bind prs_release_total (fun i _ => ?_)
  refine WP.spec_bind (push_ki k cfg _ _ _ _ _ hk) (fun self1 h1 => ?_)
  simp only [lift, bind_ok]
  have hlen : partition.val < self1.mit_violations.val.length := by
    have := self1.mit_violations.property; scalar_tac
  have hlenp : partition.val < cfg.partitions.val.length := by
    have := cfg.partitions.property; scalar_tac
  step as ⟨ i2, hi2 ⟩
  step as ⟨ old, back, hold, hback ⟩
  step as ⟨ pc, hpc ⟩
  split
  · step as ⟨ pc1, hpc1 ⟩
    exact apply_response_total _ cfg _ _ _ _ ⟨h1.1.s, h1.1.l, h1.1.n⟩ hc hp ho
  · exact (WP.spec_ok _).mpr ⟨h1.1.s, h1.1.l, h1.1.n⟩

end P18
