import Sched.Run
import Rta.Proofs
open Aeneas Aeneas.Std Result

/-!
The busy-window argument, over the naturals only: nothing here mentions
the scheduler. `L` is a set of Tasks (in the use: those of Priority at
least that of the Task under analysis, itself included), `rm k` the
execution Task `k`'s released Jobs still need, `last k` the instant of
its last Release.
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

/-- The potential: the time since the window began at `s0`, and the work
of the Tasks of `L` still to do before `s0 + R`. -/
def Phi (P : Params) (R : Nat) (L : Finset Nat) (s0 now : Nat) (last : Nat → Option Nat)
    (rm : Nat → Nat) : Nat :=
  (now - s0) + ∑ k ∈ L, term P R s0 now last rm k

/-- The invariant: some window start `s0` keeps the potential within `R`,
and Task `i`'s unfinished Job was released in the window. -/
def LInv (P : Params) (R : Nat) (L : Finset Nat) (i : Nat) (now : Nat)
    (last : Nat → Option Nat) (rm : Nat → Nat) : Prop :=
  ∃ s0, s0 ≤ now ∧ Phi P R L s0 now last rm ≤ R ∧ (0 < rm i → ∃ r, last i = some r ∧ s0 ≤ r)

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

variable {P : Params} {R : Nat} {L : Finset Nat} {i : Nat}

theorem idle_anchor (hFix : ∑ k ∈ L, P.C k * ceilDiv R (P.T k) ≤ R)
    (now : Nat) (last : Nat → Option Nat) (rm : Nat → Nat) (h0 : ∀ k ∈ L, rm k = 0) :
    Phi P R L now now last rm ≤ R := by
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

