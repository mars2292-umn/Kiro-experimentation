import Sched.Kernel
import Tools.CrcSpec
open Aeneas Aeneas.Std Result

/-! # The Kernel's checksum is CRC-32 (2026-11-26)

`Crc32` of `crates/rsk-kernel/src/arch/bound.rs` is the checksum the boot
computes over the Generated_Config tables and the layout (`config_crc`,
`layout_crc`; R12.3). Verus proves `Crc32::byte` against a definition in
that crate; `Sched/CrcTotal.lean` proves that the functions return. This
file proves, of the functions as extracted (`Sched/Kernel.lean`), that
they compute the one definition of `Tools/CrcSpec.lean`, the same the
Config_Checker's `crc32` is proven against (`Tools/CcImageProofs.lean`):

- `crc_new_spec`: the initial value;
- `crc_byte_spec`: one byte (`Crc32::byte`'s contract);
- `crc_u32_spec`, `crc_u64_spec`: a word is its little-endian bytes, in
  order (not stated in Verus);
- `crc_finish_spec`: the final complement.

Not here: that `config_crc` and `layout_crc` feed the fields of the tables
in the order the Generator writes them (the Config_Checker compares the
image byte for byte, `image_is`; the boot compares the two checksums). -/

namespace P18
open rsk_kernel

theorem kcrc_round_bv (c mask : BitVec 32)
    (hm : mask = if c &&& 1#32 = 1#32 then 0xFFFFFFFF#32 else 0#32) :
    (c >>> 1) ^^^ (0xEDB88320#32 &&& mask) = Crc.round c := by
  unfold Crc.round
  subst hm
  split <;> simp

theorem crc_byte_loop_spec (c : U32) (k : Usize) (hk : k.val ≤ 8) :
    arch.bound.Crc32.byte_loop c k ⦃ r => r.bv = Crc.rounds c.bv (8 - k.val) ⦄ := by
  unfold arch.bound.Crc32.byte_loop
  apply loop.spec_decr_nat (fun (s : U32 × Usize) => 8 - s.2.val)
    (fun s => s.2.val ≤ 8 ∧ Crc.rounds s.1.bv (8 - s.2.val) = Crc.rounds c.bv (8 - k.val))
  · intro s hs
    obtain ⟨c1, k1⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold arch.bound.Crc32.byte_loop.body
    split
    · rename_i hlt
      have hlt' : k1.val < 8 := by scalar_tac
      step as ⟨ i, hi, hib ⟩
      have hcond : i = 1#u32 ↔ c1.bv &&& 1#32 = 1#32 := by
        rw [U32.eq_equiv_bv_eq]
        constructor
        · intro h; exact hib.symm.trans h
        · intro h; exact hib.trans h
      split
      · rename_i he
        step as ⟨ i1, hi1, hi1b ⟩
        step as ⟨ i2, hi2, hi2b ⟩
        step as ⟨ next, hn, hnb ⟩
        step as ⟨ k2, hk2 ⟩
        have hr : next.bv = Crc.round c1.bv := by
          refine hnb.trans ?_
          refine (congrArg₂ (· ^^^ ·) hi1b hi2b).trans ?_
          exact kcrc_round_bv c1.bv _ (by rw [if_pos (hcond.1 he)]; rfl)
        have e8 : 8 - k1.val = (8 - k2.val) + 1 := by omega
        refine ⟨by omega, ?_, by omega⟩
        rw [← h2, e8, Crc.rounds_succ, hr]
      · rename_i hne
        step as ⟨ i1, hi1, hi1b ⟩
        step as ⟨ i2, hi2, hi2b ⟩
        step as ⟨ next, hn, hnb ⟩
        step as ⟨ k2, hk2 ⟩
        have hr : next.bv = Crc.round c1.bv := by
          refine hnb.trans ?_
          refine (congrArg₂ (· ^^^ ·) hi1b hi2b).trans ?_
          exact kcrc_round_bv c1.bv _ (by rw [if_neg (fun h => hne (hcond.2 h))]; rfl)
        have e8 : 8 - k1.val = (8 - k2.val) + 1 := by omega
        refine ⟨by omega, ?_, by omega⟩
        rw [← h2, e8, Crc.rounds_succ, hr]
    · rename_i hge
      have : k1.val = 8 := by scalar_tac
      simp only [WP.spec_ok]
      rw [← h2, this]
      rfl
  · exact ⟨hk, rfl⟩

/-- The initial value. -/
theorem crc_new_spec : arch.bound.Crc32.new = ok 4294967295#u32 := rfl

/-- **One byte** (`Crc32::byte`'s contract): the state after the byte is
the definition's. -/
theorem crc_byte_spec (c : arch.bound.Crc32) (b : U8) :
    arch.bound.Crc32.byte c b ⦃ r => r.bv = Crc.byte c.bv b.bv ⦄ := by
  unfold arch.bound.Crc32.byte
  step as ⟨ i, hi ⟩
  step as ⟨ start, hst, hstb ⟩
  step with crc_byte_loop_spec as ⟨ r, hr ⟩
  have hcast : i.bv = b.bv.setWidth 32 := by rw [hi]; rfl
  refine hr.trans ?_
  show Crc.rounds start.bv 8 = Crc.rounds (c.bv ^^^ b.bv.setWidth 32) 8
  exact congrArg (fun x => Crc.rounds x 8) (hstb.trans (congrArg (c.bv ^^^ ·) hcast))

theorem low8 (x : BitVec 32) : (x &&& 255#32).setWidth 8 = x.setWidth 8 := by
  ext i hi'
  have h8 : i < 8 := hi'
  simp only [BitVec.getElem_setWidth, BitVec.getLsbD_and]
  have : (255#32).getLsbD i = true := by
    have : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 ∨ i = 4 ∨ i = 5 ∨ i = 6 ∨ i = 7 := by omega
    rcases this with h | h | h | h | h | h | h | h <;> subst h <;> rfl
  simp [this]

theorem low32 (x : BitVec 64) : (x &&& 4294967295#64).setWidth 32 = x.setWidth 32 := by
  apply BitVec.eq_of_toNat_eq
  simp only [BitVec.toNat_setWidth, BitVec.toNat_and, BitVec.toNat_ofNat]
  have e : 4294967295 % 2 ^ 64 = 2 ^ 32 - 1 := by decide
  rw [e, Nat.and_two_pow_sub_one_eq_mod]
  exact Nat.mod_mod _ _

/-- The byte the code takes from a word: the low byte after the mask. -/
theorem masked_byte {x i : U32} {b : U8} (hib : (i.bv : BitVec 32) = (x.bv : BitVec 32) &&& 255#32)
    (hb : b = UScalar.cast .U8 i) : b.bv = x.bv.setWidth 8 := by
  have e : b.bv = i.bv.setWidth 8 := by rw [hb]; rfl
  refine e.trans ?_
  refine (congrArg (BitVec.setWidth 8) hib).trans ?_
  exact low8 x.bv

/-- **A 32-bit word** (`Crc32::u32`): its four bytes, the low one first. -/
theorem crc_u32_spec (c : arch.bound.Crc32) (v : U32) :
    arch.bound.Crc32.u32 c v ⦃ r => r.bv = Crc.run c.bv (Crc.le32 v.bv) ⦄ := by
  unfold arch.bound.Crc32.u32
  step as ⟨ i, hi, hib ⟩
  step as ⟨ b0, hb0 ⟩
  step with crc_byte_spec as ⟨ c0, hc0 ⟩
  step as ⟨ s1, hs1, hs1b ⟩
  step as ⟨ j1, hj1, hj1b ⟩
  step as ⟨ b1, hb1 ⟩
  step with crc_byte_spec as ⟨ c1, hc1 ⟩
  step as ⟨ s2, hs2, hs2b ⟩
  step as ⟨ j2, hj2, hj2b ⟩
  step as ⟨ b2, hb2 ⟩
  step with crc_byte_spec as ⟨ c2, hc2 ⟩
  step as ⟨ s3, hs3, hs3b ⟩
  step as ⟨ j3, hj3, hj3b ⟩
  step as ⟨ b3, hb3 ⟩
  step with crc_byte_spec as ⟨ c3, hc3 ⟩
  have e0 := masked_byte hib hb0
  have e1 := masked_byte hj1b hb1
  have e2 := masked_byte hj2b hb2
  have e3 := masked_byte hj3b hb3
  have e1' : b1.bv = (v.bv >>> 8).setWidth 8 := e1.trans (congrArg (BitVec.setWidth 8) hs1b)
  have e2' : b2.bv = (v.bv >>> 16).setWidth 8 := e2.trans (congrArg (BitVec.setWidth 8) hs2b)
  have e3' : b3.bv = (v.bv >>> 24).setWidth 8 := e3.trans (congrArg (BitVec.setWidth 8) hs3b)
  have f0 : c0.bv = Crc.byte c.bv (v.bv.setWidth 8) := hc0.trans (congrArg (Crc.byte c.bv) e0)
  have f1 := hc1.trans (congrArg₂ Crc.byte f0 e1')
  have f2 := hc2.trans (congrArg₂ Crc.byte f1 e2')
  have f3 := hc3.trans (congrArg₂ Crc.byte f2 e3')
  exact f3

/-- **A 64-bit word** (`Crc32::u64`): its eight bytes, the low one first. -/
theorem crc_u64_spec (c : arch.bound.Crc32) (v : U64) :
    arch.bound.Crc32.u64 c v ⦃ r => r.bv = Crc.run c.bv (Crc.le64 v.bv) ⦄ := by
  unfold arch.bound.Crc32.u64
  step as ⟨ i, hi, hib ⟩
  step as ⟨ lo, hlo ⟩
  step with crc_u32_spec as ⟨ c0, hc0 ⟩
  step as ⟨ s, hs, hsb ⟩
  step as ⟨ j, hj, hjb ⟩
  step as ⟨ hi32, hhi ⟩
  step with crc_u32_spec as ⟨ c1, hc1 ⟩
  have elo : lo.bv = v.bv.setWidth 32 := by
    have e : lo.bv = i.bv.setWidth 32 := by rw [hlo]; rfl
    exact e.trans ((congrArg (BitVec.setWidth 32) hib).trans (low32 v.bv))
  have ehi : hi32.bv = (v.bv >>> 32).setWidth 32 := by
    have e : hi32.bv = j.bv.setWidth 32 := by rw [hhi]; rfl
    refine e.trans ((congrArg (BitVec.setWidth 32) hjb).trans ?_)
    exact (low32 s.bv).trans (congrArg (BitVec.setWidth 32) hsb)
  have f0 : c0.bv = Crc.run c.bv (Crc.le32 (v.bv.setWidth 32)) :=
    hc0.trans (congrArg (fun x => Crc.run c.bv (Crc.le32 x)) elo)
  have f1 : c1.bv = Crc.run (Crc.run c.bv (Crc.le32 (v.bv.setWidth 32)))
      (Crc.le32 ((v.bv >>> 32).setWidth 32)) :=
    hc1.trans (congrArg₂ (fun a x => Crc.run a (Crc.le32 x)) f0 ehi)
  refine f1.trans ?_
  unfold Crc.le64
  rw [Crc.run_append]

/-- The final complement (`Crc32::finish`). -/
theorem crc_finish_spec (c : arch.bound.Crc32) :
    arch.bound.Crc32.finish c ⦃ r => r.bv = ~~~ c.bv ⦄ := by
  unfold arch.bound.Crc32.finish
  simp only [WP.spec_ok]
  rfl

/-- The checksum of a list of bytes fed one at a time from the initial
value and finished is the CRC-32 of the list: what `Crc32::new`, `byte`
and `finish` compute together. -/
theorem crc_of_bytes (bs : List (BitVec 8)) : ~~~ Crc.run 0xFFFFFFFF#32 bs = Crc.crc32 bs := rfl

end P18
