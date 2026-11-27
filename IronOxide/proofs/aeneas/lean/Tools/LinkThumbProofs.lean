/-
  The Link_Checker's Thumb decoder on the code as compiled (2026-11-26):
  `thumb::decode16_static` and `thumb::decode32_static` of `rsk-link`, as
  Aeneas extracts them (`Tools/Link.lean`), return for every halfword (pair
  of halfwords) at an address below 2^30 and compute `LinkThumb.D16`
  (`LinkThumb.D32`, where it says) of `Tools/LinkThumbSpec.lean`
  (`decode16_spec`, `decode32_spec`). With the agreement theorems of that
  file: for every encoding the Armv7-M semantics of `Sched/Arm.lean`
  decodes, the compiled decoder's reading of the stack-pointer change and
  of the branch target is the semantics' (`decode16_arm`, `decode32_arm`).

  The address bound: the decoder computes a target as `pc as i32 + 4 + imm`
  and `pc + 4 + imm`, which overflow for an address near 2^31 or 2^32 (a
  panic in a build with overflow checks, a wrapped value without). Every
  address of the Target's memory map is below 2^30.

  Handwritten (the proofs of `decode16_spec` and `decode32_spec` follow the
  extracted functions line by line).
-/
import Aeneas
import Tools.Link
import Tools.LinkBits
import Tools.LinkThumbDefs
open Aeneas Aeneas.Std Result

set_option maxHeartbeats 1000000

namespace rsk_link
open thumb
open LinkThumb (AEff D16 D32)

/-- A reading of the decoder as `LinkThumb.AEff`: the numbers as naturals,
and every reading that neither moves SP nor names a target as `plain`. -/
def absE : Effect → AEff
  | .SpDown n => .spDown n.val
  | .SpUp n => .spUp n.val
  | .SpDynamic => .spDynamic
  | .Call t => .call t.val
  | .Branch t => .branch t.val
  | .CompareBranch t => .cbranch t.val
  | .Return => .ret
  | .IndirectBranch r => .ibranch r.val
  | .IndirectCall r => .icall r.val
  | _ => .plain

/-- A literal load reads no stack-pointer change and no target. -/
def absD : Decoded → AEff
  | .Is e => absE e
  | .Literal _ _ => .plain

/-! ## Masks as arithmetic -/

