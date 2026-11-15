import Sched.Run
open Aeneas Aeneas.Std Result

/-! # What the Kernel's own functions do to the scheduler state

The Kernel functions of `Sched/Kernel.lean` (the translation of
`rsk_logic::logic::kernel` as `rsk-kernel` links it), read through the one
field the run of `Sched/Run.lean` is about: `sched`. Every statement has
the form "if the extracted function returns, the scheduler state it leaves
is ...", with no assumption on the state it is called in. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- Peels the binds of `h : (do ..) = ok y`, splitting on every branch, and
closes a goal that holds by reflexivity once the result is substituted. -/
syntax "frame_ok " ident : tactic
macro_rules
  | `(tactic| frame_ok $h:ident) =>
    `(tactic| (
        try simp only [bind_eq_ok, lift, bind_ok, Prod.exists, Aeneas.Std.uncurry] at $h:ident
        repeat' first
          | (obtain ⟨_, $h:ident⟩ := $h:ident)
          | (simp only [bind_eq_ok, lift, bind_ok, Prod.exists, Aeneas.Std.uncurry] at $h:ident)
          | (dsimp only at $h:ident)
          | split at $h:ident
        all_goals first
          | (have hfin := ok_inj $h:ident; subst hfin; rfl)
          | skip))

theorem push_event_sched {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    k'.sched = k.sched := by
  unfold logic.kernel.Kernel.push_event at h
  frame_ok h

theorem arm_budget_sched {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') : k'.sched = k.sched := by
  unfold logic.kernel.Kernel.arm_budget at h
  frame_ok h

theorem end_mit_window_sched {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') : k'.sched = k.sched := by
  unfold logic.kernel.Kernel.end_mit_window at h
  frame_ok h

theorem observe_sched {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64}
    (h : logic.kernel.Kernel.observe k cfg now = ok k') : k'.sched = k.sched := by
  unfold logic.kernel.Kernel.observe at h
  frame_ok h

/-- Peels the binds of `h : (do ..) = ok y`, splitting on every branch, and
replaces the result by what the branch returns. The equations of the calls
stay in the context without names; an equation whose left side is itself a
branch or a sequence is peeled in turn. -/
syntax "peel " ident : tactic
macro_rules
  | `(tactic| peel $h:ident) =>
    `(tactic| (
        repeat' first
          | (have hl := And.left $h:ident; replace $h:ident := And.right $h:ident; try peel hl)
          | (obtain ⟨_, $h:ident⟩ := $h:ident)
          | (simp only [bind_eq_ok, lift, bind_ok, Prod.exists, Aeneas.Std.uncurry] at $h:ident)
          | (dsimp only at $h:ident)
          | split at $h:ident
        all_goals try (have hfin := ok_inj $h:ident; clear $h:ident; first | subst hfin | cases hfin)))

/-! ## The responses

A Health_Monitor response changes the scheduler state by ending the Job on
top, killing Jobs on the stack, and dropping Releases: a sequence of the
three steps below, each one of the extracted `Sched` functions. -/

/-- One step of a response on the scheduler state. -/
inductive RStep (cfg : logic.Config NT NR NP) :
    logic.sched.Sched NT NR NL → logic.sched.Sched NT NR NL → Prop
  | kend (s s' : logic.sched.Sched NT NR NL) (r : Usize)
      (h : logic.sched.Sched.finish_top s cfg = ok (r, s')) : RStep cfg s s'
  | kkill (s s' : logic.sched.Sched NT NR NL) (p : Usize) (hp : p.val < s.depth.val)
      (h : logic.sched.Sched.kill s cfg p = ok s') : RStep cfg s s'
  | kclear (s s' : logic.sched.Sched NT NR NL) (t : U8)
      (h : logic.sched.Sched.clear_release s cfg t = ok s') : RStep cfg s s'

/-- A response: any number of its steps. -/
abbrev RSteps (cfg : logic.Config NT NR NP) :
    logic.sched.Sched NT NR NL → logic.sched.Sched NT NR NL → Prop :=
  Relation.ReflTransGen (RStep cfg)

theorem finish_top_eq (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP) :
    logic.sched.Sched.finish_top s cfg = logic.sched.Sched.end_job s cfg := by
  unfold logic.sched.Sched.finish_top; rfl

theorem end_top_release_r {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top_release k cfg = ok k') : RStep cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.end_top_release at h
  peel h
  all_goals
    try dsimp only
    try (have hp := push_event_sched (by assumption); rw [hp])
    try dsimp only
    exact RStep.kend _ _ _ (by rw [finish_top_eq]; assumption)

theorem end_top_r {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top k cfg = ok k') : RStep cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.end_top at h
  peel h
  · rw [arm_budget_sched h]; exact end_top_release_r ‹_›
  · exact end_top_release_r ‹_›

theorem kill_at_r {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize} (hi : i.val < k.sched.depth.val)
    (h : logic.kernel.Kernel.kill_at k cfg i = ok k') :
    RStep cfg k.sched k'.sched ∧ k'.sched.depth = k.sched.depth := by
  unfold logic.kernel.Kernel.kill_at at h
  peel h
  rename_i u hu s1 hk
  dsimp only
  refine ⟨RStep.kkill _ _ i hi hk, ?_⟩
  obtain ⟨_, _, e⟩ := kill_ok hk
  rw [e]

/-- `kill_partition_jobs`' loop: a sequence of kills. -/
theorem kill_loop_r {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) res,
      k.sched.depth.val - i.val = n →
      logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i = ok res →
      ∃ k2 : logic.kernel.Kernel NT NR NP NL NS NE, RSteps cfg k.sched k2.sched ∧
        res = (k2.sched.release, k2.sched.job, k2.sched.stack, k2.sched.depth,
          k2.sched.start_ceiling, k2.sched.ceiling, k2.sched.lock_res, k2.sched.lock_depth,
          k2.sched.lock_len, k2.timing, k2.slots, k2.budget, k2.log, k2.stopped,
          k2.mit_violations, k2.reinit_partition, k2.stop_dma, k2.safe_state, k2.started,
          k2.now, k2.history) := by
  intro n
  induction n with
  | zero =>
    intro k i res hn h
    unfold logic.kernel.Kernel.kill_partition_jobs_loop loop at h
    simp only [logic.kernel.Kernel.kill_partition_jobs_loop.body] at h
    have hlt : ¬ i < k.sched.depth := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    exact ⟨k, Relation.ReflTransGen.refl, h.symm⟩
  | succ n ih =>
    intro k i res hn h
    unfold logic.kernel.Kernel.kill_partition_jobs_loop loop at h
    simp only [logic.kernel.Kernel.kill_partition_jobs_loop.body] at h
    have hiv : i.val < k.sched.depth.val := by omega
    have hlt : i < k.sched.depth := by scalar_tac
    simp only [hlt, ite_true, lift, bind_ok, bind_eq_ok] at h
    obtain ⟨cf, ⟨u, hu, tc, htc, self1, hself1, i2, hi2, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    have hi2 := usize_add_ok hi2
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at hi2
    have hrec : logic.kernel.Kernel.kill_partition_jobs_loop self1 cfg partition i2 = ok res := by
      unfold logic.kernel.Kernel.kill_partition_jobs_loop
      simp only [logic.kernel.Kernel.kill_partition_jobs_loop.body, lift, bind_ok]
      exact hloop
    have hcase : (RStep cfg k.sched self1.sched ∧ self1.sched.depth = k.sched.depth) ∨ self1 = k := by
      peel hself1
      · exact Or.inl (kill_at_r hiv (by assumption))
      · exact Or.inr rfl
      · exact Or.inr rfl
    rcases hcase with ⟨hr, hd⟩ | rfl
    · obtain ⟨k2, h2, e⟩ := ih self1 i2 res (by rw [hd]; omega) hrec
      exact ⟨k2, Relation.ReflTransGen.head hr h2, e⟩
    · exact ih _ i2 res (by omega) hrec

theorem kill_partition_jobs_r {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') :
    RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨res, hres, h⟩ := h
  obtain ⟨k2, h2, e⟩ := kill_loop_r _ _ _ res rfl hres
  subst e
  simp only [Aeneas.Std.uncurry] at h
  have h := ok_inj h
  subst h
  exact h2

/-- `clear_partition_releases`' loop: a sequence of dropped Releases. -/
theorem clear_loop_r {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NT.val - t.val = n →
      logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k' →
      RSteps cfg k.sched k'.sched := by
  intro n
  induction n with
  | zero =>
    intro k t k' hn h
    unfold logic.kernel.Kernel.clear_partition_releases_loop loop at h
    simp only [logic.kernel.Kernel.clear_partition_releases_loop.body] at h
    have hlt : ¬ t < NT := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    exact Relation.ReflTransGen.refl
  | succ n ih =>
    intro k t k' hn h
    unfold logic.kernel.Kernel.clear_partition_releases_loop loop at h
    simp only [logic.kernel.Kernel.clear_partition_releases_loop.body] at h
    have hlt : t < NT := by scalar_tac
    simp only [hlt, ite_true, lift, bind_ok, bind_eq_ok] at h
    obtain ⟨cf, ⟨tc, htc, self1, hself1, t1, ht1, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    have ht1 := usize_add_ok ht1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at ht1
    have hrec : logic.kernel.Kernel.clear_partition_releases_loop self1 cfg partition t1 = ok k' := by
      unfold logic.kernel.Kernel.clear_partition_releases_loop
      simp only [logic.kernel.Kernel.clear_partition_releases_loop.body, lift, bind_ok]
      exact hloop
    have hcase : RSteps cfg k.sched self1.sched := by
      peel hself1
      all_goals first
        | exact Relation.ReflTransGen.refl
        | exact Relation.ReflTransGen.single (RStep.kclear _ _ _ (by assumption))
    exact hcase.trans (ih self1 t1 k' (by omega) hrec)

theorem clear_partition_releases_r {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') :
    RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  rename_i k1 hk1 a ha
  dsimp only
  exact clear_loop_r _ _ _ _ rfl hk1

/-- `enter_safe_state`'s loop: every Release dropped. -/
theorem safe_loop_r {cfg : logic.Config NT NR NP} :
    ∀ (n : Nat) (s : logic.sched.Sched NT NR NL) (t : Usize) (s' : logic.sched.Sched NT NR NL),
      NT.val - t.val = n →
      logic.kernel.Kernel.enter_safe_state_loop s cfg t = ok s' → RSteps cfg s s' := by
  intro n
  induction n with
  | zero =>
    intro s t s' hn h
    unfold logic.kernel.Kernel.enter_safe_state_loop loop at h
    simp only [logic.kernel.Kernel.enter_safe_state_loop.body] at h
    have hlt : ¬ t < NT := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    exact Relation.ReflTransGen.refl
  | succ n ih =>
    intro s t s' hn h
    unfold logic.kernel.Kernel.enter_safe_state_loop loop at h
    simp only [logic.kernel.Kernel.enter_safe_state_loop.body] at h
    have hlt : t < NT := by scalar_tac
    simp only [hlt, ite_true, lift, bind_ok, bind_eq_ok] at h
    obtain ⟨cf, ⟨s1, hs1, t1, ht1, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    have ht1 := usize_add_ok ht1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at ht1
    have hrec : logic.kernel.Kernel.enter_safe_state_loop s1 cfg t1 = ok s' := by
      unfold logic.kernel.Kernel.enter_safe_state_loop
      simp only [logic.kernel.Kernel.enter_safe_state_loop.body, lift, bind_ok]
      exact hloop
    exact Relation.ReflTransGen.head (RStep.kclear _ _ _ hs1) (ih s1 t1 s' (by omega) hrec)

theorem enter_safe_state_r {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.enter_safe_state k cfg = ok k') : RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  peel h
  · rename_i hs s hl
    dsimp only
    exact safe_loop_r _ _ _ _ rfl hl
  · rename_i k1 hs hp s hl
    dsimp only
    have := safe_loop_r _ _ _ _ rfl hl
    rw [push_event_sched hp] at this
    exact this

theorem end_partition_jobs_r {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') :
    RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals
    have hc := clear_partition_releases_r h
    have hk := kill_partition_jobs_r (by assumption)
  · exact (Relation.ReflTransGen.single (end_top_r (by assumption))).trans (hk.trans hc)
  · exact hk.trans hc
  · exact hk.trans hc

theorem stop_partition_r {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') :
    RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  rename_i k1 hp k2 he a ha
  dsimp only
  have := end_partition_jobs_r he
  rw [push_event_sched hp] at this
  exact this

theorem restart_partition_r {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') :
    RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  rename_i k1 hp k2 he a ha a1 ha1
  dsimp only
  have := end_partition_jobs_r he
  rw [push_event_sched hp] at this
  exact this

set_option maxHeartbeats 2000000 in
/-- A Health_Monitor response, whatever the response and its cause. -/
theorem apply_response_r {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') :
    RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.apply_response at h
  peel h
  all_goals first
    | exact Relation.ReflTransGen.refl
    | exact restart_partition_r h
    | exact stop_partition_r h
    | exact enter_safe_state_r h
    | skip
  · rename_i hd i hi top htop htt hn hnt k1 hp k2 he tc htc hk
    have hr := end_top_r he
    rw [push_event_sched hp] at hr
    rw [end_mit_window_sched h]
    exact Relation.ReflTransGen.single hr
  · rename_i hd i hi top htop htt hn hnt k1 hp k2 he tc htc hk
    have hr := end_top_r he
    rw [push_event_sched hp] at hr
    exact Relation.ReflTransGen.single hr

theorem mit_violation_r {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t partition offender : U8}
    (h : logic.kernel.Kernel.mit_violation k cfg t partition offender = ok k') :
    RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.mit_violation at h
  peel h
  · rename_i prs hprs k1 hp c hc c2 back hb pc hpc hgt pc1 hpc1
    have := apply_response_r h
    dsimp only at this
    rw [push_event_sched hp] at this
    exact this
  · rename_i prs hprs k1 hp c hc c2 back hb pc hpc hgt
    dsimp only
    rw [push_event_sched hp]

/-! ## The entry points -/

theorem rel_of_index {s : logic.sched.Sched NT NR NL} {t x : U8}
    (h : s.release.index_usize (UScalar.cast .Usize t) = ok x) : rel s t.val = x.val := by
  obtain ⟨_, rfl⟩ := index_ok h
  rw [U8.cast_Usize_val_eq]
  rfl

theorem u64_add_ok {a b c : U64} (h : a + b = ok c) : c.val = a.val + b.val := by
  have := UScalar.add_equiv a b
  simp_all

theorem intake_accept : logic.time.INTAKE_ACCEPT = 0#u8 := by
  unfold logic.time.INTAKE_ACCEPT; rfl
theorem intake_defer : logic.time.INTAKE_DEFER = 1#u8 := by
  unfold logic.time.INTAKE_DEFER; rfl
theorem intake_discard : logic.time.INTAKE_DISCARD = 2#u8 := by
  unfold logic.time.INTAKE_DISCARD; rfl

/-- The MIT decision (`Timing::intake_decision`, R10.1 to R10.3): an event
is accepted only with no Release of the Task held, and only when the Task
has had no accepted Release or the last one became due at least one MIT
before `now` (the MIT spacing of Property 8, on the code as extracted). -/
theorem intake_decision_accept {tm : logic.time.Timing NT} {cfg : logic.Config NT NR NP}
    {t d : U8} {rn : Bool} {now : U64}
    (h : logic.time.Timing.intake_decision tm cfg t rn now = ok d)
    (hd : d = logic.time.INTAKE_ACCEPT) :
    rn = true ∧
    (tm.has_last.val[t.val]! = true →
      ∃ tc, cfg.tasks.val[t.val]? = some tc ∧
        (tm.last_due.val[t.val]!).val + tc.period.val ≤ now.val) := by
  subst hd
  unfold logic.time.Timing.intake_decision at h
  split at h
  · rename_i hrn
    refine ⟨hrn, ?_⟩
    simp only [lift, bind_ok, bind_eq_ok] at h
    obtain ⟨tc, htc, b, hb, h⟩ := h
    obtain ⟨_, htc'⟩ := index_ok' htc
    obtain ⟨_, rfl⟩ := index_ok hb
    rw [U8.cast_Usize_val_eq] at htc'
    rw [U8.cast_Usize_val_eq] at h
    split at h
    · rename_i hb
      simp only [bind_eq_ok] at h
      obtain ⟨i3, hi3, i4, hi4, h⟩ := h
      obtain ⟨_, rfl⟩ := index_ok hi3
      rw [U8.cast_Usize_val_eq] at hi4
      split at h
      · rename_i hge
        intro _
        refine ⟨tc, htc', ?_⟩
        have := u64_add_ok hi4
        scalar_tac
      · simp only [bind_eq_ok] at h
        obtain ⟨tc1, _, h⟩ := h
        split at h
        · have h := ok_inj h
          rw [intake_accept, intake_defer] at h
          cases h
        · have h := ok_inj h
          rw [intake_accept, intake_discard] at h
          cases h
    · rename_i hb
      intro hx
      exact absurd hx hb
  · have h := ok_inj h
    rw [intake_accept, intake_discard] at h
    cases h

set_option maxHeartbeats 2000000 in
/-- The MIT intake (`intake_release`): the Release is made (`make_pending`,
only with no Release of the Task held);
or it is held to the end of the MIT (`defer`, only with no Release of the
Task held) and the MIT_Violation's response follows; or the scheduler state
changes by a response only (a discarded event, a stopped Partition, the
safe state: no change at all). -/
theorem intake_release_eff {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t sp off : U8} {now : U64}
    (h : logic.kernel.Kernel.intake_release k cfg t sp now off = ok k') :
    (rel k.sched t.val = 0 ∧ logic.sched.Sched.make_pending k.sched cfg t = ok k'.sched) ∨
    (∃ s1, rel k.sched t.val = 0 ∧ logic.sched.Sched.defer k.sched cfg t = ok s1 ∧
      RSteps cfg s1 k'.sched) ∨
    RSteps cfg k.sched k'.sched := by
  unfold logic.kernel.Kernel.intake_release at h
  peel h
  all_goals
    have ho := observe_sched (by assumption)
    first
      | (refine Or.inr (Or.inr ?_); try dsimp only
         rw [ho])
      | (have hacc := (intake_decision_accept (by assumption) (by assumption)).1
         have hr := rel_of_index (by assumption)
         rw [of_decide_eq_true hacc, rel_none, ho] at hr
         refine Or.inl ⟨hr, ?_⟩; dsimp only; rw [← ho]; assumption)
      | (refine Or.inr (Or.inr ?_); have hm := mit_violation_r h; dsimp only at hm
         rw [ho] at hm; exact hm)
      | (have hm := mit_violation_r h
         dsimp only at hm
         have hr := rel_of_index (by assumption)
         simp only [*, rel_none] at hr
         simp only [ho] at *
         exact Or.inr (Or.inl ⟨_, hr, by assumption, hm⟩))

end P18