/-- One unit of time in which, if the Tasks of `L` have work, one of them
executes. -/
theorem LInv.tick (hFix : ∑ k ∈ L, P.C k * ceilDiv R (P.T k) ≤ R) (hi : i ∈ L)
    {now : Nat} {last : Nat → Option Nat} {rm rm' : Nat → Nat}
    (hle : ∀ k ∈ L, rm' k ≤ rm k)
    (hbusy : (∃ k ∈ L, 0 < rm k) → ∃ a ∈ L, rm' a + 1 ≤ rm a)
    (h : LInv P R L i now last rm) : LInv P R L i (now + 1) last rm' := by
  obtain ⟨s0, hs0, hphi, hthird⟩ := h
  by_cases hb : ∃ k ∈ L, 0 < rm' k
  · obtain ⟨k, hk, hpos⟩ := hb
    obtain ⟨a, ha, hdec⟩ := hbusy ⟨k, hk, Nat.lt_of_lt_of_le hpos (hle k hk)⟩
    refine ⟨s0, by omega, ?_, ?_⟩
    · have hmono : ∀ k ∈ L, term P R s0 (now + 1) last rm' k ≤ term P R s0 now last rm k := by
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
      have hlt : ∑ k ∈ L, term P R s0 (now + 1) last rm' k < ∑ k ∈ L, term P R s0 now last rm k := by
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
      unfold Phi at hphi ⊢
      omega
    · intro hi'
      exact hthird (Nat.lt_of_lt_of_le hi' (hle i hi))
  · have h0 : ∀ k ∈ L, rm' k = 0 := by
      intro k hk
      by_contra hc
      exact hb ⟨k, hk, by omega⟩
    refine ⟨now + 1, Nat.le_refl _, idle_anchor hFix _ _ _ h0, ?_⟩
    intro hi'
    rw [h0 i hi] at hi'
    omega

/-- A step that changes neither the instant nor, for the Tasks of `L`,
their work or their last Release. -/
theorem LInv.congr (hi : i ∈ L) {now : Nat} {last last' : Nat → Option Nat} {rm rm' : Nat → Nat}
    (hrm : ∀ k ∈ L, rm' k = rm k) (hlast : ∀ k ∈ L, last' k = last k)
    (h : LInv P R L i now last rm) : LInv P R L i now last' rm' := by
  obtain ⟨s0, hs0, hphi, hthird⟩ := h
  refine ⟨s0, hs0, ?_, ?_⟩
  · have : ∑ k ∈ L, term P R s0 now last' rm' k = ∑ k ∈ L, term P R s0 now last rm k := by
      apply Finset.sum_congr rfl
      intro k hk
      unfold term nxt
      rw [hrm k hk, hlast k hk]
    unfold Phi at hphi ⊢
    omega
  · intro hi'
    rw [hrm i hi] at hi'
    rw [hlast i hi]
    exact hthird hi'

/-- A Release of a Task of `L`, at least its period after its last one, of
a Job within its bound. -/
theorem LInv.release (hFix : ∑ k ∈ L, P.C k * ceilDiv R (P.T k) ≤ R) (hR : 0 < R)
    (hi : i ∈ L) {t : Nat} (ht : t ∈ L) (hT : 0 < P.T t)
    {now : Nat} {last : Nat → Option Nat} {rm : Nat → Nat} {d : Nat}
    (hd : d ≤ P.C t) (hsp : ∀ r, last t = some r → r + P.T t ≤ now)
    (h : LInv P R L i now last rm) :
    LInv P R L i now (Function.update last t (some now)) (Function.update rm t (rm t + d)) := by
  have hanchor : ∃ a, a ≤ now ∧ now < a + R ∧ Phi P R L a now last rm ≤ R ∧
      (0 < rm i → ∃ r, last i = some r ∧ a ≤ r) := by
    obtain ⟨s0, hs0, hphi, hthird⟩ := h
    by_cases hb : ∃ k ∈ L, 0 < rm k
    · obtain ⟨k, hk, hpos⟩ := hb
      refine ⟨s0, hs0, ?_, hphi, hthird⟩
      have := rm_le_sum (P := P) R s0 now last rm hk
      unfold Phi at hphi
      omega
    · have h0 : ∀ k ∈ L, rm k = 0 := by
        intro k hk
        by_contra hc
        exact hb ⟨k, hk, by omega⟩
      refine ⟨now, Nat.le_refl _, by omega, idle_anchor hFix _ _ _ h0, ?_⟩
      intro hi'
      rw [h0 i hi] at hi'
      omega
  obtain ⟨a, ha, haR, hphi, hthird⟩ := hanchor
  refine ⟨a, ha, ?_, ?_⟩
  · have hnx : nxt P.T last t ≤ now := by
      unfold nxt
      cases hl : last t with
      | none => simp
      | some r => simpa using hsp r hl
    have hle : ∑ k ∈ L, term P R a now (Function.update last t (some now))
        (Function.update rm t (rm t + d)) k ≤ ∑ k ∈ L, term P R a now last rm k := by
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
        have e4 : a + R - (now + P.T k) = (a + R - now) - P.T k := by omega
        rw [e4]
        have := ceilDiv_step (a + R - now) (P.T k) hT (by omega)
        rw [← this, Nat.mul_add, Nat.mul_one]
        omega
      · unfold term nxt
        rw [Function.update_of_ne hkt, Function.update_of_ne hkt]
    unfold Phi at hphi ⊢
    omega
  · intro hi'
    by_cases hit : i = t
    · subst hit
      exact ⟨now, by simp, ha⟩
    · rw [Function.update_of_ne hit] at hi' ⊢
      exact hthird hi'

/-- What the invariant gives: while Task `i` has unfinished work, less
than `R` has passed since its Release. -/
theorem LInv.bound (hi : i ∈ L) {now : Nat} {last : Nat → Option Nat} {rm : Nat → Nat}
    (h : LInv P R L i now last rm) (hpos : 0 < rm i) :
    ∃ r, last i = some r ∧ now < r + R := by
  obtain ⟨s0, hs0, hphi, hthird⟩ := h
  obtain ⟨r, hr, hsr⟩ := hthird hpos
  refine ⟨r, hr, ?_⟩
  have := rm_le_sum (P := P) R s0 now last rm hi
  unfold Phi at hphi
  omega

end P18
