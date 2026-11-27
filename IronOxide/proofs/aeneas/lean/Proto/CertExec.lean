import Proto.CertSpec
import Proto.RoleOps
open Aeneas Aeneas.Std Result ControlFlow
/-!
# The progress certificate: the checker's parts on the code as compiled

`progress::certify` (`crates/rsk-protocol/src/progress.rs`) checks the
explorer's list of configurations against `certificate`. This file reads
the functions it is made of, as Aeneas translates them (`Proto/Proto.lean`):
the copies and comparisons (`copy_from`, `eq_u32`, `eq_usize`, `eq_cfg`,
`contains`), the initial configuration, the checks of one configuration
(`wf_config`, `capacity_ok`), and the operations (`chan_index`,
`is_enabled`, `successor`). Each theorem says that the function returns
and what it returns, over `Proto/CertSpec.lean`. `Proto/Certify.lean` has
the waiting-set decision and the checker.
-/

namespace PC
open rsk_protocol MP

/-- `cfg`: the view of a configuration. -/
def cfg (c : progress.Config) : Cfg := ⟨c.states.val, c.chans.val.map (fun v => v.val)⟩

theorem cfg_states (c : progress.Config) : (cfg c).states = c.states.val := rfl
theorem cfg_chans (c : progress.Config) : (cfg c).chans = c.chans.val.map (fun v => v.val) := rfl

instance : Inhabited progress.Trans := ⟨⟨0#usize, 0#usize, false, 0#usize, 0#u32, 0#usize⟩⟩
instance : Inhabited progress.Config := ⟨⟨alloc.vec.Vec.new _, alloc.vec.Vec.new _⟩⟩

theorem false_iff' {p : Prop} (h : ¬ p) : (false = true ↔ p) :=
  ⟨fun e => (by cases e), fun q => absurd q h⟩

theorem true_iff' {p : Prop} (h : p) : (true = true ↔ p) := ⟨fun _ => h, fun _ => rfl⟩

theorem of_not_bne {α} [BEq α] [LawfulBEq α] {a b : α} (h : ¬ (a != b) = true) : a = b := by
  by_contra hc
  exact h (bne_iff_ne.mpr hc)

theorem of_bne {α} [BEq α] [LawfulBEq α] {a b : α} (h : (a != b) = true) : a ≠ b :=
  bne_iff_ne.mp h

theorem vec_len_eq {α} {v w : alloc.vec.Vec α}
    (h : ¬ (alloc.vec.Vec.len v != alloc.vec.Vec.len w) = true) :
    v.val.length = w.val.length := by
  have h1 : alloc.vec.Vec.len v = alloc.vec.Vec.len w := of_not_bne h
  scalar_tac

/-! ## A rule for scanning loops

An index runs to a bound; an iteration either answers `false` or advances
the index by one, the index passing; past the bound the answer is `true`.
Then the loop returns, and answers `true` only when every index passes. -/
theorem scan_loop (body : Usize → Result (ControlFlow Usize Bool)) (n : Nat) (P : Nat → Prop)
    (hdone : ∀ i : Usize, ¬ i.val < n → body i = ok (done true))
    (hstep : ∀ i : Usize, i.val < n → (∀ j, j < i.val → P j) →
      body i ⦃ r => r = done false ∨
        ∃ i1 : Usize, r = cont i1 ∧ i1.val = i.val + 1 ∧ P i.val ⦄) :
    loop body 0#usize ⦃ b => b = true → ∀ j, j < n → P j ⦄ := by
  apply loop.spec_decr_nat (fun (i : Usize) => n - i.val)
    (fun i => i.val ≤ n ∧ ∀ j, j < i.val → P j)
  · intro i hi
    obtain ⟨h1, h2⟩ := hi
    by_cases hlt : i.val < n
    · refine WP.spec_mono (hstep i hlt h2) ?_
      intro r hr
      rcases hr with rfl | ⟨i1, rfl, hi1, hp⟩
      · dsimp only
        intro h; cases h
      · dsimp only
        refine ⟨⟨by omega, ?_⟩, by omega⟩
        intro j hj
        by_cases hji : j = i.val
        · rw [hji]; exact hp
        · exact h2 j (by omega)
    · rw [hdone i hlt]
      simp only [WP.spec_ok]
      intro _ j hj
      exact h2 j (by omega)
  · exact ⟨by scalar_tac, fun j hj => absurd hj (by simp)⟩

