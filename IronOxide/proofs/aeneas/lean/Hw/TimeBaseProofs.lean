import Hw.TimeBase
import Hw.Model
open Aeneas Aeneas.Std Result

/-! # The time base: wrap extension of the counter (2026-11-26)

`Hw/TimeBase.lean` is the Aeneas translation of `ArchState::extend`
(`crates/rsk-kernel/src/arch/cm4/plan.rs`), the function that extends a
reading of the 32-bit counter to the 64-bit time base (HW-12, DD-05,
R9.5). This file proves the contract Verus proves of it:

- `extend_total`: the extension always returns.
- `extend_spec`: it changes only the wrap count and the last reading, and
  for any true instant less than one counter period after the instant the
  state stood for, whose low 32 bits the reading is, the result is that
  instant and the state stands for it afterwards.
- `extends_true`: the induction the contract names: over any sequence of
  readings, each less than a period after the one before, every value
  returned is the true instant of its reading.
- `housekeeping_within_a_period`: the premise in the Kernel's numbers.
- `now_true`: joined to the register drivers (`now_spec` of
  `Hw/Model.lean`), `timer0::now` returns the true instant.

Assumed, as in Verus: that two consecutive readings are less than one
counter period apart (the rig's evidence). Not here: that the instants of
`Sched/Time.lean` and of Property 18's run are these extended values; the
Rust `Clock`, whose `extend` reaches this function through the trusted
accessor of the Kernel's one `ArchState`, is stood for by `archExtend`. -/

namespace Hw
open rsk_kernel rsk_kernel.arch.cm4

/-- Ticks in one period of the 32-bit counter (`plan::counter_period`). -/
def counterPeriod : Nat := 0x1_0000_0000

/-- The instant the wrap count and the last reading stand for
(`ArchState::time`). -/
def stime (s : plan.ArchState) : Nat := s.time_hi.val * counterPeriod + s.time_last_lo.val

/-- The counter reads `low` at the true instant `at_`, less than one period
after the instant `previous` of the reading before it, and inside the
64-bit time base (`plan::counter_reads`). -/
def counterReads (previous : Nat) (low : U32) (at_ : Nat) : Prop :=
  previous ≤ at_ ∧ at_ < previous + counterPeriod ∧ at_ < 0x1_0000_0000_0000_0000 ∧
  low.val = at_ % counterPeriod

theorem join_bv (i low : BitVec 32) :
    ((i.setWidth 64) <<< 32 ||| low.setWidth 64).toNat = i.toNat * 0x1_0000_0000 + low.toNat := by
  have h1 := i.isLt
  have h2 := low.isLt
  simp only [BitVec.toNat_or, BitVec.toNat_shiftLeft, BitVec.toNat_setWidth, Nat.shiftLeft_eq]
  have e1 : i.toNat % 2 ^ 64 = i.toNat := Nat.mod_eq_of_lt (by omega)
  have e2 : low.toNat % 2 ^ 64 = low.toNat := Nat.mod_eq_of_lt (by omega)
  have e3 : i.toNat * 2 ^ 32 % 2 ^ 64 = 2 ^ 32 * i.toNat := by omega
  rw [e1, e2, e3, ← Nat.two_pow_add_eq_or_of_lt h2]
  omega

theorem extend_val {s s' : plan.ArchState} {low : U32} {t : U64}
    (h : plan.ArchState.extend s low = ok (t, s')) :
    s' = { s with time_hi := s'.time_hi, time_last_lo := low } ∧
    t.val = s'.time_hi.val * counterPeriod + low.val ∧
    s'.time_hi.val = (if low.val < s.time_last_lo.val then (s.time_hi.val + 1) % counterPeriod
      else s.time_hi.val) := by
  unfold plan.ArchState.extend at h
  simp only [bind_eq_ok, lift] at h
  obtain ⟨i, hi, hh, e1, l, e2, i1, hs, t', e3, h⟩ := h
  have e1 := ok_inj e1; have e2 := ok_inj e2; have e3 := ok_inj e3; have h := ok_inj h
  subst e1 e2 e3
  simp only [HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft] at hs
  split at hs
  case isFalse => simp at hs
  have hs := ok_inj hs
  subst hs
  obtain ⟨rfl, rfl⟩ := Prod.mk.inj h
  refine ⟨rfl, ?_, ?_⟩
  · have := join_bv i.bv low.bv
    simp only [counterPeriod]
    exact this
  · simp only [counterPeriod]
    by_cases hlt : low < s.time_last_lo
    · rw [if_pos hlt] at hi
      have hi := ok_inj hi
      subst hi
      have hlt' : low.val < s.time_last_lo.val := hlt
      rw [if_pos hlt', core.num.U32.wrapping_add_val_eq]
      scalar_tac
    · rw [if_neg hlt] at hi
      have hi := ok_inj hi
      subst hi
      have hlt' : ¬ low.val < s.time_last_lo.val := hlt
      rw [if_neg hlt']

/-- The extension always returns. -/
theorem extend_total (s : plan.ArchState) (low : U32) :
    ∃ t s', plan.ArchState.extend s low = ok (t, s') := by
  unfold plan.ArchState.extend
  simp only [lift, HShiftLeft.hShiftLeft, UScalar.shiftLeft_IScalar, UScalar.shiftLeft]
  split <;> simp

/-- **Wrap extension of the counter** (`ArchState::extend`'s contract;
HW-12, DD-05, R9.5). The extension changes only the wrap count and the
last reading, and whatever the true instant `at_` of this reading is, if it
lies less than one counter period after the instant the state stood for
and `low` is its low 32 bits, the result is `at_` and the state stands for
`at_` afterwards. -/
theorem extend_spec {s s' : plan.ArchState} {low : U32} {t : U64}
    (h : plan.ArchState.extend s low = ok (t, s')) :
    s' = { s with time_hi := s'.time_hi, time_last_lo := low } ∧
    ∀ at_, counterReads (stime s) low at_ → t.val = at_ ∧ stime s' = at_ := by
  obtain ⟨hf, ht, hh⟩ := extend_val h
  refine ⟨hf, ?_⟩
  intro at_ ⟨h1, h2, h3, h4⟩
  have hl : s'.time_last_lo = low := by rw [hf]
  have e : s'.time_hi.val * counterPeriod + low.val = at_ := by
    have b1 := s.time_hi.hBounds
    have b2 := s.time_last_lo.hBounds
    have b3 := low.hBounds
    unfold stime counterPeriod at *
    simp only [UScalarTy.U32_numBits_eq] at b1 b2 b3
    split at hh <;> omega
  refine ⟨by rw [ht, e], ?_⟩
  unfold stime
  rw [hl, e]

/-- A sequence of readings, each with the true instant it was taken at:
`Readings s rs s'` runs the extension over the readings of `rs` in order
from the state `s`, each returning the instant listed with it. -/
inductive Extends : plan.ArchState → List (U32 × U64) → plan.ArchState → Prop where
  | nil (s) : Extends s [] s
  | cons {s s1 s' low t rs} : plan.ArchState.extend s low = ok (t, s1) → Extends s1 rs s' →
      Extends s ((low, t) :: rs) s'

/-- The true instants `ats` are those of the readings `rs`, each less than
one counter period after the one before it, the first after `prev`. -/
def ReadsFrom : Nat → List (U32 × U64) → List Nat → Prop
  | _, [], [] => True
  | prev, (low, _) :: rs, a :: ats => counterReads prev low a ∧ ReadsFrom a rs ats
  | _, _, _ => False

/-- **The time base equals the true count of ticks across every wrap**
(the induction `extend`'s contract names): from a state that stands for
the instant of the reading before, if each reading follows the one before
it by less than one counter period, every value the extension returns is
the true instant of its reading. -/
theorem extends_true {s s' : plan.ArchState} {rs : List (U32 × U64)} (h : Extends s rs s') :
    ∀ ats, ReadsFrom (stime s) rs ats → List.map (fun r => r.2.val) rs = ats := by
  induction h with
  | nil s => intro ats hr; cases ats <;> simp_all [ReadsFrom]
  | cons he _ ih =>
    intro ats hr
    cases ats with
    | nil => simp [ReadsFrom] at hr
    | cons a ats =>
      obtain ⟨hc, hrest⟩ := hr
      obtain ⟨ht, hs1⟩ := (extend_spec he).2 a hc
      rw [← hs1] at hrest
      simp only [List.map_cons, ht, ih ats hrest]

/-- The premise of the extension in the Kernel's numbers
(`lemma_housekeeping_within_a_period`): a reading that follows the one
before it by the housekeeping period (2,000,000 ticks, 125 ms) and a
service latency of up to 4,290,000,000 ticks is less than one counter
period after it. -/
theorem housekeeping_within_a_period (latency : Nat) (h : latency ≤ 4290000000) :
    2000000 + latency < counterPeriod := by
  unfold counterPeriod; omega

/-- The extension as the drivers' `Extend` over the Kernel_Arch state. In
the Rust source the implementation is of the zero-sized `Clock`, whose
`extend` calls this function on the Kernel's one `ArchState` through the
trusted accessor; this instance stands for that call. -/
def archExtend : drivers.Extend plan.ArchState where
  extend := plan.ArchState.extend

/-- **The time base driver returns the true instant** (`timer0::now` over
the register model, joined to the extension): if the capture register
holds the low 32 bits of the true instant `at_`, less than one counter
period after the instant the state stood for, `now` returns `at_`. -/
theorem now_true {s s' : RegFile} {e e' : plan.ArchState} {t : U64} {at_ : Nat}
    (h : drivers.timer0.now model archExtend s e = ok (t, s', e'))
    (hr : counterReads (stime e) (s.mem T_CC1) at_) : t.val = at_ ∧ stime e' = at_ :=
  (extend_spec (now_spec archExtend h).2).2 at_ hr

end Hw
