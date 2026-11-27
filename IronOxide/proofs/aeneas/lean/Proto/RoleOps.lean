import Proto.Exec
open Aeneas Aeneas.Std Result ControlFlow
/-!
# The role operations on the code as compiled (R38.4)

`rsk_protocol::api::role_ops` reads, for every state of a role's table,
the kind of the state, its peer, and its operations (a label and the state
after it) from the verified flattening. Verus proves `role_api` of it
(`crates/rsk-protocol/src/api.rs`). This file proves the same statement of
the function as rustc compiles it without Verus and Aeneas translates it
(`Proto/Proto.lean`: `api.next_state`, `api.ops_of`, `api.state_ops`,
`api.role_ops`), and that the function returns.

`Offers`, `RoleApi` and `RoleTransOk` restate `api::offers`, `api::role_api`
and `api::role_trans_ok`; that the texts say the same is by reading.
`L.head` and `Ls.lookup` are those of `Proto/Spec.lean`.
-/

namespace MP
open rsk_protocol

instance : Inhabited L := ⟨.fin⟩
instance : Inhabited api.Op := ⟨⟨0#u32, 0#usize⟩⟩
instance : Inhabited api.StateOps := ⟨⟨0#u8, 0#usize, alloc.vec.Vec.new api.Op⟩⟩
instance : Inhabited theory.LE := ⟨.End⟩

/-! ## The specification (`api.rs`) -/

/-- `ls_labels`: the labels of a branch list, in order. -/
def Ls.labels : Ls → List Nat
  | .nil => []
  | .cons label _ rest => label :: rest.labels

/-- `ls_of`: the branch list of a head-normal local type. -/
def L.branches : L → Ls
  | .send _ ls => ls
  | .recv _ ls => ls
  | _ => .nil

/-- What `offers` asks of one operation: its target is a state of the
table, the label is a branch, and the target's local type is the head of
that branch's continuation. -/
abbrev OpOk (tb : List L) (ls : Ls) (op : api.Op) : Prop :=
  op.«to».val < tb.length ∧
  ∃ c, ls.lookup op.label.val = some c ∧ tb[op.«to».val]! = c.head

/-- `offers`: the state `st` offers exactly the operations the local type
`e` allows, each leading to the state of the table `tb` that is the head
of its continuation. -/
def Offers (st : api.StateOps) (tb : List L) (e : L) : Prop :=
  (match e with
    | .send dst _ => st.kind = api.KIND_SEND ∧ st.peer.val = dst
    | .recv frm _ => st.kind = api.KIND_RECV ∧ st.peer.val = frm
    | _ => st.kind = api.KIND_END) ∧
  st.ops.val.length = e.branches.labels.length ∧
  ∀ k, k < st.ops.val.length →
    (st.ops.val[k]!).label.val = e.branches.labels[k]! ∧ OpOk tb e.branches st.ops.val[k]!

/-- `role_api`: one entry per state of the table, each offering what the
state's local type allows. -/
def RoleApi (out : List api.StateOps) (tb : List L) : Prop :=
  out.length = tb.length ∧ ∀ i, i < tb.length → Offers out[i]! tb tb[i]!

/-- `role_trans_ok`: every branch of every entry has a transition of the
role, and every transition of the role is a branch of its entry that leads
to the head of the continuation. -/
def RoleTransOk (trans : List progress.Trans) (r : Usize) (tb : List L) : Prop :=
  (∀ i label, i < tb.length → (tb[i]!.branches.lookup label).isSome →
    ∃ t, t ∈ trans ∧ t.role = r ∧ t.«from».val = i ∧ t.label.val = label) ∧
  (∀ t, t ∈ trans → t.role = r →
    t.«from».val < tb.length ∧ t.«to».val < tb.length ∧
    ∃ c, tb[t.«from».val]!.branches.lookup t.label.val = some c ∧ tb[t.«to».val]! = c.head)

/-! ## Lists -/

theorem getElem!_append_lt {α} [Inhabited α] (a b : List α) (j : Nat) (h : j < a.length) :
    (a ++ b)[j]! = a[j]! := by
  rw [getElem!_pos (a ++ b) j (by rw [List.length_append]; omega), getElem!_pos a j h,
    List.getElem_append_left h]

