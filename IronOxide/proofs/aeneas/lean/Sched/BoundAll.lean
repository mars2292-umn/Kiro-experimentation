import Sched.CrcTotal
open Aeneas Aeneas.Std Result

/-! # Every method of the Kernel behind the trait returns (2026-11-20)

`Sched/TotalBound.lean` proves that the methods of `impl KernelOps for
Bound` that change the Kernel return and keep `BT`. Here the rest: the
methods that read the layout (`message_permitted`, `dma_permitted`,
`in_ro_window`, `partition_view`), `basepri_now`, `next_compare`,
`source_masked`, the checksums and `validate`. They need more than `BT`:
the bounds of an accepted layout (`LOk`), the System_Ceiling within the
Priority levels, and at least one Partition. `BX` is `BT` with those, every
method keeps it, and `bound_kopst` gives the assumption class of
`Sched/CoreTotal.lean` (`KOpsT`) for the Kernel that is linked: with it,
every entry core returns on the real Kernel.

What stays assumed: that `init` returns once `validate` accepted the
tables (`Kernel::init` ends in the ghost `no_history`, opaque here, and the
view computation `Layout::view` is not proven to return), and the classes
about the hardware, the tables and the ghost helpers (`HwT`, `TabT`,
`GhostT`). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! ## The layout's reads -/

@[step] theorem message_permitted_exec_t (pl : logic.mpu.PartitionLayout) (ptr len : U32)
    (h : pl.ram.size_log2.val ≤ 31) :
    logic.mpu.PartitionLayout.message_permitted_exec pl ptr len ⦃ _ => True ⦄ := by
  unfold logic.mpu.PartitionLayout.message_permitted_exec
  ctac
  lcl

@[step] theorem dma_permitted_exec_loop_t (i : U8) (p l sub : U64) (s : U8) (ok1 : Bool) (lo : U64)
    (hp : p.val < 4294967296) (hl : l.val < 4294967296) (hsub : sub.val ≤ 4294967296)
    (hs : s.val ≤ 8) (hlo : lo.val ≤ 8589934592 + s.val * 4294967296) :
    logic.mpu.PartitionLayout.dma_permitted_exec_loop i p l sub s ok1 lo ⦃ _ => True ⦄ := by
  unfold logic.mpu.PartitionLayout.dma_permitted_exec_loop
  apply loop.spec_decr_nat (fun (x : U8 × Bool × U64) => 8 - x.1.val)
    (fun x => x.1.val ≤ 8 ∧ x.2.2.val ≤ 8589934592 + x.1.val * 4294967296)
  · intro x hx
    obtain ⟨s1, o1, lo1⟩ := x
    simp only at hx ⊢
    unfold logic.mpu.PartitionLayout.dma_permitted_exec_loop.body
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; scalar_tac; done)
      | (refine ⟨?_, ?_, ?_⟩ <;> scalar_tac)
  · exact ⟨hs, hlo⟩

@[step] theorem dma_permitted_exec_t (pl : logic.mpu.PartitionLayout) (ptr len : U32)
    (h : pl.ram.size_log2.val ≤ 31) :
    logic.mpu.PartitionLayout.dma_permitted_exec pl ptr len ⦃ _ => True ⦄ := by
  unfold logic.mpu.PartitionLayout.dma_permitted_exec
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

