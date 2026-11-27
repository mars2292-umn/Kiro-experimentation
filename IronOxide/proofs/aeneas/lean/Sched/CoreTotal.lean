import Sched.TotalBound
open Aeneas Aeneas.Std Result

/-! # The entry cores return (2026-11-20)

Property 21 for Kernel_Arch's entry cores (`arch/cm4/entry.rs`), as
extracted: called by their stub with any arguments, in a Kernel state that
satisfies the trait's invariant, each core returns, with no overflow, no
index out of bounds and no panic, and leaves the Kernel's invariant.

The cores are generic over three traits, so the statement is relative to
them: `KOpsT` (every method of `KernelOps` returns from a state satisfying
`okT` and keeps it; `job_info` answers a Priority that is a level), `HwT`
(every method of the hardware trait returns), `TabT` (every method of the
tables' trait returns), and `GhostT` (the seven ghost-value functions of
`ghostv.rs`, which the translation keeps opaque, return). -/

namespace P18
open rsk_kernel

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {okT : K → Prop} {TN TP : Nat}

/-- Steps through a function body: Aeneas's `step*`, with the conditionals
and matches it stops at split. -/
macro "ctac" : tactic => `(tactic| (step* <;> try (split <;> step* <;> try (split <;> step* <;> try (split <;> step* <;> try (split <;> step* <;> try (split <;> step* <;> try (split <;> step* <;> try (split <;> step* <;> try (split <;> step* <;> try (split <;> step* <;> try (split <;> step* <;> try (split <;> step* <;> try (split <;> step* <;> try (split <;> step* <;> try (split <;> step*))))))))))))))))

/-- `ctac` that also takes apart the tuples a call returned. -/
macro "ptac" : tactic => `(tactic| (step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*)))))))))))))))

/-- Every method of the hardware trait returns. -/
class HwT (ai : arch.cm4.entry.KernelArch A) : Prop where
  apply : ∀ hw act, ai.apply hw act ⦃ _ => True ⦄
  svc_clear_pending : ∀ hw, ai.svc_clear_pending hw ⦃ _ => True ⦄
  copy_in : ∀ hw a b c, ai.copy_in hw a b c ⦃ _ => True ⦄
  copy_out : ∀ hw a b c, ai.copy_out hw a b c ⦃ _ => True ⦄
  now : ∀ hw, ai.now hw ⦃ _ => True ⦄
  cycles : ∀ hw, ai.cycles hw ⦃ _ => True ⦄
  boot_checks : ∀ hw e, ai.boot_checks hw e ⦃ _ => True ⦄
  boot_info : ∀ hw, ai.boot_info hw ⦃ _ => True ⦄

@[step] theorem hw_apply [HwT ai] (hw : A) (act : arch.cm4.entry.Action) :
    ai.apply hw act ⦃ _ => True ⦄ := HwT.apply hw act
@[step] theorem hw_svc_clear_pending [HwT ai] (hw : A) :
    ai.svc_clear_pending hw ⦃ _ => True ⦄ := HwT.svc_clear_pending hw
@[step] theorem hw_copy_in [HwT ai] (hw : A) (a b c : U32) :
    ai.copy_in hw a b c ⦃ _ => True ⦄ := HwT.copy_in hw a b c
@[step] theorem hw_copy_out [HwT ai] (hw : A) (a b c : U32) :
    ai.copy_out hw a b c ⦃ _ => True ⦄ := HwT.copy_out hw a b c
@[step] theorem hw_now [HwT ai] (hw : A) : ai.now hw ⦃ _ => True ⦄ := HwT.now hw
@[step] theorem hw_cycles [HwT ai] (hw : A) : ai.cycles hw ⦃ _ => True ⦄ := HwT.cycles hw
@[step] theorem hw_boot_checks [HwT ai] (hw : A) (e : arch.cm4.plan.BootExpect) :
    ai.boot_checks hw e ⦃ _ => True ⦄ := HwT.boot_checks hw e
@[step] theorem hw_boot_info [HwT ai] (hw : A) : ai.boot_info hw ⦃ _ => True ⦄ := HwT.boot_info hw

/-- The ghost-value functions, opaque in the translation, return. -/
class GhostT : Prop where
  snap : ∀ {V : Type} (x : V), arch.cm4.ghostv.ghost_snap x ⦃ _ => True ⦄
  int : ∀ x, arch.cm4.ghostv.ghost_int x ⦃ _ => True ⦄
  partition : ∀ c r, arch.cm4.ghostv.ghost_partition c r ⦃ _ => True ⦄
  seq_empty : arch.cm4.ghostv.ghost_seq_empty ⦃ _ => True ⦄
  push_u8 : ∀ g x, arch.cm4.ghostv.ghost_seq_push_u8 g x ⦃ _ => True ⦄
  push_usize : ∀ g x, arch.cm4.ghostv.ghost_seq_push_usize g x ⦃ _ => True ⦄
  drop_last : ∀ g, arch.cm4.ghostv.ghost_seq_drop_last g ⦃ _ => True ⦄

@[step] theorem g_snap [GhostT] {V : Type} (x : V) :
    arch.cm4.ghostv.ghost_snap x ⦃ _ => True ⦄ := GhostT.snap x
@[step] theorem g_int [GhostT] (x : U32) : arch.cm4.ghostv.ghost_int x ⦃ _ => True ⦄ := GhostT.int x
@[step] theorem g_partition [GhostT] (c : arch.cm4.plan.EndpointCfg) (r : Usize) :
    arch.cm4.ghostv.ghost_partition c r ⦃ _ => True ⦄ := GhostT.partition c r
@[step] theorem g_seq_empty [GhostT] : arch.cm4.ghostv.ghost_seq_empty ⦃ _ => True ⦄ :=
  GhostT.seq_empty
@[step] theorem g_push_u8 [GhostT] (g : verus_builtin.Ghost (vstd.seq.Seq verus_builtin.int))
    (x : U8) : arch.cm4.ghostv.ghost_seq_push_u8 g x ⦃ _ => True ⦄ := GhostT.push_u8 g x
@[step] theorem g_push_usize [GhostT] (g : verus_builtin.Ghost (vstd.seq.Seq verus_builtin.int))
    (x : Usize) : arch.cm4.ghostv.ghost_seq_push_usize g x ⦃ _ => True ⦄ := GhostT.push_usize g x
@[step] theorem g_drop_last [GhostT] (g : verus_builtin.Ghost (vstd.seq.Seq verus_builtin.int)) :
    arch.cm4.ghostv.ghost_seq_drop_last g ⦃ _ => True ⦄ := GhostT.drop_last g

/-- The methods of the tables' trait return: `dispatch_vector` and
`intake_vector` for a Task identifier below `TN`, `stack_top` and
`stack_bottom` for a Partition below `TP` (the generated accessors index
their tables with the argument, `Tab/Proofs.lean`), the others for every
argument. -/
class TabT (ti : arch.cm4.entry.Tables T) (TN TP : outParam Nat) : Prop where
  binding : ∀ (tables : T) (n : U16), ti.binding tables n ⦃ _ => True ⦄
  dispatch_vector : ∀ (tables : T) (t : U8), t.val < TN → ti.dispatch_vector tables t ⦃ _ => True ⦄
  intake_vector : ∀ (tables : T) (t : U8), t.val < TN → ti.intake_vector tables t ⦃ _ => True ⦄
  stack_top : ∀ (tables : T) (p : U8), p.val < TP → ti.stack_top tables p ⦃ _ => True ⦄
  stack_bottom : ∀ (tables : T) (p : U8), p.val < TP → ti.stack_bottom tables p ⦃ _ => True ⦄
  task_entry_address : ∀ (tables : T) (t : U8), ti.task_entry_address tables t ⦃ _ => True ⦄
  irq_count : ∀ (tables : T) , ti.irq_count tables  ⦃ _ => True ⦄
  config_checksum : ∀ (tables : T) , ti.config_checksum tables  ⦃ _ => True ⦄
  layout_checksum : ∀ (tables : T) , ti.layout_checksum tables  ⦃ _ => True ⦄
  timer_vector : ∀ (tables : T) , ti.timer_vector tables  ⦃ _ => True ⦄
  boot_expect : ∀ (tables : T) , ti.boot_expect tables  ⦃ _ => True ⦄
  fp_flush_to_zero : ∀ (tables : T) , ti.fp_flush_to_zero tables  ⦃ _ => True ⦄
  tick_hz : ∀ (tables : T) , ti.tick_hz tables  ⦃ _ => True ⦄
  dma_count : ∀ (tables : T) , ti.dma_count tables  ⦃ _ => True ⦄
  dma : ∀ (tables : T) (i : Usize), ti.dma tables i ⦃ _ => True ⦄
  endpoint_count : ∀ (tables : T) , ti.endpoint_count tables  ⦃ _ => True ⦄
  endpoint : ∀ (tables : T) (i : Usize), ti.endpoint tables i ⦃ _ => True ⦄
  endpoint_storage : ∀ (tables : T) , ti.endpoint_storage tables  ⦃ _ => True ⦄
  signal_count : ∀ (tables : T) , ti.signal_count tables  ⦃ _ => True ⦄
  signal : ∀ (tables : T) (i : Usize), ti.signal tables i ⦃ _ => True ⦄