theorem getElem!_append_len {α} [Inhabited α] (a : List α) (x : α) :
    (a ++ [x])[a.length]! = x := by
  rw [getElem!_pos (a ++ [x]) a.length (by simp)]
  simp

theorem getElem!_map' {α β} [Inhabited α] [Inhabited β] (f : α → β) (l : List α) (j : Nat)
    (h : j < l.length) : (l.map f)[j]! = f l[j]! := by
  rw [getElem!_pos (l.map f) j (by rw [List.length_map]; exact h), getElem!_pos l j h,
    List.getElem_map]

/-- `lemma_label_found`: a label of the list is one the lookup finds. -/
theorem Ls.lookup_of_mem_labels : ∀ (ls : Ls) (label : Nat), label ∈ ls.labels →
    (ls.lookup label).isSome
  | .nil, _, h => by simp [Ls.labels] at h
  | .cons l c rest, label, h => by
    simp only [Ls.labels, List.mem_cons] at h
    unfold Ls.lookup
    by_cases e : l = label
    · rw [if_pos e]; rfl
    · rw [if_neg e]
      exact Ls.lookup_of_mem_labels rest label (h.resolve_left (fun e' => e e'.symm))

theorem ite3 {α} (a b c : Prop) [Decidable a] [Decidable b] [Decidable c] (x y : α) :
    (if a then if b then if c then x else y else y else y) = if a ∧ b ∧ c then x else y := by
  by_cases ha : a <;> by_cases hb : b <;> by_cases hc : c <;> simp [ha, hb, hc]

