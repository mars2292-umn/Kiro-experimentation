/-
  The Analyzer's input check and its Endpoint terms on the code as compiled
  (2026-11-26): `terms::check_inputs`, `terms::endpoint_sites` and
  `terms::endpoint_cost_add` of `crates/rsk-analyzer/src/terms.rs`, as
  `Rta/Rta.lean` (the generated translation) has them. Handwritten.

  - `check_inputs_spec`: `check_inputs` returns for every input (no panic,
    no index out of bounds, no overflow, every loop ends); it answers
    `None` exactly when the inputs are admissible (`inputsAdmissible`), and
    a refusal names an input that is not: the rule's number and the index
    of a site, Task or window that breaks it, the first that does.
  - `checked_inputs`: what an accepted input has, in the form in which
    Property 18 (`Sched/Link.lean`, `Sched/KRun.lean`) takes it as
    hypotheses (`hsok`, `hwl`, `hbud`, `hDT` and the positive Priority of
    `hprio`).
  - `endpoint_sites_spec`, `endpoint_site_blocks`, `endpoint_site_in_b`,
    `endpoint_site_in_derived`, `endpoint_site_no_lock_cost`,
    `endpoint_cost_add_spec`, `endpoint_cost_in_derived`: R40.2 (task
    21.5), the Endpoint operations as Critical_Section sites and their
    cost.

  `inputsAdmissible`, `taskAdmissible`, `windowAdmissible`, `endpointSite`
  and `endpointOp` restate `inputs_admissible`, `task_admissible`,
  `window_admissible`, `endpoint_site` and `endpoint_op` of `terms.rs` (the
  predicates of the Verus contracts) over the translation's types; that
  the two texts say the same is by reading them. `inputsAdmissible` ranges
  over the members of a list where the Verus predicate ranges over the
  indices of a sequence.
-/
import Aeneas
import Rta.Rta
import Rta.Proofs
import Rta.Chain
import Rta.Derive
open Aeneas Aeneas.Std Result

namespace rsk_analyzer

/-- A total specification gives a result. -/
theorem spec_returns {α} {x : Result α} {p : α → Prop} (h : x ⦃ p ⦄) :
    ∃ r, x = ok r ∧ p r := by
  cases x with
  | ret v => exact ⟨v, rfl, (WP.spec_ok v).mp h⟩
  | vis e k => exact absurd h (by simp [WP.spec_vis])
  | div => exact absurd h (by simp [WP.spec_div])

/-! ## The admissibility predicate (`inputs_admissible` of `terms.rs`) -/

/-- A Task has a positive Budget and a Priority above the idle level. -/
def taskAdmissible (t : terms.TaskIn) : Prop := 0 < t.budget.val ∧ 0 < t.priority.val

/-- A deadline is not later than its period or MIT. -/
def windowAdmissible (w : terms.Window) : Prop := w.deadline.val ≤ w.period.val

/-- Every site names a Task of the set, there is one window per Task,
every Task is admissible and every window is. -/
def inputsAdmissible (tasks : List terms.TaskIn) (sites : List terms.Site)
    (windows : List terms.Window) : Prop :=
  (∀ s ∈ sites, s.task.val < tasks.length)
  ∧ windows.length = tasks.length
  ∧ (∀ t ∈ tasks, taskAdmissible t)
  ∧ (∀ w ∈ windows, windowAdmissible w)

/-! ## The three loops -/

