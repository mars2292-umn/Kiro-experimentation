/-
  The Link_Checker's decoder, 2026-11-27: the two functions the 32-bit
  decoder calls that `Tools/LinkThumbProofs.lean` did not state a value
  for. `thumb_expand_imm`, as compiled, computes `LinkThumb.expand12` (the
  modified immediate of `sub`/`add sp, sp, #const`), and `sp_unread`, as
  compiled, decides `LinkThumb.Unread` (the classes of 32-bit instructions
  that can write register 13). Handwritten, line by line over
  `Tools/Link.lean`.
-/
import Aeneas
import Tools.Link
import Tools.LinkBits
import Tools.LinkThumbDefs
import Tools.LinkThumbProofs
open Aeneas Aeneas.Std Result

set_option maxHeartbeats 1000000

namespace rsk_link
open thumb
open LinkThumb (AEff expand12 Unread)

theorem and_3 (x : Nat) (hx : x < 65536) : x &&& 3 = x % 4 := by mask16 hx
theorem and_FBFF (x : Nat) (hx : x < 65536) : x &&& 64511 = x / 2048 * 2048 + x % 1024 := by mask16 hx

theorem shl_val (x : U32) (k : Nat) (r : U32) (h : r.val = x.val <<< k % U32.size)
    (hb : x.val * 2 ^ k < 4294967296) : r.val = x.val * 2 ^ k := by
  rw [u32_size] at h
  rw [Nat.shiftLeft_eq] at h
  omega

