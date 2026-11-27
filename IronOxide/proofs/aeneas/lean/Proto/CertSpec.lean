import Proto.Proto
open Aeneas Aeneas.Std Result
/-!
# The progress certificate: the statement (R41.3, R41.1)

`rsk-protocol`'s `progress.rs` states, for the roles' automata of one
Endpoint (`System`: the transitions `Trans`, the declared capacity), the
configurations (`Cfg`: every role's state, every directed channel's labels
in transit), the steps, the waiting set (`deadlock_set`), and the
certificate (`certificate`): a list of configurations that holds the
initial one, in which every configuration is well formed, within the
capacity and without a waiting set, and which every enabled operation
leaves inside the list. `theorem_r41_3` concludes that no reachable
configuration has a waiting set or a channel over the capacity.

This file restates those definitions over the translated types
(`progress.Trans`, `progress.System`) and proves the theorem
(`certificate_sound`). That the definitions say what the Verus ones say is
by reading; two differences of form: a run is the inductive `Reach` where
Verus has a sequence of configurations (`run`, `reachable`), joined by
`run_reach`, and an index outside a list reads a default where Verus
leaves the value unspecified (under `WfSys` no such index is read).
-/

namespace PC
open rsk_protocol

/-- `Cfg`: a configuration as the specification sees it. -/
structure Cfg where
  states : List Usize
  chans : List (List U32)

/-- `wf_sys`: at most 16 roles, every operation between two distinct roles
of the Endpoint. -/
def WfSys (s : progress.System) : Prop :=
  s.roles.val ≤ 16 ∧
  ∀ t, t ∈ s.trans.val → t.role.val < s.roles.val ∧ t.peer.val < s.roles.val ∧ t.peer ≠ t.role

/-- `wf_cfg`. -/
def WfCfg (n : Nat) (c : Cfg) : Prop := c.states.length = n ∧ c.chans.length = n * n

/-- `chan_of`: the channel an operation uses, from the sender to the
receiver. -/
def chanOf (n : Nat) (t : progress.Trans) : Nat :=
  if t.send = true then t.role.val * n + t.peer.val else t.peer.val * n + t.role.val

/-- `at`: the role is in the operation's state. -/
def At (c : Cfg) (t : progress.Trans) : Prop :=
  t.role.val < c.states.length ∧ c.states[t.role.val]! = t.«from»

/-- `enabled`: a send is enabled in its state; a receive when the oldest
message of its channel carries its label. -/
def Enabled (n : Nat) (c : Cfg) (t : progress.Trans) : Prop :=
  At c t ∧ chanOf n t < c.chans.length ∧
  (t.send = true ∨ (c.chans[chanOf n t]!).head? = some t.label)

/-- `succ`: the configuration after an enabled operation. -/
def succ (n : Nat) (c : Cfg) (t : progress.Trans) : Cfg :=
  { states := c.states.set t.role.val t.«to»,
    chans := c.chans.set (chanOf n t)
      (if t.send = true then c.chans[chanOf n t]! ++ [t.label]
       else (c.chans[chanOf n t]!).tail) }

/-- `step`. -/
def Step (s : progress.System) (a b : Cfg) : Prop :=
  ∃ t, t ∈ s.trans.val ∧ Enabled s.roles.val a t ∧ b = succ s.roles.val a t

/-- `initial`: every role in state 0, every channel empty. -/
def initial (n : Nat) : Cfg := ⟨List.replicate n 0#usize, List.replicate (n * n) []⟩

/-- `reachable`, inductively. -/
inductive Reach (s : progress.System) : Cfg → Prop
  | init : Reach s (initial s.roles.val)
  | step {a b : Cfg} : Reach s a → Step s a b → Reach s b

/-- `has_trans`: the role has an operation in its state. -/
def HasTrans (s : progress.System) (c : Cfg) (r : Nat) : Prop :=
  ∃ t, t ∈ s.trans.val ∧ t.role.val = r ∧ At c t

