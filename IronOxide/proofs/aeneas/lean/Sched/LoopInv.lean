import Aeneas
open Aeneas Aeneas.Std Result Lean.Order
open Aeneas.Data.Coinductive

/-! # An invariant rule for extracted loops, without a measure (2026-11-19)

The statements of this library about the Kernel's code have the form "if
the function returns, then ...". For a loop that form needs no termination
argument: an invariant every iteration keeps holds when the loop exits,
whenever it does. `loop_inv` is that rule for Aeneas's `loop`, by the
induction principle of its partial fixpoint; the partial specification
used for admissibility (`pspec`) puts no demand on a failing step, unlike
Aeneas's `dspec`. -/

namespace P18

unseal Result in
/-- The effect interpretation that asks nothing of a failure. -/
@[reducible]
def anyWP : EffectWP RustEffect where
  State := Unit
  wp _ _ _ := True

instance : EffectWP.Monotonic anyWP where
  wp_monotonic _ := fun _ => trivial

instance : EffectWP.Conjunctive anyWP where
  wp_conj := fun _ _ _ => trivial

unseal Result in
/-- `m` satisfies `p` whenever it returns. -/
def pspec {α} (m : Result α) (p : α → Prop) : Prop :=
  DWLP anyWP m (fun value _ => p value) ()

unseal Result in
theorem pspec_admissible {α} (p : α → Prop) : admissible (fun x : Result α => pspec x p) :=
  DWLP.admissible anyWP _ ()

unseal Result in
theorem pspec_iff {α} (m : Result α) (p : α → Prop) : pspec m p ↔ ∀ r, m = ok r → p r := by
  cases m with
  | ret v =>
    constructor
    · intro h r hr
      have hv : p v := DWLP.ret_iff.mp h
      have : v = r := by simpa using hr
      rw [← this]; exact hv
    · intro h
      exact DWLP.ret_iff.mpr (h v rfl)
  | vis e k =>
    constructor
    · intro _ r hr
      exact absurd hr (by simp)
    · intro _
      exact DWLP.vis trivial
  | div =>
    constructor
    · intro _ r hr
      exact absurd hr (by simp)
    · intro _
      exact DWLP.div

/-- **The invariant rule.** If every iteration that continues keeps `inv`
and every iteration that exits from a state satisfying `inv` establishes
`post`, then a loop started in a state satisfying `inv` satisfies `post`
whenever it returns. -/
theorem loop_inv {α β} (body : α → Result (ControlFlow α β)) (inv : α → Prop) (post : β → Prop)
    (hc : ∀ x x', inv x → body x = ok (ControlFlow.cont x') → inv x')
    (hd : ∀ x y, inv x → body x = ok (ControlFlow.done y) → post y) :
    ∀ x r, inv x → loop body x = ok r → post r := by
  have key : ∀ x, inv x → pspec (loop body x) post := by
    refine loop.fixpoint_induct body (fun f => ∀ x, inv x → pspec (f x) post) ?_ ?_
    · apply admissible_pi
      intro x
      apply admissible_pi
      intro _
      exact admissible_apply (fun _ m => pspec m post) x (pspec_admissible post)
    · intro f ih x hx
      rw [pspec_iff]
      intro r hr
      dsimp only at hr
      cases hb : body x with
      | ret a =>
        rw [hb] at hr
        cases a with
        | cont x' =>
          have hr' : f x' = ok r := by simpa using hr
          exact (pspec_iff _ _).mp (ih x' (hc x x' hx hb)) r hr'
        | done y =>
          have hr' : y = r := by simpa using hr
          rw [← hr']
          exact hd x y hx hb
      | vis e k =>
        rw [hb] at hr
        exact absurd hr (by simp)
      | div =>
        rw [hb] at hr
        exact absurd hr (by simp)
  intro x r hx h
  exact (pspec_iff _ _).mp (key x hx) r h

end P18
