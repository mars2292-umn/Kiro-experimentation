/-
  The Generator's derivation arithmetic on the code as compiled
  (2026-11-18): the statements Verus proves of `rsk_gen::derive_v`, proven
  of the functions as Aeneas extracts them. Each theorem is total: the
  function returns for every input the statement admits, and its result
  satisfies the postcondition. Handwritten.
-/
import Aeneas
import Tools.Gen
open Aeneas Aeneas.Std Result

namespace rsk_gen

/-- DD-02: the NVIC priority byte of a level, with three implemented bits. -/
def specPriorityByte (level : Nat) : Nat := if 1 ≤ level ∧ level ≤ 8 then (8 - level) * 32 else 0

theorem priority_byte_spec (level : U8) :
    derive_v.priority_byte level ⦃ b => b.val = specPriorityByte level.val ⦄ := by
  unfold derive_v.priority_byte specPriorityByte
  split
  · split
    · step as ⟨ i, hi ⟩
      step as ⟨ b, hb ⟩
      have h1 : 1 ≤ level.val ∧ level.val ≤ 8 := by scalar_tac
      simp [h1]
      scalar_tac
    · have h1 : ¬ (1 ≤ level.val ∧ level.val ≤ 8) := by scalar_tac
      simp [h1]
  · have h1 : ¬ (1 ≤ level.val ∧ level.val ≤ 8) := by scalar_tac
    simp [h1]

/-- R23.1: `c` is the Ceiling of a Resource whose accessors have the
Priorities `p`: no accessor is above it, one is at it, and it is 0 for
none. Stated of the first `n` accessors. -/
def isCeiling (p : List U8) (n : Nat) (c : U8) : Prop :=
  (∀ j, j < n → (p[j]!).val ≤ c.val) ∧ (n = 0 → c.val = 0) ∧ (0 < n → ∃ j, j < n ∧ p[j]! = c)

theorem isCeiling_new {p : List U8} {n : Nat} {x : U8} (hx : p[n]! = x)
    (hall : ∀ j, j < n → (p[j]!).val ≤ x.val) : isCeiling p (n + 1) x := by
  refine ⟨?_, by omega, fun _ => ⟨n, by omega, hx⟩⟩
  intro j hj
  by_cases h : j = n
  · subst h; rw [hx]
  · exact hall j (by omega)

theorem isCeiling_keep {p : List U8} {n : Nat} {c : U8} (hc : isCeiling p n c) (hn : 0 < n)
    (hle : (p[n]!).val ≤ c.val) : isCeiling p (n + 1) c := by
  obtain ⟨hall, _, hex⟩ := hc
  obtain ⟨j, hj, he⟩ := hex hn
  refine ⟨?_, by omega, fun _ => ⟨j, by omega, he⟩⟩
  intro k hk
  by_cases h : k = n
  · subst h; exact hle
  · exact hall k (by omega)

