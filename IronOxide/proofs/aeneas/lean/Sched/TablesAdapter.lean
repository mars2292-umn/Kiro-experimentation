import Sched.CoreTotal
open Aeneas Aeneas.Std Result

/-! # The tables adapter (2026-11-20)

Kernel_Arch reads the generated tables through `SysTables`
(`arch/cm4/mod.rs`), which forwards each method of `Tables` to the system
crate's `SystemTables`. With `SystemTables` apart from `System` (whose
`task_entry` returns a function pointer) the adapter is translated, and the
assumption about the tables (`TabT`) moves down to the generated functions
themselves: if the constants and accessors of a `SystemTables` return
(`STabT`), every method of the adapter over it does.

`Tab/*/Proofs.lean` proves what `STabT` asks of each configuration's
generated accessors. Those files are translations of the system crates, in
their own namespaces with their own copies of the Kernel's types, so no
theorem identifies their functions with the fields of a
`SystemTables` here: `STabT` is where the two meet, by name. -/

namespace P18
open rsk_kernel

variable {S : Type} {sti : arch.SystemTables S} {TN TP : Nat}

/-- The constants and accessors of the generated tables return: the four
that are read with a Task or Partition identifier, for one in range. -/
class STabT (sti : arch.SystemTables S) (TN TP : outParam Nat) : Prop where
  irq_count : sti.IRQ_COUNT ⦃ _ => True ⦄
  config_checksum : sti.CONFIG_CHECKSUM ⦃ _ => True ⦄
  layout_checksum : sti.LAYOUT_CHECKSUM ⦃ _ => True ⦄
  timer_vector : sti.TIMER_VECTOR ⦃ _ => True ⦄
  priority_bits : sti.PRIORITY_BITS ⦃ _ => True ⦄
  cpuid : sti.CPUID ⦃ _ => True ⦄
  hfclk_xtal : sti.HFCLK_XTAL ⦃ _ => True ⦄
  fpu_needed : sti.FPU_NEEDED ⦃ _ => True ⦄
  fp_flush_to_zero : sti.FP_FLUSH_TO_ZERO ⦃ _ => True ⦄
  tick_hz : sti.TICK_HZ ⦃ _ => True ⦄
  dma_count : sti.DMA_COUNT ⦃ _ => True ⦄
  signal_count : sti.SIGNAL_COUNT ⦃ _ => True ⦄
  endpoint_count : sti.ENDPOINT_COUNT ⦃ _ => True ⦄
  binding : ∀ n, sti.binding n ⦃ _ => True ⦄
  dispatch_vector : ∀ (t : U8), t.val < TN → sti.dispatch_vector t ⦃ _ => True ⦄
  intake_vector : ∀ (t : U8), t.val < TN → sti.intake_vector t ⦃ _ => True ⦄
  stack_top : ∀ (p : U8), p.val < TP → sti.stack_top p ⦃ _ => True ⦄
  stack_bottom : ∀ (p : U8), p.val < TP → sti.stack_bottom p ⦃ _ => True ⦄
  dma : ∀ i, sti.dma i ⦃ _ => True ⦄
  signal : ∀ i, sti.signal i ⦃ _ => True ⦄
  endpoint : ∀ i, sti.endpoint i ⦃ _ => True ⦄
  endpoint_storage : sti.endpoint_storage ⦃ _ => True ⦄
  task_entry_address : ∀ t, sti.task_entry_address t ⦃ _ => True ⦄

