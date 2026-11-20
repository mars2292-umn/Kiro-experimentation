import Sched.Entry
open Aeneas Aeneas.Std Result

/-! # The level record: what a dispatch saves is what the unwind returns (2026-11-19)

Property 2 of the design, the part between the scheduler state
(`Sched/Lifo.lean`) and the assembly stubs (`Sched/StubModel.lean`): the
PSP, CONTROL and EXC_RETURN of the preempted context are kept in a
per-level record. Stated here of Kernel_Arch's entry cores as Aeneas
extracts them, for any Kernel, hardware and tables behind their traits:

- `dispatch_stores_record`: a dispatch that starts a Job writes the
  preempted context it was called with into the record of the Job's level,
  marked active;
- `top_active_spec`: `top_active` returns the greatest active level;
- `epilogue_returns_record`: whatever path `epilogue_core` takes, the
  context it hands `rsk_unwind` is that of the record `top_active` names;
- `unwind_returns_dispatched_context`: so, when the record of the level is
  unchanged and no higher level is active, the unwind returns the PSP,
  CONTROL and EXC_RETURN the dispatch was entered with.

The two conditions of the last statement are what Verus's invariant
`entry::arch_inv` supplies at every entry (the active records are those of
the preemption stack, the last dispatched on top). That invariant is
stated over ghost fields of `ArchState` which the extraction erases, so it
is not restated here: in Lean the conditions stay hypotheses. -/

namespace P18
open rsk_kernel

deriving instance Inhabited for rsk_kernel.arch.cm4.plan.LevelRecord