@[step] theorem tab_binding [TabT ti TN TP] (tables : T) (n : U16) :
    ti.binding tables n ⦃ _ => True ⦄ := TabT.binding tables n
@[step] theorem tab_dispatch_vector [TabT ti TN TP] (tables : T) (t : U8)
    (h : t.val < TN) : ti.dispatch_vector tables t ⦃ _ => True ⦄ := TabT.dispatch_vector tables t h
@[step] theorem tab_intake_vector [TabT ti TN TP] (tables : T) (t : U8)
    (h : t.val < TN) : ti.intake_vector tables t ⦃ _ => True ⦄ := TabT.intake_vector tables t h
@[step] theorem tab_stack_top [TabT ti TN TP] (tables : T) (p : U8)
    (h : p.val < TP) : ti.stack_top tables p ⦃ _ => True ⦄ := TabT.stack_top tables p h
@[step] theorem tab_stack_bottom [TabT ti TN TP] (tables : T) (p : U8)
    (h : p.val < TP) : ti.stack_bottom tables p ⦃ _ => True ⦄ := TabT.stack_bottom tables p h
@[step] theorem tab_task_entry_address [TabT ti TN TP] (tables : T) (t : U8) :
    ti.task_entry_address tables t ⦃ _ => True ⦄ := TabT.task_entry_address tables t
@[step] theorem tab_irq_count [TabT ti TN TP] (tables : T)  :
    ti.irq_count tables  ⦃ _ => True ⦄ := TabT.irq_count tables 
@[step] theorem tab_config_checksum [TabT ti TN TP] (tables : T)  :
    ti.config_checksum tables  ⦃ _ => True ⦄ := TabT.config_checksum tables 
@[step] theorem tab_layout_checksum [TabT ti TN TP] (tables : T)  :
    ti.layout_checksum tables  ⦃ _ => True ⦄ := TabT.layout_checksum tables 
@[step] theorem tab_timer_vector [TabT ti TN TP] (tables : T)  :
    ti.timer_vector tables  ⦃ _ => True ⦄ := TabT.timer_vector tables 
@[step] theorem tab_boot_expect [TabT ti TN TP] (tables : T)  :
    ti.boot_expect tables  ⦃ _ => True ⦄ := TabT.boot_expect tables 
@[step] theorem tab_fp_flush_to_zero [TabT ti TN TP] (tables : T)  :
    ti.fp_flush_to_zero tables  ⦃ _ => True ⦄ := TabT.fp_flush_to_zero tables 
@[step] theorem tab_tick_hz [TabT ti TN TP] (tables : T)  :
    ti.tick_hz tables  ⦃ _ => True ⦄ := TabT.tick_hz tables 
@[step] theorem tab_dma_count [TabT ti TN TP] (tables : T)  :
    ti.dma_count tables  ⦃ _ => True ⦄ := TabT.dma_count tables 
@[step] theorem tab_dma [TabT ti TN TP] (tables : T) (i : Usize) :
    ti.dma tables i ⦃ _ => True ⦄ := TabT.dma tables i
@[step] theorem tab_endpoint_count [TabT ti TN TP] (tables : T)  :
    ti.endpoint_count tables  ⦃ _ => True ⦄ := TabT.endpoint_count tables 
@[step] theorem tab_endpoint [TabT ti TN TP] (tables : T) (i : Usize) :
    ti.endpoint tables i ⦃ _ => True ⦄ := TabT.endpoint tables i
@[step] theorem tab_endpoint_storage [TabT ti TN TP] (tables : T)  :
    ti.endpoint_storage tables  ⦃ _ => True ⦄ := TabT.endpoint_storage tables 
@[step] theorem tab_signal_count [TabT ti TN TP] (tables : T)  :
    ti.signal_count tables  ⦃ _ => True ⦄ := TabT.signal_count tables 
@[step] theorem tab_signal [TabT ti TN TP] (tables : T) (i : Usize) :
    ti.signal tables i ⦃ _ => True ⦄ := TabT.signal tables i

/-- Every method of `KernelOps` the cores call returns from a state that
satisfies `okT`, and the ones that change the Kernel keep `okT`. `job_info`
answers a Priority that is a level (it indexes the level records) and a
Partition below `TP`; `task_count` and `partition_count` are at most `TN`
and `TP`, and a Task `settle` selects is below `TN` (the tables are read
with these). -/
class KOpsT (ki : arch.bound.KernelOps K) (okT : outParam (K → Prop)) (TN TP : outParam Nat) :
    Prop where
  depth : ∀ (k : K) , okT k → ki.depth k  ⦃ r => True ⦄
  top : ∀ (k : K) , okT k → ki.top k  ⦃ r => True ⦄
  job_info : ∀ (k : K) (t : U8), okT k →
    ki.job_info k t ⦃ r => r.priority.val < 8 ∧ r.partition.val < TP ⦄
  stack_at : ∀ (k : K) (i : Usize), okT k → ki.stack_at k i ⦃ r => True ⦄
  is_killed : ∀ (k : K) (t : U8), okT k → ki.is_killed k t ⦃ r => True ⦄
  eligible : ∀ (k : K) (t : U8), okT k → ki.eligible k t ⦃ r => True ⦄
  basepri_now : ∀ (k : K) , okT k → ki.basepri_now k  ⦃ r => True ⦄
  next_compare : ∀ (k : K) , okT k → ki.next_compare k  ⦃ r => True ⦄
  source_masked : ∀ (k : K) (t : U8), okT k → ki.source_masked k t ⦃ r => True ⦄
  partition_stopped : ∀ (k : K) (p : U8), okT k → ki.partition_stopped k p ⦃ r => True ⦄
  partition_view : ∀ (k : K) (p : U8), okT k → ki.partition_view k p ⦃ r => True ⦄
  dma_permitted : ∀ (k : K) (p : U8) (a b : U32), okT k → ki.dma_permitted k p a b ⦃ r => True ⦄
  in_ro_window : ∀ (k : K) (p : U8) (a : U32), okT k → ki.in_ro_window k p a ⦃ r => True ⦄
  message_permitted : ∀ (k : K) (p : U8) (a b : U32), okT k → ki.message_permitted k p a b ⦃ r => True ⦄
  safe_state : ∀ (k : K) , okT k → ki.safe_state k  ⦃ r => True ⦄
  task_count : ∀ (k : K) , okT k → ki.task_count k  ⦃ r => r.val ≤ TN ⦄
  partition_count : ∀ (k : K) , okT k → ki.partition_count k  ⦃ r => r.val ≤ TP ⦄
  config_checksum : ∀ (k : K) , okT k → ki.config_checksum k  ⦃ r => True ⦄
  layout_checksum : ∀ (k : K) , okT k → ki.layout_checksum k  ⦃ r => True ⦄
  start : ∀ (k : K) , okT k → ki.start k  ⦃ r => okT r ⦄
  dispatch : ∀ (k : K) (t : U8) (pc : U32), okT k → ki.dispatch k t pc ⦃ r => okT r ⦄
  job_return : ∀ (k : K) (e : U32), okT k → ki.job_return k e ⦃ r => okT r ⦄
  svc_lock : ∀ (k : K) (x : U8), okT k → ki.svc_lock k x ⦃ r => okT r.2 ⦄
  svc_unlock : ∀ (k : K) (x : U8), okT k → ki.svc_unlock k x ⦃ r => okT r.2 ⦄
  settle : ∀ (k : K) , okT k →
    ki.settle k  ⦃ r => okT r.2 ∧ ∀ t, r.1.1 = some t → t.val < TN ⦄
  intake : ∀ (k : K) (t p : U8) (now : U64) (o : U8), okT k → ki.intake k t p now o ⦃ r => okT r ⦄
  timer_event : ∀ (k : K) (now : U64) (rc : U32), okT k → ki.timer_event k now rc ⦃ r => okT r ⦄
  fault : ∀ (k : K) (kind : U8) (kn : Bool), okT k → ki.fault k kind kn ⦃ r => okT r ⦄
  fault_of : ∀ (k : K) (kind t : U8), okT k → ki.fault_of k kind t ⦃ r => okT r ⦄
  record_event : ∀ (k : K) (kind t : U8), okT k → ki.record_event k kind t ⦃ r => okT r ⦄
  take_reinit : ∀ (k : K) (p : U8), okT k → ki.take_reinit k p ⦃ r => okT r.2 ⦄
  take_stop_dma : ∀ (k : K) (p : U8), okT k → ki.take_stop_dma k p ⦃ r => okT r.2 ⦄
  boot_failed : ∀ (k : K) (f : U32), okT k → ki.boot_failed k f ⦃ r => okT r ⦄
  boot_info : ∀ (k : K) (x y : Bool), okT k → ki.boot_info k x y ⦃ r => okT r ⦄
  validate : ∀ (k : K), ki.validate k ⦃ b => ki.validate k = ok b ⦄

