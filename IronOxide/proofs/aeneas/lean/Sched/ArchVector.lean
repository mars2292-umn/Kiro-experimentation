import Sched.ArchSvcKeep
import Sched.ArchDispatch
open Aeneas Aeneas.Std Result

/-! # The dispatch-vector core keeps the invariant (2026-11-20)

`dispatch_vector_core` is what a dispatch stub calls at its entry
instruction. It hands an owed unwind back to the stub (`pend_take`), defers
a vector that fired inside another stub's window, declines a stale pend, or
dispatches (`dispatch_job_core`, every exit of which is covered here, the
refused ones included). -/

namespace P18
open rsk_kernel

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

/-- A dispatch the Kernel declined leaves the stack the same Jobs. -/
theorem dispatch_same_ke [KSpec ki kv okK] {k k1 : K} {t : U8} {pre : U32} {db d1 : Usize}
    (hdisp : ki.dispatch k t pre = ok k1) (hd1 : ki.depth k1 = ok d1)
    (hdb : ki.depth k = ok db) (he : d1 = db) : KE kv okK k1 k := by
  intro hok
  obtain ⟨hok1, hpr, hpa, hst, _⟩ := KSpec.dispatch (kv := kv) hok hdisp
  have hdbv := KSpec.depth (kv := kv) (okK := okK) hdb
  have hd1v := KSpec.depth (kv := kv) (okK := okK) hd1
  have hde : (kv k1).depth = (kv k).depth := by rw [← hdbv, ← hd1v, he]
  exact ⟨hok1, ⟨by omega, fun i hi => hst i (by omega), hpr, hpa⟩, hde⟩

macro_rules | `(tactic| ke_call) => `(tactic|
  refine KE.trans ?_ (dispatch_same_ke (by assumption) (by assumption) (by assumption)
    (by assumption)))

theorem AI.frame {k : K} {s s' : arch.cm4.plan.ArchState} {hop : Nat} (hfr : Frame s' s)
    (h : AI kv k s hop) : AI kv k s' hop := by
  unfold AI at h ⊢
  rw [hfr.1, hfr.2.1, hfr.2.2]
  exact h

set_option maxHeartbeats 16000000 in
/-- **Every exit of `dispatch_job_core` keeps the invariant**, with nothing
planned: a dispatch writes the record of the Job it started, a refused one
(no room for the frame, a Partition out of range, a Task the Kernel did not
start) is a Kernel fault that ends no Job. -/
theorem dispatch_job_core_all [KSpecD ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (hok : okK k) (hai : AI kv k s 0) (hp0 : s.pending_unwind = 0#u32)
    (hn0 : n.val ≠ 0) (hn1 : n.val ≤ 511)
    (h : arch.cm4.entry.dispatch_job_core ki ai ti k hw tables s n t psp control exc_return
      = ok ((plan, safe), k', hw', s')) :
    okK k' ∧ AI kv k' s' plan.hop.val := by
  unfold arch.cm4.entry.dispatch_job_core at h
  peel h
  all_goals first
    | exact disp_push (ki := ki) (ai := ai) hok hai hp0 hn0 hn1 (by assumption) (by assumption)
        (by assumption) (by assumption) (by assumption) (by assumption) (by assumption)
        (by assumption) (by rfl) (by rfl) (by rfl)
    | (refine kentry0' (ki := ki) ?_ ?_ hok hai
       · ke_close; done
       · frame_close)

theorem dvc_loop1_frame {n : U16} {slot slot' i : Usize} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.dispatch_vector_core_loop1 s n slot i = ok (s', slot')) : Frame s' s := by
  unfold arch.cm4.entry.dispatch_vector_core_loop1 at h
  refine loop_inv _ (fun x => Frame x.1 s) (fun r => Frame r.1 s) ?_ ?_ _ (s', slot')
    (Frame.refl s) h
  · rintro ⟨st, c, d⟩ ⟨st', c', d'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.dispatch_vector_core_loop1.body at hb
    peel hb
    all_goals frame_close
  · rintro ⟨st, c, d⟩ ⟨st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.dispatch_vector_core_loop1.body at hb
    peel hb
    all_goals frame_close

frame_reg dvc_loop1_frame

theorem dvc_loop2_frame {n : U16} {slot slot' i : Usize} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.dispatch_vector_core_loop2 s n slot i = ok (s', slot')) : Frame s' s := by
  unfold arch.cm4.entry.dispatch_vector_core_loop2 at h
  refine loop_inv _ (fun x => Frame x.1 s) (fun r => Frame r.1 s) ?_ ?_ _ (s', slot')
    (Frame.refl s) h
  · rintro ⟨st, c, d⟩ ⟨st', c', d'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.dispatch_vector_core_loop2.body at hb
    peel hb
    all_goals frame_close
  · rintro ⟨st, c, d⟩ ⟨st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.dispatch_vector_core_loop2.body at hb
    peel hb
    all_goals frame_close

frame_reg dvc_loop2_frame

theorem not_bne32 {a b : U32} (h : ¬ (a != b) = true) : a = b := by
  by_contra hne
  exact h (bne_iff_ne.mpr hne)

theorem bne32_val {a : U32} (h : (a != 0#u32) = true) : a.val ≠ 0 := by
  intro e
  have : a = 0#u32 := by scalar_tac
  simp [this] at h

theorem u16_ne0 {n : U16} (h : ¬ n = 0#u16) : n.val ≠ 0 := by
  intro e
  exact h (by scalar_tac)

/-- The stub at its entry instruction is handed the unwind an interrupt
left owed: the plan is that unwind, and nothing is owed any more. -/
theorem pend_take {k : K} {s s' : arch.cm4.plan.ArchState} (hok : okK k) (hai : AI kv k s 0)
    (hne : (s.pending_unwind != 0#u32) = true) (e1 : s'.levels = s.levels)
    (e2 : s'.current_partition = s.current_partition) (e3 : s'.pending_unwind = 0#u32) :
    okK k ∧ AI kv k s' s.pending_unwind.val := by
  refine ⟨hok, ?_⟩
  have hv := bne32_val hne
  unfold AI at hai ⊢
  obtain ⟨hc, _, _, h3⟩ := hai
  rw [e1, e2, e3]
  exact ⟨hc, fun _ => h3 hv, fun h => absurd h hv, fun h => absurd rfl h⟩

set_option maxHeartbeats 32000000 in
/-- **`dispatch_vector_core` keeps the invariant.** Called between Kernel
entries (an unwind may be owed) for a vector `n` of the dispatch range,
whenever it returns it leaves the invariant with the plan it returns. -/
theorem dispatch_vector_core_keeps [KSpecD ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (hok : okK k) (hai : AI kv k s 0) (hn1 : n.val ≤ 511)
    (h : arch.cm4.entry.dispatch_vector_core ki ai ti k hw tables s n t psp control exc_return
      = ok ((plan, safe), k', hw', s')) :
    okK k' ∧ AI kv k' s' plan.hop.val := by
  unfold arch.cm4.entry.dispatch_vector_core at h
  simp only [arch.cm4.entry.WINDOW_DEFER, ↓reduceIte] at h
  peel h
  all_goals first
    | exact absurd (not_bne32 (by assumption)) (by assumption)
    | exact dispatch_job_core_all hok hai (not_bne32 (by assumption)) (u16_ne0 (by assumption))
        hn1 h
    | exact stack_violation_core_keeps hok hai (by assumption)
    | exact pend_take hok hai (by assumption) rfl rfl rfl
    | (refine kentry0' (ki := ki) ?_ ?_ hok hai
       · ke_close; done
       · frame_close)

end P18
