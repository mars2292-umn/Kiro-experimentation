import Sched.InitTotal
open Aeneas Aeneas.Std Result

/-! # The ghost helpers return (2026-11-20)

In the build without Verus a `Ghost` value is a zero-sized marker. Since
2026-11-20 the seven helpers of `arch/cm4/ghostv.rs` and `Kernel::no_history`
construct it directly in that build (`Ghost::assume_new`), so the
translation has their bodies. They return, which discharges the two
assumptions the entry cores' totality carried about them: the class
`GhostT` (`Sched/CoreTotal.lean`) and `NoHist` (`Sched/InitTotal.lean`). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

theorem assume_new_t (A : Type) : verus_builtin.Ghost.assume_new A ⦃ _ => True ⦄ := by
  unfold verus_builtin.Ghost.assume_new
  exact (WP.spec_ok _).mpr trivial

instance ghostT : GhostT where
  snap := fun x => by unfold arch.cm4.ghostv.ghost_snap; exact assume_new_t _
  int := fun x => by unfold arch.cm4.ghostv.ghost_int; exact assume_new_t _
  partition := fun c r => by unfold arch.cm4.ghostv.ghost_partition; exact assume_new_t _
  seq_empty := by unfold arch.cm4.ghostv.ghost_seq_empty; exact assume_new_t _
  push_u8 := fun g x => by unfold arch.cm4.ghostv.ghost_seq_push_u8; exact assume_new_t _
  push_usize := fun g x => by unfold arch.cm4.ghostv.ghost_seq_push_usize; exact assume_new_t _
  drop_last := fun g => by unfold arch.cm4.ghostv.ghost_seq_drop_last; exact assume_new_t _

/-- **The ghost history's constructor returns.** -/
theorem nohist (NT NR NP NL NS NE : Usize) : NoHist NT NR NP NL NS NE := by
  unfold NoHist logic.kernel.Kernel.no_history
  exact assume_new_t _

end P18
