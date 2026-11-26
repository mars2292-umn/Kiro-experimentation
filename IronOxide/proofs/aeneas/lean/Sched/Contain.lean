import Sched.Nvic
open Aeneas Aeneas.Std Result

/-! # The Health_Monitor's responses are contained (2026-11-25)

What Verus proves of `rsk_logic` as `Kernel::response_applied`,
`others_unchanged`, `partition_ended` and `fault_recorded` (R14.3, R14.5,
R14.6, R14.9), here of the Kernel's functions as Aeneas extracts them, each
statement of the form "if the function returns":

- `CStep`, `CSteps`: a response changes the scheduler state by ending the
  Job on top, marking a preempted Job killed, and dropping a Release, each
  of a Task in the set `inP` (the Tasks of the response's Partition, or the
  one Task whose Job a response ends).
- `Cont` (`csteps_cont`): what such steps leave alone. The stack keeps its
  entries and only loses Jobs from the top; a Job of a Task outside `inP`
  stays on it; the Release and Job state of a Task outside `inP` are as
  they were; a lock entry held by a Job of a Task outside `inP` stays, with
  its Resource and holder (`others_unchanged`, the scheduler's clauses).
- `apply_response_cases`: a response returns the state it was given
  (record only), or ends the Job on top through `Sched::finish_top` with
  that Job the response's Task (END_JOB of a Job holding nothing), or is
  `restart_partition`, `stop_partition` or `enter_safe_state`.
- `apply_response_contained`: so unless it is the system safe state, a
  response is `CSteps` over the Tasks of its Partition and its Task.
- `end_partition_jobs_ended` (`partition_ended`): after a stop or a
  restart no Task of the Partition has a Release pending or deferred and
  none has a Job running, given that every Job in progress is on the stack
  (`OnStack`, the run's structure).
- `stop_partition_flags`, `restart_partition_flags`: the Partition is
  marked stopped, or its re-initialization requested with its MIT_Violation
  count at 0, and its EasyDMA stop requested; `FL`: no other Partition's
  flag or count changes.
- `fault_first` (`fault_recorded`): a fault's event is pushed first,
  against no Task and the Kernel when the fault is the Kernel's or no Job
  executes, and then the safe state entered; otherwise against the Task on
  top and that Task's Partition, and then that Partition's declared fault
  response applied.

Not here, proven in Verus only: that the Budget accounts and the timer
slots of the other Partitions' Tasks are left (the accounts' and the
slots' own invariants are in `Sched/Overrun.lean` and `Sched/Slots.lean`,
not this frame); that the System_Ceiling does not rise. The timing state
under a response is in `Sched/SpFrame.lean` and `Sched/MaskFrame.lean`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The Partition of Task `u` in the configuration (0 out of range). -/
def ptOf (cfg : logic.Config NT NR NP) (u : Nat) : Nat :=
  match cfg.tasks.val[u]? with
  | some tc => tc.partition.val
  | none => 0

/-! ## The steps of a response, with whose they are -/

/-- One step of a response on the scheduler state, of a Task in `inP`. -/
inductive CStep (cfg : logic.Config NT NR NP) (inP : Nat → Prop) :
    logic.sched.Sched NT NR NL → logic.sched.Sched NT NR NL → Prop
  | kend (s s' : logic.sched.Sched NT NR NL) (r : Usize)
      (hin : inP (stk s (s.depth.val - 1)))
      (h : logic.sched.Sched.finish_top s cfg = ok (r, s')) : CStep cfg inP s s'
  | kkill (s s' : logic.sched.Sched NT NR NL) (p : Usize) (hin : inP (stk s p.val))
      (h : logic.sched.Sched.kill s cfg p = ok s') : CStep cfg inP s s'
  | kclear (s s' : logic.sched.Sched NT NR NL) (t : U8) (hin : inP t.val)
      (h : logic.sched.Sched.clear_release s cfg t = ok s') : CStep cfg inP s s'

abbrev CSteps (cfg : logic.Config NT NR NP) (inP : Nat → Prop) :
    logic.sched.Sched NT NR NL → logic.sched.Sched NT NR NL → Prop :=
  Relation.ReflTransGen (CStep cfg inP)

theorem CStep.mono {cfg : logic.Config NT NR NP} {inP inQ : Nat → Prop}
    (hq : ∀ u, inP u → inQ u) {s s' : logic.sched.Sched NT NR NL} (h : CStep cfg inP s s') :
    CStep cfg inQ s s' := by
  cases h with
  | kend r hin h => exact CStep.kend _ _ r (hq _ hin) h
  | kkill p hin h => exact CStep.kkill _ _ p (hq _ hin) h
  | kclear t hin h => exact CStep.kclear _ _ t (hq _ hin) h

theorem CSteps.mono {cfg : logic.Config NT NR NP} {inP inQ : Nat → Prop}
    (hq : ∀ u, inP u → inQ u) {s s' : logic.sched.Sched NT NR NL} (h : CSteps cfg inP s s') :
    CSteps cfg inQ s s' := by
  induction h with
  | refl => exact Relation.ReflTransGen.refl
  | tail _ hs ih => exact Relation.ReflTransGen.tail ih (CStep.mono hq hs)

/-- What steps of the Tasks in `inP` leave alone. -/
def Cont (inP : Nat → Prop) (s s' : logic.sched.Sched NT NR NL) : Prop :=
  (∀ p, stk s' p = stk s p) ∧ s'.depth.val ≤ s.depth.val ∧
  (∀ i, i < s.depth.val → ¬ inP (stk s i) → i < s'.depth.val) ∧
  (∀ u, ¬ inP u → rel s' u = rel s u ∧ job s' u = job s u) ∧
  (∀ e, lres s' e = lres s e ∧ ldep s' e = ldep s e) ∧ s'.lock_len.val ≤ s.lock_len.val ∧
  (∀ e, e < s.lock_len.val → ¬ inP (stk s (ldep s e)) → e < s'.lock_len.val) ∧
  (∀ u, rel s' u = rel s u ∨ rel s' u = 0) ∧ (∀ u, job s u ≠ 1 → job s' u ≠ 1)

theorem Cont.refl (inP : Nat → Prop) (s : logic.sched.Sched NT NR NL) : Cont inP s s :=
  ⟨fun _ => rfl, Nat.le_refl _, fun _ h _ => h, fun _ _ => ⟨rfl, rfl⟩, fun _ => ⟨rfl, rfl⟩,
    Nat.le_refl _, fun _ h _ => h, fun _ => Or.inl rfl, fun _ h => h⟩

theorem Cont.trans {inP : Nat → Prop} {a b c : logic.sched.Sched NT NR NL}
    (h1 : Cont inP a b) (h2 : Cont inP b c) : Cont inP a c := by
  obtain ⟨a1, a2, a3, a4, a5, a6, a7, a8, a9⟩ := h1
  obtain ⟨b1, b2, b3, b4, b5, b6, b7, b8, b9⟩ := h2
  refine ⟨fun p => (b1 p).trans (a1 p), by omega, fun i hi hn => ?_, fun u hu => ?_,
    fun e => ⟨(b5 e).1.trans (a5 e).1, (b5 e).2.trans (a5 e).2⟩, by omega, fun e he hn => ?_,
    fun u => ?_, fun u hu => b9 u (a9 u hu)⟩
  · exact b3 i (a3 i hi hn) (by rw [a1 i]; exact hn)
  · exact ⟨(b4 u hu).1.trans (a4 u hu).1, (b4 u hu).2.trans (a4 u hu).2⟩
  · exact b7 e (a7 e he hn) (by rw [(a5 e).2, a1]; exact hn)
  · rcases b8 u with h | h
    · rcases a8 u with h' | h'
      · exact Or.inl (h.trans h')
      · exact Or.inr (h.trans h')
    · exact Or.inr h

theorem cstep_cont {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    {s s' : logic.sched.Sched NT NR NL} (h : CStep cfg inP s s') : Cont inP s s' := by
  cases h with
  | kend r hin h =>
    obtain ⟨hd1, _, _, hrel, hjob, hstk, hdep, hlr, hld, hlen, hdrop, _, _⟩ := finish_top_views h
    refine ⟨hstk, by omega, fun i hi hn => ?_, fun u hu => ⟨hrel u, ?_⟩,
      fun e => ⟨hlr e, hld e⟩, hlen, fun e he hn => ?_, fun u => Or.inl (hrel u), fun u hu => ?_⟩
    · have : i ≠ s.depth.val - 1 := fun e => hn (by rw [e]; exact hin)
      omega
    · rw [hjob u]
      have : ¬ u = stk s (s.depth.val - 1) := fun e => hu (by rw [e]; exact hin)
      simp only [this, ite_false]
    · by_contra hge
      have := hdrop e (by omega) he
      exact hn (by rw [this]; exact hin)
    · rw [hjob u]
      split
      · omega
      · exact hu
  | kkill p hin h =>
    obtain ⟨_, hrel, hjob, hstk, hdep, hl, hld, hn, _⟩ := kill_views h
    refine ⟨hstk, by rw [hdep], fun i hi _ => by rw [hdep]; exact hi, fun u hu => ⟨hrel u, ?_⟩,
      fun e => ⟨hl e, hld e⟩, by rw [hn], fun e he _ => by rw [hn]; exact he,
      fun u => Or.inl (hrel u), fun u hu => ?_⟩
    · rw [hjob u]
      have : ¬ u = stk s p.val := fun e => hu (by rw [e]; exact hin)
      simp only [this, ite_false]
    · rw [hjob u]
      split
      · omega
      · exact hu
  | kclear t hin h =>
    obtain ⟨_, hrel, hjob, hstk, hdep, hl, hld, hn, _⟩ := clear_release_views h
    refine ⟨hstk, by rw [hdep], fun i hi _ => by rw [hdep]; exact hi, fun u hu => ⟨?_, hjob u⟩,
      fun e => ⟨hl e, hld e⟩, by rw [hn], fun e he _ => by rw [hn]; exact he,
      fun u => ?_, fun u hu => by rw [hjob u]; exact hu⟩
    · rw [hrel u]
      have : ¬ u = t.val := fun e => hu (by rw [e]; exact hin)
      simp only [this, ite_false]
    · rw [hrel u]
      split
      · exact Or.inr rfl
      · exact Or.inl rfl

/-- **A response's steps leave the other Tasks alone** (`others_unchanged`,
the scheduler's clauses). -/
theorem csteps_cont {cfg : logic.Config NT NR NP} {inP : Nat → Prop}
    {s s' : logic.sched.Sched NT NR NL} (h : CSteps cfg inP s s') : Cont inP s s' := by
  induction h with
  | refl => exact Cont.refl _ _
  | tail _ hs ih => exact Cont.trans ih (cstep_cont hs)

/-! ## The functions of a response -/

/-- The Partition of the Task at a stack position, from the reads the code
makes. -/
theorem pt_of_stack {s : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {i : Usize}
    {u : U8} {tc : logic.TaskCfg} (hu : s.stack.index_usize i = ok u)
    (htc : cfg.tasks.index_usize (UScalar.cast .Usize u) = ok tc) :
    stk s i.val = u.val ∧ ptOf cfg (stk s i.val) = tc.partition.val := by
  obtain ⟨_, rfl⟩ := index_ok hu
  obtain ⟨_, htc'⟩ := index_ok' htc
  rw [U8.cast_Usize_val_eq] at htc'
  refine ⟨rfl, ?_⟩
  unfold ptOf stk
  rw [htc']

theorem pt_of_index {cfg : logic.Config NT NR NP} {t : Usize} {tc : logic.TaskCfg}
    (htc : cfg.tasks.index_usize t = ok tc) : ptOf cfg t.val = tc.partition.val := by
  obtain ⟨_, htc'⟩ := index_ok' htc
  unfold ptOf
  rw [htc']

theorem kill_at_c {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.kill_at k cfg i = ok k') :
    logic.sched.Sched.kill k.sched cfg i = ok k'.sched := by
  unfold logic.kernel.Kernel.kill_at at h
  peel h
  rename_i u hu s1 hk
  dsimp only
  exact hk

/-- `kill_partition_jobs`' loop: kills of Jobs of the Partition's Tasks,
after which no Job of the Partition at or above the position it started at
is running. -/
theorem kill_loop_c {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) res,
      k.sched.depth.val - i.val = n →
      logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i = ok res →
      ∃ k2 : logic.kernel.Kernel NT NR NP NL NS NE,
        CSteps cfg (fun u => ptOf cfg u = partition.val) k.sched k2.sched ∧
        (∀ j, i.val ≤ j → j < k.sched.depth.val → ptOf cfg (stk k.sched j) = partition.val →
          job k2.sched (stk k.sched j) ≠ 1) ∧
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
    exact ⟨k, Relation.ReflTransGen.refl, fun j h1 h2 => by omega, h.symm⟩
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
    obtain ⟨hsu, hpt⟩ := pt_of_stack hu htc
    have hcase : CSteps cfg (fun u => ptOf cfg u = partition.val) k.sched self1.sched ∧
        (ptOf cfg (stk k.sched i.val) = partition.val → job self1.sched (stk k.sched i.val) ≠ 1) := by
      peel hself1
      · -- The Job is of the Partition and in progress: killed.
        rename_i hp i3 hi3 hne
        have hk := kill_at_c hself1
        have hin : ptOf cfg (stk k.sched i.val) = partition.val := by rw [hpt, hp]
        refine ⟨Relation.ReflTransGen.single (CStep.kkill _ _ i hin hk), fun _ => ?_⟩
        obtain ⟨_, _, hjob, _⟩ := kill_views hk
        rw [hjob]
        simp
      · -- Of the Partition, no Job in progress.
        rename_i hp i3 hi3 hne
        refine ⟨Relation.ReflTransGen.refl, fun _ => ?_⟩
        obtain ⟨_, rfl⟩ := index_ok hi3
        have e : job k.sched (stk k.sched i.val) = (k.sched.job.val[(UScalar.cast .Usize u).val]!).val := by
          rw [hsu, U8.cast_Usize_val_eq]; rfl
        rw [e]
        have : (k.sched.job.val[(UScalar.cast .Usize u).val]!).val = (logic.sched.JOB_NONE).val := by
          simpa using hne
        rw [this, job_none]
        decide
      · -- Not of the Partition.
        rename_i hp
        refine ⟨Relation.ReflTransGen.refl, fun hq => ?_⟩
        exfalso
        rw [hpt] at hq
        exact hp (by scalar_tac)
    obtain ⟨hc1, hj1⟩ := hcase
    have hcont := csteps_cont hc1
    have hdep : self1.sched.depth.val = k.sched.depth.val := by
      -- A kill keeps the depth; so does no step.
      have h3 := hcont.2.1
      have h4 : ∀ {a b : logic.sched.Sched NT NR NL},
          CSteps cfg (fun u => ptOf cfg u = partition.val) a b →
          (∀ {x y : logic.sched.Sched NT NR NL}, CStep cfg (fun u => ptOf cfg u = partition.val) x y →
            True) → True := fun _ _ => trivial
      clear h4
      peel hself1
      · rename_i hp i3 hi3 hne
        obtain ⟨_, _, _, _, hd, _⟩ := kill_views (kill_at_c hself1)
        rw [hd]
      · rfl
      · rfl
    obtain ⟨k2, h2, hj2, e⟩ := ih self1 i2 res (by rw [hdep]; omega) hrec
    have hcont2 := csteps_cont h2
    refine ⟨k2, hc1.trans h2, fun j h1 h2' hp => ?_, e⟩
    by_cases hji : j = i.val
    · rw [hji] at hp ⊢
      exact hcont2.2.2.2.2.2.2.2.2 _ (hj1 hp)
    · have := hj2 j (by omega) (by rw [hdep]; exact h2') (by rw [hcont.1 j]; exact hp)
      rw [hcont.1 j] at this
      exact this

theorem kill_partition_jobs_c {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') :
    CSteps cfg (fun u => ptOf cfg u = partition.val) k.sched k'.sched ∧
    ∀ j, j < k.sched.depth.val → ptOf cfg (stk k.sched j) = partition.val →
      job k'.sched (stk k.sched j) ≠ 1 := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨res, hres, h⟩ := h
  obtain ⟨k2, h2, hj, e⟩ := kill_loop_c _ _ _ res rfl hres
  subst e
  simp only [Aeneas.Std.uncurry] at h
  have h := ok_inj h
  subst h
  exact ⟨h2, fun j hj1 hp => hj j (Nat.zero_le _) hj1 hp⟩

variable [hNT : Fact (NT.val ≤ 256)]

include hNT in
/-- A Release of a Task of the Partition dropped. -/
theorem clear_c1 {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : Usize} {tc : logic.TaskCfg} {partition : U8}
    (hc : logic.sched.Sched.clear_release s cfg (UScalar.cast .U8 t) = ok s')
    (ht : t.val < NT.val) (htc : cfg.tasks.index_usize t = ok tc) (hp : tc.partition = partition) :
    CSteps cfg (fun u => ptOf cfg u = partition.val) s s' ∧ rel s' t.val = 0 := by
  have hN := hNT.out
  have e : (UScalar.cast .U8 t : U8).val = t.val := by
    simp only [UScalar.cast_val_eq, UScalarTy.numBits]
    omega
  refine ⟨Relation.ReflTransGen.single (CStep.kclear _ _ _ ?_ hc), ?_⟩
  · show ptOf cfg (UScalar.cast .U8 t : U8).val = partition.val
    rw [e, pt_of_index htc, hp]
  · obtain ⟨_, hrel, _⟩ := clear_release_views hc
    rw [hrel, e]
    simp

include hNT in
set_option maxHeartbeats 4000000 in
/-- `clear_partition_releases`' loop: Releases of the Partition's Tasks
dropped, after which none from the Task it started at is pending or
deferred. -/
theorem clear_loop_c {cfg : logic.Config NT NR NP} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NT.val - t.val = n →
      logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k' →
      CSteps cfg (fun u => ptOf cfg u = partition.val) k.sched k'.sched ∧
      ∀ u, t.val ≤ u → u < NT.val → ptOf cfg u = partition.val → rel k'.sched u = 0 := by
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
    exact ⟨Relation.ReflTransGen.refl, fun u h1 h2 => by omega⟩
  | succ n ih =>
    intro k t k' hn h
    unfold logic.kernel.Kernel.clear_partition_releases_loop loop at h
    simp only [logic.kernel.Kernel.clear_partition_releases_loop.body] at h
    have hlt : t < NT := by scalar_tac
    have htv : t.val < NT.val := by omega
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
    have hcase : CSteps cfg (fun u => ptOf cfg u = partition.val) k.sched self1.sched ∧
        (ptOf cfg t.val = partition.val → rel self1.sched t.val = 0) := by
      peel hself1
      all_goals first
        | (dsimp only
           have hc := clear_c1 (by assumption) htv htc (by assumption)
           exact ⟨hc.1, fun _ => hc.2⟩)
        | (refine ⟨Relation.ReflTransGen.refl, fun hq => ?_⟩
           exfalso
           rw [pt_of_index htc] at hq
           have hne : ¬ tc.partition = partition := by assumption
           exact hne (by scalar_tac))
    obtain ⟨hc1, hr1⟩ := hcase
    obtain ⟨hc2, hr2⟩ := ih self1 t1 k' (by omega) hrec
    refine ⟨hc1.trans hc2, fun u hu1 hu2 hp => ?_⟩
    by_cases hut : u = t.val
    · rw [hut] at hp ⊢
      rcases (csteps_cont hc2).2.2.2.2.2.2.2.1 t.val with h | h
      · rw [h]; exact hr1 hp
      · exact h
    · exact hr2 u (by omega) hu2 hp

include hNT in
theorem clear_partition_releases_c {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') :
    CSteps cfg (fun u => ptOf cfg u = partition.val) k.sched k'.sched ∧
    ∀ u, u < NT.val → ptOf cfg u = partition.val → rel k'.sched u = 0 := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  rename_i k1 hk1 a ha
  dsimp only
  obtain ⟨hc, hr⟩ := clear_loop_c _ _ _ _ rfl hk1
  exact ⟨hc, fun u hu hp => hr u (Nat.zero_le _) hu hp⟩

/-- Every Job in progress or killed is on the stack (the run's structure,
`Wf.on_stack`). -/
def OnStack (s : logic.sched.Sched NT NR NL) : Prop :=
  ∀ u, u < NT.val → job s u ≠ 0 → ∃ i, i < s.depth.val ∧ stk s i = u

omit hNT in
theorem onstack_kend {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : Usize}
    (h : logic.sched.Sched.finish_top s cfg = ok (r, s')) (ho : OnStack s) : OnStack s' := by
  obtain ⟨hd1, _, _, _, hjob, hstk, hdep, _⟩ := finish_top_views h
  intro u hu hj
  rw [hjob u] at hj
  by_cases hut : u = stk s (s.depth.val - 1)
  · simp only [hut, ite_true] at hj
    omega
  · simp only [hut, ite_false] at hj
    obtain ⟨i, hi, hs⟩ := ho u hu hj
    have : i ≠ s.depth.val - 1 := fun e => hut (by rw [← hs, e])
    exact ⟨i, by omega, by rw [hstk]; exact hs⟩

include hNT in
set_option maxHeartbeats 4000000 in
/-- **`Kernel::end_partition_jobs` (a stop or a restart) ends the
Partition and nothing else** (`partition_ended`, `others_unchanged`): its
steps are of the Partition's Tasks, and after it no Task of the Partition
has a Release pending or deferred and none has a Job running. -/
theorem end_partition_jobs_ended {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') :
    CSteps cfg (fun u => ptOf cfg u = partition.val) k.sched k'.sched ∧
    (OnStack k.sched → ∀ u, u < NT.val → ptOf cfg u = partition.val →
      rel k'.sched u = 0 ∧ job k'.sched u ≠ 1) := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals
    obtain ⟨hc, hr⟩ := clear_partition_releases_c h
    obtain ⟨hk, hj⟩ := kill_partition_jobs_c (by assumption)
    have hcc := csteps_cont hc
  · -- The Job on top is the Partition's: ended first.
    rename_i k1 hd i hi top htop tc htc hp he k2 hkk
    obtain ⟨r, hf⟩ := end_top_finish he
    obtain ⟨hsu, hpt⟩ := pt_of_stack htop htc
    have hiv : i.val = k.sched.depth.val - 1 := by
      have h1 : (1#usize).val = 1 := rfl
      have := (usize_sub_ok hi).2
      omega
    have hin : ptOf cfg (stk k.sched (k.sched.depth.val - 1)) = partition.val := by
      rw [← hiv, hpt, hp]
    refine ⟨(Relation.ReflTransGen.single (CStep.kend _ _ r hin hf)).trans (hk.trans hc),
      fun ho u hu hp' => ⟨hr u hu hp', fun h1 => ?_⟩⟩
    have ho1 := onstack_kend hf ho
    have hkc := csteps_cont hk
    have h2 : job k2.sched u = 1 := by
      by_contra hne
      exact hcc.2.2.2.2.2.2.2.2 u hne h1
    have h3 : job k1.sched u = 1 := by
      by_contra hne
      exact hkc.2.2.2.2.2.2.2.2 u hne h2
    obtain ⟨j, hj1, hjs⟩ := ho1 u hu (by omega)
    exact hj j hj1 (by rw [hjs]; exact hp') (by rw [hjs]; exact h2)
  · rename_i hd i hi top htop tc htc hp k2 hkk
    refine ⟨hk.trans hc, fun ho u hu hp' => ⟨hr u hu hp', fun h1 => ?_⟩⟩
    have hkc := csteps_cont hk
    have h2 : job k2.sched u = 1 := by
      by_contra hne
      exact hcc.2.2.2.2.2.2.2.2 u hne h1
    have h3 : job k.sched u = 1 := by
      by_contra hne
      exact hkc.2.2.2.2.2.2.2.2 u hne h2
    obtain ⟨j, hj1, hjs⟩ := ho u hu (by omega)
    exact hj j hj1 (by rw [hjs]; exact hp') (by rw [hjs]; exact h2)
  · rename_i hd k2 hkk
    refine ⟨hk.trans hc, fun ho u hu hp' => ⟨hr u hu hp', fun h1 => ?_⟩⟩
    have hkc := csteps_cont hk
    have h2 : job k2.sched u = 1 := by
      by_contra hne
      exact hcc.2.2.2.2.2.2.2.2 u hne h1
    have h3 : job k.sched u = 1 := by
      by_contra hne
      exact hkc.2.2.2.2.2.2.2.2 u hne h2
    obtain ⟨j, hj1, hjs⟩ := ho u hu (by omega)
    exact hj j hj1 (by rw [hjs]; exact hp') (by rw [hjs]; exact h2)

/-! ## The per-Partition flags and counts -/

omit hNT

/-- The stopped flags, the MIT_Violation counts, and the requests for a
re-initialization and for an EasyDMA stop. -/
@[reducible] def fv (k : logic.kernel.Kernel NT NR NP NL NS NE) :
    Array Bool NP × Array U32 NP × Array Bool NP × Array Bool NP :=
  (k.stopped, k.mit_violations, k.reinit_partition, k.stop_dma)

/-- From `a` to `b` no Partition other than `p` has a flag or a count
changed (`others_unchanged`, the clause on the flags). -/
def FL (p : Nat) (a b : Array Bool NP × Array U32 NP × Array Bool NP × Array Bool NP) : Prop :=
  ∀ q, q ≠ p → b.1.val[q]! = a.1.val[q]! ∧ b.2.1.val[q]! = a.2.1.val[q]! ∧
    b.2.2.1.val[q]! = a.2.2.1.val[q]! ∧ b.2.2.2.val[q]! = a.2.2.2.val[q]!

theorem FL.refl (p : Nat) (a : Array Bool NP × Array U32 NP × Array Bool NP × Array Bool NP) :
    FL p a a := fun _ _ => ⟨rfl, rfl, rfl, rfl⟩

theorem FL.trans {p : Nat} {a b c : Array Bool NP × Array U32 NP × Array Bool NP × Array Bool NP}
    (h1 : FL p a b) (h2 : FL p b c) : FL p a c := fun q hq => by
  obtain ⟨a1, a2, a3, a4⟩ := h1 q hq
  obtain ⟨b1, b2, b3, b4⟩ := h2 q hq
  exact ⟨b1.trans a1, b2.trans a2, b3.trans a3, b4.trans a4⟩

theorem set_other {α} [Inhabited α] {n : Usize} {v a : Array α n} {partition : U8} {x : α}
    (h : v.update (UScalar.cast .Usize partition) x = ok a) (q : Nat) (hq : q ≠ partition.val) :
    a.val[q]! = v.val[q]! := by
  obtain ⟨hb, rfl⟩ := update_ok h
  rw [get_set _ _ _ _ hb, U8.cast_Usize_val_eq]
  simp only [hq, ite_false]

theorem set_self {α} [Inhabited α] {n : Usize} {v a : Array α n} {partition : U8} {x : α}
    (h : v.update (UScalar.cast .Usize partition) x = ok a) : a.val[partition.val]! = x := by
  obtain ⟨hb, rfl⟩ := update_ok h
  rw [get_set _ _ _ _ hb, U8.cast_Usize_val_eq]
  simp

theorem FL.set1 {partition : U8} {s a : Array Bool NP} {m : Array U32 NP} {r d : Array Bool NP}
    {x : Bool} (h : s.update (UScalar.cast .Usize partition) x = ok a) :
    FL partition.val (s, m, r, d) (a, m, r, d) := fun q hq => ⟨set_other h q hq, rfl, rfl, rfl⟩

theorem FL.set2 {partition : U8} {s : Array Bool NP} {m a : Array U32 NP} {r d : Array Bool NP}
    {x : U32} (h : m.update (UScalar.cast .Usize partition) x = ok a) :
    FL partition.val (s, m, r, d) (s, a, r, d) := fun q hq => ⟨rfl, set_other h q hq, rfl, rfl⟩

theorem FL.set3 {partition : U8} {s : Array Bool NP} {m : Array U32 NP} {r a d : Array Bool NP}
    {x : Bool} (h : r.update (UScalar.cast .Usize partition) x = ok a) :
    FL partition.val (s, m, r, d) (s, m, a, d) := fun q hq => ⟨rfl, rfl, set_other h q hq, rfl⟩

theorem FL.set4 {partition : U8} {s : Array Bool NP} {m : Array U32 NP} {r d a : Array Bool NP}
    {x : Bool} (h : d.update (UScalar.cast .Usize partition) x = ok a) :
    FL partition.val (s, m, r, d) (s, m, r, a) := fun q hq => ⟨rfl, rfl, rfl, set_other h q hq⟩

syntax "fl_step" : tactic
macro_rules | `(tactic| fl_step) => `(tactic| exact FL.refl _ _)
macro_rules | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (FL.set1 (by assumption)))
macro_rules | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (FL.set2 (by assumption)))
macro_rules | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (FL.set3 (by assumption)))
macro_rules | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (FL.set4 (by assumption)))

syntax "fl_close" : tactic
macro_rules
  | `(tactic| fl_close) => `(tactic| (repeat (first | fl_step | dsimp only [fv])))



theorem push_event_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {p : Nat} {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    FL p (fv k) (fv k') := by
  unfold logic.kernel.Kernel.push_event at h
  peel h
  all_goals fl_close

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (push_event_fl (by assumption)))

theorem arm_budget_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {p : Nat} {t : U8}
    (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') : FL p (fv k) (fv k') := by
  unfold logic.kernel.Kernel.arm_budget at h
  peel h
  all_goals fl_close

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (arm_budget_fl (by assumption)))

theorem end_mit_window_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {p : Nat} {t : U8}
    (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') : FL p (fv k) (fv k') := by
  unfold logic.kernel.Kernel.end_mit_window at h
  peel h
  all_goals fl_close

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (end_mit_window_fl (by assumption)))

theorem enter_safe_state_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {p : Nat}
    (h : logic.kernel.Kernel.enter_safe_state k cfg = ok k') : FL p (fv k) (fv k') := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  peel h
  all_goals fl_close

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (enter_safe_state_fl (by assumption)))

theorem kill_at_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {p : Nat} {i : Usize}
    (h : logic.kernel.Kernel.kill_at k cfg i = ok k') : FL p (fv k) (fv k') := by
  unfold logic.kernel.Kernel.kill_at at h
  peel h
  all_goals fl_close

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (kill_at_fl (by assumption)))

theorem end_top_release_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {p : Nat}
    (h : logic.kernel.Kernel.end_top_release k cfg = ok k') : FL p (fv k) (fv k') := by
  unfold logic.kernel.Kernel.end_top_release at h
  peel h
  all_goals fl_close

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (end_top_release_fl (by assumption)))

theorem end_top_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {p : Nat}
    (h : logic.kernel.Kernel.end_top k cfg = ok k') : FL p (fv k) (fv k') := by
  unfold logic.kernel.Kernel.end_top at h
  peel h
  all_goals fl_close

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (end_top_fl (by assumption)))



/-- `kill_partition_jobs`' loop. -/
theorem kill_loop_fl {cfg : logic.Config NT NR NP} {p : Nat} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) res,
      k.sched.depth.val - i.val = n →
      logic.kernel.Kernel.kill_partition_jobs_loop k cfg partition i = ok res →
      ∃ k2 : logic.kernel.Kernel NT NR NP NL NS NE, FL p (fv k) (fv k2) ∧
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
    exact ⟨k, FL.refl _ _, h.symm⟩
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
    have hcase : self1.sched.depth = k.sched.depth ∧ FL p (fv k) (fv self1) := by
      peel hself1
      · exact ⟨(kill_at_r hiv (by assumption)).2, kill_at_fl (by assumption)⟩
      · exact ⟨rfl, FL.refl _ _⟩
      · exact ⟨rfl, FL.refl _ _⟩
    obtain ⟨hd, htk⟩ := hcase
    obtain ⟨k2, h2, e⟩ := ih self1 i2 res (by rw [hd]; omega) hrec
    exact ⟨k2, FL.trans htk h2, e⟩

theorem kill_partition_jobs_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {p : Nat} {partition : U8}
    (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') : FL p (fv k) (fv k') := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨res, hres, h⟩ := h
  obtain ⟨k2, h2, e⟩ := kill_loop_fl _ _ _ res rfl hres
  subst e
  simp only [Aeneas.Std.uncurry] at h
  have h := ok_inj h
  subst h
  exact h2

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (kill_partition_jobs_fl (by assumption)))

/-- `clear_partition_releases`' loop. -/
theorem clear_loop_fl {cfg : logic.Config NT NR NP} {p : Nat} {partition : U8} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (t : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NT.val - t.val = n →
      logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k' →
      FL p (fv k) (fv k') := by
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
    exact FL.refl _ _
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
    have hcase : FL p (fv k) (fv self1) := by
      peel hself1
      all_goals fl_close
    have h2 := ih self1 t1 k' (by omega) hrec
    exact FL.trans hcase h2

theorem clear_partition_releases_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') :
    FL partition.val (fv k) (fv k') := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  rename_i k1 hk1 a ha
  have h2 : FL partition.val (fv k) (fv k1) := clear_loop_fl _ _ _ _ rfl hk1
  refine FL.trans h2 ?_
  dsimp only [fv]
  exact FL.set4 ha

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (clear_partition_releases_fl (by assumption)))



theorem end_partition_jobs_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') : FL partition.val (fv k) (fv k') := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals fl_close

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (end_partition_jobs_fl (by assumption)))

theorem stop_partition_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') : FL partition.val (fv k) (fv k') := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  all_goals fl_close

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (stop_partition_fl (by assumption)))

theorem restart_partition_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') : FL partition.val (fv k) (fv k') := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  all_goals fl_close

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (restart_partition_fl (by assumption)))

set_option maxHeartbeats 16000000 in
theorem apply_response_fl {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') : FL partition.val (fv k) (fv k') := by
  unfold logic.kernel.Kernel.apply_response at h
  peel h
  all_goals fl_close

macro_rules
  | `(tactic| fl_step) => `(tactic| refine FL.trans ?_ (apply_response_fl (by assumption)))


/-! ## What a stop and a restart set -/

theorem clear_partition_releases_dma {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') :
    k'.stop_dma.val[partition.val]! = true := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  peel h
  rename_i k1 hk1 a ha
  dsimp only
  exact set_self ha

theorem end_partition_jobs_dma {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition : U8}
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') :
    k'.stop_dma.val[partition.val]! = true := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  peel h
  all_goals exact clear_partition_releases_dma h

/-- **A stopped Partition is marked stopped and its EasyDMA stop
requested** (R14.5, R17.6). -/
theorem stop_partition_flags {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') :
    k'.stopped.val[partition.val]! = true ∧ k'.stop_dma.val[partition.val]! = true := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  rename_i k1 hp k2 he a ha
  dsimp only
  exact ⟨set_self ha, end_partition_jobs_dma he⟩

/-- **A restarted Partition has its re-initialization and its EasyDMA stop
requested and its MIT_Violation count at 0** (R14.5, R10.4). -/
theorem restart_partition_flags {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') :
    k'.reinit_partition.val[partition.val]! = true ∧ k'.stop_dma.val[partition.val]! = true ∧
    k'.mit_violations.val[partition.val]! = 0#u32 := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  rename_i k1 hp k2 he a ha a1 ha1
  dsimp only
  exact ⟨set_self ha1, end_partition_jobs_dma he, set_self ha⟩

/-! ## The responses -/

include hNT in
theorem stop_partition_ended {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') :
    CSteps cfg (fun u => ptOf cfg u = partition.val) k.sched k'.sched ∧
    (OnStack k.sched → ∀ u, u < NT.val → ptOf cfg u = partition.val →
      rel k'.sched u = 0 ∧ job k'.sched u ≠ 1) := by
  unfold logic.kernel.Kernel.stop_partition at h
  peel h
  rename_i k1 hp k2 he a ha
  dsimp only
  have := end_partition_jobs_ended he
  rw [push_event_sched hp] at this
  exact this

include hNT in
theorem restart_partition_ended {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition cause : U8}
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') :
    CSteps cfg (fun u => ptOf cfg u = partition.val) k.sched k'.sched ∧
    (OnStack k.sched → ∀ u, u < NT.val → ptOf cfg u = partition.val →
      rel k'.sched u = 0 ∧ job k'.sched u ≠ 1) := by
  unfold logic.kernel.Kernel.restart_partition at h
  peel h
  rename_i k1 hp k2 he a ha a1 ha1
  dsimp only
  have := end_partition_jobs_ended he
  rw [push_event_sched hp] at this
  exact this

set_option maxHeartbeats 2000000 in
/-- **What a response is** (`response_applied`'s cases): the state it was
given (record only, or END_JOB for a Job that is not on top); the Job on
top, the response's Task, ended through `Sched::finish_top` (END_JOB of a
Job holding no Resource); `restart_partition` (RESTART_PARTITION, or
END_JOB of a Job holding a Resource, R18.4); `stop_partition`; or the
system safe state. -/
theorem apply_response_cases {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') :
    k' = k ∨
    (∃ r, logic.sched.Sched.finish_top k.sched cfg = ok (r, k'.sched) ∧
      stk k.sched (k.sched.depth.val - 1) = t.val) ∨
    logic.kernel.Kernel.restart_partition k cfg partition cause = ok k' ∨
    logic.kernel.Kernel.stop_partition k cfg partition cause = ok k' ∨
    logic.kernel.Kernel.enter_safe_state k cfg = ok k' := by
  unfold logic.kernel.Kernel.apply_response at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (Or.inr (Or.inl h))
    | exact Or.inr (Or.inr (Or.inr (Or.inl h)))
    | exact Or.inr (Or.inr (Or.inr (Or.inr h)))
    | skip
  all_goals
    have hf := end_top_finish (by assumption)
    obtain ⟨r, hf⟩ := hf
    have hp := push_event_sched (by assumption)
    rw [hp] at hf
    try rw [end_mit_window_sched h]
    refine Or.inr (Or.inl ⟨r, hf, ?_⟩)
    have hsub : ∃ i, k.sched.depth - 1#usize = ok i ∧ ∃ top, k.sched.stack.index_usize i = ok top ∧
        top = t := ⟨_, by assumption, _, by assumption, by assumption⟩
    obtain ⟨i, hi, top, htop, htt⟩ := hsub
    have hiv := (usize_sub_ok hi).2
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at hiv
    obtain ⟨_, e⟩ := index_ok htop
    unfold stk
    rw [← hiv, ← e, htt]

include hNT in
/-- **A response is contained** (R14.9): unless it is the system safe
state, it changes the scheduler state by steps of the Tasks of its
Partition and of its Task, so (`csteps_cont`) every other Task's Release
and Job, its place on the stack and its lock entries are as they were. -/
theorem apply_response_contained {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') :
    CSteps cfg (fun u => ptOf cfg u = partition.val ∨ u = t.val) k.sched k'.sched ∨
    logic.kernel.Kernel.enter_safe_state k cfg = ok k' := by
  rcases apply_response_cases h with e | ⟨r, hf, ht⟩ | hr | hs | hsafe
  · rw [e]; exact Or.inl Relation.ReflTransGen.refl
  · exact Or.inl (Relation.ReflTransGen.single (CStep.kend _ _ r (Or.inr ht) hf))
  · exact Or.inl (CSteps.mono (fun _ hu => Or.inl hu) (restart_partition_ended hr).1)
  · exact Or.inl (CSteps.mono (fun _ hu => Or.inl hu) (stop_partition_ended hs).1)
  · exact Or.inr hsafe

/-! ## A fault: the event first, then the response -/

set_option maxHeartbeats 2000000 in
/-- **A fault's event is recorded first** (R14.3, R14.6; `fault_recorded`).
`Kernel::fault` pushes the fault's event on the state it finds: against no
Task and the Kernel (255, 255) when the fault is the Kernel's or no Job
executes, and then enters the safe state (R14.4); otherwise against the
Task on top and that Task's Partition, and then applies that Partition's
declared fault response for that Task. -/
theorem fault_first {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {kind : U8} {kernel : Bool}
    (h : logic.kernel.Kernel.fault k cfg kind kernel = ok k') :
    ∃ (prs : U64) (mid : logic.kernel.Kernel NT NR NP NL NS NE),
      ((kernel = true ∨ k.sched.depth.val = 0) ∧
        logic.kernel.Kernel.push_event k cfg kind 255#u8 255#u8 prs 0#u32 = ok mid ∧
        logic.kernel.Kernel.enter_safe_state mid cfg = ok k') ∨
      (∃ (t : U8) (tc : logic.TaskCfg) (pc : logic.PartitionCfg),
        kernel = false ∧ 0 < k.sched.depth.val ∧
        t.val = stk k.sched (k.sched.depth.val - 1) ∧
        cfg.tasks.val[t.val]? = some tc ∧ cfg.partitions.val[tc.partition.val]? = some pc ∧
        logic.kernel.Kernel.push_event k cfg kind t tc.partition prs 0#u32 = ok mid ∧
        logic.kernel.Kernel.apply_response mid cfg tc.partition pc.fault_response kind t = ok k') := by
  unfold logic.kernel.Kernel.fault at h
  peel h
  · exact ⟨_, _, Or.inl ⟨Or.inl (by assumption), by assumption, h⟩⟩
  · exact ⟨_, _, Or.inl ⟨Or.inr (by scalar_tac), by assumption, h⟩⟩
  · rename_i prs hprs hk hd i hi t ht tc htc mid hp pc hpc
    refine ⟨prs, mid, Or.inr ⟨t, tc, pc, by simpa using hk, by scalar_tac, ?_, ?_, ?_, hp, h⟩⟩
    · have hiv := (usize_sub_ok hi).2
      have h1 : (1#usize).val = 1 := rfl
      rw [h1] at hiv
      obtain ⟨_, e⟩ := index_ok ht
      unfold stk
      rw [← hiv, ← e]
    · obtain ⟨_, e⟩ := index_ok' htc
      rw [U8.cast_Usize_val_eq] at e
      exact e
    · obtain ⟨_, e⟩ := index_ok' hpc
      rw [U8.cast_Usize_val_eq] at e
      exact e

end P18