/-- Whether the record of level `j` is active. -/
def act (levels : Array arch.cm4.plan.LevelRecord 8#usize) (j : Nat) : Bool :=
  (levels.val[j]!).active

theorem top_active_loop_ok (levels : Array arch.cm4.plan.LevelRecord 8#usize) :
    ∀ (n : Nat) (level i res : Usize), 8 - i.val = n → i.val ≤ 8 → level.val < i.val →
      (level.val ≠ 0 → act levels level.val = true) →
      (∀ j, level.val < j → j < i.val → act levels j = false) →
      arch.cm4.plan.top_active_loop levels level i = ok res →
      res.val < 8 ∧ (res.val ≠ 0 → act levels res.val = true) ∧
      (∀ j, res.val < j → j < 8 → act levels j = false) := by
  intro n
  induction n with
  | zero =>
    intro level i res hn hi8 hli hact hno h
    unfold arch.cm4.plan.top_active_loop loop at h
    simp only [arch.cm4.plan.top_active_loop.body] at h
    have hlt : ¬ i < 8#usize := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    exact ⟨by omega, hact, fun j h1 h2 => hno j h1 (by omega)⟩
  | succ n ih =>
    intro level i res hn hi8 hli hact hno h
    unfold arch.cm4.plan.top_active_loop loop at h
    simp only [arch.cm4.plan.top_active_loop.body] at h
    have hlt : i < 8#usize := by scalar_tac
    simp only [hlt, ite_true, lift, bind_ok, bind_eq_ok] at h
    obtain ⟨cf, ⟨lr, hlr, level1, hlevel1, i1, hi1, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    have hi1 := usize_add_ok hi1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at hi1
    obtain ⟨_, rfl⟩ := index_ok hlr
    have hrec : arch.cm4.plan.top_active_loop levels level1 i1 = ok res := by
      unfold arch.cm4.plan.top_active_loop
      simp only [arch.cm4.plan.top_active_loop.body, lift, bind_ok]
      exact hloop
    split at hlevel1
    · rename_i ha
      have hl := ok_inj hlevel1
      subst hl
      exact ih _ i1 res (by omega) (by omega) (by omega) (fun _ => ha)
        (fun j h1 h2 => by omega) hrec
    · rename_i ha
      have hl := ok_inj hlevel1
      subst hl
      refine ih _ i1 res (by omega) (by omega) (by omega) hact (fun j h1 h2 => ?_) hrec
      by_cases hj : j = i.val
      · subst hj
        simpa [act] using ha
      · exact hno j h1 (by omega)

/-- `top_active`: the greatest level 1 to 7 whose record is active, 0 when
none is. -/
theorem top_active_spec {levels : Array arch.cm4.plan.LevelRecord 8#usize} {l : Usize}
    (h : arch.cm4.plan.top_active levels = ok l) :
    l.val < 8 ∧ (l.val ≠ 0 → act levels l.val = true) ∧
    (∀ j, l.val < j → j < 8 → act levels j = false) :=
  top_active_loop_ok levels _ 0#usize 1#usize l rfl (by decide) (by decide)
    (fun hc => absurd rfl hc) (fun j h1 h2 => by
      have : (0#usize).val = 0 := rfl
      have : (1#usize).val = 1 := rfl
      omega) h

/-- With the record of level `L` active and none above it, `top_active`
names `L`. -/
theorem top_active_is {levels : Array arch.cm4.plan.LevelRecord 8#usize} {l : Usize} {L : Nat}
    (h : arch.cm4.plan.top_active levels = ok l) (hL0 : 0 < L) (hL8 : L < 8)
    (hact : act levels L = true) (hno : ∀ j, L < j → j < 8 → act levels j = false) :
    l.val = L := by
  obtain ⟨h8, ha, hn⟩ := top_active_spec h
  by_contra hne
  rcases Nat.lt_or_gt_of_ne hne with hlt | hgt
  · have := hn L hlt hL8
    rw [hact] at this
    cases this
  · have h1 := ha (by omega)
    have h2 := hno l.val hgt h8
    rw [h1] at h2
    cases h2

variable {K A T : Type}

set_option maxHeartbeats 8000000 in
/-- Whatever path `epilogue_core` takes, the PSP, CONTROL and EXC_RETURN it
returns are those of the record `top_active` names. -/
theorem epilogue_returns_record {ki : arch.bound.KernelOps K}
    {ai : arch.cm4.entry.KernelArch A} {ti : arch.cm4.entry.Tables T}
    {k k' : K} {hw hw' : A} {tables : T} {state state' : arch.cm4.plan.ArchState}
    {n p c e nx : U32} {safe : Bool}
    (h : arch.cm4.entry.epilogue_core ki ai ti k hw tables state n
      = ok ((((p, c, e, nx), safe), k', hw', state'))) :
    ∃ (level : Usize) (rc : arch.cm4.plan.LevelRecord),
      arch.cm4.plan.top_active state.levels = ok level ∧
      state.levels.index_usize level = ok rc ∧
      p = rc.psp ∧ c = rc.control ∧ e = rc.exc_return := by
  unfold arch.cm4.entry.epilogue_core at h
  peel h
  all_goals exact ⟨_, _, ‹_›, ‹_›, rfl, rfl, rfl⟩

theorem stored {v v' : Array arch.cm4.plan.LevelRecord 8#usize} {l : Usize}
    {x : arch.cm4.plan.LevelRecord} (h : v.update l x = ok v') :
    l.val < 8 ∧ v'.val[l.val]! = x := by
  obtain ⟨hl, rfl⟩ := update_ok h
  refine ⟨hl, ?_⟩
  rw [get_set _ _ _ _ hl]
  simp

set_option maxHeartbeats 8000000 in
/-- A dispatch that starts a Job (a plan with a frame) leaves, in the
record of some level, the preempted context it was called with: its PSP,
CONTROL and EXC_RETURN, the exception number and the Task, marked active. -/
theorem dispatch_stores_record {ki : arch.bound.KernelOps K}
    {ai : arch.cm4.entry.KernelArch A} {ti : arch.cm4.entry.Tables T}
    {k k' : K} {hw hw' : A} {tables : T} {state state' : arch.cm4.plan.ArchState}
    {n : U16} {t : U8} {psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (h : arch.cm4.entry.dispatch_job_core ki ai ti k hw tables state n t psp control exc_return
      = ok ((plan, safe), k', hw', state'))
    (hp : plan.psp ≠ 0#u32) :
    ∃ L : Nat, L < 8 ∧ (state'.levels.val[L]!).psp = psp ∧
      (state'.levels.val[L]!).control = control ∧
      (state'.levels.val[L]!).exc_return = exc_return ∧
      (state'.levels.val[L]!).exception_number = n ∧ (state'.levels.val[L]!).task = t ∧
      act state'.levels L = true := by
  unfold arch.cm4.entry.dispatch_job_core at h
  peel h
  all_goals first
    | exact absurd rfl hp
    | (obtain ⟨hl, he⟩ := stored ‹_›
       refine ⟨_, hl, ?_⟩
       simp only [act]
       rw [he]
       exact ⟨rfl, rfl, rfl, rfl, rfl, rfl⟩)

/-- **Property 2, the level record.** A dispatch saves the preempted
context in the record of a level `L`. When `epilogue_core` later runs in a
state whose record of level `L` is the one that dispatch left and in which
no higher level is active, it returns that context: the PSP, CONTROL and
EXC_RETURN the dispatch was entered with. -/
theorem unwind_returns_dispatched_context {ki : arch.bound.KernelOps K}
    {ai : arch.cm4.entry.KernelArch A} {ti : arch.cm4.entry.Tables T}
    {k2 k3 : K} {hw2 hw3 : A} {tables : T} {state1 state2 state3 : arch.cm4.plan.ArchState}
    {m p c e nx psp control exc_return : U32} {safe : Bool} {L : Nat}
    (hL0 : 0 < L) (hL8 : L < 8)
    (hrec : (state1.levels.val[L]!).psp = psp ∧ (state1.levels.val[L]!).control = control ∧
      (state1.levels.val[L]!).exc_return = exc_return ∧ act state1.levels L = true)
    (hkept : state2.levels.val[L]! = state1.levels.val[L]!)
    (htop : ∀ j, L < j → j < 8 → act state2.levels j = false)
    (he : arch.cm4.entry.epilogue_core ki ai ti k2 hw2 tables state2 m
      = ok ((((p, c, e, nx), safe), k3, hw3, state3))) :
    p = psp ∧ c = control ∧ e = exc_return := by
  obtain ⟨level, rc, htl, hrc, rfl, rfl, rfl⟩ := epilogue_returns_record he
  obtain ⟨r1, r2, r3, r4⟩ := hrec
  have hact2 : act state2.levels L = true := by
    simp only [act] at r4 ⊢
    rw [hkept]; exact r4
  have hl := top_active_is htl hL0 hL8 hact2 htop
  obtain ⟨_, rfl⟩ := index_ok hrc
  rw [hl, hkept]
  exact ⟨r1, r2, r3⟩

end P18
