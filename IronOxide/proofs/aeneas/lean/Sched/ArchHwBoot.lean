import Sched.ArchHw
open Aeneas Aeneas.Std Result

/-! # The start's vectors, its watchdog frame, and the floating-point access (2026-11-26)

`Sched/ArchHw.lean` reads BASEPRI, PRIMASK and the watchdog's feed off the
hardware model. This file reads three more of its fields (`HV2`, stepped
by `hstep2` as `entry::step` steps `HwModel`): the floating-point access
and each interrupt line's priority and enable bit. The trait's contract
for them is a second class (`HSpec2`), of the same form as `HSpec`.

- `boot_vectors_ok`: **after `boot_vectors` every line from 16 up to the
  board's count that has a binding is as the binding says** (R19.5; Verus
  `boot_vector_ok`): a dispatch line has the priority `encode_level` gives
  its Task's priority level and is enabled; an intake line and a Kernel
  line have `encode_level 0` and are enabled; an unbound line is disabled.
- `boot_core_hq`: **`boot_core` does not feed the watchdog and does not
  touch PRIMASK** (it starts the watchdog; the feed is the time base's).
- `dispatch_job_core_fpu`: **a dispatch that starts a Job sets the
  floating-point access to that Task's declaration** (R11.7, R11.1).
- `epilogue_core_fpu`: **an unwind that resumes a Job, with no hop owed,
  leaves the floating-point access as it was or sets it to the declaration
  of the Task on top of the Kernel's stack**.

That the lines `boot_vectors` wrote are still so at `boot_core`'s return
is in `Sched/ArchHwBootRet.lean` (`boot_core_vectors`). Not here: that
they are still so after later entries, and A13 (an intake line enabled
exactly when its source is unmasked). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T}

/-- The floating-point access, and each line's priority and enable bit
(`entry::HwModel`: `fpu_access`, `priority`, `enabled`). -/
structure HV2 where
  fpu : Bool
  prio : Nat → Option Nat
  enabled : Nat → Bool

/-- The model's step for one action (`entry::step`, these fields). -/
def hstep2 (h : HV2) : arch.cm4.entry.Action → HV2
  | .FpuAccess b => { h with fpu := b }
  | .SetPriority n v => { h with prio := fun x => if x = n.val then some v.val else h.prio x }
  | .Enable n => { h with enabled := fun x => if x = n.val then true else h.enabled x }
  | .Disable n => { h with enabled := fun x => if x = n.val then false else h.enabled x }
  | _ => h

