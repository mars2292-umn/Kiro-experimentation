import Sched.TotalAll
import Sched.ArchBound
open Aeneas Aeneas.Std Result

/-! # What the boot validation gives (2026-11-20)

The hypotheses the Lean proofs took on the configuration (`CI`, `CT`, the
configuration part of `okB`) and on the initial state (`KT`) are derived
here from the code that establishes them at boot:

- `validate_ok`: `Config::validate` returning `true` gives `CfgW` (what the
  proofs use of `Config::wf`), by its extracted loops;
- `bound_validate_ok`: `Bound::validate` adds that the slot table has the
  configuration's size and the log's capacity is in range;
- `new_ok`: the state `Kernel::init` builds satisfies `KT` with no Job on
  the stack;
- `bound_kboot`: so `validate` and `init` satisfy `KBoot`, the clause
  `boot_core_establishes` rests on.

`Layout::validate` is not translated (its loops return from inside); the
layout's facts stay hypotheses where they are used (`LayoutOk`,
`view_total`). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

theorem max_priority : logic.MAX_PRIORITY = 7#u8 := by
  unfold logic.MAX_PRIORITY; rfl

theorem repeat_get {α} [Inhabited α] (n : Usize) (x : α) (u : Nat) (hu : u < n.val) :
    (Array.repeat n x).val[u]! = x := by
  rw [Array.repeat_val, getElem!_pos _ u (by simp [hu])]
  simp

/-- What `Config::validate` checks of one Task, as far as the proofs use it. -/
def TW (NP : Usize) (od : U64) (tc : logic.TaskCfg) : Prop :=
  1 ≤ tc.priority.val ∧ tc.priority.val ≤ 7 ∧ tc.partition.val < NP.val ∧
  tc.period.val ≤ od.val ∧ tc.deadline.val ≤ tc.period.val ∧ tc.offset.val ≤ od.val

