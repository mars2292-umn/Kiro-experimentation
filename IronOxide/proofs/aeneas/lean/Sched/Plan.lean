import Sched.HwMore
import Sched.Record
open Aeneas Aeneas.Std Result

/-! # Dispatch planning and the Endpoint storage between entries (2026-11-26)

The contracts Verus proves of `crates/rsk-kernel/src/arch/cm4/plan.rs`,
proven of the extracted functions (`Sched/Kernel.lean`), each of the form
"if the function returns":

- `frame_in_stack_spec`: `frame_in_stack` is true exactly when the
  eight-word frame at `psp` lies inside `[bottom, top)`
  (`spec_frame_in_stack`, R16.5).
- `plan_frame_some`, `plan_frame_none`: a new Job's frame is 8-byte
  aligned, not below `bottom`, wholly below `floor`, the highest such
  address, and inside the stack region; `None` only when `floor` is above
  `top`, below a frame's size, or the aligned address is below `bottom`.
  `plan_frame_in_stack`: `frame_in_stack` accepts the frame `plan_frame`
  returns.
- `compare_target_spec`: the compare value is the Hardware_Model's
  `compare_for`, at least `MIN_COMPARE_DISTANCE` ticks past the counter's
  reading and not before the target (HW-12).
- `top_active_full`: `top_active`'s contract with its fourth clause
  (`Sched/Record.lean` has the first three).

And the statement of entry.rs that rests on the Hardware_Model's Kernel
RAM (`kernel_ram_kept` of `Sched/HwMore.lean`):

- `storage_kept_between_entries`
  (`theorem_storage_kept_between_entries`, R16.1 for R39): the Endpoint
  storage the tables name, lying in the Kernel's RAM, keeps every word
  while a Job executes under its Partition's view.

The wrap extension of the time base is in `Hw/TimeBaseProofs.lean`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic P18.HwM

theorem frame_bytes : arch.cm4.plan.FRAME_BYTES = 32#u32 := by
  simp [arch.cm4.plan.FRAME_BYTES]

theorem u32_sub_ok' {a b c : U32} (h : a - b = ok c) : b.val ≤ a.val ∧ c.val = a.val - b.val := by
  have := UScalar.sub_equiv a b
  simp_all