/-- Splits a goal into the conjuncts it is made of, whatever their nesting. -/
macro "leaves" : tactic => `(tactic| constructorm* _ ∧ _)

/-- The last statement of a loop body that goes on: the index plus one. -/
theorem cont_incr {α β : Type} (i : Usize) (n : Nat) (hi : i.val < n) (hn : n ≤ Usize.max)
    {f : Usize → ControlFlow α β} {Q : ControlFlow α β → Prop}
    (h : ∀ i1 : Usize, i1.val = i.val + 1 → Q (f i1)) :
    (do
      let i1 ← i + 1#usize
      ok (f i1)) ⦃ r => Q r ⦄ := by
  apply WP.spec_bind (UScalar.add_spec (x := i) (y := 1#usize) (by scalar_tac))
  intro i1 hi1
  exact (WP.spec_ok _).mpr (h i1 (by scalar_tac))

/-! ## `next_state` -/

/-- The first transition of role `r` from state `i` on `label`, or none
when the list has none (the contract of `next_state`). -/
theorem next_state_spec (trans : alloc.vec.Vec progress.Trans) (r i : Usize) (label : U32) :
    api.next_state trans r i label ⦃ o =>
      match o with
      | some j => ∃ t, t ∈ trans.val ∧ t.role = r ∧ t.«from» = i ∧ t.label = label ∧ t.«to» = j
      | none => ∀ t, t ∈ trans.val → ¬ (t.role = r ∧ t.«from» = i ∧ t.label = label) ⦄ := by
  unfold api.next_state api.next_state_loop
  apply loop.spec_decr_nat (fun (k : Usize) => trans.val.length - k.val)
    (fun k => k.val ≤ trans.val.length ∧ ∀ m (hm : m < k.val) (hl : m < trans.val.length),
      ¬ ((trans.val[m]).role = r ∧ (trans.val[m]).«from» = i ∧ (trans.val[m]).label = label))
  · intro k hk
    obtain ⟨hk, hinv⟩ := hk
    unfold api.next_state_loop.body
    dsimp only
    split
    · rename_i hlt
      have hkl : k.val < trans.val.length := by scalar_tac
      step as ⟨t, ht⟩
      rw [ite3]
      split
      · rename_i hc
        simp only [WP.spec_ok]
        exact ⟨t, by rw [ht]; exact List.getElem_mem _, hc.1, hc.2.1, hc.2.2, rfl⟩
      · rename_i hc
        apply cont_incr k trans.val.length hkl (by scalar_tac)
        intro k1 hk1
        dsimp only
        leaves <;> first | omega | skip
        intro m hm hl
        by_cases hmk : m = k.val
        · subst hmk
          rw [ht] at hc
          exact hc
        · exact hinv m (by omega) hl
    · rename_i hlt
      simp only [WP.spec_ok]
      intro t htm hc
      obtain ⟨m, hm, rfl⟩ := List.getElem_of_mem htm
      exact hinv m (by scalar_tac) hm hc
  · exact ⟨by scalar_tac, fun m hm => absurd hm (by scalar_tac)⟩

/-! ## The operations of one branch list -/

theorem ops_of_loop_spec (tb : List L) (ls0 : Ls) (trans : alloc.vec.Vec progress.Trans)
    (r i : Usize)
    (hfind : ∀ label : U32, (ls0.lookup label.val).isSome →
      ∃ t, t ∈ trans.val ∧ t.role = r ∧ t.«from» = i ∧ t.label = label)
    (hgood : ∀ t, t ∈ trans.val → t.role = r → t.«from» = i →
      t.«to».val < tb.length ∧ ∃ c, ls0.lookup t.label.val = some c ∧ tb[t.«to».val]! = c.head) :
    ∀ (cur : theory.LsE) (ops : alloc.vec.Vec api.Op),
      (∀ label, label ∈ (vLs cur).labels → (ls0.lookup label).isSome) →
      ops.val.length + (vLs cur).labels.length ≤ Usize.max →
      api.ops_of_loop trans r i ops cur ⦃ out => ∃ new : List api.Op,
        out.val = ops.val ++ new ∧ new.map (fun op => op.label.val) = (vLs cur).labels ∧
        ∀ op, op ∈ new → OpOk tb ls0 op ⦄
  | .Nil, ops, _, _ => by
    unfold api.ops_of_loop
    simp only [WP.spec_ok]
    exact ⟨[], by simp, by simp [vLs, Ls.labels], by simp⟩
  | .Cons label l rest, ops, hl, hb => by
    have hsome : (ls0.lookup label.val).isSome := hl label.val (by simp [vLs, Ls.labels])
    have hlen : ops.val.length < Usize.max := by
      simp only [vLs, Ls.labels, List.length_cons] at hb
      omega
    unfold api.ops_of_loop
    apply WP.spec_bind (next_state_spec trans r i label)
    intro o ho
    cases o with
    | none =>
      exfalso
      have ho' : ∀ t, t ∈ trans.val → ¬ (t.role = r ∧ t.«from» = i ∧ t.label = label) := ho
      obtain ⟨t, ht, h1, h2, h3⟩ := hfind label hsome
      exact ho' t ht ⟨h1, h2, h3⟩
    | some j =>
      have ho' : ∃ t, t ∈ trans.val ∧ t.role = r ∧ t.«from» = i ∧ t.label = label ∧ t.«to» = j := ho
      obtain ⟨t, htm, h1, h2, h3, h4⟩ := ho'
      obtain ⟨hto, c, hc, hhead⟩ := hgood t htm h1 h2
      simp only [bind_ok]
      apply WP.spec_bind (alloc.vec.Vec.push_spec ops _ hlen)
      intro ops1 hops1
      have ih := ops_of_loop_spec tb ls0 trans r i hfind hgood rest ops1
        (fun lb hlb => hl lb (by simp [vLs, Ls.labels, hlb]))
        (by
          rw [hops1]
          simp only [vLs, Ls.labels, List.length_cons, List.length_append,
            List.length_nil] at hb ⊢
          omega)
      refine WP.spec_mono ih ?_
      intro out hout
      obtain ⟨new, hout, hlab, hok⟩ := hout
      refine ⟨({ label := label, «to» := j } : api.Op) :: new, ?_, ?_, ?_⟩
      · rw [hout, hops1, List.append_assoc]
        rfl
      · simp only [List.map_cons, vLs, Ls.labels, hlab]
      · intro op hop
        rcases List.mem_cons.mp hop with e | hop
        · subst e
          subst h3
          subst h4
          exact ⟨hto, c, hc, hhead⟩
        · exact hok op hop

/-! ## One state -/

theorem state_ops_spec (tbl : alloc.vec.Vec theory.LE) (trans : alloc.vec.Vec progress.Trans)
    (r i : Usize) (hi : i.val < tbl.val.length)
    (ht : RoleTransOk trans.val r (tbl.val.map vL))
    (hb : (vL tbl.val[i.val]!).branches.labels.length ≤ Usize.max) :
    api.state_ops tbl trans r i ⦃ st => Offers st (tbl.val.map vL) (vL tbl.val[i.val]!) ⦄ := by
  have hnil : (alloc.vec.Vec.new api.Op).val = [] := rfl
  have hmap : (tbl.val.map vL)[i.val]! = vL tbl.val[i.val]! := getElem!_map' vL tbl.val i.val hi
  have key : ∀ ls, (vL tbl.val[i.val]!).branches = vLs ls →
      api.ops_of ls tbl trans r i ⦃ ops =>
        ops.val.length = (vLs ls).labels.length ∧
        ∀ k, k < ops.val.length →
          (ops.val[k]!).label.val = (vLs ls).labels[k]! ∧
          OpOk (tbl.val.map vL) (vLs ls) ops.val[k]! ⦄ := by
    intro ls hls
    unfold api.ops_of
    refine WP.spec_mono (ops_of_loop_spec (tbl.val.map vL) (vLs ls) trans r i ?_ ?_ ls
      (alloc.vec.Vec.new api.Op) ?_ ?_) ?_
    · intro label hsome
      obtain ⟨t, htm, h1, h2, h3⟩ := ht.1 i.val label.val
        (by rw [List.length_map]; exact hi) (by rw [hmap, hls]; exact hsome)
      exact ⟨t, htm, h1, usize_val_inj h2, u32_val_inj h3⟩
    · intro t htm h1 h2
      obtain ⟨_, hto, c, hc, hh⟩ := ht.2 t htm h1
      rw [h2, hmap, hls] at hc
      exact ⟨hto, c, hc, hh⟩
    · intro label hmem
      exact Ls.lookup_of_mem_labels _ _ hmem
    · rw [hnil, ← hls]
      simpa using hb
    · intro ops hops
      obtain ⟨new, hout, hlab, hok⟩ := hops
      have hout' : ops.val = new := by rw [hout, hnil]; rfl
      rw [hout']
      have hlen : new.length = (vLs ls).labels.length := by rw [← hlab, List.length_map]
      refine ⟨hlen, ?_⟩
      intro k hk
      refine ⟨?_, hok _ (by rw [getElem!_pos new k hk]; exact List.getElem_mem _)⟩
      rw [← hlab, getElem!_map' _ new k hk]
  unfold api.state_ops
  step as ⟨l, hl⟩
  have el : l = tbl.val[i.val]! := by rw [hl]; exact (getElem!_pos tbl.val i.val hi).symm
  rw [← el] at key ⊢
  cases l with
  | End =>
    simp only [WP.spec_ok]
    exact ⟨rfl, by rw [hnil]; rfl, fun k hk => absurd hk (by rw [hnil]; simp)⟩
  | Rec body =>
    simp only [WP.spec_ok]
    exact ⟨rfl, by rw [hnil]; rfl, fun k hk => absurd hk (by rw [hnil]; simp)⟩
  | Var n =>
    simp only [WP.spec_ok]
    exact ⟨rfl, by rw [hnil]; rfl, fun k hk => absurd hk (by rw [hnil]; simp)⟩
  | Send dst ls =>
    apply WP.spec_bind (key ls (by simp only [vL, L.branches]))
    intro ops hops
    simp only [WP.spec_ok]
    refine ⟨?_, ?_, ?_⟩
    · simp only [vL]
      first | exact ⟨rfl, rfl⟩ | exact ⟨trivial, rfl⟩ | exact ⟨trivial, trivial⟩
    · simp only [vL, L.branches]
      exact hops.1
    · simp only [vL, L.branches]
      exact hops.2
  | Recv frm ls =>
    apply WP.spec_bind (key ls (by simp only [vL, L.branches]))
    intro ops hops
    simp only [WP.spec_ok]
    refine ⟨?_, ?_, ?_⟩
    · simp only [vL]
      first | exact ⟨rfl, rfl⟩ | exact ⟨trivial, rfl⟩ | exact ⟨trivial, trivial⟩
    · simp only [vL, L.branches]
      exact hops.1
    · simp only [vL, L.branches]
      exact hops.2

/-! ## Every state of the role -/

/-- **R38.4 on the code as compiled.** Given what the flattening proves of
the transitions (`RoleTransOk`), `role_ops` returns, with one entry per
state of the table, each offering exactly the operations of the state's
local type, in the projection's order, each leading to the state whose
local type is the head of that branch's continuation. The second premise
is of the model alone: a branch list has no more entries than a vector
holds (a list in memory cannot have more). -/
theorem role_ops_spec (tbl : alloc.vec.Vec theory.LE) (trans : alloc.vec.Vec progress.Trans)
    (r : Usize) (ht : RoleTransOk trans.val r (tbl.val.map vL))
    (hb : ∀ i, i < tbl.val.length → (vL tbl.val[i]!).branches.labels.length ≤ Usize.max) :
    api.role_ops tbl trans r ⦃ out => RoleApi out.val (tbl.val.map vL) ⦄ := by
  unfold api.role_ops api.role_ops_loop
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec api.StateOps × Usize) => tbl.val.length - s.2.val)
    (fun s => s.2.val ≤ tbl.val.length ∧ s.1.val.length = s.2.val ∧
      ∀ j, j < s.2.val → Offers s.1.val[j]! (tbl.val.map vL) (vL tbl.val[j]!))
  · intro s hs
    obtain ⟨out, i⟩ := s
    obtain ⟨h1, h2, h3⟩ := hs
    dsimp only at h1 h2 h3 ⊢
    unfold api.role_ops_loop.body
    dsimp only
    split
    · rename_i hlt
      have hi : i.val < tbl.val.length := by scalar_tac
      apply WP.spec_bind (state_ops_spec tbl trans r i hi ht (hb i.val hi))
      intro st hst
      have hlen : out.val.length < Usize.max := by
        have := tbl.property
        omega
      apply WP.spec_bind (alloc.vec.Vec.push_spec out st hlen)
      intro out1 hout1
      apply cont_incr i tbl.val.length hi (by scalar_tac)
      intro i2 hi2
      have hl1 : out1.val.length = i2.val := by
        rw [hout1, List.length_append, List.length_singleton]; omega
      dsimp only
      leaves <;> first | omega | skip
      intro j hj
      rw [hout1]
      by_cases hji : j = i.val
      · rw [hji, ← h2, getElem!_append_len, h2]
        exact hst
      · rw [getElem!_append_lt _ _ j (by omega)]
        exact h3 j (by omega)
    · rename_i hlt
      simp only [WP.spec_ok]
      have hil : i.val = tbl.val.length := by scalar_tac
      refine ⟨by rw [List.length_map, h2, hil], ?_⟩
      intro j hj
      rw [List.length_map] at hj
      rw [getElem!_map' vL tbl.val j hj]
      exact h3 j (by omega)
  · exact ⟨by scalar_tac, rfl, fun j hj => absurd hj (by scalar_tac)⟩

/-- A computation that satisfies a triple returns a value that satisfies
its postcondition. -/
theorem spec_returns {α} {m : Result α} {p : α → Prop} (h : WP.spec m p) :
    ∃ x, m = ok x ∧ p x := by
  cases m with
  | ret x => exact ⟨x, rfl, (WP.spec_ok x).mp h⟩
  | vis e k => exact ((WP.spec_vis e k).mp h).elim
  | div => exact (WP.spec_div.mp h).elim

/-- `role_ops_spec` read as an equation: the function returns a table that
satisfies `RoleApi`. -/
theorem role_ops_returns (tbl : alloc.vec.Vec theory.LE) (trans : alloc.vec.Vec progress.Trans)
    (r : Usize) (ht : RoleTransOk trans.val r (tbl.val.map vL))
    (hb : ∀ i, i < tbl.val.length → (vL tbl.val[i]!).branches.labels.length ≤ Usize.max) :
    ∃ out, api.role_ops tbl trans r = ok out ∧ RoleApi out.val (tbl.val.map vL) :=
  spec_returns (role_ops_spec tbl trans r ht hb)

end MP
