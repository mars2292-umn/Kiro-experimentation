import Sched.KRun
import Sched.Arrive
import Sched.Sporadic
import Sched.Entries
import Rta.Inputs
open Aeneas Aeneas.Std Result

/-! # Property 18 for the inputs the Analyzer's own check accepts (2026-11-26)

`property18` (`Link.lean`) and the four statements over it
(`property18_kernel`, `property18_timed`, `property18_sporadic`,
`property18_entries`) take five hypotheses about the Analyzer's inputs:
every site names a Task (`hsok`), one window per Task (`hwl`), every Budget
positive (`hbud`), no deadline later than its period or MIT (`hDT`), and
every Priority positive and the configuration's (`hprio`). Until now that
the Analyzer analyses no other input was the Verus contract of
`terms::check_inputs`, matched to these hypotheses by reading.

Here each of the five statements is restated with, in their place, the
answer of the compiled `check_inputs` on the same inputs
(`rsk_analyzer.checked_inputs`, `Rta/Inputs.lean`): `hcheck`, that it
returned `None`. `inputs::prepare` calls `check_inputs` on the vectors it
hands to `analyse_all` and refuses the model on any other answer; that
call is plain Rust outside the translation, read, not proven.

What is left of the five: `hcfg`, that the Kernel's configuration gives
each Task the Priority the Analyzer was given (the second half of
`hprio`). It relates two inputs of the theorem, the Kernel's `Config` and
the Analyzer's Task list, and no check of the Analyzer sees the first.
The other hypotheses (`hall`, `hlen`, `hNT`, `hL`, the run) are as they
were. Handwritten; no statement of another file is changed. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

open rsk_analyzer.terms (TaskIn) in
/-- `hprio` from the check's answer and the configuration's Priorities. -/
theorem checked_prio {cfg : logic.Config NT NR NP} {tasks : alloc.vec.Vec TaskIn}
    (hpos : ∀ (k : Nat) (t : TaskIn), tasks.val[k]? = some t → 0 < t.priority.val)
    (hcfg : ∀ (k : Nat) (t : TaskIn), tasks.val[k]? = some t → prio cfg k = t.priority.val) :
    ∀ (k : Nat) (t : TaskIn), tasks.val[k]? = some t →
      0 < t.priority.val ∧ prio cfg k = t.priority.val :=
  fun k t h => ⟨hpos k t h, hcfg k t h⟩

open rsk_analyzer (candidateOf) in
open rsk_analyzer.terms (TaskIn Site Window Verdict) in
/-- `property18` (the specification-level run) for inputs `check_inputs`
accepts. -/
theorem property18_checked {cfg : logic.Config NT NR NP}
    (tasks : alloc.vec.Vec TaskIn) (sites : alloc.vec.Vec Site) (p : rsk_analyzer.terms.Params)
    (any_fpu : Bool) (windows : alloc.vec.Vec Window)
    (ds : alloc.vec.Vec rsk_analyzer.terms.Derived) (vs : alloc.vec.Vec Verdict)
    (tss : alloc.vec.Vec (alloc.vec.Vec rsk_analyzer.terms.TermOf)) (quiet : alloc.vec.Vec Bool)
    (L : Nat → Nat → Nat)
    (hcheck : rsk_analyzer.terms.check_inputs tasks sites windows = ok none)
    (hall : rsk_analyzer.terms.analyse_all tasks sites p any_fpu windows
      = ok (some (ds, vs, tss, quiet)))
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hcfg : ∀ (k : Nat) (t : TaskIn), tasks.val[k]? = some t → prio cfg k = t.priority.val)
    (hL : ∀ i ti k r, tasks.val[i]? = some ti → k < NT.val → prio cfg k < prio cfg i →
      prio cfg i ≤ ceil cfg r →
      L k r = 0 ∨
      ∃ (j : Nat) (s : Site) (l : TaskIn), sites.val[j]? = some s
        ∧ tasks.val[s.task.val]? = some l
        ∧ l.priority.val < ti.priority.val ∧ ti.priority.val ≤ s.ceiling.val
        ∧ L k r ≤ candidateOf ti l s p)
    (i : Nat) (v : Verdict) (hv : vs.val[i]? = some v) (hsched : v.schedulable = true)
    {st : St NT NR NL}
    (hreach : Reach cfg (runParams NT.val tasks.val sites.val p windows.val L) st)
    (hpos : 0 < st.pend i + st.run i) :
    ∃ a, st.jl i = some a ∧ st.now < a + v.r.val := by
  obtain ⟨hsok, hwl, hbud, hDT, hpp⟩ := rsk_analyzer.checked_inputs hcheck
  exact property18 tasks sites p any_fpu windows ds vs tss quiet L hall hlen hNT hsok hwl
    (checked_prio hpp hcfg) hbud hDT hL i v hv hsched hreach hpos

