/-
  The Config_Checker's core on the code as compiled (2026-11-18): the
  statements Verus proves of `rsk_confcheck::derive_v`, proven of the
  functions as Aeneas extracts them. Each theorem is total: the function
  returns for every input the statement admits, and its result satisfies
  the postcondition. Handwritten. Nothing here is shared with
  `Tools/GenProofs.lean`: the two tools' specifications are written apart
  (R26.1).
-/
import Aeneas
import Tools.Cc
open Aeneas Aeneas.Std Result

namespace rsk_confcheck

theorem MAX_val : (core.num.U64.MAX).val = 18446744073709551615 := by rfl
theorem rMax_val : U64.rMax = 18446744073709551615 := by rfl
theorem max_val : U64.max = 18446744073709551615 := by scalar_tac

/-- `scalar_tac` with the value of `u64::MAX`. -/
macro "sc" : tactic => `(tactic| (try simp only [MAX_val, rMax_val, max_val] at *) <;> scalar_tac)

/-- DD-02: the NVIC priority byte of a level. -/
def specEncodeLevel (level : Nat) : Nat := if 1 ≤ level ∧ level ≤ 8 then (8 - level) * 32 else 0

theorem encode_level_spec (level : U64) :
    derive_v.encode_level level ⦃ e => e.val = specEncodeLevel level.val ⦄ := by
  unfold derive_v.encode_level specEncodeLevel
  split
  · rename_i h0
    have : level.val = 0 := by rw [h0]; rfl
    simp [this]
  · rename_i h0
    have hpos : 1 ≤ level.val := by scalar_tac
    split
    · have h1 : ¬ (1 ≤ level.val ∧ level.val ≤ 8) := by scalar_tac
      simp [h1]
    · step as ⟨ i, hi ⟩
      step as ⟨ b, hb ⟩
      have h1 : 1 ≤ level.val ∧ level.val ≤ 8 := by scalar_tac
      simp [h1]
      scalar_tac

/-! ## Ceilings (R23.1): the maximum of the accessors' Priorities -/

/-- The greatest of the first `n` values, 0 for none. -/
def maxP (l : List U64) : Nat → Nat
  | 0 => 0
  | n + 1 => max (maxP l n) (l[n]!).val

theorem maxP_bound (l : List U64) (n : Nat) : ∀ j, j < n → (l[j]!).val ≤ maxP l n := by
  induction n with
  | zero => intro j hj; omega
  | succ n ih =>
    intro j hj
    simp only [maxP]
    by_cases h : j = n
    · subst h; exact Nat.le_max_right _ _
    · exact Nat.le_trans (ih j (by omega)) (Nat.le_max_left _ _)

theorem maxP_attained (l : List U64) (n : Nat) (hn : 0 < n) : ∃ j, j < n ∧ (l[j]!).val = maxP l n := by
  induction n with
  | zero => omega
  | succ n ih =>
    simp only [maxP]
    by_cases h : maxP l n ≤ (l[n]!).val
    · exact ⟨n, by omega, by rw [Nat.max_eq_right h]⟩
    · have hn0 : 0 < n := by
        by_contra h0
        have : n = 0 := by omega
        subst this
        simp [maxP] at h
      obtain ⟨j, hj, he⟩ := ih hn0
      exact ⟨j, by omega, by rw [he, Nat.max_eq_left (by omega)]⟩

theorem ceiling_of_spec (prios : alloc.vec.Vec U64) :
    derive_v.ceiling_of prios ⦃ c => c.val = maxP prios.val prios.val.length ⦄ := by
  unfold derive_v.ceiling_of derive_v.ceiling_of_loop
  apply loop.spec_decr_nat (fun (s : U64 × Usize) => prios.val.length - s.2.val)
    (fun s => s.2.val ≤ prios.val.length ∧ s.1.val = maxP prios.val s.2.val)
  · intro s hs
    obtain ⟨c, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    simp only [] at h1 h2 ⊢
    unfold derive_v.ceiling_of_loop.body
    simp only []
    split
    · rename_i hlt
      have hlt' : i.val < prios.val.length := by scalar_tac
      step as ⟨ x, hx ⟩
      have hx' : (prios.val)[i.val]! = x := by rw [hx]; exact getElem!_pos prios.val i.val hlt'
      split
      · rename_i hgt
        step as ⟨ i3, hi3 ⟩
        have e1 : i3.val = i.val + 1 := by scalar_tac
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [e1]
        simp only [maxP, hx', ← h2]
        scalar_tac
      · rename_i hle
        step as ⟨ i3, hi3 ⟩
        have e1 : i3.val = i.val + 1 := by scalar_tac
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [e1]
        simp only [maxP, hx', ← h2]
        scalar_tac
    · rename_i hge
      have heq : i.val = prios.val.length := by scalar_tac
      simp
      rw [← heq]
      exact h2
  · exact ⟨by scalar_tac, by simp [maxP]⟩

/-! ## Sums (the capacities of R23.4) -/

/-- The sum of the first `n` values. -/
def sumP (l : List U64) : Nat → Nat
  | 0 => 0
  | n + 1 => sumP l n + (l[n]!).val

theorem sumP_mono (l : List U64) {a b : Nat} (h : a ≤ b) : sumP l a ≤ sumP l b := by
  induction b with
  | zero => have : a = 0 := by omega
            subst this; exact Nat.le_refl _
  | succ n ih =>
    by_cases e : a = n + 1
    · subst e; exact Nat.le_refl _
    · have := ih (by omega)
      simp only [sumP]
      omega

/-- The result of `sum_checked`: the sum, or none when it does not fit. -/
def sumPost (l : List U64) : Option U64 → Prop
  | some s => s.val = sumP l l.length
  | none => U64.max < sumP l l.length

theorem sum_checked_spec (v : alloc.vec.Vec U64) :
    derive_v.sum_checked v ⦃ r => sumPost v.val r ⦄ := by
  unfold derive_v.sum_checked derive_v.sum_checked_loop
  apply loop.spec_decr_nat (fun (s : U64 × Usize) => v.val.length - s.2.val)
    (fun s => s.2.val ≤ v.val.length ∧ s.1.val = sumP v.val s.2.val)
  · intro s hs
    obtain ⟨total, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    simp only [] at h1 h2 ⊢
    unfold derive_v.sum_checked_loop.body
    simp only []
    split
    · rename_i hlt
      have hlt' : i.val < v.val.length := by scalar_tac
      step as ⟨ x, hx ⟩
      have hx' : (v.val)[i.val]! = x := by rw [hx]; exact getElem!_pos v.val i.val hlt'
      have hstep : sumP v.val (i.val + 1) = total.val + x.val := by simp only [sumP, hx', h2]
      have hmono := sumP_mono v.val (show i.val + 1 ≤ v.val.length by omega)
      have hM := MAX_val
      have htm : total.val ≤ (core.num.U64.MAX).val := by rw [hM]; scalar_tac
      step as ⟨ i3, hi3 ⟩
      split
      · rename_i hgt
        simp only [WP.spec_ok, sumPost]
        sc
      · rename_i hle
        step as ⟨ total1, ht1 ⟩
        step as ⟨ i4, hi4 ⟩
        have e1 : i4.val = i.val + 1 := by scalar_tac
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [e1, hstep]
        scalar_tac
    · rename_i hge
      have heq : i.val = v.val.length := by scalar_tac
      simp [sumPost]
      rw [← heq]
      exact h2
  · exact ⟨by scalar_tac, by simp [sumP]⟩

/-! ## The timer queue (R9.4) -/

theorem timer_slots_spec (tasks : U64) :
    derive_v.timer_slots tasks ⦃ r => match r with
      | some n => n.val = 3 * tasks.val + 2
      | none => U64.max < 3 * tasks.val + 2 ⦄ := by
  unfold derive_v.timer_slots
  have hM := MAX_val
  have h2 : (2#u64).val ≤ (core.num.U64.MAX).val := by rw [hM]; decide
  step as ⟨ i, hi ⟩
  step as ⟨ i1, hi1 ⟩
  split
  · rename_i hgt
    simp only [WP.spec_ok]
    sc
  · rename_i hle
    have hr := rMax_val
    have : tasks.val ≤ (18446744073709551615 - 2) / 3 := by sc
    step as ⟨ i2, hi2 ⟩
    step as ⟨ i3, hi3 ⟩
    scalar_tac

theorem slot_index_spec (i k : U64) (hk : k.val ≤ 2) :
    derive_v.slot_index i k ⦃ r => match r with
      | some n => n.val = 3 * i.val + k.val
      | none => U64.max < 3 * i.val + k.val ⦄ := by
  unfold derive_v.slot_index
  step as ⟨ i1, hi1 ⟩
  have e1 : i1.val = i.val := by rw [hi1]; scalar_tac
  step as ⟨ i2, hi2 ⟩
  step as ⟨ i3, hi3 ⟩
  have e3 : i3.val = k.val := by rw [hi3]; scalar_tac
  step as ⟨ v, hv ⟩
  step as ⟨ i4, hi4 ⟩
  have e4 : i4.val = 18446744073709551615 := by rw [hi4]; rfl
  split
  · rename_i hgt
    simp only [WP.spec_ok]
    sc
  · rename_i hle
    step as ⟨ i5, hi5 ⟩
    have : v.val ≤ 18446744073709551615 := by scalar_tac
    have e5 : i5.val = v.val := by rw [hi5]; scalar_tac
    scalar_tac

/-! ## Regions (R16.6, R16.7): sizes, alignment, disjointness, the stack -/

theorem two_pow_lt_64 {n : Nat} (h : n < 64) : 2 ^ n < 18446744073709551616 := by
  have := Nat.pow_lt_pow_right (show 1 < 2 by norm_num) h
  norm_num at this
  exact this

/-- The result of `size_of`: two to the exponent when it is below 64. -/
def sizePost (l : Nat) : Option U64 → Prop
  | some s => l < 64 ∧ s.val = 2 ^ l
  | none => 64 ≤ l

@[step] theorem size_of_spec (l : U8) :
    derive_v.size_of l ⦃ r => sizePost l.val r ⦄ := by
  unfold derive_v.size_of
  split
  · simp only [WP.spec_ok, sizePost]
    scalar_tac
  · rename_i h
    have hl : l.val < 64 := by scalar_tac
    step as ⟨ i, hi ⟩
    have ei : i.val = l.val := by rw [hi]; scalar_tac
    step as ⟨ s, hs, hbv ⟩
    simp only [sizePost]
    refine ⟨hl, ?_⟩
    have h1 : (1#u64).val = 1 := rfl
    rw [hs, ei, Nat.shiftLeft_eq, one_mul]
    exact Nat.mod_eq_of_lt (by have := two_pow_lt_64 hl; scalar_tac)

/-- A Boolean result against the statement it decides. -/
theorem spec_false {Q : Prop} (h : ¬ Q) : (ok false : Result Bool) ⦃ b => b = true ↔ Q ⦄ :=
  (WP.spec_ok _).mpr (Iff.intro (fun e => by cases e) (fun q => absurd q h))

theorem spec_true {Q : Prop} (h : Q) : (ok true : Result Bool) ⦃ b => b = true ↔ Q ⦄ :=
  (WP.spec_ok _).mpr (Iff.intro (fun _ => h) (fun _ => rfl))

theorem spec_decide {P Q : Prop} [Decidable P] (h : P ↔ Q) :
    (ok (decide P) : Result Bool) ⦃ b => b = true ↔ Q ⦄ :=
  (WP.spec_ok _).mpr (decide_eq_true_iff.trans h)

/-- `aligned`: the exponent is from 5 to 32 and the base is a multiple of
the size. -/
theorem aligned_spec (base : U64) (l : U8) :
    derive_v.aligned base l ⦃ ok => ok = true ↔ (5 ≤ l.val ∧ l.val ≤ 32 ∧ base.val % 2 ^ l.val = 0) ⦄ := by
  unfold derive_v.aligned
  split
  · rename_i hlt
    exact spec_false (fun h => by scalar_tac)
  · split
    · rename_i hgt
      exact spec_false (fun h => by scalar_tac)
    · step as ⟨ o, ho ⟩
      cases o with
      | none =>
        simp only [sizePost] at ho
        exfalso
        scalar_tac
      | some sz =>
        simp only [sizePost] at ho
        obtain ⟨_, hsz⟩ := ho
        have hpos : 0 < sz.val := by rw [hsz]; positivity
        dsimp only
        step as ⟨ i, hi ⟩
        refine decide_eq_true_iff.trans (Iff.intro (fun h => ⟨by scalar_tac, by scalar_tac, ?_⟩) (fun h => ?_))
        · rw [← hsz, ← hi, h]
          rfl
        · have : i.val = 0 := by rw [hi, hsz]; exact h.2.2
          scalar_tac

/-- `stack_fits`: the stack is not empty and not larger than the RAM
region. -/
theorem stack_fits_spec (stack : U64) (l : U8) :
    derive_v.stack_fits stack l ⦃ ok =>
      ok = true ↔ (l.val < 64 ∧ 0 < stack.val ∧ stack.val ≤ 2 ^ l.val) ⦄ := by
  unfold derive_v.stack_fits
  step as ⟨ o, ho ⟩
  cases o with
  | none =>
    simp only [sizePost] at ho
    exact spec_false (fun h => by omega)
  | some sz =>
    simp only [sizePost] at ho
    obtain ⟨hl, hsz⟩ := ho
    dsimp only
    split
    · rename_i hpos
      refine spec_decide (Iff.intro (fun h => ⟨hl, by scalar_tac, ?_⟩) (fun h => ?_))
      · rw [← hsz]
        scalar_tac
      · have := h.2.2
        rw [← hsz] at this
        scalar_tac
    · rename_i hz
      exact spec_false (fun h => by scalar_tac)

/-- Two intervals of positive length share no address exactly when one
ends at or before the start of the other. -/
theorem intervals_disjoint (ab sa bb sb : Nat) (ha : 0 < sa) (hb : 0 < sb) :
    (ab + sa ≤ bb ∨ bb + sb ≤ ab) ↔ ∀ x, ¬ (ab ≤ x ∧ x < ab + sa ∧ bb ≤ x ∧ x < bb + sb) := by
  constructor
  · intro h x hx
    omega
  · intro h
    by_contra hc
    exact h (max ab bb) (by omega)

/-- `disjoint`: true exactly when both exponents are below 64 and no
address lies in both regions. -/
theorem disjoint_spec (ab : U64) (al : U8) (bb : U64) (bl : U8) :
    derive_v.disjoint ab al bb bl ⦃ ok =>
      ok = true ↔ (al.val < 64 ∧ bl.val < 64 ∧
        ∀ x, ¬ (ab.val ≤ x ∧ x < ab.val + 2 ^ al.val ∧ bb.val ≤ x ∧ x < bb.val + 2 ^ bl.val)) ⦄ := by
  unfold derive_v.disjoint
  step as ⟨ o, ho ⟩
  step as ⟨ o1, ho1 ⟩
  cases o with
  | none =>
    simp only [sizePost] at ho
    exact spec_false (fun h => by omega)
  | some sa =>
    simp only [sizePost] at ho
    obtain ⟨hal, hsa⟩ := ho
    cases o1 with
    | none =>
      simp only [sizePost] at ho1
      exact spec_false (fun h => by omega)
    | some sb =>
      simp only [sizePost] at ho1
      obtain ⟨hbl, hsb⟩ := ho1
      have hpa : 0 < sa.val := by rw [hsa]; positivity
      have hpb : 0 < sb.val := by rw [hsb]; positivity
      have hd := intervals_disjoint ab.val sa.val bb.val sb.val hpa hpb
      rw [hsa, hsb] at hd
      dsimp only
      step as ⟨ i, hi ⟩
      step as ⟨ i1, hi1 ⟩
      have e0 : i.val = ab.val := by rw [hi]; scalar_tac
      have e1 : i1.val = sa.val := by rw [hi1]; scalar_tac
      step as ⟨ ea, hea ⟩
      step as ⟨ i2, hi2 ⟩
      step as ⟨ i3, hi3 ⟩
      have e2 : i2.val = bb.val := by rw [hi2]; scalar_tac
      have e3 : i3.val = sb.val := by rw [hi3]; scalar_tac
      step as ⟨ eb, heb ⟩
      step as ⟨ i4, hi4 ⟩
      have e4 : i4.val = bb.val := by rw [hi4]; scalar_tac
      split
      · rename_i hle
        have hfirst : ab.val + 2 ^ al.val ≤ bb.val := by rw [← hsa]; scalar_tac
        exact spec_true ⟨hal, hbl, hd.mp (Or.inl hfirst)⟩
      · rename_i hgt
        have hnot : ¬ ab.val + 2 ^ al.val ≤ bb.val := by rw [← hsa]; scalar_tac
        step as ⟨ i5, hi5 ⟩
        have e5 : i5.val = ab.val := by rw [hi5]; scalar_tac
        refine decide_eq_true_iff.trans (Iff.intro (fun h => ⟨hal, hbl, hd.mp (Or.inr ?_)⟩) (fun h => ?_))
        · rw [← hsb]
          scalar_tac
        · rcases hd.mpr h.2.2 with h1 | h1
          · exact absurd h1 hnot
          · rw [← hsb] at h1
            scalar_tac

/-! ## The Priority levels in use (R23.5): `levels_of`

The source pushes at the end and inserts elsewhere: Rust's `Vec::insert`
admits the length as an index, the Aeneas library's model of it does not,
so with `insert` alone the translation would fail on every non-empty
input. -/

/-- Strictly ascending. -/
def Asc (l : List U64) : Prop := l.Pairwise (fun a b => a.val < b.val)

/-- `x` is one of the first `k` Priorities. -/
def Seen (prios : List U64) (k : Nat) (x : U64) : Prop := ∃ j, j < k ∧ prios[j]! = x

theorem seen_succ (prios : List U64) (k : Nat) (x : U64) :
    Seen prios (k + 1) x ↔ Seen prios k x ∨ prios[k]! = x := by
  constructor
  · rintro ⟨j, hj, he⟩
    by_cases h : j = k
    · subst h; exact Or.inr he
    · exact Or.inl ⟨j, by omega, he⟩
  · rintro (⟨j, hj, he⟩ | he)
    · exact ⟨j, by omega, he⟩
    · exact ⟨k, by omega, he⟩

theorem seen_all (prios : List U64) (x : U64) : Seen prios prios.length x ↔ x ∈ prios := by
  constructor
  · rintro ⟨j, hj, he⟩
    rw [← he, getElem!_pos prios j hj]
    exact List.getElem_mem hj
  · intro hx
    obtain ⟨j, hj, he⟩ := List.mem_iff_getElem.mp hx
    exact ⟨j, hj, by rw [getElem!_pos prios j hj]; exact he⟩

theorem insertIdx_take_drop (l : List U64) (n : Nat) (a : U64) (h : n ≤ l.length) :
    l.insertIdx n a = l.take n ++ a :: l.drop n := by
  induction l generalizing n with
  | nil =>
    have : n = 0 := by simpa using h
    subst this
    simp
  | cons x xs ih =>
    cases n with
    | zero => simp
    | succ m =>
      have := ih m (by simpa using h)
      simp [this]

/-- The first position whose value is not below `p`, in an ascending
vector: every value before it is below `p`, and the one at it, if any, is
not. -/
theorem find_spec (out : alloc.vec.Vec U64) (p : U64) :
    derive_v.levels_of_loop0_loop0 out p 0#usize ⦃ r =>
      r.val ≤ out.val.length ∧ (∀ x ∈ out.val.take r.val, x.val < p.val) ∧
      (∀ h : r.val < out.val.length, p.val ≤ (out.val[r.val]).val) ⦄ := by
  unfold derive_v.levels_of_loop0_loop0
  apply loop.spec_decr_nat (fun (pos : Usize) => out.val.length - pos.val)
    (fun pos => pos.val ≤ out.val.length ∧ ∀ x ∈ out.val.take pos.val, x.val < p.val)
  · intro pos hs
    obtain ⟨h1, h2⟩ := hs
    unfold derive_v.levels_of_loop0_loop0.body
    simp only []
    split
    · rename_i hlt
      have hlt' : pos.val < out.val.length := by scalar_tac
      step as ⟨ x, hx ⟩
      split
      · rename_i hxp
        step as ⟨ pos1, hp1 ⟩
        have e1 : pos1.val = pos.val + 1 := by scalar_tac
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro y hy
        rw [e1, List.take_succ] at hy
        simp only [List.mem_append] at hy
        rcases hy with hy | hy
        · exact h2 y hy
        · simp [List.getElem?_eq_getElem hlt'] at hy
          rw [hy, ← hx]
          scalar_tac
      · rename_i hxp
        simp only [WP.spec_ok]
        refine ⟨h1, h2, fun _ => ?_⟩
        rw [← hx]
        scalar_tac
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨h1, h2, fun h => by exfalso; scalar_tac⟩
  · exact ⟨by scalar_tac, by simp⟩

theorem levels_body_spec (prios out : alloc.vec.Vec U64) (k : Usize)
    (hk : k.val ≤ prios.val.length) (hlen : out.val.length ≤ k.val) (hasc : Asc out.val)
    (hmem : ∀ x, x ∈ out.val ↔ Seen prios.val k.val x) :
    derive_v.levels_of_loop0.body prios out k ⦃ r =>
      match r with
      | .done o => Asc o.val ∧ ∀ x, x ∈ o.val ↔ x ∈ prios.val
      | .cont (out', k') => k'.val = k.val + 1 ∧ k'.val ≤ prios.val.length ∧
          out'.val.length ≤ k'.val ∧ Asc out'.val ∧ ∀ x, x ∈ out'.val ↔ Seen prios.val k'.val x ⦄ := by
  unfold derive_v.levels_of_loop0.body
  simp only []
  split
  · rename_i hlt
    have hlt' : k.val < prios.val.length := by scalar_tac
    step as ⟨ p, hp ⟩
    have hp' : (prios.val)[k.val]! = p := by rw [hp]; exact getElem!_pos prios.val k.val hlt'
    have hpos := find_spec out p
    step as ⟨ pos, hpl, hbefore, hat ⟩
    have hsucc : ∀ x, Seen prios.val (k.val + 1) x ↔ x ∈ out.val ∨ x = p := by
      intro x
      rw [seen_succ, hmem x, hp']
      constructor
      · rintro (h | h)
        · exact Or.inl h
        · exact Or.inr h.symm
      · rintro (h | h)
        · exact Or.inl h
        · exact Or.inr h.symm
    have hroom : out.val.length < Usize.max := by scalar_tac
    by_cases hend : pos = out.len
    · have hposv : pos.val = out.val.length := by rw [hend]; scalar_tac
      simp only [hend, if_true]
      step as ⟨ out1, ho1 ⟩
      step as ⟨ k1, hk1 ⟩
      have e1 : k1.val = k.val + 1 := by scalar_tac
      rw [hposv, List.take_length] at hbefore
      refine ⟨e1, by scalar_tac, by rw [ho1]; simp; omega, ?_, ?_⟩
      · rw [ho1]
        exact List.pairwise_append.mpr ⟨hasc, List.pairwise_singleton _ _, fun a ha b hb => by
          simp only [List.mem_singleton] at hb
          rw [hb]
          exact hbefore a ha⟩
      · intro x
        rw [e1, hsucc x, ho1]
        simp [List.mem_append]
    · have hpl' : pos.val < out.val.length := by
        have : pos.val ≠ out.val.length := fun h => hend (by scalar_tac)
        omega
      simp only [hend, if_false]
      step as ⟨ i2, hi2 ⟩
      have hm : ∀ x, (x ∈ out.val.take pos.val ∨ x ∈ out.val.drop pos.val) ↔ x ∈ out.val := fun x => by
        rw [← List.mem_append, List.take_append_drop]
      split
      · rename_i hne
        have hne' : i2 ≠ p := by simpa using hne
        have hgt : p.val < (out.val[pos.val]).val := by
          have h1 := hat hpl'
          have h2 : i2.val ≠ p.val := fun h => hne' (by scalar_tac)
          rw [hi2] at h2
          omega
        step as ⟨ out1, ho1 ⟩
        step as ⟨ k1, hk1 ⟩
        have e1 : k1.val = k.val + 1 := by scalar_tac
        rw [insertIdx_take_drop out.val pos.val p (by omega)] at ho1
        have hdrop := hasc.sublist (List.drop_sublist pos.val out.val)
        rw [List.drop_eq_getElem_cons hpl'] at hdrop
        have hafter : ∀ y ∈ out.val.drop pos.val, p.val < y.val := by
          intro y hy
          rw [List.drop_eq_getElem_cons hpl'] at hy
          rcases List.mem_cons.mp hy with h | h
          · rw [h]; exact hgt
          · exact Nat.lt_trans hgt ((List.pairwise_cons.mp hdrop).1 y h)
        refine ⟨e1, by scalar_tac, by rw [ho1]; simp; omega, ?_, ?_⟩
        · rw [ho1]
          refine List.pairwise_append.mpr ⟨hasc.sublist (List.take_sublist _ _),
            List.pairwise_cons.mpr ⟨hafter, hasc.sublist (List.drop_sublist _ _)⟩, ?_⟩
          intro a ha b hb
          rcases List.mem_cons.mp hb with h | h
          · rw [h]; exact hbefore a ha
          · exact Nat.lt_trans (hbefore a ha) (hafter b h)
        · intro x
          rw [e1, hsucc x, ho1, ← hm x]
          simp only [List.mem_append, List.mem_cons]
          constructor
          · rintro (h | h | h)
            · exact Or.inl (Or.inl h)
            · exact Or.inr h
            · exact Or.inl (Or.inr h)
          · rintro ((h | h) | h)
            · exact Or.inl h
            · exact Or.inr (Or.inr h)
            · exact Or.inr (Or.inl h)
      · rename_i heq
        have heqv : i2.val = p.val := by simpa using heq
        have heq' : i2 = p := by scalar_tac
        simp only [bind_ok]
        step as ⟨ k1, hk1 ⟩
        have e1 : k1.val = k.val + 1 := by scalar_tac
        have hin : p ∈ out.val := by rw [← heq', hi2]; exact List.getElem_mem hpl'
        refine ⟨e1, by scalar_tac, by omega, hasc, ?_⟩
        intro x
        rw [e1, hsucc x]
        constructor
        · intro h; exact Or.inl h
        · rintro (h | h)
          · exact h
          · rw [h]; exact hin
  · rename_i hge
    have heq : k.val = prios.val.length := by scalar_tac
    simp only [WP.spec_ok]
    refine ⟨hasc, fun x => ?_⟩
    rw [hmem x, heq]
    exact seen_all prios.val x

/-- R23.5: `levels_of` returns the Priorities in use, each once, in
ascending order. -/
theorem levels_of_spec (prios : alloc.vec.Vec U64) :
    derive_v.levels_of prios ⦃ out => Asc out.val ∧ ∀ x, x ∈ out.val ↔ x ∈ prios.val ⦄ := by
  unfold derive_v.levels_of derive_v.levels_of_loop0
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec U64 × Usize) => prios.val.length - s.2.val)
    (fun s => s.2.val ≤ prios.val.length ∧ s.1.val.length ≤ s.2.val ∧ Asc s.1.val
      ∧ ∀ x, x ∈ s.1.val ↔ Seen prios.val s.2.val x)
  · intro s hs
    obtain ⟨out, k⟩ := s
    obtain ⟨h1, h2, h3, h4⟩ := hs
    simp only [] at h1 h2 h3 h4
    apply WP.spec_mono (levels_body_spec prios out k h1 h2 h3 h4)
    intro r hr
    cases r with
    | done o => simpa using hr
    | cont s' =>
      obtain ⟨out', k'⟩ := s'
      simp only [] at hr ⊢
      obtain ⟨e1, e2, e3, e4, e5⟩ := hr
      exact ⟨⟨e2, e3, e4, e5⟩, by omega⟩
  · refine ⟨by scalar_tac, by simp, by simp [Asc], fun x => ?_⟩
    simp [Seen]

end rsk_confcheck