/-- The trait instance is the translated adapter, method by method. -/
structure IsSysTables (ti : arch.cm4.entry.Tables (arch.cm4.SysTables S))
    (sti : arch.SystemTables S) : Prop where
  binding : ti.binding = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.binding sti
  dispatch_vector :
    ti.dispatch_vector = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.dispatch_vector sti
  intake_vector :
    ti.intake_vector = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.intake_vector sti
  stack_top : ti.stack_top = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.stack_top sti
  stack_bottom :
    ti.stack_bottom = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.stack_bottom sti
  task_entry_address : ti.task_entry_address =
    arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.task_entry_address sti
  irq_count : ti.irq_count = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.irq_count sti
  config_checksum :
    ti.config_checksum = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.config_checksum sti
  layout_checksum :
    ti.layout_checksum = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.layout_checksum sti
  timer_vector :
    ti.timer_vector = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.timer_vector sti
  boot_expect :
    ti.boot_expect = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.boot_expect sti
  fp_flush_to_zero : ti.fp_flush_to_zero =
    arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.fp_flush_to_zero sti
  tick_hz : ti.tick_hz = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.tick_hz sti
  dma_count : ti.dma_count = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.dma_count sti
  dma : ti.dma = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.dma sti
  endpoint_count :
    ti.endpoint_count = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.endpoint_count sti
  endpoint : ti.endpoint = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.endpoint sti
  endpoint_storage : ti.endpoint_storage =
    arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.endpoint_storage sti
  signal_count :
    ti.signal_count = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.signal_count sti
  signal : ti.signal = arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.signal sti

/-- **The adapter's methods return when the generated tables' do**: `TabT`
for the `Tables` Kernel_Arch reads, from `STabT` of the system crate's
`SystemTables`. -/
theorem systables_tabt {ti : arch.cm4.entry.Tables (arch.cm4.SysTables S)}
    (h : IsSysTables ti sti) [hs : STabT sti TN TP] : TabT ti TN TP where
  binding := fun _ n => by
    rw [h.binding]; unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.binding
    exact hs.binding n
  dispatch_vector := fun _ t ht => by
    rw [h.dispatch_vector]
    unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.dispatch_vector
    exact hs.dispatch_vector t ht
  intake_vector := fun _ t ht => by
    rw [h.intake_vector]
    unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.intake_vector
    exact hs.intake_vector t ht
  stack_top := fun _ p hp => by
    rw [h.stack_top]; unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.stack_top
    exact hs.stack_top p hp
  stack_bottom := fun _ p hp => by
    rw [h.stack_bottom]
    unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.stack_bottom
    exact hs.stack_bottom p hp
  task_entry_address := fun _ t => by
    rw [h.task_entry_address]
    unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.task_entry_address
    exact hs.task_entry_address t
  irq_count := fun _ => by
    rw [h.irq_count]; unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.irq_count
    exact hs.irq_count
  config_checksum := fun _ => by
    rw [h.config_checksum]
    unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.config_checksum
    exact hs.config_checksum
  layout_checksum := fun _ => by
    rw [h.layout_checksum]
    unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.layout_checksum
    exact hs.layout_checksum
  timer_vector := fun _ => by
    rw [h.timer_vector]
    unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.timer_vector
    exact hs.timer_vector
  boot_expect := fun _ => by
    rw [h.boot_expect]
    unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.boot_expect
    refine WP.spec_bind hs.priority_bits (fun _ _ => ?_)
    refine WP.spec_bind hs.cpuid (fun _ _ => ?_)
    refine WP.spec_bind hs.hfclk_xtal (fun _ _ => ?_)
    refine WP.spec_bind hs.fpu_needed (fun _ _ => ?_)
    exact (WP.spec_ok _).mpr trivial
  fp_flush_to_zero := fun _ => by
    rw [h.fp_flush_to_zero]
    unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.fp_flush_to_zero
    exact hs.fp_flush_to_zero
  tick_hz := fun _ => by
    rw [h.tick_hz]; unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.tick_hz
    exact hs.tick_hz
  dma_count := fun _ => by
    rw [h.dma_count]; unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.dma_count
    exact hs.dma_count
  dma := fun _ i => by
    rw [h.dma]; unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.dma
    exact hs.dma i
  endpoint_count := fun _ => by
    rw [h.endpoint_count]
    unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.endpoint_count
    exact hs.endpoint_count
  endpoint := fun _ i => by
    rw [h.endpoint]; unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.endpoint
    exact hs.endpoint i
  endpoint_storage := fun _ => by
    rw [h.endpoint_storage]
    unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.endpoint_storage
    exact hs.endpoint_storage
  signal_count := fun _ => by
    rw [h.signal_count]
    unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.signal_count
    exact hs.signal_count
  signal := fun _ i => by
    rw [h.signal]; unfold arch.cm4.SysTables.Insts.Rsk_kernelArchCm4EntryTables.signal
    exact hs.signal i

end P18
