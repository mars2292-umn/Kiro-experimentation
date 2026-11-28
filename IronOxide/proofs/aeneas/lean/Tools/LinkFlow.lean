import Tools.LinkFrame

/-! # The Link_Checker's flow certificate bounds the descent (2026-11-27)

`Tools/LinkFrame.lean` bounds the stack pointer's descent along a run by
the sum of the decrements of its steps, and by a body's frame under the
premise that no instruction of the body is executed twice. The tool did
not check that premise, and a body with a loop does not satisfy it (the
Kernel's unwind stub lowers SP by 16 and raises it by 16 each time round
its loop).

Since 2026-11-27 the Link_Checker has a certificate for each body
(`crates/rsk-link/src/flow_v.rs`, `flow_ok`, verified in Verus): a height
for every instruction, 0 for the first, such that along every passage of
control inside the body the height of the instruction reached is at least
the height of the one left plus what that one adds (`wt`: its decrement,
or minus its increment when it is under no `it`), and every height plus
the instruction's own decrement is within the frame. This file proves
what such heights give, for runs of any length, loops included:

- `step_wt`: one step lowers SP by no more than the instruction's weight.
- `path_height`: along a path of passages, SP falls by no more than the
  difference of the heights.
- `path_within_frame`, `step_within_frame`: from the first instruction, SP
  is never more than the frame below where it started, at an instruction
  boundary and after the next step.

A certificate here is the heights with the three conditions (`Cert`), over
instructions numbered from 0 and passages given as a relation. That the
conditions are the ones `flow_ok` checks (`flow_spec`: `d[0] == 0`, `lands`
with `need = d[i] + delta`, `d[i] + down <= frame`) is by reading the two
texts: the check is verified in Verus and this theorem is in Lean.

What a step is taken to do: `SpStepC`, which is `SpStep` (the decoder's
reading, proven against the instruction semantics for the forms it has,
`Tools/LinkThumbArm.lean`) and, for an instruction under no `it`, that an
increment read is made (an instruction outside an `it` block executes;
that the processor's IT state is clear there is not derived). As in
`Tools/LinkFrame.lean`, no step is dynamic or unread and no increment
takes SP above `top`. That control passes only along the passages is the
path's definition: `flow_spec` lists them (the next address unless the
instruction stops, a branch's target, a table's entries).

Handwritten; no `sorry`. -/

namespace rsk_link

open LinkThumb

/-- What an instruction adds to the height: its decrement, or minus its
increment when it is under no `it` (`c` false). `flow_v::delta`. -/
def wt (e : AEff) (c : Bool) : Int :=
  match e with
  | .spDown n => (n : Int)
  | .spUp n => if c then 0 else -(n : Int)
  | _ => 0

/-- A step of an instruction read as `e`, under an `it` of the body (`c`)
or not: SP changes as read or not at all, and outside an `it` an increment
read is made. -/
def SpStepC (e : AEff) (c : Bool) (s t : Arm.St) : Prop :=
  SpStep e s t ∧ (c = false → ∀ n, e = .spUp n → t.sp = s.sp + BitVec.ofNat 32 n)

/-- **One step lowers SP by no more than the instruction's weight.** -/
theorem step_wt {top : Nat} {e : AEff} {c : Bool} {s t : Arm.St} (hs : SpStepC e c s t)
    (hd : e ≠ .spDynamic) (hu : e ≠ .spUnread) (hup : ∀ n, e = .spUp n → s.sp.toNat + n ≤ top)
    (ht : top < 4294967296) : (s.sp.toNat : Int) - t.sp.toNat ≤ wt e c := by
  obtain ⟨h1, h2⟩ := hs
  have base := step_descent h1 hd hu hup ht
  cases e with
  | spDown n => simp only [dn] at base; simp only [wt]; omega
  | spUp n =>
    simp only [wt]
    cases c with
    | true => simp only [dn] at base; simp; omega
    | false =>
      have h := h2 rfl n rfl
      have e1 := congrArg BitVec.toNat h
      rw [BitVec.toNat_add, BitVec.toNat_ofNat] at e1
      have b0 := hup n rfl
      have b2 : n % 2 ^ 32 = n := Nat.mod_eq_of_lt (by omega)
      have b3 : (s.sp.toNat + n) % 2 ^ 32 = s.sp.toNat + n := Nat.mod_eq_of_lt (by omega)
      simp; omega
  | spDynamic => exact absurd rfl hd
  | spUnread => exact absurd rfl hu
  | _ => simp only [dn] at base; simp only [wt]; omega