/-- Splits the conditionals of `h : (if .. then .. else ..) = ok r`, one
after the other, where `split` gives up on the size of the term. -/
macro "ite_split " h:ident : tactic =>
  `(tactic| repeat' (first
      | (rcases ite_cases $h:ident with ⟨_, hnew⟩ | ⟨_, hnew⟩ <;> clear $h:ident <;>
          have $h:ident := hnew <;> clear hnew)
      | (simp only [bind_eq_ok, lift, bind_ok] at $h:ident)))

set_option maxHeartbeats 4000000 in
/-- One iteration of `validate`'s loop over the Tasks that leaves the
verdict `true` found the Task well formed. -/
theorem validate_loop0_cont {a : Array logic.TaskCfg NT} {od : U64} {g g' : Bool} {t1 t2 : Usize}
    (hb : logic.Config.validate_loop0.body NP a od g t1 = ok (ControlFlow.cont (g', t2))) :
    g' = true → g = true ∧ t2.val = t1.val + 1 ∧
      ∃ task, a.val[t1.val]? = some task ∧ TW NP od task := by
  unfold logic.Config.validate_loop0.body at hb
  rcases ite_cases hb with ⟨hlt, hb1⟩ | ⟨hlt, hb1⟩ <;> clear hb
  · rcases ite_cases hb1 with ⟨hg, hb⟩ | ⟨hg, hb⟩ <;> clear hb1
    · simp only [bind_eq_ok, lift, bind_ok] at hb
      obtain ⟨task, htask, tg, htg, g1, hg1, t3, ht3, hb⟩ := hb
      have hb := ok_inj hb
      cases hb
      intro hg'
      have htt : tg = true := by
        cases tg
        · simp at hg1; rw [hg1] at hg'; cases hg'
        · rfl
      subst htt
      have htw : TW NP od task := by
        clear hg1
        ite_split htg
        all_goals first
          | (simp at htg; done)
          | (simp at htg
             simp only [max_priority] at *
             refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> scalar_tac)
      have ht3' := usize_add_ok ht3
      exact ⟨hg, by simpa using ht3', task, (index_ok' htask).2, htw⟩
    · have hb := ok_inj hb; cases hb
  · have hb := ok_inj hb1; cases hb

theorem validate_loop0_done {a : Array logic.TaskCfg NT} {od : U64} {g y : Bool} {t1 : Usize}
    (hb : logic.Config.validate_loop0.body NP a od g t1 = ok (ControlFlow.done y)) :
    y = g ∧ (y = true → NT.val ≤ t1.val) := by
  unfold logic.Config.validate_loop0.body at hb
  rcases ite_cases hb with ⟨hlt, hb1⟩ | ⟨hlt, hb1⟩ <;> clear hb
  · rcases ite_cases hb1 with ⟨hg, hb⟩ | ⟨hg, hb⟩ <;> clear hb1
    · simp only [bind_eq_ok, lift, bind_ok] at hb
      obtain ⟨task, htask, tg, htg, g1, hg1, t3, ht3, hb⟩ := hb
      have hb := ok_inj hb
      cases hb
    · have hb := ok_inj hb
      cases hb
      exact ⟨rfl, fun e => absurd e hg⟩
  · have hb := ok_inj hb1
    cases hb
    exact ⟨rfl, fun _ => by scalar_tac⟩

theorem validate_loop0_ok {a : Array logic.TaskCfg NT} {od : U64} {good r : Bool} {t : Usize}
    (hinv : good = true → ∀ (u : Nat), u < t.val → ∀ (tc : logic.TaskCfg),
      a.val[u]? = some tc → TW NP od tc)
    (h : logic.Config.validate_loop0 NP a od good t = ok r) :
    r = true → ∀ (u : Nat) (tc : logic.TaskCfg), a.val[u]? = some tc → TW NP od tc := by
  have hlen : a.val.length = NT.val := a.property
  unfold logic.Config.validate_loop0 at h
  refine loop_inv _
    (fun x => x.1 = true → ∀ (u : Nat), u < x.2.val → ∀ (tc : logic.TaskCfg),
      a.val[u]? = some tc → TW NP od tc)
    (fun r => r = true → ∀ (u : Nat) (tc : logic.TaskCfg), a.val[u]? = some tc → TW NP od tc)
    ?_ ?_ (good, t) r hinv h
  · rintro ⟨g, t1⟩ ⟨g', t2⟩ hi hb
    dsimp only at hi hb ⊢
    intro hg' u hu tc htc
    obtain ⟨hg, ht2, task, e, htw⟩ := validate_loop0_cont hb hg'
    by_cases hu' : u < t1.val
    · exact hi hg u hu' tc htc
    · have hue : u = t1.val := by omega
      rw [hue, e] at htc
      cases htc
      exact htw
  · rintro ⟨g, t1⟩ y hi hb
    dsimp only at hi hb ⊢
    intro hy u tc htc
    obtain ⟨rfl, hge⟩ := validate_loop0_done hb
    have hu : u < a.val.length := (List.getElem?_eq_some_iff.mp htc).1
    exact hi hy u (by have := hge hy; omega) tc htc

/-- What `Config::validate_slots` checks of one Task's slots. -/
def SW (limit : Usize) (tc : logic.TaskCfg) (r d b : U8) : Prop :=
  d.val < limit.val ∧ b.val < limit.val ∧
  ((tc.kind = logic.KIND_PERIODIC ∨ tc.mit_policy = logic.MIT_DEFER) → r.val < limit.val)

set_option maxHeartbeats 4000000 in
theorem validate_slots_loop0_cont {a : Array logic.TaskCfg NT} {a1 a2 a3 : Array U8 NT}
    {limit : Usize} {g g' : Bool} {t1 t2 : Usize}
    (hb : logic.Config.validate_slots_loop0.body a a1 a2 a3 limit g t1
      = ok (ControlFlow.cont (g', t2))) :
    g' = true → g = true ∧ t2.val = t1.val + 1 ∧
      ∃ task, a.val[t1.val]? = some task ∧
        SW limit task (a1.val[t1.val]!) (a2.val[t1.val]!) (a3.val[t1.val]!) := by
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
        have hsw : SW limit task (a1.val[t1.val]!) (a2.val[t1.val]!) (a3.val[t1.val]!) := by
          clear hg1 hfine hdc
          ite_split hnr
          all_goals
            have hnr' := ok_inj hnr
            subst hnr'
            try simp only [↓reduceIte] at hrow
            ite_split hrow
          all_goals first
            | (simp at hrow; done)
            | (refine ⟨?_, ?_, ?_⟩ <;> first
                | (scalar_tac; done)
                | (intro _; scalar_tac)
                | (intro hk; rcases hk with hk | hk <;> contradiction))
        have ht3' := usize_add_ok ht3
        refine ⟨hg, by simpa using ht3', task, (index_ok' htask).2, hsw⟩
      · simp only [bind_eq_ok] at hb2
        obtain ⟨t3, ht3, hb2⟩ := hb2
        have hb2 := ok_inj hb2
        cases hb2
        cases hg'
    · have hb := ok_inj hb; cases hb
  · have hb := ok_inj hb1; cases hb

theorem validate_slots_loop0_done {a : Array logic.TaskCfg NT} {a1 a2 a3 : Array U8 NT}
    {limit : Usize} {g y : Bool} {t1 : Usize}
    (hb : logic.Config.validate_slots_loop0.body a a1 a2 a3 limit g t1
      = ok (ControlFlow.done y)) :
    y = g ∧ (y = true → NT.val ≤ t1.val) := by
  unfold logic.Config.validate_slots_loop0.body at hb
  rcases ite_cases hb with ⟨hlt, hb1⟩ | ⟨hlt, hb1⟩ <;> clear hb
  · rcases ite_cases hb1 with ⟨hg, hb⟩ | ⟨hg, hb⟩ <;> clear hb1
    · simp only [bind_eq_ok, lift, bind_ok] at hb
      obtain ⟨task, htask, d, hd, b, hbb, r, hr, nr, hnr, dc, hdc, row, hrow, hb⟩ := hb
      rcases ite_cases hb with ⟨hrw, hb2⟩ | ⟨hrw, hb2⟩ <;> clear hb
      · simp only [bind_eq_ok] at hb2
        obtain ⟨fine, hfine, g1, hg1, t3, ht3, hb2⟩ := hb2
        have hb2 := ok_inj hb2
        cases hb2
      · simp only [bind_eq_ok] at hb2
        obtain ⟨t3, ht3, hb2⟩ := hb2
        have hb2 := ok_inj hb2
        cases hb2
    · have hb := ok_inj hb
      cases hb
      exact ⟨rfl, fun e => absurd e hg⟩
  · have hb := ok_inj hb1
    cases hb
    exact ⟨rfl, fun _ => by scalar_tac⟩

theorem validate_slots_loop0_ok {a : Array logic.TaskCfg NT} {a1 a2 a3 : Array U8 NT}
    {limit : Usize} {good r : Bool} {t : Usize}
    (hinv : good = true → ∀ (u : Nat), u < t.val → ∀ (tc : logic.TaskCfg), a.val[u]? = some tc →
      SW limit tc (a1.val[u]!) (a2.val[u]!) (a3.val[u]!))
    (h : logic.Config.validate_slots_loop0 a a1 a2 a3 limit good t = ok r) :
    r = true → ∀ (u : Nat) (tc : logic.TaskCfg), a.val[u]? = some tc →
      SW limit tc (a1.val[u]!) (a2.val[u]!) (a3.val[u]!) := by
  have hlen : a.val.length = NT.val := a.property
  unfold logic.Config.validate_slots_loop0 at h
  refine loop_inv _
    (fun x => x.1 = true → ∀ (u : Nat), u < x.2.val → ∀ (tc : logic.TaskCfg),
      a.val[u]? = some tc → SW limit tc (a1.val[u]!) (a2.val[u]!) (a3.val[u]!))
    (fun r => r = true → ∀ (u : Nat) (tc : logic.TaskCfg), a.val[u]? = some tc →
      SW limit tc (a1.val[u]!) (a2.val[u]!) (a3.val[u]!)) ?_ ?_ (good, t) r hinv h
  · rintro ⟨g, t1⟩ ⟨g', t2⟩ hi hb
    dsimp only at hi hb ⊢
    intro hg' u hu tc htc
    obtain ⟨hg, ht2, task, e, hsw⟩ := validate_slots_loop0_cont hb hg'
    by_cases hu' : u < t1.val
    · exact hi hg u hu' tc htc
    · have hue : u = t1.val := by omega
      rw [hue, e] at htc
      cases htc
      rw [hue]
      exact hsw
  · rintro ⟨g, t1⟩ y hi hb
    dsimp only at hi hb ⊢
    intro hy u tc htc
    obtain ⟨rfl, hge⟩ := validate_slots_loop0_done hb
    have hu : u < a.val.length := (List.getElem?_eq_some_iff.mp htc).1
    exact hi hy u (by have := hge hy; omega) tc htc

/-- What the proofs use of `Config::wf`. -/
structure CfgW (cfg : logic.Config NT NR NP) : Prop where
  nt : 0 < NT.val ∧ NT.val ≤ 255
  np : NP.val ≤ 255
  od : cfg.operating_duration.val < 4611686018427387904
  task : ∀ (u : Nat) (tc : logic.TaskCfg), cfg.tasks.val[u]? = some tc → TW NP cfg.operating_duration tc
  sc : 2 ≤ cfg.slot_count.val
  slots : ∀ (u : Nat) (tc : logic.TaskCfg), cfg.tasks.val[u]? = some tc →
    (cfg.slot_deadline.val[u]!).val < cfg.slot_count.val - 2 ∧
    (cfg.slot_budget.val[u]!).val < cfg.slot_count.val - 2 ∧
    ((tc.kind = logic.KIND_PERIODIC ∨ tc.mit_policy = logic.MIT_DEFER) →
      (cfg.slot_release.val[u]!).val < cfg.slot_count.val - 2)

theorem validate_slots_ok {cfg : logic.Config NT NR NP}
    (h : logic.Config.validate_slots cfg = ok true) :
    2 ≤ cfg.slot_count.val ∧ ∀ (u : Nat) (tc : logic.TaskCfg), cfg.tasks.val[u]? = some tc →
      (cfg.slot_deadline.val[u]!).val < cfg.slot_count.val - 2 ∧
      (cfg.slot_budget.val[u]!).val < cfg.slot_count.val - 2 ∧
      ((tc.kind = logic.KIND_PERIODIC ∨ tc.mit_policy = logic.MIT_DEFER) →
        (cfg.slot_release.val[u]!).val < cfg.slot_count.val - 2) := by
  unfold logic.Config.validate_slots at h
  split at h
  · have := ok_inj h; cases this
  · rename_i hsc
    simp only [bind_eq_ok, lift, bind_ok] at h
    obtain ⟨i, hi, h⟩ := h
    have hl := validate_slots_loop0_ok (fun _ u hu => absurd hu (by scalar_tac)) h rfl
    have hiv : i.val = cfg.slot_count.val - 2 := by
      have := UScalar.sub_equiv cfg.slot_count 2#u8
      have h2 : (2#u8).val = 2 := rfl
      simp_all
    refine ⟨by scalar_tac, fun u tc htc => ?_⟩
    obtain ⟨s1, s2, s3⟩ := hl u tc htc
    have hc : (UScalar.cast UScalarTy.Usize i).val = i.val := by scalar_tac
    rw [hc, hiv] at s1 s2 s3
    exact ⟨s1, s2, s3⟩

set_option maxHeartbeats 2000000 in
/-- **`Config::validate` returning `true` gives `CfgW`.** -/
theorem validate_ok {cfg : logic.Config NT NR NP}
    (h : logic.Config.validate cfg = ok true) : CfgW cfg := by
  unfold logic.Config.validate at h
  peel h
  obtain ⟨c1, c2⟩ := validate_slots_ok (by simp_all : logic.Config.validate_slots cfg = ok true)
  have hl0 := validate_loop0_ok (fun _ u hu => absurd hu (by scalar_tac))
    (by simp_all : logic.Config.validate_loop0 NP cfg.tasks cfg.operating_duration true 0#usize
      = ok true) rfl
  exact ⟨⟨by scalar_tac, by scalar_tac⟩, by scalar_tac, by scalar_tac, hl0, c1, c2⟩

theorem cfgw_ci {cfg : logic.Config NT NR NP} (hw : CfgW cfg)
    (hns : cfg.slot_count.val = NS.val) : CI cfg NS := by
  have hlen : cfg.tasks.val.length = NT.val := cfg.tasks.property
  refine ⟨fun t ht => ?_, by have := hw.nt.2; omega, fun i h => ?_⟩
  · have hs := hw.slots t (cfg.tasks.val[t]'(by omega)) (List.getElem?_eq_getElem (by omega))
    refine ⟨by omega, by omega, fun tc htc hk => ?_⟩
    have := (hw.slots t tc htc).2.2 hk
    omega
  · exact (hw.task i _ (List.getElem?_eq_getElem h)).2.2.1

theorem cfgw_ct {cfg : logic.Config NT NR NP} (hw : CfgW cfg)
    (hns : cfg.slot_count.val = NS.val) : CT cfg NS := by
  refine ⟨cfgw_ci hw hns, hw.od, fun i h => ?_, fun i h => ?_⟩
  · have := (hw.task i _ (List.getElem?_eq_getElem h)).2.2.2.1
    have := hw.od
    omega
  · obtain ⟨_, _, _, a, b, _⟩ := hw.task i _ (List.getElem?_eq_getElem h)
    have := hw.od
    omega

/-- **`Bound::validate` returning `true`**: the configuration is well
formed, the slot table has its size and the log's capacity is in range. -/
theorem bound_validate_ok {b : arch.bound.Bound NT NR NP NL NS NE}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.validate b = ok true) :
    CfgW b.cfg ∧ b.cfg.slot_count.val = NS.val ∧ 0 < NE.val ∧ NE.val < 1073741824 := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.validate at h
  peel h
  all_goals first
    | (refine ⟨validate_ok (by simp_all), ?_, ?_, ?_⟩ <;> scalar_tac)
    | (exfalso; simp_all; done)

/-! ## The initial state -/

theorem new_loop_ok {cfg : logic.Config NT NR NP} (hw : CfgW cfg) {a a' : Array U64 NT}
    {t : Usize}
    (hinv : ∀ u, u < NT.val → (a.val[u]!).val ≤ cfg.operating_duration.val)
    (h : logic.time.Timing.new_loop cfg a t = ok a') :
    ∀ u, u < NT.val → (a'.val[u]!).val ≤ cfg.operating_duration.val := by
  unfold logic.time.Timing.new_loop at h
  refine loop_inv _ (fun x => ∀ u, u < NT.val → (x.1.val[u]!).val ≤ cfg.operating_duration.val)
    (fun r => ∀ u, u < NT.val → (r.val[u]!).val ≤ cfg.operating_duration.val) ?_ ?_ (a, t) a'
    hinv h
  · rintro ⟨a0, t1⟩ ⟨a1, t2⟩ hi hb
    dsimp only at hi hb ⊢
    unfold logic.time.Timing.new_loop.body at hb
    split at hb
    · simp only [bind_eq_ok, lift, bind_ok] at hb
      obtain ⟨tc, htc, a2, ha2, t3, ht3, hb⟩ := hb
      have hb := ok_inj hb
      cases hb
      obtain ⟨hlt, rfl⟩ := update_ok ha2
      obtain ⟨_, e⟩ := index_ok' htc
      intro u hu
      rw [get_set _ _ _ _ hlt]
      split
      · exact (hw.task _ _ e).2.2.2.2.2
      · exact hi u hu
    · have hb := ok_inj hb
      cases hb
  · rintro ⟨a0, t1⟩ y hi hb
    dsimp only at hi hb ⊢
    unfold logic.time.Timing.new_loop.body at hb
    split at hb
    · simp only [bind_eq_ok, lift, bind_ok] at hb
      obtain ⟨tc, htc, a2, ha2, t3, ht3, hb⟩ := hb
      have hb := ok_inj hb
      cases hb
    · have hb := ok_inj hb
      cases hb
      exact hi

/-- The instants `Timing::new` starts from are within their bounds. -/
theorem timing_new_ok {cfg : logic.Config NT NR NP} (hw : CfgW cfg) {tm : logic.time.Timing NT}
    (h : logic.time.Timing.new cfg = ok tm) : TmI tm := by
  unfold logic.time.Timing.new at h
  simp only [bind_eq_ok] at h
  obtain ⟨a9, ha9, h⟩ := h
  have h := ok_inj h
  subst h
  have hnom := new_loop_ok hw (fun u hu => by rw [repeat_get _ _ u hu]; exact Nat.zero_le _) ha9
  have hod := hw.od
  refine ⟨fun u hu => ?_, fun u hu => ?_, fun u hu => ?_⟩
  · have := hnom u hu
    dsimp only
    omega
  · dsimp only
    rw [repeat_get _ _ u hu]
    simp
  · dsimp only
    rw [repeat_get _ _ u hu]
    simp

/-- **The state `Kernel::init` builds satisfies `KT`**, with no Job on the
stack. (`init` writes in place the state `Kernel::new` returned until
2026-11-27, whatever the state before, `kold`. That it returns cannot be
stated here: it builds the empty ghost history through `no_history`,
which the translation keeps opaque.) -/
theorem new_ok {cfg : logic.Config NT NR NP} (hw : CfgW cfg)
    (hne : 0 < NE.val ∧ NE.val < 1073741824) {k : logic.kernel.Kernel NT NR NP NL NS NE}
    {kold : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.init kold cfg = ok k) : KT k ∧ k.sched.depth.val = 0 := by
  unfold logic.kernel.Kernel.init logic.sched.Sched.new logic.time.Slots.new logic.log.Log.new at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨tm, htm, bg, hbg, g, hg, h⟩ := h
  have h := ok_inj h
  subst h
  refine ⟨⟨⟨⟨Nat.zero_le _, fun p hp => absurd hp (by simp), Nat.zero_le _,
    fun p hp => absurd hp (by simp)⟩, ⟨hne.1, hne.2, hne.1, Nat.zero_le _⟩, by simp⟩,
    timing_new_ok hw htm⟩, rfl⟩

/-- The configuration part of `okB`, from `CfgW`. -/
theorem cfgw_okb {b : arch.bound.Bound NT NR NP NL NS NE} (hw : CfgW b.cfg)
    (hd : b.kernel.sched.depth.val = 0) : okB b := by
  have hnp := hw.np
  refine ⟨hw.nt.2, fun t ht => ?_, fun i hi => ?_, fun i j _ hj => ?_⟩
  · have hlen : b.cfg.tasks.val.length = NT.val := b.cfg.tasks.property
    have e : b.cfg.tasks.val[t]? = some (b.cfg.tasks.val[t]'(by omega)) :=
      List.getElem?_eq_getElem (by omega)
    obtain ⟨t1, t2, t3, _⟩ := hw.task t _ e
    simp only [kvB, prio, partOf, e]
    exact ⟨t1, by omega, by omega⟩
  · simp only [kvB] at hi; omega
  · simp only [kvB] at hj; omega

/-- **`Bound::init` after a `validate` that returned `true`**: the Kernel's
invariant as Kernel_Arch needs it, no Job on the stack, and `KT`. -/
theorem bound_init_ok {b b' : arch.bound.Bound NT NR NP NL NS NE}
    (hv : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.validate b = ok true)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init b = ok b') :
    okB b' ∧ (kvB b').depth = 0 ∧ KT b'.kernel ∧ CT b'.cfg NS := by
  obtain ⟨hw, hns, hne⟩ := bound_validate_ok hv
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init at h
  simp only [bind_eq_ok] at h
  obtain ⟨k, hk, a, ha, h⟩ := h
  have h := ok_inj h
  subst h
  obtain ⟨hkt, hd⟩ := new_ok hw hne hk
  exact ⟨cfgw_okb hw hd, hd, hkt, cfgw_ct hw hns⟩

/-- `ki`'s boot methods are the extracted ones. -/
structure IsBoundBoot (ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)) :
    Prop where
  validate : ki.validate = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.validate
  init : ki.init = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.init

/-- **The Kernel's `validate` and `init` satisfy `KBoot`.** -/
theorem bound_kboot {ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)}
    (hb : IsBoundBoot ki) : KBoot ki kvB okB where
  init := fun hv h => by
    rw [hb.validate] at hv
    rw [hb.init] at h
    obtain ⟨a, b, _⟩ := bound_init_ok hv h
    exact ⟨a, b⟩

end P18
