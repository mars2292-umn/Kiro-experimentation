import Proto.RoleOps
open Aeneas Aeneas.Std Result
/-!
# The projections on the code as compiled (R38.3)

Two projections of `rsk-protocol` are executable:

- `theory::proj_exec`, the two-role projection (no merge: both roles take
  part in every communication), whose Verus contract is
  `l@ == proj(g@, r)`. `proj2` restates `theory::proj`; that the two texts
  say the same is by reading. `proj_exec_spec` proves the contract of the
  function as compiled, and that it returns.
- `multiparty::projn_exec`, the projection with the full merge, whose
  result `Proto/Exec.lean` already reads (`projn_exec_ok`: a returned type
  is `projn` of `Proto/Spec.lean`). This file adds that it returns for
  every input (`projn_exec_total`), with the functions under it, and
  states the two together (`projn_exec_spec`).
-/

namespace MP
open rsk_protocol

/-! ## The two-role projection -/

mutual
/-- `theory::proj`: the sender's side of a communication is an internal
choice, the receiver's an external one; a role not involved gets `end`. -/
def proj2 : G → Nat → L
  | .fin, _ => .fin
  | .var n, _ => .var n
  | .mu body, r => .mu (proj2 body r)
  | .comm frm dst bs, r =>
    if r = frm then .send dst (proj2Bs bs r)
    else if r = dst then .recv frm (proj2Bs bs r)
    else .fin
/-- `theory::proj_bs`. -/
def proj2Bs : Bs → Nat → Ls
  | .nil, _ => .nil
  | .cons label g rest, r => .cons label (proj2 g r) (proj2Bs rest r)
end

mutual
/-- **The contract of `proj_exec` on the code as compiled**: the function
returns, and the view of its result is the projection of the view. -/
theorem proj_exec_spec : ∀ (g : theory.GE) (r : Usize),
    ∃ l, theory.proj_exec g r = ok l ∧ vL l = proj2 (vG g) r.val
  | .End, r => ⟨.End, by unfold theory.proj_exec; rfl, by simp only [vL, vG, proj2]⟩
  | .Var n, r => ⟨.Var n, by unfold theory.proj_exec; rfl, by simp only [vL, vG, proj2]⟩
  | .Rec body, r => by
    obtain ⟨l, hl, hv⟩ := proj_exec_spec body r
    refine ⟨.Rec l, ?_, ?_⟩
    · unfold theory.proj_exec
      simp only [hl, bind_ok]
    · simp only [vL, vG, proj2, hv]
  | .Comm f t bs, r => by
    obtain ⟨ls, hls, hv⟩ := proj_bs_exec_spec bs r
    by_cases e1 : r = f
    · refine ⟨.Send t ls, ?_, ?_⟩
      · unfold theory.proj_exec
        simp only [if_pos e1, hls, bind_ok]
      · simp only [vL, vG, proj2]
        rw [if_pos (by rw [e1]), hv]
    · by_cases e2 : r = t
      · refine ⟨.Recv f ls, ?_, ?_⟩
        · unfold theory.proj_exec
          simp only [if_neg e1, if_pos e2, hls, bind_ok]
        · simp only [vL, vG, proj2]
          rw [if_neg (fun e' => e1 (usize_val_inj e')), if_pos (by rw [e2]), hv]
      · refine ⟨.End, ?_, ?_⟩
        · unfold theory.proj_exec
          simp only [if_neg e1, if_neg e2]
        · simp only [vL, vG, proj2]
          rw [if_neg (fun e' => e1 (usize_val_inj e')), if_neg (fun e' => e2 (usize_val_inj e'))]
theorem proj_bs_exec_spec : ∀ (bs : theory.BE) (r : Usize),
    ∃ ls, theory.proj_bs_exec bs r = ok ls ∧ vLs ls = proj2Bs (vBs bs) r.val
  | .Nil, r => ⟨.Nil, by unfold theory.proj_bs_exec; rfl, by simp only [vLs, vBs, proj2Bs]⟩
  | .Cons label g rest, r => by
    obtain ⟨l, hl, hv⟩ := proj_exec_spec g r
    obtain ⟨ls, hls, hvs⟩ := proj_bs_exec_spec rest r
    refine ⟨.Cons label l ls, ?_, ?_⟩
    · unfold theory.proj_bs_exec
      simp only [hl, hls, bind_ok]
    · simp only [vLs, vBs, proj2Bs, hv, hvs]
