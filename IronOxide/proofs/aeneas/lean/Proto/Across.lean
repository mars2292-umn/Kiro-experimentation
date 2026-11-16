import Proto.Exists
/-!
# The relation across a step

The relation of a role after a step is the relation before it together
with every stepped pair, the role's type being its type after the step
(the actor continues with the branch of the label, every other role keeps
its type); it is consistent again. Port of the section "the relation
across a step" of `crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

/-- The branch list of a send or a receive (`nil` otherwise). -/
def L.lsOf : L → Ls
  | .send _ ls => ls
  | .recv _ ls => ls
  | _ => .nil

/-- The role's local type after the step: the actor continues with the
branch of the label, every other role keeps its type. -/
def afterAct (m : L) (u actor l : Nat) : L :=
  if u = actor then ((mhead m).lsOf.lookup l).getD .fin else m

theorem afterAct_ne {m : L} {u a l : Nat} (h : u ≠ a) : afterAct m u a l = m := by
  simp [afterAct, h]

theorem afterAct_send {m : L} {s t l : Nat} {ls : Ls} {c : L} (hm : mhead m = .send t ls)
    (hl : ls.lookup l = some c) : afterAct m s s l = c := by
  simp [afterAct, hm, L.lsOf, hl]

theorem afterAct_recv {m : L} {q p l : Nat} {ls : Ls} {c : L} (hm : mhead m = .recv p ls)
    (hl : ls.lookup l = some c) : afterAct m q q l = c := by
  simp [afterAct, hm, L.lsOf, hl]

/-- The relation of role `u` after `s` sent `l` to `t`: every pair, and
every stepped pair with the role's type after the step. -/
def RelSend (rel : L → R → Prop) (s t l u : Nat) : L → R → Prop :=
  fun m2 x2 => rel m2 x2 ∨ ∃ m x, rel m x ∧ SendStep s t l x2 x ∧ m2 = afterAct m u s l

def RelRecv (rel : L → R → Prop) (q p l u : Nat) : L → R → Prop :=
  fun m2 x2 => rel m2 x2 ∨ ∃ m x, rel m x ∧ RecvStep q p l x2 x ∧ m2 = afterAct m u q l

/-! Reading and building `NodeOk` at a known head. -/

theorem NodeOk.at_comm {rel : L → R → Prop} {m : L} {x : R} {u n frm dst : Nat} {bs : RBs}
    (h : NodeOk rel m x u n) (hx : x.head = .comm frm dst bs) :
    if u = frm then
      ∃ ls, mhead m = .send dst ls ∧ LabelsEq ls bs ∧
        ∀ label b, bs.lookup label = some b → ∃ c, ls.lookup label = some c ∧ rel c b
    else if u = dst then
      ∃ ls, mhead m = .recv frm ls ∧
        ∀ label b, bs.lookup label = some b → ∃ c, ls.lookup label = some c ∧ rel c b
    else ∀ label b, bs.lookup label = some b → rel m b := by
  obtain ⟨_, _, hnode⟩ := h
  rw [hx] at hnode
  exact hnode

theorem NodeOk.at_sent {rel : L → R → Prop} {m : L} {x : R} {u n frm dst label : Nat} {cont : R}
    (h : NodeOk rel m x u n) (hx : x.head = .sent frm dst label cont) :
    if u = dst then ∃ ls c, mhead m = .recv frm ls ∧ ls.lookup label = some c ∧ rel c cont
    else rel m cont := by
  obtain ⟨_, _, hnode⟩ := h
  rw [hx] at hnode
  exact hnode

theorem NodeOk.mk_sent {rel : L → R → Prop} {m : L} {u n frm dst label : Nat} {cont : R}
    (hw : (R.sent frm dst label cont).wf n = true)
    (hr : (mhead m).acting = true → (R.sent frm dst label cont).reaches u = true)
    (hn : if u = dst then ∃ ls c, mhead m = .recv frm ls ∧ ls.lookup label = some c ∧ rel c cont
      else rel m cont) :
    NodeOk rel m (.sent frm dst label cont) u n :=
  ⟨hw, hr, hn⟩

