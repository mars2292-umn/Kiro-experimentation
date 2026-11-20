import Sched.Boot
open Aeneas Aeneas.Std Result

/-! # The Kernel behind the trait returns (2026-11-20)

Property 21 at the boundary Kernel_Arch calls through: the methods of
`impl KernelOps for Bound`, as extracted. Each one checks its arguments
before it calls the Kernel's logic (a Task or Partition identifier in
range, a Job in progress, an eligible Task), so that from a `Bound`
satisfying `BT` (the Kernel state within its bounds, `KT`; the
configuration's bounds, `CT`; the stack in rising Priority, `okB`) every
method returns, for any argument, and leaves a `Bound` satisfying `BT`.

Not here: `basepri_now` (it needs the System_Ceiling within the Priority
levels, which `KT` does not carry), `message_permitted`, `init` (it calls
`Kernel::new`, whose ghost history the translation keeps opaque) and
`validate` (`Layout::validate` is not translated). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The invariant of the Kernel behind the trait, as far as returning
needs it. -/
structure BT (b : arch.bound.Bound NT NR NP NL NS NE) : Prop where
  kt : KT b.kernel
  ct : CT b.cfg NS
  sc : 2 ≤ b.cfg.slot_count.val ∧ b.cfg.slot_count.val ≤ NS.val
  ok : okB b

/-- **After a boot that validated the tables, `BT` holds.** -/
theorem bound_init_bt {b b' : arch.bound.Bound NT NR NP NL NS NE}
    (hv : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.validate b = ok true)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init b = ok b') : BT b' := by
  obtain ⟨hw, hns, hne⟩ := bound_validate_ok hv
  obtain ⟨a, _, c, d⟩ := bound_init_ok hv h
  have hcfg : b'.cfg = b.cfg := by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init at h
    simp only [bind_eq_ok] at h
    obtain ⟨k, hk, a, ha, h⟩ := h
    have h := ok_inj h
    rw [← h]
  refine ⟨c, d, ?_, a⟩
  rw [hcfg]
  have := hw.sc
  omega

/-- A method that keeps `KT` and the configuration, and (when it returns)
`okB`, keeps `BT`. -/
theorem bt_spec {α} {x : Result α} (π : α → arch.bound.Bound NT NR NP NL NS NE)
    {b : arch.bound.Bound NT NR NP NL NS NE} (hb : BT b)
    (h : x ⦃ r => KT (π r).kernel ∧ (π r).cfg = b.cfg ⦄)
    (hok : ∀ r, x = ok r → okB (π r)) : x ⦃ r => BT (π r) ⦄ := by
  obtain ⟨r, hr, hk, hc⟩ := spec_returns h
  rw [hr]
  exact (WP.spec_ok _).mpr ⟨hk, by rw [hc]; exact hb.ct, by rw [hc]; exact hb.sc, hok r hr⟩

variable (b : arch.bound.Bound NT NR NP NL NS NE)

theorem bfault_total (kind : U8) (kn : Bool) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault b kind kn ⦃ b' => BT b' ⦄ :=
  bt_spec (fun x => x) hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault
    refine WP.spec_bind (fault_kt b.kernel b.cfg kind kn hb.kt hb.ct) (fun k hk => ?_)
    exact (WP.spec_ok _).mpr ⟨hk, rfl⟩) (fun _ h => ((bound_fault_bp h).kr hb.ok).1)

theorem bfault_of_total (kind t : U8) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of b kind t ⦃ b' => BT b' ⦄ :=
  bt_spec (fun x => x) hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of
    simp only [lift, bind_ok]
    split
    · refine WP.spec_bind (fault_of_kt b.kernel b.cfg kind t hb.kt hb.ct (by scalar_tac))
        (fun k hk => ?_)
      exact (WP.spec_ok _).mpr ⟨hk, rfl⟩
    · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩) (fun _ h => ((bound_fault_of_bp h).kr hb.ok).1)

theorem brecord_event_total (kind t : U8) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event b kind t ⦃ b' => BT b' ⦄ :=
  bt_spec (fun x => x) hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event
    simp only [lift, bind_ok]
    split
    · refine WP.spec_bind (record_event_kt b.kernel b.cfg kind t hb.kt (by scalar_tac))
        (fun k hk => ?_)
      exact (WP.spec_ok _).mpr ⟨hk, rfl⟩
    · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩)
    (fun _ h => ((bound_record_event_bs h).bp.kr hb.ok).1)

theorem bstart_total (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start b ⦃ b' => BT b' ⦄ :=
  bt_spec (fun x => x) hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start
    split
    · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩
    · refine WP.spec_bind (start_kt b.kernel b.cfg hb.kt hb.ct hb.sc) (fun k hk => ?_)
      exact (WP.spec_ok _).mpr ⟨hk, rfl⟩) (fun _ h => ((bound_start_bs h).bp.kr hb.ok).1)