/-- **The modified immediate.** `thumb_expand_imm` as compiled returns for
every 12-bit argument and computes `expand12`. -/
theorem thumb_expand_imm_spec (x : U32) (hx : x.val < 4096) :
    thumb_expand_imm x ⦃ r => r.val = expand12 x.val ⦄ := by
  unfold thumb_expand_imm
  step as ⟨ i, hi, _ ⟩
  have vi : i.val = x.val / 1024 := by rw [hi]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ a, ha, _ ⟩
  have va : a.val = x.val / 1024 % 4 := by rw [ha, and_val, vi]; exact and_3 _ (by omega)
  step as ⟨ i1, hi1, _ ⟩
  have vi1 : i1.val = x.val / 256 := by rw [hi1]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ b, hb, _ ⟩
  have vb : b.val = x.val / 256 % 4 := by rw [hb, and_val, vi1]; exact and_3 _ (by omega)
  step as ⟨ imm8, him, _ ⟩
  have v8 : imm8.val = x.val % 256 := by rw [him, and_val]; exact and_FF _ (by omega)
  by_cases ca : a = 0#u32
  · rw [if_pos ca]
    have ea : x.val / 1024 % 4 = 0 := by
      have e : a.val = 0 := congrArg UScalar.val ca
      omega
    split
    · have eb : x.val / 256 % 4 = 0 := vb.symm.trans rfl
      simp only [WP.spec_ok]
      unfold LinkThumb.expand12
      rw [if_pos ea, if_pos eb, v8]
    · have eb : x.val / 256 % 4 = 1 := vb.symm.trans rfl
      step as ⟨ i2, hi2, _ ⟩
      have vi2 : i2.val = imm8.val * 2 ^ 16 := shl_val imm8 16 i2 hi2 (by omega)
      unfold LinkThumb.expand12
      rw [if_pos ea, if_neg (by omega), if_pos eb, UScalar.val_or, vi2]
      rw [or_add _ _ 16 (by omega) (by omega)]
      omega
    · have eb : x.val / 256 % 4 = 2 := vb.symm.trans rfl
      step as ⟨ i2, hi2, _ ⟩
      have vi2 : i2.val = imm8.val * 2 ^ 24 := shl_val imm8 24 i2 hi2 (by omega)
      step as ⟨ i3, hi3, _ ⟩
      have vi3 : i3.val = imm8.val * 2 ^ 8 := shl_val imm8 8 i3 hi3 (by omega)
      unfold LinkThumb.expand12
      rw [if_pos ea, if_neg (by omega), if_neg (by omega), if_pos eb, UScalar.val_or, vi2, vi3]
      rw [or_add _ _ 24 (by omega) (by omega)]
      omega
    · rename_i h0 h1 h2
      have eb : x.val / 256 % 4 = 3 := by
        have e0 : b.val ≠ 0 := fun e => h0 ((UScalar.eq_equiv _ _).2 e)
        have e1 : b.val ≠ 1 := fun e => h1 ((UScalar.eq_equiv _ _).2 e)
        have e2 : b.val ≠ 2 := fun e => h2 ((UScalar.eq_equiv _ _).2 e)
        omega
      step as ⟨ i2, hi2, _ ⟩
      have vi2 : i2.val = imm8.val * 2 ^ 24 := shl_val imm8 24 i2 hi2 (by omega)
      step as ⟨ i3, hi3, _ ⟩
      have vi3 : i3.val = imm8.val * 2 ^ 16 := shl_val imm8 16 i3 hi3 (by omega)
      step as ⟨ i4, hi4, _ ⟩
      have vi4 : i4.val = i2.val + i3.val := by
        rw [hi4, UScalar.val_or]; exact or_add _ _ 24 (by omega) (by omega)
      step as ⟨ i5, hi5, _ ⟩
      have vi5 : i5.val = imm8.val * 2 ^ 8 := shl_val imm8 8 i5 hi5 (by omega)
      step as ⟨ i6, hi6, _ ⟩
      have vi6 : i6.val = i4.val + i5.val := by
        rw [hi6, UScalar.val_or]; exact or_add _ _ 16 (by omega) (by omega)
      unfold LinkThumb.expand12
      rw [if_pos ea, if_neg (by omega), if_neg (by omega), if_neg (by omega), UScalar.val_or]
      rw [or_add _ _ 8 (by omega) (by omega)]
      omega
  · rw [if_neg ca]
    have ea : ¬ (x.val / 1024 % 4 = 0) := by
      have e : a.val ≠ 0 := fun e => ca ((UScalar.eq_equiv _ _).2 e)
      omega
    step as ⟨ i2, hi2, _ ⟩
    have vi2 : i2.val = x.val % 128 := by rw [hi2, and_val]; exact and_7F _ (by omega)
    step as ⟨ un, hun, _ ⟩
    have vun : un.val = 128 + x.val % 128 := by
      rw [hun, UScalar.val_or, vi2]
      have e : (128#u32).val = 128 := rfl
      rw [e, or_add 128 (x.val % 128) 7 (by omega) (by omega)]
    step as ⟨ i3, hi3, _ ⟩
    have vi3 : i3.val = x.val / 128 := by rw [hi3]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ rot, hrot, _ ⟩
    have vrot : rot.val = x.val / 128 % 32 := by rw [hrot, and_val, vi3]; exact and_1F _ (by omega)
    step as ⟨ i4, hi4 ⟩
    have vi4 : i4.val = 32 - x.val / 128 % 32 := by scalar_tac
    have b4 : i4.val ≤ 24 := by omega
    step as ⟨ r, hr, _ ⟩
    have p24 : 2 ^ i4.val ≤ 2 ^ 24 := Nat.pow_le_pow_right (by omega) b4
    have vr : r.val = un.val * 2 ^ i4.val := by
      apply shl_val un i4.val r hr
      calc un.val * 2 ^ i4.val ≤ 255 * 2 ^ 24 := Nat.mul_le_mul (by omega) p24
        _ < 4294967296 := by norm_num
    unfold LinkThumb.expand12
    rw [if_neg ea, vr, vun, vi4]

theorem eq_n (x : U32) (n : Nat) (h : n < 4294967296 := by norm_num) :
    x = UScalar.ofNat n (by scalar_tac) ↔ x.val = n := by
  constructor
  · intro e; rw [e]; rfl
  · intro e; exact (UScalar.eq_equiv _ _).2 e

/-- A test the decoder computed with `both` and `either`, as a statement
about numbers. -/
macro "unread_nat" "at" h:ident : tactic =>
  `(tactic| simp only [Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq, UScalar.lt_equiv,
      UScalar.eq_equiv, UScalar.ofNatCore_val_eq, Bool.decide_eq_true] at $h:ident)

/-- The goal `r = decide (Unread a b)` as an equivalence of statements. -/
macro "unread_open" : tactic =>
  `(tactic| (apply Bool.eq_iff_iff.mpr; rw [decide_eq_true_eq]; unfold LinkThumb.Unread))

/-- A Boolean the decoder computed, against a statement about numbers. -/
macro "unread_close" : tactic =>
  `(tactic| (
      subst_vars
      simp only [Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq, UScalar.lt_equiv,
        UScalar.eq_equiv, UScalar.ofNatCore_val_eq, Bool.decide_eq_true]
      omega))

/-- **The unread writes of SP.** `sp_unread` as compiled returns for every
pair of halfwords and decides `Unread`. -/
theorem sp_unread_spec (a b : U32) (la : a.val < 65536) (lb : b.val < 65536) :
    sp_unread a b ⦃ r => r = decide (Unread a.val b.val) ⦄ := by
  unfold sp_unread
  step as ⟨ top, h_top, _ ⟩
  have v_top : top.val = a.val / 2048 := by rw [h_top]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ i, h_i, _ ⟩
  have v_i : i.val = a.val / 512 := by rw [h_i]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ cls, h_cls, _ ⟩
  have v_cls : cls.val = a.val / 512 % 4 := by rw [h_cls, and_val, v_i]; exact and_3 _ (by omega)
  step as ⟨ rn, h_rn, _ ⟩
  have v_rn : rn.val = a.val % 16 := by rw [h_rn, and_val]; exact and_F _ (by omega)
  step as ⟨ i1, h_i1, _ ⟩
  have v_i1 : i1.val = b.val / 256 := by rw [h_i1]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ rd, h_rd, _ ⟩
  have v_rd : rd.val = b.val / 256 % 16 := by rw [h_rd, and_val, v_i1]; exact and_F _ (by omega)
  step as ⟨ rt, h_rt, _ ⟩
  have v_rt : rt.val = b.val / 4096 := by rw [h_rt]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ i2, h_i2, _ ⟩
  have v_i2 : i2.val = a.val / 32 := by rw [h_i2]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ w, h_w, _ ⟩
  have v_w : w.val = a.val / 32 % 2 := by rw [h_w, and_val, v_i2]; exact and_1 _ (by omega)
  step as ⟨ i3, h_i3, _ ⟩
  have v_i3 : i3.val = a.val / 16 := by rw [h_i3]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ l, h_l, _ ⟩
  have v_l : l.val = a.val / 16 % 2 := by rw [h_l, and_val, v_i3]; exact and_1 _ (by omega)
  clear h_top h_i h_cls h_rn h_i1 h_rd h_rt h_i2 h_w h_i3 h_l
  by_cases c30 : top = 30#u32
  · rw [if_pos c30]
    have p30 : a.val / 2048 = 30 := by
      have e : top.val = 30 := congrArg UScalar.val c30
      omega
    step with (both_spec _ _) as ⟨ r, h_r ⟩
    unread_open
    rw [if_pos p30]
    unread_close
  rw [if_neg c30]
  have n30 : ¬ (a.val / 2048 = 30) := by
    have e : top.val ≠ 30 := fun e => c30 ((UScalar.eq_equiv _ _).2 e)
    omega
  step with (both_spec _ _) as ⟨ b1, h_b1 ⟩
  by_cases c_lsm : b1 = true
  · rw [if_pos c_lsm]
    have p_lsm : a.val / 2048 = 29 ∧ a.val / 512 % 4 = 0 := by
      rw [h_b1] at c_lsm
      unread_nat at c_lsm
      omega
    step as ⟨ i4, h_i4, _ ⟩
    have v_i4 : i4.val = a.val / 64 := by rw [h_i4]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ i5, h_i5, _ ⟩
    have v_i5 : i5.val = i4.val % 2 := by rw [h_i5, and_val]; exact and_1 _ (by omega)
    by_cases c_mult : i5 = 0#u32
    · rw [if_pos c_mult]
      have p_mult : a.val / 64 % 2 = 0 := by
        have e : i5.val = 0 := congrArg UScalar.val c_mult
        omega
      step with (both_spec _ _) as ⟨ b2, h_b2 ⟩
      step as ⟨ i6, h_i6, _ ⟩
      have v_i6 : i6.val = b.val / 8192 := by rw [h_i6]; exact Nat.shiftRight_eq_div_pow _ _
      step as ⟨ i7, h_i7, _ ⟩
      have v_i7 : i7.val = i6.val % 2 := by rw [h_i7, and_val]; exact and_1 _ (by omega)
      step with (both_spec _ _) as ⟨ b3, h_b3 ⟩
      step with (either_spec _ _) as ⟨ r, h_r ⟩
      unread_open
      rw [if_neg n30, if_pos p_lsm, if_pos p_mult]
      unread_close
    rw [if_neg c_mult]
    have n_mult : ¬ (a.val / 64 % 2 = 0) := by
      have e : i5.val ≠ 0 := fun e => c_mult ((UScalar.eq_equiv _ _).2 e)
      omega
    step as ⟨ i6, h_i6, _ ⟩
    have v_i6 : i6.val = a.val / 256 := by rw [h_i6]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ i7, h_i7, _ ⟩
    have v_i7 : i7.val = i6.val % 2 := by rw [h_i7, and_val]; exact and_1 _ (by omega)
    step with (both_spec _ _) as ⟨ b2, h_b2 ⟩
    by_cases c_excl : b2 = true
    · rw [if_pos c_excl]
      have p_excl : a.val / 256 % 2 = 0 ∧ a.val / 32 % 2 = 0 := by
        rw [h_b2] at c_excl
        unread_nat at c_excl
        omega
      step as ⟨ op, h_op, _ ⟩
      have v_op : op.val = i3.val % 16 := by rw [h_op, and_val]; exact and_F _ (by omega)
      step with (both_spec _ _) as ⟨ b3, h_b3 ⟩
      step as ⟨ i8, h_i8, _ ⟩
      have v_i8 : i8.val = b.val % 16 := by rw [h_i8, and_val]; exact and_F _ (by omega)
      step with (both_spec _ _) as ⟨ b4, h_b4 ⟩
      step with (either_spec _ _) as ⟨ b5, h_b5 ⟩
      step with (both_spec _ _) as ⟨ b6, h_b6 ⟩
      step with (either_spec _ _) as ⟨ r, h_r ⟩
      unread_open
      rw [if_neg n30, if_pos p_lsm, if_neg n_mult, if_pos p_excl]
      unread_close
    rw [if_neg c_excl]
    have n_excl : ¬ (a.val / 256 % 2 = 0 ∧ a.val / 32 % 2 = 0) := by
      rw [h_b2] at c_excl
      unread_nat at c_excl
      omega
    step with (both_spec _ _) as ⟨ b3, h_b3 ⟩
    step with (either_spec _ _) as ⟨ b4, h_b4 ⟩
    step with (both_spec _ _) as ⟨ b5, h_b5 ⟩
    step with (either_spec _ _) as ⟨ r, h_r ⟩
    unread_open
    rw [if_neg n30, if_pos p_lsm, if_neg n_mult, if_neg n_excl]
    unread_close
  rw [if_neg c_lsm]
  have n_lsm : ¬ (a.val / 2048 = 29 ∧ a.val / 512 % 4 = 0) := by
    rw [h_b1] at c_lsm
    unread_nat at c_lsm
    omega
  step with (both_spec _ _) as ⟨ b2, h_b2 ⟩
  by_cases c_dpr : b2 = true
  · rw [if_pos c_dpr]
    have p_dpr : a.val / 2048 = 29 ∧ a.val / 512 % 4 = 1 := by
      rw [h_b2] at c_dpr
      unread_nat at c_dpr
      omega
    simp only [WP.spec_ok, decide_eq_decide]
    unfold LinkThumb.Unread
    rw [if_neg n30, if_neg n_lsm, if_pos p_dpr]
    simp only [UScalar.eq_equiv, UScalar.ofNatCore_val_eq]
    omega
  rw [if_neg c_dpr]
  have n_dpr : ¬ (a.val / 2048 = 29 ∧ a.val / 512 % 4 = 1) := by
    rw [h_b2] at c_dpr
    unread_nat at c_dpr
    omega
  step with (both_spec _ _) as ⟨ b3, h_b3 ⟩
  by_cases c_ls1 : b3 = true
  · rw [if_pos c_ls1]
    have p_ls1 : a.val / 2048 = 31 ∧ a.val / 512 % 4 = 0 := by
      rw [h_b3] at c_ls1
      unread_nat at c_ls1
      omega
    step as ⟨ i4, h_i4, _ ⟩
    have v_i4 : i4.val = a.val / 128 := by rw [h_i4]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ i5, h_i5, _ ⟩
    have v_i5 : i5.val = i4.val % 2 := by rw [h_i5, and_val]; exact and_1 _ (by omega)
    step as ⟨ i6, h_i6, _ ⟩
    have v_i6 : i6.val = b.val / 2048 := by rw [h_i6]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ i7, h_i7, _ ⟩
    have v_i7 : i7.val = i6.val % 2 := by rw [h_i7, and_val]; exact and_1 _ (by omega)
    step with (both_spec _ _) as ⟨ b4, h_b4 ⟩
    step as ⟨ i8, h_i8, _ ⟩
    have v_i8 : i8.val = i1.val % 2 := by rw [h_i8, and_val]; exact and_1 _ (by omega)
    step with (both_spec _ _) as ⟨ wb, h_wb ⟩
    step with (both_spec _ _) as ⟨ b5, h_b5 ⟩
    step with (both_spec _ _) as ⟨ b6, h_b6 ⟩
    step with (either_spec _ _) as ⟨ r, h_r ⟩
    unread_open
    rw [if_neg n30, if_neg n_lsm, if_neg n_dpr, if_pos p_ls1]
    unread_close
  rw [if_neg c_ls1]
  have n_ls1 : ¬ (a.val / 2048 = 31 ∧ a.val / 512 % 4 = 0) := by
    rw [h_b3] at c_ls1
    unread_nat at c_ls1
    omega
  step with (both_spec _ _) as ⟨ b4, h_b4 ⟩
  by_cases c_mul : b4 = true
  · rw [if_pos c_mul]
    have p_mul : a.val / 2048 = 31 ∧ a.val / 512 % 4 = 1 := by
      rw [h_b4] at c_mul
      unread_nat at c_mul
      omega
    step as ⟨ i4, h_i4, _ ⟩
    have v_i4 : i4.val = a.val / 128 := by rw [h_i4]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ i5, h_i5, _ ⟩
    have v_i5 : i5.val = i4.val % 4 := by rw [h_i5, and_val]; exact and_3 _ (by omega)
    step with (both_spec _ _) as ⟨ b5, h_b5 ⟩
    step with (either_spec _ _) as ⟨ r, h_r ⟩
    unread_open
    rw [if_neg n30, if_neg n_lsm, if_neg n_dpr, if_neg n_ls1, if_pos p_mul]
    unread_close
  rw [if_neg c_mul]
  have n_mul : ¬ (a.val / 2048 = 31 ∧ a.val / 512 % 4 = 1) := by
    rw [h_b4] at c_mul
    unread_nat at c_mul
    omega
  by_cases c_cop : cls = 2#u32
  · rw [if_pos c_cop]
    have p_cop : a.val / 512 % 4 = 2 := by
      have e : cls.val = 2 := congrArg UScalar.val c_cop
      omega
    step as ⟨ i4, h_i4, _ ⟩
    have v_i4 : i4.val = a.val / 256 := by rw [h_i4]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ i5, h_i5, _ ⟩
    have v_i5 : i5.val = i4.val % 2 := by rw [h_i5, and_val]; exact and_1 _ (by omega)
    step as ⟨ i6, h_i6, _ ⟩
    have v_i6 : i6.val = a.val / 128 := by rw [h_i6]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ i7, h_i7, _ ⟩
    have v_i7 : i7.val = i6.val % 2 := by rw [h_i7, and_val]; exact and_1 _ (by omega)
    step with (both_spec _ _) as ⟨ b5, h_b5 ⟩
    step with (both_spec _ _) as ⟨ two, h_two ⟩
    step with (both_spec _ _) as ⟨ b6, h_b6 ⟩
    step with (either_spec _ _) as ⟨ b7, h_b7 ⟩
    step with (both_spec _ _) as ⟨ b8, h_b8 ⟩
    step with (either_spec _ _) as ⟨ r, h_r ⟩
    unread_open
    rw [if_neg n30, if_neg n_lsm, if_neg n_dpr, if_neg n_ls1, if_neg n_mul, if_pos p_cop]
    unread_close
  rw [if_neg c_cop]
  have n_cop : ¬ (a.val / 512 % 4 = 2) := by
    have e : cls.val ≠ 2 := fun e => c_cop ((UScalar.eq_equiv _ _).2 e)
    omega
  step as ⟨ i4, h_i4, _ ⟩
  have v_i4 : i4.val = a.val / 256 := by rw [h_i4]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ i5, h_i5, _ ⟩
  have v_i5 : i5.val = i4.val % 2 := by rw [h_i5, and_val]; exact and_1 _ (by omega)
  step as ⟨ i6, h_i6, _ ⟩
  have v_i6 : i6.val = b.val / 16 := by rw [h_i6]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ i7, h_i7, _ ⟩
  have v_i7 : i7.val = i6.val % 2 := by rw [h_i7, and_val]; exact and_1 _ (by omega)
  step with (both_spec _ _) as ⟨ b5, h_b5 ⟩
  step with (both_spec _ _) as ⟨ b6, h_b6 ⟩
  step with (both_spec _ _) as ⟨ r, h_r ⟩
  unread_open
  rw [if_neg n30, if_neg n_lsm, if_neg n_dpr, if_neg n_ls1, if_neg n_mul, if_neg n_cop]
  unread_close

end rsk_link
