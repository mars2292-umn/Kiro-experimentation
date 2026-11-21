import Sched.TotalFrame
import Sched.SpIntake
import Sched.Time
import Sched.Arrive
open Aeneas Aeneas.Std Result

/-! # The timer and intake entries return (2026-11-20)

Property 21, the time arithmetic. The additions of instants in
`Kernel::intake`, `Kernel::timer_event` and what they call do not overflow
because the instants they read stay within bounds: the observed instant
and the configuration's durations below 2^62 (`CT`, Verus: `Config::wf`,
`INSTANT_BOUND`), the nominal and last-due instants below 2^62 and the
deferral instants below 2^63 (`TmI`, Verus: `Timing::inv`). `KT` is `KI`
with those bounds, and every function here is shown to keep it. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The instants the time arithmetic reads are within their bounds. -/
structure TmI (tm : logic.time.Timing NT) : Prop where
  nom : ∀ t, t < NT.val → (tm.nominal.val[t]!).val ≤ 4611686018427387904
  ld : ∀ t, t < NT.val → (tm.last_due.val[t]!).val ≤ 4611686018427387904
  du : ∀ t, t < NT.val → (tm.deferred_until.val[t]!).val ≤ 9223372036854775808

theorem TmI.of_tft {tm tm' : logic.time.Timing NT} (h : TmI tm) (f : TFt tm' tm) : TmI tm' := by
  obtain ⟨a, b, c⟩ := f
  exact ⟨by rw [a]; exact h.nom, by rw [b]; exact h.ld, by rw [c]; exact h.du⟩

/-- `KI` and the bounds of the instants. -/
structure KT (k : logic.kernel.Kernel NT NR NP NL NS NE) : Prop where
  ki : KI k
  tm : TmI k.timing

/-- `CI` and the bounds of the configuration's durations. -/
structure CT (cfg : logic.Config NT NR NP) (NS : Usize) : Prop where
  ci : CI cfg NS
  od : cfg.operating_duration.val < 4611686018427387904
  per : ∀ (i : Nat) (h : i < cfg.tasks.val.length),
    (cfg.tasks.val[i]).period.val ≤ 4611686018427387904
  dl : ∀ (i : Nat) (h : i < cfg.tasks.val.length),
    (cfg.tasks.val[i]).deadline.val ≤ 4611686018427387904

theorem per_le {cfg : logic.Config NT NR NP} (hc : CT cfg NS) {i : Nat} {tc : logic.TaskCfg}
    {h : i < cfg.tasks.val.length} (htc : tc = cfg.tasks.val[i]) :
    tc.period.val ≤ 4611686018427387904 := by
  rw [htc]; exact hc.per i h

theorem dl_le {cfg : logic.Config NT NR NP} (hc : CT cfg NS) {i : Nat} {tc : logic.TaskCfg}
    {h : i < cfg.tasks.val.length} (htc : tc = cfg.tasks.val[i]) :
    tc.deadline.val ≤ 4611686018427387904 := by
  rw [htc]; exact hc.dl i h

/-- A function that keeps `KI` and leaves the three arrays keeps `KT`. -/
theorem kt_spec {x : Result (logic.kernel.Kernel NT NR NP NL NS NE)}
    {k : logic.kernel.Kernel NT NR NP NL NS NE} (hk : KT k) (h : x ⦃ k' => KI k' ⦄)
    (hf : ∀ r, x = ok r → TF r k) : x ⦃ k' => KT k' ⦄ := by
  obtain ⟨r, hr, hp⟩ := spec_returns h
  rw [hr]
  exact (WP.spec_ok _).mpr ⟨hp, hk.tm.of_tft (hf r hr)⟩

/-! ## The events and their responses, with the bounds -/

variable (k : logic.kernel.Kernel NT NR NP NL NS NE) (cfg : logic.Config NT NR NP)

theorem observe_kt (now : U64) (hk : KT k) (hn : now.val ≤ 4611686018427387904) :
    logic.kernel.Kernel.observe k cfg now ⦃ k' =>
      KT k' ∧ k'.sched = k.sched ∧ k'.now.val ≤ 4611686018427387904 ⦄ := by
  unfold logic.kernel.Kernel.observe
  split
  · exact (WP.spec_ok _).mpr ⟨⟨⟨hk.ki.s, hk.ki.l, hn⟩, hk.tm⟩, rfl, hn⟩
  · exact (WP.spec_ok _).mpr ⟨hk, rfl, hk.ki.n⟩

theorem push_kt (kind task partition : U8) (prs : U64) (detail : U32) (hk : KT k) :
    logic.kernel.Kernel.push_event k cfg kind task partition prs detail ⦃ k' =>
      KT k' ∧ k'.sched = k.sched ∧ k'.now = k.now ∧ k'.slots = k.slots ∧
      k'.timing = k.timing ∧ k'.budget = k.budget ⦄ :=
  WP.spec_mono (push_ki k cfg kind task partition prs detail hk.ki)
    (fun k1 h => ⟨⟨h.1, by rw [h.2.2.2.2.1]; exact hk.tm⟩, h.2⟩)

theorem enter_safe_state_kt (hk : KT k) (hNT : NT.val ≤ 256) :
    logic.kernel.Kernel.enter_safe_state k cfg ⦃ k' => KT k' ⦄ :=
  kt_spec hk (enter_safe_state_total k cfg hk.ki hNT) (fun _ h => enter_safe_state_tf h)