theorem NodeOk.mk_comm {rel : L → R → Prop} {m : L} {u n frm dst : Nat} {bs : RBs}
    (hw : (R.comm frm dst bs).wf n = true)
    (hr : (mhead m).acting = true → (R.comm frm dst bs).reaches u = true)
    (hn : if u = frm then
        ∃ ls, mhead m = .send dst ls ∧ LabelsEq ls bs ∧
          ∀ label b, bs.lookup label = some b → ∃ c, ls.lookup label = some c ∧ rel c b
      else if u = dst then
        ∃ ls, mhead m = .recv frm ls ∧
          ∀ label b, bs.lookup label = some b → ∃ c, ls.lookup label = some c ∧ rel c b
      else ∀ label b, bs.lookup label = some b → rel m b) :
    NodeOk rel m (.comm frm dst bs) u n :=
  ⟨hw, hr, hn⟩

/-- The conditions on a pair only grow with the relation. -/
theorem NodeOk.mono {rel rel2 : L → R → Prop} {m : L} {x : R} {u n : Nat}
    (h : NodeOk rel m x u n) (hsub : ∀ a b, rel a b → rel2 a b) : NodeOk rel2 m x u n := by
  obtain ⟨hw, hr, hnode⟩ := h
  refine ⟨hw, hr, ?_⟩
  cases hx : x.head with
  | fin => rw [hx] at hnode; exact hnode
  | var v => rw [hx] at hnode; exact hnode
  | mu b => rw [hx] at hnode; exact hnode
  | comm frm dst bs =>
    rw [hx] at hnode
    dsimp only at hnode ⊢
    by_cases h1 : u = frm
    · rw [if_pos h1] at hnode ⊢
      obtain ⟨ls, a, b, c⟩ := hnode
      exact ⟨ls, a, b, fun label g hg => by
        obtain ⟨cc, h1', h2'⟩ := c label g hg; exact ⟨cc, h1', hsub _ _ h2'⟩⟩
    · rw [if_neg h1] at hnode ⊢
      by_cases h2 : u = dst
      · rw [if_pos h2] at hnode ⊢
        obtain ⟨ls, a, c⟩ := hnode
        exact ⟨ls, a, fun label g hg => by
          obtain ⟨cc, h1', h2'⟩ := c label g hg; exact ⟨cc, h1', hsub _ _ h2'⟩⟩
      · rw [if_neg h2] at hnode ⊢
        exact fun label g hg => hsub _ _ (hnode label g hg)
  | sent frm dst label cont =>
    rw [hx] at hnode
    dsimp only at hnode ⊢
    by_cases h2 : u = dst
    · rw [if_pos h2] at hnode ⊢
      obtain ⟨ls, c, a, b, d⟩ := hnode
      exact ⟨ls, c, a, b, hsub _ _ d⟩
    · rw [if_neg h2] at hnode ⊢
      exact hsub _ _ hnode

/-! ## The actor is reached after its own step, if it still acts -/

