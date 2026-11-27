import Mathlib.Tactic

/-! # Masks and bit counts as arithmetic (2026-11-26)

The Link_Checker's Thumb decoder (`crates/rsk-link/src/thumb.rs`) tests an
instruction's halfwords with masks (`h & 0xFE00 == 0xB400`) and counts the
registers of a list with `count_ones`; the Armv7-M semantics of
`Sched/Arm.lean` reads the same halfwords with division and remainder.
This file has the bridge: a conjunction with a constant mask as a sum over
the mask's bits (`and_bitsum`), and the sum of the low bits of a number
(`ones`). Nothing here is about either tool. Handwritten. -/

namespace LinkBits

/-- The bits of `x` below `n` that `m` has, each at its weight. -/
def bitsum : Nat → Nat → Nat → Nat
  | 0, _, _ => 0
  | n + 1, x, m => bitsum n x m + (if m / 2 ^ n % 2 = 1 then (x / 2 ^ n % 2) * 2 ^ n else 0)

theorem bitsum_lt (n x m : Nat) : bitsum n x m < 2 ^ n := by
  induction n with
  | zero => simp [bitsum]
  | succ n ih =>
    simp only [bitsum]
    have h2 : x / 2 ^ n % 2 < 2 := Nat.mod_lt _ (by norm_num)
    have : (x / 2 ^ n % 2) * 2 ^ n ≤ 2 ^ n := by
      calc (x / 2 ^ n % 2) * 2 ^ n ≤ 1 * 2 ^ n := Nat.mul_le_mul_right _ (by omega)
        _ = 2 ^ n := by ring
    rw [pow_succ]
    split <;> omega

theorem bitsum_mod (n k x m : Nat) (hk : n ≤ k) : bitsum n (x % 2 ^ k) m = bitsum n x m := by
  induction n with
  | zero => simp [bitsum]
  | succ n ih =>
    simp only [bitsum]
    rw [ih (by omega)]
    have e : x % 2 ^ k / 2 ^ n % 2 = x / 2 ^ n % 2 := by
      have h := Nat.testBit_mod_two_pow x k n
      rw [Nat.testBit_eq_decide_div_mod_eq, Nat.testBit_eq_decide_div_mod_eq] at h
      have hn : n < k := by omega
      simp only [hn, decide_true, Bool.true_and, decide_eq_decide] at h
      generalize x % 2 ^ k / 2 ^ n = a at h ⊢
      generalize x / 2 ^ n = b at h ⊢
      omega
    rw [e]

/-- **A conjunction with a mask**, as arithmetic: for a number below
`2 ^ n`, the sum of its bits that the mask has. -/
theorem and_bitsum (n x m : Nat) (hx : x < 2 ^ n) : x &&& m = bitsum n x m := by
  induction n generalizing x with
  | zero =>
    have : x = 0 := by simpa using hx
    subst this
    simp [bitsum]
  | succ n ih =>
    simp only [bitsum]
    rw [← bitsum_mod n n x m le_rfl, ← ih (x % 2 ^ n) (Nat.mod_lt _ (by positivity))]
    apply Nat.eq_of_testBit_eq
    intro i
    have hq : x / 2 ^ n < 2 := by
      rw [Nat.div_lt_iff_lt_mul (by positivity)]
      rw [pow_succ] at hx
      omega
    have hlow : (x % 2 ^ n &&& m) < 2 ^ n :=
      lt_of_le_of_lt Nat.and_le_left (Nat.mod_lt _ (by positivity))
    by_cases hi : i < n
    · -- below `n`: the bit of the low part
      have e1 : ((x % 2 ^ n &&& m) +
          (if m / 2 ^ n % 2 = 1 then (x / 2 ^ n % 2) * 2 ^ n else 0)).testBit i =
          (x % 2 ^ n &&& m).testBit i := by
        split
        · rw [Nat.add_comm, Nat.mul_comm, Nat.testBit_two_pow_mul_add _ hlow]
          simp [hi]
        · simp
      rw [e1, Nat.testBit_and, Nat.testBit_and, Nat.testBit_mod_two_pow]
      simp [hi]
    · have hi' : n ≤ i := by omega
      have e1 : ((x % 2 ^ n &&& m) +
          (if m / 2 ^ n % 2 = 1 then (x / 2 ^ n % 2) * 2 ^ n else 0)).testBit i =
          (decide (m / 2 ^ n % 2 = 1) && (x / 2 ^ n % 2).testBit (i - n)) := by
        split
        · rename_i hm
          rw [Nat.add_comm, Nat.mul_comm, Nat.testBit_two_pow_mul_add _ hlow]
          simp [hi, hm]
        · rename_i hm
          rw [Nat.add_zero, Nat.testBit_lt_two_pow (lt_of_lt_of_le hlow (Nat.pow_le_pow_right (by norm_num) hi'))]
          simp [hm]
      rw [e1, Nat.testBit_and]
      by_cases hin : i = n
      · subst hin
        simp only [Nat.sub_self, Nat.testBit_zero, Nat.mod_mod]
        rw [Nat.testBit_eq_decide_div_mod_eq, Nat.testBit_eq_decide_div_mod_eq]
        exact Bool.and_comm _ _
      · have hxi : x.testBit i = false := by
          apply Nat.testBit_lt_two_pow
          exact lt_of_lt_of_le hx (Nat.pow_le_pow_right (by norm_num) (by omega))
        have hqi : (x / 2 ^ n % 2).testBit (i - n) = false := by
          apply Nat.testBit_lt_two_pow
          have : x / 2 ^ n % 2 < 2 := Nat.mod_lt _ (by norm_num)
          calc x / 2 ^ n % 2 < 2 ^ 1 := by simpa using this
            _ ≤ 2 ^ (i - n) := Nat.pow_le_pow_right (by norm_num) (by omega)
        rw [hxi, hqi]
        simp

/-- The number of the bits of `x` below `n` that are set. -/
def ones : Nat → Nat → Nat
  | 0, _ => 0
  | n + 1, x => ones n x + x / 2 ^ n % 2

/-- The registers of a list are as many as its bits: the count of the
indices below `n` whose bit is set. -/
theorem ones_filter (n x : Nat) :
    ones n x = ((List.range n).filter fun i => x / 2 ^ i % 2 = 1).length := by
  induction n with
  | zero => simp [ones]
  | succ n ih =>
    simp only [ones, List.range_succ, List.filter_append, List.length_append, ih]
    have h2 : x / 2 ^ n % 2 < 2 := Nat.mod_lt _ (by norm_num)
    by_cases h : x / 2 ^ n % 2 = 1
    · simp [h]
    · have : x / 2 ^ n % 2 = 0 := by omega
      simp [this]

theorem ones_le (n x : Nat) : ones n x ≤ n := by
  induction n with
  | zero => simp [ones]
  | succ n ih =>
    simp only [ones]
    have : x / 2 ^ n % 2 < 2 := Nat.mod_lt _ (by norm_num)
    omega

end LinkBits
