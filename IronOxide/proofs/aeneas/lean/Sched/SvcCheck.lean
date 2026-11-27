-- The one definition Aeneas adds to `Sched/Kernel.lean` when
-- `crate::arch::cm4::entry::svc_check_core` is added to the translation's
-- start set (proofs/aeneas/README.md). The regenerated file differed from
-- `Sched/Kernel.lean` in this definition only (and in the line numbers of
-- the `Source:` comments), so it stands here, over the definitions of
-- `Sched/Kernel.lean`, and that file is as it was. Not written by hand.
module
public import Sched.Kernel
@[expose] public section
open Aeneas Aeneas.Std Result ControlFlow Error
set_option linter.dupNamespace false
set_option linter.hashCommand false
set_option linter.unusedVariables false
set_option linter.style.whitespace false
set_option linter.style.setOption false
set_option linter.style.longLine false
set_option linter.internalConstructors false

namespace rsk_kernel

/-- [rsk_kernel::arch::cm4::entry::svc_check_core]:
    Source: 'crates/rsk-kernel/src/arch/cm4/entry.rs', lines 2879:0-2907:1
    Visibility: public -/
def arch.cm4.entry.svc_check_core
  {K : Type _} {A : Type _} {T : Type _} (boundKernelOpsInst :
  arch.bound.KernelOps K) (KernelArchInst : arch.cm4.entry.KernelArch A)
  (TablesInst : arch.cm4.entry.Tables T) (k : K) (hw : A) (tables : T)
  (state : arch.cm4.plan.ArchState) (psp : Std.U32) (exc_return : Std.U32) :
  Result ((Option (Std.U32 × Bool)) × K × A × arch.cm4.plan.ArchState)
  := do
  let i ← lift (exc_return &&& 4#u32)
  if i = 0#u32
  then ok (none, k, hw, state)
  else
    if state.current_partition = 255#u8
    then ok (none, k, hw, state)
    else
      let b ← arch.cm4.entry.psp_in_stack TablesInst tables state psp
      if b
      then ok (none, k, hw, state)
      else
        let ((hop, safe), k1, hw1, state1) ←
          arch.cm4.entry.stack_violation_core boundKernelOpsInst KernelArchInst
            TablesInst k hw tables state
        let hw2 ← KernelArchInst.stacking_fault_discard hw1
        ok (some (hop, safe), k1, hw2, state1)

end rsk_kernel