end

/-! ## The projection with the merge returns -/

theorem ok_true {α} (x : α) : (ok x : Result α) ⦃ _r => True ⦄ :=
  (WP.spec_ok x).mpr trivial

mutual
theorem le_copy_spec : ∀ l : theory.LE, theory.le_copy l ⦃ l' => l' = l ⦄
  | .End => by unfold theory.le_copy; exact (WP.spec_ok _).mpr rfl
  | .Var n => by unfold theory.le_copy; exact (WP.spec_ok _).mpr rfl
  | .Send t ls => by
    unfold theory.le_copy
    apply WP.spec_bind (lse_copy_spec ls)
    intro le hle
    subst hle
    exact (WP.spec_ok _).mpr rfl
  | .Recv f ls => by
    unfold theory.le_copy
    apply WP.spec_bind (lse_copy_spec ls)
    intro le hle
    subst hle
    exact (WP.spec_ok _).mpr rfl
  | .Rec body => by
    unfold theory.le_copy
    apply WP.spec_bind (le_copy_spec body)
    intro l1 hl1
    subst hl1
    exact (WP.spec_ok _).mpr rfl
theorem lse_copy_spec : ∀ ls : theory.LsE, theory.lse_copy ls ⦃ ls' => ls' = ls ⦄
  | .Nil => by unfold theory.lse_copy; exact (WP.spec_ok _).mpr rfl
  | .Cons label l rest => by
    unfold theory.lse_copy
    apply WP.spec_bind (le_copy_spec l)
    intro l1 hl1
    subst hl1
    apply WP.spec_bind (lse_copy_spec rest)
    intro le hle
    subst hle
    exact (WP.spec_ok _).mpr rfl
end

mutual
theorem le_eq_total : ∀ (a b : theory.LE), theory.le_eq a b ⦃ _r => True ⦄
  | .End, b => by unfold theory.le_eq; cases b <;> exact ok_true _
  | .Var x, b => by unfold theory.le_eq; cases b <;> exact ok_true _
  | .Rec x, b => by
    unfold theory.le_eq
    cases b with
    | Rec y => exact le_eq_total x y
    | End => exact ok_true _
    | Send _ _ => exact ok_true _
    | Recv _ _ => exact ok_true _
    | Var _ => exact ok_true _
  | .Send tx lx, b => by
    unfold theory.le_eq
    cases b with
    | Send ty ly =>
      by_cases e : tx = ty
      · simp only [if_pos e]; exact lse_eq_total lx ly
      · simp only [if_neg e]; exact ok_true _
    | End => exact ok_true _
    | Rec _ => exact ok_true _
    | Recv _ _ => exact ok_true _
    | Var _ => exact ok_true _
  | .Recv fx lx, b => by
    unfold theory.le_eq
    cases b with
    | Recv fy ly =>
      by_cases e : fx = fy
      · simp only [if_pos e]; exact lse_eq_total lx ly
      · simp only [if_neg e]; exact ok_true _
    | End => exact ok_true _
    | Rec _ => exact ok_true _
    | Send _ _ => exact ok_true _
    | Var _ => exact ok_true _
theorem lse_eq_total : ∀ (a b : theory.LsE), theory.lse_eq a b ⦃ _r => True ⦄
  | .Nil, b => by unfold theory.lse_eq; cases b <;> exact ok_true _
  | .Cons la xa ra, b => by
    unfold theory.lse_eq
    cases b with
    | Nil => exact ok_true _
    | Cons lb xb rb =>
      by_cases e : la = lb
      · simp only [if_pos e]
        apply WP.spec_bind (le_eq_total xa xb)
        intro b1 _
        cases b1 with
        | true => simp only [if_true]; exact lse_eq_total ra rb
        | false => simp only [Bool.false_eq_true, if_false]; exact ok_true _
      · simp only [if_neg e]; exact ok_true _
end

