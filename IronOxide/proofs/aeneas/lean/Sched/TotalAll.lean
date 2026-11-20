import Sched.TotalTime
open Aeneas Aeneas.Std Result

/-! # Every Kernel entry returns and keeps the invariant (2026-11-20)

Property 21, closed over the Kernel's entries. `Sched/Total.lean` proves
that `dispatch`, `job_return`, `svc_lock`, `svc_unlock` and `settle`
return; `Sched/TotalResp.lean` and `Sched/TotalTime.lean` that the faults,
the intake and the timer entry return and keep `KT`. This file shows the
first five keep `KT` too, adds `Kernel::start`, the acknowledgements, the
event record and `Layout::view`, so that from a state satisfying `KT`
every entry returns a state satisfying `KT`: no entry can leave the Kernel
in a state from which a later one overflows, indexes out of bounds or
panics. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! ## The log and the instant across an entry -/

/-- The log stays well formed and the observed instant is the same. -/
def LNf (l' l : logic.log.Log NE) (n' n : U64) : Prop := (Lwf l → Lwf l') ∧ n' = n

theorem LNf.refl (l : logic.log.Log NE) (n : U64) : LNf l l n n := ⟨id, rfl⟩

theorem LNf.trans {l0 l1 l2 : logic.log.Log NE} {n0 n1 n2 : U64} (h1 : LNf l1 l0 n1 n0)
    (h2 : LNf l2 l1 n2 n1) : LNf l2 l0 n2 n0 := ⟨fun h => h2.1 (h1.1 h), h2.2.trans h1.2⟩

syntax "ln_call" : tactic
macro_rules | `(tactic| ln_call) => `(tactic| fail "no call")

macro "ln_reg " l:ident : command =>
  `(macro_rules | `(tactic| ln_call) => `(tactic| refine LNf.trans ?_ ($l (by assumption))))

syntax "ln_close" : tactic
macro_rules
  | `(tactic| ln_close) => `(tactic| repeat (first
      | exact LNf.refl _ _
      | assumption
      | ln_call
      | dsimp only))

macro "ln_peel " h:ident : tactic =>
  `(tactic| (
      peel $h
      all_goals first
        | (ln_close; done)
        | (rcases ite_cases $h with ⟨_, $h⟩ | ⟨_, $h⟩ <;> peel $h <;> first
            | (ln_close; done)
            | (rcases ite_cases $h with ⟨_, $h⟩ | ⟨_, $h⟩ <;> peel $h <;> first
                | (ln_close; done)
                | (rcases ite_cases $h with ⟨_, $h⟩ | ⟨_, $h⟩ <;> peel $h <;>
                    (ln_close; done))))))

variable {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP}

theorem push_event_now {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    k'.now = k.now := by
  unfold logic.kernel.Kernel.push_event at h
  peel h
  rfl

theorem push_event_ln {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    LNf k'.log k.log k'.now k.now := by
  refine ⟨fun hl => ?_, push_event_now h⟩
  obtain ⟨r, hr, hp⟩ := spec_returns (push_event_total k cfg kind task partition prs detail hl)
  have : r = k' := ok_inj (hr.symm.trans h)
  rw [← this]
  exact hp.1

ln_reg push_event_ln

theorem arm_budget_ln {t : U8} (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') :
    LNf k'.log k.log k'.now k.now := by
  unfold logic.kernel.Kernel.arm_budget at h
  ln_peel h

ln_reg arm_budget_ln

theorem end_mit_window_ln {t : U8} (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') :
    LNf k'.log k.log k'.now k.now := by
  unfold logic.kernel.Kernel.end_mit_window at h
  ln_peel h

ln_reg end_mit_window_ln

set_option maxHeartbeats 2000000 in
theorem dispatch_ln {t : U8} {pc : U32}
    (h : logic.kernel.Kernel.dispatch k cfg t pc = ok k') : LNf k'.log k.log k'.now k.now := by
  unfold logic.kernel.Kernel.dispatch at h
  ln_peel h

set_option maxHeartbeats 4000000 in
theorem job_return_ln {ec : U32}
    (h : logic.kernel.Kernel.job_return k cfg ec = ok k') : LNf k'.log k.log k'.now k.now := by
  unfold logic.kernel.Kernel.job_return at h
  ln_peel h

set_option maxHeartbeats 2000000 in
theorem svc_lock_ln {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_lock k cfg r = ok (b, k')) :
    LNf k'.log k.log k'.now k.now := by
  unfold logic.kernel.Kernel.svc_lock at h
  ln_peel h

set_option maxHeartbeats 2000000 in
theorem svc_unlock_ln {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (b, k')) :
    LNf k'.log k.log k'.now k.now := by
  unfold logic.kernel.Kernel.svc_unlock at h
  ln_peel h

set_option maxHeartbeats 4000000 in
theorem settle_ln {x : Option U8 × Bool}
    (h : logic.kernel.Kernel.settle k cfg = ok (x, k')) : LNf k'.log k.log k'.now k.now := by
  unfold logic.kernel.Kernel.settle at h
  ln_peel h

/-! ## The three arrays of instants across the same entries -/

theorem job_started_tft {tm tm' : logic.time.Timing NT} {t : U8}
    (h : logic.time.Timing.job_started tm cfg t = ok tm') : TFt tm' tm := by
  unfold logic.time.Timing.job_started at h
  peel h
  exact ⟨rfl, rfl, rfl⟩

tf_reg job_started_tft

set_option maxHeartbeats 2000000 in
theorem dispatch_tf {t : U8} {pc : U32}
    (h : logic.kernel.Kernel.dispatch k cfg t pc = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.dispatch at h
  tf_peel h

set_option maxHeartbeats 4000000 in
theorem job_return_tf {ec : U32}
    (h : logic.kernel.Kernel.job_return k cfg ec = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.job_return at h
  tf_peel h

set_option maxHeartbeats 2000000 in
theorem svc_lock_tf {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_lock k cfg r = ok (b, k')) : TF k' k := by
  unfold logic.kernel.Kernel.svc_lock at h
  tf_peel h

set_option maxHeartbeats 2000000 in
theorem svc_unlock_tf {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (b, k')) : TF k' k := by
  unfold logic.kernel.Kernel.svc_unlock at h
  tf_peel h

set_option maxHeartbeats 4000000 in
theorem settle_tf {x : Option U8 × Bool}
    (h : logic.kernel.Kernel.settle k cfg = ok (x, k')) : TF k' k := by
  unfold logic.kernel.Kernel.settle at h
  tf_peel h

set_option maxHeartbeats 2000000 in
theorem fault_tf {kind : U8} {kn : Bool}
    (h : logic.kernel.Kernel.fault k cfg kind kn = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.fault at h
  tf_peel h

theorem fault_of_tf {kind t : U8}
    (h : logic.kernel.Kernel.fault_of k cfg kind t = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.fault_of at h
  tf_peel h

theorem record_event_tf {kind t : U8}
    (h : logic.kernel.Kernel.record_event k cfg kind t = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.record_event at h
  tf_peel h

/-! ## The scheduler state within its bounds -/

theorem start_sinv {s s' : logic.sched.Sched NT NR NL} {t : U8} (hs : SInv s)
    (h : logic.sched.Sched.start s cfg t = ok s') : SInv s' := by
  obtain ⟨ht, hdn, d, hd, rfl⟩ := start_ok h
  obtain ⟨h1, h2, h3, h4⟩ := hs
  refine ⟨by dsimp only; omega, fun p hp => ?_, h3, h4⟩
  dsimp only at hp
  simp only [stk]
  rw [get_set _ _ _ _ hdn]
  split
  · exact ht
  · exact h2 p (by omega)

theorem finish_top_sinv {s s' : logic.sched.Sched NT NR NL} {r : Usize} (hs : SInv s)
    (h : logic.sched.Sched.finish_top s cfg = ok (r, s')) : SInv s' := by
  obtain ⟨_, _, _, _, _, v_stk, v_dep, v_res, _, v_len, _, _, _⟩ := finish_top_views h
  obtain ⟨h1, h2, h3, h4⟩ := hs
  exact ⟨by omega, fun p hp => by rw [v_stk]; exact h2 p (by omega), by omega,
    fun p hp => by rw [v_res]; exact h4 p (by omega)⟩

theorem lock_sinv {s s' : logic.sched.Sched NT NR NL} {r : U8} {b : Bool} (hs : SInv s)
    (hr : r.val < NR.val) (h : logic.sched.Sched.lock s cfg r = ok (b, s')) : SInv s' := by
  cases b with
  | false => rw [lock_refused h]; exact hs
  | true =>
    obtain ⟨_, hlt, n, d, c, hn, _, _, rfl⟩ := lock_ok h
    obtain ⟨h1, h2, h3, h4⟩ := hs
    refine ⟨h1, h2, by dsimp only; omega, fun p hp => ?_⟩
    dsimp only at hp
    simp only [lres]
    rw [get_set _ _ _ _ hlt]
    split
    · exact hr
    · exact h4 p (by omega)

theorem unlock_sinv {s s' : logic.sched.Sched NT NR NL} {r : U8} {b : Bool} (hs : SInv s)
    (h : logic.sched.Sched.unlock s cfg r = ok (b, s')) : SInv s' := by
  cases b with
  | false => rw [unlock_refused h]; exact hs
  | true =>
    obtain ⟨_, _, _, _, n, c, hn, _, rfl⟩ := unlock_ok h
    obtain ⟨h1, h2, h3, h4⟩ := hs
    exact ⟨h1, h2, by dsimp only; omega, fun p hp => h4 p (by dsimp only at hp; omega)⟩

/-! ## The entries keep `KT` -/

/-- A function that returns, keeps the scheduler state within its bounds,
the log well formed, the instant and the three arrays keeps `KT`. -/
theorem kt_parts {α} {x : Result α} (π : α → logic.kernel.Kernel NT NR NP NL NS NE)
    {k : logic.kernel.Kernel NT NR NP NL NS NE} (hk : KT k) (ht : x ⦃ _ => True ⦄)
    (hs : ∀ r, x = ok r → SInv (π r).sched)
    (hl : ∀ r, x = ok r → LNf (π r).log k.log (π r).now k.now)
    (hf : ∀ r, x = ok r → TF (π r) k) : x ⦃ r => KT (π r) ⦄ := by
  obtain ⟨r, hr, _⟩ := spec_returns ht
  rw [hr]
  obtain ⟨l1, l2⟩ := hl r hr
  exact (WP.spec_ok _).mpr ⟨⟨hs r hr, l1 hk.ki.l, by rw [l2]; exact hk.ki.n⟩,
    hk.tm.of_tft (hf r hr)⟩

variable (k) (cfg)

/-- **`dispatch` returns and keeps the invariant.** -/
theorem dispatch_kt (t : U8) (pc : U32) (hk : KT k) (hc : CT cfg NS) (ht : t.val < NT.val)
    (hd : k.sched.depth.val < NT.val) :
    logic.kernel.Kernel.dispatch k cfg t pc ⦃ k' => KT k' ⦄ :=
  kt_parts (fun x => x) hk (dispatch_total k cfg t pc hk.ki.s ht hd hc.ci.ok hk.ki.n)
    (fun _ h => start_sinv hk.ki.s (dispatch_eff h)) (fun _ h => dispatch_ln h)
    (fun _ h => dispatch_tf h)

/-- **`job_return` returns and keeps the invariant.** -/
theorem job_return_kt (ec : U32) (hk : KT k) (hc : CT cfg NS) (hd : 0 < k.sched.depth.val) :
    logic.kernel.Kernel.job_return k cfg ec ⦃ k' => KT k' ⦄ :=
  kt_parts (fun x => x) hk (job_return_total k cfg ec hk.ki.s hd hc.ci.ok hk.ki.l hk.ki.n)
    (fun _ h => by
      obtain ⟨r, hr⟩ := job_return_eff h
      exact finish_top_sinv hk.ki.s hr)
    (fun _ h => job_return_ln h) (fun _ h => job_return_tf h)

/-- **`svc_lock` returns and keeps the invariant.** -/
theorem svc_lock_kt (r : U8) (hk : KT k) (hd : 0 < k.sched.depth.val) (hr : r.val < NR.val) :
    logic.kernel.Kernel.svc_lock k cfg r ⦃ x => KT x.2 ⦄ :=
  kt_parts Prod.snd hk
    (WP.spec_mono (svc_lock_total k cfg r hk.ki.s hd hr hk.ki.l) (fun _ _ => trivial))
    (fun x h => lock_sinv hk.ki.s hr (svc_lock_eff (b := x.1) (k' := x.2) h))
    (fun x h => svc_lock_ln (b := x.1) (k' := x.2) h)
    (fun x h => svc_lock_tf (b := x.1) (k' := x.2) h)

/-- **`svc_unlock` returns and keeps the invariant.** -/
theorem svc_unlock_kt (r : U8) (hk : KT k) (hd : 0 < k.sched.depth.val) :
    logic.kernel.Kernel.svc_unlock k cfg r ⦃ x => KT x.2 ⦄ :=
  kt_parts Prod.snd hk
    (WP.spec_mono (svc_unlock_total k cfg r hk.ki.s hd hk.ki.l) (fun _ _ => trivial))
    (fun x h => unlock_sinv hk.ki.s (svc_unlock_eff (b := x.1) (k' := x.2) h))
    (fun x h => svc_unlock_ln (b := x.1) (k' := x.2) h)
    (fun x h => svc_unlock_tf (b := x.1) (k' := x.2) h)

/-- **`settle` returns and keeps the invariant.** -/
theorem settle_kt (hk : KT k) (hc : CT cfg NS) :
    logic.kernel.Kernel.settle k cfg ⦃ x => KT x.2 ⦄ :=
  kt_parts Prod.snd hk
    (WP.spec_mono (settle_total k cfg hk.ki.s hc.ci.nt hc.ci.ok hk.ki.n) (fun _ _ => trivial))
    (fun x h => by rw [settle_sched (r := x.1) (k' := x.2) h]; exact hk.ki.s)
    (fun x h => settle_ln (x := x.1) (k' := x.2) h)
    (fun x h => settle_tf (x := x.1) (k' := x.2) h)

/-- **`fault` returns and keeps the invariant.** -/
theorem fault_kt (kind : U8) (kn : Bool) (hk : KT k) (hc : CT cfg NS) :
    logic.kernel.Kernel.fault k cfg kind kn ⦃ k' => KT k' ⦄ :=
  kt_spec hk (fault_total k cfg kind kn hk.ki hc.ci) (fun _ h => fault_tf h)

/-- **`fault_of` returns and keeps the invariant.** -/
theorem fault_of_kt (kind t : U8) (hk : KT k) (hc : CT cfg NS) (ht : t.val < NT.val) :
    logic.kernel.Kernel.fault_of k cfg kind t ⦃ k' => KT k' ⦄ :=
  kt_spec hk (fault_of_total k cfg kind t hk.ki hc.ci ht) (fun _ h => fault_of_tf h)

/-- **`record_event` returns and keeps the invariant.** -/
theorem record_event_kt (kind t : U8) (hk : KT k) (ht : t.val < NT.val) :
    logic.kernel.Kernel.record_event k cfg kind t ⦃ k' => KT k' ⦄ := by
  unfold logic.kernel.Kernel.record_event
  simp only [lift, bind_ok]
  step as ⟨ tc, htc ⟩
  step as ⟨ i1, hi1 ⟩
  exact WP.spec_mono (push_kt k cfg _ _ _ _ _ hk) (fun _ h => h.1)

theorem ack_reinit_kt (p : U8) (hk : KT k) (hp : p.val < NP.val) :
    logic.kernel.Kernel.ack_reinit k cfg p ⦃ k' => KT k' ⦄ := by
  have hlen : p.val < k.reinit_partition.val.length := by
    have := k.reinit_partition.property; omega
  unfold logic.kernel.Kernel.ack_reinit
  simp only [lift, bind_ok]
  step as ⟨ a, ha ⟩
  exact ⟨⟨hk.ki.s, hk.ki.l, hk.ki.n⟩, hk.tm⟩

theorem ack_stop_dma_kt (p : U8) (hk : KT k) (hp : p.val < NP.val) :
    logic.kernel.Kernel.ack_stop_dma k cfg p ⦃ k' => KT k' ⦄ := by
  have hlen : p.val < k.stop_dma.val.length := by
    have := k.stop_dma.property; omega
  unfold logic.kernel.Kernel.ack_stop_dma
  simp only [lift, bind_ok]
  step as ⟨ a, ha ⟩
  exact ⟨⟨hk.ki.s, hk.ki.l, hk.ki.n⟩, hk.tm⟩

/-! ## The start -/

theorem start_loop_total (hc : CT cfg NS) (t : Usize) (ht : t.val ≤ NT.val) (hk : KT k) :
    logic.kernel.Kernel.start_loop k cfg t ⦃ r => KT r.1 ∧ r.2 = cfg ⦄ := by
  unfold logic.kernel.Kernel.start_loop
  apply loop.spec_decr_nat
    (fun (x : logic.kernel.Kernel NT NR NP NL NS NE × Usize) => NT.val - x.2.val)
    (fun x => KT x.1 ∧ x.2.val ≤ NT.val)
  · intro x hx
    obtain ⟨k1, t1⟩ := x
    obtain ⟨hk1, h2⟩ := hx
    try dsimp only at hk1
    try dsimp only at h2
    try dsimp only
    unfold logic.kernel.Kernel.start_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : t1.val < NT.val := by scalar_tac
      have hlr : t1.val < cfg.slot_release.val.length := by
        have := cfg.slot_release.property; omega
      have hln : t1.val < k1.timing.nominal.val.length := by
        have := k1.timing.nominal.property; omega
      step as ⟨ tc, htc ⟩
      refine WP.spec_bind
        (Pₘ := fun self1 : logic.kernel.Kernel NT NR NP NL NS NE => KT self1) ?_
        (fun self1 h1 => ?_)
      · split
        · simp only [lift, bind_ok]
          step as ⟨ i, hi ⟩
          step as ⟨ at0, hat ⟩
          refine WP.spec_bind (arm_spec k1.slots _ _ _ _ (rl_slot hc.ci.ok hlt' (some_of htc) (Or.inl (by assumption)) hi)) (fun s _ => ?_)
          exact (WP.spec_ok _).mpr ⟨⟨hk1.ki.s, hk1.ki.l, hk1.ki.n⟩, hk1.tm⟩
        · exact (WP.spec_ok _).mpr hk1
      · step as ⟨ t2, ht2 ⟩
        exact ⟨h1, by scalar_tac, by scalar_tac⟩
    · exact (WP.spec_ok _).mpr ⟨hk1, rfl⟩
  · exact ⟨hk, ht⟩

/-- **`Kernel::start` returns and keeps the invariant**: the first releases,
the end of the operating duration and the housekeeping event are armed. -/
theorem start_kt (hk : KT k) (hc : CT cfg NS)
    (hsc : 2 ≤ cfg.slot_count.val ∧ cfg.slot_count.val ≤ NS.val) :
    logic.kernel.Kernel.start k cfg ⦃ k' => KT k' ⦄ := by
  unfold logic.kernel.Kernel.start
  refine WP.spec_bind (start_loop_total k cfg hc 0#usize (by scalar_tac) hk) (fun x hx => ?_)
  obtain ⟨self1, cfg1⟩ := x
  obtain ⟨g1, g2⟩ := hx
  dsimp only at g1 g2
  subst g2
  simp only [Aeneas.Std.uncurry, lift, bind_ok]
  step as ⟨ i, hi ⟩
  refine WP.spec_bind (arm_spec self1.slots _ _ _ _ (by scalar_tac)) (fun s _ => ?_)
  step as ⟨ i1, hi1 ⟩
  refine WP.spec_bind hk_period_total (fun i2 _ => ?_)
  refine WP.spec_bind (Pₘ := fun _ : U64 => True) (by ok_tac) (fun hk_at _ => ?_)
  refine WP.spec_bind (arm_spec s _ _ _ _ (by scalar_tac)) (fun s1 _ => ?_)
  exact (WP.spec_ok _).mpr ⟨⟨g1.ki.s, g1.ki.l, g1.ki.n⟩, g1.tm⟩

/-! ## The Partition view -/

theorem view_loop_total (a : Array logic.mpu.Interval 5#usize) (a1 : Array Bool 5#usize)
    (count : Usize) (hcnt : count.val ≤ 5) (regions : Array logic.mpu.Region 8#usize)
    (w : Usize) (hw : w.val ≤ count.val) :
    logic.mpu.Layout.view_loop a a1 regions count w ⦃ _ => True ⦄ := by
  unfold logic.mpu.Layout.view_loop
  apply loop.spec_decr_nat
    (fun (x : Array logic.mpu.Region 8#usize × Usize) => count.val - x.2.val)
    (fun x => x.2.val ≤ count.val)
  · intro x hx
    obtain ⟨rg, w1⟩ := x
    try dsimp only at hx
    try dsimp only
    unfold logic.mpu.Layout.view_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : w1.val < count.val := by scalar_tac
      step as ⟨ i, hi ⟩
      step as ⟨ b, hb ⟩
      step as ⟨ i1, hi1 ⟩
      step as ⟨ a2, ha2 ⟩
      step as ⟨ w2, hw2 ⟩
      exact ⟨by scalar_tac, by scalar_tac⟩
    · exact (WP.spec_ok _).mpr trivial
  · exact hw

/-- **`Layout::view` returns** (Property 12's function): for a Partition of
the layout with at most five peripheral windows. -/
theorem view_total (l : logic.mpu.Layout NP) (p : Usize) (hp : p.val < NP.val)
    (hwc : ∀ (h : p.val < l.partitions.val.length),
      (l.partitions.val[p.val]).window_count.val ≤ 5) :
    logic.mpu.Layout.view l p ⦃ _ => True ⦄ := by
  have hlen : p.val < l.partitions.val.length := by have := l.partitions.property; omega
  unfold logic.mpu.Layout.view logic.mpu.Region.disabled
  simp only [lift, bind_ok]
  step as ⟨ pl, hpl ⟩
  have hw := hwc hlen
  rw [← hpl] at hw
  step as ⟨ r1, hr1 ⟩
  step as ⟨ r2, hr2 ⟩
  step as ⟨ r3, hr3 ⟩
  refine WP.spec_bind (view_loop_total pl.windows pl.windows_ro _ (by scalar_tac) r3 0#usize
    (by scalar_tac)) (fun r _ => ?_)
  exact (WP.spec_ok _).mpr trivial

end P18
