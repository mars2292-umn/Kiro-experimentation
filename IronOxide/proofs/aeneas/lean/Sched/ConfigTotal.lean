import Sched.LayoutTotal
import Sched.CeilBound
open Aeneas Aeneas.Std Result

/-! # `Config::validate` returns (2026-11-20)

`Config::validate` and its two helpers return for every configuration: they
read their tables inside the tables' lengths and compute nothing that can
overflow. A configuration `validate` accepts has every Resource's Ceiling
within the Priority levels (`RC`, which `Sched/CeilBound.lean` needs) and at
least one Partition. What else an accepted configuration gives is in
`Sched/Boot.lean` (`validate_ok`). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NP : Usize}

/-- The first `m` Resources' Ceilings are Priority levels. -/
def RCk (a : Array logic.ResourceCfg NR) (m : Nat) : Prop :=
  ∀ (j : Nat) (h : j < a.val.length), j < m → (a.val[j]).ceiling.val ≤ 7

theorem rck_zero (a : Array logic.ResourceCfg NR) : RCk a 0 :=
  fun _ _ h => absurd h (Nat.not_lt_zero _)

theorem rck_succ (a : Array logic.ResourceCfg NR) (m : Nat) :
    RCk a (m + 1) ↔ RCk a m ∧ ∀ h : m < a.val.length, (a.val[m]).ceiling.val ≤ 7 := by
  constructor
  · intro h
    exact ⟨fun j hj hlt => h j hj (by omega), fun hm => h m hm (by omega)⟩
  · rintro ⟨h1, h2⟩ j hj hlt
    rcases Nat.lt_succ_iff_lt_or_eq.mp hlt with h | h
    · exact h1 j hj h
    · subst h; exact h2 hj

theorem RCk.mono {a : Array logic.ResourceCfg NR} {m m' : Nat} (h : RCk a m) (hle : m' ≤ m) :
    RCk a m' := fun j hj hlt => h j hj (by omega)

