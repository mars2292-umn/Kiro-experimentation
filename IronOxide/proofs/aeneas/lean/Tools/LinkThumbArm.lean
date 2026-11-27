/-
  The Link_Checker's compiled Thumb decoder against the Armv7-M semantics
  (2026-11-26): the join of `Tools/LinkThumbProofs.lean` (the compiled
  `decode16_static` and `decode32_static` compute `D16` and `D32`) and
  `Tools/LinkThumbSpec.lean` (`D16` and `D32` agree with the semantics of
  `Sched/Arm.lean`).

  `decode16_arm`, `decode32_arm`: for every encoding the semantics' decoder
  accepts, at an address below 2^30, the compiled decoder returns, the
  stack-pointer change it reads is the change the instruction's operation
  makes in every state where the operation has a result, and the target it
  reads for a `bl`, `b`, conditional branch or `cbz` is the address the
  operation loads into the PC. `decode16_sp_step`, `decode32_sp_step`: the
  same for one step of the machine (`Arm.execI`), where an instruction
  whose condition fails in an IT block leaves SP.

  No encoding is excepted. Until 2026-11-26 two were, and this comparison
  is what found them: a `vpush`/`vpop` from an odd-numbered single register
  was read as leaving SP, and a `bl`/`b.w` whose halfwords the tests for an
  SP or `movw`/`movt` instruction accepted first was read as that
  instruction. Both are corrected in `crates/rsk-link/src/thumb.rs`.

  Not covered: the encodings the semantics does not have (the Link_Checker
  reads more forms than the stubs use: the 16-bit `pop`, the 32-bit
  `sub`/`add` of a constant to SP, the pre-indexed stores, `tbb`, the
  floating-point classes, `movw`/`movt` and literal loads as constants);
  the wrapper that reads a literal from the image (`decode16`, `decode32`,
  `resolve`: a closure); `decode_function`; the frame sum and the call
  graph (`graph.rs`); the region bounds (`stack.rs`). Handwritten.
-/
import Aeneas
import Tools.Link
import Tools.LinkThumbDefs
import Tools.LinkThumbSpec
import Tools.LinkThumbProofs
open Aeneas Aeneas.Std Result

namespace rsk_link
open thumb
open LinkThumb (AEff D16 D32 SpAgrees TargetAgrees)

/-- **The compiled 16-bit decoder and the semantics.** For a halfword the
semantics' decoder accepts, the compiled decoder returns, the stack-pointer
change it reads is the change the instruction's operation makes, and the
target it reads is the address the operation loads. -/
theorem decode16_arm (hw : U16) (pc : U32) (hpc : pc.val < 1073741824) (i : Arm.Instr)
    (hd : Arm.decode16 hw.val = some i) :
    decode16_static hw pc ⦃ d => SpAgrees (absD d) i ∧ TargetAgrees (absD d) i pc.val ⦄ := by
  refine WP.spec_mono (decode16_spec hw pc hpc) ?_
  intro d hdd
  rw [hdd]
  exact LinkThumb.d16_agrees hw.val pc.val (by scalar_tac) hpc i hd

/-- **The compiled 32-bit decoder and the semantics.** For a pair of
halfwords the semantics' decoder accepts, other than the two excepted
classes, the compiled decoder returns, the stack-pointer change it reads is
the change the instruction's operation makes, and the target it reads is
the address the operation loads. -/
theorem decode32_arm (hw1 hw2 : U16) (pc : U32) (hpc : pc.val < 1073741824) (i : Arm.Instr)
    (hd : Arm.decode32 hw1.val hw2.val = some i) :
    decode32_static hw1 hw2 pc ⦃ d =>
      SpAgrees (absD d) i ∧ TargetAgrees (absD d) i pc.val ⦄ := by
  refine WP.spec_mono (decode32_spec hw1 hw2 pc hpc) ?_
  intro d hdd
  obtain ⟨e, he, h1, h2⟩ :=
    LinkThumb.d32_agrees hw1.val hw2.val pc.val (by scalar_tac) (by scalar_tac) hpc i hd
  rw [hdd e he]
  exact ⟨h1, h2⟩

