import Proto.Rel
/-!
# Queues, the structure of a runtime type, paths and depths

Port of the sections "queues and the structure" and "paths and depths" of
`crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

mutual
/-- The labels in transit from `p` to `q`, along the first branches. -/
def R.queue : R → Nat → Nat → List Nat
  | .fin, _, _ => []
  | .var _, _, _ => []
  | .mu body, p, q => body.queue p q
  | .comm _ _ bs, p, q => bs.queue p q
  | .sent frm dst label cont, p, q =>
    (if frm = p ∧ dst = q then [label] else []) ++ cont.queue p q
def RBs.queue : RBs → Nat → Nat → List Nat
  | .nil, _, _ => []
  | .cons _ g _, p, q => g.queue p q
end

mutual
def R.noSent : R → Bool
  | .mu body => body.noSent
  | .comm _ _ bs => bs.noSent
  | .sent .. => false
  | _ => true
def RBs.noSent : RBs → Bool
  | .nil => true
  | .cons _ g rest => g.noSent && rest.noSent
end

mutual
/-- No message in transit from role `s`. -/
def R.noSentFrom : R → Nat → Bool
  | .mu body, s => body.noSentFrom s
  | .comm _ _ bs, s => bs.noSentFrom s
  | .sent frm _ _ cont, s => decide (frm ≠ s) && cont.noSentFrom s
  | _, _ => true
def RBs.noSentFrom : RBs → Nat → Bool
  | .nil, _ => true
  | .cons _ g rest, s => g.noSentFrom s && rest.noSentFrom s
end

/-- Every branch has the queues of the first. -/
def QueuesAgree (bs : RBs) : Prop :=
  ∀ label g p q, bs.lookup label = some g → g.queue p q = bs.queue p q

mutual
/-- The structure of a runtime type beyond well-formedness: no message in
transit inside a recursion (a step unfolds every recursion above it), none
from the sender of a pending communication below it (a role's sends are in
order), and the branches of every choice agreeing on their queues (a step
rewrites every branch alike). -/
def R.good : R → Prop
  | .mu body => body.noSent = true
  | .comm frm _ bs => bs.noSentFrom frm = true ∧ QueuesAgree bs ∧ bs.good
  | .sent _ _ _ cont => cont.good
  | _ => True
def RBs.good : RBs → Prop
  | .nil => True
  | .cons _ g rest => g.good ∧ rest.good
end

mutual
theorem R.noSent_queue : ∀ (x : R) (p q : Nat), x.noSent = true → x.queue p q = []
  | .fin, _, _, _ => rfl
  | .var _, _, _, _ => rfl
  | .mu body, p, q, h => by
    simp only [R.noSent] at h
    simp only [R.queue, R.noSent_queue body p q h]
  | .comm _ _ bs, p, q, h => by
    simp only [R.noSent] at h
    simp only [R.queue, RBs.noSent_queue bs p q h]
  | .sent .., _, _, h => by simp [R.noSent] at h
theorem RBs.noSent_queue : ∀ (bs : RBs) (p q : Nat), bs.noSent = true → bs.queue p q = []
  | .nil, _, _, _ => rfl
  | .cons _ g rest, p, q, h => by
    simp only [RBs.noSent, Bool.and_eq_true] at h
    simp only [RBs.queue, R.noSent_queue g p q h.1]
end

mutual
theorem R.noSentFrom_queue : ∀ (x : R) (s q : Nat), x.noSentFrom s = true → x.queue s q = []
  | .fin, _, _, _ => rfl
  | .var _, _, _, _ => rfl
  | .mu body, s, q, h => by
    simp only [R.noSentFrom] at h
    simp only [R.queue, R.noSentFrom_queue body s q h]
  | .comm _ _ bs, s, q, h => by
    simp only [R.noSentFrom] at h
    simp only [R.queue, RBs.noSentFrom_queue bs s q h]
  | .sent frm dst label cont, s, q, h => by
    simp only [R.noSentFrom, Bool.and_eq_true, decide_eq_true_eq] at h
    simp only [R.queue, R.noSentFrom_queue cont s q h.2]
    rw [if_neg (fun e => h.1 e.1)]
    rfl
theorem RBs.noSentFrom_queue : ∀ (bs : RBs) (s q : Nat), bs.noSentFrom s = true →
    bs.queue s q = []
  | .nil, _, _, _ => rfl
  | .cons _ g rest, s, q, h => by
    simp only [RBs.noSentFrom, Bool.and_eq_true] at h
    simp only [RBs.queue, R.noSentFrom_queue g s q h.1]
end

theorem RBs.noSent_lookup : ∀ (bs : RBs) {label : Nat} {g : R}, bs.noSent = true →
    bs.lookup label = some g → g.noSent = true
  | .nil, _, _, _, h => by simp [RBs.lookup] at h
  | .cons l0 g0 rest, label, g, hn, h => by
    simp only [RBs.noSent, Bool.and_eq_true] at hn
    simp only [RBs.lookup] at h
    split at h
    · cases h; exact hn.1
    · exact RBs.noSent_lookup rest hn.2 h

mutual
theorem R.noSent_from_any : ∀ (x : R) (s : Nat), x.noSent = true → x.noSentFrom s = true
  | .fin, _, _ => rfl
  | .var _, _, _ => rfl
  | .mu body, s, h => by
    simp only [R.noSent] at h
    simp only [R.noSentFrom, R.noSent_from_any body s h]
  | .comm _ _ bs, s, h => by
    simp only [R.noSent] at h
    simp only [R.noSentFrom, RBs.noSent_from_any bs s h]
  | .sent .., _, h => by simp [R.noSent] at h
theorem RBs.noSent_from_any : ∀ (bs : RBs) (s : Nat), bs.noSent = true → bs.noSentFrom s = true
  | .nil, _, _ => rfl
  | .cons _ g rest, s, h => by
    simp only [RBs.noSent, Bool.and_eq_true] at h
    simp only [RBs.noSentFrom, R.noSent_from_any g s h.1, RBs.noSent_from_any rest s h.2]
    rfl
end

mutual
theorem R.noSent_good : ∀ (x : R), x.noSent = true → x.good
  | .fin, _ => trivial
  | .var _, _ => trivial
  | .mu body, h => by
    simp only [R.noSent] at h
    simpa [R.good] using h
  | .comm frm dst bs, h => by
    simp only [R.noSent] at h
    simp only [R.good]
    refine ⟨RBs.noSent_from_any bs frm h, ?_, RBs.noSent_good bs h⟩
    intro label g p q hg
    rw [R.noSent_queue g p q (RBs.noSent_lookup bs h hg), RBs.noSent_queue bs p q h]
  | .sent .., h => by simp [R.noSent] at h
theorem RBs.noSent_good : ∀ (bs : RBs), bs.noSent = true → bs.good
  | .nil, _ => trivial
  | .cons _ g rest, h => by
    simp only [RBs.noSent, Bool.and_eq_true] at h
    exact ⟨R.noSent_good g h.1, RBs.noSent_good rest h.2⟩
end

mutual
theorem R.noSent_subst : ∀ (x : R) (k : Nat) (by' : R), x.noSent = true → by'.noSent = true →
    (x.subst k by').noSent = true
  | .fin, _, _, _, _ => rfl
  | .var v, k, by', _, hb => by
    simp only [R.subst]
    split
    · exact hb
    · split <;> rfl
  | .mu body, k, by', h, hb => by
    simp only [R.noSent] at h
    simp only [R.subst, R.noSent]
    exact R.noSent_subst body (k + 1) by' h hb
  | .comm _ _ bs, k, by', h, hb => by
    simp only [R.noSent] at h
    simp only [R.subst, R.noSent]
    exact RBs.noSent_subst bs k by' h hb
  | .sent .., _, _, h, _ => by simp [R.noSent] at h
theorem RBs.noSent_subst : ∀ (bs : RBs) (k : Nat) (by' : R), bs.noSent = true →
    by'.noSent = true → (bs.subst k by').noSent = true
  | .nil, _, _, _, _ => rfl
  | .cons _ g rest, k, by', h, hb => by
    simp only [RBs.noSent, Bool.and_eq_true] at h
    simp only [RBs.subst, RBs.noSent, Bool.and_eq_true]
    exact ⟨R.noSent_subst g k by' h.1 hb, RBs.noSent_subst rest k by' h.2 hb⟩
end

/-- Head normalization keeps the structure and the queues. -/
theorem R.head_good : ∀ (k : Nat) (x : R) (n : Nat), x.leadRecs = k → x.good → x.wf n = true →
    x.head.good ∧ ∀ p q, x.head.queue p q = x.queue p q
  | 0, x, n, hk, hg, _ => by
    have : x.head = x := by
      apply R.head_flat
      intro b e; subst e; simp [R.leadRecs] at hk
    rw [this]
    exact ⟨hg, fun _ _ => rfl⟩
  | k + 1, x, n, hk, hg, hw => by
    cases x with
    | mu body =>
      have hns : body.noSent = true := by simpa [R.good] using hg
      have hu := R.unfold_wf hw
      have hkk : (R.mu body).unfold.leadRecs = k := by have := hu.2; omega
      have hsub : (R.mu body).unfold.noSent = true := by
        simp only [R.unfold]
        exact R.noSent_subst body 0 (.mu body) hns (by simpa [R.noSent] using hns)
      have ih := R.head_good k _ n hkk (R.noSent_good _ hsub) hu.1
      have hhead : (R.mu body).head = (R.mu body).unfold.head := by
        rw [R.head_mu]
        simp only [R.head]
        have : body.leadRecs = k := by simp [R.leadRecs] at hk; omega
        rw [this, hkk]
      rw [hhead]
      refine ⟨ih.1, fun p q => ?_⟩
      rw [ih.2 p q, R.noSent_queue _ p q hsub]
      simp only [R.queue, R.noSent_queue body p q hns]
    | fin => simp [R.leadRecs] at hk
    | var v => simp [R.leadRecs] at hk
    | comm f t bs => simp [R.leadRecs] at hk
    | sent f t lab c => simp [R.leadRecs] at hk

/-- A branch of a good communication is good. -/
theorem RBs.good_branch : ∀ (bs : RBs) {label : Nat} {g : R}, bs.good →
    bs.lookup label = some g → g.good
  | .nil, _, _, _, h => by simp [RBs.lookup] at h
  | .cons l0 g0 rest, label, g, hg, h => by
    simp only [RBs.good] at hg
    simp only [RBs.lookup] at h
    split at h
    · cases h; exact hg.1
    · exact RBs.good_branch rest hg.2 h

theorem RBs.noSentFrom_branch : ∀ (bs : RBs) (s : Nat) {label : Nat} {g : R},
    bs.noSentFrom s = true → bs.lookup label = some g → g.noSentFrom s = true
  | .nil, _, _, _, _, h => by simp [RBs.lookup] at h
  | .cons l0 g0 rest, s, label, g, hn, h => by
    simp only [RBs.noSentFrom, Bool.and_eq_true] at hn
    simp only [RBs.lookup] at h
    split at h
    · cases h; exact hn.1
    · exact RBs.noSentFrom_branch rest s hn.2 h

/-! ## Paths and depths -/

mutual
/-- The first path (first branches) meets a node pending for `u`. -/
def R.firstHits : R → Nat → Bool
  | .fin, _ => false
  | .var _, _ => false
  | .mu body, u => body.firstHits u
  | .comm frm dst bs, u => decide (frm = u) || decide (dst = u) || bs.firstHits u
  | .sent _ dst _ cont, u => decide (dst = u) || cont.firstHits u
def RBs.firstHits : RBs → Nat → Bool
  | .nil, _ => false
  | .cons _ g _, u => g.firstHits u
end

mutual
/-- The depth of the first pending node of `u` along the first path. -/
def R.fdepth : R → Nat → Nat
  | .fin, _ => 0
  | .var _, _ => 0
  | .mu body, u => 1 + body.fdepth u
  | .comm frm dst bs, u => if frm = u ∨ dst = u then 0 else 1 + bs.fdepth u
  | .sent _ dst _ cont, u => if dst = u then 0 else 1 + cont.fdepth u
def RBs.fdepth : RBs → Nat → Nat
  | .nil, _ => 0
  | .cons _ g _, u => g.fdepth u
end

mutual
/-- The greatest depth of a first pending node of `u` over every path. -/
def R.sdepth : R → Nat → Nat
  | .fin, _ => 0
  | .var _, _ => 0
  | .mu body, u => 1 + body.sdepth u
  | .comm frm dst bs, u => if frm = u ∨ dst = u then 0 else 1 + bs.sdepth u
  | .sent _ dst _ cont, u => if dst = u then 0 else 1 + cont.sdepth u
def RBs.sdepth : RBs → Nat → Nat
  | .nil, _ => 0
  | .cons _ g rest, u => max (g.sdepth u) (rest.sdepth u)
end

theorem RBs.sdepth_branch : ∀ (bs : RBs) (u : Nat) {label : Nat} {g : R},
    bs.lookup label = some g → g.sdepth u ≤ bs.sdepth u
  | .nil, _, _, _, h => by simp [RBs.lookup] at h
  | .cons l0 g0 rest, u, label, g, h => by
    simp only [RBs.lookup] at h
    simp only [RBs.sdepth]
    split at h
    · cases h; exact Nat.le_max_left _ _
    · exact Nat.le_trans (RBs.sdepth_branch rest u h) (Nat.le_max_right _ _)

theorem RBs.reaches_cons {label : Nat} {g : R} {rest : RBs} {u : Nat}
    (h : (RBs.cons label g rest).reaches u = true) :
    g.reaches u = true ∧ (rest = .nil ∨ rest.reaches u = true) := by
  cases rest with
  | nil => exact ⟨by simpa [RBs.reaches] using h, Or.inl rfl⟩
  | cons l2 g2 r2 =>
    simp only [RBs.reaches, Bool.and_eq_true] at h
    exact ⟨h.1, Or.inr h.2⟩

theorem RBs.reaches_cons_mk {label : Nat} {g : R} {rest : RBs} {u : Nat}
    (h1 : g.reaches u = true) (h2 : rest = .nil ∨ rest.reaches u = true) :
    (RBs.cons label g rest).reaches u = true := by
  cases rest with
  | nil => simpa [RBs.reaches] using h1
  | cons l2 g2 r2 =>
    rcases h2 with h2 | h2
    · cases h2
    · simp only [RBs.reaches, Bool.and_eq_true]
      exact ⟨h1, h2⟩

mutual
theorem R.reaches_first : ∀ (x : R) (u : Nat), x.reaches u = true → x.firstHits u = true
  | .fin, _, h => by simp [R.reaches] at h
  | .var _, _, h => by simp [R.reaches] at h
  | .mu body, u, h => by
    simp only [R.reaches] at h
    simp only [R.firstHits, R.reaches_first body u h]
  | .comm frm dst bs, u, h => by
    simp only [R.reaches, Bool.or_eq_true] at h
    simp only [R.firstHits, Bool.or_eq_true]
    rcases h with h | h
    · exact Or.inl h
    · exact Or.inr (RBs.reaches_first bs u h)
  | .sent frm dst label cont, u, h => by
    simp only [R.reaches, Bool.or_eq_true] at h
    simp only [R.firstHits, Bool.or_eq_true]
    rcases h with h | h
    · exact Or.inl h
    · exact Or.inr (R.reaches_first cont u h)
theorem RBs.reaches_first : ∀ (bs : RBs) (u : Nat), bs.reaches u = true → bs.firstHits u = true
  | .nil, _, h => by simp [RBs.reaches] at h
  | .cons _ g rest, u, h => by
    simp only [RBs.firstHits]
    exact R.reaches_first g u (RBs.reaches_cons h).1
end

theorem RBs.reaches_branch : ∀ (bs : RBs) (u : Nat) {label : Nat} {g : R},
    bs.reaches u = true → bs.lookup label = some g → g.reaches u = true
  | .nil, _, _, _, _, h => by simp [RBs.lookup] at h
  | .cons l0 g0 rest, u, label, g, hr, h => by
    have hc := RBs.reaches_cons hr
    simp only [RBs.lookup] at h
    split at h
    · cases h; exact hc.1
    · rcases hc.2 with rfl | h2
      · simp [RBs.lookup] at h
      · exact RBs.reaches_branch rest u h2 h

mutual
/-- Substitution below the pending nodes changes neither the reaching nor
the depths. -/
theorem R.reaches_subst : ∀ (x : R) (u k : Nat) (by' : R), x.reaches u = true →
    (x.subst k by').reaches u = true ∧ (x.subst k by').sdepth u = x.sdepth u ∧
      (x.subst k by').fdepth u = x.fdepth u
  | .fin, _, _, _, h => by simp [R.reaches] at h
  | .var _, _, _, _, h => by simp [R.reaches] at h
  | .mu body, u, k, by', h => by
    simp only [R.reaches] at h
    obtain ⟨a, b, c⟩ := R.reaches_subst body u (k + 1) by' h
    refine ⟨?_, ?_, ?_⟩
    · simpa [R.subst, R.reaches] using a
    · simp [R.subst, R.sdepth, b]
    · simp [R.subst, R.fdepth, c]
  | .comm frm dst bs, u, k, by', h => by
    by_cases hu : frm = u ∨ dst = u
    · refine ⟨?_, ?_, ?_⟩
      · rcases hu with e | e <;> simp [R.subst, R.reaches, e]
      · simp [R.subst, R.sdepth, hu]
      · simp [R.subst, R.fdepth, hu]
    · simp only [R.reaches, Bool.or_eq_true, decide_eq_true_eq] at h
      have hb : bs.reaches u = true := by
        rcases h with (h | h) | h
        · exact absurd (Or.inl h) hu
        · exact absurd (Or.inr h) hu
        · exact h
      obtain ⟨a, b, c⟩ := RBs.reaches_subst bs u k by' hb
      refine ⟨?_, ?_, ?_⟩
      · simp [R.subst, R.reaches, a]
      · simp [R.subst, R.sdepth, hu, b]
      · simp [R.subst, R.fdepth, hu, c]
  | .sent frm dst label cont, u, k, by', h => by
    by_cases hu : dst = u
    · refine ⟨?_, ?_, ?_⟩
      · simp [R.subst, R.reaches, hu]
      · simp [R.subst, R.sdepth, hu]
      · simp [R.subst, R.fdepth, hu]
    · simp only [R.reaches, Bool.or_eq_true, decide_eq_true_eq] at h
      have hc : cont.reaches u = true := by
        rcases h with h | h
        · exact absurd h hu
        · exact h
      obtain ⟨a, b, c⟩ := R.reaches_subst cont u k by' hc
      refine ⟨?_, ?_, ?_⟩
      · simp [R.subst, R.reaches, a]
      · simp [R.subst, R.sdepth, hu, b]
      · simp [R.subst, R.fdepth, hu, c]
theorem RBs.reaches_subst : ∀ (bs : RBs) (u k : Nat) (by' : R), bs.reaches u = true →
    (bs.subst k by').reaches u = true ∧ (bs.subst k by').sdepth u = bs.sdepth u ∧
      (bs.subst k by').fdepth u = bs.fdepth u
  | .nil, _, _, _, h => by simp [RBs.reaches] at h
  | .cons label g .nil, u, k, by', h => by
    have hg : g.reaches u = true := by simpa [RBs.reaches] using h
    obtain ⟨a, b, c⟩ := R.reaches_subst g u k by' hg
    refine ⟨?_, ?_, ?_⟩
    · simpa [RBs.subst, RBs.reaches] using a
    · simp [RBs.subst, RBs.sdepth, b]
    · simp [RBs.subst, RBs.fdepth, c]
  | .cons label g (.cons l2 g2 r2), u, k, by', h => by
    simp only [RBs.reaches, Bool.and_eq_true] at h
    obtain ⟨a1, b1, c1⟩ := R.reaches_subst g u k by' h.1
    obtain ⟨a2, b2, c2⟩ := RBs.reaches_subst (.cons l2 g2 r2) u k by' h.2
    simp only [RBs.subst] at a2 b2
    refine ⟨?_, ?_, ?_⟩
    · simp only [RBs.subst, RBs.reaches, Bool.and_eq_true]
      exact ⟨a1, a2⟩
    · simp only [RBs.subst, RBs.sdepth] at b2 ⊢
      rw [b1, b2]
    · simp [RBs.subst, RBs.fdepth, c1]
end

/-- Head normalization keeps the reaching and does not deepen the nodes. -/
theorem R.head_reaches : ∀ (k : Nat) (x : R) (n u : Nat), x.leadRecs = k → x.wf n = true →
    x.reaches u = true →
    x.head.reaches u = true ∧ x.head.sdepth u ≤ x.sdepth u ∧ x.head.fdepth u ≤ x.fdepth u
  | 0, x, n, u, hk, _, hr => by
    have : x.head = x := by
      apply R.head_flat
      intro b e; subst e; simp [R.leadRecs] at hk
    rw [this]
    exact ⟨hr, Nat.le_refl _, Nat.le_refl _⟩
  | k + 1, x, n, u, hk, hw, hr => by
    cases x with
    | mu body =>
      have hu := R.unfold_wf hw
      have hkk : (R.mu body).unfold.leadRecs = k := by have := hu.2; omega
      have hb : body.reaches u = true := by simpa [R.reaches] using hr
      have hs := R.reaches_subst body u 0 (.mu body) hb
      have ih := R.head_reaches k _ n u hkk hu.1 (by simpa [R.unfold] using hs.1)
      have hhead : (R.mu body).head = (R.mu body).unfold.head := by
        rw [R.head_mu]
        simp only [R.head]
        have : body.leadRecs = k := by simp [R.leadRecs] at hk; omega
        rw [this, hkk]
      rw [hhead]
      simp only [R.unfold] at ih
      rw [hs.2.1, hs.2.2] at ih
      simp only [R.unfold, R.sdepth, R.fdepth]
      exact ⟨ih.1, by omega, by omega⟩
    | fin => simp [R.leadRecs] at hk
    | var v => simp [R.leadRecs] at hk
    | comm f t bs => simp [R.leadRecs] at hk
    | sent f t lab c => simp [R.leadRecs] at hk

end MP
