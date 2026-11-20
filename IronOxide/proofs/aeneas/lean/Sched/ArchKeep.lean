import Sched.ArchEntry
open Aeneas Aeneas.Std Result

/-! # The entry cores keep the invariant (2026-11-19)

`AI kv k s hop` is the invariant of `Sched/ArchInv.lean` read off a Kernel
`k` (through the view `kv` of its trait) and an `ArchState` `s`. This file
proves, for each entry core as Aeneas extracts it: called in a state that
satisfies the invariant with nothing planned, if the core returns, the
state it leaves satisfies the invariant with the unwind plan the core
hands its stub. -/

namespace P18
open rsk_kernel

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

/-- The invariant of the entry cores on the extracted state. -/
def AI (kv : K → Arch.KV) (k : K) (s : arch.cm4.plan.ArchState) (hop : Nat) : Prop :=
  Arch.AInv (kv k) (lvOf s.levels) s.current_partition.val s.pending_unwind.val hop

theorem lv_act {levels : Array arch.cm4.plan.LevelRecord 8#usize} {l : Nat}
    (h : (lvOf levels).act l = true) : l < 8 ∧ act levels l = true := by
  simp only [lvOf, Bool.and_eq_true, decide_eq_true_eq] at h
  exact ⟨h.1, h.2⟩

theorem lv_act_mk {levels : Array arch.cm4.plan.LevelRecord 8#usize} {l : Nat}
    (h8 : l < 8) (h : act levels l = true) : (lvOf levels).act l = true := by
  simp only [lvOf, Bool.and_eq_true, decide_eq_true_eq]
  exact ⟨h8, h⟩

theorem lv_act_false {levels : Array arch.cm4.plan.LevelRecord 8#usize} {l : Nat}
    (h : l < 8 → act levels l = false) : (lvOf levels).act l = false := by
  simp only [lvOf]
  by_cases h8 : l < 8
  · have := h h8
    simp only [act] at this
    rw [this]; simp
  · simp [h8]

/-- `top_active` names the topmost record, when there is one. -/
theorem top_active_isTop {levels : Array arch.cm4.plan.LevelRecord 8#usize} {l : Usize} {l0 : Nat}
    (h : arch.cm4.plan.top_active levels = ok l) (h0 : (lvOf levels).act l0 = true)
    (h1 : 1 ≤ l0) : Arch.IsTop (lvOf levels) l.val := by
  obtain ⟨h8, hact, hnone⟩ := top_active_spec h
  obtain ⟨hl8, hla⟩ := lv_act h0
  have hne : l.val ≠ 0 := by
    intro hz
    have := hnone l0 (by omega) hl8
    rw [hla] at this; cases this
  exact ⟨lv_act_mk h8 (hact hne), fun j hj => lv_act_false (fun h8' => hnone j hj h8')⟩

/-- `end_plan_core`: the plan of `ended_plan`. -/
theorem end_plan_core_ok [KSpec ki kv okK] {k k0 : K} {s : arch.cm4.plan.ArchState}
    {db : Usize} {hop : U32} {g1 : verus_builtin.Ghost K}
    {g2 : verus_builtin.Ghost arch.cm4.plan.ArchState} {cp : Nat}
    (hc : Arch.ChainOk (kv k0) (lvOf s.levels) cp)
    (hdb : db.val = (kv k0).depth)
    (h : arch.cm4.entry.end_plan_core ki k s db g1 g2 = ok hop) :
    Arch.EndedPlan (kv k) (kv k0) (lvOf s.levels) hop.val := by
  unfold arch.cm4.entry.end_plan_core at h
  simp only [bind_eq_ok] at h
  obtain ⟨i, hi, h⟩ := h
  have hiv := KSpec.depth (kv := kv) (okK := okK) hi
  unfold Arch.EndedPlan
  split at h
  · rename_i hlt
    have hlt' : (kv k).depth < (kv k0).depth := by rw [← hiv, ← hdb]; scalar_tac
    rw [if_pos hlt']
    simp only [bind_eq_ok] at h
    obtain ⟨level, hlev, lr, hlr, h⟩ := h
    have h := ok_inj h
    obtain ⟨_, rfl⟩ := index_ok hlr
    obtain ⟨ha, _⟩ := hc.2.1 (kv k).depth hlt'
    have r1 := (hc.1 _ ha).1
    refine ⟨level.val, top_active_isTop hlev ha r1, ?_⟩
    rw [← h]
    simp only [lvOf]
    scalar_tac
  · rename_i hge
    have hge' : ¬ (kv k).depth < (kv k0).depth := by rw [← hiv, ← hdb]; scalar_tac
    rw [if_neg hge']
    have h := ok_inj h
    rw [← h]; rfl

/-- A Kernel entry that leaves the records alone, ends Jobs from the top of
the stack only, and plans with `end_plan_core` keeps the invariant. -/
theorem kentry [KSpec ki kv okK] {k k' : K} {s s' : arch.cm4.plan.ArchState} {db : Usize}
    {hop : U32} {g1 : verus_builtin.Ghost K} {g2 : verus_builtin.Ghost arch.cm4.plan.ArchState}
    (hend : arch.cm4.entry.end_plan_core ki k' s' db g1 g2 = ok hop)
    (hd : ki.depth k = ok db) (hkr : KR kv okK k' k) (hfr : Frame s' s)
    (hok : okK k) (hai : AI kv k s 0) : okK k' ∧ AI kv k' s' hop.val := by
  obtain ⟨hok', hpre⟩ := hkr hok
  have hk := KSpec.kok (ki := ki) (kv := kv) hok
  refine ⟨hok', ?_⟩
  have hc := hai.1
  rw [← hfr.1] at hc
  have hpl := end_plan_core_ok (okK := okK) hc (KSpec.depth (kv := kv) (okK := okK) hd) hend
  unfold AI at hai ⊢
  rw [hfr.1] at hpl
  rw [hfr.1, hfr.2.1, hfr.2.2]
  exact Arch.AInv.entry hk hai hpre hpl

/-- A Kernel entry that leaves the records alone and the stack as deep as
it was keeps the invariant with nothing planned. -/
theorem kentry0 [KSpec ki kv okK] {k k' : K} {s s' : arch.cm4.plan.ArchState}
    (hkr : KR kv okK k' k) (hdep : okK k → (kv k').depth = (kv k).depth) (hfr : Frame s' s)
    (hok : okK k) (hai : AI kv k s 0) : okK k' ∧ AI kv k' s' 0 := by
  obtain ⟨hok', hpre⟩ := hkr hok
  have hk := KSpec.kok (ki := ki) (kv := kv) hok
  refine ⟨hok', ?_⟩
  unfold AI at hai ⊢
  rw [hfr.1, hfr.2.1, hfr.2.2]
  refine Arch.AInv.entry hk hai hpre ?_
  unfold Arch.EndedPlan
  rw [if_neg (by rw [hdep hok]; omega)]

/-- Closes an exit of a Kernel-level entry core that plans with
`end_plan_core`. -/
syntax "kentry_close " ident ident : tactic
macro_rules
  | `(tactic| kentry_close $hok:ident $hai:ident) => `(tactic| (
      refine kentry (by assumption) (by assumption) ?_ ?_ $hok $hai
      · kr_close; done
      · frame_close))

theorem timer_core_keeps [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool} (hok : okK k) (hai : AI kv k s 0)
    (h : arch.cm4.entry.timer_core ki ai ti k hw tables s = ok ((hop, safe), k', hw', s')) :
    okK k' ∧ AI kv k' s' hop.val := by
  unfold arch.cm4.entry.timer_core at h
  peel h
  all_goals kentry_close hok hai

theorem fault_core_keeps [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool} {kind : U8} {in_kernel : Bool}
    (hok : okK k) (hai : AI kv k s 0)
    (h : arch.cm4.entry.fault_core ki ai ti k hw tables s kind in_kernel
      = ok ((hop, safe), k', hw', s')) :
    okK k' ∧ AI kv k' s' hop.val := by
  unfold arch.cm4.entry.fault_core at h
  peel h
  all_goals kentry_close hok hai

theorem stack_violation_core_keeps [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool} (hok : okK k) (hai : AI kv k s 0)
    (h : arch.cm4.entry.stack_violation_core ki ai ti k hw tables s
      = ok ((hop, safe), k', hw', s')) :
    okK k' ∧ AI kv k' s' hop.val := by
  unfold arch.cm4.entry.stack_violation_core at h
  peel h
  all_goals kentry_close hok hai

theorem intake_core_keeps [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool} {n : U16} {task partition : U8}
    {psp exc_return : U32} (hok : okK k) (hai : AI kv k s 0)
    (h : arch.cm4.entry.intake_core ki ai ti k hw tables s n task partition psp exc_return
      = ok ((hop, safe), k', hw', s')) :
    okK k' ∧ AI kv k' s' hop.val := by
  unfold arch.cm4.entry.intake_core at h
  peel h
  all_goals kentry_close hok hai

theorem unbound_core_keeps [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool} {n : U16}
    (hok : okK k) (hai : AI kv k s 0)
    (h : arch.cm4.entry.unbound_core ki ai ti k hw tables s n = ok ((hop, safe), k', hw', s')) :
    okK k' ∧ AI kv k' s' hop.val := by
  unfold arch.cm4.entry.unbound_core at h
  peel h
  all_goals kentry_close hok hai

end P18
