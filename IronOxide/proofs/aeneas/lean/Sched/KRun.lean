import Sched.Link
import Sched.Select
open Aeneas Aeneas.Std Result

/-! # The run over the Kernel's own functions

`KStep` is the run of `Sched/Run.lean` with the Kernel's state in it: the
transitions the Kernel makes through `Kernel::dispatch`, `job_return`,
`svc_lock`, `svc_unlock`, `periodic_release`, `deferred_release`,
`deadline_miss`, `intake_release`, `apply_response`, `budget_event`
(served inside another entry) and `settle` (the last step of every entry)
are those functions as extracted (`Sched/Kernel.lean`),
and the Task dispatched is the one `Sched::select` returns. `ksim` shows
that every such run is a run of `Sched/Run.lean`; `property18_kernel` is
`property18` over it. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- A state of the run: the Kernel's state and the model's accounts (those
of `St`). -/
structure KSt (NT NR NP NL NS NE : Usize) where
  k : logic.kernel.Kernel NT NR NP NL NS NE
  now : Nat
  jl : Nat → Option Nat
  jwait : Nat → Bool
  el : Nat → Option Nat
  ewait : Nat → Bool
  pend : Nat → Nat
  run : Nat → Nat
  allow : Nat → Nat
  ks : Nat
  kp : Nat
  rl : Nat → Nat
  ru : Nat → Nat
  re : Nat → Nat
  rb : Nat → Nat

/-- The state as the run of `Sched/Run.lean` sees it: the Kernel's
scheduler state and the accounts. -/
@[reducible] def KSt.st (x : KSt NT NR NP NL NS NE) : St NT NR NL :=
  { s := x.k.sched, now := x.now, jl := x.jl, jwait := x.jwait, el := x.el, ewait := x.ewait,
    pend := x.pend, run := x.run, allow := x.allow, ks := x.ks, kp := x.kp, rl := x.rl,
    ru := x.ru, re := x.re, rb := x.rb }

/-- The Task whose Job is on top of the stack. -/
abbrev KSt.top (x : KSt NT NR NP NL NS NE) : Nat := stk x.k.sched (x.k.sched.depth.val - 1)

/-- The execution accounts after a response: a Task keeps the execution its
Job in progress still needs only while the scheduler state has that Job in
progress, -/
def normRun (s : logic.sched.Sched NT NR NL) (run : Nat → Nat) : Nat → Nat :=
  fun k => if k < NT.val ∧ job s k ≠ 1 then 0 else run k

/-- and the execution its pending Job will ask for only while the
scheduler state has its Release pending. -/
def normPend (s : logic.sched.Sched NT NR NL) (pend : Nat → Nat) : Nat → Nat :=
  fun k => if k < NT.val ∧ rel s k ≠ 1 then 0 else pend k

/-- One transition of the run over the Kernel's functions.

**As extracted** (the Kernel state after the transition is what the
function returns):
- `periodic`: the entry of a Periodic_Task's release event,
  `Kernel::periodic_release`. If it made the Release (the Task had none
  held and has one pending after), the Release is one that awaited its
  entry, for a Job that will execute for `d`; if not (an overlap and its
  response, a stopped Partition, the end of the operating duration), the
  accounts follow the scheduler state the Kernel left.
- `intake`: the entry of a Sporadic_Task's Release_Source event,
  `Kernel::intake_release`: accepted, held to the end of the MIT, or
  discarded, with the MIT_Violation's response; the accounts as for
  `periodic`.
- `dispatch`: `Kernel::dispatch` of the Task `Sched::select` returns.
- `lock`, `unlock`: `Kernel::svc_lock`, `svc_unlock`, when they grant.
- `finish`: the Job on top executes its last unit and returns,
  `Kernel::job_return`.
- `unwind`: `Kernel::job_return` with no Kernel section of its own: the
  unwinding of a killed Job, part of the entry whose response killed it.
- `resp`: a Health_Monitor response, `Kernel::apply_response`, at any
  instant and with no Kernel section of its own (its time is that of the
  entry that applies it).

- `bserved`: a Budget event served inside a timer entry that is in
  progress, `Kernel::budget_event`, with its Overrun_Response, at any
  instant and with no Kernel section of its own (its time is that of the
  entry that serves it). This is the order the Kernel takes when a Release
  and the executing Job's Budget event are due together: the entry makes
  the Release (`periodic`, `deferred`) and then serves the Budget event
  with the dispatch pending. No condition on `Sched::select`.

- `settle`: the first step of `apply_core`, with which most Kernel entries
  end (not all: `Sched/Settle.lean`), `Kernel::settle`: when
  R7.1 admits a Task the Budget event of the Job on top is disarmed, so
  that no Budget event has a timer entry of its own ahead of the dispatch
  (2026-11-15). It leaves the scheduler state as it is and has no Kernel
  section of its own.

- `observed`: the Kernel reads the time base, `Kernel::observe`, the first
  step of a timer entry and of an intake. It writes the Kernel's reading
  and nothing else, and has no Kernel section of its own.

- `deferred`: the entry of a deferral event, `Kernel::deferred_release`:
  the held Release made, or dropped; the accounts as for `periodic`.
- `deadline`: the entry of a deadline event, `Kernel::deadline_miss`, with
  its response. It has an entry only when the Job is not complete at its
  deadline.

