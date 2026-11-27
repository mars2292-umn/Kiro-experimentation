import Sched.BoundAll
open Aeneas Aeneas.Std Result

/-! # `init` returns (2026-11-20)

`Bound::init` builds the Kernel state (`Kernel::init`) and every Partition's
MPU view (`Layout::view`). Both return once `validate` accepted the tables:
the view writes one region per live window, at most 5 of them after the
three fixed regions, inside the 8 regions of a view.

`Kernel::init` ends in `no_history`, the empty ghost history. The build
without Verus turns its body into a closure, which the translation keeps
opaque, so that it returns is the one thing assumed here (`NoHist`). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The ghost history's constructor returns. -/
def NoHist (NT NR NP NL NS NE : Usize) : Prop :=
  logic.kernel.Kernel.no_history NT NR NP NL NS NE ⦃ _ => True ⦄

@[step] theorem view_loop_t (a : Array logic.mpu.Interval 5#usize) (a1 : Array Bool 5#usize)
    (regions : Array logic.mpu.Region 8#usize) (count w : Usize) (hc : count.val ≤ 5) :
    logic.mpu.Layout.view_loop a a1 regions count w ⦃ _ => True ⦄ := by
  unfold logic.mpu.Layout.view_loop
  apply loop.spec_decr_nat (fun (x : Array logic.mpu.Region 8#usize × Usize) => count.val - x.2.val)
    (fun _ => True)
  · intro x _
    obtain ⟨r1, w1⟩ := x
    simp only
    unfold logic.mpu.Layout.view_loop.body
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

/-- **`Layout::view` returns** for a Partition of the layout with at most 5
windows. -/
@[step] theorem layout_view_t (l : logic.mpu.Layout NP) (p : Usize) (hp : p.val < NP.val)
    (hl : LOk l) : logic.mpu.Layout.view l p ⦃ _ => True ⦄ := by
  unfold logic.mpu.Layout.view logic.mpu.Region.disabled
  step as ⟨pl, hpl⟩
  have hpk : PLOk pl := by subst hpl; exact hl.1 _ _ hp
  have hwc := hpk.2.2.1
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

@[step] theorem init_loop_t (l : logic.mpu.Layout NP) (a : Array logic.mpu.View NP) (p : Usize)
    (hl : LOk l) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init_loop l a p ⦃ _ => True ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init_loop
  apply loop.spec_decr_nat (fun (x : Array logic.mpu.View NP × Usize) => NP.val - x.2.val)
    (fun _ => True)
  · intro x _
    obtain ⟨a1, p1⟩ := x
    simp only
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init_loop.body
    ctac
    all_goals first
      | (scalar_tac; done)
      | assumption
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

@[step] theorem timing_new_loop_t (cfg : logic.Config NT NR NP) (a : Array U64 NT) (t : Usize) :
    logic.time.Timing.new_loop cfg a t ⦃ _ => True ⦄ := by
  unfold logic.time.Timing.new_loop
  apply loop.spec_decr_nat (fun (x : Array U64 NT × Usize) => NT.val - x.2.val) (fun _ => True)
  · intro x _
    obtain ⟨a1, t1⟩ := x
    simp only
    unfold logic.time.Timing.new_loop.body
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

/-- **`Kernel::init` returns**, from any state, given that its ghost
history's constructor does. (The name is of `Kernel::new`, whose body
`init` has had since 2026-11-27.) -/
theorem kernel_new_t (hnh : NoHist NT NR NP NL NS NE) (kold : logic.kernel.Kernel NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) :
    logic.kernel.Kernel.init kold cfg ⦃ _ => True ⦄ := by
  unfold logic.kernel.Kernel.init logic.sched.Sched.new logic.time.Timing.new logic.time.Slots.new
    logic.budget.Budget.new logic.log.Log.new
  try simp only [bind_ok]
  refine WP.spec_bind (Pₘ := fun _ => True) ?_ (fun t _ => ?_)
  · refine WP.spec_bind (timing_new_loop_t cfg _ 0#usize) (fun a9 _ => ?_)
    exact (WP.spec_ok _).mpr trivial
  · refine WP.spec_bind hnh (fun g _ => ?_)
    exact (WP.spec_ok _).mpr trivial

/-- **`init` returns once `validate` accepted the tables.** -/
theorem binit_returns (hnh : NoHist NT NR NP NL NS NE)
    (b : arch.bound.Bound NT NR NP NL NS NE)
    (hv : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.validate b = ok true) :
    ∃ b', arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init b = ok b' := by
  obtain ⟨r, hr, hp⟩ := spec_returns (bvalidate_t b)
  rw [hv] at hr
  have hr := ok_inj hr
  subst hr
  obtain ⟨hlo, _, _⟩ := hp rfl
  have ht : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init b ⦃ _ => True ⦄ := by
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init
    refine WP.spec_bind (kernel_new_t hnh b.kernel b.cfg) (fun k _ => ?_)
    refine WP.spec_bind (init_loop_t b.layout b.views 0#usize hlo) (fun a _ => ?_)
    exact (WP.spec_ok _).mpr trivial
  obtain ⟨b', hb', _⟩ := spec_returns ht
  exact ⟨b', hb'⟩

/-- `KOpsT` for the Kernel that is linked. -/
theorem bound_kopst_nh {ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)}
    (hb : IsBoundT ki) (_hnh : NoHist NT NR NP NL NS NE) : KOpsT ki BX NT.val NP.val :=
  bound_kopst hb

/-- `KInitT` for the Kernel that is linked, with `init`'s return proven. -/
theorem bound_kinit_nh {ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)}
    (hb : IsBoundT ki) (hnh : NoHist NT NR NP NL NS NE) : KInitT ki BX :=
  bound_kinit hb (binit_returns hnh)

end P18
