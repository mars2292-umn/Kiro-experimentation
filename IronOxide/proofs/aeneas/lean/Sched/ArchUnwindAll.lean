import Sched.ArchUnwind
open Aeneas Aeneas.Std Result

/-! # The unwind core, every exit (2026-11-20)

`epilogue_core_keeps` (`Sched/ArchUnwind.lean`) covers the exits without
the safe state. The others are of two kinds. A response applied while the
Job to resume was being ended can enter the safe state: the invariant holds
there as on the other exits (`epi_killed'`). And the core has defensive
exits that raise a Kernel fault: no active record, a record that is not
the one the stub was told to unwind, a preempted Job with no record, a
record that says the idle activity was preempted while the Kernel holds a
Job. Under the invariant, called with the plan it was handed, the core
takes none of them (`epi_level0`, `epi_mismatch`, `epi_idle_depth`,
`epi_next0`, `epi_killed0`). So `epilogue_core_keeps_all` holds on every
exit. -/

namespace Arch

/-- The record of the Job the removed record preempted names a dispatch
vector. -/
theorem Popped.exc_ne {v : KV} {L L' : LV} {cp lt : Nat} (p : Popped v L L' cp lt)
    (hne : L.prevTask lt ≠ 255) : L'.exc (v.prio (L.prevTask lt)) ≠ 0 := by
  obtain ⟨j, _, hja, _, _, hprio, hexc⟩ := p.pred hne
  rw [hprio, hexc]
  exact (p.chain.1 j hja).2.2.2.2.1

