import Tab.Tables
open Aeneas Aeneas.Std Result

/-! # For which arguments the generated table accessors return (2026-11-20)

`Tab/Tables.lean` is the P1 demo's `impl System for Sys` as rustc compiles
it (the QEMU configuration). Kernel_Arch reaches it through `Tables`
(`SysTables`, `arch/cm4/mod.rs`), whose methods Verus trusts.

- `dma`, `signal`, `binding` return for every argument (`dma` and `signal`
  test their index against the table's length; `binding` walks the Tasks).
- `dispatch_vector`, `intake_vector` return for a Task identifier below
  `NT` and for no other; `stack_top` and `stack_bottom` for a Partition
  below `NP` and for no other: they index their table with the argument.
  Kernel_Arch must therefore call them in range, which
  `Sched/CoreTotal.lean` proves of the entry cores.

Not here: `endpoint` (its table, five Endpoints of nested arrays, does not
elaborate in reasonable time or memory; the accessor tests its index like
`dma`), and `endpoint_storage`, which reads an address through a pointer
cast of the Kernel_Unsafe_Module (opaque in the translation). -/

namespace rsk_system

theorem dirq_le (j : Nat) (h : j < DISPATCH_IRQ.val.length) : (DISPATCH_IRQ.val[j]).val ≤ 26 := by
  have h7 : j < 7 := by have := DISPATCH_IRQ.property; scalar_tac
  revert h
  unfold DISPATCH_IRQ
  intro h
  obtain rfl | rfl | rfl | rfl | rfl | rfl | rfl : j = 0 ∨ j = 1 ∨ j = 2 ∨ j = 3 ∨ j = 4 ∨ j = 5 ∨ j = 6 := by
    omega
  all_goals simp [Array.make]

theorem iirq_le (j : Nat) (h : j < INTAKE_IRQ.val.length) :
    INTAKE_IRQ.val[j] = 65535#u16 ∨ (INTAKE_IRQ.val[j]).val ≤ 26 := by
  have h7 : j < 7 := by have := INTAKE_IRQ.property; scalar_tac
  revert h
  unfold INTAKE_IRQ
  intro h
  obtain rfl | rfl | rfl | rfl | rfl | rfl | rfl : j = 0 ∨ j = 1 ∨ j = 2 ∨ j = 3 ∨ j = 4 ∨ j = 5 ∨ j = 6 := by
    omega
  all_goals simp [Array.make]

theorem stack_le (j : Nat) (h : j < LAYOUT.partitions.val.length) :
    (LAYOUT.partitions.val[j]).ram.base.val + (LAYOUT.partitions.val[j]).stack_size.val
      ≤ 4294967295 := by
  have h3 : j < 3 := by have := LAYOUT.partitions.property; scalar_tac
  revert h
  unfold LAYOUT
  intro h
  obtain rfl | rfl | rfl : j = 0 ∨ j = 1 ∨ j = 2 := by omega
  all_goals simp [Array.make]

theorem nt_eq : NT = 7#usize := by unfold NT; rfl
theorem nd_eq : ND = 1#usize := by unfold ND; rfl
theorem nsig_eq : NSIG = 1#usize := by unfold NSIG; rfl

theorem timer_vector_eq : Sys.Insts.Rsk_kernelArchSystemBound73362332.TIMER_VECTOR = ok 25#u16 := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.TIMER_VECTOR
  rfl

theorem config_returns : ∃ c, CONFIG = ok c := by
  unfold CONFIG
  simp only [lift, bind_ok]
  exact ⟨_, rfl⟩

/-- `dma` returns for every index. -/
theorem dma_t (i : Usize) : Sys.Insts.Rsk_kernelArchSystemBound73362332.dma i ⦃ _ => True ⦄ := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.dma
  simp only [nd_eq]
  split
  · step*
  · step*

/-- `signal` returns for every index. -/
theorem signal_t (i : Usize) : Sys.Insts.Rsk_kernelArchSystemBound73362332.signal i ⦃ _ => True ⦄ := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.signal
  simp only [nsig_eq]
  split
  · step*
  · step*

/-- `dispatch_vector` returns for a Task identifier below `NT`. -/
theorem dispatch_vector_t (t : U8) (ht : t.val < 7) : Sys.Insts.Rsk_kernelArchSystemBound73362332.dispatch_vector t ⦃ _ => True ⦄ := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.dispatch_vector
  step as ⟨i, hi⟩
  step as ⟨i1, hi1⟩
  have := dirq_le i.val (by have := DISPATCH_IRQ.property; scalar_tac)
  rw [← hi1] at this
  step*

/-- ... and for no other: the table is indexed with the argument. -/
theorem dispatch_vector_oob (t : U8) (ht : 7 ≤ t.val) (v : U16) : Sys.Insts.Rsk_kernelArchSystemBound73362332.dispatch_vector t ≠ ok v := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.dispatch_vector
  have hl : DISPATCH_IRQ.val.length = 7 := by have := DISPATCH_IRQ.property; scalar_tac
  have hn : DISPATCH_IRQ.val[t.val]? = none := List.getElem?_eq_none (by omega)
  simp [lift, Array.index_usize, hn]

/-- `intake_vector` returns for a Task identifier below `NT`. -/
theorem intake_vector_t (t : U8) (ht : t.val < 7) : Sys.Insts.Rsk_kernelArchSystemBound73362332.intake_vector t ⦃ _ => True ⦄ := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.intake_vector
  step as ⟨i, hi⟩
  step as ⟨i1, hi1⟩
  have h := iirq_le i.val (by have := INTAKE_IRQ.property; scalar_tac)
  rw [← hi1] at h
  split
  · step*
  · rename_i hne
    step as ⟨i2, hi2⟩
    step as ⟨i3, hi3⟩
    have h3 := iirq_le i2.val (by have := INTAKE_IRQ.property; scalar_tac)
    rw [← hi3] at h3
    have e : i3 = i1 := by subst hi; subst hi2; rw [hi3, hi1]
    rcases h with h | h
    · exact absurd h hne
    · rw [e]
      step*

theorem intake_vector_oob (t : U8) (ht : 7 ≤ t.val) (v : Option U16) :
    Sys.Insts.Rsk_kernelArchSystemBound73362332.intake_vector t ≠ ok v := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.intake_vector
  have hl : INTAKE_IRQ.val.length = 7 := by have := INTAKE_IRQ.property; scalar_tac
  have hn : INTAKE_IRQ.val[t.val]? = none := List.getElem?_eq_none (by omega)
  simp [lift, Array.index_usize, hn]

/-- `stack_top` returns for a Partition below `NP`. -/
theorem stack_top_t (p : U8) (hp : p.val < 3) : Sys.Insts.Rsk_kernelArchSystemBound73362332.stack_top p ⦃ _ => True ⦄ := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.stack_top
  step as ⟨i, hi⟩
  step as ⟨pl, hpl⟩
  have := stack_le i.val (by have := LAYOUT.partitions.property; scalar_tac)
  rw [← hpl] at this
  step*

theorem stack_top_oob (p : U8) (hp : 3 ≤ p.val) (v : U32) : Sys.Insts.Rsk_kernelArchSystemBound73362332.stack_top p ≠ ok v := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.stack_top
  have hl : LAYOUT.partitions.val.length = 3 := by have := LAYOUT.partitions.property; scalar_tac
  have hn : LAYOUT.partitions.val[p.val]? = none := List.getElem?_eq_none (by omega)
  simp [lift, Array.index_usize, hn]

/-- `stack_bottom` returns for a Partition below `NP`. -/
theorem stack_bottom_t (p : U8) (hp : p.val < 3) : Sys.Insts.Rsk_kernelArchSystemBound73362332.stack_bottom p ⦃ _ => True ⦄ := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.stack_bottom
  step*

theorem stack_bottom_oob (p : U8) (hp : 3 ≤ p.val) (v : U32) : Sys.Insts.Rsk_kernelArchSystemBound73362332.stack_bottom p ≠ ok v := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.stack_bottom
  have hl : LAYOUT.partitions.val.length = 3 := by have := LAYOUT.partitions.property; scalar_tac
  have hn : LAYOUT.partitions.val[p.val]? = none := List.getElem?_eq_none (by omega)
  simp [lift, Array.index_usize, hn]

/-- `binding` returns for every exception number. -/
theorem binding_t (n : U16) : Sys.Insts.Rsk_kernelArchSystemBound73362332.binding n ⦃ _ => True ⦄ := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.binding
  rw [timer_vector_eq]
  simp only [bind_ok]
  split
  · step*
  · unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.binding_loop
    apply loop.spec_decr_nat (fun (x : Usize) => 7 - x.val) (fun _ => True)
    · intro t _
      unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.binding_loop.body
      simp only [nt_eq]
      split
      · rename_i hlt
        step as ⟨i, hi⟩
        have hd := dirq_le t.val (by have := DISPATCH_IRQ.property; scalar_tac)
        rw [← hi] at hd
        step as ⟨i1, hi1⟩
        split
        · step*
        · step as ⟨i2, hi2⟩
          have h2 := iirq_le t.val (by have := INTAKE_IRQ.property; scalar_tac)
          rw [← hi2] at h2
          split
          · rename_i hne
            have h2' : i2.val ≤ 26 := by
              rcases h2 with h | h
              · exact absurd h (by simpa using hne)
              · exact h
            step as ⟨i3, hi3⟩
            split
            · obtain ⟨c, hc⟩ := config_returns
              step as ⟨i4, hi4⟩
              rw [hc]
              simp only [bind_ok]
              step*
            · step*
          · step*
      · step*
    · trivial

end rsk_system
