import Sched.EntryRun
import Sched.DispatchTask
import Sched.SelectLow
open Aeneas Aeneas.Std Result

/-! # The dispatch choice: the interrupt controller's, joined to the run (2026-11-27)

The run of `Sched/KRun.lean` dispatches the Task `Sched::select` returns.
The Kernel's code does not call `select` to choose: a dispatch vector's
entry dispatches the Task of its vector when `eligible` admits it, and
which vector is taken is the interrupt controller's choice.
`Sched/EntryRun.lean` therefore assumed, in `Follows`, that the run follows
the dispatch of whichever Task the entry dispatches. This file replaces
that part of the assumption by the Hardware_Model's statement of the
controller's arbitration (`Sched/Nvic.lean`).

- `Nv.taken_is_select`: **the Task whose vector the NVIC takes, when
  `eligible` admits it, is the Task `Sched::select` returns.** In a state
  with the run's structure, with the vectors in Task order and their group
  priorities those of the Tasks' Priorities (`Nv.VectorsOk`), the vector
  of the Task `select` returns pending, and the arbitration against the
  execution priority of thread mode under the BASEPRI the Kernel keeps
  (`Nv.dispatchExec`). No premise that every pending vector is of a Task
  with a pending Release (`Nv.NoStale`): the code's `eligible` check takes
  its place.
- `bound_dispatch_elig`: the linked `dispatch` calls `Kernel::dispatch`
  only for a Task R7.1 admits.
- `FollowsSel`: `Follows` with the dispatch field asked only of the Task
  `Sched::select` returns, which is the premise the run's own `dispatch`
  transition has. `Follows.sel`: it is the weaker assumption.
- `Taken`: what is assumed of the hardware at an interrupt entry that
  comes from thread mode with an exception number the tables bind to a
  dispatch: the NVIC selected that number among the pending vectors,
  against `Nv.dispatchExec` of the scheduler state the entry finds, and
  the vector of the Task `select` returns on that state was pending.
- `ExecN`: an execution (`Exec`) whose interrupt entries carry `Taken`.
  `ExecN.exec`: it is an execution.
- `execn_kreach`, `property18_nvic`: **with `FollowsSel`, the run follows
  every such execution from the boot state, and Property 18 holds at every
  boundary.** The Task of each dispatch is read from the entry core
  (`irq_entry_core_task`, `Sched/DispatchTask.lean`): the one the tables
  bind to the exception number in the status word, dispatched on the
  Kernel state the entry found, and only when the return word says thread
  mode.