theorem ceiling_body_spec (p : alloc.vec.Vec U8) (c : U8) (i : Usize)
    (hi : i.val ≤ p.val.length) (hc : isCeiling p.val i.val c) :
    derive_v.ceiling_loop.body p c i ⦃ r =>
      match r with
      | .done c' => isCeiling p.val p.val.length c'
      | .cont (c', i') => i'.val = i.val + 1 ∧ i'.val ≤ p.val.length ∧ isCeiling p.val i'.val c' ⦄ := by
  unfold derive_v.ceiling_loop.body
  simp only []
  split
  · rename_i hlt
    have hlt' : i.val < p.val.length := by scalar_tac
    by_cases h0 : i = 0#usize
    · have hi0 : i.val = 0 := by rw [h0]; rfl
      simp only [h0, if_true]
      step as ⟨ x, hx ⟩
      step as ⟨ i2, hi2 ⟩
      have hx' : (p.val)[0]! = x := by rw [hx]; exact getElem!_pos p.val 0 (by omega)
      have h1 : i2.val = 0 + 1 := by scalar_tac
      refine ⟨by scalar_tac, by scalar_tac, ?_⟩
      rw [h1]
      exact isCeiling_new hx' (fun j hj => by omega)
    · have hi0 : 0 < i.val := by scalar_tac
      simp only [h0, if_false]
      step as ⟨ x, hx ⟩
      have hx' : (p.val)[i.val]! = x := by rw [hx]; exact getElem!_pos p.val i.val hlt'
      split
      · rename_i hgt
        step as ⟨ i2, hi2 ⟩
        have h1 : i2.val = i.val + 1 := by scalar_tac
        refine ⟨h1, by scalar_tac, ?_⟩
        rw [h1]
        refine isCeiling_new hx' (fun j hj => ?_)
        have := hc.1 j hj
        scalar_tac
      · rename_i hle
        step as ⟨ i2, hi2 ⟩
        have h1 : i2.val = i.val + 1 := by scalar_tac
        refine ⟨h1, by scalar_tac, ?_⟩
        rw [h1]
        exact isCeiling_keep hc hi0 (by rw [hx']; scalar_tac)
  · rename_i hge
    have heq : i.val = p.val.length := by scalar_tac
    simp
    rw [← heq]
    exact hc

/-- R23.1: `ceiling` returns the Ceiling of the accessors' Priorities. -/
theorem ceiling_spec (p : alloc.vec.Vec U8) :
    derive_v.ceiling p ⦃ c => isCeiling p.val p.val.length c ⦄ := by
  unfold derive_v.ceiling derive_v.ceiling_loop
  apply loop.spec_decr_nat (fun (s : U8 × Usize) => p.val.length - s.2.val)
    (fun s => s.2.val ≤ p.val.length ∧ isCeiling p.val s.2.val s.1)
  · intro s hs
    obtain ⟨c, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    simp only [] at h1 h2
    apply WP.spec_mono (ceiling_body_spec p c i h1 h2)
    intro r hr
    cases r with
    | done o => simpa using hr
    | cont s' =>
      obtain ⟨c', i'⟩ := s'
      simp only [] at hr ⊢
      obtain ⟨e1, e2, e3⟩ := hr
      exact ⟨⟨e2, e3⟩, by omega⟩
  · refine ⟨by scalar_tac, ?_⟩
    refine ⟨fun j hj => ?_, fun _ => rfl, fun h => ?_⟩
    · simp at hj
    · simp at h

/-! ## Region size (DD-07, R16.6) -/

theorem two_pow_le_30 {k : Nat} (h : k ≤ 30) : 2 ^ k ≤ 1073741824 := by
  have := Nat.pow_le_pow_right (show 0 < 2 by norm_num) h
  norm_num at this
  exact this

@[step] theorem region_loop0_spec (min k : U8) (size : U64) (hmin : min.val ≤ 31)
    (hk : k.val ≤ min.val) (hs : size.val = 2 ^ k.val) :
    derive_v.region_log2_loop0 min k size ⦃ r => r.1.val = min.val ∧ r.2.val = 2 ^ min.val ⦄ := by
  unfold derive_v.region_log2_loop0
  apply loop.spec_decr_nat (fun (s : U8 × U64) => min.val - s.1.val)
    (fun s => s.1.val ≤ min.val ∧ s.2.val = 2 ^ s.1.val)
  · intro s hs
    obtain ⟨k, size⟩ := s
    obtain ⟨h1, h2⟩ := hs
    simp only [] at h1 h2 ⊢
    unfold derive_v.region_log2_loop0.body
    split
    · rename_i hlt
      have hk30 : k.val ≤ 30 := by scalar_tac
      have hp := two_pow_le_30 hk30
      step as ⟨ size1, hs1 ⟩
      step as ⟨ k1, hk1 ⟩
      have e1 : k1.val = k.val + 1 := by scalar_tac
      have e2 : size1.val = 2 ^ k1.val := by rw [e1, pow_succ]; scalar_tac
      have e3 : k1.val ≤ min.val := by scalar_tac
      have e4 : min.val - k1.val < min.val - k.val := by scalar_tac
      exact ⟨e3, e2, e4⟩
    · rename_i hge
      have e : k.val = min.val := by scalar_tac
      simp
      exact ⟨e, by rw [← e]; exact h2⟩
  · exact ⟨hk, hs⟩

/-- The result of `region_log2`: the smallest exponent, not below the
minimum and at most 31, whose power of two holds the bytes; none when
2^31 does not. -/
def regionPost (bytes min : Nat) : Option U8 → Prop
  | some k => min ≤ k.val ∧ k.val ≤ 31 ∧ bytes ≤ 2 ^ k.val ∧ (min < k.val → 2 ^ (k.val - 1) < bytes)
  | none => 2 ^ 31 < bytes

theorem region_loop1_spec (bytes : U64) (min k : U8) (size : U64) (hmin : min.val ≤ k.val)
    (hk : k.val ≤ 31) (hs : size.val = 2 ^ k.val) (hprev : min.val < k.val → 2 ^ (k.val - 1) < bytes.val) :
    derive_v.region_log2_loop1 bytes k size ⦃ r => regionPost bytes.val min.val r ⦄ := by
  unfold derive_v.region_log2_loop1
  apply loop.spec_decr_nat (fun (s : U8 × U64) => 31 - s.1.val)
    (fun s => min.val ≤ s.1.val ∧ s.1.val ≤ 31 ∧ s.2.val = 2 ^ s.1.val
      ∧ (min.val < s.1.val → 2 ^ (s.1.val - 1) < bytes.val))
  · intro s hs
    obtain ⟨k, size⟩ := s
    obtain ⟨h1, h2, h3, h4⟩ := hs
    simp only [] at h1 h2 h3 h4 ⊢
    unfold derive_v.region_log2_loop1.body
    split
    · rename_i hlt
      split
      · rename_i h31
        have e : k.val = 31 := by rw [h31]; rfl
        simp [regionPost]
        rw [e] at h3
        scalar_tac
      · rename_i hne
        have hk30 : k.val ≤ 30 := by scalar_tac
        have hp := two_pow_le_30 hk30
        step as ⟨ size1, hs1 ⟩
        step as ⟨ k1, hk1 ⟩
        have e1 : k1.val = k.val + 1 := by scalar_tac
        have e2 : size1.val = 2 ^ k1.val := by rw [e1, pow_succ]; scalar_tac
        have e3 : min.val ≤ k1.val := by scalar_tac
        have e4 : k1.val ≤ 31 := by scalar_tac
        have e5 : min.val < k1.val → 2 ^ (k1.val - 1) < bytes.val := by
          intro _
          have : k1.val - 1 = k.val := by omega
          rw [this, ← h3]
          scalar_tac
        have e6 : 31 - k1.val < 31 - k.val := by scalar_tac
        exact ⟨e3, e4, e2, e5, e6⟩
    · rename_i hge
      simp [regionPost]
      exact ⟨h1, h2, by scalar_tac, h4⟩
  · exact ⟨hmin, hk, hs, hprev⟩

/-- DD-07: `region_log2`. -/
theorem region_log2_spec (bytes : U64) (min : U8) (hmin : min.val ≤ 31) :
    derive_v.region_log2 bytes min ⦃ r => regionPost bytes.val min.val r ⦄ := by
  unfold derive_v.region_log2
  step as ⟨ k, size, hk, hs ⟩
  exact region_loop1_spec bytes min k size (by scalar_tac) (by scalar_tac) (by rw [hs, hk])
    (fun h => by scalar_tac)

/-! ## Time conversion (R21.2, R29.8, Property 14) -/

/-- `x` is `v / den` rounded down (`up` false) or up (`up` true). -/
def rounded (v den : Nat) (up : Bool) (x : Nat) : Prop :=
  if up then v ≤ x * den ∧ (x = 0 ∨ (x - 1) * den < v)
  else x * den ≤ v ∧ v < (x + 1) * den

/-- The result of `scale`: the product over the denominator rounded as
asked, and whether rounding changed it; none when the product does not fit
128 bits or the result 64. -/
def scalePost (value num den : Nat) (up : Bool) : Option (U64 × Bool) → Prop
  | some (x, changed) =>
    rounded (value * num) den up x.val ∧ (changed = true ↔ (value * num) % den ≠ 0)
  | none => U128.max < value * num ∨ U64.max < (value * num) / den + 1

/-- The quotient, or the quotient plus one when rounding up a division
with a remainder, is the dividend over the denominator rounded as asked. -/
theorem rounded_div (V den : Nat) (up : Bool) (hd : 0 < den) :
    rounded V den up (if up = true ∧ V % den ≠ 0 then V / den + 1 else V / den) := by
  have hdm : den * (V / den) + V % den = V := Nat.div_add_mod _ _
  have hrl : V % den < den := Nat.mod_lt _ hd
  generalize V / den = q at hdm ⊢
  generalize V % den = r at hdm hrl ⊢
  unfold rounded
  cases up
  · simp
    constructor <;> nlinarith
  · by_cases hr : r = 0
    · simp [hr]
      subst hr
      refine ⟨by nlinarith, ?_⟩
      by_cases hq : q = 0
      · exact Or.inl hq
      · right
        obtain ⟨q', rfl⟩ : ∃ q', q = q' + 1 := ⟨q - 1, by omega⟩
        simp
        nlinarith
    · simp [hr]
      constructor <;> nlinarith [Nat.pos_of_ne_zero hr]

theorem u64_max_as_u128 : (UScalar.cast .U128 core.num.U64.MAX).val = 18446744073709551615 := by
  first
    | rfl
    | (unfold core.num.U64.MAX; rfl)
    | (simp [core.num.U64.MAX]; rfl)
    | scalar_tac

theorem scale_spec (value : U64) (num den : U128) (up : Bool) (hden : 0 < den.val) :
    derive_v.scale value num den up ⦃ r => scalePost value.val num.val den.val up r ⦄ := by
  unfold derive_v.scale
  step as ⟨ i, hi ⟩
  have hiv : i.val = value.val := by rw [hi]; scalar_tac
  have hm := U128.checked_mul_bv_spec i num
  cases hcm : U128.checked_mul i num with
  | none =>
    rw [hcm] at hm
    simp [lift, scalePost]
    left
    rw [← hiv]
    exact hm
  | some v =>
    rw [hcm] at hm
    obtain ⟨_, hv, _⟩ := hm
    simp only [lift, bind_ok]
    have hVV : v.val = value.val * num.val := by rw [hv, hiv]
    step as ⟨ q, hq ⟩
    step as ⟨ rem, hrem ⟩
    have hmax := u64_max_as_u128
    have hr := rounded_div (value.val * num.val) den.val up hden
    rw [← hVV, ← hq, ← hrem] at hr
    have hrem0 : ((rem != 0#u128) = true) ↔ rem.val ≠ 0 := by
      simp <;> scalar_tac
    have hch : ((rem != 0#u128) = true) ↔ (value.val * num.val) % den.val ≠ 0 := by
      rw [hrem0, hrem, hVV]
    rw [hVV] at hr
    split
    · rename_i hge
      have hgoal : U64.max < value.val * num.val / den.val + 1 := by
        rw [← hVV, ← hq]
        scalar_tac
      simp only [WP.spec_ok]
      exact Or.inr hgoal
    · rename_i hlt
      have hqlt : q.val < U64.max := by scalar_tac
      cases up
      · simp only [Bool.false_eq_true, ite_false, bind_ok, WP.spec_ok]
        have hc : (UScalar.cast .U64 q).val = q.val := by scalar_tac
        simp only [Bool.false_eq_true, false_and, ite_false] at hr
        exact ⟨by rw [hc]; exact hr, hch⟩
      · simp only [ite_true]
        split
        · rename_i hne
          have hrn : rem.val ≠ 0 := hrem0.mp hne
          step as ⟨ x, hx ⟩
          have hc : (UScalar.cast .U64 x).val = q.val + 1 := by scalar_tac
          simp only [true_and, hrn, ne_eq, not_false_eq_true, ite_true] at hr
          exact ⟨by rw [hc]; exact hr, hch⟩
        · rename_i hne
          have hrn : ¬ rem.val ≠ 0 := fun h => hne (hrem0.mpr h)
          simp only [bind_ok, WP.spec_ok]
          have hc : (UScalar.cast .U64 q).val = q.val := by scalar_tac
          simp only [true_and, hrn, ite_false] at hr
          exact ⟨by rw [hc]; exact hr, hch⟩

/-! ## Timed-event slots (R9.4, R23.4) -/

deriving instance Inhabited for derive_v.Slots

theorem no_slot : derive_v.NO_SLOT.val = 255 := by unfold derive_v.NO_SLOT; rfl

/-- The number of slots the first `n` Tasks take: three for a Task that
wants a release slot, two for one that does not. -/
def taken (wants : List Bool) : Nat → Nat
  | 0 => 0
  | n + 1 => taken wants n + (if wants[n]! then 3 else 2)

theorem taken_mono (wants : List Bool) {a b : Nat} (h : a ≤ b) : taken wants a ≤ taken wants b := by
  induction b with
  | zero => have : a = 0 := by omega
            subst this; exact Nat.le_refl _
  | succ n ih =>
    by_cases e : a = n + 1
    · subst e; exact Nat.le_refl _
    · have := ih (by omega)
      simp only [taken]
      omega

/-- The slots of a Task whose first slot is `b`: release, deadline, Budget
in that order when it wants a release slot; deadline, Budget and no
release slot otherwise. -/
def slotIs (s : derive_v.Slots) (want : Bool) (b : Nat) : Prop :=
  if want then s.release.val = b ∧ s.deadline.val = b + 1 ∧ s.budget.val = b + 2
  else s.release.val = 255 ∧ s.deadline.val = b ∧ s.budget.val = b + 1

/-- The plan of the first `t` Tasks is the consecutive one: Task `j`'s
slots start at the number of slots the Tasks before it take. -/
def exactPlan (wants : List Bool) (plan : List derive_v.Slots) (t : Nat) : Prop :=
  plan.length = t ∧ ∀ j, j < t → slotIs plan[j]! wants[j]! (taken wants j)

/-- The result of `slot_plan`: the consecutive plan with the count of
slots, two more than the Tasks take (the Kernel's two); none when that
count is above 253. -/
def slotPlanPost (wants : List Bool) : Option (alloc.vec.Vec derive_v.Slots × U8) → Prop
  | some (plan, c) =>
    exactPlan wants plan.val wants.length ∧ c.val = taken wants wants.length + 2 ∧ c.val ≤ 253
  | none => 253 < taken wants wants.length + 2

theorem exactPlan_push {wants : List Bool} {plan : List derive_v.Slots} {t : Nat}
    {s : derive_v.Slots} (hp : exactPlan wants plan t) (hs : slotIs s wants[t]! (taken wants t)) :
    exactPlan wants (plan ++ [s]) (t + 1) := by
  obtain ⟨hl, hall⟩ := hp
  refine ⟨by simp [hl], ?_⟩
  intro j hj
  by_cases h : j < t
  · have e : (plan ++ [s])[j]! = plan[j]! := by
      simp [List.getElem!_eq_getElem?_getD, List.getElem?_append_left (show j < plan.length by omega)]
    rw [e]
    exact hall j h
  · have hjt : j = t := by omega
    subst hjt
    have e : (plan ++ [s])[j]! = s := by
      simp [List.getElem!_eq_getElem?_getD, ← hl]
    rw [e]
    exact hs

theorem slot_plan_body_spec (wants : alloc.vec.Vec Bool) (plan : alloc.vec.Vec derive_v.Slots)
    (next : U8) (t : Usize) (ht : t.val ≤ wants.val.length)
    (hp : exactPlan wants.val plan.val t.val) (hn : next.val = taken wants.val t.val)
    (hn251 : next.val ≤ 251) :
    derive_v.slot_plan_loop.body wants plan next t ⦃ r =>
      match r with
      | .done o => slotPlanPost wants.val o
      | .cont (plan', next', t') => t'.val = t.val + 1 ∧ t'.val ≤ wants.val.length ∧
          exactPlan wants.val plan'.val t'.val ∧ next'.val = taken wants.val t'.val ∧ next'.val ≤ 251 ⦄ := by
  unfold derive_v.slot_plan_loop.body
  simp only []
  split
  · rename_i hlt
    have hlt' : t.val < wants.val.length := by scalar_tac
    step as ⟨ want, hw ⟩
    have hw' : (wants.val)[t.val]! = want := by rw [hw]; exact getElem!_pos wants.val t.val hlt'
    have htk : taken wants.val (t.val + 1) = next.val + (if want then 3 else 2) := by
      simp only [taken, hw', hn]
    have hmono := taken_mono wants.val (show t.val + 1 ≤ wants.val.length by omega)
    cases want
    · simp only [Bool.false_eq_true, ite_false, bind_ok] at htk ⊢
      step as ⟨ i1, hi1 ⟩
      split
      · rename_i hgt
        simp only [WP.spec_ok, slotPlanPost]
        scalar_tac
      · rename_i hle
        step as ⟨ i2, hi2 ⟩
        step as ⟨ plan1, hplan1 ⟩
        step as ⟨ next1, hn1 ⟩
        step as ⟨ t1, ht1 ⟩
        have e1 : t1.val = t.val + 1 := by scalar_tac
        refine ⟨e1, by scalar_tac, ?_, ?_, ?_⟩
        · rw [e1, hplan1]
          refine exactPlan_push hp ?_
          rw [hw']
          simp only [slotIs, Bool.false_eq_true, ite_false]
          exact ⟨no_slot, by scalar_tac, by scalar_tac⟩
        · rw [e1, htk]; scalar_tac
        · scalar_tac
    · simp only [ite_true, bind_ok] at htk ⊢
      step as ⟨ i1, hi1 ⟩
      split
      · rename_i hgt
        simp only [WP.spec_ok, slotPlanPost]
        scalar_tac
      · rename_i hle
        step as ⟨ base, hbase ⟩
        step as ⟨ i2, hi2 ⟩
        step as ⟨ plan1, hplan1 ⟩
        step as ⟨ next1, hn1 ⟩
        step as ⟨ t1, ht1 ⟩
        have e1 : t1.val = t.val + 1 := by scalar_tac
        refine ⟨e1, by scalar_tac, ?_, ?_, ?_⟩
        · rw [e1, hplan1]
          refine exactPlan_push hp ?_
          rw [hw']
          simp only [slotIs, ite_true]
          exact ⟨by scalar_tac, by scalar_tac, by scalar_tac⟩
        · rw [e1, htk]; scalar_tac
        · scalar_tac
  · rename_i hge
    have heq : t.val = wants.val.length := by scalar_tac
    step as ⟨ c, hc ⟩
    simp only [slotPlanPost]
    rw [← heq]
    exact ⟨hp, by scalar_tac, by scalar_tac⟩

/-- R9.4, R23.4: `slot_plan` returns the consecutive plan and its count,
or none when the Tasks need more than 251 slots. -/
theorem slot_plan_spec (wants : alloc.vec.Vec Bool) :
    derive_v.slot_plan wants ⦃ r => slotPlanPost wants.val r ⦄ := by
  unfold derive_v.slot_plan derive_v.slot_plan_loop
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec derive_v.Slots × U8 × Usize) => wants.val.length - s.2.2.val)
    (fun s => s.2.2.val ≤ wants.val.length ∧ exactPlan wants.val s.1.val s.2.2.val
      ∧ s.2.1.val = taken wants.val s.2.2.val ∧ s.2.1.val ≤ 251)
  · intro s hs
    obtain ⟨plan, next, t⟩ := s
    obtain ⟨h1, h2, h3, h4⟩ := hs
    simp only [] at h1 h2 h3 h4
    apply WP.spec_mono (slot_plan_body_spec wants plan next t h1 h2 h3 h4)
    intro r hr
    cases r with
    | done o => simpa using hr
    | cont s' =>
      obtain ⟨plan', next', t'⟩ := s'
      simp only [] at hr ⊢
      obtain ⟨e1, e2, e3, e4, e5⟩ := hr
      exact ⟨⟨e2, e3, e4, e5⟩, by omega⟩
  · refine ⟨by scalar_tac, ⟨by simp, fun j hj => by simp at hj⟩, by simp [taken], by simp⟩

/-! ### What the consecutive plan gives (the Verus `plan_ok`) -/

/-- The real slots of a Task. -/
def slotsOf (s : derive_v.Slots) : List Nat :=
  if s.release.val = 255 then [s.deadline.val, s.budget.val]
  else [s.release.val, s.deadline.val, s.budget.val]

/-- Every slot of Task `j` of a consecutive plan lies from the slots the
Tasks before it take up to those it takes with them. -/
theorem slots_in_range {wants : List Bool} {plan : List derive_v.Slots} {t j : Nat}
    (hp : exactPlan wants plan t) (hj : j < t) (h251 : taken wants t ≤ 251) :
    ∀ x ∈ slotsOf plan[j]!, taken wants j ≤ x ∧ x < taken wants (j + 1) := by
  have hs := hp.2 j hj
  have hm := taken_mono wants (show j + 1 ≤ t by omega)
  have hstep : taken wants (j + 1) = taken wants j + (if wants[j]! then 3 else 2) := rfl
  intro x hx
  unfold slotIs at hs
  unfold slotsOf at hx
  cases hw : wants[j]! <;> simp only [hw, Bool.false_eq_true, ite_false, ite_true] at hs hstep
  · obtain ⟨h1, h2, h3⟩ := hs
    rw [if_pos h1] at hx
    simp only [List.mem_cons, List.mem_nil_iff, or_false] at hx
    rcases hx with rfl | rfl <;> omega
  · obtain ⟨h1, h2, h3⟩ := hs
    have hne : ¬ (plan[j]!).release.val = 255 := by omega
    rw [if_neg hne] at hx
    simp only [List.mem_cons, List.mem_nil_iff, or_false] at hx
    rcases hx with rfl | rfl | rfl <;> omega

/-- R9.4, R23.4 as Verus states it (`plan_ok`): every Task has a deadline
and a Budget slot that differ, and a release slot different from both
exactly when it wants one; every slot is below the count less the Kernel's
two; no slot is given to two Tasks. -/
theorem plan_ok_of_exact {wants : List Bool} {plan : List derive_v.Slots}
    (hp : exactPlan wants plan wants.length) (h251 : taken wants wants.length ≤ 251) :
    plan.length = wants.length ∧
    (∀ t, t < plan.length →
      (plan[t]!).deadline.val ≠ (plan[t]!).budget.val ∧
      (plan[t]!).deadline.val < taken wants wants.length ∧
      (plan[t]!).budget.val < taken wants wants.length ∧
      (wants[t]! = true → (plan[t]!).release.val ≠ 255 ∧
        (plan[t]!).release.val < taken wants wants.length ∧
        (plan[t]!).release.val ≠ (plan[t]!).deadline.val ∧
        (plan[t]!).release.val ≠ (plan[t]!).budget.val) ∧
      (wants[t]! = false → (plan[t]!).release.val = 255)) ∧
    (∀ t u, t < plan.length → u < plan.length → t ≠ u →
      ∀ x ∈ slotsOf plan[t]!, x ∉ slotsOf plan[u]!) := by
  have hl := hp.1
  refine ⟨hl, ?_, ?_⟩
  · intro t ht
    have hs := hp.2 t (by omega)
    have hm := taken_mono wants (show t + 1 ≤ wants.length by omega)
    have hstep : taken wants (t + 1) = taken wants t + (if wants[t]! then 3 else 2) := rfl
    unfold slotIs at hs
    cases hw : wants[t]! <;> simp only [hw, Bool.false_eq_true, ite_false, ite_true] at hs hstep
    · obtain ⟨h1, h2, h3⟩ := hs
      refine ⟨by omega, by omega, by omega, by simp, fun _ => h1⟩
    · obtain ⟨h1, h2, h3⟩ := hs
      refine ⟨by omega, by omega, by omega, fun _ => ⟨by omega, by omega, by omega, by omega⟩, by simp⟩
  · intro t u ht hu hne x hx hx'
    have r1 := slots_in_range hp (show t < wants.length by omega) h251 x hx
    have r2 := slots_in_range hp (show u < wants.length by omega) h251 x hx'
    rcases Nat.lt_or_gt_of_ne hne with h | h
    · have := taken_mono wants (show t + 1 ≤ u by omega)
      omega
    · have := taken_mono wants (show u + 1 ≤ t by omega)
      omega

end rsk_gen
