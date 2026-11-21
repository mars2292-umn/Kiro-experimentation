import Sched.CoreReal
import Sched.TablesAdapter
open Aeneas Aeneas.Std Result

/-! # The entry cores on the linked Kernel and the linked tables (2026-11-20)

The theorems of `Sched/CoreReal.lean` with the tables Kernel_Arch links:
the translated adapter `SysTables` over the system crate's `SystemTables`
(`IsSysTables`). What is assumed of the tables is then `STabT`, about the
generated constants and accessors themselves (proven of each
configuration's functions in `Tab/*/Proofs.lean`), and of the hardware,
`HwT`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize} {A S : Type} {ai : arch.cm4.entry.KernelArch A}
  {sti : arch.SystemTables S} {ti : arch.cm4.entry.Tables (arch.cm4.SysTables S)}
  {ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)}

theorem svc_service_core_sys [HwT ai] [STabT sti NT.val NP.val] (hb : IsBoundT ki)
    (ht : IsSysTables ti sti) (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A)
    (tables : arch.cm4.SysTables S) (state : arch.cm4.plan.ArchState)
    (frame : Array U32 8#usize) (number : U8) (hk : BX k) :
    arch.cm4.entry.svc_service_core ki ai ti k hw tables state frame number ⦃ r => BX r.2.1 ⦄ := by
  haveI := systables_tabt ht
  exact svc_service_core_real hb k hw tables state frame number hk

theorem irq_entry_core_sys [HwT ai] [STabT sti NT.val NP.val] (hb : IsBoundT ki)
    (ht : IsSysTables ti sti) (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A)
    (tables : arch.cm4.SysTables S) (state : arch.cm4.plan.ArchState)
    (ipsr psp control exc_return : U32) (hk : BX k) {hop : Nat} (hai : AI kvB k state hop) :
    arch.cm4.entry.irq_entry_core ki ai ti k hw tables state ipsr psp control exc_return
      ⦃ r => BX r.2.1 ⦄ := by
  haveI := systables_tabt ht
  exact irq_entry_core_real_ai hb k hw tables state ipsr psp control exc_return hk hai

theorem epilogue_core_sys [HwT ai] [STabT sti NT.val NP.val] (hb : IsBoundT ki)
    (ht : IsSysTables ti sti) (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A)
    (tables : arch.cm4.SysTables S) (state : arch.cm4.plan.ArchState) (n : U32) (hk : BX k) :
    arch.cm4.entry.epilogue_core ki ai ti k hw tables state n ⦃ r => BX r.2.1 ⦄ := by
  haveI := systables_tabt ht
  exact epilogue_core_real hb k hw tables state n hk

theorem fault_core_sys [HwT ai] [STabT sti NT.val NP.val] (hb : IsBoundT ki)
    (ht : IsSysTables ti sti) (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A)
    (tables : arch.cm4.SysTables S) (state : arch.cm4.plan.ArchState) (kind : U8)
    (in_kernel : Bool) (hk : BX k) :
    arch.cm4.entry.fault_core ki ai ti k hw tables state kind in_kernel ⦃ r => BX r.2.1 ⦄ := by
  haveI := systables_tabt ht
  exact fault_core_real hb k hw tables state kind in_kernel hk

theorem start_core_sys [HwT ai] [STabT sti NT.val NP.val] (hb : IsBoundT ki)
    (ht : IsSysTables ti sti) (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A)
    (tables : arch.cm4.SysTables S) (state : arch.cm4.plan.ArchState) (hk : BX k) :
    arch.cm4.entry.start_core ki ai ti k hw tables state ⦃ r => BX r.2.1 ⦄ := by
  haveI := systables_tabt ht
  exact start_core_real hb k hw tables state hk

theorem boot_core_sys [HwT ai] [STabT sti NT.val NP.val] (hb : IsBoundT ki)
    (ht : IsSysTables ti sti) (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A)
    (tables : arch.cm4.SysTables S) (state : arch.cm4.plan.ArchState) :
    arch.cm4.entry.boot_core ki ai ti k hw tables state ⦃ r => r.1 = true → BX r.2.1 ⦄ := by
  haveI := systables_tabt ht
  exact boot_core_real hb k hw tables state

end P18