/-! ## Copies and comparisons -/

/-- `copy_from`: the elements from `start` on. -/
theorem copy_from_spec (src : alloc.vec.Vec U32) (start : Usize)
    (h : start.val ≤ src.val.length) :
    progress.copy_from src start ⦃ r => r.val = src.val.drop start.val ⦄ := by
  unfold progress.copy_from progress.copy_from_loop
  apply loop.spec_decr_nat
    (fun (st : alloc.vec.Vec U32 × Usize) => src.val.length - st.2.val)
    (fun st => start.val ≤ st.2.val ∧ st.2.val ≤ src.val.length ∧
      st.1.val ++ src.val.drop st.2.val = src.val.drop start.val)
  · intro st hst
    obtain ⟨out, i⟩ := st
    obtain ⟨h1, h2, h3⟩ := hst
    dsimp only at h1 h2 h3 ⊢
    unfold progress.copy_from_loop.body
    dsimp only
    split
    · rename_i hlt
      have hi : i.val < src.val.length := by scalar_tac
      have hlen : out.val.length < Usize.max := by
        have hp := src.property
        have hl := congrArg List.length h3
        simp only [List.length_append, List.length_drop] at hl
        omega
      step as ⟨x, hx⟩
      step as ⟨out1, hout1⟩
      apply cont_incr i src.val.length hi (by scalar_tac)
      intro i3 hi3
      dsimp only
      leaves <;> first | omega | skip
      rw [hout1, hi3, ← h3, List.append_assoc]
      congr 1
      rw [hx]
      exact (List.drop_eq_getElem_cons hi).symm
    · rename_i hlt
      simp only [WP.spec_ok]
      rw [← h3, List.drop_of_length_le (by scalar_tac), List.append_nil]
  · exact ⟨le_refl _, h, rfl⟩

/-- `eq_u32`: `true` only for equal contents. -/
theorem eq_u32_spec (a b : alloc.vec.Vec U32) :
    progress.eq_u32 a b ⦃ r => r = true → a.val = b.val ⦄ := by
  unfold progress.eq_u32
  dsimp only
  split
  · exact (WP.spec_ok _).mpr (fun h => by cases h)
  · rename_i hne
    have hl : a.val.length = b.val.length := vec_len_eq hne
    unfold progress.eq_u32_loop
    refine WP.spec_mono (scan_loop _ a.val.length (fun j => a.val[j]? = b.val[j]?) ?_ ?_) ?_
    · intro i hi
      unfold progress.eq_u32_loop.body
      dsimp only
      have hc : ¬ i < alloc.vec.Vec.len a := by scalar_tac
      rw [if_neg hc]
    · intro i hi _
      unfold progress.eq_u32_loop.body
      dsimp only
      have hc : i < alloc.vec.Vec.len a := by scalar_tac
      rw [if_pos hc]
      have hib : i.val < b.val.length := by omega
      step as ⟨x, hx⟩
      step as ⟨y, hy⟩
      split
      · exact (WP.spec_ok _).mpr (Or.inl rfl)
      · rename_i hxy
        have exy : x = y := of_not_bne hxy
        apply cont_incr i a.val.length hi (by scalar_tac)
        intro i4 hi4
        refine Or.inr ⟨i4, rfl, hi4, ?_⟩
        rw [List.getElem?_eq_getElem hi, List.getElem?_eq_getElem hib, ← hx, ← hy, exy]
    · intro r hr h
      apply List.ext_getElem?
      intro k
      by_cases hk : k < a.val.length
      · exact hr h k hk
      · rw [List.getElem?_eq_none (by omega), List.getElem?_eq_none (by omega)]

