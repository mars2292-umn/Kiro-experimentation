import Sched.ArchRel
open Aeneas Aeneas.Std Result

/-! # A fault the stub attributes to the Kernel plans no unwind (2026-11-20)

`fault_core` called with `in_kernel` raises a Kernel fault, which ends no
Job, so `end_plan_core` plans nothing (Verus: `fault_core`'s
`in_kernel ==> r.0 == 0`). The nested-fault theorem of
`Sched/WorldMore.lean` rests on it. Also: a plan to unwind is the exception
number of a dispatch vector, so masking it to nine bits, as
`epilogue_core` does, changes nothing. -/

namespace P18
open rsk_kernel

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

theorem end_plan_core_zero [KSpec ki kv okK] {k k0 : K} {s : arch.cm4.plan.ArchState}
    {db : Usize} {hop : U32} {g1 : verus_builtin.Ghost K}
    {g2 : verus_builtin.Ghost arch.cm4.plan.ArchState}
    (hd : ki.depth k0 = ok db) (hde : (kv k).depth = (kv k0).depth)
    (h : arch.cm4.entry.end_plan_core ki k s db g1 g2 = ok hop) : hop = 0#u32 := by
  unfold arch.cm4.entry.end_plan_core at h
  simp only [bind_eq_ok] at h
  obtain ⟨i, hi, h⟩ := h
  have hiv := KSpec.depth (kv := kv) (okK := okK) hi
  have hdb := KSpec.depth (kv := kv) (okK := okK) hd
  split at h
  · rename_i hlt
    exfalso
    scalar_tac
  · exact (ok_inj h).symm

set_option maxHeartbeats 4000000 in
theorem fault_core_kernel_hop [KSpecD ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool} {kind : U8} (hok : okK k)
    (h : arch.cm4.entry.fault_core ki ai ti k hw tables s kind true
      = ok ((hop, safe), k', hw', s')) : hop = 0#u32 := by
  unfold arch.cm4.entry.fault_core at h
  simp only [↓reduceIte] at h
  peel h
  all_goals
    have hke := KE.trans (fault_kernel_ke (kv := kv) (okK := okK) (by assumption))
      (apply_core_ke (by assumption))
    exact end_plan_core_zero (kv := kv) (okK := okK) (by assumption) ((hke hok).2.2)
      (by assumption)

/-- The plan the stub holds is a dispatch vector's exception number: the
nine-bit mask of `epilogue_core` leaves it. -/
theorem plan_mask {k : K} {s : arch.cm4.plan.ArchState} {n : U32} (hai : AI kv k s n.val)
    (hn : n.val ≠ 0) : (UScalar.cast .U16 (n &&& 511#u32)).val = n.val := by
  obtain ⟨_, l, hl, he⟩ := hai.2.1 hn
  have hle := (hai.1.1 l hl.1).2.2.2.2.2.1
  rw [← he] at hle
  have h1 : (n &&& 511#u32).val = n.val := by
    rw [UScalar.val_and]
    have h511 : (511#u32).val = 511 := rfl
    rw [h511]
    have := Nat.and_two_pow_sub_one_eq_mod n.val 9
    simp at this
    rw [this]
    omega
  rw [UScalar.cast_val_eq, h1]
  apply Nat.mod_eq_of_lt
  simp
  omega

end P18
