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

end rsk_analyzer
