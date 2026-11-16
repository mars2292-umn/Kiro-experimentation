import Mathlib.Tactic
/-!
# Multiparty Protocols: syntax, acceptance, semantics

The definitions of `rsk-protocol`'s theory (`crates/rsk-protocol/src/theory.rs`,
"the multiparty fragment"), restated in Lean: global and local types with
de Bruijn recursion variables, well-formedness over `n` roles (`wfn`), the
full merge (`merge`), the projection with it (`projn`), the acceptance
condition (`acceptn`), and the asynchronous semantics of the `n` projected
roles with one FIFO channel per ordered pair (`NStep`, `NReach`), with
R41.3's waiting set (`NDeadlocked`).

Roles, labels and variables are naturals. A configuration gives each role
its local type and each ordered pair of roles its channel as functions;
the Verus text indexes sequences (`ch[from * n + to]`).
-/

namespace MP

mutual
/-- A global type. -/
inductive G : Type
  | fin : G
  | comm (frm dst : Nat) (bs : Bs) : G
  | mu (body : G) : G
  | var (n : Nat) : G
/-- The branches of a communication: label and continuation. -/
inductive Bs : Type
  | nil : Bs
  | cons (label : Nat) (g : G) (rest : Bs) : Bs
end

mutual
/-- A local type. -/
inductive L : Type
  | fin : L
  | send (dst : Nat) (ls : Ls) : L
  | recv (frm : Nat) (ls : Ls) : L
  | mu (body : L) : L
  | var (n : Nat) : L
inductive Ls : Type
  | nil : Ls
  | cons (label : Nat) (l : L) (rest : Ls) : Ls
end

mutual
def L.beq : L → L → Bool
  | .fin, .fin => true
  | .send t ls, .send t' ls' => t == t' && Ls.beq ls ls'
  | .recv f ls, .recv f' ls' => f == f' && Ls.beq ls ls'
  | .mu b, .mu b' => L.beq b b'
  | .var n, .var n' => n == n'
  | _, _ => false
def Ls.beq : Ls → Ls → Bool
  | .nil, .nil => true
  | .cons a l r, .cons a' l' r' => a == a' && L.beq l l' && Ls.beq r r'
  | _, _ => false
end

