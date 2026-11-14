import Sched.Run
import Rta.Proofs
open Aeneas Aeneas.Std Result

/-!
The busy-window argument, over the naturals only: nothing here mentions
the scheduler. `A` is a set of Tasks, `wk k` the work now owed on Task
`k`'s account that the level under analysis has to wait for, `cost k` the
most one arrival of Task `k` adds to it, `last k` the instant of its last
arrival, and `β` the execution a lower Task's Critical_Section that
blocks the level may still take.
-/

namespace P18
open rsk_analyzer (ceilDiv ceilDiv_mono)

/-- The earliest instant of Task `k`'s next arrival, from its last one. -/
def nxt (T : Nat → Nat) (last : Nat → Option Nat) (k : Nat) : Nat :=
  match last k with
  | none => 0
  | some r => r + T k

/-- Task `k`'s share of the work still to do before `s0 + R`: what is owed
now, and `cost k` for each arrival still possible. -/
def term (T cost : Nat → Nat) (R s0 now : Nat) (last : Nat → Option Nat) (wk : Nat → Nat)
    (k : Nat) : Nat :=
  wk k + cost k * ceilDiv (s0 + R - max now (nxt T last k)) (T k)

/-- The potential: the time since the window began at `s0`, the blocking
still possible, and the work still to do before `s0 + R`. -/
def Phi (T cost : Nat → Nat) (R : Nat) (A : Finset Nat) (s0 β now : Nat)
    (last : Nat → Option Nat) (wk : Nat → Nat) : Nat :=
  (now - s0) + β + ∑ k ∈ A, term T cost R s0 now last wk k

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

variable {T cost : Nat → Nat} {R B : Nat} {A : Finset Nat}

/-- A window that starts now is within the bound when each Task's term is
within its share of the fixed-point inequality. -/
theorem idle_anchor {Je : Nat → Nat}
    (hFix : B + ∑ k ∈ A, cost k * ceilDiv (R + Je k) (T k) ≤ R) {β : Nat} (hβ : β ≤ B)
    (now : Nat) (last : Nat → Option Nat) (wk : Nat → Nat)
    (hterm : ∀ k ∈ A, term T cost R now now last wk k ≤ cost k * ceilDiv (R + Je k) (T k)) :
    Phi T cost R A now β now last wk ≤ R := by
  unfold Phi
  have := Finset.sum_le_sum hterm
  omega

theorem wk_le_sum (R s0 now : Nat) (last : Nat → Option Nat) (wk : Nat → Nat) {k : Nat}
    (hk : k ∈ A) : wk k ≤ ∑ k ∈ A, term T cost R s0 now last wk k := by
  have h1 : wk k ≤ term T cost R s0 now last wk k := by unfold term; omega
  have h2 := Finset.single_le_sum (f := fun k => term T cost R s0 now last wk k)
    (fun _ _ => Nat.zero_le _) hk
  exact Nat.le_trans h1 h2

/-- While work is owed, the window has lasted less than `R`. -/
theorem Phi.busy_lt {s0 β now : Nat} {last : Nat → Option Nat} {wk : Nat → Nat} {k : Nat}
    (hk : k ∈ A) (hpos : 0 < wk k) (h : Phi T cost R A s0 β now last wk ≤ R) : now < s0 + R := by
  have := wk_le_sum (T := T) (cost := cost) R s0 now last wk hk
  unfold Phi at h
  omega

