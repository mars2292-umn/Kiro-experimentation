import Sched.MaskFrame
open Aeneas Aeneas.Std Result

/-! # One effective intake per MIT per source (2026-11-25)

Property 9 of the design (R19.1), which Verus proves of `rsk_logic` over
its ghost history (`Kernel::intakes_linked`, `history::intakes_spaced`,
`lemma_intake_spacing_over_run`, `lemma_intake_count_bound`), here over
the run of the Kernel's functions as Aeneas extracts them
(`Sched/KRun.lean`). The ghost history is erased from the compiled crate,
so an effective intake is named by what the code does:

- `MS`: what the three functions that set a mask do (`Timing::mask`,
  `accept`, `defer`): the source of the Task is masked, its window ends one
  MIT after the instant given, and no other Task's mask or window changes.
- `intake_release_chain`: **every outcome of an intake masks the source.**
  `Kernel::intake_release` reads the instant (`Kernel::observe`); then
  either it returns that state (the three early returns: the safe state,
  before the start, a stopped Partition), or it sets the mask of its Task
  for one MIT from that instant by one of the three functions (accepted,
  deferred, discarded, or refused as too late for the operating duration),
  and what follows (the MIT_Violation's response) is a function of
  `Sched/MaskFrame.lean`.
- `MI`: the invariant, for an instant `a` and a Task `u`: `a` is not after
  the Kernel's instant, and one MIT has passed since `a` or the source of
  `u` is masked with its window ending no earlier than `a` plus one MIT
  (`intakes_linked` for one intake). `kstep_mi`: every transition of the
  run keeps it.
- `IRun`: a stretch of a run with the instants of effective intakes of
  Task `t` in it, the latest first. An effective intake is an intake of `t`
  that finds its source unmasked and leaves it masked: an event the mask
  could not have stopped, which reached the decision.
- `irun_spaced`: **two effective intakes of one source are at least one
  MIT apart** (R19.1; `lemma_intake_spacing_over_run`), whatever the
  source's event rate: each is at least one MIT after the one before.
- `irun_count`: so `n` effective intakes span at least `n - 1` MITs, and a
  window shorter than `w` holds fewer than `w / MIT + 1` of them
  (`lemma_intake_count_bound`): the bound on a source's entries that the
  response-time analysis charges (Δ_irq per MIT).

The instants are the Kernel's (`Kernel::now` after the intake's reading),
as in the Verus history. The MIT is the Task's `period` in the
configuration (`cfgPeriod`). A stretch lists the effective intakes it
names; the statements hold of every such list, the list of all of them
included. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! ## The functions that set a mask -/

