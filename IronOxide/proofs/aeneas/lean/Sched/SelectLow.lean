import Sched.Nvic
import Sched.Total
open Aeneas Aeneas.Std Result

/-! # `Sched::select` returns the Stack Resource Policy's Job (2026-11-27)

`Sched/Select.lean` proves of the Task `Sched::select` returns that R7.1
admits it and that it has the highest Priority among the pending Tasks.
That it is the lowest Task identifier among those of that Priority (R7.3)
was proven in Verus only, and was a premise in `Sched/Nvic.lean` (the
third clause of `Nv.Srp`). This file proves it of the extracted function
and gives the converse:

- `select_loop_low`, `select_some_low`: every pending Task with a lower
  identifier than the one returned has a strictly lower Priority (the loop
  replaces its candidate only on a strictly higher Priority).
- `select_srp`: the Task `select` returns satisfies `Nv.Srp`, all three
  clauses.
- `srp_unique`: at most one Task satisfies `Nv.Srp`.
- `select_returns`: `select` returns on a scheduler state whose stack
  holds Task identifiers (`Sched/Total.lean`'s proof, which reads no more
  of its invariant than that).
- `select_of_srp`: in a state with the run's structure (`Wf`), the Task
  that satisfies `Nv.Srp` is the one `select` returns.

Handwritten; no `sorry`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- Every pending Task below the candidate has a strictly lower Priority. -/
def SelLow (cfg : logic.Config NT NR NP) (s : logic.sched.Sched NT NR NL) (bt : Nat) : Prop :=
  bt < NT.val → ∀ u, u < bt → rel s u = 1 → prio cfg u < prio cfg bt

theorem select_loop_low (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP) :
    ∀ (n : Nat) (bt t : Usize) res, NT.val - t.val = n → t.val ≤ NT.val →
      SelInv cfg s bt.val t.val → SelLow cfg s bt.val →
      logic.sched.Sched.select_loop s cfg bt t = ok res → SelLow cfg s res.2.2.val := by
  intro n
  induction n with
  | zero =>
    intro bt t res hn htn hinv hlow h
    unfold logic.sched.Sched.select_loop loop at h
    simp only [logic.sched.Sched.select_loop.body] at h
    have hlt : ¬ t < NT := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    exact hlow
  | succ n ih =>
    intro bt t res hn htn hinv hlow h
    unfold logic.sched.Sched.select_loop loop at h
    simp only [logic.sched.Sched.select_loop.body] at h
    have htv : t.val < NT.val := by omega
    have hlt : t < NT := by scalar_tac
    simp only [hlt, ite_true, bind_eq_ok, Prod.exists, Aeneas.Std.uncurry] at h
    obtain ⟨cf, ⟨i, hi, cfg1, bt1, hsel, t1, ht1, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    have ht1 := usize_add_ok ht1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at ht1
    obtain ⟨_, rfl⟩ := index_ok hi
    have hstep : cfg1 = cfg ∧ SelInv cfg s bt1.val t1.val ∧ SelLow cfg s bt1.val := by
      obtain ⟨hb1, hb2, hb3⟩ := hinv
      split at hsel
      · rename_i hp
        have hrel : rel s t.val = 1 := by
          simp only [rel]; rw [hp, rel_pending]; rfl
        split at hsel
        · rename_i hbt
          have e := Prod.mk.inj (ok_inj hsel)
          obtain ⟨rfl, rfl⟩ := e
          have hbv : bt.val = NT.val := by rw [hbt]
          refine ⟨rfl, ⟨by omega, fun hx => by omega, fun _ => ⟨hrel, ?_⟩⟩, ?_⟩
          · intro u hu hur
            by_cases hut : u = t.val
            · rw [hut]
            · exact absurd hur (hb2 hbv u (by omega))
          · intro _ u hu hur
            exact absurd hur (hb2 hbv u hu)
        · rename_i hbt
          have hbv : bt.val < NT.val := by
            have : bt.val ≠ NT.val := fun e => hbt (by scalar_tac)
            omega
          simp only [bind_eq_ok] at hsel
          obtain ⟨tc, htc, tc1, htc1, i1, hi1, hsel⟩ := hsel
          have e := Prod.mk.inj (ok_inj hsel)
          obtain ⟨rfl, rfl⟩ := e
          have hpt := prio_of_index htc
          have hpb := prio_of_index htc1
          obtain ⟨hbr, hbm⟩ := hb3 hbv
          split at hi1
          · rename_i hgt
            have := ok_inj hi1
            subst this
            have hgt' : tc1.priority.val < tc.priority.val := by scalar_tac
            refine ⟨rfl, ⟨by omega, fun hx => by omega, fun _ => ⟨hrel, ?_⟩⟩, ?_⟩
            · intro u hu hur
              by_cases hut : u = t.val
              · rw [hut]
              · have := hbm u (by omega) hur
                omega
            · intro _ u hu hur
              have := hbm u hu hur
              omega
          · rename_i hgt
            have := ok_inj hi1
            subst this
            have hgt' : tc.priority.val ≤ tc1.priority.val := by scalar_tac
            refine ⟨rfl, ⟨hb1, fun hx => by omega, fun _ => ⟨hbr, ?_⟩⟩, hlow⟩
            intro u hu hur
            by_cases hut : u = t.val
            · rw [hut]; omega
            · exact hbm u (by omega) hur
      · rename_i hp
        have e := Prod.mk.inj (ok_inj hsel)
        obtain ⟨rfl, rfl⟩ := e
        have hrel : rel s t.val ≠ 1 := by
          intro hx
          apply hp
          simp only [rel] at hx
          rw [rel_pending]
          scalar_tac
        refine ⟨rfl, ⟨hb1, fun hx u hu => ?_, fun hx => ⟨(hb3 hx).1, fun u hu hur => ?_⟩⟩, hlow⟩
        · by_cases hut : u = t.val
          · rw [hut]; exact hrel
          · exact hb2 hx u (by omega)
        · by_cases hut : u = t.val
          · rw [hut] at hur; exact absurd hur hrel
          · exact (hb3 hx).2 u (by omega) hur
    obtain ⟨rfl, hinv', hlow'⟩ := hstep
    have hrec : logic.sched.Sched.select_loop s cfg1 bt1 t1 = ok res := by
      unfold logic.sched.Sched.select_loop
      simp only [logic.sched.Sched.select_loop.body]
      exact hloop
    exact ih bt1 t1 res (by omega) (by omega) hinv' hlow' hrec

