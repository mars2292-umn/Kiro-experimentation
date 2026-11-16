import Proto.Distinct
/-!
# Binders: closedness, shifts, the recursion rule

Port of the section "binders: closedness, shifts, the rule" of
`crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

mutual
/-- Every variable of `l` refers to one of the `m` innermost binders. -/
def L.closedAt : L → Nat → Bool
  | .fin, _ => true
  | .var n, m => decide (n < m)
  | .mu body, m => body.closedAt (m + 1)
  | .send _ ls, m => ls.closedAt m
  | .recv _ ls, m => ls.closedAt m
def Ls.closedAt : Ls → Nat → Bool
  | .nil, _ => true
  | .cons _ l rest, m => l.closedAt m && rest.closedAt m
end

mutual
/-- No recursion with a body of no action (what `closeRec` keeps). -/
def L.knormal : L → Bool
  | .mu body => !body.actionless && body.knormal
  | .send _ ls => ls.knormal
  | .recv _ ls => ls.knormal
  | _ => true
def Ls.knormal : Ls → Bool
  | .nil => true
  | .cons _ l rest => l.knormal && rest.knormal
end

theorem knormal_actionless {l : L} (hk : l.knormal = true) (ha : l.actionless = true) :
    l = .fin ∨ ∃ n, l = .var n := by
  cases l with
  | fin => exact Or.inl rfl
  | var n => exact Or.inr ⟨n, rfl⟩
  | mu body =>
    simp only [L.knormal, Bool.and_eq_true, Bool.not_eq_true'] at hk
    simp only [L.actionless] at ha
    rw [ha] at hk; cases hk.1
  | send t ls => simp [L.actionless] at ha
  | recv f ls => simp [L.actionless] at ha

theorem Ls.shift_lookup : ∀ (ls : Ls) (k : Nat) (label : Nat),
    (ls.shift k).lookup label = (ls.lookup label).map (fun l => l.shift k)
  | .nil, _, _ => by simp [Ls.shift, Ls.lookup]
  | .cons l c rest, k, label => by
    simp only [Ls.shift, Ls.lookup]
    split
    · simp
    · exact Ls.shift_lookup rest k label

mutual
theorem L.shift_shape : ∀ (l : L) (k : Nat), l.distinct = true →
    (l.shift k).distinct = true ∧ (l.shift k).actionless = l.actionless ∧
      (l.knormal = true → (l.shift k).knormal = true)
  | .fin, _, _ => ⟨rfl, rfl, fun _ => rfl⟩
  | .var n, k, _ => by
    simp only [L.shift]
    split <;> exact ⟨rfl, rfl, fun _ => rfl⟩
  | .mu body, k, h => by
    simp only [L.distinct] at h
    have := L.shift_shape body (k + 1) h
    simp only [L.shift, L.distinct, L.actionless, L.knormal, Bool.and_eq_true,
      Bool.not_eq_true']
    exact ⟨this.1, this.2.1, fun hk => ⟨by rw [this.2.1]; exact hk.1, this.2.2 hk.2⟩⟩
  | .send t ls, k, h => by
    simp only [L.distinct] at h
    have := Ls.shift_shape ls k h
    simp only [L.shift, L.distinct, L.actionless, L.knormal]
    exact ⟨this.1, trivial, this.2⟩
  | .recv f ls, k, h => by
    simp only [L.distinct] at h
    have := Ls.shift_shape ls k h
    simp only [L.shift, L.distinct, L.actionless, L.knormal]
    exact ⟨this.1, trivial, this.2⟩
theorem Ls.shift_shape : ∀ (ls : Ls) (k : Nat), ls.distinct = true →
    (ls.shift k).distinct = true ∧ (ls.knormal = true → (ls.shift k).knormal = true)
  | .nil, _, _ => ⟨rfl, fun _ => rfl⟩
  | .cons label l rest, k, h => by
    simp only [Ls.distinct, Bool.and_eq_true] at h
    have h1 := L.shift_shape l k h.1.2
    have h2 := Ls.shift_shape rest k h.2
    simp only [Ls.shift, Ls.distinct, Ls.knormal, Bool.and_eq_true, Ls.shift_lookup]
    refine ⟨⟨⟨?_, h1.1⟩, h2.1⟩, fun hk => ⟨h1.2.2 hk.1, h2.2 hk.2⟩⟩
    cases hr : rest.lookup label with
    | none => rfl
    | some c => rw [hr] at h; simp at h
end

/-- `closeRec` keeps distinctness and normality. -/
theorem closeRec_shape {l : L} (hd : l.distinct = true) (hk : l.knormal = true) :
    (closeRec l).distinct = true ∧ (closeRec l).knormal = true ∧
      (closeRec l).actionless = l.actionless := by
  unfold closeRec
  by_cases ha : l.actionless = true
  · rw [if_pos ha]
    rcases knormal_actionless hk ha with rfl | ⟨n, rfl⟩
    · exact ⟨rfl, rfl, rfl⟩
    · dsimp only
      split <;> exact ⟨rfl, rfl, rfl⟩
  · rw [if_neg ha]
    by_cases hf : l.free 0 = true
    · rw [if_pos hf]
      simp only [L.distinct, L.knormal, L.actionless, Bool.and_eq_true, Bool.not_eq_true']
      exact ⟨hd, ⟨by simpa using ha, hk⟩, by simpa using ha⟩
    · rw [if_neg hf]
      have := L.shift_shape l 0 hd
      exact ⟨this.1, this.2.2 hk, this.2.1⟩

mutual
/-- A closed type has no free variable and is unchanged by a shift. -/
theorem L.closed_shift : ∀ (l : L) (m k : Nat), l.closedAt m = true → m ≤ k →
    l.free k = false ∧ l.shift k = l
  | .fin, _, _, _, _ => ⟨rfl, rfl⟩
  | .var n, m, k, h, hmk => by
    simp only [L.closedAt, decide_eq_true_eq] at h
    simp only [L.free, L.shift]
    exact ⟨by simp; omega, by rw [if_neg (by omega)]⟩
  | .mu body, m, k, h, hmk => by
    simp only [L.closedAt] at h
    have := L.closed_shift body (m + 1) (k + 1) h (by omega)
    simp only [L.free, L.shift]
    exact ⟨this.1, by rw [this.2]⟩
  | .send t ls, m, k, h, hmk => by
    simp only [L.closedAt] at h
    have := Ls.closed_shift ls m k h hmk
    simp only [L.free, L.shift]
    exact ⟨this.1, by rw [this.2]⟩
  | .recv f ls, m, k, h, hmk => by
    simp only [L.closedAt] at h
    have := Ls.closed_shift ls m k h hmk
    simp only [L.free, L.shift]
    exact ⟨this.1, by rw [this.2]⟩
theorem Ls.closed_shift : ∀ (ls : Ls) (m k : Nat), ls.closedAt m = true → m ≤ k →
    ls.free k = false ∧ ls.shift k = ls
  | .nil, _, _, _, _ => ⟨rfl, rfl⟩
  | .cons label l rest, m, k, h, hmk => by
    simp only [Ls.closedAt, Bool.and_eq_true] at h
    have h1 := L.closed_shift l m k h.1 hmk
    have h2 := Ls.closed_shift rest m k h.2 hmk
    simp [Ls.free, Ls.shift, h1.1, h1.2, h2.1, h2.2]
end

mutual
/-- Removing an unused binder keeps closedness one lower. -/
theorem L.shift_closed : ∀ (l : L) (d m : Nat), l.closedAt (m + 1) = true → l.free d = false →
    d ≤ m → (l.shift d).closedAt m = true
  | .fin, _, _, _, _, _ => rfl
  | .var n, d, m, h, hf, hd => by
    simp only [L.closedAt, decide_eq_true_eq] at h
    simp only [L.free, beq_eq_false_iff_ne, ne_eq] at hf
    simp only [L.shift]
    split
    · simp only [L.closedAt, decide_eq_true_eq]; omega
    · simp only [L.closedAt, decide_eq_true_eq]; omega
  | .mu body, d, m, h, hf, hd => by
    simp only [L.closedAt] at h
    simp only [L.free] at hf
    simp only [L.shift, L.closedAt]
    exact L.shift_closed body (d + 1) (m + 1) h hf (by omega)
  | .send t ls, d, m, h, hf, hd => by
    simp only [L.closedAt] at h
    simp only [L.free] at hf
    simp only [L.shift, L.closedAt]
    exact Ls.shift_closed ls d m h hf hd
  | .recv f ls, d, m, h, hf, hd => by
    simp only [L.closedAt] at h
    simp only [L.free] at hf
    simp only [L.shift, L.closedAt]
    exact Ls.shift_closed ls d m h hf hd
theorem Ls.shift_closed : ∀ (ls : Ls) (d m : Nat), ls.closedAt (m + 1) = true →
    ls.free d = false → d ≤ m → (ls.shift d).closedAt m = true
  | .nil, _, _, _, _, _ => rfl
  | .cons label l rest, d, m, h, hf, hd => by
    simp only [Ls.closedAt, Bool.and_eq_true] at h
    simp only [Ls.free, Bool.or_eq_false_iff] at hf
    simp only [Ls.shift, Ls.closedAt, Bool.and_eq_true]
    exact ⟨L.shift_closed l d m h.1 hf.1 hd, Ls.shift_closed rest d m h.2 hf.2 hd⟩
end

mutual
/-- Substituting for an unused variable is removing its binder. -/
theorem L.subst_notfree : ∀ (l : L) (k : Nat) (by' : L), l.free k = false →
    l.subst k by' = l.shift k
  | .fin, _, _, _ => rfl
  | .var n, k, by', hf => by
    simp only [L.free, beq_eq_false_iff_ne, ne_eq] at hf
    simp only [L.subst, L.shift, if_neg hf]
  | .mu body, k, by', hf => by
    simp only [L.free] at hf
    simp only [L.subst, L.shift, L.subst_notfree body (k + 1) by' hf]
  | .send t ls, k, by', hf => by
    simp only [L.free] at hf
    simp only [L.subst, L.shift, Ls.subst_notfree ls k by' hf]
  | .recv f ls, k, by', hf => by
    simp only [L.free] at hf
    simp only [L.subst, L.shift, Ls.subst_notfree ls k by' hf]
theorem Ls.subst_notfree : ∀ (ls : Ls) (k : Nat) (by' : L), ls.free k = false →
    ls.subst k by' = ls.shift k
  | .nil, _, _, _ => rfl
  | .cons label l rest, k, by', hf => by
    simp only [Ls.free, Bool.or_eq_false_iff] at hf
    simp only [Ls.subst, Ls.shift, L.subst_notfree l k by' hf.1, Ls.subst_notfree rest k by' hf.2]
end

mutual
/-- The free variables after substituting a closed type. -/
theorem L.free_subst : ∀ (l : L) (k : Nat) (by' : L) (j : Nat), by'.closedAt 0 = true →
    (l.subst k by').free j = if j < k then l.free j else l.free (j + 1)
  | .fin, _, _, _, _ => by simp [L.subst, L.free]
  | .var n, k, by', j, hb => by
    simp only [L.subst]
    split
    · rename_i hnk
      subst hnk
      rw [(L.closed_shift by' 0 j hb (by omega)).1]
      split <;> simp [L.free] <;> omega
    · split
      · split <;> simp [L.free] <;> omega
      · split <;> simp [L.free] <;> omega
  | .mu body, k, by', j, hb => by
    simp only [L.subst, L.free, L.free_subst body (k + 1) by' (j + 1) hb]
    by_cases h : j < k
    · rw [if_pos h, if_pos (by omega)]
    · rw [if_neg h, if_neg (by omega)]
  | .send t ls, k, by', j, hb => by
    simp only [L.subst, L.free, Ls.free_subst ls k by' j hb]
  | .recv f ls, k, by', j, hb => by
    simp only [L.subst, L.free, Ls.free_subst ls k by' j hb]
theorem Ls.free_subst : ∀ (ls : Ls) (k : Nat) (by' : L) (j : Nat), by'.closedAt 0 = true →
    (ls.subst k by').free j = if j < k then ls.free j else ls.free (j + 1)
  | .nil, _, _, _, _ => by simp [Ls.subst, Ls.free]
  | .cons label l rest, k, by', j, hb => by
    simp only [Ls.subst, Ls.free, L.free_subst l k by' j hb, Ls.free_subst rest k by' j hb]
    split <;> rfl
end

mutual
/-- Removing an unused binder commutes with substitution above it. -/
theorem L.shift_subst : ∀ (l : L) (d k : Nat) (by' : L), d ≤ k → l.free d = false →
    by'.closedAt 0 = true → (l.subst (k + 1) by').shift d = (l.shift d).subst k by'
  | .fin, _, _, _, _, _, _ => rfl
  | .var n, d, k, by', hd, hf, hb => by
    simp only [L.free, beq_eq_false_iff_ne, ne_eq] at hf
    by_cases h1 : n = k + 1
    · subst h1
      simp only [L.subst, L.shift, if_true]
      rw [if_pos (by omega)]
      simp only [L.subst, Nat.add_sub_cancel, if_true]
      exact (L.closed_shift by' 0 d hb (by omega)).2
    · by_cases h2 : n > k + 1
      · simp only [L.subst, L.shift, if_neg h1, if_pos h2]
        rw [if_pos (by omega), if_pos (by omega)]
        simp only [L.subst]
        rw [if_neg (by omega), if_pos (by omega)]
      · simp only [L.subst, if_neg h1, if_neg h2]
        by_cases h3 : n > d
        · simp only [L.shift, if_pos h3, L.subst]
          rw [if_neg (by omega), if_neg (by omega)]
        · simp only [L.shift, if_neg h3, L.subst]
          rw [if_neg (by omega), if_neg (by omega)]
  | .mu body, d, k, by', hd, hf, hb => by
    simp only [L.free] at hf
    simp only [L.subst, L.shift, L.shift_subst body (d + 1) (k + 1) by' (by omega) hf hb]
  | .send t ls, d, k, by', hd, hf, hb => by
    simp only [L.free] at hf
    simp only [L.subst, L.shift, Ls.shift_subst ls d k by' hd hf hb]
  | .recv f ls, d, k, by', hd, hf, hb => by
    simp only [L.free] at hf
    simp only [L.subst, L.shift, Ls.shift_subst ls d k by' hd hf hb]
theorem Ls.shift_subst : ∀ (ls : Ls) (d k : Nat) (by' : L), d ≤ k → ls.free d = false →
    by'.closedAt 0 = true → (ls.subst (k + 1) by').shift d = (ls.shift d).subst k by'
  | .nil, _, _, _, _, _, _ => rfl
  | .cons label l rest, d, k, by', hd, hf, hb => by
    simp only [Ls.free, Bool.or_eq_false_iff] at hf
    simp only [Ls.subst, Ls.shift, L.shift_subst l d k by' hd hf.1 hb,
      Ls.shift_subst rest d k by' hd hf.2 hb]
end

/-- Substitution keeps an action. -/
theorem L.subst_action : ∀ (l : L) (k : Nat) (by' : L), l.actionless = false →
    (l.subst k by').actionless = false
  | .fin, _, _, h => by simp [L.actionless] at h
  | .var n, _, _, h => by simp [L.actionless] at h
  | .mu body, k, by', h => by
    simp only [L.actionless] at h
    simp only [L.subst, L.actionless]
    exact L.subst_action body (k + 1) by' h
  | .send t ls, _, _, _ => rfl
  | .recv f ls, _, _, _ => rfl

/-- The recursion rule commutes with the substitution of a closed normal
type for an outer variable. -/
theorem closeRec_subst {lb lby : L} (k : Nat) (hk : lb.knormal = true)
    (hc : lby.closedAt 0 = true) (hn : lby.knormal = true) :
    closeRec (lb.subst (k + 1) lby) = (closeRec lb).subst k lby := by
  by_cases ha : lb.actionless = true
  · rcases knormal_actionless hk ha with rfl | ⟨n, rfl⟩
    · rfl
    · by_cases h1 : n = k + 1
      · subst h1
        simp only [L.subst, if_true]
        have hcr : closeRec (L.var (k + 1)) = .var k := by simp [closeRec, L.actionless]
        rw [hcr]
        simp only [L.subst, if_true]
        unfold closeRec
        by_cases hla : lby.actionless = true
        · rw [if_pos hla]
          rcases knormal_actionless hn hla with rfl | ⟨v, rfl⟩
          · rfl
          · simp [L.closedAt] at hc
        · rw [if_neg hla]
          have := L.closed_shift lby 0 0 hc (Nat.le_refl _)
          rw [if_neg (by rw [this.1]; simp), this.2]
      · by_cases h2 : n > k + 1
        · simp only [L.subst, if_neg h1, if_pos h2]
          have e1 : closeRec (L.var (n - 1)) = .var (n - 1 - 1) := by
            simp [closeRec, L.actionless]; omega
          have e2 : closeRec (L.var n) = .var (n - 1) := by
            simp [closeRec, L.actionless]; omega
          rw [e1, e2]
          simp only [L.subst]
          rw [if_neg (by omega), if_pos (by omega)]
        · simp only [L.subst, if_neg h1, if_neg h2]
          by_cases h3 : n = 0
          · subst h3
            simp [closeRec, L.actionless, L.subst]
          · have e2 : closeRec (L.var n) = .var (n - 1) := by
              simp [closeRec, L.actionless]; omega
            rw [e2]
            simp only [L.subst]
            rw [if_neg (by omega), if_neg (by omega)]
  · have ha' : lb.actionless = false := by simpa using ha
    have hs := L.subst_action lb (k + 1) lby ha'
    have hfree := L.free_subst lb (k + 1) lby 0 hc
    rw [if_pos (by omega)] at hfree
    have e1 : closeRec (lb.subst (k + 1) lby)
        = if lb.free 0 = true then .mu (lb.subst (k + 1) lby) else (lb.subst (k + 1) lby).shift 0 := by
      unfold closeRec
      rw [if_neg (by rw [hs]; simp), hfree]
    have e2 : closeRec lb = if lb.free 0 = true then .mu lb else lb.shift 0 := by
      unfold closeRec
      rw [if_neg ha]
    rw [e1, e2]
    by_cases hf : lb.free 0 = true
    · rw [if_pos hf, if_pos hf]
      rfl
    · rw [if_neg hf, if_neg hf]
      exact L.shift_subst lb 0 k lby (Nat.zero_le _) (by simpa using hf) hc

end MP
