import Aeneas
import Tools.Link
import Tools.LinkFrame
open Aeneas Aeneas.Std Result

/-! # The Link_Checker's nodes and bounds on the code as compiled (2026-11-27)

`crates/rsk-link/src/bound_v.rs` is verified in Verus. This file proves
the same of the functions as Aeneas extracts them (`Tools/Link.lean`), and
joins the frame to the frame model:

- `frame_of_spec`: the compiled `frame_of` returns, and its result is the
  sum of the decrements the decoder read in the body, as
  `Tools/LinkFrame.lean` defines it (`downs`), saturating at the largest
  32-bit value. `frame_of_bounds`: so along a run of the body's
  instructions, each at most once, SP falls by no more than the frame
  `frame_of` returned when that is below the largest value (the join of
  `descent_le_frame` to the compiled function, which the survey listed as
  missing). The bound for runs that pass an instruction more than once is
  the certificate's (`Tools/LinkFlowProofs.lean`).
- `dynamic_of_spec`: `true` exactly when an instruction of the body was
  read as `SpDynamic`.
- `unread_of_spec`: the addresses of the instructions read as `SpUnread`,
  in order, and nothing else.
- `edge_of_spec`, `direct_of_spec`: the targets of the body's calls and of
  its branches and `cbz` that leave `[address, end)`, in order.
- `msp_value_spec`: the three cases of R37.2 and the largest of them, or
  `none` exactly when one of the three does not fit 64 bits.
- `jobs_bound_spec`: the sum over the Jobs of the wrapper's usage and the
  frames stacked, or `none` exactly when it does not fit 64 bits.

The statements are equalities of lists where Verus states membership.
Handwritten; no `sorry`. -/

namespace rsk_link

open LinkThumb

instance : Inhabited thumb.Insn := ⟨⟨0#u32, 0#u32, .Other⟩⟩

/-- What the decoder read of the instructions of a body. -/
def effs (insns : List thumb.Insn) : List AEff := insns.map (fun x => absE x.effect)

theorem take_succ_getElem! {α} [Inhabited α] (l : List α) (i : Nat) (h : i < l.length) :
    l.take (i + 1) = l.take i ++ [l[i]!] := by
  rw [List.take_succ_eq_append_getElem h, getElem!_pos l i h]

theorem u32_sat (x y : U32) :
    (core.num.U32.saturating_add x y).val = min 4294967295 (x.val + y.val) := by
  unfold core.num.U32.saturating_add UScalar.saturating_add
  simp only [UScalar.val]
  have hm : U32.max = 4294967295 := by scalar_tac
  have hx : x.val ≤ 4294967295 := by scalar_tac
  have hy : y.val ≤ 4294967295 := by scalar_tac
  simp only [UScalar.val] at hx hy
  simp
  rw [hm]
  omega

