import Sched.Time
open Aeneas Aeneas.Std Result

/-! # Periodic arrivals at the Kernel's nominal instants

The run of `Sched/KRun.lean` takes "two arrivals of a Task are at least one
period apart" as a condition of each arrival (`KStep.jarrive`). For a
Periodic_Task this file derives it: when an arrival is the Kernel's own
nominal instant of the Task, each instant arriving once, the spacing is a
consequence of the code, because the nominal instant only ever moves by
the Task's period (`Sched/Time.lean`) and nothing else the run does
changes it.

- `TimeKept` for the Kernel functions of the run not reached from a served
  slot: `dispatch`, `job_return`, `svc_lock`, `svc_unlock`,
  `intake_release`, `settle`.
- `NomRel`: between two Kernel states every Task's nominal instant is
  where it was or one period later. Every code-modelled transition of the
  run satisfies it (`kstep_nomrel`); for the transitions modelled by hand
  it is a condition (they leave the rest of the Kernel's state free).
- `TStep`, `TReach`: the timed run, in which a Periodic_Task's arrival
  carries no spacing condition. `treach_kreach`: every timed run is a run
  of `KRun.lean`. `property18_timed`: Property 18 over it. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! ## The remaining Kernel functions keep the reading and the nominal instants -/

theorem defer_nom {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {now a : U64}
    (h : logic.time.Timing.defer tm cfg t now = ok (a, tm')) : tm'.nominal = tm.nominal := by
  unfold logic.time.Timing.defer at h
  peel h
  all_goals rfl

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (defer_nom (by assumption)).trans ?_)

set_option maxHeartbeats 8000000 in
theorem dispatch_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {pc : U32}
    (h : logic.kernel.Kernel.dispatch k cfg t pc = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.dispatch at h
  peel h
  all_goals tk_close

set_option maxHeartbeats 8000000 in
theorem job_return_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {ec : U32}
    (h : logic.kernel.Kernel.job_return k cfg ec = ok k') : TimeKept k k' := by
  unfold logic.kernel.Kernel.job_return at h
  peel h
  all_goals tk_close

set_option maxHeartbeats 8000000 in
theorem svc_lock_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_lock k cfg r = ok (b, k')) : TimeKept k k' := by
  unfold logic.kernel.Kernel.svc_lock at h
  peel h
  all_goals tk_close

set_option maxHeartbeats 8000000 in
theorem svc_unlock_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (b, k')) : TimeKept k k' := by
  unfold logic.kernel.Kernel.svc_unlock at h
  peel h
  all_goals tk_close

set_option maxHeartbeats 8000000 in
theorem settle_tk {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {r : Option U8 × Bool}
    (h : logic.kernel.Kernel.settle k cfg = ok (r, k')) : TimeKept k k' := by
  unfold logic.kernel.Kernel.settle at h
  peel h
  all_goals tk_close

theorem observe_nom {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64}
    (h : logic.kernel.Kernel.observe k cfg now = ok k') :
    k'.timing.nominal = k.timing.nominal := by
  unfold logic.kernel.Kernel.observe at h
  split at h <;> (have h := ok_inj h; subst h; rfl)

macro_rules
  | `(tactic| tk_step) => `(tactic| refine (observe_nom (by assumption)).trans ?_)

set_option maxHeartbeats 16000000 in
/-- `Kernel::intake_release` moves the reading (it observes the time base)
and leaves every nominal instant. -/
theorem intake_release_nom {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t sp off : U8} {now : U64}
    (h : logic.kernel.Kernel.intake_release k cfg t sp now off = ok k') :
    k'.timing.nominal = k.timing.nominal := by
  unfold logic.kernel.Kernel.intake_release at h
  peel h
  all_goals (repeat (first | tk_step | dsimp only))

/-! ## Nominal instants move by a period or not at all -/

/-- Task `u`'s nominal instant in the Kernel's state, in ticks. -/
def nomOf (k : logic.kernel.Kernel NT NR NP NL NS NE) (u : Nat) : Nat :=
  (k.timing.nominal.val[u]!).val

/-- Task `u`'s period in the configuration, in ticks. -/
def cfgPeriod (cfg : logic.Config NT NR NP) (u : Nat) : Nat :=
  match cfg.tasks.val[u]? with
  | some tc => tc.period.val
  | none => 0

/-- From `k` to `k'` every Task's nominal instant is where it was or one
period of that Task later. -/
def NomRel (cfg : logic.Config NT NR NP) (k k' : logic.kernel.Kernel NT NR NP NL NS NE) : Prop :=
  ∀ u, u < NT.val → nomOf k' u = nomOf k u ∨ nomOf k' u = nomOf k u + cfgPeriod cfg u

theorem NomRel.of_eq {cfg : logic.Config NT NR NP}
    {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : k'.timing.nominal = k.timing.nominal) : NomRel cfg k k' := by
  intro u _
  left
  unfold nomOf
  rw [h]

/-- `Timing::advance_periodic`, element by element. -/
theorem advance_nomrel {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {o : Option U64}
    (h : logic.time.Timing.advance_periodic tm cfg t = ok (o, tm')) (u : Nat) :
    (tm'.nominal.val[u]!).val = (tm.nominal.val[u]!).val ∨
    (tm'.nominal.val[u]!).val = (tm.nominal.val[u]!).val + cfgPeriod cfg u := by
  rcases advance_periodic_cases h with ⟨_, rfl⟩ | ⟨tc, n, next, htc, hn, hnext, _, _, hup⟩
  · exact Or.inl rfl
  · obtain ⟨hb, e⟩ := update_ok hup
    rw [e, get_set _ _ _ _ hb]
    split
    · rename_i hu
      right
      obtain ⟨_, hn'⟩ := index_ok hn
      obtain ⟨_, htc'⟩ := index_ok' htc
      unfold cfgPeriod
      rw [hu, ← hn', htc']
      exact hnext
    · exact Or.inl rfl

/-- `Kernel::periodic_release`, of a Task of any kind. -/
theorem periodic_release_nomrel {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.periodic_release k cfg t i = ok k') : NomRel cfg k k' := by
  have h0 := h
  unfold logic.kernel.Kernel.periodic_release at h0
  simp only [bind_eq_ok, lift, bind_ok] at h0
  obtain ⟨tc, htc, nominal, hnom, tc1, htc1, h1⟩ := h0
  by_cases hk : tc.kind = logic.KIND_PERIODIC
  · obtain ⟨k1, o, htk, hadv, _⟩ := periodic_release_nominal h htc hk
    intro u _
    have := advance_nomrel hadv u
    unfold nomOf
    rw [← htk.2]
    exact this
  · rw [htc] at htc1
    have e := ok_inj htc1
    subst e
    have hne : (tc.kind != logic.KIND_PERIODIC) = true := by simp [hk]
    simp only [hne, ite_true, bind_eq_ok] at h1
    obtain ⟨sl, _, h1⟩ := h1
    have h1 := ok_inj h1
    subst h1
    exact NomRel.of_eq rfl

/-- The shape of the transitions `KRun.lean` models by hand, which leave
the Kernel's state outside the scheduler free: the scheduler state kept,
or the Job on top ended, or the safe state entered from a Kernel state
with this scheduler state. -/
def HandShape (cfg : logic.Config NT NR NP) (x y : KSt NT NR NP NL NS NE) : Prop :=
  y.k.sched = x.k.sched ∨
  (∃ r, logic.sched.Sched.finish_top x.k.sched cfg = ok (r, y.k.sched)) ∨
  (∃ k1 : logic.kernel.Kernel NT NR NP NL NS NE,
    k1.sched = x.k.sched ∧ logic.kernel.Kernel.enter_safe_state k1 cfg = ok y.k)

/-- Every transition of the run keeps each nominal instant or moves it by
its Task's period: from the code for the transitions that are the Kernel's
functions, and for the hand-modelled ones (which fix the scheduler state
only) under the condition that they do. -/
theorem kstep_nomrel {cfg : logic.Config NT NR NP} {P : Params}
    {x y : KSt NT NR NP NL NS NE} (h : KStep cfg P x y)
    (hh : HandShape cfg x y → NomRel cfg x.k y.k) : NomRel cfg x.k y.k := by
  cases h with
  | jarrive => exact NomRel.of_eq rfl
  | earrive => exact NomRel.of_eq rfl
  | periodic q t i d c k' hq hk hw hrel hnd hc hs hmade =>
    have := periodic_release_nomrel hs
    split <;> exact this
  | intake q t sp off now d c k' hq hk hw hrel hnd hc hs hmade =>
    have := NomRel.of_eq (cfg := cfg) (intake_release_nom hs)
    split <;> exact this
  | dispatch t pc c k' hk hsel hc hs => exact NomRel.of_eq (dispatch_tk hs).2
  | lock hsel hk r a c k' hrun hrl hc ha hal hs => exact NomRel.of_eq (svc_lock_tk hs).2
  | unlock hsel hk r c k' hrun hru hc hal hs => exact NomRel.of_eq (svc_unlock_tk hs).2
  | finish hsel hk hj ec c k' hd hr0 hr hc hal hs => exact NomRel.of_eq (job_return_tk hs).2
  | unwind ec k' hs => exact NomRel.of_eq (job_return_tk hs).2
  | resp partition response cause t k' hs => exact NomRel.of_eq (apply_response_tk hs).2
  | bserved t i k' hs => exact NomRel.of_eq (budget_event_tk hs).2
  | opend k1 k' h1 hs => exact hh (Or.inr (Or.inr ⟨k1, h1, hs⟩))
  | settle r k' hs => exact NomRel.of_eq (settle_tk hs).2
  | deferred q t i d c k' hq hk hw hrel hnd hc hs hmade =>
    have := NomRel.of_eq (cfg := cfg) (deferred_release_tk hs).2
    split <;> exact this
  | deadline q c t i k' hq hk hw hc hdl hs => exact NomRel.of_eq (deadline_miss_tk hs).2
  | plain_h q c k' hq hk hw hc hnd hs => exact hh (Or.inl hs)
  | svc_h hsel hk c k' hd hrun hre hal hs => exact hh (Or.inl hs)
  | bevent_h k0 r0 hset harmed hk c k' hd hrun hrb hc hs => exact hh (Or.inl hs)
  | overrun_h k0 r0 hset harmed hk hj c r k' hd hr0 hr hstop hrb hc hal hs =>
    exact hh (Or.inr (Or.inl ⟨r, hs⟩))
  | ktick => exact NomRel.of_eq rfl
  | idle => exact NomRel.of_eq rfl
  | exec => exact NomRel.of_eq rfl

/-! ## The timed run -/

/-- The time base of a run: how many of the run's time units one tick of
the Kernel's time base is, which Tasks are periodic, and that the period
the analysis uses for a Periodic_Task is not longer than its configured
period (R29.8 rounds periods down). -/
structure TimeBase (cfg : logic.Config NT NR NP) (P : Params) where
  tk : Nat
  periodic : Nat → Bool
  hT : ∀ t, t < NT.val → periodic t = true → P.T t ≤ tk * cfgPeriod cfg t

/-- One transition of the timed run.

- `parrive`: the Job of a Periodic_Task arrives. The instant is the Task's
  nominal instant in the Kernel's state, and that instant has not arrived
  before. No spacing from the previous arrival is assumed.
- `other`: any transition of `KRun.lean` that is no arrival of a
  Periodic_Task; if it has the shape of a hand-modelled transition, it
  moves each nominal instant by a period or not at all. -/
inductive TStep (cfg : logic.Config NT NR NP) (P : Params) (B : TimeBase cfg P) :
    KSt NT NR NP NL NS NE → KSt NT NR NP NL NS NE → Prop
  | parrive (x : KSt NT NR NP NL NS NE) (t : Nat) (ht : t < NT.val)
      (hp : B.periodic t = true)
      (hnow : x.now = B.tk * nomOf x.k t)
      (hfresh : ∀ r, x.jl t = some r → r < x.now) :
      TStep cfg P B x { x with
        jl := Function.update x.jl t (some x.now),
        jwait := Function.update x.jwait t true }
  | other (x y : KSt NT NR NP NL NS NE) (h : KStep cfg P x y)
      (hjl : ∀ t, t < NT.val → B.periodic t = true → y.jl t = x.jl t)
      (hh : HandShape cfg x y → NomRel cfg x.k y.k) : TStep cfg P B x y

/-- The states the timed run reaches from boot. -/
inductive TReach (cfg : logic.Config NT NR NP) (P : Params) (B : TimeBase cfg P) :
    KSt NT NR NP NL NS NE → Prop
  | boot (k0 : logic.kernel.Kernel NT NR NP NL NS NE)
      (h : logic.sched.Sched.new NL cfg = ok k0.sched) : TReach cfg P B (KSt.boot k0)
  | step (x y : KSt NT NR NP NL NS NE) (h : TReach cfg P B x) (hs : TStep cfg P B x y) :
      TReach cfg P B y

/-- The last arrival of every Periodic_Task lies a whole number of its
periods before the Task's present nominal instant. -/
def ArrInv (cfg : logic.Config NT NR NP) (P : Params) (B : TimeBase cfg P)
    (x : KSt NT NR NP NL NS NE) : Prop :=
  ∀ t, t < NT.val → B.periodic t = true → ∀ r, x.jl t = some r →
    ∃ j : Nat, B.tk * nomOf x.k t = r + j * (B.tk * cfgPeriod cfg t)

/-- Every timed run is a run of `KRun.lean`: the spacing of a
Periodic_Task's arrivals, which that run takes as a condition, holds. -/
theorem treach_kreach {cfg : logic.Config NT NR NP} {P : Params} {B : TimeBase cfg P}
    {x : KSt NT NR NP NL NS NE} (h : TReach cfg P B x) :
    KReach cfg P x ∧ ArrInv cfg P B x := by
  induction h with
  | boot k0 h0 =>
    refine ⟨KReach.boot k0 h0, ?_⟩
    intro t _ _ r hr
    simp [KSt.boot] at hr
  | step x y _ hs ih =>
    obtain ⟨hk, hinv⟩ := ih
    cases hs with
    | parrive t ht hp hnow hfresh =>
      refine ⟨KReach.step _ _ hk (KStep.jarrive x t ht ?_), ?_⟩
      · intro r hr
        obtain ⟨j, hj⟩ := hinv t ht hp r hr
        have hlt := hfresh r hr
        have hT := B.hT t ht hp
        rw [hnow, hj] at hlt
        rw [hnow, hj]
        have hj1 : 1 ≤ j := by
          rcases Nat.eq_zero_or_pos j with h0 | h0
          · subst h0; simp at hlt
          · exact h0
        have : B.tk * cfgPeriod cfg t ≤ j * (B.tk * cfgPeriod cfg t) := Nat.le_mul_of_pos_left _ hj1
        omega
      · intro u hu hpu r hr
        dsimp only at hr ⊢
        by_cases hut : u = t
        · subst hut
          simp only [Function.update_self, Option.some.injEq] at hr
          subst hr
          exact ⟨0, by rw [hnow]; simp⟩
        · rw [Function.update_of_ne hut] at hr
          exact hinv u hu hpu r hr
    | other _ h hjl hh =>
      refine ⟨KReach.step _ _ hk h, ?_⟩
      intro u hu hpu r hr
      rw [hjl u hu hpu] at hr
      obtain ⟨j, hj⟩ := hinv u hu hpu r hr
      rcases kstep_nomrel h hh u hu with e | e
      · exact ⟨j, by rw [e]; exact hj⟩
      · refine ⟨j + 1, ?_⟩
        rw [e, Nat.mul_add, hj, Nat.add_mul, Nat.one_mul, Nat.add_assoc]

open rsk_analyzer (candidateOf) in
open rsk_analyzer.terms (TaskIn Site Window Verdict) in
/-- Property 18 over the timed run: the bound `analyse_all` reports holds
in every run in which the Jobs of the Periodic_Tasks arrive at the
Kernel's own nominal instants, with no assumption on their spacing. -/
theorem property18_timed {cfg : logic.Config NT NR NP}
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
    (hprio : ∀ k t, tasks.val[k]? = some t → 0 < t.priority.val ∧ prio cfg k = t.priority.val)
    (hbud : ∀ t ∈ tasks.val, 0 < t.budget.val)
    (hDT : ∀ w ∈ windows.val, w.deadline.val ≤ w.period.val)
    (hL : ∀ i ti k r, tasks.val[i]? = some ti → k < NT.val → prio cfg k < prio cfg i →
      prio cfg i ≤ ceil cfg r →
      L k r = 0 ∨
      ∃ (j : Nat) (s : Site) (l : TaskIn), sites.val[j]? = some s
        ∧ tasks.val[s.task.val]? = some l
        ∧ l.priority.val < ti.priority.val ∧ ti.priority.val ≤ s.ceiling.val
        ∧ L k r ≤ candidateOf ti l s p)
    (B : TimeBase cfg (runParams NT.val tasks.val sites.val p windows.val L))
    (i : Nat) (v : Verdict) (hv : vs.val[i]? = some v) (hsched : v.schedulable = true)
    {x : KSt NT NR NP NL NS NE}
    (hreach : TReach cfg (runParams NT.val tasks.val sites.val p windows.val L) B x)
    (hpos : 0 < x.pend i + x.run i) :
    ∃ a, x.jl i = some a ∧ x.now < a + v.r.val :=
  property18_kernel tasks sites p any_fpu windows ds vs tss quiet L hall hlen hNT hsok hwl hprio
    hbud hDT hL i v hv hsched (treach_kreach hreach).1 hpos

end P18
