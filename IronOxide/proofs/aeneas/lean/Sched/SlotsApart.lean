import Sched.Boot
open Aeneas Aeneas.Std Result

/-! # The slot tables `Config::validate` accepts keep the Tasks' slots apart (2026-11-26)

The Kernel gives every Task a deadline slot and a Budget slot of the timer
table, and a release slot when the Task is periodic or a Sporadic_Task that
defers (`slot_deadline`, `slot_budget`, `slot_release`; R9.4, R23.4). That no
two of these are one slot is `Config::slot_ok` and `Config::slots_distinct`
in Verus, clauses of `Config::wf`, checked at boot by
`Config::validate_slots`. `Sched/Boot.lean` derives from that check the
slots' range only (`CfgW`). Here, from the same extracted loops:

- `SlotsApart`: one Task's slots are distinct, a release slot a Task needs
  is not `NO_SLOT`, a Task that discards has none, and the slots of two
  Tasks are distinct (`slot_ok`'s and `slots_distinct`'s clauses on
  distinctness, transcribed; that they are the Verus ones is by reading).
- `validate_slots_apart`, `validate_apart`: `Config::validate_slots`, and so
  `Config::validate`, returning `true` gives `SlotsApart`.
- `TKinds`, `validate_kinds`: `Config::validate` returning `true` gives
  every Task a kind and an MIT policy of the two declared ones
  (`task_ok`'s clauses), so (`norel_no_slot`) a Task that needs no release
  slot has `NO_SLOT` there.

`Sched/ContainSlots.lean` uses these for the frame of a Health_Monitor
response over the other Partitions' timer slots. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The deadline slot of Task `u`, as an index of the slot table. -/
def dslot (cfg : logic.Config NT NR NP) (u : Nat) : Nat := (cfg.slot_deadline.val[u]!).val
/-- The Budget slot of Task `u`. -/
def bslot (cfg : logic.Config NT NR NP) (u : Nat) : Nat := (cfg.slot_budget.val[u]!).val
/-- The release slot of Task `u` (255, `NO_SLOT`, when it has none). -/
def rslot (cfg : logic.Config NT NR NP) (u : Nat) : Nat := (cfg.slot_release.val[u]!).val

/-- Task `u` has a release slot: it is periodic, or defers (the condition
of `slot_ok` and of `validate_slots`). -/
def needsRel (cfg : logic.Config NT NR NP) (u : Nat) : Prop :=
  ∃ tc, cfg.tasks.val[u]? = some tc ∧
    (tc.kind = logic.KIND_PERIODIC ∨ tc.mit_policy = logic.MIT_DEFER)

/-- `Config::slots_distinct(t, u)` on the slots' indices: the deadline,
Budget and release slots `d b r` of one Task against those of another. -/
def Dist (d b r du bu ru : Nat) : Prop :=
  d ≠ du ∧ b ≠ bu ∧ d ≠ bu ∧ (r ≠ 255 → r ≠ du ∧ r ≠ bu ∧ r ≠ ru)

/-- The distinctness clauses of `Config::wf` on the slot tables. -/
structure SlotsApart (cfg : logic.Config NT NR NP) : Prop where
  /-- `slot_ok`: one Task's slots. -/
  own : ∀ u, u < NT.val → dslot cfg u ≠ bslot cfg u ∧
    (needsRel cfg u →
      rslot cfg u ≠ 255 ∧ rslot cfg u ≠ dslot cfg u ∧ rslot cfg u ≠ bslot cfg u)
  /-- `slots_distinct`: two Tasks' slots. -/
  apart : ∀ t u, t < NT.val → u < NT.val → t ≠ u →
    Dist (dslot cfg t) (bslot cfg t) (rslot cfg t) (dslot cfg u) (bslot cfg u) (rslot cfg u)

theorem no_slot_val : logic.NO_SLOT.val = 255 := by
  unfold logic.NO_SLOT; rfl

theorem ne_val_of_bne {x y : U8} (h : (x != y) = true) : x.val ≠ y.val :=
  fun e => (bne_iff_ne.mp h) (UScalar.eq_of_val_eq e)

theorem dist_of {d b r du bu ru : U8} (h1 : (d != du) = true) (h2 : (b != bu) = true)
    (h3 : (d != bu) = true)
    (h4 : r = logic.NO_SLOT ∨ ((r != du) = true ∧ (r != bu) = true ∧ (r != ru) = true)) :
    Dist d.val b.val r.val du.val bu.val ru.val := by
  refine ⟨ne_val_of_bne h1, ne_val_of_bne h2, ne_val_of_bne h3, fun hr => ?_⟩
  rcases h4 with h4 | ⟨g1, g2, g3⟩
  · rw [h4, no_slot_val] at hr
    exact absurd rfl hr
  · exact ⟨ne_val_of_bne g1, ne_val_of_bne g2, ne_val_of_bne g3⟩

/-! ## The inner loop: one Task's slots against every other Task's -/

set_option maxHeartbeats 2000000 in
theorem vs_inner_cont {a a1 a2 : Array U8 NT} {t : Usize} {d b r : U8} {f f' : Bool}
    {u u' : Usize} {x3 x4 x5 y3 y4 y5 : Array U8 NT}
    (hb : logic.Config.validate_slots_loop0_loop0.body a a1 a2 t d b r f u x3 x4 x5
      = ok (ControlFlow.cont (f', u', y3, y4, y5))) :
    f' = true → f = true ∧ u'.val = u.val + 1 ∧
      (u.val ≠ t.val →
        Dist d.val b.val r.val (a1.val[u.val]!).val (a2.val[u.val]!).val (a.val[u.val]!).val) := by
  unfold logic.Config.validate_slots_loop0_loop0.body at hb
  rcases ite_cases hb with ⟨hlt, hb1⟩ | ⟨hlt, hb1⟩ <;> clear hb
  · rcases ite_cases hb1 with ⟨hf, hb⟩ | ⟨hf, hb⟩ <;> clear hb1
    · simp only [bind_eq_ok] at hb
      obtain ⟨f1, hf1, u1, hu1, hb⟩ := hb
      intro hf'
      have hb := ok_inj hb
      cases hb
      have hu1' := usize_add_ok hu1
      refine ⟨hf, by simpa using hu1', fun hne => ?_⟩
      rcases ite_cases hf1 with ⟨hut, h1⟩ | ⟨hut, h1⟩ <;> clear hf1
      · simp only [bind_eq_ok] at h1
        obtain ⟨du, hdu, bu, hbu, ru, hru, dist, hdist, h1⟩ := h1
        obtain ⟨_, rfl⟩ := index_ok hdu
        obtain ⟨_, rfl⟩ := index_ok hbu
        obtain ⟨_, rfl⟩ := index_ok hru
        have hdt : dist = true := by
          rcases ite_cases h1 with ⟨hd, _⟩ | ⟨_, e⟩
          · exact hd
          · have e := ok_inj e
            rw [← e] at hf'
            cases hf'
        subst hdt
        clear h1
        ite_split hdist
        all_goals first
          | (simp at hdist; done)
          | exact dist_of (by assumption) (by assumption) (by assumption) (Or.inl (by assumption))
          | exact dist_of (by assumption) (by assumption) (by assumption)
              (Or.inr ⟨by assumption, by assumption, by simpa using hdist⟩)
      · exact absurd (bne_iff_ne.mpr (fun e => hne (by rw [e]))) hut
    · have hb := ok_inj hb; cases hb
  · have hb := ok_inj hb1; cases hb

theorem vs_inner_done {a a1 a2 : Array U8 NT} {t : Usize} {d b r : U8} {f y : Bool}
    {u : Usize} {x3 x4 x5 : Array U8 NT}
    (hb : logic.Config.validate_slots_loop0_loop0.body a a1 a2 t d b r f u x3 x4 x5
      = ok (ControlFlow.done y)) :
    y = f ∧ (y = true → NT.val ≤ u.val) := by
  unfold logic.Config.validate_slots_loop0_loop0.body at hb
  rcases ite_cases hb with ⟨hlt, hb1⟩ | ⟨hlt, hb1⟩ <;> clear hb
  · rcases ite_cases hb1 with ⟨hg, hb⟩ | ⟨hg, hb⟩ <;> clear hb1
    · simp only [bind_eq_ok] at hb
      obtain ⟨f1, hf1, u1, hu1, hb⟩ := hb
      have hb := ok_inj hb
      cases hb
    · have hb := ok_inj hb
      cases hb
      exact ⟨rfl, fun e => absurd e hg⟩
  · have hb := ok_inj hb1
    cases hb
    exact ⟨rfl, fun _ => by scalar_tac⟩

/-- The inner loop of `validate_slots` answering `true` found the slots
`d b r` distinct from every other Task's. -/
theorem vs_inner_ok {a a1 a2 : Array U8 NT} {t : Usize} {d b r : U8}
    {x3 x4 x5 : Array U8 NT} {res : Bool}
    (h : logic.Config.validate_slots_loop0_loop0 a a1 a2 t d b r true 0#usize x3 x4 x5
      = ok res) :
    res = true → ∀ j, j < NT.val → j ≠ t.val →
      Dist d.val b.val r.val (a1.val[j]!).val (a2.val[j]!).val (a.val[j]!).val := by
  unfold logic.Config.validate_slots_loop0_loop0 at h
  refine loop_inv _
    (fun x => x.1 = true → ∀ j, j < x.2.1.val → j ≠ t.val →
      Dist d.val b.val r.val (a1.val[j]!).val (a2.val[j]!).val (a.val[j]!).val)
    (fun y => y = true → ∀ j, j < NT.val → j ≠ t.val →
      Dist d.val b.val r.val (a1.val[j]!).val (a2.val[j]!).val (a.val[j]!).val)
    ?_ ?_ (true, 0#usize, x3, x4, x5) res
    (fun _ j hj => absurd (show j < 0 from hj) (Nat.not_lt_zero j)) h
  · rintro ⟨f, u, y3, y4, y5⟩ ⟨f', u', z3, z4, z5⟩ hi hb
    dsimp only at hi hb ⊢
    intro hf' j hj hjt
    obtain ⟨hf, hu, hd⟩ := vs_inner_cont hb hf'
    by_cases hj' : j < u.val
    · exact hi hf j hj' hjt
    · have hje : j = u.val := by omega
      rw [hje] at hjt ⊢
      exact hd hjt
  · rintro ⟨f, u, y3, y4, y5⟩ y hi hb
    dsimp only at hi hb ⊢
    intro hy j hj hjt
    obtain ⟨rfl, hge⟩ := vs_inner_done hb
    exact hi hy j (by have := hge hy; omega) hjt

/-! ## The outer loop: every Task's row -/

/-- What `validate_slots` checks of one Task's own slots, beyond their
range (`SW` of `Sched/Boot.lean`). -/
def Row (limit : Usize) (tc : logic.TaskCfg) (r d b : U8) : Prop :=
  d.val ≠ b.val ∧
  ((tc.kind = logic.KIND_PERIODIC ∨ tc.mit_policy = logic.MIT_DEFER) →
    r.val < limit.val ∧ r.val ≠ d.val ∧ r.val ≠ b.val) ∧
  ((tc.kind = logic.KIND_SPORADIC ∧ tc.mit_policy = logic.MIT_DISCARD) → r.val = 255)

set_option maxHeartbeats 8000000 in
theorem vs_outer_cont {a : Array logic.TaskCfg NT} {a1 a2 a3 : Array U8 NT}
    {limit : Usize} {g g' : Bool} {t1 t2 : Usize}
    (hb : logic.Config.validate_slots_loop0.body a a1 a2 a3 limit g t1
      = ok (ControlFlow.cont (g', t2))) :
    g' = true → g = true ∧ t2.val = t1.val + 1 ∧
      ∃ task, a.val[t1.val]? = some task ∧
        Row limit task (a1.val[t1.val]!) (a2.val[t1.val]!) (a3.val[t1.val]!) ∧
        ∀ j, j < NT.val → j ≠ t1.val →
          Dist (a2.val[t1.val]!).val (a3.val[t1.val]!).val (a1.val[t1.val]!).val
            (a2.val[j]!).val (a3.val[j]!).val (a1.val[j]!).val := by
  unfold logic.Config.validate_slots_loop0.body at hb
  rcases ite_cases hb with ⟨hlt, hb1⟩ | ⟨hlt, hb1⟩ <;> clear hb
  · rcases ite_cases hb1 with ⟨hg, hb⟩ | ⟨hg, hb⟩ <;> clear hb1
    · simp only [bind_eq_ok, lift, bind_ok] at hb
      obtain ⟨task, htask, d, hd, b, hbb, r, hr, nr, hnr, dc, hdc, row, hrow, hb⟩ := hb
      intro hg'
      rcases ite_cases hb with ⟨hrw, hb2⟩ | ⟨hrw, hb2⟩ <;> clear hb
      · simp only [bind_eq_ok] at hb2
        obtain ⟨fine, hfine, g1, hg1, t3, ht3, hb2⟩ := hb2
        have hb2 := ok_inj hb2
        cases hb2
        obtain ⟨_, rfl⟩ := index_ok hd
        obtain ⟨_, rfl⟩ := index_ok hbb
        obtain ⟨_, rfl⟩ := index_ok hr
        subst hrw
        have hft : fine = true := by
          rcases ite_cases hg1 with ⟨hf, _⟩ | ⟨_, e⟩
          · exact hf
          · have e := ok_inj e
            rw [← e] at hg'
            cases hg'
        subst hft
        have hrowp : Row limit task (a1.val[t1.val]!) (a2.val[t1.val]!) (a3.val[t1.val]!) := by
          clear hg1 hfine
          ite_split hnr
          all_goals
            have hnr' := ok_inj hnr
            subst hnr'
            ite_split hdc
          all_goals
            have hdc' := ok_inj hdc
            subst hdc'
            try simp only [↓reduceIte] at hrow
            ite_split hrow
          all_goals first
            | (simp at hrow; done)
            | (refine ⟨ne_val_of_bne (by assumption), fun hk => ?_, fun hk => ?_⟩
               · first
                   | exact ⟨by scalar_tac, ne_val_of_bne (by assumption),
                       ne_val_of_bne (by assumption)⟩
                   | (rcases hk with hk | hk <;> contradiction)
               · first
                   | (have hns : a1.val[t1.val]! = logic.NO_SLOT := by simpa using hrow
                      rw [hns, no_slot_val])
                   | (obtain ⟨hk1, hk2⟩ := hk; contradiction))
        have ht3' := usize_add_ok ht3
        exact ⟨hg, by simpa using ht3', task, (index_ok' htask).2, hrowp,
          vs_inner_ok hfine rfl⟩
      · simp only [bind_eq_ok] at hb2
        obtain ⟨t3, ht3, hb2⟩ := hb2
        have hb2 := ok_inj hb2
        cases hb2
        cases hg'
    · have hb := ok_inj hb; cases hb
  · have hb := ok_inj hb1; cases hb

/-- What `validate_slots` found of Task `u`. -/
def RowAll (a : Array logic.TaskCfg NT) (a1 a2 a3 : Array U8 NT) (limit : Usize) (u : Nat) :
    Prop :=
  (∃ tc, a.val[u]? = some tc ∧ Row limit tc (a1.val[u]!) (a2.val[u]!) (a3.val[u]!)) ∧
  ∀ j, j < NT.val → j ≠ u →
    Dist (a2.val[u]!).val (a3.val[u]!).val (a1.val[u]!).val
      (a2.val[j]!).val (a3.val[j]!).val (a1.val[j]!).val

theorem vs_outer_ok {a : Array logic.TaskCfg NT} {a1 a2 a3 : Array U8 NT}
    {limit : Usize} {r : Bool}
    (h : logic.Config.validate_slots_loop0 a a1 a2 a3 limit true 0#usize = ok r) :
    r = true → ∀ u, u < NT.val → RowAll a a1 a2 a3 limit u := by
  unfold logic.Config.validate_slots_loop0 at h
  refine loop_inv _
    (fun x => x.1 = true → ∀ u, u < x.2.val → RowAll a a1 a2 a3 limit u)
    (fun r => r = true → ∀ u, u < NT.val → RowAll a a1 a2 a3 limit u) ?_ ?_ (true, 0#usize) r
    (fun _ u hu => absurd (show u < 0 from hu) (Nat.not_lt_zero u)) h
  · rintro ⟨g, t1⟩ ⟨g', t2⟩ hi hb
    dsimp only at hi hb ⊢
    intro hg' u hu
    obtain ⟨hg, ht2, task, e, hrow, hdist⟩ := vs_outer_cont hb hg'
    by_cases hu' : u < t1.val
    · exact hi hg u hu'
    · have hue : u = t1.val := by omega
      rw [hue]
      exact ⟨⟨task, e, hrow⟩, hdist⟩
  · rintro ⟨g, t1⟩ y hi hb
    dsimp only at hi hb ⊢
    intro hy u hu
    obtain ⟨rfl, hge⟩ := validate_slots_loop0_done hb
    exact hi hy u (by have := hge hy; omega)

/-- What `validate_slots` answering `true` found of every Task. -/
theorem validate_slots_rows {cfg : logic.Config NT NR NP}
    (h : logic.Config.validate_slots cfg = ok true) :
    ∃ limit : Usize, limit.val ≤ 253 ∧ ∀ u, u < NT.val →
      RowAll cfg.tasks cfg.slot_release cfg.slot_deadline cfg.slot_budget limit u := by
  unfold logic.Config.validate_slots at h
  split at h
  · have := ok_inj h; cases this
  · rename_i hsc
    simp only [bind_eq_ok, lift, bind_ok] at h
    obtain ⟨i, hi, h⟩ := h
    have hiv : i.val = cfg.slot_count.val - 2 := by
      have := UScalar.sub_equiv cfg.slot_count 2#u8
      have h2 : (2#u8).val = 2 := rfl
      simp_all
    have hc : (UScalar.cast UScalarTy.Usize i).val = i.val := by scalar_tac
    have hsc : cfg.slot_count.val ≤ 255 := by scalar_tac
    exact ⟨_, by rw [hc, hiv]; omega, vs_outer_ok h rfl⟩

/-- **`Config::validate_slots` returning `true` gives the slots apart**
(`slot_ok`'s and `slots_distinct`'s distinctness, for every Task and every
pair of Tasks). -/
theorem validate_slots_apart {cfg : logic.Config NT NR NP}
    (h : logic.Config.validate_slots cfg = ok true) : SlotsApart cfg := by
  obtain ⟨limit, hlim, hl⟩ := validate_slots_rows h
  refine ⟨fun u hu => ?_, fun t u ht hu hne => (hl t ht).2 u hu (Ne.symm hne)⟩
  obtain ⟨⟨tc, htc, hrow⟩, _⟩ := hl u hu
  refine ⟨hrow.1, ?_⟩
  rintro ⟨tc', htc', hk⟩
  rw [htc] at htc'
  cases htc'
  obtain ⟨h1, h2, h3⟩ := hrow.2.1 hk
  exact ⟨by unfold rslot; omega, h2, h3⟩

set_option maxHeartbeats 2000000 in
/-- **`Config::validate` returning `true` gives the slots apart.** -/
theorem validate_apart {cfg : logic.Config NT NR NP}
    (h : logic.Config.validate cfg = ok true) : SlotsApart cfg := by
  unfold logic.Config.validate at h
  peel h
  exact validate_slots_apart (by simp_all : logic.Config.validate_slots cfg = ok true)

/-! ## The Tasks' kinds and policies -/

/-- `task_ok`'s clauses on the kind and the MIT policy. -/
def TKinds (tc : logic.TaskCfg) : Prop :=
  (tc.kind = logic.KIND_PERIODIC ∨ tc.kind = logic.KIND_SPORADIC) ∧
  (tc.mit_policy = logic.MIT_DEFER ∨ tc.mit_policy = logic.MIT_DISCARD)

set_option maxHeartbeats 4000000 in
theorem validate_loop0_cont_k {a : Array logic.TaskCfg NT} {od : U64} {g g' : Bool}
    {t1 t2 : Usize}
    (hb : logic.Config.validate_loop0.body NP a od g t1 = ok (ControlFlow.cont (g', t2))) :
    g' = true → g = true ∧ t2.val = t1.val + 1 ∧
      ∃ task, a.val[t1.val]? = some task ∧ TKinds task := by
  unfold logic.Config.validate_loop0.body at hb
  rcases ite_cases hb with ⟨hlt, hb1⟩ | ⟨hlt, hb1⟩ <;> clear hb
  · rcases ite_cases hb1 with ⟨hg, hb⟩ | ⟨hg, hb⟩ <;> clear hb1
    · simp only [bind_eq_ok, lift, bind_ok] at hb
      obtain ⟨task, htask, tg, htg, g1, hg1, t3, ht3, hb⟩ := hb
      intro hg'
      have hb := ok_inj hb
      cases hb
      have htt : tg = true := by
        rcases ite_cases hg1 with ⟨hf, _⟩ | ⟨_, e⟩
        · exact hf
        · have e := ok_inj e
          rw [← e] at hg'
          cases hg'
      subst htt
      have htk : TKinds task := by
        clear hg1
        ite_split htg
        all_goals first
          | (simp at htg; done)
          | exact ⟨Or.inl (by assumption), Or.inl (by assumption)⟩
          | exact ⟨Or.inl (by assumption), Or.inr (by assumption)⟩
          | exact ⟨Or.inr (by assumption), Or.inl (by assumption)⟩
          | exact ⟨Or.inr (by assumption), Or.inr (by assumption)⟩
      have ht3' := usize_add_ok ht3
      exact ⟨hg, by simpa using ht3', task, (index_ok' htask).2, htk⟩
    · have hb := ok_inj hb; cases hb
  · have hb := ok_inj hb1; cases hb

theorem validate_loop0_ok_k {a : Array logic.TaskCfg NT} {od : U64} {r : Bool}
    (h : logic.Config.validate_loop0 NP a od true 0#usize = ok r) :
    r = true → ∀ (u : Nat) (tc : logic.TaskCfg), a.val[u]? = some tc → TKinds tc := by
  have hlen : a.val.length = NT.val := a.property
  unfold logic.Config.validate_loop0 at h
  refine loop_inv _
    (fun x => x.1 = true → ∀ (u : Nat), u < x.2.val → ∀ (tc : logic.TaskCfg),
      a.val[u]? = some tc → TKinds tc)
    (fun r => r = true → ∀ (u : Nat) (tc : logic.TaskCfg), a.val[u]? = some tc → TKinds tc)
    ?_ ?_ (true, 0#usize) r
    (fun _ u hu => absurd (show u < 0 from hu) (Nat.not_lt_zero u)) h
  · rintro ⟨g, t1⟩ ⟨g', t2⟩ hi hb
    dsimp only at hi hb ⊢
    intro hg' u hu tc htc
    obtain ⟨hg, ht2, task, e, htk⟩ := validate_loop0_cont_k hb hg'
    by_cases hu' : u < t1.val
    · exact hi hg u hu' tc htc
    · have hue : u = t1.val := by omega
      rw [hue, e] at htc
      cases htc
      exact htk
  · rintro ⟨g, t1⟩ y hi hb
    dsimp only at hi hb ⊢
    intro hy u tc htc
    obtain ⟨rfl, hge⟩ := validate_loop0_done hb
    have hu : u < a.val.length := (List.getElem?_eq_some_iff.mp htc).1
    exact hi hy u (by have := hge hy; omega) tc htc

set_option maxHeartbeats 2000000 in
/-- **`Config::validate` returning `true` gives every Task one of the two
kinds and one of the two MIT policies** (`task_ok`). -/
theorem validate_kinds {cfg : logic.Config NT NR NP}
    (h : logic.Config.validate cfg = ok true) :
    ∀ (u : Nat) (tc : logic.TaskCfg), cfg.tasks.val[u]? = some tc → TKinds tc := by
  unfold logic.Config.validate at h
  peel h
  exact validate_loop0_ok_k
    (by simp_all : logic.Config.validate_loop0 NP cfg.tasks cfg.operating_duration true 0#usize
      = ok true) rfl

set_option maxHeartbeats 2000000 in
/-- **A Task that needs no release slot has none** (`slot_ok`'s last
clause with `task_ok`): after `Config::validate`, the release slot of a
Task that is neither periodic nor deferring is `NO_SLOT`. So a release
slot other than `NO_SLOT` is one its Task needs. -/
theorem norel_no_slot {cfg : logic.Config NT NR NP}
    (h : logic.Config.validate cfg = ok true) (u : Nat) (hu : u < NT.val)
    (hr : rslot cfg u ≠ 255) : needsRel cfg u := by
  have hk := validate_kinds h
  have hs : logic.Config.validate_slots cfg = ok true := by
    unfold logic.Config.validate at h
    peel h
    simp_all
  obtain ⟨limit, _, hl⟩ := validate_slots_rows hs
  obtain ⟨⟨tc, htc, hrow⟩, _⟩ := hl u hu
  obtain ⟨hkind, hpol⟩ := hk u tc htc
  by_contra hn
  have h1 : ¬ tc.kind = logic.KIND_PERIODIC := fun e => hn ⟨tc, htc, Or.inl e⟩
  have h2 : ¬ tc.mit_policy = logic.MIT_DEFER := fun e => hn ⟨tc, htc, Or.inr e⟩
  have h3 : tc.kind = logic.KIND_SPORADIC := by
    rcases hkind with e | e
    · exact absurd e h1
    · exact e
  have h4 : tc.mit_policy = logic.MIT_DISCARD := by
    rcases hpol with e | e
    · exact absurd e h2
    · exact e
  exact hr (hrow.2.2 ⟨h3, h4⟩)

end P18
