import Sched.Settle
open Aeneas Aeneas.Std Result

/-! # The order of an entry's calls (2026-11-27)

`Sched/Trace.lean` proved that an entry core changes the Kernel's state
only through the interface's methods, in some order. This file reads the
order from the cores as extracted, for any implementor of the interface
(`KernelOps`): which methods that return a Kernel a core calls, on which
state, one after another.

- `Acks k k'`: the acknowledgements of `apply_core`, Partition after
  Partition: `take_stop_dma` and then `take_reinit`, each on the state the
  one before returned.
- `Applied k k'`: what `apply_core` does to the Kernel: `settle`, then the
  acknowledgements (`apply_core_applied`).
- `Wr l k k'`: `k'` is what the item `l` gave on `k`: a method of the
  interface (`Lbl`: `start`, `dispatch`, `job_return`, `svc_lock`,
  `svc_unlock`, `intake`, `timer_event`, `fault`, `fault_of`,
  `record_event`), or `apply_core` (`Lbl.apply`, `Applied`).
- `Sh ls k k'`: the items `ls`, in that order, each on the state the one
  before returned.
- `En L k k'`: `Sh ls k k'` for one of the lists `ls` of `L`. Each core's
  theorem gives its `L`: the paths the code has.

No boot record and no `init` is in any list: `boot_failed`, `boot_info`
and `init` are called by `boot_core`, which is not an entry. `start` is in
`start_core`'s list only. Handwritten; no `sorry`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

/-- The items of a path: the methods of the interface that return a
Kernel and that an entry's body calls, and `apply_core`. -/
inductive Lbl
  | start | dispatch | ret | lock | unlock | intake | timer | fault | fault_of | record | apply
  deriving DecidableEq, Repr

section script
variable {K A T : Type _}
  (ki : _root_.rsk_kernel.arch.bound.KernelOps K)
  (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A)
  (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T)

