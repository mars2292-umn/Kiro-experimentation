import Proto.CertExec
open Aeneas Aeneas.Std Result ControlFlow
/-!
# The progress certificate checker on the code as compiled (R41.3, R41.1)

`progress::certify` as rustc compiles it without Verus and Aeneas
translates it (`Proto/Proto.lean`), with the functions under it that
`Proto/CertExec.lean` does not read: `successor`, `is_blocked`,
`is_terminal`, `has_deadlock`, `successors_listed`, `configs_closed`.

- `certify_spec`: for a well-formed system the checker returns, and it
  answers `true` only when the list is a `Certificate` (the first clause
  of the Verus contract of `certify`).
- `certify_sound`: after a `true`, no reachable configuration of the
  system has a waiting set and none has a channel over the capacity (the
  second clause, by `certificate_sound`).

`has_deadlock` is read in the direction the checker uses: a `false` means
that the configuration has no waiting set (Verus proves both directions).
One premise is of the model alone: the declared capacity is below
`usize::MAX` (a vector of that length cannot be pushed to in the model,
and cannot exist in memory).
-/

namespace PC
open rsk_protocol MP

/-! ## Lists -/

theorem drop_cons_of_getElem? {α} {l : List α} {k : Nat} {x : α} (h : l[k]? = some x) :
    l.drop k = x :: l.drop (k + 1) := by
  have hk : k < l.length := by
    by_contra hc
    rw [List.getElem?_eq_none (by omega)] at h
    cases h
  rw [List.getElem?_eq_getElem hk] at h
  rw [List.drop_eq_getElem_cons hk, Option.some.inj h]

theorem getElem!_set_ne' {α} [Inhabited α] (l : List α) (i j : Nat) (a : α) (h : i ≠ j) :
    (l.set i a)[j]! = l[j]! := by
  rw [List.getElem!_eq_getElem?_getD, List.getElem!_eq_getElem?_getD, List.getElem?_set_ne h]

theorem count_set_false (l : List Bool) (k : Nat) (h : k < l.length) (hk : l[k] = true) :
    (l.set k false).count true + 1 = l.count true := by
  grind

/-- `Vec::set` of vstd (`self[i] = value`). -/
theorem vset_spec {α} (v : alloc.vec.Vec α) (i : Usize) (x : α) (h : i.val < v.val.length) :
    alloc.vec.Vec.Insts.VstdPervasiveVecAdditionalExecFns.set v i x
      ⦃ r => r.val = v.val.set i.val x ⦄ := by
  unfold alloc.vec.Vec.Insts.VstdPervasiveVecAdditionalExecFns.set
  rw [alloc.vec.Vec.index_mut_slice_index]
  apply WP.spec_bind (alloc.vec.Vec.index_mut_usize_spec v i (by scalar_tac))
  intro p hp
  obtain ⟨a, back⟩ := p
  have hb : back = v.set i := by
    have hp' : a = v.val[i.val] ∧ back = v.set i := hp
    exact hp'.2
  refine (WP.spec_ok _).mpr ?_
  show (back x).val = v.val.set i.val x
  rw [hb, alloc.vec.Vec.set_val_eq]

/-! ## The successor -/

theorem successor_loop0_spec (c : progress.Config) (t : progress.Trans) :
    progress.successor_loop0 c t (alloc.vec.Vec.new Usize) 0#usize
      ⦃ r => r.1 = c.chans ∧ r.2.1 = t ∧ r.2.2.val = c.states.val.set t.role.val t.«to» ⦄ := by
  unfold progress.successor_loop0
  apply loop.spec_decr_nat
    (fun (st : alloc.vec.Vec Usize × Usize) => c.states.val.length - st.2.val)
    (fun st => st.2.val ≤ c.states.val.length ∧
      st.1.val ++ (c.states.val.set t.role.val t.«to»).drop st.2.val
        = c.states.val.set t.role.val t.«to»)
  · intro st hst
    obtain ⟨states, k⟩ := st
    obtain ⟨h1, h2⟩ := hst
    dsimp only at h1 h2 ⊢
    unfold progress.successor_loop0.body
    dsimp only
    split
    · rename_i hlt
      have hk : k.val < c.states.val.length := by scalar_tac
      have hlen : states.val.length < Usize.max := by
        have hp := c.states.property
        have hl := congrArg List.length h2
        simp only [List.length_append, List.length_drop, List.length_set] at hl
        omega
      split
      · rename_i heq
        have hkv : t.role.val = k.val := by rw [heq]
        have hd := drop_cons_of_getElem? (l := c.states.val.set t.role.val t.«to») (k := k.val)
          (x := t.«to») (by rw [hkv]; exact List.getElem?_set_self hk)
        apply WP.spec_bind (alloc.vec.Vec.push_spec states t.«to» hlen)
        intro states1 hs1
        apply cont_incr k c.states.val.length hk c.states.property
        intro k1 hk1
        dsimp only
        leaves <;> first | omega | skip
        rw [hs1, hk1, List.append_assoc, List.singleton_append, ← hd]
        exact h2
      · rename_i hne
        have hkv : t.role.val ≠ k.val := fun e => hne (usize_val_inj e.symm)
        step as ⟨x, hx⟩
        have hd := drop_cons_of_getElem? (l := c.states.val.set t.role.val t.«to») (k := k.val)
          (x := x) (by rw [List.getElem?_set_ne hkv, hx]; exact List.getElem?_eq_getElem hk)
        apply WP.spec_bind (alloc.vec.Vec.push_spec states x hlen)
        intro states1 hs1
        apply cont_incr k c.states.val.length hk c.states.property
        intro k1 hk1
        dsimp only
        leaves <;> first | omega | skip
        rw [hs1, hk1, List.append_assoc, List.singleton_append, ← hd]
        exact h2
    · rename_i hlt
      refine (WP.spec_ok _).mpr ?_
      dsimp only
      refine ⟨rfl, rfl, ?_⟩
      have hkl : c.states.val.length ≤ k.val := by scalar_tac
      have hdrop : (c.states.val.set t.role.val t.«to»).drop k.val = [] :=
        List.drop_of_length_le (by rw [List.length_set]; exact hkl)
      have h3 := h2
      rw [hdrop, List.append_nil] at h3
      exact h3
  · exact ⟨by scalar_tac, rfl⟩

