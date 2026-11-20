import Sched.GhostTotal
open Aeneas Aeneas.Std Result

/-! # The entry cores return on the Kernel that is linked (2026-11-20)

Property 21 for Kernel_Arch's entry cores, with the Kernel behind the trait
the real one (`IsBoundT`): from a state satisfying `BX` every core returns
and leaves `BX`; the boot returns from any state and, when it passes its
checks, establishes `BX`.

Assumed, and named in every statement: `HwT` (the hardware operations
return) and `TabT` (the generated tables' accessors return; the four that
index a table with their argument, for an argument in range,
`Tab/Proofs.lean`). Nothing is assumed of the Kernel's own code: the ghost
helpers and `init` are proven to return (`Sched/GhostTotal.lean`,
`Sched/InitTotal.lean`). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize} {A T : Type} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)}

/-- **`svc_service_core` returns on the Kernel that is linked** and keeps `BX`. -/
theorem svc_service_core_real [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (frame : Array U32 8#usize) (number : U8)
    (hk : BX k) :
    arch.cm4.entry.svc_service_core ki ai ti k hw tables state frame number ⦃ r => BX r.2.1 ⦄ := by
  haveI := bound_kopst_nh hb (nohist NT NR NP NL NS NE)
  exact svc_service_core_t k hw tables state frame number hk

/-- **`irq_entry_core` returns on the Kernel that is linked** and keeps `BX`. -/
theorem irq_entry_core_real [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (ipsr psp control exc_return : U32)
    (hk : BX k)
    (hcp : state.current_partition = 255#u8 ∨ state.current_partition.val < NP.val) :
    arch.cm4.entry.irq_entry_core ki ai ti k hw tables state ipsr psp control exc_return ⦃ r => BX r.2.1 ⦄ := by
  haveI := bound_kopst_nh hb (nohist NT NR NP NL NS NE)
  exact irq_entry_core_t k hw tables state ipsr psp control exc_return hk hcp

/-- **`dispatch_vector_core` returns on the Kernel that is linked** and keeps `BX`. -/
theorem dispatch_vector_core_real [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (n : U16) (t : U8) (psp control exc_return : U32)
    (hk : BX k)
    (hcp : state.current_partition = 255#u8 ∨ state.current_partition.val < NP.val) :
    arch.cm4.entry.dispatch_vector_core ki ai ti k hw tables state n t psp control exc_return ⦃ r => BX r.2.1 ⦄ := by
  haveI := bound_kopst_nh hb (nohist NT NR NP NL NS NE)
  exact dispatch_vector_core_t k hw tables state n t psp control exc_return hk hcp

/-- **`epilogue_core` returns on the Kernel that is linked** and keeps `BX`. -/
theorem epilogue_core_real [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (n : U32)
    (hk : BX k) :
    arch.cm4.entry.epilogue_core ki ai ti k hw tables state n ⦃ r => BX r.2.1 ⦄ := by
  haveI := bound_kopst_nh hb (nohist NT NR NP NL NS NE)
  exact epilogue_core_t k hw tables state n hk

/-- **`fault_core` returns on the Kernel that is linked** and keeps `BX`. -/
theorem fault_core_real [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (kind : U8) (in_kernel : Bool)
    (hk : BX k) :
    arch.cm4.entry.fault_core ki ai ti k hw tables state kind in_kernel ⦃ r => BX r.2.1 ⦄ := by
  haveI := bound_kopst_nh hb (nohist NT NR NP NL NS NE)
  exact fault_core_t k hw tables state kind in_kernel hk

/-- **`start_core` returns on the Kernel that is linked** and keeps `BX`. -/
theorem start_core_real [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) 
    (hk : BX k) :
    arch.cm4.entry.start_core ki ai ti k hw tables state  ⦃ r => BX r.2.1 ⦄ := by
  haveI := bound_kopst_nh hb (nohist NT NR NP NL NS NE)
  exact start_core_t k hw tables state  hk

/-! ## The Partition taken as executing is one of the layout's

`irq_entry_core` and `dispatch_vector_core` read the executing Partition's
stack bounds from the tables. Under the stack and record invariant (`AI`,
`Sched/ArchKeep.lean`) that Partition is 255 (none) or the Partition of the
topmost record's Task, which is below `NP`. -/

theorem partOf_lt {b : arch.bound.Bound NT NR NP NL NS NE} (hb : BX b) (j : Nat) : partOf b.cfg j < NP.val := by
  have hlen : b.cfg.tasks.val.length = NT.val := b.cfg.tasks.property
  unfold partOf
  by_cases hj : j < b.cfg.tasks.val.length
  · rw [List.getElem?_eq_getElem hj]
    exact hb.bt.ct.ci.part j hj
  · rw [List.getElem?_eq_none (by omega)]
    exact hb.np

theorem top_exists (L : Arch.LV) : ∀ n, (∀ j, n ≤ j → L.act j = false) → (∃ l, L.act l = true) →
    ∃ l, Arch.IsTop L l := by
  intro n
  induction n with
  | zero =>
    rintro h ⟨l, hl⟩
    rw [h l (Nat.zero_le _)] at hl
    cases hl
  | succ n ih =>
    intro h hex
    by_cases hn : L.act n = true
    · exact ⟨n, hn, fun j hj => h j (by omega)⟩
    · refine ih (fun j hj => ?_) hex
      rcases Nat.eq_or_lt_of_le hj with e | hlt
      · subst e; simpa using hn
      · exact h j (by omega)

theorem ai_cp {b : arch.bound.Bound NT NR NP NL NS NE} {s : arch.cm4.plan.ArchState} {hop : Nat} (hb : BX b)
    (h : AI kvB b s hop) :
    s.current_partition = 255#u8 ∨ s.current_partition.val < NP.val := by
  obtain ⟨⟨_, _, _, h255, htop⟩, _⟩ := h
  by_cases hall : ∀ l, (lvOf s.levels).act l = false
  · left
    have := h255 hall
    scalar_tac
  · right
    have hex : ∃ l, (lvOf s.levels).act l = true := by
      by_contra hne
      exact hall (fun l => by
        by_contra hl
        exact hne ⟨l, by simpa using hl⟩)
    obtain ⟨l, hl⟩ := top_exists (lvOf s.levels) 8 (fun j hj => by simp [lvOf]; omega) hex
    rw [htop l hl]
    exact partOf_lt hb _

/-- `irq_entry_core` on the Kernel that is linked, from the stack and record
invariant. -/
theorem irq_entry_core_real_ai [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (ipsr psp control exc_return : U32)
    (hk : BX k) {hop : Nat} (hai : AI kvB k state hop) :
    arch.cm4.entry.irq_entry_core ki ai ti k hw tables state ipsr psp control exc_return ⦃ r => BX r.2.1 ⦄ :=
  irq_entry_core_real hb k hw tables state ipsr psp control exc_return hk (ai_cp hk hai)

/-- `dispatch_vector_core` on the Kernel that is linked, from the stack and record
invariant. -/
theorem dispatch_vector_core_real_ai [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) (n : U16) (t : U8) (psp control exc_return : U32)
    (hk : BX k) {hop : Nat} (hai : AI kvB k state hop) :
    arch.cm4.entry.dispatch_vector_core ki ai ti k hw tables state n t psp control exc_return ⦃ r => BX r.2.1 ⦄ :=
  dispatch_vector_core_real hb k hw tables state n t psp control exc_return hk (ai_cp hk hai)

/-- **`boot_core` returns on the Kernel that is linked**, from any state,
and a boot that passes its checks leaves `BX`. -/
theorem boot_core_real [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    (k : arch.bound.Bound NT NR NP NL NS NE) (hw : A) (tables : T) (state : arch.cm4.plan.ArchState) :
    arch.cm4.entry.boot_core ki ai ti k hw tables state ⦃ r => r.1 = true → BX r.2.1 ⦄ := by
  haveI := bound_kopst_nh hb (nohist NT NR NP NL NS NE)
  exact boot_core_t k hw tables state

end P18
