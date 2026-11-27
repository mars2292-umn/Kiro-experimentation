/-
  The Link_Checker's 32-bit Thumb decoder on the code as compiled:
  `decode32_spec`. Split from `Tools/LinkThumbProofs.lean` on 2026-11-27,
  when the decoder gained every encoding of `sub`/`add sp, sp, #const`,
  the pop of one register, and the refusal of the other writes of SP
  (`sp_unread`, `Tools/LinkThumbUnread.lean`). Handwritten: the proof
  follows the extracted function line by line.
-/
import Aeneas
import Tools.Link
import Tools.LinkBits
import Tools.LinkThumbDefs
import Tools.LinkThumbProofs
import Tools.LinkThumbUnread
open Aeneas Aeneas.Std Result

set_option maxHeartbeats 1000000

namespace rsk_link
open thumb
open LinkThumb (AEff D16 D32)

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
    step as ⟨ m7, h_m7, _ ⟩
    have v_m7 : m7.val = b.val / 256 := by
      rw [h_m7]; exact Nat.shiftRight_eq_div_pow _ _
    step as ⟨ m8, h_m8, _ ⟩
    have v_m8 : m8.val = m7.val % 16 := by
      rw [h_m8, and_val]; exact and_F _ (by omega)
    by_cases c_mrssp : m8 = 13#u32
    · rw [if_pos c_mrssp]
      have p_mrssp : hw2.val / 256 % 16 = 13 := by
        have e : m8.val = 13 := congrArg UScalar.val c_mrssp
        omega
      have hD : D32 hw1.val hw2.val pc.val = some (.spUnread) := by
        unfold LinkThumb.D32
        rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_pos p_mrs, if_pos p_mrssp]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
      rfl
    rw [if_neg c_mrssp]
    have n_mrssp : ¬ (hw2.val / 256 % 16 = 13) := by
      have e : m8.val ≠ 13 := fun e => c_mrssp ((UScalar.eq_equiv _ _).2 e)
      omega
    step as ⟨ y7, h_y7, _ ⟩
    step as ⟨ y8, h_y8 ⟩
    have hD : D32 hw1.val hw2.val pc.val = some (.plain) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_pos p_mrs, if_neg n_mrssp]
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
  step as ⟨ x11, h_x11, _ ⟩
  have v_x11 : x11.val = s8.val % 16 := by
    rw [h_x11, and_val]; exact and_F _ (by omega)
  step with (both_spec _ _) as ⟨ b8, h_b8 ⟩
  by_cases c_pop1 : b8 = true
  · rw [if_pos c_pop1]
    have p_pop1 : hw1.val = 63581 ∧ hw2.val / 256 % 16 = 11 := by
      rw [h_b8] at c_pop1
      unread_nat at c_pop1
      omega
    step as ⟨ y9, h_y9, _ ⟩
    have v_y9 : y9.val = b.val % 256 := by
      rw [h_y9, and_val]; exact and_FF _ (by omega)
    have hD : D32 hw1.val hw2.val pc.val = some (.spUp (hw2.val % 256)) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_pos p_pop1]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    show AEff.spUp _ = AEff.spUp _; congr 1; scalar_tac
  rw [if_neg c_pop1]
  have n_pop1 : ¬ (hw1.val = 63581 ∧ hw2.val / 256 % 16 = 11) := by
    rw [h_b8] at c_pop1
    unread_nat at c_pop1
    omega
  step as ⟨ d15, h_d15, _ ⟩
  have v_d15 : d15.val = b.val / 32768 := by
    rw [h_d15]; exact Nat.shiftRight_eq_div_pow _ _
  step as ⟨ dpb, h_dpb, _ ⟩
  have v_dpb : dpb.val = b.val / 32768 % 2 := by
    rw [h_dpb, and_val, v_d15]; exact and_1 _ (by omega)
  step as ⟨ mw12, h_mw12, _ ⟩
  have v_mw12 : mw12.val = a.val / 2048 * 2048 + a.val % 1024 := by
    rw [h_mw12, and_val]; exact and_FBFF _ (by omega)
  step as ⟨ mt12, h_mt12, _ ⟩
  have v_mt12 : mt12.val = a.val / 2048 * 2048 + a.val / 32 % 32 * 32 + a.val % 16 := by
    rw [h_mt12, and_val]; exact and_FBEF _ (by omega)
  step with (either_spec _ _) as ⟨ e12a, h_e12a ⟩
  step as ⟨ xe12, h_xe12, _ ⟩
  have v_xe12 : xe12.val = s8.val % 16 := by
    rw [h_xe12, and_val]; exact and_F _ (by omega)
  step with (both_spec _ _) as ⟨ e12d, h_e12d ⟩
  step with (both_spec _ _) as ⟨ e12c, h_e12c ⟩
  by_cases c_subc : e12c = true
  · rw [if_pos c_subc]
    have p_subc : hw2.val / 32768 % 2 = 0 ∧ ((hw1.val / 2048 = 30 ∧ hw1.val / 32 % 32 = 13 ∧ hw1.val % 16 = 13) ∨ (hw1.val / 2048 = 30 ∧ hw1.val % 1024 = 685)) ∧ hw2.val / 256 % 16 = 13 := by
      rw [h_e12c, h_e12d, h_e12a] at c_subc
      unread_nat at c_subc
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
    have v_t5 : t5.val = t3.val + t4.val := by
      rw [h_t5, UScalar.val_or]; exact or_add _ _ 11 (by omega) (by omega)
    step as ⟨ imm12, h_imm12, _ ⟩
    have v_imm12 : imm12.val = t5.val + imm8.val := by
      rw [h_imm12, UScalar.val_or]; exact or_add _ _ 8 (by omega) (by omega)
    have e_imm : imm12.val = LinkThumb.imm12 hw1.val hw2.val := by
      unfold LinkThumb.imm12
      omega
    have b_imm : imm12.val < 4096 := by omega
    by_cases c_w : mw12 = 62125#u32
    · rw [if_pos c_w]
      have p_w : hw1.val / 2048 = 30 ∧ hw1.val % 1024 = 685 := by
        have e : mw12.val = 62125 := congrArg UScalar.val c_w
        omega
      have hD : D32 hw1.val hw2.val pc.val = some (.spDown (LinkThumb.imm12 hw1.val hw2.val)) := by
        unfold LinkThumb.D32
        rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_pos p_subc, if_pos p_w]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
      show AEff.spDown _ = AEff.spDown _; congr 1
    rw [if_neg c_w]
    have n_w : ¬ (hw1.val / 2048 = 30 ∧ hw1.val % 1024 = 685) := by
      have e : mw12.val ≠ 62125 := fun e => c_w ((UScalar.eq_equiv _ _).2 e)
      omega
    step with (thumb_expand_imm_spec imm12 b_imm) as ⟨ imm121, h_imm121 ⟩
    have hD : D32 hw1.val hw2.val pc.val = some (.spDown (LinkThumb.expand12 (LinkThumb.imm12 hw1.val hw2.val))) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_pos p_subc, if_neg n_w]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    show AEff.spDown _ = AEff.spDown _; congr 1
    rw [h_imm121, e_imm]
  rw [if_neg c_subc]
  have n_subc : ¬ (hw2.val / 32768 % 2 = 0 ∧ ((hw1.val / 2048 = 30 ∧ hw1.val / 32 % 32 = 13 ∧ hw1.val % 16 = 13) ∨ (hw1.val / 2048 = 30 ∧ hw1.val % 1024 = 685)) ∧ hw2.val / 256 % 16 = 13) := by
    rw [h_e12c, h_e12d, h_e12a] at c_subc
    unread_nat at c_subc
    omega
  step as ⟨ mw13, h_mw13, _ ⟩
  have v_mw13 : mw13.val = a.val / 2048 * 2048 + a.val % 1024 := by
    rw [h_mw13, and_val]; exact and_FBFF _ (by omega)
  step as ⟨ mt13, h_mt13, _ ⟩
  have v_mt13 : mt13.val = a.val / 2048 * 2048 + a.val / 32 % 32 * 32 + a.val % 16 := by
    rw [h_mt13, and_val]; exact and_FBEF _ (by omega)
  step with (either_spec _ _) as ⟨ e13a, h_e13a ⟩
  step as ⟨ xe13, h_xe13, _ ⟩
  have v_xe13 : xe13.val = s8.val % 16 := by
    rw [h_xe13, and_val]; exact and_F _ (by omega)
  step with (both_spec _ _) as ⟨ e13d, h_e13d ⟩
  step with (both_spec _ _) as ⟨ e13c, h_e13c ⟩
  by_cases c_addc : e13c = true
  · rw [if_pos c_addc]
    have p_addc : hw2.val / 32768 % 2 = 0 ∧ ((hw1.val / 2048 = 30 ∧ hw1.val / 32 % 32 = 8 ∧ hw1.val % 16 = 13) ∨ (hw1.val / 2048 = 30 ∧ hw1.val % 1024 = 525)) ∧ hw2.val / 256 % 16 = 13 := by
      rw [h_e13c, h_e13d, h_e13a] at c_addc
      unread_nat at c_addc
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
    have v_t5 : t5.val = t3.val + t4.val := by
      rw [h_t5, UScalar.val_or]; exact or_add _ _ 11 (by omega) (by omega)
    step as ⟨ imm12, h_imm12, _ ⟩
    have v_imm12 : imm12.val = t5.val + imm8.val := by
      rw [h_imm12, UScalar.val_or]; exact or_add _ _ 8 (by omega) (by omega)
    have e_imm : imm12.val = LinkThumb.imm12 hw1.val hw2.val := by
      unfold LinkThumb.imm12
      omega
    have b_imm : imm12.val < 4096 := by omega
    by_cases c_w : mw13 = 61965#u32
    · rw [if_pos c_w]
      have p_w : hw1.val / 2048 = 30 ∧ hw1.val % 1024 = 525 := by
        have e : mw13.val = 61965 := congrArg UScalar.val c_w
        omega
      have hD : D32 hw1.val hw2.val pc.val = some (.spUp (LinkThumb.imm12 hw1.val hw2.val)) := by
        unfold LinkThumb.D32
        rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_pos p_addc, if_pos p_w]
      (try simp only [WP.spec_ok])
      intro x hx
      rw [hD] at hx
      cases hx
      show AEff.spUp _ = AEff.spUp _; congr 1
    rw [if_neg c_w]
    have n_w : ¬ (hw1.val / 2048 = 30 ∧ hw1.val % 1024 = 525) := by
      have e : mw13.val ≠ 61965 := fun e => c_w ((UScalar.eq_equiv _ _).2 e)
      omega
    step with (thumb_expand_imm_spec imm12 b_imm) as ⟨ imm121, h_imm121 ⟩
    have hD : D32 hw1.val hw2.val pc.val = some (.spUp (LinkThumb.expand12 (LinkThumb.imm12 hw1.val hw2.val))) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_pos p_addc, if_neg n_w]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    show AEff.spUp _ = AEff.spUp _; congr 1
    rw [h_imm121, e_imm]
  rw [if_neg c_addc]
  have n_addc : ¬ (hw2.val / 32768 % 2 = 0 ∧ ((hw1.val / 2048 = 30 ∧ hw1.val / 32 % 32 = 8 ∧ hw1.val % 16 = 13) ∨ (hw1.val / 2048 = 30 ∧ hw1.val % 1024 = 525)) ∧ hw2.val / 256 % 16 = 13) := by
    rw [h_e13c, h_e13d, h_e13a] at c_addc
    unread_nat at c_addc
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
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_neg n_addc, if_pos p_spreg]
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
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_pos p_movsp]
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
  step with (sp_unread_spec a b la lb) as ⟨ u, h_u ⟩
  by_cases c_unread : u = true
  · rw [if_pos c_unread]
    have p_unread : LinkThumb.Unread hw1.val hw2.val := by
      rw [h_u] at c_unread
      have q := of_decide_eq_true c_unread
      rwa [va, vb] at q
    have hD : D32 hw1.val hw2.val pc.val = some (.spUnread) := by
      unfold LinkThumb.D32
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_pos p_unread]
    (try simp only [WP.spec_ok])
    intro x hx
    rw [hD] at hx
    cases hx
    rfl
  rw [if_neg c_unread]
  have n_unread : ¬ LinkThumb.Unread hw1.val hw2.val := by
    intro q
    apply c_unread
    rw [h_u]
    apply decide_eq_true
    rw [va, vb]
    exact q
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
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_unread, if_pos p_movw]
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
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_unread, if_neg n_movw, if_pos p_lit]
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
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_unread, if_neg n_movw, if_neg n_lit, if_pos p_vfp]
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
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_unread, if_neg n_movw, if_neg n_lit, if_neg n_vfp, if_pos p_tbb]
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
        rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_unread, if_neg n_movw, if_neg n_lit, if_neg n_vfp, if_neg n_tbb, if_pos p_bl, if_pos p_link]
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
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_unread, if_neg n_movw, if_neg n_lit, if_neg n_vfp, if_neg n_tbb, if_pos p_bl, if_neg n_link]
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
        rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_unread, if_neg n_movw, if_neg n_lit, if_neg n_vfp, if_neg n_tbb, if_neg n_bl, if_pos p_bcw, if_pos p_never]
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
      rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_unread, if_neg n_movw, if_neg n_lit, if_neg n_vfp, if_neg n_tbb, if_neg n_bl, if_pos p_bcw, if_neg n_never]
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
    rw [if_neg n_dsb, if_neg n_ldrsp, if_neg n_msr, if_neg n_mrs, if_neg n_pushw, if_neg n_popw, if_neg n_vpush, if_neg n_vpop, if_neg n_strpre, if_neg n_strd, if_neg n_ldrpc, if_neg n_pop1, if_neg n_subc, if_neg n_addc, if_neg n_spreg, if_neg n_movsp, if_neg n_unread, if_neg n_movw, if_neg n_lit, if_neg n_vfp, if_neg n_tbb, if_neg n_bl, if_neg n_bcw]
  (try simp only [WP.spec_ok])
  intro x hx
  rw [hD] at hx
  cases hx
  rfl

end rsk_link
