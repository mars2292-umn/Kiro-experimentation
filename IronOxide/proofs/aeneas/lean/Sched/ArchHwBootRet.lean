import Sched.ArchHwBoot
open Aeneas Aeneas.Std Result

/-! # The vector lines at `boot_core`'s return (2026-11-26)

`Sched/ArchHwBoot.lean` proves that `boot_vectors` leaves every bound line
as its binding says. `boot_core` runs five more actions after it (the
background region of the MPU, the watchdog's start, the time base's
start, the cycle counter, a trace). This file shows that none of them
writes a line's priority or enable bit (`HE`, `NoVec`), and so:

- `boot_core_vectors`: **a boot that passes its checks returns with every
  line from 16 up to the board's count as its binding says** (A3, R19.5;
  Verus: `boot_core`'s `ensures` over `boot_vector_ok`), read with the
  Kernel's state `boot_core` returns.

The statement is of the extracted `boot_core`, over any hardware that
meets the trait's contract for these fields (`HSpec2`). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {h2 : A → HV2}

/-- From `hw` to `hw'` every line's priority and enable bit are as they
were. -/
def HE (h2 : A → HV2) (hw' hw : A) : Prop :=
  (h2 hw').prio = (h2 hw).prio ∧ (h2 hw').enabled = (h2 hw).enabled

theorem HE.refl (h2 : A → HV2) (hw : A) : HE h2 hw hw := ⟨rfl, rfl⟩

theorem HE.trans {a b c : A} (h1 : HE h2 b a) (hh : HE h2 c b) : HE h2 c a :=
  ⟨hh.1.trans h1.1, hh.2.trans h1.2⟩

/-- An action that writes no line's priority or enable bit. -/
def NoVec : arch.cm4.entry.Action → Prop
  | .SetPriority _ _ => False
  | .Enable _ => False
  | .Disable _ => False
  | _ => True

theorem HE.apply [HSpec2 ai h2] {hw hw' : A} {act : arch.cm4.entry.Action}
    (h : ai.apply hw act = ok hw') (hq : NoVec act) : HE h2 hw' hw := by
  have e := HSpec2.apply (h2 := h2) h
  unfold HE
  rw [e]
  cases act <;> first
    | exact ⟨rfl, rfl⟩
    | exact absurd hq (by simp [NoVec])

/-- The lines are as their bindings say on any hardware state with the
same priorities and enable bits. -/
theorem BootVecs.congr {k : K} {tables : T} {hw hw' : A} {lo hi : Nat} (he : HE h2 hw' hw)
    (hb : BootVecs ki ti k tables (h2 hw) lo hi) : BootVecs ki ti k tables (h2 hw') lo hi := by
  intro n hlo hhi b hbd
  exact VecOk.congr (congrFun he.1 _) (congrFun he.2 _) (hb n hlo hhi b hbd)

syntax "he_close" : tactic
macro_rules
  | `(tactic| he_close) => `(tactic| repeat (first
      | exact HE.refl _ _
      | refine HE.trans ?_ (HE.apply (by assumption) (by exact True.intro))))

set_option maxHeartbeats 8000000 in
/-- **The vector lines at `boot_core`'s return** (A3, R19.5). -/
theorem boot_core_vectors [HSpec2 ai h2] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {b : Bool}
    (h : arch.cm4.entry.boot_core ki ai ti k hw tables s = ok (b, k', hw', s')) :
    b = true → ∃ irqs, ti.irq_count tables = ok irqs ∧
      BootVecs ki ti k' tables (h2 hw') 16 (16 + irqs.val) := by
  unfold arch.cm4.entry.boot_core at h
  peel h
  all_goals first
    | (intro hb; cases hb; done)
    | (intro _
       refine ⟨_, ‹ti.irq_count tables = ok _›,
         BootVecs.congr ?_ (boot_vectors_ok (h2 := h2) (by assumption))⟩
       he_close)

end P18