/-- `eq_usize`: `true` only for equal contents. -/
theorem eq_usize_spec (a b : alloc.vec.Vec Usize) :
    progress.eq_usize a b ⦃ r => r = true → a.val = b.val ⦄ := by
  unfold progress.eq_usize
  dsimp only
  split
  · exact (WP.spec_ok _).mpr (fun h => by cases h)
  · rename_i hne
    have hl : a.val.length = b.val.length := vec_len_eq hne
    unfold progress.eq_usize_loop
    refine WP.spec_mono (scan_loop _ a.val.length (fun j => a.val[j]? = b.val[j]?) ?_ ?_) ?_
    · intro i hi
      unfold progress.eq_usize_loop.body
      dsimp only
      have hc : ¬ i < alloc.vec.Vec.len a := by scalar_tac
      rw [if_neg hc]
    · intro i hi _
      unfold progress.eq_usize_loop.body
      dsimp only
      have hc : i < alloc.vec.Vec.len a := by scalar_tac
      rw [if_pos hc]
      have hib : i.val < b.val.length := by omega
      step as ⟨x, hx⟩
      step as ⟨y, hy⟩
      split
      · exact (WP.spec_ok _).mpr (Or.inl rfl)
      · rename_i hxy
        have exy : x = y := of_not_bne hxy
        apply cont_incr i a.val.length hi (by scalar_tac)
        intro i4 hi4
        refine Or.inr ⟨i4, rfl, hi4, ?_⟩
        rw [List.getElem?_eq_getElem hi, List.getElem?_eq_getElem hib, ← hx, ← hy, exy]
    · intro r hr h
      apply List.ext_getElem?
      intro k
      by_cases hk : k < a.val.length
      · exact hr h k hk
      · rw [List.getElem?_eq_none (by omega), List.getElem?_eq_none (by omega)]

/-- `eq_cfg`: `true` only for configurations with one view. -/
theorem eq_cfg_spec (a b : progress.Config) :
    progress.eq_cfg a b ⦃ r => r = true → cfg a = cfg b ⦄ := by
  unfold progress.eq_cfg
  apply WP.spec_bind (eq_usize_spec a.states b.states)
  intro b1 hb1
  cases b1 with
  | false =>
    simp only [Bool.false_eq_true, if_false]
    exact (WP.spec_ok _).mpr (fun h => by cases h)
  | true =>
    have hs := hb1 rfl
    simp only [if_true]
    split
    · exact (WP.spec_ok _).mpr (fun h => by cases h)
    · rename_i hne
      have hl : a.chans.val.length = b.chans.val.length := vec_len_eq hne
      unfold progress.eq_cfg_loop
      refine WP.spec_mono (scan_loop _ a.chans.val.length
        (fun j => (a.chans.val[j]?).map (fun v => v.val) = (b.chans.val[j]?).map (fun v => v.val))
        ?_ ?_) ?_
      · intro i hi
        unfold progress.eq_cfg_loop.body
        dsimp only
        have hc : ¬ i < alloc.vec.Vec.len a.chans := by scalar_tac
        rw [if_neg hc]
      · intro i hi _
        unfold progress.eq_cfg_loop.body
        dsimp only
        have hc : i < alloc.vec.Vec.len a.chans := by scalar_tac
        rw [if_pos hc]
        have hib : i.val < b.chans.val.length := by omega
        step as ⟨x, hx⟩
        step as ⟨y, hy⟩
        apply WP.spec_bind (eq_u32_spec x y)
        intro bb hbb
        cases bb with
        | false =>
          simp only [Bool.false_eq_true, if_false]
          exact (WP.spec_ok _).mpr (Or.inl rfl)
        | true =>
          simp only [if_true]
          apply cont_incr i a.chans.val.length hi (by scalar_tac)
          intro i2 hi2
          refine Or.inr ⟨i2, rfl, hi2, ?_⟩
          rw [List.getElem?_eq_getElem hi, List.getElem?_eq_getElem hib, ← hx, ← hy]
          simp only [Option.map_some]
          rw [hbb rfl]
      · intro r hr h
        have hch : (cfg a).chans = (cfg b).chans := by
          rw [cfg_chans, cfg_chans]
          apply List.ext_getElem?
          intro k
          rw [List.getElem?_map, List.getElem?_map]
          by_cases hk : k < a.chans.val.length
          · exact hr h k hk
          · rw [List.getElem?_eq_none (by omega), List.getElem?_eq_none (by omega)]
        have hst : (cfg a).states = (cfg b).states := hs
        cases ha : cfg a
        cases hb : cfg b
        rw [ha, hb] at hch hst
        simp only at hch hst
        rw [hch, hst]

