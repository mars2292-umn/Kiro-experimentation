import Sched.Mutex
open Aeneas Aeneas.Std Result

/-! # The start condition and the forced end on the Kernel's code (2026-11-19)

Properties 1 and 6 of the design, which Verus proves of `rsk_logic`,
stated over the Kernel's functions as Aeneas extracts them and the run of
`Sched/KRun.lean`.

- Property 1: the Task the Kernel dispatches is the one `Sched::select`
  returns, which is eligible (released, no Job, a Priority above the
  System_Ceiling and above every Job in progress) and of the highest
  Priority among the eligible ones (`dispatch_starts_selected`); when
  `select` returns none no Task is eligible (`none_elig`, `Sched/KRun.lean`);
  and in the run every Kernel run is (`ksim`), a Job starts by no other
  transition (`start_only_eligible`).
- Property 6: the end of the Job on top of the stack (`Sched::finish_top`:
  its return, a response that ends it, the unwinding of a killed Job)
  removes from the lock stack exactly that Job's entries, leaves the others
  as they were with their holders on the stack, and the System_Ceiling is
  the greatest Ceiling of what remains (`finish_top_releases`, and for
  `Kernel::job_return` and `Kernel::end_top` in a reached state). A Job
  killed below the top keeps its entries until it is unwound
  (`kill_keeps_locks`). That each released Resource is reported is not
  stated here. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! ## Property 1 -/

