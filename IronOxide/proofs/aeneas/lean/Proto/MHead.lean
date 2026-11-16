import Proto.ProjNormal
/-!
# The canonical head; actions and reaching

Port of the sections "the canonical head" and "actions and reaching" of
`crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

/-- The spine of a local type ends in an action or an end, never in a
variable. -/
def L.spineOk : L → Bool
  | .mu body => body.spineOk
  | .var _ => false
  | _ => true

/-- The canonical form of a type with no action: the vacuous loop. -/
def stuck : L := .mu (.var 0)

/-- The head used by the invariant: the head normal form, with every
variable-spined type taken to the one canonical form. -/
def mhead (l : L) : L := if l.spineOk then l.head else stuck

theorem L.subst_spine : ∀ (l : L) (k : Nat) (by' : L), l.spineOk = true →
    (l.subst k by').spineOk = true ∧ (l.subst k by').leadRecs = l.leadRecs
  | .fin, _, _, _ => ⟨rfl, rfl⟩
  | .send .., _, _, _ => ⟨rfl, rfl⟩
  | .recv .., _, _, _ => ⟨rfl, rfl⟩
  | .var v, _, _, h => by simp [L.spineOk] at h
  | .mu body, k, by', h => by
    have := L.subst_spine body (k + 1) by' (by simpa [L.spineOk] using h)
    simp only [L.subst, L.spineOk, L.leadRecs]
    exact ⟨this.1, by rw [this.2]⟩

theorem L.subst_varspine : ∀ (l : L) (k : Nat) (by' : L), l.spineOk = false →
    by'.spineOk = false → (l.subst k by').spineOk = false
  | .fin, _, _, h, _ => by simp [L.spineOk] at h
  | .send .., _, _, h, _ => by simp [L.spineOk] at h
  | .recv .., _, _, h, _ => by simp [L.spineOk] at h
  | .var v, k, by', _, hb => by
    simp only [L.subst]
    split
    · exact hb
    · split <;> rfl
  | .mu body, k, by', h, hb => by
    simp only [L.spineOk] at h
    simp only [L.subst, L.spineOk]
    exact L.subst_varspine body (k + 1) by' h hb

theorem L.headN_varspine : ∀ (k : Nat) (l : L), l.spineOk = false → (l.headN k).spineOk = false
  | 0, l, h => by simpa [L.headN] using h
  | k + 1, l, h => by
    cases l with
    | mu body =>
      simp only [L.headN, L.unfold]
      exact L.headN_varspine k _ (L.subst_varspine body 0 (.mu body) (by simpa [L.spineOk] using h) h)
    | fin => simp [L.spineOk] at h
    | var v => simpa [L.headN] using h
    | send t ls => simp [L.spineOk] at h
    | recv f ls => simp [L.spineOk] at h

theorem mhead_of_spine {l : L} (h : l.spineOk = true) : mhead l = l.head := by
  simp [mhead, h]

theorem mhead_of_varspine {l : L} (h : l.spineOk = false) : mhead l = stuck := by
  simp [mhead, h]

/-- The head of a variable-spined type is no action and no end. -/
theorem head_of_varspine {l : L} (h : l.spineOk = false) :
    (∀ t ls, l.head ≠ .send t ls) ∧ (∀ f ls, l.head ≠ .recv f ls) ∧ l.head ≠ .fin := by
  have := L.headN_varspine l.leadRecs l h
  unfold L.head
  refine ⟨?_, ?_, ?_⟩
  · intro t ls e; rw [e] at this; simp [L.spineOk] at this
  · intro f ls e; rw [e] at this; simp [L.spineOk] at this
  · intro e; rw [e] at this; simp [L.spineOk] at this

theorem head_flat {l : L} (h1 : ∀ b, l ≠ .mu b) : l.head = l := by
  cases l with
  | mu b => exact absurd rfl (h1 b)
  | fin => rfl
  | var v => rfl
  | send t ls => rfl
  | recv f ls => rfl

/-- The head of a non-recursion that is no variable is itself. -/
theorem mhead_flat {l : L} (h1 : ∀ b, l ≠ .mu b) (h2 : ∀ v, l ≠ .var v) : mhead l = l := by
  cases l with
  | mu b => exact absurd rfl (h1 b)
  | var v => exact absurd rfl (h2 v)
  | fin => rfl
  | send t ls => rfl
  | recv f ls => rfl

theorem head_mu (body : L) : (L.mu body).head = ((L.mu body).unfold).headN body.leadRecs := by
  simp only [L.head, L.leadRecs]
  rw [show 1 + body.leadRecs = body.leadRecs + 1 by omega]
  rfl

/-- Unfolding keeps the head. -/
theorem mhead_unfold (body : L) : mhead (L.mu body).unfold = mhead (.mu body) := by
  by_cases hs : (L.mu body).spineOk = true
  · have hb : body.spineOk = true := by simpa [L.spineOk] using hs
    have := L.subst_spine body 0 (.mu body) hb
    rw [mhead_of_spine hs, mhead_of_spine (by simpa [L.unfold] using this.1), head_mu]
    simp only [L.head, L.unfold, this.2]
  · have hs' : (L.mu body).spineOk = false := by simpa using hs
    have hb : body.spineOk = false := by simpa [L.spineOk] using hs'
    have := L.subst_varspine body 0 (.mu body) hb hs'
    rw [mhead_of_varspine hs', mhead_of_varspine (by simpa [L.unfold] using this)]

theorem refines_spine : ∀ (m : L) {a : L}, Refines m a → m.spineOk = a.spineOk
  | .mu body, a, h => by
    obtain ⟨ba, rfl, hr⟩ := refines_mu_left.mp h
    simp [L.spineOk, refines_spine body hr]
  | .fin, a, h => by cases refines_fin_left.mp h; rfl
  | .var n, a, h => by cases refines_var_left.mp h; rfl
  | .send t ml, a, h => by obtain ⟨al, rfl, _⟩ := refines_send_left.mp h; rfl
  | .recv f ml, a, h => by obtain ⟨al, rfl, _⟩ := refines_recv_left.mp h; rfl

theorem refines_mhead {m a : L} (h : Refines m a) : Refines (mhead m) (mhead a) := by
  have hs := refines_spine m h
  unfold mhead
  rw [hs]
  split
  · exact refines_head h
  · exact Refines.refl _

theorem R.head_mu (body : R) :
    (R.mu body).head = ((R.mu body).unfold).headN body.leadRecs := by
  simp only [R.head, R.leadRecs]
  rw [show 1 + body.leadRecs = body.leadRecs + 1 by omega]
  rfl

theorem R.head_flat {x : R} (h1 : ∀ b, x ≠ .mu b) : x.head = x := by
  cases x with
  | mu b => exact absurd rfl (h1 b)
  | fin => rfl
  | var v => rfl
  | comm f t bs => rfl
  | sent f t l c => rfl

/-- The projection of the head normal form has the head of the projection. -/
theorem mhead_rproj : ∀ (k : Nat) (x : R) (n u : Nat) (l : L), x.leadRecs = k →
    x.wf n = true → rproj x u = some l → ∃ lh, rproj x.head u = some lh ∧ mhead lh = mhead l
  | 0, x, n, u, l, hk, hw, h => by
    have : x.head = x := by
      apply R.head_flat
      intro b e; subst e; simp [R.leadRecs] at hk
    rw [this]
    exact ⟨l, h, rfl⟩
  | k + 1, x, n, u, l, hk, hw, h => by
    cases x with
    | mu body =>
      obtain ⟨lb, hlb, rfl⟩ := rproj_mu h
      have hu := R.unfold_wf hw
      have hbw : body.wfAt n [false] = true := by simpa [R.wf, R.wfAt] using hw
      have hsub := rproj_subst body n [false] 0 (.mu body) u lb (closeRec lb) hbw hw hlb h
      have hnb := rproj_normal body n [false] u lb hbw hlb
      have hkk : (R.mu body).unfold.leadRecs = k := by
        have := hu.2; omega
      obtain ⟨lh, hlh, hm⟩ := mhead_rproj k _ n u _ hkk hu.1 (by simpa [R.unfold] using hsub)
      have hhead : (R.mu body).head = (R.mu body).unfold.head := by
        rw [R.head_mu]
        simp only [R.head]
        have : body.leadRecs = k := by simp [R.leadRecs] at hk; omega
        rw [this, hkk]
      rw [hhead]
      refine ⟨lh, hlh, ?_⟩
      rw [hm]
      -- the head of the substituted body's projection is the head of the recursion's
      by_cases ha : lb.actionless = true
      · rcases knormal_actionless hnb.1 ha with rfl | ⟨v, rfl⟩
        · simp [closeRec, L.actionless, L.subst]
        · have hv : v = 0 := by
            have := hnb.2; simp [L.closedAt] at this; omega
          subst hv
          simp [closeRec, L.actionless, L.subst]
      · have e2 : closeRec lb = if lb.free 0 = true then .mu lb else lb.shift 0 := by
          unfold closeRec; rw [if_neg ha]
        by_cases hf : lb.free 0 = true
        · rw [e2, if_pos hf]
          exact mhead_unfold lb
        · rw [e2, if_neg hf]
          rw [L.subst_notfree lb 0 _ (by simpa using hf)]
    | fin => simp [R.leadRecs] at hk
    | var v => simp [R.leadRecs] at hk
    | comm f t bs => simp [R.leadRecs] at hk
    | sent f t lab c => simp [R.leadRecs] at hk

/-! ## Actions and reaching -/

mutual
/-- Every path of `x` (through recursions and every branch of every
choice) meets a node pending for role `u` (a communication it sends or
receives, or a message in transit to it) before an end or a variable. -/
def R.reaches : R → Nat → Bool
  | .fin, _ => false
  | .var _, _ => false
  | .mu body, u => body.reaches u
  | .comm frm dst bs, u => decide (frm = u) || decide (dst = u) || bs.reaches u
  | .sent _ dst _ cont, u => decide (dst = u) || cont.reaches u
/-- Every branch reaches `u` (an empty list does not: it has no path). -/
def RBs.reaches : RBs → Nat → Bool
  | .nil, _ => false
  | .cons _ g .nil, u => g.reaches u
  | .cons _ g (.cons l2 g2 r2), u => g.reaches u && (RBs.cons l2 g2 r2).reaches u
end

theorem merge_actionless : ∀ (n : Nat) (a b m : L), a.size ≤ n → merge a b = some m →
    (a.actionless = true ∨ b.actionless = true) →
    m.actionless = true ∧ a.actionless = true ∧ b.actionless = true
  | 0, a, _, _, hn, _, _ => by cases a <;> simp [L.size] at hn
  | n + 1, a, b, m, hn, h, hab => by
    rcases merge_cases h with ⟨rfl, rfl⟩ | ⟨ba, bb, mb, rfl, rfl, hmb, rfl⟩ |
      ⟨f, la, lb, ls, rfl, rfl, hls, rfl⟩
    · rcases hab with h1 | h1 <;> exact ⟨h1, h1, h1⟩
    · simp only [L.actionless] at hab ⊢
      exact merge_actionless n ba bb mb (by simp [L.size] at hn; omega) hmb hab
    · simp [L.actionless] at hab

theorem closeRec_actionless {lb : L} (h : lb.actionless = true) : (closeRec lb).actionless = true := by
  unfold closeRec
  rw [if_pos h]
  split
  · split <;> rfl
  · rfl

mutual
/-- A role whose projection has an action is reached on every path. -/
theorem action_reaches : ∀ (x : R) (u : Nat) (l : L), rproj x u = some l →
    l.actionless = false → x.reaches u = true
  | .fin, _, l, h, ha => by simp only [rproj] at h; cases h; simp [L.actionless] at ha
  | .var v, _, l, h, ha => by simp only [rproj] at h; cases h; simp [L.actionless] at ha
  | .mu body, u, l, h, ha => by
    obtain ⟨lb, hlb, rfl⟩ := rproj_mu h
    simp only [R.reaches]
    apply action_reaches body u lb hlb
    cases hb : lb.actionless with
    | false => rfl
    | true => rw [closeRec_actionless hb] at ha; cases ha
  | .comm frm dst bs, u, l, h, ha => by
    simp only [R.reaches, Bool.or_eq_true, decide_eq_true_eq]
    rcases rproj_comm h with ⟨hu, _⟩ | ⟨_, hu, _⟩ | ⟨_, _, hm⟩
    · exact Or.inl (Or.inl hu.symm)
    · exact Or.inl (Or.inr hu.symm)
    · exact Or.inr (action_reaches_merge bs u l hm ha)
  | .sent frm dst label cont, u, l, h, ha => by
    simp only [R.reaches, Bool.or_eq_true, decide_eq_true_eq]
    obtain ⟨lc, hlc, hcase⟩ := rproj_sent h
    rcases hcase with ⟨hu, _⟩ | ⟨_, rfl⟩
    · exact Or.inl hu.symm
    · exact Or.inr (action_reaches cont u l hlc ha)
theorem action_reaches_merge : ∀ (bs : RBs) (u : Nat) (m : L), rprojMerge bs u = some m →
    m.actionless = false → bs.reaches u = true
  | .nil, _, _, h, _ => by simp [rprojMerge] at h
  | .cons label g .nil, u, m, h, ha => by
    obtain ⟨l, hl, hcase⟩ := rprojMerge_cons h
    rcases hcase with ⟨_, rfl⟩ | ⟨hne, _⟩
    · simp only [RBs.reaches]
      exact action_reaches g u _ hl ha
    · exact absurd rfl hne
  | .cons label g (.cons l2 g2 r2), u, m, h, ha => by
    obtain ⟨l, hl, hcase⟩ := rprojMerge_cons h
    rcases hcase with ⟨hnil, _⟩ | ⟨_, mr, hmr, hm⟩
    · cases hnil
    · have hla : l.actionless = false ∧ mr.actionless = false := by
        cases h1 : l.actionless with
        | true =>
          have := (merge_actionless _ l mr m (Nat.le_refl _) hm (Or.inl h1)).1
          rw [ha] at this; cases this
        | false =>
          cases h2 : mr.actionless with
          | true =>
            have := (merge_actionless _ l mr m (Nat.le_refl _) hm (Or.inr h2)).1
            rw [ha] at this; cases this
          | false => exact ⟨rfl, rfl⟩
      simp only [RBs.reaches, Bool.and_eq_true]
      exact ⟨action_reaches g u l hl hla.1, action_reaches_merge (.cons l2 g2 r2) u mr hmr hla.2⟩
end

theorem L.subst_actionless : ∀ (l : L) (k : Nat) (by' : L), l.actionless = true →
    by'.actionless = true → (l.subst k by').actionless = true
  | .fin, _, _, _, _ => rfl
  | .var v, k, by', _, hb => by
    simp only [L.subst]
    split
    · exact hb
    · split <;> rfl
  | .mu body, k, by', h, hb => by
    simp only [L.actionless] at h
    simp only [L.subst, L.actionless]
    exact L.subst_actionless body (k + 1) by' h hb
  | .send .., _, _, h, _ => by simp [L.actionless] at h
  | .recv .., _, _, h, _ => by simp [L.actionless] at h

theorem L.headN_actionless : ∀ (k : Nat) (l : L), l.actionless = true →
    (l.headN k).actionless = true
  | 0, l, h => by simpa [L.headN] using h
  | k + 1, l, h => by
    cases l with
    | mu body =>
      simp only [L.headN, L.unfold]
      exact L.headN_actionless k _
        (L.subst_actionless body 0 (.mu body) (by simpa [L.actionless] using h) h)
    | fin => simpa [L.headN] using h
    | var v => simpa [L.headN] using h
    | send t ls => simp [L.actionless] at h
    | recv f ls => simp [L.actionless] at h

theorem L.headN_spine : ∀ (k : Nat) (l : L), l.spineOk = true → l.leadRecs = k →
    (∀ b, l.headN k ≠ .mu b) ∧ (∀ v, l.headN k ≠ .var v) ∧ (l.headN k).spineOk = true
  | 0, l, h, hk => by
    simp only [L.headN]
    refine ⟨?_, ?_, h⟩
    · intro b e; subst e; simp [L.leadRecs] at hk
    · intro v e; subst e; simp [L.spineOk] at h
  | k + 1, l, h, hk => by
    cases l with
    | mu body =>
      have := L.subst_spine body 0 (.mu body) (by simpa [L.spineOk] using h)
      simp only [L.headN, L.unfold]
      exact L.headN_spine k _ this.1 (by rw [this.2]; simp [L.leadRecs] at hk; omega)
    | fin => simp [L.leadRecs] at hk
    | var v => simp [L.leadRecs] at hk
    | send t ls => simp [L.leadRecs] at hk
    | recv f ls => simp [L.leadRecs] at hk

/-- The head normal form of a spine-ok type is not a recursion. -/
theorem L.head_spine {l : L} (h : l.spineOk = true) :
    (∀ b, l.head ≠ .mu b) ∧ (∀ v, l.head ≠ .var v) ∧ l.head.spineOk = true :=
  L.headN_spine _ l h rfl

/-- The head of a type with no action is an end or the canonical loop. -/
theorem actionless_mhead {l : L} (h : l.actionless = true) : mhead l = .fin ∨ mhead l = stuck := by
  by_cases hs : l.spineOk = true
  · rw [mhead_of_spine hs]
    have h1 := L.headN_actionless l.leadRecs l h
    have h2 := L.head_spine hs
    unfold L.head at h2 ⊢
    cases hh : l.headN l.leadRecs with
    | fin => exact Or.inl rfl
    | var v => exact absurd hh (h2.2.1 v)
    | mu b => exact absurd hh (h2.1 b)
    | send t ls => rw [hh] at h1; simp [L.actionless] at h1
    | recv f ls => rw [hh] at h1; simp [L.actionless] at h1
  · exact Or.inr (mhead_of_varspine (by simpa using hs))

/-- A head with an action, in the invariant's sense. -/
def L.acting : L → Bool
  | .send .. => true
  | .recv .. => true
  | _ => false

end MP