**A Budget event's own timer entry** (since 2026-11-17 the Kernel state
after it is what `Kernel::budget_event` returns; what that call did to the
scheduler state is stated beside it, and is read from the code in
`Sched/Timer.lean` for the first two): `bevent_h` (found without an
overrun, or recorded only: the scheduler state kept), `overrun_h` (the
overrunning Job on top ended, `finish_top`, as its last unit), `overrun_p`
(the same followed by the rest of a Partition's response: the Job on top
ended, then a response on the scheduler state; the accounts follow the
scheduler state the Kernel left), `bevent_r` (any Budget event's own entry
read only through what the code did to the scheduler state: a response on
it, with no last unit of the Job counted; this is the entry whose
Overrun_Response is the safe state, which drops every Release and ends no
Job. Its Kernel section is bounded as a Budget event's entry, `Kt`).

**An Endpoint operation** (since 2026-11-17 on the code): `svc_h`, the
SVC's Kernel section, after which the Kernel state is the one before or
what `Kernel::record_event` returns (a full buffer recorded). This is all
the send and receive cores do to the Kernel but for two calls that end
them, each a transition of its own: `faulted` (`Kernel::fault`, an API
misuse and its Fault_Response) and, for a send whose message releases
the receiving Task, `intake` (`Sched/Endpoint.lean` reads this from
`endpoint_send_core` and `endpoint_recv_core` as extracted).
`faulted_of` is `Kernel::fault_of` (a rejected message's response).
Like `resp`, `faulted` and `faulted_of` have no Kernel section of their
own.

**A served slot that is none of the above** (since 2026-11-17, later, on
the code: the Kernel state after is what `Kernel::serve_slot` returns):
`plain_h` (an entry that leaves the scheduler state and is no deadline
event: unmask, housekeeping, a free slot) and `opend` (the end of the
operating duration: the safe state). No transition of the run leaves the
Kernel's state to a hypothesis any more; what stays stated beside the
code's result is what that call did to the scheduler state, read from the
code in `Sched/Timer.lean`.

**The Kernel's writes outside the scheduler state** (2026-11-22), so
that the Kernel state of a run can be the code's through a whole entry and
not only through the calls above: `charged` (`Budget::charge`, with which
a timer entry charges the Job on top the execution it measured),
`acked_reinit` and `acked_stop` (`Kernel::ack_reinit`, `ack_stop_dma`: the
acknowledgements with which `apply_core` ends an entry after a Partition's
restart or stop) and `logged` (`Kernel::push_event`: the event a refused
lock or unlock records). Each leaves the scheduler state, the slots and
the timing fields, has no Kernel section of its own and changes no
account. `Sched/Entries.lean` cuts the run into the Kernel's entries.

**Instants and time** as in `Step`: `jarrive`, `earrive`, `ktick`, `idle`,
`exec`. A Job executes, locks, unlocks and calls an Endpoint operation only when
`Sched::select` returns no Task. A Budget event has a timer entry of its
own (`bevent_h`, `overrun_h`, `overrun_p`, `bevent_r`) in a quieted Kernel
state (`Quieted`, `Sched/Calls.lean`: with the Budget slot of the Job on
top armed, `Sched::select` returns no Task) in which that slot is armed.
Until 2026-11-18 the premise was that the state is what `Kernel::settle`
returned, which is not what an entry leaves; `Sched/Settle.lean` proves the
premise for every state `apply_core` returns and names the entries that
return without it, for which it stays a premise. -/
inductive KStep (cfg : logic.Config NT NR NP) (P : Params) :
    KSt NT NR NP NL NS NE → KSt NT NR NP NL NS NE → Prop
  | jarrive (x : KSt NT NR NP NL NS NE) (t : Nat) (ht : t < NT.val)
      (hsp : ∀ r, x.jl t = some r → r + P.T t ≤ x.now) :
      KStep cfg P x { x with
        jl := Function.update x.jl t (some x.now),
        jwait := Function.update x.jwait t true }
  | earrive (x : KSt NT NR NP NL NS NE) (q : Nat) (hq : q < P.NQ)
      (hsp : ∀ r, x.el q = some r → r + P.ET q ≤ x.now) :
      KStep cfg P x { x with
        el := Function.update x.el q (some x.now),
        ewait := Function.update x.ewait q true }
  | periodic (x : KSt NT NR NP NL NS NE) (q : Nat) (t : U8) (i : Usize) (d c : Nat)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hq : q < P.NQ) (hk : x.ks = 0) (hw : x.ewait q = true)
      (hrel : P.relOf q = some t.val) (hnd : P.dlOf q = none) (hc : c ≤ P.EC q)
      (hs : logic.kernel.Kernel.periodic_release x.k cfg t i = ok k')
      (hmade : rel x.k.sched t.val = 0 → rel k'.sched t.val = 1 →
        x.jwait t.val = true ∧ 1 ≤ d ∧ d ≤ P.C t.val) :
      KStep cfg P x
        (if rel x.k.sched t.val = 0 ∧ rel k'.sched t.val = 1 then
          { x with
            k := k', ewait := Function.update x.ewait q false,
            jwait := Function.update x.jwait t.val false,
            pend := Function.update x.pend t.val (x.pend t.val + d),
            ks := c, kp := NT.val + q }
        else
          { x with
            k := k', ewait := Function.update x.ewait q false,
            run := normRun k'.sched x.run, pend := normPend k'.sched x.pend,
            ks := c, kp := NT.val + q })
  | intake (x : KSt NT NR NP NL NS NE) (q : Nat) (t sp off : U8) (now : U64) (d c : Nat)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hq : q < P.NQ) (hk : x.ks = 0) (hw : x.ewait q = true)
      (hrel : P.relOf q = some t.val) (hnd : P.dlOf q = none) (hc : c ≤ P.EC q)
      (hs : logic.kernel.Kernel.intake_release x.k cfg t sp now off = ok k')
      (hmade : rel x.k.sched t.val = 0 → rel k'.sched t.val = 1 →
        x.jwait t.val = true ∧ 1 ≤ d ∧ d ≤ P.C t.val) :
      KStep cfg P x
        (if rel x.k.sched t.val = 0 ∧ rel k'.sched t.val = 1 then
          { x with
            k := k', ewait := Function.update x.ewait q false,
            jwait := Function.update x.jwait t.val false,
            pend := Function.update x.pend t.val (x.pend t.val + d),
            ks := c, kp := NT.val + q }
        else
          { x with
            k := k', ewait := Function.update x.ewait q false,
            run := normRun k'.sched x.run, pend := normPend k'.sched x.pend,
            ks := c, kp := NT.val + q })
  | dispatch (x : KSt NT NR NP NL NS NE) (t : U8) (pc : U32) (c : Nat)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hk : x.ks = 0)
      (hsel : logic.sched.Sched.select x.k.sched cfg = ok (some t))
      (hc : c ≤ P.Kd)
      (hs : logic.kernel.Kernel.dispatch x.k cfg t pc = ok k') :
      KStep cfg P x { x with
        k := k', run := Function.update x.run t.val (x.pend t.val),
        pend := Function.update x.pend t.val 0,
        ks := c, kp := t.val,
        rl := Function.update x.rl t.val (P.S t.val),
        ru := Function.update x.ru t.val (P.S t.val),
        re := Function.update x.re t.val (P.EP t.val),
        rb := Function.update x.rb t.val 1 }
  | lock (x : KSt NT NR NP NL NS NE)
      (hsel : logic.sched.Sched.select x.k.sched cfg = ok none)
      (hk : x.ks = 0)
      (r : U8) (a c : Nat) (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hrun : 0 < x.run x.top) (hrl : 0 < x.rl x.top)
      (hc : c ≤ P.Kin) (ha : a ≤ P.L x.top r.val)
      (hal : ∀ p, topEntry x.k.sched p → c ≤ x.allow p)
      (hs : logic.kernel.Kernel.svc_lock x.k cfg r = ok (true, k')) :
      KStep cfg P x { x with
        k := k',
        allow := Function.update (deduct x.k.sched x.allow c) x.k.sched.lock_len.val (a - c),
        ks := c, kp := x.top,
        rl := Function.update x.rl x.top (x.rl x.top - 1) }
  | unlock (x : KSt NT NR NP NL NS NE)
      (hsel : logic.sched.Sched.select x.k.sched cfg = ok none)
      (hk : x.ks = 0)
      (r : U8) (c : Nat) (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hrun : 0 < x.run x.top) (hru : 0 < x.ru x.top)
      (hc : c ≤ P.Kout)
      (hal : ∀ p, topEntry x.k.sched p → c ≤ x.allow p)
      (hs : logic.kernel.Kernel.svc_unlock x.k cfg r = ok (true, k')) :
      KStep cfg P x { x with
        k := k', allow := deduct x.k.sched x.allow c,
        ks := c, kp := x.top,
        ru := Function.update x.ru x.top (x.ru x.top - 1) }
  | finish (x : KSt NT NR NP NL NS NE)
      (hsel : logic.sched.Sched.select x.k.sched cfg = ok none)
      (hk : x.ks = 0) (hj : JitterOk P x.st)
      (ec : U32) (c : Nat) (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hd : 0 < x.k.sched.depth.val)
      (hr0 : 0 < x.run x.top) (hr : x.run x.top ≤ 1)
      (hc : c ≤ P.Kc)
      (hal : ∀ p, topEntry x.k.sched p → c + 1 ≤ x.allow p)
      (hs : logic.kernel.Kernel.job_return x.k cfg ec = ok k') :
      KStep cfg P x { x with
        k := k', now := x.now + 1, run := Function.update x.run x.top 0,
        ks := c, kp := x.top }
  | unwind (x : KSt NT NR NP NL NS NE) (ec : U32)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hs : logic.kernel.Kernel.job_return x.k cfg ec = ok k') :
      KStep cfg P x { x with k := k', run := Function.update x.run x.top 0 }
  | resp (x : KSt NT NR NP NL NS NE) (partition response cause t : U8)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hs : logic.kernel.Kernel.apply_response x.k cfg partition response cause t = ok k') :
      KStep cfg P x { x with
        k := k', run := normRun k'.sched x.run, pend := normPend k'.sched x.pend }
  | faulted (x : KSt NT NR NP NL NS NE) (kind : U8) (kn : Bool)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hs : logic.kernel.Kernel.fault x.k cfg kind kn = ok k') :
      KStep cfg P x { x with
        k := k', run := normRun k'.sched x.run, pend := normPend k'.sched x.pend }
  | faulted_of (x : KSt NT NR NP NL NS NE) (kind t : U8)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hs : logic.kernel.Kernel.fault_of x.k cfg kind t = ok k') :
      KStep cfg P x { x with
        k := k', run := normRun k'.sched x.run, pend := normPend k'.sched x.pend }
  | bserved (x : KSt NT NR NP NL NS NE) (t : U8) (i : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hs : logic.kernel.Kernel.budget_event x.k cfg t i = ok k') :
      KStep cfg P x { x with
        k := k', run := normRun k'.sched x.run, pend := normPend k'.sched x.pend }
  | opend (x : KSt NT NR NP NL NS NE) (i : Usize)
      (k1 k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hcode : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
      (h1 : k1.sched = x.k.sched)
      (hs : logic.kernel.Kernel.enter_safe_state k1 cfg = ok k') :
      KStep cfg P x { x with
        k := k', run := normRun k'.sched x.run, pend := normPend k'.sched x.pend }
  | settle (x : KSt NT NR NP NL NS NE) (r : Option U8 × Bool)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hs : logic.kernel.Kernel.settle x.k cfg = ok (r, k')) :
      KStep cfg P x { x with k := k' }
  | observed (x : KSt NT NR NP NL NS NE) (now : U64)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hs : logic.kernel.Kernel.observe x.k cfg now = ok k') :
      KStep cfg P x { x with k := k' }
  | charged (x : KSt NT NR NP NL NS NE) (t : U8) (rc : U32) (b : logic.budget.Budget NT)
      (hs : logic.budget.Budget.charge x.k.budget t rc = ok b) :
      KStep cfg P x { x with k := { x.k with budget := b } }
  | acked_reinit (x : KSt NT NR NP NL NS NE) (p : U8)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hs : logic.kernel.Kernel.ack_reinit x.k cfg p = ok k') :
      KStep cfg P x { x with k := k' }
  | acked_stop (x : KSt NT NR NP NL NS NE) (p : U8)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hs : logic.kernel.Kernel.ack_stop_dma x.k cfg p = ok k') :
      KStep cfg P x { x with k := k' }
  | logged (x : KSt NT NR NP NL NS NE) (kind task partition : U8) (prs : U64) (detail : U32)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hs : logic.kernel.Kernel.push_event x.k cfg kind task partition prs detail = ok k') :
      KStep cfg P x { x with k := k' }
  | deferred (x : KSt NT NR NP NL NS NE) (q : Nat) (t : U8) (i : Usize) (d c : Nat)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hq : q < P.NQ) (hk : x.ks = 0) (hw : x.ewait q = true)
      (hrel : P.relOf q = some t.val) (hnd : P.dlOf q = none) (hc : c ≤ P.EC q)
      (hs : logic.kernel.Kernel.deferred_release x.k cfg t i = ok k')
      (hmade : rel x.k.sched t.val = 2 → rel k'.sched t.val = 1 →
        x.jwait t.val = true ∧ 1 ≤ d ∧ d ≤ P.C t.val) :
      KStep cfg P x
        (if rel x.k.sched t.val = 2 ∧ rel k'.sched t.val = 1 then
          { x with
            k := k', ewait := Function.update x.ewait q false,
            jwait := Function.update x.jwait t.val false,
            pend := Function.update x.pend t.val (x.pend t.val + d),
            ks := c, kp := NT.val + q }
        else
          { x with
            k := k', ewait := Function.update x.ewait q false,
            run := normRun k'.sched x.run, pend := normPend k'.sched x.pend,
            ks := c, kp := NT.val + q })
  | deadline (x : KSt NT NR NP NL NS NE) (q c : Nat) (t : U8) (i : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hq : q < P.NQ) (hk : x.ks = 0) (hw : x.ewait q = true) (hc : c ≤ P.EC q)
      (hdl : ∀ t', P.dlOf q = some t' → ∃ a, x.jl t' = some a ∧ a + P.D t' ≤ x.now ∧
        (0 < x.pend t' + x.run t' ∨ x.jwait t' = true))
      (hs : logic.kernel.Kernel.deadline_miss x.k cfg t i = ok k') :
      KStep cfg P x { x with
        k := k', ewait := Function.update x.ewait q false,
        run := normRun k'.sched x.run, pend := normPend k'.sched x.pend,
        ks := c, kp := NT.val + q }
  | plain_h (x : KSt NT NR NP NL NS NE) (q c : Nat)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hq : q < P.NQ) (hk : x.ks = 0) (hw : x.ewait q = true) (hc : c ≤ P.EC q)
      (hnd : P.dlOf q = none) (i : Usize)
      (hcode : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
      (hs : k'.sched = x.k.sched) :
      KStep cfg P x { x with
        k := k', ewait := Function.update x.ewait q false, ks := c, kp := NT.val + q }
  | svc_h (x : KSt NT NR NP NL NS NE)
      (hsel : logic.sched.Sched.select x.k.sched cfg = ok none)
      (hk : x.ks = 0) (c : Nat) (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hd : 0 < x.k.sched.depth.val)
      (hrun : 0 < x.run x.top) (hre : c ≤ x.re x.top)
      (hal : ∀ p, topEntry x.k.sched p → c ≤ x.allow p)
      (hcode : k' = x.k ∨
        ∃ kind t, logic.kernel.Kernel.record_event x.k cfg kind t = ok k') :
      KStep cfg P x { x with
        k := k', allow := deduct x.k.sched x.allow c,
        ks := c, kp := x.top,
        re := Function.update x.re x.top (x.re x.top - c) }
  | bevent_h (x : KSt NT NR NP NL NS NE)
      (hq : Quieted cfg x.k)
      (harmed : topBudgetArmed x.k cfg)
      (hk : x.ks = 0) (c : Nat) (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hd : 0 < x.k.sched.depth.val)
      (hrun : 0 < x.run x.top) (hrb : 0 < x.rb x.top) (hc : c ≤ P.Kt)
      (t : U8) (i : Usize)
      (hcode : logic.kernel.Kernel.budget_event x.k cfg t i = ok k')
      (hs : k'.sched = x.k.sched) :
      KStep cfg P x { x with
        k := k', ks := c, kp := x.top,
        rb := Function.update x.rb x.top (x.rb x.top - 1) }
  | overrun_h (x : KSt NT NR NP NL NS NE)
      (hq : Quieted cfg x.k)
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
      KStep cfg P x { x with
        k := k', now := x.now + 1, run := Function.update x.run x.top 0,
        ks := c, kp := x.top }
  | overrun_p (x : KSt NT NR NP NL NS NE)
      (hq : Quieted cfg x.k)
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
      KStep cfg P x { x with
        k := k', now := x.now + 1,
        run := normRun k'.sched (Function.update x.run x.top 0),
        pend := normPend k'.sched x.pend,
        ks := c, kp := x.top }
  | bevent_r (x : KSt NT NR NP NL NS NE)
      (hq : Quieted cfg x.k)
      (harmed : topBudgetArmed x.k cfg)
      (hk : x.ks = 0) (c : Nat) (k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (hd : 0 < x.k.sched.depth.val)
      (hrun : 0 < x.run x.top) (hrb : 0 < x.rb x.top) (hc : c ≤ P.Kt)
      (t : U8) (i : Usize)
      (hcode : logic.kernel.Kernel.budget_event x.k cfg t i = ok k') :
      KStep cfg P x { x with
        k := k', run := normRun k'.sched x.run, pend := normPend k'.sched x.pend,
        ks := c, kp := x.top,
        rb := Function.update x.rb x.top (x.rb x.top - 1) }
  | ktick (x : KSt NT NR NP NL NS NE) (hk : 0 < x.ks) (hj : JitterOk P x.st) :
      KStep cfg P x { x with now := x.now + 1, ks := x.ks - 1 }
  | idle (x : KSt NT NR NP NL NS NE)
      (hsel : logic.sched.Sched.select x.k.sched cfg = ok none)
      (hk : x.ks = 0) (hj : JitterOk P x.st)
      (hd : x.k.sched.depth.val = 0) :
      KStep cfg P x { x with now := x.now + 1 }
  | exec (x : KSt NT NR NP NL NS NE)
      (hsel : logic.sched.Sched.select x.k.sched cfg = ok none)
      (hk : x.ks = 0) (hj : JitterOk P x.st)
      (hd : 0 < x.k.sched.depth.val) (hr : 1 < x.run x.top)
      (hal : ∀ p, topEntry x.k.sched p → 0 < x.allow p) :
      KStep cfg P x { x with
        now := x.now + 1,
        run := Function.update x.run x.top (x.run x.top - 1),
        allow := deduct x.k.sched x.allow 1 }

/-- The state at boot: a Kernel whose scheduler state is what `Sched::new`
returns (`Kernel::new` builds it so, `new_eff`), instant 0, no Job, no
Kernel section. -/
def KSt.boot (k0 : logic.kernel.Kernel NT NR NP NL NS NE) : KSt NT NR NP NL NS NE :=
  { k := k0, now := 0, jl := fun _ => none, jwait := fun _ => false, el := fun _ => none,
    ewait := fun _ => false, pend := fun _ => 0, run := fun _ => 0, allow := fun _ => 0,
    ks := 0, kp := 0, rl := fun _ => 0, ru := fun _ => 0, re := fun _ => 0, rb := fun _ => 0 }

/-- The states a run over the Kernel's functions reaches from boot. -/
inductive KReach (cfg : logic.Config NT NR NP) (P : Params) : KSt NT NR NP NL NS NE → Prop
  | boot (k0 : logic.kernel.Kernel NT NR NP NL NS NE)
      (h : logic.sched.Sched.new NL cfg = ok k0.sched) : KReach cfg P (KSt.boot k0)
  | step (x y : KSt NT NR NP NL NS NE) (h : KReach cfg P x) (hs : KStep cfg P x y) :
      KReach cfg P y

/-! ## Every run over the Kernel's functions is a run of `Sched/Run.lean` -/

/-- In a run, a Task with a Job in progress or killed has at most the
Priority of the Job on top of the stack. -/
theorem wf_below_top {cfg : logic.Config NT NR NP} {P : Params} {m : St NT NR NL}
    (w : Wf cfg P m) (k : Nat) (hk : k < NT.val) (hj : job m.s k ≠ 0) :
    0 < m.s.depth.val ∧ prio cfg k ≤ prio cfg (stk m.s (m.s.depth.val - 1)) := by
  obtain ⟨p, hp, hpk⟩ := (w.on_stack k hk).mp hj
  refine ⟨by omega, ?_⟩
  by_cases hpt : p = m.s.depth.val - 1
  · rw [← hpk, hpt]
  · rw [← hpk]
    exact Nat.le_of_lt (w.sorted p (m.s.depth.val - 1) (by omega) (by omega))

/-- When `select` returns none in a run, R7.1 admits no Task. -/
theorem none_elig {cfg : logic.Config NT NR NP} {P : Params} {m : St NT NR NL}
    (hNT : NT.val ≤ 256) (w : Wf cfg P m)
    (h : logic.sched.Sched.select m.s cfg = ok none) : ∀ u, ¬ Elig cfg m.s u :=
  select_none hNT h (wf_below_top w)

/-- A response never makes a Release pending. -/
theorem rsteps_rel {cfg : logic.Config NT NR NP} {s s' : logic.sched.Sched NT NR NL}
    (h : RSteps cfg s s') : ∀ k, rel s' k = rel s k ∨ rel s' k = 0 := by
  induction h with
  | refl => exact fun k => Or.inl rfl
  | tail _ hbc ih =>
    intro k
    cases hbc with
    | kend r hf =>
      obtain ⟨_, _, _, hr, _⟩ := finish_top_views hf
      rw [hr]; exact ih k
    | kkill p hp hk =>
      obtain ⟨_, hr, _⟩ := kill_views hk
      rw [hr]; exact ih k
    | kclear t hc =>
      obtain ⟨_, hr, _⟩ := clear_release_views hc
      rw [hr]
      split
      · exact Or.inr rfl
      · exact ih k

/-- A response from a state of a run: the run reaches the scheduler state
the response leaves, with accounts that are those before it or, for a Task,
zero. -/
theorem rsteps_reach {cfg : logic.Config NT NR NP} {P : Params} {m : St NT NR NL}
    (hm : Reach cfg P m) {s' : logic.sched.Sched NT NR NL} (h : RSteps cfg m.s s') :
    ∃ run' pend' : Nat → Nat,
      Reach cfg P { m with s := s', run := run', pend := pend' } ∧
      (∀ k, run' k = m.run k ∨ (k < NT.val ∧ run' k = 0)) ∧
      (∀ k, pend' k = m.pend k ∨ (k < NT.val ∧ pend' k = 0)) := by
  induction h with
  | refl => exact ⟨m.run, m.pend, hm, fun _ => Or.inl rfl, fun _ => Or.inl rfl⟩
  | @tail b c _ hbc ih =>
    obtain ⟨run1, pend1, hr1, hrun1, hpend1⟩ := ih
    cases hbc with
    | kend r hf =>
      obtain ⟨_, _, hlt, _⟩ := finish_top_views hf
      refine ⟨Function.update run1 (stk b (b.depth.val - 1)) 0, pend1,
        Reach.step _ _ hr1 (Step.kend _ r c hf), ?_, hpend1⟩
      intro k
      by_cases hk : k = stk b (b.depth.val - 1)
      · right; rw [hk]; exact ⟨hlt, Function.update_self ..⟩
      · rw [Function.update_of_ne hk]; exact hrun1 k
    | kkill p hp hk =>
      obtain ⟨hlt, _⟩ := kill_views hk
      refine ⟨Function.update run1 (stk b p.val) 0, pend1,
        Reach.step _ _ hr1 (Step.kkill _ p c hp hk), ?_, hpend1⟩
      intro k
      by_cases hkk : k = stk b p.val
      · right; rw [hkk]; exact ⟨hlt, Function.update_self ..⟩
      · rw [Function.update_of_ne hkk]; exact hrun1 k
    | kclear t hc =>
      obtain ⟨hlt, _⟩ := clear_release_views hc
      refine ⟨run1, Function.update pend1 t.val 0,
        Reach.step _ _ hr1 (Step.kclear _ t c hc), hrun1, ?_⟩
      intro k
      by_cases hkk : k = t.val
      · right; rw [hkk]; exact ⟨hlt, Function.update_self ..⟩
      · rw [Function.update_of_ne hkk]; exact hpend1 k

/-- The same, with the accounts as functions of the scheduler state left. -/
theorem rsteps_norm {cfg : logic.Config NT NR NP} {P : Params} {m : St NT NR NL}
    (hm : Reach cfg P m) {s' : logic.sched.Sched NT NR NL} (h : RSteps cfg m.s s') :
    Reach cfg P { m with s := s', run := normRun s' m.run, pend := normPend s' m.pend } := by
  obtain ⟨run', pend', hr, hrun, hpend⟩ := rsteps_reach hm h
  have w := reach_wf hr
  have e1 : run' = normRun s' m.run := by
    funext k
    unfold normRun
    by_cases hc : k < NT.val ∧ job s' k ≠ 1
    · rw [ite_eq_left_of_eq_true _ _ (eq_true hc)]
      have := (w.run_iff k hc.1)
      dsimp only at this
      by_contra h0
      exact hc.2 (this.mp (by omega))
    · rw [ite_eq_right_of_eq_false _ _ (eq_false hc)]
      rcases hrun k with e | ⟨hk, e⟩
      · exact e
      · have hj : job s' k = 1 := by
          by_contra hj; exact hc ⟨hk, hj⟩
        have := (w.run_iff k hk).mpr hj
        dsimp only at this
        omega
  have e2 : pend' = normPend s' m.pend := by
    funext k
    unfold normPend
    by_cases hc : k < NT.val ∧ rel s' k ≠ 1
    · rw [ite_eq_left_of_eq_true _ _ (eq_true hc)]
      have := (w.pend_iff k hc.1)
      dsimp only at this
      by_contra h0
      exact hc.2 (this.mp (by omega))
    · rw [ite_eq_right_of_eq_false _ _ (eq_false hc)]
      rcases hpend k with e | ⟨hk, e⟩
      · exact e
      · have hj : rel s' k = 1 := by
          by_contra hj; exact hc ⟨hk, hj⟩
        have := (w.pend_iff k hk).mpr hj
        dsimp only at this
        omega
  rw [← e1, ← e2]
  exact hr

/-- `Kernel::new` leaves the scheduler state `Sched::new` returns. -/
theorem new_eff {cfg : logic.Config NT NR NP} {k0 : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.new NL NS NE cfg = ok k0) :
    logic.sched.Sched.new NL cfg = ok k0.sched := by
  unfold logic.kernel.Kernel.new at h
  peel h
  unfold logic.sched.Sched.new
  rfl

/-- `Kernel::ack_reinit` writes the re-initialization requests only. -/
theorem ack_reinit_upd {cfg : logic.Config NT NR NP}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {p : U8}
    (h : logic.kernel.Kernel.ack_reinit k cfg p = ok k') :
    ∃ a, k' = { k with reinit_partition := a } := by
  unfold logic.kernel.Kernel.ack_reinit at h
  peel h
  exact ⟨_, rfl⟩

/-- `Kernel::ack_stop_dma` writes the stop requests only. -/
theorem ack_stop_dma_upd {cfg : logic.Config NT NR NP}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {p : U8}
    (h : logic.kernel.Kernel.ack_stop_dma k cfg p = ok k') :
    ∃ a, k' = { k with stop_dma := a } := by
  unfold logic.kernel.Kernel.ack_stop_dma at h
  peel h
  exact ⟨_, rfl⟩

/-- Every state a run over the Kernel's functions reaches is, read through
its scheduler state and accounts, a state of a run of `Sched/Run.lean`. -/
theorem ksim {cfg : logic.Config NT NR NP} {P : Params} (hNT : NT.val ≤ 256)
    {x : KSt NT NR NP NL NS NE} (h : KReach cfg P x) : Reach cfg P x.st := by
  induction h with
  | boot k0 h0 => exact Reach.boot _ h0
  | step x y _ hs ih =>
    have w := reach_wf ih
    cases hs with
    | jarrive t ht hsp => exact Reach.step _ _ ih (Step.jarrive x.st t ht hsp)
    | earrive q hq hsp => exact Reach.step _ _ ih (Step.earrive x.st q hq hsp)
    | periodic q t i d c k' hq hk hw hrel hnd hc hs hmade =>
      rcases periodic_release_eff hs with ⟨hr0, hmp⟩ | hrs
      · have hr1 : rel k'.sched t.val = 1 := by
          obtain ⟨_, hrv, _⟩ := make_pending_views hmp
          rw [hrv]; simp
        obtain ⟨hj, hd1, hd2⟩ := hmade hr0 hr1
        rw [ite_eq_left_of_eq_true _ _ (eq_true ⟨hr0, hr1⟩)]
        exact Reach.step _ _ ih (Step.erel x.st q t d c k'.sched hq hk hw hrel hj hd1 hd2 hc hmp)
      · have hnm : ¬ (rel x.k.sched t.val = 0 ∧ rel k'.sched t.val = 1) := by
          rintro ⟨h0, h1⟩
          rcases rsteps_rel hrs t.val with e | e <;> omega
        rw [ite_eq_right_of_eq_false _ _ (eq_false hnm)]
        have h1 := Reach.step _ _ ih (Step.eplain x.st q c hq hk hw hc
          (fun t' ht' => by rw [hnd] at ht'; cases ht'))
        exact rsteps_norm h1 hrs
    | intake q t sp off now d c k' hq hk hw hrel hnd hc hs hmade =>
      rcases intake_release_eff hs with ⟨hr0, hmp⟩ | ⟨s1, hr0, hdf, hrs⟩ | hrs
      · have hr1 : rel k'.sched t.val = 1 := by
          obtain ⟨_, hrv, _⟩ := make_pending_views hmp
          rw [hrv]; simp
        obtain ⟨hj, hd1, hd2⟩ := hmade hr0 hr1
        rw [ite_eq_left_of_eq_true _ _ (eq_true ⟨hr0, hr1⟩)]
        exact Reach.step _ _ ih (Step.erel x.st q t d c k'.sched hq hk hw hrel hj hd1 hd2 hc hmp)
      · have hnm : ¬ (rel x.k.sched t.val = 0 ∧ rel k'.sched t.val = 1) := by
          rintro ⟨_, h1⟩
          obtain ⟨_, hrv, _⟩ := defer_views hdf
          have h2 := hrv t.val
          simp only [ite_true] at h2
          rcases rsteps_rel hrs t.val with e | e <;> omega
        rw [ite_eq_right_of_eq_false _ _ (eq_false hnm)]
        have h1 := Reach.step _ _ ih (Step.edefer x.st q t c s1 hq hk hw hrel hc hr0 hdf)
        exact rsteps_norm h1 hrs
      · have hnm : ¬ (rel x.k.sched t.val = 0 ∧ rel k'.sched t.val = 1) := by
          rintro ⟨h0, h1⟩
          rcases rsteps_rel hrs t.val with e | e <;> omega
        rw [ite_eq_right_of_eq_false _ _ (eq_false hnm)]
        have h1 := Reach.step _ _ ih (Step.eplain x.st q c hq hk hw hc
          (fun t' ht' => by rw [hnd] at ht'; cases ht'))
        exact rsteps_norm h1 hrs
    | dispatch t pc c k' hk hsel hc hs =>
      obtain ⟨he, hmax⟩ := select_some hNT hsel
      exact Reach.step _ _ ih (Step.dispatch x.st t c k'.sched hk he hmax hc (dispatch_eff hs))
    | lock hsel hk r a c k' hrun hrl hc ha hal hs =>
      exact Reach.step _ _ ih (Step.lock x.st (none_elig hNT w hsel) hk r a c k'.sched
        hrun hrl hc ha hal (svc_lock_eff hs))
    | unlock hsel hk r c k' hrun hru hc hal hs =>
      exact Reach.step _ _ ih (Step.unlock x.st (none_elig hNT w hsel) hk r c k'.sched
        hrun hru hc hal (svc_unlock_eff hs))
    | finish hsel hk hj ec c k' hd hr0 hr hc hal hs =>
      obtain ⟨r, hf⟩ := job_return_eff hs
      exact Reach.step _ _ ih (Step.finish x.st (none_elig hNT w hsel) hk hj false c r k'.sched
        hd hr0 hr (fun h => by cases h) hc hal hf)
    | unwind ec k' hs =>
      obtain ⟨r, hf⟩ := job_return_eff hs
      exact Reach.step _ _ ih (Step.kend x.st r k'.sched hf)
    | resp partition response cause t k' hs =>
      exact rsteps_norm ih (apply_response_r hs)
    | faulted kind kn k' hs => exact rsteps_norm ih (fault_r hs)
    | faulted_of kind t k' hs => exact rsteps_norm ih (fault_of_r hs)
    | bserved t i k' hs =>
      exact rsteps_norm ih (budget_event_eff hs)
    | opend i k1 k' hcode h1 hs =>
      have hr := enter_safe_state_r hs
      rw [h1] at hr
      exact rsteps_norm ih hr
    | settle r k' hs =>
      have := ih
      dsimp only [KSt.st] at this ⊢
      rw [settle_sched hs]
      exact this
    | observed now k' hs =>
      have := ih
      dsimp only [KSt.st] at this ⊢
      rw [observe_sched hs]
      exact this
    | charged t rc b hs => exact ih
    | acked_reinit p k' hs =>
      obtain ⟨a, rfl⟩ := ack_reinit_upd hs
      exact ih
    | acked_stop p k' hs =>
      obtain ⟨a, rfl⟩ := ack_stop_dma_upd hs
      exact ih
    | logged kind task partition prs detail k' hs =>
      have := ih
      dsimp only [KSt.st] at this ⊢
      rw [push_event_sched hs]
      exact this
    | deferred q t i d c k' hq hk hw hrel hnd hc hs hmade =>
      rcases deferred_release_eff hs with ⟨hr0, hmp⟩ | hrs
      · have hr1 : rel k'.sched t.val = 1 := by
          obtain ⟨_, hrv, _⟩ := make_pending_views hmp
          rw [hrv]; simp
        obtain ⟨hj, hd1, hd2⟩ := hmade hr0 hr1
        rw [ite_eq_left_of_eq_true _ _ (eq_true ⟨hr0, hr1⟩)]
        exact Reach.step _ _ ih (Step.erel x.st q t d c k'.sched hq hk hw hrel hj hd1 hd2 hc hmp)
      · have hnm : ¬ (rel x.k.sched t.val = 2 ∧ rel k'.sched t.val = 1) := by
          rintro ⟨h0, h1⟩
          rcases rsteps_rel hrs t.val with e | e <;> omega
        rw [ite_eq_right_of_eq_false _ _ (eq_false hnm)]
        have h1 := Reach.step _ _ ih (Step.eplain x.st q c hq hk hw hc
          (fun t' ht' => by rw [hnd] at ht'; cases ht'))
        exact rsteps_norm h1 hrs
    | deadline q c t i k' hq hk hw hc hdl hs =>
      have h1 := Reach.step _ _ ih (Step.eplain x.st q c hq hk hw hc hdl)
      exact rsteps_norm h1 (deadline_miss_eff hs)
    | plain_h q c k' hq hk hw hc hnd i hcode hs =>
      have := Reach.step _ _ ih (Step.eplain x.st q c hq hk hw hc
        (fun t' ht' => by rw [hnd] at ht'; cases ht'))
      dsimp only [KSt.st]
      rw [hs]
      exact this
    | svc_h hsel hk c k' hd hrun hre hal hcode =>
      have hs : k'.sched = x.k.sched := by
        rcases hcode with e | ⟨kind, t, h⟩
        · rw [e]
        · exact record_event_sched h
      have := Reach.step _ _ ih (Step.svc x.st (none_elig hNT w hsel) hk c hd hrun hre hal)
      dsimp only [KSt.st]
      rw [hs]
      exact this
    | bevent_h hq harmed hk c k' hd hrun hrb hc t i hcode hs =>
      have hsel := hq harmed
      have := Reach.step _ _ ih (Step.bevent x.st (none_elig hNT w hsel) hk c hd hrun hrb hc)
      dsimp only [KSt.st]
      rw [hs]
      exact this
    | overrun_h hq harmed hk hj c r k' hd hr0 hr hstop hrb hc hal t i hcode hs =>
      have hsel := hq harmed
      exact Reach.step _ _ ih (Step.finish x.st (none_elig hNT w hsel) hk hj true c r k'.sched
        hd hr0 hr (fun _ => ⟨hstop, hrb⟩) hc hal hs)
    | overrun_p hq harmed hk hj c r s1 k' hd hr0 hr hstop hrb hc hal t i hcode hs hrs =>
      have hsel := hq harmed
      have h1 := Reach.step _ _ ih (Step.finish x.st (none_elig hNT w hsel) hk hj true c r s1
        hd hr0 hr (fun _ => ⟨hstop, hrb⟩) hc hal hs)
      exact rsteps_norm h1 hrs
    | bevent_r hq harmed hk c k' hd hrun hrb hc t i hcode =>
      have hsel := hq harmed
      have h1 := Reach.step _ _ ih (Step.bevent x.st (none_elig hNT w hsel) hk c hd hrun hrb hc)
      exact rsteps_norm h1 (budget_event_eff hcode)
    | ktick hk hj => exact Reach.step _ _ ih (Step.ktick x.st hk hj)
    | idle hsel hk hj hd =>
      exact Reach.step _ _ ih (Step.idle x.st (none_elig hNT w hsel) hk hj hd)
    | exec hsel hk hj hd hr hal =>
      exact Reach.step _ _ ih (Step.exec x.st (none_elig hNT w hsel) hk hj hd hr hal)

open rsk_analyzer (candidateOf) in
open rsk_analyzer.terms (TaskIn Site Window Verdict) in
/-- **Property 18 over the Kernel's functions.** If `analyse_all`, as
compiled, reports Task `i` schedulable with the response time `v.r`, then
in every run over the Kernel's functions (`KStep`) with the parameters of
the Analyzer's inputs, in every state in which a released Job of `i` has
execution left, the instant is before the nominal instant of its Release
plus `v.r`. -/
theorem property18_kernel {cfg : logic.Config NT NR NP}
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
    (hreach : KReach cfg (runParams NT.val tasks.val sites.val p windows.val L) x)
    (hpos : 0 < x.pend i + x.run i) :
    ∃ a, x.jl i = some a ∧ x.now < a + v.r.val :=
  property18 tasks sites p any_fpu windows ds vs tss quiet L hall hlen hNT hsok hwl hprio hbud
    hDT hL i v hv hsched (ksim hNT hreach) hpos

end P18