mutual
theorem sendStep_reaches_actor {rel : L → R → Prop} {m : L} {s t l n : Nat}
    (hc : Consistent rel s n) (hact : (mhead (afterAct m s s l)).acting = true) :
    ∀ (x2 x : R), rel m x → SendStep s t l x2 x → x2.reaches s = true
  | .sent f2 d2 lab2 c2, x, hrel, h => by
    have hnode := hc m x hrel
    rcases sendStep_cases h with ⟨bs, b, hx, hl, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨frm, dst, label, cont, c2', hx, hd, e, hs⟩
    · cases e
      have := hnode.at_comm hx
      rw [if_pos rfl] at this
      obtain ⟨ls, hm, _, hpairs⟩ := this
      obtain ⟨c, hcl, hcr⟩ := hpairs l c2 hl
      rw [afterAct_send hm hcl] at hact
      have := (hc c c2 hcr).2.1 hact
      simp [R.reaches, this]
    · cases e
    · cases e
      have := hnode.at_sent hx
      rw [if_neg (fun e => hd e.symm)] at this
      simp only [R.reaches, Bool.or_eq_true]
      exact Or.inr (sendStep_reaches_actor hc hact c2 cont this hs)
  | .comm f2 d2 bs2, x, hrel, h => by
    have hnode := hc m x hrel
    have hhw := (R.head_wf hnode.1).1
    rcases sendStep_cases h with ⟨_, _, _, _, e⟩ | ⟨frm, dst, bs, bs2', hx, h1, h2, e, hs⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · cases e
    · cases e
      rw [hx] at hhw
      have hp := R.comm_wf_parts hhw
      have := hnode.at_comm hx
      rw [if_neg (fun e => h1 e.symm), if_neg (fun e => h2 e.symm)] at this
      simp only [R.reaches, Bool.or_eq_true]
      refine Or.inr (sendStepBs_reaches_actor hc hact bs2 bs hp.2.2.2.2.1 this hs ?_)
      obtain ⟨_, _, _, rfl⟩ := RBs.len_pos_cons hp.2.2.2.1
      simp
    · cases e
  | .fin, _, _, h => by simp [SendStep] at h
  | .mu _, _, _, h => by simp [SendStep] at h
  | .var _, _, _, h => by simp [SendStep] at h
theorem sendStepBs_reaches_actor {rel : L → R → Prop} {m : L} {s t l n : Nat}
    (hc : Consistent rel s n) (hact : (mhead (afterAct m s s l)).acting = true) :
    ∀ (bs2 bs : RBs), bs.distinct = true → (∀ label b, bs.lookup label = some b → rel m b) →
      SendStepBs s t l bs2 bs → bs ≠ .nil → bs2.reaches s = true
  | .nil, bs, _, _, h, hne => by
    simp only [SendStepBs] at h
    exact absurd h hne
  | .cons label2 g2 rest2, bs, hd, hall, h, _ => by
    cases bs with
    | nil => simp [SendStepBs] at h
    | cons label g rest =>
      simp only [SendStepBs] at h
      obtain ⟨rfl, h1, h2⟩ := h
      simp only [RBs.distinct, Bool.and_eq_true] at hd
      have hrest : ∀ lab b, rest.lookup lab = some b → rel m b := by
        intro lab b hb
        apply hall lab b
        simp only [RBs.lookup]
        rw [if_neg]
        · exact hb
        · intro e; subst e
          rw [Option.isNone_iff_eq_none.mp hd.1] at hb; cases hb
      apply RBs.reaches_cons_mk
        (sendStep_reaches_actor hc hact g2 g (hall label g RBs.lookup_first) h1)
      by_cases hr : rest = .nil
      · subst hr
        exact Or.inl (sendStepBs_nil_right h2)
      · exact Or.inr (sendStepBs_reaches_actor hc hact rest2 rest hd.2 hrest h2 hr)
end

theorem recv_here_reaches_actor {rel : L → R → Prop} {m : L} {q p l n : Nat} {x x2 cont : R}
    (hc : Consistent rel q n) (hact : (mhead (afterAct m q q l)).acting = true)
    (hrel : rel m x) (hx : x.head = .sent p q l cont) (e : x2 = cont) : x2.reaches q = true := by
  subst e
  have := (hc m x hrel).at_sent hx
  rw [if_pos rfl] at this
  obtain ⟨ls, c, hm, hcl, hcr⟩ := this
  rw [afterAct_recv hm hcl] at hact
  exact (hc c x2 hcr).2.1 hact

