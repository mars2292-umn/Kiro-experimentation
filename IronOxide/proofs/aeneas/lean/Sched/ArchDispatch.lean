import Sched.ArchKeep
open Aeneas Aeneas.Std Result

/-! # The dispatch core keeps the invariant (2026-11-19)

`dispatch_job_core` starts a Job in the Kernel and writes its level record.
Called with the invariant and nothing owed, whenever it returns a plan with
a frame (a dispatch), the invariant holds again with nothing planned: the
record written is the new topmost one, naming the Job that was executing
and its Partition as preempted (`dispatch_job_core_keeps`). Verus: the
`ensures` of `dispatch_job_core` on `arch_inv`, by `lemma_push`. -/

namespace P18
open rsk_kernel

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

/-- The Task a dispatch records as preempted: the executing Job's, 0xFF
when no Job is in progress. -/
def prevOf : Option U8 → U8
  | none => 255#u8
  | some u => u

/-- The write of a level record by a dispatch that started a Job. -/
theorem disp_push [KSpec ki kv okK] {k k1 : K} {hw : A} {s s1 s' : arch.cm4.plan.ArchState}
    {pre : U32} {o : Option U8} {info : arch.bound.JobInfo} {db d1 : Usize} {t : U8} {n : U16}
    {psp control exc_return floor_before : U32}
    {a1 : Array arch.cm4.plan.LevelRecord 8#usize}
    (hok : okK k) (hai : AI kv k s 0) (hp0 : s.pending_unwind = 0#u32)
    (hn0 : n.val ≠ 0) (hn1 : n.val ≤ 511)
    (hcp : arch.cm4.entry.charge_point ai hw s = ok (pre, s1))
    (htop : ki.top k = ok o) (hji : ki.job_info k t = ok info)
    (hdb : ki.depth k = ok db) (hdisp : ki.dispatch k t pre = ok k1)
    (hd1 : ki.depth k1 = ok d1) (hne : ¬ d1 = db)
    (hupd : Array.update s1.levels (UScalar.cast .Usize info.priority)
      ({ exception_number := n, task := t, partition := info.partition, psp, control,
         exc_return, floor_before, previous_partition := s1.current_partition,
         previous_task := prevOf o, active := true } : arch.cm4.plan.LevelRecord) = ok a1)
    (e1 : s'.levels = a1) (e2 : s'.current_partition = info.partition)
    (e3 : s'.pending_unwind = s1.pending_unwind) :
    okK k1 ∧ AI kv k1 s' 0 := by
  have hfr := charge_point_frame hcp
  obtain ⟨hok1, hpr, hpa, hst, hdep⟩ := KSpec.dispatch (kv := kv) hok hdisp
  have hdbv := KSpec.depth (kv := kv) (okK := okK) hdb
  have hd1v := KSpec.depth (kv := kv) (okK := okK) hd1
  have hdd : (kv k1).depth = (kv k).depth + 1 ∧ (kv k1).stack (kv k).depth = t.val := by
    rcases hdep with h | h
    · exfalso; apply hne; scalar_tac
    · exact h
  obtain ⟨j1, j2⟩ := KSpec.job_info (kv := kv) hok hji
  obtain ⟨t1, t2⟩ := KSpec.top (kv := kv) hok htop
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
  have hpush := Arch.ChainOk.push (L' := lvOf s'.levels) (level := (kv k1).prio t.val) hk1 hc hnex
    hdd.1 hst hpr hpa hdd.2 rfl
    (fun l hne' => by
      refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> simp only [lvOf] <;>
        rw [e1, get_set _ _ _ _ hl, if_neg (by rw [hlv]; exact hne'), hfr.1])
    (by simp only [lvOf]; rw [e1, ← hlv, get_set _ _ _ _ hl, if_pos rfl]
        simp; scalar_tac)
    (by simp only [lvOf]; rw [e1, ← hlv, get_set _ _ _ _ hl, if_pos rfl])
    (by
      simp only [lvOf]; rw [e1, ← hlv, get_set _ _ _ _ hl, if_pos rfl]
      cases o with
      | none =>
        have : (kv k).depth = 0 := t1.mp rfl
        simp [prevOf, this]
      | some u =>
        have hz : (kv k).depth ≠ 0 := fun hz => absurd (t1.mpr hz) (by simp)
        simp only [prevOf, hz, ite_false]
        exact t2 u rfl)
    (by simp only [lvOf]; rw [e1, ← hlv, get_set _ _ _ _ hl, if_pos rfl]; exact hn0)
    (by simp only [lvOf]; rw [e1, ← hlv, get_set _ _ _ _ hl, if_pos rfl]; exact hn1)
    (by simp only [lvOf]; rw [e1, ← hlv, get_set _ _ _ _ hl, if_pos rfl, hpa]; exact j2)
    (by simp only [lvOf]; rw [e1, ← hlv, get_set _ _ _ _ hl, if_pos rfl, hfr.2.1])
  obtain ⟨hc', hnex'⟩ := hpush
  refine ⟨hok1, ?_⟩
  unfold AI
  have e2' : s'.current_partition.val = (kv k1).part t.val := by rw [e2, hpa]; exact j2
  have e3' : s'.pending_unwind.val = 0 := by rw [e3, hfr.2.2, hp0]; rfl
  rw [e2', e3']
  exact ⟨hc', fun h => absurd rfl h, fun _ => ⟨fun h => absurd h hnex', fun h => absurd rfl h⟩,
    fun h => absurd rfl h⟩

set_option maxHeartbeats 16000000 in
/-- **`dispatch_job_core` keeps the invariant.** -/
theorem dispatch_job_core_keeps [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (hok : okK k) (hai : AI kv k s 0) (hp0 : s.pending_unwind = 0#u32)
    (hn0 : n.val ≠ 0) (hn1 : n.val ≤ 511)
    (h : arch.cm4.entry.dispatch_job_core ki ai ti k hw tables s n t psp control exc_return
      = ok ((plan, safe), k', hw', s'))
    (hp : plan.psp ≠ 0#u32) :
    okK k' ∧ AI kv k' s' 0 := by
  unfold arch.cm4.entry.dispatch_job_core at h
  peel h
  all_goals first
    | exact absurd rfl hp
    | exact disp_push (ki := ki) (ai := ai) hok hai hp0 hn0 hn1 (by assumption) (by assumption)
        (by assumption) (by assumption) (by assumption) (by assumption) (by assumption)
        (by assumption) (by rfl) (by rfl) (by rfl)

end P18