/-- `init` returns once `validate` accepted the tables, and establishes
`okT`. A class of its own since 2026-11-27: only the boot calls `init`,
and the other cores' theorems are then free to be used with an `okT` that
`init` does not establish (`Sched/Trace.lean`: `okT` with "reached from
the state the entry found"). -/
class KInitT (ki : arch.bound.KernelOps K) (okT : K → Prop) : Prop where
  init : ∀ (k : K), ki.validate k = ok true → ki.init k ⦃ r => okT r ⦄

@[step] theorem k_depth [KOpsT ki okT TN TP] (k : K)  (hk : okT k) :
    ki.depth k  ⦃ r => True ⦄ := KOpsT.depth k  hk
@[step] theorem k_top [KOpsT ki okT TN TP] (k : K)  (hk : okT k) :
    ki.top k  ⦃ r => True ⦄ := KOpsT.top k  hk
@[step] theorem k_job_info [KOpsT ki okT TN TP] (k : K) (t : U8) (hk : okT k) :
    ki.job_info k t ⦃ r => r.priority.val < 8 ∧ r.partition.val < TP ⦄ := KOpsT.job_info k t hk
@[step] theorem k_stack_at [KOpsT ki okT TN TP] (k : K) (i : Usize) (hk : okT k) :
    ki.stack_at k i ⦃ r => True ⦄ := KOpsT.stack_at k i hk
@[step] theorem k_is_killed [KOpsT ki okT TN TP] (k : K) (t : U8) (hk : okT k) :
    ki.is_killed k t ⦃ r => True ⦄ := KOpsT.is_killed k t hk
@[step] theorem k_eligible [KOpsT ki okT TN TP] (k : K) (t : U8) (hk : okT k) :
    ki.eligible k t ⦃ r => True ⦄ := KOpsT.eligible k t hk
@[step] theorem k_basepri_now [KOpsT ki okT TN TP] (k : K)  (hk : okT k) :
    ki.basepri_now k  ⦃ r => True ⦄ := KOpsT.basepri_now k  hk
@[step] theorem k_next_compare [KOpsT ki okT TN TP] (k : K)  (hk : okT k) :
    ki.next_compare k  ⦃ r => True ⦄ := KOpsT.next_compare k  hk
@[step] theorem k_source_masked [KOpsT ki okT TN TP] (k : K) (t : U8) (hk : okT k) :
    ki.source_masked k t ⦃ r => True ⦄ := KOpsT.source_masked k t hk
@[step] theorem k_partition_stopped [KOpsT ki okT TN TP] (k : K) (p : U8) (hk : okT k) :
    ki.partition_stopped k p ⦃ r => True ⦄ := KOpsT.partition_stopped k p hk
@[step] theorem k_partition_view [KOpsT ki okT TN TP] (k : K) (p : U8) (hk : okT k) :
    ki.partition_view k p ⦃ r => True ⦄ := KOpsT.partition_view k p hk
@[step] theorem k_dma_permitted [KOpsT ki okT TN TP] (k : K) (p : U8) (a b : U32) (hk : okT k) :
    ki.dma_permitted k p a b ⦃ r => True ⦄ := KOpsT.dma_permitted k p a b hk
@[step] theorem k_in_ro_window [KOpsT ki okT TN TP] (k : K) (p : U8) (a : U32) (hk : okT k) :
    ki.in_ro_window k p a ⦃ r => True ⦄ := KOpsT.in_ro_window k p a hk
@[step] theorem k_message_permitted [KOpsT ki okT TN TP] (k : K) (p : U8) (a b : U32) (hk : okT k) :
    ki.message_permitted k p a b ⦃ r => True ⦄ := KOpsT.message_permitted k p a b hk
@[step] theorem k_safe_state [KOpsT ki okT TN TP] (k : K)  (hk : okT k) :
    ki.safe_state k  ⦃ r => True ⦄ := KOpsT.safe_state k  hk
@[step] theorem k_task_count [KOpsT ki okT TN TP] (k : K)  (hk : okT k) :
    ki.task_count k  ⦃ r => r.val ≤ TN ⦄ := KOpsT.task_count k  hk
@[step] theorem k_partition_count [KOpsT ki okT TN TP] (k : K)  (hk : okT k) :
    ki.partition_count k  ⦃ r => r.val ≤ TP ⦄ := KOpsT.partition_count k  hk
@[step] theorem k_config_checksum [KOpsT ki okT TN TP] (k : K)  (hk : okT k) :
    ki.config_checksum k  ⦃ r => True ⦄ := KOpsT.config_checksum k  hk
@[step] theorem k_layout_checksum [KOpsT ki okT TN TP] (k : K)  (hk : okT k) :
    ki.layout_checksum k  ⦃ r => True ⦄ := KOpsT.layout_checksum k  hk
@[step] theorem k_start [KOpsT ki okT TN TP] (k : K)  (hk : okT k) :
    ki.start k  ⦃ r => okT r ⦄ := KOpsT.start k  hk
@[step] theorem k_dispatch [KOpsT ki okT TN TP] (k : K) (t : U8) (pc : U32) (hk : okT k) :
    ki.dispatch k t pc ⦃ r => okT r ⦄ := KOpsT.dispatch k t pc hk
@[step] theorem k_job_return [KOpsT ki okT TN TP] (k : K) (e : U32) (hk : okT k) :
    ki.job_return k e ⦃ r => okT r ⦄ := KOpsT.job_return k e hk
@[step] theorem k_svc_lock [KOpsT ki okT TN TP] (k : K) (x : U8) (hk : okT k) :
    ki.svc_lock k x ⦃ r => okT r.2 ⦄ := KOpsT.svc_lock k x hk
@[step] theorem k_svc_unlock [KOpsT ki okT TN TP] (k : K) (x : U8) (hk : okT k) :
    ki.svc_unlock k x ⦃ r => okT r.2 ⦄ := KOpsT.svc_unlock k x hk
@[step] theorem k_settle [KOpsT ki okT TN TP] (k : K)  (hk : okT k) :
    ki.settle k  ⦃ r => okT r.2 ∧ ∀ t, r.1.1 = some t → t.val < TN ⦄ := KOpsT.settle k  hk
@[step] theorem k_intake [KOpsT ki okT TN TP] (k : K) (t p : U8) (now : U64) (o : U8) (hk : okT k) :
    ki.intake k t p now o ⦃ r => okT r ⦄ := KOpsT.intake k t p now o hk
@[step] theorem k_timer_event [KOpsT ki okT TN TP] (k : K) (now : U64) (rc : U32) (hk : okT k) :
    ki.timer_event k now rc ⦃ r => okT r ⦄ := KOpsT.timer_event k now rc hk
@[step] theorem k_fault [KOpsT ki okT TN TP] (k : K) (kind : U8) (kn : Bool) (hk : okT k) :
    ki.fault k kind kn ⦃ r => okT r ⦄ := KOpsT.fault k kind kn hk
@[step] theorem k_fault_of [KOpsT ki okT TN TP] (k : K) (kind t : U8) (hk : okT k) :
    ki.fault_of k kind t ⦃ r => okT r ⦄ := KOpsT.fault_of k kind t hk
@[step] theorem k_record_event [KOpsT ki okT TN TP] (k : K) (kind t : U8) (hk : okT k) :
    ki.record_event k kind t ⦃ r => okT r ⦄ := KOpsT.record_event k kind t hk
@[step] theorem k_take_reinit [KOpsT ki okT TN TP] (k : K) (p : U8) (hk : okT k) :
    ki.take_reinit k p ⦃ r => okT r.2 ⦄ := KOpsT.take_reinit k p hk
@[step] theorem k_take_stop_dma [KOpsT ki okT TN TP] (k : K) (p : U8) (hk : okT k) :
    ki.take_stop_dma k p ⦃ r => okT r.2 ⦄ := KOpsT.take_stop_dma k p hk
@[step] theorem k_boot_failed [KOpsT ki okT TN TP] (k : K) (f : U32) (hk : okT k) :
    ki.boot_failed k f ⦃ r => okT r ⦄ := KOpsT.boot_failed k f hk
@[step] theorem k_boot_info [KOpsT ki okT TN TP] (k : K) (x y : Bool) (hk : okT k) :
    ki.boot_info k x y ⦃ r => okT r ⦄ := KOpsT.boot_info k x y hk

/-! ## Constants and the FIFO operations -/

theorem max_roles : arch.cm4.plan.MAX_ROLES = 4#usize := by
  unfold arch.cm4.plan.MAX_ROLES; rfl

theorem max_endpoints : arch.cm4.plan.MAX_ENDPOINTS = 8#usize := by
  unfold arch.cm4.plan.MAX_ENDPOINTS; rfl

theorem max_instant : arch.cm4.plan.MAX_INSTANT = 4611686018427387904#u64 := by
  unfold arch.cm4.plan.MAX_INSTANT; rfl

theorem min_cmp : rsk_logic.hw_model.MIN_COMPARE_DISTANCE = 16#u64 := by
  unfold rsk_logic.hw_model.MIN_COMPARE_DISTANCE; rfl

theorem max_floors : arch.cm4.entry.MAX_FLOORS = 16#usize := by
  unfold arch.cm4.entry.MAX_FLOORS; rfl

theorem max_irqs : arch.cm4.entry.MAX_IRQS = 496#u16 := by
  unfold arch.cm4.entry.MAX_IRQS; rfl

@[step] theorem fifo_is_wf_t (f : rsk_logic.logic.endpoint.Fifo) (cap : U8) :
    rsk_logic.logic.endpoint.Fifo.is_wf f cap ⦃ _ => True ⦄ := by
  unfold rsk_logic.logic.endpoint.Fifo.is_wf
  ctac

@[step] theorem fifo_clear_t (f : rsk_logic.logic.endpoint.Fifo) :
    rsk_logic.logic.endpoint.Fifo.clear f ⦃ _ => True ⦄ := by
  unfold rsk_logic.logic.endpoint.Fifo.clear
  ctac

@[step] theorem fifo_push_t (f : rsk_logic.logic.endpoint.Fifo) (cap : U8) :
    rsk_logic.logic.endpoint.Fifo.push f cap ⦃ _ => True ⦄ := by
  unfold rsk_logic.logic.endpoint.Fifo.push
  ctac

@[step] theorem fifo_pop_t (f : rsk_logic.logic.endpoint.Fifo) (cap : U8) (hc : 1 ≤ cap.val) :
    rsk_logic.logic.endpoint.Fifo.pop f cap ⦃ _ => True ⦄ := by
  unfold rsk_logic.logic.endpoint.Fifo.pop
  ctac

@[step] theorem encode_level_t (l : U8) (hl : l.val ≤ 8) :
    arch.bound.encode_level l ⦃ _ => True ⦄ := encode_level_total l hl

@[step] theorem boot_fail_layout_t : arch.cm4.entry.BOOT_FAIL_LAYOUT_CHECKSUM ⦃ _ => True ⦄ := by
  unfold arch.cm4.entry.BOOT_FAIL_LAYOUT_CHECKSUM
  step*

@[step] theorem boot_fail_dma_t : arch.cm4.entry.BOOT_FAIL_DMA_WINDOW ⦃ _ => True ⦄ := by
  unfold arch.cm4.entry.BOOT_FAIL_DMA_WINDOW
  step*

theorem wd_periods : arch.cm4.entry.WATCHDOG_HOUSEKEEPING_PERIODS = 4#u64 := by
  unfold arch.cm4.entry.WATCHDOG_HOUSEKEEPING_PERIODS; rfl

theorem wd_hz : arch.cm4.entry.WATCHDOG_HZ = 32768#u64 := by
  unfold arch.cm4.entry.WATCHDOG_HZ; rfl

@[step] theorem watchdog_reload_t (tick_hz : U64) :
    arch.cm4.entry.watchdog_reload tick_hz ⦃ _ => True ⦄ := by
  unfold arch.cm4.entry.watchdog_reload
  simp only [wd_periods, wd_hz]
  split
  · step*
  · rename_i hne
    refine WP.spec_bind hk_period_total (fun i hi => ?_)
    step as ⟨ i1, hi1 ⟩
    step as ⟨ ticks, ht ⟩
    have hz : tick_hz.val ≠ 0 := fun e => hne (by scalar_tac)
    step as ⟨ r, hr ⟩
    split <;> step*

/-! ## The functions of `entry.rs`, in the order of the file -/

-- FN arch.cm4.entry.charge_point
@[step] theorem charge_point_t [HwT ai] (hw : A) (state : arch.cm4.plan.ArchState) :
    arch.cm4.entry.charge_point ai hw state ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.charge_point
  ctac

-- FN arch.cm4.plan.compare_target
@[step] theorem compare_target_t (now «at» : U64) (h : now.val ≤ 4611686018427387904) :
    arch.cm4.plan.compare_target now «at» ⦃ r => True ⦄ := by
  unfold arch.cm4.plan.compare_target
  simp only [min_cmp]
  ctac

-- FN arch.cm4.entry.arm_compare
@[step] theorem arm_compare_t [HwT ai] (hw : A) («at» : Std.U64) :
    arch.cm4.entry.arm_compare ai hw «at» ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.arm_compare
  simp only [max_instant]
  ctac

-- FN arch.cm4.entry.rearm_compare
@[step] theorem rearm_compare_t [HwT ai] (hw : A) («at» : Std.U64) :
    arch.cm4.entry.rearm_compare ai hw «at» ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.rearm_compare
  simp only [max_instant]
  ctac

-- FN arch.cm4.entry.clear_channels_loop
@[step] theorem clear_channels_loop_t (i : Usize) (state : arch.cm4.plan.ArchState) (base c : Usize)
    (hb : base.val + i.val ≤ 128) (hc : c.val ≤ i.val) :
    arch.cm4.entry.clear_channels_loop i state base c ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.clear_channels_loop
  apply loop.spec_decr_nat (fun (x : arch.cm4.plan.ArchState × Usize) => i.val - x.2.val) (fun x => x.2.val ≤ i.val)
  · intro x hx
    obtain ⟨st, c1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.clear_channels_loop.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · exact hc

-- FN arch.cm4.entry.clear_channels
@[step] theorem clear_channels_t (state : arch.cm4.plan.ArchState) (e : Usize) (he : e.val < 8) :
    arch.cm4.entry.clear_channels state e ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.clear_channels
  simp only [max_roles]
  step as ⟨ i, hi ⟩
  have hi' : i.val = 16 := by scalar_tac
  have hm : e.val * i.val ≤ 112 := by rw [hi']; omega
  step as ⟨ base, hbase ⟩
  exact clear_channels_loop_t i state base 0#usize (by scalar_tac) (by scalar_tac)

-- FN arch.cm4.entry.partition_in_loop
@[step] theorem partition_in_loop_t (cfg : arch.cm4.plan.EndpointCfg) (p : U8) (i : Usize) (found : Bool)
    (hi : i.val ≤ 4) :
    arch.cm4.entry.partition_in_loop cfg p i found ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.partition_in_loop
  apply loop.spec_decr_nat (fun (x : Usize × Bool) => 4 - x.1.val) (fun x => x.1.val ≤ 4)
  · intro x hx
    obtain ⟨i1, f1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.partition_in_loop.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · exact hi

-- FN arch.cm4.entry.partition_in
@[step] theorem partition_in_t (cfg : arch.cm4.plan.EndpointCfg) (p : Std.U8) :
    arch.cm4.entry.partition_in cfg p ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.partition_in
  ctac

-- FN arch.cm4.entry.role_slot
@[step] theorem role_slot_t (e r : Usize) (he : e.val < 8) (hr : r.val < 4) :
    arch.cm4.entry.role_slot e r ⦃ x => x.val < 32 ⦄ := by
  unfold arch.cm4.entry.role_slot
  simp only [max_roles]
  step as ⟨ i, hi ⟩
  step as ⟨ j, hj ⟩
  scalar_tac

-- FN arch.cm4.entry.endpoint_reinit_loop0_loop0
@[step] theorem endpoint_reinit_loop0_loop0_t (state : arch.cm4.plan.ArchState) (p : U8) (e : Usize)
    (cfg : arch.cm4.plan.EndpointCfg) (r : Usize) (he : e.val < 8) (hr : r.val ≤ 4) :
    arch.cm4.entry.endpoint_reinit_loop0_loop0 state p e cfg r ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.endpoint_reinit_loop0_loop0
  apply loop.spec_decr_nat (fun (x : arch.cm4.plan.ArchState × Usize) => 4 - x.2.val) (fun x => x.2.val ≤ 4)
  · intro x hx
    obtain ⟨st, r1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.endpoint_reinit_loop0_loop0.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · exact hr

-- FN arch.cm4.entry.endpoint_reinit_loop0
@[step] theorem endpoint_reinit_loop0_t [TabT ti TN TP] (tables : T) (state : arch.cm4.plan.ArchState) (p : U8) (n e : Usize)
    (he : e.val ≤ 8) :
    arch.cm4.entry.endpoint_reinit_loop0 ti tables state p n e ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.endpoint_reinit_loop0
  apply loop.spec_decr_nat (fun (x : arch.cm4.plan.ArchState × Usize) => 8 - x.2.val) (fun x => x.2.val ≤ 8)
  · intro x hx
    obtain ⟨st, e1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.endpoint_reinit_loop0.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · exact he

-- FN arch.cm4.entry.endpoint_reinit
@[step] theorem endpoint_reinit_t [TabT ti TN TP] (tables : T) (state : arch.cm4.plan.ArchState) (p : Std.U8) :
    arch.cm4.entry.endpoint_reinit ti tables state p ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.endpoint_reinit
  ctac

-- FN arch.cm4.entry.dma_stop_partition_loop
@[step] theorem dma_stop_partition_loop_t [HwT ai] [TabT ti TN TP] (hw : A) (tables : T) (p : U8) (n i : Usize) :
    arch.cm4.entry.dma_stop_partition_loop ai ti hw tables p n i ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.dma_stop_partition_loop
  apply loop.spec_decr_nat (fun (x : A × Usize) => n.val - x.2.val) (fun x => True)
  · intro x hx
    obtain ⟨hw1, i1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.dma_stop_partition_loop.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · trivial

-- FN arch.cm4.entry.dma_stop_partition
@[step] theorem dma_stop_partition_t [HwT ai] [TabT ti TN TP] (hw : A) (tables : T) (p : Std.U8) :
    arch.cm4.entry.dma_stop_partition ai ti hw tables p ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.dma_stop_partition
  ctac

-- FN arch.cm4.entry.apply_core_loop0
@[step] theorem apply_core_loop0_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] (k : K) (hw : A) (tables : T)
    (n t : Usize) (hk : okT k) (hn : n.val ≤ TN) :
    arch.cm4.entry.apply_core_loop0 ki ai ti k hw tables n t ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.apply_core_loop0
  apply loop.spec_decr_nat (fun (x : A × Usize) => n.val - x.2.val) (fun x => True)
  · intro x hx
    obtain ⟨hw1, t1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.apply_core_loop0.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · trivial

-- FN arch.cm4.entry.apply_core_loop1
@[step] theorem apply_core_loop1_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] (k : K) (hw : A) (tables : T)
    (state : arch.cm4.plan.ArchState) (np p : Usize) (hk : okT k) :
    arch.cm4.entry.apply_core_loop1 ki ai ti k hw tables state np p ⦃ r => okT r.1 ⦄ := by
  unfold arch.cm4.entry.apply_core_loop1
  apply loop.spec_decr_nat (fun (x : K × A × arch.cm4.plan.ArchState × Usize) => np.val - x.2.2.2.val) (fun x => okT x.1)
  · intro x hx
    obtain ⟨k1, hw1, st, p1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.apply_core_loop1.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | assumption
      | (refine ⟨by assumption, ?_⟩; scalar_tac)
      | (simp_all; done)
  · exact hk

