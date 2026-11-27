import Sched.Quiet
open Aeneas Aeneas.Std Result

/-! # The run cut into the Kernel's entries (2026-11-22)

The run of `Sched/KRun.lean` (`KStep`) has no entry boundaries: its four
transitions for a Budget event's own timer entry (`bevent_h`, `overrun_h`,
`overrun_p`, `bevent_r`) ask as a premise that the Kernel state they start
from is quieted (`Quieted`). `Sched/Quiet.lean` proved that every entry
core of the linked Kernel returns a quieted Kernel when it was given one.
This file joins the two:

- `CoreRet`, `EntryOf`: the Kernel state `k'` is what one of the five
  entry cores (`svc_core`, `irq_entry_core`, `start_core`, `fault_core`,
  `epilogue_core`), as extracted and with the linked Kernel behind the
  trait, returned when called on `k`, the configuration being the run's
  before and after.
- `EStep`: one step of the run cut into entries. Either a transition that
  leaves the Kernel's state (`out`: time, an arrival, a Job executing), or
  a whole entry (`entry`): transitions that leave the scheduler state and
  the slots (`EPre`: the reading of the time base, the charge of the Job
  on top), then the entry's first transition (`EHead`), then any
  transitions of the run (`KSteps`), ending in the Kernel state the entry
  core returned from the one the entry found (`EntryOf`).
- `EHead` has the run's four Budget-event transitions **without the
  premise `Quieted`**; every other transition of the run is admitted as
  it is.
- `ereach_kreach`: every state the cut run reaches (`EReach`) is a state
  of the run of `Sched/KRun.lean`, and the Kernel is quieted at every
  entry boundary. The premise of the four transitions is derived: by
  induction over the entries from the boot, with `Sched/Quiet.lean` at
  each boundary.
- `property18_entries`: Property 18 over the cut run.
- `budget_first`: the state in which a timer entry serves its first slot
  (after `Kernel::observe` and `Budget::charge`, on a boundary state) is
  one in which a Budget slot's own entry needs no premise.

What is stated and not derived: `EntryOf` says the configuration of the
linked Kernel is the run's before and after the entry (each method of
`Bound` keeps it, `Sched/BoundAll.lean` `*_kp`; that is not composed over
the cores here), and `QBound` that the five interface functions read are
`Bound`'s. That every execution of the Kernel is such a run (each entry's
writes to the Kernel are the transitions between its two boundaries) was
read from the code function by function (`Sched/Timer.lean`,
`Sched/Endpoint.lean`). Since 2026-11-27 `Sched/Trace.lean` proves the
code's half for a whole entry: the configuration is kept, and the Kernel
state an entry leaves is reached from the one it found by a sequence of
the functions the run's transitions carry (`entryof_lsteps`). Which
functions, in what order, and the model's side of each transition stay as
they were. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

section entries
variable {A T : Type _}
  (ki : _root_.rsk_kernel.arch.bound.KernelOps (_root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE))
  (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A) (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T)

