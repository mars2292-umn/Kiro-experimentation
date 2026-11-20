import Sched.LayoutTotal
import Sched.Isolation
open Aeneas Aeneas.Std Result

/-! # The isolation theorems' layout hypothesis, from `validate` (2026-11-20)

`Sched/Isolation.lean` proves Property 12 and its neighbours (two
Partitions' views share nothing writable, no view grants Kernel memory or a
Kernel peripheral interval, a read-only window is not writable, a stack
overflow faults) from `LayoutOk`, the disjointness clauses of `Layout::wf`,
taken there as a hypothesis. Here `LayoutOk` follows from `LWf`, and so
from `Layout::validate` returning `true` on the extracted code. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NP : Usize}

theorem ivAt_bang {n : Usize} (a : Array logic.mpu.Interval n) (j : Nat)
    (h : j < a.val.length) : a.val[j]! = ivAt a j := by
  rw [getElem!_pos a.val j h]
  exact ivAt_get a j h

theorem plAt_bang (a : Array logic.mpu.PartitionLayout NP) (j : Nat) (h : j < a.val.length) :
    a.val[j]! = plAt a j := by
  rw [getElem!_pos a.val j h]
  exact plAt_get a j h

theorem disj_idisj {a b : logic.mpu.Interval} (h : Disj a b) : idisj a b := h

theorem partok_of_wf {l : logic.mpu.Layout NP} {p : Nat} (hk : l.kernel_window_count.val ≤ 8)
    (h : PartWf l p) : PartOk l (plAt l.partitions p) := by
  obtain ⟨hpl, hck, hrk, hwk, ⟨hca, hra⟩, hwa⟩ := h
  obtain ⟨_, _, _, _, _, hwc, _, hcr, hwrc, hnb, hww⟩ := hpl
  have hwl : (plAt l.partitions p).windows.val.length = 5 := (plAt l.partitions p).windows.property
  have hkl : l.kernel_windows.val.length = 8 := l.kernel_windows.property
  have ew : ∀ w, w < (plAt l.partitions p).window_count.val →
      (plAt l.partitions p).windows.val[w]! = ivAt (plAt l.partitions p).windows w :=
    fun w hlt => ivAt_bang _ w (by omega)
  have ek : ∀ k, k < l.kernel_window_count.val →
      l.kernel_windows.val[k]! = ivAt l.kernel_windows k := fun k hlt => ivAt_bang _ k (by omega)
  refine ⟨disj_idisj hcr, fun w hlt => ?_, fun w hlt => ?_, fun v w hvw hlt => ?_,
    fun w a hlt ha => ?_, ⟨disj_idisj hck.1, disj_idisj hck.2.1, disj_idisj hck.2.2⟩,
    ⟨disj_idisj hrk.1, disj_idisj hrk.2.1, disj_idisj hrk.2.2⟩, fun w hlt => ?_,
    fun k hlt => ?_, fun k hlt => ?_, fun w k hlt hkt => ?_⟩
  · rw [ew w hlt]; exact disj_idisj (hwrc w hlt).1
  · rw [ew w hlt]; exact disj_idisj (hwrc w hlt).2
  · rw [ew v (by omega), ew w hlt]; exact disj_idisj (hww v w hvw hlt)
  · rw [ew w hlt]
    have hn := hnb w hlt
    unfold NoBelow at hn
    unfold icontains isz
    simp only [iEnd] at hn
    intro hc
    apply hn
    constructor
    · omega
    · push_cast
      omega
  · rw [ew w hlt]
    exact ⟨disj_idisj (hwk w hlt).1, disj_idisj (hwk w hlt).2.1, disj_idisj (hwk w hlt).2.2⟩
  · rw [ek k hlt]; exact disj_idisj (hca k hlt)
  · rw [ek k hlt]; exact disj_idisj (hra k hlt)
  · rw [ew w hlt, ek k hkt]; exact disj_idisj (hwa w hlt k hkt)

theorem pairok_of_wf {l : logic.mpu.Layout NP} {p q : Nat}
    (hp : (plAt l.partitions p).window_count.val ≤ 5)
    (hq : (plAt l.partitions q).window_count.val ≤ 5) (h : PairWf l p q) :
    PairOk (plAt l.partitions p) (plAt l.partitions q) := by
  obtain ⟨⟨d1, d2, d3, d4⟩, ha, hb, hw⟩ := h
  have hpl : (plAt l.partitions p).windows.val.length = 5 := (plAt l.partitions p).windows.property
  have hql : (plAt l.partitions q).windows.val.length = 5 := (plAt l.partitions q).windows.property
  refine ⟨disj_idisj d1, disj_idisj d2, disj_idisj d3, disj_idisj d4, fun w hlt => ?_,
    fun w hlt => ?_, fun v w hv hw' => ?_⟩
  · rw [ivAt_bang _ w (by omega)]; exact ⟨disj_idisj (ha w hlt).1, disj_idisj (ha w hlt).2⟩
  · rw [ivAt_bang _ w (by omega)]; exact ⟨disj_idisj (hb w hlt).1, disj_idisj (hb w hlt).2⟩
  · rw [ivAt_bang _ v (by omega), ivAt_bang _ w (by omega)]; exact disj_idisj (hw v w hv hw')

/-- **`Layout::wf` gives the hypothesis of the isolation theorems.** -/
theorem layoutok_of_wf {l : logic.mpu.Layout NP} (h : LWf l) : LayoutOk l := by
  obtain ⟨_, ⟨s1, s2⟩, _, hk, _, hsa, hpart, hpair⟩ := h
  have hkl : l.kernel_windows.val.length = 8 := l.kernel_windows.property
  have hpl : l.partitions.val.length = NP.val := l.partitions.property
  refine ⟨⟨disj_idisj s1, disj_idisj s2⟩, fun k hlt => ?_, fun p hp => ?_, fun p q hp hq hne => ?_⟩
  · rw [ivAt_bang _ k (by omega)]; exact disj_idisj (hsa k hlt)
  · rw [plAt_bang _ p (by omega)]; exact partok_of_wf hk (hpart p hp)
  · rw [plAt_bang _ p (by omega), plAt_bang _ q (by omega)]
    exact pairok_of_wf (hpart p hp).1.2.2.2.2.2.1 (hpart q hq).1.2.2.2.2.2.1 (hpair p q hp hq hne)

/-- **A layout `Layout::validate` accepts satisfies `LayoutOk`**: the
isolation theorems of `Sched/Isolation.lean` hold of every layout the boot
validated. -/
theorem validate_layoutok {l : logic.mpu.Layout NP}
    (h : logic.mpu.Layout.validate l = ok true) : LayoutOk l := by
  obtain ⟨r, hr, hp⟩ := spec_returns (layout_validate_wf l)
  rw [h] at hr
  have hr := ok_inj hr
  subst hr
  exact layoutok_of_wf (hp rfl)

end P18