/-- `first_stray_site` returns: `None` when every site names a Task below
`n`, otherwise the index of the first site that does not. -/
theorem first_stray_site_spec (sites : alloc.vec.Vec terms.Site) (n : Usize) :
    terms.first_stray_site sites n ⦃ r =>
      match r with
      | none => ∀ s ∈ sites.val, s.task.val < n.val
      | some k => (∃ s, sites.val[k.val]? = some s ∧ n.val ≤ s.task.val)
          ∧ ∀ m s, m < k.val → sites.val[m]? = some s → s.task.val < n.val ⦄ := by
  unfold terms.first_stray_site terms.first_stray_site_loop
  apply loop.spec_decr_nat
    (fun (k : Usize) => sites.val.length - k.val)
    (fun k => k.val ≤ sites.val.length
      ∧ ∀ m s, m < k.val → sites.val[m]? = some s → s.task.val < n.val)
  · intro k hk
    obtain ⟨h1, h2⟩ := hk
    unfold terms.first_stray_site_loop.body
    by_cases hlt : k < alloc.vec.Vec.len sites
    · simp only [hlt, ↓reduceIte]
      have hlt' : k.val < sites.val.length := by scalar_tac
      step as ⟨ s, hs ⟩
      have hgs : sites.val[k.val]? = some s := by
        rw [hs]; exact List.getElem?_eq_getElem hlt'
      by_cases hge : s.task ≥ n
      · simp only [hge, ↓reduceIte, WP.spec_ok]
        exact ⟨⟨s, hgs, by scalar_tac⟩, h2⟩
      · simp only [hge, ↓reduceIte]
        step as ⟨ k1, hk1 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro m s' hm hms
        by_cases hmk : m < k.val
        · exact h2 m s' hmk hms
        · have : m = k.val := by scalar_tac
          subst this
          rw [hgs] at hms
          simp at hms
          subst hms
          scalar_tac
    · simp only [hlt, ↓reduceIte, WP.spec_ok]
      intro s hs
      obtain ⟨m, hm, hms⟩ := List.getElem_of_mem hs
      exact h2 m s (by scalar_tac) (by rw [← hms]; exact List.getElem?_eq_getElem hm)
  · exact ⟨by scalar_tac, by intro m s hm; exact absurd hm (by scalar_tac)⟩

/-- `first_unfit_task` returns: `None` when every Task is admissible,
otherwise the index of the first that is not. -/
theorem first_unfit_task_spec (tasks : alloc.vec.Vec terms.TaskIn) :
    terms.first_unfit_task tasks ⦃ r =>
      match r with
      | none => ∀ t ∈ tasks.val, taskAdmissible t
      | some i => (∃ t, tasks.val[i.val]? = some t ∧ ¬ taskAdmissible t)
          ∧ ∀ m t, m < i.val → tasks.val[m]? = some t → taskAdmissible t ⦄ := by
  unfold terms.first_unfit_task terms.first_unfit_task_loop
  apply loop.spec_decr_nat
    (fun (i : Usize) => tasks.val.length - i.val)
    (fun i => i.val ≤ tasks.val.length
      ∧ ∀ m t, m < i.val → tasks.val[m]? = some t → taskAdmissible t)
  · intro i hi
    obtain ⟨h1, h2⟩ := hi
    unfold terms.first_unfit_task_loop.body
    by_cases hlt : i < alloc.vec.Vec.len tasks
    · simp only [hlt, ↓reduceIte]
      have hlt' : i.val < tasks.val.length := by scalar_tac
      step as ⟨ t, ht ⟩
      have hgt : tasks.val[i.val]? = some t := by
        rw [ht]; exact List.getElem?_eq_getElem hlt'
      by_cases hb : t.budget = 0#u64
      · simp only [hb, ↓reduceIte, WP.spec_ok]
        refine ⟨⟨t, hgt, ?_⟩, h2⟩
        intro hadm
        have := hadm.1
        rw [hb] at this
        simp at this
      · simp only [hb, ↓reduceIte]
        by_cases hp : t.priority = 0#u8
        · simp only [hp, ↓reduceIte, WP.spec_ok]
          refine ⟨⟨t, hgt, ?_⟩, h2⟩
          intro hadm
          have := hadm.2
          rw [hp] at this
          simp at this
        · simp only [hp, ↓reduceIte]
          step as ⟨ i1, hi1 ⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          intro m t' hm hmt
          by_cases hmi : m < i.val
          · exact h2 m t' hmi hmt
          · have : m = i.val := by scalar_tac
            subst this
            rw [hgt] at hmt
            simp at hmt
            subst hmt
            constructor
            · have : t.budget.val ≠ 0 := by
                intro h0; apply hb; scalar_tac
              omega
            · have : t.priority.val ≠ 0 := by
                intro h0; apply hp; scalar_tac
              omega
    · simp only [hlt, ↓reduceIte, WP.spec_ok]
      intro t ht
      obtain ⟨m, hm, hmt⟩ := List.getElem_of_mem ht
      exact h2 m t (by scalar_tac) (by rw [← hmt]; exact List.getElem?_eq_getElem hm)
  · exact ⟨by scalar_tac, by intro m t hm; exact absurd hm (by scalar_tac)⟩

