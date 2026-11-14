/-
  The Analyzer's term derivation on the code as compiled (Verification_Plan
  7.2, 2026-11-14): `terms::derive_all` and the functions it calls
  (`site_stats`, `overhead`, `interference_bound`, `blocking`). When they
  return a value it is the term of R29.2 to R29.4 as restated here (`specO`,
  `specE`, `siteCount`, `longestCs`, `maxCandidate`), and they return for
  every input in which each site names a Task. What is not stated here and
  is in the Verus contract: that `None` is returned only when a term does
  not fit 64 bits, and which site attains `B_i`. Handwritten.
-/
import Aeneas
import Rta.Rta
import Rta.Proofs
import Rta.Chain
open Aeneas Aeneas.Std Result

namespace rsk_analyzer

@[step] theorem add_step (a b : U64) :
    terms.add a b ⦃ r => ∀ v, r = some v → v.val = a.val + b.val ⦄ := add_spec a b

@[step] theorem mul_step (a b : U64) :
    terms.mul a b ⦃ r => ∀ v, r = some v → v.val = a.val * b.val ⦄ := by
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
  split
  · simp
  · step as ⟨ i3, hi3 ⟩
    have h6 : i3.val = product.val := by rw [hi3]; sc
    intro v hv
    simp at hv
    subst hv
    rw [h6, hprod, h3, h4]

/-- The timed events of R9.4 a Job of the Task costs. -/
def timedEvents (t : terms.TaskIn) : Nat := if t.periodic || t.defer then 3 else 2

/-- `O_i` of R29.3 for a Task with `sites` Resource sites. -/
def specO (t : terms.TaskIn) (p : terms.Params) (sites : Nat) (any_fpu : Bool) : Nat :=
  p.release.val + p.dispatch.val + p.complete.val + p.wake.val
    + p.timer.val * timedEvents t
    + (p.lock_in.val + p.lock_out.val) * sites
    + p.hm.val + p.entry.val
    + (if any_fpu then p.fp_save.val + p.fp_restore.val else 0)
    + (if t.periodic then 0 else p.irq.val + p.stamp.val)
    + t.endpoint_cost.val

set_option maxHeartbeats 8000000 in
theorem overhead_spec (t : terms.TaskIn) (p : terms.Params) (sites : U64) (any_fpu : Bool) :
    terms.overhead t p sites any_fpu
      ⦃ r => ∀ v, r = some v → v.val = specO t p sites.val any_fpu ⦄ := by
  unfold terms.overhead
  cases hp : t.periodic <;> cases hd : t.defer <;> cases any_fpu <;> by_cases hs : sites = 0#u64
  all_goals simp [hs]
  all_goals step*
  all_goals (simp_all [specO, timedEvents])

/-- `E_j` of R29.4. -/
def specE (t : terms.TaskIn) (p : terms.Params) (longest o : Nat) : Nat :=
  if t.stops_overruns then t.budget.val + p.detect.val + p.enforce.val + longest + o
  else t.budget.val + o

theorem interference_bound_spec (t : terms.TaskIn) (p : terms.Params) (longest o : U64) :
    terms.interference_bound t p longest o
      ⦃ r => ∀ v, r = some v → v.val = specE t p longest.val o.val ⦄ := by
  unfold terms.interference_bound
  cases hs : t.stops_overruns
  all_goals simp
  all_goals step*
  all_goals (simp_all [specE])

/-- A `usize` is at most a `u64` on either platform width. -/
theorem usize_max_le : Usize.max ≤ U64.max := by
  rcases Usize.bounds_eq with h | h <;> rw [h] <;> scalar_tac

/-- The number of Resource sites of Task `t` among the first `n`. -/
def siteCount (sites : List terms.Site) (t : Nat) : Nat → Nat
  | 0 => 0
  | n + 1 => siteCount sites t n +
      (match sites[n]? with
       | some s => if s.task.val = t ∧ s.endpoint = false then 1 else 0
       | none => 0)

