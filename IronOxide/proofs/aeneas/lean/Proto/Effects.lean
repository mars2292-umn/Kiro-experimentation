import Proto.Steps
/-!
# The effects of a step

A send keeps the structure, appends the label to its channel's queue,
leaves every other queue, and creates messages from the sender alone. A
receive keeps the structure, drops the oldest label of its channel's
queue, leaves every other queue, and creates no message. A step by another
role keeps the reaching of a role. Port of the section "the effects of a
step" of `crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

mutual
theorem R.noSentFrom_subst : ∀ (x : R) (y k : Nat) (by' : R), x.noSentFrom y = true →
    by'.noSentFrom y = true → (x.subst k by').noSentFrom y = true
  | .fin, _, _, _, _, _ => rfl
  | .var v, y, k, by', _, hb => by
    simp only [R.subst]
    split
    · exact hb
    · split <;> rfl
  | .mu body, y, k, by', h, hb => by
    simp only [R.noSentFrom] at h
    simp only [R.subst, R.noSentFrom]
    exact R.noSentFrom_subst body y (k + 1) by' h hb
  | .comm _ _ bs, y, k, by', h, hb => by
    simp only [R.noSentFrom] at h
    simp only [R.subst, R.noSentFrom]
    exact RBs.noSentFrom_subst bs y k by' h hb
  | .sent frm dst label cont, y, k, by', h, hb => by
    simp only [R.noSentFrom, Bool.and_eq_true] at h
    simp only [R.subst, R.noSentFrom, Bool.and_eq_true]
    exact ⟨h.1, R.noSentFrom_subst cont y k by' h.2 hb⟩
theorem RBs.noSentFrom_subst : ∀ (bs : RBs) (y k : Nat) (by' : R), bs.noSentFrom y = true →
    by'.noSentFrom y = true → (bs.subst k by').noSentFrom y = true
  | .nil, _, _, _, _, _ => rfl
  | .cons _ g rest, y, k, by', h, hb => by
    simp only [RBs.noSentFrom, Bool.and_eq_true] at h
    simp only [RBs.subst, RBs.noSentFrom, Bool.and_eq_true]
    exact ⟨R.noSentFrom_subst g y k by' h.1 hb, RBs.noSentFrom_subst rest y k by' h.2 hb⟩
end

/-- Head normalization keeps the absence of messages from a role. -/
theorem R.noSentFrom_head : ∀ (k : Nat) (x : R) (y n : Nat), x.leadRecs = k → x.wf n = true →
    x.noSentFrom y = true → x.head.noSentFrom y = true
  | 0, x, y, n, hk, _, h => by
    have : x.head = x := by
      apply R.head_flat
      intro b e; subst e; simp [R.leadRecs] at hk
    rw [this]; exact h
  | k + 1, x, y, n, hk, hw, h => by
    cases x with
    | mu body =>
      have hu := R.unfold_wf hw
      have hkk : (R.mu body).unfold.leadRecs = k := by have := hu.2; omega
      have hsub : (R.mu body).unfold.noSentFrom y = true := by
        simp only [R.unfold]
        exact R.noSentFrom_subst body y 0 (.mu body) (by simpa [R.noSentFrom] using h) h
      have hhead : (R.mu body).head = (R.mu body).unfold.head := by
        rw [R.head_mu]
        simp only [R.head]
        have : body.leadRecs = k := by simp [R.leadRecs] at hk; omega
        rw [this, hkk]
      rw [hhead]
      exact R.noSentFrom_head k _ y n hkk hu.1 hsub
    | fin => simp [R.leadRecs] at hk
    | var v => simp [R.leadRecs] at hk
    | comm f t bs => simp [R.leadRecs] at hk
    | sent f t lab c => simp [R.leadRecs] at hk

theorem head_facts {x : R} {n : Nat} (hw : x.wf n = true) (hg : x.good) :
    x.head.wf n = true ∧ x.head.good ∧ ∀ p q, x.head.queue p q = x.queue p q :=
  ⟨(R.head_wf hw).1, (R.head_good _ x n rfl hg hw).1, (R.head_good _ x n rfl hg hw).2⟩

