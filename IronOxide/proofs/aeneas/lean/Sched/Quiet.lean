import Sched.Settle
import Sched.Total
open Aeneas Aeneas.Std Result

/-! # Every entry leaves the linked Kernel quieted (2026-11-21)

The run's Budget-event transitions ask for a quieted Kernel state
(`Quieted`, `Sched/Calls.lean`: with the Budget slot of the Job on top
armed, `Sched::select` returns no Task). `Sched/Settle.lean` proved it of
every Kernel `apply_core` returns and named the entries that return without
`apply_core`, for which it stayed a premise. This file closes them, on the
extracted code:

- `select_loop_congr`, `select_raised`: `Sched::select` reads the release
  marks, the Job marks, the stack and the System_Ceiling; on a state with
  the same marks and stack and a ceiling at least as high it returns none
  where it returned none.
- `svc_lock_quieted`, `bound_svc_lock_q`: the lock service keeps the slots
  and raises the ceiling or refuses and leaves the scheduler state, so it
  keeps a quieted Kernel quieted.
- `bound_dispatch_q`: `Bound`'s `dispatch` ends in `Kernel::settle`.
- `dma_request_core_true`, `dma_write_core_true`,
  `endpoint_recv_core_kept`: an accepted EasyDMA request or register write
  and a receive that is not refused return the Kernel they were given.
- `svc_service_core_left`, `svc_core_left`: so an SVC leaves the Kernel it
  found, one `apply_core` returned, or the lock service's result.
