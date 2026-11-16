import Proto.Binders
/-!
# The projection's normality, distinctness, and substitution

Port of the sections "the projection's normality" and "the projection and
the head normal form" (first half) of
`crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

theorem Ls.remove_knormal : ∀ (ls : Ls) (label : Nat), ls.knormal = true →
    (ls.remove label).knormal = true
  | .nil, _, _ => rfl
  | .cons l0 c rest, label, h => by
    simp only [Ls.knormal, Bool.and_eq_true] at h
    simp only [Ls.remove]
    split
    · exact Ls.remove_knormal rest label h.2
    · simp only [Ls.knormal, Bool.and_eq_true]
      exact ⟨h.1, Ls.remove_knormal rest label h.2⟩

theorem Ls.lookup_knormal : ∀ (ls : Ls) {label : Nat} {c : L}, ls.knormal = true →
    ls.lookup label = some c → c.knormal = true
  | .nil, _, _, _, h => by simp [Ls.lookup] at h
  | .cons l0 c0 rest, label, c, hk, h => by
    simp only [Ls.knormal, Bool.and_eq_true] at hk
    simp only [Ls.lookup] at h
    split at h
    · cases h; exact hk.1
    · exact Ls.lookup_knormal rest hk.2 h

theorem Ls.remove_closed : ∀ (ls : Ls) (label m : Nat), ls.closedAt m = true →
    (ls.remove label).closedAt m = true
  | .nil, _, _, _ => rfl
  | .cons l0 c rest, label, m, h => by
    simp only [Ls.closedAt, Bool.and_eq_true] at h
    simp only [Ls.remove]
    split
    · exact Ls.remove_closed rest label m h.2
    · simp only [Ls.closedAt, Bool.and_eq_true]
      exact ⟨h.1, Ls.remove_closed rest label m h.2⟩

theorem Ls.lookup_closed : ∀ (ls : Ls) {label : Nat} {c : L} (m : Nat), ls.closedAt m = true →
    ls.lookup label = some c → c.closedAt m = true
  | .nil, _, _, _, _, h => by simp [Ls.lookup] at h
  | .cons l0 c0 rest, label, c, m, hk, h => by
    simp only [Ls.closedAt, Bool.and_eq_true] at hk
    simp only [Ls.lookup] at h
    split at h
    · cases h; exact hk.1
    · exact Ls.lookup_closed rest m hk.2 h

/-- A merge with no action comes from arguments with none. -/
theorem merge_actionless_rev : ∀ (n : Nat) (a b m : L), a.size ≤ n → merge a b = some m →
    m.actionless = true → a.actionless = true ∧ b.actionless = true
  | 0, a, _, _, hn, _, _ => by cases a <;> simp [L.size] at hn
  | n + 1, a, b, m, hn, h, hm => by
    rcases merge_cases h with ⟨rfl, rfl⟩ | ⟨ba, bb, mb, rfl, rfl, hmb, rfl⟩ |
      ⟨f, la, lb, ls, rfl, rfl, hls, rfl⟩
    · exact ⟨hm, hm⟩
    · simp only [L.actionless] at hm ⊢
      exact merge_actionless_rev n ba bb mb (by simp [L.size] at hn; omega) hmb hm
    · simp [L.actionless] at hm

theorem merge_knormal_aux (n : Nat) :
    (∀ a b m : L, a.size + b.size ≤ n → merge a b = some m → a.knormal = true →
      b.knormal = true → m.knormal = true) ∧
    (∀ a b r : Ls, a.size + b.size ≤ n → mergeLs a b = some r → a.knormal = true →
      b.knormal = true → r.knormal = true) := by
  induction n using Nat.strong_induction_on with
  | _ n ih =>
    constructor
    · intro a b m hn h ha hb
      rcases merge_cases h with ⟨rfl, rfl⟩ | ⟨ba, bb, mb, rfl, rfl, hm, rfl⟩ |
        ⟨f, la, lb, ls, rfl, rfl, hls, rfl⟩
      · exact ha
      · simp only [L.knormal, Bool.and_eq_true, Bool.not_eq_true'] at ha hb ⊢
        refine ⟨?_, (ih (ba.size + bb.size) (by simp [L.size] at hn; omega)).1 ba bb mb
          (Nat.le_refl _) hm ha.2 hb.2⟩
        cases hact : mb.actionless with
        | false => rfl
        | true =>
          have := (merge_actionless_rev _ ba bb mb (Nat.le_refl _) hm hact).1
          rw [ha.1] at this; cases this
      · simp only [L.knormal] at ha hb ⊢
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
        simp only [Ls.knormal, Bool.and_eq_true] at ha ⊢
        have hsz := Ls.remove_size b label
        refine ⟨?_, (ih (rest.size + (b.remove label).size) (by simp [Ls.size] at hn; omega)).2
          rest (b.remove label) r2 (Nat.le_refl _) hr2 ha.2 (Ls.remove_knormal b label hb)⟩
        rcases hm with ⟨lb, hlb, hmm⟩ | ⟨_, rfl⟩
        · have hs := Ls.lookup_size b hlb
          exact (ih (l.size + lb.size) (by simp [Ls.size] at hn; omega)).1 l lb m
            (Nat.le_refl _) hmm ha.1 (Ls.lookup_knormal b hb hlb)
        · exact ha.1

theorem merge_knormal {a b m : L} (h : merge a b = some m) (ha : a.knormal = true)
    (hb : b.knormal = true) : m.knormal = true :=
  (merge_knormal_aux _).1 a b m (Nat.le_refl _) h ha hb

theorem merge_closed_aux (n : Nat) :
    (∀ (a b m : L) (k : Nat), a.size + b.size ≤ n → merge a b = some m → a.closedAt k = true →
      b.closedAt k = true → m.closedAt k = true) ∧
    (∀ (a b r : Ls) (k : Nat), a.size + b.size ≤ n → mergeLs a b = some r →
      a.closedAt k = true → b.closedAt k = true → r.closedAt k = true) := by
  induction n using Nat.strong_induction_on with
  | _ n ih =>
    constructor
    · intro a b m k hn h ha hb
      rcases merge_cases h with ⟨rfl, rfl⟩ | ⟨ba, bb, mb, rfl, rfl, hm, rfl⟩ |
        ⟨f, la, lb, ls, rfl, rfl, hls, rfl⟩
      · exact ha
      · simp only [L.closedAt] at ha hb ⊢
        exact (ih (ba.size + bb.size) (by simp [L.size] at hn; omega)).1 ba bb mb (k + 1)
          (Nat.le_refl _) hm ha hb
      · simp only [L.closedAt] at ha hb ⊢
        exact (ih (la.size + lb.size) (by simp [L.size] at hn; omega)).2 la lb ls k
          (Nat.le_refl _) hls ha hb
    · intro a b r k hn h ha hb
      cases a with
      | nil =>
        simp only [mergeLs] at h
        cases h
        exact hb
      | cons label l rest =>
        obtain ⟨m, r2, rfl, hr2, hm⟩ := mergeLs_cons h
        simp only [Ls.closedAt, Bool.and_eq_true] at ha ⊢
        have hsz := Ls.remove_size b label
        refine ⟨?_, (ih (rest.size + (b.remove label).size) (by simp [Ls.size] at hn; omega)).2
          rest (b.remove label) r2 k (Nat.le_refl _) hr2 ha.2 (Ls.remove_closed b label k hb)⟩
        rcases hm with ⟨lb, hlb, hmm⟩ | ⟨_, rfl⟩
        · have hs := Ls.lookup_size b hlb
          exact (ih (l.size + lb.size) (by simp [Ls.size] at hn; omega)).1 l lb m k
            (Nat.le_refl _) hmm ha.1 (Ls.lookup_closed b k hb hlb)
        · exact ha.1

theorem merge_closed {a b m : L} (h : merge a b = some m) {k : Nat} (ha : a.closedAt k = true)
    (hb : b.closedAt k = true) : m.closedAt k = true :=
  (merge_closed_aux _).1 a b m k (Nat.le_refl _) h ha hb

/-! ## Inversions of `rproj` -/

theorem rproj_mu {body : R} {u : Nat} {l : L} (h : rproj (.mu body) u = some l) :
    ∃ lb, rproj body u = some lb ∧ l = closeRec lb := by
  simp only [rproj, Option.map_eq_some_iff] at h
  obtain ⟨lb, h1, h2⟩ := h
  exact ⟨lb, h1, h2.symm⟩

theorem rproj_comm {frm dst : Nat} {bs : RBs} {u : Nat} {l : L}
    (h : rproj (.comm frm dst bs) u = some l) :
    (u = frm ∧ ∃ ls, rprojBs bs u = some ls ∧ l = .send dst ls) ∨
    (u ≠ frm ∧ u = dst ∧ ∃ ls, rprojBs bs u = some ls ∧ l = .recv frm ls) ∨
    (u ≠ frm ∧ u ≠ dst ∧ rprojMerge bs u = some l) := by
  simp only [rproj] at h
  by_cases h1 : u = frm
  · rw [if_pos h1, Option.map_eq_some_iff] at h
    obtain ⟨ls, hls, rfl⟩ := h
    exact Or.inl ⟨h1, ls, hls, rfl⟩
  · rw [if_neg h1] at h
    by_cases h2 : u = dst
    · rw [if_pos h2, Option.map_eq_some_iff] at h
      obtain ⟨ls, hls, rfl⟩ := h
      exact Or.inr (Or.inl ⟨h1, h2, ls, hls, rfl⟩)
    · rw [if_neg h2] at h
      exact Or.inr (Or.inr ⟨h1, h2, h⟩)

theorem rproj_sent {frm dst label : Nat} {cont : R} {u : Nat} {l : L}
    (h : rproj (.sent frm dst label cont) u = some l) :
    ∃ lc, rproj cont u = some lc ∧
      ((u = dst ∧ l = .recv frm (.cons label lc .nil)) ∨ (u ≠ dst ∧ l = lc)) := by
  simp only [rproj] at h
  cases hc : rproj cont u with
  | none => simp [hc] at h
  | some lc =>
    simp only [hc] at h
    refine ⟨lc, rfl, ?_⟩
    by_cases hu : u = dst
    · rw [if_pos hu] at h; cases h; exact Or.inl ⟨hu, rfl⟩
    · rw [if_neg hu] at h; cases h; exact Or.inr ⟨hu, rfl⟩

/-! ## Distinct labels -/

mutual
/-- The projection of a well-formed runtime type has distinct labels. -/
theorem rproj_distinct : ∀ (x : R) (n : Nat) (env : List Bool) (u : Nat) (l : L),
    x.wfAt n env = true → rproj x u = some l → l.distinct = true
  | .fin, _, _, _, l, _, h => by simp only [rproj] at h; cases h; rfl
  | .var v, _, _, _, l, _, h => by simp only [rproj] at h; cases h; rfl
  | .mu body, n, env, u, l, hw, h => by
    obtain ⟨lb, hlb, rfl⟩ := rproj_mu h
    simp only [R.wfAt] at hw
    have hd := rproj_distinct body n (false :: env) u lb hw hlb
    unfold closeRec
    split
    · split <;> first | rfl | (split <;> rfl)
    · split
      · simpa [L.distinct] using hd
      · exact (L.shift_shape lb 0 hd).1
  | .comm frm dst bs, n, env, u, l, hw, h => by
    simp only [R.wfAt, Bool.and_eq_true] at hw
    rcases rproj_comm h with ⟨_, ls, hls, rfl⟩ | ⟨_, _, ls, hls, rfl⟩ | ⟨_, _, hm⟩
    · simpa [L.distinct] using rprojBs_distinct bs n (allTrue env) u ls hw.2 hw.1.2 hls
    · simpa [L.distinct] using rprojBs_distinct bs n (allTrue env) u ls hw.2 hw.1.2 hls
    · exact rprojMerge_distinct bs n (allTrue env) u l hw.2 hm
  | .sent frm dst label cont, n, env, u, l, hw, h => by
    simp only [R.wfAt, Bool.and_eq_true] at hw
    obtain ⟨lc, hlc, hcase⟩ := rproj_sent h
    have hd := rproj_distinct cont n (allTrue env) u lc hw.2 hlc
    rcases hcase with ⟨_, rfl⟩ | ⟨_, rfl⟩
    · simp [L.distinct, Ls.distinct, Ls.lookup, hd]
    · exact hd
theorem rprojBs_distinct : ∀ (bs : RBs) (n : Nat) (env : List Bool) (u : Nat) (ls : Ls),
    bs.wfBs n env = true → bs.distinct = true → rprojBs bs u = some ls → ls.distinct = true
  | .nil, _, _, _, ls, _, _, h => by simp only [rprojBs] at h; cases h; rfl
  | .cons label g rest, n, env, u, ls, hw, hd, h => by
    obtain ⟨l, ls', h1, h2, rfl⟩ := rprojBs_cons h
    simp only [RBs.wfBs, Bool.and_eq_true] at hw
    simp only [RBs.distinct, Bool.and_eq_true] at hd
    simp only [Ls.distinct, Bool.and_eq_true]
    refine ⟨⟨?_, rproj_distinct g n env u l hw.1 h1⟩,
      rprojBs_distinct rest n env u ls' hw.2 hd.2 h2⟩
    rw [(rprojBs_lookup rest u ls' h2).2 label (Option.isNone_iff_eq_none.mp hd.1)]
    rfl
theorem rprojMerge_distinct : ∀ (bs : RBs) (n : Nat) (env : List Bool) (u : Nat) (m : L),
    bs.wfBs n env = true → rprojMerge bs u = some m → m.distinct = true
  | .nil, _, _, _, _, _, h => by simp [rprojMerge] at h
  | .cons label g rest, n, env, u, m, hw, h => by
    obtain ⟨l, hl, hcase⟩ := rprojMerge_cons h
    simp only [RBs.wfBs, Bool.and_eq_true] at hw
    have hd := rproj_distinct g n env u l hw.1 hl
    rcases hcase with ⟨_, rfl⟩ | ⟨_, mr, hmr, hm⟩
    · exact hd
    · exact merge_distinct hm hd (rprojMerge_distinct rest n env u mr hw.2 hmr)
end

/-! ## Normality and closedness -/

theorem closeRec_closed {lb : L} {m : Nat} (hc : lb.closedAt (m + 1) = true)
    (hk : lb.knormal = true) : (closeRec lb).closedAt m = true := by
  unfold closeRec
  by_cases ha : lb.actionless = true
  · rw [if_pos ha]
    rcases knormal_actionless hk ha with rfl | ⟨v, rfl⟩
    · rfl
    · simp only [L.closedAt, decide_eq_true_eq] at hc
      dsimp only
      split
      · rfl
      · simp only [L.closedAt, decide_eq_true_eq]; omega
  · rw [if_neg ha]
    by_cases hf : lb.free 0 = true
    · rw [if_pos hf]; simpa [L.closedAt] using hc
    · rw [if_neg hf]
      exact L.shift_closed lb 0 m hc (by simpa using hf) (Nat.zero_le _)

mutual
/-- The projection of a well-formed runtime type is normal and closed
under its environment. -/
theorem rproj_normal : ∀ (x : R) (n : Nat) (env : List Bool) (u : Nat) (l : L),
    x.wfAt n env = true → rproj x u = some l →
    l.knormal = true ∧ l.closedAt env.length = true
  | .fin, _, _, _, l, _, h => by simp only [rproj] at h; cases h; exact ⟨rfl, rfl⟩
  | .var v, n, env, _, l, hw, h => by
    simp only [rproj] at h; cases h
    refine ⟨rfl, ?_⟩
    simp only [R.wfAt] at hw
    simp only [L.closedAt, decide_eq_true_eq]
    by_contra hv
    rw [getD_of_ge _ (by omega)] at hw; cases hw
  | .mu body, n, env, u, l, hw, h => by
    obtain ⟨lb, hlb, rfl⟩ := rproj_mu h
    simp only [R.wfAt] at hw
    have ih := rproj_normal body n (false :: env) u lb hw hlb
    have hd := rproj_distinct body n (false :: env) u lb hw hlb
    exact ⟨(closeRec_shape hd ih.1).2.1, closeRec_closed (by simpa using ih.2) ih.1⟩
  | .comm frm dst bs, n, env, u, l, hw, h => by
    simp only [R.wfAt, Bool.and_eq_true] at hw
    rcases rproj_comm h with ⟨_, ls, hls, rfl⟩ | ⟨_, _, ls, hls, rfl⟩ | ⟨_, _, hm⟩
    · have := rprojBs_normal bs n (allTrue env) u ls hw.2 hls
      rw [allTrue_length] at this
      simpa [L.knormal, L.closedAt] using this
    · have := rprojBs_normal bs n (allTrue env) u ls hw.2 hls
      rw [allTrue_length] at this
      simpa [L.knormal, L.closedAt] using this
    · have := rprojMerge_normal bs n (allTrue env) u l hw.2 hm
      rwa [allTrue_length] at this
  | .sent frm dst label cont, n, env, u, l, hw, h => by
    simp only [R.wfAt, Bool.and_eq_true] at hw
    obtain ⟨lc, hlc, hcase⟩ := rproj_sent h
    have ih := rproj_normal cont n (allTrue env) u lc hw.2 hlc
    rw [allTrue_length] at ih
    rcases hcase with ⟨_, rfl⟩ | ⟨_, rfl⟩
    · simp [L.knormal, Ls.knormal, L.closedAt, Ls.closedAt, ih.1, ih.2]
    · exact ih
theorem rprojBs_normal : ∀ (bs : RBs) (n : Nat) (env : List Bool) (u : Nat) (ls : Ls),
    bs.wfBs n env = true → rprojBs bs u = some ls →
    ls.knormal = true ∧ ls.closedAt env.length = true
  | .nil, _, _, _, ls, _, h => by simp only [rprojBs] at h; cases h; exact ⟨rfl, rfl⟩
  | .cons label g rest, n, env, u, ls, hw, h => by
    obtain ⟨l, ls', h1, h2, rfl⟩ := rprojBs_cons h
    simp only [RBs.wfBs, Bool.and_eq_true] at hw
    have i1 := rproj_normal g n env u l hw.1 h1
    have i2 := rprojBs_normal rest n env u ls' hw.2 h2
    simp [Ls.knormal, Ls.closedAt, i1.1, i1.2, i2.1, i2.2]
theorem rprojMerge_normal : ∀ (bs : RBs) (n : Nat) (env : List Bool) (u : Nat) (m : L),
    bs.wfBs n env = true → rprojMerge bs u = some m →
    m.knormal = true ∧ m.closedAt env.length = true
  | .nil, _, _, _, _, _, h => by simp [rprojMerge] at h
  | .cons label g rest, n, env, u, m, hw, h => by
    obtain ⟨l, hl, hcase⟩ := rprojMerge_cons h
    simp only [RBs.wfBs, Bool.and_eq_true] at hw
    have i1 := rproj_normal g n env u l hw.1 hl
    rcases hcase with ⟨_, rfl⟩ | ⟨_, mr, hmr, hm⟩
    · exact i1
    · have i2 := rprojMerge_normal rest n env u mr hw.2 hmr
      exact ⟨merge_knormal hm i1.1 i2.1, merge_closed hm i1.2 i2.2⟩
end

/-! ## Projection commutes with substitution -/

theorem RBs.subst_ne_nil {bs : RBs} (h : bs ≠ .nil) (k : Nat) (by' : R) : bs.subst k by' ≠ .nil := by
  cases bs with
  | nil => exact absurd rfl h
  | cons l g rest => simp [RBs.subst]

mutual
theorem rproj_subst : ∀ (x : R) (n : Nat) (env : List Bool) (k : Nat) (by' : R) (u : Nat)
    (l lby : L), x.wfAt n env = true → by'.wf n = true → rproj x u = some l →
    rproj by' u = some lby → rproj (x.subst k by') u = some (l.subst k lby)
  | .fin, _, _, _, _, _, l, _, _, _, h, _ => by
    simp only [rproj] at h; cases h; rfl
  | .var v, _, _, k, by', u, l, lby, _, _, h, hb => by
    simp only [rproj] at h; cases h
    simp only [R.subst, L.subst]
    split
    · exact hb
    · split <;> rfl
  | .mu body, n, env, k, by', u, l, lby, hw, hbw, h, hb => by
    obtain ⟨lb, hlb, rfl⟩ := rproj_mu h
    simp only [R.wfAt] at hw
    have ih := rproj_subst body n (false :: env) (k + 1) by' u lb lby hw hbw hlb hb
    have hn := rproj_normal body n (false :: env) u lb hw hlb
    have hnb := rproj_normal by' n [] u lby hbw hb
    simp only [R.subst, rproj, ih, Option.map_some]
    rw [closeRec_subst k hn.1 (by simpa using hnb.2) hnb.1]
  | .comm frm dst bs, n, env, k, by', u, l, lby, hw, hbw, h, hb => by
    simp only [R.wfAt, Bool.and_eq_true] at hw
    rcases rproj_comm h with ⟨hu, ls, hls, rfl⟩ | ⟨hu1, hu, ls, hls, rfl⟩ | ⟨hu1, hu2, hm⟩
    · have := rprojBs_subst bs n (allTrue env) k by' u ls lby hw.2 hbw hls hb
      simp only [R.subst, rproj, if_pos hu, this, Option.map_some, L.subst]
    · have := rprojBs_subst bs n (allTrue env) k by' u ls lby hw.2 hbw hls hb
      simp only [R.subst, rproj, if_neg hu1, if_pos hu, this, Option.map_some, L.subst]
    · have := rprojMerge_subst bs n (allTrue env) k by' u l lby hw.2 hbw hm hb
      simp only [R.subst, rproj, if_neg hu1, if_neg hu2, this]
  | .sent frm dst label cont, n, env, k, by', u, l, lby, hw, hbw, h, hb => by
    simp only [R.wfAt, Bool.and_eq_true] at hw
    obtain ⟨lc, hlc, hcase⟩ := rproj_sent h
    have ih := rproj_subst cont n (allTrue env) k by' u lc lby hw.2 hbw hlc hb
    rcases hcase with ⟨hu, rfl⟩ | ⟨hu, rfl⟩
    · simp only [R.subst, rproj, ih, if_pos hu, L.subst, Ls.subst]
    · simp only [R.subst, rproj, ih, if_neg hu]
theorem rprojBs_subst : ∀ (bs : RBs) (n : Nat) (env : List Bool) (k : Nat) (by' : R) (u : Nat)
    (ls : Ls) (lby : L), bs.wfBs n env = true → by'.wf n = true → rprojBs bs u = some ls →
    rproj by' u = some lby → rprojBs (bs.subst k by') u = some (ls.subst k lby)
  | .nil, _, _, _, _, _, ls, _, _, _, h, _ => by
    simp only [rprojBs] at h; cases h; rfl
  | .cons label g rest, n, env, k, by', u, ls, lby, hw, hbw, h, hb => by
    obtain ⟨l, ls', h1, h2, rfl⟩ := rprojBs_cons h
    simp only [RBs.wfBs, Bool.and_eq_true] at hw
    have i1 := rproj_subst g n env k by' u l lby hw.1 hbw h1 hb
    have i2 := rprojBs_subst rest n env k by' u ls' lby hw.2 hbw h2 hb
    simp only [RBs.subst, rprojBs, i1, i2, Ls.subst]
theorem rprojMerge_subst : ∀ (bs : RBs) (n : Nat) (env : List Bool) (k : Nat) (by' : R)
    (u : Nat) (m lby : L), bs.wfBs n env = true → by'.wf n = true → rprojMerge bs u = some m →
    rproj by' u = some lby → rprojMerge (bs.subst k by') u = some (m.subst k lby)
  | .nil, _, _, _, _, _, _, _, _, _, h, _ => by simp [rprojMerge] at h
  | .cons label g rest, n, env, k, by', u, m, lby, hw, hbw, h, hb => by
    obtain ⟨l, hl, hcase⟩ := rprojMerge_cons h
    simp only [RBs.wfBs, Bool.and_eq_true] at hw
    have i1 := rproj_subst g n env k by' u l lby hw.1 hbw hl hb
    rcases hcase with ⟨rfl, rfl⟩ | ⟨hne, mr, hmr, hm⟩
    · simp only [RBs.subst, rprojMerge, i1]
    · have i2 := rprojMerge_subst rest n env k by' u mr lby hw.2 hbw hmr hb
      have hd1 := rproj_distinct g n env u l hw.1 hl
      have hd2 := rprojMerge_distinct rest n env u mr hw.2 hmr
      have hms := merge_subst hm hd1 hd2 k lby
      cases rest with
      | nil => exact absurd rfl hne
      | cons l2 g2 r2 =>
        have e : rprojMerge ((RBs.cons label g (.cons l2 g2 r2)).subst k by') u
            = (rprojMerge ((RBs.cons l2 g2 r2).subst k by') u).bind (merge (l.subst k lby)) := by
          simp only [RBs.subst]
          conv_lhs => unfold rprojMerge
          simp only [i1]
        rw [e, i2]
        exact hms
end

end MP