/-- The queues after a send of `l` from `s` to `t`. -/
def SendQ (s t l : Nat) (b2 b : R) : Prop :=
  b2.queue s t = b.queue s t ++ [l] ∧ ∀ p q, ¬ (p = s ∧ q = t) → b2.queue p q = b.queue p q

/-- The queues after a receive of `l` by `q` from `p`. -/
def RecvQ (q p l : Nat) (b2 b : R) : Prop :=
  (∃ tl, b.queue p q = l :: tl ∧ b2.queue p q = tl) ∧
    ∀ a c, ¬ (a = p ∧ c = q) → b2.queue a c = b.queue a c

theorem RBs.lookup_first {label : Nat} {g : R} {rest : RBs} :
    (RBs.cons label g rest).lookup label = some g := by simp [RBs.lookup]

theorem RBs.len_pos_cons {bs : RBs} (h : 1 ≤ bs.len) : ∃ label g rest, bs = .cons label g rest := by
  cases bs with
  | nil => simp [RBs.len] at h
  | cons label g rest => exact ⟨label, g, rest, rfl⟩

mutual
theorem sendStep_good {s t l n : Nat} : ∀ (x2 x : R), x.wf n = true → x.good →
    SendStep s t l x2 x →
    x2.good ∧ SendQ s t l x2 x ∧ ∀ y, y ≠ s → x.noSentFrom y = true → x2.noSentFrom y = true
  | .sent f2 d2 lab2 c2, x, hw, hg, h => by
    obtain ⟨hhw, hhg, hhq⟩ := head_facts hw hg
    have hnsf := fun y => R.noSentFrom_head _ x y n rfl hw
    rcases sendStep_cases h with ⟨bs, b, hx, hl, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨frm, dst, label, cont, c2', hx, hd, e, hs⟩
    · cases e
      rw [hx] at hhw hhg hhq hnsf
      simp only [R.good] at hhg
      obtain ⟨hns, hqa, hbg⟩ := hhg
      have hbns := RBs.noSentFrom_branch bs s hns hl
      refine ⟨by simpa [R.good] using RBs.good_branch bs hbg hl, ⟨?_, ?_⟩, ?_⟩
      · rw [← hhq s t]
        simp only [R.queue, R.noSentFrom_queue c2 s t hbns, RBs.noSentFrom_queue bs s t hns]
        simp
      · intro p q hpq
        rw [← hhq p q]
        simp only [R.queue]
        rw [if_neg (by intro e; exact hpq ⟨e.1.symm, e.2.symm⟩), hqa l c2 p q hl]
        rfl
      · intro y hy hx'
        have := hnsf y hx'
        simp only [R.noSentFrom] at this ⊢
        simp only [Bool.and_eq_true, decide_eq_true_eq]
        exact ⟨fun e => hy e.symm, RBs.noSentFrom_branch bs y this hl⟩
    · cases e
    · cases e
      rw [hx] at hhw hhg hhq hnsf
      have hp := R.sent_wf_parts hhw
      simp only [R.good] at hhg
      obtain ⟨ig, ⟨iq1, iq2⟩, ins⟩ := sendStep_good c2 cont hp.2.2.2 hhg hs
      refine ⟨by simpa [R.good] using ig, ⟨?_, ?_⟩, ?_⟩
      · rw [← hhq s t]
        simp only [R.queue, iq1, List.append_assoc]
      · intro p q hpq
        rw [← hhq p q]
        simp only [R.queue, iq2 p q hpq]
      · intro y hy hx'
        have := hnsf y hx'
        simp only [R.noSentFrom, Bool.and_eq_true] at this ⊢
        exact ⟨this.1, ins y hy this.2⟩
  | .comm f2 d2 bs2, x, hw, hg, h => by
    obtain ⟨hhw, hhg, hhq⟩ := head_facts hw hg
    have hnsf := fun y => R.noSentFrom_head _ x y n rfl hw
    rcases sendStep_cases h with ⟨_, _, _, _, e⟩ | ⟨frm, dst, bs, bs2', hx, h1, h2, e, hs⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · cases e
    · cases e
      rw [hx] at hhw hhg hhq hnsf
      have hp := R.comm_wf_parts hhw
      simp only [R.good] at hhg
      obtain ⟨hns, hqa, hbg⟩ := hhg
      obtain ⟨ig, ifirst, ipt, ins⟩ := sendStepBs_good bs2 bs hp.2.2.2.2.2 hbg hs
      have hlab := sendStepBs_labels bs2 bs hs
      obtain ⟨lab1, g1, r1, rfl⟩ := RBs.len_pos_cons hp.2.2.2.1
      have hfirst := ifirst (by simp)
      refine ⟨?_, ⟨?_, ?_⟩, ?_⟩
      · simp only [R.good]
        refine ⟨ins f2 h1 hns, ?_, ig⟩
        intro label g2 p q hg2
        have hsome : ((RBs.cons lab1 g1 r1).lookup label).isSome = true := by
          rw [← hlab.2.2.1 label, hg2]; rfl
        obtain ⟨b, hb⟩ := Option.isSome_iff_exists.mp hsome
        obtain ⟨b2, hb2, hq⟩ := ipt label b hb
        rw [hg2] at hb2
        cases hb2
        by_cases hpq : p = s ∧ q = t
        · obtain ⟨rfl, rfl⟩ := hpq
          rw [hq.1, hqa label b p q hb, hfirst.1]
        · rw [hq.2 p q hpq, hqa label b p q hb, hfirst.2 p q hpq]
      · rw [← hhq s t]
        simp only [R.queue]
        exact hfirst.1
      · intro p q hpq
        rw [← hhq p q]
        simp only [R.queue]
        exact hfirst.2 p q hpq
      · intro y hy hx'
        have := hnsf y hx'
        simp only [R.noSentFrom] at this ⊢
        exact ins y hy this
    · cases e
  | .fin, _, _, _, h => by simp [SendStep] at h
  | .mu _, _, _, _, h => by simp [SendStep] at h
  | .var _, _, _, _, h => by simp [SendStep] at h