- `svc_core_quieted`, `irq_entry_core_quieted`, `start_core_quieted`,
  `fault_core_quieted`, `epilogue_core_quieted`: for the linked Kernel
  (`QBound`: the five interface functions read are `Bound`'s) every entry
  core returns a quieted Kernel when it was given one, and `apply_core`,
  with which the boot ends, returns one whatever it was given.

So the premise `Quieted` of the run's four Budget-event transitions holds
of the state every entry leaves, by induction over the entries from the
boot. The run of `Sched/KRun.lean` (`KStep`) has no entry boundaries and
its transitions carry the premise; `Sched/Entries.lean` (2026-11-22) cuts
the run into the Kernel's entries and discharges it with these theorems. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The loop of `Sched::select` reads the scheduler state through its
release marks only: on a state with the same marks it returns the same
Task index. -/
theorem select_loop_congr (s s' : logic.sched.Sched NT NR NL)
    (hr : s'.release = s.release) :
    ∀ (n : Nat) (c : logic.Config NT NR NP) (bt t : Usize) res, NT.val - t.val = n →
      logic.sched.Sched.select_loop s c bt t = ok res →
      logic.sched.Sched.select_loop s' c bt t = ok (s', res.2.1, res.2.2) := by
  intro n
  induction n with
  | zero =>
    intro c bt t res hn h
    unfold logic.sched.Sched.select_loop loop at h ⊢
    simp only [logic.sched.Sched.select_loop.body] at h ⊢
    have hlt : ¬ t < NT := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h ⊢
    have h := ok_inj h
    subst h
    rfl
  | succ n ih =>
    intro c bt t res hn h
    unfold logic.sched.Sched.select_loop loop at h ⊢
    simp only [logic.sched.Sched.select_loop.body] at h ⊢
    have hlt : t < NT := by scalar_tac
    simp only [hlt, ite_true, hr] at h ⊢
    rw [bind_eq_ok] at h
    obtain ⟨cf, hcf, hloop⟩ := h
    rw [hcf]
    simp only [bind_ok]
    simp only [bind_eq_ok, Prod.exists, Aeneas.Std.uncurry] at hcf
    obtain ⟨i, hi, cfg1, bt1, hsel, t1, ht1, hcf⟩ := hcf
    have hcf := ok_inj hcf
    subst hcf
    have ht1 := usize_add_ok ht1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at ht1
    have hrec : logic.sched.Sched.select_loop s cfg1 bt1 t1 = ok res := by
      unfold logic.sched.Sched.select_loop
      simp only [logic.sched.Sched.select_loop.body]
      exact hloop
    have hgoal := ih cfg1 bt1 t1 res (by omega) hrec
    unfold logic.sched.Sched.select_loop at hgoal
    simp only [logic.sched.Sched.select_loop.body, hr] at hgoal
    exact hgoal

/-- `Sched::select` on a state that differs from one on which it returns
none only in a System_Ceiling at least as high (and in fields `select`
does not read) returns none: raising the ceiling admits no Task. -/
theorem select_raised {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    (hr : s'.release = s.release) (hj : s'.job = s.job) (hst : s'.stack = s.stack)
    (hd : s'.depth = s.depth) (hc : s.ceiling.val ≤ s'.ceiling.val)
    (h : logic.sched.Sched.select s cfg = ok none) :
    logic.sched.Sched.select s' cfg = ok none := by
  unfold logic.sched.Sched.select at h ⊢
  rw [bind_eq_ok] at h
  obtain ⟨⟨s1, cfg1, bt⟩, hloop, h⟩ := h
  have hs1 := (select_loop_ok s cfg _ NT 0#usize _ rfl (by scalar_tac)
    ⟨Nat.le_refl _, fun _ u hu => by simp at hu, fun hx => absurd hx (Nat.lt_irrefl _)⟩ hloop).1
  dsimp only at hs1
  subst hs1
  rw [select_loop_congr s1 s' hr _ cfg NT 0#usize _ rfl hloop]
  simp only [bind_ok, Aeneas.Std.uncurry] at h ⊢
  rw [hj, hst, hd]
  split at h
  · rename_i hlt
    simp only [hlt, ite_true, lift, bind_ok] at h ⊢
    simp only [bind_eq_ok] at h
    obtain ⟨tc, htc, above, habove, i2, hi2, h⟩ := h
    simp only [htc, habove, hi2, bind_ok]
    by_cases hgt : tc.priority > s'.ceiling
    · have hgt1 : tc.priority > s1.ceiling := by scalar_tac
      simp only [hgt1, ite_true] at h
      simp only [hgt, ite_true]
      exact h
    · simp only [hgt, ite_false]
  · rename_i hlt
    simp only [hlt, ite_false, bind_ok]

/-! ## The lock service keeps a quieted Kernel quieted -/

/-- `Kernel::svc_lock` keeps the slots, and its scheduler state is the one
before with, when the lock is granted, the System_Ceiling raised. -/
theorem svc_lock_quieted {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_lock k cfg r = ok (b, k')) (hq : Quieted cfg k) :
    Quieted cfg k' := by
  have hsl : k'.slots = k.slots := by
    unfold logic.kernel.Kernel.svc_lock at h
    peel h
    all_goals first
      | rfl
      | (have hp := push_event_slots (by assumption); exact hp)
  have he := svc_lock_eff h
  cases b with
  | false =>
    exact Quieted.frame (lock_refused he) hsl hq
  | true =>
    obtain ⟨_, _, n, d, c, _, _, hc, hs⟩ := lock_ok he
    intro ha
    have ha0 : topBudgetArmed k cfg := by
      unfold topBudgetArmed at ha ⊢
      rw [hsl, hs] at ha
      exact ha
    have h0 := hq ha0
    rw [hs]
    exact select_raised (s := k.sched) rfl rfl rfl rfl (by dsimp only; omega) h0

/-! ## `Bound`: the dispatch and the lock -/

/-- `Bound`'s `dispatch` ends in `Kernel::settle`: the Kernel it returns is
quieted, whatever it was given. -/
theorem bound_dispatch_q {b b' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    {t : U8} {n : U32}
    (h : _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch b t n = ok b') :
    Quieted b'.cfg b'.kernel := by
  unfold _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch at h
  peel h
  all_goals exact settle_quieted (by assumption)

/-- `Bound`'s `svc_lock` keeps a quieted Kernel quieted. -/
theorem bound_svc_lock_q {b b' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    {r : U8} {v : Bool}
    (h : _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock b r = ok (v, b'))
    (hq : Quieted b.cfg b.kernel) : Quieted b'.cfg b'.kernel := by
  unfold _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock at h
  peel h
  all_goals first
    | exact hq
    | exact svc_lock_quieted (by assumption) hq

/-! ## The services that return without `apply_core` -/

section cores
variable {K A T : Type _}
  (ki : _root_.rsk_kernel.arch.bound.KernelOps K)
  (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A)
  (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T) (Q : K → Prop)

/-- An accepted EasyDMA request leaves the Kernel as it found it. -/
theorem dma_request_core_true {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {p l : U32}
    (h : _root_.rsk_kernel.arch.cm4.entry.dma_request_core ki ai ti k hw tables i p l = ok (true, k', hw')) :
    k' = k := by
  unfold _root_.rsk_kernel.arch.cm4.entry.dma_request_core at h
  peel h
  all_goals first
    | rfl
    | simp_all

/-- An accepted EasyDMA register write leaves the Kernel as it found it. -/
theorem dma_write_core_true {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {p l : U32}
    (h : _root_.rsk_kernel.arch.cm4.entry.dma_write_core ki ai ti k hw tables i p l = ok (true, k', hw')) :
    k' = k := by
  unfold _root_.rsk_kernel.arch.cm4.entry.dma_write_core at h
  peel h
  all_goals simp_all

/-- A receive that is not refused leaves the Kernel as it found it. -/
theorem endpoint_recv_core_kept {k k' : K} {hw hw' : A} {tables : T}
    {state state' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {arg ptr len : U32} {r}
    (h : _root_.rsk_kernel.arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables state arg ptr len
      = ok (r, k', hw', state'))
    (hr : r.1 ≠ _root_.rsk_kernel.arch.cm4.entry.ENDPOINT_REFUSED) : k' = k := by
  unfold _root_.rsk_kernel.arch.cm4.entry.endpoint_recv_core at h
  peel h
  all_goals first
    | rfl
    | exact absurd rfl hr

set_option maxHeartbeats 8000000 in
/-- The Kernel a service SVC leaves is the one it found or one
`apply_core` returned: the three services that return without
`apply_core` (an accepted EasyDMA request or register write, a receive
that is not refused) do not write the Kernel. -/
theorem svc_service_core_left (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {frame frame' : Array U32 8#usize} {number : U8} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.svc_service_core ki ai ti k hw tables state frame number = ok (r, k', hw', st', frame')) :
    k' = k ∨ Q k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.svc_service_core at h
  peel h
  all_goals first
    | exact Or.inr (hac (by assumption))
    | contradiction
    | (subst_vars; first
        | exact Or.inl (dma_request_core_true ki ai ti (by assumption))
        | exact Or.inl (dma_write_core_true ki ai ti (by assumption)))
    | (refine Or.inl (endpoint_recv_core_kept ki ai ti (by assumption) ?_); simp_all)
    | (exfalso; simp_all [_root_.rsk_kernel.arch.cm4.entry.SVC_SEND, _root_.rsk_kernel.arch.cm4.entry.SVC_RECV])

set_option maxHeartbeats 8000000 in
/-- The Kernel an SVC leaves is the one it found, one `apply_core`
returned, or the result of the lock service on the one it found. -/
theorem svc_core_left (hac : ApplyQ ki ai ti Q)
    {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {frame frame' : Array U32 8#usize} {number : U8} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.svc_core ki ai ti k hw tables state frame number = ok (r, k', hw', st', frame')) :
    k' = k ∨ Q k' ∨ ∃ res b, ki.svc_lock k res = ok (b, k') := by
  unfold _root_.rsk_kernel.arch.cm4.entry.svc_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (Or.inl (hac (by assumption)))
    | exact Or.inr (Or.inr ⟨_, _, by assumption⟩)
    | exact (svc_service_core_left ki ai ti Q hac (by assumption)).imp_right Or.inl

end cores

/-! ## Every entry of the linked Kernel leaves it quieted -/

/-- The five interface functions the argument reads are `Bound`'s. The
extraction holds `Bound`'s functions one by one and not as an instance
(`Sched/Endpoint.lean`), so the link is by these equations, as for
`apply_core_quieted`. -/
structure QBound (ki : _root_.rsk_kernel.arch.bound.KernelOps (_root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)) : Prop where
  settle : ki.settle = _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle
  take_stop_dma : ki.take_stop_dma = _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma
  take_reinit : ki.take_reinit = _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit
  dispatch : ki.dispatch = _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch
  svc_lock : ki.svc_lock = _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock

section linked
variable {A T : Type _}
  (ki : _root_.rsk_kernel.arch.bound.KernelOps (_root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE))
  (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A) (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T)

/-- `Quieted` of a `Bound`. -/
abbrev BQ (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE) : Prop := Quieted b.cfg b.kernel

theorem qbound_apply (hb : QBound ki) : ApplyQ ki ai ti BQ :=
  apply_core_quieted ki ai ti hb.settle hb.take_stop_dma hb.take_reinit

/-- **An SVC leaves the linked Kernel quieted when it found it so.** Every
service either ends in `apply_core`, leaves the Kernel untouched, or is
the lock, which raises the System_Ceiling and so admits no Task. -/
theorem svc_core_quieted (hb : QBound ki)
    {k k' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {frame frame' : Array U32 8#usize} {number : U8} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.svc_core ki ai ti k hw tables state frame number = ok (r, k', hw', st', frame'))
    (hq : BQ k) : BQ k' := by
  rcases svc_core_left ki ai ti BQ (qbound_apply ki ai ti hb) h with rfl | hq' | ⟨res, b, hl⟩
  · exact hq
  · exact hq'
  · rw [hb.svc_lock] at hl
    exact bound_svc_lock_q hl hq

/-- **An interrupt entry leaves the linked Kernel quieted when it found it
so**: the timer, an intake, a dispatch vector (whose `dispatch` ends in
`Kernel::settle`), an unbound vector, the stack check. -/
theorem irq_entry_core_quieted (hb : QBound ki)
    {k k' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {ipsr psp control er : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.irq_entry_core ki ai ti k hw tables state ipsr psp control er = ok (r, k', hw', st'))
    (hq : BQ k) : BQ k' := by
  rcases irq_entry_core_q ki ai ti BQ (qbound_apply ki ai ti hb) h with rfl | hq' | ⟨t, m, hd⟩
  · exact hq
  · exact hq'
  · rw [hb.dispatch] at hd
    exact bound_dispatch_q hd

/-- The start of the system leaves the linked Kernel quieted when it found it so. -/
theorem start_core_quieted (hb : QBound ki)
    {k k' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState}  {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.start_core ki ai ti k hw tables state  = ok (r, k', hw', st'))
    (hq : BQ k) : BQ k' := by
  rcases start_core_q ki ai ti BQ (qbound_apply ki ai ti hb) h with rfl | hq'
  · exact hq
  · exact hq'

/-- A fault entry leaves the linked Kernel quieted when it found it so. -/
theorem fault_core_quieted (hb : QBound ki)
    {k k' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {kind : U8} {ink : Bool} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.fault_core ki ai ti k hw tables state kind ink = ok (r, k', hw', st'))
    (hq : BQ k) : BQ k' := by
  rcases fault_core_q ki ai ti BQ (qbound_apply ki ai ti hb) h with rfl | hq'
  · exact hq
  · exact hq'

/-- The unwind entry leaves the linked Kernel quieted when it found it so. -/
theorem epilogue_core_quieted (hb : QBound ki)
    {k k' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.epilogue_core ki ai ti k hw tables state n = ok (r, k', hw', st'))
    (hq : BQ k) : BQ k' := by
  rcases epilogue_core_q ki ai ti BQ (qbound_apply ki ai ti hb) h with rfl | hq'
  · exact hq
  · exact hq'

end linked

end P18
