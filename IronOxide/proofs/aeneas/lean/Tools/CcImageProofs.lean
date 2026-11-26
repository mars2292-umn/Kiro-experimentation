/-
  The Config_Checker's image functions on the code as compiled
  (2026-11-26): the statements Verus proves of
  `rsk_confcheck::image_v`, proven of the functions as Aeneas extracts
  them (`Tools/CcImage.lean`). Each theorem is total: the function
  returns for every input, and its result satisfies the postcondition.
  Handwritten; the repetitive reader and loop lemmas were first produced
  by a script that is not kept.

  - `crc32_spec`: the checksum is CRC-32, the one definition of
    `Tools/CrcSpec.lean` (the Kernel's `Crc32` is proven against the same
    definition in `Sched/Crc.lean`).
  - `decode_tables_spec`, `decode_layout_image_spec`,
    `decode_endpoint_image_spec`: what a decoder accepts is exactly the
    encoding of what it returns, and the result is well formed (R26.4).
  - `image_is_spec`, `layout_image_is_spec`, `endpoint_image_is_spec`: a
    check answers `true` only when the bytes are, byte for byte, the
    image of the expected value.
  - `storage_placed_ok_spec`: the storage-placement check returns `true`
    exactly when `StoragePlaced` holds (R16.1; Verus proves the direction
    from `true`); `storage_in_kernel_ram_ok_spec`.

  The encodings (`encU32`, `specImage`, ...) restate the Verus
  definitions of `image_v.rs` over Lean lists of bytes; that the two
  texts say the same is by reading. Nothing here is shared with the
  Generator's proofs (R26.1) except the loop rules of `Tools/FlagLoop.lean`,
  which state nothing of either tool.
-/
import Aeneas
import Tools.CcImage
import Tools.CrcSpec
import Tools.FlagLoop
open Aeneas Aeneas.Std Result

namespace rsk_confcheck

