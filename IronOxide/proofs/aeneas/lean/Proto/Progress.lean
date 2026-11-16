import Proto.Invariant
/-!
# No waiting set: the multiparty progress theorem

Under the invariant no waiting set exists: an awaiting member of a waiting
set is mentioned in the runtime type, hence has a first pending node along
the first branches at some depth; the role at the head of the runtime type
can move, and it is not a member, so the configuration after its move has
the same waiting set and a shallower node for the member; by induction on
the depth no such set exists. Port of the section "no waiting set" of
`crates/rsk-protocol/src/multiparty.rs`, and the theorem
`theorem_multiparty_progress`.
-/

namespace MP

/-- A role's awaiting and finishing depend on its type and on the channels
to it. -/
theorem status_same {c c2 : NC} {n u : Nat} (hl : c2.l u = c.l u)
    (hch : ∀ a, c2.ch a u = c.ch a u) :
    (NBlocked c2 n u ↔ NBlocked c n u) ∧ (NTerminal c2 u ↔ NTerminal c u) := by
  simp [NBlocked, NTerminal, hl, hch]

theorem not_blocked_of_send {c : NC} {n u t : Nat} {ls : Ls} (h : (c.l u).head = .send t ls) :
    ¬ NBlocked c n u ∧ ¬ NTerminal c u := by
  constructor
  · rintro ⟨frm, ls', h', _⟩
    rw [h] at h'; cases h'
  · intro h'
    unfold NTerminal at h'
    rw [h] at h'; cases h'

/-- The sender of a pending communication at the head has that send as its
head. -/
theorem sender_head {c : NC} {x : R} {n u dst : Nat} {bs : RBs} (h : MInv c x n) (hu : u < n)
    (hx : x.head = .comm u dst bs) :
    ∃ ls, (c.l u).head = .send dst ls ∧ LabelsEq ls bs := by
  obtain ⟨rel, hc, hrel⟩ := h.2.2.2.1 u hu
  have := (hc _ _ hrel).at_comm hx
  rw [if_pos rfl] at this
  obtain ⟨ls, hm, hle, _⟩ := this
  exact ⟨ls, by rw [head_of_mhead (Or.inl (by rw [hm]; rfl)), hm], hle⟩

/-- The receiver of a pending communication at the head has a receive from
its sender with every label of it. -/
theorem receiver_head {c : NC} {x : R} {n u frm : Nat} {bs : RBs} (h : MInv c x n) (hu : u < n)
    (hne : u ≠ frm) (hx : x.head = .comm frm u bs) :
    ∃ ls, (c.l u).head = .recv frm ls ∧
      ∀ label b, bs.lookup label = some b → ∃ cc, ls.lookup label = some cc := by
  obtain ⟨rel, hc, hrel⟩ := h.2.2.2.1 u hu
  have := (hc _ _ hrel).at_comm hx
  rw [if_neg hne, if_pos rfl] at this
  obtain ⟨ls, hm, hpairs⟩ := this
  refine ⟨ls, by rw [head_of_mhead (Or.inl (by rw [hm]; rfl)), hm], ?_⟩
  intro label b hb
  obtain ⟨cc, hcc, _⟩ := hpairs label b hb
  exact ⟨cc, hcc⟩

/-- The receiver of a message in transit at the head can receive it. -/
theorem transit_receiver {c : NC} {x : R} {n u frm label : Nat} {cont : R} (h : MInv c x n)
    (hu : u < n) (hx : x.head = .sent frm u label cont) :
    frm < n ∧ ∃ ls cc, (c.l u).head = .recv frm ls ∧ ls.lookup label = some cc ∧
      c.ch frm u = label :: cont.queue frm u := by
  obtain ⟨rel, hc, hrel⟩ := h.2.2.2.1 u hu
  have := (hc _ _ hrel).at_sent hx
  rw [if_pos rfl] at this
  obtain ⟨ls, cc, hm, hcc, _⟩ := this
  obtain ⟨hhw, _, hhq⟩ := head_facts h.2.1 h.2.2.1
  rw [hx] at hhw hhq
  have hp := R.sent_wf_parts hhw
  refine ⟨hp.1, ls, cc, by rw [head_of_mhead (Or.inl (by rw [hm]; rfl)), hm], hcc, ?_⟩
  rw [h.2.2.2.2 frm u hp.1 hu, ← hhq frm u]
  simp [R.queue]

theorem not_blocked_of_transit {c : NC} {n u frm label : Nat} {ls : Ls} {cc : L} {tl : List Nat}
    (hfrm : frm < n) (hh : (c.l u).head = .recv frm ls) (hl : ls.lookup label = some cc)
    (hq : c.ch frm u = label :: tl) : ¬ NBlocked c n u ∧ ¬ NTerminal c u := by
  constructor
  · rintro ⟨frm', ls', h', hneg⟩
    rw [hh] at h'
    cases h'
    exact hneg ⟨hfrm, label, tl, hq, by rw [hl]; rfl⟩
  · intro h'
    unfold NTerminal at h'
    rw [hh] at h'; cases h'