theorem lse_lookup_copy_total : ∀ (ls : theory.LsE) (label : U32),
    multiparty.lse_lookup_copy ls label ⦃ _r => True ⦄
  | .Nil, _ => by unfold multiparty.lse_lookup_copy; exact ok_true _
  | .Cons l c rest, label => by
    unfold multiparty.lse_lookup_copy
    by_cases e : l = label
    · simp only [if_pos e]
      apply WP.spec_bind (le_copy_spec c)
      intro l1 _
      exact ok_true _
    · simp only [if_neg e]
      exact lse_lookup_copy_total rest label

theorem lse_remove_total : ∀ (ls : theory.LsE) (label : U32),
    multiparty.lse_remove ls label ⦃ _r => True ⦄
  | .Nil, _ => by unfold multiparty.lse_remove; exact ok_true _
  | .Cons l c rest, label => by
    unfold multiparty.lse_remove
    by_cases e : l = label
    · simp only [if_pos e]
      exact lse_remove_total rest label
    · simp only [if_neg e]
      apply WP.spec_bind (le_copy_spec c)
      intro l1 _
      apply WP.spec_bind (lse_remove_total rest label)
      intro le _
      exact ok_true _

theorem lse_copy_pub_total : ∀ (ls : theory.LsE), multiparty.lse_copy_pub ls ⦃ _r => True ⦄
  | .Nil => by unfold multiparty.lse_copy_pub; exact ok_true _
  | .Cons label l rest => by
    unfold multiparty.lse_copy_pub
    apply WP.spec_bind (le_copy_spec l)
    intro l1 _
    apply WP.spec_bind (lse_copy_pub_total rest)
    intro le _
    exact ok_true _

/-- The tail of `merge_exec` and of the functions below: an optional
result wrapped again. -/
theorem opt_wrap_total {α β} (f : α → β) (o : Option α) :
    (match o with
      | none => (ok none : Result (Option β))
      | some x => ok (some (f x))) ⦃ _r => True ⦄ := by
  cases o <;> exact ok_true _

mutual
theorem merge_exec_total : ∀ (a b : theory.LE), multiparty.merge_exec a b ⦃ _r => True ⦄
  | .End, b => by
    unfold multiparty.merge_exec
    apply WP.spec_bind (le_eq_total _ b)
    intro b1 _
    cases b1 with
    | true =>
      simp only [if_true]
      apply WP.spec_bind (le_copy_spec _)
      intro l _
      exact ok_true _
    | false => simp only [Bool.false_eq_true, if_false]; exact ok_true _
  | .Var n, b => by
    unfold multiparty.merge_exec
    apply WP.spec_bind (le_eq_total _ b)
    intro b1 _
    cases b1 with
    | true =>
      simp only [if_true]
      apply WP.spec_bind (le_copy_spec _)
      intro l _
      exact ok_true _
    | false => simp only [Bool.false_eq_true, if_false]; exact ok_true _
  | .Send t ls, b => by
    unfold multiparty.merge_exec
    apply WP.spec_bind (le_eq_total _ b)
    intro b1 _
    cases b1 with
    | true =>
      simp only [if_true]
      apply WP.spec_bind (le_copy_spec _)
      intro l _
      exact ok_true _
    | false => simp only [Bool.false_eq_true, if_false]; exact ok_true _
  | .Recv fa la, b => by
    unfold multiparty.merge_exec
    apply WP.spec_bind (le_eq_total _ b)
    intro b1 _
    cases b1 with
    | true =>
      simp only [if_true]
      apply WP.spec_bind (le_copy_spec _)
      intro l _
      exact ok_true _
    | false =>
      simp only [Bool.false_eq_true, if_false]
      cases b with
      | Recv fb lb =>
        by_cases e : fa = fb
        · simp only [if_pos e]
          apply WP.spec_bind (merge_ls_exec_total la lb)
          intro o _
          cases o <;> exact ok_true _
        · simp only [if_neg e]; exact ok_true _
      | End => exact ok_true _
      | Send _ _ => exact ok_true _
      | Rec _ => exact ok_true _
      | Var _ => exact ok_true _
  | .Rec ba, b => by
    unfold multiparty.merge_exec
    apply WP.spec_bind (le_eq_total _ b)
    intro b1 _
    cases b1 with
    | true =>
      simp only [if_true]
      apply WP.spec_bind (le_copy_spec _)
      intro l _
      exact ok_true _
    | false =>
      simp only [Bool.false_eq_true, if_false]
      cases b with
      | Rec bb =>
        apply WP.spec_bind (merge_exec_total ba bb)
        intro o _
        cases o <;> exact ok_true _
      | End => exact ok_true _
      | Send _ _ => exact ok_true _
      | Recv _ _ => exact ok_true _
      | Var _ => exact ok_true _
