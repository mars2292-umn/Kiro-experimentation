import Sched.Deadline
open Aeneas Aeneas.Std Result

/-! # What a Kernel entry leaves (2026-11-18)

The run's Budget-event transitions ask for a quieted Kernel state
(`Quieted`, `Sched/Calls.lean`). This file reads from the entry cores as
extracted which Kernel each returns:

- `apply_core_inv`, `apply_core_quieted`: the Kernel `apply_core` returns
  is quieted whatever it was given (`settle`, then the two acknowledgements,
  which keep the scheduler state and the slots);
- `observe_quieted`: the first step of a timer entry keeps it;
- `*_core_q`: each entry core returns the Kernel it found, one `apply_core`
  returned, or, for the dispatch entry, `KernelOps::dispatch`'s result
  (`DispLeft`) and, for an SVC, the result of the lock, DMA-request,
  DMA-write, Endpoint-send or Endpoint-receive service (`SvcLeft`).

Not proven here: that the Kernel is quieted after those six calls. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

theorem Quieted.frame {cfg : logic.Config NT NR NP}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (hs : k'.sched = k.sched) (hl : k'.slots = k.slots) (h : Quieted cfg k) : Quieted cfg k' := by
  intro ha
  unfold topBudgetArmed at ha
  rw [hs, hl] at ha
  rw [hs]
  exact h ha

