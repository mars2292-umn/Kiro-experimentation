import Proto.Spec
/-!
# Refinement

`Refines m a`: `m` is `a` with possibly more labels in its external
choices (what the merge does to a role not involved in a choice); a send
keeps its labels (its continuations refined), an end or a variable is
refined by itself alone. Port of the sections "refinement" and "refinement
under substitution" of `crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

mutual
def L.size : L → Nat
  | .fin => 1
  | .var _ => 1
  | .mu body => 1 + body.size
  | .send _ ls => 1 + ls.size
  | .recv _ ls => 1 + ls.size
def Ls.size : Ls → Nat
  | .nil => 1
  | .cons _ l rest => 1 + l.size + rest.size
end

theorem Ls.lookup_size : ∀ (ls : Ls) {label : Nat} {c : L}, ls.lookup label = some c →
    c.size < ls.size
  | .nil, _, _, h => by simp [Ls.lookup] at h
  | .cons l c' rest, label, c, h => by
    simp only [Ls.lookup] at h
    split at h
    · cases h; simp [Ls.size]; omega
    · have := Ls.lookup_size rest h; simp [Ls.size]; omega

/-- The two lists have the same labels. -/
def SameLabels (a b : Ls) : Prop := ∀ label, (a.lookup label).isSome ↔ (b.lookup label).isSome

theorem SameLabels.refl (ls : Ls) : SameLabels ls ls := fun _ => Iff.rfl

mutual
/-- `m` refines `a`. -/
def Refines : L → L → Prop
  | .fin, a => a = .fin
  | .var n, a => a = .var n
  | .mu bm, a => ∃ ba, a = .mu ba ∧ Refines bm ba
  | .send t ml, a => ∃ al, a = .send t al ∧ SameLabels ml al ∧
      ∀ label ca, al.lookup label = some ca → RefinesAt ml label ca
  | .recv f ml, a => ∃ al, a = .recv f al ∧
      ∀ label ca, al.lookup label = some ca → RefinesAt ml label ca
/-- The continuation of `label` in the list refines `ca`. -/
def RefinesAt : Ls → Nat → L → Prop
  | .nil, _, _ => False
  | .cons l c rest, label, ca => if l = label then Refines c ca else RefinesAt rest label ca
end

theorem refinesAt_iff : ∀ (ml : Ls) (label : Nat) (ca : L),
    RefinesAt ml label ca ↔ ∃ cm, ml.lookup label = some cm ∧ Refines cm ca
  | .nil, label, ca => by simp [RefinesAt, Ls.lookup]
  | .cons l c rest, label, ca => by
    simp only [RefinesAt, Ls.lookup]
    split
    · simp
    · exact refinesAt_iff rest label ca

/-- Every label of `al` is in `ml` with a refining continuation. -/
def RefinesLs (ml al : Ls) : Prop :=
  ∀ label ca, al.lookup label = some ca → ∃ cm, ml.lookup label = some cm ∧ Refines cm ca

theorem refines_fin_left {a : L} : Refines .fin a ↔ a = .fin := by simp [Refines]

theorem refines_var_left {a : L} {n : Nat} : Refines (.var n) a ↔ a = .var n := by simp [Refines]

theorem refines_mu_left {bm a : L} : Refines (.mu bm) a ↔ ∃ ba, a = .mu ba ∧ Refines bm ba := by
  simp [Refines]

theorem refines_send_left {a : L} {t : Nat} {ml : Ls} :
    Refines (.send t ml) a ↔ ∃ al, a = .send t al ∧ SameLabels ml al ∧ RefinesLs ml al := by
  simp only [Refines, RefinesLs, refinesAt_iff]

theorem refines_recv_left {a : L} {f : Nat} {ml : Ls} :
    Refines (.recv f ml) a ↔ ∃ al, a = .recv f al ∧ RefinesLs ml al := by
  simp only [Refines, RefinesLs, refinesAt_iff]

theorem Refines.mu {bm ba : L} (h : Refines bm ba) : Refines (.mu bm) (.mu ba) :=
  refines_mu_left.mpr ⟨ba, rfl, h⟩

theorem Refines.send {t : Nat} {ml al : Ls} (h1 : SameLabels ml al) (h2 : RefinesLs ml al) :
    Refines (.send t ml) (.send t al) :=
  refines_send_left.mpr ⟨al, rfl, h1, h2⟩

