import Proto.Effects
/-!
# The existence of a step

A role whose head is a send of a label of its list can send it: the
runtime type has the step, in every branch of every pending choice. A role
whose head is a receive from `p` with the oldest label of the channel from
`p` at hand can receive it, and that label is one it accepts. Port of the
section "the existence of a step" of
`crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

theorem SendStep.here {s t l : Nat} {x b : R} {bs : RBs} (hx : x.head = .comm s t bs)
    (hb : bs.lookup l = some b) : SendStep s t l (.sent s t l b) x := by
  simp [SendStep, hx, hb]

theorem SendStep.comm {s t l frm dst : Nat} {x : R} {bs bs2 : RBs}
    (hx : x.head = .comm frm dst bs) (h1 : frm ≠ s) (h2 : dst ≠ s)
    (hs : SendStepBs s t l bs2 bs) : SendStep s t l (.comm frm dst bs2) x := by
  simp [SendStep, hx, h1, h2, hs]

theorem SendStep.sent {s t l frm dst label : Nat} {x cont c2 : R}
    (hx : x.head = .sent frm dst label cont) (hd : dst ≠ s) (hs : SendStep s t l c2 cont) :
    SendStep s t l (.sent frm dst label c2) x := by
  simp [SendStep, hx, hd, hs]

theorem SendStepBs.cons {s t l label : Nat} {g g2 : R} {rest rest2 : RBs}
    (h1 : SendStep s t l g2 g) (h2 : SendStepBs s t l rest2 rest) :
    SendStepBs s t l (.cons label g2 rest2) (.cons label g rest) := by
  simp [SendStepBs, h1, h2]

theorem RecvStep.comm {q p l frm dst : Nat} {x : R} {bs bs2 : RBs}
    (hx : x.head = .comm frm dst bs) (h1 : frm ≠ q) (h2 : dst ≠ q)
    (hs : RecvStepBs q p l bs2 bs) : RecvStep q p l (.comm frm dst bs2) x := by
  simp [RecvStep, hx, h1, h2, hs]

theorem RecvStep.sent {q p l frm dst label : Nat} {x cont c2 : R}
    (hx : x.head = .sent frm dst label cont) (hd : dst ≠ q) (hs : RecvStep q p l c2 cont) :
    RecvStep q p l (.sent frm dst label c2) x := by
  simp [RecvStep, hx, hd, hs]

theorem RecvStepBs.cons {q p l label : Nat} {g g2 : R} {rest rest2 : RBs}
    (h1 : RecvStep q p l g2 g) (h2 : RecvStepBs q p l rest2 rest) :
    RecvStepBs q p l (.cons label g2 rest2) (.cons label g rest) := by
  simp [RecvStepBs, h1, h2]

theorem sendStepBs_exists {s t l : Nat} : ∀ (bs : RBs), bs.distinct = true →
    (∀ label b, bs.lookup label = some b → ∃ b2, SendStep s t l b2 b) →
    ∃ bs2, SendStepBs s t l bs2 bs
  | .nil, _, _ => ⟨.nil, by simp [SendStepBs]⟩
  | .cons label g rest, hd, h => by
    simp only [RBs.distinct, Bool.and_eq_true] at hd
    obtain ⟨g2, hg2⟩ := h label g RBs.lookup_first
    obtain ⟨rest2, hr2⟩ := sendStepBs_exists rest hd.2 (by
      intro lab b hb
      apply h lab b
      simp only [RBs.lookup]
      rw [if_neg]
      · exact hb
      · intro e; subst e
        rw [Option.isNone_iff_eq_none.mp hd.1] at hb; cases hb)
    exact ⟨.cons label g2 rest2, SendStepBs.cons hg2 hr2⟩

theorem recvStepBs_exists {q p l : Nat} : ∀ (bs : RBs), bs.distinct = true →
    (∀ label b, bs.lookup label = some b → ∃ b2, RecvStep q p l b2 b) →
    ∃ bs2, RecvStepBs q p l bs2 bs
  | .nil, _, _ => ⟨.nil, by simp [RecvStepBs]⟩
  | .cons label g rest, hd, h => by
    simp only [RBs.distinct, Bool.and_eq_true] at hd
    obtain ⟨g2, hg2⟩ := h label g RBs.lookup_first
    obtain ⟨rest2, hr2⟩ := recvStepBs_exists rest hd.2 (by
      intro lab b hb
      apply h lab b
      simp only [RBs.lookup]
      rw [if_neg]
      · exact hb
      · intro e; subst e
        rw [Option.isNone_iff_eq_none.mp hd.1] at hb; cases hb)
    exact ⟨.cons label g2 rest2, RecvStepBs.cons hg2 hr2⟩

theorem sdepth_comm_other {frm dst u : Nat} {bs : RBs} (h1 : frm ≠ u) (h2 : dst ≠ u) :
    (R.comm frm dst bs).sdepth u = 1 + bs.sdepth u := by
  simp only [R.sdepth]
  rw [if_neg (by rintro (e | e) <;> [exact h1 e; exact h2 e])]

theorem sdepth_sent_other {frm dst label u : Nat} {cont : R} (h : dst ≠ u) :
    (R.sent frm dst label cont).sdepth u = 1 + cont.sdepth u := by
  simp only [R.sdepth]
  rw [if_neg h]

/-- A role whose head is a send of a label of its list can send it. -/
theorem send_exists {rel : L → R → Prop} {m : L} {s t l n : Nat} {ls : Ls} {c : L}
    (hc : Consistent rel s n) (hm : mhead m = .send t ls) (hl : ls.lookup l = some c) :
    ∀ (d : Nat) (x : R), x.sdepth s ≤ d → rel m x → ∃ x2, SendStep s t l x2 x := by
  intro d
  induction d using Nat.strong_induction_on with
  | _ d ih =>
    intro x hd hrel
    obtain ⟨hw, hreach, hnode⟩ := hc m x hrel
    have hr := hreach (by rw [hm]; rfl)
    have hhw := R.head_wf hw
    have hhr := R.head_reaches _ x n s rfl hw hr
    cases hx : x.head with
    | fin => rw [hx] at hnode; rw [hnode] at hm; cases hm
    | var v => exact absurd hx (hhw.2.2 v)
    | mu b => exact absurd hx (hhw.2.1 b)
    | comm frm dst bs =>
      rw [hx] at hnode hhw hhr
      have hp := R.comm_wf_parts hhw.1
      dsimp only at hnode
      by_cases h1 : s = frm
      · rw [if_pos h1] at hnode
        obtain ⟨ls', hm', hle, _⟩ := hnode
        rw [hm] at hm'
        cases hm'
        have hsome : (bs.lookup l).isSome = true := by rw [← hle l, hl]; rfl
        obtain ⟨b, hb⟩ := Option.isSome_iff_exists.mp hsome
        subst h1
        exact ⟨.sent s t l b, SendStep.here hx hb⟩
      · rw [if_neg h1] at hnode
        by_cases h2 : s = dst
        · rw [if_pos h2] at hnode
          obtain ⟨ls', hm', _⟩ := hnode
          rw [hm] at hm'; cases hm'
        · rw [if_neg h2] at hnode
          have hsd := sdepth_comm_other (bs := bs) (fun e => h1 e.symm) (fun e => h2 e.symm)
          have hbs : ∀ label b, bs.lookup label = some b → ∃ b2, SendStep s t l b2 b := by
            intro label b hb
            have := RBs.sdepth_branch bs s hb
            exact ih (d - 1) (by omega) b (by omega) (hnode label b hb)
          obtain ⟨bs2, hbs2⟩ := sendStepBs_exists bs hp.2.2.2.2.1 hbs
          exact ⟨.comm frm dst bs2, SendStep.comm hx (fun e => h1 e.symm) (fun e => h2 e.symm) hbs2⟩
    | sent frm dst label cont =>
      rw [hx] at hnode hhw hhr
      dsimp only at hnode
      by_cases h2 : s = dst
      · rw [if_pos h2] at hnode
        obtain ⟨ls', c', hm', _⟩ := hnode
        rw [hm] at hm'; cases hm'
      · rw [if_neg h2] at hnode
        have hsd := sdepth_sent_other (frm := frm) (label := label) (cont := cont)
          (fun e => h2 e.symm)
        obtain ⟨c2, hc2⟩ := ih (d - 1) (by omega) cont (by omega) hnode
        exact ⟨.sent frm dst label c2, SendStep.sent hx (fun e => h2 e.symm) hc2⟩

/-- The receive at the head is a step, whatever the continuation. -/
theorem recvStep_here {q p l : Nat} {x cont : R} (hx : x.head = .sent p q l cont) :
    RecvStep q p l cont x := by
  cases cont with
  | fin => simp [RecvStep, hx]
  | var v => simp [RecvStep, hx]
  | mu b => simp [RecvStep, hx]
  | comm f d bs => simp [RecvStep, hx]
  | sent f d lab c => simp [RecvStep, hx]

/-- A role whose head is a receive from `p` with the oldest label of the
channel from `p` at hand can receive it, and accepts that label. -/
theorem recv_exists {rel : L → R → Prop} {m : L} {q p l n : Nat} {ls : Ls}
    (hc : Consistent rel q n) (hm : mhead m = .recv p ls) :
    ∀ (d : Nat) (x : R) (tl : List Nat), x.sdepth q ≤ d → rel m x → x.good →
      x.queue p q = l :: tl →
      (∃ x2, RecvStep q p l x2 x) ∧ (ls.lookup l).isSome = true := by
  intro d
  induction d using Nat.strong_induction_on with
  | _ d ih =>
    intro x tl hd hrel hg hq
    obtain ⟨hw, hreach, hnode⟩ := hc m x hrel
    have hr := hreach (by rw [hm]; rfl)
    have hhw := R.head_wf hw
    have hhr := R.head_reaches _ x n q rfl hw hr
    obtain ⟨_, hhg, hhq⟩ := head_facts hw hg
    cases hx : x.head with
    | fin => rw [hx] at hnode; rw [hnode] at hm; cases hm
    | var v => exact absurd hx (hhw.2.2 v)
    | mu b => exact absurd hx (hhw.2.1 b)
    | sent frm dst label cont =>
      rw [hx] at hnode hhw hhr hhg hhq
      dsimp only at hnode
      have hqh := hhq p q
      rw [hq] at hqh
      simp only [R.queue] at hqh
      by_cases h2 : q = dst
      · rw [if_pos h2] at hnode
        obtain ⟨ls', c', hm', hl', _⟩ := hnode
        rw [hm] at hm'
        cases hm'
        subst h2
        rw [if_pos ⟨rfl, rfl⟩] at hqh
        have hlab : label = l := by simpa using (List.cons.inj hqh).1
        subst hlab
        exact ⟨⟨cont, recvStep_here hx⟩, by rw [hl']; rfl⟩
      · rw [if_neg h2] at hnode
        rw [if_neg (fun e => h2 e.2.symm)] at hqh
        have hsd := sdepth_sent_other (frm := frm) (label := label) (cont := cont)
          (fun e => h2 e.symm)
        simp only [R.good] at hhg
        obtain ⟨⟨c2, hc2⟩, hacc⟩ := ih (d - 1) (by omega) cont tl (by omega) hnode hhg
          (by simpa using hqh)
        exact ⟨⟨.sent frm dst label c2, RecvStep.sent hx (fun e => h2 e.symm) hc2⟩, hacc⟩
    | comm frm dst bs =>
      rw [hx] at hnode hhw hhr hhg hhq
      have hp := R.comm_wf_parts hhw.1
      dsimp only at hnode
      simp only [R.good] at hhg
      obtain ⟨hns, hqa, hbg⟩ := hhg
      have hqh := hhq p q
      rw [hq] at hqh
      simp only [R.queue] at hqh
      by_cases h1 : q = frm
      · rw [if_pos h1] at hnode
        obtain ⟨ls', hm', _⟩ := hnode
        rw [hm] at hm'; cases hm'
      · rw [if_neg h1] at hnode
        by_cases h2 : q = dst
        · rw [if_pos h2] at hnode
          obtain ⟨ls', hm', _⟩ := hnode
          rw [hm] at hm'
          cases hm'
          rw [RBs.noSentFrom_queue bs p q hns] at hqh
          cases hqh
        · rw [if_neg h2] at hnode
          have hsd := sdepth_comm_other (bs := bs) (fun e => h1 e.symm) (fun e => h2 e.symm)
          have hbr : ∀ label b, bs.lookup label = some b →
              (∃ b2, RecvStep q p l b2 b) ∧ (ls.lookup l).isSome = true := by
            intro label b hb
            have := RBs.sdepth_branch bs q hb
            exact ih (d - 1) (by omega) b tl (by omega) (hnode label b hb)
              (RBs.good_branch bs hbg hb) (by rw [hqa label b p q hb]; exact hqh)
          obtain ⟨bs2, hbs2⟩ := recvStepBs_exists bs hp.2.2.2.2.1 (fun label b hb => (hbr label b hb).1)
          obtain ⟨lab1, g1, r1, rfl⟩ := RBs.len_pos_cons hp.2.2.2.1
          exact ⟨⟨.comm frm dst bs2, RecvStep.comm hx (fun e => h1 e.symm) (fun e => h2 e.symm) hbs2⟩,
            (hbr lab1 g1 RBs.lookup_first).2⟩

end MP