mutual
theorem recvStep_reaches_actor {rel : L → R → Prop} {m : L} {q p l n : Nat}
    (hc : Consistent rel q n) (hact : (mhead (afterAct m q q l)).acting = true) :
    ∀ (x2 x : R), rel m x → RecvStep q p l x2 x → x2.reaches q = true
  | .sent f2 d2 lab2 c2, x, hrel, h => by
    have hnode := hc m x hrel
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨frm, dst, label, cont, c2', hx, hd, e, hs⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · exact recv_here_reaches_actor hc hact hrel hx e
    · cases e
      have := hnode.at_sent hx
      rw [if_neg (fun e => hd e.symm)] at this
      simp only [R.reaches, Bool.or_eq_true]
      exact Or.inr (recvStep_reaches_actor hc hact c2 cont this hs)
    · cases e
  | .comm f2 d2 bs2, x, hrel, h => by
    have hnode := hc m x hrel
    have hhw := (R.head_wf hnode.1).1
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨frm, dst, bs, bs2', hx, h1, h2, e, hs⟩
    · exact recv_here_reaches_actor hc hact hrel hx e
    · cases e
    · cases e
      rw [hx] at hhw
      have hp := R.comm_wf_parts hhw
      have := hnode.at_comm hx
      rw [if_neg (fun e => h1 e.symm), if_neg (fun e => h2 e.symm)] at this
      simp only [R.reaches, Bool.or_eq_true]
      refine Or.inr (recvStepBs_reaches_actor hc hact bs2 bs hp.2.2.2.2.1 this hs ?_)
      obtain ⟨_, _, _, rfl⟩ := RBs.len_pos_cons hp.2.2.2.1
      simp
  | .fin, x, hrel, h => by
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · exact recv_here_reaches_actor hc hact hrel hx e
    · cases e
    · cases e
  | .mu b2, x, hrel, h => by
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · exact recv_here_reaches_actor hc hact hrel hx e
    · cases e
    · cases e
  | .var v2, x, hrel, h => by
    rcases recvStep_cases h with ⟨cont, hx, e⟩ | ⟨_, _, _, _, _, _, _, e, _⟩ |
      ⟨_, _, _, _, _, _, _, e, _⟩
    · exact recv_here_reaches_actor hc hact hrel hx e
    · cases e
    · cases e
theorem recvStepBs_reaches_actor {rel : L → R → Prop} {m : L} {q p l n : Nat}
    (hc : Consistent rel q n) (hact : (mhead (afterAct m q q l)).acting = true) :
    ∀ (bs2 bs : RBs), bs.distinct = true → (∀ label b, bs.lookup label = some b → rel m b) →
      RecvStepBs q p l bs2 bs → bs ≠ .nil → bs2.reaches q = true
  | .nil, bs, _, _, h, hne => by
    simp only [RecvStepBs] at h
    exact absurd h hne
  | .cons label2 g2 rest2, bs, hd, hall, h, _ => by
    cases bs with
    | nil => simp [RecvStepBs] at h
    | cons label g rest =>
      simp only [RecvStepBs] at h
      obtain ⟨rfl, h1, h2⟩ := h
      simp only [RBs.distinct, Bool.and_eq_true] at hd
      have hrest : ∀ lab b, rest.lookup lab = some b → rel m b := by
        intro lab b hb
        apply hall lab b
        simp only [RBs.lookup]
        rw [if_neg]
        · exact hb
        · intro e; subst e
          rw [Option.isNone_iff_eq_none.mp hd.1] at hb; cases hb
      apply RBs.reaches_cons_mk
        (recvStep_reaches_actor hc hact g2 g (hall label g RBs.lookup_first) h1)
      by_cases hr : rest = .nil
      · subst hr
        exact Or.inl (recvStepBs_nil_right h2)
      · exact Or.inr (recvStepBs_reaches_actor hc hact rest2 rest hd.2 hrest h2 hr)
end

end MP
