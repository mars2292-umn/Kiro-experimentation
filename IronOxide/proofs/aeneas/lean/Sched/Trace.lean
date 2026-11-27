import Sched.CoreReal
import Sched.Entries
import Sched.Timer
import Sched.Endpoint
open Aeneas Aeneas.Std Result

/-! # What an entry does to the Kernel is a sequence of the run's functions (2026-11-27)

The run of `Sched/KRun.lean` (`KStep`) changes the Kernel's state only by
the result of an extracted function of `rsk-logic`, one per transition.
`Sched/Entries.lean` cut the run into the Kernel's entries and left one
thing read from the code, not proven: that what an entry core does to the
Kernel between its two boundaries is such transitions. This file proves
the code's half of it, for the Kernel that is linked.

- `LStep cfg k k'`: `k'` is the result on `k` of one of the functions the
  run's transitions carry: `observe`, the charge of the Job on top,
  `serve_slot` of a slot armed and due, `deadline_miss`, `intake_release`,
  `dispatch`, `settle`, `job_return`, `svc_lock`, `svc_unlock`, `fault`,
  `fault_of`, `record_event`, `ack_reinit`, `ack_stop_dma`; or of
  `Kernel::start` or one of the boot's two records (`boot_check_failed`,
  `boot_info`), which the run has no transition for. `LSteps` is a finite
  sequence of them.
- `Tr b b'`: the linked Kernel `b'` has `b`'s configuration, and its
  Kernel state is reached from `b`'s by `LSteps`.