/-- `first_late_window` returns: `None` when no deadline is later than its
period or MIT, otherwise the index of the first window whose deadline is. -/
theorem first_late_window_spec (windows : alloc.vec.Vec terms.Window) :
    terms.first_late_window windows ⦃ r =>
      match r with
      | none => ∀ w ∈ windows.val, windowAdmissible w
      | some j => (∃ w, windows.val[j.val]? = some w ∧ ¬ windowAdmissible w)
          ∧ ∀ m w, m < j.val → windows.val[m]? = some w → windowAdmissible w ⦄ := by
  unfold terms.first_late_window terms.first_late_window_loop
  apply loop.spec_decr_nat
    (fun (j : Usize) => windows.val.length - j.val)
    (fun j => j.val ≤ windows.val.length
      ∧ ∀ m w, m < j.val → windows.val[m]? = some w → windowAdmissible w)
  · intro j hj
    obtain ⟨h1, h2⟩ := hj
    unfold terms.first_late_window_loop.body
    by_cases hlt : j < alloc.vec.Vec.len windows
    · simp only [hlt, ↓reduceIte]
      have hlt' : j.val < windows.val.length := by scalar_tac
      step as ⟨ w, hw ⟩
      have hgw : windows.val[j.val]? = some w := by
        rw [hw]; exact List.getElem?_eq_getElem hlt'
      by_cases hgt : w.deadline > w.period
      · simp only [hgt, ↓reduceIte, WP.spec_ok]
        refine ⟨⟨w, hgw, ?_⟩, h2⟩
        intro hadm
        unfold windowAdmissible at hadm
        scalar_tac
      · simp only [hgt, ↓reduceIte]
        step as ⟨ j1, hj1 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro m w' hm hmw
        by_cases hmj : m < j.val
        · exact h2 m w' hmj hmw
        · have : m = j.val := by scalar_tac
          subst this
          rw [hgw] at hmw
          simp at hmw
          subst hmw
          unfold windowAdmissible
          scalar_tac
    · simp only [hlt, ↓reduceIte, WP.spec_ok]
      intro w hw
      obtain ⟨m, hm, hmw⟩ := List.getElem_of_mem hw
      exact h2 m w (by scalar_tac) (by rw [← hmw]; exact List.getElem?_eq_getElem hm)
  · exact ⟨by scalar_tac, by intro m w hm; exact absurd hm (by scalar_tac)⟩

/-! ## `check_inputs` -/