open rsk_analyzer (candidateOf) in
open rsk_analyzer.terms (TaskIn Site Window Verdict) in
/-- **Property 18 over the Kernel's functions, for the inputs the
Analyzer's check accepts.** `property18_kernel` with its hypotheses
`hsok`, `hwl`, `hbud`, `hDT` and the positive Priorities of `hprio`
replaced by `hcheck`: the compiled `check_inputs` answered `None` on the
inputs `analyse_all` was run on. -/
theorem property18_kernel_checked {cfg : logic.Config NT NR NP}
    (tasks : alloc.vec.Vec TaskIn) (sites : alloc.vec.Vec Site) (p : rsk_analyzer.terms.Params)
    (any_fpu : Bool) (windows : alloc.vec.Vec Window)
    (ds : alloc.vec.Vec rsk_analyzer.terms.Derived) (vs : alloc.vec.Vec Verdict)
    (tss : alloc.vec.Vec (alloc.vec.Vec rsk_analyzer.terms.TermOf)) (quiet : alloc.vec.Vec Bool)
    (L : Nat → Nat → Nat)
    (hcheck : rsk_analyzer.terms.check_inputs tasks sites windows = ok none)
    (hall : rsk_analyzer.terms.analyse_all tasks sites p any_fpu windows
      = ok (some (ds, vs, tss, quiet)))
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hcfg : ∀ (k : Nat) (t : TaskIn), tasks.val[k]? = some t → prio cfg k = t.priority.val)
    (hL : ∀ i ti k r, tasks.val[i]? = some ti → k < NT.val → prio cfg k < prio cfg i →
      prio cfg i ≤ ceil cfg r →
      L k r = 0 ∨
      ∃ (j : Nat) (s : Site) (l : TaskIn), sites.val[j]? = some s
        ∧ tasks.val[s.task.val]? = some l
        ∧ l.priority.val < ti.priority.val ∧ ti.priority.val ≤ s.ceiling.val
        ∧ L k r ≤ candidateOf ti l s p)
    (i : Nat) (v : Verdict) (hv : vs.val[i]? = some v) (hsched : v.schedulable = true)
    {x : KSt NT NR NP NL NS NE}
    (hreach : KReach cfg (runParams NT.val tasks.val sites.val p windows.val L) x)
    (hpos : 0 < x.pend i + x.run i) :
    ∃ a, x.jl i = some a ∧ x.now < a + v.r.val := by
  obtain ⟨hsok, hwl, hbud, hDT, hpp⟩ := rsk_analyzer.checked_inputs hcheck
  exact property18_kernel tasks sites p any_fpu windows ds vs tss quiet L hall hlen hNT hsok hwl
    (checked_prio hpp hcfg) hbud hDT hL i v hv hsched hreach hpos

open rsk_analyzer (candidateOf) in
open rsk_analyzer.terms (TaskIn Site Window Verdict) in
/-- `property18_timed` for inputs `check_inputs` accepts. -/
theorem property18_timed_checked {cfg : logic.Config NT NR NP}
    (tasks : alloc.vec.Vec TaskIn) (sites : alloc.vec.Vec Site) (p : rsk_analyzer.terms.Params)
    (any_fpu : Bool) (windows : alloc.vec.Vec Window)
    (ds : alloc.vec.Vec rsk_analyzer.terms.Derived) (vs : alloc.vec.Vec Verdict)
    (tss : alloc.vec.Vec (alloc.vec.Vec rsk_analyzer.terms.TermOf)) (quiet : alloc.vec.Vec Bool)
    (L : Nat → Nat → Nat)
    (hcheck : rsk_analyzer.terms.check_inputs tasks sites windows = ok none)
    (hall : rsk_analyzer.terms.analyse_all tasks sites p any_fpu windows
      = ok (some (ds, vs, tss, quiet)))
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hcfg : ∀ (k : Nat) (t : TaskIn), tasks.val[k]? = some t → prio cfg k = t.priority.val)
    (hL : ∀ i ti k r, tasks.val[i]? = some ti → k < NT.val → prio cfg k < prio cfg i →
      prio cfg i ≤ ceil cfg r →
      L k r = 0 ∨
      ∃ (j : Nat) (s : Site) (l : TaskIn), sites.val[j]? = some s
        ∧ tasks.val[s.task.val]? = some l
        ∧ l.priority.val < ti.priority.val ∧ ti.priority.val ≤ s.ceiling.val
        ∧ L k r ≤ candidateOf ti l s p)
    (B : TimeBase cfg (runParams NT.val tasks.val sites.val p windows.val L))
    (i : Nat) (v : Verdict) (hv : vs.val[i]? = some v) (hsched : v.schedulable = true)
    {x : KSt NT NR NP NL NS NE}
    (hreach : TReach cfg (runParams NT.val tasks.val sites.val p windows.val L) B x)
    (hpos : 0 < x.pend i + x.run i) :
    ∃ a, x.jl i = some a ∧ x.now < a + v.r.val := by
  obtain ⟨hsok, hwl, hbud, hDT, hpp⟩ := rsk_analyzer.checked_inputs hcheck
  exact property18_timed tasks sites p any_fpu windows ds vs tss quiet L hall hlen hNT hsok hwl
    (checked_prio hpp hcfg) hbud hDT hL B i v hv hsched hreach hpos