/-- Closes the goals a body leaves. -/
macro "ccl" : tactic => `(tactic| all_goals first
  | (scalar_tac; done)
  | assumption
  | (simp_all [rck_succ, rck_zero]; done)
  | (simp_all [rck_succ, rck_zero]; scalar_tac; done)
  | (simp_all [rck_succ, rck_zero]
     refine RCk.mono ?_ ?_ <;> first | assumption | (scalar_tac; done))
  | (simp_all [rck_succ, rck_zero]
     refine ⟨?_, fun _ => ?_, ?_⟩ <;> (try simp only [max_priority] at *) <;> scalar_tac))

set_option maxHeartbeats 16000000 in
@[step] theorem cvalidate_slots_loop0_loop0_t (a a1 a2 : Array U8 NT) (t : Usize) (d b r : U8) (fine : Bool) (u : Usize)
    (a3 a4 a5 : Array U8 NT) :
    logic.Config.validate_slots_loop0_loop0 a a1 a2 t d b r fine u a3 a4 a5 ⦃ r => True ⦄ := by
  unfold logic.Config.validate_slots_loop0_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize × Array U8 NT × Array U8 NT × Array U8 NT) => NT.val - x.2.1.val) (fun x => True)
  · intro x hx
    obtain ⟨f, u1, b3, b4, b5⟩ := x
    simp only at hx ⊢
    unfold logic.Config.validate_slots_loop0_loop0.body
    ctac <;> try ctac
    all_goals try (first | (scalar_tac; done) | (refine ⟨trivial, ?_⟩; scalar_tac; done))
    ccl
  · trivial

set_option maxHeartbeats 16000000 in
@[step] theorem cvalidate_slots_loop0_t (a : Array logic.TaskCfg NT) (a1 a2 a3 : Array U8 NT) (limit : Usize) (good : Bool)
    (t : Usize) :
    logic.Config.validate_slots_loop0 a a1 a2 a3 limit good t ⦃ r => True ⦄ := by
  unfold logic.Config.validate_slots_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => NT.val - x.2.val) (fun x => True)
  · intro x hx
    obtain ⟨g, t1⟩ := x
    simp only at hx ⊢
    unfold logic.Config.validate_slots_loop0.body
    ctac <;> try ctac
    all_goals try (first | (scalar_tac; done) | (refine ⟨trivial, ?_⟩; scalar_tac; done))
    ccl
  · trivial

@[step] theorem cvalidate_slots_t (cfg : logic.Config NT NR NP) :
    logic.Config.validate_slots cfg ⦃ _ => True ⦄ := by
  unfold logic.Config.validate_slots
  ctac
  ccl

set_option maxHeartbeats 16000000 in
@[step] theorem cvalidate_partitions_loop0_loop0_t (a : Array logic.PartitionCfg NP) (i : U8) (fine : Bool) (q : Usize) :
    logic.Config.validate_partitions_loop0_loop0 a i fine q ⦃ r => True ⦄ := by
  unfold logic.Config.validate_partitions_loop0_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => NP.val - x.2.val) (fun x => True)
  · intro x hx
    obtain ⟨f, q1⟩ := x
    simp only at hx ⊢
    unfold logic.Config.validate_partitions_loop0_loop0.body
    ctac <;> try ctac
    all_goals try (first | (scalar_tac; done) | (refine ⟨trivial, ?_⟩; scalar_tac; done))
    ccl
  · trivial

set_option maxHeartbeats 16000000 in
@[step] theorem cvalidate_partitions_loop0_t (cfg : logic.Config NT NR NP) (good : Bool) (p : Usize) :
    logic.Config.validate_partitions_loop0 cfg good p ⦃ r => True ⦄ := by
  unfold logic.Config.validate_partitions_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => NP.val - x.2.val) (fun x => True)
  · intro x hx
    obtain ⟨g, p1⟩ := x
    simp only at hx ⊢
    unfold logic.Config.validate_partitions_loop0.body
    ctac <;> try ctac
    all_goals try (first | (scalar_tac; done) | (refine ⟨trivial, ?_⟩; scalar_tac; done))
    ccl
  · trivial

@[step] theorem cvalidate_partitions_t (cfg : logic.Config NT NR NP) :
    logic.Config.validate_partitions cfg ⦃ _ => True ⦄ := by
  unfold logic.Config.validate_partitions
  ctac

/-- A conditional returns when both branches do (for terms `split` gives up
on). -/
theorem ite_spec {α} {c : Prop} [Decidable c] {x y : Result α} {P : α → Prop}
    (hx : c → x ⦃ P ⦄) (hy : ¬ c → y ⦃ P ⦄) : (if c then x else y) ⦃ P ⦄ := by
  by_cases h : c
  · rw [if_pos h]; exact hx h
  · rw [if_neg h]; exact hy h

theorem lift_bind_spec {α β} {a : α} {f : α → Result β} {P : β → Prop} (h : f a ⦃ P ⦄) :
    (do let v ← lift a; f v) ⦃ P ⦄ := by
  simpa [lift] using h

set_option maxHeartbeats 16000000 in
@[step] theorem cvalidate_loop0_t (a : Array logic.TaskCfg NT) (i : U64) (good : Bool) (t : Usize) :
    logic.Config.validate_loop0 NP a i good t ⦃ _ => True ⦄ := by
  unfold logic.Config.validate_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => NT.val - x.2.val) (fun _ => True)
  · intro x hx
    obtain ⟨g, t1⟩ := x
    simp only at hx ⊢
    unfold logic.Config.validate_loop0.body
    refine ite_spec (fun hlt => ?_) (fun _ => ?_)
    · refine ite_spec (fun _ => ?_) (fun _ => ?_)
      · step as ⟨task, htask⟩
        clear htask
        refine WP.spec_bind (Pₘ := fun _ => True) ?_ (fun tg _ => ?_)
        · repeat' (first
            | exact (WP.spec_ok _).mpr trivial
            | (refine ite_spec (fun _ => ?_) (fun _ => ?_))
            | (refine lift_bind_spec ?_; beta_reduce))
        · ctac
          all_goals (refine ⟨trivial, ?_⟩; scalar_tac)
      · exact (WP.spec_ok _).mpr trivial
    · exact (WP.spec_ok _).mpr trivial
  · trivial

set_option maxHeartbeats 16000000 in
@[step] theorem cvalidate_loop1_loop0_t (a : Array logic.TaskCfg NT) (a1 : Array (Array Bool NR) NT) (r : Usize) (i i1 : U8)
    (t : Usize) (attained fine : Bool) (hr : r.val < NR.val) :
    logic.Config.validate_loop1_loop0 a a1 r i i1 t attained fine ⦃ r => True ⦄ := by
  unfold logic.Config.validate_loop1_loop0
  apply loop.spec_decr_nat (fun (x : Usize × Bool × Bool) => NT.val - x.1.val) (fun x => True)
  · intro x hx
    obtain ⟨t1, at1, f1⟩ := x
    simp only at hx ⊢
    unfold logic.Config.validate_loop1_loop0.body
    ctac <;> try ctac
    all_goals try (first | (scalar_tac; done) | (refine ⟨trivial, ?_⟩; scalar_tac; done))
    ccl
  · trivial

set_option maxHeartbeats 16000000 in
@[step] theorem cvalidate_loop1_t (a : Array logic.TaskCfg NT) (a1 : Array logic.ResourceCfg NR)
    (a2 : Array (Array Bool NR) NT) (good : Bool) (r : Usize) (hr : r.val ≤ NR.val)
    (hinv : good = true → RCk a1 r.val) :
    logic.Config.validate_loop1 NP a a1 a2 good r ⦃ r => r = true → RCk a1 NR.val ⦄ := by
  unfold logic.Config.validate_loop1
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => NR.val - x.2.val) (fun x => x.2.val ≤ NR.val ∧ (x.1 = true → RCk a1 x.2.val))
  · intro x hx
    obtain ⟨g, r1⟩ := x
    simp only at hx ⊢
    unfold logic.Config.validate_loop1.body
    ctac <;> try ctac
    all_goals try (first | (scalar_tac; done) | (refine ⟨trivial, ?_⟩; scalar_tac; done))
    ccl
  · exact ⟨hr, hinv⟩

set_option maxHeartbeats 16000000 in
/-- **`Config::validate` returns for every configuration.** One it accepts
has its Resources' Ceilings within the Priority levels and at least one
Partition. -/
@[step] theorem config_validate_t (cfg : logic.Config NT NR NP) :
    logic.Config.validate cfg ⦃ b => b = true → RCk cfg.resources NR.val ∧ 0 < NP.val ⦄ := by
  unfold logic.Config.validate
  ctac
  ccl

theorem rc_of_rck {cfg : logic.Config NT NR NP} (h : RCk cfg.resources NR.val) : RC cfg := by
  intro i rc hi
  have hlen : cfg.resources.val.length = NR.val := cfg.resources.property
  have hlt : i < cfg.resources.val.length := by
    by_contra hge
    rw [List.getElem?_eq_none (by omega)] at hi
    cases hi
  rw [List.getElem?_eq_getElem hlt] at hi
  have e := Option.some.inj hi
  rw [← e]
  have := h i hlt (by omega)
  omega

end P18
