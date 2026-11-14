import Sched.Run
import Rta.Proofs
open Aeneas Aeneas.Std Result

/-!
The busy-window argument, over the naturals only: nothing here mentions
the scheduler. `L` is a set of Tasks (in the use: those of Priority at
least that of the Task under analysis, itself included), `rm k` the
execution Task `k`'s released Jobs still need, `last k` the instant of
its last Release, `β` the execution a lower Task's Critical_Section that
blocks the level may still take.
-/

namespace P18
open rsk_analyzer (ceilDiv ceilDiv_mono)

/-- The earliest instant of Task `k`'s next Release, from its last one. -/
def nxt (T : Nat → Nat) (last : Nat → Option Nat) (k : Nat) : Nat :=
  match last k with
  | none => 0
  | some r => r + T k

/-- Task `k`'s share of the work still to do before `s0 + R`: what its
released Jobs still need, and `C k` for each Release still possible. -/
def term (P : Params) (R s0 now : Nat) (last : Nat → Option Nat) (rm : Nat → Nat) (k : Nat) : Nat :=
  rm k + P.C k * ceilDiv (s0 + R - max now (nxt P.T last k)) (P.T k)

/-- The potential: the time since the window began at `s0`, the blocking
still possible, and the work of the Tasks of `L` still to do before
`s0 + R`. -/
def Phi (P : Params) (R : Nat) (L : Finset Nat) (s0 β now : Nat) (last : Nat → Option Nat)
    (rm : Nat → Nat) : Nat :=
  (now - s0) + β + ∑ k ∈ L, term P R s0 now last rm k

theorem ceilDiv_step (x T : Nat) (hT : 0 < T) (hx : 0 < x) :
    ceilDiv (x - T) T + 1 = ceilDiv x T := by
  unfold ceilDiv
  by_cases h : x ≤ T
  · have h1 : x - T = 0 := by omega
    rw [h1, Nat.zero_add, Nat.div_eq_of_lt (by omega)]
    have : x + (T - 1) = (x - 1) + T := by omega
    rw [this, Nat.add_div_right _ hT, Nat.div_eq_of_lt (by omega)]
  · have : x + (T - 1) = (x - T + (T - 1)) + T := by omega
    rw [this, Nat.add_div_right _ hT]

variable {P : Params} {R B : Nat} {L : Finset Nat}

/-- When the Tasks of `L` have no work, a window that starts now is within
the bound: this is the fixed-point inequality. -/
theorem idle_anchor (hFix : B + ∑ k ∈ L, P.C k * ceilDiv R (P.T k) ≤ R) {β : Nat} (hβ : β ≤ B)
    (now : Nat) (last : Nat → Option Nat) (rm : Nat → Nat) (h0 : ∀ k ∈ L, rm k = 0) :
    Phi P R L now β now last rm ≤ R := by
  unfold Phi
  have : ∑ k ∈ L, term P R now now last rm k ≤ ∑ k ∈ L, P.C k * ceilDiv R (P.T k) := by
    apply Finset.sum_le_sum
    intro k hk
    unfold term
    rw [h0 k hk, Nat.zero_add]
    apply Nat.mul_le_mul_left
    apply ceilDiv_mono
    have := Nat.le_max_left now (nxt P.T last k)
    omega
  omega

theorem rm_le_sum (R s0 now : Nat) (last : Nat → Option Nat) (rm : Nat → Nat) {k : Nat}
    (hk : k ∈ L) : rm k ≤ ∑ k ∈ L, term P R s0 now last rm k := by
  have h1 : rm k ≤ term P R s0 now last rm k := by unfold term; omega
  have h2 := Finset.single_le_sum (f := fun k => term P R s0 now last rm k)
    (fun _ _ => Nat.zero_le _) hk
  exact Nat.le_trans h1 h2

/-- While a Task of `L` has work, the window has lasted less than `R`. -/
theorem Phi.busy_lt {s0 β now : Nat} {last : Nat → Option Nat} {rm : Nat → Nat} {k : Nat}
    (hk : k ∈ L) (hpos : 0 < rm k) (h : Phi P R L s0 β now last rm ≤ R) : now < s0 + R := by
  have := rm_le_sum (P := P) R s0 now last rm hk
  unfold Phi at h
  omega