theorem merge_ls_exec_total : ∀ (a b : theory.LsE),
    multiparty.merge_ls_exec a b ⦃ _r => True ⦄
  | .Nil, b => by
    unfold multiparty.merge_ls_exec
    apply WP.spec_bind (lse_copy_pub_total b)
    intro le _
    exact ok_true _
  | .Cons label l rest, b => by
    unfold multiparty.merge_ls_exec
    apply WP.spec_bind (lse_lookup_copy_total b label)
    intro o _
    have htail : ∀ merged : Option theory.LE,
        (match merged with
          | none => (ok none : Result (Option theory.LsE))
          | some m => do
            let b2 ← multiparty.lse_remove b label
            let o1 ← multiparty.merge_ls_exec rest b2
            match o1 with
            | none => ok none
            | some r2 => ok (some (theory.LsE.Cons label m r2))) ⦃ _r => True ⦄ := by
      intro merged
      cases merged with
      | none => exact ok_true _
      | some m =>
        apply WP.spec_bind (lse_remove_total b label)
        intro b2 _
        apply WP.spec_bind (merge_ls_exec_total rest b2)
        intro o1 _
        cases o1 <;> exact ok_true _
    cases o with
    | none =>
      apply WP.spec_bind (WP.spec_bind (le_copy_spec l) (fun l1 _ => ok_true (some l1)))
      intro merged _
      exact htail merged
    | some lb =>
      apply WP.spec_bind (merge_exec_total l lb)
      intro merged _
      exact htail merged
end

theorem usize_lt_max {k : Usize} (h : ¬ k = core.num.Usize.MAX) : k.val < Usize.max := by
  have h1 : k.val ≤ Usize.max := by scalar_tac
  have h2 : k.val ≠ Usize.max := fun e =>
    h (usize_val_inj (by rw [e]; exact (usize_max_val rfl).symm))
  omega