/-- One round of the loop is one round of the definition. -/
theorem crc_round_bv (c mask : BitVec 32)
    (hm : mask = if c &&& 1#32 = 1#32 then 0xFFFFFFFF#32 else 0#32) :
    (c >>> 1) ^^^ (0xEDB88320#32 &&& mask) = Crc.round c := by
  unfold Crc.round
  subst hm
  split <;> simp

theorem crc32_inner_spec (c : U32) (k : Usize) (hk : k.val ≤ 8) :
    image_v.crc32_loop0_loop0 c k ⦃ r => r.bv = Crc.rounds c.bv (8 - k.val) ⦄ := by
  unfold image_v.crc32_loop0_loop0
  apply loop.spec_decr_nat (fun (s : U32 × Usize) => 8 - s.2.val)
    (fun s => s.2.val ≤ 8 ∧ Crc.rounds s.1.bv (8 - s.2.val) = Crc.rounds c.bv (8 - k.val))
  · intro s hs
    obtain ⟨c1, k1⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold image_v.crc32_loop0_loop0.body
    split
    · rename_i hlt
      have hlt' : k1.val < 8 := by scalar_tac
      step as ⟨ i, hi, hib ⟩
      have h1u : (1#u32).bv = 1#32 := rfl
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
          exact crc_round_bv c1.bv _ (by rw [if_pos (hcond.1 he)]; rfl)
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
          exact crc_round_bv c1.bv _ (by rw [if_neg (fun h => hne (hcond.2 h))]; rfl)
        have e8 : 8 - k1.val = (8 - k2.val) + 1 := by omega
        refine ⟨by omega, ?_, by omega⟩
        rw [← h2, e8, Crc.rounds_succ, hr]
    · rename_i hge
      have : k1.val = 8 := by scalar_tac
      simp only [WP.spec_ok]
      rw [← h2, this]
      rfl
  · exact ⟨hk, rfl⟩

/-- The bytes of a slice as bit vectors. -/
def bvs (l : List U8) : List (BitVec 8) := l.map (·.bv)

theorem crc32_outer_spec (bytes : Slice U8) :
    image_v.crc32_loop0 bytes 4294967295#u32 0#usize ⦃ r => r.bv = Crc.state (bvs bytes.val) ⦄ := by
  unfold image_v.crc32_loop0
  apply loop.spec_decr_nat (fun (s : U32 × Usize) => bytes.val.length - s.2.val)
    (fun s => s.2.val ≤ bytes.val.length ∧ s.1.bv = Crc.state (bvs (bytes.val.take s.2.val)))
  · intro s hs
    obtain ⟨crc, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold image_v.crc32_loop0.body
    simp only [Slice.len]
    split
    · rename_i hlt
      have hlt' : i.val < bytes.val.length := by scalar_tac
      step as ⟨ b, hb ⟩
      step as ⟨ i3, hi3 ⟩
      step as ⟨ start, hst, hstb ⟩
      step with crc32_inner_spec as ⟨ c, hc ⟩
      step as ⟨ i4, hi4 ⟩
      have htake : bytes.val.take i4.val = bytes.val.take i.val ++ [b] := by
        rw [hi4, hb]
        exact (List.take_succ_eq_append_getElem hlt')
      have hcast : i3.bv = b.bv.setWidth 32 := by rw [hi3]; rfl
      have e : c.bv = Crc.byte crc.bv b.bv := by
        refine hc.trans ?_
        show Crc.rounds start.bv 8 = Crc.rounds (crc.bv ^^^ b.bv.setWidth 32) 8
        exact congrArg (fun x => Crc.rounds x 8) (hstb.trans (congrArg (crc.bv ^^^ ·) hcast))
      refine ⟨by omega, ?_, by omega⟩
      refine e.trans ?_
      rw [htake]
      simp only [bvs, List.map_append, List.map_cons, List.map_nil, Crc.state, Crc.run_append]
      show Crc.byte crc.bv b.bv = Crc.byte (Crc.state (bvs (bytes.val.take i.val))) b.bv
      exact congrArg (Crc.byte · b.bv) h2
    · rename_i hge
      have : i.val = bytes.val.length := by scalar_tac
      simp only [WP.spec_ok]
      rw [h2, this, List.take_length]
  · exact ⟨by scalar_tac, by simp [Crc.state, Crc.run, bvs]⟩

/-- **The Config_Checker's checksum is CRC-32** (`image_v::crc32`'s
contract, R26.4): the function returns for every input, and its result is
the CRC-32 of the bytes. -/
theorem crc32_spec (bytes : Slice U8) :
    image_v.crc32 bytes ⦃ r => r.bv = Crc.crc32 (bvs bytes.val) ⦄ := by
  unfold image_v.crc32
  step with crc32_outer_spec as ⟨ crc, hc ⟩
  simp only [Crc.crc32, ← hc]
  rfl

/-! ## The readers

`Reads bytes pos pos' enc`: the reader moved from `pos` to `pos'`, inside
the image, and the bytes it passed are `enc`. Every reader's statement is
this, with `enc` the encoding of what it returned (R26.4). -/

/-- A `u32`, least significant byte first (`enc_u32`). -/
def encU32 (v : BitVec 32) : List (BitVec 8) :=
  [v.setWidth 8, (v >>> 8).setWidth 8, (v >>> 16).setWidth 8, (v >>> 24).setWidth 8]

/-- A `u64`: its low half, then its high half (`enc_u64`). -/
def encU64 (v : BitVec 64) : List (BitVec 8) :=
  encU32 (v.setWidth 32) ++ encU32 ((v >>> 32).setWidth 32)

/-- A flag byte (`enc_flag`). -/
def encFlag (b : Bool) : List (BitVec 8) := [if b then 1#8 else 0#8]

def Reads (bytes : Slice U8) (pos pos' : Usize) (enc : List (BitVec 8)) : Prop :=
  pos'.val ≤ bytes.val.length ∧
  bvs (bytes.val.take pos'.val) = bvs (bytes.val.take pos.val) ++ enc

theorem Reads.refl {bytes : Slice U8} {pos : Usize} (h : pos.val ≤ bytes.val.length) :
    Reads bytes pos pos [] := ⟨h, by simp⟩

theorem Reads.trans {bytes : Slice U8} {p p' p'' : Usize} {a b : List (BitVec 8)}
    (h1 : Reads bytes p p' a) (h2 : Reads bytes p' p'' b) : Reads bytes p p'' (a ++ b) :=
  ⟨h2.1, by rw [h2.2, h1.2, List.append_assoc]⟩

theorem Reads.cast {bytes : Slice U8} {p p' : Usize} {a b : List (BitVec 8)}
    (h : Reads bytes p p' a) (e : a = b) : Reads bytes p p' b := e ▸ h

/-- Closes the branch in which a reader found the image too short. -/
macro "rd_none " h:term : tactic =>
  `(tactic| (simp only [WP.spec_ok]; first | exact ⟨$h, trivial⟩ | exact $h))

/-- One byte, or `None` at the end of the image. -/
theorem rd_u8_spec (bytes : Slice U8) (pos : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_u8 bytes pos ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some b => Reads bytes pos r.2 [b.bv]
      | none => True ⦄ := by
  unfold image_v.rd_u8
  simp only [Slice.len]
  split
  · simp only [WP.spec_ok]
    exact ⟨hp, trivial⟩
  · rename_i hlt
    have hlt' : pos.val < bytes.val.length := by scalar_tac
    step as ⟨ b, hb ⟩
    step as ⟨ pos1, hpos1 ⟩
    refine ⟨by omega, by omega, ?_⟩
    rw [hpos1, List.take_succ_eq_append_getElem hlt', ← hb]
    simp [bvs]

theorem le32_b0 (b0 b1 b2 b3 : BitVec 8) : (b0.setWidth 32 ||| (b1.setWidth 32 <<< 8) ||| (b2.setWidth 32 <<< 16) |||
      (b3.setWidth 32 <<< 24)).setWidth 8 = b0 := by
  ext i hi'
  have h8 : i < 8 := hi'
  simp only [BitVec.getElem_setWidth, BitVec.getLsbD_or, BitVec.getLsbD_setWidth,
    BitVec.getLsbD_shiftLeft, BitVec.getLsbD_ushiftRight]
  have a1 : i < 32 := by omega
  have a2 : ¬ (8 ≤ i) := by omega
  have a3 : i < 16 := by omega
  have a4 : i < 24 := by omega
  simp [a1, a2, a3, a4, h8, BitVec.getLsbD_eq_getElem h8]
  try (intro _; omega)

theorem le32_b1 (b0 b1 b2 b3 : BitVec 8) : ((b0.setWidth 32 ||| (b1.setWidth 32 <<< 8) ||| (b2.setWidth 32 <<< 16) |||
      (b3.setWidth 32 <<< 24)) >>> 8).setWidth 8 = b1 := by
  ext i hi'
  have h8 : i < 8 := hi'
  simp only [BitVec.getElem_setWidth, BitVec.getLsbD_or, BitVec.getLsbD_setWidth,
    BitVec.getLsbD_shiftLeft, BitVec.getLsbD_ushiftRight]
  have a1 : 8 + i < 32 := by omega
  have a2 : 8 + i < 16 := by omega
  have a3 : 8 + i < 24 := by omega
  have a4 : ¬ (8 + i < 8) := by omega
  have a5 : 8 + i - 8 = i := by omega
  have a6 : b0.getLsbD (8 + i) = false := BitVec.getLsbD_of_ge b0 (8 + i) (by omega)
  simp [a1, a2, a3, a4, a5, a6, h8, BitVec.getLsbD_eq_getElem h8]
  try (intro _; omega)

theorem le32_b2 (b0 b1 b2 b3 : BitVec 8) : ((b0.setWidth 32 ||| (b1.setWidth 32 <<< 8) ||| (b2.setWidth 32 <<< 16) |||
      (b3.setWidth 32 <<< 24)) >>> 16).setWidth 8 = b2 := by
  ext i hi'
  have h8 : i < 8 := hi'
  simp only [BitVec.getElem_setWidth, BitVec.getLsbD_or, BitVec.getLsbD_setWidth,
    BitVec.getLsbD_shiftLeft, BitVec.getLsbD_ushiftRight]
  have a1 : 16 + i < 32 := by omega
  have a2 : ¬ (16 + i < 16) := by omega
  have a3 : 16 + i < 24 := by omega
  have a4 : ¬ (16 + i < 8) := by omega
  have a5 : 16 + i - 16 = i := by omega
  have a6 : b0.getLsbD (16 + i) = false := BitVec.getLsbD_of_ge b0 (16 + i) (by omega)
  have a7 : b1.getLsbD (16 + i - 8) = false := BitVec.getLsbD_of_ge b1 _ (by omega)
  simp [a1, a2, a3, a4, a5, a6, a7, h8, BitVec.getLsbD_eq_getElem h8]
  try (intro _; omega)

theorem le32_b3 (b0 b1 b2 b3 : BitVec 8) : ((b0.setWidth 32 ||| (b1.setWidth 32 <<< 8) ||| (b2.setWidth 32 <<< 16) |||
      (b3.setWidth 32 <<< 24)) >>> 24).setWidth 8 = b3 := by
  ext i hi'
  have h8 : i < 8 := hi'
  simp only [BitVec.getElem_setWidth, BitVec.getLsbD_or, BitVec.getLsbD_setWidth,
    BitVec.getLsbD_shiftLeft, BitVec.getLsbD_ushiftRight]
  have a1 : 24 + i < 32 := by omega
  have a2 : ¬ (24 + i < 16) := by omega
  have a3 : ¬ (24 + i < 24) := by omega
  have a4 : ¬ (24 + i < 8) := by omega
  have a5 : 24 + i - 24 = i := by omega
  have a6 : b0.getLsbD (24 + i) = false := BitVec.getLsbD_of_ge b0 _ (by omega)
  have a7 : b1.getLsbD (24 + i - 8) = false := BitVec.getLsbD_of_ge b1 _ (by omega)
  have a8 : b2.getLsbD (24 + i - 16) = false := BitVec.getLsbD_of_ge b2 _ (by omega)
  simp [a1, a2, a3, a4, a5, a6, a7, a8, h8, BitVec.getLsbD_eq_getElem h8]
  try (intro _; omega)

theorem le32_bytes (b0 b1 b2 b3 : BitVec 8) :
    encU32 (b0.setWidth 32 ||| (b1.setWidth 32 <<< 8) ||| (b2.setWidth 32 <<< 16) |||
      (b3.setWidth 32 <<< 24)) = [b0, b1, b2, b3] := by
  unfold encU32
  rw [le32_b0, le32_b1, le32_b2, le32_b3]

theorem rd_u32_spec (bytes : Slice U8) (pos : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_u32 bytes pos ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some v => Reads bytes pos r.2 (encU32 v.bv)
      | none => True ⦄ := by
  unfold image_v.rd_u32
  step with (rd_u8_spec bytes pos hp) as ⟨ o, pos1, h1, r1 ⟩
  rcases o with _ | b0
  · rd_none h1
  dsimp only at r1 ⊢
  step with (rd_u8_spec bytes pos1 h1) as ⟨ o, pos2, h2, r2 ⟩
  rcases o with _ | b1
  · rd_none h2
  dsimp only at r2 ⊢
  step with (rd_u8_spec bytes pos2 h2) as ⟨ o, pos3, h3, r3 ⟩
  rcases o with _ | b2
  · rd_none h3
  dsimp only at r3 ⊢
  step with (rd_u8_spec bytes pos3 h3) as ⟨ o, pos4, h4, r4 ⟩
  rcases o with _ | b3
  · rd_none h4
  dsimp only at r4 ⊢
  step as ⟨ i, hi ⟩
  step as ⟨ i1, hi1 ⟩
  step as ⟨ i2, hi2, hi2b ⟩
  step as ⟨ i3, hi3, hi3b ⟩
  step as ⟨ i4, hi4 ⟩
  step as ⟨ i5, hi5, hi5b ⟩
  step as ⟨ i6, hi6, hi6b ⟩
  step as ⟨ i7, hi7 ⟩
  step as ⟨ i8, hi8, hi8b ⟩
  step as ⟨ v, hv, hvb ⟩
  have c0 : i.bv = b0.bv.setWidth 32 := by rw [hi]; rfl
  have c1 : i1.bv = b1.bv.setWidth 32 := by rw [hi1]; rfl
  have c2 : i4.bv = b2.bv.setWidth 32 := by rw [hi4]; rfl
  have c3 : i7.bv = b3.bv.setWidth 32 := by rw [hi7]; rfl
  have e2 : i2.bv = b1.bv.setWidth 32 <<< 8 := hi2b.trans (congrArg (fun x : BitVec 32 => x <<< (8 : Nat)) c1)
  have e3 : i3.bv = b0.bv.setWidth 32 ||| (b1.bv.setWidth 32 <<< 8) :=
    hi3b.trans (congrArg₂ (· ||| ·) c0 e2)
  have e5 : i5.bv = b2.bv.setWidth 32 <<< 16 := hi5b.trans (congrArg (fun x : BitVec 32 => x <<< (16 : Nat)) c2)
  have e6 := hi6b.trans (congrArg₂ (· ||| ·) e3 e5)
  have e8 : i8.bv = b3.bv.setWidth 32 <<< 24 := hi8b.trans (congrArg (fun x : BitVec 32 => x <<< (24 : Nat)) c3)
  have ev := hvb.trans (congrArg₂ (· ||| ·) e6 e8)
  refine ⟨h4, ?_⟩
  exact (((r1.trans r2).trans r3).trans r4).cast
    ((congrArg encU32 ev).trans (le32_bytes b0.bv b1.bv b2.bv b3.bv)).symm

theorem le64_halves (lo hi : BitVec 32) :
    encU64 (lo.setWidth 64 ||| (hi.setWidth 64 <<< 32)) = encU32 lo ++ encU32 hi := by
  unfold encU64
  have e0 : (lo.setWidth 64 ||| (hi.setWidth 64 <<< 32)).setWidth 32 = lo := by
    ext i hi'
    simp only [BitVec.getElem_setWidth, BitVec.getLsbD_or, BitVec.getLsbD_setWidth,
      BitVec.getLsbD_shiftLeft]
    have : i < 32 := hi'
    simp [this]
    intro _; omega
  have e1 : ((lo.setWidth 64 ||| (hi.setWidth 64 <<< 32)) >>> 32).setWidth 32 = hi := by
    ext i hi'
    simp only [BitVec.getElem_setWidth, BitVec.getLsbD_ushiftRight, BitVec.getLsbD_or,
      BitVec.getLsbD_setWidth, BitVec.getLsbD_shiftLeft]
    have : i < 32 := hi'
    simp [this]
    have a1 : 32 + i < 64 := by omega
    have a2 : i < 64 := by omega
    simp [a1, a2]
  rw [e0, e1]

theorem rd_u64_spec (bytes : Slice U8) (pos : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_u64 bytes pos ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some v => Reads bytes pos r.2 (encU64 v.bv)
      | none => True ⦄ := by
  unfold image_v.rd_u64
  step with (rd_u32_spec bytes pos hp) as ⟨ o, pos1, h1, r1 ⟩
  rcases o with _ | lo
  · rd_none h1
  dsimp only at r1 ⊢
  step with (rd_u32_spec bytes pos1 h1) as ⟨ o, pos2, h2, r2 ⟩
  rcases o with _ | hi
  · rd_none h2
  dsimp only at r2 ⊢
  step as ⟨ i, hi0 ⟩
  step as ⟨ i1, hi1 ⟩
  step as ⟨ i2, hi2, hi2b ⟩
  step as ⟨ v, hv, hvb ⟩
  have c0 : i.bv = lo.bv.setWidth 64 := by rw [hi0]; rfl
  have c1 : i1.bv = hi.bv.setWidth 64 := by rw [hi1]; rfl
  have e2 : i2.bv = hi.bv.setWidth 64 <<< 32 := hi2b.trans (congrArg (fun x : BitVec 64 => x <<< (32 : Nat)) c1)
  have ev := hvb.trans (congrArg₂ (· ||| ·) c0 e2)
  refine ⟨h2, ?_⟩
  exact (r1.trans r2).cast ((congrArg encU64 ev).trans (le64_halves lo.bv hi.bv)).symm

/-- A flag byte: 0 or 1, anything else refused. -/
theorem rd_flag_spec (bytes : Slice U8) (pos : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_flag bytes pos ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok b => Reads bytes pos r.2 (encFlag b)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_flag
  step with (rd_u8_spec bytes pos hp) as ⟨ o, pos1, h1, r1 ⟩
  rcases o with _ | b
  · rd_none h1
  dsimp only at r1 ⊢
  split
  · rename_i hb
    simp only [WP.spec_ok]
    refine ⟨h1, r1.cast ?_⟩
    rfl
  · rename_i hb
    simp only [WP.spec_ok]
    refine ⟨h1, r1.cast ?_⟩
    rfl
  · rd_none h1

theorem bvs_append (a b : List U8) : bvs (a ++ b) = bvs a ++ bvs b := by simp [bvs]

/-! ## The `CONFIG` image -/

open image_v in
/-- `enc_task`. -/
def encTask (t : TaskCfg) : List (BitVec 8) :=
  [t.priority.bv, t.partition.bv, t.kind.bv] ++ encU64 t.offset.bv ++ encU64 t.period.bv ++
    encU64 t.deadline.bv ++ encU32 t.budget.bv ++ [t.mit_policy.bv] ++ encFlag t.fpu

def encTasks (l : List image_v.TaskCfg) : List (BitVec 8) := l.flatMap encTask

/-- `enc_resource`. -/
def encResource (r : image_v.ResourceCfg) : List (BitVec 8) := [r.ceiling.bv, r.partition.bv]

def encResources (l : List image_v.ResourceCfg) : List (BitVec 8) := l.flatMap encResource

/-- `enc_flags`. -/
def encFlags (l : List Bool) : List (BitVec 8) := l.flatMap encFlag

/-- `enc_rows`, over the rows as the decoder holds them. -/
def encRows (l : List (alloc.vec.Vec Bool)) : List (BitVec 8) := l.flatMap (fun r => encFlags r.val)

/-- `enc_partition`. -/
def encPartition (p : image_v.PartitionCfg) : List (BitVec 8) :=
  [p.level.bv, p.fault_response.bv, p.overrun_response.bv, p.deadline_response.bv] ++
    encU32 p.mit_threshold.bv

def encPartitions (l : List image_v.PartitionCfg) : List (BitVec 8) := l.flatMap encPartition

/-- The `CONFIG` image of the tables (`spec_image`). -/
def specImage (t : image_v.Tables) : List (BitVec 8) :=
  encU32 (BitVec.ofNat 32 t.tasks.val.length) ++ encU32 (BitVec.ofNat 32 t.resources.val.length) ++
    encU32 (BitVec.ofNat 32 t.partitions.val.length) ++ encTasks t.tasks.val ++
    encResources t.resources.val ++ encRows t.access.val ++ encPartitions t.partitions.val ++
    encU64 t.operating_duration.bv ++ bvs t.slot_release.val ++ bvs t.slot_deadline.val ++
    bvs t.slot_budget.val ++ [t.slot_count.bv]

/-- Every table has the length its count says (`wf`). -/
def Wf (t : image_v.Tables) : Prop :=
  t.tasks.val.length ≤ 255 ∧ t.resources.val.length ≤ 255 ∧ t.partitions.val.length ≤ 255 ∧
  t.access.val.length = t.tasks.val.length ∧
  (∀ row ∈ t.access.val, row.val.length = t.resources.val.length) ∧
  t.slot_release.val.length = t.tasks.val.length ∧
  t.slot_deadline.val.length = t.tasks.val.length ∧
  t.slot_budget.val.length = t.tasks.val.length

/-- One Task entry (`rd_task`'s contract). -/
theorem rd_task_spec (bytes : Slice U8) (pos : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_task bytes pos ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok v => Reads bytes pos r.2 (encTask v)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_task
  step with (rd_u8_spec bytes pos hp) as ⟨ o, p1, h1, r1 ⟩
  rcases o with _ | a0
  · rd_none h1
  dsimp only at r1 ⊢
  step with (rd_u8_spec bytes p1 h1) as ⟨ o, p2, h2, r2 ⟩
  rcases o with _ | a1
  · rd_none h2
  dsimp only at r2 ⊢
  step with (rd_u8_spec bytes p2 h2) as ⟨ o, p3, h3, r3 ⟩
  rcases o with _ | a2
  · rd_none h3
  dsimp only at r3 ⊢
  step with (rd_u64_spec bytes p3 h3) as ⟨ o, p4, h4, r4 ⟩
  rcases o with _ | a3
  · rd_none h4
  dsimp only at r4 ⊢
  step with (rd_u64_spec bytes p4 h4) as ⟨ o, p5, h5, r5 ⟩
  rcases o with _ | a4
  · rd_none h5
  dsimp only at r5 ⊢
  step with (rd_u64_spec bytes p5 h5) as ⟨ o, p6, h6, r6 ⟩
  rcases o with _ | a5
  · rd_none h6
  dsimp only at r6 ⊢
  step with (rd_u32_spec bytes p6 h6) as ⟨ o, p7, h7, r7 ⟩
  rcases o with _ | a6
  · rd_none h7
  dsimp only at r7 ⊢
  step with (rd_u8_spec bytes p7 h7) as ⟨ o, p8, h8, r8 ⟩
  rcases o with _ | a7
  · rd_none h8
  dsimp only at r8 ⊢
  step with (rd_flag_spec bytes p8 h8) as ⟨ o, p9, h9, r9 ⟩
  rcases o with a8 | e
  swap
  · rd_none h9
  dsimp only at r9 ⊢
  simp only [WP.spec_ok]
  refine ⟨h9, ?_⟩
  exact ((((((((r1.trans r2).trans r3).trans r4).trans r5).trans r6).trans r7).trans r8).trans r9).cast (by simp [encTask])

/-- One Partition entry (`rd_partition`'s contract). -/
theorem rd_partition_spec (bytes : Slice U8) (pos : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_partition bytes pos ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some v => Reads bytes pos r.2 (encPartition v)
      | none => True ⦄ := by
  unfold image_v.rd_partition
  step with (rd_u8_spec bytes pos hp) as ⟨ o, p1, h1, r1 ⟩
  rcases o with _ | a0
  · rd_none h1
  dsimp only at r1 ⊢
  step with (rd_u8_spec bytes p1 h1) as ⟨ o, p2, h2, r2 ⟩
  rcases o with _ | a1
  · rd_none h2
  dsimp only at r2 ⊢
  step with (rd_u8_spec bytes p2 h2) as ⟨ o, p3, h3, r3 ⟩
  rcases o with _ | a2
  · rd_none h3
  dsimp only at r3 ⊢
  step with (rd_u8_spec bytes p3 h3) as ⟨ o, p4, h4, r4 ⟩
  rcases o with _ | a3
  · rd_none h4
  dsimp only at r4 ⊢
  step with (rd_u32_spec bytes p4 h4) as ⟨ o, p5, h5, r5 ⟩
  rcases o with _ | a4
  · rd_none h5
  dsimp only at r5 ⊢
  simp only [WP.spec_ok]
  refine ⟨h5, ?_⟩
  exact ((((r1.trans r2).trans r3).trans r4).trans r5).cast (by simp [encPartition])

theorem rd_bytes_loop_spec (bytes : Slice U8) (pos0 : Usize) (n : Usize) (pos : Usize)
    (out : alloc.vec.Vec (U8)) (i : Usize) (hp : pos.val ≤ bytes.val.length)
    (hi : i.val ≤ n.val) (hlen : out.val.length = i.val) (hq : ∀ x ∈ out.val, (fun _ => True) x)
    (hr : Reads bytes pos0 pos ((bvs) out.val)) :
    image_v.rd_bytes_loop bytes pos n out i ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos0 r.2 ((bvs) o.val)
      | none => True ⦄ := by
  unfold image_v.rd_bytes_loop
  apply loop.spec_decr_nat
    (fun (s : Usize × alloc.vec.Vec (U8) × Usize) => n.val - s.2.2.val)
    (fun s => s.1.val ≤ bytes.val.length ∧ s.2.2.val ≤ n.val ∧ s.2.1.val.length = s.2.2.val ∧
      (∀ x ∈ s.2.1.val, (fun _ => True) x) ∧ Reads bytes pos0 s.1 ((bvs) s.2.1.val))
  · intro s hs
    obtain ⟨p, o, j⟩ := s
    obtain ⟨hp', hj, hl, hq', hrd⟩ := hs
    dsimp only at hp' hj hl hq' hrd ⊢
    unfold image_v.rd_bytes_loop.body
    split
    · rename_i hlt
      step with (rd_u8_spec bytes p hp') as ⟨ ob, pE, hE, rd ⟩
      rcases ob with _ | v
      · rd_none hE
      dsimp only at rd ⊢
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨hE, by scalar_tac, by rw [hout1]; simp; omega, ?_,
        (hrd.trans rd).cast (by rw [hout1]; simp [bvs]), by scalar_tac⟩
      intro x hx
      rw [hout1] at hx
      rcases List.mem_append.1 hx with hx | hx
      · exact hq' x hx
      · have hxv := List.mem_singleton.1 hx
        subst hxv
        exact trivial
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨hp', by scalar_tac, hq', hrd⟩
  · exact ⟨hp, hi, hlen, hq, hr⟩

/-- `n` bytes as they are (`rd_bytes`'s contract). -/
theorem rd_bytes_spec (bytes : Slice U8) (pos : Usize) (n : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_bytes bytes pos n ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos r.2 ((bvs) o.val)
      | none => True ⦄ := by
  unfold image_v.rd_bytes
  exact rd_bytes_loop_spec bytes pos n pos _ 0#usize hp (by scalar_tac) (by simp)
    (by simp) ((Reads.refl hp).cast (by simp [bvs]))

theorem rd_row_loop_spec (bytes : Slice U8) (pos0 : Usize) (n : Usize) (pos : Usize)
    (out : alloc.vec.Vec (Bool)) (i : Usize) (hp : pos.val ≤ bytes.val.length)
    (hi : i.val ≤ n.val) (hlen : out.val.length = i.val) (hq : ∀ x ∈ out.val, (fun _ => True) x)
    (hr : Reads bytes pos0 pos ((encFlags) out.val)) :
    image_v.rd_row_loop bytes pos n out i ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos0 r.2 ((encFlags) o.val)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_row_loop
  apply loop.spec_decr_nat
    (fun (s : Usize × alloc.vec.Vec (Bool) × Usize) => n.val - s.2.2.val)
    (fun s => s.1.val ≤ bytes.val.length ∧ s.2.2.val ≤ n.val ∧ s.2.1.val.length = s.2.2.val ∧
      (∀ x ∈ s.2.1.val, (fun _ => True) x) ∧ Reads bytes pos0 s.1 ((encFlags) s.2.1.val))
  · intro s hs
    obtain ⟨p, o, j⟩ := s
    obtain ⟨hp', hj, hl, hq', hrd⟩ := hs
    dsimp only at hp' hj hl hq' hrd ⊢
    unfold image_v.rd_row_loop.body
    split
    · rename_i hlt
      step with (rd_flag_spec bytes p hp') as ⟨ ob, pE, hE, rd ⟩
      rcases ob with v | e
      swap
      · rd_none hE
      dsimp only at rd ⊢
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨hE, by scalar_tac, by rw [hout1]; simp; omega, ?_,
        (hrd.trans rd).cast (by rw [hout1]; simp [encFlags]), by scalar_tac⟩
      intro x hx
      rw [hout1] at hx
      rcases List.mem_append.1 hx with hx | hx
      · exact hq' x hx
      · have hxv := List.mem_singleton.1 hx
        subst hxv
        exact trivial
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨hp', by scalar_tac, hq', hrd⟩
  · exact ⟨hp, hi, hlen, hq, hr⟩

/-- One row of `n` flags (`rd_row`'s contract). -/
theorem rd_row_spec (bytes : Slice U8) (pos : Usize) (n : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_row bytes pos n ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos r.2 ((encFlags) o.val)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_row
  exact rd_row_loop_spec bytes pos n pos _ 0#usize hp (by scalar_tac) (by simp)
    (by simp) ((Reads.refl hp).cast (by simp [encFlags]))

theorem rd_tasks_loop_spec (bytes : Slice U8) (pos0 : Usize) (n : Usize) (pos : Usize)
    (out : alloc.vec.Vec (image_v.TaskCfg)) (i : Usize) (hp : pos.val ≤ bytes.val.length)
    (hi : i.val ≤ n.val) (hlen : out.val.length = i.val) (hq : ∀ x ∈ out.val, (fun _ => True) x)
    (hr : Reads bytes pos0 pos ((encTasks) out.val)) :
    image_v.rd_tasks_loop bytes pos n out i ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos0 r.2 ((encTasks) o.val)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_tasks_loop
  apply loop.spec_decr_nat
    (fun (s : Usize × alloc.vec.Vec (image_v.TaskCfg) × Usize) => n.val - s.2.2.val)
    (fun s => s.1.val ≤ bytes.val.length ∧ s.2.2.val ≤ n.val ∧ s.2.1.val.length = s.2.2.val ∧
      (∀ x ∈ s.2.1.val, (fun _ => True) x) ∧ Reads bytes pos0 s.1 ((encTasks) s.2.1.val))
  · intro s hs
    obtain ⟨p, o, j⟩ := s
    obtain ⟨hp', hj, hl, hq', hrd⟩ := hs
    dsimp only at hp' hj hl hq' hrd ⊢
    unfold image_v.rd_tasks_loop.body
    split
    · rename_i hlt
      step with (rd_task_spec bytes p hp') as ⟨ ob, pE, hE, rd ⟩
      rcases ob with v | e
      swap
      · rd_none hE
      dsimp only at rd ⊢
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨hE, by scalar_tac, by rw [hout1]; simp; omega, ?_,
        (hrd.trans rd).cast (by rw [hout1]; simp [encTasks]), by scalar_tac⟩
      intro x hx
      rw [hout1] at hx
      rcases List.mem_append.1 hx with hx | hx
      · exact hq' x hx
      · have hxv := List.mem_singleton.1 hx
        subst hxv
        exact trivial
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨hp', by scalar_tac, hq', hrd⟩
  · exact ⟨hp, hi, hlen, hq, hr⟩

/-- `n` Task entries (`rd_tasks`'s contract). -/
theorem rd_tasks_spec (bytes : Slice U8) (pos : Usize) (n : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_tasks bytes pos n ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos r.2 ((encTasks) o.val)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_tasks
  exact rd_tasks_loop_spec bytes pos n pos _ 0#usize hp (by scalar_tac) (by simp)
    (by simp) ((Reads.refl hp).cast (by simp [encTasks]))

theorem rd_resources_loop_spec (bytes : Slice U8) (pos0 : Usize) (n : Usize) (pos : Usize)
    (out : alloc.vec.Vec (image_v.ResourceCfg)) (i : Usize) (hp : pos.val ≤ bytes.val.length)
    (hi : i.val ≤ n.val) (hlen : out.val.length = i.val) (hq : ∀ x ∈ out.val, (fun _ => True) x)
    (hr : Reads bytes pos0 pos ((encResources) out.val)) :
    image_v.rd_resources_loop bytes pos n out i ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos0 r.2 ((encResources) o.val)
      | none => True ⦄ := by
  unfold image_v.rd_resources_loop
  apply loop.spec_decr_nat
    (fun (s : Usize × alloc.vec.Vec (image_v.ResourceCfg) × Usize) => n.val - s.2.2.val)
    (fun s => s.1.val ≤ bytes.val.length ∧ s.2.2.val ≤ n.val ∧ s.2.1.val.length = s.2.2.val ∧
      (∀ x ∈ s.2.1.val, (fun _ => True) x) ∧ Reads bytes pos0 s.1 ((encResources) s.2.1.val))
  · intro s hs
    obtain ⟨p, o, j⟩ := s
    obtain ⟨hp', hj, hl, hq', hrd⟩ := hs
    dsimp only at hp' hj hl hq' hrd ⊢
    unfold image_v.rd_resources_loop.body
    split
    · rename_i hlt
      step with (rd_u8_spec bytes p hp') as ⟨ ob, pA, hA, rA ⟩
      rcases ob with _ | c
      · rd_none hA
      dsimp only at rA ⊢
      step with (rd_u8_spec bytes pA hA) as ⟨ ob, pE, hE, rB ⟩
      rcases ob with _ | q
      · rd_none hE
      dsimp only at rB ⊢
      have rd := rA.trans rB
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨hE, by scalar_tac, by rw [hout1]; simp; omega, ?_,
        (hrd.trans rd).cast (by rw [hout1]; simp [encResources, encResource]), by scalar_tac⟩
      intro x hx
      rw [hout1] at hx
      rcases List.mem_append.1 hx with hx | hx
      · exact hq' x hx
      · have hxv := List.mem_singleton.1 hx
        subst hxv
        exact trivial
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨hp', by scalar_tac, hq', hrd⟩
  · exact ⟨hp, hi, hlen, hq, hr⟩

/-- `n` Resource entries (`rd_resources`'s contract). -/
theorem rd_resources_spec (bytes : Slice U8) (pos : Usize) (n : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_resources bytes pos n ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos r.2 ((encResources) o.val)
      | none => True ⦄ := by
  unfold image_v.rd_resources
  exact rd_resources_loop_spec bytes pos n pos _ 0#usize hp (by scalar_tac) (by simp)
    (by simp) ((Reads.refl hp).cast (by simp [encResources, encResource]))

theorem rd_rows_loop_spec (bytes : Slice U8) (pos0 : Usize) (n : Usize) (m : Usize) (pos : Usize)
    (out : alloc.vec.Vec (alloc.vec.Vec Bool)) (i : Usize) (hp : pos.val ≤ bytes.val.length)
    (hi : i.val ≤ n.val) (hlen : out.val.length = i.val) (hq : ∀ x ∈ out.val, (fun row => row.val.length = m.val) x)
    (hr : Reads bytes pos0 pos ((encRows) out.val)) :
    image_v.rd_rows_loop bytes pos n m out i ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun row => row.val.length = m.val) x) ∧
          Reads bytes pos0 r.2 ((encRows) o.val)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_rows_loop
  apply loop.spec_decr_nat
    (fun (s : Usize × alloc.vec.Vec (alloc.vec.Vec Bool) × Usize) => n.val - s.2.2.val)
    (fun s => s.1.val ≤ bytes.val.length ∧ s.2.2.val ≤ n.val ∧ s.2.1.val.length = s.2.2.val ∧
      (∀ x ∈ s.2.1.val, (fun row => row.val.length = m.val) x) ∧ Reads bytes pos0 s.1 ((encRows) s.2.1.val))
  · intro s hs
    obtain ⟨p, o, j⟩ := s
    obtain ⟨hp', hj, hl, hq', hrd⟩ := hs
    dsimp only at hp' hj hl hq' hrd ⊢
    unfold image_v.rd_rows_loop.body
    split
    · rename_i hlt
      step with (rd_row_spec bytes p m hp') as ⟨ ob, pE, hE, rd0 ⟩
      rcases ob with v | e
      swap
      · rd_none hE
      dsimp only at rd0 ⊢
      obtain ⟨hvl, _, rd⟩ := rd0
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨hE, by scalar_tac, by rw [hout1]; simp; omega, ?_,
        (hrd.trans rd).cast (by rw [hout1]; simp [encRows]), by scalar_tac⟩
      intro x hx
      rw [hout1] at hx
      rcases List.mem_append.1 hx with hx | hx
      · exact hq' x hx
      · have hxv := List.mem_singleton.1 hx
        subst hxv
        exact hvl
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨hp', by scalar_tac, hq', hrd⟩
  · exact ⟨hp, hi, hlen, hq, hr⟩

/-- `n` rows of `m` flags (`rd_rows`'s contract). -/
theorem rd_rows_spec (bytes : Slice U8) (pos : Usize) (n : Usize) (m : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_rows bytes pos n m ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun row => row.val.length = m.val) x) ∧
          Reads bytes pos r.2 ((encRows) o.val)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_rows
  exact rd_rows_loop_spec bytes pos n m pos _ 0#usize hp (by scalar_tac) (by simp)
    (by simp) ((Reads.refl hp).cast (by simp [encRows]))

theorem rd_partitions_loop_spec (bytes : Slice U8) (pos0 : Usize) (n : Usize) (pos : Usize)
    (out : alloc.vec.Vec (image_v.PartitionCfg)) (i : Usize) (hp : pos.val ≤ bytes.val.length)
    (hi : i.val ≤ n.val) (hlen : out.val.length = i.val) (hq : ∀ x ∈ out.val, (fun _ => True) x)
    (hr : Reads bytes pos0 pos ((encPartitions) out.val)) :
    image_v.rd_partitions_loop bytes pos n out i ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos0 r.2 ((encPartitions) o.val)
      | none => True ⦄ := by
  unfold image_v.rd_partitions_loop
  apply loop.spec_decr_nat
    (fun (s : Usize × alloc.vec.Vec (image_v.PartitionCfg) × Usize) => n.val - s.2.2.val)
    (fun s => s.1.val ≤ bytes.val.length ∧ s.2.2.val ≤ n.val ∧ s.2.1.val.length = s.2.2.val ∧
      (∀ x ∈ s.2.1.val, (fun _ => True) x) ∧ Reads bytes pos0 s.1 ((encPartitions) s.2.1.val))
  · intro s hs
    obtain ⟨p, o, j⟩ := s
    obtain ⟨hp', hj, hl, hq', hrd⟩ := hs
    dsimp only at hp' hj hl hq' hrd ⊢
    unfold image_v.rd_partitions_loop.body
    split
    · rename_i hlt
      step with (rd_partition_spec bytes p hp') as ⟨ ob, pE, hE, rd ⟩
      rcases ob with _ | v
      · rd_none hE
      dsimp only at rd ⊢
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨hE, by scalar_tac, by rw [hout1]; simp; omega, ?_,
        (hrd.trans rd).cast (by rw [hout1]; simp [encPartitions]), by scalar_tac⟩
      intro x hx
      rw [hout1] at hx
      rcases List.mem_append.1 hx with hx | hx
      · exact hq' x hx
      · have hxv := List.mem_singleton.1 hx
        subst hxv
        exact trivial
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨hp', by scalar_tac, hq', hrd⟩
  · exact ⟨hp, hi, hlen, hq, hr⟩

/-- `n` Partition entries (`rd_partitions`'s contract). -/
theorem rd_partitions_spec (bytes : Slice U8) (pos : Usize) (n : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_partitions bytes pos n ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos r.2 ((encPartitions) o.val)
      | none => True ⦄ := by
  unfold image_v.rd_partitions
  exact rd_partitions_loop_spec bytes pos n pos _ 0#usize hp (by scalar_tac) (by simp)
    (by simp) ((Reads.refl hp).cast (by simp [encPartitions]))

/-- Closes a branch in which a decoder refuses the image. -/
macro "dc_fail" : tactic =>
  `(tactic| first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; trivial) | trivial)

theorem u32_bv_val (v : U32) : BitVec.ofNat 32 v.val = v.bv := by
  apply BitVec.eq_of_toNat_eq
  simp only [BitVec.toNat_ofNat]
  have := v.hBounds
  exact Nat.mod_eq_of_lt (by scalar_tac)

theorem count_bv {c : U32} {n : Usize} (hn : n = UScalar.cast .Usize c) {len : Nat}
    (hl : len = n.val) : BitVec.ofNat 32 len = c.bv := by
  rw [← u32_bv_val]
  congr 1
  rw [hl, hn]
  scalar_tac

/-- **The `CONFIG` image decoder** (`decode_tables`'s contract, R26.4): it
returns for every input, and what it accepts is exactly the encoding of
what it returns. -/
theorem decode_tables_spec (bytes : Slice U8) :
    image_v.decode_tables bytes ⦃ r =>
      match r with
      | .Ok t => Wf t ∧ bvs bytes.val = specImage t
      | .Err _ => True ⦄ := by
  unfold image_v.decode_tables
  have h0 : (0#usize).val ≤ bytes.val.length := by scalar_tac
  step with (rd_u32_spec bytes 0#usize h0) as ⟨ o, p1, h1, r1 ⟩
  rcases o with _ | c0
  · dc_fail
  dsimp only at r1 ⊢
  step with (rd_u32_spec bytes p1 h1) as ⟨ o, p2, h2, r2 ⟩
  rcases o with _ | c1
  · dc_fail
  dsimp only at r2 ⊢
  step with (rd_u32_spec bytes p2 h2) as ⟨ o, p3, h3, r3 ⟩
  rcases o with _ | c2
  · dc_fail
  dsimp only at r3 ⊢
  split
  · dc_fail
  split
  · dc_fail
  split
  · dc_fail
  rename_i hc0 hc1 hc2
  step as ⟨ nt, hnt ⟩
  step as ⟨ nr, hnr ⟩
  step as ⟨ np, hnp ⟩
  step with (rd_tasks_spec bytes p3 nt h3) as ⟨ o, p4, h4, r4 ⟩
  rcases o with tasks | e
  swap
  · dc_fail
  dsimp only at r4 ⊢
  obtain ⟨l4, _, r4⟩ := r4
  step with (rd_resources_spec bytes p4 nr h4) as ⟨ o, p5, h5, r5 ⟩
  rcases o with _ | resources
  · dc_fail
  dsimp only at r5 ⊢
  obtain ⟨l5, _, r5⟩ := r5
  step with (rd_rows_spec bytes p5 nt nr h5) as ⟨ o, p6, h6, r6 ⟩
  rcases o with access | e
  swap
  · dc_fail
  dsimp only at r6 ⊢
  obtain ⟨l6, q6, r6⟩ := r6
  step with (rd_partitions_spec bytes p6 np h6) as ⟨ o, p7, h7, r7 ⟩
  rcases o with _ | partitions
  · dc_fail
  dsimp only at r7 ⊢
  obtain ⟨l7, _, r7⟩ := r7
  step with (rd_u64_spec bytes p7 h7) as ⟨ o, p8, h8, r8 ⟩
  rcases o with _ | od
  · dc_fail
  dsimp only at r8 ⊢
  step with (rd_bytes_spec bytes p8 nt h8) as ⟨ o, p9, h9, r9 ⟩
  rcases o with _ | sr
  · dc_fail
  dsimp only at r9 ⊢
  obtain ⟨l9, _, r9⟩ := r9
  step with (rd_bytes_spec bytes p9 nt h9) as ⟨ o, p10, h10, r10 ⟩
  rcases o with _ | sd
  · dc_fail
  dsimp only at r10 ⊢
  obtain ⟨l10, _, r10⟩ := r10
  step with (rd_bytes_spec bytes p10 nt h10) as ⟨ o, p11, h11, r11 ⟩
  rcases o with _ | sb
  · dc_fail
  dsimp only at r11 ⊢
  obtain ⟨l11, _, r11⟩ := r11
  step with (rd_u8_spec bytes p11 h11) as ⟨ o, p12, h12, r12 ⟩
  rcases o with _ | sc
  · dc_fail
  dsimp only at r12 ⊢
  simp only [Slice.len]
  split
  · step as ⟨ i2, hi2 ⟩
  · rename_i hend
    simp only [WP.spec_ok]
    have hpl : p12.val = bytes.val.length := by
      simp only [bne_iff_ne, ne_eq, not_not] at hend
      rw [hend]
      scalar_tac
    have hnt' : nt.val ≤ 255 := by rw [hnt]; scalar_tac
    have hnr' : nr.val ≤ 255 := by rw [hnr]; scalar_tac
    have hnp' : np.val ≤ 255 := by rw [hnp]; scalar_tac
    unfold Wf specImage
    dsimp only
    refine ⟨⟨by omega, by omega, by omega, by omega, ?_, by omega, by omega, by omega⟩, ?_⟩
    · intro row hrow; rw [q6 row hrow, l5]
    · have all := ((((((((((r1.trans r2).trans r3).trans r4).trans r5).trans r6).trans r7).trans
        r8).trans r9).trans r10).trans r11).trans r12
      have e := all.2
      have z : (0#usize : Usize).val = 0 := rfl
      rw [hpl, List.take_length, z] at e
      simp only [List.take_zero, bvs, List.map_nil, List.nil_append] at e
      rw [count_bv hnt l4, count_bv hnr l5, count_bv hnp l7]
      simp only [bvs]
      rw [e]

/-! ## The `LAYOUT` image -/

/-- `enc_interval`. -/
def encInterval (i : image_v.IntervalImg) : List (BitVec 8) :=
  encU32 i.base.bv ++ [i.size_log2.bv, i.srd.bv]

/-- `enc_kernel_window`. -/
def encKWindow (w : U32 × U8) : List (BitVec 8) := encU32 w.1.bv ++ [w.2.bv]

def encKWindows (l : List (U32 × U8)) : List (BitVec 8) := l.flatMap encKWindow

/-- `enc_window`. -/
def encWindow (w : U32 × U8 × Bool) : List (BitVec 8) :=
  encU32 w.1.bv ++ [w.2.1.bv] ++ encFlag w.2.2

def encWindows (l : List (U32 × U8 × Bool)) : List (BitVec 8) := l.flatMap encWindow

/-- `enc_partition_layout`. -/
def encPartLayout (p : image_v.PartitionLayoutImg) : List (BitVec 8) :=
  encInterval p.code ++ encInterval p.ram ++ encU32 p.stack_size.bv ++ [p.window_count.bv] ++
    encWindows p.windows.val

def encPartLayouts (l : List image_v.PartitionLayoutImg) : List (BitVec 8) :=
  l.flatMap encPartLayout

/-- The `LAYOUT` image (`spec_layout_image`). -/
def specLayoutImage (l : image_v.LayoutImg) : List (BitVec 8) :=
  encInterval l.kernel_flash ++ encInterval l.shared_code ++ encInterval l.kernel_ram ++
    [l.kernel_window_count.bv] ++ encKWindows l.kernel_windows.val ++
    encU32 (BitVec.ofNat 32 l.partitions.val.length) ++ encPartLayouts l.partitions.val

/-- Every window table has all its slots, and the count fits a byte
(`layout_wf`). -/
def LayoutWf (l : image_v.LayoutImg) : Prop :=
  l.kernel_windows.val.length = image_v.MAX_KERNEL_WINDOWS.val ∧ l.partitions.val.length ≤ 255 ∧
  ∀ p ∈ l.partitions.val, p.windows.val.length = image_v.MAX_WINDOWS.val

/-- One interval (`rd_interval`'s contract). -/
theorem rd_interval_spec (bytes : Slice U8) (pos : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_interval bytes pos ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some v => Reads bytes pos r.2 (encInterval v)
      | none => True ⦄ := by
  unfold image_v.rd_interval
  step with (rd_u32_spec bytes pos hp) as ⟨ o, p1, h1, r1 ⟩
  rcases o with _ | a0
  · rd_none h1
  dsimp only at r1 ⊢
  step with (rd_u8_spec bytes p1 h1) as ⟨ o, p2, h2, r2 ⟩
  rcases o with _ | a1
  · rd_none h2
  dsimp only at r2 ⊢
  step with (rd_u8_spec bytes p2 h2) as ⟨ o, p3, h3, r3 ⟩
  rcases o with _ | a2
  · rd_none h3
  dsimp only at r3 ⊢
  simp only [WP.spec_ok]
  refine ⟨h3, ?_⟩
  exact ((r1.trans r2).trans r3).cast (by simp [encInterval])

theorem rd_windows_loop_spec (bytes : Slice U8) (pos0 : Usize) (pos : Usize)
    (out : alloc.vec.Vec (U32 × U8 × Bool)) (i : Usize) (hp : pos.val ≤ bytes.val.length)
    (hi : i.val ≤ image_v.MAX_WINDOWS.val) (hlen : out.val.length = i.val) (hq : ∀ x ∈ out.val, (fun _ => True) x)
    (hr : Reads bytes pos0 pos ((encWindows) out.val)) :
    image_v.rd_windows_loop bytes pos out i ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok o => o.val.length = image_v.MAX_WINDOWS.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos0 r.2 ((encWindows) o.val)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_windows_loop
  apply loop.spec_decr_nat
    (fun (s : Usize × alloc.vec.Vec (U32 × U8 × Bool) × Usize) => image_v.MAX_WINDOWS.val - s.2.2.val)
    (fun s => s.1.val ≤ bytes.val.length ∧ s.2.2.val ≤ image_v.MAX_WINDOWS.val ∧ s.2.1.val.length = s.2.2.val ∧
      (∀ x ∈ s.2.1.val, (fun _ => True) x) ∧ Reads bytes pos0 s.1 ((encWindows) s.2.1.val))
  · intro s hs
    obtain ⟨p, o, j⟩ := s
    obtain ⟨hp', hj, hl, hq', hrd⟩ := hs
    dsimp only at hp' hj hl hq' hrd ⊢
    unfold image_v.rd_windows_loop.body
    split
    · rename_i hlt
      step with (rd_u32_spec bytes p hp') as ⟨ ob, pA, hA, rA ⟩
      rcases ob with _ | w0
      · rd_none hA
      dsimp only at rA ⊢
      step with (rd_u8_spec bytes pA hA) as ⟨ ob, pB, hB, rB ⟩
      rcases ob with _ | w1
      · rd_none hB
      dsimp only at rB ⊢
      step with (rd_flag_spec bytes pB hB) as ⟨ ob, pE, hE, rC ⟩
      rcases ob with w2 | e
      swap
      · rd_none hE
      dsimp only at rC ⊢
      have rd := (rA.trans rB).trans rC
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨hE, by scalar_tac, by rw [hout1]; simp; omega, ?_,
        (hrd.trans rd).cast (by rw [hout1]; simp [encWindows, encWindow]), by scalar_tac⟩
      intro x hx
      rw [hout1] at hx
      rcases List.mem_append.1 hx with hx | hx
      · exact hq' x hx
      · have hxv := List.mem_singleton.1 hx
        subst hxv
        exact trivial
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨hp', by scalar_tac, hq', hrd⟩
  · exact ⟨hp, hi, hlen, hq, hr⟩

/-- The window slots of a Partition's layout (`rd_windows`'s contract). -/
theorem rd_windows_spec (bytes : Slice U8) (pos : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_windows bytes pos ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok o => o.val.length = image_v.MAX_WINDOWS.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos r.2 ((encWindows) o.val)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_windows
  exact rd_windows_loop_spec bytes pos pos _ 0#usize hp (by scalar_tac) (by simp)
    (by simp) ((Reads.refl hp).cast (by simp [encWindows, encWindow]))

/-- One Partition's layout (`rd_partition_layout`'s contract). -/
theorem rd_partition_layout_spec (bytes : Slice U8) (pos : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_partition_layout bytes pos ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok v => v.windows.val.length = image_v.MAX_WINDOWS.val ∧ Reads bytes pos r.2 (encPartLayout v)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_partition_layout
  step with (rd_interval_spec bytes pos hp) as ⟨ o, p1, h1, r1 ⟩
  rcases o with _ | a0
  · rd_none h1
  dsimp only at r1 ⊢
  step with (rd_interval_spec bytes p1 h1) as ⟨ o, p2, h2, r2 ⟩
  rcases o with _ | a1
  · rd_none h2
  dsimp only at r2 ⊢
  step with (rd_u32_spec bytes p2 h2) as ⟨ o, p3, h3, r3 ⟩
  rcases o with _ | a2
  · rd_none h3
  dsimp only at r3 ⊢
  step with (rd_u8_spec bytes p3 h3) as ⟨ o, p4, h4, r4 ⟩
  rcases o with _ | a3
  · rd_none h4
  dsimp only at r4 ⊢
  step with (rd_windows_spec bytes p4 h4) as ⟨ o, p5, h5, r5 ⟩
  rcases o with a4 | e
  swap
  · rd_none h5
  dsimp only at r5 ⊢
  obtain ⟨l5, q5, r5⟩ := r5
  simp only [WP.spec_ok]
  refine ⟨h5, l5, ?_⟩
  exact ((((r1.trans r2).trans r3).trans r4).trans r5).cast (by simp [encPartLayout])

theorem rd_kernel_windows_loop_spec (bytes : Slice U8) (pos0 : Usize) (pos : Usize)
    (out : alloc.vec.Vec (U32 × U8)) (i : Usize) (hp : pos.val ≤ bytes.val.length)
    (hi : i.val ≤ image_v.MAX_KERNEL_WINDOWS.val) (hlen : out.val.length = i.val) (hq : ∀ x ∈ out.val, (fun _ => True) x)
    (hr : Reads bytes pos0 pos ((encKWindows) out.val)) :
    image_v.rd_kernel_windows_loop bytes pos out i ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some o => o.val.length = image_v.MAX_KERNEL_WINDOWS.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos0 r.2 ((encKWindows) o.val)
      | none => True ⦄ := by
  unfold image_v.rd_kernel_windows_loop
  apply loop.spec_decr_nat
    (fun (s : Usize × alloc.vec.Vec (U32 × U8) × Usize) => image_v.MAX_KERNEL_WINDOWS.val - s.2.2.val)
    (fun s => s.1.val ≤ bytes.val.length ∧ s.2.2.val ≤ image_v.MAX_KERNEL_WINDOWS.val ∧ s.2.1.val.length = s.2.2.val ∧
      (∀ x ∈ s.2.1.val, (fun _ => True) x) ∧ Reads bytes pos0 s.1 ((encKWindows) s.2.1.val))
  · intro s hs
    obtain ⟨p, o, j⟩ := s
    obtain ⟨hp', hj, hl, hq', hrd⟩ := hs
    dsimp only at hp' hj hl hq' hrd ⊢
    unfold image_v.rd_kernel_windows_loop.body
    split
    · rename_i hlt
      step with (rd_u32_spec bytes p hp') as ⟨ ob, pA, hA, rA ⟩
      rcases ob with _ | w0
      · rd_none hA
      dsimp only at rA ⊢
      step with (rd_u8_spec bytes pA hA) as ⟨ ob, pE, hE, rB ⟩
      rcases ob with _ | w1
      · rd_none hE
      dsimp only at rB ⊢
      have rd := rA.trans rB
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨hE, by scalar_tac, by rw [hout1]; simp; omega, ?_,
        (hrd.trans rd).cast (by rw [hout1]; simp [encKWindows, encKWindow]), by scalar_tac⟩
      intro x hx
      rw [hout1] at hx
      rcases List.mem_append.1 hx with hx | hx
      · exact hq' x hx
      · have hxv := List.mem_singleton.1 hx
        subst hxv
        exact trivial
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨hp', by scalar_tac, hq', hrd⟩
  · exact ⟨hp, hi, hlen, hq, hr⟩

/-- The Kernel window slots (`rd_kernel_windows`'s contract). -/
theorem rd_kernel_windows_spec (bytes : Slice U8) (pos : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_kernel_windows bytes pos ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | some o => o.val.length = image_v.MAX_KERNEL_WINDOWS.val ∧ (∀ x ∈ o.val, (fun _ => True) x) ∧
          Reads bytes pos r.2 ((encKWindows) o.val)
      | none => True ⦄ := by
  unfold image_v.rd_kernel_windows
  exact rd_kernel_windows_loop_spec bytes pos pos _ 0#usize hp (by scalar_tac) (by simp)
    (by simp) ((Reads.refl hp).cast (by simp [encKWindows, encKWindow]))

theorem rd_partition_layouts_loop_spec (bytes : Slice U8) (pos0 : Usize) (n : Usize) (pos : Usize)
    (out : alloc.vec.Vec (image_v.PartitionLayoutImg)) (i : Usize) (hp : pos.val ≤ bytes.val.length)
    (hi : i.val ≤ n.val) (hlen : out.val.length = i.val) (hq : ∀ x ∈ out.val, (fun p => p.windows.val.length = image_v.MAX_WINDOWS.val) x)
    (hr : Reads bytes pos0 pos ((encPartLayouts) out.val)) :
    image_v.rd_partition_layouts_loop bytes pos n out i ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun p => p.windows.val.length = image_v.MAX_WINDOWS.val) x) ∧
          Reads bytes pos0 r.2 ((encPartLayouts) o.val)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_partition_layouts_loop
  apply loop.spec_decr_nat
    (fun (s : Usize × alloc.vec.Vec (image_v.PartitionLayoutImg) × Usize) => n.val - s.2.2.val)
    (fun s => s.1.val ≤ bytes.val.length ∧ s.2.2.val ≤ n.val ∧ s.2.1.val.length = s.2.2.val ∧
      (∀ x ∈ s.2.1.val, (fun p => p.windows.val.length = image_v.MAX_WINDOWS.val) x) ∧ Reads bytes pos0 s.1 ((encPartLayouts) s.2.1.val))
  · intro s hs
    obtain ⟨p, o, j⟩ := s
    obtain ⟨hp', hj, hl, hq', hrd⟩ := hs
    dsimp only at hp' hj hl hq' hrd ⊢
    unfold image_v.rd_partition_layouts_loop.body
    split
    · rename_i hlt
      step with (rd_partition_layout_spec bytes p hp') as ⟨ ob, pE, hE, rd0 ⟩
      rcases ob with v | e
      swap
      · rd_none hE
      dsimp only at rd0 ⊢
      obtain ⟨hvl, rd⟩ := rd0
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨hE, by scalar_tac, by rw [hout1]; simp; omega, ?_,
        (hrd.trans rd).cast (by rw [hout1]; simp [encPartLayouts]), by scalar_tac⟩
      intro x hx
      rw [hout1] at hx
      rcases List.mem_append.1 hx with hx | hx
      · exact hq' x hx
      · have hxv := List.mem_singleton.1 hx
        subst hxv
        exact hvl
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨hp', by scalar_tac, hq', hrd⟩
  · exact ⟨hp, hi, hlen, hq, hr⟩

/-- `n` Partition layouts (`rd_partition_layouts`'s contract). -/
theorem rd_partition_layouts_spec (bytes : Slice U8) (pos : Usize) (n : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_partition_layouts bytes pos n ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok o => o.val.length = n.val ∧ (∀ x ∈ o.val, (fun p => p.windows.val.length = image_v.MAX_WINDOWS.val) x) ∧
          Reads bytes pos r.2 ((encPartLayouts) o.val)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_partition_layouts
  exact rd_partition_layouts_loop_spec bytes pos n pos _ 0#usize hp (by scalar_tac) (by simp)
    (by simp) ((Reads.refl hp).cast (by simp [encPartLayouts]))

/-- **The `LAYOUT` image decoder** (`decode_layout_image`'s contract,
R26.4, R16.7): it returns for every input, and what it accepts is exactly
the encoding of what it returns. -/
theorem decode_layout_image_spec (bytes : Slice U8) :
    image_v.decode_layout_image bytes ⦃ r =>
      match r with
      | .Ok l => LayoutWf l ∧ bvs bytes.val = specLayoutImage l
      | .Err _ => True ⦄ := by
  unfold image_v.decode_layout_image
  have h0 : (0#usize).val ≤ bytes.val.length := by scalar_tac
  step with (rd_interval_spec bytes 0#usize h0) as ⟨ o, p1, h1, r1 ⟩
  rcases o with _ | kf
  · dc_fail
  dsimp only at r1 ⊢
  step with (rd_interval_spec bytes p1 h1) as ⟨ o, p2, h2, r2 ⟩
  rcases o with _ | sc
  · dc_fail
  dsimp only at r2 ⊢
  step with (rd_interval_spec bytes p2 h2) as ⟨ o, p3, h3, r3 ⟩
  rcases o with _ | kr
  · dc_fail
  dsimp only at r3 ⊢
  step with (rd_u8_spec bytes p3 h3) as ⟨ o, p4, h4, r4 ⟩
  rcases o with _ | kwc
  · dc_fail
  dsimp only at r4 ⊢
  step with (rd_kernel_windows_spec bytes p4 h4) as ⟨ o, p5, h5, r5 ⟩
  rcases o with _ | kw
  · dc_fail
  dsimp only at r5 ⊢
  obtain ⟨l5, _, r5⟩ := r5
  step with (rd_u32_spec bytes p5 h5) as ⟨ o, p6, h6, r6 ⟩
  rcases o with _ | c
  · dc_fail
  dsimp only at r6 ⊢
  split
  · dc_fail
  rename_i hc
  step as ⟨ np, hnp ⟩
  step with (rd_partition_layouts_spec bytes p6 np h6) as ⟨ o, p7, h7, r7 ⟩
  rcases o with parts | e
  swap
  · dc_fail
  dsimp only at r7 ⊢
  obtain ⟨l7, q7, r7⟩ := r7
  simp only [Slice.len]
  split
  · step as ⟨ i2, hi2 ⟩
  · rename_i hend
    simp only [WP.spec_ok]
    have hpl : p7.val = bytes.val.length := by
      simp only [bne_iff_ne, ne_eq, not_not] at hend
      rw [hend]
      scalar_tac
    have hnp' : np.val ≤ 255 := by rw [hnp]; scalar_tac
    unfold LayoutWf specLayoutImage
    dsimp only
    refine ⟨⟨l5, by omega, q7⟩, ?_⟩
    have all := (((((r1.trans r2).trans r3).trans r4).trans r5).trans r6).trans r7
    have e := all.2
    have z : (0#usize : Usize).val = 0 := rfl
    rw [hpl, List.take_length, z] at e
    simp only [List.take_zero, bvs, List.map_nil, List.nil_append] at e
    rw [count_bv hnp l7]
    simp only [bvs]
    rw [e]

/-! ## The Endpoint image -/

/-- `enc_u16`. -/
def encU16 (v : BitVec 16) : List (BitVec 8) := [v.setWidth 8, (v >>> 8).setWidth 8]

theorem le16_b0 (b0 b1 : BitVec 8) : (b0.setWidth 16 ||| (b1.setWidth 16 <<< 8)).setWidth 8 = b0 := by
  ext i hi'
  have h8 : i < 8 := hi'
  simp only [BitVec.getElem_setWidth, BitVec.getLsbD_or, BitVec.getLsbD_setWidth,
    BitVec.getLsbD_shiftLeft, BitVec.getLsbD_ushiftRight]
  have a1 : i < 16 := by omega
  simp [a1, h8, BitVec.getLsbD_eq_getElem h8]
  try (intro _; omega)

theorem le16_b1 (b0 b1 : BitVec 8) : ((b0.setWidth 16 ||| (b1.setWidth 16 <<< 8)) >>> 8).setWidth 8 = b1 := by
  ext i hi'
  have h8 : i < 8 := hi'
  simp only [BitVec.getElem_setWidth, BitVec.getLsbD_or, BitVec.getLsbD_setWidth,
    BitVec.getLsbD_shiftLeft, BitVec.getLsbD_ushiftRight]
  have a1 : 8 + i < 16 := by omega
  have a4 : ¬ (8 + i < 8) := by omega
  have a5 : 8 + i - 8 = i := by omega
  have a6 : b0.getLsbD (8 + i) = false := BitVec.getLsbD_of_ge b0 _ (by omega)
  simp [a1, a4, a5, a6, h8, BitVec.getLsbD_eq_getElem h8]
  try (intro _; omega)

theorem le16_bytes (b0 b1 : BitVec 8) :
    encU16 (b0.setWidth 16 ||| (b1.setWidth 16 <<< 8)) = [b0, b1] := by
  unfold encU16
  rw [le16_b0, le16_b1]

/-- `enc_endpoint`. -/
def encEndpoint (e : image_v.EndpointCfg) : List (BitVec 8) :=
  [e.capacity.bv] ++ encU16 e.payload.bv ++ [e.roles.bv] ++ bvs e.tasks.val ++
    bvs e.partitions.val ++ encFlags e.release.val ++ encU32 e.storage.bv ++
    encRows e.validates.val

def encEndpoints (l : List image_v.EndpointCfg) : List (BitVec 8) := l.flatMap encEndpoint

/-- Every table of an Endpoint entry has all its role slots (`endpoint_wf`). -/
def EndpointWf (e : image_v.EndpointCfg) : Prop :=
  e.tasks.val.length = image_v.MAX_ROLES.val ∧ e.partitions.val.length = image_v.MAX_ROLES.val ∧
  e.release.val.length = image_v.MAX_ROLES.val ∧ e.validates.val.length = image_v.MAX_ROLES.val ∧
  ∀ row ∈ e.validates.val, row.val.length = image_v.MAX_ROLES.val

/-- The Endpoint image: the count, the entries, the storage bytes
(`spec_endpoint_image`). -/
def specEndpointImage (es : List image_v.EndpointCfg) (storage_bytes : U32) : List (BitVec 8) :=
  encU32 (BitVec.ofNat 32 es.length) ++ encEndpoints es ++ encU32 storage_bytes.bv

/-- One Endpoint entry (`rd_endpoint`'s contract). -/
theorem rd_endpoint_spec (bytes : Slice U8) (pos : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_endpoint bytes pos ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok v => EndpointWf v ∧ Reads bytes pos r.2 (encEndpoint v)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_endpoint
  step with (rd_u8_spec bytes pos hp) as ⟨ o, p1, h1, r1 ⟩
  rcases o with _ | cap
  · rd_none h1
  dsimp only at r1 ⊢
  step with (rd_u8_spec bytes p1 h1) as ⟨ o, p2, h2, r2 ⟩
  rcases o with _ | b0
  · rd_none h2
  dsimp only at r2 ⊢
  step with (rd_u8_spec bytes p2 h2) as ⟨ o, p3, h3, r3 ⟩
  rcases o with _ | b1
  · rd_none h3
  dsimp only at r3 ⊢
  step as ⟨ i, hi ⟩
  step as ⟨ i1, hi1 ⟩
  step as ⟨ i2, hi2, hi2b ⟩
  step as ⟨ payload, hpay, hpayb ⟩
  have c0 : i.bv = b0.bv.setWidth 16 := by rw [hi]; rfl
  have c1 : i1.bv = b1.bv.setWidth 16 := by rw [hi1]; rfl
  have e2 : i2.bv = b1.bv.setWidth 16 <<< 8 :=
    hi2b.trans (congrArg (fun x : BitVec 16 => x <<< (8 : Nat)) c1)
  have ep := hpayb.trans (congrArg₂ (· ||| ·) c0 e2)
  have r23 : Reads bytes p1 p3 (encU16 payload.bv) :=
    (r2.trans r3).cast ((congrArg encU16 ep).trans (le16_bytes b0.bv b1.bv)).symm
  step with (rd_u8_spec bytes p3 h3) as ⟨ o, p4, h4, r4 ⟩
  rcases o with _ | roles
  · rd_none h4
  dsimp only at r4 ⊢
  step with (rd_bytes_spec bytes p4 image_v.MAX_ROLES h4) as ⟨ o, p5, h5, r5 ⟩
  rcases o with _ | tasks
  · rd_none h5
  dsimp only at r5 ⊢
  obtain ⟨l5, _, r5⟩ := r5
  step with (rd_bytes_spec bytes p5 image_v.MAX_ROLES h5) as ⟨ o, p6, h6, r6 ⟩
  rcases o with _ | parts
  · rd_none h6
  dsimp only at r6 ⊢
  obtain ⟨l6, _, r6⟩ := r6
  step with (rd_row_spec bytes p6 image_v.MAX_ROLES h6) as ⟨ o, p7, h7, r7 ⟩
  rcases o with release | e
  swap
  · rd_none h7
  dsimp only at r7 ⊢
  obtain ⟨l7, _, r7⟩ := r7
  step with (rd_u32_spec bytes p7 h7) as ⟨ o, p8, h8, r8 ⟩
  rcases o with _ | storage
  · rd_none h8
  dsimp only at r8 ⊢
  step with (rd_rows_spec bytes p8 image_v.MAX_ROLES image_v.MAX_ROLES h8) as ⟨ o, p9, h9, r9 ⟩
  rcases o with validates | e
  swap
  · rd_none h9
  dsimp only at r9 ⊢
  obtain ⟨l9, q9, r9⟩ := r9
  simp only [WP.spec_ok]
  unfold EndpointWf encEndpoint
  dsimp only
  refine ⟨h9, ⟨l5, l6, l7, l9, q9⟩, ?_⟩
  exact (((((((r1.trans r23).trans r4).trans r5).trans r6).trans r7).trans r8).trans r9).cast
    (by simp)

theorem rd_endpoints_loop_spec (bytes : Slice U8) (pos0 : Usize) (n : Usize) (pos : Usize)
    (out : alloc.vec.Vec (image_v.EndpointCfg)) (i : Usize) (hp : pos.val ≤ bytes.val.length)
    (hi : i.val ≤ n.val) (hlen : out.val.length = i.val) (hq : ∀ x ∈ out.val, (EndpointWf) x)
    (hr : Reads bytes pos0 pos ((encEndpoints) out.val)) :
    image_v.rd_endpoints_loop bytes pos n out i ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok o => o.val.length = n.val ∧ (∀ x ∈ o.val, (EndpointWf) x) ∧
          Reads bytes pos0 r.2 ((encEndpoints) o.val)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_endpoints_loop
  apply loop.spec_decr_nat
    (fun (s : Usize × alloc.vec.Vec (image_v.EndpointCfg) × Usize) => n.val - s.2.2.val)
    (fun s => s.1.val ≤ bytes.val.length ∧ s.2.2.val ≤ n.val ∧ s.2.1.val.length = s.2.2.val ∧
      (∀ x ∈ s.2.1.val, (EndpointWf) x) ∧ Reads bytes pos0 s.1 ((encEndpoints) s.2.1.val))
  · intro s hs
    obtain ⟨p, o, j⟩ := s
    obtain ⟨hp', hj, hl, hq', hrd⟩ := hs
    dsimp only at hp' hj hl hq' hrd ⊢
    unfold image_v.rd_endpoints_loop.body
    split
    · rename_i hlt
      step with (rd_endpoint_spec bytes p hp') as ⟨ ob, pE, hE, rd0 ⟩
      rcases ob with v | e
      swap
      · rd_none hE
      dsimp only at rd0 ⊢
      obtain ⟨hvl, rd⟩ := rd0
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨hE, by scalar_tac, by rw [hout1]; simp; omega, ?_,
        (hrd.trans rd).cast (by rw [hout1]; simp [encEndpoints]), by scalar_tac⟩
      intro x hx
      rw [hout1] at hx
      rcases List.mem_append.1 hx with hx | hx
      · exact hq' x hx
      · have hxv := List.mem_singleton.1 hx
        subst hxv
        exact hvl
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨hp', by scalar_tac, hq', hrd⟩
  · exact ⟨hp, hi, hlen, hq, hr⟩

/-- `n` Endpoint entries (`rd_endpoints`'s contract). -/
theorem rd_endpoints_spec (bytes : Slice U8) (pos : Usize) (n : Usize) (hp : pos.val ≤ bytes.val.length) :
    image_v.rd_endpoints bytes pos n ⦃ r => r.2.val ≤ bytes.val.length ∧
      match r.1 with
      | .Ok o => o.val.length = n.val ∧ (∀ x ∈ o.val, (EndpointWf) x) ∧
          Reads bytes pos r.2 ((encEndpoints) o.val)
      | .Err _ => True ⦄ := by
  unfold image_v.rd_endpoints
  exact rd_endpoints_loop_spec bytes pos n pos _ 0#usize hp (by scalar_tac) (by simp)
    (by simp) ((Reads.refl hp).cast (by simp [encEndpoints]))

/-- **The Endpoint image decoder** (`decode_endpoint_image`'s contract,
R26.4): it returns for every input, and what it accepts is exactly the
encoding of what it returns. -/
theorem decode_endpoint_image_spec (bytes : Slice U8) :
    image_v.decode_endpoint_image bytes ⦃ r =>
      match r with
      | .Ok (es, sb) => es.val.length ≤ 8 ∧ (∀ e ∈ es.val, EndpointWf e) ∧
          bvs bytes.val = specEndpointImage es.val sb
      | .Err _ => True ⦄ := by
  unfold image_v.decode_endpoint_image
  have h0 : (0#usize).val ≤ bytes.val.length := by scalar_tac
  step with (rd_u32_spec bytes 0#usize h0) as ⟨ o, p1, h1, r1 ⟩
  rcases o with _ | c
  · dc_fail
  dsimp only at r1 ⊢
  split
  · dc_fail
  rename_i hc
  step as ⟨ n, hn ⟩
  step with (rd_endpoints_spec bytes p1 n h1) as ⟨ o, p2, h2, r2 ⟩
  rcases o with es | e
  swap
  · dc_fail
  dsimp only at r2 ⊢
  obtain ⟨l2, q2, r2⟩ := r2
  step with (rd_u32_spec bytes p2 h2) as ⟨ o, p3, h3, r3 ⟩
  rcases o with _ | sb
  · dc_fail
  dsimp only at r3 ⊢
  simp only [Slice.len]
  split
  · step as ⟨ i2, hi2 ⟩
  · rename_i hend
    simp only [WP.spec_ok]
    have hpl : p3.val = bytes.val.length := by
      simp only [bne_iff_ne, ne_eq, not_not] at hend
      rw [hend]
      scalar_tac
    have hn' : n.val ≤ 8 := by rw [hn]; scalar_tac
    unfold specEndpointImage
    refine ⟨by omega, q2, ?_⟩
    have all := (r1.trans r2).trans r3
    have e := all.2
    have z : (0#usize : Usize).val = 0 := rfl
    rw [hpl, List.take_length, z] at e
    simp only [List.take_zero, bvs, List.map_nil, List.nil_append] at e
    rw [count_bv hn l2]
    simp only [bvs]
    rw [e]

/-! ## Storage placement (R16.1, R39.2) -/

instance : Inhabited image_v.IntervalImg := ⟨⟨0#u32, 0#u8, 0#u8⟩⟩
instance : Inhabited image_v.PartitionLayoutImg :=
  ⟨⟨default, default, 0#u32, 0#u8, alloc.vec.Vec.new _⟩⟩
instance : Inhabited image_v.EndpointCfg :=
  ⟨⟨0#u8, 0#u16, 0#u8, alloc.vec.Vec.new _, alloc.vec.Vec.new _, alloc.vec.Vec.new _, 0#u32,
    alloc.vec.Vec.new _⟩⟩

/-- One past the last byte of an interval of the `LAYOUT` image
(`interval_end`). -/
def intervalEnd (i : image_v.IntervalImg) : Nat := i.base.val + 2 ^ i.size_log2.val

/-- An Endpoint's slots lie in the Endpoint storage at a word-aligned
offset (`endpoint_in_storage`). -/
def EndpointInStorage (e : image_v.EndpointCfg) (total : Nat) : Prop :=
  e.storage.val % 4 = 0 ∧
  e.storage.val + (e.roles.val * e.roles.val) * e.capacity.val * e.payload.val ≤ total

/-- A Partition's RAM interval does not reach the last byte of the address
space and shares no byte with the storage (`ram_apart`). -/
def RamApart (ram : image_v.IntervalImg) (base total : Nat) : Prop :=
  intervalEnd ram ≤ 0xFFFF_FFFF ∧ (intervalEnd ram ≤ base ∨ base + total ≤ ram.base.val)

/-- Where the Endpoint storage of a binary lies (`storage_placed`). -/
def StoragePlaced (layout : image_v.LayoutImg) (endpoints : List image_v.EndpointCfg)
    (base total : Nat) : Prop :=
  base % 4 = 0 ∧ (∀ e, e < endpoints.length → EndpointInStorage endpoints[e]! total) ∧
  (∀ p, p < layout.partitions.val.length → RamApart (layout.partitions.val[p]!).ram base total)

/-- The storage lies inside the Kernel's RAM interval
(`storage_in_kernel_ram`). -/
def StorageInKernelRam (kernel_ram : image_v.IntervalImg) (base total : Nat) : Prop :=
  kernel_ram.base.val ≤ base ∧ base + total ≤ intervalEnd kernel_ram

theorem pow2_u64_spec (k : U8) (hk : k.val ≤ 32) :
    image_v.pow2_u64 k ⦃ r => r.val = 2 ^ k.val ⦄ := by
  unfold image_v.pow2_u64 image_v.pow2_u64_loop
  apply loop.spec_decr_nat (fun (s : U64 × U8) => k.val - s.2.val)
    (fun s => s.2.val ≤ k.val ∧ s.1.val = 2 ^ s.2.val)
  · intro s hs
    obtain ⟨r, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold image_v.pow2_u64_loop.body
    split
    · rename_i hlt
      have hi : i.val < 32 := by scalar_tac
      have hb : 2 ^ i.val ≤ 2 ^ 31 := Nat.pow_le_pow_right (by decide) (by omega)
      step as ⟨ r1, hr1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [hi1, hr1, h2, Nat.pow_succ]
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = k.val := by scalar_tac
      rw [h2, this]
  · exact ⟨by scalar_tac, by simp⟩

/-- `endpoint_in_storage_ok` decides `endpoint_in_storage` (Verus proves the
direction from `true`). -/
theorem endpoint_in_storage_ok_spec (e : image_v.EndpointCfg) (total : U32) :
    image_v.endpoint_in_storage_ok e total ⦃ b => b = true ↔ EndpointInStorage e total.val ⦄ := by
  unfold image_v.endpoint_in_storage_ok EndpointInStorage
  step as ⟨ n, hn ⟩
  step as ⟨ cap, hcap ⟩
  step as ⟨ pl, hpl ⟩
  have en : n.val = e.roles.val := by rw [hn]; scalar_tac
  have ecap : cap.val = e.capacity.val := by rw [hcap]; scalar_tac
  have epl : pl.val = e.payload.val := by rw [hpl]; scalar_tac
  have b1 : n.val * n.val ≤ 255 * 255 := Nat.mul_le_mul (by scalar_tac) (by scalar_tac)
  have b2 : n.val * n.val * cap.val ≤ 255 * 255 * 255 := Nat.mul_le_mul b1 (by scalar_tac)
  have b3 : n.val * n.val * cap.val * pl.val ≤ 255 * 255 * 255 * 65535 :=
    Nat.mul_le_mul b2 (by scalar_tac)
  step as ⟨ channels, hch ⟩
  have c2 : channels.val * cap.val ≤ 255 * 255 * 255 := by rw [hch]; exact b2
  step as ⟨ slots, hsl ⟩
  have c3 : slots.val * pl.val ≤ 255 * 255 * 255 * 65535 := by rw [hsl, hch]; exact b3
  step as ⟨ bytes, hby ⟩
  have eby : bytes.val = e.roles.val * e.roles.val * e.capacity.val * e.payload.val := by
    rw [hby, hsl, hch, en, ecap, epl]
  step as ⟨ i, hi ⟩
  split
  · rename_i hz
    step as ⟨ i1, hi1 ⟩
    have ei1 : i1.val = e.storage.val := by rw [hi1]; scalar_tac
    step as ⟨ i2, hi2 ⟩
    step as ⟨ i3, hi3 ⟩
    have ei3 : i3.val = total.val := by rw [hi3]; scalar_tac
    simp only [decide_eq_true_eq]
    rw [← eby]
    constructor
    · intro h; exact ⟨by scalar_tac, by scalar_tac⟩
    · intro h; scalar_tac
  · rename_i hz
    simp only [WP.spec_ok]
    constructor
    · intro h; cases h
    · intro h; exact absurd (by scalar_tac) hz

theorem pow_gt_32 {k : Nat} (h : 32 < k) : 0xFFFF_FFFF < 2 ^ k := by
  have : 2 ^ 33 ≤ 2 ^ k := Nat.pow_le_pow_right (by decide) (by omega)
  omega

/-- `ram_apart_ok` decides `ram_apart` (Verus proves the direction from
`true`). -/
theorem ram_apart_ok_spec (ram : image_v.IntervalImg) (base total : U32) :
    image_v.ram_apart_ok ram base total ⦃ b => b = true ↔ RamApart ram base.val total.val ⦄ := by
  unfold image_v.ram_apart_ok RamApart intervalEnd
  split
  · rename_i hgt
    simp only [WP.spec_ok]
    have := pow_gt_32 (k := ram.size_log2.val) (by scalar_tac)
    constructor
    · intro h; cases h
    · intro h; omega
  · rename_i hle
    have hk : ram.size_log2.val ≤ 32 := by scalar_tac
    have hb : 2 ^ ram.size_log2.val ≤ 2 ^ 32 := Nat.pow_le_pow_right (by decide) hk
    step as ⟨ i, hi ⟩
    have ei : i.val = ram.base.val := by rw [hi]; scalar_tac
    step with (pow2_u64_spec ram.size_log2 hk) as ⟨ i1, hi1 ⟩
    step as ⟨ en, hen ⟩
    rw [← hi1, ← ei, ← hen]
    split
    · rename_i hfit
      step as ⟨ i2, hi2 ⟩
      have ei2 : i2.val = base.val := by rw [hi2]; scalar_tac
      split
      · rename_i hbelow
        simp only [WP.spec_ok]
        constructor
        · intro _; exact ⟨by scalar_tac, Or.inl (by scalar_tac)⟩
        · intro _; trivial
      · rename_i habove
        step as ⟨ i3, hi3 ⟩
        step as ⟨ i4, hi4 ⟩
        have ei3 : i3.val = base.val := by rw [hi3]; scalar_tac
        have ei4 : i4.val = total.val := by rw [hi4]; scalar_tac
        step as ⟨ i5, hi5 ⟩
        step as ⟨ i6, hi6 ⟩
        have ei6 : i6.val = ram.base.val := by rw [hi6]; scalar_tac
        simp only [decide_eq_true_eq]
        constructor
        · intro h; exact ⟨by scalar_tac, Or.inr (by scalar_tac)⟩
        · intro h
          rcases h.2 with h2 | h2
          · exact absurd (by scalar_tac) habove
          · scalar_tac
    · rename_i hfit
      simp only [WP.spec_ok]
      constructor
      · intro h; cases h
      · intro h; exact absurd (by scalar_tac) hfit

/-- `storage_in_kernel_ram_ok` returns `true` only when the storage lies
inside the Kernel's RAM interval. -/
theorem storage_in_kernel_ram_ok_spec (kr : image_v.IntervalImg) (base total : U32) :
    image_v.storage_in_kernel_ram_ok kr base total
      ⦃ b => b = true → StorageInKernelRam kr base.val total.val ⦄ := by
  unfold image_v.storage_in_kernel_ram_ok StorageInKernelRam intervalEnd
  split
  · simp only [WP.spec_ok]
    intro h; cases h
  · rename_i hle
    have hk : kr.size_log2.val ≤ 32 := by scalar_tac
    have hb : 2 ^ kr.size_log2.val ≤ 2 ^ 32 := Nat.pow_le_pow_right (by decide) hk
    step as ⟨ i, hi ⟩
    have ei : i.val = kr.base.val := by rw [hi]; scalar_tac
    step with (pow2_u64_spec kr.size_log2 hk) as ⟨ i1, hi1 ⟩
    step as ⟨ en, hen ⟩
    rw [← hi1, ← ei, ← hen]
    split
    · rename_i hlo
      step as ⟨ i2, hi2 ⟩
      step as ⟨ i3, hi3 ⟩
      have ei2 : i2.val = base.val := by rw [hi2]; scalar_tac
      have ei3 : i3.val = total.val := by rw [hi3]; scalar_tac
      step as ⟨ i4, hi4 ⟩
      simp only [decide_eq_true_eq]
      intro h
      exact ⟨by scalar_tac, by scalar_tac⟩
    · simp only [WP.spec_ok]
      intro h; cases h

theorem storage_loop0_spec (endpoints : alloc.vec.Vec image_v.EndpointCfg) (total : U32) :
    image_v.storage_placed_ok_loop0 endpoints total true 0#usize
      ⦃ b => b = true ↔ ∀ e, e < endpoints.val.length →
        EndpointInStorage endpoints.val[e]! total.val ⦄ := by
  unfold image_v.storage_placed_ok_loop0
  apply FlagLoop.flag_loop _ endpoints.val.length
    (fun e => EndpointInStorage endpoints.val[e]! total.val)
  · intro ok1 i h
    dsimp only
    unfold image_v.storage_placed_ok_loop0.body
    simp only [alloc.vec.Vec.len]
    rcases h with h | h
    · rw [if_neg (by scalar_tac)]
    · subst h; split <;> rfl
  · intro i hi _
    dsimp only
    unfold image_v.storage_placed_ok_loop0.body
    simp only [alloc.vec.Vec.len]
    rw [if_pos (by scalar_tac)]
    simp only [if_true]
    step as ⟨ ec, hec ⟩
    have eec : ec = endpoints.val[i.val]! := by
      rw [hec]; exact (getElem!_pos endpoints.val i.val hi).symm
    rw [← eec]
    step with (endpoint_in_storage_ok_spec ec total) as ⟨ b, hb ⟩
    split
    · rename_i hbt
      step as ⟨ j1, hj1 ⟩
      right
      exact ⟨j1, rfl, hj1, hb.1 hbt⟩
    · rename_i hbf
      simp only [WP.spec_ok]
      left
      exact ⟨trivial, fun hc => hbf (hb.2 hc)⟩

theorem storage_loop1_spec (layout : image_v.LayoutImg) (base total : U32) :
    image_v.storage_placed_ok_loop1 layout base total true 0#usize
      ⦃ b => b = true ↔ ∀ p, p < layout.partitions.val.length →
        RamApart (layout.partitions.val[p]!).ram base.val total.val ⦄ := by
  unfold image_v.storage_placed_ok_loop1
  apply FlagLoop.flag_loop _ layout.partitions.val.length
    (fun p => RamApart (layout.partitions.val[p]!).ram base.val total.val)
  · intro ok1 i h
    dsimp only
    unfold image_v.storage_placed_ok_loop1.body
    simp only [alloc.vec.Vec.len]
    rcases h with h | h
    · rw [if_neg (by scalar_tac)]
    · subst h; split <;> rfl
  · intro i hi _
    dsimp only
    unfold image_v.storage_placed_ok_loop1.body
    simp only [alloc.vec.Vec.len]
    rw [if_pos (by scalar_tac)]
    simp only [if_true]
    step as ⟨ pli, hpli ⟩
    have epli : pli = layout.partitions.val[i.val]! := by
      rw [hpli]; exact (getElem!_pos layout.partitions.val i.val hi).symm
    rw [← epli]
    step with (ram_apart_ok_spec pli.ram base total) as ⟨ b, hb ⟩
    split
    · rename_i hbt
      step as ⟨ j1, hj1 ⟩
      right
      exact ⟨j1, rfl, hj1, hb.1 hbt⟩
    · rename_i hbf
      simp only [WP.spec_ok]
      left
      exact ⟨trivial, fun hc => hbf (hb.2 hc)⟩

/-- **The storage-placement check means what it says**
(`storage_placed_ok`, R16.1): the function returns for every input, and
returns `true` exactly when `StoragePlaced` holds of the decoded layout,
the decoded Endpoint table and the storage's address and size. Verus
proves the direction from `true`. -/
theorem storage_placed_ok_spec (layout : image_v.LayoutImg)
    (endpoints : alloc.vec.Vec image_v.EndpointCfg) (base total : U32) :
    image_v.storage_placed_ok layout endpoints base total
      ⦃ b => b = true ↔ StoragePlaced layout endpoints.val base.val total.val ⦄ := by
  unfold image_v.storage_placed_ok StoragePlaced
  step as ⟨ i, hi ⟩
  split
  · rename_i hnz
    simp only [WP.spec_ok]
    constructor
    · intro h; cases h
    · intro h
      simp only [bne_iff_ne, ne_eq] at hnz
      exact absurd (by scalar_tac) hnz
  · rename_i hz
    have hb4 : base.val % 4 = 0 := by
      simp only [bne_iff_ne, ne_eq, not_not] at hz
      rw [← hi, hz]; rfl
    step with (storage_loop0_spec endpoints total) as ⟨ ok1, hok1 ⟩
    split
    · rename_i hk
      subst hk
      have heps := hok1.1 rfl
      refine WP.spec_mono (storage_loop1_spec layout base total) ?_
      intro b hb
      rw [hb]
      constructor
      · intro hall; exact ⟨hb4, heps, hall⟩
      · intro hc; exact hc.2.2
    · rename_i hk
      simp only [WP.spec_ok]
      constructor
      · intro hc; cases hc
      · intro hc; exact absurd (hok1.2 hc.2.1) hk

/-! ## The comparisons and the three image checks (R26.4)

Each comparison answers `true` only when its two arguments are equal, and
each image check answers `true` only when the bytes are, byte for byte,
the image of the expected value. -/

instance : Inhabited image_v.TaskCfg :=
  ⟨⟨0#u8, 0#u8, 0#u8, 0#u64, 0#u64, 0#u64, 0#u32, 0#u8, false⟩⟩
instance : Inhabited image_v.ResourceCfg := ⟨⟨0#u8, 0#u8⟩⟩
instance : Inhabited image_v.PartitionCfg := ⟨⟨0#u8, 0#u8, 0#u8, 0#u8, 0#u32⟩⟩
instance : Inhabited (alloc.vec.Vec Bool) := ⟨alloc.vec.Vec.new _⟩

theorem bytes_agree_spec (a b : alloc.vec.Vec (U8)) :
    image_v.bytes_agree a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.bytes_agree
  simp only [alloc.vec.Vec.len]
  split
  · simp only [WP.spec_ok]
    intro h; cases h
  · rename_i hl
    have hlen : a.val.length = b.val.length := by
      simp only [bne_iff_ne, ne_eq, not_not] at hl
      scalar_tac
    unfold image_v.bytes_agree_loop
    refine WP.spec_mono (FlagLoop.agree_loop _ a.val b.val hlen ?_ ?_)
      (fun r hr h => alloc.vec.Vec.ext _ _ (hr h))
    · intro i hi
      unfold image_v.bytes_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_neg (by scalar_tac)]
    · intro i hi
      unfold image_v.bytes_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_pos (by scalar_tac)]
      step as ⟨ x, hx ⟩
      step as ⟨ y, hy ⟩
      have ex : x = a.val[i.val]! := by rw [hx]; exact (getElem!_pos a.val i.val hi).symm
      have ey : y = b.val[i.val]! := by
        rw [hy]; exact (getElem!_pos b.val i.val (by omega)).symm
      rw [← ex, ← ey]
      split
      · first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done)
      · rename_i hne
        step as ⟨ i2, hi2 ⟩
        right
        simp only [bne_iff_ne, ne_eq, not_not] at hne
        exact ⟨i2, rfl, hi2, hne⟩

theorem flags_agree_spec (a b : alloc.vec.Vec (Bool)) :
    image_v.flags_agree a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.flags_agree
  simp only [alloc.vec.Vec.len]
  split
  · simp only [WP.spec_ok]
    intro h; cases h
  · rename_i hl
    have hlen : a.val.length = b.val.length := by
      simp only [bne_iff_ne, ne_eq, not_not] at hl
      scalar_tac
    unfold image_v.flags_agree_loop
    refine WP.spec_mono (FlagLoop.agree_loop _ a.val b.val hlen ?_ ?_)
      (fun r hr h => alloc.vec.Vec.ext _ _ (hr h))
    · intro i hi
      unfold image_v.flags_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_neg (by scalar_tac)]
    · intro i hi
      unfold image_v.flags_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_pos (by scalar_tac)]
      step as ⟨ x, hx ⟩
      step as ⟨ y, hy ⟩
      have ex : x = a.val[i.val]! := by rw [hx]; exact (getElem!_pos a.val i.val hi).symm
      have ey : y = b.val[i.val]! := by
        rw [hy]; exact (getElem!_pos b.val i.val (by omega)).symm
      rw [← ex, ← ey]
      split
      · first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done)
      · rename_i hne
        step as ⟨ i2, hi2 ⟩
        right
        simp only [bne_iff_ne, ne_eq, not_not] at hne
        exact ⟨i2, rfl, hi2, hne⟩

theorem task_agrees_spec (a b : image_v.TaskCfg) :
    image_v.task_agrees a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.task_agrees
  repeat' split
  all_goals simp only [WP.spec_ok]
  all_goals first
    | (intro h; cases h; done)
    | (intro h
       cases a
       cases b
       simp_all)

theorem tasks_agree_spec (a b : alloc.vec.Vec (image_v.TaskCfg)) :
    image_v.tasks_agree a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.tasks_agree
  simp only [alloc.vec.Vec.len]
  split
  · simp only [WP.spec_ok]
    intro h; cases h
  · rename_i hl
    have hlen : a.val.length = b.val.length := by
      simp only [bne_iff_ne, ne_eq, not_not] at hl
      scalar_tac
    unfold image_v.tasks_agree_loop
    refine WP.spec_mono (FlagLoop.agree_loop _ a.val b.val hlen ?_ ?_)
      (fun r hr h => alloc.vec.Vec.ext _ _ (hr h))
    · intro i hi
      unfold image_v.tasks_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_neg (by scalar_tac)]
    · intro i hi
      unfold image_v.tasks_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_pos (by scalar_tac)]
      step as ⟨ x, hx ⟩
      step as ⟨ y, hy ⟩
      have ex : x = a.val[i.val]! := by rw [hx]; exact (getElem!_pos a.val i.val hi).symm
      have ey : y = b.val[i.val]! := by
        rw [hy]; exact (getElem!_pos b.val i.val (by omega)).symm
      rw [← ex, ← ey]
      step with (task_agrees_spec x y) as ⟨ b1, hb1 ⟩
      split
      · rename_i hbt
        step as ⟨ i2, hi2 ⟩
        right
        exact ⟨i2, rfl, hi2, hb1 hbt⟩
      · first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done)

theorem resources_agree_spec (a b : alloc.vec.Vec (image_v.ResourceCfg)) :
    image_v.resources_agree a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.resources_agree
  simp only [alloc.vec.Vec.len]
  split
  · simp only [WP.spec_ok]
    intro h; cases h
  · rename_i hl
    have hlen : a.val.length = b.val.length := by
      simp only [bne_iff_ne, ne_eq, not_not] at hl
      scalar_tac
    unfold image_v.resources_agree_loop
    refine WP.spec_mono (FlagLoop.agree_loop _ a.val b.val hlen ?_ ?_)
      (fun r hr h => alloc.vec.Vec.ext _ _ (hr h))
    · intro i hi
      unfold image_v.resources_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_neg (by scalar_tac)]
    · intro i hi
      unfold image_v.resources_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_pos (by scalar_tac)]
      step as ⟨ x, hx ⟩
      step as ⟨ y, hy ⟩
      have ex : x = a.val[i.val]! := by rw [hx]; exact (getElem!_pos a.val i.val hi).symm
      have ey : y = b.val[i.val]! := by
        rw [hy]; exact (getElem!_pos b.val i.val (by omega)).symm
      rw [← ex, ← ey]
      repeat' split
      all_goals first
        | (first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done))
        | (step as ⟨ i2, hi2 ⟩
           right
           refine ⟨i2, rfl, hi2, ?_⟩
           clear ex ey hx hy
           simp only [bne_iff_ne, ne_eq, not_not] at *
           cases x
           cases y
           simp_all)

theorem rows_agree_spec (a b : alloc.vec.Vec (alloc.vec.Vec Bool)) :
    image_v.rows_agree a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.rows_agree
  simp only [alloc.vec.Vec.len]
  split
  · simp only [WP.spec_ok]
    intro h; cases h
  · rename_i hl
    have hlen : a.val.length = b.val.length := by
      simp only [bne_iff_ne, ne_eq, not_not] at hl
      scalar_tac
    unfold image_v.rows_agree_loop
    refine WP.spec_mono (FlagLoop.agree_loop _ a.val b.val hlen ?_ ?_)
      (fun r hr h => alloc.vec.Vec.ext _ _ (hr h))
    · intro i hi
      unfold image_v.rows_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_neg (by scalar_tac)]
    · intro i hi
      unfold image_v.rows_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_pos (by scalar_tac)]
      step as ⟨ x, hx ⟩
      step as ⟨ y, hy ⟩
      have ex : x = a.val[i.val]! := by rw [hx]; exact (getElem!_pos a.val i.val hi).symm
      have ey : y = b.val[i.val]! := by
        rw [hy]; exact (getElem!_pos b.val i.val (by omega)).symm
      rw [← ex, ← ey]
      step with (flags_agree_spec x y) as ⟨ b1, hb1 ⟩
      split
      · rename_i hbt
        step as ⟨ i2, hi2 ⟩
        right
        exact ⟨i2, rfl, hi2, hb1 hbt⟩
      · first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done)

theorem partitions_agree_spec (a b : alloc.vec.Vec (image_v.PartitionCfg)) :
    image_v.partitions_agree a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.partitions_agree
  simp only [alloc.vec.Vec.len]
  split
  · simp only [WP.spec_ok]
    intro h; cases h
  · rename_i hl
    have hlen : a.val.length = b.val.length := by
      simp only [bne_iff_ne, ne_eq, not_not] at hl
      scalar_tac
    unfold image_v.partitions_agree_loop
    refine WP.spec_mono (FlagLoop.agree_loop _ a.val b.val hlen ?_ ?_)
      (fun r hr h => alloc.vec.Vec.ext _ _ (hr h))
    · intro i hi
      unfold image_v.partitions_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_neg (by scalar_tac)]
    · intro i hi
      unfold image_v.partitions_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_pos (by scalar_tac)]
      step as ⟨ x, hx ⟩
      step as ⟨ y, hy ⟩
      have ex : x = a.val[i.val]! := by rw [hx]; exact (getElem!_pos a.val i.val hi).symm
      have ey : y = b.val[i.val]! := by
        rw [hy]; exact (getElem!_pos b.val i.val (by omega)).symm
      rw [← ex, ← ey]
      repeat' split
      all_goals first
        | (first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done))
        | (step as ⟨ i2, hi2 ⟩
           right
           refine ⟨i2, rfl, hi2, ?_⟩
           clear ex ey hx hy
           simp only [bne_iff_ne, ne_eq, not_not] at *
           cases x
           cases y
           simp_all)

/-- The rule for a conditional, applied without simplification. -/
theorem spec_ite {α} {c : Prop} [Decidable c] {x y : Result α} {P : α → Prop}
    (ht : c → x ⦃ P ⦄) (hf : ¬ c → y ⦃ P ⦄) : (if c then x else y) ⦃ P ⦄ := by
  by_cases h : c
  · rw [if_pos h]; exact ht h
  · rw [if_neg h]; exact hf h

/-- `tables_agree` answers `true` only when the two tables are equal. -/
theorem tables_agree_spec (a b : image_v.Tables) :
    image_v.tables_agree a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.tables_agree
  refine spec_ite (fun _ => ?_) (fun _ => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun _ => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun _ => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun _ => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (tasks_agree_spec a.tasks b.tasks) as ⟨ b1, h1 ⟩
  refine spec_ite (fun t1 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (resources_agree_spec a.resources b.resources) as ⟨ b2, h2 ⟩
  refine spec_ite (fun t2 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (rows_agree_spec a.access b.access) as ⟨ b3, h3 ⟩
  refine spec_ite (fun t3 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (partitions_agree_spec a.partitions b.partitions) as ⟨ b4, h4 ⟩
  refine spec_ite (fun t4 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun d1 => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun d2 => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (bytes_agree_spec a.slot_release b.slot_release) as ⟨ b5, h5 ⟩
  refine spec_ite (fun t5 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (bytes_agree_spec a.slot_deadline b.slot_deadline) as ⟨ b6, h6 ⟩
  refine spec_ite (fun t6 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (bytes_agree_spec a.slot_budget b.slot_budget) as ⟨ b7, h7 ⟩
  refine spec_ite (fun hb7 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  simp only [WP.spec_ok]
  intro _
  have e1 := h1 t1
  have e2 := h2 t2
  have e3 := h3 t3
  have e4 := h4 t4
  have e5 := h5 t5
  have e6 := h6 t6
  have e7 := h7 hb7
  simp only [bne_iff_ne, ne_eq, not_not] at d1 d2
  clear h1 h2 h3 h4 h5 h6 h7
  cases a
  cases b
  dsimp only at e1 e2 e3 e4 e5 e6 e7 d1 d2
  subst e1 e2 e3 e4 e5 e6 e7 d1 d2
  rfl

/-- **The `CONFIG` image check** (`image_is`'s contract, R26.4): the
function returns for every input, and answers `true` only when `bytes`
is, byte for byte, the image of the expected tables. -/
theorem image_is_spec (bytes : Slice U8) (expected : image_v.Tables) :
    image_v.image_is bytes expected ⦃ r => r = true → bvs bytes.val = specImage expected ⦄ := by
  unfold image_v.image_is
  step with (decode_tables_spec bytes) as ⟨ r, hr ⟩
  rcases r with found | e
  · dsimp only at hr ⊢
    refine WP.spec_mono (tables_agree_spec found expected) ?_
    intro b hb ht
    rw [← hb ht]
    exact hr.2
  · (simp only [WP.spec_ok]; intro h; cases h)

theorem interval_agrees_spec (a b : image_v.IntervalImg) :
    image_v.interval_agrees a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.interval_agrees
  repeat' split
  all_goals simp only [WP.spec_ok]
  all_goals first
    | (intro h; cases h; done)
    | (intro h
       cases a
       cases b
       simp_all)

theorem windows_agree_spec (a b : alloc.vec.Vec (U32 × U8 × Bool)) :
    image_v.windows_agree a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.windows_agree
  simp only [alloc.vec.Vec.len]
  split
  · simp only [WP.spec_ok]
    intro h; cases h
  · rename_i hl
    have hlen : a.val.length = b.val.length := by
      simp only [bne_iff_ne, ne_eq, not_not] at hl
      scalar_tac
    unfold image_v.windows_agree_loop
    refine WP.spec_mono (FlagLoop.agree_loop _ a.val b.val hlen ?_ ?_)
      (fun r hr h => alloc.vec.Vec.ext _ _ (hr h))
    · intro i hi
      unfold image_v.windows_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_neg (by scalar_tac)]
    · intro i hi
      unfold image_v.windows_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_pos (by scalar_tac)]
      step as ⟨ x0, x1, x2, hx ⟩
      step as ⟨ y0, y1, y2, hy ⟩
      have ex : (x0, x1, x2) = a.val[i.val]! := by
        rw [hx]; exact (getElem!_pos a.val i.val hi).symm
      have ey : (y0, y1, y2) = b.val[i.val]! := by
        rw [hy]; exact (getElem!_pos b.val i.val (by omega)).symm
      rw [← ex, ← ey]
      repeat' split
      all_goals first
        | (first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done))
        | (step as ⟨ i2, hi2 ⟩
           right
           refine ⟨i2, rfl, hi2, ?_⟩
           clear ex ey hx hy
           simp only [bne_iff_ne, ne_eq, not_not] at *
           simp_all)

theorem kernel_windows_agree_spec (a b : alloc.vec.Vec (U32 × U8)) :
    image_v.kernel_windows_agree a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.kernel_windows_agree
  simp only [alloc.vec.Vec.len]
  split
  · simp only [WP.spec_ok]
    intro h; cases h
  · rename_i hl
    have hlen : a.val.length = b.val.length := by
      simp only [bne_iff_ne, ne_eq, not_not] at hl
      scalar_tac
    unfold image_v.kernel_windows_agree_loop
    refine WP.spec_mono (FlagLoop.agree_loop _ a.val b.val hlen ?_ ?_)
      (fun r hr h => alloc.vec.Vec.ext _ _ (hr h))
    · intro i hi
      unfold image_v.kernel_windows_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_neg (by scalar_tac)]
    · intro i hi
      unfold image_v.kernel_windows_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_pos (by scalar_tac)]
      step as ⟨ x0, x1, hx ⟩
      step as ⟨ y0, y1, hy ⟩
      have ex : (x0, x1) = a.val[i.val]! := by
        rw [hx]; exact (getElem!_pos a.val i.val hi).symm
      have ey : (y0, y1) = b.val[i.val]! := by
        rw [hy]; exact (getElem!_pos b.val i.val (by omega)).symm
      rw [← ex, ← ey]
      repeat' split
      all_goals first
        | (first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done))
        | (step as ⟨ i2, hi2 ⟩
           right
           refine ⟨i2, rfl, hi2, ?_⟩
           clear ex ey hx hy
           simp only [bne_iff_ne, ne_eq, not_not] at *
           simp_all)

theorem parts_agree_spec (a b : alloc.vec.Vec (image_v.PartitionLayoutImg)) :
    image_v.parts_agree a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.parts_agree
  simp only [alloc.vec.Vec.len]
  split
  · simp only [WP.spec_ok]
    intro h; cases h
  · rename_i hl
    have hlen : a.val.length = b.val.length := by
      simp only [bne_iff_ne, ne_eq, not_not] at hl
      scalar_tac
    unfold image_v.parts_agree_loop
    refine WP.spec_mono (FlagLoop.agree_loop _ a.val b.val hlen ?_ ?_)
      (fun r hr h => alloc.vec.Vec.ext _ _ (hr h))
    · intro i hi
      unfold image_v.parts_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_neg (by scalar_tac)]
    · intro i hi
      unfold image_v.parts_agree_loop.body
      simp only [alloc.vec.Vec.len]
      rw [if_pos (by scalar_tac)]
      step as ⟨ x, hx ⟩
      step as ⟨ y, hy ⟩
      have ex : x = a.val[i.val]! := by rw [hx]; exact (getElem!_pos a.val i.val hi).symm
      have ey : y = b.val[i.val]! := by
        rw [hy]; exact (getElem!_pos b.val i.val (by omega)).symm
      rw [← ex, ← ey]
      step with (interval_agrees_spec x.code y.code) as ⟨ b1, hb1 ⟩
      refine spec_ite (fun t1 => ?_) (fun _ => ?_)
      swap
      · first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done)
      step with (interval_agrees_spec x.ram y.ram) as ⟨ b2, hb2 ⟩
      refine spec_ite (fun t2 => ?_) (fun _ => ?_)
      swap
      · first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done)
      refine spec_ite (fun _ => ?_) (fun d1 => ?_)
      · first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done)
      refine spec_ite (fun _ => ?_) (fun d2 => ?_)
      · first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done)
      step with (windows_agree_spec x.windows y.windows) as ⟨ b3, hb3 ⟩
      refine spec_ite (fun t3 => ?_) (fun _ => ?_)
      swap
      · first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done)
      step as ⟨ i2, hi2 ⟩
      right
      refine ⟨i2, rfl, hi2, ?_⟩
      have e1 := hb1 t1
      have e2 := hb2 t2
      have e3 := hb3 t3
      simp only [bne_iff_ne, ne_eq, not_not] at d1 d2
      clear ex ey hx hy hb1 hb2 hb3
      cases x
      cases y
      dsimp only at e1 e2 e3 d1 d2
      subst e1 e2 e3 d1 d2
      rfl

/-- `layouts_agree` answers `true` only when the two layouts are equal. -/
theorem layouts_agree_spec (a b : image_v.LayoutImg) :
    image_v.layouts_agree a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.layouts_agree
  step with (interval_agrees_spec a.kernel_flash b.kernel_flash) as ⟨ b1, h1 ⟩
  refine spec_ite (fun t1 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (interval_agrees_spec a.shared_code b.shared_code) as ⟨ b2, h2 ⟩
  refine spec_ite (fun t2 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (interval_agrees_spec a.kernel_ram b.kernel_ram) as ⟨ b3, h3 ⟩
  refine spec_ite (fun t3 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun d1 => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun _ => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun _ => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (kernel_windows_agree_spec a.kernel_windows b.kernel_windows) as ⟨ b4, h4 ⟩
  refine spec_ite (fun t4 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine WP.spec_mono (parts_agree_spec a.partitions b.partitions) ?_
  intro r hr h
  have e1 := h1 t1
  have e2 := h2 t2
  have e3 := h3 t3
  have e4 := h4 t4
  have e5 := hr h
  simp only [bne_iff_ne, ne_eq, not_not] at d1
  clear h1 h2 h3 h4 hr
  cases a
  cases b
  dsimp only at e1 e2 e3 e4 e5 d1
  subst e1 e2 e3 e4 e5 d1
  rfl

/-- **The `LAYOUT` image check** (`layout_image_is`'s contract, R26.4,
R16.7): the function returns for every input, and answers `true` only when
`bytes` is, byte for byte, the image of the expected layout. -/
theorem layout_image_is_spec (bytes : Slice U8) (expected : image_v.LayoutImg) :
    image_v.layout_image_is bytes expected
      ⦃ r => r = true → bvs bytes.val = specLayoutImage expected ⦄ := by
  unfold image_v.layout_image_is
  step with (decode_layout_image_spec bytes) as ⟨ r, hr ⟩
  rcases r with found | e
  · dsimp only at hr ⊢
    refine WP.spec_mono (layouts_agree_spec found expected) ?_
    intro b hb ht
    rw [← hb ht]
    exact hr.2
  · (simp only [WP.spec_ok]; intro h; cases h)

/-- `endpoint_agrees` answers `true` only when the two entries are equal. -/
theorem endpoint_agrees_spec (a b : image_v.EndpointCfg) :
    image_v.endpoint_agrees a b ⦃ r => r = true → a = b ⦄ := by
  unfold image_v.endpoint_agrees
  refine spec_ite (fun _ => ?_) (fun d1 => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun d2 => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun d3 => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun d4 => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (bytes_agree_spec a.tasks b.tasks) as ⟨ b1, h1 ⟩
  refine spec_ite (fun t1 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (bytes_agree_spec a.partitions b.partitions) as ⟨ b2, h2 ⟩
  refine spec_ite (fun t2 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  step with (flags_agree_spec a.release b.release) as ⟨ b3, h3 ⟩
  refine spec_ite (fun t3 => ?_) (fun _ => ?_)
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun hl => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  have hlen : a.validates.val.length = b.validates.val.length := by
    simp only [alloc.vec.Vec.len, bne_iff_ne, ne_eq, not_not] at hl
    scalar_tac
  unfold image_v.endpoint_agrees_loop
  refine WP.spec_mono (FlagLoop.agree_loop _ a.validates.val b.validates.val hlen ?_ ?_) ?_
  · intro i hi
    unfold image_v.endpoint_agrees_loop.body
    simp only [alloc.vec.Vec.len]
    rw [if_neg (by scalar_tac)]
  · intro i hi
    unfold image_v.endpoint_agrees_loop.body
    simp only [alloc.vec.Vec.len]
    rw [if_pos (by scalar_tac)]
    step as ⟨ x, hx ⟩
    step as ⟨ y, hy ⟩
    have ex : x = a.validates.val[i.val]! := by rw [hx]; exact (getElem!_pos a.validates.val i.val hi).symm
    have ey : y = b.validates.val[i.val]! := by
      rw [hy]; exact (getElem!_pos b.validates.val i.val (by omega)).symm
    rw [← ex, ← ey]
    step with (flags_agree_spec x y) as ⟨ b1, hb1 ⟩
    split
    · rename_i hbt
      step as ⟨ i2, hi2 ⟩
      right
      exact ⟨i2, rfl, hi2, hb1 hbt⟩
    · first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done)

  · intro r hr h
    have e1 := h1 t1
    have e2 := h2 t2
    have e3 := h3 t3
    have e4 := alloc.vec.Vec.ext _ _ (hr h)
    simp only [bne_iff_ne, ne_eq, not_not] at d1 d2 d3 d4
    clear h1 h2 h3 hr hl hlen
    cases a
    cases b
    dsimp only at e1 e2 e3 e4 d1 d2 d3 d4
    subst e1 e2 e3 e4 d1 d2 d3 d4
    rfl

/-- **The Endpoint image check** (`endpoint_image_is`'s contract, R26.4):
the function returns for every input, and answers `true` only when
`bytes` is, byte for byte, the image of the expected Endpoint table and
storage size. -/
theorem endpoint_image_is_spec (bytes : Slice U8) (expected : alloc.vec.Vec image_v.EndpointCfg)
    (storage_bytes : U32) :
    image_v.endpoint_image_is bytes expected storage_bytes
      ⦃ r => r = true → bvs bytes.val = specEndpointImage expected.val storage_bytes ⦄ := by
  unfold image_v.endpoint_image_is
  step with (decode_endpoint_image_spec bytes) as ⟨ r, hr ⟩
  rcases r with ⟨found, fs⟩ | e
  swap
  · (simp only [WP.spec_ok]; intro h; cases h)
  dsimp only at hr ⊢
  refine spec_ite (fun _ => ?_) (fun hs => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  refine spec_ite (fun _ => ?_) (fun hl => ?_)
  · (simp only [WP.spec_ok]; intro h; cases h)
  have hlen : found.val.length = expected.val.length := by
    simp only [alloc.vec.Vec.len, bne_iff_ne, ne_eq, not_not] at hl
    scalar_tac
  unfold image_v.endpoint_image_is_loop
  refine WP.spec_mono (FlagLoop.agree_loop _ found.val expected.val hlen ?_ ?_) ?_
  · intro i hi
    unfold image_v.endpoint_image_is_loop.body
    simp only [alloc.vec.Vec.len]
    rw [if_neg (by scalar_tac)]
  · intro i hi
    unfold image_v.endpoint_image_is_loop.body
    simp only [alloc.vec.Vec.len]
    rw [if_pos (by scalar_tac)]
    step as ⟨ x, hx ⟩
    step as ⟨ y, hy ⟩
    have ex : x = found.val[i.val]! := by rw [hx]; exact (getElem!_pos found.val i.val hi).symm
    have ey : y = expected.val[i.val]! := by
      rw [hy]; exact (getElem!_pos expected.val i.val (by omega)).symm
    rw [← ex, ← ey]
    step with (endpoint_agrees_spec x y) as ⟨ b1, hb1 ⟩
    split
    · rename_i hbt
      step as ⟨ i2, hi2 ⟩
      right
      exact ⟨i2, rfl, hi2, hb1 hbt⟩
    · first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; left; rfl) | (simp [WP.spec_ok]; done)

  · intro r hr2 h
    have e := hr2 h
    simp only [bne_iff_ne, ne_eq, not_not] at hs
    rw [← e, ← hs]
    exact hr.2.2

end rsk_confcheck
