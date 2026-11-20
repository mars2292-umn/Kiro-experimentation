import Sched.ArchUnwindAll
import Sched.ArchIrq
open Aeneas Aeneas.Std Result

/-! # What the interrupt and unwind cores do to the level records (2026-11-20)

The `_keeps` theorems say the invariant survives each core. The join with
the machine model of the stubs (`Sched/World.lean`) needs more: which
record a dispatch writes and which record an unwind clears, so that the
register records on the main stack can be matched with them.

- `irq_entry_core_ir`: with no dispatch the level records are as they
  were; a dispatch writes one record, above every active one, keeping the
  EXC_RETURN value the stub was entered with, and happens only on an entry
  from Thread mode; and an unwind is planned only on such an entry.
- `epilogue_core_pop`: the unwind core clears the topmost record and leaves
  every other one. -/

namespace Arch

/-- A dispatch from a state with nothing to unwind starts a Job whose level
is above every active record. -/
theorem ChainOk.push_low {v v0 : KV} {L : LV} {cp t level : Nat} (hk : KOk v)
    (h : ChainOk v0 L cp) (hne : ¬ Extra v0 L)
    (hd : v.depth = v0.depth + 1) (hst : ∀ i, i < v0.depth → v.stack i = v0.stack i)
    (hpr : v.prio = v0.prio) (ht : v.stack v0.depth = t) (hlev : level = v.prio t) :
    ∀ l, L.act l = true → l < level := by
  obtain ⟨c1, c2, c3, c4, c5⟩ := h
  have hstk : ∀ i, i < v0.depth → v0.prio (v0.stack i) < level := by
    intro i hi
    have := hk.1 i v0.depth hi (by omega)
    rw [ht, ← hlev, hst i hi, hpr] at this
    exact this
  intro l ha
  have hle : l ≤ topLevel v0 := by
    by_contra hgt
    exact hne ⟨l, ha, by omega⟩
  obtain ⟨i, hi, he⟩ := c3 l ha hle
  rw [he]; exact hstk i hi

end Arch

namespace P18
open rsk_kernel

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