-- FN arch.cm4.entry.apply_core
@[step] theorem apply_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (hk : okT k) :
    arch.cm4.entry.apply_core ki ai ti k hw tables state ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.apply_core
  ctac

-- FN arch.cm4.entry.vectors_ok_loop
@[step] theorem vectors_ok_loop_t [TabT ti TN TP] (tables : T) (n t : Usize) (hn : n.val ≤ TN) :
    arch.cm4.entry.vectors_ok_loop ti tables n t ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.vectors_ok_loop
  apply loop.spec_decr_nat (fun (x : Usize) => n.val - x.val) (fun x => True)
  · intro x hx
    obtain ⟨t1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.vectors_ok_loop.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · trivial

-- FN arch.cm4.entry.vectors_ok
@[step] theorem vectors_ok_t [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (tables : T) (hk : okT k) :
    arch.cm4.entry.vectors_ok ki ti k tables ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.vectors_ok
  ctac

-- FN arch.cm4.entry.boot_vectors_loop
@[step] theorem boot_vectors_loop_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] (k : K) (hw : A) (tables : T)
    («end» n : U16) (hk : okT k) :
    arch.cm4.entry.boot_vectors_loop ki ai ti k hw tables «end» n ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.boot_vectors_loop
  apply loop.spec_decr_nat (fun (x : A × U16) => «end».val - x.2.val) (fun x => True)
  · intro x hx
    obtain ⟨hw1, n1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.boot_vectors_loop.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · trivial