/-- From `tm` to `tm'` the source of Task `t` is masked with its window
ending one MIT after `n`, and no other Task's mask or window changes. -/
def MS (cfg : logic.Config NT NR NP) (n : U64) (t : U8) (tm tm' : logic.time.Timing NT) : Prop :=
  mk tm' t.val = true ∧ mu tm' t.val = n.val + cfgPeriod cfg t.val ∧
  ∀ u, u ≠ t.val → mk tm' u = mk tm u ∧ mu tm' u = mu tm u

/-- The two writes of a mask, as `MS`. -/
theorem ms_of_sets {cfg : logic.Config NT NR NP} {n e : U64} {t : U8}
    {tm tm' : logic.time.Timing NT} {tc : logic.TaskCfg}
    (ht : t.val < NT.val) (htc : cfg.tasks.val[t.val]? = some tc)
    (he : e.val = n.val + tc.period.val)
    (hm : tm'.masked = tm.masked.set (UScalar.cast .Usize t) true)
    (hu : tm'.mask_until = tm.mask_until.set (UScalar.cast .Usize t) e) :
    MS cfg n t tm tm' := by
  have hc : (UScalar.cast .Usize t).val = t.val := U8.cast_Usize_val_eq t
  have hlt : (UScalar.cast .Usize t).val < NT.val := by rw [hc]; exact ht
  refine ⟨?_, ?_, fun u hne => ⟨?_, ?_⟩⟩
  · unfold mk
    rw [hm, get_set _ _ _ _ hlt, hc]
    simp
  · unfold mu cfgPeriod
    rw [hu, get_set _ _ _ _ hlt, hc, htc]
    simp only [ite_true]
    exact he
  · unfold mk
    rw [hm, get_set _ _ _ _ hlt, hc]
    simp only [hne, ite_false]
  · unfold mu
    rw [hu, get_set _ _ _ _ hlt, hc]
    simp only [hne, ite_false]

theorem mask_ms {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8} {now : U64}
    (h : logic.time.Timing.mask tm cfg t now = ok tm') : MS cfg now t tm tm' := by
  unfold logic.time.Timing.mask at h
  simp only [lift, bind_ok, bind_eq_ok] at h
  obtain ⟨a, ha, tc, htc, i2, hi2, a1, ha1, h⟩ := h
  have h := ok_inj h
  subst h
  obtain ⟨hb, rfl⟩ := update_ok ha
  obtain ⟨_, rfl⟩ := update_ok ha1
  obtain ⟨_, htc'⟩ := index_ok' htc
  rw [U8.cast_Usize_val_eq] at hb htc'
  exact ms_of_sets hb htc' (u64_add_ok hi2) rfl rfl

theorem accept_ms {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    {now : U64}
    (h : logic.time.Timing.accept tm cfg t now = ok tm') : MS cfg now t tm tm' := by
  unfold logic.time.Timing.accept at h
  simp only [lift, bind_ok, bind_eq_ok] at h
  obtain ⟨a, ha, a1, ha1, a2, ha2, a3, ha3, tc, htc, i5, hi5, a4, ha4, h⟩ := h
  have h := ok_inj h
  subst h
  obtain ⟨hb, rfl⟩ := update_ok ha3
  obtain ⟨_, rfl⟩ := update_ok ha4
  obtain ⟨_, htc'⟩ := index_ok' htc
  rw [U8.cast_Usize_val_eq] at hb htc'
  exact ms_of_sets hb htc' (u64_add_ok hi5) rfl rfl

theorem defer_ms {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    {now a : U64}
    (h : logic.time.Timing.defer tm cfg t now = ok (a, tm')) : MS cfg now t tm tm' := by
  unfold logic.time.Timing.defer at h
  simp only [lift, bind_ok, bind_eq_ok] at h
  obtain ⟨i1, hi1, tc, htc, at0, hat, a0, ha0, a1, ha1, tc1, htc1, i6, hi6, a2, ha2, h⟩ := h
  have h := ok_inj h
  rw [Prod.mk.injEq] at h
  obtain ⟨_, rfl⟩ := h
  obtain ⟨hb, rfl⟩ := update_ok ha1
  obtain ⟨_, rfl⟩ := update_ok ha2
  obtain ⟨_, htc'⟩ := index_ok' htc1
  rw [U8.cast_Usize_val_eq] at hb htc'
  exact ms_of_sets hb htc' (u64_add_ok hi6) rfl rfl

/-! ## The invariant -/

/-- For an instant `a` and a Task `u`, at the instant and timing state `v`:
`a` is not after the instant, and one MIT of `u` has passed since `a` or
the source of `u` is masked with its window ending no earlier than `a` plus
one MIT. -/
def MI (cfg : logic.Config NT NR NP) (v : U64 × logic.time.Timing NT) (a u : Nat) : Prop :=
  a ≤ v.1.val ∧
  (a + cfgPeriod cfg u ≤ v.1.val ∨ (mk v.2 u = true ∧ a + cfgPeriod cfg u ≤ mu v.2 u))

/-- A function of `Sched/MaskFrame.lean` keeps it. -/
theorem km_keeps {cfg : logic.Config NT NR NP} {v v' : U64 × logic.time.Timing NT} {a u : Nat}
    (h : KM v v') (hi : MI cfg v a u) : MI cfg v' a u := by
  obtain ⟨hn, hm, hr⟩ := h
  obtain ⟨ha, hd⟩ := hi
  refine ⟨by rw [hn]; exact ha, ?_⟩
  rcases hd with hd | ⟨hk, hd⟩
  · exact Or.inl (by rw [hn]; exact hd)
  · rcases hr u hk with h' | h'
    · right
      refine ⟨h', ?_⟩
      have e : mu v'.2 u = mu v.2 u := by unfold mu; rw [hm]
      rw [e]; exact hd
    · left
      rw [hn]; omega

/-- A mask set at an instant not before `a` keeps it, -/
theorem ms_keeps {cfg : logic.Config NT NR NP} {n : U64} {t : U8}
    {tm tm' : logic.time.Timing NT} {a u : Nat}
    (h : MS cfg n t tm tm') (hi : MI cfg (n, tm) a u) : MI cfg (n, tm') a u := by
  obtain ⟨h1, h2, h3⟩ := h
  obtain ⟨ha, hd⟩ := hi
  dsimp only at ha hd
  refine ⟨ha, ?_⟩
  dsimp only
  by_cases hu : u = t.val
  · right
    subst hu
    exact ⟨h1, by rw [h2]; omega⟩
  · obtain ⟨e1, e2⟩ := h3 u hu
    rw [e1, e2]
    exact hd

/-- and establishes it for the instant it is set at. -/
theorem ms_sets {cfg : logic.Config NT NR NP} {n : U64} {t : U8}
    {tm tm' : logic.time.Timing NT} (h : MS cfg n t tm tm') : MI cfg (n, tm') n.val t.val :=
  ⟨Nat.le_refl _, Or.inr ⟨h.1, by rw [h.2.1]⟩⟩

/-- `Kernel::observe` never moves the instant back and leaves the timing
state. -/
theorem observe_now_le {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64}
    (h : logic.kernel.Kernel.observe k cfg now = ok k') : k.now.val ≤ k'.now.val := by
  unfold logic.kernel.Kernel.observe at h
  split at h
  · rename_i hgt
    have h := ok_inj h
    subst h
    dsimp only
    scalar_tac
  · have h := ok_inj h
    subst h
    exact Nat.le_refl _

theorem observe_keeps {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64} {a u : Nat}
    (h : logic.kernel.Kernel.observe k cfg now = ok k') (hi : MI cfg (kv k) a u) :
    MI cfg (kv k') a u := by
  have hn := observe_now_le h
  have ht := observe_timing h
  obtain ⟨ha, hd⟩ := hi
  have ha' : a ≤ k.now.val := ha
  have hd' : a + cfgPeriod cfg u ≤ k.now.val ∨
      (mk k.timing u = true ∧ a + cfgPeriod cfg u ≤ mu k.timing u) := hd
  show a ≤ k'.now.val ∧ (a + cfgPeriod cfg u ≤ k'.now.val ∨
      (mk k'.timing u = true ∧ a + cfgPeriod cfg u ≤ mu k'.timing u))
  rw [ht]
  refine ⟨by omega, ?_⟩
  rcases hd' with hd' | hd'
  · exact Or.inl (by omega)
  · exact Or.inr hd'

/-! ## Every outcome of an intake masks the source -/

set_option maxHeartbeats 4000000 in
/-- `Kernel::intake_release`: the instant is read; then the state read is
returned (the safe state, before the start, a stopped Partition), or the
mask of the Task is set for one MIT from that instant, by `Timing::mask`
(too late for the operating duration, or discarded), `Timing::accept` or
`Timing::defer`, and what follows keeps the masks (`KM`). -/
theorem intake_release_chain {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t sp off : U8} {now : U64}
    (h : logic.kernel.Kernel.intake_release k cfg t sp now off = ok k') :
    ∃ k1, logic.kernel.Kernel.observe k cfg now = ok k1 ∧
      (kv k' = kv k1 ∨
       ∃ tm2, MS cfg k1.now t k1.timing tm2 ∧ KM (k1.now, tm2) (kv k')) := by
  unfold logic.kernel.Kernel.intake_release at h
  peel h
  all_goals
    refine ⟨_, ‹logic.kernel.Kernel.observe k cfg now = ok _›, ?_⟩
    first
      | exact Or.inl rfl
      | exact Or.inr ⟨_, mask_ms (by assumption), KM.refl _⟩
      | exact Or.inr ⟨_, accept_ms (by assumption), KM.refl _⟩
      | (have hv := mit_violation_km h
         dsimp only [kv] at hv
         first
           | exact Or.inr ⟨_, mask_ms (by assumption), hv⟩
           | exact Or.inr ⟨_, defer_ms (by assumption), hv⟩)

/-- An intake keeps the invariant of every instant and Task. -/
theorem intake_release_keeps {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t sp off : U8} {now : U64} {a u : Nat}
    (h : logic.kernel.Kernel.intake_release k cfg t sp now off = ok k')
    (hi : MI cfg (kv k) a u) : MI cfg (kv k') a u := by
  obtain ⟨k1, ho, hc⟩ := intake_release_chain h
  have h1 := observe_keeps ho hi
  rcases hc with e | ⟨tm2, hms, hkm⟩
  · rw [e]; exact h1
  · exact km_keeps hkm (ms_keeps hms h1)

/-- An intake never moves the instant back. -/
theorem intake_release_now {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t sp off : U8} {now : U64}
    (h : logic.kernel.Kernel.intake_release k cfg t sp now off = ok k') :
    k.now.val ≤ k'.now.val := by
  obtain ⟨k1, ho, hc⟩ := intake_release_chain h
  have h1 := observe_now_le ho
  rcases hc with e | ⟨tm2, _, hkm⟩
  · have : k'.now = k1.now := congrArg Prod.fst e
    rw [this]; exact h1
  · have : k'.now = k1.now := hkm.1
    rw [this]; exact h1

/-- **An intake that finds its source unmasked and leaves it masked
establishes the invariant for its own instant**: the source stays masked
until one MIT after it. -/
theorem intake_release_sets {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t sp off : U8} {now : U64}
    (h : logic.kernel.Kernel.intake_release k cfg t sp now off = ok k')
    (h0 : mk k.timing t.val = false) (h1 : mk k'.timing t.val = true) :
    MI cfg (kv k') k'.now.val t.val := by
  obtain ⟨k1, ho, hc⟩ := intake_release_chain h
  rcases hc with e | ⟨tm2, hms, hkm⟩
  · exfalso
    have et : k'.timing = k1.timing := congrArg Prod.snd e
    rw [et, observe_timing ho, h0] at h1
    cases h1
  · have en : k'.now = k1.now := hkm.1
    rw [en]
    exact km_keeps hkm (ms_sets hms)

/-! ## Over the run -/

/-- Every transition of the run over the Kernel's functions keeps the
invariant of every instant and Task. -/
theorem kstep_mi {cfg : logic.Config NT NR NP} {P : Params} {x y : KSt NT NR NP NL NS NE}
    {a u : Nat} (h : KStep cfg P x y) (hi : MI cfg (kv x.k) a u) : MI cfg (kv y.k) a u := by
  cases h <;> (try dsimp only) <;> (try split) <;> (try dsimp only) <;> first
    | exact hi
    | exact intake_release_keeps (by assumption) hi
    | exact observe_keeps (by assumption) hi
    | exact km_keeps (periodic_release_km (by assumption)) hi
    | exact km_keeps (dispatch_km (by assumption)) hi
    | exact km_keeps (svc_lock_km (by assumption)) hi
    | exact km_keeps (svc_unlock_km (by assumption)) hi
    | exact km_keeps (job_return_km (by assumption)) hi
    | exact km_keeps (apply_response_km (by assumption)) hi
    | exact km_keeps (fault_km (by assumption)) hi
    | exact km_keeps (fault_of_km (by assumption)) hi
    | exact km_keeps (budget_event_km (by assumption)) hi
    | exact km_keeps (serve_slot_km (by assumption)) hi
    | exact km_keeps (settle_km (by assumption)) hi
    | exact km_keeps (ack_reinit_km (by assumption)) hi
    | exact km_keeps (ack_stop_dma_km (by assumption)) hi
    | exact km_keeps (push_event_km (by assumption)) hi
    | exact km_keeps (deferred_release_km (by assumption)) hi
    | exact km_keeps (deadline_miss_km (by assumption)) hi
    | (rename_i hcode
       rcases hcode with e | ⟨kind, t, hr⟩
       · rw [e]; exact hi
       · exact km_keeps (record_event_km hr) hi)

/-- No transition moves the Kernel's instant back. -/
theorem kstep_now {cfg : logic.Config NT NR NP} {P : Params} {x y : KSt NT NR NP NL NS NE}
    (h : KStep cfg P x y) : x.k.now.val ≤ y.k.now.val := by
  have e : ∀ (k k' : logic.kernel.Kernel NT NR NP NL NS NE), KM (kv k) (kv k') →
      k.now.val ≤ k'.now.val := fun k k' hk => by
    have : k'.now = k.now := hk.1
    rw [this]
  cases h <;> (try dsimp only) <;> (try split) <;> (try dsimp only) <;> first
    | exact Nat.le_refl _
    | exact intake_release_now (by assumption)
    | exact observe_now_le (by assumption)
    | exact e _ _ (periodic_release_km (by assumption))
    | exact e _ _ (dispatch_km (by assumption))
    | exact e _ _ (svc_lock_km (by assumption))
    | exact e _ _ (svc_unlock_km (by assumption))
    | exact e _ _ (job_return_km (by assumption))
    | exact e _ _ (apply_response_km (by assumption))
    | exact e _ _ (fault_km (by assumption))
    | exact e _ _ (fault_of_km (by assumption))
    | exact e _ _ (budget_event_km (by assumption))
    | exact e _ _ (serve_slot_km (by assumption))
    | exact e _ _ (settle_km (by assumption))
    | exact e _ _ (ack_reinit_km (by assumption))
    | exact e _ _ (ack_stop_dma_km (by assumption))
    | exact e _ _ (push_event_km (by assumption))
    | exact e _ _ (deferred_release_km (by assumption))
    | exact e _ _ (deadline_miss_km (by assumption))
    | (rename_i hcode
       rcases hcode with e' | ⟨kind, t, hr⟩
       · rw [e']
       · exact e _ _ (record_event_km hr))

/-- A stretch of a run from `x` to `y`, with the Kernel's instants of
effective intakes of Task `t` in it, the latest first. An effective intake
is a transition that is an intake of `t` which finds the source unmasked
and leaves it masked. -/
inductive IRun (cfg : logic.Config NT NR NP) (P : Params) (t : U8) :
    KSt NT NR NP NL NS NE → List Nat → KSt NT NR NP NL NS NE → Prop
  | refl (x : KSt NT NR NP NL NS NE) : IRun cfg P t x [] x
  | step (x y z : KSt NT NR NP NL NS NE) (l : List Nat) (h : IRun cfg P t x l y)
      (hs : KStep cfg P y z) : IRun cfg P t x l z
  | eff (x y z : KSt NT NR NP NL NS NE) (l : List Nat) (h : IRun cfg P t x l y)
      (hs : KStep cfg P y z) (sp off : U8) (now : U64)
      (hi : logic.kernel.Kernel.intake_release y.k cfg t sp now off = ok z.k)
      (h0 : mk y.k.timing t.val = false) (h1 : mk z.k.timing t.val = true) :
      IRun cfg P t x (z.k.now.val :: l) z

/-- Each instant of the list is at least one MIT after the next. -/
def Spaced (per : Nat) : List Nat → Prop
  | [] => True
  | [_] => True
  | a :: b :: l => b + per ≤ a ∧ Spaced per (b :: l)

/-- Along a stretch, the invariant holds of every effective intake in it,
and each is at least one MIT after the one before. -/
theorem irun_inv {cfg : logic.Config NT NR NP} {P : Params} {t : U8}
    {x y : KSt NT NR NP NL NS NE} {l : List Nat} (h : IRun cfg P t x l y) :
    (∀ a, a ∈ l → MI cfg (kv y.k) a t.val) ∧ Spaced (cfgPeriod cfg t.val) l := by
  induction h with
  | refl => exact ⟨fun a ha => (by cases ha), trivial⟩
  | step y z l _ hs ih => exact ⟨fun a ha => kstep_mi hs (ih.1 a ha), ih.2⟩
  | eff y z l _ hs sp off now hi h0 h1 ih =>
    obtain ⟨hall, hsp⟩ := ih
    refine ⟨fun a ha => ?_, ?_⟩
    · rcases List.mem_cons.mp ha with e | hm
      · rw [e]; exact intake_release_sets hi h0 h1
      · exact kstep_mi hs (hall a hm)
    · cases l with
      | nil => trivial
      | cons b l' =>
        refine ⟨?_, hsp⟩
        obtain ⟨_, hd⟩ := hall b (List.mem_cons_self ..)
        dsimp only [kv] at hd
        have hn := intake_release_now hi
        rcases hd with hd | ⟨hk, _⟩
        · omega
        · rw [h0] at hk; cases hk

/-- **Two effective intakes of one source are at least one MIT apart
(R19.1; `lemma_intake_spacing_over_run`).** -/
theorem irun_spaced {cfg : logic.Config NT NR NP} {P : Params} {t : U8}
    {x y : KSt NT NR NP NL NS NE} {l : List Nat} (h : IRun cfg P t x l y) :
    Spaced (cfgPeriod cfg t.val) l :=
  (irun_inv h).2

/-- Instants each at least `per` after the next: the first is at least
`(n - 1) * per` after the last, `n` their number. -/
theorem spaced_span {per : Nat} : ∀ (l : List Nat) (a : Nat), Spaced per (a :: l) →
    (a :: l).getLast (List.cons_ne_nil _ _) + l.length * per ≤ a
  | [], a, _ => by simp
  | b :: l, a, h => by
    obtain ⟨hab, hs⟩ := h
    have ih := spaced_span l b hs
    have e : (a :: b :: l).getLast (List.cons_ne_nil _ _) = (b :: l).getLast (List.cons_ne_nil _ _) :=
      List.getLast_cons (List.cons_ne_nil _ _)
    rw [e]
    simp only [List.length_cons, Nat.add_mul, Nat.one_mul]
    omega

/-- **The count bound (R19.1; `lemma_intake_count_bound`).** In a stretch
of a run, the latest of `n` effective intakes of one source is at least
`(n - 1)` MITs after the earliest; so if they all lie in a window shorter
than `w`, then `(n - 1) * MIT < w`: at most one entry per MIT, whatever the
source's event rate. -/
theorem irun_count {cfg : logic.Config NT NR NP} {P : Params} {t : U8}
    {x y : KSt NT NR NP NL NS NE} {a : Nat} {l : List Nat} (h : IRun cfg P t x (a :: l) y) :
    (a :: l).getLast (List.cons_ne_nil _ _) + l.length * cfgPeriod cfg t.val ≤ a ∧
    ∀ w, a < (a :: l).getLast (List.cons_ne_nil _ _) + w → l.length * cfgPeriod cfg t.val < w := by
  have hs := spaced_span l a (irun_spaced h)
  exact ⟨hs, fun w hw => by omega⟩

end P18
