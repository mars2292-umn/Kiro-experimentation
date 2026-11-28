import Sched.Arm
import Tools.LinkBits
import Tools.LinkThumbDefs

/-! # The Link_Checker's Thumb decoder against the Armv7-M semantics (2026-11-26)

The Link_Checker (`crates/rsk-link/src/thumb.rs`) reads from each
instruction of a linked binary what the stack analysis needs: by how much
the instruction moves the stack pointer, and where a branch goes. The
stubs' machine-code proof has a semantics of the instructions the stubs use
(`Sched/Arm.lean`). This file compares the two, in arithmetic:

- `D16` and `D32` (`Tools/LinkThumbDefs.lean`) say what the decoder answers
  for a halfword, or two, as functions of natural numbers
  (`Tools/LinkThumbProofs.lean` proves that the compiled decoder computes
  them);
- `d16_agrees` and `d32_agrees`: for every encoding the semantics'
  decoder accepts, the stack-pointer change the Link_Checker reads is the
  change the semantics' operation makes (`SpAgrees`), and the target it
  reads for a `bl`, a `b`, a conditional branch or a `cbz` is the address
  the semantics loads into the PC (`TargetAgrees`).

No encoding the semantics decodes is excepted (two were until the
Link_Checker's decoder was corrected on 2026-11-26).

Nothing here is about the compiled code; handwritten. -/

namespace LinkThumb

open Arm LinkBits

/-! ## The operations and the stack pointer -/

theorem setR_sp (s : St) (n : Nat) (v : W) (hn : n ≠ 13) : (setR s n v).sp = s.sp := by
  unfold setR
  rw [if_neg hn]
  split <;> rfl

theorem map_some {α β : Type} {f : α → β} {o : Option α} {b : β} (h : o.map f = some b) :
    ∃ a, o = some a ∧ f a = b := by
  cases o with
  | none => exact absurd h (by simp)
  | some a => exact ⟨a, rfl, by simpa using h⟩

theorem bind_some' {α β : Type} {o : Option α} {f : α → Option β} {b : β}
    (h : o.bind f = some b) : ∃ a, o = some a ∧ f a = some b := by
  cases o with
  | none => exact absurd h (by simp)
  | some a => exact ⟨a, rfl, by simpa using h⟩

/-- The two cases of a conditional that has a value. -/
theorem ite_some_cases {α : Type} {c : Prop} [Decidable c] {x y : Option α} {v : α}
    (h : (if c then x else y) = some v) : (c ∧ x = some v) ∨ (¬ c ∧ y = some v) := by
  by_cases hc : c
  · left; exact ⟨hc, by rwa [if_pos hc] at h⟩
  · right; exact ⟨hc, by rwa [if_neg hc] at h⟩

/-- Decides each conditional of the goal in turn, by arithmetic over the
hypotheses. -/
macro "eval_ifs" : tactic =>
  `(tactic| repeat (first | rw [if_pos (by omega)] | rw [if_neg (by omega)]))

theorem fpCheck_sp {s u : St} (h : fpCheck s = some u) : u.sp = s.sp := by
  unfold fpCheck at h
  split at h
  · exact absurd h (by simp)
  · have e := Option.some.inj h
    rw [← e]
    split <;> rfl

/-- The instructions of the semantics whose operation leaves SP. -/
def KeepsSp : Instr → Prop
  | .cps _ _ _ => True
  | .push _ => False
  | .pop _ => False
  | .tst _ _ => True
  | .it _ _ => True
  | .vpush _ _ => False
  | .vpop _ _ => False
  | .mrs rd _ => rd ≠ 13
  | .msr _ _ => True
  | .movr rd _ => rd ≠ 13
  | .movi rd _ => rd ≠ 13
  | .subsp _ => False
  | .addsp _ => False
  | .addrsp rd _ => rd ≠ 13
  | .strsp _ _ => True
  | .ldrsp rt _ => rt ≠ 13
  | .cmpi _ _ => True
  | .dsb => True
  | .isb => True
  | .b _ => True
  | .bcond _ _ => True
  | .cbz _ _ _ => True
  | .bl _ => True
  | .bx _ => True

theorem eff_keeps_sp (i : Instr) (len : W) (s t : St) (h : eff i len s = some t)
    (hk : KeepsSp i) : t.sp = s.sp := by
  cases i with
  | cps d fi ff =>
    unfold eff at h; (try dsimp only at h)
    split at h <;> (rw [← Option.some.inj h])
  | push regs => exact False.elim hk
  | pop regs => exact False.elim hk
  | tst rn imm =>
    unfold eff at h; (try dsimp only at h)
    rw [← Option.some.inj h]
  | it fc m =>
    unfold eff at h; (try dsimp only at h)
    exact absurd h (by simp)
  | vpush d regs => exact False.elim hk
  | vpop d regs => exact False.elim hk
  | mrs rd sysm =>
    unfold eff at h; (try dsimp only at h)
    obtain ⟨v, _, e⟩ := map_some h
    rw [← e]
    exact setR_sp s rd v hk
  | msr sysm rn =>
    unfold eff at h; (try dsimp only at h)
    split at h
    · rw [← Option.some.inj h]
    · split at h
      · rw [← Option.some.inj h]
      · exact absurd h (by simp)
  | movr rd rm =>
    unfold eff at h; (try dsimp only at h)
    rw [← Option.some.inj h]
    exact setR_sp s rd _ hk
  | movi rd imm =>
    unfold eff at h; (try dsimp only at h)
    split at h <;> (rw [← Option.some.inj h]; exact setR_sp s rd imm hk)
  | subsp imm => exact False.elim hk
  | addsp imm => exact False.elim hk
  | addrsp rd imm =>
    unfold eff at h; (try dsimp only at h)
    rw [← Option.some.inj h]
    exact setR_sp s rd _ hk
  | strsp rt imm =>
    unfold eff at h; (try dsimp only at h)
    rw [← Option.some.inj h]
  | ldrsp rt imm =>
    unfold eff at h; (try dsimp only at h)
    rw [← Option.some.inj h]
    exact setR_sp s rt _ hk
  | cmpi rn imm =>
    unfold eff at h; (try dsimp only at h)
    rw [← Option.some.inj h]
  | dsb =>
    unfold eff at h; (try dsimp only at h)
    rw [← Option.some.inj h]
  | isb =>
    unfold eff at h; (try dsimp only at h)
    rw [← Option.some.inj h]
  | b off =>
    unfold eff at h; (try dsimp only at h)
    rw [← Option.some.inj h]
  | bcond c off =>
    unfold eff at h; (try dsimp only at h)
    exact absurd h (by simp)
  | cbz nz rn off =>
    unfold eff at h; (try dsimp only at h)
    rw [← Option.some.inj h]
  | bl off =>
    unfold eff at h; (try dsimp only at h)
    rw [← Option.some.inj h]
  | bx rm =>
    unfold eff at h; (try dsimp only at h)
    rw [← Option.some.inj h]

theorem eff_push_sp (regs : Nat) (len : W) (s t : St) (h : eff (.push regs) len s = some t) :
    t.sp = s.sp - BitVec.ofNat 32 (4 * (maskRegs regs).length) := by
  unfold eff at h; (try dsimp only at h)
  split at h
  · exact absurd h (by simp)
  · rw [← Option.some.inj h]

theorem eff_pop_sp (regs : Nat) (len : W) (s t : St) (h : eff (.pop regs) len s = some t) :
    t.sp = s.sp + BitVec.ofNat 32 (4 * (maskRegs regs).length) := by
  unfold eff at h; (try dsimp only at h)
  split at h
  · exact absurd h (by simp)
  · rw [← Option.some.inj h]

theorem eff_subsp_sp (imm : W) (len : W) (s t : St) (h : eff (.subsp imm) len s = some t) :
    t.sp = s.sp - imm := by
  unfold eff at h; (try dsimp only at h)
  rw [← Option.some.inj h]

theorem eff_addsp_sp (imm : W) (len : W) (s t : St) (h : eff (.addsp imm) len s = some t) :
    t.sp = s.sp + imm := by
  unfold eff at h; (try dsimp only at h)
  rw [← Option.some.inj h]

theorem eff_vpush_sp (d regs : Nat) (len : W) (s t : St)
    (h : eff (.vpush d regs) len s = some t) : t.sp = s.sp - BitVec.ofNat 32 (4 * regs) := by
  unfold eff at h; (try dsimp only at h)
  obtain ⟨u, hf, h⟩ := bind_some' h
  split at h
  · exact absurd h (by simp)
  · rw [← Option.some.inj h]
    show u.sp - _ = s.sp - _
    rw [fpCheck_sp hf]

theorem eff_vpop_sp (d regs : Nat) (len : W) (s t : St)
    (h : eff (.vpop d regs) len s = some t) : t.sp = s.sp + BitVec.ofNat 32 (4 * regs) := by
  unfold eff at h; (try dsimp only at h)
  obtain ⟨u, hf, h⟩ := bind_some' h
  split at h
  · exact absurd h (by simp)
  · rw [← Option.some.inj h]
    show u.sp + _ = s.sp + _
    rw [fpCheck_sp hf]

/-- A reading that neither moves SP nor is a dynamic write agrees with
every instruction whose operation leaves SP. -/
theorem sp_plain (i : Instr) (hk : KeepsSp i) (e : AEff)
    (he : (∀ n, e ≠ .spDown n) ∧ (∀ n, e ≠ .spUp n)) : SpAgrees e i := by
  intro len s t h
  have hs := eff_keeps_sp i len s t h hk
  cases e with
  | spDown n => exact absurd rfl (he.1 n)
  | spUp n => exact absurd rfl (he.2 n)
  | spDynamic => exact True.intro
  | spUnread => exact True.intro
  | call _ => exact hs
  | branch _ => exact hs
  | cbranch _ => exact hs
  | ret => exact hs
  | ibranch _ => exact hs
  | icall _ => exact hs
  | plain => exact hs

/-! ## Register lists -/

/-- The registers of a 16-bit `push`: the low byte's bits and LR. -/
theorem maskRegs_push16 (h : Nat) :
    (maskRegs ((h / 256 % 2) * 16384 + h % 256)).length = ones 16 (h % 256) + h / 256 % 2 := by
  unfold maskRegs
  rw [← ones_filter]
  have e0 : ((h / 256 % 2) * 16384 + h % 256) / 1 % 2 = h % 256 / 1 % 2 := by omega
  have e1 : ((h / 256 % 2) * 16384 + h % 256) / 2 % 2 = h % 256 / 2 % 2 := by omega
  have e2 : ((h / 256 % 2) * 16384 + h % 256) / 4 % 2 = h % 256 / 4 % 2 := by omega
  have e3 : ((h / 256 % 2) * 16384 + h % 256) / 8 % 2 = h % 256 / 8 % 2 := by omega
  have e4 : ((h / 256 % 2) * 16384 + h % 256) / 16 % 2 = h % 256 / 16 % 2 := by omega
  have e5 : ((h / 256 % 2) * 16384 + h % 256) / 32 % 2 = h % 256 / 32 % 2 := by omega
  have e6 : ((h / 256 % 2) * 16384 + h % 256) / 64 % 2 = h % 256 / 64 % 2 := by omega
  have e7 : ((h / 256 % 2) * 16384 + h % 256) / 128 % 2 = h % 256 / 128 % 2 := by omega
  have e8 : ((h / 256 % 2) * 16384 + h % 256) / 256 % 2 = 0 := by omega
  have e9 : ((h / 256 % 2) * 16384 + h % 256) / 512 % 2 = 0 := by omega
  have e10 : ((h / 256 % 2) * 16384 + h % 256) / 1024 % 2 = 0 := by omega
  have e11 : ((h / 256 % 2) * 16384 + h % 256) / 2048 % 2 = 0 := by omega
  have e12 : ((h / 256 % 2) * 16384 + h % 256) / 4096 % 2 = 0 := by omega
  have e13 : ((h / 256 % 2) * 16384 + h % 256) / 8192 % 2 = 0 := by omega
  have e14 : ((h / 256 % 2) * 16384 + h % 256) / 16384 % 2 = h / 256 % 2 := by omega
  have z8 : h % 256 / 256 % 2 = 0 := by omega
  have z9 : h % 256 / 512 % 2 = 0 := by omega
  have z10 : h % 256 / 1024 % 2 = 0 := by omega
  have z11 : h % 256 / 2048 % 2 = 0 := by omega
  have z12 : h % 256 / 4096 % 2 = 0 := by omega
  have z13 : h % 256 / 8192 % 2 = 0 := by omega
  have z14 : h % 256 / 16384 % 2 = 0 := by omega
  have z15 : h % 256 / 32768 % 2 = 0 := by omega
  norm_num [ones]
  omega

/-- The registers of a 32-bit `push` or `pop` whose bit 15 is clear. -/
theorem maskRegs_wide (b : Nat) (hb : b < 32768) : (maskRegs b).length = ones 16 b := by
  unfold maskRegs
  rw [← ones_filter]
  have z : b / 32768 % 2 = 0 := by omega
  norm_num [ones]
  omega

/-! ## Targets -/

/-- Two words are equal when their numbers are: the goal as arithmetic. -/
macro "bv_nat" : tactic =>
  `(tactic| (apply BitVec.eq_of_toNat_eq
             simp only [BitVec.toNat_add, BitVec.toNat_ofNat]
             (try norm_num)
             (try omega)))

theorem tgt_bv (pc : Nat) (imm : Int) (n : Nat) (hpc : pc < 1073741824)
    (hn : n < 4294967296) (himm : (n : Int) = imm ∨ (n : Int) = imm + 4294967296) :
    BitVec.ofNat 32 (tgt pc imm) = BitVec.ofNat 32 pc + 4#32 + BitVec.ofNat 32 n := by
  apply BitVec.eq_of_toNat_eq
  simp only [BitVec.toNat_add, BitVec.toNat_ofNat, tgt]
  (try norm_num)
  omega

/-! ## The 16-bit encodings -/

theorem d16_agrees (h pc : Nat) (hh : h < 65536) (hpc : pc < 1073741824) (i : Instr)
    (hd : decode16 h = some i) : SpAgrees (D16 h pc) i ∧ TargetAgrees (D16 h pc) i pc := by
  unfold decode16 at hd
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- cps
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .plain := by
      unfold D16
      eval_ifs
    rw [hD]
    exact ⟨sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- push
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .spDown (4 * (ones 16 (h % 256) + h / 256 % 2)) := by
      unfold D16
      eval_ifs
    rw [hD]
    refine ⟨?_, trivial⟩
    intro len s t ht
    have := eff_push_sp _ len s t ht
    rw [maskRegs_push16] at this
    exact this
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- it
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .plain := by
      unfold D16
      eval_ifs
    rw [hD]
    exact ⟨sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- mov
    dsimp only at hd
    rcases ite_some_cases hd with ⟨c2, hd⟩ | ⟨c2, hd⟩
    · exact absurd hd (by simp)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .plain := by
      unfold D16
      eval_ifs
    rw [hD]
    exact ⟨sp_plain _ (by show _ ≠ 13; omega) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- sub sp
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .spDown (h % 128 * 4) := by
      unfold D16
      eval_ifs
    rw [hD]
    exact ⟨fun len s t ht => eff_subsp_sp _ len s t ht, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- add sp
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .spUp (h % 128 * 4) := by
      unfold D16
      eval_ifs
    rw [hD]
    exact ⟨fun len s t ht => eff_addsp_sp _ len s t ht, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- addr sp
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .plain := by
      unfold D16
      eval_ifs
    rw [hD]
    exact ⟨sp_plain _ (by show _ ≠ 13; omega) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- str
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .plain := by
      unfold D16
      eval_ifs
    rw [hD]
    exact ⟨sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- ldr
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .plain := by
      unfold D16
      eval_ifs
    rw [hD]
    exact ⟨sp_plain _ (by show _ ≠ 13; omega) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- movs
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .plain := by
      unfold D16
      eval_ifs
    rw [hD]
    exact ⟨sp_plain _ (by show _ ≠ 13; omega) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- cbz, cbnz
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .cbranch (pc + 4 + ((h / 512 % 2) * 64 + (h / 8 % 32) * 2)) := by
      unfold D16
      eval_ifs
    rw [hD]
    refine ⟨sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, _, rfl, ?_⟩
    bv_nat
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- bcond
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .branch (tgt pc (off9 h)) := by
      unfold D16
      eval_ifs
    rw [hD]
    refine ⟨sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, _, rfl, ?_⟩
    refine tgt_bv pc (off9 h) _ hpc ?_ ?_
    · split <;> omega
    · by_cases cs : h % 256 < 128
      · left; simp only [off9, if_pos cs]; omega
      · right; simp only [off9, if_neg cs]; omega
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- b
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .branch (tgt pc (off12 h)) := by
      unfold D16
      eval_ifs
    rw [hD]
    refine ⟨sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, _, rfl, ?_⟩
    refine tgt_bv pc (off12 h) _ hpc ?_ ?_
    · split <;> omega
    · by_cases cs : h % 2048 < 1024
      · left; simp only [off12, if_pos cs]; omega
      · right; simp only [off12, if_neg cs]; omega
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- bx
    rcases ite_some_cases hd with ⟨c2, hd⟩ | ⟨c2, hd⟩
    · exact absurd hd (by simp)
    have e := Option.some.inj hd
    subst e
    by_cases hlr : h / 8 % 16 = 14
    · have hD : D16 h pc = .ret := by
        unfold D16
        eval_ifs
      rw [hD]
      refine ⟨sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, ?_⟩
      show (if h / 8 % 16 = 14 then AEff.ret = .ret else _)
      rw [if_pos hlr]
    · have hD : D16 h pc = .ibranch (h / 8 % 16) := by
        unfold D16
        eval_ifs
      rw [hD]
      refine ⟨sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, ?_⟩
      show (if h / 8 % 16 = 14 then _ else AEff.ibranch (h / 8 % 16) = .ibranch (h / 8 % 16))
      rw [if_neg hlr]
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- pop (T1, without the PC; 2026-11-27)
    have e := Option.some.inj hd
    subst e
    have hD : D16 h pc = .spUp (4 * ones 16 (h % 256)) := by
      unfold D16
      eval_ifs
    rw [hD]
    refine ⟨?_, trivial⟩
    intro len s t ht
    have := eff_pop_sp _ len s t ht
    rw [maskRegs_wide _ (by omega)] at this
    exact this
  exact absurd hd (by simp)

/-! ## The 32-bit encodings -/

/-- A byte rotated right by 8 to 31 places in 32 bits is the byte shifted
left by 32 minus the amount: what `thumb_expand_imm` computes. -/
theorem rot_shift (x r : Nat) (hx : x < 256) (h8 : 8 ≤ r) (hr : r < 32) :
    (BitVec.ofNat 32 x).rotateRight r = BitVec.ofNat 32 (x * 2 ^ (32 - r)) := by
  apply BitVec.eq_of_toNat_eq
  rw [BitVec.rotateRight_def]
  simp only [BitVec.toNat_or, BitVec.toNat_ushiftRight, BitVec.toNat_shiftLeft, BitVec.toNat_ofNat]
  have e1 : x % 2 ^ 32 = x := Nat.mod_eq_of_lt (by omega)
  have e2 : r % 32 = r := Nat.mod_eq_of_lt hr
  have e3 : x / 2 ^ r = 0 :=
    Nat.div_eq_of_lt (lt_of_lt_of_le hx (by
      calc 256 = 2 ^ 8 := by norm_num
        _ ≤ 2 ^ r := Nat.pow_le_pow_right (by omega) h8))
  rw [e1, e2, Nat.shiftRight_eq_div_pow, e3, Nat.zero_or, Nat.shiftLeft_eq]

/-- The decoder's expansion of a modified immediate is the manual's, where
the manual's has a value. -/
theorem expand12_agrees (n : Nat) (w : W) (h : expandImm n = some w) :
    BitVec.ofNat 32 (expand12 n) = w := by
  unfold expandImm at h
  unfold expand12
  by_cases c : n / 1024 % 4 = 0
  · rw [if_pos c] at h
    rw [if_pos c]
    by_cases c0 : n / 256 % 4 = 0
    · rw [if_pos c0] at h
      rw [if_pos c0]
      exact Option.some.inj h
    rw [if_neg c0] at h
    rw [if_neg c0]
    by_cases z : n % 256 = 0
    · rw [if_pos z] at h
      exact absurd h (by simp)
    rw [if_neg z] at h
    by_cases c1 : n / 256 % 4 = 1
    · rw [if_pos c1] at h
      rw [if_pos c1]
      exact Option.some.inj h
    rw [if_neg c1] at h
    rw [if_neg c1]
    by_cases c2 : n / 256 % 4 = 2
    · rw [if_pos c2] at h
      rw [if_pos c2]
      exact Option.some.inj h
    rw [if_neg c2] at h
    rw [if_neg c2]
    exact Option.some.inj h
  · rw [if_neg c] at h
    rw [if_neg c]
    rw [← Option.some.inj h]
    exact (rot_shift (128 + n % 128) (n / 128 % 32) (by omega) (by omega) (by omega)).symm

theorem imm12_eq (a b : Nat) : imm12 a b = imm12Of a b := rfl

/-- `Unread` of a first halfword 11110...: bit 15 of the second clear and
Rd = 13. -/
theorem unread_30 (a b : Nat) (h : a / 2048 = 30) :
    Unread a b ↔ (b / 4096 < 8 ∧ b / 256 % 16 = 13) := by
  unfold Unread
  rw [if_pos h]

/-- `Unread` of `ldr.w Rt, [sp, #imm12]`: the load to SP. -/
theorem unread_ldrsp (b : Nat) : Unread 0xF8DD b ↔ b / 4096 = 13 := by
  unfold Unread
  simp


-- One declaration with a case for each of the semantics' 32-bit forms:
-- the budget is per declaration.
set_option maxHeartbeats 4000000 in
theorem d32_agrees (a b pc : Nat) (ha : a < 65536) (hb : b < 65536) (hpc : pc < 1073741824)
    (i : Instr) (hd : decode32 a b = some i) :
    ∃ e, D32 a b pc = some e ∧ SpAgrees e i ∧ TargetAgrees e i pc := by
  unfold decode32 at hd
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- bl
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have nu : ¬ Unread a b := by
      rw [unread_30 a b (by omega)]
      omega
    have hD : D32 a b pc = some (.call (tgt pc (off25 a b))) := by
      unfold D32
      eval_ifs
    refine ⟨_, hD, sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, _, rfl, ?_⟩
    unfold brOff
    (try dsimp only)
    refine tgt_bv pc (off25 a b) _ hpc ?_ ?_
    · split_ifs <;> omega
    · unfold off25 raw25
      split_ifs <;> first | (left; omega) | (right; omega)
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- bw
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have nu : ¬ Unread a b := by
      rw [unread_30 a b (by omega)]
      omega
    have hD : D32 a b pc = some (.branch (tgt pc (off25 a b))) := by
      unfold D32
      eval_ifs
    refine ⟨_, hD, sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, _, rfl, ?_⟩
    unfold brOff
    (try dsimp only)
    refine tgt_bv pc (off25 a b) _ hpc ?_ ?_
    · split_ifs <;> omega
    · unfold off25 raw25
      split_ifs <;> first | (left; omega) | (right; omega)
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- push
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D32 a b pc = some (.spDown (4 * ones 16 b)) := by
      unfold D32
      eval_ifs
    refine ⟨_, hD, ?_, trivial⟩
    intro len s t ht
    have := eff_push_sp _ len s t ht
    rw [maskRegs_wide b (by omega)] at this
    exact this
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- pop
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D32 a b pc = some (.spUp (4 * ones 16 b)) := by
      unfold D32
      eval_ifs
    refine ⟨_, hD, ?_, trivial⟩
    intro len s t ht
    have := eff_pop_sp _ len s t ht
    rw [maskRegs_wide b (by omega)] at this
    exact this
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- tst
    obtain ⟨imm, _, hd⟩ := bind_some' hd
    (try dsimp only at hd)
    rcases ite_some_cases hd with ⟨c2, hd⟩ | ⟨c2, hd⟩
    · exact absurd hd (by simp)
    have e := Option.some.inj hd
    subst e
    have nu : ¬ Unread a b := by
      rw [unread_30 a b (by omega)]
      omega
    have hD : D32 a b pc = some (.plain) := by
      unfold D32
      eval_ifs
    exact ⟨_, hD, sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- cmp
    obtain ⟨imm, _, hd⟩ := bind_some' hd
    (try dsimp only at hd)
    rcases ite_some_cases hd with ⟨c2, hd⟩ | ⟨c2, hd⟩
    · exact absurd hd (by simp)
    have e := Option.some.inj hd
    subst e
    have nu : ¬ Unread a b := by
      rw [unread_30 a b (by omega)]
      omega
    have hD : D32 a b pc = some (.plain) := by
      unfold D32
      eval_ifs
    exact ⟨_, hD, sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- vpush
    (try dsimp only at hd)
    rcases ite_some_cases hd with ⟨c2, hd⟩ | ⟨c2, hd⟩
    · exact absurd hd (by simp)
    have e := Option.some.inj hd
    subst e
    have hD : D32 a b pc = some (.spDown (4 * (b % 256))) := by
      unfold D32
      eval_ifs
    exact ⟨_, hD, fun len s t ht => eff_vpush_sp _ _ len s t ht, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- vpop
    (try dsimp only at hd)
    rcases ite_some_cases hd with ⟨c2, hd⟩ | ⟨c2, hd⟩
    · exact absurd hd (by simp)
    have e := Option.some.inj hd
    subst e
    have hD : D32 a b pc = some (.spUp (4 * (b % 256))) := by
      unfold D32
      eval_ifs
    exact ⟨_, hD, fun len s t ht => eff_vpop_sp _ _ len s t ht, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- mrs
    rcases ite_some_cases hd with ⟨c2, hd⟩ | ⟨c2, hd⟩
    · exact absurd hd (by simp)
    have e := Option.some.inj hd
    subst e
    have hD : D32 a b pc = some (.plain) := by
      unfold D32
      eval_ifs
    exact ⟨_, hD, sp_plain _ (by show _ ≠ 13; omega) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- msr
    rcases ite_some_cases hd with ⟨c2, hd⟩ | ⟨c2, hd⟩
    · exact absurd hd (by simp)
    have e := Option.some.inj hd
    subst e
    have hD : D32 a b pc = some (.plain) := by
      unfold D32
      eval_ifs
    exact ⟨_, hD, sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- dsb
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D32 a b pc = some (.plain) := by
      unfold D32
      eval_ifs
    exact ⟨_, hD, sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- isb
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have nu : ¬ Unread a b := by
      rw [unread_30 a b (by omega)]
      omega
    have hD : D32 a b pc = some (.plain) := by
      unfold D32
      eval_ifs
    exact ⟨_, hD, sp_plain _ (by first | trivial | simp [KeepsSp]) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- ldr
    rcases ite_some_cases hd with ⟨c2, hd⟩ | ⟨c2, hd⟩
    · exact absurd hd (by simp)
    have e := Option.some.inj hd
    subst e
    have nu : ¬ Unread a b := by
      rw [c, unread_ldrsp]
      omega
    have hD : D32 a b pc = some (.plain) := by
      unfold D32
      eval_ifs
    exact ⟨_, hD, sp_plain _ (by show _ ≠ 13; omega) _ ⟨fun _ => by simp, fun _ => by simp⟩, trivial⟩
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- sub sp, sp, #const (T2)
    obtain ⟨w, hw, e⟩ := map_some hd
    subst e
    have hD : D32 a b pc = some (.spDown (expand12 (imm12 a b))) := by
      unfold D32
      eval_ifs
    refine ⟨_, hD, ?_, trivial⟩
    intro len s t ht
    have q := eff_subsp_sp _ len s t ht
    show t.sp = s.sp - BitVec.ofNat 32 (expand12 (imm12 a b))
    rw [q, expand12_agrees (imm12 a b) w (by rw [imm12_eq]; exact hw)]
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- subw sp, sp, #imm12 (T3)
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D32 a b pc = some (.spDown (imm12 a b)) := by
      unfold D32
      eval_ifs
    refine ⟨_, hD, ?_, trivial⟩
    intro len s t ht
    exact eff_subsp_sp _ len s t ht
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- add sp, sp, #const (T3)
    obtain ⟨w, hw, e⟩ := map_some hd
    subst e
    have hD : D32 a b pc = some (.spUp (expand12 (imm12 a b))) := by
      unfold D32
      eval_ifs
    refine ⟨_, hD, ?_, trivial⟩
    intro len s t ht
    have q := eff_addsp_sp _ len s t ht
    show t.sp = s.sp + BitVec.ofNat 32 (expand12 (imm12 a b))
    rw [q, expand12_agrees (imm12 a b) w (by rw [imm12_eq]; exact hw)]
  rcases ite_some_cases hd with ⟨c, hd⟩ | ⟨cn, hd⟩
  · -- addw sp, sp, #imm12 (T4)
    (try dsimp only at hd)
    have e := Option.some.inj hd
    subst e
    have hD : D32 a b pc = some (.spUp (imm12 a b)) := by
      unfold D32
      eval_ifs
    refine ⟨_, hD, ?_, trivial⟩
    intro len s t ht
    exact eff_addsp_sp _ len s t ht
  exact absurd hd (by simp)

end LinkThumb