mutual
theorem L.beq_iff : ∀ a b : L, L.beq a b = true ↔ a = b
  | .fin, b => by cases b <;> simp [L.beq]
  | .send t ls, b => by
    cases b <;> simp [L.beq]
    rename_i t' ls'
    rw [Ls.beq_iff ls ls']
    exact fun _ => Iff.rfl
  | .recv f ls, b => by
    cases b <;> simp [L.beq]
    rename_i f' ls'
    rw [Ls.beq_iff ls ls']
    exact fun _ => Iff.rfl
  | .mu body, b => by
    cases b <;> simp [L.beq]
    rename_i b'
    exact L.beq_iff body b'
  | .var n, b => by cases b <;> simp [L.beq]
theorem Ls.beq_iff : ∀ a b : Ls, Ls.beq a b = true ↔ a = b
  | .nil, b => by cases b <;> simp [Ls.beq]
  | .cons a l r, b => by
    cases b <;> simp [Ls.beq]
    rename_i a' l' r'
    rw [L.beq_iff l l', Ls.beq_iff r r']
    constructor
    · rintro ⟨⟨h1, h2⟩, h3⟩; exact ⟨h1, h2, h3⟩
    · rintro ⟨h1, h2, h3⟩; exact ⟨⟨h1, h2⟩, h3⟩
end

instance : DecidableEq L := fun a b =>
  if h : L.beq a b = true then isTrue ((L.beq_iff a b).mp h)
  else isFalse (fun e => h ((L.beq_iff a b).mpr e))

instance : DecidableEq Ls := fun a b =>
  if h : Ls.beq a b = true then isTrue ((Ls.beq_iff a b).mp h)
  else isFalse (fun e => h ((Ls.beq_iff a b).mpr e))

/-! ## Branch lists -/

def Bs.len : Bs → Nat
  | .nil => 0
  | .cons _ _ rest => 1 + rest.len

def Bs.hasLabel : Bs → Nat → Bool
  | .nil, _ => false
  | .cons l _ rest, label => l == label || rest.hasLabel label

def Bs.lookup : Bs → Nat → Option G
  | .nil, _ => none
  | .cons l g rest, label => if l = label then some g else rest.lookup label

def Bs.distinct : Bs → Bool
  | .nil => true
  | .cons label _ rest => !rest.hasLabel label && rest.distinct

def Ls.lookup : Ls → Nat → Option L
  | .nil, _ => none
  | .cons l c rest, label => if l = label then some c else rest.lookup label

def Ls.remove : Ls → Nat → Ls
  | .nil, _ => .nil
  | .cons l c rest, label =>
    if l = label then rest.remove label else .cons l c (rest.remove label)

/-! ## Well-formedness over `n` roles -/

/-- The guard environment with every enclosing recursion guarded. -/
def allTrue (env : List Bool) : List Bool := env.map (fun _ => true)

mutual
/-- `wfn_at`: closed, every recursion variable guarded by a message since
its own recursion, every communication between two distinct roles below
`n` with at least one branch and pairwise distinct labels. The guard
environment has one flag per enclosing recursion, the innermost first
(the Verus text and the checker keep the outermost first and index from
the end: the same environment read from the other side). -/
def G.wfnAt : G → Nat → List Bool → Bool
  | .fin, _, _ => true
  | .var v, _, env => env.getD v false
  | .mu body, n, env => body.wfnAt n (false :: env)
  | .comm frm dst bs, n, env =>
    decide (frm < n) && decide (dst < n) && decide (frm ≠ dst) && decide (1 ≤ bs.len) &&
      bs.distinct && bs.wfnBs n (allTrue env)
def Bs.wfnBs : Bs → Nat → List Bool → Bool
  | .nil, _, _ => true
  | .cons _ g rest, n, env => g.wfnAt n env && rest.wfnBs n env
end

def G.wfn (g : G) (n : Nat) : Bool := g.wfnAt n []

/-! ## The full merge -/

mutual
/-- The full merge of two local types (R38.3): equal types merge to
themselves; two external choices from the same role merge branch by
branch; two recursions merge by their bodies; anything else does not. -/
def merge : L → L → Option L
  | a, b =>
    if a = b then some a else
    match a, b with
    | .mu ba, .mu bb => (merge ba bb).map L.mu
    | .recv fa la, .recv fb lb =>
      if fa = fb then (mergeLs la lb).map (L.recv fa) else none
    | _, _ => none
/-- The branches of `a`, each merged with the branch of the same label in
`b` when there is one, then the branches of `b` whose labels `a` lacks. -/
def mergeLs : Ls → Ls → Option Ls
  | .nil, b => some b
  | .cons label l rest, b =>
    let m := match b.lookup label with
      | some lb => merge l lb
      | none => some l
    match m, mergeLs rest (b.remove label) with
    | some m, some r => some (.cons label m r)
    | _, _ => none
end

/-! ## Projection -/

/-- A local type with no action. -/
def L.actionless : L → Bool
  | .fin => true
  | .var _ => true
  | .mu body => body.actionless
  | _ => false

mutual
/-- Variable `k` occurs free. -/
def L.free : L → Nat → Bool
  | .fin, _ => false
  | .var n, k => n == k
  | .mu body, k => body.free (k + 1)
  | .send _ ls, k => ls.free k
  | .recv _ ls, k => ls.free k
def Ls.free : Ls → Nat → Bool
  | .nil, _ => false
  | .cons _ l rest, k => l.free k || rest.free k
end

mutual
/-- The removal of binder `k`: every variable above `k` renumbered down. -/
def L.shift : L → Nat → L
  | .fin, _ => .fin
  | .var n, k => if n > k then .var (n - 1) else .var n
  | .mu body, k => .mu (body.shift (k + 1))
  | .send dst ls, k => .send dst (ls.shift k)
  | .recv frm ls, k => .recv frm (ls.shift k)
def Ls.shift : Ls → Nat → Ls
  | .nil, _ => .nil
  | .cons label l rest, k => .cons label (l.shift k) (rest.shift k)
end

/-- The projection of a recursion from the projection `l` of its body. -/
def closeRec (l : L) : L :=
  if l.actionless then
    match l with
    | .var n => if n = 0 then .fin else .var (n - 1)
    | _ => .fin
  else if l.free 0 then .mu l
  else l.shift 0

mutual
/-- The projection onto role `r` with the full merge. -/
def projn : G → Nat → Option L
  | .fin, _ => some .fin
  | .var v, _ => some (.var v)
  | .mu body, r => (projn body r).map closeRec
  | .comm frm dst bs, r =>
    if r = frm then (projnBs bs r).map (L.send dst)
    else if r = dst then (projnBs bs r).map (L.recv frm)
    else projnMerge bs r
def projnBs : Bs → Nat → Option Ls
  | .nil, _ => some .nil
  | .cons label g rest, r =>
    match projn g r, projnBs rest r with
    | some l, some ls => some (.cons label l ls)
    | _, _ => none
/-- The merge of the projections of every branch (the role not involved). -/
def projnMerge : Bs → Nat → Option L
  | .nil, _ => none
  | .cons _ g rest, r =>
    match projn g r with
    | none => none
    | some l =>
      match rest with
      | .nil => some l
      | .cons .. => (projnMerge rest r).bind (merge l)
end

/-- The acceptance condition of an `n`-role Protocol over its syntax. -/
def acceptn (g : G) (n : Nat) : Prop :=
  2 ≤ n ∧ g.wfn n = true ∧ ∀ r, r < n → (projn g r).isSome

/-! ## Local substitution and the head normal form -/

mutual
def L.subst : L → Nat → L → L
  | .fin, _, _ => .fin
  | .var n, k, by' => if n = k then by' else if n > k then .var (n - 1) else .var n
  | .mu body, k, by' => .mu (body.subst (k + 1) by')
  | .send dst ls, k, by' => .send dst (ls.subst k by')
  | .recv frm ls, k, by' => .recv frm (ls.subst k by')
