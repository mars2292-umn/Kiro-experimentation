import Aeneas
open Aeneas Aeneas.Std Result

/-! # A rule for loops that end by a flag (2026-11-26)

The checking loops of the Generator's `accept` and of the Config_Checker's
`storage_placed_ok` have one shape: a flag and an index; while the index is
below the bound and the flag is set, the element at the index is checked;
a failed check clears the flag and leaves the index, a passed one advances
the index. `flag_loop` is the rule for that shape: the loop returns, and
returns `true` exactly when every index below the bound passes. It states
nothing of either tool; the two tools' specifications stay apart (R26.1). -/

namespace FlagLoop

/-- **The rule.** `hdone`: outside the bound or with the flag clear the
body exits with the flag. `hstep`: inside the bound with the flag set, and
every earlier index passing, the body either clears the flag and leaves
the index, the index failing, or advances the index by one, the index
passing. -/
theorem flag_loop (body : Bool × Usize → Result (ControlFlow (Bool × Usize) Bool)) (n : Nat)
    (P : Nat → Prop)
    (hdone : ∀ ok1 i, (¬ i.val < n ∨ ok1 = false) → body (ok1, i) = ok (ControlFlow.done ok1))
    (hstep : ∀ i : Usize, i.val < n → (∀ j, j < i.val → P j) →
      body (true, i) ⦃ r =>
        (r = ControlFlow.cont (false, i) ∧ ¬ P i.val) ∨
        (∃ i1 : Usize, r = ControlFlow.cont (true, i1) ∧ i1.val = i.val + 1 ∧ P i.val) ⦄) :
    loop body (true, 0#usize) ⦃ b => b = true ↔ ∀ j, j < n → P j ⦄ := by
  apply loop.spec_decr_nat
    (fun (s : Bool × Usize) => 2 * (n - s.2.val) + (if s.1 then 1 else 0))
    (fun s => s.2.val ≤ n ∧ (∀ j, j < s.2.val → P j) ∧ (s.1 = false → s.2.val < n ∧ ¬ P s.2.val))
  · intro s hs
    obtain ⟨ok1, i⟩ := s
    obtain ⟨h1, h2, h3⟩ := hs
    dsimp only at h1 h2 h3 ⊢
    by_cases hlt : i.val < n
    · cases hk : ok1 with
      | false =>
        rw [hdone false i (Or.inr rfl)]
        simp only [WP.spec_ok]
        obtain ⟨_, hbad⟩ := h3 hk
        constructor
        · intro hc; cases hc
        · intro hall; exact absurd (hall i.val hlt) hbad
      | true =>
        refine WP.spec_mono (hstep i hlt h2) ?_
        intro r hr
        rcases hr with ⟨rfl, hbad⟩ | ⟨i1, rfl, hi1, hgood⟩
        · dsimp only
          refine ⟨⟨h1, h2, fun _ => ⟨hlt, hbad⟩⟩, ?_⟩
          simp
        · dsimp only
          refine ⟨⟨by omega, ?_, fun hc => by cases hc⟩, ?_⟩
          · intro j hj
            by_cases hji : j = i.val
            · rw [hji]; exact hgood
            · exact h2 j (by omega)
          · simp only [if_true]
            omega
    · rw [hdone ok1 i (Or.inl hlt)]
      simp only [WP.spec_ok]
      have hin : i.val = n := by omega
      constructor
      · intro _ j hj; exact h2 j (by omega)
      · intro hall
        cases hk : ok1 with
        | true => rfl
        | false => exact absurd (h3 hk).1 (by omega)
  · exact ⟨by scalar_tac, fun j hj => absurd hj (by simp), fun hc => by cases hc⟩

/-- **The rule for a comparison loop**: an index runs over two lists of
one length; an iteration either answers `false` or advances the index by
one, the two elements at the index being equal; past the end the answer
is `true`. Then the loop returns, and answers `true` only when the lists
are equal. -/
theorem agree_loop {α} [Inhabited α] (body : Usize → Result (ControlFlow Usize Bool))
    (a b : List α) (hlen : a.length = b.length)
    (hdone : ∀ i : Usize, ¬ i.val < a.length → body i = ok (ControlFlow.done true))
    (hstep : ∀ i : Usize, i.val < a.length →
      body i ⦃ r => r = ControlFlow.done false ∨
        ∃ i1 : Usize, r = ControlFlow.cont i1 ∧ i1.val = i.val + 1 ∧ a[i.val]! = b[i.val]! ⦄) :
    loop body 0#usize ⦃ r => r = true → a = b ⦄ := by
  apply loop.spec_decr_nat (fun (i : Usize) => a.length - i.val)
    (fun i => i.val ≤ a.length ∧ ∀ k, k < i.val → a[k]! = b[k]!)
  · intro i hi
    obtain ⟨h1, h2⟩ := hi
    by_cases hlt : i.val < a.length
    · refine WP.spec_mono (hstep i hlt) ?_
      intro r hr
      rcases hr with rfl | ⟨i1, rfl, hi1, heq⟩
      · dsimp only
        intro h; cases h
      · dsimp only
        refine ⟨⟨by omega, ?_⟩, by omega⟩
        intro k hk
        by_cases hki : k = i.val
        · rw [hki]; exact heq
        · exact h2 k (by omega)
    · rw [hdone i hlt]
      simp only [WP.spec_ok]
      intro _
      apply List.ext_getElem hlen
      intro k hk1 hk2
      have := h2 k (by omega)
      rw [getElem!_pos a k hk1, getElem!_pos b k hk2] at this
      exact this
  · exact ⟨by scalar_tac, fun k hk => absurd hk (by simp)⟩

end FlagLoop