theorem Refines.recv {f : Nat} {ml al : Ls} (h2 : RefinesLs ml al) :
    Refines (.recv f ml) (.recv f al) :=
  refines_recv_left.mpr ⟨al, rfl, h2⟩

theorem Refines.refl (a : L) : Refines a a := by
  induction hn : a.size using Nat.strong_induction_on generalizing a with
  | _ n ih =>
    subst hn
    have hls : ∀ ls : Ls, ls.size < a.size → RefinesLs ls ls := fun ls hs label ca hl =>
      ⟨ca, hl, ih _ (by have := Ls.lookup_size ls hl; omega) ca rfl⟩
    cases a with
    | fin => exact refines_fin_left.mpr rfl
    | var n => exact refines_var_left.mpr rfl
    | mu body => exact Refines.mu (ih _ (by simp [L.size]) body rfl)
    | send t ls => exact Refines.send (SameLabels.refl ls) (hls ls (by simp [L.size]))
    | recv f ls => exact Refines.recv (hls ls (by simp [L.size]))

theorem RefinesLs.refl (ls : Ls) : RefinesLs ls ls :=
  fun _ ca h => ⟨ca, h, Refines.refl ca⟩

/-! The kinds a refinement fixes. -/

theorem refines_fin_right {m : L} (h : Refines m .fin) : m = .fin := by
  cases m with
  | fin => rfl
  | var n => cases refines_var_left.mp h
  | mu b => obtain ⟨_, h, _⟩ := refines_mu_left.mp h; cases h
  | send t ml => obtain ⟨_, h, _⟩ := refines_send_left.mp h; cases h
  | recv f ml => obtain ⟨_, h, _⟩ := refines_recv_left.mp h; cases h

theorem refines_var_right {m : L} {n : Nat} (h : Refines m (.var n)) : m = .var n := by
  cases m with
  | fin => cases refines_fin_left.mp h
  | var n' => cases refines_var_left.mp h; rfl
  | mu b => obtain ⟨_, h, _⟩ := refines_mu_left.mp h; cases h
  | send t ml => obtain ⟨_, h, _⟩ := refines_send_left.mp h; cases h
  | recv f ml => obtain ⟨_, h, _⟩ := refines_recv_left.mp h; cases h

theorem refines_mu_right {m ba : L} (h : Refines m (.mu ba)) :
    ∃ bm, m = .mu bm ∧ Refines bm ba := by
  cases m with
  | fin => cases refines_fin_left.mp h
  | var n' => cases refines_var_left.mp h
  | mu b => obtain ⟨ba', h, hr⟩ := refines_mu_left.mp h; cases h; exact ⟨b, rfl, hr⟩
  | send t ml => obtain ⟨_, h, _⟩ := refines_send_left.mp h; cases h
  | recv f ml => obtain ⟨_, h, _⟩ := refines_recv_left.mp h; cases h

theorem refines_send_right {m : L} {t : Nat} {al : Ls} (h : Refines m (.send t al)) :
    ∃ ml, m = .send t ml ∧ SameLabels ml al ∧ RefinesLs ml al := by
  cases m with
  | fin => cases refines_fin_left.mp h
  | var n' => cases refines_var_left.mp h
  | mu b => obtain ⟨_, h, _⟩ := refines_mu_left.mp h; cases h
  | send t' ml =>
    obtain ⟨al', h, h1, h2⟩ := refines_send_left.mp h; cases h; exact ⟨ml, rfl, h1, h2⟩
  | recv f ml => obtain ⟨_, h, _⟩ := refines_recv_left.mp h; cases h

theorem refines_recv_right {m : L} {f : Nat} {al : Ls} (h : Refines m (.recv f al)) :
    ∃ ml, m = .recv f ml ∧ RefinesLs ml al := by
  cases m with
  | fin => cases refines_fin_left.mp h
  | var n' => cases refines_var_left.mp h
  | mu b => obtain ⟨_, h, _⟩ := refines_mu_left.mp h; cases h
  | send t ml => obtain ⟨_, h, _⟩ := refines_send_left.mp h; cases h
  | recv f' ml => obtain ⟨al', h, h2⟩ := refines_recv_left.mp h; cases h; exact ⟨ml, rfl, h2⟩

theorem RefinesLs.trans' {a b c : Ls}
    (ih : ∀ x y z : L, x.size < a.size → Refines x y → Refines y z → Refines x z)
    (h1 : RefinesLs a b) (h2 : RefinesLs b c) : RefinesLs a c := by
  intro label cc hc
  obtain ⟨cb, hb, hbc⟩ := h2 label cc hc
  obtain ⟨ca, ha, hab⟩ := h1 label cb hb
  exact ⟨ca, ha, ih ca cb cc (Ls.lookup_size a ha) hab hbc⟩

