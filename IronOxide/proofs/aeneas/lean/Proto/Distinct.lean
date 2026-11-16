import Proto.RProj
/-!
# Distinct labels, and the merge under substitution

Port of the sections "distinctness" and "the merge commutes with
substitution" of `crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

mutual
/-- Every choice of the type has pairwise distinct labels. -/
def L.distinct : L → Bool
  | .mu body => body.distinct
  | .send _ ls => ls.distinct
  | .recv _ ls => ls.distinct
  | _ => true
def Ls.distinct : Ls → Bool
  | .nil => true
  | .cons label l rest => (rest.lookup label).isNone && l.distinct && rest.distinct
end

theorem Ls.distinct_lookup : ∀ (ls : Ls) {label : Nat} {c : L}, ls.distinct = true →
    ls.lookup label = some c → c.distinct = true
  | .nil, _, _, _, h => by simp [Ls.lookup] at h
  | .cons l0 c0 rest, label, c, hd, h => by
    simp only [Ls.distinct, Bool.and_eq_true] at hd
    simp only [Ls.lookup] at h
    split at h
    · cases h; exact hd.1.2
    · exact Ls.distinct_lookup rest hd.2 h

mutual
theorem L.subst_distinct : ∀ (l : L) (k : Nat) (by' : L), l.distinct = true →
    by'.distinct = true → (l.subst k by').distinct = true
  | .fin, _, _, _, _ => rfl
  | .var n, k, by', _, hb => by
    simp only [L.subst]
    split
    · exact hb
    · split <;> rfl
  | .mu body, k, by', h, hb => by
    simp only [L.distinct] at h
    simp only [L.subst, L.distinct]
    exact L.subst_distinct body (k + 1) by' h hb
  | .send t ls, k, by', h, hb => by
    simp only [L.distinct] at h
    simp only [L.subst, L.distinct]
    exact Ls.subst_distinct ls k by' h hb
  | .recv f ls, k, by', h, hb => by
    simp only [L.distinct] at h
    simp only [L.subst, L.distinct]
    exact Ls.subst_distinct ls k by' h hb
theorem Ls.subst_distinct : ∀ (ls : Ls) (k : Nat) (by' : L), ls.distinct = true →
    by'.distinct = true → (ls.subst k by').distinct = true
  | .nil, _, _, _, _ => rfl
  | .cons label l rest, k, by', h, hb => by
    simp only [Ls.distinct, Bool.and_eq_true] at h
    simp only [Ls.subst, Ls.distinct, Bool.and_eq_true, Ls.subst_lookup]
    refine ⟨⟨?_, L.subst_distinct l k by' h.1.2 hb⟩, Ls.subst_distinct rest k by' h.2 hb⟩
    cases hr : rest.lookup label with
    | none => rfl
    | some c => rw [hr] at h; simp at h
end

/-- A label of the merge of two lists is a label of one of them. -/
theorem mergeLs_labels : ∀ (a b r : Ls), mergeLs a b = some r → ∀ label,
    (r.lookup label).isSome = true → (a.lookup label).isSome = true ∨ (b.lookup label).isSome = true
  | .nil, b, r, h, label, hl => by
    simp only [mergeLs] at h
    cases h
    exact Or.inr hl
  | .cons l0 l rest, b, r, h, label, hl => by
    obtain ⟨m, r2, rfl, hr2, _⟩ := mergeLs_cons h
    simp only [Ls.lookup] at hl ⊢
    by_cases e : l0 = label
    · left; rw [if_pos e]; rfl
    · rw [if_neg e] at hl ⊢
      rcases mergeLs_labels rest (b.remove l0) r2 hr2 label hl with h1 | h1
      · exact Or.inl h1
      · rw [Ls.remove_lookup_ne b (by omega)] at h1
        exact Or.inr h1

theorem Ls.remove_distinct : ∀ (ls : Ls) (label : Nat), ls.distinct = true →
    (ls.remove label).distinct = true
  | .nil, _, _ => rfl
  | .cons l0 c rest, label, h => by
    simp only [Ls.distinct, Bool.and_eq_true] at h
    simp only [Ls.remove]
    split
    · exact Ls.remove_distinct rest label h.2
    · rename_i hne
      simp only [Ls.distinct, Bool.and_eq_true]
      refine ⟨⟨?_, h.1.2⟩, Ls.remove_distinct rest label h.2⟩
      rw [Ls.remove_lookup_ne rest hne]
      exact h.1.1