theorem u32_div8_ok {a c : U32} (h : a / 8#u32 = ok c) : c.val = a.val / 8 := by
  have hs := UScalar.div_spec a (y := 8#u32) (by decide)
  rw [h] at hs
  simpa [Aeneas.Std.WP.spec_ok] using hs

theorem u32_mul8_ok {a c : U32} (h : a * 8#u32 = ok c) : c.val = a.val * 8 := by
  have := UScalar.mul_equiv a 8#u32
  rw [show a * 8#u32 = UScalar.mul a 8#u32 from rfl] at h
  rw [h] at this
  exact this.2.1

theorem pl_bind {α β} (x : Result α) (f : α → Result β) (y : β) :
    Std.bind x f = ok y ↔ ∃ a, x = ok a ∧ f a = ok y := by
  cases x <;> simp

/-- The bytes of the frame a dispatch writes (`FRAME_BYTES`). -/
def frameBytes : Nat := 32

/-- The frame at `psp` lies inside `[bottom, top)` (`spec_frame_in_stack`). -/
def frameInStack (psp bottom top : Nat) : Prop := bottom ≤ psp ∧ psp + frameBytes ≤ top

/-- `frame_in_stack` decides `spec_frame_in_stack`. -/
theorem frame_in_stack_spec {psp bottom top : U32} {b : Bool}
    (h : arch.cm4.plan.frame_in_stack psp bottom top = ok b) :
    b = true ↔ frameInStack psp.val bottom.val top.val := by
  unfold arch.cm4.plan.frame_in_stack at h
  rw [frame_bytes] at h
  unfold frameInStack frameBytes
  by_cases h1 : psp ≥ bottom
  · rw [if_pos h1] at h
    by_cases h2 : top ≥ 32#u32
    · rw [if_pos h2] at h
      simp only [pl_bind] at h
      obtain ⟨i, hi, h⟩ := h
      obtain ⟨_, ei⟩ := u32_sub_ok' hi
      have h := ok_inj h
      subst h
      have h1' : bottom.val ≤ psp.val := h1
      have h2' : 32 ≤ top.val := h2
      have e32 : (32#u32).val = 32 := rfl
      simp only [decide_eq_true_eq]
      constructor
      · intro hle
        have : psp.val ≤ i.val := hle
        omega
      · intro ⟨_, _⟩
        show psp.val ≤ i.val
        omega
    · rw [if_neg h2] at h
      have h := ok_inj h
      subst h
      have h2' : ¬ 32 ≤ top.val := h2
      constructor
      · intro hf; cases hf
      · intro ⟨_, _⟩; omega
  · rw [if_neg h1] at h
    have h := ok_inj h
    subst h
    have h1' : ¬ bottom.val ≤ psp.val := h1
    constructor
    · intro hf; cases hf
    · intro ⟨_, _⟩; omega

/-- **A new Job's frame is aligned and inside its stack** (`plan_frame`,
the `Some` clauses): the address is a multiple of 8, not below `bottom`,
its frame ends at or below `floor` and no higher aligned address would,
`floor` is not above `top`, and the frame lies inside `[bottom, top)`. -/
theorem plan_frame_some {floor bottom top f : U32}
    (h : arch.cm4.plan.plan_frame floor bottom top = ok (some f)) :
    f.val % 8 = 0 ∧ bottom.val ≤ f.val ∧ f.val + frameBytes ≤ floor.val ∧
    floor.val < f.val + frameBytes + 8 ∧ floor.val ≤ top.val ∧
    frameInStack f.val bottom.val top.val := by
  unfold arch.cm4.plan.plan_frame at h
  rw [frame_bytes] at h
  unfold frameInStack frameBytes
  by_cases h1 : floor > top
  · rw [if_pos h1] at h; cases ok_inj h
  rw [if_neg h1] at h
  by_cases h2 : floor < 32#u32
  · rw [if_pos h2] at h; cases ok_inj h
  rw [if_neg h2] at h
  simp only [pl_bind] at h
  obtain ⟨t1, ht1, i, hi, g, hg, h⟩ := h
  obtain ⟨_, e1⟩ := u32_sub_ok' ht1
  have e2 := u32_div8_ok hi
  have e3 := u32_mul8_ok hg
  by_cases h3 : g < bottom
  · rw [if_pos h3] at h; cases ok_inj h
  rw [if_neg h3] at h
  have h := Option.some.inj (ok_inj h)
  subst h
  have h1' : ¬ top.val < floor.val := h1
  have h2' : ¬ floor.val < 32 := h2
  have h3' : ¬ g.val < bottom.val := h3
  have e32 : (32#u32).val = 32 := rfl
  omega

/-- `plan_frame` returns `None` only when the frame would lie outside the
region. -/
theorem plan_frame_none {floor bottom top : U32}
    (h : arch.cm4.plan.plan_frame floor bottom top = ok none) :
    top.val < floor.val ∨ floor.val < frameBytes ∨
    ((floor.val - frameBytes) / 8) * 8 < bottom.val := by
  unfold arch.cm4.plan.plan_frame at h
  rw [frame_bytes] at h
  unfold frameBytes
  by_cases h1 : floor > top
  · exact Or.inl h1
  rw [if_neg h1] at h
  by_cases h2 : floor < 32#u32
  · exact Or.inr (Or.inl h2)
  rw [if_neg h2] at h
  simp only [pl_bind] at h
  obtain ⟨t1, ht1, i, hi, g, hg, h⟩ := h
  obtain ⟨_, e1⟩ := u32_sub_ok' ht1
  have e2 := u32_div8_ok hi
  have e3 := u32_mul8_ok hg
  by_cases h3 : g < bottom
  · have h3' : g.val < bottom.val := h3
    have e32 : (32#u32).val = 32 := rfl
    right; right
    omega
  rw [if_neg h3] at h
  cases ok_inj h

/-- The frame `plan_frame` returns is one `frame_in_stack` accepts. -/
theorem plan_frame_in_stack {floor bottom top f : U32} {b : Bool}
    (h : arch.cm4.plan.plan_frame floor bottom top = ok (some f))
    (hb : arch.cm4.plan.frame_in_stack f bottom top = ok b) : b = true :=
  (frame_in_stack_spec hb).2 (plan_frame_some h).2.2.2.2.2

/-- **The timer compare value** (`compare_target`, HW-12): with the
counter's reading inside the time base, the value is the Hardware_Model's
`compare_for`, at least the minimum distance past the reading and not
before the target. -/
theorem compare_target_spec {now at_ t : U64}
    (h : arch.cm4.plan.compare_target now at_ = ok t) :
    t.val = compareFor now.val at_.val ∧ now.val + minCompareDistance ≤ t.val ∧
    at_.val ≤ t.val := by
  unfold arch.cm4.plan.compare_target at h
  rw [min_cmp] at h
  simp only [pl_bind] at h
  obtain ⟨i, hi, h⟩ := h
  have ei := u64_add_ok hi
  have e16 : (16#u64).val = 16 := rfl
  unfold compareFor minCompareDistance
  by_cases hc : at_ < i
  · rw [if_pos hc] at h
    have hc' : at_.val < i.val := hc
    have h := ok_inj h
    subst h
    rw [if_pos (by omega)]
    omega
  · rw [if_neg hc] at h
    have h := ok_inj h
    subst h
    have hc' : ¬ at_.val < i.val := hc
    rw [if_neg (by omega)]
    omega

/-- `top_active`'s contract in full: below 8; active unless 0; no record
above it active; and 0 only when no record 1 to 7 is active. -/
theorem top_active_full {levels : Array arch.cm4.plan.LevelRecord 8#usize} {l : Usize}
    (h : arch.cm4.plan.top_active levels = ok l) :
    l.val < 8 ∧ (l.val ≠ 0 → act levels l.val = true) ∧
    (∀ j, l.val < j → j < 8 → act levels j = false) ∧
    (l.val = 0 → ∀ j, 1 ≤ j → j < 8 → act levels j = false) := by
  obtain ⟨h1, h2, h3⟩ := top_active_spec h
  exact ⟨h1, h2, h3, fun h0 j hj1 hj8 => h3 j (by omega) hj8⟩

/-- **The Endpoint storage persists between two entries**
(`theorem_storage_kept_between_entries`; R16.1 for R39). `m1` is the
memory an entry left and `m2` the memory the next entry finds; between
them a Job of Partition `p` ran under its view, so a word changed only if
the MPU let it write one of the word's bytes. The storage the tables name
lies in the Kernel's RAM (the Config_Checker's check of every binary), so
every word of it is as it was: a message a send stored is the message a
later receive copies out. -/
theorem storage_kept_between_entries {T : Type} (ti : arch.cm4.entry.Tables T) {tables : T}
    {NP : Usize} {l : logic.mpu.Layout NP} (hl : LayoutOk l) {p : Usize} {v : logic.mpu.View}
    (hv : logic.mpu.Layout.view l p = ok v) {m1 m2 : Nat → Nat} (hj : jobWritesOnly v m1 m2)
    {base total : U32} (_hs : ti.endpoint_storage tables = ok (base, total))
    (hlo : l.kernel_ram.base.val ≤ base.val)
    (hhi : base.val + total.val ≤ l.kernel_ram.base.val + isz l.kernel_ram) :
    ∀ a, base.val ≤ a → a + 4 ≤ base.val + total.val → m2 a = m1 a := by
  intro a h1 h2
  exact kernel_ram_kept hl hv hj (a := a) ⟨⟨by omega, by omega⟩, ⟨by omega, by omega⟩⟩

end P18