/-- What a reading says of the stack pointer across one step of the
machine: the change, or none when the instruction's condition failed. -/
def SpStep (e : AEff) (s t : Arm.St) : Prop :=
  match e with
  | .spDown n => t.sp = s.sp - BitVec.ofNat 32 n ∨ t.sp = s.sp
  | .spUp n => t.sp = s.sp + BitVec.ofNat 32 n ∨ t.sp = s.sp
  | .spDynamic => True
  | _ => t.sp = s.sp

/-- A step of the machine leaves SP, or does the instruction's operation
and then leaves SP. -/
theorem execI_sp (i : Arm.Instr) (len : Arm.W) (s t : Arm.St)
    (hx : Arm.execI i len s = some t) :
    t.sp = s.sp ∨ ∃ u, Arm.eff i len s = some u ∧ t.sp = u.sp := by
  have gen : ∀ (c : Prop) [Decidable c] (y : Arm.St),
      (if c then (Arm.eff i len s).map (fun u => { u with it := Arm.itAdvance s.it })
        else some y) = some t → y.sp = s.sp →
      t.sp = s.sp ∨ ∃ u, Arm.eff i len s = some u ∧ t.sp = u.sp := by
    intro c _ y h hy
    rcases LinkThumb.ite_some_cases h with ⟨_, h⟩ | ⟨_, h⟩
    · right
      obtain ⟨u, hu, e⟩ := LinkThumb.map_some h
      exact ⟨u, hu, by rw [← e]⟩
    · left; rw [← Option.some.inj h]; exact hy
  unfold Arm.execI at hx
  cases i
  case it fc m =>
    dsimp only at hx
    left; rw [← Option.some.inj hx]
  case bcond c off =>
    dsimp only at hx
    left
    rcases LinkThumb.ite_some_cases hx with ⟨_, hx⟩ | ⟨_, hx⟩
    · exact absurd hx (by simp)
    · rw [← Option.some.inj hx]
  all_goals
    (try dsimp only at hx)
    exact gen _ _ hx rfl

/-- From the operation to the step. -/
theorem sp_step (e : AEff) (i : Arm.Instr) (h : SpAgrees e i) (len : Arm.W) (s t : Arm.St)
    (hx : Arm.execI i len s = some t) : SpStep e s t := by
  rcases execI_sp i len s t hx with k | ⟨u, hu, k⟩
  · cases e with
    | spDown n => exact Or.inr k
    | spUp n => exact Or.inr k
    | spDynamic => exact True.intro
    | call _ => exact k
    | branch _ => exact k
    | cbranch _ => exact k
    | ret => exact k
    | ibranch _ => exact k
    | icall _ => exact k
    | plain => exact k
  · have hu' := h len s u hu
    cases e with
    | spDown n => exact Or.inl (k.trans hu')
    | spUp n => exact Or.inl (k.trans hu')
    | spDynamic => exact True.intro
    | call _ => exact k.trans hu'
    | branch _ => exact k.trans hu'
    | cbranch _ => exact k.trans hu'
    | ret => exact k.trans hu'
    | ibranch _ => exact k.trans hu'
    | icall _ => exact k.trans hu'
    | plain => exact k.trans hu'

/-- **One step of the machine, 16-bit.** For a halfword the semantics
decodes, whatever one step of the machine does to SP is what the compiled
decoder read: the change, or none. -/
theorem decode16_sp_step (hw : U16) (pc : U32) (hpc : pc.val < 1073741824) (i : Arm.Instr)
    (hd : Arm.decode16 hw.val = some i) :
    decode16_static hw pc ⦃ d =>
      ∀ s t, Arm.execI i 2#32 s = some t → SpStep (absD d) s t ⦄ := by
  refine WP.spec_mono (decode16_arm hw pc hpc i hd) ?_
  intro d hdd s t hx
  exact sp_step _ i hdd.1 _ s t hx

/-- **One step of the machine, 32-bit.** -/
theorem decode32_sp_step (hw1 hw2 : U16) (pc : U32) (hpc : pc.val < 1073741824)
    (i : Arm.Instr) (hd : Arm.decode32 hw1.val hw2.val = some i) :
    decode32_static hw1 hw2 pc ⦃ d =>
      ∀ s t, Arm.execI i 4#32 s = some t → SpStep (absD d) s t ⦄ := by
  refine WP.spec_mono (decode32_arm hw1 hw2 pc hpc i hd) ?_
  intro d hdd s t hx
  exact sp_step _ i hdd.1 _ s t hx

end rsk_link
