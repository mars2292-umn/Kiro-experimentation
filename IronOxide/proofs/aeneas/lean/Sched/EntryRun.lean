import Sched.Script
import Sched.ScriptEpilogue
import Sched.Trace
open Aeneas Aeneas.Std Result

/-! # An execution of the linked Kernel, call by call, and the run that follows it (2026-11-27)

`Sched/Script.lean` gives, for any implementor of the interface, the
methods each entry core calls and their order. This file does three
things with it for the Kernel that is linked (`IsBoundT`).

**The Kernel's functions, in order.** `KW cfg l k k'` is what the method
`l` of `impl KernelOps for Bound` does to the Kernel's state, read from
its body: which functions of `rsk-logic` it calls, on which state, one
after another (`wr_kw`). `KApplied` is the same for `apply_core`'s part
(`settle`, then per Partition `ack_stop_dma` and `ack_reinit`), and
`KSh cfg ls k k'` for a whole path. `coreret_ksh`: **whichever of the
five entry cores returned `b'` when called on `b`, the configuration is
the same and `b'.kernel` is reached from `b.kernel` by one of the paths of
`ENTRY`**, each a fixed sequence of the Kernel's functions.

**The start.** `start_first`: the Kernel state the start entry leaves has
the scheduler state `Sched::new` returns, when the state it found had
(`Kernel::init`'s, `new_eff`): `Kernel::start`, `settle` and the
acknowledgements leave the scheduler state. So the state after the start
is a boot state of the run of `Sched/KRun.lean` (`KReach.boot`): the run
begins there, and needs no transition for `Kernel::start`.

**The run follows the execution, call by call.** `Follows I cfg P` is the
model's side, one statement for each function of the Kernel an entry
calls: from a state `x` of the run whose Kernel state is the one the
function is called on and that satisfies `I`, the run has transitions to
a state whose Kernel state is the function's result and that satisfies
`I`. `exec_follows`: **if the run follows each call, it follows every
execution**: for every boundary `b` of an execution from the first
boundary `b1` (`Exec`: entries of the linked Kernel one after another)
there is a state of the run, reached from the given first state by the
run's transitions, whose Kernel state is `b.kernel`. `exec_kreach` takes
the first state to be the boot state. The order of the calls and the
states they are made on are the code's; the theorem asks nothing of a
whole entry beyond what it asks of each call.

`I` is the reader's: the link between the run's accounts and the system
(an arrival awaiting its entry, the execution a Job still needs).

What is assumed, and stays assumed: `Follows`. For `observe`, the
charge, `settle`, the two acknowledgements, `fault` and `fault_of` the
run's transition (`observed`, `charged`, `settle`, `acked_reinit`,
`acked_stop`, `faulted`, `faulted_of`) has no premise but the function's
result, so that field asks only that `I` is kept. For the others the
run's transition has premises on the accounts and on time, which are the
model's statement about the system and are not derived from the code:
a served slot and an intake (an arrival awaiting its entry, the bound of
the Kernel section), a dispatch (that the Task is the one `Sched::select`
returns: the code checks that it is eligible, the interrupt controller
chooses it, `Sched/Nvic.lean`), a lock, an unlock, the end of a Job and an
Endpoint's record (that `Sched::select` returns no Task while a Job
executes, the execution left, the bounds). `Sched/Timer.lean` and
`Sched/Endpoint.lean` give the run's transition for a served slot and an
Endpoint operation from such premises. Not proven: that the Kernel's
configuration gives each Task the Priority the Analyzer was given.
Handwritten; no `sorry`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! ## The Kernel's functions behind each method -/