/-- A body's certificate: a height for each instruction with the
conditions `flow_v::flow_spec` states. `edge i j`: control can pass from
instruction `i` to instruction `j` inside the body. -/
structure Cert where
  eff : Nat → AEff
  cond : Nat → Bool
  edge : Nat → Nat → Prop
  d : Nat → Int
  frame : Int
  first : d 0 = 0
  passes : ∀ i j, edge i j → d i + wt (eff i) (cond i) ≤ d j
  within : ∀ i, d i + (dn (eff i) : Int) ≤ frame

/-- The weight is at most the decrement. -/
theorem wt_le_dn (e : AEff) (c : Bool) : wt e c ≤ (dn e : Int) := by
  cases e <;> simp [wt, dn]
  split <;> omega

/-- A path of the body: from instruction `i` in state `s` to instruction
`k` in state `u`, each step an instruction of the body executed as read
and followed by a passage. -/
inductive Path (C : Cert) (top : Nat) : Nat → Arm.St → Nat → Arm.St → Prop
  | nil (i : Nat) (s : Arm.St) : Path C top i s i s
  | cons {i j k : Nat} {s t u : Arm.St} :
      SpStepC (C.eff i) (C.cond i) s t → C.eff i ≠ .spDynamic → C.eff i ≠ .spUnread →
      (∀ n, C.eff i = .spUp n → s.sp.toNat + n ≤ top) → top < 4294967296 →
      C.edge i j → Path C top j t k u → Path C top i s k u

/-- **Along a path SP falls by no more than the difference of the
heights**, however often the path passes an instruction. -/
theorem path_height {C : Cert} {top : Nat} {i k : Nat} {s u : Arm.St}
    (h : Path C top i s k u) : (s.sp.toNat : Int) - u.sp.toNat ≤ C.d k - C.d i := by
  induction h with
  | nil i s => simp
  | cons hs hd hu hup ht he _ ih =>
    have a := step_wt hs hd hu hup ht
    have b := C.passes _ _ he
    omega

/-- **From the first instruction, SP at an instruction boundary is within
the frame of where it started.** -/
theorem path_within_frame {C : Cert} {top : Nat} {k : Nat} {s u : Arm.St}
    (h : Path C top 0 s k u) : (s.sp.toNat : Int) ≤ u.sp.toNat + C.frame := by
  have a := path_height h
  have b := C.within k
  have c := C.first
  have d : (0 : Int) ≤ dn (C.eff k) := Int.natCast_nonneg _
  omega

/-- **And after the next step**, whatever passage follows it or none (the
instruction that leaves the body). -/
theorem step_within_frame {C : Cert} {top : Nat} {k : Nat} {s u t : Arm.St}
    (h : Path C top 0 s k u) (hs : SpStepC (C.eff k) (C.cond k) u t)
    (hd : C.eff k ≠ .spDynamic) (hu : C.eff k ≠ .spUnread)
    (hup : ∀ n, C.eff k = .spUp n → u.sp.toNat + n ≤ top) (ht : top < 4294967296) :
    (s.sp.toNat : Int) ≤ t.sp.toNat + C.frame := by
  have a := path_height h
  have b := step_wt hs hd hu hup ht
  have c := wt_le_dn (C.eff k) (C.cond k)
  have d := C.within k
  have e := C.first
  omega

end rsk_link