theorem mit_violation_kt (t partition offender : U8) (hk : KT k) (hc : CT cfg NS)
    (hp : partition.val < NP.val) (ho : offender.val < NT.val) :
    logic.kernel.Kernel.mit_violation k cfg t partition offender ⦃ k' => KT k' ⦄ :=
  kt_spec hk (mit_violation_total k cfg t partition offender hk.ki hc.ci hp ho)
    (fun _ h => mit_violation_tf h)

theorem deadline_miss_kt (t : U8) (i : Usize) (hk : KT k) (hc : CT cfg NS)
    (ht : t.val < NT.val) (hi : i.val < NS.val) :
    logic.kernel.Kernel.deadline_miss k cfg t i ⦃ k' => KT k' ⦄ :=
  kt_spec hk (deadline_miss_total k cfg t i hk.ki hc.ci ht hi) (fun _ h => deadline_miss_tf h)

theorem budget_event_kt (t : U8) (i : Usize) (hk : KT k) (hc : CT cfg NS)
    (ht : t.val < NT.val) (hi : i.val < NS.val) :
    logic.kernel.Kernel.budget_event k cfg t i ⦃ k' => KT k' ⦄ :=
  kt_spec hk (budget_event_total k cfg t i hk.ki hc.ci ht hi) (fun _ h => budget_event_tf h)

/-! ## The `Timing` functions -/

variable (tm : logic.time.Timing NT)

