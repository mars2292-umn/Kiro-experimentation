import Proto.Refine
/-!
# The merge refines each of its arguments

Port of the section "the merge" of `crates/rsk-protocol/src/multiparty.rs`
(`lemma_merge_refines`, `lemma_merge_ls_refines`).
-/

namespace MP

theorem merge_self (a : L) : merge a a = some a := by
  cases a <;> simp [merge]

theorem merge_cases {a b m : L} (h : merge a b = some m) :
    (a = b ∧ m = a) ∨
    (∃ ba bb mb, a = .mu ba ∧ b = .mu bb ∧ merge ba bb = some mb ∧ m = .mu mb) ∨
    (∃ f la lb ls, a = .recv f la ∧ b = .recv f lb ∧ mergeLs la lb = some ls ∧ m = .recv f ls) := by
  by_cases hab : a = b
  · subst hab
    rw [merge_self] at h
    cases h
    exact Or.inl ⟨rfl, rfl⟩
  · right
    cases a with
    | mu ba =>
      cases b with
      | mu bb =>
        simp [merge, hab] at h
        obtain ⟨mb, hmb, rfl⟩ := h
        exact Or.inl ⟨_, _, _, rfl, rfl, hmb, rfl⟩
      | fin => simp [merge] at h
      | var n => simp [merge] at h
      | send t ls => simp [merge] at h
      | recv f ls => simp [merge] at h
    | recv fa la =>
      cases b with
      | recv fb lb =>
        simp [merge, hab] at h
        obtain ⟨hf, ls, hls, rfl⟩ := h
        subst hf
        exact Or.inr ⟨_, _, _, _, rfl, rfl, hls, rfl⟩
      | fin => simp [merge] at h
      | var n => simp [merge] at h
      | send t ls => simp [merge] at h
      | mu bb => simp [merge] at h
    | fin => cases b <;> simp_all [merge]
    | var n => cases b <;> simp_all [merge]
    | send t ls => cases b <;> simp_all [merge]

theorem mergeLs_cons {label : Nat} {l : L} {rest b r : Ls}
    (h : mergeLs (.cons label l rest) b = some r) :
    ∃ m r2, r = .cons label m r2 ∧ mergeLs rest (b.remove label) = some r2 ∧
      ((∃ lb, b.lookup label = some lb ∧ merge l lb = some m) ∨ (b.lookup label = none ∧ m = l)) := by
  simp only [mergeLs] at h
  split at h
  · rename_i m r2 hm hr2
    cases h
    refine ⟨m, r2, rfl, hr2, ?_⟩
    cases hb : b.lookup label with
    | none => simp [hb] at hm; exact Or.inr ⟨rfl, hm.symm⟩
    | some lb => simp [hb] at hm; exact Or.inl ⟨lb, rfl, hm⟩
  · cases h

/-- The merge refines each of its arguments; the merge of two lists has
every label of either, with a continuation refining the one it came
from. -/
theorem merge_refines_aux (n : Nat) :
    (∀ a b m : L, a.size + b.size ≤ n → merge a b = some m → Refines m a ∧ Refines m b) ∧
    (∀ a b r : Ls, a.size + b.size ≤ n → mergeLs a b = some r → RefinesLs r a ∧ RefinesLs r b) := by
  induction n using Nat.strong_induction_on with
  | _ n ih =>
    constructor
    · intro a b m hn h
      rcases merge_cases h with ⟨rfl, rfl⟩ | ⟨ba, bb, mb, rfl, rfl, hm, rfl⟩ |
        ⟨f, la, lb, ls, rfl, rfl, hls, rfl⟩
      · exact ⟨Refines.refl _, Refines.refl _⟩
      · have := (ih (ba.size + bb.size) (by simp [L.size] at hn; omega)).1 ba bb mb (Nat.le_refl _) hm
        exact ⟨Refines.mu this.1, Refines.mu this.2⟩
      · have := (ih (la.size + lb.size) (by simp [L.size] at hn; omega)).2 la lb ls (Nat.le_refl _) hls
        exact ⟨Refines.recv this.1, Refines.recv this.2⟩
    · intro a b r hn h
      cases a with
      | nil =>
        simp only [mergeLs] at h
        cases h
        exact ⟨fun label ca hl => by simp [Ls.lookup] at hl, RefinesLs.refl _⟩
      | cons label l rest =>
        obtain ⟨m, r2, rfl, hr2, hm⟩ := mergeLs_cons h
        have hsz := Ls.remove_size b label
        have ih2 := (ih (rest.size + (b.remove label).size) (by simp [Ls.size] at hn; omega)).2
          rest (b.remove label) r2 (Nat.le_refl _) hr2
        have hml : Refines m l ∧ ∀ lb, b.lookup label = some lb → Refines m lb := by
          rcases hm with ⟨lb, hlb, hmm⟩ | ⟨hnone, rfl⟩
          · have hs := Ls.lookup_size b hlb
            have := (ih (l.size + lb.size) (by simp [Ls.size] at hn; omega)).1 l lb m (Nat.le_refl _) hmm
            exact ⟨this.1, fun lb' h' => by rw [hlb] at h'; cases h'; exact this.2⟩
          · exact ⟨Refines.refl _, fun lb h' => by rw [hnone] at h'; cases h'⟩
        constructor
        · intro lab ca hl
          simp only [Ls.lookup] at hl ⊢
          by_cases hlab : label = lab
          · rw [if_pos hlab] at hl ⊢
            cases hl
            exact ⟨m, rfl, hml.1⟩
          · rw [if_neg hlab] at hl ⊢
            exact ih2.1 lab ca hl
        · intro lab cb hl
          simp only [Ls.lookup]
          by_cases hlab : label = lab
          · rw [if_pos hlab]
            subst hlab
            exact ⟨m, rfl, hml.2 cb hl⟩
          · rw [if_neg hlab]
            have : (b.remove label).lookup lab = some cb := by
              rw [Ls.remove_lookup_ne b (by omega)]; exact hl
            exact ih2.2 lab cb this

theorem merge_refines {a b m : L} (h : merge a b = some m) : Refines m a ∧ Refines m b :=
  (merge_refines_aux _).1 a b m (Nat.le_refl _) h

theorem mergeLs_refines {a b r : Ls} (h : mergeLs a b = some r) : RefinesLs r a ∧ RefinesLs r b :=
  (merge_refines_aux _).2 a b r (Nat.le_refl _) h

end MP