/-- Branchwise: the stepped branches are good, with the sender's queue
extended and the others kept, and no new message from anyone else. -/
theorem sendStepBs_good {s t l n : Nat} : ∀ (bs2 bs : RBs), bs.wfBs n [] = true → bs.good →
    SendStepBs s t l bs2 bs →
    bs2.good ∧
    (bs ≠ .nil → bs2.queue s t = bs.queue s t ++ [l] ∧
      ∀ p q, ¬ (p = s ∧ q = t) → bs2.queue p q = bs.queue p q) ∧
    (∀ label b, bs.lookup label = some b → ∃ b2, bs2.lookup label = some b2 ∧ SendQ s t l b2 b) ∧
    (∀ y, y ≠ s → bs.noSentFrom y = true → bs2.noSentFrom y = true)
  | .nil, bs, _, _, h => by
    simp only [SendStepBs] at h
    subst h
    exact ⟨trivial, fun hne => absurd rfl hne, fun label b hb => by simp [RBs.lookup] at hb,
      fun _ _ _ => rfl⟩
  | .cons label2 g2 rest2, bs, hw, hg, h => by
    cases bs with
    | nil => simp [SendStepBs] at h
    | cons label g rest =>
      simp only [SendStepBs] at h
      obtain ⟨rfl, h1, h2⟩ := h
      simp only [RBs.wfBs, Bool.and_eq_true] at hw
      simp only [RBs.good] at hg
      obtain ⟨ig, iq, ins⟩ := sendStep_good g2 g hw.1 hg.1 h1
      obtain ⟨jg, _, jpt, jns⟩ := sendStepBs_good rest2 rest hw.2 hg.2 h2
      refine ⟨⟨ig, jg⟩, fun _ => ?_, ?_, ?_⟩
      · simp only [RBs.queue]
        exact iq
      · intro lab b hb
        simp only [RBs.lookup] at hb ⊢
        split at hb
        · cases hb; rename_i e; rw [if_pos e]; exact ⟨g2, rfl, iq⟩
        · rename_i e; rw [if_neg e]; exact jpt lab b hb
      · intro y hy hns
        simp only [RBs.noSentFrom, Bool.and_eq_true] at hns ⊢
        exact ⟨ins y hy hns.1, jns y hy hns.2⟩