/-- `blocked`: the role awaits a message: it has operations, each a
receive, none enabled. -/
def Blocked (s : progress.System) (c : Cfg) (r : Nat) : Prop :=
  HasTrans s c r ∧
  ∀ t, t ∈ s.trans.val → t.role.val = r → At c t →
    t.send = false ∧ ¬ Enabled s.roles.val c t

/-- `terminal`: the role has finished. -/
def Terminal (s : progress.System) (c : Cfg) (r : Nat) : Prop := ¬ HasTrans s c r

/-- `deadlock_set`: R41.3's waiting set: at least one role of the set
awaits a message, every role of the set awaits or has finished, and the
sender of every awaited channel is in the set. -/
def DeadlockSet (s : progress.System) (c : Cfg) (set : List Bool) : Prop :=
  set.length = s.roles.val ∧
  (∃ r, r < set.length ∧ set[r]! = true ∧ Blocked s c r) ∧
  (∀ r, r < set.length → set[r]! = true → Blocked s c r ∨ Terminal s c r) ∧
  (∀ t, t ∈ s.trans.val → set[t.role.val]! = true → At c t → set[t.peer.val]! = true)

/-- `deadlocked`. -/
def Deadlocked (s : progress.System) (c : Cfg) : Prop := ∃ set, DeadlockSet s c set

/-- `within_capacity`: no channel holds more than the declared capacity
(R41.1). -/
def WithinCapacity (s : progress.System) (c : Cfg) : Prop :=
  ∀ q, q ∈ c.chans → q.length ≤ s.capacity.val

/-- `certificate`. -/
def Certificate (s : progress.System) (list : List Cfg) : Prop :=
  initial s.roles.val ∈ list ∧
  (∀ c, c ∈ list → WfCfg s.roles.val c ∧ WithinCapacity s c ∧ ¬ Deadlocked s c) ∧
  (∀ c t, c ∈ list → t ∈ s.trans.val → Enabled s.roles.val c t →
    succ s.roles.val c t ∈ list)

/-- `lemma_run_in_list`: every reachable configuration is in a
certificate's list. -/
theorem reach_in_list {s : progress.System} {list : List Cfg} (h : Certificate s list) :
    ∀ {c}, Reach s c → c ∈ list := by
  intro c hr
  induction hr with
  | init => exact h.1
  | step _ hs ih =>
    obtain ⟨t, ht, hen, rfl⟩ := hs
    exact h.2.2 _ t ih ht hen

/-- **`theorem_r41_3`**: with a certificate, no reachable configuration
has a waiting set and none has a channel over the capacity. -/
theorem certificate_sound {s : progress.System} {list : List Cfg} (h : Certificate s list)
    {c : Cfg} (hr : Reach s c) : ¬ Deadlocked s c ∧ WithinCapacity s c :=
  have hc := h.2.1 c (reach_in_list h hr)
  ⟨hc.2.2, hc.2.1⟩

/-! ## Runs as sequences (the form of the Verus text) -/

/-- `run`: the initial configuration, then a step at a time. -/
def Run (s : progress.System) : List Cfg → Prop
  | [] => False
  | [c] => c = initial s.roles.val
  | b :: a :: rest => Step s a b ∧ Run s (a :: rest)

/-- The configuration a run (written newest first) ends in is reachable,
so `certificate_sound` covers every configuration `reachable` of the Verus
text speaks of. -/
theorem run_reach {s : progress.System} : ∀ (path : List Cfg) (c : Cfg),
    Run s (c :: path) → Reach s c
  | [], c, h => by
    have e : c = initial s.roles.val := h
    rw [e]
    exact Reach.init
  | a :: rest, c, h => by
    have h' : Step s a c ∧ Run s (a :: rest) := h
    exact Reach.step (run_reach rest a h'.2) h'.1

/-- Every reachable configuration ends a run. -/
theorem reach_run {s : progress.System} {c : Cfg} (h : Reach s c) :
    ∃ path, Run s (c :: path) := by
  induction h with
  | init => exact ⟨[], rfl⟩
  | step _ hs ih =>
    obtain ⟨path, hp⟩ := ih
    exact ⟨_ :: path, hs, hp⟩

end PC
