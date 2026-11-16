import Proto.Queue
/-!
# The steps of a runtime type

`SendStep s t l x2 x`: `x2` is `x` after role `s` sent `l` to `t`: at every
first pending node of `s` along every path of the head normal form, the
communication becomes a message in transit with the chosen branch.
`RecvStep q p l x2 x`: `x2` is `x` after role `q` received `l` from `p`:
at every first pending node of `q`, the message in transit is consumed.
Both are defined by recursion on the stepped type `x2` (the first
argument), as in the Verus text. Port of the section "the steps" and the
first lemmas of "the effects of a step" of
`crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

mutual
def SendStep (s t l : Nat) : R → R → Prop
  | .sent f2 d2 lab2 c2, x =>
    match x.head with
    | .comm frm dst bs => frm = s ∧ dst = t ∧ f2 = s ∧ d2 = t ∧ lab2 = l ∧ bs.lookup l = some c2
    | .sent frm dst label cont =>
      dst ≠ s ∧ f2 = frm ∧ d2 = dst ∧ lab2 = label ∧ SendStep s t l c2 cont
    | _ => False
  | .comm f2 d2 bs2, x =>
    match x.head with
    | .comm frm dst bs => frm ≠ s ∧ dst ≠ s ∧ f2 = frm ∧ d2 = dst ∧ SendStepBs s t l bs2 bs
    | _ => False
  | _, _ => False
def SendStepBs (s t l : Nat) : RBs → RBs → Prop
  | .nil, bs => bs = .nil
  | .cons label2 g2 rest2, bs =>
    match bs with
    | .cons label g rest => label = label2 ∧ SendStep s t l g2 g ∧ SendStepBs s t l rest2 rest
    | .nil => False
end

mutual
def RecvStep (q p l : Nat) : R → R → Prop
  | .sent f2 d2 lab2 c2, x =>
    match x.head with
    | .sent frm dst label cont =>
      if dst = q then frm = p ∧ label = l ∧ R.sent f2 d2 lab2 c2 = cont
      else f2 = frm ∧ d2 = dst ∧ lab2 = label ∧ RecvStep q p l c2 cont
    | _ => False
  | .comm f2 d2 bs2, x =>
    match x.head with
    | .sent frm dst label cont => dst = q ∧ frm = p ∧ label = l ∧ R.comm f2 d2 bs2 = cont
    | .comm frm dst bs => frm ≠ q ∧ dst ≠ q ∧ f2 = frm ∧ d2 = dst ∧ RecvStepBs q p l bs2 bs
    | _ => False
  | .fin, x =>
    match x.head with
    | .sent frm dst label cont => dst = q ∧ frm = p ∧ label = l ∧ R.fin = cont
    | _ => False
  | .mu b2, x =>
    match x.head with
    | .sent frm dst label cont => dst = q ∧ frm = p ∧ label = l ∧ R.mu b2 = cont
    | _ => False
  | .var v2, x =>
    match x.head with
    | .sent frm dst label cont => dst = q ∧ frm = p ∧ label = l ∧ R.var v2 = cont
    | _ => False
def RecvStepBs (q p l : Nat) : RBs → RBs → Prop
  | .nil, bs => bs = .nil
  | .cons label2 g2 rest2, bs =>
    match bs with
    | .cons label g rest => label = label2 ∧ RecvStep q p l g2 g ∧ RecvStepBs q p l rest2 rest
    | .nil => False
end

/-! Case analyses of a step by the head of the stepped-from type. -/

theorem sendStep_cases {s t l : Nat} {x2 x : R} (h : SendStep s t l x2 x) :
    (∃ bs b, x.head = .comm s t bs ∧ bs.lookup l = some b ∧ x2 = .sent s t l b) ∨
    (∃ frm dst bs bs2, x.head = .comm frm dst bs ∧ frm ≠ s ∧ dst ≠ s ∧ x2 = .comm frm dst bs2 ∧
      SendStepBs s t l bs2 bs) ∨
    (∃ frm dst label cont c2, x.head = .sent frm dst label cont ∧ dst ≠ s ∧
      x2 = .sent frm dst label c2 ∧ SendStep s t l c2 cont) := by
  cases x2 with
  | sent f2 d2 lab2 c2 =>
    simp only [SendStep] at h
    cases hx : x.head with
    | comm frm dst bs =>
      rw [hx] at h
      obtain ⟨rfl, rfl, rfl, rfl, rfl, hl⟩ := h
      exact Or.inl ⟨bs, c2, rfl, hl, rfl⟩
    | sent frm dst label cont =>
      rw [hx] at h
      obtain ⟨hd, rfl, rfl, rfl, hs⟩ := h
      exact Or.inr (Or.inr ⟨_, _, _, cont, c2, rfl, hd, rfl, hs⟩)
    | fin => rw [hx] at h; exact absurd h id
    | mu b => rw [hx] at h; exact absurd h id
    | var v => rw [hx] at h; exact absurd h id
  | comm f2 d2 bs2 =>
    simp only [SendStep] at h
    cases hx : x.head with
    | comm frm dst bs =>
      rw [hx] at h
      obtain ⟨h1, h2, rfl, rfl, hs⟩ := h
      exact Or.inr (Or.inl ⟨_, _, bs, bs2, rfl, h1, h2, rfl, hs⟩)
    | sent frm dst label cont => rw [hx] at h; exact absurd h id
    | fin => rw [hx] at h; exact absurd h id
    | mu b => rw [hx] at h; exact absurd h id
    | var v => rw [hx] at h; exact absurd h id
  | fin => simp [SendStep] at h
  | mu b => simp [SendStep] at h
  | var v => simp [SendStep] at h

theorem sendStepBs_cases {s t l : Nat} {bs2 bs : RBs} (h : SendStepBs s t l bs2 bs) :
    (bs2 = .nil ∧ bs = .nil) ∨
    (∃ label g g2 rest rest2, bs = .cons label g rest ∧ bs2 = .cons label g2 rest2 ∧
      SendStep s t l g2 g ∧ SendStepBs s t l rest2 rest) := by
  cases bs2 with
  | nil => simp only [SendStepBs] at h; exact Or.inl ⟨rfl, h⟩
  | cons label2 g2 rest2 =>
    cases bs with
    | nil => simp [SendStepBs] at h
    | cons label g rest =>
      simp only [SendStepBs] at h
      obtain ⟨rfl, h1, h2⟩ := h
      exact Or.inr ⟨_, _, _, _, _, rfl, rfl, h1, h2⟩

theorem recvStep_cases {q p l : Nat} {x2 x : R} (h : RecvStep q p l x2 x) :
    (∃ cont, x.head = .sent p q l cont ∧ x2 = cont) ∨
    (∃ frm dst label cont c2, x.head = .sent frm dst label cont ∧ dst ≠ q ∧
      x2 = .sent frm dst label c2 ∧ RecvStep q p l c2 cont) ∨
    (∃ frm dst bs bs2, x.head = .comm frm dst bs ∧ frm ≠ q ∧ dst ≠ q ∧ x2 = .comm frm dst bs2 ∧
      RecvStepBs q p l bs2 bs) := by
  cases x2 with
  | sent f2 d2 lab2 c2 =>
    simp only [RecvStep] at h
    cases hx : x.head with
    | sent frm dst label cont =>
      rw [hx] at h
      dsimp only at h
      by_cases hd : dst = q
      · rw [if_pos hd] at h
        obtain ⟨rfl, rfl, hc⟩ := h
        subst hd
        exact Or.inl ⟨cont, rfl, hc⟩
      · rw [if_neg hd] at h
        obtain ⟨rfl, rfl, rfl, hs⟩ := h
        exact Or.inr (Or.inl ⟨_, _, _, cont, c2, rfl, hd, rfl, hs⟩)
    | comm frm dst bs => rw [hx] at h; exact absurd h id
    | fin => rw [hx] at h; exact absurd h id
    | mu b => rw [hx] at h; exact absurd h id
    | var v => rw [hx] at h; exact absurd h id
  | comm f2 d2 bs2 =>
    simp only [RecvStep] at h
    cases hx : x.head with
    | sent frm dst label cont =>
      rw [hx] at h
      obtain ⟨rfl, rfl, rfl, hc⟩ := h
      exact Or.inl ⟨cont, rfl, hc⟩
    | comm frm dst bs =>
      rw [hx] at h
      obtain ⟨h1, h2, rfl, rfl, hs⟩ := h
      exact Or.inr (Or.inr ⟨_, _, bs, bs2, rfl, h1, h2, rfl, hs⟩)
    | fin => rw [hx] at h; exact absurd h id
    | mu b => rw [hx] at h; exact absurd h id
    | var v => rw [hx] at h; exact absurd h id
  | fin =>
    simp only [RecvStep] at h
    cases hx : x.head with
    | sent frm dst label cont =>
      rw [hx] at h
      obtain ⟨rfl, rfl, rfl, hc⟩ := h
      exact Or.inl ⟨cont, rfl, hc⟩
    | comm frm dst bs => rw [hx] at h; exact absurd h id
    | fin => rw [hx] at h; exact absurd h id
    | mu b => rw [hx] at h; exact absurd h id
    | var v => rw [hx] at h; exact absurd h id
  | mu b2 =>
    simp only [RecvStep] at h
    cases hx : x.head with
    | sent frm dst label cont =>
      rw [hx] at h
      obtain ⟨rfl, rfl, rfl, hc⟩ := h
      exact Or.inl ⟨cont, rfl, hc⟩
    | comm frm dst bs => rw [hx] at h; exact absurd h id
    | fin => rw [hx] at h; exact absurd h id
    | mu b => rw [hx] at h; exact absurd h id
    | var v => rw [hx] at h; exact absurd h id
  | var v2 =>
    simp only [RecvStep] at h
    cases hx : x.head with
    | sent frm dst label cont =>
      rw [hx] at h
      obtain ⟨rfl, rfl, rfl, hc⟩ := h
      exact Or.inl ⟨cont, rfl, hc⟩
    | comm frm dst bs => rw [hx] at h; exact absurd h id
    | fin => rw [hx] at h; exact absurd h id
    | mu b => rw [hx] at h; exact absurd h id
    | var v => rw [hx] at h; exact absurd h id

theorem recvStepBs_cases {q p l : Nat} {bs2 bs : RBs} (h : RecvStepBs q p l bs2 bs) :
    (bs2 = .nil ∧ bs = .nil) ∨
    (∃ label g g2 rest rest2, bs = .cons label g rest ∧ bs2 = .cons label g2 rest2 ∧
      RecvStep q p l g2 g ∧ RecvStepBs q p l rest2 rest) := by
  cases bs2 with
  | nil => simp only [RecvStepBs] at h; exact Or.inl ⟨rfl, h⟩
  | cons label2 g2 rest2 =>
    cases bs with
    | nil => simp [RecvStepBs] at h
    | cons label g rest =>
      simp only [RecvStepBs] at h
      obtain ⟨rfl, h1, h2⟩ := h
      exact Or.inr ⟨_, _, _, _, _, rfl, rfl, h1, h2⟩

/-- The stepped branch list has the labels of the original, each branch
stepped. -/
theorem sendStepBs_labels {s t l : Nat} : ∀ (bs2 bs : RBs), SendStepBs s t l bs2 bs →
    bs2.len = bs.len ∧ bs2.distinct = bs.distinct ∧
    (∀ label, (bs2.lookup label).isSome = (bs.lookup label).isSome) ∧
    (∀ label b, bs.lookup label = some b → ∃ b2, bs2.lookup label = some b2 ∧ SendStep s t l b2 b)
  | .nil, bs, h => by
    simp only [SendStepBs] at h
    subst h
    exact ⟨rfl, rfl, fun _ => rfl, fun label b hb => by simp [RBs.lookup] at hb⟩
  | .cons label2 g2 rest2, bs, h => by
    cases bs with
    | nil => simp [SendStepBs] at h
    | cons label g rest =>
      simp only [SendStepBs] at h
      obtain ⟨rfl, h1, h2⟩ := h
      obtain ⟨i1, i2, i3, i4⟩ := sendStepBs_labels rest2 rest h2
      refine ⟨by simp [RBs.len, i1], ?_, ?_, ?_⟩
      · simp only [RBs.distinct, i2]
        have := i3 label
        cases h1' : rest2.lookup label <;> cases h2' : rest.lookup label <;> simp_all
      · intro lab
        simp only [RBs.lookup]
        split
        · rfl
        · exact i3 lab
      · intro lab b hb
        simp only [RBs.lookup] at hb ⊢
        split at hb
        · cases hb; rename_i e; rw [if_pos e]; exact ⟨g2, rfl, h1⟩
        · rename_i e; rw [if_neg e]; exact i4 lab b hb

theorem recvStepBs_labels {q p l : Nat} : ∀ (bs2 bs : RBs), RecvStepBs q p l bs2 bs →
    bs2.len = bs.len ∧ bs2.distinct = bs.distinct ∧
    (∀ label, (bs2.lookup label).isSome = (bs.lookup label).isSome) ∧
    (∀ label b, bs.lookup label = some b → ∃ b2, bs2.lookup label = some b2 ∧ RecvStep q p l b2 b)
  | .nil, bs, h => by
    simp only [RecvStepBs] at h
    subst h
    exact ⟨rfl, rfl, fun _ => rfl, fun label b hb => by simp [RBs.lookup] at hb⟩
  | .cons label2 g2 rest2, bs, h => by
    cases bs with
    | nil => simp [RecvStepBs] at h
    | cons label g rest =>
      simp only [RecvStepBs] at h
      obtain ⟨rfl, h1, h2⟩ := h
      obtain ⟨i1, i2, i3, i4⟩ := recvStepBs_labels rest2 rest h2
      refine ⟨by simp [RBs.len, i1], ?_, ?_, ?_⟩
      · simp only [RBs.distinct, i2]
        have := i3 label
        cases h1' : rest2.lookup label <;> cases h2' : rest.lookup label <;> simp_all
      · intro lab
        simp only [RBs.lookup]
        split
        · rfl
        · exact i3 lab
      · intro lab b hb
        simp only [RBs.lookup] at hb ⊢
        split at hb
        · cases hb; rename_i e; rw [if_pos e]; exact ⟨g2, rfl, h1⟩
        · rename_i e; rw [if_neg e]; exact i4 lab b hb

theorem R.wf_comm_mk {frm dst : Nat} {bs : RBs} {n : Nat} (h1 : frm < n) (h2 : dst < n)
    (h3 : frm ≠ dst) (h4 : 1 ≤ bs.len) (h5 : bs.distinct = true) (h6 : bs.wfBs n [] = true) :
    (R.comm frm dst bs).wf n = true := by
  simp [R.wf, R.wfAt, allTrue_nil, h1, h2, h3, h4, h5, h6]

theorem R.wf_sent_mk {frm dst label : Nat} {cont : R} {n : Nat} (h1 : frm < n) (h2 : dst < n)
    (h3 : frm ≠ dst) (h4 : cont.wf n = true) : (R.sent frm dst label cont).wf n = true := by
  simp only [R.wf] at h4
  simp [R.wf, R.wfAt, allTrue_nil, h1, h2, h3, h4]

mutual
/-- A send keeps well-formedness and yields a head-normal type. -/
theorem sendStep_wf {s t l n : Nat} : ∀ (x2 x : R), x.wf n = true → SendStep s t l x2 x →
    x2.wf n = true ∧ x2.head = x2
  | .sent f2 d2 lab2 c2, x, hw, h => by
    have hhw := (R.head_wf hw).1
    rcases sendStep_cases h with ⟨bs, b, hx, hl, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨frm, dst, label, cont, c2', hx, hd, e, hs⟩
    · cases e
      rw [hx] at hhw
      have hp := R.comm_wf_parts hhw
      exact ⟨R.wf_sent_mk hp.1 hp.2.1 hp.2.2.1 (RBs.branch_wf bs n l _ hp.2.2.2.2.2 hl), rfl⟩
    · cases e
    · cases e
      rw [hx] at hhw
      have hp := R.sent_wf_parts hhw
      exact ⟨R.wf_sent_mk hp.1 hp.2.1 hp.2.2.1 (sendStep_wf c2 cont hp.2.2.2 hs).1, rfl⟩
  | .comm f2 d2 bs2, x, hw, h => by
    have hhw := (R.head_wf hw).1
    rcases sendStep_cases h with ⟨_, _, _, _, e⟩ | ⟨frm, dst, bs, bs2', hx, h1, h2, e, hs⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · cases e
    · cases e
      rw [hx] at hhw
      have hp := R.comm_wf_parts hhw
      have hl := sendStepBs_labels bs2 bs hs
      exact ⟨R.wf_comm_mk hp.1 hp.2.1 hp.2.2.1 (by rw [hl.1]; exact hp.2.2.2.1)
        (by rw [hl.2.1]; exact hp.2.2.2.2.1) (sendStepBs_wf bs2 bs hp.2.2.2.2.2 hs), rfl⟩
    · cases e
  | .fin, _, _, h => by simp [SendStep] at h
  | .mu _, _, _, h => by simp [SendStep] at h
  | .var _, _, _, h => by simp [SendStep] at h
theorem sendStepBs_wf {s t l n : Nat} : ∀ (bs2 bs : RBs), bs.wfBs n [] = true →
    SendStepBs s t l bs2 bs → bs2.wfBs n [] = true
  | .nil, _, _, _ => rfl
  | .cons label2 g2 rest2, bs, hw, h => by
    cases bs with
    | nil => simp [SendStepBs] at h
    | cons label g rest =>
      simp only [SendStepBs] at h
      simp only [RBs.wfBs, Bool.and_eq_true] at hw ⊢
      have := sendStep_wf g2 g hw.1 h.2.1
      exact ⟨this.1, sendStepBs_wf rest2 rest hw.2 h.2.2⟩
end

mutual
/-- A receive keeps well-formedness. -/
theorem recvStep_wf {q p l n : Nat} : ∀ (x2 x : R), x.wf n = true → RecvStep q p l x2 x →
    x2.wf n = true
  | .sent f2 d2 lab2 c2, x, hw, h => by
    have hhw := (R.head_wf hw).1
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨frm, dst, label, cont, c2', hx, hd, e, hs⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · rw [hx] at hhw
      rw [e]
      exact (R.sent_wf_parts hhw).2.2.2
    · cases e
      rw [hx] at hhw
      have hp := R.sent_wf_parts hhw
      exact R.wf_sent_mk hp.1 hp.2.1 hp.2.2.1 (recvStep_wf c2 cont hp.2.2.2 hs)
    · cases e
  | .comm f2 d2 bs2, x, hw, h => by
    have hhw := (R.head_wf hw).1
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨frm, dst, bs, bs2', hx, h1, h2, e, hs⟩
    · rw [hx] at hhw
      rw [e]
      exact (R.sent_wf_parts hhw).2.2.2
    · cases e
    · cases e
      rw [hx] at hhw
      have hp := R.comm_wf_parts hhw
      have hl := recvStepBs_labels bs2 bs hs
      exact R.wf_comm_mk hp.1 hp.2.1 hp.2.2.1 (by rw [hl.1]; exact hp.2.2.2.1)
        (by rw [hl.2.1]; exact hp.2.2.2.2.1) (recvStepBs_wf bs2 bs hp.2.2.2.2.2 hs)
  | .fin, x, hw, h => by
    have hhw := (R.head_wf hw).1
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · rfl
    · cases e
    · cases e
  | .mu b2, x, hw, h => by
    have hhw := (R.head_wf hw).1
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · rw [hx] at hhw
      rw [e]
      exact (R.sent_wf_parts hhw).2.2.2
    · cases e
    · cases e
  | .var v2, x, hw, h => by
    have hhw := (R.head_wf hw).1
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · rw [hx] at hhw
      rw [e]
      exact (R.sent_wf_parts hhw).2.2.2
    · cases e
    · cases e
theorem recvStepBs_wf {q p l n : Nat} : ∀ (bs2 bs : RBs), bs.wfBs n [] = true →
    RecvStepBs q p l bs2 bs → bs2.wfBs n [] = true
  | .nil, _, _, _ => rfl
  | .cons label2 g2 rest2, bs, hw, h => by
    cases bs with
    | nil => simp [RecvStepBs] at h
    | cons label g rest =>
      simp only [RecvStepBs] at h
      simp only [RBs.wfBs, Bool.and_eq_true] at hw ⊢
      exact ⟨recvStep_wf g2 g hw.1 h.2.1, recvStepBs_wf rest2 rest hw.2 h.2.2⟩
end

end MP