-- FN arch.cm4.entry.boot_vectors
@[step] theorem boot_vectors_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] (k : K) (hw : A) (tables : T)
    (irqs : U16) (hk : okT k) (hi : irqs.val ≤ 496) :
    arch.cm4.entry.boot_vectors ki ai ti k hw tables irqs ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.boot_vectors
  ctac

-- FN arch.cm4.entry.dma_boot_loop
@[step] theorem dma_boot_loop_t [HwT ai] [TabT ti TN TP] (hw : A) (tables : T) (n i : Usize) :
    arch.cm4.entry.dma_boot_loop ai ti hw tables n i ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.dma_boot_loop
  apply loop.spec_decr_nat (fun (x : A × Usize) => n.val - x.2.val) (fun x => True)
  · intro x hx
    obtain ⟨hw1, i1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.dma_boot_loop.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · trivial

-- FN arch.cm4.entry.dma_boot
@[step] theorem dma_boot_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (hk : okT k) :
    arch.cm4.entry.dma_boot ki ai ti k hw tables ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.dma_boot
  ctac

-- FN arch.cm4.entry.dma_windows_ok_loop
@[step] theorem dma_windows_ok_loop_t [KOpsT ki okT TN TP] [TabT ti TN TP] (k : K) (tables : T) (n i : Usize)
    (hk : okT k) :
    arch.cm4.entry.dma_windows_ok_loop ki ti k tables n i ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.dma_windows_ok_loop
  apply loop.spec_decr_nat (fun (x : Usize) => n.val - x.val) (fun x => True)
  · intro x hx
    obtain ⟨i1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.dma_windows_ok_loop.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · trivial