/-- What a refusal `(rule, at)` of `check_inputs` says of the inputs: the
input it names breaks the rule, and (rules 1, 3 and 4) no earlier one
does. -/
def refusalOk (tasks : List terms.TaskIn) (sites : List terms.Site)
    (windows : List terms.Window) (rule : Nat) (at' : Nat) : Prop :=
  (rule = 1 ∧ (∃ s, sites[at']? = some s ∧ tasks.length ≤ s.task.val)
      ∧ ∀ m s, m < at' → sites[m]? = some s → s.task.val < tasks.length)
  ∨ (rule = 2 ∧ at' = 0 ∧ windows.length ≠ tasks.length)
  ∨ (rule = 3 ∧ (∃ t, tasks[at']? = some t ∧ ¬ taskAdmissible t)
      ∧ ∀ m t, m < at' → tasks[m]? = some t → taskAdmissible t)
  ∨ (rule = 4 ∧ (∃ w, windows[at']? = some w ∧ ¬ windowAdmissible w)
      ∧ ∀ m w, m < at' → windows[m]? = some w → windowAdmissible w)

/-- A refusal names an input that is not admissible. -/
theorem refusalOk_not_admissible {tasks : List terms.TaskIn} {sites : List terms.Site}
    {windows : List terms.Window} {rule at' : Nat}
    (h : refusalOk tasks sites windows rule at') : ¬ inputsAdmissible tasks sites windows := by
  intro ⟨a1, a2, a3, a4⟩
  rcases h with ⟨_, ⟨s, hs, hge⟩, _⟩ | ⟨_, _, hne⟩ | ⟨_, ⟨t, ht, hna⟩, _⟩ | ⟨_, ⟨w, hw, hna⟩, _⟩
  · have := a1 s (List.mem_of_getElem? hs); omega
  · exact hne a2
  · exact hna (a3 t (List.mem_of_getElem? ht))
  · exact hna (a4 w (List.mem_of_getElem? hw))

/-- **`check_inputs` on the code as compiled.** For every input it returns
(no panic, no index out of bounds, no overflow, every loop ends). It
answers `None` exactly when the inputs are admissible, and a refusal
`(rule, at)` is `refusalOk`: rule 1 with the first site that names no
Task, rule 2 when the windows are not one per Task, rule 3 with the first
Task without a Budget or a Priority, rule 4 with the first window whose
deadline is later than its period or MIT. -/
theorem check_inputs_spec (tasks : alloc.vec.Vec terms.TaskIn)
    (sites : alloc.vec.Vec terms.Site) (windows : alloc.vec.Vec terms.Window) :
    terms.check_inputs tasks sites windows ⦃ r =>
      (r = none ↔ inputsAdmissible tasks.val sites.val windows.val)
      ∧ ∀ rule at', r = some (rule, at') →
          refusalOk tasks.val sites.val windows.val rule.val at'.val ⦄ := by
  unfold terms.check_inputs
  have hlen : (alloc.vec.Vec.len tasks).val = tasks.val.length := by simp [alloc.vec.Vec.len]
  have hwlen : (alloc.vec.Vec.len windows).val = windows.val.length := by
    simp [alloc.vec.Vec.len]
  -- a refusal that is `refusalOk` is a correct answer
  have refuse : ∀ (c : U8) (a : Usize),
      refusalOk tasks.val sites.val windows.val c.val a.val →
      ((some (c, a) = (none : Option (U8 × Usize)))
          ↔ inputsAdmissible tasks.val sites.val windows.val)
        ∧ ∀ rule at', some (c, a) = some (rule, at') →
            refusalOk tasks.val sites.val windows.val rule.val at'.val := by
    intro c a h
    refine ⟨⟨(fun h' => by cases h'), fun hadm => absurd hadm (refusalOk_not_admissible h)⟩, ?_⟩
    intro rule at' he
    simp at he
    obtain ⟨e1, e2⟩ := he
    subst e1; subst e2
    exact h
  apply WP.spec_bind (first_stray_site_spec sites (alloc.vec.Vec.len tasks))
  intro o ho
  cases o with
  | some k =>
    simp only [WP.spec_ok]
    obtain ⟨⟨s, hs, hge⟩, hfirst⟩ := ho
    apply refuse
    left
    exact ⟨by scalar_tac, ⟨s, hs, by omega⟩, fun m s' hm hms => by
      have := hfirst m s' hm hms; omega⟩
  | none =>
    simp only at ho ⊢
    split
    · rename_i hne
      simp only [WP.spec_ok]
      apply refuse
      right; left
      refine ⟨by scalar_tac, by scalar_tac, ?_⟩
      intro heq
      have hl : alloc.vec.Vec.len windows = alloc.vec.Vec.len tasks := by scalar_tac
      simp [hl] at hne
    · rename_i heq
      have hwl : windows.val.length = tasks.val.length := by simpa using heq
      apply WP.spec_bind (first_unfit_task_spec tasks)
      intro o1 ho1
      cases o1 with
      | some i =>
        simp only [WP.spec_ok]
        obtain ⟨hex, hfirst⟩ := ho1
        apply refuse
        right; right; left
        exact ⟨by scalar_tac, hex, hfirst⟩
      | none =>
        simp only at ho1 ⊢
        apply WP.spec_bind (first_late_window_spec windows)
        intro o2 ho2
        cases o2 with
        | some j =>
          simp only [WP.spec_ok]
          obtain ⟨hex, hfirst⟩ := ho2
          apply refuse
          right; right; right
          exact ⟨by scalar_tac, hex, hfirst⟩
        | none =>
          simp only [WP.spec_ok]
          simp only at ho2
          refine ⟨⟨fun _ => ⟨fun s hs => by have := ho s hs; omega, hwl, ho1, ho2⟩,
            fun _ => trivial⟩, ?_⟩
          intro rule at' he
          cases he

/-- `check_inputs` returns for every input, with the answer of
`check_inputs_spec` (the same statement as an equation). -/
theorem check_inputs_returns (tasks : alloc.vec.Vec terms.TaskIn)
    (sites : alloc.vec.Vec terms.Site) (windows : alloc.vec.Vec terms.Window) :
    ∃ r, terms.check_inputs tasks sites windows = ok r
      ∧ (r = none ↔ inputsAdmissible tasks.val sites.val windows.val)
      ∧ ∀ rule at', r = some (rule, at') →
          refusalOk tasks.val sites.val windows.val rule.val at'.val :=
  spec_returns (check_inputs_spec tasks sites windows)

/-- `check_inputs` accepts exactly the admissible inputs. -/
theorem check_inputs_accepts_iff (tasks : alloc.vec.Vec terms.TaskIn)
    (sites : alloc.vec.Vec terms.Site) (windows : alloc.vec.Vec terms.Window) :
    terms.check_inputs tasks sites windows = ok none
      ↔ inputsAdmissible tasks.val sites.val windows.val := by
  obtain ⟨r, hr, hiff, _⟩ := check_inputs_returns tasks sites windows
  rw [hr]
  constructor
  · intro h
    simp at h
    exact hiff.mp h
  · intro h
    rw [hiff.mpr h]

/-- **The inputs `check_inputs` accepts, as Property 18 takes them.** The
conclusions are, in this order, the hypotheses `hsok`, `hwl`, `hbud` and
`hDT` of `property18` (`Sched/Link.lean`) and of `property18_kernel`
(`Sched/KRun.lean`), word for word, and the first half of their `hprio`
(every Task's Priority is positive). -/
theorem checked_inputs {tasks : alloc.vec.Vec terms.TaskIn}
    {sites : alloc.vec.Vec terms.Site} {windows : alloc.vec.Vec terms.Window}
    (h : terms.check_inputs tasks sites windows = ok none) :
    (∀ s ∈ sites.val, s.task.val < tasks.val.length)
    ∧ windows.val.length = tasks.val.length
    ∧ (∀ t ∈ tasks.val, 0 < t.budget.val)
    ∧ (∀ w ∈ windows.val, w.deadline.val ≤ w.period.val)
    ∧ (∀ (k : Nat) (t : terms.TaskIn), tasks.val[k]? = some t → 0 < t.priority.val) := by
  obtain ⟨a1, a2, a3, a4⟩ := (check_inputs_accepts_iff tasks sites windows).mp h
  exact ⟨a1, a2, fun t ht => (a3 t ht).1, a4, fun k t hk => (a3 t (List.mem_of_getElem? hk)).2⟩

/-- **The check refuses nothing Property 18 covers.** `check_inputs`
accepts an input exactly when the input has what Property 18 asks of the
inputs alone: its hypotheses `hsok`, `hwl`, `hbud`, `hDT` and the positive
Priorities of `hprio`. -/
theorem check_inputs_accepts_iff_p18 (tasks : alloc.vec.Vec terms.TaskIn)
    (sites : alloc.vec.Vec terms.Site) (windows : alloc.vec.Vec terms.Window) :
    terms.check_inputs tasks sites windows = ok none
      ↔ ((∀ s ∈ sites.val, s.task.val < tasks.val.length)
        ∧ windows.val.length = tasks.val.length
        ∧ (∀ t ∈ tasks.val, 0 < t.budget.val)
        ∧ (∀ w ∈ windows.val, w.deadline.val ≤ w.period.val)
        ∧ (∀ (k : Nat) (t : terms.TaskIn), tasks.val[k]? = some t → 0 < t.priority.val)) := by
  constructor
  · exact checked_inputs
  · intro ⟨a1, a2, a3, a4, a5⟩
    refine (check_inputs_accepts_iff tasks sites windows).mpr ⟨a1, a2, ?_, a4⟩
    intro t ht
    obtain ⟨m, hm, hmt⟩ := List.getElem_of_mem ht
    exact ⟨a3 t ht, a5 m t (by rw [← hmt]; exact List.getElem?_eq_getElem hm)⟩

/-! ## The Endpoint operations as sites, and their cost (R40.2, task 21.5) -/

/-- An Endpoint operation of Task `task` as a Critical_Section site
(`endpoint_site` of `terms.rs`): its Ceiling is above every Priority and
it is marked as an Endpoint site. -/
def endpointSite (task : Usize) (wcet : U64) : terms.Site :=
  { task, ceiling := core.num.U8.MAX, wcet, endpoint := true }

/-- `endpoint_sites` returns the two sites of an Endpoint role: a send of
Δ_send and a receive of Δ_recv. -/
theorem endpoint_sites_spec (task : Usize) (p : terms.Params) :
    terms.endpoint_sites task p = ok (endpointSite task p.send, endpointSite task p.recv) := by
  rfl

theorem U8_MAX_val : (core.num.U8.MAX).val = 255 := by rfl

/-- An Endpoint site can block every Task of a higher Priority than its
holder (the two conditions of R29.2 that `maxCandidate` tests: the holder
below the Task, the Ceiling at least the Task's Priority), and what it
costs that Task is its WCET, whatever the Partitions and the
Overrun_Response are. (`lemma_endpoint_site_blocks` of `terms.rs`, with
the candidate.) -/
theorem endpoint_site_blocks (ti l : terms.TaskIn) (task : Usize) (wcet : U64)
    (p : terms.Params) (hlow : l.priority.val < ti.priority.val) :
    l.priority.val < ti.priority.val
    ∧ ti.priority.val ≤ (endpointSite task wcet).ceiling.val
    ∧ candidateOf ti l (endpointSite task wcet) p = wcet.val := by
  refine ⟨hlow, ?_, ?_⟩
  · simp only [endpointSite, U8_MAX_val]; scalar_tac
  · simp [candidateOf, endpointSite]

/-- `B_i` over a prefix is at most `B_i` over a longer one. -/
theorem maxCandidate_mono (tasks : List terms.TaskIn) (sites : List terms.Site)
    (ti : terms.TaskIn) (p : terms.Params) (m n : Nat) (h : m ≤ n) :
    maxCandidate tasks sites ti p m ≤ maxCandidate tasks sites ti p n := by
  induction n with
  | zero => have : m = 0 := by omega
            subst this; exact Nat.le_refl _
  | succ n ih =>
    by_cases hm : m = n + 1
    · subst hm; exact Nat.le_refl _
    · have h1 := ih (by omega)
      refine Nat.le_trans h1 ?_
      simp only [maxCandidate]
      split
      · exact Nat.le_refl _
      · split
        · exact Nat.le_refl _
        · split
          · omega
          · exact Nat.le_refl _

/-- **An Endpoint site in `B_i`.** If site `k` of the Analyzer's sites is
an Endpoint site of a Task `l` of lower Priority than `ti`, then the
blocking term of `ti` over all the sites (`maxCandidate`, which
`derive_all` is proven to compute as `B_i`: `derive_all_spec`,
`blocking_spec`) is at least the site's WCET. With `endpoint_sites_spec`:
Δ_send and Δ_recv of every lower Task's Endpoint role are in `B_i`. -/
theorem endpoint_site_in_b (tasks : List terms.TaskIn) (sites : List terms.Site)
    (ti l : terms.TaskIn) (p : terms.Params) (k : Nat) (task : Usize) (wcet : U64)
    (hk : sites[k]? = some (endpointSite task wcet))
    (hl : tasks[task.val]? = some l)
    (hlow : l.priority.val < ti.priority.val) :
    wcet.val ≤ maxCandidate tasks sites ti p sites.length := by
  have hklt : k < sites.length := (List.getElem?_eq_some_iff.mp hk).1
  refine Nat.le_trans ?_ (maxCandidate_mono tasks sites ti p (k + 1) sites.length (by omega))
  obtain ⟨_, hceil, hcand⟩ := endpoint_site_blocks ti l task wcet p hlow
  simp only [maxCandidate, hk]
  have hl' : tasks[(endpointSite task wcet).task.val]? = some l := hl
  simp only [hl']
  split
  · omega
  · rename_i hnot
    by_contra hlt
    apply hnot
    exact ⟨hlow, hceil, by omega⟩

/-- **An Endpoint site in the `B_i` the compiled derivation returns.** When
`derive_all` returns terms, the blocking term `b` it returns for a Task is
at least the WCET of every Endpoint site of a Task of lower Priority.
(`endpoint_site_in_b` through `derive_all_spec`.) -/
theorem endpoint_site_in_derived (tasks : alloc.vec.Vec terms.TaskIn)
    (sites : alloc.vec.Vec terms.Site) (p : terms.Params) (any_fpu : Bool)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length)
    (v : alloc.vec.Vec terms.Derived)
    (hder : terms.derive_all tasks sites p any_fpu = ok (some v))
    (i : Nat) (d : terms.Derived) (ti l : terms.TaskIn) (k : Nat) (task : Usize) (wcet : U64)
    (hd : v.val[i]? = some d) (hti : tasks.val[i]? = some ti)
    (hk : sites.val[k]? = some (endpointSite task wcet))
    (hl : tasks.val[task.val]? = some l)
    (hlow : l.priority.val < ti.priority.val) :
    wcet.val ≤ d.b.val := by
  have hspec := derive_all_spec tasks sites p any_fpu hsok
  rw [hder] at hspec
  simp only [WP.spec_ok] at hspec
  obtain ⟨_, hall⟩ := hspec v rfl
  obtain ⟨_, _, _, _, hb, _⟩ := hall i d ti hd hti
  rw [hb]
  exact endpoint_site_in_b tasks.val sites.val ti l p k task wcet hk hl hlow

/-- **The Endpoint cost in the `O_i` the compiled derivation returns.**
When `derive_all` returns terms, the overhead `o` it returns for a Task is
at least the Task's `endpoint_cost` (the sum `endpoint_cost_add` builds
over the Task's roles). -/
theorem endpoint_cost_in_derived (tasks : alloc.vec.Vec terms.TaskIn)
    (sites : alloc.vec.Vec terms.Site) (p : terms.Params) (any_fpu : Bool)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length)
    (v : alloc.vec.Vec terms.Derived)
    (hder : terms.derive_all tasks sites p any_fpu = ok (some v))
    (i : Nat) (d : terms.Derived) (ti : terms.TaskIn)
    (hd : v.val[i]? = some d) (hti : tasks.val[i]? = some ti) :
    ti.endpoint_cost.val ≤ d.o.val := by
  have hspec := derive_all_spec tasks sites p any_fpu hsok
  rw [hder] at hspec
  simp only [WP.spec_ok] at hspec
  obtain ⟨_, hall⟩ := hspec v rfl
  obtain ⟨_, _, ho, _⟩ := hall i d ti hd hti
  rw [ho]
  unfold specO
  omega

/-- An Endpoint site costs no lock entry and exit: it is not among the
sites `O_i` multiplies Δ_lock_in + Δ_lock_out by (`siteCount`, which
`derive_all` is proven to compute: `site_stats_spec`). Its cost in `O_i`
is the Task's `endpoint_cost` (`specO`). -/
theorem endpoint_site_no_lock_cost (sites : List terms.Site) (t n : Nat)
    (task : Usize) (wcet : U64) (hn : sites[n]? = some (endpointSite task wcet)) :
    siteCount sites t (n + 1) = siteCount sites t n := by
  simp [siteCount, hn, endpointSite]

/-- The larger of Δ_send and Δ_recv: what one declared Endpoint operation
of a Job costs at most (`endpoint_op` of `terms.rs`). -/
def endpointOp (p : terms.Params) : Nat :=
  if p.send.val ≥ p.recv.val then p.send.val else p.recv.val

/-- `add` returns, with a value exactly when the sum fits 64 bits. -/
theorem add_full (a b : U64) :
    terms.add a b ⦃ r =>
      (r.isSome ↔ a.val + b.val ≤ U64.max) ∧ ∀ v, r = some v → v.val = a.val + b.val ⦄ := by
  unfold terms.add
  step as ⟨ i, hi ⟩
  · sc
  have hi' : i.val = 18446744073709551615 - b.val := by
    rw [hi, rMax_val]
  split
  · rename_i hgt
    simp only [WP.spec_ok]
    refine ⟨?_, by intro v hv; cases hv⟩
    simp only [Option.isSome_none, Bool.false_eq_true, false_iff, not_le]
    rw [max_val]; scalar_tac
  · rename_i hngt
    step as ⟨ i1, hi1 ⟩
    refine ⟨?_, ?_⟩
    · simp only [Option.isSome_some, true_iff]
      rw [max_val]; scalar_tac
    · intro v hv
      simp at hv
      subst hv
      exact hi1

/-- `mul` returns, with a value exactly when the product fits 64 bits. -/
theorem mul_full (a b : U64) :
    terms.mul a b ⦃ r =>
      (r.isSome ↔ a.val * b.val ≤ U64.max) ∧ ∀ v, r = some v → v.val = a.val * b.val ⦄ := by
  unfold terms.mul
  step as ⟨ i, hi ⟩
  step as ⟨ i1, hi1 ⟩
  have h3 : i.val = a.val := by rw [hi]; scalar_tac
  have h4 : i1.val = b.val := by rw [hi1]; scalar_tac
  have hmul : a.val * b.val ≤ 18446744073709551615 * 18446744073709551615 :=
    Nat.mul_le_mul (by sc) (by sc)
  step as ⟨ product, hprod ⟩
  step as ⟨ i2, hi2 ⟩
  have h5 : i2.val = (core.num.U64.MAX).val := by rw [hi2]; scalar_tac
  rw [MAX_val] at h5
  have hp : product.val = a.val * b.val := by rw [hprod, h3, h4]
  split
  · rename_i hgt
    simp only [WP.spec_ok]
    refine ⟨?_, by intro v hv; cases hv⟩
    simp only [Option.isSome_none, Bool.false_eq_true, false_iff, not_le]
    rw [max_val, ← hp]; scalar_tac
  · rename_i hngt
    step as ⟨ i3, hi3 ⟩
    have h6 : i3.val = product.val := by rw [hi3]; sc
    refine ⟨?_, ?_⟩
    · simp only [Option.isSome_some, true_iff]
      rw [max_val, ← hp]; scalar_tac
    · intro v hv
      simp at hv
      subst hv
      rw [h6, hp]

/-- **`endpoint_cost_add` on the code as compiled (R40.2, the
execution-time bound).** It returns for every input. The Endpoint cost of
a Task after one more role with `ops` declared operations per Job is
`acc` plus `ops` times the larger of Δ_send and Δ_recv; the answer is
`None` exactly when that does not fit 64 bits (R29.6). The sum over a
Task's roles is its `endpoint_cost`, a summand of `O_i` (`specO`). -/
theorem endpoint_cost_add_spec (acc ops : U64) (p : terms.Params) :
    terms.endpoint_cost_add acc ops p ⦃ r =>
      (r.isSome ↔ acc.val + ops.val * endpointOp p ≤ U64.max)
      ∧ ∀ v, r = some v → v.val = acc.val + ops.val * endpointOp p ⦄ := by
  unfold terms.endpoint_cost_add
  have key : ∀ op : U64, op.val = endpointOp p →
      (do
        let o ← terms.mul ops op
        match o with
        | none => ok none
        | some c => terms.add acc c) ⦃ r =>
      (r.isSome ↔ acc.val + ops.val * endpointOp p ≤ U64.max)
      ∧ ∀ v, r = some v → v.val = acc.val + ops.val * endpointOp p ⦄ := by
    intro op hop
    apply WP.spec_bind (mul_full ops op)
    intro o ho
    obtain ⟨ho1, ho2⟩ := ho
    cases o with
    | none =>
      simp only [WP.spec_ok]
      refine ⟨?_, by intro v hv; cases hv⟩
      simp only [Option.isSome_none, Bool.false_eq_true, false_iff, not_le] at ho1 ⊢
      rw [← hop]; omega
    | some c =>
      have hc := ho2 c rfl
      simp only
      apply WP.spec_mono (add_full acc c)
      intro r ⟨hr1, hr2⟩
      rw [← hop, ← hc]
      exact ⟨hr1, hr2⟩
  split
  · rename_i hge
    simp only [bind_ok]
    exact key p.send (by unfold endpointOp; split <;> scalar_tac)
  · rename_i hnge
    simp only [bind_ok]
    exact key p.recv (by unfold endpointOp; split <;> scalar_tac)

end rsk_analyzer