/-- One unit of time spent either on owed work or inside the blocking
Critical_Section. -/
theorem Phi.tick {s0 β β' now : Nat} {last : Nat → Option Nat} {wk wk' : Nat → Nat}
    (hs0 : s0 ≤ now) (hle : ∀ k ∈ A, wk' k ≤ wk k)
    (h : ((∃ a ∈ A, wk' a + 1 ≤ wk a) ∧ β' ≤ β) ∨ β' + 1 ≤ β) :
    Phi T cost R A s0 β' (now + 1) last wk' ≤ Phi T cost R A s0 β now last wk := by
  have hN : ∀ k, cost k * ceilDiv (s0 + R - max (now + 1) (nxt T last k)) (T k)
      ≤ cost k * ceilDiv (s0 + R - max now (nxt T last k)) (T k) := by
    intro k
    apply Nat.mul_le_mul_left
    apply ceilDiv_mono
    have : max now (nxt T last k) ≤ max (now + 1) (nxt T last k) :=
      max_le_max (Nat.le_succ now) (Nat.le_refl _)
    omega
  have hmono : ∀ k ∈ A, term T cost R s0 (now + 1) last wk' k ≤ term T cost R s0 now last wk k := by
    intro k hk
    unfold term
    have h1 := hle k hk
    have h2 := hN k
    omega
  have hsum := Finset.sum_le_sum hmono
  unfold Phi
  rcases h with ⟨⟨a, ha, hdec⟩, hβ⟩ | hβ
  · have hlt : ∑ k ∈ A, term T cost R s0 (now + 1) last wk' k
        < ∑ k ∈ A, term T cost R s0 now last wk k := by
      apply Finset.sum_lt_sum hmono
      refine ⟨a, ha, ?_⟩
      have h2 := hN a
      unfold term
      omega
    omega
  · omega

/-- A step at one instant that lowers or keeps every Task's owed work and
keeps the arrivals. -/
theorem Phi.mono {s0 β now : Nat} {last last' : Nat → Option Nat} {wk wk' : Nat → Nat}
    (hwk : ∀ k ∈ A, wk' k ≤ wk k) (hlast : ∀ k ∈ A, last' k = last k) :
    Phi T cost R A s0 β now last' wk' ≤ Phi T cost R A s0 β now last wk := by
  have : ∑ k ∈ A, term T cost R s0 now last' wk' k ≤ ∑ k ∈ A, term T cost R s0 now last wk k := by
    apply Finset.sum_le_sum
    intro k hk
    unfold term nxt
    rw [hlast k hk]
    have := hwk k hk
    omega
  unfold Phi
  omega

/-- An arrival of Task `t`, at least its period after its last one, in a
window that has lasted less than `R`: it adds at most `cost t`. -/
theorem Phi.arrive {t : Nat} (ht : t ∈ A) (hT : 0 < T t)
    {s0 β now : Nat} {last : Nat → Option Nat} {wk wk' : Nat → Nat}
    (hadd : wk' t ≤ wk t + cost t) (hsame : ∀ k ∈ A, k ≠ t → wk' k = wk k)
    (hsp : ∀ r, last t = some r → r + T t ≤ now) (hlt : now < s0 + R) :
    Phi T cost R A s0 β now (Function.update last t (some now)) wk'
      ≤ Phi T cost R A s0 β now last wk := by
  have hnx : nxt T last t ≤ now := by
    unfold nxt
    cases hl : last t with
    | none => simp
    | some r => simpa using hsp r hl
  have hle : ∑ k ∈ A, term T cost R s0 now (Function.update last t (some now)) wk' k
      ≤ ∑ k ∈ A, term T cost R s0 now last wk k := by
    apply Finset.sum_le_sum
    intro k hk
    by_cases hkt : k = t
    · subst hkt
      unfold term
      have e1 : nxt T (Function.update last k (some now)) k = now + T k := by
        simp [nxt]
      have e2 : max now (nxt T last k) = now := max_eq_left hnx
      have e3 : max now (now + T k) = now + T k := max_eq_right (by omega)
      rw [e1, e2, e3]
      have e4 : s0 + R - (now + T k) = (s0 + R - now) - T k := by omega
      rw [e4]
      have := ceilDiv_step (s0 + R - now) (T k) hT (by omega)
      rw [← this, Nat.mul_add, Nat.mul_one]
      omega
    · unfold term nxt
      rw [Function.update_of_ne hkt, hsame k hk hkt]
  unfold Phi
  omega

end P18