/-- The longest site of Task `t` among the first `n` (0 without one). -/
def longestCs (sites : List terms.Site) (t : Nat) : Nat → Nat
  | 0 => 0
  | n + 1 =>
    let r := longestCs sites t n
    match sites[n]? with
    | some s => if s.task.val = t ∧ s.wcet.val > r then s.wcet.val else r
    | none => r

theorem site_stats_body_spec (sites : alloc.vec.Vec terms.Site) (t : Usize)
    (count longest : U64) (k : Usize)
    (hk : k.val ≤ sites.val.length) (hck : count.val ≤ k.val)
    (hc : count.val = siteCount sites.val t.val k.val)
    (hl : longest.val = longestCs sites.val t.val k.val) :
    terms.site_stats_loop.body sites t count longest k ⦃ r =>
      match r with
      | .done (c, l) => c.val = siteCount sites.val t.val sites.val.length
          ∧ l.val = longestCs sites.val t.val sites.val.length
      | .cont (c, l, k') => k'.val = k.val + 1 ∧ k'.val ≤ sites.val.length ∧ c.val ≤ k'.val
          ∧ c.val = siteCount sites.val t.val k'.val
          ∧ l.val = longestCs sites.val t.val k'.val ⦄ := by
  unfold terms.site_stats_loop.body
  simp only []
  split
  · rename_i hlt
    have hlt' : k.val < sites.val.length := by scalar_tac
    step as ⟨ s, hs ⟩
    have hget : sites.val[k.val]? = some s := by
      rw [hs]; exact List.getElem?_eq_getElem hlt'
    have hcn : siteCount sites.val t.val (k.val + 1) = siteCount sites.val t.val k.val +
        (if s.task.val = t.val ∧ s.endpoint = false then 1 else 0) := by
      simp [siteCount, hget]
    have hln : longestCs sites.val t.val (k.val + 1) =
        (if s.task.val = t.val ∧ s.wcet.val > longestCs sites.val t.val k.val then s.wcet.val
         else longestCs sites.val t.val k.val) := by
      simp [longestCs, hget]
    by_cases hst : s.task = t
    · have hst' : s.task.val = t.val := by rw [hst]
      have hlenb : sites.val.length ≤ Usize.max := by scalar_tac
      have hu : Usize.max ≤ U64.max := usize_max_le
      have hmax : count.val + 1 ≤ U64.max := by omega
      cases hse : s.endpoint
      · by_cases hw : s.wcet.val > longest.val
        · simp [hst, hse, hw]
          step as ⟨ c2, hc2 ⟩
          step as ⟨ k1, hk1 ⟩
          refine ⟨hk1, by scalar_tac, by scalar_tac, ?_, ?_⟩
          · rw [hk1, hcn]; simp [hst', hse, hc2, hc]
          · rw [hk1, hln]; simp [hst', ← hl, hw]
        · simp [hst, hse, hw]
          step as ⟨ c2, hc2 ⟩
          step as ⟨ k1, hk1 ⟩
          refine ⟨hk1, by scalar_tac, by scalar_tac, ?_, ?_⟩
          · rw [hk1, hcn]; simp [hst', hse, hc2, hc]
          · rw [hk1, hln]; simp [hst', ← hl, hw]
      · by_cases hw : s.wcet.val > longest.val
        · simp [hst, hse, hw]
          step as ⟨ k1, hk1 ⟩
          refine ⟨hk1, by scalar_tac, by scalar_tac, ?_, ?_⟩
          · rw [hk1, hcn]; simp [hst', hse, hc]
          · rw [hk1, hln]; simp [hst', ← hl, hw]
        · simp [hst, hse, hw]
          step as ⟨ k1, hk1 ⟩
          refine ⟨hk1, by scalar_tac, by scalar_tac, ?_, ?_⟩
          · rw [hk1, hcn]; simp [hst', hse, hc]
          · rw [hk1, hln]; simp [hst', ← hl, hw]
    · have hst' : s.task.val ≠ t.val := by
        intro h; exact hst (UScalar.eq_of_val_eq h)
      simp [hst]
      step as ⟨ k1, hk1 ⟩
      refine ⟨hk1, by scalar_tac, by scalar_tac, ?_, ?_⟩
      · rw [hk1, hcn]; simp [hst', hc]
      · rw [hk1, hln]; simp [hst', hl]
  · rename_i hge
    have heq : k.val = sites.val.length := by scalar_tac
    simp
    rw [← heq]
    exact ⟨hc, hl⟩

theorem site_stats_spec (sites : alloc.vec.Vec terms.Site) (t : Usize) :
    terms.site_stats sites t ⦃ r =>
      r.count.val = siteCount sites.val t.val sites.val.length
        ∧ r.longest.val = longestCs sites.val t.val sites.val.length ⦄ := by
  unfold terms.site_stats
  have hloop : terms.site_stats_loop sites t 0#u64 0#u64 0#usize ⦃ r =>
      r.1.val = siteCount sites.val t.val sites.val.length
        ∧ r.2.val = longestCs sites.val t.val sites.val.length ⦄ := by
    unfold terms.site_stats_loop
    apply loop.spec_decr_nat
      (fun (s : U64 × U64 × Usize) => sites.val.length - s.2.2.val)
      (fun s => s.2.2.val ≤ sites.val.length ∧ s.1.val ≤ s.2.2.val
        ∧ s.1.val = siteCount sites.val t.val s.2.2.val
        ∧ s.2.1.val = longestCs sites.val t.val s.2.2.val)
    · intro s hs
      obtain ⟨c, l, k⟩ := s
      obtain ⟨h1, h2, h3, h4⟩ := hs
      simp only [] at h1 h2 h3 h4
      apply WP.spec_mono (site_stats_body_spec sites t c l k h1 h2 h3 h4)
      intro r hr
      cases r with
      | done q =>
        obtain ⟨c', l'⟩ := q
        simpa using hr
      | cont s' =>
        obtain ⟨c', l', k'⟩ := s'
        simp only [] at hr ⊢
        obtain ⟨e1, e2, e3, e4, e5⟩ := hr
        exact ⟨⟨e2, e3, e4, e5⟩, by omega⟩
    · simp [siteCount, longestCs]
  step with hloop as ⟨ c, l, hc, hl ⟩
  exact ⟨hc, hl⟩

/-- The Kernel-level blocking: `L_kernel` or the Partition stop and restart
work, whichever is larger. -/
def bFloor (p : terms.Params) : Nat :=
  if p.l_kernel.val ≥ (if p.restart.val ≥ p.stop.val then p.restart.val else p.stop.val)
  then p.l_kernel.val
  else (if p.restart.val ≥ p.stop.val then p.restart.val else p.stop.val)

/-- The blocking a site of a lower Task `l` costs Task `ti` (R29.2). -/
def candidateOf (ti l : terms.TaskIn) (s : terms.Site) (p : terms.Params) : Nat :=
  if s.endpoint = false ∧ l.partition ≠ ti.partition ∧ l.stops_overruns = true
  then l.budget.val + p.detect.val + p.enforce.val
  else s.wcet.val

/-- `B_i` over the first `n` sites: the largest candidate of the sites
that can block the Task (a holder of lower Priority, a Ceiling of at least
the Task's Priority), and at least the Kernel-level blocking. -/
def maxCandidate (tasks : List terms.TaskIn) (sites : List terms.Site) (ti : terms.TaskIn)
    (p : terms.Params) : Nat → Nat
  | 0 => bFloor p
  | n + 1 =>
    let r := maxCandidate tasks sites ti p n
    match sites[n]? with
    | none => r
    | some s =>
      match tasks[s.task.val]? with
      | none => r
      | some l =>
        if l.priority.val < ti.priority.val ∧ ti.priority.val ≤ s.ceiling.val
            ∧ candidateOf ti l s p > r
        then candidateOf ti l s p else r

set_option maxHeartbeats 8000000 in
theorem blocking_body_spec (tasks : alloc.vec.Vec terms.TaskIn) (sites : alloc.vec.Vec terms.Site)
    (p : terms.Params) (ti : terms.TaskIn) (b : U64) (site : Option Usize) (k : Usize)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length)
    (hk : k.val ≤ sites.val.length)
    (hb : b.val = maxCandidate tasks.val sites.val ti p k.val) :
    terms.blocking_loop.body tasks sites p.detect p.enforce ti b site k ⦃ r =>
      match r with
      | .done o => ∀ bl, o = some bl →
          bl.b.val = maxCandidate tasks.val sites.val ti p sites.val.length
      | .cont (b', site', k') => k'.val = k.val + 1 ∧ k'.val ≤ sites.val.length
          ∧ b'.val = maxCandidate tasks.val sites.val ti p k'.val ⦄ := by
  unfold terms.blocking_loop.body
  simp only []
  split
  · rename_i hlt
    have hlt' : k.val < sites.val.length := by scalar_tac
    step as ⟨ s, hs ⟩
    have hgets : sites.val[k.val]? = some s := by
      rw [hs]; exact List.getElem?_eq_getElem hlt'
    have hst : s.task.val < tasks.val.length := by
      apply hsok; rw [hs]; exact List.getElem_mem _
    step as ⟨ l, hl ⟩
    have hgetl : tasks.val[s.task.val]? = some l := by
      rw [hl]; exact List.getElem?_eq_getElem hst
    have hnext : maxCandidate tasks.val sites.val ti p (k.val + 1) =
        (if l.priority.val < ti.priority.val ∧ ti.priority.val ≤ s.ceiling.val
            ∧ candidateOf ti l s p > b.val
         then candidateOf ti l s p else b.val) := by
      simp [maxCandidate, hgets, hgetl, hb]
    by_cases h1 : l.priority.val < ti.priority.val <;>
      by_cases h2 : ti.priority.val ≤ s.ceiling.val <;>
      cases h3 : s.endpoint <;>
      by_cases h4 : l.partition = ti.partition <;>
      cases h5 : l.stops_overruns
    all_goals simp [h1, h2, h3, h4, h5]
    all_goals (repeat' (first | split | step))
    all_goals first
      | (simp_all [candidateOf]; done)
      | (simp_all [candidateOf]; omega)
      | (simp_all [candidateOf]; scalar_tac)
      | skip
  · rename_i hge
    have heq : k.val = sites.val.length := by scalar_tac
    simp
    rw [← heq]
    exact hb

theorem blocking_loop_spec (tasks : alloc.vec.Vec terms.TaskIn) (sites : alloc.vec.Vec terms.Site)
    (p : terms.Params) (ti : terms.TaskIn) (b0 : U64)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length)
    (hb0 : b0.val = bFloor p) :
    terms.blocking_loop tasks sites p.detect p.enforce ti b0 none 0#usize ⦃ r =>
      ∀ bl, r = some bl → bl.b.val = maxCandidate tasks.val sites.val ti p sites.val.length ⦄ := by
  unfold terms.blocking_loop
  apply loop.spec_decr_nat
    (fun (s : U64 × Option Usize × Usize) => sites.val.length - s.2.2.val)
    (fun s => s.2.2.val ≤ sites.val.length
      ∧ s.1.val = maxCandidate tasks.val sites.val ti p s.2.2.val)
  · intro s hs
    obtain ⟨b, site, k⟩ := s
    obtain ⟨h1, h2⟩ := hs
    simp only [] at h1 h2
    apply WP.spec_mono (blocking_body_spec tasks sites p ti b site k hsok h1 h2)
    intro r hr
    cases r with
    | done o => simpa using hr
    | cont s' =>
      obtain ⟨b', site', k'⟩ := s'
      simp only [] at hr ⊢
      obtain ⟨e1, e2, e3⟩ := hr
      exact ⟨⟨e2, e3⟩, by omega⟩
  · simp [maxCandidate, hb0]

/-- `B_i` of R29.2 for Task `i`, when the function returns one. -/
theorem blocking_spec (tasks : alloc.vec.Vec terms.TaskIn) (sites : alloc.vec.Vec terms.Site)
    (i : Usize) (p : terms.Params) (ti : terms.TaskIn) (hti : tasks.val[i.val]? = some ti)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length) :
    terms.blocking tasks sites i p ⦃ r =>
      ∀ bl, r = some bl → bl.b.val = maxCandidate tasks.val sites.val ti p sites.val.length ⦄ := by
  have hi : i.val < tasks.val.length := by
    rcases List.getElem?_eq_some_iff.mp hti with ⟨h, _⟩; exact h
  unfold terms.blocking
  step as ⟨ ti0, hti0 ⟩
  have hti1 : ti0 = ti := by
    have := List.getElem?_eq_getElem hi
    rw [this] at hti; simp at hti; rw [hti0]; exact hti
  subst hti1
  by_cases hr : p.restart.val ≥ p.stop.val <;> by_cases hlr : p.l_kernel.val ≥ p.restart.val <;>
    by_cases hls : p.l_kernel.val ≥ p.stop.val
  all_goals simp [hr, hlr, hls]
  all_goals first
    | (apply blocking_loop_spec tasks sites p ti0 _ hsok; simp [bFloor, hr, hlr, hls]; done)
    | (apply blocking_loop_spec tasks sites p ti0 _ hsok; simp [bFloor, hr, hlr, hls]; omega)
    | (split <;> (apply blocking_loop_spec tasks sites p ti0 _ hsok; simp_all [bFloor]; try omega))

/-- `d` holds the terms of the Task `ti` at index `j` (R29.2 to R29.4). -/
def derivedOk (tasks : List terms.TaskIn) (sites : List terms.Site) (p : terms.Params)
    (any_fpu : Bool) (j : Nat) (ti : terms.TaskIn) (d : terms.Derived) : Prop :=
  d.sites.val = siteCount sites j sites.length
  ∧ d.longest_cs.val = longestCs sites j sites.length
  ∧ d.o.val = specO ti p d.sites.val any_fpu
  ∧ d.e.val = specE ti p d.longest_cs.val d.o.val
  ∧ d.b.val = maxCandidate tasks sites ti p sites.length
  ∧ d.j = p.j_rel

/-- Every entry of `out` holds the terms of the Task at its index. -/
def allDerived (tasks : List terms.TaskIn) (sites : List terms.Site) (p : terms.Params)
    (any_fpu : Bool) (out : List terms.Derived) : Prop :=
  ∀ j d ti, out[j]? = some d → tasks[j]? = some ti → derivedOk tasks sites p any_fpu j ti d

theorem derive_all_body_spec (tasks : alloc.vec.Vec terms.TaskIn) (sites : alloc.vec.Vec terms.Site)
    (p : terms.Params) (any_fpu : Bool) (out : alloc.vec.Vec terms.Derived) (i : Usize)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length)
    (hi : i.val ≤ tasks.val.length) (hlen : out.val.length = i.val)
    (hall : allDerived tasks.val sites.val p any_fpu out.val) :
    terms.derive_all_loop.body tasks sites p any_fpu out i ⦃ r =>
      match r with
      | .done o => ∀ v, o = some v → v.val.length = tasks.val.length
          ∧ allDerived tasks.val sites.val p any_fpu v.val
      | .cont (out', i') => i'.val = i.val + 1 ∧ i'.val ≤ tasks.val.length
          ∧ out'.val.length = i'.val
          ∧ allDerived tasks.val sites.val p any_fpu out'.val ⦄ := by
  unfold terms.derive_all_loop.body
  simp only []
  split
  · rename_i hlt
    have hlt' : i.val < tasks.val.length := by scalar_tac
    step as ⟨ t, ht ⟩
    have hget : tasks.val[i.val]? = some t := by
      rw [ht]; exact List.getElem?_eq_getElem hlt'
    step with site_stats_spec as ⟨ stats, hcount, hlongest ⟩
    step with overhead_spec as ⟨ o, ho ⟩
    cases o with
    | none => simp
    | some v =>
      simp only []
      have hv := ho v rfl
      step with interference_bound_spec as ⟨ o1, ho1 ⟩
      cases o1 with
      | none => simp
      | some v1 =>
        simp only []
        have hv1 := ho1 v1 rfl
        step with blocking_spec tasks sites i p t hget hsok as ⟨ o2, ho2 ⟩
        cases o2 with
        | none => simp
        | some v2 =>
          simp only []
          have hv2 := ho2 v2 rfl
          step as ⟨ out1, hout1 ⟩
          step as ⟨ i2, hi2 ⟩
          refine ⟨hi2, by scalar_tac, by rw [hout1]; simp; omega, ?_⟩
          intro j d tj hd htj
          rw [hout1] at hd
          by_cases hj : j < out.val.length
          · rw [List.getElem?_append_left hj] at hd
            exact hall j d tj hd htj
          · have hjl : j = out.val.length := by
              have := (List.getElem?_eq_some_iff.mp hd).1
              simp at this; omega
            subst hjl
            simp at hd
            subst hd
            rw [hlen] at htj
            rw [hget] at htj
            simp at htj
            subst htj
            simp only [derivedOk]
            rw [hlen]
            exact ⟨hcount, hlongest, hv, hv1, hv2, trivial⟩
  · rename_i hge
    have heq : i.val = tasks.val.length := by scalar_tac
    simp
    exact ⟨by omega, hall⟩

/-- The term derivation (R29.2 to R29.4), on the code as compiled. When
`derive_all` returns a list it has one entry per Task and every entry
holds that Task's terms: its site count and longest Critical_Section,
`O_i`, `E_i`, `B_i`, and `J_i = J_rel`. It returns for every input in
which each site names a Task of the set (no panic, no overflow: an
overflowing sum is answered with `None`). -/
theorem derive_all_spec (tasks : alloc.vec.Vec terms.TaskIn) (sites : alloc.vec.Vec terms.Site)
    (p : terms.Params) (any_fpu : Bool)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length) :
    terms.derive_all tasks sites p any_fpu ⦃ r =>
      ∀ v, r = some v → v.val.length = tasks.val.length
        ∧ allDerived tasks.val sites.val p any_fpu v.val ⦄ := by
  unfold terms.derive_all terms.derive_all_loop
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec terms.Derived × Usize) => tasks.val.length - s.2.val)
    (fun s => s.2.val ≤ tasks.val.length ∧ s.1.val.length = s.2.val
      ∧ allDerived tasks.val sites.val p any_fpu s.1.val)
  · intro s hs
    obtain ⟨out, i⟩ := s
    obtain ⟨h1, h2, h3⟩ := hs
    simp only [] at h1 h2 h3
    apply WP.spec_mono (derive_all_body_spec tasks sites p any_fpu out i hsok h1 h2 h3)
    intro r hr
    cases r with
    | done o => simpa using hr
    | cont s' =>
      obtain ⟨out', i'⟩ := s'
      simp only [] at hr ⊢
      obtain ⟨e1, e2, e3, e4⟩ := hr
      exact ⟨⟨e2, e3, e4⟩, by omega⟩
  · simp [allDerived]

end rsk_analyzer
