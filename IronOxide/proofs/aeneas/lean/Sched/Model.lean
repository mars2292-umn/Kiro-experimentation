import Aeneas
import Sched.Sched
import Sched.Eligible
open Aeneas Aeneas.Std Result

namespace P18
open rsk_logic

theorem bind_eq_ok {α β} (x : Result α) (f : α → Result β) (y : β) :
    Std.bind x f = ok y ↔ ∃ a, x = ok a ∧ f a = ok y := by
  cases x <;> simp

theorem ok_inj {α} {x y : α} (h : (ok x : Result α) = ok y) : x = y := by
  simpa using h

theorem usize_add_ok {a b c : Usize} (h : a + b = ok c) : c.val = a.val + b.val := by
  have := UScalar.add_equiv a b
  simp_all

theorem usize_sub_ok {a b c : Usize} (h : a - b = ok c) : b.val ≤ a.val ∧ c.val = a.val - b.val := by
  have := UScalar.sub_equiv a b
  simp_all

theorem update_ok {α} {n : Usize} {v v' : Array α n} {i : Usize} {x : α}
    (h : v.update i x = ok v') : i.val < n.val ∧ v' = v.set i x := by
  unfold Array.update at h
  split at h
  · simp at h
  · rename_i y hy
    simp only [Array.getElem?_Usize_eq] at hy
    have hl := List.getElem?_eq_some_iff.mp hy
    obtain ⟨hl, _⟩ := hl
    have := v.property
    simp at h
    constructor
    · omega
    · rw [← h]; rfl

theorem index_ok {α} [Inhabited α] {n : Usize} {v : Array α n} {i : Usize} {x : α}
    (h : v.index_usize i = ok x) : i.val < n.val ∧ x = v.val[i.val]! := by
  unfold Array.index_usize at h
  split at h
  · simp at h
  · rename_i y hy
    simp only [Array.getElem?_Usize_eq] at hy
    have hl := List.getElem?_eq_some_iff.mp hy
    obtain ⟨hl, he⟩ := hl
    have := v.property
    simp at h
    constructor
    · omega
    · subst h; simp [hy]

variable {NT NR NL NP NS NE : Usize}

/-- The Priority of Task `k` in the Kernel's configuration (0 out of range). -/
def prio (cfg : logic.Config NT NR NP) (k : Nat) : Nat :=
  match cfg.tasks.val[k]? with
  | some tc => tc.priority.val
  | none => 0

theorem make_pending_ok {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.sched.Sched.make_pending s cfg t = ok s') :
    t.val < NT.val ∧
    s' = { s with release := s.release.set (UScalar.cast .Usize t) logic.sched.REL_PENDING } := by
  simp only [logic.sched.Sched.make_pending, lift, bind_ok, bind_eq_ok] at h
  obtain ⟨a, ha, h⟩ := h
  obtain ⟨hb, rfl⟩ := update_ok ha
  simp at h
  simp at hb
  exact ⟨hb, h.symm⟩

theorem start_ok {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {t : U8}
    (h : logic.sched.Sched.start s cfg t = ok s') :
    t.val < NT.val ∧ s.depth.val < NT.val ∧ ∃ d : Usize, d.val = s.depth.val + 1 ∧
    s' = { s with
      release := s.release.set (UScalar.cast .Usize t) logic.sched.REL_NONE,
      job := s.job.set (UScalar.cast .Usize t) logic.sched.JOB_RUNNING,
      stack := s.stack.set s.depth t,
      depth := d,
      start_ceiling := s.start_ceiling.set s.depth s.ceiling } := by
  simp only [logic.sched.Sched.start, lift, bind_ok, bind_eq_ok] at h
  obtain ⟨a, ha, a1, ha1, a2, ha2, a3, ha3, d, hd, h⟩ := h
  obtain ⟨hb, rfl⟩ := update_ok ha
  obtain ⟨hb1, rfl⟩ := update_ok ha1
  obtain ⟨hb2, rfl⟩ := update_ok ha2
  obtain ⟨hb3, rfl⟩ := update_ok ha3
  have hd := usize_add_ok hd
  simp at h
  simp at hb2
  exact ⟨hb2, hb, d, by simpa using hd, h.symm⟩

theorem complete_ok {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    (h : logic.sched.Sched.complete s cfg = ok s') :
    1 ≤ s.depth.val ∧ s.depth.val ≤ NT.val ∧ (s.stack.val[s.depth.val - 1]!).val < NT.val ∧
    ∃ d : Usize, d.val = s.depth.val - 1 ∧
    s' = { s with
      job := s.job.set (UScalar.cast .Usize (s.stack.val[s.depth.val - 1]!)) logic.sched.JOB_NONE,
      depth := d } := by
  simp only [logic.sched.Sched.complete, lift, bind_ok, bind_eq_ok] at h
  obtain ⟨d, hd, t, ht, a, ha, h⟩ := h
  obtain ⟨hd1, hd2⟩ := usize_sub_ok hd
  obtain ⟨hb, rfl⟩ := index_ok ht
  obtain ⟨hb1, rfl⟩ := update_ok ha
  simp at hd1 hd2
  rw [U8.cast_Usize_val_eq, hd2] at hb1
  rw [hd2] at hb
  have h := ok_inj h
  refine ⟨hd1, by omega, hb1, d, hd2, ?_⟩
  rw [← h, hd2]

theorem end_job_loop_zero (a : Array Usize NL) (d : Usize) :
    logic.sched.Sched.end_job_loop a d 0#usize = ok 0#usize := by
  unfold logic.sched.Sched.end_job_loop loop
  simp [logic.sched.Sched.end_job_loop.body]

theorem recompute_zero (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP) :
    logic.sched.Sched.recompute_ceiling s cfg 0#usize = ok logic.IDLE_CEILING := by
  unfold logic.sched.Sched.recompute_ceiling logic.sched.Sched.recompute_ceiling_loop loop
  simp [logic.sched.Sched.recompute_ceiling_loop.body]

/-- Job end with no Resource held (the first stage has no lock). -/
theorem finish_top_ok0 {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : Usize}
    (hl : s.lock_len = 0#usize)
    (h : logic.sched.Sched.finish_top s cfg = ok (r, s')) :
    1 ≤ s.depth.val ∧ s.depth.val ≤ NT.val ∧ (s.stack.val[s.depth.val - 1]!).val < NT.val ∧
    ∃ d : Usize, d.val = s.depth.val - 1 ∧
    s' = { s with
      job := s.job.set (UScalar.cast .Usize (s.stack.val[s.depth.val - 1]!)) logic.sched.JOB_NONE,
      depth := d, ceiling := logic.IDLE_CEILING } := by
  simp only [logic.sched.Sched.finish_top, logic.sched.Sched.end_job, hl, end_job_loop_zero,
    recompute_zero, bind_ok, bind_eq_ok] at h
  obtain ⟨d, hd, rel, hrel, s1, hs1, h⟩ := h
  obtain ⟨h1, h2, h3, d', hd', rfl⟩ := complete_ok hs1
  have h := ok_inj h
  simp only [Prod.mk.injEq] at h
  refine ⟨h1, h2, h3, d', hd', ?_⟩
  rw [← h.2, hl]

theorem index_ok' {α} {n : Usize} {v : Array α n} {i : Usize} {x : α}
    (h : v.index_usize i = ok x) : i.val < n.val ∧ v.val[i.val]? = some x := by
  unfold Array.index_usize at h
  split at h
  · simp at h
  · rename_i y hy
    simp only [Array.getElem?_Usize_eq] at hy
    have hl := (List.getElem?_eq_some_iff.mp hy).1
    have := v.property
    have h := ok_inj h
    subst h
    exact ⟨by omega, hy⟩

/-- The eligibility translation has its own copy of the Kernel's types
(it is extracted from another crate); these read one as the other. -/
def schedOf (ks : _root_.rsk_kernel.rsk_logic.logic.sched.Sched NT NR NL) :
    logic.sched.Sched NT NR NL :=
  { release := ks.release, job := ks.job, stack := ks.stack, depth := ks.depth,
    start_ceiling := ks.start_ceiling, ceiling := ks.ceiling, lock_res := ks.lock_res,
    lock_depth := ks.lock_depth, lock_len := ks.lock_len }

def taskOf (tc : _root_.rsk_kernel.rsk_logic.logic.TaskCfg) : logic.TaskCfg :=
  { priority := tc.priority, partition := tc.partition, kind := tc.kind, offset := tc.offset,
    period := tc.period, deadline := tc.deadline, budget := tc.budget,
    mit_policy := tc.mit_policy, fpu := tc.fpu }

/-- The Kernel object `b` (`Bound`: configuration and Kernel state) holds
the configuration's Tasks `cfg.tasks` and the scheduler state `s`. -/
def Matches (b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE)
    (cfg : logic.Config NT NR NP) (s : logic.sched.Sched NT NR NL) : Prop :=
  schedOf b.kernel.sched = s ∧ b.cfg.tasks.val.map taskOf = cfg.tasks.val

/-- R7.1 as a statement: Task `t` is pending, has no Job in progress, and
its Priority is above the System_Ceiling and above that of the Job on top
of the stack. -/
def Elig (cfg : logic.Config NT NR NP) (s : logic.sched.Sched NT NR NL) (t : Nat) : Prop :=
  t < NT.val ∧ (s.release.val[t]!).val = 1 ∧ (s.job.val[t]!).val = 0 ∧
  s.ceiling.val < prio cfg t ∧
  (s.depth.val = 0 ∨ prio cfg (s.stack.val[s.depth.val - 1]!).val < prio cfg t)

theorem prio_of_match {b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {s : logic.sched.Sched NT NR NL} (hm : Matches b cfg s)
    {k : Nat} {tc : _root_.rsk_kernel.rsk_logic.logic.TaskCfg}
    (h : b.cfg.tasks.val[k]? = some tc) : prio cfg k = tc.priority.val := by
  have : cfg.tasks.val[k]? = some (taskOf tc) := by
    rw [← hm.2, List.getElem?_map, h]; rfl
  simp [prio, this, taskOf]

/-- What the executed eligibility test returns, when it returns. -/
theorem eligible_ok {b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {s : logic.sched.Sched NT NR NL} (hm : Matches b cfg s)
    {t : U8} {e : Bool}
    (h : _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.eligible b t = ok e) :
    e = true ↔ Elig cfg s t.val := by
  have hs := hm.1
  simp only [_root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.eligible,
    lift, bind_ok] at h
  split at h
  · rename_i hge
    have h := ok_inj h
    subst h
    simp [Elig]
    intro hlt
    scalar_tac
  · rename_i hlt
    simp only [bind_eq_ok] at h
    obtain ⟨tc, htc, r, hr, h⟩ := h
    obtain ⟨hb, htc'⟩ := index_ok' htc
    obtain ⟨hb2, rfl⟩ := index_ok hr
    rw [U8.cast_Usize_val_eq] at htc' hb h
    have hp := prio_of_match hm htc'
    subst hs
    have hrp : _root_.rsk_kernel.rsk_logic.logic.sched.REL_PENDING = 1#u8 := by
      unfold _root_.rsk_kernel.rsk_logic.logic.sched.REL_PENDING; rfl
    have hjn : _root_.rsk_kernel.rsk_logic.logic.sched.JOB_NONE = 0#u8 := by
      unfold _root_.rsk_kernel.rsk_logic.logic.sched.JOB_NONE; rfl
    rw [hrp, hjn] at h
    split at h
    · rename_i hrel
      simp only [bind_eq_ok] at h
      obtain ⟨j, hj, h⟩ := h
      obtain ⟨_, rfl⟩ := index_ok hj
      rw [U8.cast_Usize_val_eq] at h
      split at h
      · rename_i hjob
        split at h
        · rename_i hceil
          split at h
          · rename_i hd0
            have h := ok_inj h
            subst h
            have hc : b.kernel.sched.ceiling.val < tc.priority.val := by scalar_tac
            have h0 : b.kernel.sched.depth.val = 0 := by rw [hd0]; rfl
            simp only [Elig, schedOf, hp, hrel, hjob]
            simp [hb, hc, h0]
          · rename_i hd0
            simp only [bind_eq_ok] at h
            obtain ⟨d, hd, top, htop, tc1, htc1, h⟩ := h
            obtain ⟨hd1, hd2⟩ := usize_sub_ok hd
            obtain ⟨_, rfl⟩ := index_ok htop
            obtain ⟨_, htc1'⟩ := index_ok' htc1
            have h1 : (1#usize).val = 1 := by rfl
            rw [U8.cast_Usize_val_eq, hd2, h1] at htc1'
            have hp1 := prio_of_match hm htc1'
            have h := ok_inj h
            subst h
            have hc : b.kernel.sched.ceiling.val < tc.priority.val := by scalar_tac
            have hne : b.kernel.sched.depth.val ≠ 0 := by rw [h1] at hd1; omega
            simp only [Elig, schedOf, hp, hrel, hjob, hp1]
            have h8 : (1#u8).val = 1 := by rfl
            have h9 : (0#u8).val = 0 := by rfl
            simp only [h8, h9, hb, hc, hne, true_and, false_or, decide_eq_true_eq]
            constructor <;> intro hx <;> scalar_tac
        · rename_i hceil
          have h := ok_inj h
          subst h
          have hc : ¬ b.kernel.sched.ceiling.val < tc.priority.val := by scalar_tac
          simp only [Elig, schedOf, hp]
          constructor
          · intro hx; cases hx
          · rintro ⟨_, _, _, h4, _⟩; exact absurd h4 hc
      · rename_i hjob
        have h := ok_inj h
        subst h
        simp only [Elig, schedOf]
        constructor
        · intro hx; cases hx
        · rintro ⟨_, _, h3, _⟩
          exact absurd (by scalar_tac) hjob
    · rename_i hrel
      have h := ok_inj h
      subst h
      simp only [Elig, schedOf]
      constructor
      · intro hx; cases hx
      · rintro ⟨_, h2, _⟩
        exact absurd (by scalar_tac) hrel

theorem rel_pending : logic.sched.REL_PENDING = 1#u8 := by
  unfold logic.sched.REL_PENDING; rfl
theorem rel_none : logic.sched.REL_NONE = 0#u8 := by
  unfold logic.sched.REL_NONE; rfl
theorem job_none : logic.sched.JOB_NONE = 0#u8 := by
  unfold logic.sched.JOB_NONE; rfl
theorem job_running : logic.sched.JOB_RUNNING = 1#u8 := by
  unfold logic.sched.JOB_RUNNING; rfl
theorem idle_ceiling : logic.IDLE_CEILING = 0#u8 := by
  unfold logic.IDLE_CEILING; rfl

end P18