-- FN arch.cm4.entry.dma_windows_ok
@[step] theorem dma_windows_ok_t [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (tables : T) (hk : okT k) :
    arch.cm4.entry.dma_windows_ok ki ti k tables ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.dma_windows_ok
  ctac

-- FN arch.cm4.entry.boot_core_loop0
@[step] theorem boot_core_loop0_t (state : arch.cm4.plan.ArchState) (l : Usize) (hl : l.val ≤ 8) :
    arch.cm4.entry.boot_core_loop0 state l ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.boot_core_loop0
  apply loop.spec_decr_nat (fun (x : arch.cm4.plan.ArchState × Usize) => 8 - x.2.val) (fun x => x.2.val ≤ 8)
  · intro x hx
    obtain ⟨st, l1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.boot_core_loop0.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · exact hl

-- FN arch.cm4.entry.boot_core_loop1
@[step] theorem boot_core_loop1_t [TabT ti TN TP] (tables : T) (a : Array U32 16#usize) (np p : Usize)
    (hnp : np.val ≤ TP) : arch.cm4.entry.boot_core_loop1 ti tables a np p ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.boot_core_loop1
  apply loop.spec_decr_nat (fun (x : Array U32 16#usize × Usize) => np.val - x.2.val) (fun x => True)
  · intro x hx
    obtain ⟨a1, p1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.boot_core_loop1.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · trivial

-- FN arch.cm4.entry.boot_core
set_option maxHeartbeats 16000000 in
/-- `boot_core` returns from any state: nothing is asked of the Kernel
before `validate` and `init`. A boot that passes its checks (the result
`true`) leaves `okT`. -/
@[step] theorem boot_core_t [HwT ai] [KOpsT ki okT TN TP] [KInitT ki okT] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) :
    arch.cm4.entry.boot_core ki ai ti k hw tables state ⦃ r => r.1 = true → okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.boot_core
  simp only [max_irqs]
  step as ⟨ hw1 ⟩
  refine WP.spec_bind (KOpsT.validate (okT := okT) k) (fun b hb => ?_)
  split
  · rename_i hbt
    have hv : ki.validate k = ok true := by rw [hb, hbt]
    have hin := KInitT.init (ki := ki) (okT := okT) k hv
    step as ⟨ state1 ⟩
    step as ⟨ g ⟩
    refine WP.spec_bind hin (fun k1 hk1 => ?_)
    ctac
    all_goals first
      | (scalar_tac; done)
      | assumption
      | (simp_all; done)
      | (simp_all; scalar_tac)
  · ctac

-- FN arch.cm4.entry.dma_request_core
@[step] theorem dma_request_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (i : Std.Usize) (ptr : Std.U32) (len : Std.U32) (hk : okT k) :
    arch.cm4.entry.dma_request_core ki ai ti k hw tables i ptr len ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.dma_request_core
  ctac

-- FN arch.cm4.entry.dma_write_core_loop
@[step] theorem dma_write_core_loop_t [TabT ti TN TP] (tables : T) (n : Usize) (reg window : U32)
    (mediated : Bool) (j : Usize) :
    arch.cm4.entry.dma_write_core_loop ti tables n reg window mediated j ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.dma_write_core_loop
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => n.val - x.2.val) (fun x => True)
  · intro x hx
    obtain ⟨m1, j1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.dma_write_core_loop.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · trivial

-- FN arch.cm4.entry.dma_write_core
@[step] theorem dma_write_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (i : Std.Usize) (offset : Std.U32) (value : Std.U32) (hk : okT k) :
    arch.cm4.entry.dma_write_core ki ai ti k hw tables i offset value ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.dma_write_core
  ctac

-- FN arch.cm4.entry.raise_core
@[step] theorem raise_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (i : Std.Usize) (hk : okT k) :
    arch.cm4.entry.raise_core ki ai ti k hw tables i ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.raise_core
  ctac

-- FN arch.cm4.entry.channel_index
@[step] theorem channel_index_t (e «from» «to» : Usize) (he : e.val < 8) (hf : «from».val < 4)
    (ht : «to».val < 4) : arch.cm4.entry.channel_index e «from» «to» ⦃ x => x.val < 128 ⦄ := by
  unfold arch.cm4.entry.channel_index
  simp only [max_roles]
  step as ⟨ i, hi ⟩
  have hi' : i.val = 16 := by scalar_tac
  have hm : e.val * i.val ≤ 112 := by rw [hi']; omega
  step as ⟨ i1, hi1 ⟩
  step as ⟨ i2, hi2 ⟩
  step as ⟨ i3, hi3 ⟩
  step as ⟨ i4, hi4 ⟩
  rw [hi4, hi3, hi2, hi1, hi']
  omega

-- FN arch.cm4.entry.channel_push
@[step] theorem channel_push_t (state : arch.cm4.plan.ArchState) (ch : Usize) (cap : U8) (hch : ch.val < 128) :
    arch.cm4.entry.channel_push state ch cap ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.channel_push
  ctac

-- FN arch.cm4.entry.channel_pop
@[step] theorem channel_pop_t (state : arch.cm4.plan.ArchState) (ch : Usize) (cap : U8) (hch : ch.val < 128)
    (hc : 1 ≤ cap.val) : arch.cm4.entry.channel_pop state ch cap ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.channel_pop
  ctac

-- FN arch.cm4.entry.endpoint_reinit_one_loop0
@[step] theorem endpoint_reinit_one_loop0_t (i : Usize) (state : arch.cm4.plan.ArchState) (base c : Usize)
    (hb : base.val + i.val ≤ 128) (hc : c.val ≤ i.val) :
    arch.cm4.entry.endpoint_reinit_one_loop0 i state base c ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.endpoint_reinit_one_loop0
  apply loop.spec_decr_nat (fun (x : arch.cm4.plan.ArchState × Usize) => i.val - x.2.val) (fun x => x.2.val ≤ i.val)
  · intro x hx
    obtain ⟨st, c1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.endpoint_reinit_one_loop0.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · exact hc

-- FN arch.cm4.entry.endpoint_reinit_one_loop1
@[step] theorem endpoint_reinit_one_loop1_t (state : arch.cm4.plan.ArchState) (rbase r : Usize)
    (hb : rbase.val + 4 ≤ 32) (hr : r.val ≤ 4) :
    arch.cm4.entry.endpoint_reinit_one_loop1 state rbase r ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.endpoint_reinit_one_loop1
  apply loop.spec_decr_nat (fun (x : arch.cm4.plan.ArchState × Usize) => 4 - x.2.val) (fun x => x.2.val ≤ 4)
  · intro x hx
    obtain ⟨st, r1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.endpoint_reinit_one_loop1.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · exact hr

-- FN arch.cm4.entry.endpoint_reinit_one
@[step] theorem endpoint_reinit_one_t (state : arch.cm4.plan.ArchState) (e : Usize) (he : e.val < 8) :
    arch.cm4.entry.endpoint_reinit_one state e ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.endpoint_reinit_one
  simp only [max_roles]
  step as ⟨ i, hi ⟩
  have hi' : i.val = 16 := by scalar_tac
  have hm : e.val * i.val ≤ 112 := by rw [hi']; omega
  step as ⟨ base, hbase ⟩
  refine WP.spec_bind (endpoint_reinit_one_loop0_t i state base 0#usize (by scalar_tac)
    (by scalar_tac)) (fun st1 _ => ?_)
  step as ⟨ rbase, hrbase ⟩
  exact endpoint_reinit_one_loop1_t st1 rbase 0#usize (by scalar_tac) (by scalar_tac)

-- FN arch.cm4.entry.rejection_core
@[step] theorem rejection_core_t [KOpsT ki okT TN TP] [GhostT] (k : K) (state : arch.cm4.plan.ArchState) (cfg : arch.cm4.plan.EndpointCfg) (e : Std.Usize) (role : Std.Usize) («from» : Std.Usize) (hk : okT k)
    (he : e.val < 8) (hr : role.val < 4) (hf : «from».val < 4) :
    arch.cm4.entry.rejection_core ki k state cfg e role «from» ⦃ r => okT r.1 ⦄ := by
  unfold arch.cm4.entry.rejection_core
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; done)
    | (simp_all; scalar_tac)

-- FN arch.cm4.entry.violation_core
set_option maxHeartbeats 4000000 in
@[step] theorem violation_core_t [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (tables : T) (state : arch.cm4.plan.ArchState) (arg : Std.U32) (hk : okT k) :
    arch.cm4.entry.violation_core ki ti k tables state arg ⦃ r => okT r.1 ⦄ := by
  unfold arch.cm4.entry.violation_core
  simp only [max_roles, max_endpoints]
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; done)
    | (simp_all; scalar_tac)

-- FN arch.cm4.entry.endpoint_args
set_option maxHeartbeats 4000000 in
@[step] theorem endpoint_args_t [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (tables : T) (arg : Std.U32) (ptr : Std.U32) (len : Std.U32) (hk : okT k) :
    arch.cm4.entry.endpoint_args ki ti k tables arg ptr len ⦃ r => match r with
      | none => True
      | some (e, role, peer, cfg) => e.val < 8 ∧ role.val < 4 ∧ peer.val < 4 ∧
          1 ≤ cfg.capacity.val ∧ cfg.roles.val ≤ 4 ⦄ := by
  unfold arch.cm4.entry.endpoint_args
  simp only [max_roles, max_endpoints]
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; done)
    | (simp_all; scalar_tac)
    | (refine ⟨?_, ?_, ?_, ?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))

-- FN arch.cm4.entry.copy_in_checked
@[step] theorem copy_in_checked_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (verus_tmp_p : verus_builtin.Ghost verus_builtin.int) (dst : Std.U32) (src : Std.U32) (len : Std.U32) (hk : okT k) :
    arch.cm4.entry.copy_in_checked ki ai ti k hw tables verus_tmp_p dst src len ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.copy_in_checked
  ctac

-- FN arch.cm4.entry.copy_out_checked
@[step] theorem copy_out_checked_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (verus_tmp_p : verus_builtin.Ghost verus_builtin.int) (dst : Std.U32) (src : Std.U32) (len : Std.U32) (hk : okT k) :
    arch.cm4.entry.copy_out_checked ki ai ti k hw tables verus_tmp_p dst src len ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.copy_out_checked
  ctac

-- FN arch.cm4.entry.endpoint_slot
@[step] theorem endpoint_slot_t [TabT ti TN TP] (tables : T) (cfg : arch.cm4.plan.EndpointCfg) (d : Std.Usize) (slot : Std.U8)
    (hd : d.val ≤ 65536) :
    arch.cm4.entry.endpoint_slot ti tables cfg d slot ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.endpoint_slot
  have hp : cfg.payload.val ≤ 65535 := by scalar_tac
  have hc : cfg.capacity.val ≤ 255 := by scalar_tac
  have hs : slot.val ≤ 255 := by scalar_tac
  have h1 : cfg.capacity.val * cfg.payload.val ≤ 255 * 65535 := Nat.mul_le_mul hc hp
  have h2 : d.val * (cfg.capacity.val * cfg.payload.val) ≤ 65536 * (255 * 65535) :=
    Nat.mul_le_mul hd h1
  have h3 : slot.val * cfg.payload.val ≤ 255 * 65535 := Nat.mul_le_mul hs hp
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; done)
    | (simp_all; scalar_tac)

-- FN arch.cm4.entry.endpoint_send_core
set_option maxHeartbeats 4000000 in
@[step] theorem endpoint_send_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (arg : Std.U32) (ptr : Std.U32) (len : Std.U32) (hk : okT k) :
    arch.cm4.entry.endpoint_send_core ki ai ti k hw tables state arg ptr len ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.endpoint_send_core
  refine WP.spec_bind (endpoint_args_t (ki := ki) (ti := ti) k tables arg ptr len hk) (fun o ho => ?_)
  cases o with
  | none => simp only; ctac
  | some a =>
    obtain ⟨e, role, peer, cfg⟩ := a
    simp only at ho ⊢
    obtain ⟨he, hr, hp, hc, hro⟩ := ho
    have hm : ∀ (x : Usize), x.val < 4 → x.val * cfg.roles.val ≤ 16 := fun x hx =>
      Nat.mul_le_mul (Nat.le_of_lt hx) hro |>.trans (by omega)
    have hm1 := hm role hr
    have hm2 := hm peer hp
    ctac
    all_goals first
      | (scalar_tac; done)
      | assumption
      | (simp_all; done)
      | (simp_all; scalar_tac)

-- FN arch.cm4.entry.endpoint_recv_core
set_option maxHeartbeats 4000000 in
@[step] theorem endpoint_recv_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (arg : Std.U32) (ptr : Std.U32) (len : Std.U32) (hk : okT k) :
    arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables state arg ptr len ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.endpoint_recv_core
  refine WP.spec_bind (endpoint_args_t (ki := ki) (ti := ti) k tables arg ptr len hk) (fun o ho => ?_)
  cases o with
  | none => simp only; ctac
  | some a =>
    obtain ⟨e, role, «from», cfg⟩ := a
    simp only at ho ⊢
    obtain ⟨he, hr, hp, hc, hro⟩ := ho
    have hm : ∀ (x : Usize), x.val < 4 → x.val * cfg.roles.val ≤ 16 := fun x hx =>
      Nat.mul_le_mul (Nat.le_of_lt hx) hro |>.trans (by omega)
    have hm1 := hm role hr
    have hm2 := hm «from» hp
    ctac
    all_goals first
      | (scalar_tac; done)
      | assumption
      | (simp_all; done)
      | (simp_all; scalar_tac)

-- FN arch.cm4.entry.start_core
@[step] theorem start_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (hk : okT k) :
    arch.cm4.entry.start_core ki ai ti k hw tables state ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.start_core
  ctac

-- FN arch.cm4.plan.frame_in_stack
@[step] theorem frame_in_stack_t (psp : Std.U32) (bottom : Std.U32) (top : Std.U32) :
    arch.cm4.plan.frame_in_stack psp bottom top ⦃ r => True ⦄ := by
  unfold arch.cm4.plan.frame_in_stack
  ctac

-- FN arch.cm4.entry.psp_in_stack
@[step] theorem psp_in_stack_t [TabT ti TN TP] (tables : T) (state : arch.cm4.plan.ArchState) (psp : Std.U32)
    (hcp : state.current_partition = 255#u8 ∨ state.current_partition.val < TP) :
    arch.cm4.entry.psp_in_stack ti tables state psp ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.psp_in_stack
  ctac

-- FN arch.cm4.entry.svc_of_ended_job_core
@[step] theorem svc_of_ended_job_core_t [HwT ai] [KOpsT ki okT TN TP] [GhostT] (k : K) (hw : A) (depth_before : Std.Usize) (hk : okT k) :
    arch.cm4.entry.svc_of_ended_job_core ki ai k hw depth_before ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.svc_of_ended_job_core
  ctac

-- FN arch.cm4.plan.top_active_loop
@[step] theorem top_active_loop_t (levels : Array arch.cm4.plan.LevelRecord 8#usize) (level i : Usize)
    (hl : level.val < 8) (hi : i.val ≤ 8) :
    arch.cm4.plan.top_active_loop levels level i ⦃ r => r.val < 8 ⦄ := by
  unfold arch.cm4.plan.top_active_loop
  apply loop.spec_decr_nat (fun (x : Usize × Usize) => 8 - x.2.val) (fun x => x.1.val < 8 ∧ x.2.val ≤ 8)
  · intro x hx
    obtain ⟨l1, i1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.plan.top_active_loop.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · exact ⟨hl, hi⟩

-- FN arch.cm4.plan.top_active
@[step] theorem top_active_t (levels : Array arch.cm4.plan.LevelRecord 8#usize) :
    arch.cm4.plan.top_active levels ⦃ r => r.val < 8 ⦄ :=
  top_active_loop_t levels 0#usize 1#usize (by scalar_tac) (by scalar_tac)

-- FN arch.cm4.entry.end_plan_core
@[step] theorem end_plan_core_t [KOpsT ki okT TN TP] [GhostT] (k : K) (state : arch.cm4.plan.ArchState) (depth_before : Std.Usize) (verus_tmp_k0 : verus_builtin.Ghost K) (verus_tmp_s0 : verus_builtin.Ghost arch.cm4.plan.ArchState) (hk : okT k) :
    arch.cm4.entry.end_plan_core ki k state depth_before verus_tmp_k0 verus_tmp_s0 ⦃ r => True ⦄ := by
  unfold arch.cm4.entry.end_plan_core
  ctac

-- FN arch.cm4.entry.stack_violation_core
@[step] theorem stack_violation_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (hk : okT k) :
    arch.cm4.entry.stack_violation_core ki ai ti k hw tables state ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.stack_violation_core
  ctac

-- FN arch.cm4.plan.plan_frame
@[step] theorem plan_frame_t (floor : Std.U32) (bottom : Std.U32) (top : Std.U32) :
    arch.cm4.plan.plan_frame floor bottom top ⦃ r => True ⦄ := by
  unfold arch.cm4.plan.plan_frame
  ctac

-- FN arch.cm4.entry.dispatch_job_core
set_option maxHeartbeats 16000000 in
@[step] theorem dispatch_job_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (n : Std.U16) (t : Std.U8) (psp : Std.U32) (control : Std.U32) (exc_return : Std.U32) (hk : okT k) :
    arch.cm4.entry.dispatch_job_core ki ai ti k hw tables state n t psp control exc_return ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.dispatch_job_core
  simp only [max_floors]
  ctac
  all_goals first
    | (scalar_tac; done)
    | assumption
    | (simp_all; done)
    | (simp_all; scalar_tac)

-- FN arch.cm4.entry.dispatch_vector_core_loop0
@[step] theorem dispatch_vector_core_loop0_t (a : Array U16 8#usize) (n : U16) (slot i : Usize) (hi : i.val ≤ 8)
    (hs : slot.val ≤ 8) :
    arch.cm4.entry.dispatch_vector_core_loop0 a n slot i ⦃ r => r.val ≤ 8 ⦄ := by
  unfold arch.cm4.entry.dispatch_vector_core_loop0
  apply loop.spec_decr_nat (fun (x : Usize × Usize) => 8 - x.2.val) (fun x => x.1.val ≤ 8 ∧ x.2.val ≤ 8)
  · intro x hx
    obtain ⟨sl, i1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.dispatch_vector_core_loop0.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · exact ⟨hs, hi⟩

-- FN arch.cm4.entry.dispatch_vector_core_loop1
@[step] theorem dispatch_vector_core_loop1_t (state : arch.cm4.plan.ArchState) (n : U16) (slot i : Usize) (hi : i.val ≤ 8)
    (hs : slot.val ≤ 8) :
    arch.cm4.entry.dispatch_vector_core_loop1 state n slot i ⦃ r => r.2.val ≤ 8 ⦄ := by
  unfold arch.cm4.entry.dispatch_vector_core_loop1
  apply loop.spec_decr_nat (fun (x : arch.cm4.plan.ArchState × Usize × Usize) => 8 - x.2.2.val) (fun x => x.2.1.val ≤ 8 ∧ x.2.2.val ≤ 8)
  · intro x hx
    obtain ⟨st, sl, i1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.dispatch_vector_core_loop1.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · exact ⟨hs, hi⟩

-- FN arch.cm4.entry.dispatch_vector_core_loop2
@[step] theorem dispatch_vector_core_loop2_t (state : arch.cm4.plan.ArchState) (n : U16) (slot i : Usize) (hi : i.val ≤ 8)
    (hs : slot.val ≤ 8) :
    arch.cm4.entry.dispatch_vector_core_loop2 state n slot i ⦃ r => r.2.val ≤ 8 ⦄ := by
  unfold arch.cm4.entry.dispatch_vector_core_loop2
  apply loop.spec_decr_nat (fun (x : arch.cm4.plan.ArchState × Usize × Usize) => 8 - x.2.2.val) (fun x => x.2.1.val ≤ 8 ∧ x.2.2.val ≤ 8)
  · intro x hx
    obtain ⟨st, sl, i1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.dispatch_vector_core_loop2.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · exact ⟨hs, hi⟩

-- FN arch.cm4.entry.dispatch_vector_core
set_option maxHeartbeats 16000000 in
@[step] theorem dispatch_vector_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (n : Std.U16) (t : Std.U8) (psp : Std.U32) (control : Std.U32) (exc_return : Std.U32) (hk : okT k)
    (hcp : state.current_partition = 255#u8 ∨ state.current_partition.val < TP) :
    arch.cm4.entry.dispatch_vector_core ki ai ti k hw tables state n t psp control exc_return ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.dispatch_vector_core
  simp only [arch.cm4.entry.WINDOW_DEFER, ↓reduceIte]
  ptac
  all_goals first
    | (scalar_tac; done)
    | assumption
    | (simp_all; done)
    | (simp_all; scalar_tac)

-- FN arch.cm4.entry.intake_core
@[step] theorem intake_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (n : Std.U16) (task : Std.U8) (partition : Std.U8) (psp : Std.U32) (exc_return : Std.U32) (hk : okT k)
    (hcp : state.current_partition = 255#u8 ∨ state.current_partition.val < TP) :
    arch.cm4.entry.intake_core ki ai ti k hw tables state n task partition psp exc_return ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.intake_core
  ctac

-- FN arch.cm4.entry.unbound_core
@[step] theorem unbound_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (n : Std.U16) (hk : okT k) :
    arch.cm4.entry.unbound_core ki ai ti k hw tables state n ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.unbound_core
  ctac

-- FN arch.cm4.entry.timer_core
@[step] theorem timer_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (hk : okT k) :
    arch.cm4.entry.timer_core ki ai ti k hw tables state ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.timer_core
  ctac

-- FN arch.cm4.entry.irq_entry_core_loop
@[step] theorem irq_entry_core_loop_t [HwT ai] (hw : A) (state : arch.cm4.plan.ArchState) (i : Usize) (hi : i.val ≤ 8) :
    arch.cm4.entry.irq_entry_core_loop ai hw state i ⦃ r =>
      r.2.current_partition = state.current_partition ⦄ := by
  unfold arch.cm4.entry.irq_entry_core_loop
  apply loop.spec_decr_nat (fun (x : A × arch.cm4.plan.ArchState × Usize) => 8 - x.2.2.val)
    (fun x => x.2.2.val ≤ 8 ∧ x.2.1.current_partition = state.current_partition)
  · intro x hx
    obtain ⟨hw1, st, i1⟩ := x
    try simp only at hx
    try simp only
    unfold arch.cm4.entry.irq_entry_core_loop.body
    try simp only [max_roles, max_endpoints, max_floors]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (simp_all; done)
      | (refine ⟨?_, ?_⟩ <;> first | (scalar_tac; done) | (simp_all; done) | (simp_all; scalar_tac))
      | (simp_all; scalar_tac)
  · exact ⟨hi, rfl⟩

-- FN arch.cm4.entry.irq_entry_core
set_option maxHeartbeats 16000000 in
@[step] theorem irq_entry_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (ipsr : Std.U32) (psp : Std.U32) (control : Std.U32) (exc_return : Std.U32) (hk : okT k)
    (hcp : state.current_partition = 255#u8 ∨ state.current_partition.val < TP) :
    arch.cm4.entry.irq_entry_core ki ai ti k hw tables state ipsr psp control exc_return ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.irq_entry_core
  simp only [arch.cm4.entry.WINDOW_DEFER, ↓reduceIte]
  ptac
  all_goals first
    | (scalar_tac; done)
    | assumption
    | (simp_all; done)
    | (simp_all; scalar_tac)

-- FN arch.cm4.entry.svc_service_core
set_option maxHeartbeats 16000000 in
@[step] theorem svc_service_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (frame : Array Std.U32 8#usize) (number : Std.U8) (hk : okT k) :
    arch.cm4.entry.svc_service_core ki ai ti k hw tables state frame number ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.svc_service_core
  ptac
  all_goals first
    | (scalar_tac; done)
    | assumption
    | (simp_all; done)
    | (simp_all; scalar_tac)

-- FN arch.cm4.entry.svc_core
@[step] theorem svc_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (frame : Array Std.U32 8#usize) (number : Std.U8) (hk : okT k) :
    arch.cm4.entry.svc_core ki ai ti k hw tables state frame number ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.svc_core
  ctac

-- FN arch.cm4.entry.epilogue_core
set_option maxHeartbeats 16000000 in
@[step] theorem epilogue_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (n : Std.U32) (hk : okT k) :
    arch.cm4.entry.epilogue_core ki ai ti k hw tables state n ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.epilogue_core
  simp only [max_floors]
  ctac
  all_goals first
    | (scalar_tac; done)
    | assumption
    | (simp_all; done)
    | (simp_all; scalar_tac)

-- FN arch.cm4.ghostv.no_job_in_progress
@[step] theorem no_job_in_progress_t [KOpsT ki okT TN TP] [GhostT] (k : K) (hk : okT k) :
    arch.cm4.ghostv.no_job_in_progress ki k ⦃ r => True ⦄ := by
  unfold arch.cm4.ghostv.no_job_in_progress
  ctac

-- FN arch.cm4.entry.fault_core
@[step] theorem fault_core_t [HwT ai] [KOpsT ki okT TN TP] [TabT ti TN TP] [GhostT] (k : K) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (kind : Std.U8) (in_kernel : Bool) (hk : okT k) :
    arch.cm4.entry.fault_core ki ai ti k hw tables state kind in_kernel ⦃ r => okT r.2.1 ⦄ := by
  unfold arch.cm4.entry.fault_core
  ctac

end P18