/-- The Task dispatched in a reached state, the one `Sched::select`
returns, is eligible, of the highest Priority among the eligible Tasks,
above the System_Ceiling and above every Job in progress; after the
dispatch its Job is in progress on top of the stack. -/
theorem dispatch_starts_selected {cfg : logic.Config NT NR NP} {P : Params} (hNT : NT.val ≤ 256)
    {x : KSt NT NR NP NL NS NE} (hx : KReach cfg P x) {t : U8} {pc : U32}
    {k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (hsel : logic.sched.Sched.select x.k.sched cfg = ok (some t))
    (hd : logic.kernel.Kernel.dispatch x.k cfg t pc = ok k') :
    Elig cfg x.k.sched t.val ∧
    (∀ u, Elig cfg x.k.sched u → prio cfg u ≤ prio cfg t.val) ∧
    x.k.sched.ceiling.val < prio cfg t.val ∧
    (∀ p, p < x.k.sched.depth.val → prio cfg (stk x.k.sched p) < prio cfg t.val) ∧
    job k'.sched t.val = 1 ∧ stk k'.sched (k'.sched.depth.val - 1) = t.val := by
  have w : Wf cfg P x.st := reach_wf (ksim hNT hx)
  obtain ⟨he, hmax⟩ := select_some hNT hsel
  obtain ⟨_, _, _, hc, htop⟩ := elig_views.mp he
  obtain ⟨_, _, _, hjob, hstk, hdep, _⟩ := start_views (dispatch_eff hd)
  refine ⟨he, hmax, hc, ?_, ?_, ?_⟩
  · intro p hp
    rcases htop with h0 | h1
    · omega
    · by_cases hpt : p = x.k.sched.depth.val - 1
      · rw [hpt]; exact h1
      · have hsrt := w.sorted p (x.k.sched.depth.val - 1) (by omega)
          (show x.k.sched.depth.val - 1 < x.k.sched.depth.val by omega)
        exact Nat.lt_trans hsrt h1
  · rw [hjob]; simp
  · rw [hdep, hstk]; simp

theorem job_make_pending {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {t : U8}
    {k : Nat} (h : logic.sched.Sched.make_pending s cfg t = ok s') : job s' k = job s k :=
  (make_pending_views h).2.2.1 k

theorem job_defer {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {t : U8}
    {k : Nat} (h : logic.sched.Sched.defer s cfg t = ok s') : job s' k = job s k :=
  (defer_views h).2.2.1 k

theorem job_clear {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {t : U8}
    {k : Nat} (h : logic.sched.Sched.clear_release s cfg t = ok s') : job s' k = job s k :=
  (clear_release_views h).2.2.1 k

theorem job_lock {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    {k : Nat} (h : logic.sched.Sched.lock s cfg r = ok (true, s')) : job s' k = job s k :=
  (lock_views h).1.2.1 k

theorem job_unlock {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    {k : Nat} (h : logic.sched.Sched.unlock s cfg r = ok (true, s')) : job s' k = job s k :=
  (unlock_views h).1.2.1 k

/-- The end of the Job on top and the killing of a Job make no Job
in progress that was not. -/
theorem job_finish_top {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {r : Usize} {k : Nat} (h : logic.sched.Sched.finish_top s cfg = ok (r, s'))
    (h1 : job s' k = 1) : job s k = 1 := by
  have hj := (finish_top_views h).2.2.2.2.1 k
  rw [hj] at h1
  split at h1
  · omega
  · exact h1

theorem job_kill {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {i : Usize} {k : Nat} (h : logic.sched.Sched.kill s cfg i = ok s')
    (h1 : job s' k = 1) : job s k = 1 := by
  have hj := (kill_views h).2.2.1 k
  rw [hj] at h1
  split at h1
  · omega
  · exact h1

/-- In the run, a Job comes to be in progress by the dispatch transition
alone, and that transition starts an eligible Task of the highest Priority
among the eligible ones. -/
theorem start_only_eligible {cfg : logic.Config NT NR NP} {P : Params} {st st' : St NT NR NL}
    (h : Step cfg P st st') {k : Nat} (hne : job st.s k ≠ 1) (h1 : job st'.s k = 1) :
    Elig cfg st.s k ∧ ∀ u, Elig cfg st.s u → prio cfg u ≤ prio cfg k := by
  cases h with
  | dispatch t c s' hk he hmax hc hs =>
    dsimp only at h1
    have hj := (start_views hs).2.2.2.1 k
    rw [hj] at h1
    by_cases hkt : k = t.val
    · rw [hkt]; exact ⟨he, hmax⟩
    · simp only [hkt, ite_false] at h1
      exact absurd h1 hne
  | _ =>
    exfalso
    try dsimp only at h1
    first
      | exact hne h1
      | exact hne (by rw [← job_make_pending (by assumption)]; exact h1)
      | exact hne (by rw [← job_defer (by assumption)]; exact h1)
      | exact hne (by rw [← job_clear (by assumption)]; exact h1)
      | exact hne (by rw [← job_lock (by assumption)]; exact h1)
      | exact hne (by rw [← job_unlock (by assumption)]; exact h1)
      | exact hne (job_finish_top (by assumption) h1)
      | exact hne (job_kill (by assumption) h1)

/-! ## Property 6 -/

/-- The end of the Job on top of the stack releases every lock it held and
no other: the entries removed from the lock stack are that Job's, every
entry that remains is as it was and its holder is still on the stack, and
the System_Ceiling is the greatest Ceiling of the Resources still held. -/
theorem finish_top_releases {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) {r : Usize} {s' : logic.sched.Sched NT NR NL}
    (h : logic.sched.Sched.finish_top st.s cfg = ok (r, s')) :
    s'.lock_len.val ≤ st.s.lock_len.val ∧
    (∀ p, s'.lock_len.val ≤ p → p < st.s.lock_len.val → ldep st.s p = st.s.depth.val - 1) ∧
    (∀ p, p < s'.lock_len.val →
      lres s' p = lres st.s p ∧ ldep s' p = ldep st.s p ∧ ldep s' p < s'.depth.val) ∧
    IsCeil cfg s' s'.lock_len.val s'.ceiling.val := by
  obtain ⟨hd1, _, _, _, _, _, hdep, hlres, hldep, hlen, hown, hlast, hc⟩ := finish_top_views h
  refine ⟨hlen, hown, ?_, IsCeil.congr (fun p _ => hlres p) hc⟩
  intro p hp
  refine ⟨hlres p, hldep p, ?_⟩
  rw [hldep p, hdep]
  rcases hlast with h0 | hne
  · omega
  · have h1 := w.ldep_mono p (s'.lock_len.val - 1) (by omega) (by omega)
    have h2 := w.ldep_lt (s'.lock_len.val - 1) (by omega)
    omega

/-- The statement of `finish_top_releases` about the lock stacks before
and after. -/
def Released (cfg : logic.Config NT NR NP) (s s' : logic.sched.Sched NT NR NL) : Prop :=
  s'.lock_len.val ≤ s.lock_len.val ∧
  (∀ p, s'.lock_len.val ≤ p → p < s.lock_len.val → ldep s p = s.depth.val - 1) ∧
  (∀ p, p < s'.lock_len.val →
    lres s' p = lres s p ∧ ldep s' p = ldep s p ∧ ldep s' p < s'.depth.val) ∧
  IsCeil cfg s' s'.lock_len.val s'.ceiling.val

/-- Property 6 for a Job that returns, in a reached state. -/
theorem job_return_releases {cfg : logic.Config NT NR NP} {P : Params} (hNT : NT.val ≤ 256)
    {x : KSt NT NR NP NL NS NE} (hx : KReach cfg P x) {ec : U32}
    {k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.job_return x.k cfg ec = ok k') : Released cfg x.k.sched k'.sched := by
  obtain ⟨r, hf⟩ := job_return_eff h
  exact finish_top_releases (reach_wf (ksim hNT hx)) hf

/-- Property 6 for a Job the Health_Monitor ends, in a reached state. -/
theorem end_top_releases {cfg : logic.Config NT NR NP} {P : Params} (hNT : NT.val ≤ 256)
    {x : KSt NT NR NP NL NS NE} (hx : KReach cfg P x)
    {k' : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : logic.kernel.Kernel.end_top x.k cfg = ok k') : Released cfg x.k.sched k'.sched := by
  obtain ⟨r, hf⟩ := end_top_finish h
  exact finish_top_releases (reach_wf (ksim hNT hx)) hf

/-- A Job killed below the top of the stack (a Partition's stop or
restart) keeps its entries of the lock stack and the System_Ceiling stays:
they are released when the Job is unwound, by `Sched::finish_top`. -/
theorem kill_keeps_locks {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {i : Usize} (h : logic.sched.Sched.kill s cfg i = ok s') : LockSame s' s :=
  (kill_views h).2.2.2.2.2

end P18
