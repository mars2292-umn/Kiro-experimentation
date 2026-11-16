import Proto.Merge
/-!
# Runtime global types

A reachable configuration is described by a runtime global type `R`: the
syntax `G` with one more form, `sent`, a communication whose message is in
transit (the sender has moved on, the receiver has not). Port of the
sections "runtime types" and "the embedding" of
`crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

mutual
/-- A runtime global type. -/
inductive R : Type
  | fin : R
  | comm (frm dst : Nat) (bs : RBs) : R
  | sent (frm dst label : Nat) (cont : R) : R
  | mu (body : R) : R
  | var (n : Nat) : R
inductive RBs : Type
  | nil : RBs
  | cons (label : Nat) (g : R) (rest : RBs) : RBs
end

mutual
def emb : G → R
  | .fin => .fin
  | .var n => .var n
  | .mu body => .mu (emb body)
  | .comm frm dst bs => .comm frm dst (embBs bs)
def embBs : Bs → RBs
  | .nil => .nil
  | .cons label g rest => .cons label (emb g) (embBs rest)
end

def RBs.len : RBs → Nat
  | .nil => 0
  | .cons _ _ rest => 1 + rest.len

def RBs.lookup : RBs → Nat → Option R
  | .nil, _ => none
  | .cons l g rest, label => if l = label then some g else rest.lookup label

def RBs.distinct : RBs → Bool
  | .nil => true
  | .cons label _ rest => (rest.lookup label).isNone && rest.distinct

/-- The first branch's continuation (`fin` for an empty list, which
well-formedness excludes). -/
def RBs.first : RBs → R
  | .nil => .fin
  | .cons _ g _ => g

mutual
/-- Well-formedness under a guard environment, as `G.wfnAt`, a `sent`
guarding like a communication. -/
def R.wfAt : R → Nat → List Bool → Bool
  | .fin, _, _ => true
  | .var v, _, env => env.getD v false
  | .mu body, n, env => body.wfAt n (false :: env)
  | .comm frm dst bs, n, env =>
    decide (frm < n) && decide (dst < n) && decide (frm ≠ dst) && decide (1 ≤ bs.len) &&
      bs.distinct && bs.wfBs n (allTrue env)
  | .sent frm dst _ cont, n, env =>
    decide (frm < n) && decide (dst < n) && decide (frm ≠ dst) && cont.wfAt n (allTrue env)
def RBs.wfBs : RBs → Nat → List Bool → Bool
  | .nil, _, _ => true
  | .cons _ g rest, n, env => g.wfAt n env && rest.wfBs n env
end

def R.wf (x : R) (n : Nat) : Bool := x.wfAt n []

mutual
def R.closedAt : R → Nat → Bool
  | .fin, _ => true
  | .var n, m => decide (n < m)
  | .mu body, m => body.closedAt (m + 1)
  | .comm _ _ bs, m => bs.closedAt m
  | .sent _ _ _ cont, m => cont.closedAt m
def RBs.closedAt : RBs → Nat → Bool
  | .nil, _ => true
  | .cons _ g rest, m => g.closedAt m && rest.closedAt m
end

mutual
def R.subst : R → Nat → R → R
  | .fin, _, _ => .fin
  | .var n, k, by' => if n = k then by' else if n > k then .var (n - 1) else .var n
  | .mu body, k, by' => .mu (body.subst (k + 1) by')
  | .comm frm dst bs, k, by' => .comm frm dst (bs.subst k by')
  | .sent frm dst label cont, k, by' => .sent frm dst label (cont.subst k by')
def RBs.subst : RBs → Nat → R → RBs
  | .nil, _, _ => .nil
  | .cons label g rest, k, by' => .cons label (g.subst k by') (rest.subst k by')
end

def R.unfold : R → R
  | .mu body => body.subst 0 (.mu body)
  | x => x

def R.leadRecs : R → Nat
  | .mu body => 1 + body.leadRecs
  | _ => 0

def R.spineOk : R → Bool
  | .mu body => body.spineOk
  | .var _ => false
  | _ => true

/-- The head normal form by at most `k` unfoldings. -/
def R.headN : R → Nat → R
  | x, 0 => x
  | .mu body, k + 1 => (R.mu body).unfold.headN k
  | x, _ + 1 => x

def R.head (x : R) : R := x.headN x.leadRecs

theorem allTrue_length (env : List Bool) : (allTrue env).length = env.length := by
  simp [allTrue]

theorem allTrue_append (a b : List Bool) : allTrue (a ++ b) = allTrue a ++ allTrue b := by
  simp [allTrue]

theorem allTrue_nil : allTrue [] = [] := rfl

theorem getD_of_lt {l : List Bool} {i : Nat} (d : Bool) (h : i < l.length) : l.getD i d = l[i] := by
  simp [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h]

theorem getD_of_ge {l : List Bool} {i : Nat} (d : Bool) (h : l.length ≤ i) : l.getD i d = d := by
  simp [List.getD_eq_getElem?_getD, List.getElem?_eq_none h]

theorem R.wf_spine : ∀ (x : R) (n : Nat) (env : List Bool), x.wfAt n env = true →
    (∀ b ∈ env, b = false) → x.spineOk = true
  | .fin, _, _, _, _ => rfl
  | .comm .., _, _, _, _ => rfl
  | .sent .., _, _, _, _ => rfl
  | .var v, n, env, h, he => by
    simp only [R.wfAt] at h
    by_cases hv : v < env.length
    · rw [getD_of_lt _ hv] at h
      have := he _ (List.getElem_mem hv)
      rw [h] at this; cases this
    · rw [getD_of_ge _ (by omega)] at h; cases h
  | .mu body, n, env, h, he => by
    simp only [R.wfAt] at h
    simp only [R.spineOk]
    exact R.wf_spine body n (false :: env) h (by
      intro b hb
      rcases List.mem_cons.mp hb with rfl | hb
      · rfl
      · exact he b hb)

theorem R.subst_spine : ∀ (x : R) (k : Nat) (by' : R), x.spineOk = true →
    (x.subst k by').spineOk = true ∧ (x.subst k by').leadRecs = x.leadRecs
  | .fin, _, _, _ => ⟨rfl, rfl⟩
  | .comm .., _, _, _ => ⟨rfl, rfl⟩
  | .sent .., _, _, _ => ⟨rfl, rfl⟩
  | .var v, _, _, h => by simp [R.spineOk] at h
  | .mu body, k, by', h => by
    have := R.subst_spine body (k + 1) by' (by simpa [R.spineOk] using h)
    simp only [R.subst, R.spineOk, R.leadRecs]
    exact ⟨this.1, by rw [this.2]⟩

mutual
theorem R.wf_closed : ∀ (x : R) (n : Nat) (env : List Bool), x.wfAt n env = true →
    x.closedAt env.length = true
  | .fin, _, _, _ => rfl
  | .var v, n, env, h => by
    simp only [R.wfAt] at h
    simp only [R.closedAt, decide_eq_true_eq]
    by_contra hv
    rw [getD_of_ge _ (by omega)] at h; cases h
  | .mu body, n, env, h => by
    simp only [R.wfAt] at h
    simp only [R.closedAt]
    exact R.wf_closed body n (false :: env) h
  | .comm frm dst bs, n, env, h => by
    simp only [R.wfAt, Bool.and_eq_true] at h
    simp only [R.closedAt]
    have := RBs.wf_closed bs n (allTrue env) h.2
    rwa [allTrue_length] at this
  | .sent frm dst label cont, n, env, h => by
    simp only [R.wfAt, Bool.and_eq_true] at h
    simp only [R.closedAt]
    have := R.wf_closed cont n (allTrue env) h.2
    rwa [allTrue_length] at this
theorem RBs.wf_closed : ∀ (bs : RBs) (n : Nat) (env : List Bool), bs.wfBs n env = true →
    bs.closedAt env.length = true
  | .nil, _, _, _ => rfl
  | .cons label g rest, n, env, h => by
    simp only [RBs.wfBs, Bool.and_eq_true] at h
    simp only [RBs.closedAt, Bool.and_eq_true]
    exact ⟨R.wf_closed g n env h.1, RBs.wf_closed rest n env h.2⟩
end

mutual
/-- The guard flags of the recursions outside the `inner` ones do not
matter to a type closed under the inner ones. -/
theorem R.env_swap : ∀ (x : R) (n : Nat) (outer1 outer2 inner : List Bool),
    x.wfAt n (inner ++ outer1) = true → x.closedAt inner.length = true →
    x.wfAt n (inner ++ outer2) = true
  | .fin, _, _, _, _, _, _ => rfl
  | .var v, n, o1, o2, inner, h, hc => by
    simp only [R.closedAt, decide_eq_true_eq] at hc
    simp only [R.wfAt] at h ⊢
    rw [getD_of_lt _ (by simp; omega)] at h ⊢
    rw [List.getElem_append_left hc] at h ⊢
    exact h
  | .mu body, n, o1, o2, inner, h, hc => by
    simp only [R.wfAt] at h ⊢
    simp only [R.closedAt] at hc
    exact R.env_swap body n o1 o2 (false :: inner) h hc
  | .comm frm dst bs, n, o1, o2, inner, h, hc => by
    simp only [R.wfAt, Bool.and_eq_true, allTrue_append] at h ⊢
    simp only [R.closedAt] at hc
    exact ⟨h.1, RBs.env_swap bs n (allTrue o1) (allTrue o2) (allTrue inner) h.2
      (by rwa [allTrue_length])⟩
  | .sent frm dst label cont, n, o1, o2, inner, h, hc => by
    simp only [R.wfAt, Bool.and_eq_true, allTrue_append] at h ⊢
    simp only [R.closedAt] at hc
    exact ⟨h.1, R.env_swap cont n (allTrue o1) (allTrue o2) (allTrue inner) h.2
      (by rwa [allTrue_length])⟩
theorem RBs.env_swap : ∀ (bs : RBs) (n : Nat) (outer1 outer2 inner : List Bool),
    bs.wfBs n (inner ++ outer1) = true → bs.closedAt inner.length = true →
    bs.wfBs n (inner ++ outer2) = true
  | .nil, _, _, _, _, _, _ => rfl
  | .cons label g rest, n, o1, o2, inner, h, hc => by
    simp only [RBs.wfBs, Bool.and_eq_true] at h ⊢
    simp only [RBs.closedAt, Bool.and_eq_true] at hc
    exact ⟨R.env_swap g n o1 o2 inner h.1 hc.1, RBs.env_swap rest n o1 o2 inner h.2 hc.2⟩
end

theorem R.closed_any_env {by' : R} {n : Nat} (h : by'.wf n = true) (env : List Bool) :
    by'.wfAt n env = true := by
  have hc := R.wf_closed by' n [] h
  have := R.env_swap by' n [] env [] (by simpa [R.wf] using h) hc
  simpa using this

theorem RBs.subst_lookup : ∀ (bs : RBs) (k : Nat) (by' : R) (label : Nat),
    (bs.subst k by').lookup label = (bs.lookup label).map (fun g => g.subst k by')
  | .nil, _, _, _ => by simp [RBs.subst, RBs.lookup]
  | .cons l g rest, k, by', label => by
    simp only [RBs.subst, RBs.lookup]
    split
    · simp
    · exact RBs.subst_lookup rest k by' label

theorem RBs.subst_len : ∀ (bs : RBs) (k : Nat) (by' : R), (bs.subst k by').len = bs.len
  | .nil, _, _ => rfl
  | .cons l g rest, k, by' => by simp [RBs.subst, RBs.len, RBs.subst_len rest k by']

theorem RBs.subst_distinct : ∀ (bs : RBs) (k : Nat) (by' : R),
    (bs.subst k by').distinct = bs.distinct
  | .nil, _, _ => rfl
  | .cons l g rest, k, by' => by
    simp only [RBs.subst, RBs.distinct, RBs.subst_lookup, RBs.subst_distinct rest k by']
    cases rest.lookup l <;> simp

mutual
theorem R.subst_wf : ∀ (x : R) (n : Nat) (e0 : Bool) (rest : List Bool) (by' : R),
    x.wfAt n (rest ++ [e0]) = true → by'.wf n = true →
    (x.subst rest.length by').wfAt n rest = true
  | .fin, _, _, _, _, _, _ => rfl
  | .var v, n, e0, rest, by', h, hb => by
    simp only [R.subst]
    split
    · exact R.closed_any_env hb rest
    · rename_i hne
      simp only [R.wfAt] at h
      have hv : v < rest.length := by
        by_contra hv
        have hlt : ¬ v < (rest ++ [e0]).length := by simp; omega
        rw [getD_of_ge _ (by omega)] at h; cases h
      rw [if_neg (by omega)]
      simp only [R.wfAt]
      rw [getD_of_lt _ (by simp; omega), List.getElem_append_left hv] at h
      rw [getD_of_lt _ hv]
      exact h
  | .mu body, n, e0, rest, by', h, hb => by
    simp only [R.wfAt] at h
    simp only [R.subst, R.wfAt]
    exact R.subst_wf body n e0 (false :: rest) by' h hb
  | .comm frm dst bs, n, e0, rest, by', h, hb => by
    simp only [R.wfAt, Bool.and_eq_true, allTrue_append] at h
    simp only [R.subst, R.wfAt, Bool.and_eq_true, RBs.subst_len, RBs.subst_distinct]
    refine ⟨h.1, ?_⟩
    have := RBs.subst_wf bs n true (allTrue rest) by' h.2 hb
    rwa [allTrue_length] at this
  | .sent frm dst label cont, n, e0, rest, by', h, hb => by
    simp only [R.wfAt, Bool.and_eq_true, allTrue_append] at h
    simp only [R.subst, R.wfAt, Bool.and_eq_true]
    refine ⟨h.1, ?_⟩
    have := R.subst_wf cont n true (allTrue rest) by' h.2 hb
    rwa [allTrue_length] at this
theorem RBs.subst_wf : ∀ (bs : RBs) (n : Nat) (e0 : Bool) (rest : List Bool) (by' : R),
    bs.wfBs n (rest ++ [e0]) = true → by'.wf n = true →
    (bs.subst rest.length by').wfBs n rest = true
  | .nil, _, _, _, _, _, _ => rfl
  | .cons label g tail, n, e0, rest, by', h, hb => by
    simp only [RBs.wfBs, Bool.and_eq_true] at h
    simp only [RBs.subst, RBs.wfBs, Bool.and_eq_true]
    exact ⟨R.subst_wf g n e0 rest by' h.1 hb, RBs.subst_wf tail n e0 rest by' h.2 hb⟩
end

/-- Unfolding a closed well-formed recursion gives a closed well-formed
type with one recursion fewer at the top. -/
theorem R.unfold_wf {body : R} {n : Nat} (h : (R.mu body).wf n = true) :
    (R.mu body).unfold.wf n = true ∧ (R.mu body).unfold.leadRecs + 1 = (R.mu body).leadRecs := by
  have hb : body.wfAt n [false] = true := by simpa [R.wf, R.wfAt] using h
  have h1 := R.subst_wf body n false [] (.mu body) (by simpa using hb) h
  have h2 := R.wf_spine body n [false] hb (by simp)
  have h3 := R.subst_spine body 0 (.mu body) h2
  refine ⟨by simpa [R.unfold, R.wf] using h1, ?_⟩
  simp only [R.unfold, R.leadRecs, h3.2]
  omega

theorem R.headN_wf : ∀ (k : Nat) (x : R) (n : Nat), x.wf n = true → x.leadRecs = k →
    (x.headN k).wf n = true ∧ (∀ b, x.headN k ≠ .mu b) ∧ (∀ v, x.headN k ≠ .var v)
  | 0, x, n, h, hk => by
    simp only [R.headN]
    refine ⟨h, ?_, ?_⟩
    · intro b e; subst e; simp [R.leadRecs] at hk
    · intro v e; subst e
      have := R.wf_spine (.var v) n [] h (by simp)
      simp [R.spineOk] at this
  | k + 1, x, n, h, hk => by
    cases x with
    | mu body =>
      have hu := R.unfold_wf h
      simp only [R.headN]
      exact R.headN_wf k _ n hu.1 (by omega)
    | fin => simp [R.leadRecs] at hk
    | var v => simp [R.leadRecs] at hk
    | comm frm dst bs => simp [R.leadRecs] at hk
    | sent frm dst label cont => simp [R.leadRecs] at hk

theorem R.head_wf {x : R} {n : Nat} (h : x.wf n = true) :
    x.head.wf n = true ∧ (∀ b, x.head ≠ .mu b) ∧ (∀ v, x.head ≠ .var v) :=
  R.headN_wf _ x n h rfl

/-- A branch of a closed well-formed communication is closed and well
formed. -/
theorem RBs.branch_wf : ∀ (bs : RBs) (n : Nat) (label : Nat) (g : R), bs.wfBs n [] = true →
    bs.lookup label = some g → g.wf n = true
  | .nil, _, _, _, _, h => by simp [RBs.lookup] at h
  | .cons l g' rest, n, label, g, hw, h => by
    simp only [RBs.wfBs, Bool.and_eq_true] at hw
    simp only [RBs.lookup] at h
    split at h
    · cases h; exact hw.1
    · exact RBs.branch_wf rest n label g hw.2 h

theorem R.comm_branches_wf {frm dst : Nat} {bs : RBs} {n : Nat}
    (h : (R.comm frm dst bs).wf n = true) : bs.wfBs n [] = true := by
  simp only [R.wf, R.wfAt, Bool.and_eq_true, allTrue_nil] at h
  exact h.2

theorem R.comm_wf_parts {frm dst : Nat} {bs : RBs} {n : Nat}
    (h : (R.comm frm dst bs).wf n = true) :
    frm < n ∧ dst < n ∧ frm ≠ dst ∧ 1 ≤ bs.len ∧ bs.distinct = true ∧ bs.wfBs n [] = true := by
  simp only [R.wf, R.wfAt, Bool.and_eq_true, allTrue_nil, decide_eq_true_eq] at h
  exact ⟨h.1.1.1.1.1, h.1.1.1.1.2, h.1.1.1.2, h.1.1.2, h.1.2, h.2⟩

theorem R.sent_wf_parts {frm dst label : Nat} {cont : R} {n : Nat}
    (h : (R.sent frm dst label cont).wf n = true) :
    frm < n ∧ dst < n ∧ frm ≠ dst ∧ cont.wf n = true := by
  simp only [R.wf, R.wfAt, Bool.and_eq_true, allTrue_nil, decide_eq_true_eq] at h
  exact ⟨h.1.1.1, h.1.1.2, h.1.2, h.2⟩

/-! ## The embedding -/

theorem embBs_len : ∀ bs : Bs, (embBs bs).len = bs.len
  | .nil => rfl
  | .cons l g rest => by simp [embBs, RBs.len, Bs.len, embBs_len rest]

theorem embBs_lookup : ∀ (bs : Bs) (label : Nat),
    (embBs bs).lookup label = (bs.lookup label).map emb
  | .nil, _ => rfl
  | .cons l g rest, label => by
    simp only [embBs, RBs.lookup, Bs.lookup]
    split
    · simp
    · exact embBs_lookup rest label

theorem Bs.hasLabel_iff : ∀ (bs : Bs) (label : Nat),
    bs.hasLabel label = (bs.lookup label).isSome
  | .nil, _ => rfl
  | .cons l g rest, label => by
    simp only [Bs.hasLabel, Bs.lookup, Bs.hasLabel_iff rest label]
    by_cases h : l = label <;> simp [h]

theorem embBs_distinct : ∀ bs : Bs, (embBs bs).distinct = bs.distinct
  | .nil => rfl
  | .cons l g rest => by
    simp only [embBs, RBs.distinct, Bs.distinct, embBs_lookup, embBs_distinct rest,
      Bs.hasLabel_iff]
    cases rest.lookup l <;> simp

mutual
theorem emb_wf : ∀ (g : G) (n : Nat) (env : List Bool), (emb g).wfAt n env = g.wfnAt n env
  | .fin, _, _ => rfl
  | .var v, _, _ => rfl
  | .mu body, n, env => by simp only [emb, R.wfAt, G.wfnAt, emb_wf body n (false :: env)]
  | .comm frm dst bs, n, env => by
    simp only [emb, R.wfAt, G.wfnAt, embBs_len, embBs_distinct, embBs_wf bs n (allTrue env)]
theorem embBs_wf : ∀ (bs : Bs) (n : Nat) (env : List Bool), (embBs bs).wfBs n env = bs.wfnBs n env
  | .nil, _, _ => rfl
  | .cons l g rest, n, env => by
    simp only [embBs, RBs.wfBs, Bs.wfnBs, emb_wf g n env, embBs_wf rest n env]
end

end MP