- `b*_tr`: every method of `impl KernelOps for Bound` that returns a
  Kernel gives `Tr`, by its body: `dispatch` is `Kernel::dispatch` (or
  nothing) and then `settle`; `timer_event` is the reading, the charge and
  the slots served (`timer_event_served`'s chain); `intake` is the
  reading, the Task's deadline slot when it is armed and due, and
  `intake_release`.
- `bound_kopst_tr`: the assumption class of the entry cores' totality
  proofs (`KOpsT`, `Sched/CoreTotal.lean`) holds with, as the state's
  invariant, `BX` and "reached from `b0` by `Tr`". Every theorem of that
  file is then a theorem about the trace: the invariant a core leaves is
  the invariant with the trace extended by whatever the core called.
- `svc_core_trace`, `irq_entry_core_trace`, `start_core_trace`,
  `fault_core_trace`, `epilogue_core_trace`, and `coreret_trace` for the
  five together: **when an entry core of the linked Kernel, called on `b`
  satisfying `BX`, returns `b'`, then `Tr b b'`.** The configuration is
  the same before and after (which `EntryOf` of `Sched/Entries.lean`
  states of an entry and did not derive), and the Kernel's state changed
  by a sequence of the functions above and by nothing else.

What this does not say (the first and the third are proven since, later
the same day: `Sched/Script.lean` reads the order from the cores, and
`Sched/EntryRun.lean` gives the Kernel's functions of each path
(`coreret_ksh`) and shows the state the start leaves is a boot state of the
run, `start_first`). (1) Which of the functions an entry calls, and in
what order: the relation admits any sequence, the two boot records and
`Kernel::start` included in every entry (the assumption class has one
invariant for every method, so a method no core calls cannot be told from
one it does). (2) The model's side of each transition: that the run's
clock, its pending and running counts and its Kernel sections are in the
state the transition asks for (an arrival awaiting its entry, the
section's bound, the Job's execution) is the model's, and its timing
conditions are measured. (3) `Kernel::start` and the boot's records have no
transition in `KStep`: the run begins at the state `init` leaves
(`KSt.boot`), and a run with the start in it is not stated. Assumed, as
in `Sched/CoreReal.lean`: `HwT` and `TabT` (the hardware operations and
the tables' accessors return), and for an interrupt entry that the
Partition taken as executing is 255 or one of the layout's (`ai_cp` gives
it from the entry cores' invariant). Handwritten; no `sorry`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- One function of the run's transitions, on the Kernel's state. -/
inductive LStep (cfg : logic.Config NT NR NP) :
    logic.kernel.Kernel NT NR NP NL NS NE → logic.kernel.Kernel NT NR NP NL NS NE → Prop
  | observed (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (now : U64)
      (h : logic.kernel.Kernel.observe k cfg now = ok k') : LStep cfg k k'
  | charged (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : U8) (rc : U32)
      (b : logic.budget.Budget NT)
      (h : logic.budget.Budget.charge k.budget t rc = ok b) : LStep cfg k { k with budget := b }
  | served (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) (hdue : SlotDue k i)
      (h : logic.kernel.Kernel.serve_slot k cfg i = ok k') : LStep cfg k k'
  | deadline (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (t : U8) (i : Usize)
      (h : logic.kernel.Kernel.deadline_miss k cfg t i = ok k') : LStep cfg k k'
  | intake (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (t sp off : U8) (now : U64)
      (h : logic.kernel.Kernel.intake_release k cfg t sp now off = ok k') : LStep cfg k k'
  | dispatch (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (t : U8) (pc : U32)
      (h : logic.kernel.Kernel.dispatch k cfg t pc = ok k') : LStep cfg k k'
  | settle (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (r : Option U8 × Bool)
      (h : logic.kernel.Kernel.settle k cfg = ok (r, k')) : LStep cfg k k'
  | finish (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (ec : U32)
      (h : logic.kernel.Kernel.job_return k cfg ec = ok k') : LStep cfg k k'
  | lock (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (r : U8) (v : Bool)
      (h : logic.kernel.Kernel.svc_lock k cfg r = ok (v, k')) : LStep cfg k k'
  | unlock (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (r : U8) (v : Bool)
      (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (v, k')) : LStep cfg k k'
  | faulted (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (kind : U8) (kn : Bool)
      (h : logic.kernel.Kernel.fault k cfg kind kn = ok k') : LStep cfg k k'
  | faulted_of (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (kind t : U8)
      (h : logic.kernel.Kernel.fault_of k cfg kind t = ok k') : LStep cfg k k'
  | recorded (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (kind t : U8)
      (h : logic.kernel.Kernel.record_event k cfg kind t = ok k') : LStep cfg k k'
  | acked_reinit (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (p : U8)
      (h : logic.kernel.Kernel.ack_reinit k cfg p = ok k') : LStep cfg k k'
  | acked_stop (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (p : U8)
      (h : logic.kernel.Kernel.ack_stop_dma k cfg p = ok k') : LStep cfg k k'
  | started (k k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (h : logic.kernel.Kernel.start k cfg = ok k') : LStep cfg k k'
  | boot_failed (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (f : U32)
      (h : logic.kernel.Kernel.boot_check_failed k cfg f = ok k') : LStep cfg k k'
  | boot_info (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (x y : Bool)
      (h : logic.kernel.Kernel.boot_info k cfg x y = ok k') : LStep cfg k k'

/-- A finite sequence of them. -/
abbrev LSteps (cfg : logic.Config NT NR NP) :
    logic.kernel.Kernel NT NR NP NL NS NE → logic.kernel.Kernel NT NR NP NL NS NE → Prop :=
  Relation.ReflTransGen (LStep cfg)

/-- The linked Kernel `b'` has `b`'s configuration and its Kernel state is
reached from `b`'s by the run's functions. -/
structure Tr (b b' : arch.bound.Bound NT NR NP NL NS NE) : Prop where
  cfg : b'.cfg = b.cfg
  steps : LSteps b.cfg b.kernel b'.kernel

theorem Tr.rfl' (b : arch.bound.Bound NT NR NP NL NS NE) : Tr b b := ⟨rfl, .refl⟩

theorem Tr.trans {a b c : arch.bound.Bound NT NR NP NL NS NE} (h1 : Tr a b) (h2 : Tr b c) :
    Tr a c := by
  refine ⟨h2.cfg.trans h1.cfg, h1.steps.trans ?_⟩
  have := h2.steps
  rwa [h1.cfg] at this

/-- A state with the same configuration and Kernel state. -/
theorem Tr.same {b b' : arch.bound.Bound NT NR NP NL NS NE} (hc : b'.cfg = b.cfg)
    (hk : b'.kernel = b.kernel) : Tr b b' := ⟨hc, by rw [hk]⟩

/-- One function's result put in the Kernel state. -/
theorem Tr.one {b : arch.bound.Bound NT NR NP NL NS NE}
    {k : logic.kernel.Kernel NT NR NP NL NS NE} (h : LStep b.cfg b.kernel k) :
    Tr b { b with kernel := k } := ⟨rfl, .single h⟩

/-! ## The methods of the linked Kernel -/

theorem bstart_tr {b b' : arch.bound.Bound NT NR NP NL NS NE}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start b = ok b') : Tr b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start at h
  split at h
  · have h := ok_inj h; subst h; exact Tr.rfl' _
  · simp only [bind_eq_ok] at h
    obtain ⟨k, hk, h⟩ := h
    have h := ok_inj h; subst h
    exact Tr.one (.started _ _ hk)

theorem bdispatch_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {t : U8} {pc : U32}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch b t pc = ok b') :
    Tr b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch at h
  simp only [bind_eq_ok] at h
  obtain ⟨k, hk, ⟨r, k1⟩, hs, h⟩ := h
  have h := ok_inj h; subst h
  have hd : LSteps b.cfg b.kernel k := by
    split at hk
    · simp only [bind_eq_ok] at hk
      obtain ⟨e, _, hk⟩ := hk
      split at hk
      · exact .single (.dispatch _ _ _ _ hk)
      · have hk := ok_inj hk; subst hk; exact .refl
    · have hk := ok_inj hk; subst hk; exact .refl
  exact ⟨rfl, hd.tail (.settle _ _ r hs)⟩

theorem bjob_return_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {ec : U32}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return b ec = ok b') :
    Tr b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return at h
  split at h
  · simp only [bind_eq_ok] at h
    obtain ⟨k, hk, h⟩ := h
    have h := ok_inj h; subst h
    exact Tr.one (.finish _ _ _ hk)
  · have h := ok_inj h; subst h; exact Tr.rfl' _

theorem bsvc_lock_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {r : U8} {v : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock b r = ok (v, b')) :
    Tr b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock at h
  split at h
  · have h := ok_inj h; cases h; exact Tr.rfl' _
  · simp only [bind_eq_ok, lift, bind_ok] at h
    split at h
    · have h := ok_inj h; cases h; exact Tr.rfl' _
    · simp only [bind_eq_ok] at h
      obtain ⟨⟨v1, k⟩, hk, h⟩ := h
      have h := ok_inj h; cases h
      exact Tr.one (.lock _ _ _ _ hk)

theorem bsvc_unlock_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {r : U8} {v : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock b r = ok (v, b')) :
    Tr b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock at h
  split at h
  · have h := ok_inj h; cases h; exact Tr.rfl' _
  · simp only [bind_eq_ok] at h
    obtain ⟨⟨v1, k⟩, hk, h⟩ := h
    have h := ok_inj h; cases h
    exact Tr.one (.unlock _ _ _ _ hk)

theorem bsettle_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {p : Option U8 × Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle b = ok (p, b')) :
    Tr b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle at h
  simp only [bind_eq_ok] at h
  obtain ⟨⟨p1, k⟩, hk, h⟩ := h
  have h := ok_inj h; cases h
  exact Tr.one (.settle _ _ _ hk)

theorem bintake_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {t sp off : U8} {now : U64}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.intake b t sp now off = ok b') :
    Tr b b' := by
  obtain ⟨hc, e | ⟨off1, hi⟩⟩ := bound_intake h
  · exact Tr.same hc e
  · obtain ⟨k1, k2, hobs, hk2, hrel⟩ := kernel_intake_cases hi
    refine ⟨hc, ?_⟩
    have h1 : LSteps b.cfg b.kernel k1 := .single (.observed _ _ _ hobs)
    have h2 : LSteps b.cfg k1 k2 := by
      rcases hk2 with e | ⟨sd, hd⟩
      · subst e; exact .refl
      · exact .single (.deadline _ _ _ _ hd)
    exact (h1.trans h2).tail (.intake _ _ _ _ _ _ hrel)

theorem ServedChain.lsteps {cfg : logic.Config NT NR NP}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE} (h : ServedChain cfg k k') :
    LSteps cfg k k' := by
  induction h with
  | done => exact .refl
  | serve k k1 k' i hdue hs _ ih => exact .head (.served _ _ i hdue hs) ih

/-- `Kernel::timer_event`: the reading of the time base, the charge of the
Job on top, and the slots served. -/
theorem ktimer_event_tr {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64} {rc : U32}
    (h : logic.kernel.Kernel.timer_event k cfg now rc = ok k') : LSteps cfg k k' := by
  unfold logic.kernel.Kernel.timer_event at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨k0, hobs, h⟩ := h
  have ho : LSteps cfg k k0 := .single (.observed _ _ now hobs)
  split at h
  · have h := ok_inj h; subst h; exact ho
  · simp only [bind_eq_ok] at h
    obtain ⟨b, hb, r, hl0, hl1⟩ := h
    have hc : LSteps cfg k0 { k0 with budget := b } := by
      split at hb
      · simp only [bind_eq_ok] at hb
        obtain ⟨i, _, t, _, hb⟩ := hb
        exact .single (.charged _ t rc b hb)
      · have hb := ok_inj hb; subst hb; exact .refl
    exact (ho.trans hc).trans (timer_loops_served hl0 hl1).lsteps

theorem btimer_event_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {now : U64} {rc : U32}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event b now rc = ok b') :
    Tr b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event at h
  simp only [bind_eq_ok] at h
  obtain ⟨now1, _, k, hk, h⟩ := h
  have h := ok_inj h; subst h
  exact ⟨rfl, ktimer_event_tr hk⟩

theorem bfault_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {kind : U8} {kn : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault b kind kn = ok b') :
    Tr b b' := by
  obtain ⟨hk, hc⟩ := bound_fault h
  exact ⟨hc, .single (.faulted _ _ _ _ hk)⟩

theorem bfault_of_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {kind t : U8}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of b kind t = ok b') :
    Tr b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  split at h
  · simp only [bind_eq_ok] at h
    obtain ⟨k, hk, h⟩ := h
    have h := ok_inj h; subst h
    exact Tr.one (.faulted_of _ _ _ _ hk)
  · have h := ok_inj h; subst h; exact Tr.rfl' _

theorem brecord_event_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {kind t : U8}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event b kind t = ok b') :
    Tr b b' := by
  obtain ⟨hc, e | hk⟩ := bound_record_event h
  · exact Tr.same hc e
  · exact ⟨hc, .single (.recorded _ _ _ _ hk)⟩

theorem btake_reinit_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {p : U8} {v : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit b p = ok (v, b')) :
    Tr b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  split at h
  · have h := ok_inj h; cases h; exact Tr.rfl' _
  · simp only [bind_eq_ok] at h
    obtain ⟨v1, _, h⟩ := h
    split at h
    · simp only [bind_eq_ok] at h
      obtain ⟨k, hk, h⟩ := h
      have h := ok_inj h; cases h
      exact Tr.one (.acked_reinit _ _ _ hk)
    · have h := ok_inj h; cases h; exact Tr.rfl' _

theorem btake_stop_dma_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {p : U8} {v : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma b p = ok (v, b')) :
    Tr b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  split at h
  · have h := ok_inj h; cases h; exact Tr.rfl' _
  · simp only [bind_eq_ok] at h
    obtain ⟨v1, _, h⟩ := h
    split at h
    · simp only [bind_eq_ok] at h
      obtain ⟨k, hk, h⟩ := h
      have h := ok_inj h; cases h
      exact Tr.one (.acked_stop _ _ _ hk)
    · have h := ok_inj h; cases h; exact Tr.rfl' _

theorem bboot_failed_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {f : U32}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_failed b f = ok b') :
    Tr b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_failed at h
  simp only [bind_eq_ok] at h
  obtain ⟨k, hk, h⟩ := h
  have h := ok_inj h; subst h
  exact Tr.one (.boot_failed _ _ _ hk)

theorem bboot_info_tr {b b' : arch.bound.Bound NT NR NP NL NS NE} {x y : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_info b x y = ok b') :
    Tr b b' := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_info at h
  simp only [bind_eq_ok] at h
  obtain ⟨k, hk, h⟩ := h
  have h := ok_inj h; subst h
  exact Tr.one (.boot_info _ _ _ _ hk)

/-! ## The entry cores' assumption class with the trace in the invariant -/

/-- `BX`, and reached from `b0` by the run's functions. -/
def BXT (b0 b : arch.bound.Bound NT NR NP NL NS NE) : Prop := BX b ∧ Tr b0 b

/-- A method that keeps `BX` and gives `Tr` keeps `BXT`. -/
theorem tr_spec {α} {x : Result α} (π : α → arch.bound.Bound NT NR NP NL NS NE)
    {b0 b : arch.bound.Bound NT NR NP NL NS NE} (hk : BXT b0 b)
    (ht : x ⦃ r => BX (π r) ⦄) (hs : ∀ r, x = ok r → Tr b (π r)) :
    x ⦃ r => BXT b0 (π r) ⦄ := by
  obtain ⟨r, hr, hp⟩ := spec_returns ht
  rw [hr]
  exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (hs r hr)⟩

variable {ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)}

/-- **The assumption class of the entry cores, for the Kernel that is
linked, with the trace from `b0` in the invariant.** -/
theorem bound_kopst_tr (hb : IsBoundT ki) (b0 : arch.bound.Bound NT NR NP NL NS NE) :
    KOpsT ki (BXT b0) NT.val NP.val where
  depth := fun k hk => KOpsT.depth (self := bound_kopst hb) k hk.1
  top := fun k hk => KOpsT.top (self := bound_kopst hb) k hk.1
  job_info := fun k t hk => KOpsT.job_info (self := bound_kopst hb) k t hk.1
  stack_at := fun k i hk => KOpsT.stack_at (self := bound_kopst hb) k i hk.1
  is_killed := fun k t hk => KOpsT.is_killed (self := bound_kopst hb) k t hk.1
  eligible := fun k t hk => KOpsT.eligible (self := bound_kopst hb) k t hk.1
  basepri_now := fun k hk => KOpsT.basepri_now (self := bound_kopst hb) k hk.1
  next_compare := fun k hk => KOpsT.next_compare (self := bound_kopst hb) k hk.1
  source_masked := fun k t hk => KOpsT.source_masked (self := bound_kopst hb) k t hk.1
  partition_stopped := fun k p hk => KOpsT.partition_stopped (self := bound_kopst hb) k p hk.1
  partition_view := fun k p hk => KOpsT.partition_view (self := bound_kopst hb) k p hk.1
  dma_permitted := fun k p a c hk => KOpsT.dma_permitted (self := bound_kopst hb) k p a c hk.1
  in_ro_window := fun k p a hk => KOpsT.in_ro_window (self := bound_kopst hb) k p a hk.1
  message_permitted := fun k p a c hk => KOpsT.message_permitted (self := bound_kopst hb) k p a c hk.1
  safe_state := fun k hk => KOpsT.safe_state (self := bound_kopst hb) k hk.1
  task_count := fun k hk => KOpsT.task_count (self := bound_kopst hb) k hk.1
  partition_count := fun k hk => KOpsT.partition_count (self := bound_kopst hb) k hk.1
  config_checksum := fun k hk => KOpsT.config_checksum (self := bound_kopst hb) k hk.1
  layout_checksum := fun k hk => KOpsT.layout_checksum (self := bound_kopst hb) k hk.1
  start := fun k hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.start (self := bound_kopst hb) k hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start k = ok r := by
      rw [← hb.start]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (bstart_tr e)⟩
  dispatch := fun k t pc hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.dispatch (self := bound_kopst hb) k t pc hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch k t pc = ok r := by
      rw [← hb.dispatch]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (bdispatch_tr e)⟩
  job_return := fun k e hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.job_return (self := bound_kopst hb) k e hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return k e = ok r := by
      rw [← hb.job_return]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (bjob_return_tr e)⟩
  svc_lock := fun k x hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.svc_lock (self := bound_kopst hb) k x hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock k x = ok r := by
      rw [← hb.svc_lock]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (bsvc_lock_tr (v := r.1) e)⟩
  svc_unlock := fun k x hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.svc_unlock (self := bound_kopst hb) k x hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock k x = ok r := by
      rw [← hb.svc_unlock]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (bsvc_unlock_tr (v := r.1) e)⟩
  intake := fun k t p now o hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.intake (self := bound_kopst hb) k t p now o hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.intake k t p now o = ok r := by
      rw [← hb.intake]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (bintake_tr e)⟩
  timer_event := fun k now rc hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.timer_event (self := bound_kopst hb) k now rc hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event k now rc = ok r := by
      rw [← hb.timer_event]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (btimer_event_tr e)⟩
  fault := fun k kind kn hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.fault (self := bound_kopst hb) k kind kn hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault k kind kn = ok r := by
      rw [← hb.fault]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (bfault_tr e)⟩
  fault_of := fun k kind t hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.fault_of (self := bound_kopst hb) k kind t hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of k kind t = ok r := by
      rw [← hb.fault_of]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (bfault_of_tr e)⟩
  record_event := fun k kind t hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.record_event (self := bound_kopst hb) k kind t hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event k kind t = ok r := by
      rw [← hb.record_event]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (brecord_event_tr e)⟩
  take_reinit := fun k p hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.take_reinit (self := bound_kopst hb) k p hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit k p = ok r := by
      rw [← hb.take_reinit]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (btake_reinit_tr (v := r.1) e)⟩
  take_stop_dma := fun k p hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.take_stop_dma (self := bound_kopst hb) k p hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma k p = ok r := by
      rw [← hb.take_stop_dma]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (btake_stop_dma_tr (v := r.1) e)⟩
  boot_failed := fun k f hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.boot_failed (self := bound_kopst hb) k f hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_failed k f = ok r := by
      rw [← hb.boot_failed]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (bboot_failed_tr e)⟩
  boot_info := fun k x y hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.boot_info (self := bound_kopst hb) k x y hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_info k x y = ok r := by
      rw [← hb.boot_info]; exact hr
    exact (WP.spec_ok _).mpr ⟨hp, hk.2.trans (bboot_info_tr e)⟩
  settle := fun k hk => by
    obtain ⟨r, hr, hp⟩ := spec_returns (KOpsT.settle (self := bound_kopst hb) k hk.1)
    rw [hr]
    have e : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle k = ok r := by
      rw [← hb.settle]; exact hr
    exact (WP.spec_ok _).mpr ⟨⟨hp.1, hk.2.trans (bsettle_tr (p := r.1) e)⟩, hp.2⟩
  validate := fun k => KOpsT.validate (self := bound_kopst hb) k

/-! ## The five entry cores -/

variable {A T : Type} {ai : arch.cm4.entry.KernelArch A} {ti : arch.cm4.entry.Tables T}

/-- **An SVC's writes to the linked Kernel are a sequence of the run's
functions.** -/
theorem svc_core_trace [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    {b b' : arch.bound.Bound NT NR NP NL NS NE} {hw hw' : A} {tables : T}
    {state st' : arch.cm4.plan.ArchState} {frame frame' : Array U32 8#usize} {number : U8}
    {r : _} (hk : BX b)
    (h : arch.cm4.entry.svc_core ki ai ti b hw tables state frame number
      = ok (r, b', hw', st', frame')) : Tr b b' := by
  haveI := bound_kopst_tr hb b
  have s := svc_core_t (ki := ki) (ai := ai) (ti := ti) b hw tables state frame number
    ⟨hk, Tr.rfl' b⟩
  rw [h] at s
  exact ((WP.spec_ok _).mp s).2

/-- **An interrupt entry's writes** (a dispatch, an intake, the time base). -/
theorem irq_entry_core_trace [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    {b b' : arch.bound.Bound NT NR NP NL NS NE} {hw hw' : A} {tables : T}
    {state st' : arch.cm4.plan.ArchState} {ipsr psp control er : U32}
    {r : _} (hk : BX b)
    (hcp : state.current_partition = 255#u8 ∨ state.current_partition.val < NP.val)
    (h : arch.cm4.entry.irq_entry_core ki ai ti b hw tables state ipsr psp control er
      = ok (r, b', hw', st')) : Tr b b' := by
  haveI := bound_kopst_tr hb b
  have s := irq_entry_core_t (ki := ki) (ai := ai) (ti := ti) b hw tables state ipsr psp control er
    ⟨hk, Tr.rfl' b⟩ hcp
  rw [h] at s
  exact ((WP.spec_ok _).mp s).2

/-- **The start's writes.** -/
theorem start_core_trace [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    {b b' : arch.bound.Bound NT NR NP NL NS NE} {hw hw' : A} {tables : T}
    {state st' : arch.cm4.plan.ArchState} {r : _} (hk : BX b)
    (h : arch.cm4.entry.start_core ki ai ti b hw tables state = ok (r, b', hw', st')) :
    Tr b b' := by
  haveI := bound_kopst_tr hb b
  have s := start_core_t (ki := ki) (ai := ai) (ti := ti) b hw tables state ⟨hk, Tr.rfl' b⟩
  rw [h] at s
  exact ((WP.spec_ok _).mp s).2

/-- **A fault entry's writes.** -/
theorem fault_core_trace [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    {b b' : arch.bound.Bound NT NR NP NL NS NE} {hw hw' : A} {tables : T}
    {state st' : arch.cm4.plan.ArchState} {kind : U8} {ink : Bool}
    {r : _} (hk : BX b)
    (h : arch.cm4.entry.fault_core ki ai ti b hw tables state kind ink = ok (r, b', hw', st')) :
    Tr b b' := by
  haveI := bound_kopst_tr hb b
  have s := fault_core_t (ki := ki) (ai := ai) (ti := ti) b hw tables state kind ink
    ⟨hk, Tr.rfl' b⟩
  rw [h] at s
  exact ((WP.spec_ok _).mp s).2

/-- **An unwind's writes** (the end of a Job). -/
theorem epilogue_core_trace [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    {b b' : arch.bound.Bound NT NR NP NL NS NE} {hw hw' : A} {tables : T}
    {state st' : arch.cm4.plan.ArchState} {n : U32}
    {r : _} (hk : BX b)
    (h : arch.cm4.entry.epilogue_core ki ai ti b hw tables state n = ok (r, b', hw', st')) :
    Tr b b' := by
  haveI := bound_kopst_tr hb b
  have s := epilogue_core_t (ki := ki) (ai := ai) (ti := ti) b hw tables state n ⟨hk, Tr.rfl' b⟩
  rw [h] at s
  exact ((WP.spec_ok _).mp s).2

/-- **The five entry cores together** (`CoreRet`, `Sched/Entries.lean`):
whichever returned `b'` when called on `b`, `Tr b b'`. `hcp`: an interrupt
entry is taken with the Partition taken as executing 255 or one of the
layout's. -/
theorem coreret_trace [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    {b b' : arch.bound.Bound NT NR NP NL NS NE} (hk : BX b)
    (hcp : ∀ (hw hw' : A) (tables : T) (state st' : arch.cm4.plan.ArchState)
      (ipsr psp control er : U32) r,
      arch.cm4.entry.irq_entry_core ki ai ti b hw tables state ipsr psp control er
        = ok (r, b', hw', st') →
      state.current_partition = 255#u8 ∨ state.current_partition.val < NP.val)
    (h : CoreRet ki ai ti b b') : Tr b b' := by
  rcases h with ⟨_, _, _, _, _, _, _, _, _, h⟩ | ⟨_, _, _, _, _, _, _, _, _, _, h⟩ |
    ⟨_, _, _, _, _, _, h⟩ | ⟨_, _, _, _, _, _, _, _, h⟩ | ⟨_, _, _, _, _, _, _, h⟩
  · exact svc_core_trace (ai := ai) (ti := ti) hb hk h
  · exact irq_entry_core_trace (ai := ai) (ti := ti) hb hk (hcp _ _ _ _ _ _ _ _ _ _ h) h
  · exact start_core_trace (ai := ai) (ti := ti) hb hk h
  · exact fault_core_trace (ai := ai) (ti := ti) hb hk h
  · exact epilogue_core_trace (ai := ai) (ti := ti) hb hk h

/-- **An entry of the cut run** (`EntryOf`): the Kernel state it leaves is
reached from the one it found by a sequence of the run's functions, and
the configuration it was stated to keep is kept by the code. -/
theorem entryof_lsteps [HwT ai] [TabT ti NT.val NP.val] (hb : IsBoundT ki)
    {cfg : logic.Config NT NR NP} {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (hbx : ∀ b : arch.bound.Bound NT NR NP NL NS NE, b.cfg = cfg → b.kernel = k → BX b)
    (hcp : ∀ (b b' : arch.bound.Bound NT NR NP NL NS NE) (hw hw' : A) (tables : T)
      (state st' : arch.cm4.plan.ArchState) (ipsr psp control er : U32) r,
      arch.cm4.entry.irq_entry_core ki ai ti b hw tables state ipsr psp control er
        = ok (r, b', hw', st') →
      state.current_partition = 255#u8 ∨ state.current_partition.val < NP.val)
    (h : EntryOf ki ai ti cfg k k') : LSteps cfg k k' := by
  obtain ⟨b, b', hc, hkk, _, hk', hr⟩ := h
  have t := coreret_trace (ai := ai) (ti := ti) hb (hbx b hc hkk) (hcp b b') hr
  have s := t.steps
  rwa [hc, hkk, hk'] at s

end P18
