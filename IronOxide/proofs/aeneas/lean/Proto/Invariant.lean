import Proto.Node
/-!
# The invariant of a reachable configuration

`MInv c x n`: `x` is a well-formed, good runtime type that every role's
type fits and whose queues are the channels of `c`. It holds initially
with `x` the embedding of the Protocol, and every step of the
configuration has a step of the runtime type that keeps it. Port of the
section "the invariant" of `crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

def MInv (c : NC) (x : R) (n : Nat) : Prop :=
  2 ≤ n ∧ x.wf n = true ∧ x.good ∧ (∀ u, u < n → Fits (c.l u) x u n) ∧
    ∀ p q, p < n → q < n → c.ch p q = x.queue p q

mutual
theorem emb_noSent : ∀ g : G, (emb g).noSent = true
  | .fin => rfl
  | .var _ => rfl
  | .mu body => by simp only [emb, R.noSent, emb_noSent body]
  | .comm _ _ bs => by simp only [emb, R.noSent, embBs_noSent bs]
theorem embBs_noSent : ∀ bs : Bs, (embBs bs).noSent = true
  | .nil => rfl
  | .cons _ g rest => by simp [embBs, RBs.noSent, emb_noSent g, embBs_noSent rest]
end

/-- The initial configuration satisfies the invariant with the embedded
Protocol. -/
theorem minv_init {g : G} {n : Nat} (h : acceptn g n) : MInv (ninit g) (emb g) n := by
  obtain ⟨hn, hwf, hproj⟩ := h
  have hw : (emb g).wf n = true := by
    simp only [R.wf, emb_wf]
    exact hwf
  refine ⟨hn, hw, R.noSent_good _ (emb_noSent g), ?_, ?_⟩
  · intro u hu
    obtain ⟨l, hl⟩ := Option.isSome_iff_exists.mp (hproj u hu)
    refine ⟨Rel0 u n, rel0_consistent u n, hw, l, ?_, ?_⟩
    · rw [rproj_emb]; exact hl
    · simp only [ninit, hl, Option.getD_some]
      exact Refines.refl _
  · intro p q _ _
    simp only [ninit]
    exact (R.noSent_queue _ p q (emb_noSent g)).symm

/-- An action or an end at the semantics' head is the invariant's head. -/
theorem mhead_of_head {m : L} (h : m.head.acting = true ∨ m.head = .fin) : mhead m = m.head := by
  by_cases hs : m.spineOk = true
  · exact mhead_of_spine hs
  · have hv := head_of_varspine (l := m) (by simpa using hs)
    rcases h with h | h
    · cases hh : m.head with
      | send t ls => exact absurd hh (hv.1 t ls)
      | recv f ls => exact absurd hh (hv.2.1 f ls)
      | fin => rw [hh] at h; simp [L.acting] at h
      | var v => rw [hh] at h; simp [L.acting] at h
      | mu b => rw [hh] at h; simp [L.acting] at h
    · exact absurd h hv.2.2

/-- An action or an end at the invariant's head is the semantics' head. -/
theorem head_of_mhead {m : L} (h : (mhead m).acting = true ∨ mhead m = .fin) : m.head = mhead m := by
  by_cases hs : m.spineOk = true
  · exact (mhead_of_spine hs).symm
  · have := mhead_of_varspine (l := m) (by simpa using hs)
    rw [this] at h
    rcases h with h | h
    · simp [stuck, L.acting] at h
    · simp [stuck] at h

/-- A send by role `s` keeps the invariant, with the stepped runtime type. -/
theorem send_minv {c : NC} {x x2 : R} {n s t label : Nat} {ls : Ls} {cont : L}
    (h : MInv c x n) (hh : (c.l s).head = .send t ls) (hl : ls.lookup label = some cont)
    (hs : SendStep s t label x2 x) :
    MInv { l := Function.update c.l s cont,
           ch := Function.update c.ch s (Function.update (c.ch s) t (c.ch s t ++ [label])) }
      x2 n := by
  obtain ⟨hn, hw, hg, hfits, hch⟩ := h
  have hm : mhead (c.l s) = .send t ls := by
    rw [mhead_of_head (Or.inl (by rw [hh]; rfl)), hh]
  obtain ⟨hg2, ⟨hq1, hq2⟩, _⟩ := sendStep_good x2 x hw hg hs
  have hhw := (R.head_wf hw).1
  refine ⟨hn, (sendStep_wf x2 x hw hs).1, hg2, ?_, ?_⟩
  · intro u hu
    obtain ⟨rel, hc, hrel⟩ := hfits u hu
    refine ⟨RelSend rel s t label u, relSend_consistent hc, ?_⟩
    have := relSend_has (u := u) hrel hs
    by_cases hus : u = s
    · subst hus
      rw [afterAct_send hm hl] at this
      simpa using this
    · rw [afterAct_ne hus] at this
      simpa [Function.update_of_ne hus] using this
  · intro p q hp hq
    by_cases hps : p = s
    · subst hps
      by_cases hqt : q = t
      · subst hqt
        simp only [Function.update_self]
        rw [hq1, hch p q hp hq]
      · simp only [Function.update_self, Function.update_of_ne hqt]
        rw [hq2 p q (fun e => hqt e.2), hch p q hp hq]
    · simp only [Function.update_of_ne hps]
      rw [hq2 p q (fun e => hps e.1), hch p q hp hq]

/-- A receive by role `q` keeps the invariant, with the stepped runtime
type. -/
theorem recv_minv {c : NC} {x x2 : R} {n q p label : Nat} {ls : Ls} {cont : L} {tl : List Nat}
    (h : MInv c x n) (hq : q < n) (hp : p < n) (hh : (c.l q).head = .recv p ls)
    (hch0 : c.ch p q = label :: tl) (hl : ls.lookup label = some cont)
    (hs : RecvStep q p label x2 x) :
    MInv { l := Function.update c.l q cont,
           ch := Function.update c.ch p (Function.update (c.ch p) q tl) }
      x2 n := by
  obtain ⟨hn, hw, hg, hfits, hch⟩ := h
  have hm : mhead (c.l q) = .recv p ls := by
    rw [mhead_of_head (Or.inl (by rw [hh]; rfl)), hh]
  obtain ⟨hg2, ⟨⟨tl', hq1, hq1'⟩, hq2⟩, _⟩ := recvStep_good x2 x hw hg hs
  have htl : tl' = tl := by
    have := hch p q hp hq
    rw [hch0, hq1] at this
    exact (List.cons.inj this).2.symm
  subst htl
  refine ⟨hn, recvStep_wf x2 x hw hs, hg2, ?_, ?_⟩
  · intro u hu
    obtain ⟨rel, hc, hrel⟩ := hfits u hu
    refine ⟨RelRecv rel q p label u, relRecv_consistent hc, ?_⟩
    have := relRecv_has (u := u) hrel hs
    by_cases huq : u = q
    · subst huq
      rw [afterAct_recv hm hl] at this
      simpa using this
    · rw [afterAct_ne huq] at this
      simpa [Function.update_of_ne huq] using this
  · intro a b ha hb
    by_cases hap : a = p
    · subst hap
      by_cases hbq : b = q
      · subst hbq
        simp only [Function.update_self]
        exact hq1'.symm
      · simp only [Function.update_self, Function.update_of_ne hbq]
        rw [hq2 a b (fun e => hbq e.2), hch a b ha hb]
    · simp only [Function.update_of_ne hap]
      rw [hq2 a b (fun e => hap e.1), hch a b ha hb]

/-- Every step of the configuration has a step of the runtime type that
keeps the invariant. -/
theorem minv_step {c c2 : NC} {x : R} {n : Nat} (h : MInv c x n) (hs : NStep n c c2) :
    ∃ x2, MInv c2 x2 n := by
  cases hs with
  | send r dst label ls cont hr hh hto hne hl =>
    obtain ⟨rel, hc, hrel⟩ := h.2.2.2.1 r hr
    have hm : mhead (c.l r) = .send dst ls := by
      rw [mhead_of_head (Or.inl (by rw [hh]; rfl)), hh]
    obtain ⟨x2, hx2⟩ := send_exists hc hm hl _ x (Nat.le_refl _) hrel
    exact ⟨x2, send_minv h hh hl hx2⟩
  | recv r frm label ls cont tl hr hh hfrm hq hl =>
    obtain ⟨rel, hc, hrel⟩ := h.2.2.2.1 r hr
    have hm : mhead (c.l r) = .recv frm ls := by
      rw [mhead_of_head (Or.inl (by rw [hh]; rfl)), hh]
    have hxq : x.queue frm r = label :: tl := by rw [← h.2.2.2.2 frm r hfrm hr]; exact hq
    obtain ⟨⟨x2, hx2⟩, _⟩ := recv_exists hc hm _ x tl (Nat.le_refl _) hrel h.2.2.1 hxq
    exact ⟨x2, recv_minv h hr hfrm hh hq hl hx2⟩

/-- Every reachable configuration of an accepted Protocol has the
invariant with some runtime type. -/
theorem minv_reach {g : G} {n : Nat} (ha : acceptn g n) {c : NC} (h : NReach g n c) :
    ∃ x, MInv c x n := by
  induction h with
  | init => exact ⟨emb g, minv_init ha⟩
  | step c c' _ hs ih =>
    obtain ⟨x, hx⟩ := ih
    exact minv_step hx hs

/-- No unexpected reception: in a reachable configuration of an accepted
Protocol, a role whose head is a receive from a role below `n` and whose
awaited channel is not empty accepts the channel's oldest label. -/
theorem multiparty_reception {g : G} {n : Nat} (ha : acceptn g n) {c : NC} (h : NReach g n c)
    {r frm label : Nat} {ls : Ls} {tl : List Nat} (hr : r < n) (hfrm : frm < n)
    (hh : (c.l r).head = .recv frm ls) (hq : c.ch frm r = label :: tl) :
    (ls.lookup label).isSome = true := by
  obtain ⟨x, hx⟩ := minv_reach ha h
  obtain ⟨rel, hc, hrel⟩ := hx.2.2.2.1 r hr
  have hm : mhead (c.l r) = .recv frm ls := by
    rw [mhead_of_head (Or.inl (by rw [hh]; rfl)), hh]
  have hxq : x.queue frm r = label :: tl := by rw [← hx.2.2.2.2 frm r hfrm hr]; exact hq
  exact (recv_exists hc hm _ x tl (Nat.le_refl _) hrel hx.2.2.1 hxq).2

end MP