/-- Closes a loop step's goal (the invariant's two parts and the measure)
from the three facts, in whichever grouping `step` left it. -/
macro "close3 " k:term : tactic =>
  `(tactic| (obtain ⟨⟨ka, kb⟩, kc⟩ := $k
             first
               | exact ⟨⟨ka, kb⟩, kc⟩
               | exact ⟨ka, kb, kc⟩
               | exact ⟨kb, kc⟩
               | exact kb))

theorem downs_append (a b : List AEff) : downs (a ++ b) = downs a + downs b := by
  simp [downs]

/-! ## The frame -/

theorem frame_of_spec (insns : alloc.vec.Vec thumb.Insn) :
    bound_v.frame_of insns ⦃ f => f.val = min 4294967295 (downs (effs insns.val)) ⦄ := by
  unfold bound_v.frame_of bound_v.frame_of_loop
  apply loop.spec_decr_nat (fun (s : U32 × Usize) => insns.val.length - s.2.val)
    (fun s => s.2.val ≤ insns.val.length ∧
      s.1.val = min 4294967295 (downs (effs (insns.val.take s.2.val))))
  · intro s hs
    obtain ⟨f, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold bound_v.frame_of_loop.body
    by_cases hlt : i.val < insns.val.length
    · rw [if_pos (by scalar_tac)]
      step as ⟨ x, hx ⟩
      have ex : x = insns.val[i.val]! := by
        rw [hx]; exact (getElem!_pos insns.val i.val hlt).symm
      have key : ∀ (f1 : U32) (i3 : Usize), f1.val = min 4294967295 (f.val + dn (absE x.effect)) →
          i3.val = i.val + 1 → (i3.val ≤ insns.val.length ∧
            f1.val = min 4294967295 (downs (effs (insns.val.take i3.val)))) ∧
            insns.val.length - i3.val < insns.val.length - i.val := by
        intro f1 i3 hf1 hi3
        refine ⟨⟨by omega, ?_⟩, by omega⟩
        rw [hi3, take_succ_getElem! _ _ hlt, ← ex]
        simp only [effs, List.map_append, List.map_cons, List.map_nil] at h2 ⊢
        rw [downs_append]
        simp only [downs, List.map_cons, List.map_nil, List.sum_cons, List.sum_nil, Nat.add_zero] at h2 ⊢
        omega
      cases he : x.effect with
      | SpDown n =>
        simp only [bind_ok]
        step as ⟨ i3, hi3 ⟩
        close3 (key (core.num.U32.saturating_add f n) i3 (by rw [u32_sat]; simp [he, absE, dn]) hi3)
      | _ =>
        simp only [bind_ok]
        step as ⟨ i3, hi3 ⟩
        close3 (key f i3 (by simp [he, absE, dn]; scalar_tac) hi3)
    · rw [if_neg (by scalar_tac)]
      simp only [WP.spec_ok]
      have : i.val = insns.val.length := by omega
      rw [this, List.take_length] at h2
      exact h2
  · exact ⟨by scalar_tac, by simp [effs, downs]⟩

/-- **The frame the compiled `frame_of` returns bounds the descent** of a
run of the body's instructions, each at most once. -/
theorem frame_of_bounds (insns : alloc.vec.Vec thumb.Insn) :
    bound_v.frame_of insns ⦃ f => f.val < 4294967295 →
      ∀ (top : Nat) (es : List AEff) (s u : Arm.St), RunBelow top es s u →
        es.Sublist (effs insns.val) → s.sp.toNat ≤ u.sp.toNat + f.val ⦄ := by
  refine WP.spec_mono (frame_of_spec insns) ?_
  intro f hf hlt top es s u hrun hsub
  have a := descent_le_frame hrun hsub
  omega

/-! ## The dynamic mark -/

theorem dynamic_of_spec (insns : alloc.vec.Vec thumb.Insn) :
    bound_v.dynamic_of insns ⦃ d => d = true ↔ ∃ x ∈ insns.val, x.effect = .SpDynamic ⦄ := by
  unfold bound_v.dynamic_of bound_v.dynamic_of_loop
  apply loop.spec_decr_nat (fun (s : Bool × Usize) => insns.val.length - s.2.val)
    (fun s => s.2.val ≤ insns.val.length ∧
      (s.1 = true ↔ ∃ x ∈ insns.val.take s.2.val, x.effect = .SpDynamic))
  · intro s hs
    obtain ⟨d, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold bound_v.dynamic_of_loop.body
    by_cases hlt : i.val < insns.val.length
    · rw [if_pos (by scalar_tac)]
      step as ⟨ x, hx ⟩
      have ex : x = insns.val[i.val]! := by
        rw [hx]; exact (getElem!_pos insns.val i.val hlt).symm
      have key : ∀ (d1 : Bool) (i3 : Usize), (d1 = true ↔ d = true ∨ x.effect = .SpDynamic) →
          i3.val = i.val + 1 → (i3.val ≤ insns.val.length ∧
            (d1 = true ↔ ∃ y ∈ insns.val.take i3.val, y.effect = .SpDynamic)) ∧
            insns.val.length - i3.val < insns.val.length - i.val := by
        intro d1 i3 hd1 hi3
        refine ⟨⟨by omega, ?_⟩, by omega⟩
        rw [hi3, take_succ_getElem! _ _ hlt, ← ex, hd1, h2]
        simp only [List.mem_append, List.mem_singleton]
        constructor
        · rintro (⟨y, hy, he⟩ | he)
          · exact ⟨y, Or.inl hy, he⟩
          · exact ⟨x, Or.inr rfl, he⟩
        · rintro ⟨y, hy | rfl, he⟩
          · exact Or.inl ⟨y, hy, he⟩
          · exact Or.inr he
      cases he : x.effect with
      | SpDynamic =>
        simp only [bind_ok]
        step as ⟨ i3, hi3 ⟩
        obtain ⟨⟨ka, kb⟩, kc⟩ := key true i3 (by simp [he]) hi3
        simp only [true_iff, eq_self_iff_true] at kb ⊢
        first | exact ⟨ka, kb, kc⟩ | exact ⟨kb, kc⟩
      | _ =>
        simp only [bind_ok]
        step as ⟨ i3, hi3 ⟩
        close3 (key d i3 (by simp [he]) hi3)
    · rw [if_neg (by scalar_tac)]
      simp only [WP.spec_ok]
      have : i.val = insns.val.length := by omega
      rw [this, List.take_length] at h2
      exact h2
  · exact ⟨by scalar_tac, by simp⟩

/-! ## The unread instructions -/

/-- The address of an instruction read as `SpUnread`. -/
def unreadF (x : thumb.Insn) : Option U32 :=
  match x.effect with
  | .SpUnread => some x.address
  | _ => none

theorem unread_of_spec (insns : alloc.vec.Vec thumb.Insn) :
    bound_v.unread_of insns ⦃ v => v.val = insns.val.filterMap unreadF ⦄ := by
  unfold bound_v.unread_of bound_v.unread_of_loop
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec U32 × Usize) => insns.val.length - s.2.val)
    (fun s => s.2.val ≤ insns.val.length ∧
      s.1.val = (insns.val.take s.2.val).filterMap unreadF)
  · intro s hs
    obtain ⟨v, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold bound_v.unread_of_loop.body
    by_cases hlt : i.val < insns.val.length
    · rw [if_pos (by scalar_tac)]
      step as ⟨ x, hx ⟩
      have ex : x = insns.val[i.val]! := by
        rw [hx]; exact (getElem!_pos insns.val i.val hlt).symm
      have hvl : v.val.length ≤ i.val := by
        rw [h2]; exact (List.length_filterMap_le _ _).trans (by simp <;> omega)
      have key : ∀ (v1 : alloc.vec.Vec U32) (i3 : Usize), v1.val = v.val ++ (unreadF x).toList →
          i3.val = i.val + 1 → (i3.val ≤ insns.val.length ∧
            v1.val = (insns.val.take i3.val).filterMap unreadF) ∧
            insns.val.length - i3.val < insns.val.length - i.val := by
        intro v1 i3 hv1 hi3
        refine ⟨⟨by omega, ?_⟩, by omega⟩
        rw [hi3, take_succ_getElem! _ _ hlt, ← ex, List.filterMap_append, ← h2, hv1]
        cases hu : unreadF x <;> simp [hu]
      cases he : x.effect with
      | SpUnread =>
        simp only []
        step as ⟨ v1, hv1 ⟩
        step as ⟨ i3, hi3 ⟩
        close3 (key v1 i3 (by rw [hv1]; simp [unreadF, he]) hi3)
      | _ =>
        simp only [bind_ok]
        step as ⟨ i3, hi3 ⟩
        close3 (key v i3 (by simp [unreadF, he]) hi3)
    · rw [if_neg (by scalar_tac)]
      simp only [WP.spec_ok]
      have : i.val = insns.val.length := by omega
      rw [this, List.take_length] at h2
      exact h2
  · exact ⟨by scalar_tac, by simp⟩

/-! ## The direct edges -/

/-- The edge an instruction of the body `[address, end)` makes: the target
of a call, or of a branch or `cbz` that leaves the body. `bound_v::edge`. -/
def edgeF (address «end» : U32) (e : thumb.Effect) : Option U32 :=
  match e with
  | .Call t => some t
  | .Branch t => if t.val < address.val ∨ «end».val ≤ t.val then some t else none
  | .CompareBranch t => if t.val < address.val ∨ «end».val ≤ t.val then some t else none
  | _ => none

theorem edge_of_spec (e : thumb.Effect) (address «end» : U32) :
    bound_v.edge_of e address «end» = ok (edgeF address «end» e) := by
  unfold bound_v.edge_of edgeF
  cases e <;> simp only []
  all_goals
    split
    · rename_i h
      have h' : _ := h
      rw [if_pos (Or.inl (by scalar_tac))]
    · rename_i h
      split
      · rename_i h2
        rw [if_pos (Or.inr (by scalar_tac))]
      · rename_i h2
        rw [if_neg (by rintro (a | a) <;> scalar_tac)]

theorem direct_of_spec (insns : alloc.vec.Vec thumb.Insn) (address «end» : U32) :
    bound_v.direct_of insns address «end»
      ⦃ v => v.val = insns.val.filterMap (fun x => edgeF address «end» x.effect) ⦄ := by
  unfold bound_v.direct_of bound_v.direct_of_loop
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec U32 × Usize) => insns.val.length - s.2.val)
    (fun s => s.2.val ≤ insns.val.length ∧
      s.1.val = (insns.val.take s.2.val).filterMap (fun x => edgeF address «end» x.effect))
  · intro s hs
    obtain ⟨v, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold bound_v.direct_of_loop.body
    by_cases hlt : i.val < insns.val.length
    · rw [if_pos (by scalar_tac)]
      step as ⟨ x, hx ⟩
      have ex : x = insns.val[i.val]! := by
        rw [hx]; exact (getElem!_pos insns.val i.val hlt).symm
      have hvl : v.val.length ≤ i.val := by
        rw [h2]; exact (List.length_filterMap_le _ _).trans (by simp <;> omega)
      rw [edge_of_spec]
      simp only [bind_ok]
      have key : ∀ (v1 : alloc.vec.Vec U32) (i3 : Usize),
          v1.val = v.val ++ (edgeF address «end» x.effect).toList →
          i3.val = i.val + 1 → (i3.val ≤ insns.val.length ∧
            v1.val = (insns.val.take i3.val).filterMap (fun x => edgeF address «end» x.effect)) ∧
            insns.val.length - i3.val < insns.val.length - i.val := by
        intro v1 i3 hv1 hi3
        refine ⟨⟨by omega, ?_⟩, by omega⟩
        rw [hi3, take_succ_getElem! _ _ hlt, ← ex, List.filterMap_append, ← h2, hv1]
        cases hu : edgeF address «end» x.effect <;> simp [hu]
      cases ho : edgeF address «end» x.effect with
      | none =>
        simp only [bind_ok]
        step as ⟨ i3, hi3 ⟩
        close3 (key v i3 (by simp [ho]) hi3)
      | some t =>
        simp only []
        step as ⟨ v1, hv1 ⟩
        step as ⟨ i3, hi3 ⟩
        close3 (key v1 i3 (by rw [hv1]; simp [ho]) hi3)
    · rw [if_neg (by scalar_tac)]
      simp only [WP.spec_ok]
      have : i.val = insns.val.length := by omega
      rw [this, List.take_length] at h2
      exact h2
  · exact ⟨by scalar_tac, by simp⟩

/-! ## The bounds -/

/-- The sum over the Jobs of a Partition's Tasks: the wrapper's usage and
the frames stacked for the Job. -/
def jobsSum (jobs : List (U32 × U32)) : Nat := (jobs.map (fun j => j.1.val + j.2.val)).sum

theorem jobsSum_take_le (jobs : List (U32 × U32)) (k : Nat) :
    jobsSum (jobs.take k) ≤ jobsSum jobs := by
  unfold jobsSum
  have := List.sum_take_add_sum_drop (jobs.map (fun j => j.1.val + j.2.val)) k
  rw [← List.map_take] at this
  omega

theorem jobs_bound_spec (jobs : alloc.vec.Vec (U32 × U32)) :
    bound_v.jobs_bound jobs ⦃ r =>
      match r with
      | some b => b.val = jobsSum jobs.val
      | none => U64.max < jobsSum jobs.val ⦄ := by
  unfold bound_v.jobs_bound bound_v.jobs_bound_loop
  apply loop.spec_decr_nat (fun (s : U64 × Usize) => jobs.val.length - s.2.val)
    (fun s => s.2.val ≤ jobs.val.length ∧ s.1.val = jobsSum (jobs.val.take s.2.val))
  · intro s hs
    obtain ⟨b, k⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold bound_v.jobs_bound_loop.body
    by_cases hlt : k.val < jobs.val.length
    · rw [if_pos (by scalar_tac)]
      step as ⟨ j1, j2, hj ⟩
      have ej : (j1, j2) = jobs.val[k.val]! := by
        rw [hj]; exact (getElem!_pos jobs.val k.val hlt).symm
      simp only [lift, bind_ok]
      have c1 : (UScalar.cast .U64 j1).val = j1.val := U32.cast_U64_val_eq _
      have c2 : (UScalar.cast .U64 j2).val = j2.val := U32.cast_U64_val_eq _
      step as ⟨ per, hper ⟩
      have hnext : jobsSum (jobs.val.take (k.val + 1)) = b.val + per.val := by
        rw [take_succ_getElem! _ _ hlt, ← ej, h2]
        simp [jobsSum]
        omega
      have hm := U64.checked_add_bv_spec b per
      cases hc : U64.checked_add b per with
      | none =>
        rw [hc] at hm
        simp only [WP.spec_ok]
        have := jobsSum_take_le jobs.val (k.val + 1)
        scalar_tac
      | some x =>
        rw [hc] at hm
        obtain ⟨_, hx, _⟩ := hm
        simp only []
        step as ⟨ k1, hk1 ⟩
        have kk : (k1.val ≤ jobs.val.length ∧ x.val = jobsSum (jobs.val.take k1.val)) ∧
            jobs.val.length - k1.val < jobs.val.length - k.val :=
          ⟨⟨by omega, by rw [hk1, hnext, hx]⟩, by omega⟩
        close3 kk
    · rw [if_neg (by scalar_tac)]
      simp only [WP.spec_ok]
      have : k.val = jobs.val.length := by omega
      rw [this, List.take_length] at h2
      exact h2
  · exact ⟨by scalar_tac, by simp [jobsSum]⟩

/-- The three cases of R37.2 for the main stack, as numbers. -/
def mspA (i : bound_v.MspInputs) : Nat := i.worst.val + i.levels.val * i.record.val + 32
def mspB (i : bound_v.MspInputs) : Nat := i.reset.val + 32 + i.fatal.val
def mspC (i : bound_v.MspInputs) : Nat :=
  i.worst_handler.val + i.levels.val * i.record.val + 32 + i.levels.val * 32 +
    (if i.any_fp then 104 else 32) + i.fatal.val

theorem basic_frame : bound_v.BASIC_FRAME = 32#u32 := by unfold bound_v.BASIC_FRAME; rfl
theorem fp_frame : bound_v.FP_FRAME = 104#u32 := by unfold bound_v.FP_FRAME; rfl

/-- One checked addition: two goals, the first for the sum that does not
fit and the second for the sum's value, each with the fact about the sum
as its first hypothesis to introduce. -/
syntax "chk_add " term:max term:max " as " ident : tactic
macro_rules
  | `(tactic| chk_add $a $b as $v) =>
    `(tactic| (have hm := U64.checked_add_bv_spec $a $b
               rcases hc : U64.checked_add $a $b with _ | $v
               all_goals (rw [hc] at hm; simp only at hm; simp only [WP.spec_ok]; revert hm; clear hc)))

syntax "chk_mul " term:max term:max " as " ident : tactic
macro_rules
  | `(tactic| chk_mul $a $b as $v) =>
    `(tactic| (have hm := U64.checked_mul_bv_spec $a $b
               rcases hc : U64.checked_mul $a $b with _ | $v
               all_goals (rw [hc] at hm; simp only at hm; simp only [WP.spec_ok]; revert hm; clear hc)))

set_option maxHeartbeats 4000000 in
/-- **The main-stack bound**: the three cases of R37.2 and the largest of
them, or none exactly when one of the three does not fit 64 bits. -/
theorem msp_value_spec (i : bound_v.MspInputs) :
    bound_v.msp_value i ⦃ r =>
      match r with
      | some (a, b, c, m) =>
        a.val = mspA i ∧ b.val = mspB i ∧ c.val = mspC i ∧
          m.val = max (mspA i) (max (mspB i) (mspC i))
      | none => U64.max < mspA i ∨ U64.max < mspB i ∨ U64.max < mspC i ⦄ := by
  unfold bound_v.msp_value mspA mspB mspC
  simp only [lift, bind_ok, basic_frame, fp_frame]
  have cb : (UScalar.cast .U64 32#u32).val = 32 := by rw [U32.cast_U64_val_eq]; rfl
  have cf : (UScalar.cast .U64 104#u32).val = 104 := by rw [U32.cast_U64_val_eq]; rfl
  have hmax : U64.max = 18446744073709551615 := by scalar_tac
  obtain ⟨ff, hff, hffv⟩ : ∃ ff : U64,
      (if i.any_fp = true then (ok (UScalar.cast .U64 104#u32) : Result U64)
        else ok (UScalar.cast .U64 32#u32)) = ok ff ∧
      ff.val = (if i.any_fp = true then 104 else 32) := by
    cases i.any_fp
    · exact ⟨UScalar.cast .U64 32#u32, by simp, by simp [cb]⟩
    · exact ⟨UScalar.cast .U64 104#u32, by simp, by simp [cf]⟩
  rw [hff]
  simp only [bind_ok]
  generalize (if i.any_fp = true then 104 else 32) = FF at hffv ⊢
  chk_mul i.levels i.record as x
  · intro hm; generalize i.levels.val * i.record.val = P at *; omega
  rintro ⟨-, hx, -⟩
  chk_mul i.levels (UScalar.cast .U64 32#u32) as x1
  · intro hm; rw [cb] at hm; generalize i.levels.val * i.record.val = P at *; omega
  rintro ⟨-, hx1, -⟩
  rw [cb] at hx1
  chk_add i.worst x as x2
  · intro hm; generalize i.levels.val * i.record.val = P at *; omega
  rintro ⟨-, hx2, -⟩
  chk_add x2 (UScalar.cast .U64 32#u32) as y
  · intro hm; rw [cb] at hm; generalize i.levels.val * i.record.val = P at *; omega
  rintro ⟨-, hy, -⟩
  rw [cb] at hy
  chk_add i.reset (UScalar.cast .U64 32#u32) as x3
  · intro hm; rw [cb] at hm; generalize i.levels.val * i.record.val = P at *; omega
  rintro ⟨-, hx3, -⟩
  rw [cb] at hx3
  chk_add x3 i.fatal as y1
  · intro hm; generalize i.levels.val * i.record.val = P at *; omega
  rintro ⟨-, hy1, -⟩
  chk_add i.worst_handler x as x4
  · intro hm; generalize i.levels.val * i.record.val = P at *; omega
  rintro ⟨-, hx4, -⟩
  chk_add x4 (UScalar.cast .U64 32#u32) as x5
  · intro hm; rw [cb] at hm; generalize i.levels.val * i.record.val = P at *; omega
  rintro ⟨-, hx5, -⟩
  rw [cb] at hx5
  chk_add x5 x1 as x6
  · intro hm; generalize i.levels.val * i.record.val = P at *; omega
  rintro ⟨-, hx6, -⟩
  chk_add x6 ff as x7
  · intro hm; generalize i.levels.val * i.record.val = P at *; omega
  rintro ⟨-, hx7, -⟩
  chk_add x7 i.fatal as x8
  · intro hm; generalize i.levels.val * i.record.val = P at *; omega
  rintro ⟨-, hx8, -⟩
  have fin : ∀ m : U64, m.val = max y.val (max y1.val x8.val) →
      y.val = i.worst.val + i.levels.val * i.record.val + 32 ∧
      y1.val = i.reset.val + 32 + i.fatal.val ∧
      x8.val = i.worst_handler.val + i.levels.val * i.record.val + 32 + i.levels.val * 32 + FF +
        i.fatal.val ∧
      m.val = max (i.worst.val + i.levels.val * i.record.val + 32)
        (max (i.reset.val + 32 + i.fatal.val)
          (i.worst_handler.val + i.levels.val * i.record.val + 32 + i.levels.val * 32 + FF +
            i.fatal.val)) := by
    intro m hm
    generalize i.levels.val * i.record.val = P at *
    omega
  split
  · rename_i c1
    split
    · rename_i c2
      simp only [WP.spec_ok]
      exact fin y (by scalar_tac)
    · rename_i c2
      split
      · rename_i c3
        simp only [WP.spec_ok]
        exact fin y1 (by scalar_tac)
      · rename_i c3
        simp only [WP.spec_ok]
        exact fin x8 (by scalar_tac)
  · rename_i c1
    split
    · rename_i c3
      simp only [WP.spec_ok]
      exact fin y1 (by scalar_tac)
    · rename_i c3
      simp only [WP.spec_ok]
      exact fin x8 (by scalar_tac)

end rsk_link
