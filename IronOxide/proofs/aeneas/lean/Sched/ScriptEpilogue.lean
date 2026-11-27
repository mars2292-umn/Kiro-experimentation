import Sched.Script
open Aeneas Aeneas.Std Result

/-! # The order of an unwind's calls (2026-11-27)

`epilogue_core`'s theorem of `Sched/Script.lean`, in a file of its own:
the core is the longest and its paths take the longest to read. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

section script
variable {K A T : Type _}
  (ki : _root_.rsk_kernel.arch.bound.KernelOps K)
  (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A)
  (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T)

set_option maxHeartbeats 8000000 in
/-- **An unwind** (the end of a Job, or of a killed Job's handler): nothing;
a fault and `apply_core`; `job_return` and `apply_core`; or `job_return`,
`apply_core`, and then a fault on the result and `apply_core` again (the
level record of the Task resumed names no exception). -/
theorem epilogue_core_sh {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.epilogue_core ki ai ti k hw tables state n = ok (r, k', hw', st')) :
    En ki [[], [.fault, .apply], [.ret, .apply], [.ret, .apply, .fault, .apply]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.epilogue_core at h
  peel h
  all_goals first
    | exact en_plain (by decide)
    | exact en_wa (by decide) (.fault _ _ _ _ (by assumption)) (by assumption)
    | exact en_wa (by decide) (.ret _ _ _ (by assumption)) (by assumption)
    | exact en_wawa (by decide) (.ret _ _ _ (by assumption)) (by assumption)
        (.fault _ _ _ _ (by assumption)) (by assumption)

end script

end P18