/-- `contains`: `true` only when the list holds a configuration with the
view of `c`. -/
theorem contains_spec (list : alloc.vec.Vec progress.Config) (c : progress.Config) :
    progress.contains list c ⦃ r => r = true → cfg c ∈ list.val.map cfg ⦄ := by
  unfold progress.contains progress.contains_loop
  apply loop.spec_decr_nat (fun (i : Usize) => list.val.length - i.val)
    (fun i => i.val ≤ list.val.length)
  · intro i hi
    unfold progress.contains_loop.body
    dsimp only
    split
    · rename_i hlt
      have hil : i.val < list.val.length := by scalar_tac
      step as ⟨c1, hc1⟩
      apply WP.spec_bind (eq_cfg_spec c1 c)
      intro b hb
      cases b with
      | true =>
        simp only [if_true, WP.spec_ok]
        intro _
        rw [← hb rfl, hc1]
        exact List.mem_map.mpr ⟨_, List.getElem_mem _, rfl⟩
      | false =>
        simp only [Bool.false_eq_true, if_false]
        apply cont_incr i list.val.length hil (by scalar_tac)
        intro i2 hi2
        dsimp only
        leaves <;> omega
    · exact (WP.spec_ok _).mpr (fun h => by cases h)
  · scalar_tac

/-! ## The initial configuration -/