theorem Refines.trans {a b c : L} (h1 : Refines a b) (h2 : Refines b c) : Refines a c := by
  induction hn : a.size using Nat.strong_induction_on generalizing a b c with
  | _ n ih =>
    subst hn
    cases a with
    | fin => cases refines_fin_left.mp h1; exact h2
    | var n => cases refines_var_left.mp h1; exact h2
    | mu bm =>
      obtain ⟨bb, rfl, hb⟩ := refines_mu_left.mp h1
      obtain ⟨bc, rfl, hc⟩ := refines_mu_left.mp h2
      exact Refines.mu (ih _ (by simp [L.size]) hb hc rfl)
    | send t ml =>
      obtain ⟨bl, rfl, s1, r1⟩ := refines_send_left.mp h1
      obtain ⟨cl, rfl, s2, r2⟩ := refines_send_left.mp h2
      refine Refines.send (fun l => (s1 l).trans (s2 l)) ?_
      exact RefinesLs.trans'
        (fun x y z hx hxy hyz => ih _ (by simp [L.size]; omega) hxy hyz rfl) r1 r2
    | recv f ml =>
      obtain ⟨bl, rfl, r1⟩ := refines_recv_left.mp h1
      obtain ⟨cl, rfl, r2⟩ := refines_recv_left.mp h2
      refine Refines.recv ?_
      exact RefinesLs.trans'
        (fun x y z hx hxy hyz => ih _ (by simp [L.size]; omega) hxy hyz rfl) r1 r2

theorem RefinesLs.trans {a b c : Ls} (h1 : RefinesLs a b) (h2 : RefinesLs b c) : RefinesLs a c :=
  RefinesLs.trans' (fun _ _ _ _ h h' => h.trans h') h1 h2

/-! ## Refinement under substitution -/

