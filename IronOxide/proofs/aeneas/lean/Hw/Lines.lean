import Hw.Model
open Aeneas Aeneas.Std Result

/-! # The interrupt lines of the register model and the hardware model's step (2026-11-26)

The entry cores are proven over the trait's contract: an action steps the
hardware model (`entry::step`; in Lean `hstep2` of
`Sched/ArchHwBoot.lean` for a line's priority and enable bit). The
register drivers are proven on a register file (`Hw/Model.lean`). This
file joins the two for the interrupt lines: read off the register file, a
line's enable bit and priority byte (`lineEn`, `linePrio`) change under
the drivers `Cm4Hw::apply` calls exactly as the model's step says.

- `enable_lines` (`Action::Enable`), `disable_lines` (`Action::Disable`),
  `set_priority_lines` (`Action::SetPriority`): for a number that names a
  line, the line is enabled, disabled, or given the priority, and every
  other line's bit and priority are as they were.
- `set_pending_lines`, `clear_pending_lines` (`Action::Pend`,
  `Action::ClearPending`): no line's enable bit or priority changes.
- `not_a_line`: for a number that names no line the three drivers change
  nothing, where the model's step records the number. The model's value
  for such a number says nothing about the hardware; the Kernel issues
  these actions for bound vectors (`boot_vectors`: 16 up to the board's
  count, at most `MAX_IRQS`).

The step is restated here (the two libraries cannot be imported
together): that `lineEn`/`linePrio` under these theorems is `hstep2` is by
reading. Not proven: that `Cm4Hw::apply` calls these drivers for these
actions (trusted, one line each), that `Mmio` is the register file, and
the actions that are not register writes (BASEPRI, PRIMASK), the MPU, the
watchdog and the EasyDMA registers. -/

namespace Hw
open rsk_kernel rsk_kernel.arch.cm4 rsk_kernel.arch.cm4.drivers

/-- An exception number that names an interrupt line of the NVIC. -/
def IsLine (x : Nat) : Prop := 16 ≤ x ∧ x - 16 < 496

/-- Whether line `x` is enabled: its bit of the enabled lines. -/
def lineEn (s : RegFile) (x : Nat) : Bool :=
  (s.enabled ((x - 16) / 32)).bv.getLsbD ((x - 16) % 32)

/-- Line `x`'s priority byte. -/
def linePrio (s : RegFile) (x : Nat) : U8 := s.prio (0xE000E400 + (x - 16))

theorem bit_one_shl (k j : Nat) (hk : k < 32) (_hj : j < 32) :
    (1#32 <<< k).getLsbD j = decide (j = k) := by
  rw [Bool.eq_iff_iff]
  simp [BitVec.getLsbD_shiftLeft]
  omega

theorem or_mask_bit {a m : U32} {k j : Nat} (hm : m.bv = 1#32 <<< k) (hk : k < 32) (hj : j < 32) :
    (a ||| m).bv.getLsbD j = (a.bv.getLsbD j || decide (j = k)) := by
  simp [hm, bit_one_shl k j hk hj]

theorem andn_mask_bit {a m : U32} {k j : Nat} (hm : m.bv = 1#32 <<< k) (hk : k < 32)
    (hj : j < 32) :
    (a &&& ~~~m).bv.getLsbD j = (a.bv.getLsbD j && !decide (j = k)) := by
  have hb := bit_one_shl k j hk hj
  have e : (a &&& ~~~m).bv.getLsbD j = (a.bv.getLsbD j && !(m.bv.getLsbD j)) := by simp [hj]
  rw [e, hm, hb]

/-- `Action::Enable` on the lines. -/
theorem enable_lines {s s' : RegFile} {n : U16} (h : drivers.nvic_enable model s n = ok s')
    (hn : IsLine n.val) :
    ∀ x, IsLine x → lineEn s' x = (if x = n.val then true else lineEn s x) ∧
      linePrio s' x = linePrio s x := by
  intro x hx
  unfold IsLine at hn hx
  rcases nvic_enable_spec h with ⟨_, hbad⟩ | ⟨mask, h16, hlt, hm, rfl⟩
  · omega
  · refine ⟨?_, rfl⟩
    unfold lineEn
    dsimp only
    by_cases hw : (x - 16) / 32 = (n.val - 16) / 32
    · rw [hw, Function.update_self,
        or_mask_bit hm (Nat.mod_lt _ (by decide)) (Nat.mod_lt _ (by decide))]
      by_cases hb : (x - 16) % 32 = (n.val - 16) % 32
      · have e : x = n.val := by omega
        simp [e]
      · have e : x ≠ n.val := fun e => hb (by rw [e])
        simp [hb, e]
    · have e : x ≠ n.val := fun e => hw (by rw [e])
      rw [Function.update_of_ne hw]
      simp [e]

/-- `Action::Disable` on the lines. -/
theorem disable_lines {s s' : RegFile} {n : U16} (h : drivers.nvic_disable model s n = ok s')
    (hn : IsLine n.val) :
    ∀ x, IsLine x → lineEn s' x = (if x = n.val then false else lineEn s x) ∧
      linePrio s' x = linePrio s x := by
  intro x hx
  unfold IsLine at hn hx
  rcases nvic_disable_spec h with ⟨_, hbad⟩ | ⟨mask, h16, hlt, hm, rfl⟩
  · omega
  · refine ⟨?_, rfl⟩
    unfold lineEn
    dsimp only
    by_cases hw : (x - 16) / 32 = (n.val - 16) / 32
    · rw [hw, Function.update_self,
        andn_mask_bit hm (Nat.mod_lt _ (by decide)) (Nat.mod_lt _ (by decide))]
      by_cases hb : (x - 16) % 32 = (n.val - 16) % 32
      · have e : x = n.val := by omega
        simp [e]
      · have e : x ≠ n.val := fun e => hb (by rw [e])
        simp [hb, e]
    · have e : x ≠ n.val := fun e => hw (by rw [e])
      rw [Function.update_of_ne hw]
      simp [e]

/-- `Action::SetPriority` on the lines. -/
theorem set_priority_lines {s s' : RegFile} {n : U16} {p : U8}
    (h : drivers.nvic_set_priority model s n p = ok s') (hn : IsLine n.val) :
    ∀ x, IsLine x → linePrio s' x = (if x = n.val then p else linePrio s x) ∧
      lineEn s' x = lineEn s x := by
  intro x hx
  unfold IsLine at hn hx
  rcases nvic_set_priority_spec h with ⟨_, hbad⟩ | ⟨h16, hlt, rfl⟩
  · omega
  · refine ⟨?_, rfl⟩
    unfold linePrio
    dsimp only
    by_cases e : x = n.val
    · subst e
      simp
    · have hne : 0xE000E400 + (x - 16) ≠ 0xE000E400 + (n.val - 16) := by omega
      rw [Function.update_of_ne hne]
      simp [e]

/-- `Action::Pend` leaves every line's enable bit and priority. -/
theorem set_pending_lines {s s' : RegFile} {n : U16}
    (h : drivers.nvic_set_pending model s n = ok s') :
    ∀ x, lineEn s' x = lineEn s x ∧ linePrio s' x = linePrio s x := by
  intro x
  rcases nvic_set_pending_spec h with ⟨rfl, _⟩ | ⟨mask, _, _, _, rfl⟩ <;> exact ⟨rfl, rfl⟩

/-- `Action::ClearPending` leaves every line's enable bit and priority. -/
theorem clear_pending_lines {s s' : RegFile} {n : U16}
    (h : drivers.nvic_clear_pending model s n = ok s') :
    ∀ x, lineEn s' x = lineEn s x ∧ linePrio s' x = linePrio s x := by
  intro x
  rcases nvic_clear_pending_spec h with ⟨rfl, _⟩ | ⟨mask, _, _, _, rfl⟩ <;> exact ⟨rfl, rfl⟩

/-- For a number that names no line the three drivers change nothing. -/
theorem not_a_line {s s' : RegFile} {n : U16} {p : U8} (hn : ¬ IsLine n.val) :
    (drivers.nvic_enable model s n = ok s' → s' = s) ∧
    (drivers.nvic_disable model s n = ok s' → s' = s) ∧
    (drivers.nvic_set_priority model s n p = ok s' → s' = s) := by
  unfold IsLine at hn
  refine ⟨fun h => ?_, fun h => ?_, fun h => ?_⟩
  · rcases nvic_enable_spec h with ⟨e, _⟩ | ⟨_, _, _, _, _⟩
    · exact e
    · omega
  · rcases nvic_disable_spec h with ⟨e, _⟩ | ⟨_, _, _, _, _⟩
    · exact e
    · omega
  · rcases nvic_set_priority_spec h with ⟨e, _⟩ | ⟨_, _, _⟩
    · exact e
    · omega

end Hw