/-- **Every pending Task with a lower identifier than the Task `select`
returns has a strictly lower Priority.** -/
theorem select_some_low {s : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    (hNT : NT.val ≤ 256) {t : U8} (h : logic.sched.Sched.select s cfg = ok (some t)) :
    ∀ u, u < t.val → rel s u = 1 → prio cfg u < prio cfg t.val := by
  unfold logic.sched.Sched.select at h
  simp only [bind_eq_ok, Prod.exists, Aeneas.Std.uncurry] at h
  obtain ⟨s1, cfg1, bt, hloop, best, hbest, h⟩ := h
  have hinv0 : SelInv cfg s NT.val (0#usize).val :=
    ⟨Nat.le_refl _, fun _ u hu => by simp at hu, fun hx => absurd hx (Nat.lt_irrefl _)⟩
  have hlow := select_loop_low s cfg _ NT 0#usize _ rfl (by scalar_tac) hinv0
    (fun hx => absurd hx (Nat.lt_irrefl _)) hloop
  dsimp only at hlow
  split at hbest
  · rename_i hlt
    have hbv : bt.val < NT.val := by scalar_tac
    simp only [lift, bind_ok] at hbest
    have hbest := ok_inj hbest
    subst hbest
    have hcv : (UScalar.cast .U8 bt).val = bt.val := by
      have : bt.val < 256 := by omega
      scalar_tac
    simp only [lift, bind_ok, bind_eq_ok] at h
    obtain ⟨tc, htc, above, habove, i2, hi2, h⟩ := h
    have key : some (UScalar.cast .U8 bt) = some t := by
      split at h
      · split at h
        · split at h
          · exact ok_inj h
          · cases ok_inj h
        · cases ok_inj h
      · cases ok_inj h
    cases key
    intro u hu hur
    rw [hcv] at hu ⊢
    exact hlow hbv u hu hur
  · have hbest := ok_inj hbest
    subst hbest
    cases ok_inj h

/-- **The Task `Sched::select` returns is the Job the Stack Resource Policy
starts**: admitted, of the highest Priority among the pending Releases,
the lowest Task identifier among equals. -/
theorem select_srp {s : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    (hNT : NT.val ≤ 256) {t : U8} (h : logic.sched.Sched.select s cfg = ok (some t)) :
    Nv.Srp cfg s t.val := by
  obtain ⟨he, hmax⟩ := Nv.select_is_srp hNT h
  refine ⟨he, hmax, fun u _ hur hpe => ?_⟩
  by_contra hlt
  have := select_some_low hNT h u (by omega) hur
  omega

/-- At most one Task satisfies `Nv.Srp`. -/
theorem srp_unique {s : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {t u : Nat}
    (ht : Nv.Srp cfg s t) (hu : Nv.Srp cfg s u) : t = u := by
  obtain ⟨het, hmt, hlt⟩ := ht
  obtain ⟨heu, hmu, hlu⟩ := hu
  obtain ⟨htn, htr, _⟩ := elig_views.mp het
  obtain ⟨hun, hur, _⟩ := elig_views.mp heu
  have h1 := hmt u hun hur
  have h2 := hmu t htn htr
  have h3 := hlt u hun hur (by omega)
  have h4 := hlu t htn htr (by omega)
  omega

/-- `Sched::select` returns on a scheduler state whose stack holds Task
identifiers. -/
theorem select_returns (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (h1 : s.depth.val ≤ NT.val) (h2 : ∀ p, p < s.depth.val → stk s p < NT.val)
    (hNT : NT.val ≤ 256) : ∃ o, logic.sched.Sched.select s cfg = ok o := by
  have ht : logic.sched.Sched.select s cfg ⦃ _ => True ⦄ := by
    unfold logic.sched.Sched.select
    refine WP.spec_bind (select_loop_total s cfg NT 0#usize (Nat.le_refl _) (by scalar_tac))
      (fun x hx => ?_)
    obtain ⟨s1, cfg1, bt⟩ := x
    obtain ⟨e1, e2, hbt⟩ := hx
    try dsimp only at e1
    try dsimp only at e2
    try dsimp only at hbt
    subst e1; subst e2
    simp only [Aeneas.Std.uncurry, lift, bind_ok]
    by_cases hlt : bt < NT
    · simp only [hlt, ite_true, bind_ok]
      have hlt' : bt.val < NT.val := by scalar_tac
      have hc : (UScalar.cast .Usize (UScalar.cast .U8 bt)).val = bt.val := by scalar_tac
      step as ⟨ tc, htc ⟩
      split
      · try simp only [bind_ok]
        step as ⟨ i2, hi2 ⟩
        ok_tac
      · rename_i hd
        have hdp : 0 < s1.depth.val := by scalar_tac
        step as ⟨ i1, hi1 ⟩
        have hi1' : i1.val = s1.depth.val - 1 := by scalar_tac
        have hlen : i1.val < s1.stack.val.length := by have := s1.stack.property; scalar_tac
        step as ⟨ i2, hi2 ⟩
        have hi2' : s1.stack.val[i1.val]! = i2 := by
          rw [hi2]; exact getElem!_pos s1.stack.val i1.val hlen
        have hst := h2 i1.val (by omega)
        simp only [stk] at hst
        rw [hi2'] at hst
        step as ⟨ tc1, htc1 ⟩
        split
        · try simp only [bind_ok]
          step as ⟨ i3, hi3 ⟩
          ok_tac
        · try simp only [bind_ok]
          step as ⟨ i3, hi3 ⟩
          ok_tac
    · simp only [hlt, ite_false, bind_ok]
      exact (WP.spec_ok _).mpr trivial
  obtain ⟨o, ho, _⟩ := spec_returns ht
  exact ⟨o, ho⟩

/-- **In a state with the run's structure, the Task that satisfies
`Nv.Srp` is the one `Sched::select` returns.** -/
theorem select_of_srp {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) (hNT : NT.val ≤ 256) {t : U8} (h : Nv.Srp cfg st.s t.val) :
    logic.sched.Sched.select st.s cfg = ok (some t) := by
  obtain ⟨o, ho⟩ := select_returns st.s cfg w.depth_le w.stk_lt hNT
  cases o with
  | none => exact absurd h.1 (select_none hNT ho (wf_below_top w) t.val)
  | some t' =>
    have e : t'.val = t.val := srp_unique (select_srp hNT ho) h
    have : t' = t := by scalar_tac
    rw [ho, this]

end P18