theorem fdepth_comm_other {frm dst u : Nat} {bs : RBs} (h1 : frm ≠ u) (h2 : dst ≠ u) :
    (R.comm frm dst bs).fdepth u = 1 + bs.fdepth u := by
  simp only [R.fdepth]
  rw [if_neg (by rintro (e | e) <;> [exact h1 e; exact h2 e])]

theorem fdepth_sent_other {frm dst label u : Nat} {cont : R} (h : dst ≠ u) :
    (R.sent frm dst label cont).fdepth u = 1 + cont.fdepth u := by
  simp only [R.fdepth]
  rw [if_neg h]

/-- An awaiting member of a waiting set at first-path depth `d` of the
runtime type: impossible, by induction on `d`. -/
theorem blocked_depth {n : Nat} {set : Nat → Bool} : ∀ (d : Nat) (c : NC) (x : R) (r : Nat),
    MInv c x n → NDeadlockSet c n set → r < n → set r = true → NBlocked c n r →
    x.fdepth r ≤ d → False := by
  intro d
  induction d using Nat.strong_induction_on with
  | _ d ih =>
    intro c x r hinv hdl hr hset hblk hd
    obtain ⟨hn, hw, hg, hfits, hch⟩ := hinv
    obtain ⟨hd1, hd2, hd3⟩ := hdl
    obtain ⟨p0, ls0, hh0, hneg0⟩ := hblk
    have hblk : NBlocked c n r := ⟨p0, ls0, hh0, hneg0⟩
    have hm0 : mhead (c.l r) = .recv p0 ls0 := by
      rw [mhead_of_head (Or.inl (by rw [hh0]; rfl)), hh0]
    obtain ⟨rel, hc, hrel⟩ := hfits r hr
    have hnode := hc _ _ hrel
    have hreach := hnode.2.1 (by rw [hm0]; rfl)
    have hhw := R.head_wf hw
    have hhr := R.head_reaches _ x n r rfl hw hreach
    obtain ⟨_, hhg, hhq⟩ := head_facts hw hg
    have hinv : MInv c x n := ⟨hn, hw, hg, hfits, hch⟩
    have notmem : ∀ u, u < n → ¬ NBlocked c n u → ¬ NTerminal c u → set u = false := by
      intro u hu h1 h2
      cases hsu : set u with
      | false => rfl
      | true => rcases hd2 u hu hsu with h | h <;> [exact absurd h h1; exact absurd h h2]
    cases hx : x.head with
    | fin =>
      have := hnode.2.2
      rw [hx] at this
      rw [hm0] at this
      cases this
    | var v => exact absurd hx (hhw.2.2 v)
    | mu b => exact absurd hx (hhw.2.1 b)
    | comm frm dst bs =>
      rw [hx] at hhw hhr hhg hhq
      have hp := R.comm_wf_parts hhw.1
      simp only [R.good] at hhg
      obtain ⟨hns, hqa, hbg⟩ := hhg
      by_cases h1 : frm = r
      · -- the awaiting role is the sender at the head
        subst h1
        obtain ⟨ls, hs, _⟩ := sender_head hinv hr hx
        exact (not_blocked_of_send (n := n) hs).1 hblk
      · by_cases h2 : dst = r
        · -- it is the receiver: its sender is a member, and can send
          subst h2
          obtain ⟨ls, hrh, _⟩ := receiver_head hinv hr (fun e => h1 e.symm) hx
          rw [hh0] at hrh
          cases hrh
          have hsf := hd3 dst p0 ls0 hr hset hh0 hblk
          obtain ⟨ls', hs, _⟩ := sender_head hinv hp.1 hx
          have := notmem p0 hp.1 (not_blocked_of_send (n := n) hs).1 (not_blocked_of_send (n := n) hs).2
          rw [hsf] at this
          cases this
        · -- neither: the sender at the head sends its first label, the receiver takes it
          obtain ⟨lsP, hsP, hleP⟩ := sender_head hinv hp.1 hx
          obtain ⟨lsQ, hrQ, hsupQ⟩ := receiver_head hinv hp.2.1 (fun e => hp.2.2.1 e.symm) hx
          have hsetP := notmem frm hp.1 (not_blocked_of_send (n := n) hsP).1 (not_blocked_of_send (n := n) hsP).2
          have hsetQ : set dst = false := by
            cases hsq : set dst with
            | false => rfl
            | true =>
              rcases hd2 dst hp.2.1 hsq with hb | ht
              · have := hd3 dst frm lsQ hp.2.1 hsq hrQ hb
                rw [hsetP] at this; cases this
              · unfold NTerminal at ht
                rw [hrQ] at ht; cases ht
          obtain ⟨l1, first, rest, rfl⟩ := RBs.len_pos_cons hp.2.2.2.1
          have hl1 : (RBs.cons l1 first rest).lookup l1 = some first := RBs.lookup_first
          obtain ⟨contP, hcontP⟩ : ∃ cP, lsP.lookup l1 = some cP := by
            have : (lsP.lookup l1).isSome = true := by rw [hleP l1, hl1]; rfl
            exact Option.isSome_iff_exists.mp this
          obtain ⟨contQ, hcontQ⟩ := hsupQ l1 first hl1
          -- the channel from the sender to the receiver is empty
          have hempty : c.ch frm dst = [] := by
            rw [hch frm dst hp.1 hp.2.1, ← hhq frm dst]
            simp only [R.queue]
            exact RBs.noSentFrom_queue _ frm dst hns
          have hstep1 : SendStep frm dst l1 (.sent frm dst l1 first) x := SendStep.here hx hl1
          have hinv1 := send_minv hinv hsP hcontP hstep1
          have hstep2 : RecvStep dst frm l1 first (.sent frm dst l1 first) :=
            recvStep_here (x := .sent frm dst l1 first) rfl
          have hl1q : (Function.update c.l frm contP dst).head = .recv frm lsQ := by
            rw [Function.update_of_ne (fun e => hp.2.2.1 e.symm)]; exact hrQ
          have hinv2 := recv_minv (tl := []) hinv1 hp.2.1 hp.1 hl1q
            (by simp [hempty]) hcontQ hstep2
          -- the configuration after the two steps
          have hl2 : ∀ u, u ≠ frm → u ≠ dst →
              Function.update (Function.update c.l frm contP) dst contQ u = c.l u := by
            intro u hu1 hu2
            rw [Function.update_of_ne hu2, Function.update_of_ne hu1]
          have hch2 : ∀ a b,
              Function.update
                (Function.update c.ch frm (Function.update (c.ch frm) dst (c.ch frm dst ++ [l1])))
                frm (Function.update
                  ((Function.update c.ch frm
                    (Function.update (c.ch frm) dst (c.ch frm dst ++ [l1]))) frm) dst []) a b
              = c.ch a b := by
            intro a b
            by_cases ha : a = frm
            · subst ha
              simp only [Function.update_self]
              by_cases hb : b = dst
              · subst hb; simp [hempty]
              · simp [Function.update_of_ne hb]
            · simp [Function.update_of_ne ha]
          have hstat : ∀ u, u ≠ frm → u ≠ dst →
              (NBlocked ({ l := Function.update (Function.update c.l frm contP) dst contQ,
                           ch := Function.update
                             (Function.update c.ch frm
                               (Function.update (c.ch frm) dst (c.ch frm dst ++ [l1])))
                             frm (Function.update
                               ((Function.update c.ch frm
                                 (Function.update (c.ch frm) dst (c.ch frm dst ++ [l1]))) frm)
                               dst []) } : NC) n u
                ↔ NBlocked c n u) ∧
              (NTerminal ({ l := Function.update (Function.update c.l frm contP) dst contQ,
                            ch := Function.update
                              (Function.update c.ch frm
                                (Function.update (c.ch frm) dst (c.ch frm dst ++ [l1])))
                              frm (Function.update
                                ((Function.update c.ch frm
                                  (Function.update (c.ch frm) dst (c.ch frm dst ++ [l1]))) frm)
                                dst []) } : NC) u
                ↔ NTerminal c u) :=
            fun u hu1 hu2 => status_same (n := n) (hl2 u hu1 hu2) (fun a => hch2 a u)
          have hmem : ∀ u, set u = true → u ≠ frm ∧ u ≠ dst := by
            intro u hu
            constructor
            · intro e; rw [e, hsetP] at hu; cases hu
            · intro e; rw [e, hsetQ] at hu; cases hu
          have hfd : first.fdepth r < d := by
            have := hhr.2.2
            rw [fdepth_comm_other h1 h2] at this
            simp only [RBs.fdepth] at this
            omega
          refine ih (first.fdepth r) hfd _ first r hinv2 ⟨?_, ?_, ?_⟩ hr hset
            ((hstat r (fun e => h1 e.symm) (fun e => h2 e.symm)).1.mpr hblk) (Nat.le_refl _)
          · exact ⟨r, hr, hset, (hstat r (fun e => h1 e.symm) (fun e => h2 e.symm)).1.mpr hblk⟩
          · intro u hu hsu
            obtain ⟨hu1, hu2⟩ := hmem u hsu
            rcases hd2 u hu hsu with h | h
            · exact Or.inl ((hstat u hu1 hu2).1.mpr h)
            · exact Or.inr ((hstat u hu1 hu2).2.mpr h)
          · intro u frm' ls' hu hsu hhu hbu
            obtain ⟨hu1, hu2⟩ := hmem u hsu
            dsimp only at hhu
            rw [hl2 u hu1 hu2] at hhu
            exact hd3 u frm' ls' hu hsu hhu ((hstat u hu1 hu2).1.mp hbu)
    | sent frm dst label cont =>
      rw [hx] at hhw hhr hhg hhq
      have hp := R.sent_wf_parts hhw.1
      by_cases h2 : dst = r
      · -- the awaiting role is the receiver of the message at the head
        subst h2
        obtain ⟨hfrm, ls, cc, hrh, hcc, hq⟩ := transit_receiver hinv hr hx
        exact (not_blocked_of_transit hfrm hrh hcc hq).1 hblk
      · -- the receiver of the message at the head takes it
        obtain ⟨hfrm, lsQ, contQ, hrQ, hcontQ, hq⟩ := transit_receiver hinv hp.2.1 hx
        have hnb := not_blocked_of_transit hfrm hrQ hcontQ hq
        have hsetQ := notmem dst hp.2.1 hnb.1 hnb.2
        have hstep : RecvStep dst frm label cont x := recvStep_here hx
        have hinv2 := recv_minv hinv hp.2.1 hfrm hrQ hq hcontQ hstep
        have hl2 : ∀ u, u ≠ dst → Function.update c.l dst contQ u = c.l u :=
          fun u hu => Function.update_of_ne hu _ _
        have hch2 : ∀ a u, u ≠ dst →
            Function.update c.ch frm (Function.update (c.ch frm) dst (cont.queue frm dst)) a u
              = c.ch a u := by
          intro a u hu
          by_cases ha : a = frm
          · subst ha; simp [Function.update_of_ne hu]
          · simp [Function.update_of_ne ha]
        have hstat : ∀ u, u ≠ dst →
            (NBlocked ({ l := Function.update c.l dst contQ,
                         ch := Function.update c.ch frm
                           (Function.update (c.ch frm) dst (cont.queue frm dst)) } : NC) n u
              ↔ NBlocked c n u) ∧
            (NTerminal ({ l := Function.update c.l dst contQ,
                          ch := Function.update c.ch frm
                            (Function.update (c.ch frm) dst (cont.queue frm dst)) } : NC) u
              ↔ NTerminal c u) :=
          fun u hu => status_same (n := n) (hl2 u hu) (fun a => hch2 a u hu)
        have hmem : ∀ u, set u = true → u ≠ dst := by
          intro u hu e
          rw [e, hsetQ] at hu; cases hu
        have hfd : cont.fdepth r < d := by
          have := hhr.2.2
          rw [fdepth_sent_other h2] at this
          omega
        refine ih (cont.fdepth r) hfd _ cont r hinv2 ⟨?_, ?_, ?_⟩ hr hset
          ((hstat r (fun e => h2 e.symm)).1.mpr hblk) (Nat.le_refl _)
        · exact ⟨r, hr, hset, (hstat r (fun e => h2 e.symm)).1.mpr hblk⟩
        · intro u hu hsu
          have hu2 := hmem u hsu
          rcases hd2 u hu hsu with h | h
          · exact Or.inl ((hstat u hu2).1.mpr h)
          · exact Or.inr ((hstat u hu2).2.mpr h)
        · intro u frm' ls' hu hsu hhu hbu
          have hu2 := hmem u hsu
          dsimp only at hhu
          rw [hl2 u hu2] at hhu
          exact hd3 u frm' ls' hu hsu hhu ((hstat u hu2).1.mp hbu)

/-- Under the invariant no waiting set exists. -/
theorem no_deadlock {c : NC} {x : R} {n : Nat} (h : MInv c x n) : ¬ NDeadlocked c n := by
  rintro ⟨set, hset⟩
  obtain ⟨r, hr, hsr, hb⟩ := hset.1
  exact blocked_depth _ c x r h hset hr hsr hb (Nat.le_refl _)

/-- **The multiparty progress theorem** (R41.3): every Protocol of `n`
roles that the acceptance conditions over the syntax admit (closed,
guarded, projectable onto every role with the full merge) has no waiting
set in any reachable configuration of the asynchronous product of its
projections. -/
theorem multiparty_progress {g : G} {n : Nat} (ha : acceptn g n) {c : NC} (h : NReach g n c) :
    ¬ NDeadlocked c n := by
  obtain ⟨x, hx⟩ := minv_reach ha h
  exact no_deadlock hx

end MP
