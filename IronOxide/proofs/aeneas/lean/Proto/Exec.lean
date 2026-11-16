import Proto.Progress
import Proto.Proto
open Aeneas Aeneas.Std Result
/-!
# The checker as compiled accepts only what `acceptn` accepts

`Proto/Proto.lean` is the Aeneas translation of `rsk-protocol`'s
multiparty acceptance check (`multiparty::accept_multiparty` and what it
calls) from the Verus-erased build. This file reads each extracted
function through the views of its types and shows: whenever
`accept_multiparty g n` returns `true`, the Protocol's view satisfies
`MP.acceptn`. With `MP.multiparty_progress` that is the multiparty
progress theorem for the checker as compiled
(`accept_multiparty_progress`).
-/

namespace MP
open rsk_protocol

/-! ## Helpers over the Aeneas primitives -/

theorem bind_eq_ok' {α β} (x : Result α) (f : α → Result β) (y : β) :
    Std.bind x f = ok y ↔ ∃ a, x = ok a ∧ f a = ok y := by
  cases x <;> simp

theorem ok_inj' {α} {x y : α} (h : (ok x : Result α) = ok y) : x = y := by
  simpa using h

theorem usize_add_ok' {a b c : Usize} (h : a + b = ok c) : c.val = a.val + b.val := by
  have := UScalar.add_equiv a b
  simp_all

theorem usize_sub_ok' {a b c : Usize} (h : a - b = ok c) : b.val ≤ a.val ∧ c.val = a.val - b.val := by
  have := UScalar.sub_equiv a b
  simp_all

theorem vec_push_ok {α} {v v' : alloc.vec.Vec α} {x : α} (h : v.push x = ok v') :
    v'.val = v.val ++ [x] := by
  by_cases hlen : v.val.length < Usize.max
  · have hs := alloc.vec.Vec.push_spec v x hlen
    rw [h] at hs
    simpa using hs
  · exfalso
    unfold alloc.vec.Vec.push at h
    simp only at h
    split at h
    · rename_i hc
      simp only [Bool.or_eq_true, decide_eq_true_eq] at hc
      scalar_tac
    · simp at h

/-! ## Views -/

mutual
def vL : theory.LE → L
  | .End => .fin
  | .Send t ls => .send t.val (vLs ls)
  | .Recv f ls => .recv f.val (vLs ls)
  | .Rec body => .mu (vL body)
  | .Var n => .var n.val
def vLs : theory.LsE → Ls
  | .Nil => .nil
  | .Cons label l rest => .cons label.val (vL l) (vLs rest)
end

mutual
def vG : theory.GE → G
  | .End => .fin
  | .Comm f t bs => .comm f.val t.val (vBs bs)
  | .Rec body => .mu (vG body)
  | .Var n => .var n.val
def vBs : theory.BE → Bs
  | .Nil => .nil
  | .Cons label g rest => .cons label.val (vG g) (vBs rest)
end

theorem usize_val_inj {a b : Usize} (h : a.val = b.val) : a = b := by scalar_tac

theorem u32_val_inj {a b : U32} (h : a.val = b.val) : a = b := by scalar_tac

mutual
theorem vL_inj : ∀ (a b : theory.LE), vL a = vL b → a = b
  | .End, b, h => by cases b <;> simp [vL] at h ⊢
  | .Send t ls, b, h => by
    cases b <;> simp [vL] at h ⊢
    rename_i t' ls'
    exact ⟨usize_val_inj h.1, vLs_inj ls ls' h.2⟩
  | .Recv f ls, b, h => by
    cases b <;> simp [vL] at h ⊢
    rename_i f' ls'
    exact ⟨usize_val_inj h.1, vLs_inj ls ls' h.2⟩
  | .Rec body, b, h => by
    cases b <;> simp [vL] at h ⊢
    rename_i body'
    exact vL_inj body body' h
  | .Var n, b, h => by
    cases b <;> simp [vL] at h ⊢
    exact usize_val_inj h
theorem vLs_inj : ∀ (a b : theory.LsE), vLs a = vLs b → a = b
  | .Nil, b, h => by cases b <;> simp [vLs] at h ⊢
  | .Cons label l rest, b, h => by
    cases b <;> simp [vLs] at h ⊢
    rename_i label' l' rest'
    exact ⟨u32_val_inj h.1, vL_inj l l' h.2.1, vLs_inj rest rest' h.2.2⟩
end

/-! ## Copies and equality -/