theorem initial_loop0_spec (n : Usize) :
    progress.initial_config_loop0 n (alloc.vec.Vec.new Usize) 0#usize
      ⦃ r => r.val = List.replicate n.val 0#usize ⦄ := by
  unfold progress.initial_config_loop0
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec Usize × Usize) => n.val - st.2.val)
    (fun st => st.2.val ≤ n.val ∧ st.1.val = List.replicate st.2.val 0#usize)
  · intro st hst
    obtain ⟨states, i⟩ := st
    obtain ⟨h1, h2⟩ := hst
    dsimp only at h1 h2 ⊢
    unfold progress.initial_config_loop0.body
    split
    · rename_i hlt
      have hin : i.val < n.val := by scalar_tac
      have hlen : states.val.length < Usize.max := by
        rw [h2, List.length_replicate]; scalar_tac
      step as ⟨states1, hs1⟩
      apply cont_incr i n.val (by scalar_tac) (by scalar_tac)
      intro i1 hi1
      dsimp only
      leaves <;> first | omega | skip
      rw [hs1, h2, hi1, List.replicate_succ']
    · rename_i hlt
      simp only [WP.spec_ok]
      have e : i.val = n.val := by scalar_tac
      rw [h2, e]
  · exact ⟨by scalar_tac, rfl⟩

theorem initial_loop1_spec (n : Usize) (hn : n.val ≤ 16) :
    progress.initial_config_loop1 n (alloc.vec.Vec.new (alloc.vec.Vec U32)) 0#usize
      ⦃ r => r.val.map (fun v => v.val) = List.replicate (n.val * n.val) [] ⦄ := by
  have hnn : n.val * n.val ≤ 256 := Nat.mul_le_mul hn hn
  unfold progress.initial_config_loop1
  apply loop.spec_decr_nat
    (fun (st : alloc.vec.Vec (alloc.vec.Vec U32) × Usize) => n.val * n.val - st.2.val)
    (fun st => st.2.val ≤ n.val * n.val ∧
      st.1.val.map (fun v => v.val) = List.replicate st.2.val [])
  · intro st hst
    obtain ⟨chans, j⟩ := st
    obtain ⟨h1, h2⟩ := hst
    dsimp only at h1 h2 ⊢
    unfold progress.initial_config_loop1.body
    have hmul : n.val * n.val ≤ Usize.max := by scalar_tac
    step as ⟨nn, hnn'⟩
    split
    · rename_i hlt
      have hjn : j.val < n.val * n.val := by scalar_tac
      have hlen : chans.val.length < Usize.max := by
        have hl := congrArg List.length h2
        simp only [List.length_map, List.length_replicate] at hl
        scalar_tac
      step as ⟨chans1, hc1⟩
      apply cont_incr j (n.val * n.val) (by scalar_tac) (by scalar_tac)
      intro j1 hj1
      dsimp only
      leaves <;> first | omega | skip
      rw [hc1, List.map_append, h2, hj1, List.replicate_succ']
      rfl
    · rename_i hlt
      simp only [WP.spec_ok]
      have e : j.val = n.val * n.val := by scalar_tac
      rw [h2, e]
  · exact ⟨by scalar_tac, rfl⟩

/-- `initial_config`: the view of the result is `initial`. -/
theorem initial_config_spec (n : Usize) (hn : n.val ≤ 16) :
    progress.initial_config n ⦃ r => cfg r = initial n.val ⦄ := by
  unfold progress.initial_config
  apply WP.spec_bind (initial_loop0_spec n)
  intro states hs
  apply WP.spec_bind (initial_loop1_spec n hn)
  intro chans hc
  simp only [WP.spec_ok]
  show (⟨states.val, chans.val.map (fun v => v.val)⟩ : Cfg) = initial n.val
  rw [hs, hc]
  rfl

/-! ## The checks of one configuration -/

/-- `wf_config`: `true` only for a well-formed configuration. -/
theorem wf_config_spec (s : progress.System) (c : progress.Config) (hs : s.roles.val ≤ 16) :
    progress.wf_config s c ⦃ r => r = true → WfCfg s.roles.val (cfg c) ⦄ := by
  have hnn : s.roles.val * s.roles.val ≤ 256 := Nat.mul_le_mul hs hs
  have hmul : s.roles.val * s.roles.val ≤ Usize.max := by scalar_tac
  unfold progress.wf_config
  dsimp only
  split
  · rename_i h1
    apply WP.spec_bind (UScalar.mul_spec (x := s.roles) (y := s.roles) (by scalar_tac))
    intro i2 hi2
    refine (WP.spec_ok _).mpr ?_
    intro h
    have h' : alloc.vec.Vec.len c.chans = i2 := by
      first | exact of_decide_eq_true h | exact h | (apply usize_val_inj; simpa using h)
    have h2 : c.chans.val.length = i2.val := by scalar_tac
    have h4 : c.states.val.length = s.roles.val := by scalar_tac
    refine ⟨h4, ?_⟩
    rw [cfg_chans, List.length_map, h2, hi2]
  · exact (WP.spec_ok _).mpr (fun h => by cases h)

/-- `capacity_ok`: `true` only when no channel is over the capacity. -/
theorem capacity_ok_spec (s : progress.System) (c : progress.Config) :
    progress.capacity_ok s c ⦃ r => r = true → WithinCapacity s (cfg c) ⦄ := by
  unfold progress.capacity_ok progress.capacity_ok_loop
  refine WP.spec_mono (scan_loop _ c.chans.val.length
    (fun j => ∀ (h : j < c.chans.val.length), (c.chans.val[j]).val.length ≤ s.capacity.val)
    ?_ ?_) ?_
  · intro i hi
    unfold progress.capacity_ok_loop.body
    dsimp only
    have hc : ¬ i < alloc.vec.Vec.len c.chans := by scalar_tac
    rw [if_neg hc]
  · intro i hi _
    unfold progress.capacity_ok_loop.body
    dsimp only
    have hc : i < alloc.vec.Vec.len c.chans := by scalar_tac
    rw [if_pos hc]
    step as ⟨v, hv⟩
    split
    · exact (WP.spec_ok _).mpr (Or.inl rfl)
    · rename_i hgt
      apply cont_incr i c.chans.val.length hi (by scalar_tac)
      intro i3 hi3
      refine Or.inr ⟨i3, rfl, hi3, ?_⟩
      intro _
      rw [← hv]
      scalar_tac
  · intro r hr h q hq
    rw [cfg_chans] at hq
    obtain ⟨v, hv, rfl⟩ := List.mem_map.mp hq
    obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hv
    exact hr h j hj hj

/-! ## The operations -/

theorem chan_bound {a b n : Nat} (h1 : a < n) (h2 : b < n) : a * n + b < n * n := by
  have h3 : (a + 1) * n ≤ n * n := Nat.mul_le_mul_right n h1
  rw [Nat.add_mul, Nat.one_mul] at h3
  omega

/-- `chan_index`: the channel of an operation, in range. -/
theorem chan_index_spec (s : progress.System) (t : progress.Trans) (hs : s.roles.val ≤ 16)
    (h1 : t.role.val < s.roles.val) (h2 : t.peer.val < s.roles.val) :
    progress.chan_index s t
      ⦃ r => r.val = chanOf s.roles.val t ∧ r.val < s.roles.val * s.roles.val ⦄ := by
  have hnn : s.roles.val * s.roles.val ≤ 256 := Nat.mul_le_mul hs hs
  have hb1 := chan_bound h1 h2
  have hb2 := chan_bound h2 h1
  unfold progress.chan_index
  by_cases hsend : t.send = true
  · rw [if_pos hsend]
    have hm : t.role.val * s.roles.val ≤ Usize.max := by scalar_tac
    apply WP.spec_bind (UScalar.mul_spec (x := t.role) (y := s.roles) (by scalar_tac))
    intro i hi
    refine WP.spec_mono (UScalar.add_spec (x := i) (y := t.peer) (by scalar_tac)) ?_
    intro r hr
    unfold chanOf
    rw [if_pos hsend]
    exact ⟨by scalar_tac, by scalar_tac⟩
  · rw [if_neg hsend]
    have hm : t.peer.val * s.roles.val ≤ Usize.max := by scalar_tac
    apply WP.spec_bind (UScalar.mul_spec (x := t.peer) (y := s.roles) (by scalar_tac))
    intro i hi
    refine WP.spec_mono (UScalar.add_spec (x := i) (y := t.role) (by scalar_tac)) ?_
    intro r hr
    unfold chanOf
    rw [if_neg hsend]
    exact ⟨by scalar_tac, by scalar_tac⟩

/-- `is_enabled`: exactly `enabled` of the operation. -/
theorem is_enabled_spec (s : progress.System) (c : progress.Config) (i : Usize)
    (hs : WfSys s) (hi : i.val < s.trans.val.length) (hc : WfCfg s.roles.val (cfg c)) :
    progress.is_enabled s c i
      ⦃ r => r = true ↔ Enabled s.roles.val (cfg c) (s.trans.val[i.val]'hi) ⦄ := by
  unfold progress.is_enabled
  step as ⟨t, ht⟩
  rw [← ht]
  have htm : t ∈ s.trans.val := by rw [ht]; exact List.getElem_mem _
  obtain ⟨hr, hp, _⟩ := hs.2 t htm
  have hsl : c.states.val.length = s.roles.val := hc.1
  have hcl : c.chans.val.length = s.roles.val * s.roles.val := by
    have h2 := hc.2
    rw [cfg_chans, List.length_map] at h2
    exact h2
  have hge : ¬ t.role ≥ alloc.vec.Vec.len c.states := by scalar_tac
  rw [if_neg hge]
  step as ⟨st, hst⟩
  have hat : At (cfg c) t ↔ st = t.«from» := by
    unfold At
    rw [cfg_states, getElem!_pos c.states.val t.role.val (by omega), ← hst]
    exact ⟨fun h => h.2, fun h => ⟨by omega, h⟩⟩
  split
  · rename_i hne
    have hne' : st ≠ t.«from» := of_bne hne
    exact (WP.spec_ok _).mpr (false_iff' (fun hen => hne' (hat.mp hen.1)))
  · rename_i heq
    have heq' : st = t.«from» := of_not_bne heq
    apply WP.spec_bind (chan_index_spec s t hs.1 hr hp)
    intro ch hch
    obtain ⟨hch1, hch2⟩ := hch
    have hchl : chanOf s.roles.val t < (cfg c).chans.length := by
      rw [cfg_chans, List.length_map, ← hch1]; omega
    by_cases hsend : t.send = true
    · rw [if_pos hsend]
      exact (WP.spec_ok _).mpr (true_iff' ⟨hat.mpr heq', hchl, Or.inl hsend⟩)
    · rw [if_neg hsend]
      step as ⟨q, hq⟩
      have eq' : (cfg c).chans[chanOf s.roles.val t]! = q.val := by
        have hchv : ch.val < c.chans.val.length := by omega
        rw [cfg_chans, ← hch1, getElem!_map' (fun v => v.val) c.chans.val ch.val hchv, hq,
          getElem!_pos c.chans.val ch.val hchv]
      split
      · rename_i hpos
        have hq0 : 0 < q.val.length := by scalar_tac
        first | rw [alloc.vec.Vec.index_slice_index] | skip
        apply WP.spec_bind (alloc.vec.Vec.index_usize_spec q 0#usize (by scalar_tac))
        intro x hx
        have hhead : q.val.head? = some x := by
          rw [hx, List.head?_eq_getElem?]
          exact List.getElem?_eq_getElem _
        refine (WP.spec_ok _).mpr ?_
        simp only [decide_eq_true_eq]
        constructor
        · intro h
          exact ⟨hat.mpr heq', hchl, Or.inr (by rw [eq', hhead, h])⟩
        · intro hen
          rcases hen.2.2 with h | h
          · exact absurd h hsend
          · rw [eq', hhead] at h
            exact Option.some.inj h
      · rename_i hpos
        have hq0 : q.val = [] := by
          have h0 : q.val.length = 0 := by scalar_tac
          exact List.length_eq_zero_iff.mp h0
        refine (WP.spec_ok _).mpr (false_iff' (fun hen => ?_))
        rcases hen.2.2 with h | h
        · exact hsend h
        · rw [eq', hq0] at h
          cases h

end PC