/-- A conjunction of a number below 2^16 with a constant mask, as the sum
of the fields the mask keeps. -/
macro "mask16" h:term : tactic =>
  `(tactic| (rw [LinkBits.and_bitsum 16 _ _ (by norm_num; exact $h)]
             (simp [LinkBits.bitsum]) <;> omega))

theorem and_FE00 (x : Nat) (hx : x < 65536) : x &&& 65024 = x / 512 * 512 := by mask16 hx
theorem and_FF80 (x : Nat) (hx : x < 65536) : x &&& 65408 = x / 128 * 128 := by mask16 hx
theorem and_FF87 (x : Nat) (hx : x < 65536) : x &&& 65415 = x / 128 * 128 + x % 8 := by mask16 hx
theorem and_F800 (x : Nat) (hx : x < 65536) : x &&& 63488 = x / 2048 * 2048 := by mask16 hx
theorem and_F000 (x : Nat) (hx : x < 65536) : x &&& 61440 = x / 4096 * 4096 := by mask16 hx
theorem and_FFEC (x : Nat) (hx : x < 65536) : x &&& 65516 = x / 32 * 32 + x / 4 % 4 * 4 := by mask16 hx
theorem and_F500 (x : Nat) (hx : x < 65536) : x &&& 62720 = x / 4096 * 4096 + x / 1024 % 2 * 1024 + x / 256 % 2 * 256 := by mask16 hx
theorem and_FF (x : Nat) (hx : x < 65536) : x &&& 255 = x % 256 := by mask16 hx
theorem and_7F (x : Nat) (hx : x < 65536) : x &&& 127 = x % 128 := by mask16 hx
theorem and_7FF (x : Nat) (hx : x < 65536) : x &&& 2047 = x % 2048 := by mask16 hx
theorem and_1F (x : Nat) (hx : x < 65536) : x &&& 31 = x % 32 := by mask16 hx
theorem and_F (x : Nat) (hx : x < 65536) : x &&& 15 = x % 16 := by mask16 hx
theorem and_7 (x : Nat) (hx : x < 65536) : x &&& 7 = x % 8 := by mask16 hx
theorem and_1 (x : Nat) (hx : x < 65536) : x &&& 1 = x % 2 := by mask16 hx
theorem and_FFF0 (x : Nat) (hx : x < 65536) : x &&& 65520 = x / 16 * 16 := by mask16 hx
theorem and_FFBF (x : Nat) (hx : x < 65536) : x &&& 65471 = x / 128 * 128 + x % 64 := by mask16 hx
theorem and_FBEF (x : Nat) (hx : x < 65536) : x &&& 64495 = x / 2048 * 2048 + x / 32 % 32 * 32 + x % 16 := by mask16 hx
theorem and_FBF0 (x : Nat) (hx : x < 65536) : x &&& 64496 = x / 2048 * 2048 + x / 16 % 64 * 16 := by mask16 hx
theorem and_EC00 (x : Nat) (hx : x < 65536) : x &&& 60416 = x / 8192 * 8192 + x / 1024 % 4 * 1024 := by mask16 hx
theorem and_E (x : Nat) (hx : x < 65536) : x &&& 14 = x / 2 % 8 * 2 := by mask16 hx
theorem and_FFE0 (x : Nat) (hx : x < 65536) : x &&& 65504 = x / 32 * 32 := by mask16 hx
theorem and_D000 (x : Nat) (hx : x < 65536) : x &&& 53248 = x / 16384 * 16384 + x / 4096 % 2 * 4096 := by mask16 hx
theorem and_4000 (x : Nat) (hx : x < 65536) : x &&& 16384 = x / 16384 % 2 * 16384 := by mask16 hx
theorem and_3FF (x : Nat) (hx : x < 65536) : x &&& 1023 = x % 1024 := by mask16 hx
theorem and_FFF (x : Nat) (hx : x < 65536) : x &&& 4095 = x % 4096 := by mask16 hx
theorem and_3F (x : Nat) (hx : x < 65536) : x &&& 63 = x % 64 := by mask16 hx

/-- The value of a conjunction of two `U32`. -/
theorem and_val (x m : U32) : (x &&& m).val = x.val &&& m.val := UScalar.val_and x m

theorem or_add (x y k : Nat) (hx : x % 2 ^ k = 0) (hy : y < 2 ^ k) : x ||| y = x + y := by
  have e : x = (x / 2 ^ k) <<< k := by
    rw [Nat.shiftLeft_eq]
    have := Nat.div_add_mod x (2 ^ k)
    rw [hx] at this
    rw [Nat.mul_comm]
    omega
  rw [e]
  exact (Nat.shiftLeft_add_eq_or_of_lt hy _).symm

theorem hcast_small (x : U32) (hx : x.val < 2147483648) :
    (UScalar.hcast .I32 x).val = x.val := by
  have e : (UScalar.hcast .I32 x).val = Int.bmod (x.val : Int) 4294967296 :=
    UScalar.hcast_val_eq .I32 x
  rw [e, Int.bmod_def]
  split <;> omega

theorem to_u32 (x : I32) : (IScalar.hcast .U32 x).val = (x.val % 4294967296).toNat :=
  IScalar.hcast_val_eq .U32 x

theorem cast_u8 (x : U32) (hx : x.val < 256) : (UScalar.cast .U8 x).val = x.val := by
  have e : (UScalar.cast .U8 x).val = x.val % 256 := UScalar.cast_val_eq .U8 x
  rw [e]
  omega

theorem u32_size : U32.size = 4294967296 := by
  first | scalar_tac | simp [U32.size_def, U32.numBits] | decide

/-! ## The bit count -/

theorem ones16_spec (v : U32) : ones16 v ⦃ r => r.val = LinkBits.ones 16 v.val ⦄ := by
  unfold ones16
  step as ⟨ t0, h0, _ ⟩
  have b0 : t0.val = v.val / 1 % 2 := by
    rw [h0, and_val]; show v.val &&& 1 = _; rw [Nat.and_one_is_mod]; omega
  have s0 : t0.val = LinkBits.ones 1 v.val := by
    rw [b0]; simp [LinkBits.ones]
  step as ⟨ q1, hq1, _ ⟩
  step as ⟨ r1, hr1, _ ⟩
  have b1 : r1.val = v.val / 2 % 2 := by
    rw [hr1, and_val]; show q1.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq1]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 1)
  have l1 : t0.val ≤ 1 := by
    rw [s0]; exact LinkBits.ones_le 1 v.val
  step as ⟨ t1, ht1 ⟩
  have s1 : t1.val = LinkBits.ones 2 v.val := by
    rw [ht1, s0, b1]; rfl
  step as ⟨ q2, hq2, _ ⟩
  step as ⟨ r2, hr2, _ ⟩
  have b2 : r2.val = v.val / 4 % 2 := by
    rw [hr2, and_val]; show q2.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq2]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 2)
  have l2 : t1.val ≤ 2 := by
    rw [s1]; exact LinkBits.ones_le 2 v.val
  step as ⟨ t2, ht2 ⟩
  have s2 : t2.val = LinkBits.ones 3 v.val := by
    rw [ht2, s1, b2]; rfl
  step as ⟨ q3, hq3, _ ⟩
  step as ⟨ r3, hr3, _ ⟩
  have b3 : r3.val = v.val / 8 % 2 := by
    rw [hr3, and_val]; show q3.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq3]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 3)
  have l3 : t2.val ≤ 3 := by
    rw [s2]; exact LinkBits.ones_le 3 v.val
  step as ⟨ t3, ht3 ⟩
  have s3 : t3.val = LinkBits.ones 4 v.val := by
    rw [ht3, s2, b3]; rfl
  step as ⟨ q4, hq4, _ ⟩
  step as ⟨ r4, hr4, _ ⟩
  have b4 : r4.val = v.val / 16 % 2 := by
    rw [hr4, and_val]; show q4.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq4]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 4)
  have l4 : t3.val ≤ 4 := by
    rw [s3]; exact LinkBits.ones_le 4 v.val
  step as ⟨ t4, ht4 ⟩
  have s4 : t4.val = LinkBits.ones 5 v.val := by
    rw [ht4, s3, b4]; rfl
  step as ⟨ q5, hq5, _ ⟩
  step as ⟨ r5, hr5, _ ⟩
  have b5 : r5.val = v.val / 32 % 2 := by
    rw [hr5, and_val]; show q5.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq5]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 5)
  have l5 : t4.val ≤ 5 := by
    rw [s4]; exact LinkBits.ones_le 5 v.val
  step as ⟨ t5, ht5 ⟩
  have s5 : t5.val = LinkBits.ones 6 v.val := by
    rw [ht5, s4, b5]; rfl
  step as ⟨ q6, hq6, _ ⟩
  step as ⟨ r6, hr6, _ ⟩
  have b6 : r6.val = v.val / 64 % 2 := by
    rw [hr6, and_val]; show q6.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq6]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 6)
  have l6 : t5.val ≤ 6 := by
    rw [s5]; exact LinkBits.ones_le 6 v.val
  step as ⟨ t6, ht6 ⟩
  have s6 : t6.val = LinkBits.ones 7 v.val := by
    rw [ht6, s5, b6]; rfl
  step as ⟨ q7, hq7, _ ⟩
  step as ⟨ r7, hr7, _ ⟩
  have b7 : r7.val = v.val / 128 % 2 := by
    rw [hr7, and_val]; show q7.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq7]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 7)
  have l7 : t6.val ≤ 7 := by
    rw [s6]; exact LinkBits.ones_le 7 v.val
  step as ⟨ t7, ht7 ⟩
  have s7 : t7.val = LinkBits.ones 8 v.val := by
    rw [ht7, s6, b7]; rfl
  step as ⟨ q8, hq8, _ ⟩
  step as ⟨ r8, hr8, _ ⟩
  have b8 : r8.val = v.val / 256 % 2 := by
    rw [hr8, and_val]; show q8.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq8]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 8)
  have l8 : t7.val ≤ 8 := by
    rw [s7]; exact LinkBits.ones_le 8 v.val
  step as ⟨ t8, ht8 ⟩
  have s8 : t8.val = LinkBits.ones 9 v.val := by
    rw [ht8, s7, b8]; rfl
  step as ⟨ q9, hq9, _ ⟩
  step as ⟨ r9, hr9, _ ⟩
  have b9 : r9.val = v.val / 512 % 2 := by
    rw [hr9, and_val]; show q9.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq9]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 9)
  have l9 : t8.val ≤ 9 := by
    rw [s8]; exact LinkBits.ones_le 9 v.val
  step as ⟨ t9, ht9 ⟩
  have s9 : t9.val = LinkBits.ones 10 v.val := by
    rw [ht9, s8, b9]; rfl
  step as ⟨ q10, hq10, _ ⟩
  step as ⟨ r10, hr10, _ ⟩
  have b10 : r10.val = v.val / 1024 % 2 := by
    rw [hr10, and_val]; show q10.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq10]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 10)
  have l10 : t9.val ≤ 10 := by
    rw [s9]; exact LinkBits.ones_le 10 v.val
  step as ⟨ t10, ht10 ⟩
  have s10 : t10.val = LinkBits.ones 11 v.val := by
    rw [ht10, s9, b10]; rfl
  step as ⟨ q11, hq11, _ ⟩
  step as ⟨ r11, hr11, _ ⟩
  have b11 : r11.val = v.val / 2048 % 2 := by
    rw [hr11, and_val]; show q11.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq11]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 11)
  have l11 : t10.val ≤ 11 := by
    rw [s10]; exact LinkBits.ones_le 11 v.val
  step as ⟨ t11, ht11 ⟩
  have s11 : t11.val = LinkBits.ones 12 v.val := by
    rw [ht11, s10, b11]; rfl
  step as ⟨ q12, hq12, _ ⟩
  step as ⟨ r12, hr12, _ ⟩
  have b12 : r12.val = v.val / 4096 % 2 := by
    rw [hr12, and_val]; show q12.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq12]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 12)
  have l12 : t11.val ≤ 12 := by
    rw [s11]; exact LinkBits.ones_le 12 v.val
  step as ⟨ t12, ht12 ⟩
  have s12 : t12.val = LinkBits.ones 13 v.val := by
    rw [ht12, s11, b12]; rfl
  step as ⟨ q13, hq13, _ ⟩
  step as ⟨ r13, hr13, _ ⟩
  have b13 : r13.val = v.val / 8192 % 2 := by
    rw [hr13, and_val]; show q13.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq13]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 13)
  have l13 : t12.val ≤ 13 := by
    rw [s12]; exact LinkBits.ones_le 13 v.val
  step as ⟨ t13, ht13 ⟩
  have s13 : t13.val = LinkBits.ones 14 v.val := by
    rw [ht13, s12, b13]; rfl
  step as ⟨ q14, hq14, _ ⟩
  step as ⟨ r14, hr14, _ ⟩
  have b14 : r14.val = v.val / 16384 % 2 := by
    rw [hr14, and_val]; show q14.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq14]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 14)
  have l14 : t13.val ≤ 14 := by
    rw [s13]; exact LinkBits.ones_le 14 v.val
  step as ⟨ t14, ht14 ⟩
  have s14 : t14.val = LinkBits.ones 15 v.val := by
    rw [ht14, s13, b14]; rfl
  step as ⟨ q15, hq15, _ ⟩
  step as ⟨ r15, hr15, _ ⟩
  have b15 : r15.val = v.val / 32768 % 2 := by
    rw [hr15, and_val]; show q15.val &&& 1 = _
    rw [Nat.and_one_is_mod, hq15]
    exact congrArg (· % 2) (Nat.shiftRight_eq_div_pow v.val 15)
  have l15 : t14.val ≤ 15 := by
    rw [s14]; exact LinkBits.ones_le 15 v.val
  step as ⟨ t15, ht15 ⟩
  have s15 : t15.val = LinkBits.ones 16 v.val := by
    rw [ht15, s14, b15]; rfl
  exact s15

/-! ## Sign extension -/

theorem sign_extend_9 (v : U32) (hv : v.val < 512) :
    sign_extend v 9#u32 ⦃ r =>
      r.val = if v.val < 256 then (v.val : Int) else (v.val : Int) - 512 ⦄ := by
  unfold sign_extend
  step as ⟨ shift, hs ⟩
  have e1 : shift.val = 23 := by scalar_tac
  step as ⟨ i, hi, _ ⟩
  step as ⟨ i1, hi1 ⟩
  step as ⟨ r, hr, _ ⟩
  rw [e1] at hi hr
  rw [u32_size] at hi
  have x1 := Nat.shiftLeft_eq v.val 23
  have e2 : i.val = v.val * 8388608 := by omega
  have e3 : i1.val = Int.bmod (i.val : Int) 4294967296 := by
    rw [hi1]; exact UScalar.hcast_val_eq .I32 i
  rw [Int.bmod_def] at e3
  have x2 := Int.shiftRight_eq_div_pow i1.val 23
  have e4 : r.val = i1.val / 8388608 := by omega
  split at e3 <;> split <;> omega

theorem sign_extend_12 (v : U32) (hv : v.val < 4096) :
    sign_extend v 12#u32 ⦃ r =>
      r.val = if v.val < 2048 then (v.val : Int) else (v.val : Int) - 4096 ⦄ := by
  unfold sign_extend
  step as ⟨ shift, hs ⟩
  have e1 : shift.val = 20 := by scalar_tac
  step as ⟨ i, hi, _ ⟩
  step as ⟨ i1, hi1 ⟩
  step as ⟨ r, hr, _ ⟩
  rw [e1] at hi hr
  rw [u32_size] at hi
  have x1 := Nat.shiftLeft_eq v.val 20
  have e2 : i.val = v.val * 1048576 := by omega
  have e3 : i1.val = Int.bmod (i.val : Int) 4294967296 := by
    rw [hi1]; exact UScalar.hcast_val_eq .I32 i
  rw [Int.bmod_def] at e3
  have x2 := Int.shiftRight_eq_div_pow i1.val 20
  have e4 : r.val = i1.val / 1048576 := by omega
  split at e3 <;> split <;> omega

theorem sign_extend_21 (v : U32) (hv : v.val < 2097152) :
    sign_extend v 21#u32 ⦃ r =>
      r.val = if v.val < 1048576 then (v.val : Int) else (v.val : Int) - 2097152 ⦄ := by
  unfold sign_extend
  step as ⟨ shift, hs ⟩
  have e1 : shift.val = 11 := by scalar_tac
  step as ⟨ i, hi, _ ⟩
  step as ⟨ i1, hi1 ⟩
  step as ⟨ r, hr, _ ⟩
  rw [e1] at hi hr
  rw [u32_size] at hi
  have x1 := Nat.shiftLeft_eq v.val 11
  have e2 : i.val = v.val * 2048 := by omega
  have e3 : i1.val = Int.bmod (i.val : Int) 4294967296 := by
    rw [hi1]; exact UScalar.hcast_val_eq .I32 i
  rw [Int.bmod_def] at e3
  have x2 := Int.shiftRight_eq_div_pow i1.val 11
  have e4 : r.val = i1.val / 2048 := by omega
  split at e3 <;> split <;> omega

theorem sign_extend_25 (v : U32) (hv : v.val < 33554432) :
    sign_extend v 25#u32 ⦃ r =>
      r.val = if v.val < 16777216 then (v.val : Int) else (v.val : Int) - 33554432 ⦄ := by
  unfold sign_extend
  step as ⟨ shift, hs ⟩
  have e1 : shift.val = 7 := by scalar_tac
  step as ⟨ i, hi, _ ⟩
  step as ⟨ i1, hi1 ⟩
  step as ⟨ r, hr, _ ⟩
  rw [e1] at hi hr
  rw [u32_size] at hi
  have x1 := Nat.shiftLeft_eq v.val 7
  have e2 : i.val = v.val * 128 := by omega
  have e3 : i1.val = Int.bmod (i.val : Int) 4294967296 := by
    rw [hi1]; exact UScalar.hcast_val_eq .I32 i
  rw [Int.bmod_def] at e3
  have x2 := Int.shiftRight_eq_div_pow i1.val 7
  have e4 : r.val = i1.val / 128 := by omega
  split at e3 <;> split <;> omega

/-! ## The 16-bit decoder -/

/-- **The 16-bit decoder.** `decode16_static` as compiled returns for every
halfword at an address below 2^30, and its reading is `D16`. -/
theorem decode16_spec (hw : U16) (pc : U32) (hpc : pc.val < 1073741824) :
    decode16_static hw pc ⦃ d => absD d = D16 hw.val pc.val ⦄ := by
  unfold decode16_static
  step as ⟨ h, hh ⟩
  have hv : h.val = hw.val := by rw [hh]; exact U16.cast_U32_val_eq hw
  have hlt : h.val < 65536 := by scalar_tac
  step as ⟨ m1, h_m1, _ ⟩
  have v_m1 : m1.val = h.val / 512 * 512 := by
    rw [h_m1, and_val]; exact and_FE00 _ (by omega)
  by_cases c_push : m1 = 46080#u32
  · rw [if_pos c_push]
    have p_push : hw.val / 512 = 90 := by
      have e : m1.val = 46080 := congrArg UScalar.val c_push
      omega
    step as ⟨ a1, h_a1, _ ⟩
    have v_a1 : a1.val = h.val % 256 := by
      rw [h_a1, and_val]; exact and_FF _ (by omega)
    step with (ones16_spec a1) as ⟨ a2, h_a2 ⟩
    rw [v_a1, hv] at h_a2
    have b_a2 : a2.val ≤ 16 := by rw [h_a2]; exact LinkBits.ones_le 16 _
    step as ⟨ a3, h_a3, _ ⟩
    have v_a3 : a3.val = h.val / 256 := by
      rw [h_a3]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ a4, h_a4, _ ⟩
    have v_a4 : a4.val = a3.val % 2 := by
      rw [h_a4, and_val]; exact and_1 _ (by omega)
    step as ⟨ regs, h_regs ⟩
    step as ⟨ a5, h_a5 ⟩
    have hD : D16 hw.val pc.val = .spDown (4 * (LinkBits.ones 16 (hw.val % 256) + hw.val / 256 % 2)) := by
      unfold LinkThumb.D16
      rw [if_pos p_push]
    (try simp only [WP.spec_ok])
    rw [hD]; show AEff.spDown _ = AEff.spDown _; congr 1; scalar_tac
  rw [if_neg c_push]
  have n_push : ¬ (hw.val / 512 = 90) := by
    have e : m1.val ≠ 46080 := fun e => c_push ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ m2, h_m2, _ ⟩
  have v_m2 : m2.val = h.val / 512 * 512 := by
    rw [h_m2, and_val]; exact and_FE00 _ (by omega)
  by_cases c_pop : m2 = 48128#u32
  · rw [if_pos c_pop]
    have p_pop : hw.val / 512 = 94 := by
      have e : m2.val = 48128 := congrArg UScalar.val c_pop
      omega
    step as ⟨ a2, h_a2, _ ⟩
    have v_a2 : a2.val = h.val / 256 := by
      rw [h_a2]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ a3, h_a3, _ ⟩
    have v_a3 : a3.val = a2.val % 2 := by
      rw [h_a3, and_val]; exact and_1 _ (by omega)
    by_cases c_poppc : a3 = 1#u32
    · rw [if_pos c_poppc]
      have p_poppc : hw.val / 256 % 2 = 1 := by
        have e : a3.val = 1 := congrArg UScalar.val c_poppc
        omega
      have hD : D16 hw.val pc.val = .ret := by
        unfold LinkThumb.D16
        rw [if_neg n_push, if_pos p_pop, if_pos p_poppc]
      (try simp only [WP.spec_ok])
      rw [hD]; rfl
    rw [if_neg c_poppc]
    have n_poppc : ¬ (hw.val / 256 % 2 = 1) := by
      have e : a3.val ≠ 1 := fun e => c_poppc ((UScalar.eq_equiv _ _).2 e)
      omega
    step as ⟨ a4, h_a4, _ ⟩
    have v_a4 : a4.val = h.val % 256 := by
      rw [h_a4, and_val]; exact and_FF _ (by omega)
    step with (ones16_spec a4) as ⟨ a5, h_a5 ⟩
    rw [v_a4, hv] at h_a5
    have b_a5 : a5.val ≤ 16 := by rw [h_a5]; exact LinkBits.ones_le 16 _
    step as ⟨ a6, h_a6 ⟩
    have hD : D16 hw.val pc.val = .spUp (4 * LinkBits.ones 16 (hw.val % 256)) := by
      unfold LinkThumb.D16
      rw [if_neg n_push, if_pos p_pop, if_neg n_poppc]
    (try simp only [WP.spec_ok])
    rw [hD]; show AEff.spUp _ = AEff.spUp _; congr 1; scalar_tac
  rw [if_neg c_pop]
  have n_pop : ¬ (hw.val / 512 = 94) := by
    have e : m2.val ≠ 48128 := fun e => c_pop ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ m3, h_m3, _ ⟩
  have v_m3 : m3.val = h.val / 128 * 128 := by
    rw [h_m3, and_val]; exact and_FF80 _ (by omega)
  by_cases c_addsp : m3 = 45056#u32
  · rw [if_pos c_addsp]
    have p_addsp : hw.val / 128 = 352 := by
      have e : m3.val = 45056 := congrArg UScalar.val c_addsp
      omega
    step as ⟨ a3, h_a3, _ ⟩
    have v_a3 : a3.val = h.val % 128 := by
      rw [h_a3, and_val]; exact and_7F _ (by omega)
    step as ⟨ a4, h_a4 ⟩
    have hD : D16 hw.val pc.val = .spUp (hw.val % 128 * 4) := by
      unfold LinkThumb.D16
      rw [if_neg n_push, if_neg n_pop, if_pos p_addsp]
    (try simp only [WP.spec_ok])
    rw [hD]; show AEff.spUp _ = AEff.spUp _; congr 1; scalar_tac
  rw [if_neg c_addsp]
  have n_addsp : ¬ (hw.val / 128 = 352) := by
    have e : m3.val ≠ 45056 := fun e => c_addsp ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ m4, h_m4, _ ⟩
  have v_m4 : m4.val = h.val / 128 * 128 := by
    rw [h_m4, and_val]; exact and_FF80 _ (by omega)
  by_cases c_subsp : m4 = 45184#u32
  · rw [if_pos c_subsp]
    have p_subsp : hw.val / 128 = 353 := by
      have e : m4.val = 45184 := congrArg UScalar.val c_subsp
      omega
    step as ⟨ a4, h_a4, _ ⟩
    have v_a4 : a4.val = h.val % 128 := by
      rw [h_a4, and_val]; exact and_7F _ (by omega)
    step as ⟨ a5, h_a5 ⟩
    have hD : D16 hw.val pc.val = .spDown (hw.val % 128 * 4) := by
      unfold LinkThumb.D16
      rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_pos p_subsp]
    (try simp only [WP.spec_ok])
    rw [hD]; show AEff.spDown _ = AEff.spDown _; congr 1; scalar_tac
  rw [if_neg c_subsp]
  have n_subsp : ¬ (hw.val / 128 = 353) := by
    have e : m4.val ≠ 45184 := fun e => c_subsp ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ m5, h_m5, _ ⟩
  have v_m5 : m5.val = h.val / 128 * 128 + h.val % 8 := by
    rw [h_m5, and_val]; exact and_FF87 _ (by omega)
  by_cases c_bx : m5 = 18176#u32
  · rw [if_pos c_bx]
    have p_bx : hw.val / 128 = 142 ∧ hw.val % 8 = 0 := by
      have e : m5.val = 18176 := congrArg UScalar.val c_bx
      omega
    step as ⟨ g1, h_g1, _ ⟩
    have v_g1 : g1.val = h.val / 8 := by
      rw [h_g1]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ g2, h_g2, _ ⟩
    have v_g2 : g2.val = g1.val % 16 := by
      rw [h_g2, and_val]; exact and_F _ (by omega)
    step as ⟨ rm, h_rm ⟩
    have v_rm : rm.val = g2.val := by rw [h_rm]; exact cast_u8 g2 (by omega)
    by_cases c_lr : rm = 14#u8
    · rw [if_pos c_lr]
      have p_lr : hw.val / 8 % 16 = 14 := by
        have e : rm.val = 14 := congrArg UScalar.val c_lr
        omega
      have hD : D16 hw.val pc.val = .ret := by
        unfold LinkThumb.D16
        rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_pos p_bx, if_pos p_lr]
      (try simp only [WP.spec_ok])
      rw [hD]; rfl
    rw [if_neg c_lr]
    have n_lr : ¬ (hw.val / 8 % 16 = 14) := by
      have e : rm.val ≠ 14 := fun e => c_lr ((UScalar.eq_equiv _ _).2 e)
      omega
    have hD : D16 hw.val pc.val = .ibranch (hw.val / 8 % 16) := by
      unfold LinkThumb.D16
      rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_pos p_bx, if_neg n_lr]
    (try simp only [WP.spec_ok])
    rw [hD]; show AEff.ibranch _ = AEff.ibranch _; congr 1; scalar_tac
  rw [if_neg c_bx]
  have n_bx : ¬ (hw.val / 128 = 142 ∧ hw.val % 8 = 0) := by
    have e : m5.val ≠ 18176 := fun e => c_bx ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ m6, h_m6, _ ⟩
  have v_m6 : m6.val = h.val / 128 * 128 + h.val % 8 := by
    rw [h_m6, and_val]; exact and_FF87 _ (by omega)
  by_cases c_blx : m6 = 18304#u32
  · rw [if_pos c_blx]
    have p_blx : hw.val / 128 = 143 ∧ hw.val % 8 = 0 := by
      have e : m6.val = 18304 := congrArg UScalar.val c_blx
      omega
    step as ⟨ g1, h_g1, _ ⟩
    have v_g1 : g1.val = h.val / 8 := by
      rw [h_g1]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ g2, h_g2, _ ⟩
    have v_g2 : g2.val = g1.val % 16 := by
      rw [h_g2, and_val]; exact and_F _ (by omega)
    step as ⟨ rm, h_rm ⟩
    have v_rm : rm.val = g2.val := by rw [h_rm]; exact cast_u8 g2 (by omega)
    have hD : D16 hw.val pc.val = .icall (hw.val / 8 % 16) := by
      unfold LinkThumb.D16
      rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_neg n_bx, if_pos p_blx]
    (try simp only [WP.spec_ok])
    rw [hD]; show AEff.icall _ = AEff.icall _; congr 1; scalar_tac
  rw [if_neg c_blx]
  have n_blx : ¬ (hw.val / 128 = 143 ∧ hw.val % 8 = 0) := by
    have e : m6.val ≠ 18304 := fun e => c_blx ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ m7, h_m7, _ ⟩
  have v_m7 : m7.val = h.val / 128 * 128 + h.val % 8 := by
    rw [h_m7, and_val]; exact and_FF87 _ (by omega)
  by_cases c_movsp : m7 = 18053#u32
  · rw [if_pos c_movsp]
    have p_movsp : hw.val / 128 = 141 ∧ hw.val % 8 = 5 := by
      have e : m7.val = 18053 := congrArg UScalar.val c_movsp
      omega
    have hD : D16 hw.val pc.val = .spDynamic := by
      unfold LinkThumb.D16
      rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_neg n_bx, if_neg n_blx, if_pos p_movsp]
    (try simp only [WP.spec_ok])
    rw [hD]; rfl
  rw [if_neg c_movsp]
  have n_movsp : ¬ (hw.val / 128 = 141 ∧ hw.val % 8 = 5) := by
    have e : m7.val ≠ 18053 := fun e => c_movsp ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ m8, h_m8, _ ⟩
  have v_m8 : m8.val = h.val / 128 * 128 + h.val % 8 := by
    rw [h_m8, and_val]; exact and_FF87 _ (by omega)
  by_cases c_addspr : m8 = 17541#u32
  · rw [if_pos c_addspr]
    have p_addspr : hw.val / 128 = 137 ∧ hw.val % 8 = 5 := by
      have e : m8.val = 17541 := congrArg UScalar.val c_addspr
      omega
    have hD : D16 hw.val pc.val = .spDynamic := by
      unfold LinkThumb.D16
      rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_neg n_bx, if_neg n_blx, if_neg n_movsp, if_pos p_addspr]
    (try simp only [WP.spec_ok])
    rw [hD]; rfl
  rw [if_neg c_addspr]
  have n_addspr : ¬ (hw.val / 128 = 137 ∧ hw.val % 8 = 5) := by
    have e : m8.val ≠ 17541 := fun e => c_addspr ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ m9, h_m9, _ ⟩
  have v_m9 : m9.val = h.val / 2048 * 2048 := by
    rw [h_m9, and_val]; exact and_F800 _ (by omega)
  by_cases c_lit : m9 = 18432#u32
  · rw [if_pos c_lit]
    have p_lit : hw.val / 2048 = 9 := by
      have e : m9.val = 18432 := congrArg UScalar.val c_lit
      omega
    step as ⟨ l1, h_l1, _ ⟩
    have v_l1 : l1.val = h.val / 256 := by
      rw [h_l1]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ l2, h_l2, _ ⟩
    have v_l2 : l2.val = l1.val % 8 := by
      rw [h_l2, and_val]; exact and_7 _ (by omega)
    step as ⟨ rt, h_rt ⟩
    step as ⟨ l3, h_l3 ⟩
    step as ⟨ l4, h_l4 ⟩
    step as ⟨ l5, h_l5, _ ⟩
    have b_l5 : l5.val ≤ l3.val := by rw [h_l5, and_val]; exact Nat.and_le_left
    step as ⟨ l6, h_l6, _ ⟩
    have v_l6 : l6.val = h.val % 256 := by
      rw [h_l6, and_val]; exact and_FF _ (by omega)
    step as ⟨ l7, h_l7 ⟩
    step as ⟨ addr, h_addr ⟩
    have hD : D16 hw.val pc.val = .plain := by
      unfold LinkThumb.D16
      rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_neg n_bx, if_neg n_blx, if_neg n_movsp, if_neg n_addspr, if_pos p_lit]
    (try simp only [WP.spec_ok])
    rw [hD]; rfl
  rw [if_neg c_lit]
  have n_lit : ¬ (hw.val / 2048 = 9) := by
    have e : m9.val ≠ 18432 := fun e => c_lit ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ m10, h_m10, _ ⟩
  have v_m10 : m10.val = h.val / 4096 * 4096 := by
    rw [h_m10, and_val]; exact and_F000 _ (by omega)
  by_cases c_bc : m10 = 53248#u32
  · rw [if_pos c_bc]
    have p_bc : hw.val / 4096 = 13 := by
      have e : m10.val = 53248 := congrArg UScalar.val c_bc
      omega
    step as ⟨ q1, h_q1, _ ⟩
    have v_q1 : q1.val = h.val / 256 := by
      rw [h_q1]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ cond, h_cond, _ ⟩
    have v_cond : cond.val = q1.val % 16 := by
      rw [h_cond, and_val]; exact and_F _ (by omega)
    by_cases c_svc : cond = 15#u32
    · rw [if_pos c_svc]
      have p_svc : hw.val / 256 % 16 = 15 := by
        have e : cond.val = 15 := congrArg UScalar.val c_svc
        omega
      step as ⟨ s1, h_s1, _ ⟩
      have v_s1 : s1.val = h.val % 256 := by
        rw [h_s1, and_val]; exact and_FF _ (by omega)
      step as ⟨ s2, h_s2 ⟩
      have hD : D16 hw.val pc.val = .plain := by
        unfold LinkThumb.D16
        rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_neg n_bx, if_neg n_blx, if_neg n_movsp, if_neg n_addspr, if_neg n_lit, if_pos p_bc, if_pos p_svc]
      (try simp only [WP.spec_ok])
      rw [hD]; rfl
    rw [if_neg c_svc]
    have n_svc : ¬ (hw.val / 256 % 16 = 15) := by
      have e : cond.val ≠ 15 := fun e => c_svc ((UScalar.eq_equiv _ _).2 e)
      omega
    by_cases c_udf : cond = 14#u32
    · rw [if_pos c_udf]
      have p_udf : hw.val / 256 % 16 = 14 := by
        have e : cond.val = 14 := congrArg UScalar.val c_udf
        omega
      have hD : D16 hw.val pc.val = .plain := by
        unfold LinkThumb.D16
        rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_neg n_bx, if_neg n_blx, if_neg n_movsp, if_neg n_addspr, if_neg n_lit, if_pos p_bc, if_neg n_svc, if_pos p_udf]
      (try simp only [WP.spec_ok])
      rw [hD]; rfl
    rw [if_neg c_udf]
    have n_udf : ¬ (hw.val / 256 % 16 = 14) := by
      have e : cond.val ≠ 14 := fun e => c_udf ((UScalar.eq_equiv _ _).2 e)
      omega
    step as ⟨ b1, h_b1, _ ⟩
    have v_b1 : b1.val = h.val % 256 := by
      rw [h_b1, and_val]; exact and_FF _ (by omega)
    step as ⟨ b2, h_b2, _ ⟩
    have v_b2 : b2.val = b1.val * 2 := by
      rw [u32_size] at h_b2
      have x := Nat.shiftLeft_eq b1.val 1
      omega
    step with (sign_extend_9 b2 (by omega)) as ⟨ imm, h_imm ⟩
    have b_imm : -256 ≤ imm.val ∧ imm.val < 256 := by rw [h_imm]; split <;> omega
    step as ⟨ j1, h_j1 ⟩
    have v_j1 : j1.val = pc.val := by rw [h_j1]; exact hcast_small pc (by omega)
    step as ⟨ j2, h_j2 ⟩
    step as ⟨ j3, h_j3 ⟩
    step as ⟨ j4, h_j4 ⟩
    have v_j4 : j4.val = (j3.val % 4294967296).toNat := by rw [h_j4]; exact to_u32 j3
    have key : j3.val = (pc.val : Int) + 4 + LinkThumb.off9 hw.val := by
      unfold LinkThumb.off9
      rw [h_imm] at h_j3
      split at h_j3 <;> split <;> scalar_tac
    have hD : D16 hw.val pc.val = .branch (LinkThumb.tgt pc.val (LinkThumb.off9 hw.val)) := by
      unfold LinkThumb.D16
      rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_neg n_bx, if_neg n_blx, if_neg n_movsp, if_neg n_addspr, if_neg n_lit, if_pos p_bc, if_neg n_svc, if_neg n_udf]
    (try simp only [WP.spec_ok])
    rw [hD]; show AEff.branch _ = AEff.branch _; congr 1
    rw [v_j4, key]; first | rfl | skip
  rw [if_neg c_bc]
  have n_bc : ¬ (hw.val / 4096 = 13) := by
    have e : m10.val ≠ 53248 := fun e => c_bc ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ m11, h_m11, _ ⟩
  have v_m11 : m11.val = h.val / 2048 * 2048 := by
    rw [h_m11, and_val]; exact and_F800 _ (by omega)
  by_cases c_b : m11 = 57344#u32
  · rw [if_pos c_b]
    have p_b : hw.val / 2048 = 28 := by
      have e : m11.val = 57344 := congrArg UScalar.val c_b
      omega
    step as ⟨ b1, h_b1, _ ⟩
    have v_b1 : b1.val = h.val % 2048 := by
      rw [h_b1, and_val]; exact and_7FF _ (by omega)
    step as ⟨ b2, h_b2, _ ⟩
    have v_b2 : b2.val = b1.val * 2 := by
      rw [u32_size] at h_b2
      have x := Nat.shiftLeft_eq b1.val 1
      omega
    step with (sign_extend_12 b2 (by omega)) as ⟨ imm, h_imm ⟩
    have b_imm : -2048 ≤ imm.val ∧ imm.val < 2048 := by rw [h_imm]; split <;> omega
    step as ⟨ j1, h_j1 ⟩
    have v_j1 : j1.val = pc.val := by rw [h_j1]; exact hcast_small pc (by omega)
    step as ⟨ j2, h_j2 ⟩
    step as ⟨ j3, h_j3 ⟩
    step as ⟨ j4, h_j4 ⟩
    have v_j4 : j4.val = (j3.val % 4294967296).toNat := by rw [h_j4]; exact to_u32 j3
    have key : j3.val = (pc.val : Int) + 4 + LinkThumb.off12 hw.val := by
      unfold LinkThumb.off12
      rw [h_imm] at h_j3
      split at h_j3 <;> split <;> scalar_tac
    have hD : D16 hw.val pc.val = .branch (LinkThumb.tgt pc.val (LinkThumb.off12 hw.val)) := by
      unfold LinkThumb.D16
      rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_neg n_bx, if_neg n_blx, if_neg n_movsp, if_neg n_addspr, if_neg n_lit, if_neg n_bc, if_pos p_b]
    (try simp only [WP.spec_ok])
    rw [hD]; show AEff.branch _ = AEff.branch _; congr 1
    rw [v_j4, key]; first | rfl | skip
  rw [if_neg c_b]
  have n_b : ¬ (hw.val / 2048 = 28) := by
    have e : m11.val ≠ 57344 := fun e => c_b ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ m12, h_m12, _ ⟩
  have v_m12 : m12.val = h.val / 32 * 32 + h.val / 4 % 4 * 4 := by
    rw [h_m12, and_val]; exact and_FFEC _ (by omega)
  by_cases c_cps : m12 = 46688#u32
  · rw [if_pos c_cps]
    have p_cps : hw.val / 32 = 1459 ∧ hw.val / 4 % 4 = 0 := by
      have e : m12.val = 46688 := congrArg UScalar.val c_cps
      omega
    have hD : D16 hw.val pc.val = .plain := by
      unfold LinkThumb.D16
      rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_neg n_bx, if_neg n_blx, if_neg n_movsp, if_neg n_addspr, if_neg n_lit, if_neg n_bc, if_neg n_b, if_pos p_cps]
    (try simp only [WP.spec_ok])
    rw [hD]; rfl
  rw [if_neg c_cps]
  have n_cps : ¬ (hw.val / 32 = 1459 ∧ hw.val / 4 % 4 = 0) := by
    have e : m12.val ≠ 46688 := fun e => c_cps ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ m13, h_m13, _ ⟩
  have v_m13 : m13.val = h.val / 4096 * 4096 + h.val / 1024 % 2 * 1024 + h.val / 256 % 2 * 256 := by
    rw [h_m13, and_val]; exact and_F500 _ (by omega)
  by_cases c_cbz : m13 = 45312#u32
  · rw [if_pos c_cbz]
    have p_cbz : hw.val / 4096 = 11 ∧ hw.val / 1024 % 2 = 0 ∧ hw.val / 256 % 2 = 1 := by
      have e : m13.val = 45312 := congrArg UScalar.val c_cbz
      omega
    step as ⟨ z1, h_z1, _ ⟩
    have v_z1 : z1.val = h.val / 512 := by
      rw [h_z1]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ z2, h_z2, _ ⟩
    have v_z2 : z2.val = z1.val % 2 := by
      rw [h_z2, and_val]; exact and_1 _ (by omega)
    step as ⟨ z3, h_z3, _ ⟩
    have v_z3 : z3.val = h.val / 8 := by
      rw [h_z3]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ imm5, h_imm5, _ ⟩
    have v_imm5 : imm5.val = z3.val % 32 := by
      rw [h_imm5, and_val]; exact and_1F _ (by omega)
    step as ⟨ z4, h_z4, _ ⟩
    have v_z4 : z4.val = z2.val * 64 := by
      rw [u32_size] at h_z4
      have x := Nat.shiftLeft_eq z2.val 6
      omega
    step as ⟨ z5, h_z5, _ ⟩
    have v_z5 : z5.val = imm5.val * 2 := by
      rw [u32_size] at h_z5
      have x := Nat.shiftLeft_eq imm5.val 1
      omega
    step as ⟨ imm, h_imm, _ ⟩
    have v_imm : imm.val = z4.val + z5.val := by
      rw [h_imm, UScalar.val_or]; exact or_add _ _ 6 (by omega) (by omega)
    step as ⟨ z6, h_z6 ⟩
    step as ⟨ z7, h_z7 ⟩
    have hD : D16 hw.val pc.val = .cbranch (pc.val + 4 + ((hw.val / 512 % 2) * 64 + (hw.val / 8 % 32) * 2)) := by
      unfold LinkThumb.D16
      rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_neg n_bx, if_neg n_blx, if_neg n_movsp, if_neg n_addspr, if_neg n_lit, if_neg n_bc, if_neg n_b, if_neg n_cps, if_pos p_cbz]
    (try simp only [WP.spec_ok])
    rw [hD]; show AEff.cbranch _ = AEff.cbranch _; congr 1; scalar_tac
  rw [if_neg c_cbz]
  have n_cbz : ¬ (hw.val / 4096 = 11 ∧ hw.val / 1024 % 2 = 0 ∧ hw.val / 256 % 2 = 1) := by
    have e : m13.val ≠ 45312 := fun e => c_cbz ((UScalar.eq_equiv _ _).2 e)
    omega
  have hD : D16 hw.val pc.val = .plain := by
    unfold LinkThumb.D16
    rw [if_neg n_push, if_neg n_pop, if_neg n_addsp, if_neg n_subsp, if_neg n_bx, if_neg n_blx, if_neg n_movsp, if_neg n_addspr, if_neg n_lit, if_neg n_bc, if_neg n_b, if_neg n_cps, if_neg n_cbz]
  by_cases c_nop : h = 48896#u32
  · rw [if_pos c_nop]
    simp only [WP.spec_ok]
    rw [hD]; rfl
  rw [if_neg c_nop]
  simp only [WP.spec_ok]
  rw [hD]; rfl

/-! ## The 32-bit decoder -/

theorem both_spec (p q : Bool) : both p q ⦃ r => r = (p && q) ⦄ := by
  unfold both
  cases p <;> cases q <;> simp [WP.spec_ok]

theorem either_spec (p q : Bool) : either p q ⦃ r => r = (p || q) ⦄ := by
  unfold either
  cases p <;> cases q <;> simp [WP.spec_ok]

/-- `(!(j ^ s)) & 1` for two bits: whether they are equal. -/
theorem xnor1 (j s : U32) (hj : j.val ≤ 1) (hs : s.val ≤ 1) :
    ((~~~(j ^^^ s)) &&& 1#u32).val = if j.val = s.val then 1 else 0 := by
  have ej : j = 0#u32 ∨ j = 1#u32 := by
    rcases (show j.val = 0 ∨ j.val = 1 by omega) with e | e
    · left; exact (UScalar.eq_equiv _ _).2 e
    · right; exact (UScalar.eq_equiv _ _).2 e
  have es : s = 0#u32 ∨ s = 1#u32 := by
    rcases (show s.val = 0 ∨ s.val = 1 by omega) with e | e
    · left; exact (UScalar.eq_equiv _ _).2 e
    · right; exact (UScalar.eq_equiv _ _).2 e
  rcases ej with rfl | rfl <;> rcases es with rfl | rfl <;> first | rfl | decide | simp

/-- The modified-immediate expansion returns (its value is not used here). -/
theorem thumb_expand_imm_total (x : U32) : thumb_expand_imm x ⦃ _ => True ⦄ := by
  unfold thumb_expand_imm
  step as ⟨ i, hi, _ ⟩
  step as ⟨ a, ha, _ ⟩
  step as ⟨ i1, hi1, _ ⟩
  step as ⟨ b, hb, _ ⟩
  step as ⟨ imm8, him, _ ⟩
  split
  · split
    · simp only [WP.spec_ok]
    · step*
    · step*
    · step*
  · step*

theorem eq_0 (x : U32) : x = 0#u32 ↔ x.val = 0 := UScalar.eq_equiv x 0#u32
theorem eq_5 (x : U32) : x = 5#u32 ↔ x.val = 5 := UScalar.eq_equiv x 5#u32
theorem eq_10 (x : U32) : x = 10#u32 ↔ x.val = 10 := UScalar.eq_equiv x 10#u32
theorem eq_13 (x : U32) : x = 13#u32 ↔ x.val = 13 := UScalar.eq_equiv x 13#u32
theorem eq_15 (x : U32) : x = 15#u32 ↔ x.val = 15 := UScalar.eq_equiv x 15#u32
theorem eq_32768 (x : U32) : x = 32768#u32 ↔ x.val = 32768 := UScalar.eq_equiv x 32768#u32
theorem eq_36672 (x : U32) : x = 36672#u32 ↔ x.val = 36672 := UScalar.eq_equiv x 36672#u32
theorem eq_36864 (x : U32) : x = 36864#u32 ↔ x.val = 36864 := UScalar.eq_equiv x 36864#u32
theorem eq_53248 (x : U32) : x = 53248#u32 ↔ x.val = 53248 := UScalar.eq_equiv x 53248#u32
theorem eq_59600 (x : U32) : x = 59600#u32 ↔ x.val = 59600 := UScalar.eq_equiv x 59600#u32
theorem eq_60173 (x : U32) : x = 60173#u32 ↔ x.val = 60173 := UScalar.eq_equiv x 60173#u32
theorem eq_60189 (x : U32) : x = 60189#u32 ↔ x.val = 60189 := UScalar.eq_equiv x 60189#u32
theorem eq_60333 (x : U32) : x = 60333#u32 ↔ x.val = 60333 := UScalar.eq_equiv x 60333#u32
theorem eq_60349 (x : U32) : x = 60349#u32 ↔ x.val = 60349 := UScalar.eq_equiv x 60349#u32
theorem eq_60416 (x : U32) : x = 60416#u32 ↔ x.val = 60416 := UScalar.eq_equiv x 60416#u32
theorem eq_61440 (x : U32) : x = 61440#u32 ↔ x.val = 61440 := UScalar.eq_equiv x 61440#u32
theorem eq_61519 (x : U32) : x = 61519#u32 ↔ x.val = 61519 := UScalar.eq_equiv x 61519#u32
theorem eq_61709 (x : U32) : x = 61709#u32 ↔ x.val = 61709 := UScalar.eq_equiv x 61709#u32
theorem eq_61725 (x : U32) : x = 61725#u32 ↔ x.val = 61725 := UScalar.eq_equiv x 61725#u32
theorem eq_61869 (x : U32) : x = 61869#u32 ↔ x.val = 61869 := UScalar.eq_equiv x 61869#u32
theorem eq_61885 (x : U32) : x = 61885#u32 ↔ x.val = 61885 := UScalar.eq_equiv x 61885#u32
theorem eq_61965 (x : U32) : x = 61965#u32 ↔ x.val = 61965 := UScalar.eq_equiv x 61965#u32
theorem eq_62016 (x : U32) : x = 62016#u32 ↔ x.val = 62016 := UScalar.eq_equiv x 62016#u32
theorem eq_62125 (x : U32) : x = 62125#u32 ↔ x.val = 62125 := UScalar.eq_equiv x 62125#u32
theorem eq_62144 (x : U32) : x = 62144#u32 ↔ x.val = 62144 := UScalar.eq_equiv x 62144#u32
theorem eq_62336 (x : U32) : x = 62336#u32 ↔ x.val = 62336 := UScalar.eq_equiv x 62336#u32
theorem eq_62399 (x : U32) : x = 62399#u32 ↔ x.val = 62399 := UScalar.eq_equiv x 62399#u32
theorem eq_62447 (x : U32) : x = 62447#u32 ↔ x.val = 62447 := UScalar.eq_equiv x 62447#u32
theorem eq_63565 (x : U32) : x = 63565#u32 ↔ x.val = 63565 := UScalar.eq_equiv x 63565#u32
theorem eq_63568 (x : U32) : x = 63568#u32 ↔ x.val = 63568 := UScalar.eq_equiv x 63568#u32
theorem eq_63581 (x : U32) : x = 63581#u32 ↔ x.val = 63581 := UScalar.eq_equiv x 63581#u32
theorem eq_63583 (x : U32) : x = 63583#u32 ↔ x.val = 63583 := UScalar.eq_equiv x 63583#u32
theorem eq_63696 (x : U32) : x = 63696#u32 ↔ x.val = 63696 := UScalar.eq_equiv x 63696#u32
theorem eq_63711 (x : U32) : x = 63711#u32 ↔ x.val = 63711 := UScalar.eq_equiv x 63711#u32

/-- A test built from `both` and `either` over comparisons with constants,
as a statement about numbers. -/
theorem raw25_lt (a b : Nat) : LinkThumb.raw25 a b < 33554432 := by
  unfold LinkThumb.raw25
  split <;> split <;> omega

macro "bool_nat" "at" h:ident : tactic =>
  `(tactic| simp only [Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq,
      eq_0, eq_5, eq_10, eq_13, eq_15, eq_32768, eq_36672, eq_36864, eq_53248, eq_59600, eq_60173, eq_60189, eq_60333, eq_60349, eq_60416, eq_61440, eq_61519, eq_61709, eq_61725, eq_61869, eq_61885, eq_61965, eq_62016, eq_62125, eq_62144, eq_62336, eq_62399, eq_62447, eq_63565, eq_63568, eq_63581, eq_63583, eq_63696, eq_63711] at $h:ident)