/-- A record that preempted the idle activity was the only one: the Kernel
holds no Job. -/
theorem Popped.depth0 {v : KV} {L L' : LV} {cp lt : Nat} (p : Popped v L L' cp lt)
    (h255 : L.prevTask lt = 255) : v.depth = 0 := by
  by_contra hz
  have hn := p.none h255
  have := (p.chain'.2.1 0 (by omega)).1
  rw [hn] at this
  cases this

end Arch

namespace P18
open rsk_kernel

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

/-- The Job to resume was killed: the Kernel ends it and one more level is
to be unwound. -/
theorem epi_killed' [KSpec ki kv okK] {k k1 k' : K} {hw2 hw' : A} {tables : T}
    {s s1 s' : arch.cm4.plan.ArchState} {hop : Nat}
    {level : Usize} {rc x lr : arch.cm4.plan.LevelRecord} {v1 v : U8} {ji : arch.bound.JobInfo}
    {safe : Bool}
    {back : arch.cm4.plan.LevelRecord → Array arch.cm4.plan.LevelRecord 8#usize}
    (hok : okK k) (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (htl : arch.cm4.plan.top_active s.levels = ok level)
    (hrc : s.levels.index_usize level = ok rc)
    (hmut : s.levels.index_mut_usize level = ok (x, back))
    (hne : ¬ rc.previous_task = 255#u8) (hv1 : ¬ (v1 != rc.previous_task) = true)
    (htop1 : ki.top k = ok (some v1))
    (hjr : ki.job_return k 0#u32 = ok k1)
    (hap : arch.cm4.entry.apply_core ki ai ti k1 hw2 tables s1 = ok (safe, k', hw', s'))
    (e1 : s1.levels = back (offRec rc)) (e2 : s1.current_partition = rc.previous_partition)
    (e3 : s1.pending_unwind = 0#u32)
    (hji : ki.job_info k' v = ok ji)
    (hlr : s'.levels.index_usize (UScalar.cast .Usize ji.priority) = ok lr)
    (htop : ki.top k = ok (some v)) :
    okK k' ∧ AI kv k' s' (UScalar.cast .U32 lr.exception_number).val := by
  obtain ⟨p, hpt, hpp⟩ := popped_set (ki := ki) hok hai hhop htl hrc hmut
  obtain ⟨t1, t2⟩ := KSpec.top (kv := kv) hok htop1
  have hvv : v = v1 := by
    have := htop1.symm.trans htop
    simpa using this.symm
  have hd : 0 < (kv k).depth := by
    by_contra hz
    exact absurd (t1.mpr (by omega)) (by simp)
  have hve := not_bne hv1
  have hn255 : (lvOf s.levels).prevTask level.val ≠ 255 := by
    rw [hpt]; intro e; apply hne; scalar_tac
  obtain ⟨hok1, hpre1⟩ := KSpec.job_return (kv := kv) (okK := okK) hjr hok
  have hd1 := KSpec.job_return_depth (kv := kv) hok hjr hd
  obtain ⟨hok', hpre'⟩ := apply_core_kr (kv := kv) (okK := okK) hap hok1
  have hfr := apply_core_frame hap
  have hpre : Arch.Prefix (kv k') (kv k) :=
    ⟨by have := hpre'.1; have := hpre1.1; omega,
     fun i hi => (hpre'.2.1 i hi).trans (hpre1.2.1 i (by have := hpre'.1; omega)),
     hpre'.2.2.1.trans hpre1.2.2.1, hpre'.2.2.2.trans hpre1.2.2.2⟩
  have := p.killed (v2 := kv k') hn255 hd (by rw [hpt, ← t2 v1 rfl, hve]) hpre
    (by have := hpre'.1; omega)
  rw [hpp, hpt] at this
  obtain ⟨j1, j2⟩ := KSpec.job_info (kv := kv) hok' hji
  obtain ⟨_, rfl⟩ := index_ok hlr
  refine ⟨hok', ai_of ?_ (hfr.1.trans e1) (hfr.2.1.trans e2) (hfr.2.2.trans e3)⟩
  have he : (UScalar.cast UScalarTy.U32
      (s'.levels.val[(UScalar.cast UScalarTy.Usize ji.priority).val]!).exception_number).val
      = (lvOf (back (offRec rc))).exc ((kv k').prio rc.previous_task.val) := by
    simp only [lvOf]
    rw [← e1, ← hfr.1, ← hve, ← hvv, ← j1]
    scalar_tac
  rw [he]
  exact this


theorem epi_level0 {k : K} {s : arch.cm4.plan.ArchState} {hop : Nat} {level : Usize}
    (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (htl : arch.cm4.plan.top_active s.levels = ok level) (h0 : level = 0#usize) : False := by
  obtain ⟨hex, _⟩ := hai.2.1 hhop
  obtain ⟨l0, ha0, hl0⟩ := hex
  have htop := top_active_isTop htl ha0 (by omega)
  have := (hai.1.1 _ htop.1).1
  rw [h0] at this
  simp at this

theorem epi_mismatch {k : K} {s : arch.cm4.plan.ArchState} {hop : Nat} {level : Usize}
    {rc : arch.cm4.plan.LevelRecord} {n1 : U16}
    (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (htl : arch.cm4.plan.top_active s.levels = ok level)
    (hrc : s.levels.index_usize level = ok rc) (hn : n1.val = hop)
    (hne : (rc.exception_number != n1) = true) : False := by
  obtain ⟨hex, l, hl, he⟩ := hai.2.1 hhop
  obtain ⟨l0, ha0, hl0⟩ := hex
  have htop := top_active_isTop htl ha0 (by omega)
  have hll := Arch.IsTop.unique hl htop
  obtain ⟨_, rfl⟩ := index_ok hrc
  rw [hll] at he
  simp only [lvOf] at he
  have : (s.levels.val[level.val]!).exception_number = n1 := by scalar_tac
  rw [this] at hne
  simp at hne

theorem epi_idle_depth [KSpec ki kv okK] {k : K} {s : arch.cm4.plan.ArchState} {hop : Nat}
    {level d : Usize} {rc x : arch.cm4.plan.LevelRecord}
    {back : arch.cm4.plan.LevelRecord → Array arch.cm4.plan.LevelRecord 8#usize}
    (hok : okK k) (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (htl : arch.cm4.plan.top_active s.levels = ok level)
    (hrc : s.levels.index_usize level = ok rc)
    (hmut : s.levels.index_mut_usize level = ok (x, back))
    (h255 : rc.previous_task = 255#u8) (hd : ki.depth k = ok d)
    (hnz : (d != 0#usize) = true) : False := by
  obtain ⟨p, hpt, hpp⟩ := popped_set (ki := ki) hok hai hhop htl hrc hmut
  have h0 := p.depth0 (by rw [hpt, h255]; rfl)
  have hdv := KSpec.depth (kv := kv) (okK := okK) hd
  have : d = 0#usize := by scalar_tac
  simp [this] at hnz

theorem epi_next0 [KSpec ki kv okK] {k : K} {s : arch.cm4.plan.ArchState} {hop : Nat}
    {level : Usize} {rc x lr : arch.cm4.plan.LevelRecord} {ji : arch.bound.JobInfo}
    {back : arch.cm4.plan.LevelRecord → Array arch.cm4.plan.LevelRecord 8#usize}
    (hok : okK k) (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (htl : arch.cm4.plan.top_active s.levels = ok level)
    (hrc : s.levels.index_usize level = ok rc)
    (hmut : s.levels.index_mut_usize level = ok (x, back))
    (hne : (rc.previous_task != 255#u8) = true)
    (hji : ki.job_info k rc.previous_task = ok ji)
    (hlr : (back (offRec rc)).index_usize (UScalar.cast .Usize ji.priority) = ok lr)
    (h0 : UScalar.cast .U32 lr.exception_number = 0#u32) : False := by
  obtain ⟨p, hpt, hpp⟩ := popped_set (ki := ki) hok hai hhop htl hrc hmut
  have hn255 : (lvOf s.levels).prevTask level.val ≠ 255 := by
    rw [hpt]; exact bne_true hne
  have hx := p.exc_ne hn255
  rw [hpt] at hx
  obtain ⟨j1, j2⟩ := KSpec.job_info (kv := kv) hok hji
  obtain ⟨_, rfl⟩ := index_ok hlr
  apply hx
  have h0' : (UScalar.cast UScalarTy.U32
      ((back (offRec rc)).val[(UScalar.cast UScalarTy.Usize ji.priority).val]!).exception_number).val
      = 0 := by rw [h0]; rfl
  simp only [lvOf]
  rw [← j1]
  scalar_tac

theorem epi_killed0 [KSpec ki kv okK] {k k1 k' : K} {hw2 hw' : A} {tables : T}
    {s s1 s' : arch.cm4.plan.ArchState} {hop : Nat}
    {level : Usize} {rc x lr : arch.cm4.plan.LevelRecord} {v1 v : U8} {ji : arch.bound.JobInfo}
    {safe : Bool}
    {back : arch.cm4.plan.LevelRecord → Array arch.cm4.plan.LevelRecord 8#usize}
    (hok : okK k) (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (htl : arch.cm4.plan.top_active s.levels = ok level)
    (hrc : s.levels.index_usize level = ok rc)
    (hmut : s.levels.index_mut_usize level = ok (x, back))
    (hne : ¬ rc.previous_task = 255#u8) (hv1 : ¬ (v1 != rc.previous_task) = true)
    (htop1 : ki.top k = ok (some v1))
    (hjr : ki.job_return k 0#u32 = ok k1)
    (hap : arch.cm4.entry.apply_core ki ai ti k1 hw2 tables s1 = ok (safe, k', hw', s'))
    (e1 : s1.levels = back (offRec rc))
    (hji : ki.job_info k' v = ok ji)
    (hlr : s'.levels.index_usize (UScalar.cast .Usize ji.priority) = ok lr)
    (htop : ki.top k = ok (some v))
    (h0 : UScalar.cast .U32 lr.exception_number = 0#u32) : False := by
  obtain ⟨p, hpt, hpp⟩ := popped_set (ki := ki) hok hai hhop htl hrc hmut
  have hvv : v = v1 := by
    have := htop1.symm.trans htop
    simpa using this.symm
  have hve := not_bne hv1
  have hn255 : (lvOf s.levels).prevTask level.val ≠ 255 := by
    rw [hpt]; intro e; apply hne; scalar_tac
  obtain ⟨hok1, hpre1⟩ := KSpec.job_return (kv := kv) (okK := okK) hjr hok
  obtain ⟨hok', hpre'⟩ := apply_core_kr (kv := kv) (okK := okK) hap hok1
  have hfr := apply_core_frame hap
  have hpr : (kv k').prio = (kv k).prio := hpre'.2.2.1.trans hpre1.2.2.1
  have hx := p.exc_ne hn255
  rw [hpt] at hx
  obtain ⟨j1, j2⟩ := KSpec.job_info (kv := kv) hok' hji
  obtain ⟨_, rfl⟩ := index_ok hlr
  apply hx
  have h0' : (UScalar.cast UScalarTy.U32
      (s'.levels.val[(UScalar.cast UScalarTy.Usize ji.priority).val]!).exception_number).val
      = 0 := by rw [h0]; rfl
  simp only [lvOf]
  rw [← hpr, ← e1, ← hfr.1, ← hve, ← hvv, ← j1]
  scalar_tac

set_option maxHeartbeats 64000000 in
/-- **`epilogue_core` keeps the invariant on every exit.** Called with the
invariant in the form its plan left and with that plan (`n`), whenever it
returns, with or without the safe state, it leaves the invariant with the
plan it returns. -/
theorem epilogue_core_keeps_all [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n p c e nx : U32} {hop : Nat} {safe : Bool}
    (hok : okK k) (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (hn : (UScalar.cast .U16 (n &&& 511#u32)).val = hop)
    (h : arch.cm4.entry.epilogue_core ki ai ti k hw tables s n
      = ok ((((p, c, e, nx), safe), k', hw', s'))) :
    okK k' ∧ AI kv k' s' nx.val := by
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
    | exact epi_idle (ki := ki) hok hai hhop (by assumption) (by assumption) (by assumption) (by assumption)
        rfl rfl rfl
    | exact epi_resume (ki := ki) hok hai hhop (by assumption) (by assumption) (by assumption)
        (by assumption) (by assumption) (by assumption) rfl rfl rfl
    | exact epi_next (ki := ki) hok hai hhop (by assumption) (by assumption) (by assumption) (by assumption)
        (by assumption) (Or.inl rfl) (by assumption) (by assumption) rfl rfl rfl
    | exact epi_next (ki := ki) hok hai hhop (by assumption) (by assumption) (by assumption) (by assumption)
        (by assumption) (Or.inr ⟨_, rfl, by assumption⟩) (by assumption) (by assumption)
        rfl rfl rfl
    | exact epi_killed' (ki := ki) hok hai hhop (by assumption) (by assumption) (by assumption)
        (by assumption) (by assumption) (by assumption) (by assumption) (by assumption)
        rfl rfl rfl (by assumption) (by assumption) (by assumption)
    | (exfalso; simp_all; done)

end P18