open rsk_analyzer (candidateOf) in
open rsk_analyzer.terms (TaskIn Site Window Verdict) in
/-- `property18_sporadic` for inputs `check_inputs` accepts. -/
theorem property18_sporadic_checked {cfg : logic.Config NT NR NP}
    (tasks : alloc.vec.Vec TaskIn) (sites : alloc.vec.Vec Site) (p : rsk_analyzer.terms.Params)
    (any_fpu : Bool) (windows : alloc.vec.Vec Window)
    (ds : alloc.vec.Vec rsk_analyzer.terms.Derived) (vs : alloc.vec.Vec Verdict)
    (tss : alloc.vec.Vec (alloc.vec.Vec rsk_analyzer.terms.TermOf)) (quiet : alloc.vec.Vec Bool)
    (L : Nat → Nat → Nat)
    (hcheck : rsk_analyzer.terms.check_inputs tasks sites windows = ok none)
    (hall : rsk_analyzer.terms.analyse_all tasks sites p any_fpu windows
      = ok (some (ds, vs, tss, quiet)))
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hcfg : ∀ (k : Nat) (t : TaskIn), tasks.val[k]? = some t → prio cfg k = t.priority.val)
    (hL : ∀ i ti k r, tasks.val[i]? = some ti → k < NT.val → prio cfg k < prio cfg i →
      prio cfg i ≤ ceil cfg r →
      L k r = 0 ∨
      ∃ (j : Nat) (s : Site) (l : TaskIn), sites.val[j]? = some s
        ∧ tasks.val[s.task.val]? = some l
        ∧ l.priority.val < ti.priority.val ∧ ti.priority.val ≤ s.ceiling.val
        ∧ L k r ≤ candidateOf ti l s p)
    (B : SBase cfg (runParams NT.val tasks.val sites.val p windows.val L))
    (i : Nat) (v : Verdict) (hv : vs.val[i]? = some v) (hsched : v.schedulable = true)
    {x : KSt NT NR NP NL NS NE}
    (hreach : SReach cfg (runParams NT.val tasks.val sites.val p windows.val L) B x)
    (hpos : 0 < x.pend i + x.run i) :
    ∃ a, x.jl i = some a ∧ x.now < a + v.r.val := by
  obtain ⟨hsok, hwl, hbud, hDT, hpp⟩ := rsk_analyzer.checked_inputs hcheck
  exact property18_sporadic tasks sites p any_fpu windows ds vs tss quiet L hall hlen hNT hsok
    hwl (checked_prio hpp hcfg) hbud hDT hL B i v hv hsched hreach hpos

section entries
variable {A T : Type _}
  (ki : _root_.rsk_kernel.arch.bound.KernelOps (_root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE))
  (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A) (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T)

open rsk_analyzer (candidateOf) in
open rsk_analyzer.terms (TaskIn Site Window Verdict) in
/-- `property18_entries` for inputs `check_inputs` accepts. -/
theorem property18_entries_checked (hb : QBound ki) {cfg : logic.Config NT NR NP}
    (tasks : alloc.vec.Vec TaskIn) (sites : alloc.vec.Vec Site) (p : rsk_analyzer.terms.Params)
    (any_fpu : Bool) (windows : alloc.vec.Vec Window)
    (ds : alloc.vec.Vec rsk_analyzer.terms.Derived) (vs : alloc.vec.Vec Verdict)
    (tss : alloc.vec.Vec (alloc.vec.Vec rsk_analyzer.terms.TermOf)) (quiet : alloc.vec.Vec Bool)
    (L : Nat → Nat → Nat)
    (hcheck : rsk_analyzer.terms.check_inputs tasks sites windows = ok none)
    (hall : rsk_analyzer.terms.analyse_all tasks sites p any_fpu windows
      = ok (some (ds, vs, tss, quiet)))
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hcfg : ∀ (k : Nat) (t : TaskIn), tasks.val[k]? = some t → prio cfg k = t.priority.val)
    (hL : ∀ i ti k r, tasks.val[i]? = some ti → k < NT.val → prio cfg k < prio cfg i →
      prio cfg i ≤ ceil cfg r →
      L k r = 0 ∨
      ∃ (j : Nat) (s : Site) (l : TaskIn), sites.val[j]? = some s
        ∧ tasks.val[s.task.val]? = some l
        ∧ l.priority.val < ti.priority.val ∧ ti.priority.val ≤ s.ceiling.val
        ∧ L k r ≤ candidateOf ti l s p)
    (i : Nat) (v : Verdict) (hv : vs.val[i]? = some v) (hsched : v.schedulable = true)
    {x : KSt NT NR NP NL NS NE}
    (hreach : EReach ki ai ti cfg (runParams NT.val tasks.val sites.val p windows.val L) x)
    (hpos : 0 < x.pend i + x.run i) :
    ∃ a, x.jl i = some a ∧ x.now < a + v.r.val := by
  obtain ⟨hsok, hwl, hbud, hDT, hpp⟩ := rsk_analyzer.checked_inputs hcheck
  exact property18_entries ki ai ti hb tasks sites p any_fpu windows ds vs tss quiet L hall hlen
    hNT hsok hwl (checked_prio hpp hcfg) hbud hDT hL i v hv hsched hreach hpos

end entries

end P18
