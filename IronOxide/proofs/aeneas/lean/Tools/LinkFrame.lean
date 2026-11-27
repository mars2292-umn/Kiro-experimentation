import Tools.LinkThumbArm

/-! # The frame model of the Link_Checker (2026-11-26)

The Link_Checker takes as a function's frame the sum of the stack-pointer
decrements the decoder reads in its body (`bound_v::frame_of`, verified in
Verus), and bounds a Task's stack by sums of frames along call paths
(`cert`). This file is the step between the instructions and that sum: along
a run of the machine, the stack pointer descends by no more than the sum of
the decrements read for the instructions executed, and so by no more than
the frame when no instruction is executed twice.

- `RunBelow top es s u`: a run from `s` to `u` whose steps change SP as the
  effects `es` read for them say (`SpStep`, which `decode16_sp_step` and
  `decode32_sp_step` of `Tools/LinkThumbArm.lean` give for each instruction
  of the compiled decoder), with no effect the decoder cannot read
  (`spDynamic`), and in which no increment takes SP above `top` (a body
  pops no more than was pushed below the stack's top).
- `descent_le`: SP at the start is at most SP at the end plus the sum of
  the decrements of the steps.
- `descent_le_frame`: when the effects executed are a sublist of the
  body's (each instruction at most once, in any selection), the descent is
  at most the sum of the body's decrements, the frame.

What this does not say: that a path executes no instruction twice (a loop
whose body lowers SP is outside the frame model, and the Link_Checker has
no check for it), and nothing about calls between functions (the frames of
a call path are added by the certificate, `Tools/LinkCertProofs.lean`).
Handwritten; no compiled code is involved beyond `SpStep`'s link above. -/

namespace rsk_link

open LinkThumb

/-- The decrement an effect reads. -/
def dn : AEff → Nat
  | .spDown n => n
  | _ => 0

/-- The sum of the decrements of a list of effects. -/
def downs (es : List AEff) : Nat := (es.map dn).sum

/-- A run whose steps change SP as their effects say, none dynamic, no
increment taking SP above `top`. -/
inductive RunBelow (top : Nat) : List AEff → Arm.St → Arm.St → Prop
  | nil (s : Arm.St) : RunBelow top [] s s
  | cons {e : AEff} {es : List AEff} {s t u : Arm.St} :
      SpStep e s t → e ≠ .spDynamic →
      (∀ n, e = .spUp n → s.sp.toNat + n ≤ top) → top < 4294967296 →
      RunBelow top es t u → RunBelow top (e :: es) s u

theorem step_descent {top : Nat} {e : AEff} {s t : Arm.St} (hs : SpStep e s t)
    (hd : e ≠ .spDynamic) (hup : ∀ n, e = .spUp n → s.sp.toNat + n ≤ top)
    (ht : top < 4294967296) : s.sp.toNat ≤ t.sp.toNat + dn e := by
  cases e with
  | spDown n =>
    simp only [SpStep] at hs
    simp only [dn]
    rcases hs with h | h
    · have e1 := congrArg BitVec.toNat h
      rw [BitVec.toNat_sub, BitVec.toNat_ofNat] at e1
      have b1 := s.sp.isLt
      have b2 : n % 2 ^ 32 ≤ n := Nat.mod_le _ _
      have b3 : n % 2 ^ 32 < 2 ^ 32 := Nat.mod_lt _ (by norm_num)
      omega
    · rw [h]; omega
  | spUp n =>
    simp only [SpStep] at hs
    simp only [dn]
    rcases hs with h | h
    · have e1 := congrArg BitVec.toNat h
      rw [BitVec.toNat_add, BitVec.toNat_ofNat] at e1
      have b0 := hup n rfl
      have b2 : n % 2 ^ 32 = n := Nat.mod_eq_of_lt (by omega)
      have b3 : (s.sp.toNat + n) % 2 ^ 32 = s.sp.toNat + n := Nat.mod_eq_of_lt (by omega)
      omega
    · rw [h]; omega
  | spDynamic => exact absurd rfl hd
  | _ =>
    simp only [SpStep] at hs
    rw [hs]; omega

/-- **Descent.** Along a run, SP falls by no more than the sum of the
decrements read for its steps. -/
theorem descent_le {top : Nat} {es : List AEff} {s u : Arm.St} (h : RunBelow top es s u) :
    s.sp.toNat ≤ u.sp.toNat + downs es := by
  induction h with
  | nil s => simp [downs]
  | cons hs hd hup ht _ ih =>
    have a := step_descent hs hd hup ht
    simp only [downs, List.map_cons, List.sum_cons] at ih ⊢
    omega

theorem downs_sublist {es fs : List AEff} (h : es.Sublist fs) : downs es ≤ downs fs := by
  unfold downs
  exact List.Sublist.sum_le_sum (h.map dn) (fun _ _ => Nat.zero_le _)

/-- **Within the frame.** When the steps of a run are instructions of a
body, each at most once, SP falls by no more than the sum of the body's
decrements. -/
theorem descent_le_frame {top : Nat} {es fs : List AEff} {s u : Arm.St}
    (h : RunBelow top es s u) (hsub : es.Sublist fs) :
    s.sp.toNat ≤ u.sp.toNat + downs fs := by
  have a := descent_le h
  have b := downs_sublist hsub
  omega

end rsk_link