theorem merge_distinct_aux (n : Nat) :
    (∀ a b m : L, a.size + b.size ≤ n → merge a b = some m → a.distinct = true →
      b.distinct = true → m.distinct = true) ∧
    (∀ a b r : Ls, a.size + b.size ≤ n → mergeLs a b = some r → a.distinct = true →
      b.distinct = true → r.distinct = true) := by
  induction n using Nat.strong_induction_on with
  | _ n ih =>
    constructor
    · intro a b m hn h ha hb
      rcases merge_cases h with ⟨rfl, rfl⟩ | ⟨ba, bb, mb, rfl, rfl, hm, rfl⟩ |
        ⟨f, la, lb, ls, rfl, rfl, hls, rfl⟩
      · exact ha
      · simp only [L.distinct] at ha hb ⊢
        exact (ih (ba.size + bb.size) (by simp [L.size] at hn; omega)).1 ba bb mb
          (Nat.le_refl _) hm ha hb
      · simp only [L.distinct] at ha hb ⊢
        exact (ih (la.size + lb.size) (by simp [L.size] at hn; omega)).2 la lb ls
          (Nat.le_refl _) hls ha hb
    · intro a b r hn h ha hb
      cases a with
      | nil =>
        simp only [mergeLs] at h
        cases h
        exact hb
      | cons label l rest =>
        obtain ⟨m, r2, rfl, hr2, hm⟩ := mergeLs_cons h
        simp only [Ls.distinct, Bool.and_eq_true] at ha
        have hsz := Ls.remove_size b label
        have hb2 := Ls.remove_distinct b label hb
        have ih2 := (ih (rest.size + (b.remove label).size) (by simp [Ls.size] at hn; omega)).2
          rest (b.remove label) r2 (Nat.le_refl _) hr2 ha.2 hb2
        simp only [Ls.distinct, Bool.and_eq_true]
        refine ⟨⟨?_, ?_⟩, ih2⟩
        · cases hr : r2.lookup label with
          | none => rfl
          | some c =>
            exfalso
            rcases mergeLs_labels rest (b.remove label) r2 hr2 label (by simp [hr]) with h1 | h1
            · rw [Option.isNone_iff_eq_none.mp ha.1.1] at h1; cases h1
            · rw [Ls.remove_lookup_self] at h1; cases h1
        · rcases hm with ⟨lb, hlb, hmm⟩ | ⟨_, rfl⟩
          · have hs := Ls.lookup_size b hlb
            exact (ih (l.size + lb.size) (by simp [Ls.size] at hn; omega)).1 l lb m
              (Nat.le_refl _) hmm ha.1.2 (Ls.distinct_lookup b hb hlb)
          · exact ha.1.2

theorem merge_distinct {a b m : L} (h : merge a b = some m) (ha : a.distinct = true)
    (hb : b.distinct = true) : m.distinct = true :=
  (merge_distinct_aux _).1 a b m (Nat.le_refl _) h ha hb

theorem mergeLs_distinct {a b r : Ls} (h : mergeLs a b = some r) (ha : a.distinct = true)
    (hb : b.distinct = true) : r.distinct = true :=
  (merge_distinct_aux _).2 a b r (Nat.le_refl _) h ha hb

/-! ## The merge commutes with substitution -/