theorem bjob_return_total (ec : U32) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return b ec ⦃ b' => BT b' ⦄ :=
  bt_spec (fun x => x) hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return
    split
    · refine WP.spec_bind (job_return_kt b.kernel b.cfg ec hb.kt hb.ct (by scalar_tac))
        (fun k hk => ?_)
      exact (WP.spec_ok _).mpr ⟨hk, rfl⟩
    · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩)
    (fun _ h => ((bound_job_return_bp h).1.kr hb.ok).1)

theorem bsvc_lock_total (r : U8) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock b r ⦃ x => BT x.2 ⦄ :=
  bt_spec Prod.snd hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock
    split
    · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩
    · simp only [lift, bind_ok]
      split
      · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩
      · refine WP.spec_bind (svc_lock_kt b.kernel b.cfg r hb.kt (by scalar_tac) (by scalar_tac))
          (fun x hx => ?_)
        exact (WP.spec_ok _).mpr ⟨hx, rfl⟩)
    (fun x h => ((bound_svc_lock_bs (x := x.1) (b' := x.2) h).bp.kr hb.ok).1)

theorem bsvc_unlock_total (r : U8) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock b r ⦃ x => BT x.2 ⦄ :=
  bt_spec Prod.snd hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock
    split
    · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩
    · refine WP.spec_bind (svc_unlock_kt b.kernel b.cfg r hb.kt (by scalar_tac)) (fun x hx => ?_)
      exact (WP.spec_ok _).mpr ⟨hx, rfl⟩)
    (fun x h => ((bound_svc_unlock_bs (x := x.1) (b' := x.2) h).bp.kr hb.ok).1)

theorem bsettle_total (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle b ⦃ x => BT x.2 ⦄ :=
  bt_spec Prod.snd hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle
    refine WP.spec_bind (settle_kt b.kernel b.cfg hb.kt hb.ct) (fun x hx => ?_)
    exact (WP.spec_ok _).mpr ⟨hx, rfl⟩)
    (fun x h => ((bound_settle_bs (x := x.1) (b' := x.2) h).bp.kr hb.ok).1)

theorem btimer_event_total (now : U64) (rc : U32) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event b now rc ⦃ b' => BT b' ⦄ :=
  bt_spec (fun x => x) hb (by
    have hod := hb.ct.od
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event
    refine WP.spec_bind (Pₘ := fun n : U64 => n.val ≤ 4611686018427387904) ?_ (fun n hn => ?_)
    · split
      · exact (WP.spec_ok _).mpr (by omega)
      · exact (WP.spec_ok _).mpr (by scalar_tac)
    · refine WP.spec_bind (timer_event_kt b.kernel b.cfg n rc hb.kt hb.ct hn) (fun k hk => ?_)
      exact (WP.spec_ok _).mpr ⟨hk, rfl⟩) (fun _ h => ((bound_timer_event_bp h).kr hb.ok).1)

theorem bintake_total (t sp off : U8) (now : U64) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.intake b t sp now off ⦃ b' => BT b' ⦄ :=
  bt_spec (fun x => x) hb (by
    have hod := hb.ct.od
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.intake
    simp only [lift, bind_ok]
    split
    · split
      · step as ⟨ tc, htc ⟩
        split
        · split
          · refine WP.spec_bind (Pₘ := fun o : U8 => o.val < NT.val) ?_ (fun o ho => ?_)
            · split
              · exact (WP.spec_ok _).mpr (by scalar_tac)
              · exact (WP.spec_ok _).mpr (by scalar_tac)
            · refine WP.spec_bind (intake_kt b.kernel b.cfg t sp o now hb.kt hb.ct (by scalar_tac)
                (by scalar_tac) ho (by scalar_tac)) (fun k hk => ?_)
              exact (WP.spec_ok _).mpr ⟨hk, rfl⟩
          · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩
        · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩
      · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩
    · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩) (fun _ h => ((bound_intake_bp h).kr hb.ok).1)

theorem btake_reinit_total (p : U8) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit b p ⦃ x => BT x.2 ⦄ :=
  bt_spec Prod.snd hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit
    simp only [lift, bind_ok]
    split
    · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩
    · have hlen : p.val < b.kernel.reinit_partition.val.length := by
        have := b.kernel.reinit_partition.property; scalar_tac
      step as ⟨ v, hv ⟩
      split
      · refine WP.spec_bind (ack_reinit_kt b.kernel b.cfg p hb.kt (by scalar_tac)) (fun k hk => ?_)
        exact (WP.spec_ok _).mpr ⟨hk, rfl⟩
      · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩)
    (fun x h => ((bound_take_reinit_bs (x := x.1) (b' := x.2) h).bp.kr hb.ok).1)

theorem btake_stop_dma_total (p : U8) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma b p ⦃ x => BT x.2 ⦄ :=
  bt_spec Prod.snd hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma
    simp only [lift, bind_ok]
    split
    · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩
    · have hlen : p.val < b.kernel.stop_dma.val.length := by
        have := b.kernel.stop_dma.property; scalar_tac
      step as ⟨ v, hv ⟩
      split
      · refine WP.spec_bind (ack_stop_dma_kt b.kernel b.cfg p hb.kt (by scalar_tac))
          (fun k hk => ?_)
        exact (WP.spec_ok _).mpr ⟨hk, rfl⟩
      · exact (WP.spec_ok _).mpr ⟨hb.kt, rfl⟩)
    (fun x h => ((bound_take_stop_dma_bs (x := x.1) (b' := x.2) h).bp.kr hb.ok).1)

theorem boot_check_failed_kt (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (f : U32) (hk : KT k) (hNT : NT.val ≤ 256) :
    logic.kernel.Kernel.boot_check_failed k cfg f ⦃ k' => KT k' ⦄ := by
  unfold logic.kernel.Kernel.boot_check_failed
  refine WP.spec_bind (push_kt k cfg _ _ _ _ _ hk) (fun k1 h1 => ?_)
  exact enter_safe_state_kt k1 cfg h1.1 hNT

theorem boot_info_kt (k : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (x y : Bool) (hk : KT k) :
    logic.kernel.Kernel.boot_info k cfg x y ⦃ k' => KT k' ⦄ := by
  unfold logic.kernel.Kernel.boot_info
  refine WP.spec_bind
    (Pₘ := fun k1 : logic.kernel.Kernel NT NR NP NL NS NE => KT k1) ?_ (fun k1 h1 => ?_)
  · split
    · exact WP.spec_mono (push_kt k cfg _ _ _ _ _ hk) (fun _ h => h.1)
    · exact (WP.spec_ok _).mpr hk
  · split
    · exact WP.spec_mono (push_kt k1 cfg _ _ _ _ _ h1) (fun _ h => h.1)
    · exact (WP.spec_ok _).mpr h1

theorem bboot_failed_total (f : U32) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_failed b f ⦃ b' => BT b' ⦄ :=
  bt_spec (fun x => x) hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_failed
    refine WP.spec_bind (boot_check_failed_kt b.kernel b.cfg f hb.kt hb.ct.ci.nt) (fun k hk => ?_)
    exact (WP.spec_ok _).mpr ⟨hk, rfl⟩) (fun _ h => ((bound_boot_failed_bs h).bp.kr hb.ok).1)

theorem bboot_info_total (x y : Bool) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_info b x y ⦃ b' => BT b' ⦄ :=
  bt_spec (fun x => x) hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_info
    refine WP.spec_bind (boot_info_kt b.kernel b.cfg x y hb.kt) (fun k hk => ?_)
    exact (WP.spec_ok _).mpr ⟨hk, rfl⟩) (fun _ h => ((bound_boot_info_bs h).bp.kr hb.ok).1)

/-! ## The methods that read -/

theorem btop_total (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.top b ⦃ _ => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.top
  split
  · exact (WP.spec_ok _).mpr trivial
  · step as ⟨ i, hi ⟩
    have hlen : i.val < b.kernel.sched.stack.val.length := by
      have := b.kernel.sched.stack.property; have := hb.kt.ki.s.1; scalar_tac
    step as ⟨ t, ht ⟩

theorem bjob_info_total (t : U8) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_info b t ⦃ _ => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_info
  simp only [lift, bind_ok]
  split
  · step as ⟨ tc, htc ⟩
  · exact (WP.spec_ok _).mpr trivial

theorem bselect_total (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.select b ⦃ _ => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.select
  exact kernel_select_total b.kernel b.cfg hb.kt.ki.s hb.ct.ci.nt

theorem bpartition_stopped_total (p : U8) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.partition_stopped b p ⦃ _ => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.partition_stopped
  simp only [lift, bind_ok]
  split
  · have hlen : p.val < b.kernel.stopped.val.length := by
      have := b.kernel.stopped.property; scalar_tac
    step as ⟨ v, hv ⟩
  · exact (WP.spec_ok _).mpr trivial

/-! ## The dispatch -/

/-- A Task that may start is not on the stack, so the stack has room for
it: its Priority is above that of the Job on top, and the Priorities on the
stack rise. -/
theorem elig_depth_lt {t : Nat} (hok : okB b) (hs : SInv b.kernel.sched)
    (he : Elig b.cfg b.kernel.sched t) : b.kernel.sched.depth.val < NT.val := by
  obtain ⟨_, _, h3, h4⟩ := hok
  obtain ⟨ht, _, _, _, hp⟩ := he
  have hd := hs.1
  by_contra hge
  have hdn : b.kernel.sched.depth.val = NT.val := by omega
  have hpos : 0 < NT.val := by omega
  -- the stack is an injection of `Fin NT` into itself, hence onto: `t` is on it
  let f : Fin NT.val → Fin NT.val := fun i =>
    ⟨stk b.kernel.sched i.val, h3 i.val (by simp only [kvB]; omega)⟩
  have hinj : Function.Injective f := by
    intro i j hij
    have hv : stk b.kernel.sched i.val = stk b.kernel.sched j.val := by
      have := hij
      simp only [f] at this
      exact Fin.mk.inj this
    by_contra hne
    rcases Nat.lt_or_gt_of_ne (fun e => hne (Fin.ext e)) with hlt | hlt
    · have := h4 i.val j.val hlt (by simp only [kvB]; omega)
      simp only [kvB] at this
      rw [hv] at this
      omega
    · have := h4 j.val i.val hlt (by simp only [kvB]; omega)
      simp only [kvB] at this
      rw [hv] at this
      omega
  obtain ⟨i, hi⟩ := Finite.surjective_of_injective hinj ⟨t, ht⟩
  have hiv : stk b.kernel.sched i.val = t := by
    have := hi
    simp only [f] at this
    exact Fin.mk.inj this
  rcases hp with h0 | hlt
  · omega
  · have hle : prio b.cfg (stk b.kernel.sched i.val) ≤
        prio b.cfg (stk b.kernel.sched (b.kernel.sched.depth.val - 1)) := by
      by_cases hie : i.val = b.kernel.sched.depth.val - 1
      · rw [hie]
      · have := h4 i.val (b.kernel.sched.depth.val - 1) (by omega) (by simp only [kvB]; omega)
        simp only [kvB] at this
        omega
    rw [hiv] at hle
    simp only [stk] at hle
    omega

set_option maxHeartbeats 1000000 in
/-- `eligible` returns, and answers R7.1. -/
theorem beligible_total (t : U8) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.eligible b t ⦃ e =>
      e = true → Elig b.cfg b.kernel.sched t.val ⦄ := by
  refine spec_of_total ?_ (fun e h => (eligible_ok (matches_self b) h).mp)
  have hs := hb.kt.ki.s
  obtain ⟨h1, h2, _, _⟩ := hs
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.eligible
  simp only [lift, bind_ok]
  split
  · exact (WP.spec_ok _).mpr trivial
  · rename_i hlt
    have ht : t.val < NT.val := by scalar_tac
    have hlr : t.val < b.kernel.sched.release.val.length := by
      have := b.kernel.sched.release.property; omega
    have hlj : t.val < b.kernel.sched.job.val.length := by
      have := b.kernel.sched.job.property; omega
    step as ⟨ tc, htc ⟩
    step as ⟨ i3, hi3 ⟩
    split
    · step as ⟨ i5, hi5 ⟩
      split
      · split
        · split
          · exact (WP.spec_ok _).mpr trivial
          · step as ⟨ i6, hi6 ⟩
            have hlen : i6.val < b.kernel.sched.stack.val.length := by
              have := b.kernel.sched.stack.property; scalar_tac
            step as ⟨ i7, hi7 ⟩
            have hst := h2 i6.val (by scalar_tac)
            simp only [stk] at hst
            rw [getE hlen hi7] at hst
            step as ⟨ tc1, htc1 ⟩
        · exact (WP.spec_ok _).mpr trivial
      · exact (WP.spec_ok _).mpr trivial
    · exact (WP.spec_ok _).mpr trivial

/-- **`dispatch` returns**: for any Task identifier; it starts the Task
only if R7.1 admits it. -/
theorem bdispatch_total (t : U8) (pc : U32) (hb : BT b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch b t pc ⦃ b' => BT b' ⦄ :=
  bt_spec (fun x => x) hb (by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch
    refine WP.spec_bind
      (Pₘ := fun k : logic.kernel.Kernel NT NR NP NL NS NE => KT k) ?_ (fun k hk => ?_)
    · split
      · refine WP.spec_bind (beligible_total b t hb) (fun e he => ?_)
        split
        · rename_i het
          have hel := he het
          exact dispatch_kt b.kernel b.cfg t pc hb.kt hb.ct hel.1
            (elig_depth_lt b hb.ok hb.kt.ki.s hel)
        · exact (WP.spec_ok _).mpr hb.kt
      · exact (WP.spec_ok _).mpr hb.kt
    · refine WP.spec_bind (settle_kt k b.cfg hk hb.ct) (fun x hx => ?_)
      exact (WP.spec_ok _).mpr ⟨hx, rfl⟩) (fun _ h => (bound_dispatch hb.ok h).1)


end P18