/-- The acknowledgements, Partition after Partition. -/
inductive KAcks (cfg : logic.Config NT NR NP) :
    logic.kernel.Kernel NT NR NP NL NS NE → logic.kernel.Kernel NT NR NP NL NS NE → Prop
  | done (k : logic.kernel.Kernel NT NR NP NL NS NE) : KAcks cfg k k
  | step (k k1 k2 k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (h1 : k1 = k ∨ ∃ p, logic.kernel.Kernel.ack_stop_dma k cfg p = ok k1)
      (h2 : k2 = k1 ∨ ∃ p, logic.kernel.Kernel.ack_reinit k1 cfg p = ok k2)
      (h : KAcks cfg k2 k') : KAcks cfg k k'

/-- `apply_core`'s part: `settle`, then the acknowledgements. -/
def KApplied (cfg : logic.Config NT NR NP)
    (k k' : logic.kernel.Kernel NT NR NP NL NS NE) : Prop :=
  ∃ (r : Option U8 × Bool) (k1 : logic.kernel.Kernel NT NR NP NL NS NE),
    logic.kernel.Kernel.settle k cfg = ok (r, k1) ∧ KAcks cfg k1 k'

/-- What the item `l` of a path does to the Kernel's state, for the linked
interface. -/
def KW (cfg : logic.Config NT NR NP) :
    Lbl → logic.kernel.Kernel NT NR NP NL NS NE → logic.kernel.Kernel NT NR NP NL NS NE → Prop
  | .start, k, k' => k' = k ∨ (k.started = false ∧ logic.kernel.Kernel.start k cfg = ok k')
  | .dispatch, k, k' => ∃ k1 r,
      (k1 = k ∨ (k.started = true ∧ ∃ t pc, logic.kernel.Kernel.dispatch k cfg t pc = ok k1)) ∧
      logic.kernel.Kernel.settle k1 cfg = ok (r, k')
  | .ret, k, k' => k' = k ∨
      (k.sched.depth > 0#usize ∧ ∃ ec, logic.kernel.Kernel.job_return k cfg ec = ok k')
  | .lock, k, k' => k' = k ∨ ∃ r v, logic.kernel.Kernel.svc_lock k cfg r = ok (v, k')
  | .unlock, k, k' => k' = k ∨ ∃ r v, logic.kernel.Kernel.svc_unlock k cfg r = ok (v, k')
  | .intake, k, k' => k' = k ∨ ∃ t sp now off k1 k2,
      logic.kernel.Kernel.observe k cfg now = ok k1 ∧
      (k2 = k1 ∨ ∃ sd, logic.kernel.Kernel.deadline_miss k1 cfg t sd = ok k2) ∧
      logic.kernel.Kernel.intake_release k2 cfg t sp now off = ok k'
  | .timer, k, k' => ∃ now k0, logic.kernel.Kernel.observe k cfg now = ok k0 ∧
      (k' = k0 ∨ ∃ kc,
        (kc = k0 ∨ ∃ t rc b, logic.budget.Budget.charge k0.budget t rc = ok b ∧
          kc = { k0 with budget := b }) ∧
        ServedChain cfg kc k')
  | .fault, k, k' => ∃ kind kn, logic.kernel.Kernel.fault k cfg kind kn = ok k'
  | .fault_of, k, k' => k' = k ∨ ∃ kind t, logic.kernel.Kernel.fault_of k cfg kind t = ok k'
  | .record, k, k' => k' = k ∨ ∃ kind t, logic.kernel.Kernel.record_event k cfg kind t = ok k'
  | .apply, k, k' => KApplied cfg k k'

/-- A path on the Kernel's state. -/
inductive KSh (cfg : logic.Config NT NR NP) : List Lbl →
    logic.kernel.Kernel NT NR NP NL NS NE → logic.kernel.Kernel NT NR NP NL NS NE → Prop
  | nil (k : logic.kernel.Kernel NT NR NP NL NS NE) : KSh cfg [] k k
  | cons (l : Lbl) (ls : List Lbl) (k k1 k' : logic.kernel.Kernel NT NR NP NL NS NE)
      (h : KW cfg l k k1) (hs : KSh cfg ls k1 k') : KSh cfg (l :: ls) k k'

variable {ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)}

theorem take_stop_dma_k {b b' : arch.bound.Bound NT NR NP NL NS NE} {p : U8} {v : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma b p = ok (v, b')) :
    b'.cfg = b.cfg ∧
    (b'.kernel = b.kernel ∨ ∃ p, logic.kernel.Kernel.ack_stop_dma b.kernel b.cfg p = ok b'.kernel) := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma at h
  simp only [lift, bind_ok] at h
  split at h
  · have h := ok_inj h; cases h; exact ⟨rfl, Or.inl rfl⟩
  · simp only [bind_eq_ok] at h
    obtain ⟨v1, _, h⟩ := h
    split at h
    · simp only [bind_eq_ok] at h
      obtain ⟨k, hk, h⟩ := h
      have h := ok_inj h; cases h
      exact ⟨rfl, Or.inr ⟨_, hk⟩⟩
    · have h := ok_inj h; cases h; exact ⟨rfl, Or.inl rfl⟩

theorem take_reinit_k {b b' : arch.bound.Bound NT NR NP NL NS NE} {p : U8} {v : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit b p = ok (v, b')) :
    b'.cfg = b.cfg ∧
    (b'.kernel = b.kernel ∨ ∃ p, logic.kernel.Kernel.ack_reinit b.kernel b.cfg p = ok b'.kernel) := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit at h
  simp only [lift, bind_ok] at h
  split at h
  · have h := ok_inj h; cases h; exact ⟨rfl, Or.inl rfl⟩
  · simp only [bind_eq_ok] at h
    obtain ⟨v1, _, h⟩ := h
    split at h
    · simp only [bind_eq_ok] at h
      obtain ⟨k, hk, h⟩ := h
      have h := ok_inj h; cases h
      exact ⟨rfl, Or.inr ⟨_, hk⟩⟩
    · have h := ok_inj h; cases h; exact ⟨rfl, Or.inl rfl⟩

theorem acks_kacks (hb : IsBoundT ki) {b b' : arch.bound.Bound NT NR NP NL NS NE}
    (h : Acks ki b b') : b'.cfg = b.cfg ∧ KAcks b.cfg b.kernel b'.kernel := by
  induction h with
  | done b => exact ⟨rfl, .done _⟩
  | step b b1 b2 b' i j v v1 h1 h2 _ ih =>
    rw [hb.take_stop_dma] at h1
    rw [hb.take_reinit] at h2
    obtain ⟨c1, d1⟩ := take_stop_dma_k h1
    obtain ⟨c2, d2⟩ := take_reinit_k h2
    obtain ⟨c3, d3⟩ := ih
    rw [c2, c1] at c3 d3
    rw [c1] at d2
    exact ⟨c3, .step _ _ _ _ d1 d2 d3⟩

theorem applied_kapplied (hb : IsBoundT ki) {b b' : arch.bound.Bound NT NR NP NL NS NE}
    (h : Applied ki b b') : b'.cfg = b.cfg ∧ KApplied b.cfg b.kernel b'.kernel := by
  obtain ⟨r, b1, hs, ha⟩ := h
  rw [hb.settle] at hs
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle at hs
  simp only [bind_eq_ok] at hs
  obtain ⟨⟨p1, k⟩, hk, hs⟩ := hs
  have hs := ok_inj hs; cases hs
  obtain ⟨c, d⟩ := acks_kacks hb ha
  exact ⟨c, _, _, hk, d⟩

/-- **Each method of the linked interface, as the Kernel's functions.** -/
theorem wr_kw (hb : IsBoundT ki) {l : Lbl} {b b' : arch.bound.Bound NT NR NP NL NS NE}
    (h : Wr ki l b b') : b'.cfg = b.cfg ∧ KW b.cfg l b.kernel b'.kernel := by
  cases h with
  | start _ _ h =>
    rw [hb.start] at h
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start at h
    split at h
    · have h := ok_inj h; subst h; exact ⟨rfl, Or.inl rfl⟩
    · simp only [bind_eq_ok] at h
      obtain ⟨k, hk, h⟩ := h
      have h := ok_inj h; subst h
      exact ⟨rfl, Or.inr ⟨by simpa using ‹¬ b.kernel.started = true›, hk⟩⟩
  | dispatch _ _ t pc h =>
    rw [hb.dispatch] at h
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch at h
    simp only [bind_eq_ok] at h
    obtain ⟨k, hk, ⟨r, k1⟩, hs, h⟩ := h
    have h := ok_inj h; subst h
    refine ⟨rfl, k, r, ?_, hs⟩
    split at hk
    · simp only [bind_eq_ok] at hk
      obtain ⟨e, _, hk⟩ := hk
      split at hk
      · exact Or.inr ⟨by assumption, _, _, hk⟩
      · exact Or.inl (ok_inj hk).symm
    · exact Or.inl (ok_inj hk).symm
  | ret _ _ ec h =>
    rw [hb.job_return] at h
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return at h
    split at h
    · simp only [bind_eq_ok] at h
      obtain ⟨k, hk, h⟩ := h
      have h := ok_inj h; subst h
      exact ⟨rfl, Or.inr ⟨by assumption, _, hk⟩⟩
    · have h := ok_inj h; subst h; exact ⟨rfl, Or.inl rfl⟩
  | lock _ _ r v h =>
    rw [hb.svc_lock] at h
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock at h
    split at h
    · have h := ok_inj h; cases h; exact ⟨rfl, Or.inl rfl⟩
    · simp only [lift, bind_ok] at h
      split at h
      · have h := ok_inj h; cases h; exact ⟨rfl, Or.inl rfl⟩
      · simp only [bind_eq_ok] at h
        obtain ⟨⟨v1, k⟩, hk, h⟩ := h
        have h := ok_inj h; cases h
        exact ⟨rfl, Or.inr ⟨_, _, hk⟩⟩
  | unlock _ _ r v h =>
    rw [hb.svc_unlock] at h
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock at h
    split at h
    · have h := ok_inj h; cases h; exact ⟨rfl, Or.inl rfl⟩
    · simp only [bind_eq_ok] at h
      obtain ⟨⟨v1, k⟩, hk, h⟩ := h
      have h := ok_inj h; cases h
      exact ⟨rfl, Or.inr ⟨_, _, hk⟩⟩
  | intake _ _ t sp now off h =>
    rw [hb.intake] at h
    obtain ⟨hc, e | ⟨off1, hi⟩⟩ := bound_intake h
    · exact ⟨hc, Or.inl e⟩
    · obtain ⟨k1, k2, hobs, hk2, hrel⟩ := kernel_intake_cases hi
      exact ⟨hc, Or.inr ⟨_, _, _, _, k1, k2, hobs, hk2, hrel⟩⟩
  | timer _ _ now rc h =>
    rw [hb.timer_event] at h
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event at h
    simp only [bind_eq_ok] at h
    obtain ⟨now1, _, k, hk, h⟩ := h
    have h := ok_inj h; subst h
    refine ⟨rfl, ?_⟩
    unfold logic.kernel.Kernel.timer_event at hk
    simp only [bind_eq_ok] at hk
    obtain ⟨k0, hobs, hk⟩ := hk
    refine ⟨now1, k0, hobs, ?_⟩
    split at hk
    · exact Or.inl (ok_inj hk).symm
    · simp only [bind_eq_ok] at hk
      obtain ⟨bg, hbg, r, hl0, hl1⟩ := hk
      refine Or.inr ⟨{ k0 with budget := bg }, ?_, timer_loops_served hl0 hl1⟩
      split at hbg
      · simp only [bind_eq_ok] at hbg
        obtain ⟨i, _, t, _, hbg⟩ := hbg
        exact Or.inr ⟨t, rc, bg, hbg, rfl⟩
      · have hbg := ok_inj hbg; subst hbg; exact Or.inl rfl
  | fault _ _ kind kn h =>
    rw [hb.fault] at h
    obtain ⟨hk, hc⟩ := bound_fault h
    exact ⟨hc, _, _, hk⟩
  | fault_of _ _ kind t h =>
    rw [hb.fault_of] at h
    unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of at h
    simp only [lift, bind_ok] at h
    split at h
    · simp only [bind_eq_ok] at h
      obtain ⟨k, hk, h⟩ := h
      have h := ok_inj h; subst h
      exact ⟨rfl, Or.inr ⟨_, _, hk⟩⟩
    · have h := ok_inj h; subst h; exact ⟨rfl, Or.inl rfl⟩
  | record _ _ kind t h =>
    rw [hb.record_event] at h
    obtain ⟨hc, e | hk⟩ := bound_record_event h
    · exact ⟨hc, Or.inl e⟩
    · exact ⟨hc, Or.inr ⟨_, _, hk⟩⟩
  | apply _ _ h => exact applied_kapplied hb h

/-- **A path of the linked interface, as the Kernel's functions.** -/
theorem sh_ksh (hb : IsBoundT ki) {ls : List Lbl}
    {b b' : arch.bound.Bound NT NR NP NL NS NE} (h : Sh ki ls b b') :
    b'.cfg = b.cfg ∧ KSh b.cfg ls b.kernel b'.kernel := by
  induction h with
  | nil b => exact ⟨rfl, .nil _⟩
  | cons l ls b b1 b' h1 _ ih =>
    obtain ⟨c1, d1⟩ := wr_kw hb h1
    obtain ⟨c2, d2⟩ := ih
    rw [c1] at c2 d2
    exact ⟨c2, .cons _ _ _ _ _ d1 d2⟩

/-! ## The scheduler state through `apply_core`'s part and the start -/

theorem kacks_sched {cfg : logic.Config NT NR NP}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE} (h : KAcks cfg k k') : k'.sched = k.sched := by
  induction h with
  | done k => rfl
  | step k k1 k2 k' h1 h2 _ ih =>
    have e1 : k1.sched = k.sched := by
      rcases h1 with rfl | ⟨p, h⟩
      · rfl
      · exact (ack_stop_dma_frame h).1
    have e2 : k2.sched = k1.sched := by
      rcases h2 with rfl | ⟨p, h⟩
      · rfl
      · exact (ack_reinit_frame h).1
    rw [ih, e2, e1]

theorem kapplied_sched {cfg : logic.Config NT NR NP}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE} (h : KApplied cfg k k') :
    k'.sched = k.sched := by
  obtain ⟨r, k1, hs, ha⟩ := h
  rw [kacks_sched ha, settle_sched hs]

variable {A T : Type} {ai : arch.cm4.entry.KernelArch A} {ti : arch.cm4.entry.Tables T}

/-- **The state the start entry leaves is a boot state of the run.** When
the state the start found has the scheduler state `Sched::new` returns
(`Kernel::init`'s result has, `new_eff`), so has the state it leaves, with
the same configuration. -/
theorem start_first (hb : IsBoundT ki) {b0 b1 : arch.bound.Bound NT NR NP NL NS NE}
    {hw hw' : A} {tables : T} {state st' : arch.cm4.plan.ArchState} {r : _}
    (h0 : logic.sched.Sched.new NL b0.cfg = ok b0.kernel.sched)
    (h : arch.cm4.entry.start_core ki ai ti b0 hw tables state = ok (r, b1, hw', st')) :
    b1.cfg = b0.cfg ∧ logic.sched.Sched.new NL b1.cfg = ok b1.kernel.sched := by
  obtain ⟨ls, hm, hs⟩ := start_core_sh ki ai ti h
  simp only [List.mem_singleton] at hm
  subst hm
  obtain ⟨c, d⟩ := sh_ksh hb hs
  refine ⟨c, ?_⟩
  cases d with
  | cons _ _ _ k1 _ h1 h2 =>
    cases h2 with
    | cons _ _ _ k2 _ ha h3 =>
      cases h3 with
      | nil =>
        have e1 : k1.sched = b0.kernel.sched := by
          rcases h1 with e | ⟨_, hst⟩
          · rw [e]
          · exact kernel_start_sched hst
        rw [c, kapplied_sched ha, e1]
        exact h0

/-! ## The run follows an entry, call by call -/

theorem KSteps.trans {cfg : logic.Config NT NR NP} {P : Params}
    {x y z : KSt NT NR NP NL NS NE} (h1 : KSteps cfg P x y) (h2 : KSteps cfg P y z) :
    KSteps cfg P x z := by
  induction h2 with
  | refl => exact h1
  | step y z _ hs ih => exact .step _ _ _ ih hs

/-- The run has transitions from `x` to a state whose Kernel state is `k'`
and that satisfies `I`. -/
def Fol (I : KSt NT NR NP NL NS NE → Prop) (cfg : logic.Config NT NR NP) (P : Params)
    (x : KSt NT NR NP NL NS NE) (k' : logic.kernel.Kernel NT NR NP NL NS NE) : Prop :=
  ∃ y, KSteps cfg P x y ∧ y.k = k' ∧ I y

theorem Fol.here {I : KSt NT NR NP NL NS NE → Prop} {cfg : logic.Config NT NR NP} {P : Params}
    {x : KSt NT NR NP NL NS NE} (h : I x) : Fol I cfg P x x.k := ⟨x, .refl x, rfl, h⟩

theorem Fol.then {I : KSt NT NR NP NL NS NE → Prop} {cfg : logic.Config NT NR NP} {P : Params}
    {x : KSt NT NR NP NL NS NE} {k1 k2 : logic.kernel.Kernel NT NR NP NL NS NE}
    (h1 : Fol I cfg P x k1) (h2 : ∀ y, I y → y.k = k1 → Fol I cfg P y k2) : Fol I cfg P x k2 := by
  obtain ⟨y, hs, hk, hi⟩ := h1
  obtain ⟨z, hs2, hk2, hi2⟩ := h2 y hi hk
  exact ⟨z, hs.trans hs2, hk2, hi2⟩

/-- **The model's side, call by call.** For each function of the Kernel an
entry calls: from a state of the run that satisfies `I` and whose Kernel
state is the one the function is called on, the run reaches a state that
satisfies `I` and whose Kernel state is the function's result. What each
field is given beside the call is what the code has established where it
makes the call. `served` is told whether the slot is the first a timer
entry serves, and then that the Kernel state is quieted. -/
structure Follows (I : KSt NT NR NP NL NS NE → Prop) (cfg : logic.Config NT NR NP) (P : Params) :
    Prop where
  start : ∀ (x : KSt NT NR NP NL NS NE) k', I x → x.k.started = false →
    logic.kernel.Kernel.start x.k cfg = ok k' → Fol I cfg P x k'
  dispatch : ∀ (x : KSt NT NR NP NL NS NE) t pc k', I x → x.k.started = true →
    logic.kernel.Kernel.dispatch x.k cfg t pc = ok k' → Fol I cfg P x k'
  settle : ∀ (x : KSt NT NR NP NL NS NE) r k', I x →
    logic.kernel.Kernel.settle x.k cfg = ok (r, k') → Fol I cfg P x k'
  ret : ∀ (x : KSt NT NR NP NL NS NE) ec k', I x → x.k.sched.depth > 0#usize →
    logic.kernel.Kernel.job_return x.k cfg ec = ok k' → Fol I cfg P x k'
  lock : ∀ (x : KSt NT NR NP NL NS NE) r v k', I x →
    logic.kernel.Kernel.svc_lock x.k cfg r = ok (v, k') → Fol I cfg P x k'
  unlock : ∀ (x : KSt NT NR NP NL NS NE) r v k', I x →
    logic.kernel.Kernel.svc_unlock x.k cfg r = ok (v, k') → Fol I cfg P x k'
  observe : ∀ (x : KSt NT NR NP NL NS NE) now k', I x →
    logic.kernel.Kernel.observe x.k cfg now = ok k' → Fol I cfg P x k'
  charge : ∀ (x : KSt NT NR NP NL NS NE) t rc b, I x →
    logic.budget.Budget.charge x.k.budget t rc = ok b → Fol I cfg P x { x.k with budget := b }
  served : ∀ (x : KSt NT NR NP NL NS NE) (first : Bool) i k', I x →
    (first = true → Quieted cfg x.k) → SlotDue x.k i →
    logic.kernel.Kernel.serve_slot x.k cfg i = ok k' → Fol I cfg P x k'
  deadline : ∀ (x : KSt NT NR NP NL NS NE) t sd k', I x →
    logic.kernel.Kernel.deadline_miss x.k cfg t sd = ok k' → Fol I cfg P x k'
  intake : ∀ (x : KSt NT NR NP NL NS NE) t sp now off k', I x →
    logic.kernel.Kernel.intake_release x.k cfg t sp now off = ok k' → Fol I cfg P x k'
  fault : ∀ (x : KSt NT NR NP NL NS NE) kind kn k', I x →
    logic.kernel.Kernel.fault x.k cfg kind kn = ok k' → Fol I cfg P x k'
  fault_of : ∀ (x : KSt NT NR NP NL NS NE) kind t k', I x →
    logic.kernel.Kernel.fault_of x.k cfg kind t = ok k' → Fol I cfg P x k'
  record : ∀ (x : KSt NT NR NP NL NS NE) kind t k', I x →
    logic.kernel.Kernel.record_event x.k cfg kind t = ok k' → Fol I cfg P x k'
  ack_stop : ∀ (x : KSt NT NR NP NL NS NE) p k', I x →
    logic.kernel.Kernel.ack_stop_dma x.k cfg p = ok k' → Fol I cfg P x k'
  ack_reinit : ∀ (x : KSt NT NR NP NL NS NE) p k', I x →
    logic.kernel.Kernel.ack_reinit x.k cfg p = ok k' → Fol I cfg P x k'

section follows
variable {I : KSt NT NR NP NL NS NE → Prop} {cfg : logic.Config NT NR NP} {P : Params}

/-! The calls whose transition of the run has no premise but the
function's result: the field of `Follows` asks only that `I` holds of the
state the transition gives. -/

theorem Fol.one {x y : KSt NT NR NP NL NS NE} (hs : KStep cfg P x y) (hI : I y) :
    Fol I cfg P x y.k := ⟨y, .step _ _ _ (.refl x) hs, rfl, hI⟩

theorem fol_observe {x : KSt NT NR NP NL NS NE} {now : U64}
    {k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.observe x.k cfg now = ok k') (hI : I { x with k := k' }) :
    Fol I cfg P x k' := Fol.one (KStep.observed x now k' h) hI

theorem fol_charge {x : KSt NT NR NP NL NS NE} {t : U8} {rc : U32} {b : logic.budget.Budget NT}
    (h : logic.budget.Budget.charge x.k.budget t rc = ok b)
    (hI : I { x with k := { x.k with budget := b } }) :
    Fol I cfg P x { x.k with budget := b } := Fol.one (KStep.charged x t rc b h) hI

theorem fol_settle {x : KSt NT NR NP NL NS NE} {r : Option U8 × Bool}
    {k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.settle x.k cfg = ok (r, k')) (hI : I { x with k := k' }) :
    Fol I cfg P x k' := Fol.one (KStep.settle x r k' h) hI

theorem fol_ack_stop {x : KSt NT NR NP NL NS NE} {p : U8}
    {k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.ack_stop_dma x.k cfg p = ok k') (hI : I { x with k := k' }) :
    Fol I cfg P x k' := Fol.one (KStep.acked_stop x p k' h) hI

theorem fol_ack_reinit {x : KSt NT NR NP NL NS NE} {p : U8}
    {k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.ack_reinit x.k cfg p = ok k') (hI : I { x with k := k' }) :
    Fol I cfg P x k' := Fol.one (KStep.acked_reinit x p k' h) hI

theorem fol_fault {x : KSt NT NR NP NL NS NE} {kind : U8} {kn : Bool}
    {k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.fault x.k cfg kind kn = ok k')
    (hI : I { x with k := k', run := normRun k'.sched x.run, pend := normPend k'.sched x.pend }) :
    Fol I cfg P x k' := Fol.one (KStep.faulted x kind kn k' h) hI

theorem fol_fault_of {x : KSt NT NR NP NL NS NE} {kind t : U8}
    {k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.fault_of x.k cfg kind t = ok k')
    (hI : I { x with k := k', run := normRun k'.sched x.run, pend := normPend k'.sched x.pend }) :
    Fol I cfg P x k' := Fol.one (KStep.faulted_of x kind t k' h) hI

/-- The end of a killed Job's handler (`unwind`): the same, for
`job_return`. The end of a Job that ran to its end is the run's `finish`,
which has premises. -/
theorem fol_unwind {x : KSt NT NR NP NL NS NE} {ec : U32}
    {k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.job_return x.k cfg ec = ok k')
    (hI : I { x with k := k', run := Function.update x.run x.top 0 }) :
    Fol I cfg P x k' := Fol.one (KStep.unwind x ec k' h) hI

theorem served_fol (F : Follows I cfg P) {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : ServedChain cfg k k') :
    ∀ (x : KSt NT NR NP NL NS NE) (first : Bool), I x → x.k = k →
      (first = true → Quieted cfg k) → Fol I cfg P x k' := by
  induction h with
  | done k => intro x _ hI hk _; subst hk; exact Fol.here hI
  | serve k k1 k' i hdue hs _ ih =>
    intro x first hI hk hq
    subst hk
    exact (F.served x first i k1 hI hq hdue hs).then fun y hy hyk =>
      ih y false hy hyk (fun e => by cases e)

theorem kacks_fol (F : Follows I cfg P) {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : KAcks cfg k k') :
    ∀ (x : KSt NT NR NP NL NS NE), I x → x.k = k → Fol I cfg P x k' := by
  induction h with
  | done k => intro x hI hk; subst hk; exact Fol.here hI
  | step k k1 k2 k' h1 h2 _ ih =>
    intro x hI hk
    subst hk
    have f1 : Fol I cfg P x k1 := by
      rcases h1 with rfl | ⟨p, h⟩
      · exact Fol.here hI
      · exact F.ack_stop x p k1 hI h
    refine f1.then fun y hy hyk => ?_
    subst hyk
    have f2 : Fol I cfg P y k2 := by
      rcases h2 with rfl | ⟨p, h⟩
      · exact Fol.here hy
      · exact F.ack_reinit y p k2 hy h
    exact f2.then fun z hz hzk => ih z hz hzk

theorem kapplied_fol (F : Follows I cfg P) {x : KSt NT NR NP NL NS NE}
    {k' : logic.kernel.Kernel NT NR NP NL NS NE} (hI : I x) (h : KApplied cfg x.k k') :
    Fol I cfg P x k' := by
  obtain ⟨r, k1, hs, ha⟩ := h
  exact (F.settle x r k1 hI hs).then fun y hy hyk => kacks_fol F ha y hy hyk

/-- **The run follows one method's calls.** -/
theorem kw_fol (F : Follows I cfg P) {l : Lbl} {x : KSt NT NR NP NL NS NE}
    {k' : logic.kernel.Kernel NT NR NP NL NS NE} (first : Bool) (hI : I x)
    (hq : first = true → Quieted cfg x.k) (h : KW cfg l x.k k') : Fol I cfg P x k' := by
  cases l with
  | start =>
    rcases h with rfl | ⟨hs, h⟩
    · exact Fol.here hI
    · exact F.start x k' hI hs h
  | dispatch =>
    obtain ⟨k1, r, hd, hs⟩ := h
    have f1 : Fol I cfg P x k1 := by
      rcases hd with rfl | ⟨hst, t, pc, hd⟩
      · exact Fol.here hI
      · exact F.dispatch x t pc k1 hI hst hd
    refine f1.then fun y hy hyk => ?_
    subst hyk
    exact F.settle y r k' hy hs
  | ret =>
    rcases h with rfl | ⟨hd, ec, h⟩
    · exact Fol.here hI
    · exact F.ret x ec k' hI hd h
  | lock =>
    rcases h with rfl | ⟨r, v, h⟩
    · exact Fol.here hI
    · exact F.lock x r v k' hI h
  | unlock =>
    rcases h with rfl | ⟨r, v, h⟩
    · exact Fol.here hI
    · exact F.unlock x r v k' hI h
  | intake =>
    rcases h with rfl | ⟨t, sp, now, off, k1, k2, hobs, hk2, hrel⟩
    · exact Fol.here hI
    · refine (F.observe x now k1 hI hobs).then fun y hy hyk => ?_
      subst hyk
      have f2 : Fol I cfg P y k2 := by
        rcases hk2 with rfl | ⟨sd, hd⟩
        · exact Fol.here hy
        · exact F.deadline y t sd k2 hy hd
      refine f2.then fun z hz hzk => ?_
      subst hzk
      exact F.intake z t sp now off k' hz hrel
  | timer =>
    obtain ⟨now, k0, hobs, h⟩ := h
    refine (F.observe x now k0 hI hobs).then fun y hy hyk => ?_
    subst hyk
    have hq0 : first = true → Quieted cfg y.k := fun e => observe_quieted hobs (hq e)
    rcases h with rfl | ⟨kc, hc, hch⟩
    · exact Fol.here hy
    · have f2 : Fol I cfg P y kc := by
        rcases hc with rfl | ⟨t, rc, b, hc, rfl⟩
        · exact Fol.here hy
        · exact F.charge y t rc b hy hc
      have hqc : first = true → Quieted cfg kc := by
        intro e
        rcases hc with rfl | ⟨t, rc, b, hc, rfl⟩
        · exact hq0 e
        · exact Quieted.frame rfl rfl (hq0 e)
      exact f2.then fun z hz hzk => served_fol F hch z first hz hzk hqc
  | fault =>
    obtain ⟨kind, kn, h⟩ := h
    exact F.fault x kind kn k' hI h
  | fault_of =>
    rcases h with rfl | ⟨kind, t, h⟩
    · exact Fol.here hI
    · exact F.fault_of x kind t k' hI h
  | record =>
    rcases h with rfl | ⟨kind, t, h⟩
    · exact Fol.here hI
    · exact F.record x kind t k' hI h
  | apply => exact kapplied_fol F hI h

/-- **The run follows a path.** The first item of the path is called on
the state the entry found, which is quieted. -/
theorem ksh_fol (F : Follows I cfg P) {ls : List Lbl}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE} (h : KSh cfg ls k k') :
    ∀ (x : KSt NT NR NP NL NS NE) (first : Bool), I x → x.k = k →
      (first = true → Quieted cfg k) → Fol I cfg P x k' := by
  induction h with
  | nil k => intro x _ hI hk _; subst hk; exact Fol.here hI
  | cons l ls k k1 k' h1 _ ih =>
    intro x first hI hk hq
    subst hk
    exact (kw_fol F first hI hq h1).then fun y hy hyk =>
      ih y false hy hyk (fun e => by cases e)

end follows

/-! ## The five entry cores, and an execution -/

/-- The paths of an entry: the methods called and `apply_core`, in order. -/
def ENTRY : List (List Lbl) :=
  [[], [.apply], [.fault, .apply], [.intake, .apply], [.record, .apply], [.fault_of, .apply],
   [.ret, .apply], [.ret, .apply, .fault, .apply], [.lock], [.unlock, .apply], [.dispatch],
   [.dispatch, .fault, .apply], [.fault, .intake, .apply], [.timer, .apply], [.start, .apply]]

/-- **Whichever entry core returned `b'` when called on `b`, it called the
interface's methods along one of the paths of `ENTRY`.** For any
implementor of the interface. -/
theorem coreret_en {K : Type} {ki : arch.bound.KernelOps K} {b b' : K}
    (h : (∃ (hw hw' : A) (tables : T) (state st' : arch.cm4.plan.ArchState)
        (frame frame' : Array U32 8#usize) (number : U8), ∃ r,
      arch.cm4.entry.svc_core ki ai ti b hw tables state frame number
        = ok (r, b', hw', st', frame')) ∨
    (∃ (hw hw' : A) (tables : T) (state st' : arch.cm4.plan.ArchState)
        (ipsr psp control er : U32), ∃ r,
      arch.cm4.entry.irq_entry_core ki ai ti b hw tables state ipsr psp control er
        = ok (r, b', hw', st')) ∨
    (∃ (hw hw' : A) (tables : T) (state st' : arch.cm4.plan.ArchState), ∃ r,
      arch.cm4.entry.start_core ki ai ti b hw tables state = ok (r, b', hw', st')) ∨
    (∃ (hw hw' : A) (tables : T) (state st' : arch.cm4.plan.ArchState)
        (kind : U8) (ink : Bool), ∃ r,
      arch.cm4.entry.fault_core ki ai ti b hw tables state kind ink
        = ok (r, b', hw', st')) ∨
    (∃ (hw hw' : A) (tables : T) (state st' : arch.cm4.plan.ArchState)
        (n : U32), ∃ r,
      arch.cm4.entry.epilogue_core ki ai ti b hw tables state n
        = ok (r, b', hw', st'))) : En ki ENTRY b b' := by
  rcases h with ⟨_, _, _, _, _, _, _, _, _, h⟩ | ⟨_, _, _, _, _, _, _, _, _, _, h⟩ |
    ⟨_, _, _, _, _, _, h⟩ | ⟨_, _, _, _, _, _, _, _, h⟩ | ⟨_, _, _, _, _, _, _, h⟩
  · exact (svc_core_sh ki ai ti h).mono (by decide)
  · exact (irq_entry_core_sh ki ai ti h).mono (by decide)
  · exact (start_core_sh ki ai ti h).mono (by decide)
  · exact (fault_core_sh ki ai ti h).mono (by decide)
  · exact (epilogue_core_sh ki ai ti h).mono (by decide)

/-- **An entry of the linked Kernel, as the Kernel's functions**: the
configuration is kept, and the Kernel's state is reached along one of the
paths of `ENTRY`, each method being the functions `KW` gives it. -/
theorem coreret_ksh (hb : IsBoundT ki) {b b' : arch.bound.Bound NT NR NP NL NS NE}
    (h : CoreRet ki ai ti b b') :
    b'.cfg = b.cfg ∧ ∃ ls, ls ∈ ENTRY ∧ KSh b.cfg ls b.kernel b'.kernel := by
  obtain ⟨ls, hm, hs⟩ := coreret_en (ai := ai) (ti := ti) h
  obtain ⟨c, d⟩ := sh_ksh hb hs
  exact ⟨c, ls, hm, d⟩

theorem isboundt_qbound (hb : IsBoundT ki) : QBound ki :=
  ⟨hb.settle, hb.take_stop_dma, hb.take_reinit, hb.dispatch, hb.svc_lock⟩

/-- **The run follows an entry.** From a state of the run whose Kernel
state is the one the entry found, quieted, the run reaches a state whose
Kernel state is the one the entry left. -/
theorem entry_follows {I : KSt NT NR NP NL NS NE → Prop} {P : Params}
    (hb : IsBoundT ki) {b b' : arch.bound.Bound NT NR NP NL NS NE}
    (F : Follows I b.cfg P) (h : CoreRet ki ai ti b b')
    {x : KSt NT NR NP NL NS NE} (hI : I x) (hx : x.k = b.kernel) (hq : Quieted b.cfg b.kernel) :
    b'.cfg = b.cfg ∧ Quieted b'.cfg b'.kernel ∧ Fol I b.cfg P x b'.kernel := by
  obtain ⟨c, ls, _, d⟩ := coreret_ksh (ai := ai) (ti := ti) hb h
  exact ⟨c, coreret_quieted ki ai ti (isboundt_qbound hb) h hq,
    ksh_fol F d x true hI hx (fun _ => hq)⟩

/-- The boundaries of an execution: the states of the linked Kernel from
`b1`, each the result of an entry core called on the one before. -/
inductive Exec (ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE))
    (ai : arch.cm4.entry.KernelArch A) (ti : arch.cm4.entry.Tables T)
    (b1 : arch.bound.Bound NT NR NP NL NS NE) : arch.bound.Bound NT NR NP NL NS NE → Prop
  | first : Exec ki ai ti b1 b1
  | entry (b b' : arch.bound.Bound NT NR NP NL NS NE) (h : Exec ki ai ti b1 b)
      (he : CoreRet ki ai ti b b') : Exec ki ai ti b1 b'

/-- **If the run follows each call, it follows every execution.** For
every boundary `b` of an execution from `b1`: the configuration is
`b1`'s, the Kernel is quieted, and the run has transitions from the given
first state to a state whose Kernel state is `b.kernel` and that satisfies
`I`. -/
theorem exec_follows {I : KSt NT NR NP NL NS NE → Prop} {P : Params}
    (hb : IsBoundT ki) {b1 b : arch.bound.Bound NT NR NP NL NS NE}
    (F : Follows I b1.cfg P) {x1 : KSt NT NR NP NL NS NE} (hI : I x1) (hx : x1.k = b1.kernel)
    (hq : Quieted b1.cfg b1.kernel) (h : Exec ki ai ti b1 b) :
    b.cfg = b1.cfg ∧ Quieted b.cfg b.kernel ∧ Fol I b1.cfg P x1 b.kernel := by
  induction h with
  | first => exact ⟨rfl, hq, hx ▸ Fol.here hI⟩
  | entry b b' _ he ih =>
    obtain ⟨c, q, y, hs, hk, hy⟩ := ih
    have F' : Follows I b.cfg P := c ▸ F
    obtain ⟨c', q', f'⟩ := entry_follows (ai := ai) (ti := ti) hb F' he hy hk q
    rw [c] at f'
    obtain ⟨w, hs2, hk2, hw⟩ := f'
    exact ⟨c'.trans c, q', w, hs.trans hs2, hk2, hw⟩

/-- **From the boot state of the run.** With the first boundary the state
the start entry left (`start_first`: its scheduler state is `Sched::new`'s
result), every boundary of the execution is the Kernel state of a state
the run of `Sched/KRun.lean` reaches, when the run follows each call. -/
theorem exec_kreach {I : KSt NT NR NP NL NS NE → Prop} {P : Params}
    (hb : IsBoundT ki) {b1 b : arch.bound.Bound NT NR NP NL NS NE}
    (h1 : logic.sched.Sched.new NL b1.cfg = ok b1.kernel.sched)
    (F : Follows I b1.cfg P) (hI : I (KSt.boot b1.kernel)) (h : Exec ki ai ti b1 b) :
    ∃ x : KSt NT NR NP NL NS NE, KReach b1.cfg P x ∧ x.k = b.kernel ∧ I x := by
  obtain ⟨_, _, x, hs, hk, hx⟩ :=
    exec_follows (ai := ai) (ti := ti) hb F hI rfl (new_quieted h1) h
  exact ⟨x, hs.reach (KReach.boot b1.kernel h1), hk, hx⟩

open rsk_analyzer (candidateOf) in
open rsk_analyzer.terms (TaskIn Site Window Verdict) in
/-- **Property 18 at every boundary of an execution the run follows.** As
`property18_kernel`, for the state of the run that `exec_kreach` gives for
a boundary `b`: its Kernel state is `b.kernel`, and a Task the Analyzer
reports schedulable with bound `R` that has a Job pending or in progress
in it was released less than `R` before its instant. -/
theorem property18_exec {I : KSt NT NR NP NL NS NE → Prop}
    (hb : IsBoundT ki) {b1 b : arch.bound.Bound NT NR NP NL NS NE}
    (tasks : alloc.vec.Vec TaskIn) (sites : alloc.vec.Vec Site) (p : rsk_analyzer.terms.Params)
    (any_fpu : Bool) (windows : alloc.vec.Vec Window)
    (ds : alloc.vec.Vec rsk_analyzer.terms.Derived) (vs : alloc.vec.Vec Verdict)
    (tss : alloc.vec.Vec (alloc.vec.Vec rsk_analyzer.terms.TermOf)) (quiet : alloc.vec.Vec Bool)
    (L : Nat → Nat → Nat)
    (hall : rsk_analyzer.terms.analyse_all tasks sites p any_fpu windows
      = ok (some (ds, vs, tss, quiet)))
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length)
    (hwl : windows.val.length = tasks.val.length)
    (hprio : ∀ k t, tasks.val[k]? = some t →
      0 < t.priority.val ∧ prio b1.cfg k = t.priority.val)
    (hbud : ∀ t ∈ tasks.val, 0 < t.budget.val)
    (hDT : ∀ w ∈ windows.val, w.deadline.val ≤ w.period.val)
    (hL : ∀ i ti k r, tasks.val[i]? = some ti → k < NT.val → prio b1.cfg k < prio b1.cfg i →
      prio b1.cfg i ≤ ceil b1.cfg r →
      L k r = 0 ∨
      ∃ (j : Nat) (s : Site) (l : TaskIn), sites.val[j]? = some s
        ∧ tasks.val[s.task.val]? = some l
        ∧ l.priority.val < ti.priority.val ∧ ti.priority.val ≤ s.ceiling.val
        ∧ L k r ≤ candidateOf ti l s p)
    (i : Nat) (v : Verdict) (hv : vs.val[i]? = some v) (hsched : v.schedulable = true)
    (h1 : logic.sched.Sched.new NL b1.cfg = ok b1.kernel.sched)
    (F : Follows I b1.cfg (runParams NT.val tasks.val sites.val p windows.val L))
    (hI : I (KSt.boot b1.kernel)) (h : Exec ki ai ti b1 b) :
    ∃ x : KSt NT NR NP NL NS NE, x.k = b.kernel ∧ I x ∧
      (0 < x.pend i + x.run i → ∃ a, x.jl i = some a ∧ x.now < a + v.r.val) := by
  obtain ⟨x, hx, hk, hi⟩ := exec_kreach (ai := ai) (ti := ti) hb h1 F hI h
  exact ⟨x, hk, hi, fun hpos =>
    property18_kernel tasks sites p any_fpu windows ds vs tss quiet L hall hlen hNT hsok hwl
      hprio hbud hDT hL i v hv hsched hx hpos⟩

end P18
