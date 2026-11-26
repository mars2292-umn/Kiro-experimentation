import Sched.World
import Sched.StubMask
import Sched.ArmExc

/-! # The exception model of the stubs, on the machine (2026-11-26)

The stub model assumes three things of the processor's exception entry
and return (`Sched/StubModel.lean`, `Sched/World.lean`). This file proves
them of `excEntry` and `excReturnHandler` (`Sched/ArmExc.lean`):

- `excEntry_hwEntry` (`hwEntry`, HW-06): after an entry, LR tells whether
  the exception was taken from Thread mode and on which stack; an entry
  from the process stack stacks nothing on the main stack; an entry that
  stacks on the main stack takes at least a basic frame from it.
- `excEntry_sim` (`exc_enter_handler`): an exception taken while a handler
  executes with CONTROL.FPCA clear, as at a stub's entry instruction: the
  machine state after the entry is the model's, for a frame of 32 or 36
  bytes and some values of what the model leaves open (`hx`).
- `excReturn_sim` (`exc_return_handler`, HW-07): the return to that
  handler gives the model's state.

Writing these found that the model's exception entry kept R0 to R3, R12
and the Z flag, which the architecture leaves UNKNOWN, and that its
return kept Z where the processor restores the flags from the stacked
xPSR. Both stub models were corrected the same day.

Premises: the exception is not the NMI or the HardFault, room for the
frame on the main stack, and CONTROL.FPCA clear at the entry and at the
return. -/

namespace Stub
open Arm

theorem ofNat_toNat_lt (q : Nat) (h : q < 4294967296) : (BitVec.ofNat 32 q).toNat = q := by
  rw [BitVec.toNat_ofNat]; exact Nat.mod_eq_of_lt h

theorem hasBit_nat (n mask : Nat) (hn : n < 4294967296) :
    hasBit ((n : Nat) : Int) mask = decide (n &&& mask ≠ 0) := by
  unfold hasBit
  rw [Int.toNat_natCast, Nat.mod_eq_of_lt hn]
  by_cases c : n &&& mask = 0 <;> simp [c]

