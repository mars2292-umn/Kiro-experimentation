import Sched.LayoutIso
import Sched.LayoutReads
open Aeneas Aeneas.Std Result

/-! # Isolation of every layout the boot validated (2026-11-20)

The isolation theorems of `Sched/Isolation.lean`, with their hypothesis
`LayoutOk` replaced by what the boot checks: `Layout::validate` returned
`true` on the extracted code (`validate_layoutok`). These are the Lean
counterparts of `lemma_hw_partition_isolation`, `lemma_hw_write_isolation`,
`lemma_view_no_kernel_window` and `lemma_stack_overflow_faults`, on the
view the extracted `Layout::view` computes.

Then the EasyDMA request: `dma_request_core` accepts exactly when the
channel exists, a Job is executing, the channel belongs to that Job's
Partition and the Kernel permits the buffer (Verus: `dma_accepts`), and on
the Kernel that is linked "permits" is `DmaPermitted` of that Partition's
layout. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- No address is writable by one Partition and accessible to another. -/
theorem views_disjoint_boot {l : logic.mpu.Layout NP}
    (hv : logic.mpu.Layout.validate l = ok true) {p q : Usize} {vp vq : logic.mpu.View}
    (hp : logic.mpu.Layout.view l p = ok vp) (hq : logic.mpu.Layout.view l q = ok vq)
    (hne : p.val ≠ q.val) (a : Nat) : ¬ (writable vp a ∧ readable vq a) :=
  views_disjoint (validate_layoutok hv) hp hq hne a

/-- No view grants an address of the Kernel's flash or RAM. -/
theorem view_no_kernel_memory_boot {l : logic.mpu.Layout NP}
    (hv : logic.mpu.Layout.validate l = ok true) {p : Usize} {v : logic.mpu.View}
    (hp : logic.mpu.Layout.view l p = ok v) (a : Nat) :
    ¬ (readable v a ∧ (icontains l.kernel_flash a ∨ icontains l.kernel_ram a)) :=
  view_no_kernel_memory (validate_layoutok hv) hp a

/-- No view grants an address of a Kernel peripheral interval (R16.2). -/
theorem view_no_kernel_window_boot {l : logic.mpu.Layout NP}
    (hv : logic.mpu.Layout.validate l = ok true) {p : Usize} {v : logic.mpu.View}
    (hp : logic.mpu.Layout.view l p = ok v) (a : Nat) :
    ¬ (readable v a ∧ ∃ k, k < l.kernel_window_count.val ∧
      icontains (l.kernel_windows.val[k]!) a) :=
  view_no_kernel_window (validate_layoutok hv) hp a

/-- A read-only window of a Partition is writable in no view (DD-08). -/
theorem ro_window_not_writable_boot {l : logic.mpu.Layout NP}
    (hv : logic.mpu.Layout.validate l = ok true) {p q : Usize} {vq : logic.mpu.View}
    (hp : p.val < NP.val) (hq : logic.mpu.Layout.view l q = ok vq) {a w : Nat}
    (hw : w < (l.partitions.val[p.val]!).window_count.val)
    (hro : (l.partitions.val[p.val]!).windows_ro.val[w]! = true)
    (hin : icontains ((l.partitions.val[p.val]!).windows.val[w]!) a) : ¬ writable vq a :=
  ro_window_not_writable (validate_layoutok hv) hp hq hw hro hin

/-- The word below a Partition's RAM is not writable in its view (R16.5). -/
theorem stack_overflow_faults_boot {l : logic.mpu.Layout NP}
    (hv : logic.mpu.Layout.validate l = ok true) {p : Usize} {v : logic.mpu.View}
    (hp : logic.mpu.Layout.view l p = ok v) {a : Nat}
    (ha : a + 1 = (l.partitions.val[p.val]!).ram.base.val) : ¬ writable v a :=
  stack_overflow_faults (validate_layoutok hv) hp ha

/-! ## The EasyDMA request -/

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T}

