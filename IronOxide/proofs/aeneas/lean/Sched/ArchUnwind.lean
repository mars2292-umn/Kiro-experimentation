import Sched.ArchKeep
open Aeneas Aeneas.Std Result

/-! # The unwind core keeps the invariant (2026-11-19)

`epilogue_core`, the Rust half of `rsk_unwind`, removes the topmost level
record and decides what follows. Called with the invariant in the form its
plan left (`AI kv k s hop`, `hop` the plan, not 0), whenever it returns
without the safe state it leaves the invariant with the next plan it
returns: 0 when the idle activity or the executing Job resumes, the
exception number of the next record when one more level is to be unwound
(`epilogue_core_keeps`). Verus: the `ensures` of `epilogue_core` on
`arch_inv`. -/

namespace P18
open rsk_kernel

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

/-- The topmost record with its `active` flag cleared. -/
abbrev offRec (rc : arch.cm4.plan.LevelRecord) : arch.cm4.plan.LevelRecord :=
  { rc with active := false }

/-- The state `epilogue_core` reaches once it has removed the topmost
record is the `Popped` state of `Sched/ArchInv.lean`. -/
theorem popped_set [KSpec ki kv okK] {k : K} {s : arch.cm4.plan.ArchState} {hop : Nat}
    {level : Usize} {rc x : arch.cm4.plan.LevelRecord}
    {back : arch.cm4.plan.LevelRecord → Array arch.cm4.plan.LevelRecord 8#usize}
    (hok : okK k) (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (htl : arch.cm4.plan.top_active s.levels = ok level)
    (hrc : s.levels.index_usize level = ok rc)
    (hmut : s.levels.index_mut_usize level = ok (x, back)) :
    Arch.Popped (kv k) (lvOf s.levels) (lvOf (back (offRec rc))) s.current_partition.val level.val ∧
    (lvOf s.levels).prevTask level.val = rc.previous_task.val ∧
    (lvOf s.levels).prevPart level.val = rc.previous_partition.val := by
  obtain ⟨_, rfl⟩ := index_ok hrc
  obtain ⟨hl, rfl⟩ := index_mut_ok hmut
  obtain ⟨hex, _⟩ := hai.2.1 hhop
  have htop : Arch.IsTop (lvOf s.levels) level.val := by
    obtain ⟨l0, ha0, hl0⟩ := hex
    exact top_active_isTop htl ha0 (by omega)
  refine ⟨⟨KSpec.kok (ki := ki) (kv := kv) hok, hai.1, hex, htop, fun l hne => ?_, ?_⟩, rfl, rfl⟩
  · refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> simp only [lvOf] <;> rw [get_set _ _ _ _ hl, if_neg hne]
  · simp only [lvOf]
    rw [get_set _ _ _ _ hl, if_pos rfl]
    simp

/-- The invariant of a state whose three fields are the popped ones. -/
theorem ai_of {k' : K} {s' : arch.cm4.plan.ArchState}
    {lv' : Array arch.cm4.plan.LevelRecord 8#usize} {pp : U8} {hop' : Nat}
    (h : Arch.AInv (kv k') (lvOf lv') pp.val 0 hop') (e1 : s'.levels = lv')
    (e2 : s'.current_partition = pp) (e3 : s'.pending_unwind = 0#u32) : AI kv k' s' hop' := by
  unfold AI
  rw [e1, e2, e3]
  exact h

theorem bne_true {a b : U8} (h : (a != b) = true) : a.val ≠ b.val := by
  intro e
  have : a = b := by scalar_tac
  simp [this] at h

theorem not_bne {a b : U8} (h : ¬ (a != b) = true) : a = b := by
  by_contra hne
  exact h (bne_iff_ne.mpr hne)

/-- The Kernel's `top` and `depth` cannot disagree. -/
theorem top_depth0 [KSpec ki kv okK] {k : K} {d : Usize} {v : U8} (hok : okK k)
    (h0 : ¬ (d != 0#usize) = true) (hd : ki.depth k = ok d) (htop : ki.top k = ok (some v)) :
    False := by
  have hdv := KSpec.depth (kv := kv) (okK := okK) hd
  have hz : d = 0#usize := by
    by_contra hne
    exact h0 (bne_iff_ne.mpr hne)
  have := (KSpec.top (kv := kv) hok htop).1
  have h1 : (kv k).depth = 0 := by rw [← hdv, hz]; rfl
  exact absurd (this.mpr h1) (by simp)

/-- The idle activity resumes. -/
theorem epi_idle [KSpec ki kv okK] {k : K} {s s' : arch.cm4.plan.ArchState} {hop : Nat}
    {level : Usize} {rc x : arch.cm4.plan.LevelRecord}
    {back : arch.cm4.plan.LevelRecord → Array arch.cm4.plan.LevelRecord 8#usize}
    (hok : okK k) (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (htl : arch.cm4.plan.top_active s.levels = ok level)
    (hrc : s.levels.index_usize level = ok rc)
    (hmut : s.levels.index_mut_usize level = ok (x, back))
    (h255 : rc.previous_task = 255#u8)
    (e1 : s'.levels = back (offRec rc)) (e2 : s'.current_partition = rc.previous_partition)
    (e3 : s'.pending_unwind = 0#u32) : okK k ∧ AI kv k s' (0#u32).val := by
  obtain ⟨p, hpt, hpp⟩ := popped_set (ki := ki) hok hai hhop htl hrc hmut
  have := p.idle (by rw [hpt, h255]; rfl)
  rw [hpp] at this
  exact ⟨hok, ai_of this e1 e2 e3⟩

/-- The executing Job resumes. -/
theorem epi_resume [KSpec ki kv okK] {k : K} {s s' : arch.cm4.plan.ArchState} {hop : Nat}
    {level : Usize} {rc x : arch.cm4.plan.LevelRecord} {v : U8}
    {back : arch.cm4.plan.LevelRecord → Array arch.cm4.plan.LevelRecord 8#usize}
    (hok : okK k) (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (htl : arch.cm4.plan.top_active s.levels = ok level)
    (hrc : s.levels.index_usize level = ok rc)
    (hmut : s.levels.index_mut_usize level = ok (x, back))
    (hne : ¬ rc.previous_task = 255#u8) (hv : ¬ (v != rc.previous_task) = true)
    (htop : ki.top k = ok (some v))
    (e1 : s'.levels = back (offRec rc)) (e2 : s'.current_partition = rc.previous_partition)
    (e3 : s'.pending_unwind = 0#u32) : okK k ∧ AI kv k s' (0#u32).val := by
  obtain ⟨p, hpt, hpp⟩ := popped_set (ki := ki) hok hai hhop htl hrc hmut
  obtain ⟨t1, t2⟩ := KSpec.top (kv := kv) hok htop
  have hd : 0 < (kv k).depth := by
    by_contra hz
    exact absurd (t1.mpr (by omega)) (by simp)
  have hve := not_bne hv
  have hn255 : (lvOf s.levels).prevTask level.val ≠ 255 := by
    rw [hpt]; intro e; apply hne; scalar_tac
  have := p.resume hn255 hd (by rw [hpt, ← t2 v rfl, hve])
  rw [hpp] at this
  exact ⟨hok, ai_of this e1 e2 e3⟩

/-- One more level to unwind: the record below is not the executing Job's. -/
theorem epi_next [KSpec ki kv okK] {k : K} {s s' : arch.cm4.plan.ArchState} {hop : Nat}
    {level : Usize} {rc x lr : arch.cm4.plan.LevelRecord} {o : Option U8}
    {ji : arch.bound.JobInfo}
    {back : arch.cm4.plan.LevelRecord → Array arch.cm4.plan.LevelRecord 8#usize}
    (hok : okK k) (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (htl : arch.cm4.plan.top_active s.levels = ok level)
    (hrc : s.levels.index_usize level = ok rc)
    (hmut : s.levels.index_mut_usize level = ok (x, back))
    (hne : (rc.previous_task != 255#u8) = true) (htop : ki.top k = ok o)
    (hnot : o = none ∨ ∃ v, o = some v ∧ (v != rc.previous_task) = true)
    (hji : ki.job_info k rc.previous_task = ok ji)
    (hlr : (back (offRec rc)).index_usize (UScalar.cast .Usize ji.priority) = ok lr)
    (e1 : s'.levels = back (offRec rc)) (e2 : s'.current_partition = rc.previous_partition)
    (e3 : s'.pending_unwind = 0#u32) :
    okK k ∧ AI kv k s' (UScalar.cast .U32 lr.exception_number).val := by
  obtain ⟨p, hpt, hpp⟩ := popped_set (ki := ki) hok hai hhop htl hrc hmut
  obtain ⟨t1, t2⟩ := KSpec.top (kv := kv) hok htop
  have hn255 : (lvOf s.levels).prevTask level.val ≠ 255 := by
    rw [hpt]; exact bne_true hne
  have hcase : (kv k).depth = 0 ∨
      (kv k).stack ((kv k).depth - 1) ≠ (lvOf s.levels).prevTask level.val := by
    rcases hnot with h0 | ⟨v, hv, hvn⟩
    · exact Or.inl (t1.mp h0)
    · right
      rw [hpt, ← t2 v hv]
      exact bne_true hvn
  have := p.next hn255 hcase
  rw [hpp, hpt] at this
  obtain ⟨j1, j2⟩ := KSpec.job_info (kv := kv) hok hji
  obtain ⟨_, rfl⟩ := index_ok hlr
  refine ⟨hok, ai_of ?_ e1 e2 e3⟩
  have he : (UScalar.cast UScalarTy.U32
      ((back (offRec rc)).val[(UScalar.cast UScalarTy.Usize ji.priority).val]!).exception_number).val
      = (lvOf (back (offRec rc))).exc ((kv k).prio rc.previous_task.val) := by
    simp only [lvOf]
    rw [← j1]
    scalar_tac
  rw [he]
  exact this

/-- The Job to resume was killed: the Kernel ends it and one more level is
to be unwound. -/
theorem epi_killed [KSpec ki kv okK] {k k1 k' : K} {hw2 hw' : A} {tables : T}
    {s s1 s' : arch.cm4.plan.ArchState} {hop : Nat}
    {level : Usize} {rc x lr : arch.cm4.plan.LevelRecord} {v1 v : U8} {ji : arch.bound.JobInfo}
    {back : arch.cm4.plan.LevelRecord → Array arch.cm4.plan.LevelRecord 8#usize}
    (hok : okK k) (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (htl : arch.cm4.plan.top_active s.levels = ok level)
    (hrc : s.levels.index_usize level = ok rc)
    (hmut : s.levels.index_mut_usize level = ok (x, back))
    (hne : ¬ rc.previous_task = 255#u8) (hv1 : ¬ (v1 != rc.previous_task) = true)
    (htop1 : ki.top k = ok (some v1))
    (hjr : ki.job_return k 0#u32 = ok k1)
    (hap : arch.cm4.entry.apply_core ki ai ti k1 hw2 tables s1 = ok (false, k', hw', s'))
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

set_option maxHeartbeats 64000000 in
/-- **`epilogue_core` keeps the invariant.** Called with the invariant in
the form its plan left, whenever it returns without the safe state it
leaves the invariant with the plan it returns. -/
theorem epilogue_core_keeps [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n p c e nx : U32} {hop : Nat}
    (hok : okK k) (hai : AI kv k s hop) (hhop : hop ≠ 0)
    (h : arch.cm4.entry.epilogue_core ki ai ti k hw tables s n
      = ok ((((p, c, e, nx), false), k', hw', s'))) :
    okK k' ∧ AI kv k' s' nx.val := by
  unfold arch.cm4.entry.epilogue_core at h
  peel h
  all_goals first
    | (exfalso; exact top_depth0 (ki := ki) (kv := kv) hok (by assumption) (by assumption) (by assumption))
    | exact epi_idle (ki := ki) hok hai hhop (by assumption) (by assumption) (by assumption) (by assumption)
        rfl rfl rfl
    | exact epi_resume (ki := ki) hok hai hhop (by assumption) (by assumption) (by assumption)
        (by assumption) (by assumption) (by assumption) rfl rfl rfl
    | exact epi_next (ki := ki) hok hai hhop (by assumption) (by assumption) (by assumption) (by assumption)
        (by assumption) (Or.inl rfl) (by assumption) (by assumption) rfl rfl rfl
    | exact epi_next (ki := ki) hok hai hhop (by assumption) (by assumption) (by assumption) (by assumption)
        (by assumption) (Or.inr ⟨_, rfl, by assumption⟩) (by assumption) (by assumption)
        rfl rfl rfl
    | exact epi_killed (ki := ki) hok hai hhop (by assumption) (by assumption) (by assumption)
        (by assumption) (by assumption) (by assumption) (by assumption) (by assumption)
        rfl rfl rfl (by assumption) (by assumption) (by assumption)
    | (exfalso; simp_all; done)

end P18