theorem successor_loop1_spec (v : alloc.vec.Vec (alloc.vec.Vec U32)) (t : progress.Trans)
    (ch : Usize) (E : List U32) (hch : ch.val < v.val.length)
    (hcap : ∀ q, q ∈ v.val → q.val.length < Usize.max)
    (hE : ∀ w, v.val[ch.val]? = some w →
      E = if t.send = true then w.val ++ [t.label] else w.val.drop 1)
    (hrecv : ¬ t.send = true → ∀ w, v.val[ch.val]? = some w → 1 ≤ w.val.length) :
    progress.successor_loop1 v t ch (alloc.vec.Vec.new (alloc.vec.Vec U32)) 0#usize
      ⦃ r => r.val.map (fun q => q.val) = (v.val.map (fun q => q.val)).set ch.val E ⦄ := by
  unfold progress.successor_loop1
  apply loop.spec_decr_nat
    (fun (st : progress.Trans × alloc.vec.Vec (alloc.vec.Vec U32) × Usize) =>
      v.val.length - st.2.2.val)
    (fun st => t = st.1 ∧ st.2.2.val ≤ v.val.length ∧
      st.2.1.val.map (fun q => q.val) ++
          ((v.val.map (fun q => q.val)).set ch.val E).drop st.2.2.val
        = (v.val.map (fun q => q.val)).set ch.val E)
  · intro st hst
    obtain ⟨t1, chans, m⟩ := st
    obtain ⟨h0, h1, h2⟩ := hst
    dsimp only at h0 h1 h2 ⊢
    subst h0
    unfold progress.successor_loop1.body
    dsimp only
    split
    · rename_i hlt
      have hm : m.val < v.val.length := by scalar_tac
      have hlen : chans.val.length < Usize.max := by
        have hp := v.property
        have hl := congrArg List.length h2
        simp only [List.length_append, List.length_drop, List.length_set, List.length_map] at hl
        omega
      have hvm := hcap _ (List.getElem_mem hm)
      refine WP.spec_bind (Pₘ := fun p => t = p.1 ∧
        p.2.val.map (fun q => q.val) ++
            ((v.val.map (fun q => q.val)).set ch.val E).drop (m.val + 1)
          = (v.val.map (fun q => q.val)).set ch.val E) ?_ ?_
      · by_cases hmc : m = ch
        · rw [if_pos hmc]
          have hd := drop_cons_of_getElem? (l := (v.val.map (fun q => q.val)).set ch.val E)
            (k := m.val) (x := E)
            (by rw [hmc]; exact List.getElem?_set_self (by rw [List.length_map]; exact hch))
          refine WP.spec_bind (Pₘ := fun v1 =>
            v1.val.map (fun q => q.val) ++
                ((v.val.map (fun q => q.val)).set ch.val E).drop (m.val + 1)
              = (v.val.map (fun q => q.val)).set ch.val E) ?_ ?_
          · by_cases hsend : t.send = true
            · rw [if_pos hsend]
              step as ⟨v2, hv2⟩
              have hE' := hE v2 (by rw [hv2, ← hmc]; exact List.getElem?_eq_getElem hm)
              rw [if_pos hsend] at hE'
              apply WP.spec_bind (copy_from_spec v2 0#usize (by scalar_tac))
              intro q hq
              have hqv : q.val = v2.val := hq
              have hql : q.val.length < Usize.max := by rw [hqv, hv2]; exact hvm
              apply WP.spec_bind (alloc.vec.Vec.push_spec q t.label hql)
              intro q1 hq1
              refine WP.spec_mono (alloc.vec.Vec.push_spec chans q1 hlen) ?_
              intro chans1 hc1
              have hq1v : q1.val = E := by rw [hq1, hqv, hE']
              rw [hc1, List.map_append, List.map_cons, List.map_nil, List.append_assoc,
                List.singleton_append, hq1v, ← hd]
              exact h2
            · rw [if_neg hsend]
              step as ⟨v2, hv2⟩
              have hget : v.val[ch.val]? = some v2 := by
                rw [hv2, ← hmc]; exact List.getElem?_eq_getElem hm
              have hE' := hE v2 hget
              rw [if_neg hsend] at hE'
              have h1l := hrecv hsend v2 hget
              apply WP.spec_bind (copy_from_spec v2 1#usize (by scalar_tac))
              intro q hq
              have hqv : q.val = v2.val.drop 1 := hq
              refine WP.spec_mono (alloc.vec.Vec.push_spec chans q hlen) ?_
              intro chans1 hc1
              have hq1v : q.val = E := by rw [hqv, hE']
              rw [hc1, List.map_append, List.map_cons, List.map_nil, List.append_assoc,
                List.singleton_append, hq1v, ← hd]
              exact h2
          · intro v1 hv1
            exact (WP.spec_ok _).mpr ⟨rfl, hv1⟩
        · rw [if_neg hmc]
          have hmv : ch.val ≠ m.val := fun e => hmc (usize_val_inj e.symm)
          step as ⟨v1, hv1⟩
          have hd := drop_cons_of_getElem? (l := (v.val.map (fun q => q.val)).set ch.val E)
            (k := m.val) (x := v1.val)
            (by rw [List.getElem?_set_ne hmv, List.getElem?_map, List.getElem?_eq_getElem hm, hv1]
                rfl)
          apply WP.spec_bind (copy_from_spec v1 0#usize (by scalar_tac))
          intro q hq
          have hqv : q.val = v1.val := hq
          apply WP.spec_bind (alloc.vec.Vec.push_spec chans q hlen)
          intro chans2 hc2
          refine (WP.spec_ok _).mpr ⟨rfl, ?_⟩
          show chans2.val.map (fun q => q.val) ++ _ = _
          rw [hc2, List.map_append, List.map_cons, List.map_nil, List.append_assoc,
            List.singleton_append, hqv, ← hd]
          exact h2
      · intro p hp
        obtain ⟨t1, chans1⟩ := p
        obtain ⟨ht1, hc1⟩ := hp
        dsimp only at ht1 hc1 ⊢
        subst ht1
        apply cont_incr m v.val.length hm v.property
        intro m1 hm1
        dsimp only
        leaves <;> first | rfl | omega | skip
        rw [hm1]
        exact hc1
    · rename_i hlt
      refine (WP.spec_ok _).mpr ?_
      dsimp only
      have hml : v.val.length ≤ m.val := by scalar_tac
      have hdrop : ((v.val.map (fun q => q.val)).set ch.val E).drop m.val = [] :=
        List.drop_of_length_le (by rw [List.length_set, List.length_map]; exact hml)
      have h3 := h2
      rw [hdrop, List.append_nil] at h3
      exact h3
  · exact ⟨rfl, by scalar_tac, rfl⟩

/-- `successor`: the view of the result is `succ` of the view. -/
theorem successor_spec (s : progress.System) (c : progress.Config) (i : Usize)
    (hs : WfSys s) (hi : i.val < s.trans.val.length) (hc : WfCfg s.roles.val (cfg c))
    (hcap : WithinCapacity s (cfg c)) (hmax : s.capacity.val < Usize.max)
    (hen : Enabled s.roles.val (cfg c) (s.trans.val[i.val]'hi)) :
    progress.successor s c i
      ⦃ r => cfg r = succ s.roles.val (cfg c) (s.trans.val[i.val]'hi) ⦄ := by
  unfold progress.successor
  step as ⟨t, ht⟩
  rw [← ht] at hen ⊢
  have htm : t ∈ s.trans.val := by rw [ht]; exact List.getElem_mem _
  obtain ⟨hr, hp, _⟩ := hs.2 t htm
  have hcl : c.chans.val.length = s.roles.val * s.roles.val := by
    have h2 := hc.2
    rw [cfg_chans, List.length_map] at h2
    exact h2
  apply WP.spec_bind (chan_index_spec s t hs.1 hr hp)
  intro ch hch
  obtain ⟨hch1, hch2⟩ := hch
  have hchl : ch.val < c.chans.val.length := by omega
  have he : ∀ w, c.chans.val[ch.val]? = some w →
      (cfg c).chans[chanOf s.roles.val t]! = w.val := by
    intro w hw
    rw [List.getElem?_eq_getElem hchl] at hw
    rw [cfg_chans, ← hch1, getElem!_map' _ _ _ hchl, getElem!_pos c.chans.val ch.val hchl,
      Option.some.inj hw]
  apply WP.spec_bind (successor_loop0_spec c t)
  intro p hp0
  obtain ⟨v, t1, states⟩ := p
  obtain ⟨hv, ht1, hst⟩ := hp0
  dsimp only at hv ht1 hst ⊢
  subst hv
  subst ht1
  refine WP.spec_bind (successor_loop1_spec c.chans t1 ch
    (if t1.send = true then (cfg c).chans[chanOf s.roles.val t1]! ++ [t1.label]
      else ((cfg c).chans[chanOf s.roles.val t1]!).tail) hchl ?_ ?_ ?_) ?_
  · intro q hq
    have h1 := hcap q.val (by rw [cfg_chans]; exact List.mem_map.mpr ⟨q, hq, rfl⟩)
    omega
  · intro w hw
    rw [he w hw, List.drop_one]
  · intro hsf w hw
    rcases hen.2.2 with h | h
    · exact absurd h hsf
    · rw [he w hw] at h
      cases hwv : w.val with
      | nil => rw [hwv] at h; cases h
      | cons x xs => simp
  · intro chans hchans
    refine (WP.spec_ok _).mpr ?_
    show (⟨states.val, chans.val.map (fun q => q.val)⟩ : Cfg) = _
    rw [hst, hchans, hch1]
    rfl

/-! ## Awaiting and finished roles -/

/-- `is_blocked`: a role that awaits a message is answered `true`. -/
theorem is_blocked_spec (s : progress.System) (c : progress.Config) (r : Usize)
    (hs : WfSys s) (hc : WfCfg s.roles.val (cfg c)) :
    progress.is_blocked s c r ⦃ b => Blocked s (cfg c) r.val → b = true ⦄ := by
  unfold progress.is_blocked
  refine WP.spec_bind
    (Pₘ := fun p => Blocked s (cfg c) r.val → p.1 = true ∧ p.2 = true) ?_ ?_
  · unfold progress.is_blocked_loop
    apply loop.spec_decr_nat
      (fun (st : progress.Config × Bool × Bool × Usize) => s.trans.val.length - st.2.2.2.val)
      (fun st => c = st.1 ∧ st.2.2.2.val ≤ s.trans.val.length ∧
        (Blocked s (cfg c) r.val → st.2.2.1 = true ∧
          (st.2.1 = false → ∀ j (hj : j < s.trans.val.length), j < st.2.2.2.val →
            ¬ ((s.trans.val[j]).role = r ∧ At (cfg c) (s.trans.val[j])))))
    · intro st hst
      obtain ⟨c1, any, waits, i⟩ := st
      obtain ⟨h0, h1, h2⟩ := hst
      dsimp only at h0 h1 h2 ⊢
      subst h0
      unfold progress.is_blocked_loop.body
      dsimp only
      split
      · rename_i hlt
        have hi : i.val < s.trans.val.length := by scalar_tac
        step as ⟨t, ht⟩
        have htm : t ∈ s.trans.val := by rw [ht]; exact List.getElem_mem _
        refine WP.spec_bind (Pₘ := fun p => c = p.1 ∧
          (Blocked s (cfg c) r.val → p.2.2 = true ∧
            (p.2.1 = false → any = false ∧ ¬ (t.role = r ∧ At (cfg c) t)))) ?_ ?_
        · by_cases hrole : t.role = r
          · rw [if_pos hrole]
            refine WP.spec_bind (Pₘ := fun q => Blocked s (cfg c) r.val → q.2 = true ∧
              (q.1 = false → any = false ∧ ¬ (t.role = r ∧ At (cfg c) t))) ?_ ?_
            · by_cases hlen : t.role < alloc.vec.Vec.len c.states
              · rw [if_pos hlen]
                have hrl : t.role.val < c.states.val.length := by scalar_tac
                step as ⟨st, hst⟩
                by_cases hfrom : st = t.«from»
                · rw [if_pos hfrom]
                  have hAt : At (cfg c) t :=
                    ⟨by rw [cfg_states]; exact hrl,
                     by rw [cfg_states, getElem!_pos c.states.val t.role.val hrl, ← hst]
                        exact hfrom⟩
                  refine WP.spec_bind (Pₘ := fun b2 => Blocked s (cfg c) r.val → b2 = true)
                    ?_ ?_
                  · by_cases hsend : t.send = true
                    · rw [if_pos hsend]
                      refine (WP.spec_ok _).mpr (fun hB => ?_)
                      have hf := (hB.2 t htm (by rw [hrole]) hAt).1
                      rw [hsend] at hf
                      cases hf
                    · rw [if_neg hsend]
                      apply WP.spec_bind (is_enabled_spec s c i hs hi hc)
                      intro b3 hb3
                      cases b3 with
                      | true =>
                        simp only [if_true]
                        refine (WP.spec_ok _).mpr (fun hB => ?_)
                        exfalso
                        exact (hB.2 t htm (by rw [hrole]) hAt).2 (by rw [ht]; exact hb3.mp rfl)
                      | false =>
                        simp only [Bool.false_eq_true, if_false]
                        exact (WP.spec_ok _).mpr (fun hB => (h2 hB).1)
                  · intro b2 hb2
                    exact (WP.spec_ok _).mpr (fun hB => ⟨hb2 hB, fun h => by cases h⟩)
                · rw [if_neg hfrom]
                  refine (WP.spec_ok _).mpr (fun hB => ⟨(h2 hB).1, fun hany => ⟨hany, fun hh => ?_⟩⟩)
                  have h3 := hh.2.2
                  rw [cfg_states, getElem!_pos c.states.val t.role.val hrl, ← hst] at h3
                  exact hfrom h3
              · rw [if_neg hlen]
                refine (WP.spec_ok _).mpr (fun hB => ⟨(h2 hB).1, fun hany => ⟨hany, fun hh => ?_⟩⟩)
                have h3 := hh.2.1
                rw [cfg_states] at h3
                exact hlen (by scalar_tac)
            · intro q hq
              obtain ⟨b, b1⟩ := q
              exact (WP.spec_ok _).mpr ⟨rfl, hq⟩
          · rw [if_neg hrole]
            exact (WP.spec_ok _).mpr
              ⟨rfl, fun hB => ⟨(h2 hB).1, fun hany => ⟨hany, fun hh => hrole hh.1⟩⟩⟩
        · intro p hp
          obtain ⟨c1, any1, waits1⟩ := p
          obtain ⟨hc1, hp⟩ := hp
          dsimp only at hc1 hp ⊢
          subst hc1
          apply cont_incr i s.trans.val.length hi (by scalar_tac)
          intro i2 hi2
          dsimp only
          leaves <;> first | rfl | omega | skip
          intro hB
          refine ⟨(hp hB).1, fun hany j hj hji => ?_⟩
          obtain ⟨ha, hnot⟩ := (hp hB).2 hany
          by_cases hjk : j = i.val
          · subst hjk
            rw [← ht]
            exact hnot
          · exact (h2 hB).2 ha j hj (by omega)
      · rename_i hlt
        refine (WP.spec_ok _).mpr ?_
        dsimp only
        intro hB
        obtain ⟨hw, hnone⟩ := h2 hB
        refine ⟨?_, hw⟩
        cases hany : any with
        | true => rfl
        | false =>
          exfalso
          obtain ⟨t, htm, hrv, hAt⟩ := hB.1
          obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem htm
          exact hnone hany j hj (by scalar_tac) ⟨usize_val_inj hrv, hAt⟩
    · exact ⟨rfl, by scalar_tac, fun _ => ⟨rfl, fun _ j _ hj => absurd hj (by simp)⟩⟩
  · intro p hp
    obtain ⟨any, waits⟩ := p
    dsimp only at hp ⊢
    cases any with
    | true =>
      exact (WP.spec_ok _).mpr (fun hB => (hp hB).2)
    | false =>
      exact (WP.spec_ok _).mpr (fun hB => (hp hB).1)

/-- `is_terminal`: a role that has finished is answered `true`. -/
theorem is_terminal_spec (s : progress.System) (c : progress.Config) (r : Usize) :
    progress.is_terminal s c r ⦃ b => Terminal s (cfg c) r.val → b = true ⦄ := by
  unfold progress.is_terminal
  refine WP.spec_bind (Pₘ := fun any => Terminal s (cfg c) r.val → any = false) ?_ ?_
  · unfold progress.is_terminal_loop
    apply loop.spec_decr_nat
      (fun (st : progress.Config × Bool × Usize) => s.trans.val.length - st.2.2.val)
      (fun st => c = st.1 ∧ st.2.2.val ≤ s.trans.val.length ∧
        (Terminal s (cfg c) r.val → st.2.1 = false))
    · intro st hst
      obtain ⟨c1, any, i⟩ := st
      obtain ⟨h0, h1, h2⟩ := hst
      dsimp only at h0 h1 h2 ⊢
      subst h0
      unfold progress.is_terminal_loop.body
      dsimp only
      split
      · rename_i hlt
        have hi : i.val < s.trans.val.length := by scalar_tac
        step as ⟨t, ht⟩
        have htm : t ∈ s.trans.val := by rw [ht]; exact List.getElem_mem _
        refine WP.spec_bind (Pₘ := fun p => c = p.1 ∧
          (Terminal s (cfg c) r.val → p.2 = false)) ?_ ?_
        · by_cases hrole : t.role = r
          · rw [if_pos hrole]
            refine WP.spec_bind (Pₘ := fun b => Terminal s (cfg c) r.val → b = false) ?_ ?_
            · by_cases hlen : t.role < alloc.vec.Vec.len c.states
              · rw [if_pos hlen]
                have hrl : t.role.val < c.states.val.length := by scalar_tac
                step as ⟨st, hst⟩
                by_cases hfrom : st = t.«from»
                · rw [if_pos hfrom]
                  have hAt : At (cfg c) t :=
                    ⟨by rw [cfg_states]; exact hrl,
                     by rw [cfg_states, getElem!_pos c.states.val t.role.val hrl, ← hst]
                        exact hfrom⟩
                  exact (WP.spec_ok _).mpr
                    (fun hT => absurd ⟨t, htm, by rw [hrole], hAt⟩ hT)
                · rw [if_neg hfrom]
                  exact (WP.spec_ok _).mpr h2
              · rw [if_neg hlen]
                exact (WP.spec_ok _).mpr h2
            · intro b hb
              exact (WP.spec_ok _).mpr ⟨rfl, hb⟩
          · rw [if_neg hrole]
            exact (WP.spec_ok _).mpr ⟨rfl, h2⟩
        · intro p hp
          obtain ⟨c1, any1⟩ := p
          obtain ⟨hc1, hp⟩ := hp
          dsimp only at hc1 hp ⊢
          subst hc1
          apply cont_incr i s.trans.val.length hi (by scalar_tac)
          intro i2 hi2
          dsimp only
          leaves <;> first | rfl | omega | exact hp
      · rename_i hlt
        refine (WP.spec_ok _).mpr ?_
        dsimp only
        exact h2
    · exact ⟨rfl, by scalar_tac, fun _ => rfl⟩
  · intro any hany
    refine (WP.spec_ok _).mpr (fun hT => ?_)
    rw [hany hT]
    rfl

/-! ## The waiting-set decision -/

/-- Every waiting set lies inside `cur`. -/
def Sub (s : progress.System) (c : Cfg) (cur : List Bool) : Prop :=
  ∀ set, DeadlockSet s c set → ∀ q, q < s.roles.val → set[q]! = true → cur[q]! = true

/-- The first loop: the roles that await or have finished. -/
theorem dl_init_spec (s : progress.System) (c : progress.Config) (hs : WfSys s)
    (hc : WfCfg s.roles.val (cfg c)) :
    progress.has_deadlock_loop0 s c (alloc.vec.Vec.new Bool) 0#usize
      ⦃ r => r.1 = s.roles ∧ r.2.1 = s.capacity ∧ r.2.2.1 = s.trans ∧
        r.2.2.2.val.length = s.roles.val ∧
        ∀ q, q < s.roles.val → (Blocked s (cfg c) q ∨ Terminal s (cfg c) q) →
          r.2.2.2.val[q]! = true ⦄ := by
  unfold progress.has_deadlock_loop0
  apply loop.spec_decr_nat
    (fun (st : alloc.vec.Vec Bool × Usize) => s.roles.val - st.2.val)
    (fun st => st.2.val ≤ s.roles.val ∧ st.1.val.length = st.2.val ∧
      ∀ q, q < st.2.val → (Blocked s (cfg c) q ∨ Terminal s (cfg c) q) → st.1.val[q]! = true)
  · intro st hst
    obtain ⟨cur, r⟩ := st
    obtain ⟨h1, h2, h3⟩ := hst
    dsimp only at h1 h2 h3 ⊢
    unfold progress.has_deadlock_loop0.body
    split
    · rename_i hlt
      have hr : r.val < s.roles.val := by scalar_tac
      have hlen : cur.val.length < Usize.max := by
        have h16 := hs.1
        scalar_tac
      apply WP.spec_bind (is_blocked_spec s c r hs hc)
      intro b hb
      refine WP.spec_bind (Pₘ := fun b1 =>
        (Blocked s (cfg c) r.val ∨ Terminal s (cfg c) r.val) → b1 = true) ?_ ?_
      · cases b with
        | true =>
          simp only [if_true]
          exact (WP.spec_ok _).mpr (fun _ => rfl)
        | false =>
          simp only [Bool.false_eq_true, if_false]
          refine WP.spec_mono (is_terminal_spec s c r) ?_
          intro b1 hb1 h
          rcases h with h | h
          · exact absurd (hb h) (by simp)
          · exact hb1 h
      · intro b1 hb1
        apply WP.spec_bind (alloc.vec.Vec.push_spec cur b1 hlen)
        intro cur1 hc1
        apply cont_incr r s.roles.val hr (by scalar_tac)
        intro r1 hr1
        have hl1 : cur1.val.length = r1.val := by
          rw [hc1, List.length_append, List.length_singleton]; omega
        dsimp only
        leaves <;> first | omega | skip
        intro q hq hbt
        rw [hc1]
        by_cases hqr : q = r.val
        · rw [hqr, ← h2, getElem!_append_len]
          exact hb1 (hqr ▸ hbt)
        · rw [getElem!_append_lt _ _ q (by omega)]
          exact h3 q (by omega) hbt
    · rename_i hlt
      refine (WP.spec_ok _).mpr ?_
      dsimp only
      have e : r.val = s.roles.val := by scalar_tac
      exact ⟨rfl, rfl, rfl, by omega, fun q hq hbt => h3 q (by omega) hbt⟩
  · exact ⟨by scalar_tac, rfl, fun q hq => absurd hq (by simp)⟩

/-- The innermost loop: one member `q` of the set is removed when it
awaits a sender outside the set. -/
theorem dl_inner_spec (s : progress.System) (c : progress.Config) (hs : WfSys s)
    (cur : alloc.vec.Vec Bool) (changed : Bool) (q : Usize) (hq : q.val < s.roles.val)
    (hlen : cur.val.length = s.roles.val) (hsub : Sub s (cfg c) cur.val)
    (hcq : cur.val[q.val]! = true) :
    progress.has_deadlock_loop1_loop0_loop0 s.trans c cur changed q 0#usize false
      ⦃ r => c = r.1 ∧ r.2.1.val.length = s.roles.val ∧ Sub s (cfg c) r.2.1.val ∧
        ((r.2.1 = cur ∧ r.2.2 = changed) ∨
          (r.2.1.val.count true + 1 = cur.val.count true ∧ r.2.2 = true)) ⦄ := by
  unfold progress.has_deadlock_loop1_loop0_loop0
  apply loop.spec_decr_nat
    (fun (st : progress.Config × alloc.vec.Vec Bool × Bool × Usize × Bool) =>
      s.trans.val.length - st.2.2.2.1.val)
    (fun st => c = st.1 ∧ st.2.2.2.1.val ≤ s.trans.val.length ∧
      st.2.1.val.length = s.roles.val ∧ Sub s (cfg c) st.2.1.val ∧
      ((st.2.2.2.2 = false ∧ st.2.1 = cur ∧ st.2.2.1 = changed) ∨
        (st.2.2.2.2 = true ∧ st.2.1.val.count true + 1 = cur.val.count true ∧
          st.2.2.1 = true)))
  · intro st hst
    obtain ⟨c1, cur1, changed1, i, removed⟩ := st
    obtain ⟨h0, h1, h2, h3, h4⟩ := hst
    dsimp only at h0 h1 h2 h3 h4 ⊢
    subst h0
    unfold progress.has_deadlock_loop1_loop0_loop0.body
    dsimp only
    split
    · rename_i hlt
      have hi : i.val < s.trans.val.length := by scalar_tac
      step as ⟨t, ht⟩
      have htm : t ∈ s.trans.val := by rw [ht]; exact List.getElem_mem _
      obtain ⟨hr, hp, _⟩ := hs.2 t htm
      refine WP.spec_bind (Pₘ := fun p => c = p.1 ∧
        p.2.1.val.length = s.roles.val ∧ Sub s (cfg c) p.2.1.val ∧
        ((p.2.2.2 = false ∧ p.2.1 = cur ∧ p.2.2.1 = changed) ∨
          (p.2.2.2 = true ∧ p.2.1.val.count true + 1 = cur.val.count true ∧
            p.2.2.1 = true))) ?_ ?_
      · by_cases hrem : removed = true
        · rw [if_pos hrem]
          exact (WP.spec_ok _).mpr ⟨rfl, h2, h3, h4⟩
        · rw [if_neg hrem]
          by_cases hrole : t.role = q
          · rw [if_pos hrole]
            refine WP.spec_bind (Pₘ := fun p =>
              p.1.val.length = s.roles.val ∧ Sub s (cfg c) p.1.val ∧
              ((p.2.2 = false ∧ p.1 = cur ∧ p.2.1 = changed) ∨
                (p.2.2 = true ∧ p.1.val.count true + 1 = cur.val.count true ∧
                  p.2.1 = true))) ?_ ?_
            · by_cases hlen' : t.role < alloc.vec.Vec.len c.states
              · rw [if_pos hlen']
                have hrl : t.role.val < c.states.val.length := by scalar_tac
                step as ⟨st, hst⟩
                by_cases hfrom : st = t.«from»
                · rw [if_pos hfrom]
                  have hAt : At (cfg c) t :=
                    ⟨by rw [cfg_states]; exact hrl,
                     by rw [cfg_states, getElem!_pos c.states.val t.role.val hrl, ← hst]
                        exact hfrom⟩
                  have hpl : t.peer.val < cur1.val.length := by omega
                  step as ⟨b2, hb2⟩
                  cases b2 with
                  | true =>
                    simp only [if_true]
                    exact (WP.spec_ok _).mpr ⟨h2, h3, h4⟩
                  | false =>
                    simp only [Bool.false_eq_true, if_false]
                    have hql : q.val < cur1.val.length := by omega
                    apply WP.spec_bind (vset_spec cur1 q false hql)
                    intro cur2 hc2
                    have hcc : cur1 = cur := by
                      rcases h4 with ⟨_, e, _⟩ | ⟨e0, _, _⟩
                      · exact e
                      · exact absurd e0 hrem
                    refine (WP.spec_ok _).mpr ⟨?_, ?_, Or.inr ⟨rfl, ?_, rfl⟩⟩
                    · show cur2.val.length = s.roles.val
                      rw [hc2, List.length_set]
                      exact h2
                    · show Sub s (cfg c) cur2.val
                      intro set hset q' hq' hsq'
                      rw [hc2]
                      by_cases e : q.val = q'
                      · exfalso
                        have hsq : set[t.role.val]! = true := by rw [hrole, e]; exact hsq'
                        have hpeer := hset.2.2.2 t htm hsq hAt
                        have hcp := h3 set hset t.peer.val hp hpeer
                        rw [getElem!_pos cur1.val t.peer.val hpl, ← hb2] at hcp
                        cases hcp
                      · rw [getElem!_set_ne' _ _ _ _ e]
                        exact h3 set hset q' hq' hsq'
                    · show cur2.val.count true + 1 = cur.val.count true
                      rw [hc2, hcc]
                      have hqc : q.val < cur.val.length := by omega
                      exact count_set_false cur.val q.val hqc
                        (by rw [← getElem!_pos cur.val q.val hqc]; exact hcq)
                · rw [if_neg hfrom]
                  exact (WP.spec_ok _).mpr ⟨h2, h3, h4⟩
              · rw [if_neg hlen']
                exact (WP.spec_ok _).mpr ⟨h2, h3, h4⟩
            · intro p hp'
              obtain ⟨v1, b, b1⟩ := p
              exact (WP.spec_ok _).mpr ⟨rfl, hp'⟩
          · rw [if_neg hrole]
            exact (WP.spec_ok _).mpr ⟨rfl, h2, h3, h4⟩
      · intro p hp'
        obtain ⟨c1, cur2, changed2, removed2⟩ := p
        obtain ⟨hc1, hl2, hs2, hd2⟩ := hp'
        dsimp only at hc1 hl2 hs2 hd2 ⊢
        subst hc1
        apply cont_incr i s.trans.val.length hi (by scalar_tac)
        intro i2 hi2
        dsimp only
        leaves <;> first | rfl | omega | assumption
    · rename_i hlt
      refine (WP.spec_ok _).mpr ?_
      dsimp only
      refine ⟨rfl, h2, h3, ?_⟩
      rcases h4 with ⟨_, e1, e2⟩ | ⟨_, h⟩
      · exact Or.inl ⟨e1, e2⟩
      · exact Or.inr h
  · exact ⟨rfl, by scalar_tac, hlen, hsub, Or.inl ⟨rfl, rfl, rfl⟩⟩

/-- One round over the roles: the set shrinks or stays, every waiting set
still inside it, and a round that reports a change has removed a member. -/
theorem dl_round_spec (s : progress.System) (c : progress.Config) (hs : WfSys s)
    (cur0 : alloc.vec.Vec Bool) (hlen : cur0.val.length = s.roles.val)
    (hsub : Sub s (cfg c) cur0.val) :
    progress.has_deadlock_loop1_loop0 s.roles s.trans c cur0 false 0#usize
      ⦃ r => c = r.1 ∧ r.2.1.val.length = s.roles.val ∧ Sub s (cfg c) r.2.1.val ∧
        r.2.1.val.count true ≤ cur0.val.count true ∧
        (r.2.2 = true → r.2.1.val.count true < cur0.val.count true) ⦄ := by
  unfold progress.has_deadlock_loop1_loop0
  apply loop.spec_decr_nat
    (fun (st : progress.Config × alloc.vec.Vec Bool × Bool × Usize) =>
      s.roles.val - st.2.2.2.val)
    (fun st => c = st.1 ∧ st.2.2.2.val ≤ s.roles.val ∧
      st.2.1.val.length = s.roles.val ∧ Sub s (cfg c) st.2.1.val ∧
      st.2.1.val.count true ≤ cur0.val.count true ∧
      (st.2.2.1 = true → st.2.1.val.count true < cur0.val.count true))
  · intro st hst
    obtain ⟨c1, cur, changed, q⟩ := st
    obtain ⟨h0, h1, h2, h3, h5, h6⟩ := hst
    dsimp only at h0 h1 h2 h3 h5 h6 ⊢
    subst h0
    unfold progress.has_deadlock_loop1_loop0.body
    split
    · rename_i hlt
      have hq : q.val < s.roles.val := by scalar_tac
      have hql : q.val < cur.val.length := by omega
      step as ⟨b, hb⟩
      cases b with
      | true =>
        simp only [if_true]
        have hcq : cur.val[q.val]! = true := by
          rw [getElem!_pos cur.val q.val hql]; exact hb.symm
        apply WP.spec_bind (dl_inner_spec s c hs cur changed q hq h2 h3 hcq)
        intro p hp
        obtain ⟨c1, cur1, changed1⟩ := p
        obtain ⟨hc1, hl1, hs1, hd⟩ := hp
        dsimp only at hc1 hl1 hs1 hd ⊢
        subst hc1
        apply cont_incr q s.roles.val hq (by scalar_tac)
        intro q1 hq1
        dsimp only
        rcases hd with ⟨e1, e2⟩ | ⟨e1, e2⟩
        · rw [e1, e2]
          leaves <;> first | rfl | omega | assumption
        · leaves <;> first | rfl | omega | assumption | (intro _; omega)
      | false =>
        simp only [Bool.false_eq_true, if_false]
        apply cont_incr q s.roles.val hq (by scalar_tac)
        intro q1 hq1
        dsimp only
        leaves <;> first | rfl | omega | assumption
    · rename_i hlt
      refine (WP.spec_ok _).mpr ?_
      dsimp only
      exact ⟨rfl, h2, h3, h5, h6⟩
  · exact ⟨rfl, by scalar_tac, hlen, hsub, le_refl _, fun h => by cases h⟩

/-- The rounds, to the fixed point: it ends, with every waiting set inside
the result. -/
theorem dl_fix_spec (s : progress.System) (c : progress.Config) (hs : WfSys s)
    (cur0 : alloc.vec.Vec Bool) (hlen : cur0.val.length = s.roles.val)
    (hsub : Sub s (cfg c) cur0.val) :
    progress.has_deadlock_loop1 s.roles s.trans c cur0 true
      ⦃ r => c = r.1 ∧ r.2.val.length = s.roles.val ∧ Sub s (cfg c) r.2.val ⦄ := by
  unfold progress.has_deadlock_loop1
  apply loop.spec_decr_nat
    (fun (st : progress.Config × alloc.vec.Vec Bool × Bool) =>
      2 * st.2.1.val.count true + (if st.2.2 = true then 1 else 0))
    (fun st => c = st.1 ∧ st.2.1.val.length = s.roles.val ∧ Sub s (cfg c) st.2.1.val)
  · intro st hst
    obtain ⟨c1, cur, changed⟩ := st
    obtain ⟨h0, h2, h3⟩ := hst
    dsimp only at h0 h2 h3 ⊢
    subst h0
    unfold progress.has_deadlock_loop1.body
    cases changed with
    | true =>
      simp only [if_true]
      apply WP.spec_bind (dl_round_spec s c hs cur h2 h3)
      intro p hp
      obtain ⟨c1, cur1, changed1⟩ := p
      obtain ⟨hc1, hl1, hs1, h5, h6⟩ := hp
      dsimp only at hc1 hl1 hs1 h5 h6 ⊢
      subst hc1
      refine (WP.spec_ok _).mpr ?_
      dsimp only
      leaves <;> first | rfl | assumption | skip
      cases changed1 with
      | false =>
        simp only [Bool.false_eq_true, if_false, if_true]
        omega
      | true =>
        have h7 := h6 rfl
        simp only [if_true]
        omega
    | false =>
      simp only [Bool.false_eq_true, if_false]
      refine (WP.spec_ok _).mpr ?_
      dsimp only
      exact ⟨rfl, h2, h3⟩
  · exact ⟨rfl, hlen, hsub⟩

/-- The last loop: `false` only when no member of the set awaits. -/
theorem dl_any_spec (s : progress.System) (c : progress.Config) (hs : WfSys s)
    (hc : WfCfg s.roles.val (cfg c)) (cur : alloc.vec.Vec Bool)
    (hlen : cur.val.length = s.roles.val) :
    progress.has_deadlock_loop2 s.roles s.capacity s.trans c cur false 0#usize
      ⦃ b => b = false → ∀ y, y < s.roles.val → cur.val[y]! = true →
        ¬ Blocked s (cfg c) y ⦄ := by
  unfold progress.has_deadlock_loop2
  apply loop.spec_decr_nat
    (fun (st : Bool × Usize) => s.roles.val - st.2.val)
    (fun st => st.2.val ≤ s.roles.val ∧
      (st.1 = false → ∀ y, y < st.2.val → cur.val[y]! = true → ¬ Blocked s (cfg c) y))
  · intro st hst
    obtain ⟨any, x⟩ := st
    obtain ⟨h1, h2⟩ := hst
    dsimp only at h1 h2 ⊢
    unfold progress.has_deadlock_loop2.body
    split
    · rename_i hlt
      have hx : x.val < s.roles.val := by scalar_tac
      have hxl : x.val < cur.val.length := by omega
      step as ⟨b, hb⟩
      refine WP.spec_bind (Pₘ := fun any1 => any1 = false →
        any = false ∧ (cur.val[x.val]! = true → ¬ Blocked s (cfg c) x.val)) ?_ ?_
      · cases b with
        | true =>
          simp only [if_true]
          apply WP.spec_bind (is_blocked_spec s c x hs hc)
          intro b1 hb1
          cases b1 with
          | true =>
            simp only [if_true]
            exact (WP.spec_ok _).mpr (fun h => by cases h)
          | false =>
            simp only [Bool.false_eq_true, if_false]
            exact (WP.spec_ok _).mpr (fun h => ⟨h, fun _ hB => by
              have h3 := hb1 hB
              cases h3⟩)
        | false =>
          simp only [Bool.false_eq_true, if_false]
          exact (WP.spec_ok _).mpr (fun h => ⟨h, fun hcx => by
            rw [getElem!_pos cur.val x.val hxl, ← hb] at hcx
            cases hcx⟩)
      · intro any1 ha1
        apply cont_incr x s.roles.val hx (by scalar_tac)
        intro x1 hx1
        dsimp only
        leaves <;> first | omega | skip
        intro hf y hy hcy
        obtain ⟨ha, hxb⟩ := ha1 hf
        by_cases hyx : y = x.val
        · rw [hyx] at hcy ⊢
          exact hxb hcy
        · exact h2 ha y (by omega) hcy
    · rename_i hlt
      refine (WP.spec_ok _).mpr ?_
      dsimp only
      have e : x.val = s.roles.val := by scalar_tac
      exact fun hf y hy hcy => h2 hf y (by omega) hcy
  · exact ⟨by scalar_tac, fun _ y hy => absurd hy (by simp)⟩

/-- `has_deadlock`: it returns, and a `false` means that the configuration
has no waiting set. -/
theorem has_deadlock_spec (s : progress.System) (c : progress.Config) (hs : WfSys s)
    (hc : WfCfg s.roles.val (cfg c)) :
    progress.has_deadlock s c ⦃ b => b = false → ¬ Deadlocked s (cfg c) ⦄ := by
  unfold progress.has_deadlock
  apply WP.spec_bind (dl_init_spec s c hs hc)
  intro p hp
  obtain ⟨i, i1, v, cur⟩ := p
  obtain ⟨e1, e2, e3, hlen, hbt⟩ := hp
  dsimp only at e1 e2 e3 hlen hbt ⊢
  subst e1
  subst e2
  subst e3
  have hsub0 : Sub s (cfg c) cur.val := by
    intro set hset q hq hsq
    exact hbt q hq (hset.2.2.1 q (by rw [hset.1]; exact hq) hsq)
  apply WP.spec_bind (dl_fix_spec s c hs cur hlen hsub0)
  intro p hp
  obtain ⟨c1, cur1⟩ := p
  obtain ⟨hc1, hl1, hs1⟩ := hp
  dsimp only at hc1 hl1 hs1 ⊢
  subst hc1
  refine WP.spec_mono (dl_any_spec s c hs hc cur1 hl1) ?_
  intro b hb hbf hdead
  obtain ⟨set, hset⟩ := hdead
  obtain ⟨r, hr, hsr, hB⟩ := hset.2.1
  have hr' : r < s.roles.val := by rw [← hset.1]; exact hr
  exact hb hbf r hr' (hs1 set hset r hr' hsr) hB

/-! ## The checker -/

/-- `successors_listed`: `true` only when every enabled operation of the
configuration leads to a configuration of the list. -/
theorem successors_listed_spec (s : progress.System) (list : alloc.vec.Vec progress.Config)
    (c : progress.Config) (hs : WfSys s) (hc : WfCfg s.roles.val (cfg c))
    (hcap : WithinCapacity s (cfg c)) (hmax : s.capacity.val < Usize.max) :
    progress.successors_listed s list c
      ⦃ r => r = true → ∀ t, t ∈ s.trans.val → Enabled s.roles.val (cfg c) t →
        succ s.roles.val (cfg c) t ∈ list.val.map cfg ⦄ := by
  unfold progress.successors_listed progress.successors_listed_loop
  refine WP.spec_mono (scan_loop _ s.trans.val.length
    (fun j => ∀ (h : j < s.trans.val.length), Enabled s.roles.val (cfg c) (s.trans.val[j]) →
      succ s.roles.val (cfg c) (s.trans.val[j]) ∈ list.val.map cfg) ?_ ?_) ?_
  · intro j hj
    unfold progress.successors_listed_loop.body
    dsimp only
    have hc' : ¬ j < alloc.vec.Vec.len s.trans := by scalar_tac
    rw [if_neg hc']
  · intro j hj _
    unfold progress.successors_listed_loop.body
    dsimp only
    have hc' : j < alloc.vec.Vec.len s.trans := by scalar_tac
    rw [if_pos hc']
    apply WP.spec_bind (is_enabled_spec s c j hs hj hc)
    intro b hb
    cases b with
    | true =>
      simp only [if_true]
      have hen := hb.mp rfl
      apply WP.spec_bind (successor_spec s c j hs hj hc hcap hmax hen)
      intro next hnext
      apply WP.spec_bind (contains_spec list next)
      intro b1 hb1
      cases b1 with
      | true =>
        simp only [if_true]
        apply cont_incr j s.trans.val.length hj (by scalar_tac)
        intro j1 hj1
        exact Or.inr ⟨j1, rfl, hj1, fun _ _ => by rw [← hnext]; exact hb1 rfl⟩
      | false =>
        simp only [Bool.false_eq_true, if_false]
        exact (WP.spec_ok _).mpr (Or.inl rfl)
    | false =>
      simp only [Bool.false_eq_true, if_false]
      apply cont_incr j s.trans.val.length hj (by scalar_tac)
      intro j1 hj1
      exact Or.inr ⟨j1, rfl, hj1, fun _ hen => absurd (hb.mpr hen) (by simp)⟩
  · intro r hr h t htm hen
    obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem htm
    exact hr h j hj hj hen

/-- What `certificate` asks of one configuration of the list. -/
def Closed (s : progress.System) (list : alloc.vec.Vec progress.Config)
    (c : progress.Config) : Prop :=
  WfCfg s.roles.val (cfg c) ∧ WithinCapacity s (cfg c) ∧ ¬ Deadlocked s (cfg c) ∧
  ∀ t, t ∈ s.trans.val → Enabled s.roles.val (cfg c) t →
    succ s.roles.val (cfg c) t ∈ list.val.map cfg

/-- `configs_closed`: `true` only when every configuration of the list is
well formed, within the capacity and without a waiting set, with every
enabled operation leading to a configuration of the list. -/
theorem configs_closed_spec (s : progress.System) (list : alloc.vec.Vec progress.Config)
    (hs : WfSys s) (hmax : s.capacity.val < Usize.max) :
    progress.configs_closed s list
      ⦃ r => r = true → ∀ c, c ∈ list.val → Closed s list c ⦄ := by
  unfold progress.configs_closed progress.configs_closed_loop
  refine WP.spec_mono (scan_loop _ list.val.length
    (fun j => ∀ (h : j < list.val.length), Closed s list (list.val[j])) ?_ ?_) ?_
  · intro i hi
    unfold progress.configs_closed_loop.body
    dsimp only
    have hc' : ¬ i < alloc.vec.Vec.len list := by scalar_tac
    rw [if_neg hc']
  · intro i hi _
    unfold progress.configs_closed_loop.body
    dsimp only
    have hc' : i < alloc.vec.Vec.len list := by scalar_tac
    rw [if_pos hc']
    step as ⟨c, hc⟩
    apply WP.spec_bind (wf_config_spec s c hs.1)
    intro b hb
    cases b with
    | false =>
      simp only [Bool.false_eq_true, if_false]
      exact (WP.spec_ok _).mpr (Or.inl rfl)
    | true =>
      simp only [if_true]
      have hwf := hb rfl
      apply WP.spec_bind (capacity_ok_spec s c)
      intro b1 hb1
      cases b1 with
      | false =>
        simp only [Bool.false_eq_true, if_false]
        exact (WP.spec_ok _).mpr (Or.inl rfl)
      | true =>
        simp only [if_true]
        have hcap := hb1 rfl
        apply WP.spec_bind (has_deadlock_spec s c hs hwf)
        intro b2 hb2
        cases b2 with
        | true =>
          simp only [if_true]
          exact (WP.spec_ok _).mpr (Or.inl rfl)
        | false =>
          simp only [Bool.false_eq_true, if_false]
          have hnd := hb2 rfl
          apply WP.spec_bind (successors_listed_spec s list c hs hwf hcap hmax)
          intro b3 hb3
          cases b3 with
          | true =>
            simp only [if_true]
            apply cont_incr i list.val.length hi (by scalar_tac)
            intro i2 hi2
            exact Or.inr ⟨i2, rfl, hi2, fun _ => by rw [← hc]; exact ⟨hwf, hcap, hnd, hb3 rfl⟩⟩
          | false =>
            simp only [Bool.false_eq_true, if_false]
            exact (WP.spec_ok _).mpr (Or.inl rfl)
  · intro r hr h c hc
    obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hc
    exact hr h j hj hj

/-- **The checker on the code as compiled.** For a well-formed system
`certify` returns, and it answers `true` only when the list's view is a
certificate. -/
theorem certify_spec (s : progress.System) (list : alloc.vec.Vec progress.Config)
    (hs : WfSys s) (hmax : s.capacity.val < Usize.max) :
    progress.certify s list ⦃ r => r = true → Certificate s (list.val.map cfg) ⦄ := by
  unfold progress.certify
  apply WP.spec_bind (initial_config_spec s.roles hs.1)
  intro init hinit
  apply WP.spec_bind (contains_spec list init)
  intro b hb
  cases b with
  | false =>
    simp only [Bool.false_eq_true, if_false]
    exact (WP.spec_ok _).mpr (fun h => by cases h)
  | true =>
    simp only [if_true]
    refine WP.spec_mono (configs_closed_spec s list hs hmax) ?_
    intro r hr h
    refine ⟨by rw [← hinit]; exact hb rfl, ?_, ?_⟩
    · intro c hc
      obtain ⟨c0, hc0, rfl⟩ := List.mem_map.mp hc
      obtain ⟨h1, h2, h3, _⟩ := hr h c0 hc0
      exact ⟨h1, h2, h3⟩
    · intro c t hc ht hen
      obtain ⟨c0, hc0, rfl⟩ := List.mem_map.mp hc
      exact (hr h c0 hc0).2.2.2 t ht hen

/-- `certify` returns for every list. -/
theorem certify_returns (s : progress.System) (list : alloc.vec.Vec progress.Config)
    (hs : WfSys s) (hmax : s.capacity.val < Usize.max) :
    ∃ b, progress.certify s list = ok b := by
  obtain ⟨b, hb, _⟩ := spec_returns (certify_spec s list hs hmax)
  exact ⟨b, hb⟩

/-- **R41.3 and R41.1 from the checker's answer.** When `certify`, as
compiled, answers `true` for a well-formed system and a list, no
configuration the system reaches has a waiting set, and none has a channel
over the declared capacity. -/
theorem certify_sound (s : progress.System) (list : alloc.vec.Vec progress.Config)
    (hs : WfSys s) (hmax : s.capacity.val < Usize.max)
    (h : progress.certify s list = ok true) {c : Cfg} (hr : Reach s c) :
    ¬ Deadlocked s c ∧ WithinCapacity s c := by
  have hsp := certify_spec s list hs hmax
  rw [h] at hsp
  exact certificate_sound ((WP.spec_ok _).mp hsp rfl) hr

end PC
