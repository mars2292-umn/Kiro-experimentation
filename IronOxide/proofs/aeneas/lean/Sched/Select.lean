import Sched.Calls
open Aeneas Aeneas.Std Result

/-! # `Sched::select` as extracted

The Task the Kernel pends for dispatch: the pending Task of highest
Priority if R7.1 admits it, and none otherwise. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The loop's invariant after the Tasks below `t`: `bt` is the Task count
while none of them is pending, and otherwise a pending Task of the highest
Priority among them. -/
def SelInv (cfg : logic.Config NT NR NP) (s : logic.sched.Sched NT NR NL) (bt t : Nat) : Prop :=
  bt ≤ NT.val ∧ (bt = NT.val → ∀ u, u < t → rel s u ≠ 1) ∧
  (bt < NT.val → rel s bt = 1 ∧ ∀ u, u < t → rel s u = 1 → prio cfg u ≤ prio cfg bt)

theorem prio_of_index {cfg : logic.Config NT NR NP} {i : Usize} {tc : logic.TaskCfg}
    (h : cfg.tasks.index_usize i = ok tc) : prio cfg i.val = tc.priority.val := by
  obtain ⟨_, h'⟩ := index_ok' h
  simp [prio, h']

theorem select_loop_ok (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP) :
    ∀ (n : Nat) (bt t : Usize) res, NT.val - t.val = n → t.val ≤ NT.val →
      SelInv cfg s bt.val t.val → logic.sched.Sched.select_loop s cfg bt t = ok res →
      res.1 = s ∧ res.2.1 = cfg ∧ SelInv cfg s res.2.2.val NT.val := by
  intro n
  induction n with
  | zero =>
    intro bt t res hn htn hinv h
    unfold logic.sched.Sched.select_loop loop at h
    simp only [logic.sched.Sched.select_loop.body] at h
    have hlt : ¬ t < NT := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    have : t.val = NT.val := by omega
    rw [this] at hinv
    exact ⟨rfl, rfl, hinv⟩
  | succ n ih =>
    intro bt t res hn htn hinv h
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
    have hstep : cfg1 = cfg ∧ SelInv cfg s bt1.val t1.val := by
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
          refine ⟨rfl, by omega, fun hx => by omega, fun _ => ⟨hrel, ?_⟩⟩
          intro u hu hur
          by_cases hut : u = t.val
          · rw [hut]
          · exact absurd hur (hb2 hbv u (by omega))
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
            refine ⟨rfl, by omega, fun hx => by omega, fun _ => ⟨hrel, ?_⟩⟩
            intro u hu hur
            by_cases hut : u = t.val
            · rw [hut]
            · have := hbm u (by omega) hur
              omega
          · rename_i hgt
            have := ok_inj hi1
            subst this
            have hgt' : tc.priority.val ≤ tc1.priority.val := by scalar_tac
            refine ⟨rfl, hb1, fun hx => by omega, fun _ => ⟨hbr, ?_⟩⟩
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
        refine ⟨rfl, hb1, fun hx u hu => ?_, fun hx => ⟨(hb3 hx).1, fun u hu hur => ?_⟩⟩
        · by_cases hut : u = t.val
          · rw [hut]; exact hrel
          · exact hb2 hx u (by omega)
        · by_cases hut : u = t.val
          · rw [hut] at hur; exact absurd hur hrel
          · exact (hb3 hx).2 u (by omega) hur
    obtain ⟨rfl, hinv'⟩ := hstep
    have hrec : logic.sched.Sched.select_loop s cfg1 bt1 t1 = ok res := by
      unfold logic.sched.Sched.select_loop
      simp only [logic.sched.Sched.select_loop.body]
      exact hloop
    exact ih bt1 t1 res (by omega) (by omega) hinv' hrec

/-- What `Sched::select` returns, when it returns: with no pending Task,
none; otherwise, for a pending Task `b` of the highest Priority among the
pending ones, `b` if its Priority is above the System_Ceiling and above
that of the Job on top and it has no Job in progress, and none if not. -/
theorem select_cases {s : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    (hNT : NT.val ≤ 256) {o : Option U8} (h : logic.sched.Sched.select s cfg = ok o) :
    (o = none ∧ ∀ u, u < NT.val → rel s u ≠ 1) ∨
    ∃ b : U8, b.val < NT.val ∧ rel s b.val = 1 ∧
      (∀ u, u < NT.val → rel s u = 1 → prio cfg u ≤ prio cfg b.val) ∧
      ((o = some b ∧ s.ceiling.val < prio cfg b.val ∧
          (s.depth.val = 0 ∨ prio cfg (stk s (s.depth.val - 1)) < prio cfg b.val) ∧
          job s b.val = 0) ∨
       (o = none ∧ (prio cfg b.val ≤ s.ceiling.val ∨
          (s.depth.val ≠ 0 ∧ prio cfg b.val ≤ prio cfg (stk s (s.depth.val - 1))) ∨
          job s b.val ≠ 0))) := by
  unfold logic.sched.Sched.select at h
  simp only [bind_eq_ok, Prod.exists, Aeneas.Std.uncurry] at h
  obtain ⟨s1, cfg1, bt, hloop, best, hbest, h⟩ := h
  obtain ⟨e1, e2, hb1, hb2, hb3⟩ := select_loop_ok s cfg _ NT 0#usize _ rfl (by scalar_tac)
    ⟨Nat.le_refl _, fun _ u hu => by simp at hu, fun hx => absurd hx (Nat.lt_irrefl _)⟩ hloop
  dsimp only at e1 e2 hb1 hb2 hb3
  subst s1 cfg1
  split at hbest
  · rename_i hlt
    have hbv : bt.val < NT.val := by scalar_tac
    simp only [lift, bind_ok] at hbest
    have hbest := ok_inj hbest
    subst hbest
    obtain ⟨hrel, hmax⟩ := hb3 hbv
    have hcv : (UScalar.cast .U8 bt).val = bt.val := by
      have : bt.val < 256 := by omega
      scalar_tac
    refine Or.inr ⟨UScalar.cast .U8 bt, by omega, by rw [hcv]; exact hrel,
      fun u hu hur => by rw [hcv]; exact hmax u hu hur, ?_⟩
    simp only [lift, bind_ok, bind_eq_ok] at h
    obtain ⟨tc, htc, above, habove, i2, hi2, h⟩ := h
    have hp := prio_of_index htc
    rw [U8.cast_Usize_val_eq] at hp
    obtain ⟨_, rfl⟩ := index_ok hi2
    rw [U8.cast_Usize_val_eq] at h
    have habv : (above = true ∧ (s.depth.val = 0 ∨
          prio cfg (stk s (s.depth.val - 1)) < prio cfg (UScalar.cast .U8 bt).val)) ∨
        (above = false ∧ s.depth.val ≠ 0 ∧
          prio cfg (UScalar.cast .U8 bt).val ≤ prio cfg (stk s (s.depth.val - 1))) := by
      split at habove
      · rename_i hd
        have := ok_inj habove
        subst this
        exact Or.inl ⟨rfl, Or.inl (by rw [hd]; rfl)⟩
      · rename_i hd
        simp only [bind_eq_ok] at habove
        obtain ⟨d1, hd1, top, htop, tc1, htc1, habove⟩ := habove
        obtain ⟨hd1a, hd1b⟩ := usize_sub_ok hd1
        have h1 : (1#usize).val = 1 := rfl
        rw [h1] at hd1a hd1b
        obtain ⟨_, rfl⟩ := index_ok htop
        have hp1 := prio_of_index htc1
        rw [U8.cast_Usize_val_eq, hd1b] at hp1
        have hdne : s.depth.val ≠ 0 := by omega
        split at habove
        · rename_i hgt
          have := ok_inj habove
          subst this
          refine Or.inl ⟨rfl, Or.inr ?_⟩
          simp only [stk]
          rw [hp1, hp]
          scalar_tac
        · rename_i hgt
          have := ok_inj habove
          subst this
          refine Or.inr ⟨rfl, hdne, ?_⟩
          simp only [stk]
          rw [hp1, hp]
          scalar_tac
    split at h
    · rename_i hc
      have hc' : s.ceiling.val < prio cfg (UScalar.cast .U8 bt).val := by rw [hp]; scalar_tac
      split at h
      · rename_i ha
        rcases habv with ⟨_, hat⟩ | ⟨haf, _⟩
        · split at h
          · rename_i hj
            have h := ok_inj h
            refine Or.inl ⟨h.symm, hc', hat, ?_⟩
            simp only [job]
            rw [hj, job_none]; rfl
          · rename_i hj
            have h := ok_inj h
            refine Or.inr ⟨h.symm, Or.inr (Or.inr ?_)⟩
            intro hx
            apply hj
            simp only [job] at hx
            rw [job_none]
            scalar_tac
        · rw [haf] at ha; cases ha
      · rename_i ha
        have h := ok_inj h
        rcases habv with ⟨hat, _⟩ | ⟨_, hd, hle⟩
        · exact absurd hat ha
        · exact Or.inr ⟨h.symm, Or.inr (Or.inl ⟨hd, hle⟩)⟩
    · rename_i hc
      have h := ok_inj h
      refine Or.inr ⟨h.symm, Or.inl ?_⟩
      rw [hp]; scalar_tac
  · rename_i hlt
    have hbest := ok_inj hbest
    subst hbest
    have h := ok_inj h
    have hbv : bt.val = NT.val := by
      have : ¬ bt.val < NT.val := by scalar_tac
      omega
    exact Or.inl ⟨h.symm, hb2 hbv⟩

/-- The Task `select` returns is one R7.1 admits, of the highest Priority
among those it admits. -/
theorem select_some {s : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    (hNT : NT.val ≤ 256) {t : U8} (h : logic.sched.Sched.select s cfg = ok (some t)) :
    Elig cfg s t.val ∧ ∀ u, Elig cfg s u → prio cfg u ≤ prio cfg t.val := by
  rcases select_cases hNT h with ⟨h0, _⟩ | ⟨b, hb, hrel, hmax, ⟨hs, hc, htop, hj⟩ | ⟨hn, _⟩⟩
  · cases h0
  · cases hs
    refine ⟨elig_views.mpr ⟨hb, hrel, hj, hc, htop⟩, ?_⟩
    intro u hu
    obtain ⟨hun, hur, _⟩ := elig_views.mp hu
    exact hmax u hun hur
  · cases hn

/-- When `select` returns none, R7.1 admits no Task, provided a Task with
a Job in progress or killed has at most the Priority of the Job on top of
the stack (the structure of the stack in a run, `Wf`). -/
theorem select_none {s : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    (hNT : NT.val ≤ 256) (h : logic.sched.Sched.select s cfg = ok none)
    (hst : ∀ k, k < NT.val → job s k ≠ 0 →
      0 < s.depth.val ∧ prio cfg k ≤ prio cfg (stk s (s.depth.val - 1))) :
    ∀ u, ¬ Elig cfg s u := by
  intro u hu
  obtain ⟨hun, hur, huj, huc, hut⟩ := elig_views.mp hu
  rcases select_cases hNT h with ⟨_, h0⟩ | ⟨b, hb, hrel, hmax, ⟨hs, _⟩ | ⟨_, hc | ⟨hd, hle⟩ | hj⟩⟩
  · exact h0 u hun hur
  · cases hs
  · have := hmax u hun hur
    omega
  · have := hmax u hun hur
    rcases hut with h0 | h1 <;> omega
  · have := hmax u hun hur
    obtain ⟨hd, hle⟩ := hst b.val hb hj
    rcases hut with h0 | h1 <;> omega

end P18