mutual
theorem lshift_exec_total : ∀ (l : theory.LE) (k : Usize),
    multiparty.lshift_exec l k ⦃ _r => True ⦄
  | .End, k => by unfold multiparty.lshift_exec; exact ok_true _
  | .Send t ls, k => by
    unfold multiparty.lshift_exec
    apply WP.spec_bind (lshift_ls_exec_total ls k)
    intro o _
    cases o <;> exact ok_true _
  | .Recv f ls, k => by
    unfold multiparty.lshift_exec
    apply WP.spec_bind (lshift_ls_exec_total ls k)
    intro o _
    cases o <;> exact ok_true _
  | .Rec body, k => by
    unfold multiparty.lshift_exec
    by_cases e : k = core.num.Usize.MAX
    · simp only [if_pos e]; exact ok_true _
    · simp only [if_neg e]
      have hk := usize_lt_max e
      apply WP.spec_bind (UScalar.add_spec (x := k) (y := 1#usize) (by scalar_tac))
      intro i _
      apply WP.spec_bind (lshift_exec_total body i)
      intro o _
      cases o <;> exact ok_true _
  | .Var n, k => by
    unfold multiparty.lshift_exec
    by_cases e : n > k
    · simp only [if_pos e]
      apply WP.spec_bind (UScalar.sub_spec (x := n) (y := 1#usize) (by scalar_tac))
      intro i _
      exact ok_true _
    · simp only [if_neg e]; exact ok_true _
theorem lshift_ls_exec_total : ∀ (ls : theory.LsE) (k : Usize),
    multiparty.lshift_ls_exec ls k ⦃ _r => True ⦄
  | .Nil, k => by unfold multiparty.lshift_ls_exec; exact ok_true _
  | .Cons label l rest, k => by
    unfold multiparty.lshift_ls_exec
    apply WP.spec_bind (lshift_exec_total l k)
    intro o _
    cases o with
    | none => exact ok_true _
    | some l2 =>
      apply WP.spec_bind (lshift_ls_exec_total rest k)
      intro o1 _
      cases o1 <;> exact ok_true _
end

mutual
theorem lfree_deep_total : ∀ (l : theory.LE), multiparty.lfree_deep l ⦃ _r => True ⦄
  | .End => by unfold multiparty.lfree_deep; exact ok_true _
  | .Var _ => by unfold multiparty.lfree_deep; exact ok_true _
  | .Rec body => by unfold multiparty.lfree_deep multiparty.lfree_deeper; exact ok_true _
  | .Send _ ls => by unfold multiparty.lfree_deep; exact lfree_ls_deep_total ls
  | .Recv _ ls => by unfold multiparty.lfree_deep; exact lfree_ls_deep_total ls
theorem lfree_ls_deep_total : ∀ (ls : theory.LsE), multiparty.lfree_ls_deep ls ⦃ _r => True ⦄
  | .Nil => by unfold multiparty.lfree_ls_deep; exact ok_true _
  | .Cons _ l rest => by
    unfold multiparty.lfree_ls_deep
    apply WP.spec_bind (lfree_deep_total l)
    intro b _
    cases b with
    | true => simp only [if_true]; exact ok_true _
    | false => simp only [Bool.false_eq_true, if_false]; exact lfree_ls_deep_total rest
end

mutual
theorem lfree_exec_total : ∀ (l : theory.LE) (k : Usize),
    multiparty.lfree_exec l k ⦃ _r => True ⦄
  | .End, k => by unfold multiparty.lfree_exec; exact ok_true _
  | .Var _, k => by unfold multiparty.lfree_exec; exact ok_true _
  | .Send _ ls, k => by unfold multiparty.lfree_exec; exact lfree_ls_exec_total ls k
  | .Recv _ ls, k => by unfold multiparty.lfree_exec; exact lfree_ls_exec_total ls k
  | .Rec body, k => by
    unfold multiparty.lfree_exec
    by_cases e : k = core.num.Usize.MAX
    · simp only [if_pos e]; exact lfree_deep_total body
    · simp only [if_neg e]
      have hk := usize_lt_max e
      apply WP.spec_bind (UScalar.add_spec (x := k) (y := 1#usize) (by scalar_tac))
      intro i _
      exact lfree_exec_total body i
theorem lfree_ls_exec_total : ∀ (ls : theory.LsE) (k : Usize),
    multiparty.lfree_ls_exec ls k ⦃ _r => True ⦄
  | .Nil, k => by unfold multiparty.lfree_ls_exec; exact ok_true _
  | .Cons _ l rest, k => by
    unfold multiparty.lfree_ls_exec
    apply WP.spec_bind (lfree_exec_total l k)
    intro b _
    cases b with
    | true => simp only [if_true]; exact ok_true _
    | false => simp only [Bool.false_eq_true, if_false]; exact lfree_ls_exec_total rest k
end

theorem lactionless_exec_total : ∀ (l : theory.LE), multiparty.lactionless_exec l ⦃ _r => True ⦄
  | .End => by unfold multiparty.lactionless_exec; exact ok_true _
  | .Var _ => by unfold multiparty.lactionless_exec; exact ok_true _
  | .Send _ _ => by unfold multiparty.lactionless_exec; exact ok_true _
  | .Recv _ _ => by unfold multiparty.lactionless_exec; exact ok_true _
  | .Rec body => by unfold multiparty.lactionless_exec; exact lactionless_exec_total body

theorem close_rec_exec_total (l : theory.LE) : multiparty.close_rec_exec l ⦃ _r => True ⦄ := by
  unfold multiparty.close_rec_exec
  apply WP.spec_bind (lactionless_exec_total l)
  intro b _
  cases b with
  | true =>
    simp only [if_true]
    cases l with
    | End => exact ok_true _
    | Send _ _ => exact ok_true _
    | Recv _ _ => exact ok_true _
    | Rec _ => exact ok_true _
    | Var n =>
      by_cases e : n = 0#usize
      · simp only [if_pos e]; exact ok_true _
      · simp only [if_neg e]
        have hn : 1 ≤ n.val := by
          have : n.val ≠ 0 := fun h0 => e (usize_val_inj (by rw [h0]; rfl))
          omega
        apply WP.spec_bind (UScalar.sub_spec (x := n) (y := 1#usize) (by scalar_tac))
        intro i _
        exact ok_true _
  | false =>
    simp only [Bool.false_eq_true, if_false]
    apply WP.spec_bind (lfree_exec_total l 0#usize)
    intro b1 _
    cases b1 with
    | true =>
      simp only [if_true]
      apply WP.spec_bind (le_copy_spec l)
      intro l1 _
      exact ok_true _
    | false =>
      simp only [Bool.false_eq_true, if_false]
      exact lshift_exec_total l 0#usize

mutual
theorem projn_exec_total : ∀ (g : theory.GE) (r : Usize),
    multiparty.projn_exec g r ⦃ _r => True ⦄
  | .End, r => by unfold multiparty.projn_exec; exact ok_true _
  | .Var n, r => by unfold multiparty.projn_exec; exact ok_true _
  | .Rec body, r => by
    unfold multiparty.projn_exec
    apply WP.spec_bind (projn_exec_total body r)
    intro o _
    cases o with
    | none => exact ok_true _
    | some l => exact close_rec_exec_total l
  | .Comm f t bs, r => by
    unfold multiparty.projn_exec
    by_cases e1 : r = f
    · simp only [if_pos e1]
      apply WP.spec_bind (projn_bs_exec_total bs r)
      intro o _
      cases o <;> exact ok_true _
    · simp only [if_neg e1]
      by_cases e2 : r = t
      · simp only [if_pos e2]
        apply WP.spec_bind (projn_bs_exec_total bs r)
        intro o _
        cases o <;> exact ok_true _
      · simp only [if_neg e2]
        exact projn_merge_exec_total bs r
theorem projn_bs_exec_total : ∀ (bs : theory.BE) (r : Usize),
    multiparty.projn_bs_exec bs r ⦃ _r => True ⦄
  | .Nil, r => by unfold multiparty.projn_bs_exec; exact ok_true _
  | .Cons label g rest, r => by
    unfold multiparty.projn_bs_exec
    apply WP.spec_bind (projn_exec_total g r)
    intro o _
    cases o with
    | none => exact ok_true _
    | some l =>
      apply WP.spec_bind (projn_bs_exec_total rest r)
      intro o1 _
      cases o1 <;> exact ok_true _
theorem projn_merge_exec_total : ∀ (bs : theory.BE) (r : Usize),
    multiparty.projn_merge_exec bs r ⦃ _r => True ⦄
  | .Nil, r => by unfold multiparty.projn_merge_exec; exact ok_true _
  | .Cons label g .Nil, r => by
    unfold multiparty.projn_merge_exec
    apply WP.spec_bind (projn_exec_total g r)
    intro o _
    cases o <;> exact ok_true _
  | .Cons label g (.Cons l2 g2 r2), r => by
    unfold multiparty.projn_merge_exec
    apply WP.spec_bind (projn_exec_total g r)
    intro o _
    cases o with
    | none => exact ok_true _
    | some l =>
      apply WP.spec_bind (projn_merge_exec_total (.Cons l2 g2 r2) r)
      intro o1 _
      cases o1 with
      | none => exact ok_true _
      | some m => exact merge_exec_total l m
end

/-- **The projection with the full merge on the code as compiled**: for
every Protocol and role `projn_exec` returns, and a local type it returns
is the projection `projn` of `Proto/Spec.lean` (the direction of its
Verus contract that the acceptance check uses; a refusal is not read). -/
theorem projn_exec_spec (g : theory.GE) (r : Usize) :
    ∃ o, multiparty.projn_exec g r = ok o ∧
      ∀ l, o = some l → projn (vG g) r.val = some (vL l) := by
  obtain ⟨o, ho, _⟩ := spec_returns (projn_exec_total g r)
  exact ⟨o, ho, fun l hl => projn_exec_ok g r l (by rw [ho, hl])⟩

end MP