@[step] theorem in_ro_window_exec_loop_t (a : Array logic.mpu.Interval 5#usize)
    (a1 : Array Bool 5#usize) (a2 : U32) (count w : Usize) (hc : count.val ≤ 5)
    (hw : SzOk a count.val) :
    logic.mpu.PartitionLayout.in_ro_window_exec_loop a a1 a2 count w ⦃ _ => True ⦄ := by
  unfold logic.mpu.PartitionLayout.in_ro_window_exec_loop
  apply loop.spec_decr_nat (fun (x : Usize) => count.val - x.val) (fun _ => True)
  · intro x _
    unfold logic.mpu.PartitionLayout.in_ro_window_exec_loop.body
    have hwk := hw x.val
    ctac
    lcl
  · trivial

@[step] theorem in_ro_window_exec_t (pl : logic.mpu.PartitionLayout) (a : U32) (h : PLOk pl) :
    logic.mpu.PartitionLayout.in_ro_window_exec pl a ⦃ _ => True ⦄ := by
  unfold logic.mpu.PartitionLayout.in_ro_window_exec
  step as ⟨count, hcount⟩
  have hcv : count.val = pl.window_count.val := by scalar_tac
  exact in_ro_window_exec_loop_t _ _ a count 0#usize (by have := h.2.2.1; omega)
    (by rw [hcv]; exact h.2.2.2)

/-! ## The invariant -/

/-- `BT`, the bounds of the layout `validate` accepted, the Resources'
Ceilings and the System_Ceiling within the Priority levels, and at least one
Partition. -/
structure BX (b : arch.bound.Bound NT NR NP NL NS NE) : Prop where
  bt : BT b
  lo : LOk b.layout
  rc : RC b.cfg
  cl : b.kernel.sched.ceiling.val ≤ 8
  np : 0 < NP.val

/-- What a method that changes the Kernel leaves of the rest. -/
structure KP (b b' : arch.bound.Bound NT NR NP NL NS NE) : Prop where
  lay : b'.layout = b.layout
  cfg : b'.cfg = b.cfg
  cl : CLs b.kernel.sched b'.kernel.sched

theorem BX.keep {b b' : arch.bound.Bound NT NR NP NL NS NE} (hb : BX b) (hbt : BT b') (hk : KP b b') : BX b' :=
  ⟨hbt, by rw [hk.lay]; exact hb.lo, by rw [hk.cfg]; exact hb.rc, hk.cl hb.cl, hb.np⟩

theorem bx_spec {α} {x : Result α} (π : α → arch.bound.Bound NT NR NP NL NS NE) {b : arch.bound.Bound NT NR NP NL NS NE} (hb : BX b)
    (ht : x ⦃ r => BT (π r) ⦄) (hk : ∀ r, x = ok r → KP b (π r)) : x ⦃ r => BX (π r) ⦄ := by
  obtain ⟨r, hr, hp⟩ := spec_returns ht
  rw [hr]
  exact (WP.spec_ok _).mpr (hb.keep hp (hk r hr))

macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_of_eq (kernel_start_sched (by assumption)) ?_)
macro_rules
  | `(tactic| cl_step) => `(tactic| refine cl_of_eq (boot_info_sched (by assumption)) ?_)

/-! ## The methods that change the Kernel keep `BX` -/

set_option maxHeartbeats 8000000 in
theorem bstart_kp {b b' : arch.bound.Bound NT NR NP NL NS NE}  (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start b  = ok b') : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem bstart_x (b : arch.bound.Bound NT NR NP NL NS NE)  (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start b  ⦃ r => BX r ⦄ :=
  bx_spec (fun x => x) hb (bstart_total b hb.bt) (fun _ h => bstart_kp hb.rc h)

set_option maxHeartbeats 8000000 in
theorem bdispatch_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {t : U8} {pc : U32} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch b t pc = ok b') : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem bdispatch_x (b : arch.bound.Bound NT NR NP NL NS NE) (t : U8) (pc : U32) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch b t pc ⦃ r => BX r ⦄ :=
  bx_spec (fun x => x) hb (bdispatch_total b t pc hb.bt) (fun _ h => bdispatch_kp hb.rc h)

set_option maxHeartbeats 8000000 in
theorem bjob_return_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {ec : U32} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return b ec = ok b') : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem bjob_return_x (b : arch.bound.Bound NT NR NP NL NS NE) (ec : U32) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return b ec ⦃ r => BX r ⦄ :=
  bx_spec (fun x => x) hb (bjob_return_total b ec hb.bt) (fun _ h => bjob_return_kp hb.rc h)

set_option maxHeartbeats 8000000 in
theorem bsvc_lock_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {r : U8} {v : _} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock b r = ok (v, b')) : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem bsvc_lock_x (b : arch.bound.Bound NT NR NP NL NS NE) (r : U8) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock b r ⦃ r => BX r.2 ⦄ :=
  bx_spec Prod.snd hb (bsvc_lock_total b r hb.bt) (fun r h => bsvc_lock_kp (v := r.1) (b' := r.2) hb.rc h)

set_option maxHeartbeats 8000000 in
theorem bsvc_unlock_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {r : U8} {v : _} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock b r = ok (v, b')) : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem bsvc_unlock_x (b : arch.bound.Bound NT NR NP NL NS NE) (r : U8) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock b r ⦃ r => BX r.2 ⦄ :=
  bx_spec Prod.snd hb (bsvc_unlock_total b r hb.bt) (fun r h => bsvc_unlock_kp (v := r.1) (b' := r.2) hb.rc h)

set_option maxHeartbeats 8000000 in
theorem bsettle_kp {b b' : arch.bound.Bound NT NR NP NL NS NE}  {v : _} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle b  = ok (v, b')) : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem bsettle_x (b : arch.bound.Bound NT NR NP NL NS NE)  (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle b  ⦃ r => BX r.2 ⦄ :=
  bx_spec Prod.snd hb (bsettle_total b hb.bt) (fun r h => bsettle_kp (v := r.1) (b' := r.2) hb.rc h)

set_option maxHeartbeats 8000000 in
theorem bintake_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {t sp : U8} {now : U64} {off : U8} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.intake b t sp now off = ok b') : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.intake at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem bintake_x (b : arch.bound.Bound NT NR NP NL NS NE) (t sp : U8) (now : U64) (off : U8) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.intake b t sp now off ⦃ r => BX r ⦄ :=
  bx_spec (fun x => x) hb (bintake_total b t sp off now hb.bt) (fun _ h => bintake_kp hb.rc h)

set_option maxHeartbeats 8000000 in
theorem btimer_event_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {now : U64} {rc : U32} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event b now rc = ok b') : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem btimer_event_x (b : arch.bound.Bound NT NR NP NL NS NE) (now : U64) (rc : U32) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event b now rc ⦃ r => BX r ⦄ :=
  bx_spec (fun x => x) hb (btimer_event_total b now rc hb.bt) (fun _ h => btimer_event_kp hb.rc h)

set_option maxHeartbeats 8000000 in
theorem bfault_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {kind : U8} {kn : Bool} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault b kind kn = ok b') : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem bfault_x (b : arch.bound.Bound NT NR NP NL NS NE) (kind : U8) (kn : Bool) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault b kind kn ⦃ r => BX r ⦄ :=
  bx_spec (fun x => x) hb (bfault_total b kind kn hb.bt) (fun _ h => bfault_kp hb.rc h)

set_option maxHeartbeats 8000000 in
theorem bfault_of_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {kind t : U8} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of b kind t = ok b') : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem bfault_of_x (b : arch.bound.Bound NT NR NP NL NS NE) (kind t : U8) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of b kind t ⦃ r => BX r ⦄ :=
  bx_spec (fun x => x) hb (bfault_of_total b kind t hb.bt) (fun _ h => bfault_of_kp hb.rc h)

set_option maxHeartbeats 8000000 in
theorem brecord_event_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {kind t : U8} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event b kind t = ok b') : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem brecord_event_x (b : arch.bound.Bound NT NR NP NL NS NE) (kind t : U8) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event b kind t ⦃ r => BX r ⦄ :=
  bx_spec (fun x => x) hb (brecord_event_total b kind t hb.bt) (fun _ h => brecord_event_kp hb.rc h)

set_option maxHeartbeats 8000000 in
theorem btake_reinit_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {p : U8} {v : _} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit b p = ok (v, b')) : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem btake_reinit_x (b : arch.bound.Bound NT NR NP NL NS NE) (p : U8) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit b p ⦃ r => BX r.2 ⦄ :=
  bx_spec Prod.snd hb (btake_reinit_total b p hb.bt) (fun r h => btake_reinit_kp (v := r.1) (b' := r.2) hb.rc h)

set_option maxHeartbeats 8000000 in
theorem btake_stop_dma_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {p : U8} {v : _} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma b p = ok (v, b')) : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem btake_stop_dma_x (b : arch.bound.Bound NT NR NP NL NS NE) (p : U8) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma b p ⦃ r => BX r.2 ⦄ :=
  bx_spec Prod.snd hb (btake_stop_dma_total b p hb.bt) (fun r h => btake_stop_dma_kp (v := r.1) (b' := r.2) hb.rc h)

set_option maxHeartbeats 8000000 in
theorem bboot_failed_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {f : U32} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_failed b f = ok b') : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_failed at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem bboot_failed_x (b : arch.bound.Bound NT NR NP NL NS NE) (f : U32) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_failed b f ⦃ r => BX r ⦄ :=
  bx_spec (fun x => x) hb (bboot_failed_total b f hb.bt) (fun _ h => bboot_failed_kp hb.rc h)

set_option maxHeartbeats 8000000 in
theorem bboot_info_kp {b b' : arch.bound.Bound NT NR NP NL NS NE} {x y : Bool} (hrc : RC b.cfg)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_info b x y = ok b') : KP b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_info at h
  peel h
  all_goals (refine ⟨rfl, rfl, ?_⟩; cl_close)

theorem bboot_info_x (b : arch.bound.Bound NT NR NP NL NS NE) (x y : Bool) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_info b x y ⦃ r => BX r ⦄ :=
  bx_spec (fun x => x) hb (bboot_info_total b x y hb.bt) (fun _ h => bboot_info_kp hb.rc h)

/-! ## The methods that read -/

variable (b : arch.bound.Bound NT NR NP NL NS NE)

theorem bjob_info_x (t : U8) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_info b t ⦃ r => r.priority.val < 8 ∧ r.partition.val < NP.val ⦄ :=
  spec_of_total (bjob_info_total b t) (fun r hr => by
    rw [(bound_job_info hr).1, (bound_job_info hr).2]
    have hlen : b.cfg.tasks.val.length = NT.val := b.cfg.tasks.property
    by_cases hlt : t.val < NT.val
    · refine ⟨(hb.bt.ok.2.1 t.val hlt).2.1, ?_⟩
      have hp := hb.bt.ct.ci.part t.val (by omega)
      simp only [kvB, partOf, List.getElem?_eq_getElem (show t.val < b.cfg.tasks.val.length by omega)]
      exact hp
    · have hv : b.cfg.tasks.val[t.val]? = none := List.getElem?_eq_none (by omega)
      have := hb.np
      simp [kvB, prio, partOf, hv]
      omega)

/-- A Task `Kernel::settle` selects is one of the configuration's. -/
theorem sel_lt {s : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} (hNT : NT.val ≤ 256)
    {o : Option U8} {t : U8} (h : logic.sched.Sched.select s cfg = ok o) (ho : o = some t) :
    t.val < NT.val := by
  subst ho
  rcases select_cases hNT h with ⟨h0, _⟩ | ⟨u, hu, _, _, ⟨hs, _⟩ | ⟨hn, _⟩⟩
  · cases h0
  · cases hs; exact hu
  · cases hn

theorem settle_sel_lt {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP}
    (hNT : NT.val ≤ 256) {sel : Option U8} {q : Bool}
    (h : logic.kernel.Kernel.settle k cfg = ok ((sel, q), k')) :
    ∀ t, sel = some t → t.val < NT.val := by
  unfold logic.kernel.Kernel.settle at h
  peel h
  all_goals
    intro t ht
    first
      | (cases ht; done)
      | exact sel_lt hNT (by assumption) ht

theorem bsettle_sel (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle b ⦃ r => BX r.2 ∧ ∀ t, r.1.1 = some t → t.val < NT.val ⦄ := by
  obtain ⟨r, hr, hp⟩ := spec_returns (bsettle_x b hb)
  rw [hr]
  refine (WP.spec_ok _).mpr ⟨hp, ?_⟩
  obtain ⟨⟨sel, q⟩, b'⟩ := r
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle at hr
  simp only [bind_eq_ok] at hr
  obtain ⟨⟨x, k1⟩, hk1, hr⟩ := hr
  have hr := ok_inj hr
  have e : x = (sel, q) := congrArg Prod.fst hr
  subst e
  exact settle_sel_lt hb.bt.ct.ci.nt hk1

theorem btop_priority_x (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.top_priority b ⦃ r => r.val ≤ 8 ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.top_priority
  split
  · exact (WP.spec_ok _).mpr (by decide)
  · rename_i hne
    have hs := hb.bt.kt.ki.s
    have hlen : b.kernel.sched.stack.val.length = NT.val := b.kernel.sched.stack.property
    have hd := hs.1
    step as ⟨i, hi⟩
    step as ⟨t, ht⟩
    have h1 := hs.2.1 i.val (by scalar_tac)
    unfold stk at h1
    have e : b.kernel.sched.stack.val[i.val]! = b.kernel.sched.stack.val[i.val] :=
      getElem!_pos _ _ (by scalar_tac)
    rw [e, ← ht] at h1
    have hlt : (UScalar.cast .Usize t).val < NT.val := by scalar_tac
    have hlent : b.cfg.tasks.val.length = NT.val := b.cfg.tasks.property
    step as ⟨i2, hi2⟩
    step as ⟨tc, htc⟩
    have hsome := some_of htc
    have hp := (hb.bt.ok.2.1 i2.val (by scalar_tac)).2.1
    simp only [kvB, prio, hsome] at hp
    omega

theorem bbasepri_now_x (hb : BX b) : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.basepri_now b ⦃ _ => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.basepri_now arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.ceiling
  refine WP.spec_bind (btop_priority_x b hb) (fun i hi => ?_)
  simp only [bind_ok]
  exact basepri_for_total i _ hi hb.cl

theorem bnext_compare_x : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.next_compare b ⦃ _ => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.next_compare
  refine WP.spec_bind (next_spec b.kernel.slots) (fun o ho => ?_)
  cases o with
  | none => exact (WP.spec_ok _).mpr trivial
  | some i =>
    have hi := ho.1
    have hlen : b.kernel.slots.slots.val.length = NS.val := b.kernel.slots.slots.property
    simp only
    step*

theorem bmessage_permitted_x (p : U8) (ptr len : U32) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.message_permitted b p ptr len ⦃ _ => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.message_permitted
  simp only [lift, bind_ok]
  split
  · exact (WP.spec_ok _).mpr trivial
  · rename_i hge
    step as ⟨pl, hpl⟩
    have hpk : PLOk pl := by subst hpl; exact hb.lo.1 _ _ (by scalar_tac)
    exact message_permitted_exec_t pl ptr len hpk.1

theorem bdma_permitted_x (p : U8) (ptr len : U32) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dma_permitted b p ptr len ⦃ _ => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dma_permitted
  simp only [lift, bind_ok]
  split
  · exact (WP.spec_ok _).mpr trivial
  · rename_i hge
    step as ⟨pl, hpl⟩
    have hpk : PLOk pl := by subst hpl; exact hb.lo.1 _ _ (by scalar_tac)
    exact dma_permitted_exec_t pl ptr len hpk.1

theorem bin_ro_window_x (p : U8) (a : U32) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.in_ro_window b p a ⦃ _ => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.in_ro_window
  simp only [lift, bind_ok]
  split
  · exact (WP.spec_ok _).mpr trivial
  · rename_i hge
    step as ⟨pl, hpl⟩
    have hpk : PLOk pl := by subst hpl; exact hb.lo.1 _ _ (by scalar_tac)
    exact in_ro_window_exec_t pl a hpk

theorem bpartition_view_x (p : U8) (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.partition_view b p ⦃ _ => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.partition_view
  have hnp := hb.np
  have hlen : b.views.val.length = NP.val := b.views.property
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

theorem bdepth_x  : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.depth b  ⦃ r => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.depth
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

theorem bstack_at_x (i : Usize) : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.stack_at b i ⦃ r => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.stack_at
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

theorem bis_killed_x (t : U8) : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.is_killed b t ⦃ r => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.is_killed
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

theorem bsource_masked_x (t : U8) : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.source_masked b t ⦃ r => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.source_masked
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

theorem bsafe_state_x  : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.safe_state b  ⦃ r => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.safe_state
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

theorem btask_count_x  : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.task_count b  ⦃ r => r.val ≤ NT.val ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.task_count
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

theorem bpartition_count_x  : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.partition_count b  ⦃ r => r.val ≤ NP.val ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.partition_count
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

theorem bconfig_checksum_x  : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.config_checksum b  ⦃ r => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.config_checksum
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

theorem blayout_checksum_x  : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.layout_checksum b  ⦃ r => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.layout_checksum
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

/-! ## `validate` and `init` -/

/-- **`validate` returns for every table set**, and the tables it accepts
have the bounds `BX` asks. -/
theorem bvalidate_t : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.validate b ⦃ r =>
    r = true → LOk b.layout ∧ RC b.cfg ∧ 0 < NP.val ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.validate
  refine WP.spec_bind (config_validate_t b.cfg) (fun c hc => ?_)
  split
  · rename_i hct
    obtain ⟨h1, h2⟩ := hc hct
    simp only [lift, bind_ok]
    repeat' split
    all_goals first
      | exact (WP.spec_ok _).mpr (fun h => Bool.noConfusion h)
      | exact WP.spec_mono (layout_validate_t b.layout) (fun r hr hrt => ⟨hr hrt, rc_of_rck h1, h2⟩)
  · exact (WP.spec_ok _).mpr (fun h => Bool.noConfusion h)

/-- **After a boot that validated the tables, `BX` holds.** -/
theorem bound_init_bx {b b' : arch.bound.Bound NT NR NP NL NS NE} (hv : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.validate b = ok true)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init b = ok b') : BX b' := by
  have hbt := bound_init_bt hv h
  obtain ⟨r, hr, hp⟩ := spec_returns (bvalidate_t b)
  rw [hv] at hr
  have hr := ok_inj hr
  subst hr
  obtain ⟨hlo, hrc, hnp⟩ := hp rfl
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init at h
  simp only [bind_eq_ok] at h
  obtain ⟨k, hk, a, ha, h⟩ := h
  have h := ok_inj h
  subst h
  refine ⟨hbt, hlo, hrc, ?_, hnp⟩
  unfold logic.kernel.Kernel.init logic.sched.Sched.new logic.time.Slots.new logic.log.Log.new at hk
  simp only [bind_eq_ok, lift, bind_ok] at hk
  obtain ⟨tm, htm, bg, hbg, g, hg, hk⟩ := hk
  have hk := ok_inj hk
  subst hk
  show logic.IDLE_CEILING.val ≤ 8
  rw [idle_ceiling]
  decide

/-! ## The assumption class of the entry cores, for the Kernel that is linked -/

/-- The trait instance is the extracted `impl KernelOps for Bound`, method
by method. -/
structure IsBoundT (ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)) : Prop where
  depth : ki.depth = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.depth
  top : ki.top = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.top
  job_info : ki.job_info = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_info
  stack_at : ki.stack_at = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.stack_at
  is_killed : ki.is_killed = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.is_killed
  eligible : ki.eligible = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.eligible
  basepri_now : ki.basepri_now = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.basepri_now
  next_compare : ki.next_compare = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.next_compare
  source_masked : ki.source_masked = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.source_masked
  partition_stopped : ki.partition_stopped = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.partition_stopped
  partition_view : ki.partition_view = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.partition_view
  dma_permitted : ki.dma_permitted = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dma_permitted
  in_ro_window : ki.in_ro_window = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.in_ro_window
  message_permitted : ki.message_permitted = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.message_permitted
  safe_state : ki.safe_state = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.safe_state
  task_count : ki.task_count = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.task_count
  partition_count : ki.partition_count = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.partition_count
  config_checksum : ki.config_checksum = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.config_checksum
  layout_checksum : ki.layout_checksum = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.layout_checksum
  start : ki.start = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start
  dispatch : ki.dispatch = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch
  job_return : ki.job_return = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return
  svc_lock : ki.svc_lock = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock
  svc_unlock : ki.svc_unlock = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock
  settle : ki.settle = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle
  intake : ki.intake = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.intake
  timer_event : ki.timer_event = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event
  fault : ki.fault = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault
  fault_of : ki.fault_of = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of
  record_event : ki.record_event = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event
  take_reinit : ki.take_reinit = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit
  take_stop_dma : ki.take_stop_dma = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma
  boot_failed : ki.boot_failed = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_failed
  boot_info : ki.boot_info = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_info
  validate : ki.validate = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.validate
  init : ki.init = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init

/-- **Every method the entry cores call returns on the Kernel that is
linked, from a state satisfying `BX`, and the ones that change it keep
`BX`.** Nothing is assumed (`init` is `bound_kinit`'s, since 2026-11-27). -/
theorem bound_kopst {ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)} (hb : IsBoundT ki) :
    KOpsT ki BX NT.val NP.val where
  depth := fun k _ => by rw [hb.depth]; exact bdepth_x k
  top := fun k hk => by rw [hb.top]; exact btop_total k hk.bt
  job_info := fun k t hk => by rw [hb.job_info]; exact bjob_info_x k t hk
  stack_at := fun k i _ => by rw [hb.stack_at]; exact bstack_at_x k i
  is_killed := fun k t _ => by rw [hb.is_killed]; exact bis_killed_x k t
  eligible := fun k t hk => by
    rw [hb.eligible]; exact WP.spec_mono (beligible_total k t hk.bt) (fun _ _ => trivial)
  basepri_now := fun k hk => by rw [hb.basepri_now]; exact bbasepri_now_x k hk
  next_compare := fun k _ => by rw [hb.next_compare]; exact bnext_compare_x k
  source_masked := fun k t _ => by rw [hb.source_masked]; exact bsource_masked_x k t
  partition_stopped := fun k p _ => by
    rw [hb.partition_stopped]; exact bpartition_stopped_total k p
  partition_view := fun k p hk => by rw [hb.partition_view]; exact bpartition_view_x k p hk
  dma_permitted := fun k p a c hk => by rw [hb.dma_permitted]; exact bdma_permitted_x k p a c hk
  in_ro_window := fun k p a hk => by rw [hb.in_ro_window]; exact bin_ro_window_x k p a hk
  message_permitted := fun k p a c hk => by
    rw [hb.message_permitted]; exact bmessage_permitted_x k p a c hk
  safe_state := fun k _ => by rw [hb.safe_state]; exact bsafe_state_x k
  task_count := fun k _ => by rw [hb.task_count]; exact btask_count_x k
  partition_count := fun k _ => by rw [hb.partition_count]; exact bpartition_count_x k
  config_checksum := fun k _ => by rw [hb.config_checksum]; exact bconfig_checksum_x k
  layout_checksum := fun k _ => by rw [hb.layout_checksum]; exact blayout_checksum_x k
  start := fun k hk => by rw [hb.start]; exact bstart_x k hk
  dispatch := fun k t pc hk => by rw [hb.dispatch]; exact bdispatch_x k t pc hk
  job_return := fun k e hk => by rw [hb.job_return]; exact bjob_return_x k e hk
  svc_lock := fun k x hk => by rw [hb.svc_lock]; exact bsvc_lock_x k x hk
  svc_unlock := fun k x hk => by rw [hb.svc_unlock]; exact bsvc_unlock_x k x hk
  settle := fun k hk => by rw [hb.settle]; exact bsettle_sel k hk
  intake := fun k t p now o hk => by rw [hb.intake]; exact bintake_x k t p now o hk
  timer_event := fun k now rc hk => by rw [hb.timer_event]; exact btimer_event_x k now rc hk
  fault := fun k kind kn hk => by rw [hb.fault]; exact bfault_x k kind kn hk
  fault_of := fun k kind t hk => by rw [hb.fault_of]; exact bfault_of_x k kind t hk
  record_event := fun k kind t hk => by rw [hb.record_event]; exact brecord_event_x k kind t hk
  take_reinit := fun k p hk => by rw [hb.take_reinit]; exact btake_reinit_x k p hk
  take_stop_dma := fun k p hk => by rw [hb.take_stop_dma]; exact btake_stop_dma_x k p hk
  boot_failed := fun k f hk => by rw [hb.boot_failed]; exact bboot_failed_x k f hk
  boot_info := fun k x y hk => by rw [hb.boot_info]; exact bboot_info_x k x y hk
  validate := fun k => by
    rw [hb.validate]
    obtain ⟨r, hr, _⟩ := spec_returns (bvalidate_t k)
    rw [hr]
    exact (WP.spec_ok _).mpr rfl

/-- **`init` establishes `BX` on the Kernel that is linked.** The one
assumption is `hinit`: `init` returns once `validate` accepted the tables. -/
theorem bound_kinit {ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)} (hb : IsBoundT ki)
    (hinit : ∀ b : arch.bound.Bound NT NR NP NL NS NE, arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.validate b = ok true → ∃ b', arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init b = ok b') :
    KInitT ki BX where
  init := fun k hv => by
    rw [hb.validate] at hv
    rw [hb.init]
    obtain ⟨b', h'⟩ := hinit k hv
    rw [h']
    exact (WP.spec_ok _).mpr (bound_init_bx hv h')

end P18