/-- The contract of the `KernelArch` trait for these fields. -/
class HSpec2 (ai : arch.cm4.entry.KernelArch A) (h2 : A → HV2) : Prop where
  apply : ∀ {hw act hw'}, ai.apply hw act = ok hw' → h2 hw' = hstep2 (h2 hw) act
  svc_clear_pending : ∀ {hw hw'}, ai.svc_clear_pending hw = ok hw' → h2 hw' = h2 hw
  stacking_fault_discard : ∀ {hw hw'}, ai.stacking_fault_discard hw = ok hw' → h2 hw' = h2 hw
  copy_in : ∀ {hw a b c x hw'}, ai.copy_in hw a b c = ok (x, hw') → h2 hw' = h2 hw
  copy_out : ∀ {hw a b c x hw'}, ai.copy_out hw a b c = ok (x, hw') → h2 hw' = h2 hw

variable {h2 : A → HV2}

/-! ## The start's vectors -/

/-- Line `v` is as its binding `b` says (`entry::boot_vector_ok`). -/
def VecOk (ki : arch.bound.KernelOps K) (k : K) (h : HV2) (v : Nat) :
    arch.cm4.entry.Binding → Prop
  | .Dispatch t => ∀ ji i, ki.job_info k t = ok ji → arch.bound.encode_level ji.priority = ok i →
      h.prio v = some i.val ∧ h.enabled v = true
  | .Unbound => h.enabled v = false
  | _ => ∀ i, arch.bound.encode_level 0#u8 = ok i → h.prio v = some i.val ∧ h.enabled v = true

/-- Every bound line from `lo` up to `hi` is as its binding says. -/
def BootVecs (ki : arch.bound.KernelOps K) (ti : arch.cm4.entry.Tables T) (k : K) (tables : T)
    (h : HV2) (lo hi : Nat) : Prop :=
  ∀ n : U16, lo ≤ n.val → n.val < hi → ∀ b, ti.binding tables n = ok b → VecOk ki k h n.val b

theorem VecOk.congr {k : K} {h h' : HV2} {v : Nat} {b : arch.cm4.entry.Binding}
    (hp : h'.prio v = h.prio v) (he : h'.enabled v = h.enabled v) (hv : VecOk ki k h v b) :
    VecOk ki k h' v b := by
  cases b <;> simp only [VecOk, hp, he] at hv ⊢ <;> exact hv

theorem u16_add_ok {a b c : U16} (h : a + b = ok c) : c.val = a.val + b.val := by
  have := UScalar.add_equiv a b
  simp_all

theorem u16_add_one {a c : U16} (h : a + 1#u16 = ok c) : c.val = a.val + 1 :=
  u16_add_ok h

theorem encode_zero {i : U8} (h : arch.bound.encode_level 0#u8 = ok i) : i.val = 0 := by
  unfold arch.bound.encode_level at h
  simp at h
  rw [← h]
  rfl

/-- One line written: the lines below it are as they were, and it is as
its binding says. -/
theorem boot_vecs_step {k : K} {tables : T} {h h' : HV2} {n : U16} {b0 : arch.cm4.entry.Binding}
    (hinv : BootVecs ki ti k tables h 16 n.val) (hb0 : ti.binding tables n = ok b0)
    (hfr : ∀ x, x ≠ n.val → h'.prio x = h.prio x ∧ h'.enabled x = h.enabled x)
    (hn : VecOk ki k h' n.val b0) : BootVecs ki ti k tables h' 16 (n.val + 1) := by
  intro v hlo hhi b hb
  by_cases e : v.val = n.val
  · have ev : v = n := by scalar_tac
    subst ev
    rw [hb0] at hb
    have eb := ok_inj hb
    rw [← eb]
    exact hn
  · exact VecOk.congr (hfr _ e).1 (hfr _ e).2 (hinv v hlo (by omega) b hb)

theorem boot_vectors_loop_ok [HSpec2 ai h2] {k : K} {hw hw' : A} {tables : T} {e n : U16}
    (h : arch.cm4.entry.boot_vectors_loop ki ai ti k hw tables e n = ok hw')
    (hinv : BootVecs ki ti k tables (h2 hw) 16 n.val) (hne : n.val ≤ e.val) :
    BootVecs ki ti k tables (h2 hw') 16 e.val := by
  unfold arch.cm4.entry.boot_vectors_loop at h
  refine loop_inv _ (fun x => BootVecs ki ti k tables (h2 x.1) 16 x.2.val ∧ x.2.val ≤ e.val)
    (fun r => BootVecs ki ti k tables (h2 r) 16 e.val) ?_ ?_ _ hw' ⟨hinv, hne⟩ h
  · rintro ⟨hw1, c⟩ ⟨hw2, c'⟩ ⟨hi, hle⟩ hb
    dsimp only at hi hle hb ⊢
    unfold arch.cm4.entry.boot_vectors_loop.body at hb
    rcases ite_cases hb with ⟨hlt, hb⟩ | ⟨_, hb⟩
    · have hlt' : c.val < e.val := by scalar_tac
      peel hb
      all_goals
        have hc := u16_add_one ‹c + 1#u16 = ok _›
        rw [hc]
        refine ⟨?_, by omega⟩
        refine boot_vecs_step hi ‹ti.binding tables c = ok _› ?_ ?_
      all_goals
        repeat rw [HSpec2.apply (h2 := h2) ‹ai.apply _ (arch.cm4.entry.Action.Enable _) = ok _›]
        repeat rw [HSpec2.apply (h2 := h2) ‹ai.apply _ (arch.cm4.entry.Action.SetPriority _ _) = ok _›]
        repeat rw [HSpec2.apply (h2 := h2) ‹ai.apply _ (arch.cm4.entry.Action.Disable _) = ok _›]
      all_goals first
        | (intro x hx; simp [hstep2, hx]; done)
        | (simp_all [VecOk, hstep2]; done)
        | (simp_all [VecOk, hstep2]
           have e0 := ‹arch.bound.encode_level _ = ok _›
           intro ji i e1 e2
           subst e1
           rw [e0] at e2
           exact congrArg UScalar.val (ok_inj e2))
        | (simp_all [VecOk, hstep2]
           intro i e
           exact encode_zero e)
    · peel hb
  · rintro ⟨hw1, c⟩ hw2 ⟨hi, hle⟩ hb
    dsimp only at hi hle hb ⊢
    unfold arch.cm4.entry.boot_vectors_loop.body at hb
    rcases ite_cases hb with ⟨hlt, hb⟩ | ⟨hge, hb⟩
    · peel hb
    · have hce : c.val = e.val := by scalar_tac
      peel hb
      all_goals (rw [← hce]; exact hi)

/-- **The start's vectors** (`boot_vectors`; R19.5). -/
theorem boot_vectors_ok [HSpec2 ai h2] {k : K} {hw hw' : A} {tables : T} {irqs : U16}
    (h : arch.cm4.entry.boot_vectors ki ai ti k hw tables irqs = ok hw') :
    BootVecs ki ti k tables (h2 hw') 16 (16 + irqs.val) := by
  unfold arch.cm4.entry.boot_vectors at h
  peel h
  have hev := u16_add_ok ‹16#u16 + irqs = ok _›
  have h16 : (16#u16).val = 16 := rfl
  rw [h16] at hev
  rw [← hev]
  refine boot_vectors_loop_ok (by assumption) ?_ ?_
  · intro v hlo hhi
    rw [h16] at hhi
    omega
  · rw [hev, h16]
    omega

/-! ## `boot_core` and the watchdog -/

variable {hv : A → HV}

theorem boot_vectors_loop_hq [HSpec ai hv] {k : K} {hw hw' : A} {tables : T} {e n : U16}
    (h : arch.cm4.entry.boot_vectors_loop ki ai ti k hw tables e n = ok hw') : HQ hv hw' hw := by
  unfold arch.cm4.entry.boot_vectors_loop at h
  refine loop_inv _ (fun x => HQ hv x.1 hw) (fun r => HQ hv r hw) ?_ ?_ _ hw' (HQ.refl _ _) h
  · rintro ⟨hw1, c⟩ ⟨hw2, c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.boot_vectors_loop.body at hb
    peel hb
    hq_core hb
  · rintro ⟨hw1, c⟩ hw2 hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.boot_vectors_loop.body at hb
    peel hb
    hq_core hb

hq_reg boot_vectors_loop_hq

theorem boot_vectors_hq [HSpec ai hv] {k : K} {hw hw' : A} {tables : T} {irqs : U16}
    (h : arch.cm4.entry.boot_vectors ki ai ti k hw tables irqs = ok hw') : HQ hv hw' hw := by
  unfold arch.cm4.entry.boot_vectors at h
  peel h
  hq_core h

hq_reg boot_vectors_hq

theorem dma_boot_loop_hq [HSpec ai hv] {hw hw' : A} {tables : T} {n i : Usize}
    (h : arch.cm4.entry.dma_boot_loop ai ti hw tables n i = ok hw') : HQ hv hw' hw := by
  unfold arch.cm4.entry.dma_boot_loop at h
  refine loop_inv _ (fun x => HQ hv x.1 hw) (fun r => HQ hv r hw) ?_ ?_ _ hw' (HQ.refl _ _) h
  · rintro ⟨hw1, c⟩ ⟨hw2, c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.dma_boot_loop.body at hb
    peel hb
    hq_core hb
  · rintro ⟨hw1, c⟩ hw2 hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.dma_boot_loop.body at hb
    peel hb
    hq_core hb

hq_reg dma_boot_loop_hq

theorem dma_boot_hq [HSpec ai hv] {k : K} {hw hw' : A} {tables : T}
    (h : arch.cm4.entry.dma_boot ki ai ti k hw tables = ok hw') : HQ hv hw' hw := by
  unfold arch.cm4.entry.dma_boot at h
  peel h
  hq_core h

hq_reg dma_boot_hq

set_option maxHeartbeats 8000000 in
/-- **`boot_core` does not feed the watchdog and does not touch PRIMASK.** -/
theorem boot_core_hq [HSpec ai hv] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {b : Bool}
    (h : arch.cm4.entry.boot_core ki ai ti k hw tables s = ok (b, k', hw', s')) : HQ hv hw' hw := by
  unfold arch.cm4.entry.boot_core at h
  peel h
  hq_core h

/-! ## The floating-point access -/

/-- From `hw` to `hw'` the floating-point access is as it was. -/
def HF (h2 : A → HV2) (hw' hw : A) : Prop := (h2 hw').fpu = (h2 hw).fpu

theorem HF.refl (h2 : A → HV2) (hw : A) : HF h2 hw hw := rfl

theorem HF.trans {a b c : A} (h1 : HF h2 b a) (hh : HF h2 c b) : HF h2 c a :=
  Eq.trans hh h1

/-- An action that does not set the floating-point access. -/
def NoFpu : arch.cm4.entry.Action → Prop
  | .FpuAccess _ => False
  | _ => True

theorem HF.apply [HSpec2 ai h2] {hw hw' : A} {act : arch.cm4.entry.Action}
    (h : ai.apply hw act = ok hw') (hq : NoFpu act) : HF h2 hw' hw := by
  have e := HSpec2.apply (h2 := h2) h
  unfold HF
  rw [e]
  cases act <;> first
    | rfl
    | exact absurd hq (by simp [NoFpu])

syntax "hf_call" : tactic
macro_rules | `(tactic| hf_call) => `(tactic| fail "no call")

macro "hf_reg " l:ident : command =>
  `(macro_rules | `(tactic| hf_call) => `(tactic| refine HF.trans ?_ ($l (by assumption))))

macro_rules
  | `(tactic| hf_call) => `(tactic| refine HF.trans ?_ (HF.apply (by assumption) (by exact True.intro)))

syntax "hf_close" : tactic
macro_rules
  | `(tactic| hf_close) => `(tactic| repeat (first
      | exact HF.refl _ _
      | assumption
      | hf_call
      | dsimp only))

theorem arm_compare_hf [HSpec2 ai h2] {hw hw' : A} {at_ : U64}
    (h : arch.cm4.entry.arm_compare ai hw at_ = ok hw') : HF h2 hw' hw := by
  unfold arch.cm4.entry.arm_compare at h
  peel h
  all_goals (hf_close; done)

hf_reg arm_compare_hf

/-- The access set by an action and kept since. -/
theorem fpu_set [HSpec2 ai h2] {h0 h1 hw' : A} {b : Bool}
    (hap : ai.apply h0 (arch.cm4.entry.Action.FpuAccess b) = ok h1) (hf : HF h2 hw' h1) :
    (h2 hw').fpu = b := by
  unfold HF at hf
  rw [hf, HSpec2.apply (h2 := h2) hap]
  rfl

set_option maxHeartbeats 16000000 in
/-- **A dispatch that starts a Job sets the floating-point access to its
Task's declaration** (`dispatch_job_core`; R11.7, R11.1). A dispatch that
starts no Job returns the frame address 0. -/
theorem dispatch_job_core_fpu [HSpec2 ai h2] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (h : arch.cm4.entry.dispatch_job_core ki ai ti k hw tables s n t psp control exc_return
      = ok ((plan, safe), k', hw', s')) :
    plan.psp ≠ 0#u32 → ∃ info, ki.job_info k t = ok info ∧ (h2 hw').fpu = info.fpu := by
  unfold arch.cm4.entry.dispatch_job_core at h
  peel h
  all_goals first
    | (intro hp; exact absurd rfl hp)
    | (intro _
       refine ⟨_, ‹ki.job_info k t = ok _›, fpu_set (by assumption) ?_⟩
       hf_close
       done)
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (intro hp; exact absurd rfl hp)
        | (intro _
           refine ⟨_, ‹ki.job_info k t = ok _›, fpu_set (by assumption) ?_⟩
           hf_close
           done))

/-- The access set for the Task at the top of the Kernel's stack. -/
theorem epi_fpu [HSpec2 ai h2] {k : K} {h0 h1 hw hw' : A} {ji : arch.bound.JobInfo} {tk : U8}
    {idx d : Usize}
    (hap : ai.apply h0 (arch.cm4.entry.Action.FpuAccess ji.fpu) = ok h1)
    (hji : ki.job_info k tk = ok ji) (hst : ki.stack_at k idx = ok tk) (hd : ki.depth k = ok d)
    (hidx : idx.val = d.val - 1) (hf : HF h2 hw' h1) :
    (h2 hw').fpu = (h2 hw).fpu ∨
    ∃ (d idx : Usize) (tk : U8) (ji : arch.bound.JobInfo), ki.depth k = ok d ∧ idx.val = d.val - 1 ∧
      ki.stack_at k idx = ok tk ∧ ki.job_info k tk = ok ji ∧ (h2 hw').fpu = ji.fpu :=
  Or.inr ⟨d, idx, tk, ji, hd, hidx, hst, hji, fpu_set hap hf⟩


syntax "epi_goal" : tactic
macro_rules
  | `(tactic| epi_goal) => `(tactic| first
      | (intro hs; cases hs; done)
      | (intro _ hn; exact absurd hn (by assumption))
      | (intro _ _; refine Or.inl ?_; hf_close; done)
      | (intro _ _
         refine epi_fpu (by assumption) (by assumption) (by assumption) (by assumption) ?_ ?_
         · first
             | (have hsub := usize_sub_ok ‹_ - 1#usize = ok _›
                have h1 : (1#usize).val = 1 := rfl
                omega)
             | (subst_vars; rfl)
             | (simp_all; done)
         · hf_close; done))

set_option maxHeartbeats 16000000 in
/-- **An unwind that resumes a Job, with no hop owed, leaves the
floating-point access as it was or sets it to the declaration of the Task
on top of the Kernel's stack** (`epilogue_core`; R11.7, R11.1). It is left
when the unwound level's record names no Partition before it. -/
theorem epilogue_core_fpu [HSpec2 ai h2] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {n : U32} {plan : U32 × U32 × U32 × U32} {safe : Bool}
    (h : arch.cm4.entry.epilogue_core ki ai ti k hw tables s n = ok ((plan, safe), k', hw', s')) :
    safe = false → plan.2.2.2 = 0#u32 →
    ((h2 hw').fpu = (h2 hw).fpu ∨
    ∃ (d idx : Usize) (tk : U8) (ji : arch.bound.JobInfo), ki.depth k = ok d ∧ idx.val = d.val - 1 ∧
      ki.stack_at k idx = ok tk ∧ ki.job_info k tk = ok ji ∧ (h2 hw').fpu = ji.fpu) := by
  unfold arch.cm4.entry.epilogue_core at h
  peel h
  all_goals first
    | epi_goal
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | epi_goal
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | epi_goal
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> epi_goal)))

end P18