/-- `s'` is `s` with one record written: at a level above every active
one, active, keeping the EXC_RETURN value `e`. -/
def Pushed (s s' : arch.cm4.plan.ArchState) (e : U32) : Prop :=
  ∃ level : Nat, level < 8 ∧ (∀ l, l ≠ level → s'.levels.val[l]! = s.levels.val[l]!) ∧
    (s'.levels.val[level]!).exc_return = e ∧ (s'.levels.val[level]!).active = true ∧
    (∀ l, (lvOf s.levels).act l = true → l < level)

/-- The records after an interrupt entry that returned `plan`. -/
def IR (s s' : arch.cm4.plan.ArchState) (e : U32) (plan : arch.cm4.entry.DispatchPlan) : Prop :=
  (plan.psp = 0#u32 → s'.levels = s.levels) ∧ (plan.psp ≠ 0#u32 → Pushed s s' e)

theorem IR.left {s0 s s' : arch.cm4.plan.ArchState} {e : U32}
    {plan : arch.cm4.entry.DispatchPlan} (hs : s.levels = s0.levels) (h : IR s s' e plan) :
    IR s0 s' e plan := by
  refine ⟨fun hz => (h.1 hz).trans hs, fun hne => ?_⟩
  obtain ⟨level, h8, h1, h2, h3, h4⟩ := h.2 hne
  exact ⟨level, h8, fun l hl => by rw [h1 l hl, hs], h2, h3, fun l ha => h4 l (by rw [hs]; exact ha)⟩

/-- The record a dispatch that started a Job writes. -/
theorem disp_rel [KSpec ki kv okK] {k k1 : K} {hw : A} {s s1 s' : arch.cm4.plan.ArchState}
    {pre : U32} {o : Option U8} {info : arch.bound.JobInfo} {db d1 : Usize} {t : U8} {n : U16}
    {psp control exc_return floor_before : U32}
    {a1 : Array arch.cm4.plan.LevelRecord 8#usize}
    (hok : okK k) (hai : AI kv k s 0) (hp0 : s.pending_unwind = 0#u32)
    (hcp : arch.cm4.entry.charge_point ai hw s = ok (pre, s1))
    (_htop : ki.top k = ok o) (hji : ki.job_info k t = ok info)
    (hdb : ki.depth k = ok db) (hdisp : ki.dispatch k t pre = ok k1)
    (hd1 : ki.depth k1 = ok d1) (hne : ¬ d1 = db)
    (hupd : Array.update s1.levels (UScalar.cast .Usize info.priority)
      ({ exception_number := n, task := t, partition := info.partition, psp, control,
         exc_return, floor_before, previous_partition := s1.current_partition,
         previous_task := prevOf o, active := true } : arch.cm4.plan.LevelRecord) = ok a1)
    (e1 : s'.levels = a1) : Pushed s s' exc_return := by
  have hfr := charge_point_frame hcp
  obtain ⟨hok1, hpr, hpa, hst, hdep⟩ := KSpec.dispatch (kv := kv) hok hdisp
  have hdbv := KSpec.depth (kv := kv) (okK := okK) hdb
  have hd1v := KSpec.depth (kv := kv) (okK := okK) hd1
  have hdd : (kv k1).depth = (kv k).depth + 1 ∧ (kv k1).stack (kv k).depth = t.val := by
    rcases hdep with h | h
    · exfalso; apply hne; scalar_tac
    · exact h
  obtain ⟨j1, j2⟩ := KSpec.job_info (kv := kv) hok hji
  obtain ⟨hl, rfl⟩ := update_ok hupd
  have hlv : (UScalar.cast UScalarTy.Usize info.priority).val = (kv k1).prio t.val := by
    rw [hpr, ← j1]; scalar_tac
  have hk1 := KSpec.kok (ki := ki) (kv := kv) hok1
  unfold AI at hai
  obtain ⟨hc, _, h0, _⟩ := hai
  have hnex : ¬ Arch.Extra (kv k) (lvOf s.levels) := by
    intro hex
    have := (h0 rfl).mp hex
    rw [hp0] at this
    exact this rfl
  have hlow := Arch.ChainOk.push_low (level := (kv k1).prio t.val) hk1 hc hnex hdd.1 hst hpr
    hdd.2 rfl
  refine ⟨(UScalar.cast UScalarTy.Usize info.priority).val, by scalar_tac, fun l hne' => ?_, ?_, ?_,
    fun l ha => by rw [hlv]; exact hlow l ha⟩
  · rw [e1, get_set _ _ _ _ hl, if_neg hne', hfr.1]
  · rw [e1, get_set _ _ _ _ hl, if_pos rfl]
  · rw [e1, get_set _ _ _ _ hl, if_pos rfl]

set_option maxHeartbeats 16000000 in
theorem dispatch_job_core_ir [KSpecD ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (hok : okK k) (hai : AI kv k s 0) (hp0 : s.pending_unwind = 0#u32)
    (h : arch.cm4.entry.dispatch_job_core ki ai ti k hw tables s n t psp control exc_return
      = ok ((plan, safe), k', hw', s')) : IR s s' exc_return plan := by
  unfold arch.cm4.entry.dispatch_job_core at h
  peel h
  all_goals first
    | exact absurd trivial (by assumption)
    | exact ⟨fun hz => (by dsimp only at hz; contradiction), fun _ =>
        disp_rel (ki := ki) (ai := ai) hok hai hp0 (by assumption) (by assumption)
          (by assumption) (by assumption) (by assumption) (by assumption) (by assumption)
          (by assumption) (by rfl)⟩
    | (refine ⟨fun _ => ?_, fun hne => absurd rfl hne⟩
       frame_proj
       done)

frame_reg irq_loop_frame

theorem bne32_ne {a : U32} (h : (a != 0#u32) = true) : a ≠ 0#u32 := by
  intro e
  simp [e] at h

set_option maxHeartbeats 32000000 in
theorem dispatch_vector_core_ir [KSpecD ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (hok : okK k) (hai : AI kv k s 0)
    (h : arch.cm4.entry.dispatch_vector_core ki ai ti k hw tables s n t psp control exc_return
      = ok ((plan, safe), k', hw', s')) :
    IR s s' exc_return plan ∧ (plan.psp ≠ 0#u32 → exc_return &&& 8#u32 ≠ 0#u32) := by
  unfold arch.cm4.entry.dispatch_vector_core at h
  simp only [arch.cm4.entry.WINDOW_DEFER, ↓reduceIte] at h
  peel h
  all_goals first
    | exact absurd (not_bne32 (by assumption)) (by assumption)
    | exact ⟨dispatch_job_core_ir hok hai (not_bne32 (by assumption)) h,
        fun _ => bne32_ne (by assumption)⟩
    | (refine ⟨⟨fun _ => ?_, fun hne => absurd rfl hne⟩, fun hne => absurd rfl hne⟩
       frame_proj
       done)

theorem bool_tf {b : Bool} {P : Prop} (h1 : b = true) (h2 : b = false) : P := by
  rw [h1] at h2; cases h2

/-- Closes an exit of `irq_entry_core` for `irq_entry_core_ir`. -/
syntax "irq_rel_exit " ident ident : tactic
macro_rules
  | `(tactic| irq_rel_exit $hok:ident $hai:ident) => `(tactic| (
      first
        | have hs := (irq_loop_frame (by assumption)).1
        | have hs : ($hai = $hai) := rfl
      first
        | (have hai' := AI.frame (irq_loop_frame (by assumption)) $hai
           have hd := dispatch_vector_core_ir $hok hai' (by assumption)
           first
             | exact ⟨IR.left (by assumption) hd.1, hd.2, fun _ _ => bne32_ne (by assumption)⟩
             | exact ⟨IR.left (by assumption) hd.1, hd.2, fun _ _ => (by assumption)⟩
             | exact ⟨IR.left (by assumption) hd.1, hd.2, fun hsf => bool_tf (by assumption) hsf⟩
             | exact ⟨IR.left (by assumption) hd.1, hd.2, fun hsf => Bool.noConfusion hsf⟩
             | exact ⟨IR.left (by assumption) hd.1, hd.2,
                 fun _ hh => absurd (not_bne32 (by assumption)) hh⟩
             | exact ⟨IR.left (by assumption) hd.1, hd.2, fun _ _ => hd.2 (by assumption)⟩
             | exact ⟨⟨fun _ => (hd.1.1 (by assumption)).trans (by assumption),
                 fun hne => absurd rfl hne⟩, fun hne => absurd rfl hne, fun _ hh => absurd rfl hh⟩)
        | (have hd := dispatch_vector_core_ir $hok $hai (by assumption)
           first
             | exact ⟨hd.1, hd.2, fun _ _ => bne32_ne (by assumption)⟩
             | exact ⟨hd.1, hd.2, fun _ _ => (by assumption)⟩
             | exact ⟨hd.1, hd.2, fun hsf => bool_tf (by assumption) hsf⟩
             | exact ⟨hd.1, hd.2, fun hsf => Bool.noConfusion hsf⟩
             | exact ⟨hd.1, hd.2, fun _ hh => absurd (not_bne32 (by assumption)) hh⟩
             | exact ⟨hd.1, hd.2, fun _ _ => hd.2 (by assumption)⟩
             | exact ⟨⟨fun _ => hd.1.1 (by assumption), fun hne => absurd rfl hne⟩,
                 fun hne => absurd rfl hne, fun _ hh => absurd rfl hh⟩)
        | (refine ⟨⟨fun _ => ?_, fun hne => absurd rfl hne⟩, fun hne => absurd rfl hne, ?_⟩
           · frame_proj
             done
           · first
               | exact fun _ _ => bne32_ne (by assumption)
               | exact fun _ _ => (by assumption)
               | exact fun hsf => bool_tf (by assumption) hsf
               | exact fun _ hh => absurd rfl hh
               | exact fun hsf => Bool.noConfusion hsf
               | exact fun _ hh => absurd (not_bne32 (by assumption)) hh
               | (intro _ hh; simp_all; done))))

set_option maxHeartbeats 64000000 in
/-- **The level records after `irq_entry_core`.** -/
theorem irq_entry_core_ir [KSpecD ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {ipsr psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (hok : okK k) (hai : AI kv k s 0)
    (h : arch.cm4.entry.irq_entry_core ki ai ti k hw tables s ipsr psp control exc_return
      = ok ((plan, safe), k', hw', s')) :
    IR s s' exc_return plan ∧ (plan.psp ≠ 0#u32 → exc_return &&& 8#u32 ≠ 0#u32) ∧
    (safe = false → plan.hop ≠ 0#u32 → exc_return &&& 8#u32 ≠ 0#u32) := by
  unfold arch.cm4.entry.irq_entry_core at h
  simp only [arch.cm4.entry.WINDOW_DEFER, ↓reduceIte] at h
  peel h
  all_goals (irq_rel_exit hok hai; done)

/-- The records after the removal of the topmost one. -/
theorem pop_rel {levels lv' : Array arch.cm4.plan.LevelRecord 8#usize} {level : Usize}
    {rc x : arch.cm4.plan.LevelRecord}
    {back : arch.cm4.plan.LevelRecord → Array arch.cm4.plan.LevelRecord 8#usize}
    (hmut : levels.index_mut_usize level = ok (x, back)) (e1 : lv' = back (offRec rc)) :
    (∀ l, l ≠ level.val → lv'.val[l]! = levels.val[l]!) ∧ (lv'.val[level.val]!).active = false := by
  obtain ⟨hl, rfl⟩ := index_mut_ok hmut
  subst e1
  refine ⟨fun l hne => ?_, ?_⟩
  · rw [get_set _ _ _ _ hl, if_neg hne]
  · rw [get_set _ _ _ _ hl, if_pos rfl]

set_option maxHeartbeats 64000000 in
/-- **`epilogue_core` clears the topmost record and no other.** -/
theorem epilogue_core_pop [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n p c e nx : U32} {hop : Nat} {safe : Bool}
    (hok : okK k) (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (hn : (UScalar.cast .U16 (n &&& 511#u32)).val = hop)
    (h : arch.cm4.entry.epilogue_core ki ai ti k hw tables s n
      = ok ((((p, c, e, nx), safe), k', hw', s'))) :
    ∃ level, arch.cm4.plan.top_active s.levels = ok level ∧
      (∀ l, l ≠ level.val → s'.levels.val[l]! = s.levels.val[l]!) ∧
      (s'.levels.val[level.val]!).active = false := by
  unfold arch.cm4.entry.epilogue_core at h
  peel h
  all_goals first
    | (exfalso; exact epi_level0 hai hhop (by assumption) (by assumption))
    | (exfalso; exact epi_mismatch hai hhop (by assumption) (by assumption) hn (by assumption))
    | (exfalso; exact top_depth0 (ki := ki) (kv := kv) hok (by assumption) (by assumption) (by assumption))
    | (exfalso; exact epi_idle_depth (ki := ki) (kv := kv) hok hai hhop (by assumption)
        (by assumption) (by assumption) (by assumption) (by assumption) (by assumption))
    | (exfalso; exact epi_next0 (ki := ki) (kv := kv) hok hai hhop (by assumption) (by assumption)
        (by assumption) (by assumption) (by assumption) (by assumption) (by assumption))
    | (exfalso; exact epi_killed0 (ki := ki) (kv := kv) hok hai hhop (by assumption)
        (by assumption) (by assumption) (by assumption) (by assumption) (by assumption)
        (by assumption) (by assumption) (by rfl) (by assumption) (by assumption) (by assumption)
        (by assumption))
    | exact ⟨_, by assumption, pop_rel (by assumption) (by rfl)⟩
    | exact ⟨_, by assumption, pop_rel (by assumption) (apply_core_frame (by assumption)).1⟩
    | (exfalso; simp_all; done)

end P18
