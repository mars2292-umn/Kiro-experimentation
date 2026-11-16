import Proto.Runtime
/-!
# The merged projection of a runtime type

`rproj` is `projn` on runtime types: a `sent` is, for its receiver, the
external choice of the one label in transit, and for anyone else the
continuation. Used for the initial relation alone. Port of the section
"the merged projection" of `crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

mutual
def rproj : R → Nat → Option L
  | .fin, _ => some .fin
  | .var n, _ => some (.var n)
  | .mu body, u => (rproj body u).map closeRec
  | .comm frm dst bs, u =>
    if u = frm then (rprojBs bs u).map (L.send dst)
    else if u = dst then (rprojBs bs u).map (L.recv frm)
    else rprojMerge bs u
  | .sent frm dst label cont, u =>
    match rproj cont u with
    | none => none
    | some l => if u = dst then some (.recv frm (.cons label l .nil)) else some l
def rprojBs : RBs → Nat → Option Ls
  | .nil, _ => some .nil
  | .cons label g rest, u =>
    match rproj g u, rprojBs rest u with
    | some l, some ls => some (.cons label l ls)
    | _, _ => none
def rprojMerge : RBs → Nat → Option L
  | .nil, _ => none
  | .cons _ g rest, u =>
    match rproj g u with
    | none => none
    | some l =>
      match rest with
      | .nil => some l
      | .cons .. => (rprojMerge rest u).bind (merge l)
end

mutual
theorem rproj_emb : ∀ (g : G) (u : Nat), rproj (emb g) u = projn g u
  | .fin, _ => rfl
  | .var _, _ => rfl
  | .mu body, u => by simp only [emb, rproj, projn, rproj_emb body u]
  | .comm frm dst bs, u => by
    simp only [emb, rproj, projn, rprojBs_emb bs u, rprojMerge_emb bs u]
theorem rprojBs_emb : ∀ (bs : Bs) (u : Nat), rprojBs (embBs bs) u = projnBs bs u
  | .nil, _ => rfl
  | .cons label g rest, u => by
    simp only [embBs, rprojBs, projnBs, rproj_emb g u, rprojBs_emb rest u]
    cases projn g u <;> cases projnBs rest u <;> rfl
theorem rprojMerge_emb : ∀ (bs : Bs) (u : Nat), rprojMerge (embBs bs) u = projnMerge bs u
  | .nil, _ => rfl
  | .cons label g rest, u => by
    simp only [embBs, rprojMerge, projnMerge, rproj_emb g u]
    cases projn g u with
    | none => rfl
    | some l =>
      cases rest with
      | nil => rfl
      | cons l2 g2 r2 =>
        have ih := rprojMerge_emb (.cons l2 g2 r2) u
        simp only [embBs] at ih ⊢
        rw [ih]
end

theorem rprojBs_cons {label : Nat} {g : R} {rest : RBs} {u : Nat} {ls : Ls}
    (h : rprojBs (.cons label g rest) u = some ls) :
    ∃ l ls', rproj g u = some l ∧ rprojBs rest u = some ls' ∧ ls = .cons label l ls' := by
  simp only [rprojBs] at h
  split at h
  · rename_i l ls' h1 h2
    cases h
    exact ⟨l, ls', h1, h2, rfl⟩
  · cases h

/-- The lookups of a projected branch list are the projections of the
branches. -/
theorem rprojBs_lookup : ∀ (bs : RBs) (u : Nat) (ls : Ls), rprojBs bs u = some ls →
    (∀ label g, bs.lookup label = some g → ∃ l, rproj g u = some l ∧ ls.lookup label = some l) ∧
    (∀ label, bs.lookup label = none → ls.lookup label = none)
  | .nil, u, ls, h => by
    simp only [rprojBs] at h
    cases h
    exact ⟨fun label g hg => by simp [RBs.lookup] at hg, fun _ _ => rfl⟩
  | .cons l0 g0 rest, u, ls, h => by
    obtain ⟨l, ls', h1, h2, rfl⟩ := rprojBs_cons h
    have ih := rprojBs_lookup rest u ls' h2
    constructor
    · intro label g hg
      simp only [RBs.lookup] at hg
      simp only [Ls.lookup]
      split at hg
      · cases hg; rename_i e; rw [if_pos e]; exact ⟨l, h1, rfl⟩
      · rename_i e; rw [if_neg e]; exact ih.1 label g hg
    · intro label hn
      simp only [RBs.lookup] at hn
      simp only [Ls.lookup]
      split at hn
      · cases hn
      · rename_i e; rw [if_neg e]; exact ih.2 label hn

theorem rprojMerge_cons {label : Nat} {g : R} {rest : RBs} {u : Nat} {m : L}
    (h : rprojMerge (.cons label g rest) u = some m) :
    ∃ l, rproj g u = some l ∧
      ((rest = .nil ∧ m = l) ∨ (rest ≠ .nil ∧ ∃ mr, rprojMerge rest u = some mr ∧ merge l mr = some m)) := by
  simp only [rprojMerge] at h
  cases hg : rproj g u with
  | none => simp [hg] at h
  | some l =>
    simp only [hg] at h
    refine ⟨l, rfl, ?_⟩
    cases rest with
    | nil => simp at h; exact Or.inl ⟨rfl, h.symm⟩
    | cons l2 g2 r2 =>
      right
      refine ⟨by simp, ?_⟩
      cases hm : rprojMerge (.cons l2 g2 r2) u with
      | none => simp [hm] at h
      | some mr => simp [hm] at h; exact ⟨mr, rfl, h⟩

/-- The merge of the branches' projections refines each branch's. -/
theorem rprojMerge_refines : ∀ (bs : RBs) (u : Nat) (m : L), rprojMerge bs u = some m →
    ∀ label g, bs.lookup label = some g → ∃ l, rproj g u = some l ∧ Refines m l
  | .nil, u, m, h => by simp [rprojMerge] at h
  | .cons l0 g0 rest, u, m, h => by
    obtain ⟨l, hl, hcase⟩ := rprojMerge_cons h
    intro label g hg
    simp only [RBs.lookup] at hg
    rcases hcase with ⟨rfl, rfl⟩ | ⟨_, mr, hmr, hm⟩
    · split at hg
      · cases hg; exact ⟨_, hl, Refines.refl _⟩
      · simp [RBs.lookup] at hg
    · have hr := merge_refines hm
      split at hg
      · cases hg; exact ⟨l, hl, hr.1⟩
      · obtain ⟨l', hl', hr'⟩ := rprojMerge_refines rest u mr hmr label g hg
        exact ⟨l', hl', hr.2.trans hr'⟩

end MP