def Ls.subst : Ls → Nat → L → Ls
  | .nil, _, _ => .nil
  | .cons label l rest, k, by' => .cons label (l.subst k by') (rest.subst k by')
end

def L.unfold : L → L
  | .mu body => body.subst 0 (.mu body)
  | l => l

def L.leadRecs : L → Nat
  | .mu body => 1 + body.leadRecs
  | _ => 0

/-- The head of a local type, by at most `n` unfoldings. -/
def L.headN : L → Nat → L
  | l, 0 => l
  | .mu body, n + 1 => (L.mu body).unfold.headN n
  | l, _ + 1 => l

/-- The head normal form of a local type. -/
def L.head (l : L) : L := l.headN l.leadRecs

/-! ## The semantics of `n` roles -/

/-- A configuration: every role's local type and the channel of every
ordered pair of roles. -/
structure NC where
  l : Nat → L
  ch : Nat → Nat → List Nat

def ninit (g : G) : NC :=
  { l := fun r => (projn g r).getD .fin, ch := fun _ _ => [] }

/-- One step of the configuration: a role whose head is a send to another
role below `n` appends a label of its choice to the channel to that role;
a role whose head is a receive from a role below `n`, with a label at the
head of the channel from it that the receive has a branch for, takes it. -/
inductive NStep (n : Nat) : NC → NC → Prop
  | send (c : NC) (r dst label : Nat) (ls : Ls) (cont : L)
      (hr : r < n) (hh : (c.l r).head = .send dst ls) (hto : dst < n) (hne : dst ≠ r)
      (hl : ls.lookup label = some cont) :
      NStep n c
        { l := Function.update c.l r cont,
          ch := Function.update c.ch r (Function.update (c.ch r) dst (c.ch r dst ++ [label])) }
  | recv (c : NC) (r frm label : Nat) (ls : Ls) (cont : L) (tl : List Nat)
      (hr : r < n) (hh : (c.l r).head = .recv frm ls) (hfrm : frm < n)
      (hq : c.ch frm r = label :: tl) (hl : ls.lookup label = some cont) :
      NStep n c
        { l := Function.update c.l r cont,
          ch := Function.update c.ch frm (Function.update (c.ch frm) r tl) }

/-- The configurations reachable from the projections of `g`. -/
inductive NReach (g : G) (n : Nat) : NC → Prop
  | init : NReach g n (ninit g)
  | step (c c' : NC) (h : NReach g n c) (hs : NStep n c c') : NReach g n c'

/-- The role's head is a receive it cannot take. -/
def NBlocked (c : NC) (n r : Nat) : Prop :=
  ∃ frm ls, (c.l r).head = .recv frm ls ∧
    ¬ (frm < n ∧ ∃ label tl, c.ch frm r = label :: tl ∧ (ls.lookup label).isSome)

def NTerminal (c : NC) (r : Nat) : Prop := (c.l r).head = .fin

/-- R41.3's waiting set over `n` roles: some member awaits, every member
awaits or has finished, and the sender each awaiting member waits for is a
member. -/
def NDeadlockSet (c : NC) (n : Nat) (set : Nat → Bool) : Prop :=
  (∃ r, r < n ∧ set r = true ∧ NBlocked c n r) ∧
  (∀ r, r < n → set r = true → NBlocked c n r ∨ NTerminal c r) ∧
  (∀ r frm ls, r < n → set r = true → (c.l r).head = .recv frm ls → NBlocked c n r →
    set frm = true)

def NDeadlocked (c : NC) (n : Nat) : Prop := ∃ set, NDeadlockSet c n set

end MP