/-- The acknowledgements of `apply_core`, Partition after Partition. -/
inductive Acks : K → K → Prop
  | done (k : K) : Acks k k
  | step (k k1 k2 k' : K) (i j : U8) (b b1 : Bool)
      (h1 : ki.take_stop_dma k i = ok (b, k1)) (h2 : ki.take_reinit k1 j = ok (b1, k2))
      (h : Acks k2 k') : Acks k k'

/-- What `apply_core` does to the Kernel. -/
def Applied (k k' : K) : Prop :=
  ∃ (r : Option U8 × Bool) (k1 : K), ki.settle k = ok (r, k1) ∧ Acks ki k1 k'

/-- One item of a path. -/
inductive Wr : Lbl → K → K → Prop
  | start (k k' : K) (h : ki.start k = ok k') : Wr .start k k'
  | dispatch (k k' : K) (t : U8) (pc : U32) (h : ki.dispatch k t pc = ok k') : Wr .dispatch k k'
  | ret (k k' : K) (ec : U32) (h : ki.job_return k ec = ok k') : Wr .ret k k'
  | lock (k k' : K) (r : U8) (v : Bool) (h : ki.svc_lock k r = ok (v, k')) : Wr .lock k k'
  | unlock (k k' : K) (r : U8) (v : Bool) (h : ki.svc_unlock k r = ok (v, k')) : Wr .unlock k k'
  | intake (k k' : K) (t sp : U8) (now : U64) (off : U8)
      (h : ki.intake k t sp now off = ok k') : Wr .intake k k'
  | timer (k k' : K) (now : U64) (rc : U32) (h : ki.timer_event k now rc = ok k') : Wr .timer k k'
  | fault (k k' : K) (kind : U8) (kn : Bool) (h : ki.fault k kind kn = ok k') : Wr .fault k k'
  | fault_of (k k' : K) (kind t : U8) (h : ki.fault_of k kind t = ok k') : Wr .fault_of k k'
  | record (k k' : K) (kind t : U8) (h : ki.record_event k kind t = ok k') : Wr .record k k'
  | apply (k k' : K) (h : Applied ki k k') : Wr .apply k k'

/-- The items `ls` in order. -/
inductive Sh : List Lbl → K → K → Prop
  | nil (k : K) : Sh [] k k
  | cons (l : Lbl) (ls : List Lbl) (k k1 k' : K) (h : Wr ki l k k1) (hs : Sh ls k1 k') :
      Sh (l :: ls) k k'

/-- One of the paths of `L`. -/
def En (L : List (List Lbl)) (k k' : K) : Prop :=
  ∃ ls, ls ∈ L ∧ Sh ki ls k k'

variable {ki}

theorem En.mono {L L' : List (List Lbl)} {k k' : K} (h : En ki L k k')
    (hs : ∀ x, x ∈ L → x ∈ L') : En ki L' k k' := by
  obtain ⟨ls, hm, h⟩ := h
  exact ⟨ls, hs _ hm, h⟩

theorem Sh.append {ls ls' : List Lbl} {k k1 k' : K} (h : Sh ki ls k k1) (h' : Sh ki ls' k1 k') :
    Sh ki (ls ++ ls') k k' := by
  induction h with
  | nil k => exact h'
  | cons l ls k k1 k2 h1 _ ih => exact .cons _ _ _ _ _ h1 (ih h')

variable (ki)

theorem apply_core_loop1_acks {tables : T} {np : Usize} :
    ∀ (n : Nat) (k : K) (hw : A) (state : _root_.rsk_kernel.arch.cm4.plan.ArchState) (p : Usize)
      (res : K × A × _root_.rsk_kernel.arch.cm4.plan.ArchState),
      np.val - p.val = n →
      _root_.rsk_kernel.arch.cm4.entry.apply_core_loop1 ki ai ti k hw tables state np p = ok res →
      Acks ki k res.1 := by
  intro n
  induction n with
  | zero =>
    intro k hw state p res hn h
    unfold _root_.rsk_kernel.arch.cm4.entry.apply_core_loop1 loop at h
    simp only [_root_.rsk_kernel.arch.cm4.entry.apply_core_loop1.body] at h
    have hlt : ¬ p < np := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    exact .done _
  | succ n ih =>
    intro k hw state p res hn h
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
      refine .step _ _ _ _ _ _ _ _ hsd1 (by assumption) ?_
      apply ih (res := res)
      pick_goal 2
      · unfold _root_.rsk_kernel.arch.cm4.entry.apply_core_loop1
        simp only [_root_.rsk_kernel.arch.cm4.entry.apply_core_loop1.body, lift, bind_ok]
        exact hloop
      · omega

/-- **`apply_core` writes the Kernel by `settle` and then the
acknowledgements, and by nothing else.** -/
theorem apply_core_applied {k k' : K} {hw hw' : A} {tables : T}
    {state state' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {safe : Bool}
    (h : _root_.rsk_kernel.arch.cm4.entry.apply_core ki ai ti k hw tables state
      = ok (safe, k', hw', state')) : Applied ki k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.apply_core at h
  peel h
  all_goals
    exact ⟨_, _, by assumption, apply_core_loop1_acks ki ai ti _ _ _ _ _ _ rfl (by assumption)⟩

/-! ## Closing a path

One lemma for each shape a path has, so that a path's goal is closed by
naming the calls it found. -/

variable {ki ai ti}

theorem wr_apply {k k' : K} {hw hw' : A} {tables : T}
    {state state' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {safe : Bool}
    (h : _root_.rsk_kernel.arch.cm4.entry.apply_core ki ai ti k hw tables state
      = ok (safe, k', hw', state')) : Wr ki .apply k k' :=
  .apply _ _ (apply_core_applied ki ai ti h)

theorem en_plain {L : List (List Lbl)} {k : K} (hm : [] ∈ L) : En ki L k k :=
  ⟨_, hm, .nil _⟩

theorem en_w {L : List (List Lbl)} {l : Lbl} {k k' : K} (hm : [l] ∈ L)
    (h : Wr ki l k k') : En ki L k k' :=
  ⟨_, hm, .cons _ _ _ _ _ h (.nil _)⟩

theorem en_ww {L : List (List Lbl)} {l l2 : Lbl} {k k1 k' : K} (hm : [l, l2] ∈ L)
    (h : Wr ki l k k1) (h2 : Wr ki l2 k1 k') : En ki L k k' :=
  ⟨_, hm, .cons _ _ _ _ _ h (.cons _ _ _ _ _ h2 (.nil _))⟩

theorem en_a {L : List (List Lbl)} {k k' : K} {hw hw' : A} {tables : T}
    {state state' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {safe : Bool}
    (hm : [.apply] ∈ L)
    (h : _root_.rsk_kernel.arch.cm4.entry.apply_core ki ai ti k hw tables state
      = ok (safe, k', hw', state')) : En ki L k k' :=
  en_w hm (wr_apply h)

theorem en_wa {L : List (List Lbl)} {l : Lbl} {k k1 k' : K} {hw hw' : A} {tables : T}
    {state state' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {safe : Bool}
    (hm : [l, .apply] ∈ L) (h1 : Wr ki l k k1)
    (h : _root_.rsk_kernel.arch.cm4.entry.apply_core ki ai ti k1 hw tables state
      = ok (safe, k', hw', state')) : En ki L k k' :=
  en_ww hm h1 (wr_apply h)

theorem en_wwa {L : List (List Lbl)} {l l2 : Lbl} {k k1 k2 k' : K} {hw hw' : A} {tables : T}
    {state state' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {safe : Bool}
    (hm : [l, l2, .apply] ∈ L) (h1 : Wr ki l k k1) (h2 : Wr ki l2 k1 k2)
    (h : _root_.rsk_kernel.arch.cm4.entry.apply_core ki ai ti k2 hw tables state
      = ok (safe, k', hw', state')) : En ki L k k' :=
  ⟨_, hm, .cons _ _ _ _ _ h1 (.cons _ _ _ _ _ h2 (.cons _ _ _ _ _ (wr_apply h) (.nil _)))⟩

/-- Two calls, each followed by `apply_core`. -/
theorem en_wawa {L : List (List Lbl)} {l l2 : Lbl} {k k1 k2 k3 k' : K} {hw hw' hw2 hw2' : A}
    {tables : T} {state state' state2 state2' : _root_.rsk_kernel.arch.cm4.plan.ArchState}
    {safe safe2 : Bool}
    (hm : [l, .apply, l2, .apply] ∈ L) (h1 : Wr ki l k k1)
    (ha1 : _root_.rsk_kernel.arch.cm4.entry.apply_core ki ai ti k1 hw tables state
      = ok (safe, k2, hw', state'))
    (h2 : Wr ki l2 k2 k3)
    (ha2 : _root_.rsk_kernel.arch.cm4.entry.apply_core ki ai ti k3 hw2 tables state2
      = ok (safe2, k', hw2', state2')) : En ki L k k' :=
  ⟨_, hm, .cons _ _ _ _ _ h1 (.cons _ _ _ _ _ (wr_apply ha1)
    (.cons _ _ _ _ _ h2 (.cons _ _ _ _ _ (wr_apply ha2) (.nil _))))⟩

/-- A path of a sub-core, then `apply_core`. -/
theorem En.then_apply {L L' : List (List Lbl)} {k k1 k' : K} {hw hw' : A} {tables : T}
    {state state' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {safe : Bool}
    (h1 : En ki L k k1) (hf : ∀ x, x ∈ L → x ++ [.apply] ∈ L')
    (h : _root_.rsk_kernel.arch.cm4.entry.apply_core ki ai ti k1 hw tables state
      = ok (safe, k', hw', state')) : En ki L' k k' := by
  obtain ⟨ls, hm, hs⟩ := h1
  exact ⟨_, hf _ hm, hs.append (.cons _ _ _ _ _ (wr_apply h) (.nil _))⟩

theorem svc_send_ne_recv {number : U8} (a : number = _root_.rsk_kernel.arch.cm4.entry.SVC_SEND)
    (b : number = _root_.rsk_kernel.arch.cm4.entry.SVC_RECV) : False := by
  subst a
  simp [_root_.rsk_kernel.arch.cm4.entry.SVC_SEND, _root_.rsk_kernel.arch.cm4.entry.SVC_RECV] at b

variable (ki ai ti)

/-! ## The cores

Each theorem's list is exact for the tactic that proves it: an alternative
that closes no path is reported by the linter, and none is. -/

set_option maxHeartbeats 8000000 in
/-- The start: `start`, then `apply_core`. -/
theorem start_core_sh {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState}  {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.start_core ki ai ti k hw tables state = ok (r, k', hw', st')) :
    En ki [[.start, .apply]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.start_core at h
  peel h
  all_goals first
    | exact en_wa (by decide) (.start _ _ (by assumption)) (by assumption)

set_option maxHeartbeats 8000000 in
/-- The time base's entry: `timer_event`, then `apply_core`. -/
theorem timer_core_sh {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState}  {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.timer_core ki ai ti k hw tables state = ok (r, k', hw', st')) :
    En ki [[.timer, .apply]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.timer_core at h
  peel h
  all_goals first
    | exact en_wa (by decide) (.timer _ _ _ _ (by assumption)) (by assumption)

set_option maxHeartbeats 8000000 in
/-- An interrupt with no binding: a fault, then `apply_core`. -/
theorem unbound_core_sh {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U16} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.unbound_core ki ai ti k hw tables state n = ok (r, k', hw', st')) :
    En ki [[.fault, .apply]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.unbound_core at h
  peel h
  all_goals first
    | exact en_wa (by decide) (.fault _ _ _ _ (by assumption)) (by assumption)

set_option maxHeartbeats 8000000 in
/-- A Release_Source's entry: `intake`, after a fault when the interrupted Job's stack pointer is outside its stack; then `apply_core`. -/
theorem intake_core_sh {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U16} {task partition : U8} {psp er : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.intake_core ki ai ti k hw tables state n task partition psp er = ok (r, k', hw', st')) :
    En ki [[.intake, .apply], [.fault, .intake, .apply]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.intake_core at h
  peel h
  all_goals first
    | exact en_wa (by decide) (.intake _ _ _ _ _ _ (by assumption)) (by assumption)
    | exact en_wwa (by decide) (.fault _ _ _ _ (by assumption)) (.intake _ _ _ _ _ _ (by assumption)) (by assumption)

set_option maxHeartbeats 8000000 in
/-- A stack violation: a fault, then `apply_core`. -/
theorem stack_violation_core_sh {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState}  {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.stack_violation_core ki ai ti k hw tables state = ok (r, k', hw', st')) :
    En ki [[.fault, .apply]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.stack_violation_core at h
  peel h
  all_goals first
    | exact en_wa (by decide) (.fault _ _ _ _ (by assumption)) (by assumption)

set_option maxHeartbeats 8000000 in
/-- A fault entry: `fault`, then `apply_core`. -/
theorem fault_core_sh {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {kind : U8} {ink : Bool} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.fault_core ki ai ti k hw tables state kind ink = ok (r, k', hw', st')) :
    En ki [[.fault, .apply]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.fault_core at h
  peel h
  all_goals first
    | exact en_wa (by decide) (.fault _ _ _ _ (by assumption)) (by assumption)

set_option maxHeartbeats 8000000 in
/-- The start of a Job: a fault and `apply_core`; or `dispatch` alone (which settles, `wr_kw` of `Sched/EntryRun.lean`); or `dispatch`, a fault on its result and `apply_core`. -/
theorem dispatch_job_core_sh {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control er : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.dispatch_job_core ki ai ti k hw tables state n t psp control er = ok (r, k', hw', st')) :
    En ki [[.fault, .apply], [.dispatch], [.dispatch, .fault, .apply]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.dispatch_job_core at h
  peel h
  all_goals first
    | exact en_wa (by decide) (.fault _ _ _ _ (by assumption)) (by assumption)
    | exact en_w (by decide) (.dispatch _ _ _ _ (by assumption))
    | exact en_wwa (by decide) (.dispatch _ _ _ _ (by assumption)) (.fault _ _ _ _ (by assumption)) (by assumption)

set_option maxHeartbeats 8000000 in
/-- A dispatch vector: nothing, a fault and `apply_core`, or the start of a Job. -/
theorem dispatch_vector_core_sh {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {n : U16} {t : U8} {psp control er : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.dispatch_vector_core ki ai ti k hw tables state n t psp control er = ok (r, k', hw', st')) :
    En ki [[], [.fault, .apply], [.dispatch], [.dispatch, .fault, .apply]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.dispatch_vector_core at h
  by_cases hwd : _root_.rsk_kernel.arch.cm4.entry.WINDOW_DEFER = true <;>
    simp only [hwd, ↓reduceIte, Bool.false_eq_true] at h <;> peel h
  all_goals first
    | exact en_plain (by decide)
    | exact en_wa (by decide) (.fault _ _ _ _ (by assumption)) (by assumption)
    | exact (dispatch_job_core_sh ki ai ti (by assumption)).mono (by decide)
    | exact (stack_violation_core_sh ki ai ti (by assumption)).mono (by decide)

set_option maxHeartbeats 8000000 in
/-- **An interrupt entry.** -/
theorem irq_entry_core_sh {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {ipsr psp control er : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.irq_entry_core ki ai ti k hw tables state ipsr psp control er = ok (r, k', hw', st')) :
    En ki [[], [.fault, .apply], [.dispatch], [.dispatch, .fault, .apply], [.intake, .apply], [.fault, .intake, .apply], [.timer, .apply]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.irq_entry_core at h
  peel h
  all_goals first
    | exact en_plain (by decide)
    | exact (dispatch_vector_core_sh ki ai ti (by assumption)).mono (by decide)
    | exact (timer_core_sh ki ai ti (by assumption)).mono (by decide)
    | exact (intake_core_sh ki ai ti (by assumption)).mono (by decide)
    | exact (unbound_core_sh ki ai ti (by assumption)).mono (by decide)

set_option maxHeartbeats 8000000 in
/-- An EasyDMA request: accepted and the Kernel as it was, or refused and one fault. -/
theorem dma_request_core_fl {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {p l : U32} {b : Bool}
    (h : _root_.rsk_kernel.arch.cm4.entry.dma_request_core ki ai ti k hw tables i p l = ok (b, k', hw')) :
    (b = true ∧ k' = k) ∨ (¬ b = true ∧ Wr ki .fault k k') := by
  unfold _root_.rsk_kernel.arch.cm4.entry.dma_request_core at h
  peel h
  all_goals first
    | exact Or.inl ⟨by assumption, rfl⟩
    | exact Or.inr ⟨by assumption, .fault _ _ _ _ (by assumption)⟩

set_option maxHeartbeats 8000000 in
/-- An EasyDMA register write: the same. -/
theorem dma_write_core_fl {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {p l : U32} {b : Bool}
    (h : _root_.rsk_kernel.arch.cm4.entry.dma_write_core ki ai ti k hw tables i p l = ok (b, k', hw')) :
    (b = true ∧ k' = k) ∨ (¬ b = true ∧ Wr ki .fault k k') := by
  unfold _root_.rsk_kernel.arch.cm4.entry.dma_write_core at h
  peel h
  all_goals first
    | exact Or.inl ⟨by assumption, rfl⟩
    | exact Or.inr ⟨by first | assumption | simp_all, .fault _ _ _ _ (by assumption)⟩

set_option linter.unusedSimpArgs false in
set_option maxHeartbeats 8000000 in
/-- An Endpoint receive: an answer other than "refused" and the Kernel as it was, or "refused" and one fault. -/
theorem endpoint_recv_core_fl {k k' : K} {hw hw' : A} {tables : T} {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {a p l : U32} {c : U32} {g : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables state a p l = ok ((c, g), k', hw', st')) :
    ((c != _root_.rsk_kernel.arch.cm4.entry.ENDPOINT_REFUSED) = true ∧ k' = k) ∨ (¬ (c != _root_.rsk_kernel.arch.cm4.entry.ENDPOINT_REFUSED) = true ∧ Wr ki .fault k k') := by
  unfold _root_.rsk_kernel.arch.cm4.entry.endpoint_recv_core at h
  peel h
  all_goals first
    | exact Or.inl ⟨by simp [_root_.rsk_kernel.arch.cm4.entry.ENDPOINT_OK, _root_.rsk_kernel.arch.cm4.entry.ENDPOINT_EMPTY, _root_.rsk_kernel.arch.cm4.entry.ENDPOINT_PEER, _root_.rsk_kernel.arch.cm4.entry.ENDPOINT_REFUSED], rfl⟩
    | exact Or.inr ⟨by simp [_root_.rsk_kernel.arch.cm4.entry.ENDPOINT_OK, _root_.rsk_kernel.arch.cm4.entry.ENDPOINT_EMPTY, _root_.rsk_kernel.arch.cm4.entry.ENDPOINT_PEER, _root_.rsk_kernel.arch.cm4.entry.ENDPOINT_REFUSED], .fault _ _ _ _ (by assumption)⟩

set_option maxHeartbeats 8000000 in
/-- A Signal raised: one fault, or one `intake`. -/
theorem raise_core_sh {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {b : Bool}
    (h : _root_.rsk_kernel.arch.cm4.entry.raise_core ki ai ti k hw tables i = ok (b, k', hw')) :
    En ki [[.fault], [.intake]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.raise_core at h
  peel h
  all_goals first
    | exact en_w (by decide) (.fault _ _ _ _ (by assumption))
    | exact en_w (by decide) (.intake _ _ _ _ _ _ (by assumption))

set_option maxHeartbeats 8000000 in
/-- A rejected message: one `fault`, or one `fault_of`. -/
theorem rejection_core_sh {k k' : K} {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {cfg : _root_.rsk_kernel.arch.cm4.plan.EndpointCfg} {e role fr : Usize}
    (h : _root_.rsk_kernel.arch.cm4.entry.rejection_core ki k state cfg e role fr = ok (k', st')) :
    En ki [[.fault], [.fault_of]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.rejection_core at h
  peel h
  all_goals first
    | exact en_w (by decide) (.fault _ _ _ _ (by assumption))
    | exact en_w (by decide) (.fault_of _ _ _ _ (by assumption))

set_option maxHeartbeats 8000000 in
/-- A report: nothing, one `fault`, one `fault_of`, or one `record_event`. -/
theorem violation_core_sh {k k' : K} {tables : T} {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {arg : U32}
    (h : _root_.rsk_kernel.arch.cm4.entry.violation_core ki ti k tables state arg = ok (k', st')) :
    En ki [[], [.fault], [.fault_of], [.record]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.violation_core at h
  peel h
  iterate 4
    all_goals try (rw [ite_eq_iff] at h; rcases h with ⟨-, h⟩ | ⟨-, h⟩ <;> peel h)
  all_goals first
    | exact en_plain (by decide)
    | exact en_w (by decide) (.fault _ _ _ _ (by assumption))
    | exact en_w (by decide) (.record _ _ _ _ (by assumption))
    | exact (rejection_core_sh ki (by assumption)).mono (by decide)

set_option maxHeartbeats 8000000 in
/-- An Endpoint send: nothing, one `fault`, one `record_event`, or one `intake`. -/
theorem endpoint_send_core_sh {k k' : K} {hw hw' : A} {tables : T} {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {a p l : U32} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.endpoint_send_core ki ai ti k hw tables state a p l = ok (r, k', hw', st')) :
    En ki [[], [.fault], [.record], [.intake]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.endpoint_send_core at h
  peel h
  all_goals first
    | exact en_plain (by decide)
    | exact en_w (by decide) (.fault _ _ _ _ (by assumption))
    | exact en_w (by decide) (.record _ _ _ _ (by assumption))
    | exact en_w (by decide) (.intake _ _ _ _ _ _ (by assumption))

set_option maxHeartbeats 8000000 in
/-- The services of an SVC other than lock, unlock and the clock. A service that leaves the Kernel as it was and answers "accepted" returns without `apply_core`; every write is followed by `apply_core`. -/
theorem svc_service_core_sh {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {frame frame' : Array U32 8#usize} {number : U8} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.svc_service_core ki ai ti k hw tables state frame number = ok (r, k', hw', st', frame')) :
    En ki [[], [.apply], [.fault, .apply], [.intake, .apply], [.record, .apply], [.fault_of, .apply], [.ret, .apply]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.svc_service_core at h
  peel h
  all_goals first
    | contradiction
    | exact (svc_send_ne_recv (by assumption) (by assumption)).elim
    | (rcases dma_write_core_fl ki ai ti (by assumption) with ⟨hb, rfl⟩ | ⟨hb, hwr⟩ <;> first
        | contradiction
        | exact en_plain (by decide)
        | exact en_wa (by decide) hwr (by assumption))
    | (rcases dma_request_core_fl ki ai ti (by assumption) with ⟨hb, rfl⟩ | ⟨hb, hwr⟩ <;> first
        | contradiction
        | exact en_plain (by decide)
        | exact en_wa (by decide) hwr (by assumption))
    | (rcases endpoint_recv_core_fl ki ai ti (by assumption) with ⟨hb, rfl⟩ | ⟨hb, hwr⟩ <;> first
        | contradiction
        | exact en_plain (by decide)
        | exact en_wa (by decide) hwr (by assumption))
    | exact (endpoint_send_core_sh ki ai ti (by assumption)).then_apply (by decide) (by assumption)
    | exact (raise_core_sh ki ai ti (by assumption)).then_apply (by decide) (by assumption)
    | exact (violation_core_sh ki ti (by assumption)).then_apply (by decide) (by assumption)
    | exact en_wa (by decide) (.ret _ _ _ (by assumption)) (by assumption)
    | exact en_wa (by decide) (.fault _ _ _ _ (by assumption)) (by assumption)

set_option maxHeartbeats 8000000 in
/-- **An SVC.** The lock alone writes the Kernel without `apply_core` after it. -/
theorem svc_core_sh {k k' : K} {hw hw' : A} {tables : T}
    {state st' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {frame frame' : Array U32 8#usize} {number : U8} {r : _}
    (h : _root_.rsk_kernel.arch.cm4.entry.svc_core ki ai ti k hw tables state frame number = ok (r, k', hw', st', frame')) :
    En ki [[], [.apply], [.fault, .apply], [.intake, .apply], [.record, .apply], [.fault_of, .apply], [.ret, .apply], [.lock], [.unlock, .apply]] k k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.svc_core at h
  peel h
  all_goals first
    | exact en_plain (by decide)
    | exact en_w (by decide) (.lock _ _ _ _ (by assumption))
    | exact en_wa (by decide) (.unlock _ _ _ _ (by assumption)) (by assumption)
    | exact (svc_service_core_sh ki ai ti (by assumption)).mono (by decide)

end script

end P18
