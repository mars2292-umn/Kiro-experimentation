import Tab.Tables
open Aeneas Aeneas.Std Result

/-! # For which arguments the generated table accessors return (2026-11-20)

`Tab/Tables.lean` is the P1 demo's `impl System for Sys` as rustc compiles
it (the QEMU configuration). Kernel_Arch reaches it through `Tables`
(`SysTables`, `arch/cm4/mod.rs`), whose methods Verus trusts.

Every accessor translated here returns for every argument: `dma`, `signal`,
`dispatch_vector`, `intake_vector`, `stack_top` and `stack_bottom` test
their argument against the table's length, and `binding` walks the Tasks.
Until 2026-11-20 the last four of those six indexed their table with the
argument and failed outside its range (the Generator now emits the test);
`Sched/CoreTotal.lean` proves besides that the entry cores call them in
range.

Not here: the content of the Endpoint table (five Endpoints of nested
arrays, which Lean does not elaborate in reasonable time or memory; the
constant is opaque in the translation and `endpoint_t` assumes only that
it evaluates), and `endpoint_storage`, which reads an address through a
pointer cast of the Kernel_Unsafe_Module (opaque too). -/

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

theorem np_eq : NP = 3#usize := by unfold NP; rfl

/-- `dispatch_vector` returns for every Task identifier (0 for one that is
not a Task's). -/
theorem dispatch_vector_t (t : U8) : Sys.Insts.Rsk_kernelArchSystemBound73362332.dispatch_vector t ⦃ _ => True ⦄ := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.dispatch_vector
  simp only [nt_eq]
  step as ⟨i, hi⟩
  split
  · rename_i hlt
    step as ⟨i1, hi1⟩
    step as ⟨i2, hi2⟩
    have := dirq_le i1.val (by have := DISPATCH_IRQ.property; scalar_tac)
    rw [← hi2] at this
    step*
  · step*

/-- `intake_vector` returns for every Task identifier. -/
theorem intake_vector_t (t : U8) : Sys.Insts.Rsk_kernelArchSystemBound73362332.intake_vector t ⦃ _ => True ⦄ := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.intake_vector
  simp only [nt_eq]
  step as ⟨i, hi⟩
  split
  · rename_i hlt
    step as ⟨i1, hi1⟩
    step as ⟨i2, hi2⟩
    have h := iirq_le i1.val (by have := INTAKE_IRQ.property; scalar_tac)
    rw [← hi2] at h
    split
    · step*
    · rename_i hne
      step as ⟨i3, hi3⟩
      step as ⟨i4, hi4⟩
      have e : i4 = i2 := by subst hi1; subst hi3; rw [hi4, hi2]
      rcases h with h | h
      · exact absurd h hne
      · rw [e]
        step*
  · step*

/-- `stack_top` returns for every Partition identifier. -/
theorem stack_top_t (p : U8) : Sys.Insts.Rsk_kernelArchSystemBound73362332.stack_top p ⦃ _ => True ⦄ := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.stack_top
  simp only [np_eq]
  step as ⟨i, hi⟩
  split
  · rename_i hlt
    step as ⟨i1, hi1⟩
    step as ⟨pl, hpl⟩
    have := stack_le i1.val (by have := LAYOUT.partitions.property; scalar_tac)
    rw [← hpl] at this
    step*
  · step*

/-- `stack_bottom` returns for every Partition identifier. -/
theorem stack_bottom_t (p : U8) : Sys.Insts.Rsk_kernelArchSystemBound73362332.stack_bottom p ⦃ _ => True ⦄ := by
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.stack_bottom
  simp only [np_eq]
  step as ⟨i, hi⟩
  split
  · step*
  · step*

theorem nep_eq : NEP = 5#usize := by unfold NEP; rfl

/-- `endpoint` returns for every index, given that the table constant
evaluates (it is opaque here). -/
theorem endpoint_t (h : ∃ a, ENDPOINTS = ok a) (i : Usize) : Sys.Insts.Rsk_kernelArchSystemBound73362332.endpoint i ⦃ _ => True ⦄ := by
  obtain ⟨a, ha⟩ := h
  unfold Sys.Insts.Rsk_kernelArchSystemBound73362332.endpoint
  simp only [nep_eq]
  split
  · rw [ha]
    simp only [bind_ok]
    step*
  · step*

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
