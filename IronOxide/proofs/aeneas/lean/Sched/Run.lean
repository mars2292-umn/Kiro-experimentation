import Sched.Views
open Aeneas Aeneas.Std Result

namespace P18
open rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! ## The run -/

/-- A state of a run: the Kernel's scheduler state, the instant, and the
model's accounts.

- Per Task: the nominal instant of its last Release (`jl`) and whether that
  Release still awaits the Kernel entry that makes it (`jwait`); the
  execution its pending Job will ask for (`pend`) and the execution its Job
  in progress still needs (`run`); how many locks, unlocks and Budget
  events its Job in progress may still take and how much Kernel time its
  Endpoint operations may still take (`rl`, `ru`, `rb`, `re`).
- Per event source (a timed event of a Task, the intake of a
  Release_Source, the housekeeping event): the instant of its last event
  (`el`) and whether that event still awaits its Kernel entry (`ewait`).
- Per entry of the lock stack: the time its Critical_Section may still take
  (`allow`).
- The Kernel section in progress: the units it still takes (`ks`, 0 when
  the Kernel is not executing) and on whose account it runs (`kp`: `k`
  below the Task count for the Job of Task `k`, the Task count plus `q`
  for event source `q`). -/
structure St (NT NR NL : Usize) where
  s : logic.sched.Sched NT NR NL
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

/-- The timing parameters of a run.

Per Task: its period or MIT `T`, its Budget `C`, its release jitter `J`,
its deadline `D`, the number of its Critical_Section sites `S`, the Kernel
time of its Endpoint operations per Job `EP`, and whether its Partition's
Overrun_Response stops an overrunning Job (`stop`). Per Task and Resource:
the bound `L` on one Critical_Section. The Kernel_Timing_Parameters: a
dispatch `Kd`, a Job end `Kc`, a lock `Kin`, an unlock `Kout`, the timer
entry of a Budget event `Kt`, and the entry that ends an overrunning Job
`Kov`. Per event source `q < NQ`: the least
distance `ET` between two of its events, the bound `EC` on its Kernel
entry, its entry's jitter `EJ`, the Task its entry may release (`relOf`)
and the Task whose deadline event it is (`dlOf`). -/
structure Params where
  T : Nat → Nat
  C : Nat → Nat
  J : Nat → Nat
  D : Nat → Nat
  S : Nat → Nat
  EP : Nat → Nat
  stop : Nat → Bool
  L : Nat → Nat → Nat
  Kd : Nat
  Kc : Nat
  Kin : Nat
  Kout : Nat
  Kt : Nat
  Kov : Nat
  NQ : Nat
  ET : Nat → Nat
  EC : Nat → Nat
  EJ : Nat → Nat
  relOf : Nat → Option Nat
  dlOf : Nat → Option Nat

/-- No Release and no event that awaits its Kernel entry is past its
jitter one unit on. -/
def JitterOk (P : Params) (st : St NT NR NL) : Prop :=
  (∀ k a, st.jwait k = true → st.jl k = some a → st.now + 1 ≤ a + P.J k) ∧
  (∀ q a, st.ewait q = true → st.el q = some a → st.now + 1 ≤ a + P.EJ q)

abbrev eligibleFn {NT NR NP NL NS NE : Usize} :=
  @_root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.eligible NT NR NP NL NS NE

/-- The lock entries of the Job on top of the stack. -/
def topEntry (s : logic.sched.Sched NT NR NL) (p : Nat) : Prop :=
  p < s.lock_len.val ∧ ldep s p = s.depth.val - 1

instance (s : logic.sched.Sched NT NR NL) (p : Nat) : Decidable (topEntry s p) := by
  unfold topEntry; infer_instance

/-- `c` units taken from every Critical_Section the Job on top is in. -/
def deduct (s : logic.sched.Sched NT NR NL) (allow : Nat → Nat) (c : Nat) : Nat → Nat :=
  fun p => if topEntry s p then allow p - c else allow p

/-- One transition of a run. The scheduler state changes only through the
Kernel's functions as extracted.

**Instants** (no time passes, no Kernel work):
- `jarrive`: the nominal instant of a Release of Task `t`, at least `T t`
  after its last one.
- `earrive`: the instant of an event of source `q`, at least `ET q` after
  its last one.

**Kernel sections.** The Kernel executes one section at a time and no Job
executes meanwhile; a section begins only when none is in progress
(`ks = 0`), changes the scheduler state at its beginning, and then takes
`c` units (`ktick`), `c` at most the section's Kernel_Timing_Parameter.
- `erel`: the entry of event source `q` (a timer entry or an intake) makes
  the Release of Task `t` that awaits it (`make_pending`): a Job that will
  execute for `d`, `1 ≤ d ≤ C t`.
- `eplain`: an entry of source `q` that leaves the scheduler state as it
  is (an unmask, the housekeeping event, a discarded intake, a deadline
  event). A deadline event of Task `t` has an entry only when the Job is
  not complete at its deadline (the Kernel disarms the event at the Job's
  end).
- `edefer`: an intake that holds the Release to the end of the MIT
  (`defer`).
- `dispatch`: the start of a pending Task the eligibility test accepts and
  that has the highest Priority among those it accepts (`start`).
- `lock`, `unlock`: the Job on top takes or returns a Resource (`lock`,
  `unlock`, when they grant it); at most `S` of each per Job. The
  Critical_Section may take `a`, at most `L` of the Task and the Resource,
  from the lock call on.
- `svc`: an Endpoint send or receive of the Job on top (no change of the
  scheduler state); at most `EP` of Kernel time per Job.
- `bevent`: the Budget event of the Job on top, found without an overrun
  or recorded only (no change of the scheduler state); once per Job.