mutual
theorem le_copy_ok : ∀ (l l' : theory.LE), theory.le_copy l = ok l' → l' = l
  | .End, l', h => by unfold theory.le_copy at h; exact (ok_inj' h).symm
  | .Var n, l', h => by unfold theory.le_copy at h; exact (ok_inj' h).symm
  | .Send t ls, l', h => by
    unfold theory.le_copy at h
    simp only [bind_eq_ok'] at h
    obtain ⟨le, hle, h⟩ := h
    rw [← ok_inj' h, lse_copy_ok ls le hle]
  | .Recv f ls, l', h => by
    unfold theory.le_copy at h
    simp only [bind_eq_ok'] at h
    obtain ⟨le, hle, h⟩ := h
    rw [← ok_inj' h, lse_copy_ok ls le hle]
  | .Rec body, l', h => by
    unfold theory.le_copy at h
    simp only [bind_eq_ok'] at h
    obtain ⟨l1, hl1, h⟩ := h
    rw [← ok_inj' h, le_copy_ok body l1 hl1]
theorem lse_copy_ok : ∀ (ls ls' : theory.LsE), theory.lse_copy ls = ok ls' → ls' = ls
  | .Nil, ls', h => by unfold theory.lse_copy at h; exact (ok_inj' h).symm
  | .Cons label l rest, ls', h => by
    unfold theory.lse_copy at h
    simp only [bind_eq_ok'] at h
    obtain ⟨l1, hl1, le, hle, h⟩ := h
    rw [← ok_inj' h, le_copy_ok l l1 hl1, lse_copy_ok rest le hle]
end

mutual
theorem le_eq_ok : ∀ (a b : theory.LE) (r : Bool), theory.le_eq a b = ok r → (r = true ↔ a = b)
  | .End, b, r, h => by
    unfold theory.le_eq at h
    cases b <;> simp at h <;> subst h <;> simp
  | .Var x, b, r, h => by
    unfold theory.le_eq at h
    cases b <;> simp at h <;> subst h <;> simp
  | .Send tx lx, b, r, h => by
    unfold theory.le_eq at h
    cases b with
    | Send ty ly =>
      simp only at h
      by_cases e : tx = ty
      · rw [if_pos e] at h
        have := lse_eq_ok lx ly r h
        simp [e, this]
      · rw [if_neg e] at h
        have h := ok_inj' h
        subst h
        simp [e]
    | End => simp at h; subst h; simp
    | Recv _ _ => simp at h; subst h; simp
    | Rec _ => simp at h; subst h; simp
    | Var _ => simp at h; subst h; simp
  | .Recv fx lx, b, r, h => by
    unfold theory.le_eq at h
    cases b with
    | Recv fy ly =>
      simp only at h
      by_cases e : fx = fy
      · rw [if_pos e] at h
        have := lse_eq_ok lx ly r h
        simp [e, this]
      · rw [if_neg e] at h
        have h := ok_inj' h
        subst h
        simp [e]
    | End => simp at h; subst h; simp
    | Send _ _ => simp at h; subst h; simp
    | Rec _ => simp at h; subst h; simp
    | Var _ => simp at h; subst h; simp
  | .Rec x, b, r, h => by
    unfold theory.le_eq at h
    cases b with
    | Rec y =>
      simp only at h
      have := le_eq_ok x y r h
      simp [this]
    | End => simp at h; subst h; simp
    | Send _ _ => simp at h; subst h; simp
    | Recv _ _ => simp at h; subst h; simp
    | Var _ => simp at h; subst h; simp
theorem lse_eq_ok : ∀ (a b : theory.LsE) (r : Bool), theory.lse_eq a b = ok r →
    (r = true ↔ a = b)
  | .Nil, b, r, h => by
    unfold theory.lse_eq at h
    cases b <;> simp at h <;> subst h <;> simp
  | .Cons la xa ra, b, r, h => by
    unfold theory.lse_eq at h
    cases b with
    | Nil => simp at h; subst h; simp
    | Cons lb xb rb =>
      simp only at h
      by_cases e : la = lb
      · rw [if_pos e] at h
        simp only [bind_eq_ok'] at h
        obtain ⟨b1, hb1, h⟩ := h
        have i1 := le_eq_ok xa xb b1 hb1
        cases b1 with
        | true =>
          simp only [ite_true] at h
          have i2 := lse_eq_ok ra rb r h
          simp [e, i1.mp rfl, i2]
        | false =>
          simp only [Bool.false_eq_true, ite_false] at h
          have h := ok_inj' h
          subst h
          have : xa ≠ xb := fun e' => by simpa using i1.mpr e'
          simp [this]
      · rw [if_neg e] at h
        have h := ok_inj' h
        subst h
        simp [e]
end

theorem lse_lookup_copy_ok : ∀ (ls : theory.LsE) (label : U32) (o : Option theory.LE),
    multiparty.lse_lookup_copy ls label = ok o → o.map vL = (vLs ls).lookup label.val
  | .Nil, label, o, h => by
    unfold multiparty.lse_lookup_copy at h
    rw [← ok_inj' h]; rfl
  | .Cons l cont rest, label, o, h => by
    unfold multiparty.lse_lookup_copy at h
    simp only [vLs, Ls.lookup]
    by_cases e : l = label
    · rw [if_pos e] at h
      simp only [bind_eq_ok'] at h
      obtain ⟨l1, hl1, h⟩ := h
      rw [← ok_inj' h, le_copy_ok cont l1 hl1, if_pos (by rw [e])]
      rfl
    · rw [if_neg e] at h
      rw [if_neg (fun e' => e (u32_val_inj e'))]
      exact lse_lookup_copy_ok rest label o h

theorem lse_remove_ok : ∀ (ls : theory.LsE) (label : U32) (r : theory.LsE),
    multiparty.lse_remove ls label = ok r → vLs r = (vLs ls).remove label.val
  | .Nil, label, r, h => by
    unfold multiparty.lse_remove at h
    rw [← ok_inj' h]; rfl
  | .Cons l cont rest, label, r, h => by
    unfold multiparty.lse_remove at h
    simp only [vLs, Ls.remove]
    by_cases e : l = label
    · rw [if_pos e] at h
      rw [if_pos (by rw [e])]
      exact lse_remove_ok rest label r h
    · rw [if_neg e] at h
      rw [if_neg (fun e' => e (u32_val_inj e'))]
      simp only [bind_eq_ok'] at h
      obtain ⟨l1, hl1, le, hle, h⟩ := h
      rw [← ok_inj' h, le_copy_ok cont l1 hl1]
      simp only [vLs, lse_remove_ok rest label le hle]

theorem lse_copy_pub_ok : ∀ (ls ls' : theory.LsE), multiparty.lse_copy_pub ls = ok ls' →
    ls' = ls
  | .Nil, ls', h => by unfold multiparty.lse_copy_pub at h; exact (ok_inj' h).symm
  | .Cons label l rest, ls', h => by
    unfold multiparty.lse_copy_pub at h
    simp only [bind_eq_ok'] at h
    obtain ⟨l1, hl1, le, hle, h⟩ := h
    rw [← ok_inj' h, le_copy_ok l l1 hl1, lse_copy_pub_ok rest le hle]


/-! ## The merge -/

theorem merge_ne_mu {ba bb : L} (h : L.mu ba ≠ L.mu bb) :
    merge (.mu ba) (.mu bb) = (merge ba bb).map L.mu := by
  simp [merge, h]

theorem merge_ne_recv {fa fb : Nat} {la lb : Ls} (h : L.recv fa la ≠ L.recv fb lb) :
    merge (.recv fa la) (.recv fb lb) = if fa = fb then (mergeLs la lb).map (L.recv fa) else none := by
  simp [merge, h]

theorem merge_send_ne {t : Nat} {ls : Ls} {b : L} (h : L.send t ls ≠ b) :
    merge (.send t ls) b = none := by
  cases b <;> simp_all [merge]

theorem mergeLs_cons_eq (label : Nat) (l : L) (rest b : Ls) :
    mergeLs (.cons label l rest) b =
      (Option.elim (b.lookup label) (some l) (merge l)).bind
        (fun m => (mergeLs rest (b.remove label)).map (Ls.cons label m)) := by
  simp only [mergeLs]
  cases hb : b.lookup label with
  | none =>
    simp only [Option.elim]
    cases mergeLs rest (b.remove label) <;> rfl
  | some lb =>
    simp only [Option.elim]
    cases merge l lb with
    | none => rfl
    | some m => cases mergeLs rest (b.remove label) <;> rfl

/-- The first test of `merge_exec`: equal types merge to a copy. -/
theorem merge_exec_same {a b l : theory.LE} (hl : theory.le_copy a = ok l) (e : a = b) :
    (some l).map vL = merge (vL a) (vL b) := by
  rw [le_copy_ok a l hl, ← e]
  simp [merge_self]

mutual
theorem merge_exec_ok : ∀ (a b : theory.LE) (o : Option theory.LE),
    multiparty.merge_exec a b = ok o → o.map vL = merge (vL a) (vL b)
  | .End, b, o, h => by
    unfold multiparty.merge_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨b1, hb1, h⟩ := h
    have heq := le_eq_ok _ b b1 hb1
    cases b1 with
    | true =>
      simp only [ite_true, bind_eq_ok'] at h
      obtain ⟨l, hl, h⟩ := h
      rw [← ok_inj' h]
      exact merge_exec_same hl (heq.mp rfl)
    | false =>
      simp only [Bool.false_eq_true, ite_false] at h
      have hne : theory.LE.End ≠ b := fun e => by simpa using heq.mpr e
      rw [← ok_inj' h]
      cases b <;> simp_all [vL, merge]
  | .Send t ls, b, o, h => by
    unfold multiparty.merge_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨b1, hb1, h⟩ := h
    have heq := le_eq_ok _ b b1 hb1
    cases b1 with
    | true =>
      simp only [ite_true, bind_eq_ok'] at h
      obtain ⟨l, hl, h⟩ := h
      rw [← ok_inj' h]
      exact merge_exec_same hl (heq.mp rfl)
    | false =>
      simp only [Bool.false_eq_true, ite_false] at h
      have hvne : vL (.Send t ls) ≠ vL b := fun e =>
        (by simpa using heq.mpr (vL_inj _ _ e) : False)
      rw [← ok_inj' h]
      simp only [vL] at hvne ⊢
      rw [merge_send_ne hvne]
      rfl
  | .Var n, b, o, h => by
    unfold multiparty.merge_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨b1, hb1, h⟩ := h
    have heq := le_eq_ok _ b b1 hb1
    cases b1 with
    | true =>
      simp only [ite_true, bind_eq_ok'] at h
      obtain ⟨l, hl, h⟩ := h
      rw [← ok_inj' h]
      exact merge_exec_same hl (heq.mp rfl)
    | false =>
      simp only [Bool.false_eq_true, ite_false] at h
      have hvne : vL (.Var n) ≠ vL b := fun e =>
        (by simpa using heq.mpr (vL_inj _ _ e) : False)
      rw [← ok_inj' h]
      cases b <;> simp_all [vL, merge]
  | .Recv fa la, b, o, h => by
    unfold multiparty.merge_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨b1, hb1, h⟩ := h
    have heq := le_eq_ok _ b b1 hb1
    cases b1 with
    | true =>
      simp only [ite_true, bind_eq_ok'] at h
      obtain ⟨l, hl, h⟩ := h
      rw [← ok_inj' h]
      exact merge_exec_same hl (heq.mp rfl)
    | false =>
      simp only [Bool.false_eq_true, ite_false] at h
      have hvne : vL (.Recv fa la) ≠ vL b := fun e =>
        (by simpa using heq.mpr (vL_inj _ _ e) : False)
      cases b with
      | Recv fb lb =>
        simp only [vL] at hvne ⊢
        rw [merge_ne_recv hvne]
        simp only at h
        by_cases e : fa = fb
        · rw [if_pos e] at h
          rw [if_pos (by rw [e])]
          simp only [bind_eq_ok'] at h
          obtain ⟨o1, ho1, h⟩ := h
          have ih := merge_ls_exec_ok la lb o1 ho1
          cases o1 with
          | none => rw [← ok_inj' h, ← ih]; rfl
          | some ls => rw [← ok_inj' h, ← ih]; rfl
        · rw [if_neg e] at h
          rw [if_neg (fun e' => e (usize_val_inj e')), ← ok_inj' h]
          rfl
      | End => simp only at h; rw [← ok_inj' h]; simp [vL, merge]
      | Send _ _ => simp only at h; rw [← ok_inj' h]; simp [vL, merge]
      | Rec _ => simp only at h; rw [← ok_inj' h]; simp [vL, merge]
      | Var _ => simp only at h; rw [← ok_inj' h]; simp [vL, merge]
  | .Rec ba, b, o, h => by
    unfold multiparty.merge_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨b1, hb1, h⟩ := h
    have heq := le_eq_ok _ b b1 hb1
    cases b1 with
    | true =>
      simp only [ite_true, bind_eq_ok'] at h
      obtain ⟨l, hl, h⟩ := h
      rw [← ok_inj' h]
      exact merge_exec_same hl (heq.mp rfl)
    | false =>
      simp only [Bool.false_eq_true, ite_false] at h
      have hvne : vL (.Rec ba) ≠ vL b := fun e =>
        (by simpa using heq.mpr (vL_inj _ _ e) : False)
      cases b with
      | Rec bb =>
        simp only [vL] at hvne ⊢
        rw [merge_ne_mu hvne]
        simp only [bind_eq_ok'] at h
        obtain ⟨o1, ho1, h⟩ := h
        have ih := merge_exec_ok ba bb o1 ho1
        cases o1 with
        | none => rw [← ok_inj' h, ← ih]; rfl
        | some m => rw [← ok_inj' h, ← ih]; rfl
      | End => simp only at h; rw [← ok_inj' h]; simp [vL, merge]
      | Send _ _ => simp only at h; rw [← ok_inj' h]; simp [vL, merge]
      | Recv _ _ => simp only at h; rw [← ok_inj' h]; simp [vL, merge]
      | Var _ => simp only at h; rw [← ok_inj' h]; simp [vL, merge]
theorem merge_ls_exec_ok : ∀ (a b : theory.LsE) (o : Option theory.LsE),
    multiparty.merge_ls_exec a b = ok o → o.map vLs = mergeLs (vLs a) (vLs b)
  | .Nil, b, o, h => by
    unfold multiparty.merge_ls_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨le, hle, h⟩ := h
    rw [← ok_inj' h, lse_copy_pub_ok b le hle]
    simp [vLs, mergeLs]
  | .Cons label l rest, b, o, h => by
    unfold multiparty.merge_ls_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨o0, ho0, merged, hmerged, h⟩ := h
    have hlook := lse_lookup_copy_ok b label o0 ho0
    have hm : merged.map vL
        = Option.elim ((vLs b).lookup label.val) (some (vL l)) (merge (vL l)) := by
      rw [← hlook]
      cases o0 with
      | none =>
        simp only [bind_eq_ok'] at hmerged
        obtain ⟨l1, hl1, hmerged⟩ := hmerged
        rw [← ok_inj' hmerged, le_copy_ok l l1 hl1]
        rfl
      | some lb => exact merge_exec_ok l lb merged hmerged
    simp only [vLs]
    rw [mergeLs_cons_eq, ← hm]
    cases merged with
    | none =>
      rw [← ok_inj' h]
      rfl
    | some m =>
      simp only [bind_eq_ok'] at h
      obtain ⟨b2, hb2, o1, ho1, h⟩ := h
      have ih := merge_ls_exec_ok rest b2 o1 ho1
      rw [lse_remove_ok b label b2 hb2] at ih
      simp only [Option.map_some, Option.bind_some]
      rw [← ih]
      cases o1 with
      | none => rw [← ok_inj' h]; rfl
      | some r2 => rw [← ok_inj' h]; rfl
end


/-! ## Actions, free variables, shifts, the recursion rule -/

theorem lactionless_exec_ok : ∀ (l : theory.LE) (b : Bool),
    multiparty.lactionless_exec l = ok b → b = (vL l).actionless
  | .End, b, h => by unfold multiparty.lactionless_exec at h; rw [← ok_inj' h]; rfl
  | .Var _, b, h => by unfold multiparty.lactionless_exec at h; rw [← ok_inj' h]; rfl
  | .Send _ _, b, h => by unfold multiparty.lactionless_exec at h; rw [← ok_inj' h]; rfl
  | .Recv _ _, b, h => by unfold multiparty.lactionless_exec at h; rw [← ok_inj' h]; rfl
  | .Rec body, b, h => by
    unfold multiparty.lactionless_exec at h
    simp only [vL, L.actionless]
    exact lactionless_exec_ok body b h

mutual
/-- No variable of an executable type is beyond the machine word. -/
theorem vL_free_big : ∀ (l : theory.LE) (K : Nat), Usize.max < K → (vL l).free K = false
  | .End, _, _ => rfl
  | .Var n, K, hK => by
    simp only [vL, L.free, beq_eq_false_iff_ne, ne_eq]
    have := n.hBounds
    scalar_tac
  | .Send _ ls, K, hK => by simp only [vL, L.free, vLs_free_big ls K hK]
  | .Recv _ ls, K, hK => by simp only [vL, L.free, vLs_free_big ls K hK]
  | .Rec body, K, hK => by simp only [vL, L.free, vL_free_big body (K + 1) (by omega)]
theorem vLs_free_big : ∀ (ls : theory.LsE) (K : Nat), Usize.max < K → (vLs ls).free K = false
  | .Nil, _, _ => rfl
  | .Cons _ l rest, K, hK => by
    simp only [vLs, Ls.free, vL_free_big l K hK, vLs_free_big rest K hK]
    rfl
end

mutual
theorem lfree_deep_ok : ∀ (l : theory.LE) (b : Bool), multiparty.lfree_deep l = ok b → b = false
  | .End, b, h => by unfold multiparty.lfree_deep at h; exact (ok_inj' h).symm
  | .Var _, b, h => by unfold multiparty.lfree_deep at h; exact (ok_inj' h).symm
  | .Send _ ls, b, h => by unfold multiparty.lfree_deep at h; exact lfree_ls_deep_ok ls b h
  | .Recv _ ls, b, h => by unfold multiparty.lfree_deep at h; exact lfree_ls_deep_ok ls b h
  | .Rec body, b, h => by
    unfold multiparty.lfree_deep at h
    unfold multiparty.lfree_deeper at h
    exact (ok_inj' h).symm
theorem lfree_ls_deep_ok : ∀ (ls : theory.LsE) (b : Bool),
    multiparty.lfree_ls_deep ls = ok b → b = false
  | .Nil, b, h => by unfold multiparty.lfree_ls_deep at h; exact (ok_inj' h).symm
  | .Cons _ l rest, b, h => by
    unfold multiparty.lfree_ls_deep at h
    simp only [bind_eq_ok'] at h
    obtain ⟨b1, hb1, h⟩ := h
    rw [lfree_deep_ok l b1 hb1] at h
    simp only [Bool.false_eq_true, ite_false] at h
    exact lfree_ls_deep_ok rest b h
end

theorem usize_max_val {k : Usize} (h : k = core.num.Usize.MAX) : k.val = Usize.max := by
  subst h; rfl

mutual
theorem lfree_exec_ok : ∀ (l : theory.LE) (k : Usize) (b : Bool),
    multiparty.lfree_exec l k = ok b → b = (vL l).free k.val
  | .End, k, b, h => by unfold multiparty.lfree_exec at h; rw [← ok_inj' h]; rfl
  | .Var n, k, b, h => by
    unfold multiparty.lfree_exec at h
    rw [← ok_inj' h]
    simp only [vL, L.free]
    by_cases e : n = k
    · simp [e]
    · have : n.val ≠ k.val := fun e' => e (usize_val_inj e')
      simp [e, this]
  | .Send _ ls, k, b, h => by
    unfold multiparty.lfree_exec at h
    simp only [vL, L.free]
    exact lfree_ls_exec_ok ls k b h
  | .Recv _ ls, k, b, h => by
    unfold multiparty.lfree_exec at h
    simp only [vL, L.free]
    exact lfree_ls_exec_ok ls k b h
  | .Rec body, k, b, h => by
    unfold multiparty.lfree_exec at h
    simp only [vL, L.free]
    by_cases e : k = core.num.Usize.MAX
    · rw [if_pos e] at h
      rw [lfree_deep_ok body b h, vL_free_big body (k.val + 1) (by rw [usize_max_val e]; omega)]
    · rw [if_neg e] at h
      simp only [bind_eq_ok'] at h
      obtain ⟨i, hi, h⟩ := h
      have := usize_add_ok' hi
      have h1 : (1#usize).val = 1 := rfl
      rw [h1] at this
      rw [← this]
      exact lfree_exec_ok body i b h
theorem lfree_ls_exec_ok : ∀ (ls : theory.LsE) (k : Usize) (b : Bool),
    multiparty.lfree_ls_exec ls k = ok b → b = (vLs ls).free k.val
  | .Nil, k, b, h => by unfold multiparty.lfree_ls_exec at h; rw [← ok_inj' h]; rfl
  | .Cons _ l rest, k, b, h => by
    unfold multiparty.lfree_ls_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨b1, hb1, h⟩ := h
    have i1 := lfree_exec_ok l k b1 hb1
    simp only [vLs, Ls.free, ← i1]
    cases b1 with
    | true =>
      simp only [ite_true] at h
      rw [← ok_inj' h]; rfl
    | false =>
      simp only [Bool.false_eq_true, ite_false] at h
      rw [lfree_ls_exec_ok rest k b h]
      rfl
end

mutual
theorem lshift_exec_ok : ∀ (l : theory.LE) (k : Usize) (r : theory.LE),
    multiparty.lshift_exec l k = ok (some r) → vL r = (vL l).shift k.val
  | .End, k, r, h => by
    unfold multiparty.lshift_exec at h
    have := ok_inj' h
    cases this
    rfl
  | .Var n, k, r, h => by
    unfold multiparty.lshift_exec at h
    simp only [vL, L.shift]
    by_cases e : n > k
    · rw [if_pos e] at h
      simp only [bind_eq_ok'] at h
      obtain ⟨i, hi, h⟩ := h
      have hs := usize_sub_ok' hi
      have h1 : (1#usize).val = 1 := rfl
      rw [h1] at hs
      have := ok_inj' h
      cases this
      rw [if_pos (by scalar_tac)]
      simp only [vL, hs.2]
    · rw [if_neg e] at h
      have := ok_inj' h
      cases this
      rw [if_neg (by scalar_tac)]
      rfl
  | .Send t ls, k, r, h => by
    unfold multiparty.lshift_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨o, ho, h⟩ := h
    cases o with
    | none => simp at h
    | some ls2 =>
      have := ok_inj' h
      cases this
      simp only [vL, L.shift, lshift_ls_exec_ok ls k ls2 ho]
  | .Recv f ls, k, r, h => by
    unfold multiparty.lshift_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨o, ho, h⟩ := h
    cases o with
    | none => simp at h
    | some ls2 =>
      have := ok_inj' h
      cases this
      simp only [vL, L.shift, lshift_ls_exec_ok ls k ls2 ho]
  | .Rec body, k, r, h => by
    unfold multiparty.lshift_exec at h
    by_cases e : k = core.num.Usize.MAX
    · rw [if_pos e] at h
      simp at h
    · rw [if_neg e] at h
      simp only [bind_eq_ok'] at h
      obtain ⟨i, hi, o, ho, h⟩ := h
      have hadd := usize_add_ok' hi
      have h1 : (1#usize).val = 1 := rfl
      rw [h1] at hadd
      cases o with
      | none => simp at h
      | some b =>
        have := ok_inj' h
        cases this
        simp only [vL, L.shift, lshift_exec_ok body i b ho, hadd]
theorem lshift_ls_exec_ok : ∀ (ls : theory.LsE) (k : Usize) (r : theory.LsE),
    multiparty.lshift_ls_exec ls k = ok (some r) → vLs r = (vLs ls).shift k.val
  | .Nil, k, r, h => by
    unfold multiparty.lshift_ls_exec at h
    have := ok_inj' h
    cases this
    rfl
  | .Cons label l rest, k, r, h => by
    unfold multiparty.lshift_ls_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨o, ho, h⟩ := h
    cases o with
    | none => simp at h
    | some l2 =>
      simp only [bind_eq_ok'] at h
      obtain ⟨o1, ho1, h⟩ := h
      cases o1 with
      | none => simp at h
      | some r2 =>
        have := ok_inj' h
        cases this
        simp only [vLs, Ls.shift, lshift_exec_ok l k l2 ho, lshift_ls_exec_ok rest k r2 ho1]
end

/-- The recursion rule as compiled is `closeRec`. -/
theorem close_rec_exec_ok (l r : theory.LE) (h : multiparty.close_rec_exec l = ok (some r)) :
    vL r = closeRec (vL l) := by
  unfold multiparty.close_rec_exec at h
  simp only [bind_eq_ok'] at h
  obtain ⟨b, hb, h⟩ := h
  have hact := lactionless_exec_ok l b hb
  unfold closeRec
  cases b with
  | true =>
    simp only [ite_true] at h
    rw [if_pos hact.symm]
    cases l with
    | End => have := ok_inj' h; cases this; rfl
    | Send _ _ => have := ok_inj' h; cases this; rfl
    | Recv _ _ => have := ok_inj' h; cases this; rfl
    | Rec _ => have := ok_inj' h; cases this; rfl
    | Var n =>
      simp only at h
      simp only [vL]
      by_cases e : n = 0#usize
      · rw [if_pos e] at h
        have := ok_inj' h
        cases this
        subst e
        rfl
      · rw [if_neg e] at h
        simp only [bind_eq_ok'] at h
        obtain ⟨i, hi, h⟩ := h
        have hs := usize_sub_ok' hi
        have h1 : (1#usize).val = 1 := rfl
        rw [h1] at hs
        have := ok_inj' h
        cases this
        rw [if_neg (by scalar_tac)]
        simp only [vL, hs.2]
  | false =>
    simp only [Bool.false_eq_true, ite_false, bind_eq_ok'] at h
    rw [if_neg (by rw [← hact]; simp)]
    obtain ⟨b1, hb1, h⟩ := h
    have hfree := lfree_exec_ok l 0#usize b1 hb1
    have h0 : (0#usize).val = 0 := rfl
    rw [h0] at hfree
    cases b1 with
    | true =>
      simp only [ite_true, bind_eq_ok'] at h
      obtain ⟨l1, hl1, h⟩ := h
      have := ok_inj' h
      cases this
      rw [if_pos hfree.symm, le_copy_ok l l1 hl1]
      rfl
    | false =>
      simp only [Bool.false_eq_true, ite_false] at h
      rw [if_neg (by rw [← hfree]; simp)]
      have := lshift_exec_ok l 0#usize r h
      rwa [h0] at this


/-! ## The projection -/

theorem projnMerge_cons_cons (label l2 : Nat) (g g2 : G) (r2 : Bs) (r : Nat) :
    projnMerge (.cons label g (.cons l2 g2 r2)) r
      = (projn g r).bind (fun l => (projnMerge (.cons l2 g2 r2) r).bind (merge l)) := by
  conv_lhs => unfold projnMerge
  cases projn g r <;> rfl


mutual
theorem projn_exec_ok : ∀ (g : theory.GE) (r : Usize) (l : theory.LE),
    multiparty.projn_exec g r = ok (some l) → projn (vG g) r.val = some (vL l)
  | .End, r, l, h => by
    unfold multiparty.projn_exec at h
    have := ok_inj' h
    cases this
    rfl
  | .Var n, r, l, h => by
    unfold multiparty.projn_exec at h
    have := ok_inj' h
    cases this
    rfl
  | .Rec body, r, l, h => by
    unfold multiparty.projn_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨o, ho, h⟩ := h
    cases o with
    | none => simp at h
    | some lb =>
      simp only at h
      simp only [vG, projn, projn_exec_ok body r lb ho, Option.map_some,
        close_rec_exec_ok lb l h]
  | .Comm f t bs, r, l, h => by
    unfold multiparty.projn_exec at h
    simp only [vG, projn]
    by_cases e1 : r = f
    · rw [if_pos e1] at h
      rw [if_pos (by rw [e1])]
      simp only [bind_eq_ok'] at h
      obtain ⟨o, ho, h⟩ := h
      cases o with
      | none => simp at h
      | some ls =>
        have := ok_inj' h
        cases this
        simp only [projn_bs_exec_ok bs r ls ho, Option.map_some, vL]
    · rw [if_neg e1] at h
      rw [if_neg (fun e' => e1 (usize_val_inj e'))]
      by_cases e2 : r = t
      · rw [if_pos e2] at h
        rw [if_pos (by rw [e2])]
        simp only [bind_eq_ok'] at h
        obtain ⟨o, ho, h⟩ := h
        cases o with
        | none => simp at h
        | some ls =>
          have := ok_inj' h
          cases this
          simp only [projn_bs_exec_ok bs r ls ho, Option.map_some, vL]
      · rw [if_neg e2] at h
        rw [if_neg (fun e' => e2 (usize_val_inj e'))]
        exact projn_merge_exec_ok bs r l h
theorem projn_bs_exec_ok : ∀ (bs : theory.BE) (r : Usize) (ls : theory.LsE),
    multiparty.projn_bs_exec bs r = ok (some ls) → projnBs (vBs bs) r.val = some (vLs ls)
  | .Nil, r, ls, h => by
    unfold multiparty.projn_bs_exec at h
    have := ok_inj' h
    cases this
    rfl
  | .Cons label g rest, r, ls, h => by
    unfold multiparty.projn_bs_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨o, ho, h⟩ := h
    cases o with
    | none => simp at h
    | some l =>
      simp only [bind_eq_ok'] at h
      obtain ⟨o1, ho1, h⟩ := h
      cases o1 with
      | none => simp at h
      | some ls' =>
        have := ok_inj' h
        cases this
        simp only [vBs, projnBs, projn_exec_ok g r l ho, projn_bs_exec_ok rest r ls' ho1, vLs]
theorem projn_merge_exec_ok : ∀ (bs : theory.BE) (r : Usize) (m : theory.LE),
    multiparty.projn_merge_exec bs r = ok (some m) → projnMerge (vBs bs) r.val = some (vL m)
  | .Nil, r, m, h => by
    unfold multiparty.projn_merge_exec at h
    simp at h
  | .Cons label g rest, r, m, h => by
    unfold multiparty.projn_merge_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨o, ho, h⟩ := h
    cases o with
    | none => simp at h
    | some l =>
      have hl := projn_exec_ok g r l ho
      cases rest with
      | Nil =>
        simp only at h
        have := ok_inj' h
        cases this
        simp only [vBs, projnMerge, hl]
      | Cons l2 g2 r2 =>
        simp only [bind_eq_ok'] at h
        obtain ⟨o1, ho1, h⟩ := h
        cases o1 with
        | none => simp at h
        | some mr =>
          simp only at h
          have ih := projn_merge_exec_ok (.Cons l2 g2 r2) r mr ho1
          have hm := merge_exec_ok l mr (some m) h
          simp only [vBs] at ih ⊢
          rw [projnMerge_cons_cons, hl]
          simp only [Option.bind_some]
          rw [ih]
          simpa using hm.symm
end

/-! ## Well-formedness -/

theorem be_has_label_exec_ok : ∀ (bs : theory.BE) (label : U32) (b : Bool),
    multiparty.be_has_label_exec bs label = ok b → b = (vBs bs).hasLabel label.val
  | .Nil, label, b, h => by
    unfold multiparty.be_has_label_exec at h
    rw [← ok_inj' h]; rfl
  | .Cons l g rest, label, b, h => by
    unfold multiparty.be_has_label_exec at h
    simp only [vBs, Bs.hasLabel]
    by_cases e : l = label
    · rw [if_pos e] at h
      rw [← ok_inj' h, e]
      simp
    · rw [if_neg e] at h
      have : l.val ≠ label.val := fun e' => e (u32_val_inj e')
      rw [be_has_label_exec_ok rest label b h]
      simp [this]

theorem be_distinct_exec_ok : ∀ (bs : theory.BE) (b : Bool),
    multiparty.be_distinct_exec bs = ok b → b = (vBs bs).distinct
  | .Nil, b, h => by
    unfold multiparty.be_distinct_exec at h
    rw [← ok_inj' h]; rfl
  | .Cons label g rest, b, h => by
    unfold multiparty.be_distinct_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨b1, hb1, h⟩ := h
    have i1 := be_has_label_exec_ok rest label b1 hb1
    simp only [vBs, Bs.distinct, ← i1]
    cases b1 with
    | true =>
      simp only [ite_true] at h
      rw [← ok_inj' h]; rfl
    | false =>
      simp only [Bool.false_eq_true, ite_false] at h
      rw [be_distinct_exec_ok rest b h]
      rfl

theorem be_len_ge_one_ok (bs : theory.BE) (b : Bool) (h : multiparty.be_len_ge_one bs = ok b) :
    b = decide (1 ≤ (vBs bs).len) := by
  unfold multiparty.be_len_ge_one at h
  cases bs with
  | Nil => simp only at h; rw [← ok_inj' h]; simp [vBs, Bs.len]
  | Cons _ _ _ => simp only at h; rw [← ok_inj' h]; simp [vBs, Bs.len]

/-- The loop that builds the guard environment of a communication: as many
`true` flags as the environment has flags. -/
theorem wfn_loop0_ok (env : alloc.vec.Vec Bool) : ∀ (k : Nat) (guarded : alloc.vec.Vec Bool)
    (i : Usize) (r : alloc.vec.Vec Bool), env.val.length - i.val = k → i.val ≤ env.val.length →
    guarded.val = List.replicate i.val true →
    multiparty.wfn_exec_loop0 env guarded i = ok r →
    r.val = List.replicate env.val.length true := by
  intro k
  induction k with
  | zero =>
    intro guarded i r hk hi hg h
    unfold multiparty.wfn_exec_loop0 loop at h
    simp only [multiparty.wfn_exec_loop0.body] at h
    have hlt : ¬ i < alloc.vec.Vec.len env := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj' h
    subst h
    rw [hg]
    congr 1
    omega
  | succ k ih =>
    intro guarded i r hk hi hg h
    unfold multiparty.wfn_exec_loop0 loop at h
    simp only [multiparty.wfn_exec_loop0.body] at h
    have hlt : i < alloc.vec.Vec.len env := by scalar_tac
    simp only [hlt, ite_true, bind_eq_ok'] at h
    obtain ⟨cf, ⟨g1, hg1, i2, hi2, hcf⟩, hloop⟩ := h
    have hcf := ok_inj' hcf
    subst hcf
    have hadd := usize_add_ok' hi2
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at hadd
    have hg1v : g1.val = guarded.val ++ [true] := vec_push_ok hg1
    refine ih g1 i2 r (by omega) (by omega) ?_ ?_
    · rw [hg1v, hg, hadd, List.replicate_succ']
    · unfold multiparty.wfn_exec_loop0
      exact hloop


theorem vec_index_ok {env : alloc.vec.Vec Bool} {i : Usize} {b : Bool}
    (h : alloc.vec.Vec.index (core.slice.index.SliceIndexUsizeSlice Bool) env i = ok b) :
    i.val < env.val.length ∧ env.val[i.val]? = some b := by
  simp only [alloc.vec.Vec.index, core.slice.index.Usize.index] at h
  unfold Slice.index_usize at h
  split at h
  · simp at h
  · rename_i x hx
    have := ok_inj' h
    subst this
    simp only [Slice.getElem?_Usize_eq] at hx
    have hx' : env.val[i.val]? = some x := hx
    refine ⟨?_, hx'⟩
    by_contra hc
    rw [List.getElem?_eq_none (by omega)] at hx'
    cases hx'

/-- The loop that copies the guard environment for a recursion. -/
theorem wfn_loop1_ok (env : alloc.vec.Vec Bool) : ∀ (k : Nat) (inner : alloc.vec.Vec Bool)
    (i : Usize) (r : alloc.vec.Vec Bool), env.val.length - i.val = k → i.val ≤ env.val.length →
    inner.val = env.val.take i.val →
    multiparty.wfn_exec_loop1 env inner i = ok r → r.val = env.val := by
  intro k
  induction k with
  | zero =>
    intro inner i r hk hi hg h
    unfold multiparty.wfn_exec_loop1 loop at h
    simp only [multiparty.wfn_exec_loop1.body] at h
    have hlt : ¬ i < alloc.vec.Vec.len env := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj' h
    subst h
    rw [hg, List.take_of_length_le (by omega)]
  | succ k ih =>
    intro inner i r hk hi hg h
    unfold multiparty.wfn_exec_loop1 loop at h
    simp only [multiparty.wfn_exec_loop1.body] at h
    have hlt : i < alloc.vec.Vec.len env := by scalar_tac
    simp only [hlt, ite_true, bind_eq_ok'] at h
    obtain ⟨cf, ⟨b, hb, g1, hg1, i2, hi2, hcf⟩, hloop⟩ := h
    have hcf := ok_inj' hcf
    subst hcf
    have hadd := usize_add_ok' hi2
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at hadd
    obtain ⟨hib, hbv⟩ := vec_index_ok hb
    refine ih g1 i2 r (by omega) (by omega) ?_ ?_
    · rw [vec_push_ok hg1, hg, hadd, List.take_succ, hbv]
      rfl
    · unfold multiparty.wfn_exec_loop1
      exact hloop

theorem allTrue_eq_replicate (l : List Bool) : allTrue l = List.replicate l.length true := by
  simp [allTrue, List.map_const']

mutual
/-- The well-formedness check as compiled decides `wfnAt`, the checker's
environment (outermost recursion first) being the specification's read
from the other side. -/
theorem wfn_exec_ok : ∀ (g : theory.GE) (n : Usize) (env : alloc.vec.Vec Bool) (b : Bool),
    multiparty.wfn_exec g n env = ok b → b = (vG g).wfnAt n.val env.val.reverse
  | .End, n, env, b, h => by
    unfold multiparty.wfn_exec at h
    rw [← ok_inj' h]; rfl
  | .Var v, n, env, b, h => by
    unfold multiparty.wfn_exec at h
    simp only [vG, G.wfnAt]
    by_cases e : v < alloc.vec.Vec.len env
    · simp only [e, ite_true, bind_eq_ok'] at h
      obtain ⟨i2, hi2, i3, hi3, h⟩ := h
      have h2 := usize_sub_ok' hi2
      have h3 := usize_sub_ok' hi3
      have h1 : (1#usize).val = 1 := rfl
      rw [h1] at h2
      have hlen : (alloc.vec.Vec.len env).val = env.val.length := by simp
      rw [hlen] at h2
      obtain ⟨_, hbv⟩ := vec_index_ok h
      have hv : v.val < env.val.length := by scalar_tac
      rw [List.getD_eq_getElem?_getD, List.getElem?_reverse (by simpa using hv)]
      have : env.val.length - 1 - v.val = i3.val := by omega
      rw [this, hbv]
      rfl
    · simp only [e, ite_false] at h
      rw [← ok_inj' h]
      have hv : env.val.reverse.length ≤ v.val := by simp; scalar_tac
      rw [getD_of_ge _ hv]
  | .Rec body, n, env, b, h => by
    unfold multiparty.wfn_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨inner, hinner, inner1, hinner1, h⟩ := h
    have hi := wfn_loop1_ok env _ (alloc.vec.Vec.new Bool) 0#usize inner rfl (by simp)
      (by simp) hinner
    have ih := wfn_exec_ok body n inner1 b h
    rw [vec_push_ok hinner1, hi, List.reverse_append] at ih
    simpa [vG, G.wfnAt] using ih
  | .Comm f t bs, n, env, b, h => by
    unfold multiparty.wfn_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨roles, hroles, some1, hsome1, distinct, hdistinct, h⟩ := h
    have hr : roles = (decide (f.val < n.val) && decide (t.val < n.val) && decide (f.val ≠ t.val)) := by
      by_cases e1 : f < n
      · rw [if_pos e1] at hroles
        by_cases e2 : t < n
        · rw [if_pos e2] at hroles
          rw [← ok_inj' hroles]
          have h1 : f.val < n.val := by scalar_tac
          have h2 : t.val < n.val := by scalar_tac
          by_cases e3 : f = t
          · subst e3; simp [h1]
          · have : f.val ≠ t.val := fun e' => e3 (usize_val_inj e')
            simp [h1, h2, this, e3]
        · rw [if_neg e2] at hroles
          rw [← ok_inj' hroles]
          have h2 : ¬ t.val < n.val := by scalar_tac
          simp [h2]
      · rw [if_neg e1] at hroles
        rw [← ok_inj' hroles]
        have h1 : ¬ f.val < n.val := by scalar_tac
        simp [h1]
    have hs := be_len_ge_one_ok bs some1 hsome1
    have hd := be_distinct_exec_ok bs distinct hdistinct
    simp only [vG, G.wfnAt, ← hr, ← hs, ← hd]
    cases roles with
    | false => simp only [Bool.false_eq_true, ite_false] at h; rw [← ok_inj' h]; rfl
    | true =>
      simp only [ite_true] at h
      cases some1 with
      | false => simp only [Bool.false_eq_true, ite_false] at h; rw [← ok_inj' h]; rfl
      | true =>
        simp only [ite_true] at h
        cases distinct with
        | false => simp only [Bool.false_eq_true, ite_false] at h; rw [← ok_inj' h]; rfl
        | true =>
          simp only [ite_true, bind_eq_ok'] at h
          obtain ⟨guarded, hguarded, h⟩ := h
          have hgv := wfn_loop0_ok env _ (alloc.vec.Vec.new Bool) 0#usize guarded rfl (by simp)
            (by simp) hguarded
          have ih := wfn_bs_exec_ok bs n guarded b h
          rw [hgv, List.reverse_replicate] at ih
          rw [allTrue_eq_replicate, List.length_reverse]
          simpa using ih
theorem wfn_bs_exec_ok : ∀ (bs : theory.BE) (n : Usize) (env : alloc.vec.Vec Bool) (b : Bool),
    multiparty.wfn_bs_exec bs n env = ok b → b = (vBs bs).wfnBs n.val env.val.reverse
  | .Nil, n, env, b, h => by
    unfold multiparty.wfn_bs_exec at h
    rw [← ok_inj' h]; rfl
  | .Cons label g rest, n, env, b, h => by
    unfold multiparty.wfn_bs_exec at h
    simp only [bind_eq_ok'] at h
    obtain ⟨a, ha, b1, hb1, h⟩ := h
    have i1 := wfn_exec_ok g n env a ha
    have i2 := wfn_bs_exec_ok rest n env b1 hb1
    simp only [vBs, Bs.wfnBs, ← i1, ← i2]
    cases a with
    | true => simp only [ite_true] at h; rw [← ok_inj' h]; rfl
    | false => simp only [Bool.false_eq_true, ite_false] at h; rw [← ok_inj' h]; rfl
end

/-! ## The acceptance check -/

/-- The loop over the roles: every role from `i` on has a projection. -/
theorem acceptn_loop_ok (g : theory.GE) (n : Usize) : ∀ (k : Nat) (i : Usize),
    n.val - i.val = k → multiparty.acceptn_exec_loop g n i = ok true →
    ∀ r, i.val ≤ r → r < n.val → (projn (vG g) r).isSome = true := by
  intro k
  induction k with
  | zero =>
    intro i hk _ r hr1 hr2
    omega
  | succ k ih =>
    intro i hk h r hr1 hr2
    unfold multiparty.acceptn_exec_loop loop at h
    simp only [multiparty.acceptn_exec_loop.body] at h
    have hlt : i < n := by scalar_tac
    simp only [hlt, ite_true, bind_eq_ok'] at h
    obtain ⟨cf, ⟨o, ho, hcf⟩, hloop⟩ := h
    cases o with
    | none =>
      simp only [core.option.Option.is_none, Option.isNone_none, ite_true] at hcf
      have hcf := ok_inj' hcf
      subst hcf
      simp at hloop
    | some l =>
      simp only [core.option.Option.is_none, Option.isNone_some, Bool.false_eq_true, ite_false,
        bind_eq_ok'] at hcf
      obtain ⟨i1, hi1, hcf⟩ := hcf
      have hcf := ok_inj' hcf
      subst hcf
      have hadd := usize_add_ok' hi1
      have h1 : (1#usize).val = 1 := rfl
      rw [h1] at hadd
      by_cases hri : r = i.val
      · rw [hri, projn_exec_ok g i l ho]; rfl
      · exact ih i1 (by omega) (by unfold multiparty.acceptn_exec_loop; exact hloop) r (by omega) hr2

/-- Whenever the acceptance check as compiled returns `true`, the Protocol
satisfies the acceptance condition. -/
theorem acceptn_exec_ok {g : theory.GE} {n : Usize} (h : multiparty.acceptn_exec g n = ok true) :
    acceptn (vG g) n.val := by
  unfold multiparty.acceptn_exec at h
  by_cases e : n < 2#usize
  · rw [if_pos e] at h
    simp at h
  · rw [if_neg e] at h
    simp only [bind_eq_ok'] at h
    obtain ⟨wf, hwf, h⟩ := h
    have hw := wfn_exec_ok g n (alloc.vec.Vec.new Bool) wf hwf
    cases wf with
    | false => simp at h
    | true =>
      simp only [ite_true] at h
      refine ⟨by scalar_tac, ?_, ?_⟩
      · rw [G.wfn, hw]; simp
      · intro r hr
        exact acceptn_loop_ok g n _ 0#usize rfl h r (by simp) hr

theorem accept_multiparty_ok {g : theory.GE} {n : Usize}
    (h : multiparty.accept_multiparty g n = ok true) : acceptn (vG g) n.val := by
  unfold multiparty.accept_multiparty at h
  exact acceptn_exec_ok h

/-- **The multiparty progress theorem for the checker as compiled.** If
`rsk-protocol`'s `accept_multiparty`, as rustc compiles it without Verus
and Aeneas translates it, returns `true` for a Protocol `g` of `n` roles,
then no reachable configuration of the asynchronous product of the
Protocol's projections has a waiting set (R41.3). -/
theorem accept_multiparty_progress {g : theory.GE} {n : Usize}
    (h : multiparty.accept_multiparty g n = ok true) {c : NC} (hr : NReach (vG g) n.val c) :
    ¬ NDeadlocked c n.val :=
  multiparty_progress (accept_multiparty_ok h) hr

/-- And every reception is expected: a role whose head is a receive from a
role below `n` and whose awaited channel is not empty accepts the
channel's oldest label. -/
theorem accept_multiparty_reception {g : theory.GE} {n : Usize}
    (h : multiparty.accept_multiparty g n = ok true) {c : NC} (hr : NReach (vG g) n.val c)
    {r frm label : Nat} {ls : Ls} {tl : List Nat} (h1 : r < n.val) (h2 : frm < n.val)
    (hh : (c.l r).head = .recv frm ls) (hq : c.ch frm r = label :: tl) :
    (ls.lookup label).isSome = true :=
  multiparty_reception (accept_multiparty_ok h) hr h1 h2 hh hq


end MP
