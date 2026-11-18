import Hw.Drivers
open Aeneas Aeneas.Std Result

namespace Hw
open rsk_kernel rsk_kernel.arch.cm4 rsk_kernel.arch.cm4.unsafe_module.hw

theorem bind_eq_ok {α β} (x : Result α) (f : α → Result β) (y : β) :
    Std.bind x f = ok y ↔ ∃ a, x = ok a ∧ f a = ok y := by
  cases x <;> simp

theorem ok_inj {α} {x y : α} (h : (ok x : Result α) = ok y) : x = y := by
  simpa using h

theorem u32_add_ok {a b c : U32} (h : a + b = ok c) : c.val = a.val + b.val := by
  have := UScalar.add_equiv a b
  simp_all

theorem u32_sub_ok {a b c : U32} (h : a - b = ok c) : b.val ≤ a.val ∧ c.val = a.val - b.val := by
  have := UScalar.sub_equiv a b
  simp_all

theorem u32_mul_ok {a b c : U32} (h : a * b = ok c) : c.val = a.val * b.val := by
  have e : a * b = UScalar.mul a b := rfl
  rw [e] at h
  have := UScalar.mul_equiv a b
  simp_all

theorem u32_div32_ok {a c : U32} (h : a / 32#u32 = ok c) : c.val = a.val / 32 := by
  obtain ⟨z, hz, hv⟩ := UScalar.div_spec a (y := 32#u32) (by decide)
  rw [hz] at h
  have h := ok_inj h
  subst h
  simpa using hv

theorem u32_rem32_ok {a c : U32} (h : a % 32#u32 = ok c) : c.val = a.val % 32 := by
  have hs := UScalar.rem_spec a (y := 32#u32) (by decide)
  rw [h] at hs
  simpa [Aeneas.Std.WP.spec_ok] using hs

theorem u32_shl_ok {x s m : U32} (h : x <<< s = ok m) : s.val < 32 ∧ m.bv = x.bv <<< s.val := by
  simp only [HShiftLeft.hShiftLeft, UScalar.shiftLeft_UScalar, UScalar.shiftLeft] at h
  split at h
  · have h := ok_inj h
    subst h
    exact ⟨by assumption, rfl⟩
  · simp at h

/-- The state the hardware keeps: the last value written to each plain
register, the NVIC's enabled and pending lines by word (32 lines each), the
time base's interrupt enable, and the NVIC's priority bytes. -/
structure RegFile where
  mem : Nat → U32
  enabled : Nat → U32
  pending : Nat → U32
  inten : U32
  prio : Nat → U8

/-- The word index a set or clear register at `a` addresses, for the 16
registers from `base`. -/
def word (a base : Nat) : Option Nat :=
  if base ≤ a ∧ a < base + 64 ∧ (a - base) % 4 = 0 then some ((a - base) / 4) else none

def ISER : Nat := 0xE000E100
def ICER : Nat := 0xE000E180
def ISPR : Nat := 0xE000E200
def ICPR : Nat := 0xE000E280
def INTENSET : Nat := 0x40008304
def INTENCLR : Nat := 0x40008308

/-- A word write (HW-05; nRF52840 PS 6.30): the NVIC's set registers set
the written bits of their word and its clear registers clear them, for the
enabled and for the pending lines; TIMER0's INTENSET and INTENCLR do the
same for its interrupt enable; any other register keeps the value. -/
def RegFile.write (s : RegFile) (a v : U32) : RegFile :=
  match word a.val ISER with
  | some w => { s with enabled := Function.update s.enabled w (s.enabled w ||| v) }
  | none =>
  match word a.val ICER with
  | some w => { s with enabled := Function.update s.enabled w (s.enabled w &&& ~~~v) }
  | none =>
  match word a.val ISPR with
  | some w => { s with pending := Function.update s.pending w (s.pending w ||| v) }
  | none =>
  match word a.val ICPR with
  | some w => { s with pending := Function.update s.pending w (s.pending w &&& ~~~v) }
  | none =>
  if a.val = INTENSET then { s with inten := s.inten ||| v }
  else if a.val = INTENCLR then { s with inten := s.inten &&& ~~~v }
  else { s with mem := Function.update s.mem a.val v }

/-- The register model as the drivers' register file. -/
def model : drivers.Regs RegFile where
  read := fun s a => ok (s.mem a.val, s)
  write := fun s a v => ok (s.write a v)
  write_u8 := fun s a v => ok { s with prio := Function.update s.prio a.val v }

/-! ## The constants -/

theorem nvic_lines : addr.NVIC_LINES = 496#u32 := by unfold addr.NVIC_LINES; rfl
theorem a_iser : addr.NVIC_ISER.val = ISER := by unfold addr.NVIC_ISER; rfl
theorem a_icer : addr.NVIC_ICER.val = ICER := by unfold addr.NVIC_ICER; rfl
theorem a_ispr : addr.NVIC_ISPR.val = ISPR := by unfold addr.NVIC_ISPR; rfl
theorem a_icpr : addr.NVIC_ICPR.val = ICPR := by unfold addr.NVIC_ICPR; rfl

/-! ## The line of an exception number -/

/-- `irq_of`: an exception number from 16 up to the last implemented line
is its line, the number less 16; any other number is no line. -/
theorem irq_of_spec {n : U16} {o : Option U32} (h : drivers.irq_of n = ok o) :
    (o = none ∧ (n.val < 16 ∨ 496 ≤ n.val - 16)) ∨
    ∃ irq : U32, o = some irq ∧ 16 ≤ n.val ∧ irq.val = n.val - 16 ∧ irq.val < 496 := by
  unfold drivers.irq_of at h
  simp only [nvic_lines] at h
  have hc : (UScalar.cast .U32 n).val = n.val := by scalar_tac
  split at h
  · have h := ok_inj h
    exact Or.inl ⟨h.symm, Or.inl (by scalar_tac)⟩
  · simp only [bind_eq_ok, lift, bind_ok] at h
    obtain ⟨i1, hi1, h⟩ := h
    obtain ⟨hle, hv⟩ := u32_sub_ok hi1
    split at h
    · have h := ok_inj h
      exact Or.inl ⟨h.symm, Or.inr (by scalar_tac)⟩
    · simp only [bind_eq_ok] at h
      obtain ⟨i3, hi3, h⟩ := h
      have h := ok_inj h
      obtain ⟨_, hv3⟩ := u32_sub_ok hi3
      exact Or.inr ⟨i3, h.symm, by scalar_tac, by scalar_tac, by scalar_tac⟩

/-! ## The set and clear registers' words -/

theorem word_hit (base w : Nat) (hw : w < 16) : word (base + 4 * w) base = some w := by
  unfold word
  have h1 : base + 4 * w - base = 4 * w := by omega
  rw [h1]
  have h2 : 4 * w % 4 = 0 := by omega
  have h3 : 4 * w / 4 = w := by omega
  simp [h2, h3]
  omega

theorem word_miss (a base : Nat) (h : a < base ∨ base + 64 ≤ a) : word a base = none := by
  unfold word
  have : ¬ (base ≤ a ∧ a < base + 64 ∧ (a - base) % 4 = 0) := by omega
  simp [this]

/-! ## The NVIC's lines -/

/-- The word and the bit the four line drivers compute for a line: the
register of word `irq / 32` from `base`, and the value `1 <<< (irq % 32)`. -/
theorem line_write {base irq i i1 i2 i3 i4 : U32} {w : Nat} (hw : w = irq.val / 32)
    (hlt : irq.val < 496)
    (hi : irq / 32#u32 = ok i) (hi1 : 4#u32 * i = ok i1) (hi2 : base + i1 = ok i2)
    (hi3 : irq % 32#u32 = ok i3) (hi4 : 1#u32 <<< i3 = ok i4) :
    i2.val = base.val + 4 * w ∧ w < 16 ∧ i4.bv = 1#32 <<< (irq.val % 32) := by
  have e1 := u32_div32_ok hi
  have e2 := u32_mul_ok hi1
  have e3 := u32_add_ok hi2
  have e4 := u32_rem32_ok hi3
  obtain ⟨_, e5⟩ := u32_shl_ok hi4
  have h4 : (4#u32).val = 4 := rfl
  refine ⟨by rw [e3, e2, e1, h4, hw], by omega, ?_⟩
  rw [e5, e4]
  rfl

/-- `nvic_enable` (`Action::Enable`): a number that names a line sets that
line's bit in the enabled lines and changes nothing else; any other number
changes nothing. -/
theorem nvic_enable_spec {s s' : RegFile} {n : U16}
    (h : drivers.nvic_enable model s n = ok s') :
    (s' = s ∧ (n.val < 16 ∨ 496 ≤ n.val - 16)) ∨
    ∃ mask : U32, 16 ≤ n.val ∧ n.val - 16 < 496 ∧ mask.bv = 1#32 <<< ((n.val - 16) % 32) ∧
      s' = { s with enabled :=
        (Function.update s.enabled ((n.val - 16) / 32) (s.enabled ((n.val - 16) / 32) ||| mask)) } := by
  unfold drivers.nvic_enable at h
  simp only [bind_eq_ok] at h
  obtain ⟨o, ho, h⟩ := h
  rcases irq_of_spec ho with ⟨rfl, hn⟩ | ⟨irq, rfl, h16, hv, hlt⟩
  · exact Or.inl ⟨(ok_inj h).symm, hn⟩
  · simp only [bind_eq_ok] at h
    obtain ⟨i, hi, i1, hi1, i2, hi2, i3, hi3, i4, hi4, h⟩ := h
    obtain ⟨ea, hw16, em⟩ := line_write rfl hlt hi hi1 hi2 hi3 hi4
    have h := ok_inj h
    rw [a_iser] at ea
    have hword : word i2.val ISER = some (irq.val / 32) := by rw [ea]; exact word_hit ISER _ hw16
    right
    refine ⟨i4, h16, by omega, by rw [em, hv], ?_⟩
    subst h
    unfold RegFile.write
    simp only [hword, hv]

/-- `nvic_disable` (`Action::Disable`): a number that names a line clears that line's bit in the enabled lines and changes nothing else; any other number changes nothing. -/
theorem nvic_disable_spec {s s' : RegFile} {n : U16}
    (h : drivers.nvic_disable model s n = ok s') :
    (s' = s ∧ (n.val < 16 ∨ 496 ≤ n.val - 16)) ∨
    ∃ mask : U32, 16 ≤ n.val ∧ n.val - 16 < 496 ∧ mask.bv = 1#32 <<< ((n.val - 16) % 32) ∧
      s' = { s with enabled :=
        (Function.update s.enabled ((n.val - 16) / 32) (s.enabled ((n.val - 16) / 32) &&& ~~~mask)) } := by
  unfold drivers.nvic_disable at h
  simp only [bind_eq_ok] at h
  obtain ⟨o, ho, h⟩ := h
  rcases irq_of_spec ho with ⟨rfl, hn⟩ | ⟨irq, rfl, h16, hv, hlt⟩
  · exact Or.inl ⟨(ok_inj h).symm, hn⟩
  · simp only [bind_eq_ok] at h
    obtain ⟨i, hi, i1, hi1, i2, hi2, i3, hi3, i4, hi4, h⟩ := h
    obtain ⟨ea, hw16, em⟩ := line_write rfl hlt hi hi1 hi2 hi3 hi4
    have h := ok_inj h
    rw [a_icer] at ea
    have hm0 : word i2.val ISER = none := by
      rw [ea]; exact word_miss _ _ (Or.inr (by unfold ISER ICER; omega))
    have hword : word i2.val ICER = some (irq.val / 32) := by rw [ea]; exact word_hit ICER _ hw16
    right
    refine ⟨i4, h16, by omega, by rw [em, hv], ?_⟩
    subst h
    unfold RegFile.write
    simp only [hm0, hword, hv]

/-- `nvic_set_pending` (`Action::Pend`): a number that names a line sets that line's bit in the pending lines and changes nothing else; any other number changes nothing. -/
theorem nvic_set_pending_spec {s s' : RegFile} {n : U16}
    (h : drivers.nvic_set_pending model s n = ok s') :
    (s' = s ∧ (n.val < 16 ∨ 496 ≤ n.val - 16)) ∨
    ∃ mask : U32, 16 ≤ n.val ∧ n.val - 16 < 496 ∧ mask.bv = 1#32 <<< ((n.val - 16) % 32) ∧
      s' = { s with pending :=
        (Function.update s.pending ((n.val - 16) / 32) (s.pending ((n.val - 16) / 32) ||| mask)) } := by
  unfold drivers.nvic_set_pending at h
  simp only [bind_eq_ok] at h
  obtain ⟨o, ho, h⟩ := h
  rcases irq_of_spec ho with ⟨rfl, hn⟩ | ⟨irq, rfl, h16, hv, hlt⟩
  · exact Or.inl ⟨(ok_inj h).symm, hn⟩
  · simp only [bind_eq_ok] at h
    obtain ⟨i, hi, i1, hi1, i2, hi2, i3, hi3, i4, hi4, h⟩ := h
    obtain ⟨ea, hw16, em⟩ := line_write rfl hlt hi hi1 hi2 hi3 hi4
    have h := ok_inj h
    rw [a_ispr] at ea
    have hm0 : word i2.val ISER = none := by
      rw [ea]; exact word_miss _ _ (Or.inr (by unfold ISER ISPR; omega))
    have hm1 : word i2.val ICER = none := by
      rw [ea]; exact word_miss _ _ (Or.inr (by unfold ICER ISPR; omega))
    have hword : word i2.val ISPR = some (irq.val / 32) := by rw [ea]; exact word_hit ISPR _ hw16
    right
    refine ⟨i4, h16, by omega, by rw [em, hv], ?_⟩
    subst h
    unfold RegFile.write
    simp only [hm0, hm1, hword, hv]

/-- `nvic_clear_pending` (`Action::ClearPending`): a number that names a line clears that line's bit in the pending lines and changes nothing else; any other number changes nothing. -/
theorem nvic_clear_pending_spec {s s' : RegFile} {n : U16}
    (h : drivers.nvic_clear_pending model s n = ok s') :
    (s' = s ∧ (n.val < 16 ∨ 496 ≤ n.val - 16)) ∨
    ∃ mask : U32, 16 ≤ n.val ∧ n.val - 16 < 496 ∧ mask.bv = 1#32 <<< ((n.val - 16) % 32) ∧
      s' = { s with pending :=
        (Function.update s.pending ((n.val - 16) / 32) (s.pending ((n.val - 16) / 32) &&& ~~~mask)) } := by
  unfold drivers.nvic_clear_pending at h
  simp only [bind_eq_ok] at h
  obtain ⟨o, ho, h⟩ := h
  rcases irq_of_spec ho with ⟨rfl, hn⟩ | ⟨irq, rfl, h16, hv, hlt⟩
  · exact Or.inl ⟨(ok_inj h).symm, hn⟩
  · simp only [bind_eq_ok] at h
    obtain ⟨i, hi, i1, hi1, i2, hi2, i3, hi3, i4, hi4, h⟩ := h
    obtain ⟨ea, hw16, em⟩ := line_write rfl hlt hi hi1 hi2 hi3 hi4
    have h := ok_inj h
    rw [a_icpr] at ea
    have hm0 : word i2.val ISER = none := by
      rw [ea]; exact word_miss _ _ (Or.inr (by unfold ISER ICPR; omega))
    have hm1 : word i2.val ICER = none := by
      rw [ea]; exact word_miss _ _ (Or.inr (by unfold ICER ICPR; omega))
    have hm2 : word i2.val ISPR = none := by
      rw [ea]; exact word_miss _ _ (Or.inr (by unfold ISPR ICPR; omega))
    have hword : word i2.val ICPR = some (irq.val / 32) := by rw [ea]; exact word_hit ICPR _ hw16
    right
    refine ⟨i4, h16, by omega, by rw [em, hv], ?_⟩
    subst h
    unfold RegFile.write
    simp only [hm0, hm1, hm2, hword, hv]

/-- `nvic_set_priority` (`Action::SetPriority`): a number that names a line
writes that line's priority byte and nothing else; any other number changes
nothing. -/
theorem nvic_set_priority_spec {s s' : RegFile} {n : U16} {p : U8}
    (h : drivers.nvic_set_priority model s n p = ok s') :
    (s' = s ∧ (n.val < 16 ∨ 496 ≤ n.val - 16)) ∨
    (16 ≤ n.val ∧ n.val - 16 < 496 ∧
      s' = { s with prio := (Function.update s.prio (0xE000E400 + (n.val - 16)) p) }) := by
  unfold drivers.nvic_set_priority at h
  simp only [bind_eq_ok] at h
  obtain ⟨o, ho, h⟩ := h
  rcases irq_of_spec ho with ⟨rfl, hn⟩ | ⟨irq, rfl, h16, hv, hlt⟩
  · exact Or.inl ⟨(ok_inj h).symm, hn⟩
  · simp only [bind_eq_ok] at h
    obtain ⟨i, hi, h⟩ := h
    have e := u32_add_ok hi
    have ha : addr.NVIC_IPR.val = 0xE000E400 := by unfold addr.NVIC_IPR; rfl
    have h := ok_inj h
    right
    refine ⟨h16, by omega, ?_⟩
    subst h
    rw [e, ha, hv]

/-! ## The plain registers -/

/-- A write to a register that is none of the NVIC's set and clear
registers and neither of TIMER0's interrupt-enable registers keeps the
value written and changes nothing else. -/
theorem write_plain (s : RegFile) (a v : U32)
    (h : (a.val < 0xE000E100 ∨ 0xE000E2C0 ≤ a.val) ∧ a.val ≠ 0x40008304 ∧ a.val ≠ 0x40008308) :
    s.write a v = { s with mem := Function.update s.mem a.val v } := by
  unfold RegFile.write
  have h1 : word a.val ISER = none := word_miss _ _ (by unfold ISER; omega)
  have h2 : word a.val ICER = none := word_miss _ _ (by unfold ICER; omega)
  have h3 : word a.val ISPR = none := word_miss _ _ (by unfold ISPR; omega)
  have h4 : word a.val ICPR = none := word_miss _ _ (by unfold ICPR; omega)
  have h5 : ¬ a.val = INTENSET := by unfold INTENSET; omega
  have h6 : ¬ a.val = INTENCLR := by unfold INTENCLR; omega
  simp only [h1, h2, h3, h4, h5, h6, if_false]

theorem a_ccr : addr.CCR.val = 0xE000ED14 := by unfold addr.CCR; rfl

/-- `set_nonbasethrdena` (`Action::Nonbasethrdena`): CCR's bit 0 is set or
cleared and every other bit of the register, and every other register, is
as it was. The driver never fails. -/
theorem set_nonbasethrdena_spec (s : RegFile) (on : Bool) :
    drivers.set_nonbasethrdena model s on = ok { s with mem :=
      (Function.update s.mem 0xE000ED14
        (if on then s.mem 0xE000ED14 ||| 1#u32 else s.mem 0xE000ED14 &&& ~~~1#u32)) } := by
  unfold drivers.set_nonbasethrdena
  have hp := write_plain s addr.CCR
  simp only [a_ccr] at hp
  cases on <;> simp [model, lift, a_ccr, hp]

/-! ## The plain registers the configuration drivers write -/

theorem a_aircr : addr.AIRCR.val = 3758157068 := by unfold addr.AIRCR; rfl
theorem w_aircr (s : RegFile) (v : U32) :
    s.write addr.AIRCR v = { s with mem := (Function.update s.mem 3758157068 v) } := by
  rw [write_plain s _ v (by rw [a_aircr]; omega), a_aircr]

theorem a_actlr : addr.ACTLR.val = 3758153736 := by unfold addr.ACTLR; rfl
theorem w_actlr (s : RegFile) (v : U32) :
    s.write addr.ACTLR v = { s with mem := (Function.update s.mem 3758153736 v) } := by
  rw [write_plain s _ v (by rw [a_actlr]; omega), a_actlr]

theorem a_shpr1 : addr.SHPR1.val = 3758157080 := by unfold addr.SHPR1; rfl
theorem w_shpr1 (s : RegFile) (v : U32) :
    s.write addr.SHPR1 v = { s with mem := (Function.update s.mem 3758157080 v) } := by
  rw [write_plain s _ v (by rw [a_shpr1]; omega), a_shpr1]

theorem a_shpr2 : addr.SHPR2.val = 3758157084 := by unfold addr.SHPR2; rfl
theorem w_shpr2 (s : RegFile) (v : U32) :
    s.write addr.SHPR2 v = { s with mem := (Function.update s.mem 3758157084 v) } := by
  rw [write_plain s _ v (by rw [a_shpr2]; omega), a_shpr2]

theorem a_shpr3 : addr.SHPR3.val = 3758157088 := by unfold addr.SHPR3; rfl
theorem w_shpr3 (s : RegFile) (v : U32) :
    s.write addr.SHPR3 v = { s with mem := (Function.update s.mem 3758157088 v) } := by
  rw [write_plain s _ v (by rw [a_shpr3]; omega), a_shpr3]

theorem a_shcsr : addr.SHCSR.val = 3758157092 := by unfold addr.SHCSR; rfl
theorem w_shcsr (s : RegFile) (v : U32) :
    s.write addr.SHCSR v = { s with mem := (Function.update s.mem 3758157092 v) } := by
  rw [write_plain s _ v (by rw [a_shcsr]; omega), a_shcsr]

theorem a_fpccr : addr.FPCCR.val = 3758157620 := by unfold addr.FPCCR; rfl
theorem w_fpccr (s : RegFile) (v : U32) :
    s.write addr.FPCCR v = { s with mem := (Function.update s.mem 3758157620 v) } := by
  rw [write_plain s _ v (by rw [a_fpccr]; omega), a_fpccr]

theorem a_fpdscr : addr.FPDSCR.val = 3758157628 := by unfold addr.FPDSCR; rfl
theorem w_fpdscr (s : RegFile) (v : U32) :
    s.write addr.FPDSCR v = { s with mem := (Function.update s.mem 3758157628 v) } := by
  rw [write_plain s _ v (by rw [a_fpdscr]; omega), a_fpdscr]

theorem a_cpacr : addr.CPACR.val = 3758157192 := by unfold addr.CPACR; rfl
theorem w_cpacr (s : RegFile) (v : U32) :
    s.write addr.CPACR v = { s with mem := (Function.update s.mem 3758157192 v) } := by
  rw [write_plain s _ v (by rw [a_cpacr]; omega), a_cpacr]

theorem a_mpu_ctrl : addr.MPU_CTRL.val = 3758157204 := by unfold addr.MPU_CTRL; rfl
theorem w_mpu_ctrl (s : RegFile) (v : U32) :
    s.write addr.MPU_CTRL v = { s with mem := (Function.update s.mem 3758157204 v) } := by
  rw [write_plain s _ v (by rw [a_mpu_ctrl]; omega), a_mpu_ctrl]

theorem a_demcr : addr.DEMCR.val = 3758157308 := by unfold addr.DEMCR; rfl
theorem w_demcr (s : RegFile) (v : U32) :
    s.write addr.DEMCR v = { s with mem := (Function.update s.mem 3758157308 v) } := by
  rw [write_plain s _ v (by rw [a_demcr]; omega), a_demcr]

theorem a_dwt_cyccnt : addr.DWT_CYCCNT.val = 3758100484 := by unfold addr.DWT_CYCCNT; rfl
theorem w_dwt_cyccnt (s : RegFile) (v : U32) :
    s.write addr.DWT_CYCCNT v = { s with mem := (Function.update s.mem 3758100484 v) } := by
  rw [write_plain s _ v (by rw [a_dwt_cyccnt]; omega), a_dwt_cyccnt]

theorem a_dwt_ctrl : addr.DWT_CTRL.val = 3758100480 := by unfold addr.DWT_CTRL; rfl
theorem w_dwt_ctrl (s : RegFile) (v : U32) :
    s.write addr.DWT_CTRL v = { s with mem := (Function.update s.mem 3758100480 v) } := by
  rw [write_plain s _ v (by rw [a_dwt_ctrl]; omega), a_dwt_ctrl]

theorem w_ccr (s : RegFile) (v : U32) :
    s.write addr.CCR v = { s with mem := (Function.update s.mem 0xE000ED14 v) } := by
  rw [write_plain s _ v (by rw [a_ccr]; omega), a_ccr]

/-! ## The shifts by a constant -/

theorem shl_1_16 : (1#u32 <<< 16#i32 : Result U32) = ok 65536#u32 := by
  first
    | rfl
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; rfl)
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; decide)

theorem shl_1_17 : (1#u32 <<< 17#i32 : Result U32) = ok 131072#u32 := by
  first
    | rfl
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; rfl)
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; decide)

theorem shl_1_18 : (1#u32 <<< 18#i32 : Result U32) = ok 262144#u32 := by
  first
    | rfl
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; rfl)
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; decide)

theorem shl_1_2 : (1#u32 <<< 2#i32 : Result U32) = ok 4#u32 := by
  first
    | rfl
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; rfl)
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; decide)

theorem shl_1_24 : (1#u32 <<< 24#i32 : Result U32) = ok 16777216#u32 := by
  first
    | rfl
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; rfl)
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; decide)

theorem shl_1_30 : (1#u32 <<< 30#i32 : Result U32) = ok 1073741824#u32 := by
  first
    | rfl
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; rfl)
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; decide)

theorem shl_1_31 : (1#u32 <<< 31#i32 : Result U32) = ok 2147483648#u32 := by
  first
    | rfl
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; rfl)
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; decide)

theorem shl_15_20 : (15#u32 <<< 20#i32 : Result U32) = ok 15728640#u32 := by
  first
    | rfl
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; rfl)
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; decide)

theorem shl_1530_16 : (1530#u32 <<< 16#i32 : Result U32) = ok 100270080#u32 := by
  first
    | rfl
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; rfl)
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; decide)

theorem shl_4_8 : (4#u32 <<< 8#i32 : Result U32) = ok 1024#u32 := by
  first
    | rfl
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; rfl)
    | (simp [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]; decide)

/-! ## The configuration drivers

Each is an equation: on the model the driver returns, for every state, the
state with the register it names changed as stated and nothing else. -/

/-- `set_prigroup_all_preempt` (`Action::PrigroupAllPreempt`): AIRCR gets
the write key, PRIGROUP 4 and the bits of the mask 0xF8FF it had. -/
theorem set_prigroup_all_preempt_spec (s : RegFile) :
    drivers.set_prigroup_all_preempt model s = ok { s with mem :=
      (Function.update s.mem 3758157068
        (100270080#u32 ||| (s.mem 3758157068 &&& 63743#u32) ||| 1024#u32)) } := by
  unfold drivers.set_prigroup_all_preempt
  simp [model, lift, a_aircr, w_aircr, shl_1530_16, shl_4_8]

/-- `set_disdefwbuf` (`Action::Disdefwbuf`): ACTLR's bit 1 set or cleared. -/
theorem set_disdefwbuf_spec (s : RegFile) (on : Bool) :
    drivers.set_disdefwbuf model s on = ok { s with mem :=
      (Function.update s.mem 3758153736
        (if on then s.mem 3758153736 ||| 2#u32 else s.mem 3758153736 &&& ~~~2#u32)) } := by
  unfold drivers.set_disdefwbuf
  cases on <;> simp [model, lift, a_actlr, w_actlr]

/-- `set_kernel_exception_priorities` (`Action::KernelExceptionPriorities`):
the three system-handler priority registers are zero and SHCSR has its
bits 16 to 18 set beside what it held. -/
theorem set_kernel_exception_priorities_spec (s : RegFile) :
    drivers.set_kernel_exception_priorities model s = ok { s with mem :=
      (Function.update (Function.update (Function.update (Function.update s.mem
        3758157080 0#u32) 3758157084 0#u32) 3758157088 0#u32) 3758157092
        (s.mem 3758157092 ||| 65536#u32 ||| 131072#u32 ||| 262144#u32)) } := by
  unfold drivers.set_kernel_exception_priorities
  simp [model, lift, a_shcsr, w_shpr1, w_shpr2, w_shpr3, w_shcsr, shl_1_16, shl_1_17, shl_1_18,
    Function.update_apply]

/-- `fpu_configure_eager` (`Action::FpuEager`): FPCCR has ASPEN (bit 31)
set and LSPEN (bit 30) clear beside what it held, FPDSCR is the
flush-to-zero bit or zero, and CPACR has the CP10 and CP11 fields set
beside what it held. -/
theorem fpu_configure_eager_spec (s : RegFile) (fz : Bool) :
    drivers.fpu_configure_eager model s fz = ok { s with mem :=
      (Function.update (Function.update (Function.update s.mem
        3758157620 ((s.mem 3758157620 ||| 2147483648#u32) &&& ~~~1073741824#u32))
        3758157628 (if fz then 16777216#u32 else 0#u32))
        3758157192 (s.mem 3758157192 ||| 15728640#u32)) } := by
  unfold drivers.fpu_configure_eager
  cases fz <;>
    simp [model, lift, a_fpccr, a_cpacr, w_fpccr, w_fpdscr, w_cpacr, shl_1_31, shl_1_30, shl_1_24,
      shl_15_20, Function.update_apply]

/-- `mpu_enable_background` (`Action::MpuEnableBackground`): MPU_CTRL is
ENABLE and PRIVDEFENA. -/
theorem mpu_enable_background_spec (s : RegFile) :
    drivers.mpu_enable_background model s = ok { s with mem :=
      (Function.update s.mem 3758157204 (4#u32 ||| 1#u32)) } := by
  unfold drivers.mpu_enable_background
  simp [model, lift, w_mpu_ctrl, shl_1_2]

/-- `dwt_enable` (`Action::DwtEnable`): DEMCR has TRCENA (bit 24) set
beside what it held, the cycle counter is zero, and DWT_CTRL has its
enable bit set beside what it held. -/
theorem dwt_enable_spec (s : RegFile) :
    drivers.dwt_enable model s = ok { s with mem :=
      (Function.update (Function.update (Function.update s.mem
        3758157308 (s.mem 3758157308 ||| 16777216#u32))
        3758100484 0#u32)
        3758100480 (s.mem 3758100480 ||| 1#u32)) } := by
  unfold drivers.dwt_enable
  simp [model, lift, a_demcr, a_dwt_ctrl, w_demcr, w_dwt_cyccnt, w_dwt_ctrl, shl_1_24,
    Function.update_apply]

/-! ## TIMER0: the time base's compare -/

/-- The state with one plain register, the interrupt enable, or word 0 of
the pending lines replaced. -/
@[reducible] def RegFile.setMem (s : RegFile) (k : Nat) (v : U32) : RegFile :=
  { s with mem := Function.update s.mem k v }
@[reducible] def RegFile.setInten (s : RegFile) (v : U32) : RegFile := { s with inten := v }
@[reducible] def RegFile.setPending0 (s : RegFile) (v : U32) : RegFile :=
  { s with pending := Function.update s.pending 0 v }

def T_CAP : Nat := 0x40008044
def T_EVT : Nat := 0x40008140
def T_CC0 : Nat := 0x40008540
def T_CC1 : Nat := 0x40008544

theorem a_timer0 : addr.NRF_TIMER0.val = 0x40008000 := by unfold addr.NRF_TIMER0; rfl
theorem t_cap : drivers.timer0.TASKS_CAPTURE1.val = 0x44 := by unfold drivers.timer0.TASKS_CAPTURE1; rfl
theorem t_evt : drivers.timer0.EVENTS_COMPARE0.val = 0x140 := by unfold drivers.timer0.EVENTS_COMPARE0; rfl
theorem t_set : drivers.timer0.INTENSET.val = 0x304 := by unfold drivers.timer0.INTENSET; rfl
theorem t_clr : drivers.timer0.INTENCLR.val = 0x308 := by unfold drivers.timer0.INTENCLR; rfl
theorem t_cc0 : drivers.timer0.CC0.val = 0x540 := by unfold drivers.timer0.CC0; rfl
theorem t_cc1 : drivers.timer0.CC1.val = 0x544 := by unfold drivers.timer0.CC1; rfl

/-- A write to a plain register of TIMER0, by its address. -/
theorem write_timer (s : RegFile) (a v : U32) (k : Nat) (hk : a.val = k)
    (hp : 0x40008000 ≤ k ∧ k < 0x40009000 ∧ k ≠ 0x40008304 ∧ k ≠ 0x40008308) :
    s.write a v = { s with mem := (Function.update s.mem k v) } := by
  rw [write_plain s a v (by omega), hk]

theorem write_intenset (s : RegFile) (a v : U32) (hk : a.val = 0x40008304) :
    s.write a v = { s with inten := s.inten ||| v } := by
  unfold RegFile.write
  have e : a.val = INTENSET := by unfold INTENSET; omega
  have h1 : word INTENSET ISER = none := word_miss _ _ (by unfold INTENSET ISER; omega)
  have h2 : word INTENSET ICER = none := word_miss _ _ (by unfold INTENSET ICER; omega)
  have h3 : word INTENSET ISPR = none := word_miss _ _ (by unfold INTENSET ISPR; omega)
  have h4 : word INTENSET ICPR = none := word_miss _ _ (by unfold INTENSET ICPR; omega)
  simp only [e, h1, h2, h3, h4, if_true]

theorem write_intenclr (s : RegFile) (a v : U32) (hk : a.val = 0x40008308) :
    s.write a v = { s with inten := s.inten &&& ~~~v } := by
  unfold RegFile.write
  have e : a.val = INTENCLR := by unfold INTENCLR; omega
  have h1 : word INTENCLR ISER = none := word_miss _ _ (by unfold INTENCLR ISER; omega)
  have h2 : word INTENCLR ICER = none := word_miss _ _ (by unfold INTENCLR ICER; omega)
  have h3 : word INTENCLR ISPR = none := word_miss _ _ (by unfold INTENCLR ISPR; omega)
  have h4 : word INTENCLR ICPR = none := word_miss _ _ (by unfold INTENCLR ICPR; omega)
  have h5 : ¬ INTENCLR = INTENSET := by unfold INTENCLR INTENSET; omega
  simp only [e, h1, h2, h3, h4, h5, if_true, if_false]

/-- `timer0::now`: the capture task is written, the capture register read,
and the reading extended. The model keeps no counter: the reading is
whatever the capture register holds, and the theorems below hold for any
reading. -/
theorem now_spec {E : Type _} (ei : drivers.Extend E) {s s' : RegFile} {e e' : E} {t : U64}
    (h : drivers.timer0.now model ei s e = ok (t, s', e')) :
    s' = { s with mem := (Function.update s.mem T_CAP 1#u32) } ∧
    ei.extend e (s.mem T_CC1) = ok (t, e') := by
  unfold drivers.timer0.now at h
  simp only [bind_eq_ok, Prod.exists, Aeneas.Std.uncurry] at h
  obtain ⟨i, hi, r1, hr1, i1, hi1, low, r2, hrd, t2, e1, hext, h⟩ := h
  have ei1 := u32_add_ok hi
  have ei2 := u32_add_ok hi1
  rw [a_timer0, t_cap] at ei1
  rw [a_timer0, t_cc1] at ei2
  have hr1 := ok_inj hr1
  have hrd := ok_inj hrd
  have h := ok_inj h
  rw [write_timer s i 1#u32 T_CAP (by rw [ei1]; rfl) (by unfold T_CAP; omega)] at hr1
  simp only [Prod.mk.injEq] at h hrd
  obtain ⟨ht, hs, he⟩ := h
  obtain ⟨hl, hr⟩ := hrd
  subst_vars
  refine ⟨rfl, ?_⟩
  have hm : (Function.update s.mem T_CAP 1#u32) i1.val = s.mem T_CC1 := by
    rw [ei2]
    unfold T_CAP T_CC1
    simp [Function.update_apply]
  simp only [] at hext
  rw [hm] at hext
  exact hext

theorem vector24 : drivers.timer0.TIMER0_VECTOR = ok 24#u16 := by
  unfold drivers.timer0.TIMER0_VECTOR
  first
    | rfl
    | (have h := UScalar.add_equiv 16#u16 8#u16; simp_all; rfl)
    | scalar_tac

theorem pend_line8 {s s' : RegFile} (h : drivers.nvic_set_pending model s 24#u16 = ok s') :
    ∃ m : U32, m.bv = 1#32 <<< 8 ∧
      s' = { s with pending := (Function.update s.pending 0 (s.pending 0 ||| m)) } := by
  have e : (24#u16).val = 24 := rfl
  rcases nvic_set_pending_spec h with ⟨_, hn⟩ | ⟨m, _, _, hm, hs⟩
  · exfalso
    rw [e] at hn
    omega
  · rw [e] at hm hs
    exact ⟨m, by simpa using hm, by simpa using hs⟩

theorem clear_line8 {s s' : RegFile} (h : drivers.nvic_clear_pending model s 24#u16 = ok s') :
    ∃ m : U32, m.bv = 1#32 <<< 8 ∧
      s' = { s with pending := (Function.update s.pending 0 (s.pending 0 &&& ~~~m)) } := by
  have e : (24#u16).val = 24 := rfl
  rcases nvic_clear_pending_spec h with ⟨_, hn⟩ | ⟨m, _, _, hm, hs⟩
  · exfalso
    rw [e] at hn
    omega
  · rw [e] at hm hs
    exact ⟨m, by simpa using hm, by simpa using hs⟩

/-- `timer0::ack` (`Action::TimebaseAck`): the compare event is cleared. -/
theorem ack_spec {s s' : RegFile} (h : drivers.timer0.ack model s = ok s') :
    s' = { s with mem := (Function.update s.mem T_EVT 0#u32) } := by
  unfold drivers.timer0.ack at h
  simp only [bind_eq_ok] at h
  obtain ⟨i, hi, h⟩ := h
  have ei := u32_add_ok hi
  rw [a_timer0, t_evt] at ei
  have h := ok_inj h
  rw [write_timer s i 0#u32 T_EVT (by rw [ei]; rfl) (by unfold T_EVT; omega)] at h
  exact h.symm

/-- `timer0::arm` (`Action::ArmCompare`): the compare register holds the
instant's low word and the compare interrupt is enabled beside what was;
the time base is read once after the writes; when that reading has reached
the instant, the time base's line (8) is set pending, and otherwise the
pending lines are as they were. So the compare is armed, or the interrupt
is pending: no due instant is left to the counter's wrap. No line's
enable bit and no priority byte changes. -/
theorem arm_spec {E : Type _} (ei : drivers.Extend E) {s s' : RegFile} {e e' : E} {at_ post : U64}
    (h : drivers.timer0.arm model ei s e at_ = ok (post, s', e')) :
    ei.extend e (s.mem T_CC1) = ok (post, e') ∧
    ((post.val < at_.val ∧
        s' = (((s.setMem T_CC0 (UScalar.cast .U32 at_)).setInten (s.inten ||| 65536#u32)).setMem
          T_CAP 1#u32)) ∨
     (at_.val ≤ post.val ∧ ∃ m : U32, m.bv = 1#32 <<< 8 ∧
        s' = ((((s.setMem T_CC0 (UScalar.cast .U32 at_)).setInten (s.inten ||| 65536#u32)).setMem
          T_CAP 1#u32).setPending0 (s.pending 0 ||| m)))) := by
  unfold drivers.timer0.arm at h
  simp only [shl_1_16, vector24, lift, bind_ok] at h
  simp only [bind_eq_ok, Prod.exists, Aeneas.Std.uncurry] at h
  obtain ⟨i, hi, r1, hr1, i2, hi2, r2, hr2, p, r3, e1, hnow, h⟩ := h
  have ei0 := u32_add_ok hi
  rw [a_timer0, t_cc0] at ei0
  have ei2 := u32_add_ok hi2
  rw [a_timer0, t_set] at ei2
  have hr1 := ok_inj hr1
  have hr2 := ok_inj hr2
  rw [write_timer s i _ T_CC0 (by rw [ei0]; rfl) (by unfold T_CC0; omega)] at hr1
  subst hr1
  rw [write_intenset _ i2 _ (by omega)] at hr2
  subst hr2
  obtain ⟨hs3, hext⟩ := now_spec ei hnow
  have hm : (Function.update s.mem T_CC0 (UScalar.cast .U32 at_)) T_CC1 = s.mem T_CC1 := by
    unfold T_CC0 T_CC1
    simp [Function.update_apply]
  simp only [hm] at hext
  split at h
  · rename_i hge
    simp only [bind_eq_ok] at h
    obtain ⟨r4, hp, h⟩ := h
    have h := ok_inj h
    simp only [Prod.mk.injEq] at h
    obtain ⟨rfl, rfl, rfl⟩ := h
    obtain ⟨m, hmb, hr4⟩ := pend_line8 hp
    refine ⟨hext, Or.inr ⟨by scalar_tac, m, hmb, ?_⟩⟩
    rw [hr4, hs3]
  · rename_i hlt
    have h := ok_inj h
    simp only [Prod.mk.injEq] at h
    obtain ⟨rfl, rfl, rfl⟩ := h
    exact ⟨hext, Or.inl ⟨by scalar_tac, hs3⟩⟩

/-- `timer0::cancel` (`Action::CancelCompare`): the compare interrupt is
disabled, the compare event cleared, and the pending bit of the time
base's line cleared; nothing else changes. -/
theorem cancel_spec {s s' : RegFile} (h : drivers.timer0.cancel model s = ok s') :
    ∃ m : U32, m.bv = 1#32 <<< 8 ∧
      s' = (((s.setInten (s.inten &&& ~~~65536#u32)).setMem T_EVT 0#u32).setPending0
        (s.pending 0 &&& ~~~m)) := by
  unfold drivers.timer0.cancel at h
  simp only [shl_1_16, vector24, lift, bind_ok] at h
  simp only [bind_eq_ok, Prod.exists, Aeneas.Std.uncurry] at h
  obtain ⟨i, hi, r1, hr1, i2, hi2, r2, hr2, x, r3, hrd, h⟩ := h
  have ei0 := u32_add_ok hi
  rw [a_timer0, t_clr] at ei0
  have ei2 := u32_add_ok hi2
  rw [a_timer0, t_evt] at ei2
  have hr1 := ok_inj hr1
  have hr2 := ok_inj hr2
  have hrd := ok_inj hrd
  rw [write_intenclr s i _ (by omega)] at hr1
  subst hr1
  rw [write_timer _ i2 _ T_EVT (by rw [ei2]; rfl) (by unfold T_EVT; omega)] at hr2
  subst hr2
  simp only [Prod.mk.injEq] at hrd
  obtain ⟨_, rfl⟩ := hrd
  obtain ⟨m, hmb, hs⟩ := clear_line8 h
  exact ⟨m, hmb, hs⟩

/-- `timer0::rearm` (`Action::RearmCompare`): the compare interrupt is
disabled while the compare register is given the instant's low word and
the compare event and the pending bit of the time base's line are
cleared, then enabled again; the time base is read once after the writes,
and when that reading has reached the instant the line is set pending.
So no match of the earlier compare value is left, and the new compare is
armed or the interrupt is pending. No line's enable bit and no priority
byte changes. -/
theorem rearm_spec {E : Type _} (ei : drivers.Extend E) {s s' : RegFile} {e e' : E}
    {at_ post : U64} (h : drivers.timer0.rearm model ei s e at_ = ok (post, s', e')) :
    ei.extend e (s.mem T_CC1) = ok (post, e') ∧
    ∃ m1 : U32, m1.bv = 1#32 <<< 8 ∧
    ((post.val < at_.val ∧
        s' = ((((((s.setInten (s.inten &&& ~~~65536#u32)).setMem T_CC0
          (UScalar.cast .U32 at_)).setMem T_EVT 0#u32).setPending0 (s.pending 0 &&& ~~~m1)).setInten
          ((s.inten &&& ~~~65536#u32) ||| 65536#u32)).setMem T_CAP 1#u32)) ∨
     (at_.val ≤ post.val ∧ ∃ m2 : U32, m2.bv = 1#32 <<< 8 ∧
        s' = (((((((s.setInten (s.inten &&& ~~~65536#u32)).setMem T_CC0
          (UScalar.cast .U32 at_)).setMem T_EVT 0#u32).setPending0 (s.pending 0 &&& ~~~m1)).setInten
          ((s.inten &&& ~~~65536#u32) ||| 65536#u32)).setMem T_CAP 1#u32).setPending0
          ((s.pending 0 &&& ~~~m1) ||| m2)))) := by
  unfold drivers.timer0.rearm at h
  simp only [shl_1_16, vector24, lift, bind_ok] at h
  simp only [bind_eq_ok, Prod.exists, Aeneas.Std.uncurry] at h
  obtain ⟨i, hi, r1, hr1, i2, hi2, r2, hr2, i4, hi4, r3, hr3, x, r4, hrd, r5, hcl, i6, hi6, r6, hr6,
    p, r7, e1, hnow, h⟩ := h
  have ei0 := u32_add_ok hi
  rw [a_timer0, t_clr] at ei0
  have ei2 := u32_add_ok hi2
  rw [a_timer0, t_cc0] at ei2
  have ei4 := u32_add_ok hi4
  rw [a_timer0, t_evt] at ei4
  have ei6 := u32_add_ok hi6
  rw [a_timer0, t_set] at ei6
  have hr1 := ok_inj hr1
  have hr2 := ok_inj hr2
  have hr3 := ok_inj hr3
  have hrd := ok_inj hrd
  have hr6 := ok_inj hr6
  rw [write_intenclr s i _ (by omega)] at hr1
  subst hr1
  rw [write_timer _ i2 _ T_CC0 (by rw [ei2]; rfl) (by unfold T_CC0; omega)] at hr2
  subst hr2
  rw [write_timer _ i4 _ T_EVT (by rw [ei4]; rfl) (by unfold T_EVT; omega)] at hr3
  subst hr3
  simp only [Prod.mk.injEq] at hrd
  obtain ⟨_, rfl⟩ := hrd
  obtain ⟨m1, hm1, hs5⟩ := clear_line8 hcl
  subst hs5
  rw [write_intenset _ i6 _ (by omega)] at hr6
  subst hr6
  obtain ⟨hs7, hext⟩ := now_spec ei hnow
  have hm : (Function.update (Function.update s.mem T_CC0 (UScalar.cast .U32 at_)) T_EVT 0#u32) T_CC1
      = s.mem T_CC1 := by
    unfold T_CC0 T_CC1 T_EVT
    simp [Function.update_apply]
  simp only [hm] at hext
  refine ⟨?_, m1, hm1, ?_⟩
  · split at h
    · simp only [bind_eq_ok] at h
      obtain ⟨r8, _, h⟩ := h
      have h := ok_inj h
      simp only [Prod.mk.injEq] at h
      obtain ⟨rfl, _, rfl⟩ := h
      exact hext
    · have h := ok_inj h
      simp only [Prod.mk.injEq] at h
      obtain ⟨rfl, _, rfl⟩ := h
      exact hext
  · split at h
    · simp only [bind_eq_ok] at h
      obtain ⟨r8, hp, h⟩ := h
      have h := ok_inj h
      simp only [Prod.mk.injEq] at h
      obtain ⟨rfl, rfl, rfl⟩ := h
      obtain ⟨m2, hm2, hr8⟩ := pend_line8 hp
      refine Or.inr ⟨by scalar_tac, m2, hm2, ?_⟩
      rw [hr8, hs7]
      simp [RegFile.setPending0, RegFile.setMem, RegFile.setInten]
    · have h := ok_inj h
      simp only [Prod.mk.injEq] at h
      obtain ⟨rfl, rfl, rfl⟩ := h
      exact Or.inl ⟨by scalar_tac, hs7⟩

end Hw