- `finish`: the Job on top executes its last unit and ends (`finish_top`,
  which is `end_job`): it returns (`ov = false`), or it has used its
  Budget and its Partition stops overrunning Jobs (`ov = true`: its Budget
  event, not yet taken, ends it; the entry takes at most `Kov`).
A section the Job on top begins itself (`lock`, `unlock`, `svc`, `finish`)
is inside every Critical_Section the Job is in: its time is taken from
their allowances.

**Responses** (no time of their own: a response's time is that of the
Kernel section that applies it): the Health_Monitor's responses as the
scheduler sees them, at any instant.
- `kend`: the Job on top is ended or, if killed, unwound (`finish_top`).
- `kkill`: a Job on the stack is killed (`kill`).
- `kclear`: a Task's Release is dropped (`clear_release`).

**Time**, one unit each:
- `ktick`: of the Kernel section in progress.
- `idle`: no Job is in progress.
- `exec`: the Job on top executes and needs more.
- `finish`: above.

A Job executes, locks, unlocks, calls an Endpoint operation, and takes its
Budget event only when it is not killed, no Kernel section is in progress
and the eligibility test accepts no Task (a pending eligible Task is
dispatched at once, the Hardware_Model's rule). A unit of execution is taken from every
Critical_Section the Job is in, and none of them may be exhausted. No unit
of time passes that would leave a Release or an event waiting for its
entry beyond its jitter. -/
inductive Step (cfg : logic.Config NT NR NP) (P : Params) : St NT NR NL → St NT NR NL → Prop
  | jarrive (st : St NT NR NL) (t : Nat) (ht : t < NT.val)
      (hsp : ∀ r, st.jl t = some r → r + P.T t ≤ st.now) :
      Step cfg P st { st with
        jl := Function.update st.jl t (some st.now),
        jwait := Function.update st.jwait t true }
  | earrive (st : St NT NR NL) (q : Nat) (hq : q < P.NQ)
      (hsp : ∀ r, st.el q = some r → r + P.ET q ≤ st.now) :
      Step cfg P st { st with
        el := Function.update st.el q (some st.now),
        ewait := Function.update st.ewait q true }
  | erel (st : St NT NR NL) (q : Nat) (t : U8) (d c : Nat) (s' : logic.sched.Sched NT NR NL)
      (hq : q < P.NQ) (hk : st.ks = 0) (hw : st.ewait q = true)
      (hrel : P.relOf q = some t.val) (hj : st.jwait t.val = true)
      (hd1 : 1 ≤ d) (hd2 : d ≤ P.C t.val) (hc : c ≤ P.EC q)
      (hs : logic.sched.Sched.make_pending st.s cfg t = ok s') :
      Step cfg P st { st with
        s := s', ewait := Function.update st.ewait q false,
        jwait := Function.update st.jwait t.val false,
        pend := Function.update st.pend t.val (st.pend t.val + d),
        ks := c, kp := NT.val + q }
  | eplain (st : St NT NR NL) (q c : Nat)
      (hq : q < P.NQ) (hk : st.ks = 0) (hw : st.ewait q = true) (hc : c ≤ P.EC q)
      (hdl : ∀ t, P.dlOf q = some t → ∃ a, st.jl t = some a ∧ a + P.D t ≤ st.now ∧
        (0 < st.pend t + st.run t ∨ st.jwait t = true)) :
      Step cfg P st { st with
        ewait := Function.update st.ewait q false, ks := c, kp := NT.val + q }
  | edefer (st : St NT NR NL) (q : Nat) (t : U8) (c : Nat) (s' : logic.sched.Sched NT NR NL)
      (hq : q < P.NQ) (hk : st.ks = 0) (hw : st.ewait q = true)
      (hrel : P.relOf q = some t.val) (hc : c ≤ P.EC q)
      (hr0 : rel st.s t.val = 0)
      (hs : logic.sched.Sched.defer st.s cfg t = ok s') :
      Step cfg P st { st with
        s := s', ewait := Function.update st.ewait q false, ks := c, kp := NT.val + q }
  | ktick (st : St NT NR NL) (hk : 0 < st.ks) (hj : JitterOk P st) :
      Step cfg P st { st with now := st.now + 1, ks := st.ks - 1 }
  | dispatch (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE) (t : U8) (c : Nat)
      (s' : logic.sched.Sched NT NR NL)
      (hk : st.ks = 0)
      (hm : Matches b cfg st.s) (he : eligibleFn b t = ok true)
      (hmax : ∀ u, Elig cfg st.s u → prio cfg u ≤ prio cfg t.val)
      (hc : c ≤ P.Kd)
      (hs : logic.sched.Sched.start st.s cfg t = ok s') :
      Step cfg P st { st with
        s := s', run := Function.update st.run t.val (st.pend t.val),
        pend := Function.update st.pend t.val 0,
        ks := c, kp := t.val,
        rl := Function.update st.rl t.val (P.S t.val),
        ru := Function.update st.ru t.val (P.S t.val),
        re := Function.update st.re t.val (P.EP t.val),
        rb := Function.update st.rb t.val 1 }
  | lock (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hk : st.ks = 0)
      (r : U8) (a c : Nat) (s' : logic.sched.Sched NT NR NL)
      (hrun : 0 < st.run (stk st.s (st.s.depth.val - 1)))
      (hrl : 0 < st.rl (stk st.s (st.s.depth.val - 1)))
      (hc : c ≤ P.Kin)
      (ha : a ≤ P.L (stk st.s (st.s.depth.val - 1)) r.val)
      (hal : ∀ p, topEntry st.s p → c ≤ st.allow p)
      (hs : logic.sched.Sched.lock st.s cfg r = ok (true, s')) :
      Step cfg P st { st with
        s := s',
        allow := Function.update (deduct st.s st.allow c) st.s.lock_len.val (a - c),
        ks := c, kp := stk st.s (st.s.depth.val - 1),
        rl := Function.update st.rl (stk st.s (st.s.depth.val - 1))
          (st.rl (stk st.s (st.s.depth.val - 1)) - 1) }
  | unlock (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hk : st.ks = 0)
      (r : U8) (c : Nat) (s' : logic.sched.Sched NT NR NL)
      (hrun : 0 < st.run (stk st.s (st.s.depth.val - 1)))
      (hru : 0 < st.ru (stk st.s (st.s.depth.val - 1)))
      (hc : c ≤ P.Kout)
      (hal : ∀ p, topEntry st.s p → c ≤ st.allow p)
      (hs : logic.sched.Sched.unlock st.s cfg r = ok (true, s')) :
      Step cfg P st { st with
        s := s', allow := deduct st.s st.allow c,
        ks := c, kp := stk st.s (st.s.depth.val - 1),
        ru := Function.update st.ru (stk st.s (st.s.depth.val - 1))
          (st.ru (stk st.s (st.s.depth.val - 1)) - 1) }
  | svc (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hk : st.ks = 0) (c : Nat)
      (hd : 0 < st.s.depth.val)
      (hrun : 0 < st.run (stk st.s (st.s.depth.val - 1)))
      (hre : c ≤ st.re (stk st.s (st.s.depth.val - 1)))
      (hal : ∀ p, topEntry st.s p → c ≤ st.allow p) :
      Step cfg P st { st with
        allow := deduct st.s st.allow c,
        ks := c, kp := stk st.s (st.s.depth.val - 1),
        re := Function.update st.re (stk st.s (st.s.depth.val - 1))
          (st.re (stk st.s (st.s.depth.val - 1)) - c) }
  | bevent (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hk : st.ks = 0) (c : Nat)
      (hd : 0 < st.s.depth.val)
      (hrun : 0 < st.run (stk st.s (st.s.depth.val - 1)))
      (hrb : 0 < st.rb (stk st.s (st.s.depth.val - 1))) (hc : c ≤ P.Kt) :
      Step cfg P st { st with
        ks := c, kp := stk st.s (st.s.depth.val - 1),
        rb := Function.update st.rb (stk st.s (st.s.depth.val - 1))
          (st.rb (stk st.s (st.s.depth.val - 1)) - 1) }
  | idle (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hk : st.ks = 0) (hj : JitterOk P st)
      (hd : st.s.depth.val = 0) :
      Step cfg P st { st with now := st.now + 1 }
  | exec (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hk : st.ks = 0) (hj : JitterOk P st)
      (hd : 0 < st.s.depth.val) (hr : 1 < st.run (stk st.s (st.s.depth.val - 1)))
      (hal : ∀ p, topEntry st.s p → 0 < st.allow p) :
      Step cfg P st { st with
        now := st.now + 1,
        run := Function.update st.run (stk st.s (st.s.depth.val - 1))
          (st.run (stk st.s (st.s.depth.val - 1)) - 1),
        allow := deduct st.s st.allow 1 }
  | finish (st : St NT NR NL) {NS NE : Usize}
      (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
      (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
      (hk : st.ks = 0) (hj : JitterOk P st)
      (ov : Bool) (c : Nat) (r : Usize) (s' : logic.sched.Sched NT NR NL)
      (hd : 0 < st.s.depth.val)
      (hr0 : 0 < st.run (stk st.s (st.s.depth.val - 1)))
      (hr : st.run (stk st.s (st.s.depth.val - 1)) ≤ 1)
      (hov : ov = true → P.stop (stk st.s (st.s.depth.val - 1)) = true ∧
        0 < st.rb (stk st.s (st.s.depth.val - 1)))
      (hc : c ≤ if ov then P.Kov else P.Kc)
      (hal : ∀ p, topEntry st.s p → c + 1 ≤ st.allow p)
      (hs : logic.sched.Sched.finish_top st.s cfg = ok (r, s')) :
      Step cfg P st { st with
        s := s', now := st.now + 1,
        run := Function.update st.run (stk st.s (st.s.depth.val - 1)) 0,
        ks := c, kp := stk st.s (st.s.depth.val - 1) }
  | kend (st : St NT NR NL) (r : Usize) (s' : logic.sched.Sched NT NR NL)
      (hs : logic.sched.Sched.finish_top st.s cfg = ok (r, s')) :
      Step cfg P st { st with
        s := s', run := Function.update st.run (stk st.s (st.s.depth.val - 1)) 0 }
  | kkill (st : St NT NR NL) (p : Usize) (s' : logic.sched.Sched NT NR NL)
      (hp : p.val < st.s.depth.val)
      (hs : logic.sched.Sched.kill st.s cfg p = ok s') :
      Step cfg P st { st with
        s := s', run := Function.update st.run (stk st.s p.val) 0 }
  | kclear (st : St NT NR NL) (t : U8) (s' : logic.sched.Sched NT NR NL)
      (hs : logic.sched.Sched.clear_release st.s cfg t = ok s') :
      Step cfg P st { st with
        s := s', pend := Function.update st.pend t.val 0 }

/-- The state at boot (`Sched::new`, instant 0, no Job, no Kernel section). -/
def St.boot (s0 : logic.sched.Sched NT NR NL) : St NT NR NL :=
  { s := s0, now := 0, jl := fun _ => none, jwait := fun _ => false, el := fun _ => none,
    ewait := fun _ => false, pend := fun _ => 0, run := fun _ => 0, allow := fun _ => 0,
    ks := 0, kp := 0, rl := fun _ => 0, ru := fun _ => 0, re := fun _ => 0, rb := fun _ => 0 }

/-- The states a run reaches from boot. -/
inductive Reach (cfg : logic.Config NT NR NP) (P : Params) : St NT NR NL → Prop
  | boot (s0 : logic.sched.Sched NT NR NL) (h : logic.sched.Sched.new NL cfg = ok s0) :
      Reach cfg P (St.boot s0)
  | step (st st' : St NT NR NL) (h : Reach cfg P st) (hs : Step cfg P st st') : Reach cfg P st'

/-! ## The structure of the scheduler state in a run -/

structure Wf (cfg : logic.Config NT NR NP) (P : Params) (st : St NT NR NL) : Prop where
  depth_le : st.s.depth.val ≤ NT.val
  stk_lt : ∀ p, p < st.s.depth.val → stk st.s p < NT.val
  sorted : ∀ p q, p < q → q < st.s.depth.val → prio cfg (stk st.s p) < prio cfg (stk st.s q)
  /-- A Task has a Job in progress or killed exactly when it is on the stack. -/
  on_stack : ∀ k, k < NT.val → (job st.s k ≠ 0 ↔ ∃ p, p < st.s.depth.val ∧ stk st.s p = k)
  run_iff : ∀ k, k < NT.val → (0 < st.run k ↔ job st.s k = 1)
  pend_iff : ∀ k, k < NT.val → (0 < st.pend k ↔ rel st.s k = 1)
  /-- The System_Ceiling is that of the lock stack. -/
  ceil_is : IsCeil cfg st.s st.s.lock_len.val st.s.ceiling.val
  /-- An entry is held by a Job on the stack; -/
  ldep_lt : ∀ p, p < st.s.lock_len.val → ldep st.s p < st.s.depth.val
  /-- the entries of a Job higher on the stack are higher on the lock stack; -/
  ldep_mono : ∀ p q, p ≤ q → q < st.s.lock_len.val → ldep st.s p ≤ ldep st.s q
  /-- and a Job above an entry's holder has a Priority above the entry's
  Ceiling (the Stack Resource Policy). -/
  srp : ∀ p q, p < st.s.lock_len.val → q < st.s.depth.val → ldep st.s p < q →
    ceil cfg (lres st.s p) < prio cfg (stk st.s q)
  /-- A Critical_Section's remaining time is within its bound. -/
  allow_le : ∀ p, p < st.s.lock_len.val →
    st.allow p ≤ P.L (stk st.s (ldep st.s p)) (lres st.s p)
  /-- A Release or an event that awaits its entry is within its jitter. -/
  jwait_inv : ∀ k, st.jwait k = true → ∃ a, st.jl k = some a ∧ st.now ≤ a + P.J k
  ewait_inv : ∀ q, st.ewait q = true → ∃ a, st.el q = some a ∧ st.now ≤ a + P.EJ q
  rb_le : ∀ k, st.rb k ≤ 1
  /-- A Kernel section runs on the account of a Task's Job or of an event
  source. -/
  kp_lt : 0 < st.ks → st.kp < NT.val + P.NQ

theorem elig_views {cfg : logic.Config NT NR NP} {s : logic.sched.Sched NT NR NL} {t : Nat} :
    Elig cfg s t ↔ (t < NT.val ∧ rel s t = 1 ∧ job s t = 0 ∧ s.ceiling.val < prio cfg t ∧
      (s.depth.val = 0 ∨ prio cfg (stk s (s.depth.val - 1)) < prio cfg t)) := Iff.rfl

theorem IsCeil.same {cfg : logic.Config NT NR NP} {s s' : logic.sched.Sched NT NR NL} {n c : Nat}
    (hl : ∀ p, lres s' p = lres s p) (h : IsCeil cfg s n c) : IsCeil cfg s' n c := by
  obtain ⟨h1, h2⟩ := h
  refine ⟨fun p hp => by rw [hl]; exact h1 p hp, ?_⟩
  rcases h2 with h | ⟨p, hp, hc⟩
  · left; exact h
  · right; exact ⟨p, hp, by rw [hl]; exact hc⟩

theorem deduct_le (s : logic.sched.Sched NT NR NL) (allow : Nat → Nat) (c p : Nat) :
    deduct s allow c p ≤ allow p := by
  unfold deduct; split <;> omega

/-- The Job on top ended (`finish_top`), at one instant. -/
theorem Wf.kend {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) {r : Usize} {s' : logic.sched.Sched NT NR NL}
    (hs : logic.sched.Sched.finish_top st.s cfg = ok (r, s')) :
    Wf cfg P { st with
      s := s', run := Function.update st.run (stk st.s (st.s.depth.val - 1)) 0 } := by
  obtain ⟨h1, h2, h3, hrel, hjob, hstk, hdep, hlr, hld, hn, hpop, hkeep, hc⟩ :=
    finish_top_views hs
  have hbelow : ∀ p, p < s'.lock_len.val → ldep st.s p < st.s.depth.val - 1 := by
    intro p hp
    rcases hkeep with h0 | hne'
    · omega
    · have m := w.ldep_mono p (s'.lock_len.val - 1) (by omega) (by omega)
      have l := w.ldep_lt (s'.lock_len.val - 1) (by omega)
      omega
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, w.jwait_inv, w.ewait_inv, w.rb_le,
    w.kp_lt⟩ <;> dsimp only
  · rw [hdep]; omega
  · intro p hp; rw [hstk]; rw [hdep] at hp; exact w.stk_lt p (by omega)
  · intro p q hpq hq; rw [hstk, hstk]; rw [hdep] at hq; exact w.sorted p q hpq (by omega)
  · intro k hk; rw [hjob, hdep]; simp only [hstk]
    by_cases h : k = stk st.s (st.s.depth.val - 1)
    · simp only [h, ite_true]
      constructor
      · intro hx; exact absurd rfl hx
      · rintro ⟨p, hp, hpk⟩
        have := w.sorted p (st.s.depth.val - 1) (by omega) (by omega)
        rw [hpk] at this; omega
    · simp only [h, ite_false]
      rw [w.on_stack k hk]
      constructor
      · rintro ⟨p, hp, hpk⟩
        refine ⟨p, ?_, hpk⟩
        by_contra hc
        have : p = st.s.depth.val - 1 := by omega
        rw [this] at hpk; exact h hpk.symm
      · rintro ⟨p, hp, hpk⟩; exact ⟨p, by omega, hpk⟩
  · intro k hk; rw [hjob]
    by_cases h : k = stk st.s (st.s.depth.val - 1)
    · subst h; simp [Function.update]
    · rw [Function.update_of_ne h]; simp only [h, ite_false]; exact w.run_iff k hk
  · intro k hk; rw [hrel]; exact w.pend_iff k hk
  · exact hc.same hlr
  · intro p hp; rw [hld, hdep]; exact hbelow p hp
  · intro p q hpq hq; rw [hld, hld]; exact w.ldep_mono p q hpq (by omega)
  · intro p q hp hq hpq; rw [hdep] at hq; rw [hld] at hpq
    rw [hlr, hstk]; exact w.srp p q (by omega) (by omega) hpq
  · intro p hp; rw [hld, hlr, hstk]; exact w.allow_le p (by omega)

theorem Wf.step {cfg : logic.Config NT NR NP} {P : Params} {st st' : St NT NR NL}
    (w : Wf cfg P st) (h : Step cfg P st st') : Wf cfg P st' := by
  cases h with
  | jarrive t ht hsp =>
    refine ⟨w.depth_le, w.stk_lt, w.sorted, w.on_stack, w.run_iff, w.pend_iff, w.ceil_is,
      w.ldep_lt, w.ldep_mono, w.srp, w.allow_le, ?_, w.ewait_inv, w.rb_le, w.kp_lt⟩
    dsimp only
    intro k hk
    by_cases h : k = t
    · subst h; exact ⟨st.now, by simp, by omega⟩
    · rw [Function.update_of_ne h] at hk ⊢; exact w.jwait_inv k hk
  | earrive q hq hsp =>
    refine ⟨w.depth_le, w.stk_lt, w.sorted, w.on_stack, w.run_iff, w.pend_iff, w.ceil_is,
      w.ldep_lt, w.ldep_mono, w.srp, w.allow_le, w.jwait_inv, ?_, w.rb_le, w.kp_lt⟩
    dsimp only
    intro k hk
    by_cases h : k = q
    · subst h; exact ⟨st.now, by simp, by omega⟩
    · rw [Function.update_of_ne h] at hk ⊢; exact w.ewait_inv k hk
  | erel q t d c s' hq hk0 hw hrel hj hd1 hd2 hc hs =>
    obtain ⟨ht, hrl, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := make_pending_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, w.rb_le, ?_⟩ <;> dsimp only
    · rw [hdep]; exact w.depth_le
    · intro p hp; rw [hstk]; rw [hdep] at hp; exact w.stk_lt p hp
    · intro p q' hpq hq'; rw [hstk, hstk]; rw [hdep] at hq'; exact w.sorted p q' hpq hq'
    · intro k hk; rw [hjob, hdep]; simp only [hstk]; exact w.on_stack k hk
    · intro k hk; rw [hjob]; exact w.run_iff k hk
    · intro k hk; rw [hrl]
      by_cases h : k = t.val
      · subst h; simp [Function.update]; omega
      · rw [Function.update_of_ne h]; simp only [h, ite_false]; exact w.pend_iff k hk
    · rw [hll, hlc]; exact w.ceil_is.same hlr
    · intro p hp; rw [hld, hdep]; rw [hll] at hp; exact w.ldep_lt p hp
    · intro p q' hpq hq'; rw [hld, hld]; rw [hll] at hq'; exact w.ldep_mono p q' hpq hq'
    · intro p q' hp hq' hpq; rw [hll] at hp; rw [hdep] at hq'; rw [hld] at hpq
      rw [hlr, hstk]; exact w.srp p q' hp hq' hpq
    · intro p hp; rw [hll] at hp; rw [hld, hlr, hstk]; exact w.allow_le p hp
    · intro k hk
      by_cases h : k = t.val
      · subst h; simp at hk
      · rw [Function.update_of_ne h] at hk; exact w.jwait_inv k hk
    · intro k hk
      by_cases h : k = q
      · subst h; simp at hk
      · rw [Function.update_of_ne h] at hk; exact w.ewait_inv k hk
    · intro _; omega
  | eplain q c hq hk0 hw hc hdl =>
    refine ⟨w.depth_le, w.stk_lt, w.sorted, w.on_stack, w.run_iff, w.pend_iff, w.ceil_is,
      w.ldep_lt, w.ldep_mono, w.srp, w.allow_le, w.jwait_inv, ?_, w.rb_le, ?_⟩ <;> dsimp only
    · intro k hk
      by_cases h : k = q
      · subst h; simp at hk
      · rw [Function.update_of_ne h] at hk; exact w.ewait_inv k hk
    · intro _; omega
  | edefer q t c s' hq hk0 hw hrel hc hr0 hs =>
    obtain ⟨ht, hrl, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := defer_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, w.jwait_inv, ?_, w.rb_le, ?_⟩ <;>
      dsimp only
    · rw [hdep]; exact w.depth_le
    · intro p hp; rw [hstk]; rw [hdep] at hp; exact w.stk_lt p hp
    · intro p q' hpq hq'; rw [hstk, hstk]; rw [hdep] at hq'; exact w.sorted p q' hpq hq'
    · intro k hk; rw [hjob, hdep]; simp only [hstk]; exact w.on_stack k hk
    · intro k hk; rw [hjob]; exact w.run_iff k hk
    · intro k hk; rw [hrl]
      by_cases h : k = t.val
      · subst h
        have := w.pend_iff t.val hk
        simp only [ite_true]
        omega
      · simp only [h, ite_false]; exact w.pend_iff k hk
    · rw [hll, hlc]; exact w.ceil_is.same hlr
    · intro p hp; rw [hld, hdep]; rw [hll] at hp; exact w.ldep_lt p hp
    · intro p q' hpq hq'; rw [hld, hld]; rw [hll] at hq'; exact w.ldep_mono p q' hpq hq'
    · intro p q' hp hq' hpq; rw [hll] at hp; rw [hdep] at hq'; rw [hld] at hpq
      rw [hlr, hstk]; exact w.srp p q' hp hq' hpq
    · intro p hp; rw [hll] at hp; rw [hld, hlr, hstk]; exact w.allow_le p hp
    · intro k hk
      by_cases h : k = q
      · subst h; simp at hk
      · rw [Function.update_of_ne h] at hk; exact w.ewait_inv k hk
    · intro _; omega
  | ktick hk0 hj =>
    refine ⟨w.depth_le, w.stk_lt, w.sorted, w.on_stack, w.run_iff, w.pend_iff, w.ceil_is,
      w.ldep_lt, w.ldep_mono, w.srp, w.allow_le, ?_, ?_, w.rb_le, ?_⟩ <;> dsimp only
    · intro k hk
      obtain ⟨a, ha, _⟩ := w.jwait_inv k hk
      exact ⟨a, ha, hj.1 k a hk ha⟩
    · intro k hk
      obtain ⟨a, ha, _⟩ := w.ewait_inv k hk
      exact ⟨a, ha, hj.2 k a hk ha⟩
    · intro hx; exact w.kp_lt (by omega)
  | dispatch b t c s' hk0 hm he hmax hc hs =>
    obtain ⟨_, hr1, hj0, hcl, htop⟩ := elig_views.mp ((eligible_ok hm he).mp rfl)
    obtain ⟨ht, hdl, hrl, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := start_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, w.jwait_inv, w.ewait_inv, ?_, ?_⟩ <;>
      dsimp only
    · rw [hdep]; omega
    · intro p hp; rw [hstk]; rw [hdep] at hp
      by_cases h : p = st.s.depth.val
      · simp only [h, ite_true]; exact ht
      · simp only [h, ite_false]; exact w.stk_lt p (by omega)
    · intro p q hpq hq; rw [hstk, hstk]; rw [hdep] at hq
      have hp : p ≠ st.s.depth.val := by omega
      simp only [hp, ite_false]
      by_cases h : q = st.s.depth.val
      · simp only [h, ite_true]
        rcases htop with h0 | h1
        · omega
        · by_cases hp1 : p = st.s.depth.val - 1
          · rw [hp1]; exact h1
          · exact Nat.lt_trans (w.sorted p (st.s.depth.val - 1) (by omega) (by omega)) h1
      · simp only [h, ite_false]; exact w.sorted p q hpq (by omega)
    · intro k hk; rw [hjob, hdep]
      by_cases h : k = t.val
      · simp only [h, ite_true]
        constructor
        · intro _; exact ⟨st.s.depth.val, by omega, by rw [hstk]; simp⟩
        · intro _; omega
      · simp only [h, ite_false]
        rw [w.on_stack k hk]
        constructor
        · rintro ⟨p, hp, hpk⟩
          refine ⟨p, by omega, ?_⟩
          rw [hstk]; simp only [show p ≠ st.s.depth.val by omega, ite_false]; exact hpk
        · rintro ⟨p, hp, hpk⟩
          rw [hstk] at hpk
          by_cases hpd : p = st.s.depth.val
          · simp only [hpd, ite_true] at hpk; exact absurd hpk.symm h
          · simp only [hpd, ite_false] at hpk; exact ⟨p, by omega, hpk⟩
    · intro k hk; rw [hjob]
      by_cases h : k = t.val
      · subst h
        have := (w.pend_iff t.val ht).mpr hr1
        simp [Function.update]; exact this
      · rw [Function.update_of_ne h]; simp only [h, ite_false]; exact w.run_iff k hk
    · intro k hk; rw [hrl]
      by_cases h : k = t.val
      · subst h; simp [Function.update]
      · rw [Function.update_of_ne h]; simp only [h, ite_false]; exact w.pend_iff k hk
    · rw [hll, hlc]; exact w.ceil_is.same hlr
    · intro p hp; rw [hld, hdep]; rw [hll] at hp; have := w.ldep_lt p hp; omega
    · intro p q hpq hq; rw [hld, hld]; rw [hll] at hq; exact w.ldep_mono p q hpq hq
    · intro p q hp hq hpq; rw [hll] at hp; rw [hdep] at hq; rw [hld] at hpq
      rw [hlr, hstk]
      by_cases h : q = st.s.depth.val
      · simp only [h, ite_true]
        have := w.ceil_is.1 p hp
        omega
      · simp only [h, ite_false]; exact w.srp p q hp (by omega) hpq
    · intro p hp; rw [hll] at hp; rw [hld, hlr, hstk]
      have := w.ldep_lt p hp
      simp only [show ldep st.s p ≠ st.s.depth.val by omega, ite_false]
      exact w.allow_le p hp
    · intro k
      by_cases h : k = t.val
      · subst h; simp
      · rw [Function.update_of_ne h]; exact w.rb_le k
    · intro _; omega
  | lock b hm hne hk0 r a c s' hrun hrl hc ha hal hs =>
    obtain ⟨⟨hrel, hjob, hstk, hdep⟩, hd1, hl, hll, hlr, hld, hlc⟩ := lock_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, w.jwait_inv, w.ewait_inv, w.rb_le,
      ?_⟩ <;> dsimp only
    · rw [hdep]; exact w.depth_le
    · intro p hp; rw [hstk]; rw [hdep] at hp; exact w.stk_lt p hp
    · intro p q hpq hq; rw [hstk, hstk]; rw [hdep] at hq; exact w.sorted p q hpq hq
    · intro k hk; rw [hjob, hdep]; simp only [hstk]; exact w.on_stack k hk
    · intro k hk; rw [hjob]; exact w.run_iff k hk
    · intro k hk; rw [hrel]; exact w.pend_iff k hk
    · rw [hll, hlc]
      obtain ⟨c1, c2⟩ := w.ceil_is
      constructor
      · intro p hp
        rw [hlr]
        by_cases h : p = st.s.lock_len.val
        · simp only [h, ite_true]; exact Nat.le_max_right _ _
        · simp only [h, ite_false]
          exact Nat.le_trans (c1 p (by omega)) (Nat.le_max_left _ _)
      · by_cases hge : st.s.ceiling.val ≤ ceil cfg r.val
        · right
          refine ⟨st.s.lock_len.val, by omega, ?_⟩
          rw [hlr]; simp only [ite_true]; exact Nat.max_eq_right hge
        · rw [Nat.max_eq_left (by omega)]
          rcases c2 with h | ⟨p, hp, hcp⟩
          · left; exact h
          · right
            refine ⟨p, by omega, ?_⟩
            rw [hlr]; simp only [show p ≠ st.s.lock_len.val by omega, ite_false]; exact hcp
    · intro p hp; rw [hld, hdep]; rw [hll] at hp
      by_cases h : p = st.s.lock_len.val
      · simp only [h, ite_true]; omega
      · simp only [h, ite_false]; exact w.ldep_lt p (by omega)
    · intro p q hpq hq; rw [hld, hld]; rw [hll] at hq
      by_cases h : q = st.s.lock_len.val
      · simp only [h, ite_true]
        by_cases h' : p = st.s.lock_len.val
        · simp only [h', ite_true]; exact Nat.le_refl _
        · simp only [h', ite_false]
          have := w.ldep_lt p (by omega)
          omega
      · simp only [h, ite_false, show p ≠ st.s.lock_len.val by omega]
        exact w.ldep_mono p q hpq (by omega)
    · intro p q hp hq hpq; rw [hll] at hp; rw [hdep] at hq; rw [hld] at hpq
      rw [hlr, hstk]
      by_cases h : p = st.s.lock_len.val
      · simp only [h, ite_true] at hpq; omega
      · simp only [h, ite_false] at hpq ⊢; exact w.srp p q (by omega) hq hpq
    · intro p hp; rw [hll] at hp; rw [hld, hlr]; simp only [hstk]
      by_cases h : p = st.s.lock_len.val
      · simp only [h, ite_true, Function.update_self]; omega
      · simp only [h, ite_false]; rw [Function.update_of_ne h]
        exact Nat.le_trans (deduct_le _ _ _ _) (w.allow_le p (by omega))
    · intro _; have := w.stk_lt (st.s.depth.val - 1) (by omega); omega
  | unlock b hm hne hk0 r c s' hrun hru hc hal hs =>
    obtain ⟨⟨hrel, hjob, hstk, hdep⟩, hl1, hd1, _, _, hll, hlr, hld, hcl⟩ := unlock_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, w.jwait_inv, w.ewait_inv, w.rb_le,
      ?_⟩ <;> dsimp only
    · rw [hdep]; exact w.depth_le
    · intro p hp; rw [hstk]; rw [hdep] at hp; exact w.stk_lt p hp
    · intro p q hpq hq; rw [hstk, hstk]; rw [hdep] at hq; exact w.sorted p q hpq hq
    · intro k hk; rw [hjob, hdep]; simp only [hstk]; exact w.on_stack k hk
    · intro k hk; rw [hjob]; exact w.run_iff k hk
    · intro k hk; rw [hrel]; exact w.pend_iff k hk
    · rw [hll]; exact hcl.same hlr
    · intro p hp; rw [hld, hdep]; rw [hll] at hp; exact w.ldep_lt p (by omega)
    · intro p q hpq hq; rw [hld, hld]; rw [hll] at hq; exact w.ldep_mono p q hpq (by omega)
    · intro p q hp hq hpq; rw [hll] at hp; rw [hdep] at hq; rw [hld] at hpq
      rw [hlr, hstk]; exact w.srp p q (by omega) hq hpq
    · intro p hp; rw [hll] at hp; rw [hld, hlr, hstk]
      exact Nat.le_trans (deduct_le _ _ _ _) (w.allow_le p (by omega))
    · intro _; have := w.stk_lt (st.s.depth.val - 1) (by omega); omega
  | svc b hm hne hk0 c hd hrun hre hal =>
    refine ⟨w.depth_le, w.stk_lt, w.sorted, w.on_stack, w.run_iff, w.pend_iff, w.ceil_is,
      w.ldep_lt, w.ldep_mono, w.srp, ?_, w.jwait_inv, w.ewait_inv, w.rb_le, ?_⟩ <;> dsimp only
    · intro p hp; exact Nat.le_trans (deduct_le _ _ _ _) (w.allow_le p hp)
    · intro _; have := w.stk_lt (st.s.depth.val - 1) (by omega); omega
  | bevent b hm hne hk0 c hd hrun hrb hc =>
    refine ⟨w.depth_le, w.stk_lt, w.sorted, w.on_stack, w.run_iff, w.pend_iff, w.ceil_is,
      w.ldep_lt, w.ldep_mono, w.srp, w.allow_le, w.jwait_inv, w.ewait_inv, ?_, ?_⟩ <;>
      dsimp only
    · intro k
      by_cases h : k = stk st.s (st.s.depth.val - 1)
      · subst h; simp only [Function.update_self]; have := w.rb_le (stk st.s (st.s.depth.val - 1)); omega
      · rw [Function.update_of_ne h]; exact w.rb_le k
    · intro _; have := w.stk_lt (st.s.depth.val - 1) (by omega); omega
  | idle b hm hne hk0 hj hd =>
    refine ⟨w.depth_le, w.stk_lt, w.sorted, w.on_stack, w.run_iff, w.pend_iff, w.ceil_is,
      w.ldep_lt, w.ldep_mono, w.srp, w.allow_le, ?_, ?_, w.rb_le, w.kp_lt⟩ <;> dsimp only
    · intro k hk
      obtain ⟨a, ha, _⟩ := w.jwait_inv k hk
      exact ⟨a, ha, hj.1 k a hk ha⟩
    · intro k hk
      obtain ⟨a, ha, _⟩ := w.ewait_inv k hk
      exact ⟨a, ha, hj.2 k a hk ha⟩
  | exec b hm hne hk0 hj hd hr hal =>
    refine ⟨w.depth_le, w.stk_lt, w.sorted, w.on_stack, ?_, w.pend_iff, w.ceil_is,
      w.ldep_lt, w.ldep_mono, w.srp, ?_, ?_, ?_, w.rb_le, w.kp_lt⟩ <;> dsimp only
    · intro k hk
      by_cases h : k = stk st.s (st.s.depth.val - 1)
      · subst h
        have := (w.run_iff _ hk).mp (by omega)
        simp [Function.update]; exact ⟨fun _ => this, fun _ => hr⟩
      · rw [Function.update_of_ne h]; exact w.run_iff k hk
    · intro p hp; exact Nat.le_trans (deduct_le _ _ _ _) (w.allow_le p hp)
    · intro k hk
      obtain ⟨a, ha, _⟩ := w.jwait_inv k hk
      exact ⟨a, ha, hj.1 k a hk ha⟩
    · intro k hk
      obtain ⟨a, ha, _⟩ := w.ewait_inv k hk
      exact ⟨a, ha, hj.2 k a hk ha⟩
  | finish b hm hne hk0 hj ov c r s' hd hr0 hr hov hc hal hs =>
    have w1 := w.kend hs
    obtain ⟨h1, h2, h3, _⟩ := finish_top_views hs
    refine ⟨w1.depth_le, w1.stk_lt, w1.sorted, w1.on_stack, w1.run_iff, w1.pend_iff, w1.ceil_is,
      w1.ldep_lt, w1.ldep_mono, w1.srp, w1.allow_le, ?_, ?_, w.rb_le, ?_⟩ <;> dsimp only
    · intro k hk
      obtain ⟨a, ha, _⟩ := w.jwait_inv k hk
      exact ⟨a, ha, hj.1 k a hk ha⟩
    · intro k hk
      obtain ⟨a, ha, _⟩ := w.ewait_inv k hk
      exact ⟨a, ha, hj.2 k a hk ha⟩
    · intro _; omega
  | kend r s' hs => exact w.kend hs
  | kkill p s' hp hs =>
    obtain ⟨ht, hrel, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := kill_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, w.jwait_inv, w.ewait_inv, w.rb_le,
      w.kp_lt⟩ <;> dsimp only
    · rw [hdep]; exact w.depth_le
    · intro p' hp'; rw [hstk]; rw [hdep] at hp'; exact w.stk_lt p' hp'
    · intro p' q hpq hq; rw [hstk, hstk]; rw [hdep] at hq; exact w.sorted p' q hpq hq
    · intro k hk; rw [hjob, hdep]; simp only [hstk]
      by_cases h : k = stk st.s p.val
      · simp only [h, ite_true]
        constructor
        · intro _; exact ⟨p.val, hp, rfl⟩
        · intro _; omega
      · simp only [h, ite_false]; exact w.on_stack k hk
    · intro k hk; rw [hjob]
      by_cases h : k = stk st.s p.val
      · subst h; simp [Function.update]
      · rw [Function.update_of_ne h]; simp only [h, ite_false]; exact w.run_iff k hk
    · intro k hk; rw [hrel]; exact w.pend_iff k hk
    · rw [hll, hlc]; exact w.ceil_is.same hlr
    · intro p' hp'; rw [hld, hdep]; rw [hll] at hp'; exact w.ldep_lt p' hp'
    · intro p' q hpq hq; rw [hld, hld]; rw [hll] at hq; exact w.ldep_mono p' q hpq hq
    · intro p' q hp' hq hpq; rw [hll] at hp'; rw [hdep] at hq; rw [hld] at hpq
      rw [hlr, hstk]; exact w.srp p' q hp' hq hpq
    · intro p' hp'; rw [hll] at hp'; rw [hld, hlr, hstk]; exact w.allow_le p' hp'
  | kclear t s' hs =>
    obtain ⟨ht, hrl, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := clear_release_views hs
    refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, w.jwait_inv, w.ewait_inv, w.rb_le,
      w.kp_lt⟩ <;> dsimp only
    · rw [hdep]; exact w.depth_le
    · intro p hp; rw [hstk]; rw [hdep] at hp; exact w.stk_lt p hp
    · intro p q hpq hq; rw [hstk, hstk]; rw [hdep] at hq; exact w.sorted p q hpq hq
    · intro k hk; rw [hjob, hdep]; simp only [hstk]; exact w.on_stack k hk
    · intro k hk; rw [hjob]; exact w.run_iff k hk
    · intro k hk; rw [hrl]
      by_cases h : k = t.val
      · subst h; simp [Function.update]
      · rw [Function.update_of_ne h]; simp only [h, ite_false]; exact w.pend_iff k hk
    · rw [hll, hlc]; exact w.ceil_is.same hlr
    · intro p hp; rw [hld, hdep]; rw [hll] at hp; exact w.ldep_lt p hp
    · intro p q hpq hq; rw [hld, hld]; rw [hll] at hq; exact w.ldep_mono p q hpq hq
    · intro p q hp hq hpq; rw [hll] at hp; rw [hdep] at hq; rw [hld] at hpq
      rw [hlr, hstk]; exact w.srp p q hp hq hpq
    · intro p hp; rw [hll] at hp; rw [hld, hlr, hstk]; exact w.allow_le p hp

end P18