theorem mask_tft {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    {now : U64} (h : logic.time.Timing.mask tm cfg t now = ok tm') : TFt tm' tm := by
  unfold logic.time.Timing.mask at h
  peel h
  exact ⟨rfl, rfl, rfl⟩

theorem mask_spec (t : U8) (now : U64) (ht : t.val < NT.val)
    (hn : now.val ≤ 4611686018427387904) (hc : CT cfg NS) (hi : TmI tm) :
    logic.time.Timing.mask tm cfg t now ⦃ tm' => TmI tm' ⦄ := by
  refine spec_of_total ?_ (fun r h => hi.of_tft (mask_tft h))
  unfold logic.time.Timing.mask
  simp only [lift, bind_ok]
  step as ⟨ a, ha ⟩
  step as ⟨ tc, htc ⟩
  have hper := per_le hc htc
  step*

theorem accept_spec (t : U8) (now : U64) (ht : t.val < NT.val)
    (hn : now.val ≤ 4611686018427387904) (hc : CT cfg NS) (hi : TmI tm) :
    logic.time.Timing.accept tm cfg t now ⦃ tm' => TmI tm' ⦄ := by
  refine spec_of_total ?_ (fun r h => ?_)
  · unfold logic.time.Timing.accept
    simp only [lift, bind_ok]
    step as ⟨ a, ha ⟩
    step as ⟨ a1, ha1 ⟩
    step as ⟨ a2, ha2 ⟩
    step as ⟨ a3, ha3 ⟩
    step as ⟨ tc, htc ⟩
    have hper := per_le hc htc
    step*
  · obtain ⟨_, _, e2, e3⟩ := accept_elem h
    refine ⟨by rw [accept_nom h]; exact hi.nom, fun u hu => ?_, by rw [e3]; exact hi.du⟩
    rw [e2, get_set _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact ht)]
    split
    · exact hn
    · exact hi.ld u hu

theorem defer_spec (t : U8) (now : U64) (ht : t.val < NT.val)
    (hn : now.val ≤ 4611686018427387904) (hc : CT cfg NS) (hi : TmI tm) :
    logic.time.Timing.defer tm cfg t now ⦃ r => TmI r.2 ⦄ := by
  have hld := hi.ld t.val ht
  have hlen : t.val < tm.last_due.val.length := by have := tm.last_due.property; omega
  have hlent : t.val < cfg.tasks.val.length := by have := cfg.tasks.property; omega
  refine spec_of_total ?_ (fun r h => ?_)
  · unfold logic.time.Timing.defer
    simp only [lift, bind_ok]
    step as ⟨ i1, hi1 ⟩
    simp only [U8.cast_Usize_val_eq] at hi1
    have hi1' := getE hlen hi1
    rw [hi1'] at hld
    step as ⟨ tc, htc ⟩
    have hper := per_le hc htc
    step as ⟨ at0, hat ⟩
    step as ⟨ a, ha ⟩
    step as ⟨ a1, ha1 ⟩
    step as ⟨ tc1, htc1 ⟩
    have hper1 := per_le hc htc1
    step*
  · obtain ⟨a, tm'⟩ := r
    obtain ⟨_, _, e2, e3, tc, htc, hav⟩ := defer_elem h
    have htc' : tc = cfg.tasks.val[t.val] := by
      rw [List.getElem?_eq_getElem hlent] at htc
      exact (Option.some.inj htc).symm
    have hper := per_le hc htc'
    refine ⟨by rw [defer_nom h]; exact hi.nom, by rw [e2]; exact hi.ld, fun u hu => ?_⟩
    dsimp only
    rw [e3, get_set _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact ht)]
    split
    · omega
    · exact hi.du u hu

theorem intake_decision_total (t : U8) (rn : Bool) (now : U64) (ht : t.val < NT.val)
    (hc : CT cfg NS) (hi : TmI tm) :
    logic.time.Timing.intake_decision tm cfg t rn now ⦃ _ => True ⦄ := by
  have hld := hi.ld t.val ht
  have hlen : t.val < tm.last_due.val.length := by have := tm.last_due.property; omega
  have hlenh : t.val < tm.has_last.val.length := by have := tm.has_last.property; omega
  unfold logic.time.Timing.intake_decision
  split
  · simp only [lift, bind_ok]
    step as ⟨ tc, htc ⟩
    have hper := per_le hc htc
    step as ⟨ b, hb ⟩
    split
    · step as ⟨ i3, hi3 ⟩
      simp only [U8.cast_Usize_val_eq] at hi3
      have hi3' := getE hlen hi3
      rw [hi3'] at hld
      step as ⟨ i4, hi4 ⟩
      split
      · exact (WP.spec_ok _).mpr trivial
      · step as ⟨ tc1, htc1 ⟩
        split
        · exact (WP.spec_ok _).mpr trivial
        · exact (WP.spec_ok _).mpr trivial
    · exact (WP.spec_ok _).mpr trivial
  · exact (WP.spec_ok _).mpr trivial

/-- The MIT decision answers "defer" only for a Task whose policy defers. -/
theorem intake_decision_defer_policy {tm : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t d : U8} {rn : Bool} {now : U64}
    (h : logic.time.Timing.intake_decision tm cfg t rn now = ok d)
    (hd : d = logic.time.INTAKE_DEFER) :
    ∃ tc, cfg.tasks.val[t.val]? = some tc ∧ tc.mit_policy = logic.MIT_DEFER := by
  subst hd
  unfold logic.time.Timing.intake_decision at h
  split at h
  · simp only [lift, bind_ok, bind_eq_ok] at h
    obtain ⟨tc, htc, b, hb, h⟩ := h
    split at h
    · simp only [bind_eq_ok] at h
      obtain ⟨i3, hi3, i4, hi4, h⟩ := h
      split at h
      · have := ok_inj h
        simp [intake_accept, intake_defer] at this
      · simp only [bind_eq_ok] at h
        obtain ⟨tc1, htc1, h⟩ := h
        split at h
        · rename_i hpol
          obtain ⟨_, e⟩ := index_ok' htc1
          rw [U8.cast_Usize_val_eq] at e
          exact ⟨tc1, e, hpol⟩
        · have := ok_inj h
          simp [intake_discard, intake_defer] at this
    · have := ok_inj h
      simp [intake_accept, intake_defer] at this
  · have := ok_inj h
    simp [intake_discard, intake_defer] at this

theorem intake_decision_spec (t : U8) (rn : Bool) (now : U64) (ht : t.val < NT.val)
    (hc : CT cfg NS) (hi : TmI tm) :
    logic.time.Timing.intake_decision tm cfg t rn now ⦃ d =>
      d = logic.time.INTAKE_DEFER →
        ∃ tc, cfg.tasks.val[t.val]? = some tc ∧ tc.mit_policy = logic.MIT_DEFER ⦄ :=
  spec_of_total (intake_decision_total cfg tm t rn now ht hc hi)
    (fun _ h hd => intake_decision_defer_policy h hd)

theorem ddc_nom {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    {a : U64} (h : logic.time.Timing.deferred_due_clamped tm cfg t a = ok tm') :
    tm'.nominal = tm.nominal := by
  unfold logic.time.Timing.deferred_due_clamped at h
  peel h
  rfl

theorem ddc_spec (t : U8) (a : U64) (ht : t.val < NT.val)
    (ha : a.val ≤ 4611686018427387904) (hi : TmI tm) :
    logic.time.Timing.deferred_due_clamped tm cfg t a ⦃ tm' => TmI tm' ⦄ := by
  refine spec_of_total ?_ (fun r h => ?_)
  · unfold logic.time.Timing.deferred_due_clamped
    simp only [lift, bind_ok]
    step*
  · obtain ⟨_, _, e2, e3⟩ := deferred_due_clamped_elem h
    refine ⟨by rw [ddc_nom h]; exact hi.nom, fun u hu => ?_, by rw [e3]; exact hi.du⟩
    rw [e2, get_set _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact ht)]
    split
    · exact ha
    · exact hi.ld u hu

theorem periodic_due_tft {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    {a : U64} (h : logic.time.Timing.periodic_due tm cfg t a = ok tm') : TFt tm' tm := by
  unfold logic.time.Timing.periodic_due at h
  peel h
  exact ⟨rfl, rfl, rfl⟩

theorem periodic_due_spec (t : U8) (a : U64) (ht : t.val < NT.val) (hi : TmI tm) :
    logic.time.Timing.periodic_due tm cfg t a ⦃ tm' => TmI tm' ⦄ := by
  refine spec_of_total ?_ (fun r h => hi.of_tft (periodic_due_tft h))
  unfold logic.time.Timing.periodic_due
  simp only [lift, bind_ok]
  step*

theorem unmask_tft {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.time.Timing.unmask tm cfg t = ok tm') : TFt tm' tm := by
  unfold logic.time.Timing.unmask at h
  peel h
  exact ⟨rfl, rfl, rfl⟩

theorem unmask_spec (t : U8) (ht : t.val < NT.val) (hi : TmI tm) :
    logic.time.Timing.unmask tm cfg t ⦃ tm' => TmI tm' ⦄ := by
  refine spec_of_total ?_ (fun r h => hi.of_tft (unmask_tft h))
  unfold logic.time.Timing.unmask
  simp only [lift, bind_ok]
  step*

theorem advance_periodic_tmi {tm tm' : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t : U8} {o : Option U64} (hc : CT cfg NS) (hi : TmI tm)
    (h : logic.time.Timing.advance_periodic tm cfg t = ok (o, tm')) : TmI tm' := by
  unfold logic.time.Timing.advance_periodic at h
  peel h
  all_goals first
    | exact hi
    | (obtain ⟨hb, rfl⟩ := update_ok ‹Array.update tm.nominal _ _ = ok _›
       refine ⟨fun u hu => ?_, hi.ld, hi.du⟩
       dsimp only
       rw [get_set _ _ _ _ hb]
       split
       · have := hc.od
         scalar_tac
       · exact hi.nom u hu)

theorem advance_periodic_spec (t : U8) (ht : t.val < NT.val) (hc : CT cfg NS) (hi : TmI tm) :
    logic.time.Timing.advance_periodic tm cfg t ⦃ r => TmI r.2 ⦄ := by
  refine spec_of_total ?_ (fun r h => advance_periodic_tmi (o := r.1) (tm' := r.2) hc hi h)
  have hlenk : t.val < tm.k.val.length := by have := tm.k.property; omega
  unfold logic.time.Timing.advance_periodic
  simp only [lift, bind_ok]
  step as ⟨ tc, htc ⟩
  step as ⟨ n, hn ⟩
  split
  · exact (WP.spec_ok _).mpr trivial
  · split
    · step as ⟨ i3, hi3 ⟩
      split
      · exact (WP.spec_ok _).mpr trivial
      · rename_i hne
        step as ⟨ i5, hi5 ⟩
        have he : i5 = i3 := by rw [hi5, hi3]
        have hmv : (core.num.U64.MAX).val = 18446744073709551615 := by
          simp [core.num.U64.MAX, U64.rMax]
        have hmax : i5.val < 18446744073709551615 := by
          rw [he]
          by_contra hge
          apply hne
          apply UScalar.eq_of_val_eq
          rw [hmv]
          scalar_tac
        step*
    · exact (WP.spec_ok _).mpr trivial

theorem make_pending_spec (s : logic.sched.Sched NT NR NL) (t : U8) (ht : t.val < NT.val)
    (hs : SInv s) : logic.sched.Sched.make_pending s cfg t ⦃ s' => SInv s' ⦄ :=
  spec_of_total (make_pending_total s cfg t ht) (fun r h => by
    obtain ⟨_, rfl⟩ := make_pending_ok h
    exact hs)

theorem defer_sched_spec (s : logic.sched.Sched NT NR NL) (t : U8) (ht : t.val < NT.val)
    (hs : SInv s) : logic.sched.Sched.defer s cfg t ⦃ s' => SInv s' ⦄ :=
  spec_of_total (defer_total s cfg t ht) (fun r h => by
    obtain ⟨_, rfl⟩ := defer_ok h
    exact hs)

/-! ## The intake of a sporadic event -/

set_option maxHeartbeats 4000000 in
/-- **`intake_release` returns** (R10.1 to R10.4): the MIT decision, the
Release or its deferral, the MIT_Violation and its response. -/
theorem intake_release_kt (t sp off : U8) (now : U64) (hk : KT k) (hc : CT cfg NS)
    (ht : t.val < NT.val) (hsp : sp.val < NP.val) (ho : off.val < NT.val)
    (hn : now.val ≤ 4611686018427387904) :
    logic.kernel.Kernel.intake_release k cfg t sp now off ⦃ k' => KT k' ⦄ := by
  have hld : t.val < cfg.slot_deadline.val.length := by
    have := cfg.slot_deadline.property; omega
  have hlr : t.val < cfg.slot_release.val.length := by
    have := cfg.slot_release.property; omega
  unfold logic.kernel.Kernel.intake_release
  refine WP.spec_bind (observe_kt k cfg now hk hn) (fun self1 h1 => ?_)
  obtain ⟨h1, _, _⟩ := h1
  simp only [lift, bind_ok]
  step as ⟨ tc, htc ⟩
  have hpart := part_lt hc.ci htc
  split
  · exact (WP.spec_ok _).mpr h1
  · split
    · have hlens : tc.partition.val < self1.stopped.val.length := by
        have := self1.stopped.property; scalar_tac
      step as ⟨ b, hb ⟩
      split
      · exact (WP.spec_ok _).mpr h1
      · step as ⟨ tc1, htc1 ⟩
        have hdl1 := dl_le hc htc1
        have hnow := h1.ki.n
        step as ⟨ i2, hi2 ⟩
        split
        · refine WP.spec_bind (mask_spec cfg self1.timing t self1.now ht hnow hc h1.tm)
            (fun t1 ht1 => ?_)
          exact (WP.spec_ok _).mpr ⟨⟨h1.ki.s, h1.ki.l, h1.ki.n⟩, ht1⟩
        · have hlenr : t.val < self1.sched.release.val.length := by
            have := self1.sched.release.property; scalar_tac
          step as ⟨ i4, hi4 ⟩
          refine WP.spec_bind (intake_decision_spec cfg self1.timing t _ self1.now ht hc h1.tm)
            (fun d hdp => ?_)
          split
          · refine WP.spec_bind (accept_spec cfg self1.timing t self1.now ht hnow hc h1.tm)
              (fun t1 ht1 => ?_)
            refine WP.spec_bind (make_pending_spec cfg self1.sched t ht h1.ki.s) (fun s hs => ?_)
            step as ⟨ tc2, htc2 ⟩
            have hdl2 := dl_le hc htc2
            step as ⟨ deadline, hdeadline ⟩
            step as ⟨ i7, hi7 ⟩
            simp only [U8.cast_Usize_val_eq] at hi7
            refine WP.spec_bind (arm_spec self1.slots _ _ _ _ (dl_slot hc.ci.ok ht hi7))
              (fun s1 _ => ?_)
            exact (WP.spec_ok _).mpr ⟨⟨hs, h1.ki.l, h1.ki.n⟩, ht1⟩
          · refine WP.spec_bind
              (Pₘ := fun x : logic.sched.Sched NT NR NL × logic.time.Timing NT ×
                logic.time.Slots NS => SInv x.1 ∧ TmI x.2.1) ?_ (fun x hx => ?_)
            · split
              · rename_i hdef
                obtain ⟨tcd, htcd, hpol⟩ := hdp hdef
                split
                · refine WP.spec_bind (defer_spec cfg self1.timing t self1.now ht hnow hc h1.tm)
                    (fun p hp => ?_)
                  obtain ⟨at0, t2⟩ := p
                  simp only [Aeneas.Std.uncurry]
                  refine WP.spec_bind (Pₘ := fun _ : U64 => True) (by ok_tac) (fun at1 _ => ?_)
                  refine WP.spec_bind (defer_sched_spec cfg self1.sched t ht h1.ki.s)
                    (fun s2 hs2 => ?_)
                  step as ⟨ i6, hi6 ⟩
                  simp only [U8.cast_Usize_val_eq] at hi6
                  refine WP.spec_bind (arm_spec self1.slots _ _ _ _ (rl_slot hc.ci.ok ht htcd (Or.inr hpol) hi6))
                    (fun s3 _ => ?_)
                  exact (WP.spec_ok _).mpr ⟨hs2, hp⟩
                · refine WP.spec_bind (mask_spec cfg self1.timing t self1.now ht hnow hc h1.tm)
                    (fun t2 ht2 => ?_)
                  exact (WP.spec_ok _).mpr ⟨h1.ki.s, ht2⟩
              · refine WP.spec_bind (mask_spec cfg self1.timing t self1.now ht hnow hc h1.tm)
                  (fun t2 ht2 => ?_)
                exact (WP.spec_ok _).mpr ⟨h1.ki.s, ht2⟩
            · obtain ⟨s, t1, s1⟩ := x
              simp only [Aeneas.Std.uncurry]
              exact mit_violation_kt _ cfg t sp off ⟨⟨hx.1, h1.ki.l, h1.ki.n⟩, hx.2⟩ hc hsp ho
    · exact (WP.spec_ok _).mpr h1

/-- **`intake` returns**: the deadline of the Job in progress is checked
first (KD-R9.6), then the event is taken in. -/
theorem intake_kt (t sp off : U8) (now : U64) (hk : KT k) (hc : CT cfg NS)
    (ht : t.val < NT.val) (hsp : sp.val < NP.val) (ho : off.val < NT.val)
    (hn : now.val ≤ 4611686018427387904) :
    logic.kernel.Kernel.intake k cfg t sp now off ⦃ k' => KT k' ⦄ := by
  have hld : t.val < cfg.slot_deadline.val.length := by
    have := cfg.slot_deadline.property; omega
  unfold logic.kernel.Kernel.intake
  refine WP.spec_bind (observe_kt k cfg now hk hn) (fun self1 h1 => ?_)
  obtain ⟨h1, _, _⟩ := h1
  simp only [lift, bind_ok]
  step as ⟨ i1, hi1 ⟩
  simp only [U8.cast_Usize_val_eq] at hi1
  have hsl := dl_slot hc.ci.ok ht hi1
  have hlens : (UScalar.cast .Usize i1).val < self1.slots.slots.val.length := by
    have := self1.slots.slots.property; omega
  step as ⟨ s, hs ⟩
  refine WP.spec_bind
    (Pₘ := fun self2 : logic.kernel.Kernel NT NR NP NL NS NE => KT self2) ?_ (fun self2 h2 => ?_)
  · split
    · split
      · split
        · exact deadline_miss_kt self1 cfg t _ h1 hc ht hsl
        · exact (WP.spec_ok _).mpr h1
      · exact (WP.spec_ok _).mpr h1
    · exact (WP.spec_ok _).mpr h1
  · exact intake_release_kt self2 cfg t sp off now h2 hc ht hsp ho hn

/-! ## The timer entry -/

set_option maxHeartbeats 2000000 in
/-- **`deferred_release` returns** (R10.3). -/
theorem deferred_release_kt (t : U8) (i : Usize) (hk : KT k) (hc : CT cfg NS)
    (ht : t.val < NT.val) (hi : i.val < NS.val) :
    logic.kernel.Kernel.deferred_release k cfg t i ⦃ k' => KT k' ⦄ := by
  have hld : t.val < cfg.slot_deadline.val.length := by
    have := cfg.slot_deadline.property; omega
  have hlu : t.val < k.timing.deferred_until.val.length := by
    have := k.timing.deferred_until.property; omega
  have hlenr : t.val < k.sched.release.val.length := by
    have := k.sched.release.property; omega
  have hdu := hk.tm.du t.val ht
  unfold logic.kernel.Kernel.deferred_release
  refine WP.spec_bind (disarm_spec k.slots i hi) (fun s _ => ?_)
  simp only [lift, bind_ok]
  step as ⟨ tc, htc ⟩
  have hpart := part_lt hc.ci htc
  have hlens : tc.partition.val < k.stopped.val.length := by
    have := k.stopped.property; scalar_tac
  step as ⟨ b, hb ⟩
  split
  · exact (WP.spec_ok _).mpr ⟨⟨hk.ki.s, hk.ki.l, hk.ki.n⟩, hk.tm⟩
  · step as ⟨ i4, hi4 ⟩
    split
    · exact (WP.spec_ok _).mpr ⟨⟨hk.ki.s, hk.ki.l, hk.ki.n⟩, hk.tm⟩
    · step as ⟨ at0, hat ⟩
      simp only [U8.cast_Usize_val_eq] at hat
      rw [getE hlu hat] at hdu
      step as ⟨ tc1, htc1 ⟩
      have hdl1 := dl_le hc htc1
      step as ⟨ i7, hi7 ⟩
      split
      · refine WP.spec_bind (clear_release_spec k.sched cfg t ht hk.ki.s) (fun s1 hs1 => ?_)
        exact (WP.spec_ok _).mpr ⟨⟨hs1.1, hk.ki.l, hk.ki.n⟩, hk.tm⟩
      · rename_i hle
        have hat' : at0.val ≤ 4611686018427387904 := by
          have := hc.od
          scalar_tac
        refine WP.spec_bind (ddc_spec cfg k.timing t at0 ht hat' hk.tm) (fun t1 ht1 => ?_)
        refine WP.spec_bind (make_pending_spec cfg k.sched t ht hk.ki.s) (fun s1 hs1 => ?_)
        step as ⟨ tc2, htc2 ⟩
        have hdl2 := dl_le hc htc2
        step as ⟨ deadline, hdeadline ⟩
        step as ⟨ i10, hi10 ⟩
        simp only [U8.cast_Usize_val_eq] at hi10
        refine WP.spec_bind (arm_spec s _ _ _ _ (dl_slot hc.ci.ok ht hi10)) (fun s2 _ => ?_)
        exact (WP.spec_ok _).mpr ⟨⟨hs1, hk.ki.l, hk.ki.n⟩, ht1⟩

set_option maxHeartbeats 4000000 in
/-- **`periodic_release` returns** (R9.1 to R9.4): the Release at its
nominal instant and the next one armed, without drift. -/
theorem periodic_release_kt (t : U8) (i : Usize) (hk : KT k) (hc : CT cfg NS)
    (ht : t.val < NT.val) (hi : i.val < NS.val) :
    logic.kernel.Kernel.periodic_release k cfg t i ⦃ k' => KT k' ⦄ := by
  have hld : t.val < cfg.slot_deadline.val.length := by
    have := cfg.slot_deadline.property; omega
  have hln : t.val < k.timing.nominal.val.length := by
    have := k.timing.nominal.property; omega
  have hlenr : t.val < k.sched.release.val.length := by
    have := k.sched.release.property; omega
  have hlenj : t.val < k.sched.job.val.length := by
    have := k.sched.job.property; omega
  have hnom := hk.tm.nom t.val ht
  unfold logic.kernel.Kernel.periodic_release
  simp only [lift, bind_ok]
  step as ⟨ tc, htc ⟩
  have hpart := part_lt hc.ci htc
  step as ⟨ nominal, hnm ⟩
  simp only [U8.cast_Usize_val_eq] at hnm
  rw [getE hln hnm] at hnom
  step as ⟨ tc1, htc1 ⟩
  split
  · refine WP.spec_bind (disarm_spec k.slots i hi) (fun s _ => ?_)
    exact (WP.spec_ok _).mpr ⟨⟨hk.ki.s, hk.ki.l, hk.ki.n⟩, hk.tm⟩
  · step as ⟨ tc2, htc2 ⟩
    have hdl2 := dl_le hc htc2
    step as ⟨ i5, hi5 ⟩
    have hlens : tc.partition.val < k.stopped.val.length := by
      have := k.stopped.property; scalar_tac
    step as ⟨ b, hb ⟩
    refine WP.spec_bind
      (Pₘ := fun self1 : logic.kernel.Kernel NT NR NP NL NS NE => KT self1) ?_
      (fun self1 h1 => ?_)
    · split
      · exact (WP.spec_ok _).mpr hk
      · split
        · step as ⟨ i8, hi8 ⟩
          split
          · step as ⟨ i10, hi10 ⟩
            refine WP.spec_bind
              (Pₘ := fun self2 : logic.kernel.Kernel NT NR NP NL NS NE => KT self2) ?_
              (fun self2 h2 => ?_)
            · split
              · refine WP.spec_bind prs_release_total (fun i11 _ => ?_)
                exact WP.spec_mono (push_kt k cfg _ _ _ _ _ hk) (fun _ h => h.1)
              · exact (WP.spec_ok _).mpr hk
            · refine WP.spec_bind (make_pending_spec cfg self2.sched t ht h2.ki.s) (fun s hs => ?_)
              refine WP.spec_bind (periodic_due_spec cfg self2.timing t nominal ht h2.tm)
                (fun t1 ht1 => ?_)
              step as ⟨ tc3, htc3 ⟩
              have hdl3 := dl_le hc htc3
              step as ⟨ deadline, hdeadline ⟩
              step as ⟨ i13, hi13 ⟩
              simp only [U8.cast_Usize_val_eq] at hi13
              refine WP.spec_bind (arm_spec self2.slots _ _ _ _ (dl_slot hc.ci.ok ht hi13))
                (fun s1 _ => ?_)
              exact (WP.spec_ok _).mpr ⟨⟨hs, h2.ki.l, h2.ki.n⟩, ht1⟩
          · exact mit_violation_kt k cfg t tc.partition t hk hc hpart ht
        · exact (WP.spec_ok _).mpr hk
    · refine WP.spec_bind (advance_periodic_spec cfg self1.timing t ht hc h1.tm) (fun p hp => ?_)
      obtain ⟨o, t1⟩ := p
      simp only [Aeneas.Std.uncurry]
      split
      · refine WP.spec_bind (disarm_spec self1.slots i hi) (fun s _ => ?_)
        exact (WP.spec_ok _).mpr ⟨⟨h1.ki.s, h1.ki.l, h1.ki.n⟩, hp⟩
      · refine WP.spec_bind (arm_spec self1.slots i _ _ _ hi) (fun s _ => ?_)
        exact (WP.spec_ok _).mpr ⟨⟨h1.ki.s, h1.ki.l, h1.ki.n⟩, hp⟩

theorem hk_period_total : logic.kernel.HOUSEKEEPING_PERIOD ⦃ r => r.val ≤ 2000000 ⦄ := by
  unfold logic.kernel.HOUSEKEEPING_PERIOD
  step as ⟨ r, hr ⟩
  scalar_tac

set_option maxHeartbeats 4000000 in
/-- **`serve_slot` returns**: every kind of slot. -/
theorem serve_slot_kt (i : Usize) (hk : KT k) (hc : CT cfg NS) (hi : i.val < NS.val) :
    logic.kernel.Kernel.serve_slot k cfg i ⦃ k' => KT k' ⦄ := by
  have hlen : i.val < k.slots.slots.val.length := by
    have := k.slots.slots.property; omega
  have hnow := hk.ki.n
  have hkk : ∀ (s : logic.time.Slots NS),
      KT ({ k with slots := s } : logic.kernel.Kernel NT NR NP NL NS NE) :=
    fun s => ⟨⟨hk.ki.s, hk.ki.l, hk.ki.n⟩, hk.tm⟩
  unfold logic.kernel.Kernel.serve_slot
  step as ⟨ slot, hslot ⟩
  split
  · refine WP.spec_bind (disarm_spec k.slots i hi) (fun s _ => ?_)
    refine WP.spec_bind (push_kt _ cfg _ _ _ _ _ (hkk s)) (fun self1 h1 => ?_)
    exact enter_safe_state_kt self1 cfg h1.1 hc.ci.nt
  · split
    · refine WP.spec_bind hk_period_total (fun i1 hi1 => ?_)
      step as ⟨ i2, hi2 ⟩
      refine WP.spec_bind (Pₘ := fun _ : U64 => True) ?_ (fun next _ => ?_)
      · split
        · exact (WP.spec_ok _).mpr trivial
        · step*
      · refine WP.spec_bind (arm_spec k.slots i _ _ _ hi) (fun s _ => ?_)
        exact (WP.spec_ok _).mpr (hkk s)
    · simp only [lift, bind_ok]
      split
      · rename_i hlt
        have htk : slot.task.val < NT.val := by scalar_tac
        split
        · exact periodic_release_kt k cfg slot.task i hk hc htk hi
        split
        · exact deferred_release_kt k cfg slot.task i hk hc htk hi
        split
        · exact deadline_miss_kt k cfg slot.task i hk hc htk hi
        split
        · exact budget_event_kt k cfg slot.task i hk hc htk hi
        split
        · refine WP.spec_bind (disarm_spec k.slots i hi) (fun s _ => ?_)
          have hlm : slot.task.val < k.timing.mask_until.val.length := by
            have := k.timing.mask_until.property; omega
          step as ⟨ i3, hi3 ⟩
          split
          · refine WP.spec_bind (unmask_spec cfg k.timing slot.task htk hk.tm) (fun t1 ht1 => ?_)
            exact (WP.spec_ok _).mpr ⟨⟨hk.ki.s, hk.ki.l, hk.ki.n⟩, ht1⟩
          · exact (WP.spec_ok _).mpr (hkk s)
        · refine WP.spec_bind (disarm_spec k.slots i hi) (fun s _ => ?_)
          exact (WP.spec_ok _).mpr (hkk s)
      · refine WP.spec_bind (disarm_spec k.slots i hi) (fun s _ => ?_)
        exact (WP.spec_ok _).mpr (hkk s)

theorem timer_loop0_kt (hc : CT cfg NS) (i : Usize) (hk : KT k) (hi : i.val ≤ NT.val) :
    logic.kernel.Kernel.timer_event_loop0 k cfg i ⦃ r => KT r.1 ∧ r.2 = cfg ⦄ := by
  unfold logic.kernel.Kernel.timer_event_loop0
  apply loop.spec_decr_nat
    (fun (x : logic.kernel.Kernel NT NR NP NL NS NE × Usize) => NT.val - x.2.val)
    (fun x => KT x.1 ∧ x.2.val ≤ NT.val)
  · intro x hx
    obtain ⟨k1, t1⟩ := x
    obtain ⟨hk1, h2⟩ := hx
    try dsimp only at hk1
    try dsimp only at h2
    try dsimp only
    unfold logic.kernel.Kernel.timer_event_loop0.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : t1.val < NT.val := by scalar_tac
      have hld : t1.val < cfg.slot_deadline.val.length := by
        have := cfg.slot_deadline.property; omega
      step as ⟨ i0, hi0 ⟩
      have hi0' : cfg.slot_deadline.val[t1.val]! = i0 := by
        rw [hi0]; exact getElem!_pos cfg.slot_deadline.val t1.val hld
      have hsl := (hc.ci.ok t1.val hlt').2.1
      rw [hi0'] at hsl
      simp only [lift, bind_ok]
      have hcv : (UScalar.cast .Usize i0).val = i0.val := U8.cast_Usize_val_eq _
      have hlen : (UScalar.cast .Usize i0).val < k1.slots.slots.val.length := by
        have := k1.slots.slots.property; omega
      step as ⟨ slot, hslot ⟩
      refine WP.spec_bind
        (Pₘ := fun self1 : logic.kernel.Kernel NT NR NP NL NS NE => KT self1) ?_
        (fun self1 h1 => ?_)
      · split
        · split
          · split
            · exact serve_slot_kt k1 cfg _ hk1 hc (by omega)
            · exact (WP.spec_ok _).mpr hk1
          · exact (WP.spec_ok _).mpr hk1
        · exact (WP.spec_ok _).mpr hk1
      · step as ⟨ i2, hi2 ⟩
        exact ⟨h1, by scalar_tac, by scalar_tac⟩
    · exact (WP.spec_ok _).mpr ⟨hk1, rfl⟩
  · exact ⟨hk, hi⟩

theorem timer_loop1_kt (hc : CT cfg NS) (i : Usize) (hk : KT k) (hi : i.val ≤ NS.val) :
    logic.kernel.Kernel.timer_event_loop1 k cfg i ⦃ k' => KT k' ⦄ := by
  unfold logic.kernel.Kernel.timer_event_loop1
  apply loop.spec_decr_nat
    (fun (x : logic.kernel.Kernel NT NR NP NL NS NE × Usize) => NS.val - x.2.val)
    (fun x => KT x.1 ∧ x.2.val ≤ NS.val)
  · intro x hx
    obtain ⟨k1, i1⟩ := x
    obtain ⟨hk1, h2⟩ := hx
    try dsimp only at hk1
    try dsimp only at h2
    try dsimp only
    unfold logic.kernel.Kernel.timer_event_loop1.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : i1.val < NS.val := by scalar_tac
      have hlen : i1.val < k1.slots.slots.val.length := by
        have := k1.slots.slots.property; omega
      step as ⟨ slot, hslot ⟩
      refine WP.spec_bind
        (Pₘ := fun self1 : logic.kernel.Kernel NT NR NP NL NS NE => KT self1) ?_
        (fun self1 h1 => ?_)
      · split
        · split
          · exact serve_slot_kt k1 cfg i1 hk1 hc hlt'
          · exact (WP.spec_ok _).mpr hk1
        · exact (WP.spec_ok _).mpr hk1
      · step as ⟨ i2, hi2 ⟩
        exact ⟨h1, by scalar_tac, by scalar_tac⟩
    · exact (WP.spec_ok _).mpr hk1
  · exact ⟨hk, hi⟩

/-- **`timer_event` returns** (R9, R18): the charge of the Job on top, the
deadline slots first, then every slot that is due. -/
theorem timer_event_kt (now : U64) (rc : U32) (hk : KT k) (hc : CT cfg NS)
    (hn : now.val ≤ 4611686018427387904) :
    logic.kernel.Kernel.timer_event k cfg now rc ⦃ k' => KT k' ⦄ := by
  unfold logic.kernel.Kernel.timer_event
  refine WP.spec_bind (observe_kt k cfg now hk hn) (fun self1 h1 => ?_)
  obtain ⟨h1, _, _⟩ := h1
  split
  · exact (WP.spec_ok _).mpr h1
  · refine WP.spec_bind (Pₘ := fun _ : logic.budget.Budget NT => True) ?_ (fun b _ => ?_)
    · split
      · step as ⟨ i, hi ⟩
        have hlen : i.val < self1.sched.stack.val.length := by
          have := self1.sched.stack.property; have := h1.ki.s.1; scalar_tac
        step as ⟨ t, ht ⟩
        have ht' := getE hlen ht
        have hst := h1.ki.s.2.1 i.val (by scalar_tac)
        simp only [stk] at hst
        rw [ht'] at hst
        exact WP.spec_mono (charge_total self1.budget t rc hst) (fun _ _ => trivial)
      · exact (WP.spec_ok _).mpr trivial
    · refine WP.spec_bind (timer_loop0_kt _ cfg hc 0#usize
        ⟨⟨h1.ki.s, h1.ki.l, h1.ki.n⟩, h1.tm⟩ (by scalar_tac)) (fun r2 h2 => ?_)
      obtain ⟨self2, cfg2⟩ := r2
      obtain ⟨h2, hc2⟩ := h2
      dsimp only at h2 hc2 ⊢
      subst hc2
      exact timer_loop1_kt self2 _ hc 0#usize h2 (by scalar_tac)

end P18