theorem Ls.subst_lookup : ∀ (ls : Ls) (k : Nat) (by' : L) (label : Nat),
    (ls.subst k by').lookup label = (ls.lookup label).map (fun l => l.subst k by')
  | .nil, _, _, _ => by simp [Ls.subst, Ls.lookup]
  | .cons l c rest, k, by', label => by
    simp only [Ls.subst, Ls.lookup]
    split
    · simp
    · exact Ls.subst_lookup rest k by' label

theorem SameLabels.subst {a b : Ls} (h : SameLabels a b) (k : Nat) (x y : L) :
    SameLabels (a.subst k x) (b.subst k y) := by
  intro label
  rw [Ls.subst_lookup, Ls.subst_lookup]
  simpa using h label

theorem refines_subst_same (a : L) (k : Nat) {bm ba : L} (h : Refines bm ba) :
    Refines (a.subst k bm) (a.subst k ba) := by
  induction hn : a.size using Nat.strong_induction_on generalizing a k with
  | _ n ih =>
    subst hn
    have hls : ∀ ls : Ls, ls.size < a.size → RefinesLs (ls.subst k bm) (ls.subst k ba) := by
      intro ls hs label ca hl
      rw [Ls.subst_lookup] at hl ⊢
      cases hc : ls.lookup label with
      | none => simp [hc] at hl
      | some c =>
        simp only [hc, Option.map_some, Option.some.injEq] at hl ⊢
        subst hl
        exact ⟨_, rfl, ih _ (by have := Ls.lookup_size ls hc; omega) c k rfl⟩
    cases a with
    | fin => exact Refines.refl _
    | var n =>
      simp only [L.subst]
      split
      · exact h
      · split <;> exact Refines.refl _
    | mu body => exact Refines.mu (ih _ (by simp [L.size]) body (k + 1) rfl)
    | send t ls =>
      exact Refines.send ((SameLabels.refl ls).subst k bm ba) (hls ls (by simp [L.size]))
    | recv f ls => exact Refines.recv (hls ls (by simp [L.size]))

theorem refines_subst {m a : L} (h : Refines m a) (k : Nat) {bm ba : L} (hb : Refines bm ba) :
    Refines (m.subst k bm) (a.subst k ba) := by
  induction hn : m.size using Nat.strong_induction_on generalizing m a k with
  | _ n ih =>
    subst hn
    have hls : ∀ ml al : Ls, ml.size < m.size → RefinesLs ml al →
        RefinesLs (ml.subst k bm) (al.subst k ba) := by
      intro ml al hs hr label ca hl
      rw [Ls.subst_lookup] at hl ⊢
      cases hc : al.lookup label with
      | none => simp [hc] at hl
      | some c =>
        simp only [hc, Option.map_some, Option.some.injEq] at hl
        subst hl
        obtain ⟨cm, hm, hcm⟩ := hr label c hc
        exact ⟨_, by simp [hm], ih _ (by have := Ls.lookup_size ml hm; omega) hcm k rfl⟩
    cases m with
    | fin => cases refines_fin_left.mp h; exact Refines.refl _
    | var n => cases refines_var_left.mp h; exact refines_subst_same _ k hb
    | mu body =>
      obtain ⟨ba', rfl, hr⟩ := refines_mu_left.mp h
      exact Refines.mu (ih _ (by simp [L.size]) hr (k + 1) rfl)
    | send t ml =>
      obtain ⟨al, rfl, s1, r1⟩ := refines_send_left.mp h
      exact Refines.send (s1.subst k bm ba) (hls ml al (by simp [L.size]) r1)
    | recv f ml =>
      obtain ⟨al, rfl, r1⟩ := refines_recv_left.mp h
      exact Refines.recv (hls ml al (by simp [L.size]) r1)

/-- The leading recursions of a refinement are those of the refined type. -/
theorem refines_lead : ∀ (m : L) {a : L}, Refines m a → m.leadRecs = a.leadRecs
  | .mu body, a, h => by
    obtain ⟨ba, rfl, hr⟩ := refines_mu_left.mp h
    simp [L.leadRecs, refines_lead body hr]
  | .fin, a, h => by cases refines_fin_left.mp h; rfl
  | .var n, a, h => by cases refines_var_left.mp h; rfl
  | .send t ml, a, h => by obtain ⟨al, rfl, _⟩ := refines_send_left.mp h; rfl
  | .recv f ml, a, h => by obtain ⟨al, rfl, _⟩ := refines_recv_left.mp h; rfl

theorem refines_headN {m a : L} (h : Refines m a) (n : Nat) : Refines (m.headN n) (a.headN n) := by
  induction n generalizing m a with
  | zero => simpa [L.headN] using h
  | succ n ih =>
    cases a with
    | mu ba =>
      obtain ⟨bm, rfl, hr⟩ := refines_mu_right h
      simp only [L.headN, L.unfold]
      exact ih (refines_subst hr 0 h)
    | fin => cases refines_fin_right h; simpa [L.headN] using Refines.refl _
    | var v => cases refines_var_right h; simpa [L.headN] using Refines.refl _
    | send t al =>
      obtain ⟨ml, rfl, _⟩ := refines_send_right h
      simpa [L.headN] using h
    | recv f al =>
      obtain ⟨ml, rfl, _⟩ := refines_recv_right h
      simpa [L.headN] using h

/-- Refinement is kept by head normalization. -/
theorem refines_head {m a : L} (h : Refines m a) : Refines m.head a.head := by
  unfold L.head
  rw [refines_lead m h]
  exact refines_headN h _

/-! ## Removal from a branch list -/

theorem Ls.remove_lookup_ne : ∀ (ls : Ls) {label lab : Nat}, lab ≠ label →
    (ls.remove label).lookup lab = ls.lookup lab
  | .nil, _, _, _ => rfl
  | .cons l c rest, label, lab, h => by
    simp only [Ls.remove]
    split
    · rename_i hl
      simp only [Ls.lookup, Ls.remove_lookup_ne rest h]
      rw [if_neg (by omega)]
    · simp only [Ls.lookup, Ls.remove_lookup_ne rest h]

theorem Ls.remove_lookup_self : ∀ (ls : Ls) (label : Nat), (ls.remove label).lookup label = none
  | .nil, _ => rfl
  | .cons l c rest, label => by
    simp only [Ls.remove]
    split
    · exact Ls.remove_lookup_self rest label
    · rename_i hl
      simp only [Ls.lookup, Ls.remove_lookup_self rest label]
      rw [if_neg hl]

theorem Ls.remove_size : ∀ (ls : Ls) (label : Nat), (ls.remove label).size ≤ ls.size
  | .nil, _ => by simp [Ls.remove]
  | .cons l c rest, label => by
    have := Ls.remove_size rest label
    simp only [Ls.remove]
    split <;> simp [Ls.size] <;> omega

end MP