theorem Ls.remove_subst : ∀ (ls : Ls) (label k : Nat) (by' : L),
    (ls.subst k by').remove label = (ls.remove label).subst k by'
  | .nil, _, _, _ => rfl
  | .cons l0 c rest, label, k, by' => by
    simp only [Ls.subst, Ls.remove]
    split
    · exact Ls.remove_subst rest label k by'
    · simp only [Ls.subst, Ls.remove_subst rest label k by']

theorem Ls.remove_absent : ∀ (ls : Ls) (label : Nat), ls.lookup label = none →
    ls.remove label = ls
  | .nil, _, _ => rfl
  | .cons l0 c rest, label, h => by
    simp only [Ls.lookup] at h
    split at h
    · cases h
    · rename_i hne
      simp only [Ls.remove, if_neg hne, Ls.remove_absent rest label h]

/-- Two distinct types that merge stay distinct under substitution (their
difference is in their labels or below equal labels, never a variable
against a term). -/
theorem merge_neq_subst_aux (n : Nat) :
    (∀ (a b m : L) (k : Nat) (by' : L), a.size + b.size ≤ n → merge a b = some m → a ≠ b →
      a.distinct = true → b.distinct = true → a.subst k by' ≠ b.subst k by') ∧
    (∀ (a b r : Ls) (k : Nat) (by' : L), a.size + b.size ≤ n → mergeLs a b = some r → a ≠ b →
      a.distinct = true → b.distinct = true → a.subst k by' ≠ b.subst k by') := by
  induction n using Nat.strong_induction_on with
  | _ n ih =>
    constructor
    · intro a b m k by' hn h hab ha hb
      rcases merge_cases h with ⟨rfl, rfl⟩ | ⟨ba, bb, mb, rfl, rfl, hm, rfl⟩ |
        ⟨f, la, lb, ls, rfl, rfl, hls, rfl⟩
      · exact absurd rfl hab
      · simp only [L.distinct] at ha hb
        have := (ih (ba.size + bb.size) (by simp [L.size] at hn; omega)).1 ba bb mb (k + 1) by'
          (Nat.le_refl _) hm (fun e => hab (by rw [e])) ha hb
        simp only [L.subst]
        intro e
        exact this (L.mu.inj e)
      · simp only [L.distinct] at ha hb
        have := (ih (la.size + lb.size) (by simp [L.size] at hn; omega)).2 la lb ls k by'
          (Nat.le_refl _) hls (fun e => hab (by rw [e])) ha hb
        simp only [L.subst]
        intro e
        exact this (L.recv.inj e).2
    · intro a b r k by' hn h hab ha hb
      cases a with
      | nil =>
        cases b with
        | nil => exact absurd rfl hab
        | cons lb0 lbl lbrest => simp [Ls.subst]
      | cons label l rest =>
        cases b with
        | nil => simp [Ls.subst]
        | cons lb0 lbl lbrest =>
          simp only [Ls.subst]
          intro e
          obtain ⟨e1, e2, e3⟩ := Ls.cons.inj e
          subst e1
          obtain ⟨m, r2, rfl, hr2, hm⟩ := mergeLs_cons h
          simp only [Ls.distinct, Bool.and_eq_true] at ha hb
          have hlook : (Ls.cons label lbl lbrest).lookup label = some lbl := by simp [Ls.lookup]
          by_cases hl : l = lbl
          · subst hl
            have hrem : (Ls.cons label l lbrest).remove label = lbrest := by
              simp only [Ls.remove, if_true]
              exact Ls.remove_absent lbrest label (Option.isNone_iff_eq_none.mp hb.1.1)
            rw [hrem] at hr2
            exact (ih (rest.size + lbrest.size) (by simp [Ls.size] at hn; omega)).2 rest lbrest r2
              k by' (Nat.le_refl _) hr2 (fun e' => hab (by rw [e'])) ha.2 hb.2 e3
          · rcases hm with ⟨lb, hlb, hmm⟩ | ⟨hnone, _⟩
            · rw [hlook] at hlb
              cases hlb
              exact (ih (l.size + lbl.size) (by simp [Ls.size] at hn; omega)).1 l lbl m k by'
                (Nat.le_refl _) hmm hl ha.1.2 hb.1.2 e2
            · rw [hlook] at hnone; cases hnone

theorem merge_subst_aux (n : Nat) :
    (∀ (a b m : L) (k : Nat) (by' : L), a.size + b.size ≤ n → merge a b = some m →
      a.distinct = true → b.distinct = true →
      merge (a.subst k by') (b.subst k by') = some (m.subst k by')) ∧
    (∀ (a b r : Ls) (k : Nat) (by' : L), a.size + b.size ≤ n → mergeLs a b = some r →
      a.distinct = true → b.distinct = true →
      mergeLs (a.subst k by') (b.subst k by') = some (r.subst k by')) := by
  induction n using Nat.strong_induction_on with
  | _ n ih =>
    constructor
    · intro a b m k by' hn h ha hb
      by_cases hab : a = b
      · subst hab
        rw [merge_self] at h
        cases h
        exact merge_self _
      · have hne := (merge_neq_subst_aux _).1 a b m k by' (Nat.le_refl _) h hab ha hb
        rcases merge_cases h with ⟨rfl, rfl⟩ | ⟨ba, bb, mb, rfl, rfl, hm, rfl⟩ |
          ⟨f, la, lb, ls, rfl, rfl, hls, rfl⟩
        · exact absurd rfl hab
        · simp only [L.distinct] at ha hb
          have := (ih (ba.size + bb.size) (by simp [L.size] at hn; omega)).1 ba bb mb (k + 1) by'
            (Nat.le_refl _) hm ha hb
          simp only [L.subst] at hne ⊢
          simp [merge, hne, this]
        · simp only [L.distinct] at ha hb
          have := (ih (la.size + lb.size) (by simp [L.size] at hn; omega)).2 la lb ls k by'
            (Nat.le_refl _) hls ha hb
          simp only [L.subst] at hne ⊢
          simp [merge, hne, this]
    · intro a b r k by' hn h ha hb
      cases a with
      | nil =>
        simp only [mergeLs] at h
        cases h
        simp [Ls.subst, mergeLs]
      | cons label l rest =>
        obtain ⟨m, r2, rfl, hr2, hm⟩ := mergeLs_cons h
        simp only [Ls.distinct, Bool.and_eq_true] at ha
        have hsz := Ls.remove_size b label
        have hb2 := Ls.remove_distinct b label hb
        have ih2 := (ih (rest.size + (b.remove label).size) (by simp [Ls.size] at hn; omega)).2
          rest (b.remove label) r2 k by' (Nat.le_refl _) hr2 ha.2 hb2
        simp only [Ls.subst, mergeLs, Ls.subst_lookup, Ls.remove_subst, ih2]
        rcases hm with ⟨lb, hlb, hmm⟩ | ⟨hnone, rfl⟩
        · have hs := Ls.lookup_size b hlb
          have := (ih (l.size + lb.size) (by simp [Ls.size] at hn; omega)).1 l lb m k by'
            (Nat.le_refl _) hmm ha.1.2 (Ls.distinct_lookup b hb hlb)
          simp [hlb, this]
        · simp [hnone]

theorem merge_subst {a b m : L} (h : merge a b = some m) (ha : a.distinct = true)
    (hb : b.distinct = true) (k : Nat) (by' : L) :
    merge (a.subst k by') (b.subst k by') = some (m.subst k by') :=
  (merge_subst_aux _).1 a b m k by' (Nat.le_refl _) h ha hb

end MP