end

/-- The receive at the head: the message in transit is consumed. -/
theorem recv_here {q p l n : Nat} {x x2 cont : R} (_hhw : x.head.wf n = true) (hhg : x.head.good)
    (hhq : ∀ a c, x.head.queue a c = x.queue a c)
    (hnsf : ∀ y, x.noSentFrom y = true → x.head.noSentFrom y = true)
    (hx : x.head = .sent p q l cont) (e : x2 = cont) :
    x2.good ∧ RecvQ q p l x2 x ∧ ∀ y, x.noSentFrom y = true → x2.noSentFrom y = true := by
  subst e
  rw [hx] at hhg hhq hnsf
  refine ⟨by simpa [R.good] using hhg, ⟨⟨x2.queue p q, ?_, rfl⟩, ?_⟩, ?_⟩
  · rw [← hhq p q]
    simp [R.queue]
  · intro a c hac
    rw [← hhq a c]
    simp only [R.queue]
    rw [if_neg (fun e => hac ⟨e.1.symm, e.2.symm⟩)]
    rfl
  · intro y hy
    have := hnsf y hy
    simp only [R.noSentFrom, Bool.and_eq_true] at this
    exact this.2

mutual
theorem recvStep_good {q p l n : Nat} : ∀ (x2 x : R), x.wf n = true → x.good →
    RecvStep q p l x2 x →
    x2.good ∧ RecvQ q p l x2 x ∧ ∀ y, x.noSentFrom y = true → x2.noSentFrom y = true
  | .sent f2 d2 lab2 c2, x, hw, hg, h => by
    obtain ⟨hhw, hhg, hhq⟩ := head_facts hw hg
    have hnsf := fun y => R.noSentFrom_head _ x y n rfl hw
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨frm, dst, label, cont, c2', hx, hd, e, hs⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · exact recv_here hhw hhg hhq hnsf hx e
    · cases e
      rw [hx] at hhw hhg hhq hnsf
      have hp := R.sent_wf_parts hhw
      simp only [R.good] at hhg
      obtain ⟨ig, ⟨⟨tl, iq1, iq1'⟩, iq2⟩, ins⟩ := recvStep_good c2 cont hp.2.2.2 hhg hs
      refine ⟨by simpa [R.good] using ig, ⟨⟨tl, ?_, ?_⟩, ?_⟩, ?_⟩
      · rw [← hhq p q]
        simp only [R.queue, iq1]
        rw [if_neg (fun e => hd e.2)]
        rfl
      · simp only [R.queue, iq1']
        rw [if_neg (fun e => hd e.2)]
        rfl
      · intro a c hac
        rw [← hhq a c]
        simp only [R.queue, iq2 a c hac]
      · intro y hx'
        have := hnsf y hx'
        simp only [R.noSentFrom, Bool.and_eq_true] at this ⊢
        exact ⟨this.1, ins y this.2⟩
    · cases e
  | .comm f2 d2 bs2, x, hw, hg, h => by
    obtain ⟨hhw, hhg, hhq⟩ := head_facts hw hg
    have hnsf := fun y => R.noSentFrom_head _ x y n rfl hw
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨frm, dst, bs, bs2', hx, h1, h2, e, hs⟩
    · exact recv_here hhw hhg hhq hnsf hx e
    · cases e
    · cases e
      rw [hx] at hhw hhg hhq hnsf
      have hp := R.comm_wf_parts hhw
      simp only [R.good] at hhg
      obtain ⟨hns, hqa, hbg⟩ := hhg
      obtain ⟨ig, ifirst, ipt, ins⟩ := recvStepBs_good bs2 bs hp.2.2.2.2.2 hbg hs
      have hlab := recvStepBs_labels bs2 bs hs
      obtain ⟨lab1, g1, r1, rfl⟩ := RBs.len_pos_cons hp.2.2.2.1
      obtain ⟨⟨tl, hf1, hf2⟩, hf3⟩ := ifirst (by simp)
      refine ⟨?_, ⟨⟨tl, ?_, ?_⟩, ?_⟩, ?_⟩
      · simp only [R.good]
        refine ⟨ins f2 hns, ?_, ig⟩
        intro label g2 a c hg2
        have hsome : ((RBs.cons lab1 g1 r1).lookup label).isSome = true := by
          rw [← hlab.2.2.1 label, hg2]; rfl
        obtain ⟨b, hb⟩ := Option.isSome_iff_exists.mp hsome
        obtain ⟨b2, hb2, ⟨tl', hq1, hq2⟩, hq3⟩ := ipt label b hb
        rw [hg2] at hb2
        cases hb2
        by_cases hac : a = p ∧ c = q
        · obtain ⟨rfl, rfl⟩ := hac
          rw [hq2, hf2]
          have := hqa label b a c hb
          rw [hq1, hf1] at this
          exact (List.cons.inj this).2
        · rw [hq3 a c hac, hqa label b a c hb, hf3 a c hac]
      · rw [← hhq p q]
        simp only [R.queue]
        exact hf1
      · simp only [R.queue]
        exact hf2
      · intro a c hac
        rw [← hhq a c]
        simp only [R.queue]
        exact hf3 a c hac
      · intro y hx'
        have := hnsf y hx'
        simp only [R.noSentFrom] at this ⊢
        exact ins y this
  | .fin, x, hw, hg, h => by
    obtain ⟨hhw, hhg, hhq⟩ := head_facts hw hg
    have hnsf := fun y => R.noSentFrom_head _ x y n rfl hw
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · exact recv_here hhw hhg hhq hnsf hx e
    · cases e
    · cases e
  | .mu b2, x, hw, hg, h => by
    obtain ⟨hhw, hhg, hhq⟩ := head_facts hw hg
    have hnsf := fun y => R.noSentFrom_head _ x y n rfl hw
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · exact recv_here hhw hhg hhq hnsf hx e
    · cases e
    · cases e
  | .var v2, x, hw, hg, h => by
    obtain ⟨hhw, hhg, hhq⟩ := head_facts hw hg
    have hnsf := fun y => R.noSentFrom_head _ x y n rfl hw
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · exact recv_here hhw hhg hhq hnsf hx e
    · cases e
    · cases e
theorem recvStepBs_good {q p l n : Nat} : ∀ (bs2 bs : RBs), bs.wfBs n [] = true → bs.good →
    RecvStepBs q p l bs2 bs →
    bs2.good ∧
    (bs ≠ .nil → (∃ tl, bs.queue p q = l :: tl ∧ bs2.queue p q = tl) ∧
      ∀ a c, ¬ (a = p ∧ c = q) → bs2.queue a c = bs.queue a c) ∧
    (∀ label b, bs.lookup label = some b → ∃ b2, bs2.lookup label = some b2 ∧ RecvQ q p l b2 b) ∧
    (∀ y, bs.noSentFrom y = true → bs2.noSentFrom y = true)
  | .nil, bs, _, _, h => by
    simp only [RecvStepBs] at h
    subst h
    exact ⟨trivial, fun hne => absurd rfl hne, fun label b hb => by simp [RBs.lookup] at hb,
      fun _ _ => rfl⟩
  | .cons label2 g2 rest2, bs, hw, hg, h => by
    cases bs with
    | nil => simp [RecvStepBs] at h
    | cons label g rest =>
      simp only [RecvStepBs] at h
      obtain ⟨rfl, h1, h2⟩ := h
      simp only [RBs.wfBs, Bool.and_eq_true] at hw
      simp only [RBs.good] at hg
      obtain ⟨ig, iq, ins⟩ := recvStep_good g2 g hw.1 hg.1 h1
      obtain ⟨jg, _, jpt, jns⟩ := recvStepBs_good rest2 rest hw.2 hg.2 h2
      refine ⟨⟨ig, jg⟩, fun _ => ?_, ?_, ?_⟩
      · simp only [RBs.queue]
        exact iq
      · intro lab b hb
        simp only [RBs.lookup] at hb ⊢
        split at hb
        · cases hb; rename_i e; rw [if_pos e]; exact ⟨g2, rfl, iq⟩
        · rename_i e; rw [if_neg e]; exact jpt lab b hb
      · intro y hns
        simp only [RBs.noSentFrom, Bool.and_eq_true] at hns ⊢
        exact ⟨ins y hns.1, jns y hns.2⟩
end

/-! ## A step by another role keeps the reaching -/

theorem sendStepBs_nil_right {s t l : Nat} {bs2 : RBs} (h : SendStepBs s t l bs2 .nil) :
    bs2 = .nil := by
  cases bs2 with
  | nil => rfl
  | cons _ _ _ => simp [SendStepBs] at h

theorem recvStepBs_nil_right {q p l : Nat} {bs2 : RBs} (h : RecvStepBs q p l bs2 .nil) :
    bs2 = .nil := by
  cases bs2 with
  | nil => rfl
  | cons _ _ _ => simp [RecvStepBs] at h

mutual
theorem sendStep_reaches {s t l n u : Nat} : ∀ (x2 x : R), x.wf n = true →
    SendStep s t l x2 x → u ≠ s → x.reaches u = true → x2.reaches u = true
  | .sent f2 d2 lab2 c2, x, hw, h, hu, hr => by
    have hhw := (R.head_wf hw).1
    have hhr := (R.head_reaches _ x n u rfl hw hr).1
    rcases sendStep_cases h with ⟨bs, b, hx, hl, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨frm, dst, label, cont, c2', hx, hd, e, hs⟩
    · cases e
      rw [hx] at hhr
      simp only [R.reaches, Bool.or_eq_true, decide_eq_true_eq] at hhr ⊢
      rcases hhr with (h1 | h1) | h1
      · exact absurd h1.symm hu
      · exact Or.inl h1
      · exact Or.inr (RBs.reaches_branch bs u h1 hl)
    · cases e
    · cases e
      rw [hx] at hhr hhw
      simp only [R.reaches, Bool.or_eq_true, decide_eq_true_eq] at hhr ⊢
      rcases hhr with h1 | h1
      · exact Or.inl h1
      · exact Or.inr (sendStep_reaches c2 cont (R.sent_wf_parts hhw).2.2.2 hs hu h1)
  | .comm f2 d2 bs2, x, hw, h, hu, hr => by
    have hhw := (R.head_wf hw).1
    have hhr := (R.head_reaches _ x n u rfl hw hr).1
    rcases sendStep_cases h with ⟨_, _, _, _, e⟩ | ⟨frm, dst, bs, bs2', hx, h1, h2, e, hs⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · cases e
    · cases e
      rw [hx] at hhr hhw
      simp only [R.reaches, Bool.or_eq_true, decide_eq_true_eq] at hhr ⊢
      rcases hhr with h3 | h3
      · exact Or.inl h3
      · exact Or.inr (sendStepBs_reaches bs2 bs (R.comm_wf_parts hhw).2.2.2.2.2 hs hu h3)
    · cases e
  | .fin, _, _, h, _, _ => by simp [SendStep] at h
  | .mu _, _, _, h, _, _ => by simp [SendStep] at h
  | .var _, _, _, h, _, _ => by simp [SendStep] at h
theorem sendStepBs_reaches {s t l n u : Nat} : ∀ (bs2 bs : RBs), bs.wfBs n [] = true →
    SendStepBs s t l bs2 bs → u ≠ s → bs.reaches u = true → bs2.reaches u = true
  | .nil, bs, _, h, _, hr => by
    simp only [SendStepBs] at h
    subst h
    simp [RBs.reaches] at hr
  | .cons label2 g2 rest2, bs, hw, h, hu, hr => by
    cases bs with
    | nil => simp [SendStepBs] at h
    | cons label g rest =>
      simp only [SendStepBs] at h
      obtain ⟨rfl, h1, h2⟩ := h
      simp only [RBs.wfBs, Bool.and_eq_true] at hw
      have hc := RBs.reaches_cons hr
      apply RBs.reaches_cons_mk (sendStep_reaches g2 g hw.1 h1 hu hc.1)
      rcases hc.2 with rfl | h3
      · exact Or.inl (sendStepBs_nil_right h2)
      · exact Or.inr (sendStepBs_reaches rest2 rest hw.2 h2 hu h3)
end

theorem recv_here_reaches {q p l u : Nat} {x x2 cont : R} (hhr : x.head.reaches u = true)
    (hu : u ≠ q) (hx : x.head = .sent p q l cont) (e : x2 = cont) : x2.reaches u = true := by
  subst e
  rw [hx] at hhr
  simp only [R.reaches, Bool.or_eq_true, decide_eq_true_eq] at hhr
  rcases hhr with h1 | h1
  · exact absurd h1.symm hu
  · exact h1

mutual
theorem recvStep_reaches {q p l n u : Nat} : ∀ (x2 x : R), x.wf n = true →
    RecvStep q p l x2 x → u ≠ q → x.reaches u = true → x2.reaches u = true
  | .sent f2 d2 lab2 c2, x, hw, h, hu, hr => by
    have hhw := (R.head_wf hw).1
    have hhr := (R.head_reaches _ x n u rfl hw hr).1
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨frm, dst, label, cont, c2', hx, hd, e, hs⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · exact recv_here_reaches hhr hu hx e
    · cases e
      rw [hx] at hhr hhw
      simp only [R.reaches, Bool.or_eq_true, decide_eq_true_eq] at hhr ⊢
      rcases hhr with h1 | h1
      · exact Or.inl h1
      · exact Or.inr (recvStep_reaches c2 cont (R.sent_wf_parts hhw).2.2.2 hs hu h1)
    · cases e
  | .comm f2 d2 bs2, x, hw, h, hu, hr => by
    have hhw := (R.head_wf hw).1
    have hhr := (R.head_reaches _ x n u rfl hw hr).1
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨frm, dst, bs, bs2', hx, h1, h2, e, hs⟩
    · exact recv_here_reaches hhr hu hx e
    · cases e
    · cases e
      rw [hx] at hhr hhw
      simp only [R.reaches, Bool.or_eq_true, decide_eq_true_eq] at hhr ⊢
      rcases hhr with h3 | h3
      · exact Or.inl h3
      · exact Or.inr (recvStepBs_reaches bs2 bs (R.comm_wf_parts hhw).2.2.2.2.2 hs hu h3)
  | .fin, x, hw, h, hu, hr => by
    have hhr := (R.head_reaches _ x n u rfl hw hr).1
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · exact recv_here_reaches hhr hu hx e
    · cases e
    · cases e
  | .mu b2, x, hw, h, hu, hr => by
    have hhr := (R.head_reaches _ x n u rfl hw hr).1
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · exact recv_here_reaches hhr hu hx e
    · cases e
    · cases e
  | .var v2, x, hw, h, hu, hr => by
    have hhr := (R.head_reaches _ x n u rfl hw hr).1
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · exact recv_here_reaches hhr hu hx e
    · cases e
    · cases e
theorem recvStepBs_reaches {q p l n u : Nat} : ∀ (bs2 bs : RBs), bs.wfBs n [] = true →
    RecvStepBs q p l bs2 bs → u ≠ q → bs.reaches u = true → bs2.reaches u = true
  | .nil, bs, _, h, _, hr => by
    simp only [RecvStepBs] at h
    subst h
    simp [RBs.reaches] at hr
  | .cons label2 g2 rest2, bs, hw, h, hu, hr => by
    cases bs with
    | nil => simp [RecvStepBs] at h
    | cons label g rest =>
      simp only [RecvStepBs] at h
      obtain ⟨rfl, h1, h2⟩ := h
      simp only [RBs.wfBs, Bool.and_eq_true] at hw
      have hc := RBs.reaches_cons hr
      apply RBs.reaches_cons_mk (recvStep_reaches g2 g hw.1 h1 hu hc.1)
      rcases hc.2 with rfl | h3
      · exact Or.inl (recvStepBs_nil_right h2)
      · exact Or.inr (recvStepBs_reaches rest2 rest hw.2 h2 hu h3)
end

end MP
