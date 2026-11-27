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
  | .SpUnread => .spUnread
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

end rsk_link