/-- One unit of time spent either on a Job of a Task of `L` or inside the
blocking Critical_Section. -/
theorem Phi.tick {s0 β β' now : Nat} {last : Nat → Option Nat} {rm rm' : Nat → Nat}
    (hs0 : s0 ≤ now) (hle : ∀ k ∈ L, rm' k ≤ rm k)
    (h : ((∃ a ∈ L, rm' a + 1 ≤ rm a) ∧ β' ≤ β) ∨ β' + 1 ≤ β) :
    Phi P R L s0 β' (now + 1) last rm' ≤ Phi P R L s0 β now last rm := by
  have hmono : ∀ k ∈ L, term P R s0 (now + 1) last rm' k ≤ term P R s0 now last rm k := by
    intro k hk
    unfold term
    have h1 := hle k hk
    have h2 : P.C k * ceilDiv (s0 + R - max (now + 1) (nxt P.T last k)) (P.T k)
        ≤ P.C k * ceilDiv (s0 + R - max now (nxt P.T last k)) (P.T k) := by
      apply Nat.mul_le_mul_left
      apply ceilDiv_mono
      have : max now (nxt P.T last k) ≤ max (now + 1) (nxt P.T last k) :=
        max_le_max (Nat.le_succ now) (Nat.le_refl _)
      omega
    omega
  have hsum := Finset.sum_le_sum hmono
  unfold Phi
  rcases h with ⟨⟨a, ha, hdec⟩, hβ⟩ | hβ
  · have hlt : ∑ k ∈ L, term P R s0 (now + 1) last rm' k < ∑ k ∈ L, term P R s0 now last rm k := by
      apply Finset.sum_lt_sum hmono
      refine ⟨a, ha, ?_⟩
      have h2 : P.C a * ceilDiv (s0 + R - max (now + 1) (nxt P.T last a)) (P.T a)
          ≤ P.C a * ceilDiv (s0 + R - max now (nxt P.T last a)) (P.T a) := by
        apply Nat.mul_le_mul_left
        apply ceilDiv_mono
        have : max now (nxt P.T last a) ≤ max (now + 1) (nxt P.T last a) :=
          max_le_max (Nat.le_succ now) (Nat.le_refl _)
        omega
      unfold term
      omega
    omega
  · omega

/-- A step that changes neither the instant nor, for the Tasks of `L`,
their work or their last Release. -/
theorem Phi.congr {s0 β now : Nat} {last last' : Nat → Option Nat} {rm rm' : Nat → Nat}
    (hrm : ∀ k ∈ L, rm' k = rm k) (hlast : ∀ k ∈ L, last' k = last k) :
    Phi P R L s0 β now last' rm' = Phi P R L s0 β now last rm := by
  have : ∑ k ∈ L, term P R s0 now last' rm' k = ∑ k ∈ L, term P R s0 now last rm k := by
    apply Finset.sum_congr rfl
    intro k hk
    unfold term nxt
    rw [hrm k hk, hlast k hk]
  unfold Phi
  omega

/-- A Release of a Task of `L`, at least its period after its last one, of
a Job within its bound, in a window that has lasted less than `R`. -/
theorem Phi.release {t : Nat} (ht : t ∈ L) (hT : 0 < P.T t)
    {s0 β now : Nat} {last : Nat → Option Nat} {rm : Nat → Nat} {d : Nat}
    (hd : d ≤ P.C t) (hsp : ∀ r, last t = some r → r + P.T t ≤ now) (hlt : now < s0 + R) :
    Phi P R L s0 β now (Function.update last t (some now)) (Function.update rm t (rm t + d))
      ≤ Phi P R L s0 β now last rm := by
  have hnx : nxt P.T last t ≤ now := by
    unfold nxt
    cases hl : last t with
    | none => simp
    | some r => simpa using hsp r hl
  have hle : ∑ k ∈ L, term P R s0 now (Function.update last t (some now))
      (Function.update rm t (rm t + d)) k ≤ ∑ k ∈ L, term P R s0 now last rm k := by
    apply Finset.sum_le_sum
    intro k hk
    by_cases hkt : k = t
    · subst hkt
      unfold term
      have e1 : nxt P.T (Function.update last k (some now)) k = now + P.T k := by
        simp [nxt]
      have e2 : max now (nxt P.T last k) = now := max_eq_left hnx
      have e3 : max now (now + P.T k) = now + P.T k := max_eq_right (by omega)
      rw [e1, e2, e3, Function.update_self]
      have e4 : s0 + R - (now + P.T k) = (s0 + R - now) - P.T k := by omega
      rw [e4]
      have := ceilDiv_step (s0 + R - now) (P.T k) hT (by omega)
      rw [← this, Nat.mul_add, Nat.mul_one]
      omega
    · unfold term nxt
      rw [Function.update_of_ne hkt, Function.update_of_ne hkt]
  unfold Phi
  omega

end P18