theorem ack_reinit_frame {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {p : U8}
    (h : logic.kernel.Kernel.ack_reinit k cfg p = ok k') :
    k'.sched = k.sched ∧ k'.slots = k.slots := by
  unfold logic.kernel.Kernel.ack_reinit at h
  peel h
  all_goals exact ⟨rfl, rfl⟩

theorem ack_stop_dma_frame {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {p : U8}
    (h : logic.kernel.Kernel.ack_stop_dma k cfg p = ok k') :
    k'.sched = k.sched ∧ k'.slots = k.slots := by
  unfold logic.kernel.Kernel.ack_stop_dma at h
  peel h
  all_goals exact ⟨rfl, rfl⟩

theorem bound_settle_q {b b' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    {r : Option U8 × Bool}
    (h : _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle b = ok (r, b')) :
    Quieted b'.cfg b'.kernel := by
  unfold _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle at h
  peel h
  all_goals exact settle_quieted (by assumption)

theorem bound_take_reinit_q {b b' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    {p : U8} {v : Bool}
    (h : _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit b p = ok (v, b'))
    (hq : Quieted b.cfg b.kernel) : Quieted b'.cfg b'.kernel := by
  unfold _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit at h
  peel h
  all_goals first
    | exact hq
    | exact Quieted.frame (ack_reinit_frame (by assumption)).1 (ack_reinit_frame (by assumption)).2 hq

theorem bound_take_stop_dma_q {b b' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    {p : U8} {v : Bool}
    (h : _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma b p = ok (v, b'))
    (hq : Quieted b.cfg b.kernel) : Quieted b'.cfg b'.kernel := by
  unfold _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma at h
  peel h
  all_goals first
    | exact hq
    | exact Quieted.frame (ack_stop_dma_frame (by assumption)).1 (ack_stop_dma_frame (by assumption)).2 hq

theorem apply_core_loop1_inv {K A T : Type _}
    (ki : _root_.rsk_kernel.arch.bound.KernelOps K)
    (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A)
    (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T) (Q : K → Prop)
    (hsd : ∀ k p v k1, ki.take_stop_dma k p = ok (v, k1) → Q k → Q k1)
    (hri : ∀ k p v k1, ki.take_reinit k p = ok (v, k1) → Q k → Q k1)
    {tables : T} {np : Usize} :
    ∀ (n : Nat) (k : K) (hw : A) (state : _root_.rsk_kernel.arch.cm4.plan.ArchState) (p : Usize)
      (res : K × A × _root_.rsk_kernel.arch.cm4.plan.ArchState),
      np.val - p.val = n → Q k →
      _root_.rsk_kernel.arch.cm4.entry.apply_core_loop1 ki ai ti k hw tables state np p = ok res →
      Q res.1 := by
  intro n
  induction n with
  | zero =>
    intro k hw state p res hn hq h
    unfold _root_.rsk_kernel.arch.cm4.entry.apply_core_loop1 loop at h
    simp only [_root_.rsk_kernel.arch.cm4.entry.apply_core_loop1.body] at h
    have hlt : ¬ p < np := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    exact hq
  | succ n ih =>
    intro k hw state p res hn hq h
    unfold _root_.rsk_kernel.arch.cm4.entry.apply_core_loop1 loop at h
    simp only [_root_.rsk_kernel.arch.cm4.entry.apply_core_loop1.body] at h
    have hlt : p < np := by scalar_tac
    simp only [hlt, ite_true, lift, bind_ok, bind_eq_ok] at h
    obtain ⟨cf, hbody, hloop⟩ := h
    obtain ⟨⟨b, k1⟩, hsd1, hbody⟩ := hbody
    peel hbody
    all_goals
      try dsimp only at hloop
      have hp := usize_add_ok ‹p + 1#usize = ok _›
      have h1 : (1#usize).val = 1 := rfl
      rw [h1] at hp
      apply ih (res := res)
      pick_goal 3
      · unfold _root_.rsk_kernel.arch.cm4.entry.apply_core_loop1
        simp only [_root_.rsk_kernel.arch.cm4.entry.apply_core_loop1.body, lift, bind_ok]
        exact hloop
      · omega
      · exact hri _ _ _ _ (by assumption) (hsd _ _ _ _ hsd1 hq)

/-- Whatever Kernel state `apply_core` is given, the one it returns
satisfies `Q`, when the interface's `settle` establishes `Q` and its two
acknowledgements keep it: nothing else in `apply_core` writes the Kernel. -/
theorem apply_core_inv {K A T : Type _}
    (ki : _root_.rsk_kernel.arch.bound.KernelOps K)
    (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A)
    (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T) (Q : K → Prop)
    (hst : ∀ k r k1, ki.settle k = ok (r, k1) → Q k1)
    (hsd : ∀ k p v k1, ki.take_stop_dma k p = ok (v, k1) → Q k → Q k1)
    (hri : ∀ k p v k1, ki.take_reinit k p = ok (v, k1) → Q k → Q k1)
    {k k' : K} {hw hw' : A} {tables : T}
    {state state' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {safe : Bool}
    (h : _root_.rsk_kernel.arch.cm4.entry.apply_core ki ai ti k hw tables state
      = ok (safe, k', hw', state')) : Q k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.apply_core at h
  peel h
  all_goals
    exact apply_core_loop1_inv ki ai ti Q hsd hri _ _ _ _ _ _ rfl (hst _ _ _ (by assumption))
      (by assumption)

/-! ## The entry cores -/

section cores
variable {K A T : Type _}
  (ki : _root_.rsk_kernel.arch.bound.KernelOps K)
  (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A)
  (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T) (Q : K → Prop)

/-- What `apply_core` returns satisfies `Q` (the premise of this section). -/
def ApplyQ : Prop :=
  ∀ {k : K} {hw : A} {tables : T} {st : _root_.rsk_kernel.arch.cm4.plan.ArchState} {safe : Bool}
    {k2 : K} {hw2 : A} {st2 : _root_.rsk_kernel.arch.cm4.plan.ArchState},
    _root_.rsk_kernel.arch.cm4.entry.apply_core ki ai ti k hw tables st = ok (safe, k2, hw2, st2) → Q k2

/-- The Kernel a dispatch entry leaves: the one it found, one `apply_core`
returned, or the result of the interface's `dispatch` on the one it found
(the entry that starts the selected Job does not go through `apply_core`). -/
def DispLeft (k k' : K) : Prop :=
  k' = k ∨ Q k' ∨ ∃ t n, ki.dispatch k t n = ok k'

/-- The Kernel an SVC leaves: the one it found, one `apply_core` returned,
or the result on the one it found of the lock SVC, of a DMA request, of a
DMA register write, or of an Endpoint send or receive (`Sched/Endpoint.lean` reads what those two do
to the Kernel): the services that return without `apply_core`. -/
def SvcLeft (k k' : K) : Prop :=
  k' = k ∨ Q k' ∨ (∃ res b, ki.svc_lock k res = ok (b, k')) ∨
  (∃ (hw : A) (tables : T) (i : Usize) (p l : U32) (b : Bool) (hw' : A),
    _root_.rsk_kernel.arch.cm4.entry.dma_request_core ki ai ti k hw tables i p l = ok (b, k', hw')) ∨
  (∃ (hw : A) (tables : T) (i : Usize) (p l : U32) (b : Bool) (hw' : A),
    _root_.rsk_kernel.arch.cm4.entry.dma_write_core ki ai ti k hw tables i p l = ok (b, k', hw')) ∨
  (∃ (hw : A) (tables : T) (st : _root_.rsk_kernel.arch.cm4.plan.ArchState) (a p l : U32) (r : U32) (hw' : A) (st' : _root_.rsk_kernel.arch.cm4.plan.ArchState),
    _root_.rsk_kernel.arch.cm4.entry.endpoint_send_core ki ai ti k hw tables st a p l = ok (r, k', hw', st')) ∨
  (∃ (hw : A) (tables : T) (st : _root_.rsk_kernel.arch.cm4.plan.ArchState) (a p l : U32) (r : U32 × verus_builtin.Ghost verus_builtin.int) (hw' : A) (st' : _root_.rsk_kernel.arch.cm4.plan.ArchState),
    _root_.rsk_kernel.arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables st a p l = ok (r, k', hw', st'))

set_option maxHeartbeats 8000000 in
theorem start_core_q (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState}  {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.start_core ki ai ti k hw tables state  = ok (r, k', hw', st')) :
    k' = k ∨ Q k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.start_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (hac (by assumption))

set_option maxHeartbeats 8000000 in
theorem timer_core_q (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState}  {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.timer_core ki ai ti k hw tables state  = ok (r, k', hw', st')) :
    k' = k ∨ Q k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.timer_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (hac (by assumption))

set_option maxHeartbeats 8000000 in
theorem unbound_core_q (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U16} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.unbound_core ki ai ti k hw tables state n = ok (r, k', hw', st')) :
    k' = k ∨ Q k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.unbound_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (hac (by assumption))

set_option maxHeartbeats 8000000 in
theorem intake_core_q (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U16} {task partition : U8} {psp er : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.intake_core ki ai ti k hw tables state n task partition psp er = ok (r, k', hw', st')) :
    k' = k ∨ Q k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.intake_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (hac (by assumption))

set_option maxHeartbeats 8000000 in
theorem stack_violation_core_q (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState}  {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.stack_violation_core ki ai ti k hw tables state  = ok (r, k', hw', st')) :
    k' = k ∨ Q k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.stack_violation_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (hac (by assumption))

set_option maxHeartbeats 8000000 in
theorem fault_core_q (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {kind : U8} {ink : Bool} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.fault_core ki ai ti k hw tables state kind ink = ok (r, k', hw', st')) :
    k' = k ∨ Q k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.fault_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (hac (by assumption))

set_option maxHeartbeats 8000000 in
theorem epilogue_core_q (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.epilogue_core ki ai ti k hw tables state n = ok (r, k', hw', st')) :
    k' = k ∨ Q k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.epilogue_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (hac (by assumption))
    | exact stack_violation_core_q ki ai ti Q hac (by assumption)

set_option maxHeartbeats 8000000 in
theorem dispatch_job_core_q (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control er : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.dispatch_job_core ki ai ti k hw tables state n t psp control er = ok (r, k', hw', st')) :
    DispLeft ki Q k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.dispatch_job_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (Or.inl (hac (by assumption)))
    | exact Or.inr (Or.inr ⟨_, _, by assumption⟩)
    | exact (stack_violation_core_q ki ai ti Q hac (by assumption)).imp_right Or.inl

set_option maxHeartbeats 8000000 in
theorem dispatch_vector_core_q (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control er : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.dispatch_vector_core ki ai ti k hw tables state n t psp control er = ok (r, k', hw', st')) :
    DispLeft ki Q k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.dispatch_vector_core at h
  by_cases hwd : _root_.rsk_kernel.arch.cm4.entry.WINDOW_DEFER = true <;>
    simp only [hwd, ↓reduceIte, Bool.false_eq_true] at h <;> peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (Or.inl (hac (by assumption)))
    | exact Or.inr (Or.inr ⟨_, _, by assumption⟩)
    | exact dispatch_job_core_q ki ai ti Q hac (by assumption)
    | exact (stack_violation_core_q ki ai ti Q hac (by assumption)).imp_right Or.inl

set_option maxHeartbeats 8000000 in
theorem irq_entry_core_q (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {ipsr psp control er : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.irq_entry_core ki ai ti k hw tables state ipsr psp control er = ok (r, k', hw', st')) :
    DispLeft ki Q k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.irq_entry_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (Or.inl (hac (by assumption)))
    | exact Or.inr (Or.inr ⟨_, _, by assumption⟩)
    | exact dispatch_vector_core_q ki ai ti Q hac (by assumption)
    | exact dispatch_job_core_q ki ai ti Q hac (by assumption)
    | exact (timer_core_q ki ai ti Q hac (by assumption)).imp_right Or.inl
    | exact (intake_core_q ki ai ti Q hac (by assumption)).imp_right Or.inl
    | exact (unbound_core_q ki ai ti Q hac (by assumption)).imp_right Or.inl
    | exact (stack_violation_core_q ki ai ti Q hac (by assumption)).imp_right Or.inl

set_option maxHeartbeats 8000000 in
theorem svc_service_core_q (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {frame frame' : Array U32 8#usize} {number : U8} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.svc_service_core ki ai ti k hw tables state frame number = ok (r, k', hw', st', frame')) :
    SvcLeft ki ai ti Q k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.svc_service_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (Or.inl (hac (by assumption)))
    | exact Or.inr (Or.inr (Or.inl ⟨_, _, by assumption⟩))
    | exact Or.inr (Or.inr (Or.inr (Or.inl ⟨_, _, _, _, _, _, _, by assumption⟩)))
    | exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inl ⟨_, _, _, _, _, _, _, by assumption⟩))))
    | exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inl ⟨_, _, _, _, _, _, _, _, _, by assumption⟩)))))
    | exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr ⟨_, _, _, _, _, _, _, _, _, by assumption⟩)))))

set_option maxHeartbeats 8000000 in
theorem svc_core_q (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {frame frame' : Array U32 8#usize} {number : U8} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.svc_core ki ai ti k hw tables state frame number = ok (r, k', hw', st', frame')) :
    SvcLeft ki ai ti Q k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.svc_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (Or.inl (hac (by assumption)))
    | exact Or.inr (Or.inr (Or.inl ⟨_, _, by assumption⟩))
    | exact Or.inr (Or.inr (Or.inr (Or.inl ⟨_, _, _, _, _, _, _, by assumption⟩)))
    | exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inl ⟨_, _, _, _, _, _, _, by assumption⟩))))
    | exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inl ⟨_, _, _, _, _, _, _, _, _, by assumption⟩)))))
    | exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr ⟨_, _, _, _, _, _, _, _, _, by assumption⟩)))))
    | exact svc_service_core_q ki ai ti Q hac (by assumption)

end cores

/-! ## `Bound`, the implementor `rsk-kernel` links -/

/-- For an implementor whose `settle` and two acknowledgements are
`Bound`'s, the Kernel that `apply_core` returns is quieted, whatever state
it was given: `Kernel::settle` establishes it and `ack_stop_dma` and
`ack_reinit`, the only writes after it, keep the scheduler state and the
slots. -/
theorem apply_core_quieted {A T : Type _}
    (ki : _root_.rsk_kernel.arch.bound.KernelOps (_root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE))
    (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A) (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T)
    (hst : ki.settle = _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle)
    (hsd : ki.take_stop_dma = _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma)
    (hri : ki.take_reinit = _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit) :
    ApplyQ ki ai ti (fun b => Quieted b.cfg b.kernel) := by
  intro k hw tables st safe k2 hw2 st2 h
  refine apply_core_inv ki ai ti (fun b => Quieted b.cfg b.kernel) ?_ ?_ ?_ h
  · intro k r k1 h1
    rw [hst] at h1
    exact bound_settle_q h1
  · intro k p v k1 h1 hq
    rw [hsd] at h1
    exact bound_take_stop_dma_q h1 hq
  · intro k p v k1 h1 hq
    rw [hri] at h1
    exact bound_take_reinit_q h1 hq

/-- `Kernel::observe`, the first step of a timer entry and of an intake,
keeps a quieted Kernel quieted: it writes the Kernel's reading only. -/
theorem observe_quieted {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64}
    (h : logic.kernel.Kernel.observe k cfg now = ok k') (hq : Quieted cfg k) : Quieted cfg k' := by
  refine Quieted.frame (observe_sched h) ?_ hq
  unfold logic.kernel.Kernel.observe at h
  frame_ok h

end P18