-- One declaration of some 950 lines: the budget is per declaration.
set_option maxHeartbeats 8000000 in
/-- **The 32-bit decoder.** `decode32_static` as compiled returns for every
pair of halfwords at an address below 2^30, and its reading is `D32` where
`D32` says. -/
theorem decode32_spec (hw1 hw2 : U16) (pc : U32) (hpc : pc.val < 1073741824) :
    decode32_static hw1 hw2 pc ⦃ d => ∀ x, D32 hw1.val hw2.val pc.val = some x → absD d = x ⦄ := by
  unfold decode32_static
  step as ⟨ a, ha ⟩
  step as ⟨ b, hb ⟩
  have va : a.val = hw1.val := by rw [ha]; exact U16.cast_U32_val_eq hw1
  have vb : b.val = hw2.val := by rw [hb]; exact U16.cast_U32_val_eq hw2
  have la : a.val < 65536 := by scalar_tac
  have lb : b.val < 65536 := by scalar_tac
  step as ⟨ i, h_i, _ ⟩
  have v_i : i.val = b.val / 16 * 16 := by
    rw [h_i, and_val]; exact and_FFF0 _ (by omega)
  step with (both_spec _ _) as ⟨ b1, h_b1 ⟩
  by_cases c_dsb : b1 = true
  · rw [if_pos c_dsb]
    have p_dsb : hw1.val = 62399 ∧ hw2.val / 16 = 2292 := by
      rw [h_b1] at c_dsb
      bool_nat at c_dsb
      omega
    have hD : D32 hw1.val hw2.val pc.val = some (.plain) := by
      unfold LinkThumb.D32
      rw [if_pos p_dsb]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    rfl
  rw [if_neg c_dsb]
  have n_dsb : ¬ (hw1.val = 62399 ∧ hw2.val / 16 = 2292) := by
    rw [h_b1] at c_dsb
    bool_nat at c_dsb
    omega
  step as ⟨ x1, h_x1, _ ⟩
  have v_x1 : x1.val = a.val / 16 * 16 := by
    rw [h_x1, and_val]; exact and_FFF0 _ (by omega)
  step as ⟨ x2, h_x2, _ ⟩
  have v_x2 : x2.val = a.val / 16 * 16 := by
    rw [h_x2, and_val]; exact and_FFF0 _ (by omega)
  step with (either_spec _ _) as ⟨ b2, h_b2 ⟩
  step as ⟨ s12, h_s12, _ ⟩
  have v_s12 : s12.val = b.val / 4096 := by
    rw [h_s12]; exact Nat.shiftRight_eq_div_pow _ _
  step with (both_spec _ _) as ⟨ b3, h_b3 ⟩
  by_cases c_ldrsp : b3 = true
  · rw [if_pos c_ldrsp]
    have p_ldrsp : (hw1.val / 16 = 3981 ∨ hw1.val / 16 = 3973) ∧ hw2.val / 4096 = 13 := by
      rw [h_b3, h_b2] at c_ldrsp
      bool_nat at c_ldrsp
      omega
    have hD : D32 hw1.val hw2.val pc.val = some (.spDynamic) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_pos p_ldrsp]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    rfl
  rw [if_neg c_ldrsp]
  have n_ldrsp : ¬ ((hw1.val / 16 = 3981 ∨ hw1.val / 16 = 3973) ∧ hw2.val / 4096 = 13) := by
    rw [h_b3, h_b2] at c_ldrsp
    bool_nat at c_ldrsp
    omega
  step as ⟨ x4, h_x4, _ ⟩
  have v_x4 : x4.val = a.val / 16 * 16 := by
    rw [h_x4, and_val]; exact and_FFF0 _ (by omega)
  step as ⟨ x5, h_x5, _ ⟩
  have v_x5 : x5.val = b.val / 4096 * 4096 := by
    rw [h_x5, and_val]; exact and_F000 _ (by omega)
  step with (both_spec _ _) as ⟨ b4, h_b4 ⟩
  by_cases c_msr : b4 = true
  · rw [if_pos c_msr]
    have p_msr : hw1.val / 16 = 3896 ∧ hw2.val / 4096 = 8 := by
      rw [h_b4] at c_msr
      bool_nat at c_msr
      omega
    step as ⟨ y6, h_y6, _ ⟩
    step as ⟨ y7, h_y7 ⟩
    have hD : D32 hw1.val hw2.val pc.val = some (.plain) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_pos p_msr]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    rfl
  rw [if_neg c_msr]
  have n_msr : ¬ (hw1.val / 16 = 3896 ∧ hw2.val / 4096 = 8) := by
    rw [h_b4] at c_msr
    bool_nat at c_msr
    omega
  step as ⟨ x6, h_x6, _ ⟩
  have v_x6 : x6.val = b.val / 4096 * 4096 := by
    rw [h_x6, and_val]; exact and_F000 _ (by omega)
  step with (both_spec _ _) as ⟨ b5, h_b5 ⟩
  by_cases c_mrs : b5 = true
  · rw [if_pos c_mrs]
    have p_mrs : hw1.val = 62447 ∧ hw2.val / 4096 = 8 := by
      rw [h_b5] at c_mrs
      bool_nat at c_mrs
      omega
    step as ⟨ y7, h_y7, _ ⟩
    step as ⟨ y8, h_y8 ⟩
    have hD : D32 hw1.val hw2.val pc.val = some (.plain) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_pos p_mrs]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    rfl
  rw [if_neg c_mrs]
  have n_mrs : ¬ (hw1.val = 62447 ∧ hw2.val / 4096 = 8) := by
    rw [h_b5] at c_mrs
    bool_nat at c_mrs
    omega
  by_cases c_pushw : a = 59693#u32
  · rw [if_pos c_pushw]
    have p_pushw : hw1.val = 59693 := by
      have e : a.val = 59693 := congrArg UScalar.val c_pushw
      omega
    step with (ones16_spec b) as ⟨ y7, h_y7 ⟩
    rw [vb] at h_y7
    have b_y7 : y7.val ≤ 16 := by rw [h_y7]; exact LinkBits.ones_le 16 _
    step as ⟨ y8, h_y8 ⟩
    have hD : D32 hw1.val hw2.val pc.val = some (.spDown (4 * LinkBits.ones 16 hw2.val)) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_pos p_pushw]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    show AEff.spDown _ = AEff.spDown _; congr 1; scalar_tac
  rw [if_neg c_pushw]
  have n_pushw : ¬ (hw1.val = 59693) := by
    have e : a.val ≠ 59693 := fun e => c_pushw ((UScalar.eq_equiv _ _).2 e)
    omega
  by_cases c_popw : a = 59581#u32
  · rw [if_pos c_popw]
    have p_popw : hw1.val = 59581 := by
      have e : a.val = 59581 := congrArg UScalar.val c_popw
      omega
    step as ⟨ y7, h_y7, _ ⟩
    have v_y7 : y7.val = b.val / 32768 := by
      rw [h_y7]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ y8, h_y8, _ ⟩
    have v_y8 : y8.val = y7.val % 2 := by
      rw [h_y8, and_val]; exact and_1 _ (by omega)
    by_cases c_popwpc : y8 = 1#u32
    · rw [if_pos c_popwpc]
      have p_popwpc : hw2.val / 32768 % 2 = 1 := by
        have e : y8.val = 1 := congrArg UScalar.val c_popwpc
        omega
      have hD : D32 hw1.val hw2.val pc.val = some (.ret) := by
        unfold LinkThumb.D32
        rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_pos p_popw, if_pos p_popwpc]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
      rfl
    rw [if_neg c_popwpc]
    have n_popwpc : ¬ (hw2.val / 32768 % 2 = 1) := by
      have e : y8.val ≠ 1 := fun e => c_popwpc ((UScalar.eq_equiv _ _).2 e)
      omega
    step with (ones16_spec b) as ⟨ y9, h_y9 ⟩
    rw [vb] at h_y9
    have b_y9 : y9.val ≤ 16 := by rw [h_y9]; exact LinkBits.ones_le 16 _
    step as ⟨ y10, h_y10 ⟩
    have hD : D32 hw1.val hw2.val pc.val = some (.spUp (4 * LinkBits.ones 16 hw2.val)) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_pos p_popw, if_neg n_popwpc]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    show AEff.spUp _ = AEff.spUp _; congr 1; scalar_tac
  rw [if_neg c_popw]
  have n_popw : ¬ (hw1.val = 59581) := by
    have e : a.val ≠ 59581 := fun e => c_popw ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ vm1, h_vm1, _ ⟩
  have v_vm1 : vm1.val = a.val / 128 * 128 + a.val % 64 := by
    rw [h_vm1, and_val]; exact and_FFBF _ (by omega)
  by_cases c_vpush : vm1 = 60717#u32
  · rw [if_pos c_vpush]
    have p_vpush : hw1.val = 60717 ∨ hw1.val = 60781 := by
      have e : vm1.val = 60717 := congrArg UScalar.val c_vpush
      omega
    step as ⟨ y7, h_y7, _ ⟩
    have v_y7 : y7.val = b.val % 256 := by
      rw [h_y7, and_val]; exact and_FF _ (by omega)
    step as ⟨ y8, h_y8 ⟩
    have hD : D32 hw1.val hw2.val pc.val = some (.spDown (4 * (hw2.val % 256))) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_pos p_vpush]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    show AEff.spDown _ = AEff.spDown _; congr 1; scalar_tac
  rw [if_neg c_vpush]
  have n_vpush : ¬ (hw1.val = 60717 ∨ hw1.val = 60781) := by
    have e : vm1.val ≠ 60717 := fun e => c_vpush ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ vm2, h_vm2, _ ⟩
  have v_vm2 : vm2.val = a.val / 128 * 128 + a.val % 64 := by
    rw [h_vm2, and_val]; exact and_FFBF _ (by omega)
  by_cases c_vpop : vm2 = 60605#u32
  · rw [if_pos c_vpop]
    have p_vpop : hw1.val = 60605 ∨ hw1.val = 60669 := by
      have e : vm2.val = 60605 := congrArg UScalar.val c_vpop
      omega
    step as ⟨ y7, h_y7, _ ⟩
    have v_y7 : y7.val = b.val % 256 := by
      rw [h_y7, and_val]; exact and_FF _ (by omega)
    step as ⟨ y8, h_y8 ⟩
    have hD : D32 hw1.val hw2.val pc.val = some (.spUp (4 * (hw2.val % 256))) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_pos p_vpop]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    show AEff.spUp _ = AEff.spUp _; congr 1; scalar_tac
  rw [if_neg c_vpop]
  have n_vpop : ¬ (hw1.val = 60605 ∨ hw1.val = 60669) := by
    have e : vm2.val ≠ 60605 := fun e => c_vpop ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ s8, h_s8, _ ⟩
  have v_s8 : s8.val = b.val / 256 := by
    rw [h_s8]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ x8, h_x8, _ ⟩
  have v_x8 : x8.val = s8.val % 8 := by
    rw [h_x8, and_val]; exact and_7 _ (by omega)
  step with (both_spec _ _) as ⟨ b6, h_b6 ⟩
  by_cases c_strpre : b6 = true
  · rw [if_pos c_strpre]
    have p_strpre : hw1.val = 63565 ∧ hw2.val / 256 % 8 = 5 := by
      rw [h_b6] at c_strpre
      bool_nat at c_strpre
      omega
    step as ⟨ y9, h_y9, _ ⟩
    have v_y9 : y9.val = b.val % 256 := by
      rw [h_y9, and_val]; exact and_FF _ (by omega)
    have hD : D32 hw1.val hw2.val pc.val = some (.spDown (hw2.val % 256)) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_pos p_strpre]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    show AEff.spDown _ = AEff.spDown _; congr 1; scalar_tac
  rw [if_neg c_strpre]
  have n_strpre : ¬ (hw1.val = 63565 ∧ hw2.val / 256 % 8 = 5) := by
    rw [h_b6] at c_strpre
    bool_nat at c_strpre
    omega
  by_cases c_strd : a = 59757#u32
  · rw [if_pos c_strd]
    have p_strd : hw1.val = 59757 := by
      have e : a.val = 59757 := congrArg UScalar.val c_strd
      omega
    step as ⟨ y9, h_y9, _ ⟩
    have v_y9 : y9.val = b.val % 256 := by
      rw [h_y9, and_val]; exact and_FF _ (by omega)
    step as ⟨ y10, h_y10 ⟩
    have hD : D32 hw1.val hw2.val pc.val = some (.spDown (4 * (hw2.val % 256))) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_pos p_strd]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    show AEff.spDown _ = AEff.spDown _; congr 1; scalar_tac
  rw [if_neg c_strd]
  have n_strd : ¬ (hw1.val = 59757) := by
    have e : a.val ≠ 59757 := fun e => c_strd ((UScalar.eq_equiv _ _).2 e)
    omega
  step with (both_spec _ _) as ⟨ b7, h_b7 ⟩
  by_cases c_ldrpc : b7 = true
  · rw [if_pos c_ldrpc]
    have p_ldrpc : hw1.val = 63581 ∧ hw2.val / 4096 = 15 := by
      rw [h_b7] at c_ldrpc
      bool_nat at c_ldrpc
      omega
    have hD : D32 hw1.val hw2.val pc.val = some (.ret) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_pos p_ldrpc]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    rfl
  rw [if_neg c_ldrpc]
  have n_ldrpc : ¬ (hw1.val = 63581 ∧ hw2.val / 4096 = 15) := by
    rw [h_b7] at c_ldrpc
    bool_nat at c_ldrpc
    omega
  step as ⟨ d15, h_d15, _ ⟩
  have v_d15 : d15.val = b.val / 32768 := by
    rw [h_d15]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ dpb, h_dpb, _ ⟩
  have v_dpb : dpb.val = b.val / 32768 % 2 := by
    rw [h_dpb, and_val, v_d15]; exact and_1 _ (by omega)
  step with (either_spec _ _) as ⟨ e12a, h_e12a ⟩
  step with (either_spec _ _) as ⟨ e12b, h_e12b ⟩
  step as ⟨ x12, h_x12, _ ⟩
  have v_x12 : x12.val = s8.val % 16 := by
    rw [h_x12, and_val]; exact and_F _ (by omega)
  step with (both_spec _ _) as ⟨ e12d, h_e12d ⟩
  step with (both_spec _ _) as ⟨ e12c, h_e12c ⟩
  by_cases c_subc : e12c = true
  · rw [if_pos c_subc]
    have p_subc : hw2.val / 32768 % 2 = 0 ∧ (hw1.val = 61869 ∨ hw1.val = 61885 ∨ hw1.val = 62125) ∧ hw2.val / 256 % 16 = 13 := by
      rw [h_e12c, h_e12d, h_e12b, h_e12a] at c_subc
      bool_nat at c_subc
      omega
    step as ⟨ t1, h_t1, _ ⟩
    have v_t1 : t1.val = a.val / 1024 := by
      rw [h_t1]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ t2, h_t2, _ ⟩
    have v_t2 : t2.val = t1.val % 2 := by
      rw [h_t2, and_val]; exact and_1 _ (by omega)
    step as ⟨ imm3, h_imm3, _ ⟩
    have v_imm3 : imm3.val = s12.val % 8 := by
      rw [h_imm3, and_val]; exact and_7 _ (by omega)
    step as ⟨ imm8, h_imm8, _ ⟩
    have v_imm8 : imm8.val = b.val % 256 := by
      rw [h_imm8, and_val]; exact and_FF _ (by omega)
    step as ⟨ t3, h_t3, _ ⟩
    have v_t3 : t3.val = t2.val * 2048 := by
      rw [u32_size] at h_t3
      have x := Nat.shiftLeft_eq t2.val 11
      omega
    step as ⟨ t4, h_t4, _ ⟩
    have v_t4 : t4.val = imm3.val * 256 := by
      rw [u32_size] at h_t4
      have x := Nat.shiftLeft_eq imm3.val 8
      omega
    step as ⟨ t5, h_t5, _ ⟩
    step as ⟨ imm12, h_imm12, _ ⟩
    by_cases c_w : a = 62125#u32
    · rw [if_pos c_w]
      have hD : D32 hw1.val hw2.val pc.val = none := by
        unfold LinkThumb.D32
        rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_pos p_subc]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
    rw [if_neg c_w]
    step with (thumb_expand_imm_total imm12) as ⟨ imm121 ⟩
    have hD : D32 hw1.val hw2.val pc.val = none := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_pos p_subc]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
  rw [if_neg c_subc]
  have n_subc : ¬ (hw2.val / 32768 % 2 = 0 ∧ (hw1.val = 61869 ∨ hw1.val = 61885 ∨ hw1.val = 62125) ∧ hw2.val / 256 % 16 = 13) := by
    rw [h_e12c, h_e12d, h_e12b, h_e12a] at c_subc
    bool_nat at c_subc
    omega
  step with (either_spec _ _) as ⟨ e13a, h_e13a ⟩
  step with (either_spec _ _) as ⟨ e13b, h_e13b ⟩
  step as ⟨ x13, h_x13, _ ⟩
  have v_x13 : x13.val = s8.val % 16 := by
    rw [h_x13, and_val]; exact and_F _ (by omega)
  step with (both_spec _ _) as ⟨ e13d, h_e13d ⟩
  step with (both_spec _ _) as ⟨ e13c, h_e13c ⟩
  by_cases c_addc : e13c = true
  · rw [if_pos c_addc]
    have p_addc : hw2.val / 32768 % 2 = 0 ∧ (hw1.val = 61709 ∨ hw1.val = 61725 ∨ hw1.val = 61965) ∧ hw2.val / 256 % 16 = 13 := by
      rw [h_e13c, h_e13d, h_e13b, h_e13a] at c_addc
      bool_nat at c_addc
      omega
    step as ⟨ t1, h_t1, _ ⟩
    have v_t1 : t1.val = a.val / 1024 := by
      rw [h_t1]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ t2, h_t2, _ ⟩
    have v_t2 : t2.val = t1.val % 2 := by
      rw [h_t2, and_val]; exact and_1 _ (by omega)
    step as ⟨ imm3, h_imm3, _ ⟩
    have v_imm3 : imm3.val = s12.val % 8 := by
      rw [h_imm3, and_val]; exact and_7 _ (by omega)
    step as ⟨ imm8, h_imm8, _ ⟩
    have v_imm8 : imm8.val = b.val % 256 := by
      rw [h_imm8, and_val]; exact and_FF _ (by omega)
    step as ⟨ t3, h_t3, _ ⟩
    have v_t3 : t3.val = t2.val * 2048 := by
      rw [u32_size] at h_t3
      have x := Nat.shiftLeft_eq t2.val 11
      omega
    step as ⟨ t4, h_t4, _ ⟩
    have v_t4 : t4.val = imm3.val * 256 := by
      rw [u32_size] at h_t4
      have x := Nat.shiftLeft_eq imm3.val 8
      omega
    step as ⟨ t5, h_t5, _ ⟩
    step as ⟨ imm12, h_imm12, _ ⟩
    by_cases c_w : a = 61965#u32
    · rw [if_pos c_w]
      have hD : D32 hw1.val hw2.val pc.val = none := by
        unfold LinkThumb.D32
        rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_pos p_addc]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
    rw [if_neg c_w]
    step with (thumb_expand_imm_total imm12) as ⟨ imm121 ⟩
    have hD : D32 hw1.val hw2.val pc.val = none := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_pos p_addc]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
  rw [if_neg c_addc]
  have n_addc : ¬ (hw2.val / 32768 % 2 = 0 ∧ (hw1.val = 61709 ∨ hw1.val = 61725 ∨ hw1.val = 61965) ∧ hw2.val / 256 % 16 = 13) := by
    rw [h_e13c, h_e13d, h_e13b, h_e13a] at c_addc
    bool_nat at c_addc
    omega
  step with (either_spec _ _) as ⟨ f1, h_f1 ⟩
  step with (either_spec _ _) as ⟨ f2, h_f2 ⟩
  step with (either_spec _ _) as ⟨ f3, h_f3 ⟩
  step as ⟨ x14, h_x14, _ ⟩
  have v_x14 : x14.val = s8.val % 16 := by
    rw [h_x14, and_val]; exact and_F _ (by omega)
  step with (both_spec _ _) as ⟨ f4, h_f4 ⟩
  by_cases c_spreg : f4 = true
  · rw [if_pos c_spreg]
    have p_spreg : (hw1.val = 60333 ∨ hw1.val = 60349 ∨ hw1.val = 60173 ∨ hw1.val = 60189) ∧ hw2.val / 256 % 16 = 13 := by
      rw [h_f4, h_f3, h_f2, h_f1] at c_spreg
      bool_nat at c_spreg
      omega
    have hD : D32 hw1.val hw2.val pc.val = some (.spDynamic) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_neg n_addc, if_pos p_spreg]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    rfl
  rw [if_neg c_spreg]
  have n_spreg : ¬ ((hw1.val = 60333 ∨ hw1.val = 60349 ∨ hw1.val = 60173 ∨ hw1.val = 60189) ∧ hw2.val / 256 % 16 = 13) := by
    rw [h_f4, h_f3, h_f2, h_f1] at c_spreg
    bool_nat at c_spreg
    omega
  step as ⟨ x15a, h_x15a, _ ⟩
  have v_x15a : x15a.val = a.val / 2048 * 2048 + a.val / 32 % 32 * 32 + a.val % 16 := by
    rw [h_x15a, and_val]; exact and_FBEF _ (by omega)
  step as ⟨ x15b, h_x15b, _ ⟩
  have v_x15b : x15b.val = a.val / 2048 * 2048 + a.val / 16 % 64 * 16 := by
    rw [h_x15b, and_val]; exact and_FBF0 _ (by omega)
  step with (either_spec _ _) as ⟨ g1, h_g1 ⟩
  step as ⟨ x15c, h_x15c, _ ⟩
  have v_x15c : x15c.val = s8.val % 16 := by
    rw [h_x15c, and_val]; exact and_F _ (by omega)
  step with (both_spec _ _) as ⟨ g2d, h_g2d ⟩
  step with (both_spec _ _) as ⟨ g2, h_g2 ⟩
  by_cases c_movsp : g2 = true
  · rw [if_pos c_movsp]
    have p_movsp : hw2.val / 32768 % 2 = 0 ∧ ((hw1.val / 2048 = 30 ∧ hw1.val / 32 % 32 = 2 ∧ hw1.val % 16 = 15) ∨ (hw1.val / 2048 = 30 ∧ hw1.val / 16 % 64 = 36)) ∧ hw2.val / 256 % 16 = 13 := by
      rw [h_g2, h_g2d, h_g1] at c_movsp
      bool_nat at c_movsp
      omega
    have hD : D32 hw1.val hw2.val pc.val = some (.spDynamic) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_pos p_movsp]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    rfl
  rw [if_neg c_movsp]
  have n_movsp : ¬ (hw2.val / 32768 % 2 = 0 ∧ ((hw1.val / 2048 = 30 ∧ hw1.val / 32 % 32 = 2 ∧ hw1.val % 16 = 15) ∨ (hw1.val / 2048 = 30 ∧ hw1.val / 16 % 64 = 36)) ∧ hw2.val / 256 % 16 = 13) := by
    rw [h_g2, h_g2d, h_g1] at c_movsp
    bool_nat at c_movsp
    omega
  step as ⟨ x16a, h_x16a, _ ⟩
  have v_x16a : x16a.val = a.val / 2048 * 2048 + a.val / 16 % 64 * 16 := by
    rw [h_x16a, and_val]; exact and_FBF0 _ (by omega)
  step as ⟨ x16b, h_x16b, _ ⟩
  have v_x16b : x16b.val = a.val / 2048 * 2048 + a.val / 16 % 64 * 16 := by
    rw [h_x16b, and_val]; exact and_FBF0 _ (by omega)
  step with (either_spec _ _) as ⟨ g3d, h_g3d ⟩
  step with (both_spec _ _) as ⟨ g3, h_g3 ⟩
  by_cases c_movw : g3 = true
  · rw [if_pos c_movw]
    have p_movw : hw2.val / 32768 % 2 = 0 ∧ hw1.val / 2048 = 30 ∧ (hw1.val / 16 % 64 = 36 ∨ hw1.val / 16 % 64 = 44) := by
      rw [h_g3, h_g3d] at c_movw
      bool_nat at c_movw
      omega
    have hD : D32 hw1.val hw2.val pc.val = some (.plain) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_pos p_movw]
    step as ⟨ imm4, h_imm4, _ ⟩
    step as ⟨ u1, h_u1, _ ⟩
    step as ⟨ u2, h_u2, _ ⟩
    step as ⟨ imm3, h_imm3, _ ⟩
    step as ⟨ u3, h_u3, _ ⟩
    step as ⟨ rd, h_rd ⟩
    step as ⟨ imm8, h_imm8, _ ⟩
    step as ⟨ u4, h_u4, _ ⟩
    step as ⟨ u5, h_u5, _ ⟩
    step as ⟨ u6, h_u6, _ ⟩
    step as ⟨ u7, h_u7, _ ⟩
    step as ⟨ u8, h_u8, _ ⟩
    step as ⟨ imm16, h_imm16, _ ⟩
    step as ⟨ u9, h_u9, _ ⟩
    by_cases c_t : u9 = 62144#u32
    · rw [if_pos c_t]
      step as ⟨ imm161, h_imm161, _ ⟩
      intro x hx
      rw [hD] at hx
      cases hx
      rfl
    rw [if_neg c_t]
    simp only [WP.spec_ok]
    intro x hx
    rw [hD] at hx
    cases hx
    rfl
  rw [if_neg c_movw]
  have n_movw : ¬ (hw2.val / 32768 % 2 = 0 ∧ hw1.val / 2048 = 30 ∧ (hw1.val / 16 % 64 = 36 ∨ hw1.val / 16 % 64 = 44)) := by
    rw [h_g3, h_g3d] at c_movw
    bool_nat at c_movw
    omega
  step with (either_spec _ _) as ⟨ g4, h_g4 ⟩
  by_cases c_lit : g4 = true
  · rw [if_pos c_lit]
    have p_lit : hw1.val = 63711 ∨ hw1.val = 63583 := by
      rw [h_g4] at c_lit
      bool_nat at c_lit
      omega
    have hD : D32 hw1.val hw2.val pc.val = some (.plain) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_movw, if_pos p_lit]
    step as ⟨ v1, h_v1, _ ⟩
    step as ⟨ rt, h_rt ⟩
    step as ⟨ v2, h_v2 ⟩
    step as ⟨ v3, h_v3 ⟩
    step as ⟨ base, h_base, _ ⟩
    have b_base : base.val ≤ v2.val := by rw [h_base, and_val]; exact Nat.and_le_left
    by_cases c_u : a = 63711#u32
    · rw [if_pos c_u]
      step as ⟨ v4, h_v4, _ ⟩
      have v_v4 : v4.val = b.val % 4096 := by
        rw [h_v4, and_val]; exact and_FFF _ (by omega)
      step as ⟨ addr, h_addr ⟩
      intro x hx
      rw [hD] at hx
      cases hx
      rfl
    rw [if_neg c_u]
    step as ⟨ v4, h_v4, _ ⟩
    have v_v4 : v4.val = b.val % 4096 := by
      rw [h_v4, and_val]; exact and_FFF _ (by omega)
    first | (step as ⟨ addr, h_addr ⟩) | (simp only [lift, bind_tc_ok, WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    rfl
  rw [if_neg c_lit]
  have n_lit : ¬ (hw1.val = 63711 ∨ hw1.val = 63583) := by
    rw [h_g4] at c_lit
    bool_nat at c_lit
    omega
  step as ⟨ x18a, h_x18a, _ ⟩
  have v_x18a : x18a.val = a.val / 8192 * 8192 + a.val / 1024 % 4 * 1024 := by
    rw [h_x18a, and_val]; exact and_EC00 _ (by omega)
  step as ⟨ x18b, h_x18b, _ ⟩
  have v_x18b : x18b.val = s8.val / 2 % 8 * 2 := by
    rw [h_x18b, and_val]; exact and_E _ (by omega)
  step with (both_spec _ _) as ⟨ g5, h_g5 ⟩
  by_cases c_vfp : g5 = true
  · rw [if_pos c_vfp]
    have p_vfp : (hw1.val / 8192 = 7 ∧ hw1.val / 1024 % 4 = 3) ∧ hw2.val / 512 % 8 = 5 := by
      rw [h_g5] at c_vfp
      bool_nat at c_vfp
      omega
    have hD : D32 hw1.val hw2.val pc.val = some (.plain) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_movw, if_neg n_lit, if_pos p_vfp]
    step as ⟨ w1, h_w1, _ ⟩
    by_cases c_v1 : w1 = 60416#u32
    · rw [if_pos c_v1]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
      rfl
    rw [if_neg c_v1]
    step as ⟨ w2a, h_w2a, _ ⟩
    step as ⟨ w2b, h_w2b, _ ⟩
    step with (both_spec _ _) as ⟨ w2, h_w2 ⟩
    by_cases c_v2 : w2 = true
    · rw [if_pos c_v2]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
      rfl
    rw [if_neg c_v2]
    step as ⟨ w3b, h_w3b, _ ⟩
    step with (both_spec _ _) as ⟨ w3, h_w3 ⟩
    by_cases c_v3 : w3 = true
    · rw [if_pos c_v3]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
      rfl
    rw [if_neg c_v3]
    step as ⟨ w4b, h_w4b, _ ⟩
    step with (both_spec _ _) as ⟨ w4, h_w4 ⟩
    by_cases c_v4 : w4 = true
    · rw [if_pos c_v4]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
      rfl
    rw [if_neg c_v4]
    step as ⟨ w5a, h_w5a, _ ⟩
    step as ⟨ w5b, h_w5b, _ ⟩
    step with (both_spec _ _) as ⟨ w5, h_w5 ⟩
    by_cases c_v5 : w5 = true
    · rw [if_pos c_v5]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
      rfl
    rw [if_neg c_v5]
    step as ⟨ w6a, h_w6a, _ ⟩
    step as ⟨ w6b, h_w6b, _ ⟩
    step with (both_spec _ _) as ⟨ w6, h_w6 ⟩
    by_cases c_v6 : w6 = true
    · rw [if_pos c_v6]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
      rfl
    rw [if_neg c_v6]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    rfl
  rw [if_neg c_vfp]
  have n_vfp : ¬ ((hw1.val / 8192 = 7 ∧ hw1.val / 1024 % 4 = 3) ∧ hw2.val / 512 % 8 = 5) := by
    rw [h_g5] at c_vfp
    bool_nat at c_vfp
    omega
  step as ⟨ x19a, h_x19a, _ ⟩
  have v_x19a : x19a.val = a.val / 16 * 16 := by
    rw [h_x19a, and_val]; exact and_FFF0 _ (by omega)
  step as ⟨ x19b, h_x19b, _ ⟩
  have v_x19b : x19b.val = b.val / 32 * 32 := by
    rw [h_x19b, and_val]; exact and_FFE0 _ (by omega)
  step with (both_spec _ _) as ⟨ g6, h_g6 ⟩
  by_cases c_tbb : g6 = true
  · rw [if_pos c_tbb]
    have p_tbb : hw1.val / 16 = 3725 ∧ hw2.val / 32 = 1920 := by
      rw [h_g6] at c_tbb
      bool_nat at c_tbb
      omega
    have hD : D32 hw1.val hw2.val pc.val = some (.plain) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_movw, if_neg n_lit, if_neg n_vfp, if_pos p_tbb]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    rfl
  rw [if_neg c_tbb]
  have n_tbb : ¬ (hw1.val / 16 = 3725 ∧ hw2.val / 32 = 1920) := by
    rw [h_g6] at c_tbb
    bool_nat at c_tbb
    omega
  step as ⟨ x20a, h_x20a, _ ⟩
  have v_x20a : x20a.val = a.val / 2048 * 2048 := by
    rw [h_x20a, and_val]; exact and_F800 _ (by omega)
  step as ⟨ x20b, h_x20b, _ ⟩
  have v_x20b : x20b.val = b.val / 16384 * 16384 + b.val / 4096 % 2 * 4096 := by
    rw [h_x20b, and_val]; exact and_D000 _ (by omega)
  step as ⟨ x20c, h_x20c, _ ⟩
  have v_x20c : x20c.val = b.val / 16384 * 16384 + b.val / 4096 % 2 * 4096 := by
    rw [h_x20c, and_val]; exact and_D000 _ (by omega)
  step with (either_spec _ _) as ⟨ g7, h_g7 ⟩
  step with (both_spec _ _) as ⟨ g8, h_g8 ⟩
  by_cases c_bl : g8 = true
  · rw [if_pos c_bl]
    have p_bl : hw1.val / 2048 = 30 ∧ (hw2.val / 16384 = 3 ∨ hw2.val / 16384 = 2) ∧ hw2.val / 4096 % 2 = 1 := by
      rw [h_g8, h_g7] at c_bl
      bool_nat at c_bl
      omega
    step as ⟨ k1, h_k1, _ ⟩
    have v_k1 : k1.val = a.val / 1024 := by
      rw [h_k1]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ s, h_s, _ ⟩
    have v_s : s.val = k1.val % 2 := by
      rw [h_s, and_val]; exact and_1 _ (by omega)
    step as ⟨ k2, h_k2, _ ⟩
    have v_k2 : k2.val = b.val / 8192 := by
      rw [h_k2]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ j1, h_j1, _ ⟩
    have v_j1 : j1.val = k2.val % 2 := by
      rw [h_j1, and_val]; exact and_1 _ (by omega)
    step as ⟨ k3, h_k3, _ ⟩
    have v_k3 : k3.val = b.val / 2048 := by
      rw [h_k3]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ j2, h_j2, _ ⟩
    have v_j2 : j2.val = k3.val % 2 := by
      rw [h_j2, and_val]; exact and_1 _ (by omega)
    step as ⟨ i1x, h_i1x, _ ⟩
    step as ⟨ i1n, h_i1n ⟩
    step as ⟨ i1, h_i1, _ ⟩
    have v_i1 : i1.val = if j1.val = s.val then 1 else 0 := by
      have e1 : i1x = j1 ^^^ s := (UScalar.eq_equiv _ _).2 h_i1x
      rw [h_i1, h_i1n, e1]
      exact xnor1 j1 s (by omega) (by omega)
    have b_i1 : i1.val ≤ 1 := by rw [v_i1]; split <;> omega
    step as ⟨ i2x, h_i2x, _ ⟩
    step as ⟨ i2n, h_i2n ⟩
    step as ⟨ i2, h_i2, _ ⟩
    have v_i2 : i2.val = if j2.val = s.val then 1 else 0 := by
      have e1 : i2x = j2 ^^^ s := (UScalar.eq_equiv _ _).2 h_i2x
      rw [h_i2, h_i2n, e1]
      exact xnor1 j2 s (by omega) (by omega)
    have b_i2 : i2.val ≤ 1 := by rw [v_i2]; split <;> omega
    step as ⟨ imm10, h_imm10, _ ⟩
    have v_imm10 : imm10.val = a.val % 1024 := by
      rw [h_imm10, and_val]; exact and_3FF _ (by omega)
    step as ⟨ imm11, h_imm11, _ ⟩
    have v_imm11 : imm11.val = b.val % 2048 := by
      rw [h_imm11, and_val]; exact and_7FF _ (by omega)
    step as ⟨ r1, h_r1, _ ⟩
    have v_r1 : r1.val = s.val * 16777216 := by
      rw [u32_size] at h_r1
      have x := Nat.shiftLeft_eq s.val 24
      omega
    step as ⟨ r2, h_r2, _ ⟩
    have v_r2 : r2.val = i1.val * 8388608 := by
      rw [u32_size] at h_r2
      have x := Nat.shiftLeft_eq i1.val 23
      omega
    step as ⟨ r3, h_r3, _ ⟩
    have v_r3 : r3.val = r1.val + r2.val := by
      rw [h_r3, UScalar.val_or]; exact or_add _ _ 24 (by omega) (by omega)
    step as ⟨ r4, h_r4, _ ⟩
    have v_r4 : r4.val = i2.val * 4194304 := by
      rw [u32_size] at h_r4
      have x := Nat.shiftLeft_eq i2.val 22
      omega
    step as ⟨ r5, h_r5, _ ⟩
    have v_r5 : r5.val = r3.val + r4.val := by
      rw [h_r5, UScalar.val_or]; exact or_add _ _ 23 (by omega) (by omega)
    step as ⟨ r6, h_r6, _ ⟩
    have v_r6 : r6.val = imm10.val * 4096 := by
      rw [u32_size] at h_r6
      have x := Nat.shiftLeft_eq imm10.val 12
      omega
    step as ⟨ r7, h_r7, _ ⟩
    have v_r7 : r7.val = r5.val + r6.val := by
      rw [h_r7, UScalar.val_or]; exact or_add _ _ 22 (by omega) (by omega)
    step as ⟨ r8, h_r8, _ ⟩
    have v_r8 : r8.val = imm11.val * 2 := by
      rw [u32_size] at h_r8
      have x := Nat.shiftLeft_eq imm11.val 1
      omega
    step as ⟨ raw, h_raw, _ ⟩
    have v_raw : raw.val = r7.val + r8.val := by
      rw [h_raw, UScalar.val_or]; exact or_add _ _ 12 (by omega) (by omega)
    have e_raw : raw.val = LinkThumb.raw25 hw1.val hw2.val := by
      unfold LinkThumb.raw25
      rw [v_j1, v_k2, vb, v_s, v_k1, va] at v_i1
      rw [v_j2, v_k3, vb, v_s, v_k1, va] at v_i2
      rw [← v_i1, ← v_i2]
      omega
    have l_raw : raw.val < 33554432 := by rw [e_raw]; exact raw25_lt _ _
    step with (sign_extend_25 raw l_raw) as ⟨ imm, h_imm ⟩
    have b_imm : -16777216 ≤ imm.val ∧ imm.val < 16777216 := by
      clear * - h_imm l_raw
      rw [h_imm]; split <;> omega
    step as ⟨ q1, h_q1 ⟩
    have v_q1 : q1.val = pc.val := by rw [h_q1]; exact hcast_small pc (by omega)
    step as ⟨ q2, h_q2 ⟩
    step as ⟨ q3, h_q3 ⟩
    step as ⟨ target, h_target ⟩
    have v_target : target.val = (q3.val % 4294967296).toNat := by rw [h_target]; exact to_u32 q3
    have key : q3.val = (pc.val : Int) + 4 + LinkThumb.off25 hw1.val hw2.val := by
      unfold LinkThumb.off25
      rw [← e_raw]
      rw [h_imm] at h_q3
      clear * - h_q3 h_q2 v_q1 hpc l_raw
      split at h_q3 <;> split <;> scalar_tac
    step as ⟨ x20d, h_x20d, _ ⟩
    have v_x20d : x20d.val = b.val / 16384 % 2 * 16384 := by
      rw [h_x20d, and_val]; exact and_4000 _ (by omega)
    by_cases c_link : x20d = 16384#u32
    · rw [if_pos c_link]
      have p_link : hw2.val / 16384 % 2 = 1 := by
        have e : x20d.val = 16384 := congrArg UScalar.val c_link
        omega
      have hD : D32 hw1.val hw2.val pc.val = some (.call (LinkThumb.tgt pc.val (LinkThumb.off25 hw1.val hw2.val))) := by
        unfold LinkThumb.D32
        rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_movw, if_neg n_lit, if_neg n_vfp, if_neg n_tbb, if_pos p_bl, if_pos p_link]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
      show AEff.call _ = AEff.call _; congr 1
      rw [v_target, key]; first | rfl | skip
    rw [if_neg c_link]
    have n_link : ¬ (hw2.val / 16384 % 2 = 1) := by
      have e : x20d.val ≠ 16384 := fun e => c_link ((UScalar.eq_equiv _ _).2 e)
      omega
    have hD : D32 hw1.val hw2.val pc.val = some (.branch (LinkThumb.tgt pc.val (LinkThumb.off25 hw1.val hw2.val))) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_movw, if_neg n_lit, if_neg n_vfp, if_neg n_tbb, if_pos p_bl, if_neg n_link]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    show AEff.branch _ = AEff.branch _; congr 1
    rw [v_target, key]; first | rfl | skip
  rw [if_neg c_bl]
  have n_bl : ¬ (hw1.val / 2048 = 30 ∧ (hw2.val / 16384 = 3 ∨ hw2.val / 16384 = 2) ∧ hw2.val / 4096 % 2 = 1) := by
    rw [h_g8, h_g7] at c_bl
    bool_nat at c_bl
    omega
  step as ⟨ x21a, h_x21a, _ ⟩
  have v_x21a : x21a.val = a.val / 2048 * 2048 := by
    rw [h_x21a, and_val]; exact and_F800 _ (by omega)
  step as ⟨ x21b, h_x21b, _ ⟩
  have v_x21b : x21b.val = b.val / 16384 * 16384 + b.val / 4096 % 2 * 4096 := by
    rw [h_x21b, and_val]; exact and_D000 _ (by omega)
  step with (both_spec _ _) as ⟨ g9, h_g9 ⟩
  by_cases c_bcw : g9 = true
  · rw [if_pos c_bcw]
    have p_bcw : hw1.val / 2048 = 30 ∧ hw2.val / 16384 = 2 ∧ hw2.val / 4096 % 2 = 0 := by
      rw [h_g9] at c_bcw
      bool_nat at c_bcw
      omega
    step as ⟨ k1, h_k1, _ ⟩
    have v_k1 : k1.val = a.val / 1024 := by
      rw [h_k1]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ s, h_s, _ ⟩
    have v_s : s.val = k1.val % 2 := by
      rw [h_s, and_val]; exact and_1 _ (by omega)
    step as ⟨ k4, h_k4, _ ⟩
    have v_k4 : k4.val = a.val / 64 := by
      rw [h_k4]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ cond, h_cond, _ ⟩
    have v_cond : cond.val = k4.val % 16 := by
      rw [h_cond, and_val]; exact and_F _ (by omega)
    step as ⟨ k5, h_k5, _ ⟩
    have v_k5 : k5.val = cond.val / 2 % 8 * 2 := by
      rw [h_k5, and_val]; exact and_E _ (by omega)
    by_cases c_never : k5 = 14#u32
    · rw [if_pos c_never]
      have p_never : hw1.val / 128 % 8 = 7 := by
        have e : k5.val = 14 := congrArg UScalar.val c_never
        omega
      have hD : D32 hw1.val hw2.val pc.val = some (.plain) := by
        unfold LinkThumb.D32
        rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_movw, if_neg n_lit, if_neg n_vfp, if_neg n_tbb, if_neg n_bl, if_pos p_bcw, if_pos p_never]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
      rfl
    rw [if_neg c_never]
    have n_never : ¬ (hw1.val / 128 % 8 = 7) := by
      have e : k5.val ≠ 14 := fun e => c_never ((UScalar.eq_equiv _ _).2 e)
      omega
    step as ⟨ k2, h_k2, _ ⟩
    have v_k2 : k2.val = b.val / 8192 := by
      rw [h_k2]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ j1, h_j1, _ ⟩
    have v_j1 : j1.val = k2.val % 2 := by
      rw [h_j1, and_val]; exact and_1 _ (by omega)
    step as ⟨ k3, h_k3, _ ⟩
    have v_k3 : k3.val = b.val / 2048 := by
      rw [h_k3]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ j2, h_j2, _ ⟩
    have v_j2 : j2.val = k3.val % 2 := by
      rw [h_j2, and_val]; exact and_1 _ (by omega)
    step as ⟨ imm6, h_imm6, _ ⟩
    have v_imm6 : imm6.val = a.val % 64 := by
      rw [h_imm6, and_val]; exact and_3F _ (by omega)
    step as ⟨ imm11, h_imm11, _ ⟩
    have v_imm11 : imm11.val = b.val % 2048 := by
      rw [h_imm11, and_val]; exact and_7FF _ (by omega)
    step as ⟨ r1, h_r1, _ ⟩
    have v_r1 : r1.val = s.val * 1048576 := by
      rw [u32_size] at h_r1
      have x := Nat.shiftLeft_eq s.val 20
      omega
    step as ⟨ r2, h_r2, _ ⟩
    have v_r2 : r2.val = j2.val * 524288 := by
      rw [u32_size] at h_r2
      have x := Nat.shiftLeft_eq j2.val 19
      omega
    step as ⟨ r3, h_r3, _ ⟩
    have b_r3 : r3.val < 2097152 := by
      rw [h_r3, UScalar.val_or]; exact Nat.or_lt_two_pow (n := 21) (by omega) (by omega)
    step as ⟨ r4, h_r4, _ ⟩
    have v_r4 : r4.val = j1.val * 262144 := by
      rw [u32_size] at h_r4
      have x := Nat.shiftLeft_eq j1.val 18
      omega
    step as ⟨ r5, h_r5, _ ⟩
    have b_r5 : r5.val < 2097152 := by
      rw [h_r5, UScalar.val_or]; exact Nat.or_lt_two_pow (n := 21) (by omega) (by omega)
    step as ⟨ r6, h_r6, _ ⟩
    have v_r6 : r6.val = imm6.val * 4096 := by
      rw [u32_size] at h_r6
      have x := Nat.shiftLeft_eq imm6.val 12
      omega
    step as ⟨ r7, h_r7, _ ⟩
    have b_r7 : r7.val < 2097152 := by
      rw [h_r7, UScalar.val_or]; exact Nat.or_lt_two_pow (n := 21) (by omega) (by omega)
    step as ⟨ r8, h_r8, _ ⟩
    have v_r8 : r8.val = imm11.val * 2 := by
      rw [u32_size] at h_r8
      have x := Nat.shiftLeft_eq imm11.val 1
      omega
    step as ⟨ raw, h_raw, _ ⟩
    have b_raw : raw.val < 2097152 := by
      rw [h_raw, UScalar.val_or]; exact Nat.or_lt_two_pow (n := 21) (by omega) (by omega)
    step with (sign_extend_21 raw (by omega)) as ⟨ imm, h_imm ⟩
    have b_imm : -1048576 ≤ imm.val ∧ imm.val < 1048576 := by rw [h_imm]; split <;> omega
    step as ⟨ q1, h_q1 ⟩
    have v_q1 : q1.val = pc.val := by rw [h_q1]; exact hcast_small pc (by omega)
    step as ⟨ q2, h_q2 ⟩
    step as ⟨ q3, h_q3 ⟩
    step as ⟨ q4, h_q4 ⟩
    have hD : D32 hw1.val hw2.val pc.val = none := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_movw, if_neg n_lit, if_neg n_vfp, if_neg n_tbb, if_neg n_bl, if_pos p_bcw, if_neg n_never]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
  rw [if_neg c_bcw]
  have n_bcw : ¬ (hw1.val / 2048 = 30 ∧ hw2.val / 16384 = 2 ∧ hw2.val / 4096 % 2 = 0) := by
    rw [h_g9] at c_bcw
    bool_nat at c_bcw
    omega
  have hD : D32 hw1.val hw2.val pc.val = some (.plain) := by
    unfold LinkThumb.D32
    rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_movw, if_neg n_lit, if_neg n_vfp, if_neg n_tbb, if_neg n_bl, if_neg n_bcw]
  (try simp only [WP.spec_ok])
  intro x hx
  rw [hD] at hx
  cases hx
  rfl

end rsk_link