/-- One of the five entry cores, called on `b`, returned `b'`. -/
def CoreRet (b b' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE) : Prop :=
  (∃ (hw hw' : A) (tables : T) (state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState)
      (frame frame' : Array U32 8#usize) (number : U8), ∃ r,
    _root_.rsk_kernel.arch.cm4.entry.svc_core ki ai ti b hw tables state frame number
      = ok (r, b', hw', st', frame')) ∨
  (∃ (hw hw' : A) (tables : T) (state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState)
      (ipsr psp control er : U32), ∃ r,
    _root_.rsk_kernel.arch.cm4.entry.irq_entry_core ki ai ti b hw tables state ipsr psp control er
      = ok (r, b', hw', st')) ∨
  (∃ (hw hw' : A) (tables : T) (state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState), ∃ r,
    _root_.rsk_kernel.arch.cm4.entry.start_core ki ai ti b hw tables state = ok (r, b', hw', st')) ∨
  (∃ (hw hw' : A) (tables : T) (state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState)
      (kind : U8) (ink : Bool), ∃ r,
    _root_.rsk_kernel.arch.cm4.entry.fault_core ki ai ti b hw tables state kind ink
      = ok (r, b', hw', st')) ∨
  (∃ (hw hw' : A) (tables : T) (state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState)
      (n : U32), ∃ r,
    _root_.rsk_kernel.arch.cm4.entry.epilogue_core ki ai ti b hw tables state n
      = ok (r, b', hw', st'))

/-- **Every entry core of the linked Kernel returns a quieted Kernel when
it was given one** (`Sched/Quiet.lean`, the five cores together). -/
theorem coreret_quieted (hb : QBound ki)
    {b b' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    (h : CoreRet ki ai ti b b') (hq : BQ b) : BQ b' := by
  rcases h with ⟨_, _, _, _, _, _, _, _, _, h⟩ | ⟨_, _, _, _, _, _, _, _, _, _, h⟩ |
    ⟨_, _, _, _, _, _, h⟩ | ⟨_, _, _, _, _, _, _, _, h⟩ | ⟨_, _, _, _, _, _, _, h⟩
  · exact svc_core_quieted ki ai ti hb h hq
  · exact irq_entry_core_quieted ki ai ti hb h hq
  · exact start_core_quieted ki ai ti hb h hq
  · exact fault_core_quieted ki ai ti hb h hq
  · exact epilogue_core_quieted ki ai ti hb h hq

/-- The Kernel state `k'` is what an entry core of the linked Kernel
returned when called on `k`, with the configuration `cfg` before and
after. -/
def EntryOf (cfg : logic.Config NT NR NP)
    (k k' : logic.kernel.Kernel NT NR NP NL NS NE) : Prop :=
  ∃ b b' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE,
    b.cfg = cfg ∧ b.kernel = k ∧ b'.cfg = cfg ∧ b'.kernel = k' ∧ CoreRet ki ai ti b b'

theorem entryof_quieted (hb : QBound ki) {cfg : logic.Config NT NR NP}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : EntryOf ki ai ti cfg k k') (hq : Quieted cfg k) : Quieted cfg k' := by
  obtain ⟨b, b', hc, hk, hc', hk', hr⟩ := h
  subst hc hk hk'
  have := coreret_quieted ki ai ti hb hr hq
  rw [BQ, hc'] at this
  exact this

end entries

/-! ## The parts of an entry -/

/-- Transitions of the run that leave the scheduler state and the slots:
what an entry does before its first transition on them (`observed`,
`charged`), and time and arrivals. -/
inductive EPre (cfg : logic.Config NT NR NP) (P : Params) :
    KSt NT NR NP NL NS NE → KSt NT NR NP NL NS NE → Prop
  | refl (x : KSt NT NR NP NL NS NE) : EPre cfg P x x
  | step (x y z : KSt NT NR NP NL NS NE) (h : EPre cfg P x y) (hs : KStep cfg P y z)
      (hsc : z.k.sched = y.k.sched) (hsl : z.k.slots = y.k.slots) : EPre cfg P x z

/-- Transitions of the run, one after another. -/
inductive KSteps (cfg : logic.Config NT NR NP) (P : Params) :
    KSt NT NR NP NL NS NE → KSt NT NR NP NL NS NE → Prop
  | refl (x : KSt NT NR NP NL NS NE) : KSteps cfg P x x
  | step (x y z : KSt NT NR NP NL NS NE) (h : KSteps cfg P x y) (hs : KStep cfg P y z) :
      KSteps cfg P x z

/-- The first transition of an entry on the scheduler state or the slots:
any transition of the run, or one of the four of a Budget event's own
timer entry, here **without the premise that the state is quieted**. The
rest of each is `KStep`'s, premise for premise. -/
inductive EHead (cfg : logic.Config NT NR NP) (P : Params) :
    KSt NT NR NP NL NS NE → KSt NT NR NP NL NS NE → Prop
  | step (x y : KSt NT NR NP NL NS NE) (h : KStep cfg P x y) : EHead cfg P x y
  | bevent_h (x : KSt NT NR NP NL NS NE)
      (harmed : topBudgetArmed x.k cfg)
      (hk : x.ks = 0) (c : Nat) (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hd : 0 < x.k.sched.depth.val)
      (hrun : 0 < x.run x.top) (hrb : 0 < x.rb x.top) (hc : c ≤ P.Kt)
      (t : U8) (i : Usize)
      (hcode : logic.kernel.Kernel.budget_event x.k cfg t i = ok k')
      (hs : k'.sched = x.k.sched) :
      EHead cfg P x { x with
        k := k', ks := c, kp := x.top,
        rb := Function.update x.rb x.top (x.rb x.top - 1) }
  | overrun_h (x : KSt NT NR NP NL NS NE)
      (harmed : topBudgetArmed x.k cfg)
      (hk : x.ks = 0) (hj : JitterOk P x.st)
      (c : Nat) (r : Usize) (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hd : 0 < x.k.sched.depth.val)
      (hr0 : 0 < x.run x.top) (hr : x.run x.top ≤ 1)
      (hstop : P.stop x.top = true) (hrb : 0 < x.rb x.top)
      (hc : c ≤ P.Kov)
      (hal : ∀ p, topEntry x.k.sched p → c + 1 ≤ x.allow p)
      (t : U8) (i : Usize)
      (hcode : logic.kernel.Kernel.budget_event x.k cfg t i = ok k')
      (hs : logic.sched.Sched.finish_top x.k.sched cfg = ok (r, k'.sched)) :
      EHead cfg P x { x with
        k := k', now := x.now + 1, run := Function.update x.run x.top 0,
        ks := c, kp := x.top }
  | overrun_p (x : KSt NT NR NP NL NS NE)
      (harmed : topBudgetArmed x.k cfg)
      (hk : x.ks = 0) (hj : JitterOk P x.st)
      (c : Nat) (r : Usize) (s1 : logic.sched.Sched NT NR NL)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hd : 0 < x.k.sched.depth.val)
      (hr0 : 0 < x.run x.top) (hr : x.run x.top ≤ 1)
      (hstop : P.stop x.top = true) (hrb : 0 < x.rb x.top)
      (hc : c ≤ P.Kov)
      (hal : ∀ p, topEntry x.k.sched p → c + 1 ≤ x.allow p)
      (t : U8) (i : Usize)
      (hcode : logic.kernel.Kernel.budget_event x.k cfg t i = ok k')
      (hs : logic.sched.Sched.finish_top x.k.sched cfg = ok (r, s1))
      (hrs : RSteps cfg s1 k'.sched) :
      EHead cfg P x { x with
        k := k', now := x.now + 1,
        run := normRun k'.sched (Function.update x.run x.top 0),
        pend := normPend k'.sched x.pend,
        ks := c, kp := x.top }
  | bevent_r (x : KSt NT NR NP NL NS NE)
      (harmed : topBudgetArmed x.k cfg)
      (hk : x.ks = 0) (c : Nat) (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hd : 0 < x.k.sched.depth.val)
      (hrun : 0 < x.run x.top) (hrb : 0 < x.rb x.top) (hc : c ≤ P.Kt)
      (t : U8) (i : Usize)
      (hcode : logic.kernel.Kernel.budget_event x.k cfg t i = ok k') :
      EHead cfg P x { x with
        k := k', run := normRun k'.sched x.run, pend := normPend k'.sched x.pend,
        ks := c, kp := x.top,
        rb := Function.update x.rb x.top (x.rb x.top - 1) }

/-- In a quieted state the entry's first transition is a transition of the
run. -/
theorem EHead.kstep {cfg : logic.Config NT NR NP} {P : Params}
    {x y : KSt NT NR NP NL NS NE} (h : EHead cfg P x y) (hq : Quieted cfg x.k) :
    KStep cfg P x y := by
  cases h with
  | step _ hk0 => exact hk0
  | bevent_h harmed hk c k' hd hrun hrb hc t i hcode hs =>
    exact KStep.bevent_h x hq harmed hk c k' hd hrun hrb hc t i hcode hs
  | overrun_h harmed hk hj c r k' hd hr0 hr hstop hrb hc hal t i hcode hs =>
    exact KStep.overrun_h x hq harmed hk hj c r k' hd hr0 hr hstop hrb hc hal t i hcode hs
  | overrun_p harmed hk hj c r s1 k' hd hr0 hr hstop hrb hc hal t i hcode hs hrs =>
    exact KStep.overrun_p x hq harmed hk hj c r s1 k' hd hr0 hr hstop hrb hc hal t i hcode hs hrs
  | bevent_r harmed hk c k' hd hrun hrb hc t i hcode =>
    exact KStep.bevent_r x hq harmed hk c k' hd hrun hrb hc t i hcode

/-- The transitions before an entry's first keep the run and a quieted
Kernel. -/
theorem EPre.reach {cfg : logic.Config NT NR NP} {P : Params}
    {x y : KSt NT NR NP NL NS NE} (h : EPre cfg P x y)
    (hx : KReach cfg P x) (hq : Quieted cfg x.k) : KReach cfg P y ∧ Quieted cfg y.k := by
  induction h with
  | refl => exact ⟨hx, hq⟩
  | step y z _ hs hsc hsl ih =>
    exact ⟨KReach.step _ _ ih.1 hs, Quieted.frame hsc hsl ih.2⟩

theorem KSteps.reach {cfg : logic.Config NT NR NP} {P : Params}
    {x y : KSt NT NR NP NL NS NE} (h : KSteps cfg P x y)
    (hx : KReach cfg P x) : KReach cfg P y := by
  induction h with
  | refl => exact hx
  | step y z _ hs ih => exact KReach.step _ _ ih hs

/-! ## The run cut into entries -/

section run
variable {A T : Type _}
  (ki : _root_.rsk_kernel.arch.bound.KernelOps (_root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE))
  (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A) (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T)

/-- One step of the run cut into the Kernel's entries.

- `out`: a transition of the run that leaves the Kernel's state as it is
  (an arrival, a unit of time, a Job executing).
- `entry`: one entry of the Kernel, from the boundary before it to the
  boundary after it. The Kernel state at the second boundary is what an
  entry core of the linked Kernel returned when called on the Kernel state
  at the first (`EntryOf`); between them the run makes the transitions of
  the entry. -/
inductive EStep (cfg : logic.Config NT NR NP) (P : Params) :
    KSt NT NR NP NL NS NE → KSt NT NR NP NL NS NE → Prop
  | out (x y : KSt NT NR NP NL NS NE) (h : KStep cfg P x y) (hk : y.k = x.k) :
      EStep cfg P x y
  | entry (x x1 y z : KSt NT NR NP NL NS NE)
      (hpre : EPre cfg P x x1) (hhead : EHead cfg P x1 y) (htail : KSteps cfg P y z)
      (hcode : EntryOf ki ai ti cfg x.k z.k) : EStep cfg P x z

/-- The states at the entry boundaries of a run from the boot. -/
inductive EReach (cfg : logic.Config NT NR NP) (P : Params) : KSt NT NR NP NL NS NE → Prop
  | boot (k0 : logic.kernel.Kernel NT NR NP NL NS NE)
      (h : logic.sched.Sched.new NL cfg = ok k0.sched) : EReach cfg P (KSt.boot k0)
  | step (x y : KSt NT NR NP NL NS NE) (h : EReach cfg P x) (hs : EStep ki ai ti cfg P x y) :
      EReach cfg P y

/-- A Kernel whose scheduler state is what `Sched::new` returns is quieted:
no Job is in progress. -/
theorem new_quieted {cfg : logic.Config NT NR NP}
    {k0 : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.sched.Sched.new NL cfg = ok k0.sched) : Quieted cfg k0 := by
  intro ha
  unfold logic.sched.Sched.new at h
  have h := ok_inj h
  have hd : k0.sched.depth.val = 0 := by rw [← h]; rfl
  have := ha.1
  omega

/-- **The cut run is a run of `Sched/KRun.lean`, and the Kernel is quieted
at every entry boundary.** The premise `Quieted` of the four Budget-event
transitions is derived at each: the boot state has no Job, and every entry
core returns a quieted Kernel when the entry found one. -/
theorem ereach_kreach (hb : QBound ki) {cfg : logic.Config NT NR NP} {P : Params}
    {x : KSt NT NR NP NL NS NE} (h : EReach ki ai ti cfg P x) :
    KReach cfg P x ∧ Quieted cfg x.k := by
  induction h with
  | boot k0 h0 => exact ⟨KReach.boot k0 h0, new_quieted h0⟩
  | step x y _ hs ih =>
    cases hs with
    | out _ h hk => exact ⟨KReach.step _ _ ih.1 h, by rw [hk]; exact ih.2⟩
    | entry _ _ _ hpre hhead htail hcode =>
      obtain ⟨hr1, hq1⟩ := hpre.reach ih.1 ih.2
      exact ⟨htail.reach (KReach.step _ _ hr1 (hhead.kstep hq1)),
        entryof_quieted ki ai ti hb hcode ih.2⟩

open rsk_analyzer (candidateOf) in
open rsk_analyzer.terms (TaskIn Site Window Verdict) in
/-- **Property 18 over the run cut into the Kernel's entries.** As
`property18_kernel`, for every state at an entry boundary of a run in which
the Kernel's state between two boundaries changes as an entry core of the
linked Kernel changes it. No transition of this run asks that the Kernel
state it starts from is quieted. -/
theorem property18_entries (hb : QBound ki) {cfg : logic.Config NT NR NP}
    (tasks : alloc.vec.Vec TaskIn) (sites : alloc.vec.Vec Site) (p : rsk_analyzer.terms.Params)
    (any_fpu : Bool) (windows : alloc.vec.Vec Window)
    (ds : alloc.vec.Vec rsk_analyzer.terms.Derived) (vs : alloc.vec.Vec Verdict)
    (tss : alloc.vec.Vec (alloc.vec.Vec rsk_analyzer.terms.TermOf)) (quiet : alloc.vec.Vec Bool)
    (L : Nat → Nat → Nat)
    (hall : rsk_analyzer.terms.analyse_all tasks sites p any_fpu windows
      = ok (some (ds, vs, tss, quiet)))
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length)
    (hwl : windows.val.length = tasks.val.length)
    (hprio : ∀ k t, tasks.val[k]? = some t → 0 < t.priority.val ∧ prio cfg k = t.priority.val)
    (hbud : ∀ t ∈ tasks.val, 0 < t.budget.val)
    (hDT : ∀ w ∈ windows.val, w.deadline.val ≤ w.period.val)
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
    ∃ a, x.jl i = some a ∧ x.now < a + v.r.val :=
  property18_kernel tasks sites p any_fpu windows ds vs tss quiet L hall hlen hNT hsok hwl hprio
    hbud hDT hL i v hv hsched (ereach_kreach ki ai ti hb hreach).1 hpos

/-- **A Budget slot the timer entry serves first.** On a boundary state of
the cut run, after `Kernel::observe` and `Budget::charge` (the two steps of
`Kernel::timer_event` before it serves a slot), a Budget slot served by
`Kernel::serve_slot` is the first transition of an entry, with nothing
asked of the Kernel state but what the slot shows. -/
theorem budget_first {cfg : logic.Config NT NR NP} {P : Params}
    {x : KSt NT NR NP NL NS NE} (_hx : EReach ki ai ti cfg P x)
    {now : U64} {ka k' : logic.kernel.Kernel NT NR NP NL NS NE} {t : U8} {rc : U32}
    {b : logic.budget.Budget NT} (i : Usize) (c : Nat)
    (hobs : logic.kernel.Kernel.observe x.k cfg now = ok ka)
    (hch : logic.budget.Budget.charge ka.budget t rc = ok b)
    (hs : logic.kernel.Kernel.serve_slot { ka with budget := b } cfg i = ok k')
    (hkind : (slotOf { ka with budget := b } i).kind = logic.time.SLOT_BUDGET)
    (ht : (slotOf { ka with budget := b } i).task.val < NT.val)
    (harmed : topBudgetArmed { ka with budget := b } cfg)
    (hk : x.ks = 0) (hrun : 0 < x.run (stk ka.sched (ka.sched.depth.val - 1)))
    (hrb : 0 < x.rb (stk ka.sched (ka.sched.depth.val - 1))) (hc : c ≤ P.Kt) :
    ∃ x1 y, EPre cfg P x x1 ∧ x1.k = { ka with budget := b } ∧ EHead cfg P x1 y ∧ y.k = k' := by
  have hsl : ka.slots = x.k.slots := by
    unfold logic.kernel.Kernel.observe at hobs
    frame_ok hobs
  have h1 : EPre cfg P x { x with k := ka } :=
    EPre.step _ _ _ (EPre.refl x) (KStep.observed x now ka hobs) (observe_sched hobs) hsl
  have h2 : EPre cfg P x { x with k := { ka with budget := b } } :=
    EPre.step _ _ _ h1 (KStep.charged { x with k := ka } t rc b hch) rfl rfl
  exact ⟨_, _, h2, rfl,
    EHead.bevent_r { x with k := { ka with budget := b } } harmed hk c k' harmed.1 hrun hrb hc _ _
      (serve_slot_budget hs hkind ht), rfl⟩

end run

end P18