Assumed, and not derived here: `Taken` (that the exception number the
entry reads is the one the NVIC selected, that no exception was active and
PRIMASK clear, that BASEPRI held the value for the scheduler state, and
that the Kernel had pended the vector of `select`'s Task: the pend is
`apply_core`'s plan from `settle`'s answer, and the pending bit is the
hardware's); `Nv.VectorsOk` and the link `hbind` between the tables'
binding and the vector numbering (the Generator's tables); `Nv.LevelsOk`
(`Config::validate`); and the rest of `Follows` (`FollowsRest` and the
dispatch field for `select`'s Task), as in `Sched/EntryRun.lean`.
Handwritten; no `sorry`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

namespace Nv

/-- **The Task whose vector the NVIC takes, when R7.1 admits it, is the
Task `Sched::select` returns.** -/
theorem taken_is_select {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) (hNT : NT.val ≤ 256) (hl : LevelsOk cfg)
    {vector group : Nat → Nat} {pending : Nat → Prop} (hv : VectorsOk cfg vector group)
    (hp : ∀ t : U8, logic.sched.Sched.select st.s cfg = ok (some t) → pending (vector t.val))
    {u : U8} (he : Elig cfg st.s u.val)
    (hn : nvicSelects pending group (dispatchExec cfg st.s) (vector u.val)) :
    logic.sched.Sched.select st.s cfg = ok (some u) := by
  obtain ⟨o, ho⟩ := select_returns st.s cfg w.depth_le w.stk_lt hNT
  cases o with
  | none => exact absurd he (select_none hNT ho (wf_below_top w) u.val)
  | some t =>
    obtain ⟨het, hmax, hlow⟩ := select_srp hNT ho
    obtain ⟨hun, hur, _⟩ := elig_views.mp he
    have htn : t.val < NT.val := het.1
    have htr : rel st.s t.val = 1 := (elig_views.mp het).2.1
    obtain ⟨hpt1, hpt7⟩ := hl.1 t.val htn
    obtain ⟨hpu1, hpu7⟩ := hl.1 u.val hun
    have htakes : nvicTakes (group (vector t.val)) (dispatchExec cfg st.s) := by
      rw [hv.2 t.val htn]; exact (dispatch_takes_iff_elig w hl htn htr).mpr het
    have hle := hmax u.val hun hur
    obtain ⟨_, _, hall⟩ := hn
    have e : t.val = u.val := by
      rcases hall (vector t.val) (hp t ho) htakes with h | ⟨h, hnn⟩
      · rw [hv.2 u.val hun, hv.2 t.val htn, encode_lt hpu1 hpu7 hpt1 hpt7] at h
        omega
      · rw [hv.2 u.val hun, hv.2 t.val htn, encode_eq hpu1 hpu7 hpt1 hpt7] at h
        have h2 := hlow u.val hun hur h
        rcases Nat.lt_or_ge t.val u.val with hlt | hge
        · have := hv.1 t.val u.val hlt hun
          omega
        · omega
    have : t = u := by scalar_tac
    rw [ho, this]

end Nv

variable {ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)}

/-- **The linked `dispatch` calls `Kernel::dispatch` only for a Task R7.1
admits**, and then settles. -/
theorem bound_dispatch_elig (hb : IsBoundT ki) {b b1 : arch.bound.Bound NT NR NP NL NS NE}
    {t : U8} {pc : U32} (h : ki.dispatch b t pc = ok b1) :
    b1.cfg = b.cfg ∧ ∃ k1 r,
      (k1 = b.kernel ∨ (b.kernel.started = true ∧ Elig b.cfg b.kernel.sched t.val ∧
        logic.kernel.Kernel.dispatch b.kernel b.cfg t pc = ok k1)) ∧
      logic.kernel.Kernel.settle k1 b.cfg = ok (r, b1.kernel) := by
  rw [hb.dispatch] at h
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch at h
  simp only [bind_eq_ok] at h
  obtain ⟨k, hk, ⟨r, k1⟩, hs, h⟩ := h
  have h := ok_inj h; subst h
  refine ⟨rfl, k, r, ?_, hs⟩
  split at hk
  · simp only [bind_eq_ok] at hk
    obtain ⟨e, he, hk⟩ := hk
    split at hk
    · exact Or.inr ⟨by assumption, (eligible_ok (matches_self b) he).mp (by assumption), hk⟩
    · exact Or.inl (ok_inj hk).symm
  · exact Or.inl (ok_inj hk).symm

/-- `Follows` with the dispatch field asked only of the Task
`Sched::select` returns on the state the dispatch is called on. -/
structure FollowsSel (I : KSt NT NR NP NL NS NE → Prop) (cfg : logic.Config NT NR NP)
    (P : Params) : Prop extends FollowsRest I cfg P where
  dispatch : ∀ (x : KSt NT NR NP NL NS NE) t pc k', I x → x.k.started = true →
    logic.sched.Sched.select x.k.sched cfg = ok (some t) →
    logic.kernel.Kernel.dispatch x.k cfg t pc = ok k' → Fol I cfg P x k'

/-- `FollowsSel` is the weaker assumption. -/
theorem Follows.sel {I : KSt NT NR NP NL NS NE → Prop} {cfg : logic.Config NT NR NP} {P : Params}
    (F : Follows I cfg P) : FollowsSel I cfg P :=
  { F.toFollowsRest with dispatch := fun x t pc k' hI hst _ hd => F.dispatch x t pc k' hI hst hd }

/-- What is assumed of the hardware at the entry of the exception number
`n` on the scheduler state `s`: the NVIC selected `n` among the pending
vectors against the execution priority of thread mode under the BASEPRI
for `s`, and the vector of the Task `Sched::select` returns on `s` was
pending. -/
def Taken (cfg : logic.Config NT NR NP) (s : logic.sched.Sched NT NR NL)
    (vector group : Nat → Nat) (n : Nat) : Prop :=
  ∃ pending : Nat → Prop,
    (∀ t : U8, logic.sched.Sched.select s cfg = ok (some t) → pending (vector t.val)) ∧
    Nv.nvicSelects pending group (Nv.dispatchExec cfg s) n

variable {A T : Type} {ai : arch.cm4.entry.KernelArch A} {ti : arch.cm4.entry.Tables T}

/-- An entry core other than the interrupt entry, called on `b`, returned
`b'`. -/
def CoreRetO (ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE))
    (ai : arch.cm4.entry.KernelArch A) (ti : arch.cm4.entry.Tables T)
    (b b' : arch.bound.Bound NT NR NP NL NS NE) : Prop :=
  (∃ (hw hw' : A) (tables : T) (state st' : arch.cm4.plan.ArchState)
      (frame frame' : Array U32 8#usize) (number : U8), ∃ r,
    arch.cm4.entry.svc_core ki ai ti b hw tables state frame number
      = ok (r, b', hw', st', frame')) ∨
  (∃ (hw hw' : A) (tables : T) (state st' : arch.cm4.plan.ArchState), ∃ r,
    arch.cm4.entry.start_core ki ai ti b hw tables state = ok (r, b', hw', st')) ∨
  (∃ (hw hw' : A) (tables : T) (state st' : arch.cm4.plan.ArchState)
      (kind : U8) (ink : Bool), ∃ r,
    arch.cm4.entry.fault_core ki ai ti b hw tables state kind ink
      = ok (r, b', hw', st')) ∨
  (∃ (hw hw' : A) (tables : T) (state st' : arch.cm4.plan.ArchState)
      (n : U32), ∃ r,
    arch.cm4.entry.epilogue_core ki ai ti b hw tables state n
      = ok (r, b', hw', st'))

theorem CoreRetO.coreret {b b' : arch.bound.Bound NT NR NP NL NS NE}
    (h : CoreRetO ki ai ti b b') : CoreRet ki ai ti b b' := by
  rcases h with h | h | h | h
  · exact Or.inl h
  · exact Or.inr (Or.inr (Or.inl h))
  · exact Or.inr (Or.inr (Or.inr (Or.inl h)))
  · exact Or.inr (Or.inr (Or.inr (Or.inr h)))

/-- The boundaries of an execution whose interrupt entries carry the
Hardware_Model's statement: each interrupt entry that comes from thread
mode (bit 3 of the exception return word) with an exception number the
tables bind to the dispatch of a Task was taken as `Taken` says, on the
scheduler state the entry finds. -/
inductive ExecN (ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE))
    (ai : arch.cm4.entry.KernelArch A) (ti : arch.cm4.entry.Tables T) (tables : T)
    (vector group : Nat → Nat)
    (b1 : arch.bound.Bound NT NR NP NL NS NE) : arch.bound.Bound NT NR NP NL NS NE → Prop
  | first : ExecN ki ai ti tables vector group b1 b1
  | irq (b b' : arch.bound.Bound NT NR NP NL NS NE) (h : ExecN ki ai ti tables vector group b1 b)
      (hw hw' : A) (state st' : arch.cm4.plan.ArchState) (ipsr psp control er : U32)
      (r : arch.cm4.entry.DispatchPlan × Bool)
      (he : arch.cm4.entry.irq_entry_core ki ai ti b hw tables state ipsr psp control er
        = ok (r, b', hw', st'))
      (hn : ∀ t : U8, (er &&& 8#u32) ≠ 0#u32 →
        ti.binding tables (vecOf ipsr) = ok (arch.cm4.entry.Binding.Dispatch t) →
        Taken b.cfg b.kernel.sched vector group (vecOf ipsr).val) :
      ExecN ki ai ti tables vector group b1 b'
  | other (b b' : arch.bound.Bound NT NR NP NL NS NE)
      (h : ExecN ki ai ti tables vector group b1 b) (he : CoreRetO ki ai ti b b') :
      ExecN ki ai ti tables vector group b1 b'

/-- Such an execution is an execution. -/
theorem ExecN.exec {tables : T} {vector group : Nat → Nat}
    {b1 b : arch.bound.Bound NT NR NP NL NS NE}
    (h : ExecN ki ai ti tables vector group b1 b) : Exec ki ai ti b1 b := by
  induction h with
  | first => exact .first
  | irq b b' _ hw hw' state st' ipsr psp control er r he _ ih =>
    exact .entry _ _ ih (Or.inr (Or.inl ⟨hw, hw', tables, state, st', ipsr, psp, control, er, r, he⟩))
  | other b b' _ he ih => exact .entry _ _ ih he.coreret

/-- The paths of an entry that have no dispatch. -/
def ENTRY0 : List (List Lbl) :=
  [[], [.apply], [.fault, .apply], [.intake, .apply], [.record, .apply], [.fault_of, .apply],
   [.ret, .apply], [.ret, .apply, .fault, .apply], [.lock], [.unlock, .apply],
   [.fault, .intake, .apply], [.timer, .apply], [.start, .apply]]

theorem entry0_nodispatch : ∀ ls, ls ∈ ENTRY0 → Lbl.dispatch ∉ ls := by decide

/-- The run follows a path with no dispatch of the linked interface. -/
theorem en_follows_rest {I : KSt NT NR NP NL NS NE → Prop} {P : Params} (hb : IsBoundT ki)
    {b b' : arch.bound.Bound NT NR NP NL NS NE} (F : FollowsRest I b.cfg P)
    (h : En ki ENTRY0 b b') {x : KSt NT NR NP NL NS NE} (first : Bool) (hI : I x)
    (hx : x.k = b.kernel) (hq : first = true → Quieted b.cfg b.kernel) :
    b'.cfg = b.cfg ∧ Fol I b.cfg P x b'.kernel := by
  obtain ⟨ls, hm, hs⟩ := h
  obtain ⟨c, d⟩ := sh_ksh hb hs
  exact ⟨c, ksh_fol_rest F d x first (entry0_nodispatch ls hm) hI hx hq⟩

/-- **With the NVIC's choice for each dispatch, the run follows every
execution from the boot state.** For every boundary `b`: the
configuration is `b1`'s, the Kernel is quieted, and a state the run of
`Sched/KRun.lean` reaches has `b.kernel` as its Kernel state and satisfies
`I`. The run is asked to follow a dispatch only of the Task
`Sched::select` returns. -/
theorem execn_kreach {I : KSt NT NR NP NL NS NE → Prop} {P : Params}
    (hb : IsBoundT ki) (hNT : NT.val ≤ 256) {tables : T} {vector group : Nat → Nat}
    {b1 b : arch.bound.Bound NT NR NP NL NS NE}
    (hl : Nv.LevelsOk b1.cfg) (hv : Nv.VectorsOk b1.cfg vector group)
    (hbind : ∀ (n : U16) (t : U8),
      ti.binding tables n = ok (arch.cm4.entry.Binding.Dispatch t) → vector t.val = n.val)
    (h1 : logic.sched.Sched.new NL b1.cfg = ok b1.kernel.sched)
    (F : FollowsSel I b1.cfg P) (hI : I (KSt.boot b1.kernel))
    (h : ExecN ki ai ti tables vector group b1 b) :
    b.cfg = b1.cfg ∧ Quieted b.cfg b.kernel ∧
    ∃ x : KSt NT NR NP NL NS NE, KReach b1.cfg P x ∧ x.k = b.kernel ∧ I x := by
  induction h with
  | first => exact ⟨rfl, new_quieted h1, _, KReach.boot b1.kernel h1, rfl, hI⟩
  | other b b' _ he ih =>
    obtain ⟨c, q, x, hx, hk, hi⟩ := ih
    have q' := coreret_quieted ki ai ti (isboundt_qbound hb) he.coreret q
    have hen : En ki ENTRY0 b b' := by
      rcases he with ⟨_, _, _, _, _, _, _, _, _, h⟩ | ⟨_, _, _, _, _, _, h⟩ |
        ⟨_, _, _, _, _, _, _, _, h⟩ | ⟨_, _, _, _, _, _, _, h⟩
      · exact (svc_core_sh ki ai ti h).mono (by decide)
      · exact (start_core_sh ki ai ti h).mono (by decide)
      · exact (fault_core_sh ki ai ti h).mono (by decide)
      · exact (epilogue_core_sh ki ai ti h).mono (by decide)
    have F' : FollowsRest I b.cfg P := c ▸ F.toFollowsRest
    obtain ⟨c', y, hs, hyk, hy⟩ := en_follows_rest hb F' hen true hi hk (fun _ => q)
    rw [c] at hs
    exact ⟨c'.trans c, q', y, hs.reach hx, hyk, hy⟩
  | irq b b' _ hw hw' state st' ipsr psp control er r he hn ih =>
    obtain ⟨c, q, x, hx, hk, hi⟩ := ih
    have q' := coreret_quieted ki ai ti (isboundt_qbound hb)
      (Or.inr (Or.inl ⟨hw, hw', tables, state, st', ipsr, psp, control, er, r, he⟩)) q
    have F' : FollowsRest I b.cfg P := c ▸ F.toFollowsRest
    rcases irq_entry_core_task ki ai ti he with hen | ⟨t, hbnd, hthr, pc, bm, hd, hen⟩
    · obtain ⟨c', y, hs, hyk, hy⟩ :=
        en_follows_rest hb F' (hen.mono (by decide)) true hi hk (fun _ => q)
      rw [c] at hs
      exact ⟨c'.trans c, q', y, hs.reach hx, hyk, hy⟩
    · obtain ⟨cm, k1, rr, hdk, hset⟩ := bound_dispatch_elig hb hd
      rw [c] at hdk hset
      -- the dispatch
      have f1 : Fol I b1.cfg P x k1 := by
        rcases hdk with rfl | ⟨hst, hel, hdk⟩
        · exact hk ▸ Fol.here hi
        · have w : Wf b1.cfg P x.st := reach_wf (ksim hNT hx)
          obtain ⟨pending, hp, hsel⟩ := hn t hthr hbnd
          rw [c, ← hk] at hp hsel
          rw [← hbind _ _ hbnd] at hsel
          rw [← hk] at hel hst hdk
          have hs := Nv.taken_is_select w hNT hl hv hp hel hsel
          exact F.dispatch x t pc k1 hi hst hs hdk
      -- its `settle`
      have f2 : Fol I b1.cfg P x bm.kernel := f1.then fun y hy hyk => by
        subst hyk
        exact F.settle y rr bm.kernel hy hset
      -- a fault on its result, when there is one
      obtain ⟨y, hs, hyk, hy⟩ := f2
      have Fm : FollowsRest I bm.cfg P := (cm.trans c) ▸ F.toFollowsRest
      obtain ⟨c', z, hs2, hzk, hz⟩ :=
        en_follows_rest hb Fm (hen.mono (by decide)) false hy hyk (fun e => by cases e)
      rw [cm.trans c] at hs2
      exact ⟨c'.trans (cm.trans c), q', z, (hs.trans hs2).reach hx, hzk, hz⟩

open rsk_analyzer (candidateOf) in
open rsk_analyzer.terms (TaskIn Site Window Verdict) in
/-- **Property 18 at every boundary of an execution, with the dispatch
choice the NVIC's.** As `property18_exec`, with `FollowsSel` in place of
`Follows` and the execution's interrupt entries carrying `Taken`. -/
theorem property18_nvic {I : KSt NT NR NP NL NS NE → Prop}
    (hb : IsBoundT ki) {tables : T} {vector group : Nat → Nat}
    {b1 b : arch.bound.Bound NT NR NP NL NS NE}
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
    (hl : Nv.LevelsOk b1.cfg) (hvec : Nv.VectorsOk b1.cfg vector group)
    (hbind : ∀ (n : U16) (t : U8),
      ti.binding tables n = ok (arch.cm4.entry.Binding.Dispatch t) → vector t.val = n.val)
    (h1 : logic.sched.Sched.new NL b1.cfg = ok b1.kernel.sched)
    (F : FollowsSel I b1.cfg (runParams NT.val tasks.val sites.val p windows.val L))
    (hI : I (KSt.boot b1.kernel)) (h : ExecN ki ai ti tables vector group b1 b) :
    ∃ x : KSt NT NR NP NL NS NE, x.k = b.kernel ∧ I x ∧
      (0 < x.pend i + x.run i → ∃ a, x.jl i = some a ∧ x.now < a + v.r.val) := by
  obtain ⟨_, _, x, hx, hk, hi⟩ := execn_kreach hb hNT hl hvec hbind h1 F hI h
  exact ⟨x, hk, hi, fun hpos =>
    property18_kernel tasks sites p any_fpu windows ds vs tss quiet L hall hlen hNT hsok hwl
      hprio hbud hDT hL i v hv hsched hx hpos⟩

end P18