/-- What `dma_request_core` tests (Verus: `dma_accepts`), in the calls it
makes. -/
def DmaAccepts (ki : arch.bound.KernelOps K) (ti : arch.cm4.entry.Tables T) (k : K) (tables : T)
    (i : Usize) (ptr len : U32) : Prop :=
  ∃ (n : Usize) (d : arch.cm4.plan.DmaCfg) (t : U8) (info : arch.bound.JobInfo),
    ti.dma_count tables = ok n ∧ i.val < n.val ∧ ti.dma tables i = ok d ∧
    ki.top k = ok (some t) ∧ ki.job_info k t = ok info ∧ info.partition = d.partition ∧
    ki.dma_permitted k d.partition ptr len = ok true

set_option maxHeartbeats 4000000 in
/-- **`dma_request_core` accepts exactly when `DmaAccepts` holds** (R17.1,
R17.2). -/
theorem dma_request_core_iff {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {ptr len : U32}
    {b : Bool}
    (h : arch.cm4.entry.dma_request_core ki ai ti k hw tables i ptr len = ok (b, k', hw')) :
    b = true ↔ DmaAccepts ki ti k tables i ptr len := by
  unfold arch.cm4.entry.dma_request_core at h
  unfold DmaAccepts
  peel h
  all_goals first
    | (simp_all; done)
    | (simp_all; scalar_tac)

/-- **On the Kernel that is linked, `dma_permitted` answers `DmaPermitted`
of the Partition's layout**: `true` exactly for a Partition of the layout
and a buffer in its RAM above the stack that touches only enabled
subregions. -/
theorem bdma_permitted_v (b : arch.bound.Bound NT NR NP NL NS NE) (p : U8) (ptr len : U32)
    (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dma_permitted b p ptr len ⦃ r =>
      r = true ↔ (p.val < NP.val ∧
        DmaPermitted (plAt b.layout.partitions p.val) ptr.val len.val) ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dma_permitted
  simp only [lift, bind_ok]
  split
  · rename_i hge
    refine (WP.spec_ok _).mpr ⟨fun hf => Bool.noConfusion hf, fun hd => ?_⟩
    have := hd.1
    scalar_tac
  · rename_i hge
    step as ⟨pl, hpl⟩
    have hlt : p.val < NP.val := by scalar_tac
    have hpk : PLOk pl := by subst hpl; exact hb.lo.1 _ _ (by scalar_tac)
    have e : pl = plAt b.layout.partitions p.val := by
      rw [hpl]
      have := plAt_get b.layout.partitions (UScalar.cast .Usize p).val
        (by have := b.layout.partitions.property; scalar_tac)
      rw [this]
      congr 1
      scalar_tac
    refine WP.spec_mono (dma_permitted_exec_v pl ptr len hpk.1) (fun r hr => ?_)
    rw [hr, e]
    exact ⟨fun h => ⟨hlt, h⟩, fun h => h.2⟩

/-- The message buffer of an Endpoint operation, likewise. -/
theorem bmessage_permitted_v (b : arch.bound.Bound NT NR NP NL NS NE) (p : U8) (ptr len : U32)
    (hb : BX b) :
    arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.message_permitted b p ptr len ⦃ r =>
      r = true ↔ (p.val < NP.val ∧
        MsgPermitted (plAt b.layout.partitions p.val) ptr.val len.val) ⦄ := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.message_permitted
  simp only [lift, bind_ok]
  split
  · rename_i hge
    refine (WP.spec_ok _).mpr ⟨fun hf => Bool.noConfusion hf, fun hd => ?_⟩
    have := hd.1
    scalar_tac
  · rename_i hge
    step as ⟨pl, hpl⟩
    have hlt : p.val < NP.val := by scalar_tac
    have hpk : PLOk pl := by subst hpl; exact hb.lo.1 _ _ (by scalar_tac)
    have e : pl = plAt b.layout.partitions p.val := by
      rw [hpl]
      have := plAt_get b.layout.partitions (UScalar.cast .Usize p).val
        (by have := b.layout.partitions.property; scalar_tac)
      rw [this]
      congr 1
      scalar_tac
    refine WP.spec_mono (message_permitted_exec_v pl ptr len hpk.1) (fun r hr => ?_)
    rw [hr, e]
    exact ⟨fun h => ⟨hlt, h⟩, fun h => h.2⟩

end P18
