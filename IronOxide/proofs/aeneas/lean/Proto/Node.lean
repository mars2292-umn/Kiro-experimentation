import Proto.Across
/-!
# The relation after a step is consistent

Port of `lemma_send_node`, `lemma_rel_send_consistent`, `lemma_recv_node`
and `lemma_rel_recv_consistent` of
`crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

theorem relSend_sub {rel : L → R → Prop} {s t l u : Nat} :
    ∀ a b, rel a b → RelSend rel s t l u a b := fun _ _ h => Or.inl h

theorem relRecv_sub {rel : L → R → Prop} {q p l u : Nat} :
    ∀ a b, rel a b → RelRecv rel q p l u a b := fun _ _ h => Or.inl h

theorem relSend_has {rel : L → R → Prop} {s t l u : Nat} {m : L} {x x2 : R} (h : rel m x)
    (hs : SendStep s t l x2 x) : RelSend rel s t l u (afterAct m u s l) x2 :=
  Or.inr ⟨m, x, h, hs, rfl⟩

theorem relRecv_has {rel : L → R → Prop} {q p l u : Nat} {m : L} {x x2 : R} (h : rel m x)
    (hs : RecvStep q p l x2 x) : RelRecv rel q p l u (afterAct m u q l) x2 :=
  Or.inr ⟨m, x, h, hs, rfl⟩

/-- The reaching of role `u` after a send, whoever `u` is. -/
theorem send_reaches_any {rel : L → R → Prop} {m : L} {x x2 : R} {s t l u n : Nat}
    (hc : Consistent rel u n) (hrel : rel m x) (hs : SendStep s t l x2 x)
    (hact : (mhead (afterAct m u s l)).acting = true) : x2.reaches u = true := by
  by_cases hu : u = s
  · subst hu
    exact sendStep_reaches_actor hc hact x2 x hrel hs
  · rw [afterAct_ne hu] at hact
    have hnode := hc m x hrel
    exact sendStep_reaches x2 x hnode.1 hs hu (hnode.2.1 hact)

theorem recv_reaches_any {rel : L → R → Prop} {m : L} {x x2 : R} {q p l u n : Nat}
    (hc : Consistent rel u n) (hrel : rel m x) (hs : RecvStep q p l x2 x)
    (hact : (mhead (afterAct m u q l)).acting = true) : x2.reaches u = true := by
  by_cases hu : u = q
  · subst hu
    exact recvStep_reaches_actor hc hact x2 x hrel hs
  · rw [afterAct_ne hu] at hact
    have hnode := hc m x hrel
    exact recvStep_reaches x2 x hnode.1 hs hu (hnode.2.1 hact)

/-- A stepped pair satisfies the conditions in the relation after a send. -/
theorem send_node {rel : L → R → Prop} {m : L} {x x2 : R} {s t l u n : Nat}
    (hc : Consistent rel u n) (hrel : rel m x) (hs : SendStep s t l x2 x) :
    NodeOk (RelSend rel s t l u) (afterAct m u s l) x2 u n := by
  have hnode := hc m x hrel
  have hw2 := (sendStep_wf x2 x hnode.1 hs).1
  have hhw := (R.head_wf hnode.1).1
  have hreach := send_reaches_any hc hrel hs
  rcases sendStep_cases hs with ⟨bs, b, hx, hl, rfl⟩ | ⟨frm, dst, bs, bs2, hx, h1, h2, rfl, hbs⟩ |
    ⟨frm, dst, label, cont, c2, hx, hd, rfl, hcs⟩
  · -- the send at the head
    rw [hx] at hhw
    have hp := R.comm_wf_parts hhw
    have hn := hnode.at_comm hx
    by_cases hu : u = s
    · subst hu
      rw [if_pos rfl] at hn
      obtain ⟨ls, hm, _, hpairs⟩ := hn
      obtain ⟨c, hcl, hcr⟩ := hpairs l b hl
      rw [afterAct_send hm hcl] at hreach ⊢
      refine NodeOk.mk_sent hw2 hreach ?_
      rw [if_neg hp.2.2.1]
      exact Or.inl hcr
    · rw [afterAct_ne hu] at hreach ⊢
      rw [if_neg hu] at hn
      by_cases hut : u = t
      · subst hut
        rw [if_pos rfl] at hn
        obtain ⟨ls, hm, hpairs⟩ := hn
        obtain ⟨c, hcl, hcr⟩ := hpairs l b hl
        refine NodeOk.mk_sent hw2 hreach ?_
        rw [if_pos rfl]
        exact ⟨ls, c, hm, hcl, Or.inl hcr⟩
      · rw [if_neg hut] at hn
        refine NodeOk.mk_sent hw2 hreach ?_
        rw [if_neg hut]
        exact Or.inl (hn l b hl)
  · -- a pending choice the sender is not part of
    rw [hx] at hhw
    have hn := hnode.at_comm hx
    have hlab := sendStepBs_labels bs2 bs hbs
    have hback : ∀ label b2, bs2.lookup label = some b2 →
        ∃ b, bs.lookup label = some b ∧ SendStep s t l b2 b := by
      intro label b2 hb2
      have hsome : (bs.lookup label).isSome = true := by rw [← hlab.2.2.1 label, hb2]; rfl
      obtain ⟨b, hb⟩ := Option.isSome_iff_exists.mp hsome
      obtain ⟨b2', hb2', hst⟩ := hlab.2.2.2 label b hb
      rw [hb2] at hb2'
      cases hb2'
      exact ⟨b, hb, hst⟩
    by_cases hu1 : u = frm
    · have hus : u ≠ s := fun e => h1 (hu1.symm.trans e)
      rw [afterAct_ne hus] at hreach ⊢
      rw [if_pos hu1] at hn
      obtain ⟨ls, hm, hle, hpairs⟩ := hn
      refine NodeOk.mk_comm hw2 hreach ?_
      rw [if_pos hu1]
      refine ⟨ls, hm, fun label => by rw [hle label, hlab.2.2.1 label], ?_⟩
      intro label b2 hb2
      obtain ⟨b, hb, hst⟩ := hback label b2 hb2
      obtain ⟨c, hcl, hcr⟩ := hpairs label b hb
      refine ⟨c, hcl, ?_⟩
      have := relSend_has (u := u) hcr hst
      rwa [afterAct_ne hus] at this
    · rw [if_neg hu1] at hn
      by_cases hu2 : u = dst
      · have hus : u ≠ s := fun e => h2 (hu2.symm.trans e)
        rw [afterAct_ne hus] at hreach ⊢
        rw [if_pos hu2] at hn
        obtain ⟨ls, hm, hpairs⟩ := hn
        refine NodeOk.mk_comm hw2 hreach ?_
        rw [if_neg hu1, if_pos hu2]
        refine ⟨ls, hm, ?_⟩
        intro label b2 hb2
        obtain ⟨b, hb, hst⟩ := hback label b2 hb2
        obtain ⟨c, hcl, hcr⟩ := hpairs label b hb
        refine ⟨c, hcl, ?_⟩
        have := relSend_has (u := u) hcr hst
        rwa [afterAct_ne hus] at this
      · rw [if_neg hu2] at hn
        refine NodeOk.mk_comm hw2 hreach ?_
        rw [if_neg hu1, if_neg hu2]
        intro label b2 hb2
        obtain ⟨b, hb, hst⟩ := hback label b2 hb2
        exact relSend_has (hn label b hb) hst
  · -- a message in transit above the sender's node
    have hn := hnode.at_sent hx
    by_cases hu2 : u = dst
    · have hus : u ≠ s := fun e => hd (hu2.symm.trans e)
      rw [afterAct_ne hus] at hreach ⊢
      rw [if_pos hu2] at hn
      obtain ⟨ls, c, hm, hcl, hcr⟩ := hn
      refine NodeOk.mk_sent hw2 hreach ?_
      rw [if_pos hu2]
      refine ⟨ls, c, hm, hcl, ?_⟩
      have := relSend_has (u := u) hcr hcs
      rwa [afterAct_ne hus] at this
    · rw [if_neg hu2] at hn
      refine NodeOk.mk_sent hw2 hreach ?_
      rw [if_neg hu2]
      exact relSend_has hn hcs

/-- The relation after a send is consistent. -/
theorem relSend_consistent {rel : L → R → Prop} {s t l u n : Nat} (hc : Consistent rel u n) :
    Consistent (RelSend rel s t l u) u n := by
  intro m2 x2 h
  rcases h with h | ⟨m, x, hrel, hs, rfl⟩
  · exact (hc m2 x2 h).mono relSend_sub
  · exact send_node hc hrel hs

/-- A stepped pair satisfies the conditions in the relation after a
receive. -/
theorem recv_node {rel : L → R → Prop} {m : L} {x x2 : R} {q p l u n : Nat}
    (hc : Consistent rel u n) (hrel : rel m x) (hs : RecvStep q p l x2 x) :
    NodeOk (RelRecv rel q p l u) (afterAct m u q l) x2 u n := by
  have hnode := hc m x hrel
  have hw2 := recvStep_wf x2 x hnode.1 hs
  have hhw := (R.head_wf hnode.1).1
  have hreach := recv_reaches_any hc hrel hs
  rcases recvStep_cases hs with ⟨cont, hx, rfl⟩ | ⟨frm, dst, label, cont, c2, hx, hd, rfl, hcs⟩ |
    ⟨frm, dst, bs, bs2, hx, h1, h2, rfl, hbs⟩
  · -- the receive at the head: the pair of the continuation is in the relation already
    have hn := hnode.at_sent hx
    by_cases hu : u = q
    · subst hu
      rw [if_pos rfl] at hn
      obtain ⟨ls, c, hm, hcl, hcr⟩ := hn
      rw [afterAct_recv hm hcl]
      exact (hc c x2 hcr).mono relRecv_sub
    · rw [afterAct_ne hu]
      rw [if_neg hu] at hn
      exact (hc m x2 hn).mono relRecv_sub
  · -- a message in transit above the receiver's node
    have hn := hnode.at_sent hx
    by_cases hu2 : u = dst
    · have huq : u ≠ q := fun e => hd (hu2.symm.trans e)
      rw [afterAct_ne huq] at hreach ⊢
      rw [if_pos hu2] at hn
      obtain ⟨ls, c, hm, hcl, hcr⟩ := hn
      refine NodeOk.mk_sent hw2 hreach ?_
      rw [if_pos hu2]
      refine ⟨ls, c, hm, hcl, ?_⟩
      have := relRecv_has (u := u) hcr hcs
      rwa [afterAct_ne huq] at this
    · rw [if_neg hu2] at hn
      refine NodeOk.mk_sent hw2 hreach ?_
      rw [if_neg hu2]
      exact relRecv_has hn hcs
  · -- a pending choice the receiver is not part of
    have hn := hnode.at_comm hx
    have hlab := recvStepBs_labels bs2 bs hbs
    have hback : ∀ label b2, bs2.lookup label = some b2 →
        ∃ b, bs.lookup label = some b ∧ RecvStep q p l b2 b := by
      intro label b2 hb2
      have hsome : (bs.lookup label).isSome = true := by rw [← hlab.2.2.1 label, hb2]; rfl
      obtain ⟨b, hb⟩ := Option.isSome_iff_exists.mp hsome
      obtain ⟨b2', hb2', hst⟩ := hlab.2.2.2 label b hb
      rw [hb2] at hb2'
      cases hb2'
      exact ⟨b, hb, hst⟩
    by_cases hu1 : u = frm
    · have huq : u ≠ q := fun e => h1 (hu1.symm.trans e)
      rw [afterAct_ne huq] at hreach ⊢
      rw [if_pos hu1] at hn
      obtain ⟨ls, hm, hle, hpairs⟩ := hn
      refine NodeOk.mk_comm hw2 hreach ?_
      rw [if_pos hu1]
      refine ⟨ls, hm, fun label => by rw [hle label, hlab.2.2.1 label], ?_⟩
      intro label b2 hb2
      obtain ⟨b, hb, hst⟩ := hback label b2 hb2
      obtain ⟨c, hcl, hcr⟩ := hpairs label b hb
      refine ⟨c, hcl, ?_⟩
      have := relRecv_has (u := u) hcr hst
      rwa [afterAct_ne huq] at this
    · rw [if_neg hu1] at hn
      by_cases hu2 : u = dst
      · have huq : u ≠ q := fun e => h2 (hu2.symm.trans e)
        rw [afterAct_ne huq] at hreach ⊢
        rw [if_pos hu2] at hn
        obtain ⟨ls, hm, hpairs⟩ := hn
        refine NodeOk.mk_comm hw2 hreach ?_
        rw [if_neg hu1, if_pos hu2]
        refine ⟨ls, hm, ?_⟩
        intro label b2 hb2
        obtain ⟨b, hb, hst⟩ := hback label b2 hb2
        obtain ⟨c, hcl, hcr⟩ := hpairs label b hb
        refine ⟨c, hcl, ?_⟩
        have := relRecv_has (u := u) hcr hst
        rwa [afterAct_ne huq] at this
      · rw [if_neg hu2] at hn
        refine NodeOk.mk_comm hw2 hreach ?_
        rw [if_neg hu1, if_neg hu2]
        intro label b2 hb2
        obtain ⟨b, hb, hst⟩ := hback label b2 hb2
        exact relRecv_has (hn label b hb) hst

/-- The relation after a receive is consistent. -/
theorem relRecv_consistent {rel : L → R → Prop} {q p l u n : Nat} (hc : Consistent rel u n) :
    Consistent (RelRecv rel q p l u) u n := by
  intro m2 x2 h
  rcases h with h | ⟨m, x, hrel, hs, rfl⟩
  · exact (hc m2 x2 h).mono relRecv_sub
  · exact recv_node hc hrel hs

end MP