/-- **HW-06 on the machine.** -/
theorem excEntry_hwEntry (x : ExcIn) (a : Arm.St) (hal : a.sp.toNat % 4 = 0)
    (hroom : onPsp x a = false → 108 ≤ a.sp.toNat) :
    P18.hwEntry (((excEntry x a).lr.toNat : Nat) : Int)
      (((a.sp.toNat : Nat) : Int) - (((excEntry x a).sp.toNat : Nat) : Int)) (onPsp x a) := by
  have hlt := a.sp.isLt
  unfold P18.hwEntry
  cases hp : onPsp x a
  · -- stacked on the main stack
    have hr := hroom hp
    have hfs : Arm.frameSize a = 32 ∨ Arm.frameSize a = 104 := by unfold Arm.frameSize; split <;> simp
    have hsp : spIn x a = a.sp := by unfold spIn; rw [hp]; rfl
    have e1 : (a.sp - BitVec.ofNat 32 (Arm.frameSize a)).toNat = a.sp.toNat - Arm.frameSize a :=
      toNat_sub_lit a.sp _ (by rcases hfs with h | h <;> omega)
    have hfp : (framePtr x a).toNat ≤ a.sp.toNat - Arm.frameSize a := by
      unfold framePtr
      rw [hsp, e1]
      split
      · rw [ofNat_toNat_lt _ (by omega)]; omega
      · rw [ofNat_toNat_lt _ (by omega)]
    have hsp' : (excEntry x a).sp = framePtr x a := by simp [excEntry, hp]
    rw [hsp']
    have hlr : (excEntry x a).lr = excReturn x a := rfl
    rw [hlr]
    unfold excReturn
    rw [hp]
    cases hf : fpcaSet a <;> cases ht : x.thread <;>
      simp only [Bool.false_eq_true, if_false, if_true] <;>
      (rw [ofNat_toNat_lt _ (by decide)]; refine ⟨by omega, ?_, ?_, ?_⟩ <;>
        first
          | (intro h8; exact absurd h8 (by decide))
          | (intro _ h4; exact absurd h4 (by decide))
          | (intro _; rcases hfs with h | h <;> omega)
          | (intro _; constructor <;> intro hh <;> first | exact absurd hh (by decide) | cases hh))
  · -- stacked on the process stack
    have ht : x.thread = true := by
      unfold onPsp at hp; cases hx : x.thread <;> simp_all
    have hsp' : (excEntry x a).sp = a.sp := by simp [excEntry, hp]
    rw [hsp']
    have hlr : (excEntry x a).lr = excReturn x a := rfl
    rw [hlr]
    unfold excReturn
    rw [hp, ht]
    cases hf : fpcaSet a <;>
      simp only [Bool.false_eq_true, if_false, if_true] <;>
      (rw [ofNat_toNat_lt _ (by decide)]; refine ⟨by omega, ?_, ?_, ?_⟩ <;>
        first
          | (intro _ _; omega)
          | (intro h4; exact absurd h4 (by decide))
          | (intro _; constructor <;> intro _ <;> first | rfl | decide))

theorem get8 (x0 x1 x2 x3 x4 x5 x6 x7 : W) (j : Nat) (hj : j < 8) :
    [x0, x1, x2, x3, x4, x5, x6, x7][j]! =
      if j = 0 then x0 else if j = 1 then x1 else if j = 2 then x2 else if j = 3 then x3
      else if j = 4 then x4 else if j = 5 then x5 else if j = 6 then x6 else x7 := by
  interval_cases j <;> rfl

set_option maxHeartbeats 4000000 in
/-- **An exception taken while a handler executes** (`exc_enter_handler`,
HW-06): with CONTROL.FPCA clear, room for the frame, and the exception
not the NMI or the HardFault, the machine state after the entry is the
model state, for a frame of 32 or 36 bytes and some `hx`. -/
theorem excEntry_sim {a : Arm.St} {m : M} (h : R 0 (fun _ => 0) a m) (x : ExcIn)
    (hth : x.thread = false) (hfp : a.control.toNat < 4) (hfat : x.fatal = false)
    (hroom : 36 ≤ a.sp.toNat) :
    ∃ (hx : M) (g : Int), (g = 32 ∨ g = 36) ∧
      R 0 (fun _ => 0) (excEntry x a) (exc_enter_handler m ((x.num.toNat : Nat) : Int) g hx) := by
  have hlt := a.sp.isLt
  have hal := h.spal
  have hcb := h.cb
  have hfs : fpcaSet a = false := by unfold fpcaSet; simp; omega
  have hop : onPsp x a = false := by unfold onPsp; rw [hth]; rfl
  have hsz : Arm.frameSize a = 32 := by unfold Arm.frameSize; rw [hfs]; rfl
  have hspi : spIn x a = a.sp := by unfold spIn; rw [hop]; rfl
  have e1 : (a.sp - BitVec.ofNat 32 32).toNat = a.sp.toNat - 32 := toNat_sub_lit a.sp 32 (by omega)
  -- the frame pointer: the stack pointer less 32, or less 36 when it is re-aligned
  obtain ⟨q, hq, hg⟩ : ∃ q, (framePtr x a).toNat = 4 * q ∧
      (a.sp.toNat = 4 * q + 32 ∨ a.sp.toNat = 4 * q + 36) := by
    unfold framePtr
    rw [hspi, hsz, e1]
    dsimp only
    by_cases hfa : forceAlign x a = true
    · rw [if_pos hfa, ofNat_toNat_lt _ (by omega)]
      refine ⟨(a.sp.toNat - 32) / 8 * 2, by omega, by omega⟩
    · rw [if_neg hfa, ofNat_toNat_lt _ (by omega)]
      exact ⟨(a.sp.toNat - 32) / 4, by omega, by omega⟩
  have hwords : frameWords x a = [a.r 0, a.r 1, a.r 2, a.r 3, a.r 12, a.lr, x.ret,
      BitVec.ofNat 32 (x.xpsr.toNat / 1024 * 1024 + (if frameAlign x a then 512 else 0) +
        x.xpsr.toNat % 512)] := by
    unfold frameWords; rw [hfs]; simp
  have hmem : (excEntry x a).mem = wrws a.mem (framePtr x a) (frameWords x a) := by
    simp only [excEntry]
  refine ⟨{ m with
      r := fun i => (((x.unk.r i).toNat : Nat) : Int), z := x.unk.z,
      mem := fun y => (((wv (excEntry x a).mem (y / 4).toNat).toNat : Nat) : Int) },
    ((a.sp.toNat : Nat) : Int) - (((framePtr x a).toNat : Nat) : Int), by omega, ?_⟩
  have hsp := h.sp
  refine ⟨?r, ?sp, ?lr, ?psp, ?control, ?ipsr, ?primask, ?faultmask, ?z, ?fp, ?mem, ?spal, ?hf,
    ?lsp, ?asp, ?cb⟩
  case r =>
    intro i hi
    show (if i ≤ 3 ∨ i = 12 then (((x.unk.r i).toNat : Nat) : Int) else m.r i) + 0 =
      (((if i ≤ 3 ∨ i = 12 then x.unk.r i else a.r i).toNat : Nat) : Int)
    by_cases c : i ≤ 3 ∨ i = 12
    · rw [if_pos c, if_pos c]; omega
    · rw [if_neg c, if_neg c]; have := h.r i hi; omega
  case sp =>
    show m.sp - _ = (((if onPsp x a then a.sp else framePtr x a).toNat : Nat) : Int)
    rw [hop, hsp]; simp only [Bool.false_eq_true, if_false]; omega
  case lr =>
    show (0xFFFFFFF1 : Int) = (((excReturn x a).toNat : Nat) : Int)
    unfold excReturn
    rw [hfs, hop, hth]
    rfl
  case psp =>
    show m.psp = (((if onPsp x a then framePtr x a else a.psp).toNat : Nat) : Int)
    rw [hop]; exact h.psp
  case control =>
    show m.control + 0 = (((BitVec.ofNat 32 (a.control.toNat % 2)).toNat : Nat) : Int)
    rw [ofNat_toNat_lt _ (by omega)]
    have := h.control
    omega
  case ipsr => rfl
  case primask => exact h.primask
  case faultmask => exact h.faultmask
  case z => rfl
  case fp => exact h.fp
  case spal =>
    show (if onPsp x a then a.sp else framePtr x a).toNat % 4 = 0
    rw [hop]; simp only [Bool.false_eq_true, if_false]; omega
  case hf =>
    show (a.hfnmi || x.fatal) = false
    rw [h.hf, hfat]; rfl
  case lsp => exact h.lsp
  case asp => exact h.asp
  case cb =>
    show (BitVec.ofNat 32 (a.control.toNat % 2)).toNat < 8 ∧
      (BitVec.ofNat 32 (a.control.toNat % 2)).toNat / 2 % 2 = 0
    rw [ofNat_toNat_lt _ (by omega)]
    omega
  case mem =>
    intro k hk
    have r0 := h.r 0 (by omega)
    have r1 := h.r 1 (by omega)
    have r2 := h.r 2 (by omega)
    have r3 := h.r 3 (by omega)
    have r12 := h.r 12 (by omega)
    have rl := h.lr
    have hm := h.mem k hk
    have hk4 : (4 * (k : Int) / 4).toNat = k := by omega
    show (if 4 * (k : Int) = m.sp - _ then m.r 0 else if 4 * (k : Int) = m.sp - _ + 4 then m.r 1
      else if 4 * (k : Int) = m.sp - _ + 8 then m.r 2
      else if 4 * (k : Int) = m.sp - _ + 12 then m.r 3
      else if 4 * (k : Int) = m.sp - _ + 16 then m.r 12
      else if 4 * (k : Int) = m.sp - _ + 20 then m.lr
      else if m.sp - _ ≤ 4 * (k : Int) ∧ 4 * (k : Int) < m.sp then
        (((wv (excEntry x a).mem (4 * (k : Int) / 4).toNat).toNat : Nat) : Int)
      else m.mem (4 * (k : Int))) = _
    rw [hk4]
    generalize hW : wv (excEntry x a).mem k = Wk
    rw [hmem, hwords, wv_wrws _ a.mem _ q hq (by simp; omega) k hk] at hW
    simp only [List.length_cons, List.length_nil] at hW
    by_cases c : q ≤ k ∧ k < q + 8
    · rw [if_pos c] at hW
      obtain ⟨j, hj⟩ : ∃ j, k = q + j := ⟨k - q, by omega⟩
      have hj8 : j < 8 := by omega
      subst hj
      rw [show q + j - q = j by omega] at hW
      interval_cases j
      · have e : a.r 0 = Wk := hW
        subst e; split_ifs <;> omega
      · have e : a.r 1 = Wk := hW
        subst e; split_ifs <;> omega
      · have e : a.r 2 = Wk := hW
        subst e; split_ifs <;> omega
      · have e : a.r 3 = Wk := hW
        subst e; split_ifs <;> omega
      · have e : a.r 12 = Wk := hW
        subst e; split_ifs <;> omega
      · have e : a.lr = Wk := hW
        subst e; split_ifs <;> omega
      · split_ifs <;> omega
      · split_ifs <;> omega
    · rw [if_neg c] at hW
      subst hW
      split_ifs <;> omega

theorem bit30 (n : Nat) : (n &&& 1073741824 ≠ 0) ↔ n / 1073741824 % 2 = 1 := by
  have e : (1073741824 : Nat) = 2 ^ 30 := by decide
  rw [e, Nat.and_two_pow]
  have t : n.testBit 30 = decide (n / 2 ^ 30 % 2 = 1) := by
    rw [Nat.testBit, Nat.shiftRight_eq_div_pow, Nat.and_comm, Nat.and_one_is_mod]
    by_cases c : n / 2 ^ 30 % 2 = 1
    · rw [c]; rfl
    · have c0 : n / 2 ^ 30 % 2 = 0 := by omega
      rw [c0]; rfl
  rw [t]
  by_cases c : n / 2 ^ 30 % 2 = 1 <;> simp [c]

set_option maxHeartbeats 4000000 in
/-- **The exception return to a handler** (`exc_return_handler`, HW-07):
with EXC_RETURN 0xFFFFFFF1 in the PC, CONTROL.FPCA clear, and the
exception returned from not the NMI, the machine state after the return is
the model state, for a frame of 32 or 36 bytes and the IPSR of the
stacked xPSR. -/
theorem excReturn_sim {a : Arm.St} {m : M} (h : R 0 (fun _ => 0) a m) (stkalign : Bool)
    (hpc : a.pc = 0xFFFFFFF1#32) (hfp : a.control.toNat < 4) (hn : a.ipsr ≠ 2#32)
    (hroom : a.sp.toNat + 36 < 4294967296) :
    ∃ (a' : Arm.St) (g : Int), excReturnHandler stkalign a = some a' ∧ (g = 32 ∨ g = 36) ∧
      R 0 (fun _ => 0) a'
        (exc_return_handler m g
          ((((rdw a.mem (a.sp + BitVec.ofNat 32 28)).toNat % 512 : Nat)) : Int)) := by
  have hal := h.spal
  have hlt := a.sp.isLt
  have hcb := h.cb
  have hsp := h.sp
  obtain ⟨q, hq⟩ : ∃ q, a.sp.toNat = 4 * q := ⟨a.sp.toNat / 4, by omega⟩
  have rd : ∀ j, j < 8 → rdw a.mem (a.sp + BitVec.ofNat 32 (4 * j)) = wv a.mem (q + j) := by
    intro j hj
    apply rdw_wv
    rw [toNat_add_lit a.sp (4 * j) (by omega)]
    omega
  have mw : ∀ j : Nat, j < 8 → m.mem (m.sp + 4 * (j : Int)) = (((wv a.mem (q + j)).toNat : Nat) : Int) := by
    intro j hj
    rw [← h.mem (q + j) (by omega)]
    congr 1
    rw [hsp]; omega
  have hc : ¬ (a.pc ≠ 0xFFFFFFF1#32 ∨ a.sp.toNat % 4 ≠ 0) := by
    rw [hpc]; simp; omega
  unfold excReturnHandler
  rw [if_neg hc]
  dsimp only
  -- the alignment word
  generalize hadj : (if stkalign = true ∧
      (rdw a.mem (a.sp + BitVec.ofNat 32 (4 * 7))).toNat / 512 % 2 = 1 then 4 else 0 : Nat) = adj
  have hadj' : adj = 0 ∨ adj = 4 := by rw [← hadj]; split <;> simp
  have esp : (a.sp + BitVec.ofNat 32 (32 + adj)).toNat = a.sp.toNat + 32 + adj := by
    rw [toNat_add_lit a.sp (32 + adj) (by omega)]; omega
  refine ⟨_, (32 + adj : Nat), rfl, by omega, ?_⟩
  refine ⟨?g_r, ?g_sp, ?g_lr, ?g_psp, ?g_control, ?g_ipsr, ?g_primask, ?g_faultmask, ?g_z, ?g_fp,
    ?g_mem, ?g_spal, ?g_hf, ?g_lsp, ?g_asp, ?g_cb⟩
  case g_r =>
    intro i hi
    show (if i = 0 then m.mem m.sp else if i = 1 then m.mem (m.sp + 4)
      else if i = 2 then m.mem (m.sp + 8) else if i = 3 then m.mem (m.sp + 12)
      else if i = 12 then m.mem (m.sp + 16) else m.r i) + 0 =
      (((if i = 0 then rdw a.mem (a.sp + BitVec.ofNat 32 (4 * 0))
        else if i = 1 then rdw a.mem (a.sp + BitVec.ofNat 32 (4 * 1))
        else if i = 2 then rdw a.mem (a.sp + BitVec.ofNat 32 (4 * 2))
        else if i = 3 then rdw a.mem (a.sp + BitVec.ofNat 32 (4 * 3))
        else if i = 12 then rdw a.mem (a.sp + BitVec.ofNat 32 (4 * 4)) else a.r i).toNat : Nat) : Int)
    have w0 : m.mem m.sp = (((wv a.mem (q + 0)).toNat : Nat) : Int) := by
      have := mw 0 (by omega); simpa using this
    have w1 : m.mem (m.sp + 4) = (((wv a.mem (q + 1)).toNat : Nat) : Int) := by
      have := mw 1 (by omega); simpa using this
    have w2 : m.mem (m.sp + 8) = (((wv a.mem (q + 2)).toNat : Nat) : Int) := by
      have := mw 2 (by omega); simpa using this
    have w3 : m.mem (m.sp + 12) = (((wv a.mem (q + 3)).toNat : Nat) : Int) := by
      have := mw 3 (by omega); simpa using this
    have w4 : m.mem (m.sp + 16) = (((wv a.mem (q + 4)).toNat : Nat) : Int) := by
      have := mw 4 (by omega); simpa using this
    have hr := h.r i hi
    rw [rd 0 (by omega), rd 1 (by omega), rd 2 (by omega), rd 3 (by omega), rd 4 (by omega)]
    split_ifs <;> omega
  case g_sp =>
    show m.sp + ((32 + adj : Nat) : Int) = _
    rw [hsp]
    have : (a.sp + BitVec.ofNat 32 (32 + adj)).toNat = a.sp.toNat + 32 + adj := esp
    show _ = (((a.sp + BitVec.ofNat 32 (32 + adj)).toNat : Nat) : Int)
    rw [this]; omega
  case g_lr =>
    show m.mem (m.sp + 20) = (((rdw a.mem (a.sp + BitVec.ofNat 32 (4 * 5))).toNat : Nat) : Int)
    rw [rd 5 (by omega), ← mw 5 (by omega)]
    rfl
  case g_psp => exact h.psp
  case g_control =>
    show m.control + 0 = (((BitVec.ofNat 32 (a.control.toNat % 4)).toNat : Nat) : Int)
    rw [ofNat_toNat_lt _ (by omega)]
    have := h.control
    omega
  case g_ipsr =>
    show _ = (((BitVec.ofNat 32 ((rdw a.mem (a.sp + BitVec.ofNat 32 (4 * 7))).toNat % 512)).toNat : Nat) : Int)
    rw [ofNat_toNat_lt _ (by omega)]
    rfl
  case g_primask => exact h.primask
  case g_faultmask =>
    show false = (if a.ipsr = 2#32 then a.faultmask else false)
    rw [if_neg hn]
  case g_z =>
    show hasBit (m.mem (m.sp + 28)) 0x40000000 =
      decide ((rdw a.mem (a.sp + BitVec.ofNat 32 (4 * 7))).toNat / 1073741824 % 2 = 1)
    have w7 := mw 7 (by omega)
    have e7 : m.mem (m.sp + 28) = (((wv a.mem (q + 7)).toNat : Nat) : Int) := by
      rw [← w7]; rfl
    rw [e7, rd 7 (by omega), hasBit_nat _ _ (wv a.mem (q + 7)).isLt]
    by_cases c : (wv a.mem (q + 7)).toNat / 1073741824 % 2 = 1
    · simp [c, (bit30 _).2 c]
    · have : ¬ ((wv a.mem (q + 7)).toNat &&& 1073741824 ≠ 0) := fun hh => c ((bit30 _).1 hh)
      simp [c, this]
  case g_fp => exact h.fp
  case g_mem => exact h.mem
  case g_spal =>
    show (a.sp + BitVec.ofNat 32 (32 + adj)).toNat % 4 = 0
    rw [esp]; omega
  case g_hf => exact h.hf
  case g_lsp => exact h.lsp
  case g_asp => exact h.asp
  case g_cb =>
    show (BitVec.ofNat 32 (a.control.toNat % 4)).toNat < 8 ∧
      (BitVec.ofNat 32 (a.control.toNat % 4)).toNat / 2 % 2 = 0
    rw [ofNat_toNat_lt _ (by omega)]
    omega

end Stub
