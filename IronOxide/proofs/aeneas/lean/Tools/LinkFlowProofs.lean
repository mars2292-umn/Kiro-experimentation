import Aeneas
import Tools.Link
open Aeneas Aeneas.Std Result

/-! # The Link_Checker's body certificate on the code as compiled (2026-11-27)

`flow_v::flow_ok` of `rsk-link` (verified in Verus,
`crates/rsk-link/src/flow_v.rs`) as Aeneas extracts it (`Tools/Link.lean`):
when it returns true, the list of a body's instructions, the `it` marks
and the heights it was given satisfy `FlowL` (`flow_ok_sound`). `FlowL`
restates `flow_v::flow_spec`:

- `Ordered`: the list is in strictly increasing address order; the first
  instruction is at the body's first address and its height is 0.
- `NodeOk` for every instruction: `PlacedL` (in the body, with the length
  its first halfword gives); its height within the bounds and, with its
  own decrement (`downL`), within the frame; `FallOk` (the next address is
  an instruction of the list with the height needed, `Lands`, unless the
  instruction stops, `Stops`, or is a call that ends the body);
  `BranchOk` (the target of a branch or `cbz` in the body lands; every
  entry of the table of a `tbb`/`tbh [pc, Rm]` up to the next instruction
  of the list is the padding or lands, `TableOk`, `SlotOk`).
- `ItOk` for every instruction: when it is an `it`, each instruction of
  the list at one of the addresses it governs (`Chain`, `LenAt`) is
  marked.

The bytes are read through the compiled readers: where the Verus text has
`hw_spec`, `hard`, `len_spec`, this file has the results of the extracted
`flow_v.hw`, `flow_v.hard_of`, `flow_v.len_of`, `flow_v.halves`, which are
functions of their arguments. One lemma for each function under
`flow_ok`, in the form "if it returned true (or this value), then"; the
six loops by induction on the distance to the bound.

The direction from true only, as Verus proves. Not proven here: that
`flow_ok` returns for every input. `Tools/LinkFlowCert.lean` joins `FlowL`
to the descent theorem of `Tools/LinkFlow.lean`. Handwritten; no `sorry`. -/

namespace rsk_link
namespace FlowP

/-! ## What a returned value says -/

theorem bind_eq_ok {α β} (x : Result α) (f : α → Result β) (y : β) :
    Std.bind x f = ok y ↔ ∃ a, x = ok a ∧ f a = ok y := by
  cases x <;> simp

theorem ok_inj {α} {x y : α} (h : (ok x : Result α) = ok y) : x = y := by
  simpa using h

theorem of_spec {α} {f : Result α} {P : α → Prop} {x : α} (hs : f ⦃ P ⦄) (h : f = ok x) : P x := by
  rw [h] at hs
  exact (WP.spec_ok x).mp hs

theorem uadd {ty} {a b c : UScalar ty} (h : a + b = ok c) : c.val = a.val + b.val := by
  have := UScalar.add_equiv a b
  simp_all

theorem usub {ty} {a b c : UScalar ty} (h : a - b = ok c) :
    b.val ≤ a.val ∧ a.val = c.val + b.val := by
  have := UScalar.sub_equiv a b
  simp_all

theorem iadd {ty} {a b c : IScalar ty} (h : a + b = ok c) : c.val = a.val + b.val := by
  have := IScalar.add_equiv a b
  simp_all

instance : Inhabited thumb.Insn := ⟨⟨0#u32, 0#u32, .Other⟩⟩

theorem vidx {α} [Inhabited α] {v : alloc.vec.Vec α} {i : Usize} {x : α}
    (hlt : i.val < v.val.length)
    (h : alloc.vec.Vec.index (core.slice.index.SliceIndexUsizeSlice α) v i = ok x) :
    x = v.val[i.val]! := by
  have hs : alloc.vec.Vec.index (core.slice.index.SliceIndexUsizeSlice α) v i
      ⦃ y => y = v.val[i.val]! ⦄ := by
    step as ⟨ y, hy ⟩
    rw [hy]; exact (getElem!_pos v.val i.val hlt).symm
  exact of_spec (P := fun y => y = v.val[i.val]!) hs h

theorem len64 (code : Slice U8) :
    (UScalar.cast .U64 (Slice.len code)).val = code.val.length := by
  have e : (UScalar.cast .U64 (Slice.len code)).val
      = (Slice.len code).val % 18446744073709551616 := UScalar.cast_val_eq .U64 _
  have hb : (Slice.len code).val < 18446744073709551616 := by
    have hq := (Slice.len code).hBounds
    simp only [UScalarTy.numBits] at hq
    rcases System.Platform.numBits_eq with h | h <;> rw [h] at hq <;> omega
  rw [e, Nat.mod_eq_of_lt hb]
  simp

/-! ## Landing -/

/-- The address `t` is an instruction of the list whose height is at least
`need`. `flow_v::lands`. -/
def Lands (insns : List thumb.Insn) (d : List I64) (t : Nat) (need : Int) : Prop :=
  ∃ j, j < insns.length ∧ (insns[j]!).address.val = t ∧ need ≤ (d[j]!).val

theorem lands_at_ok {insns : alloc.vec.Vec thumb.Insn} {d : alloc.vec.Vec I64} {t : U32}
    {need : I64} {j : Usize} (hd : d.val.length = insns.val.length)
    (h : flow_v.lands_at insns d t need j = ok true) :
    j.val < insns.val.length ∧ (insns.val[j.val]!).address.val = t.val ∧
      need.val ≤ (d.val[j.val]!).val := by
  unfold flow_v.lands_at at h
  dsimp only at h
  split at h
  · rename_i hj
    have hjl : j.val < insns.val.length := by scalar_tac
    simp only [bind_eq_ok] at h
    obtain ⟨x, hx, h⟩ := h
    have ex := vidx hjl hx
    split at h
    · rename_i hat
      simp only [bind_eq_ok] at h
      obtain ⟨y, hy, h⟩ := h
      have ey := vidx (by omega) hy
      have hge := ok_inj h
      refine ⟨hjl, ?_, ?_⟩
      · rw [← ex, hat]
      · rw [← ey]; simp at hge; scalar_tac
    · exact absurd (ok_inj h) (by simp)
  · exact absurd (ok_inj h) (by simp)

theorem lands_loop_ok (insns : alloc.vec.Vec thumb.Insn) (d : alloc.vec.Vec I64) (t : U32)
    (need : I64) (hd : d.val.length = insns.val.length) :
    ∀ (n : Nat) (found : Bool) (j : Usize), insns.val.length - j.val = n →
      flow_v.lands_check_loop insns d t need found j = ok true →
      found = true ∨ Lands insns.val d.val t.val need.val := by
  intro n
  induction n with
  | zero =>
    intro found j hn h
    unfold flow_v.lands_check_loop loop at h
    simp only [flow_v.lands_check_loop.body] at h
    have hlt : ¬ j < alloc.vec.Vec.len insns := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    exact Or.inl (ok_inj h)
  | succ n ih =>
    intro found j hn h
    unfold flow_v.lands_check_loop loop at h
    simp only [flow_v.lands_check_loop.body] at h
    have hlt : j < alloc.vec.Vec.len insns := by scalar_tac
    simp only [hlt, ite_true] at h
    cases found with
    | true => exact Or.inl rfl
    | false =>
      simp only [Bool.false_eq_true, ite_false, bind_eq_ok, bind_ok] at h
      obtain ⟨cf, ⟨f1, hf1, j1, hj1, hcf⟩, hloop⟩ := h
      have hcf := ok_inj hcf
      subst hcf
      have ej := uadd hj1
      have hrec : flow_v.lands_check_loop insns d t need f1 j1 = ok true := by
        unfold flow_v.lands_check_loop
        simp only [flow_v.lands_check_loop.body]
        exact hloop
      rcases ih f1 j1 (by scalar_tac) hrec with hf | hl
      · subst hf
        obtain ⟨a, b, c⟩ := lands_at_ok hd hf1
        exact Or.inr ⟨j.val, a, b, c⟩
      · exact Or.inr hl

theorem lands_check_ok {insns : alloc.vec.Vec thumb.Insn} {d : alloc.vec.Vec I64} {t : U32}
    {need : I64} {hint : Usize} (hd : d.val.length = insns.val.length)
    (h : flow_v.lands_check insns d t need hint = ok true) :
    Lands insns.val d.val t.val need.val := by
  unfold flow_v.lands_check at h
  simp only [bind_eq_ok] at h
  obtain ⟨found, hf, h⟩ := h
  rcases lands_loop_ok insns d t need hd _ found 0#usize rfl h with hfd | hl
  · subst hfd
    obtain ⟨a, b, c⟩ := lands_at_ok hd hf
    exact ⟨hint.val, a, b, c⟩
  · exact hl

/-! ## Address order -/

/-- The list is in strictly increasing address order. -/
def Ordered (insns : List thumb.Insn) : Prop :=
  ∀ i, i + 1 < insns.length → (insns[i]!).address.val < (insns[i + 1]!).address.val

theorem order_loop_ok (insns : alloc.vec.Vec thumb.Insn) (n : Usize)
    (hn : n.val = insns.val.length) :
    ∀ (m : Nat) (i : Usize), n.val - i.val = m →
      flow_v.order_check_loop insns n i = ok true →
      ∀ k, i.val ≤ k → k + 1 < insns.val.length →
        (insns.val[k]!).address.val < (insns.val[k + 1]!).address.val := by
  intro m
  induction m with
  | zero =>
    intro i hm h k hk hk1
    omega
  | succ m ih =>
    intro i hm h k hk hk1
    unfold flow_v.order_check_loop loop at h
    simp only [flow_v.order_check_loop.body, bind_eq_ok] at h
    obtain ⟨cf, ⟨i1, hi1, hb⟩, hloop⟩ := h
    obtain ⟨h1n, e1⟩ := usub hi1
    have o1 : (1#usize).val = 1 := rfl
    rw [o1] at h1n e1
    split at hb
    · rename_i hlt
      have hil : i.val + 1 < insns.val.length := by scalar_tac
      simp only [bind_eq_ok] at hb
      obtain ⟨x, hx, i3, hi3, y, hy, hb⟩ := hb
      have e3 := uadd hi3
      rw [o1] at e3
      have ex := vidx (by omega) hx
      have ey := vidx (by omega) hy
      split at hb
      · have hb := ok_inj hb
        subst hb
        simp at hloop
      · rename_i hge
        have hb := ok_inj hb
        subst hb
        have hrec : flow_v.order_check_loop insns n i3 = ok true := by
          unfold flow_v.order_check_loop
          simp only [flow_v.order_check_loop.body]
          exact hloop
        by_cases hki : k = i.val
        · subst hki
          rw [← ex, ← e3, ← ey]
          scalar_tac
        · exact ih i3 (by omega) hrec k (by omega) hk1
    · rename_i hlt
      have : ¬ i.val < i1.val := by scalar_tac
      omega

theorem order_check_ok {insns : alloc.vec.Vec thumb.Insn}
    (h : flow_v.order_check insns = ok true) : Ordered insns.val := by
  unfold flow_v.order_check at h
  dsimp only at h
  intro k hk
  exact order_loop_ok insns _ (by simp [alloc.vec.Vec.len]) _ 0#usize rfl h k (by simp) hk

/-! ## Weights -/

/-- What an instruction adds to the height. `flow_v::delta`. -/
def deltaL (e : thumb.Effect) (c : Bool) : Int :=
  match e with
  | .SpDown n => (n.val : Int)
  | .SpUp n => if c then 0 else -(n.val : Int)
  | _ => 0

/-- The decrement an instruction makes. `bound_v::down`. -/
def downL (e : thumb.Effect) : Nat :=
  match e with
  | .SpDown n => n.val
  | _ => 0

theorem hcast64 (x : U32) : (UScalar.hcast .I64 x).val = (x.val : Int) := by
  have e : (UScalar.hcast .I64 x).val = Int.bmod (x.val : Int) 18446744073709551616 :=
    UScalar.hcast_val_eq .I64 x
  have h : x.val < 4294967296 := by scalar_tac
  rw [e, Int.bmod_def]
  split <;> omega

theorem ineg {x r : I64} (hx : 0 ≤ x.val) (h : IScalar.neg x = ok r) : r.val = -x.val := by
  have hne : (x.val : Int) ≠ IScalar.min .I64 := by
    intro e
    rw [e] at hx
    scalar_tac
  exact of_spec (P := fun r : I64 => r.val = -x.val) (IScalar.neg_step x hne) h

theorem delta_of_ok {e : thumb.Effect} {c : Bool} {r : I64} (h : flow_v.delta_of e c = ok r) :
    r.val = deltaL e c := by
  unfold flow_v.delta_of at h
  cases e <;> simp only [deltaL] at h ⊢ <;>
    first
    | (have h := ok_inj h; subst h; rfl)
    | (have h := ok_inj h; subst h; exact hcast64 _)
    | (cases c
       · simp only [Bool.false_eq_true, ite_false, lift, bind_ok] at h ⊢
         rw [ineg (by rw [hcast64]; omega) h, hcast64]
       · simp only [ite_true] at h ⊢
         have h := ok_inj h; subst h; rfl)

/-! ## Tables -/

theorem umul {ty} {a b c : UScalar ty} (h : a * b = ok c) : c.val = a.val * b.val := by
  have := UScalar.mul_equiv a b
  have e : a * b = UScalar.mul a b := rfl
  rw [e] at h
  simp_all

/-- `e` is the entry at the address `pos` of a table: a halfword (`wide`)
or a byte of the body, as the compiled check reads it. -/
def EntryAt (code : Slice U8) (base : U32) (wide : Bool) (pos e : Nat) : Prop :=
  ∃ off : Usize, off.val + base.val = pos ∧
    (if wide = true then ∃ h, flow_v.hw code off = ok h ∧ h.val = e
     else ∃ b, Slice.index_usize code off = ok b ∧ b.val = e)

/-- The entry at `pos` of the table of the instruction at `a` is the byte
that fills a `tbb` table to a halfword, or branches to an instruction of
the list in the body whose height is at least `need`. `flow_v::tslot_ok`. -/
def SlotOk (code : Slice U8) (base : U32) (insns : List thumb.Insn) (d : List I64)
    (a : Nat) (wide : Bool) (lim pos : Nat) (need : Int) : Prop :=
  ∃ e, EntryAt code base wide pos e ∧
    ((wide = false ∧ pos + 1 = lim ∧ a + 4 + 2 * e < lim) ∨
     (a + 4 + 2 * e < base.val + code.val.length ∧ Lands insns d (a + 4 + 2 * e) need))

theorem slot_check_ok {code : Slice U8} {base : U32} {insns : alloc.vec.Vec thumb.Insn}
    {d : alloc.vec.Vec I64} {a : U32} {wide : Bool} {lim pos : U32} {need : I64}
    (hd : d.val.length = insns.val.length) (hend : base.val + code.val.length ≤ 4294967295)
    (h : flow_v.slot_check code base insns d a wide lim pos need = ok true) :
    SlotOk code base insns.val d.val a.val wide lim.val pos.val need.val := by
  unfold flow_v.slot_check at h
  dsimp only at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨en, hen, i3, hi3, e, he, i5, hi5, i7, hi7, t, ht, pad, hpad, h⟩ := h
  have een := uadd hen
  obtain ⟨_, e3⟩ := usub hi3
  have e5 := uadd hi5
  have e7 := umul hi7
  have et := uadd ht
  have c1 : (UScalar.cast .U64 base).val = base.val := U32.cast_U64_val_eq _
  have c2 := len64 code
  have c3 : (UScalar.cast .U64 a).val = a.val := U32.cast_U64_val_eq _
  have c4 : (UScalar.cast .U64 e).val = e.val := U32.cast_U64_val_eq _
  have o4 : (4#u64).val = 4 := rfl
  have o2 : (2#u64).val = 2 := rfl
  rw [c1, c2] at een
  rw [c3, o4] at e5
  rw [c4, o2] at e7
  have tv : t.val = a.val + 4 + 2 * e.val := by omega
  have hoff : (UScalar.cast .Usize i3).val + base.val = pos.val := by
    rw [U32.cast_Usize_val_eq]; omega
  have hE : EntryAt code base wide pos.val e.val := by
    refine ⟨_, hoff, ?_⟩
    cases wide
    · simp only [Bool.false_eq_true, ite_false, bind_eq_ok] at he ⊢
      obtain ⟨b, hb, he⟩ := he
      have he := ok_inj he
      subst he
      exact ⟨b, hb, (U8.cast_U32_val_eq b).symm⟩
    · simp only [ite_true] at he ⊢
      exact ⟨e, he, rfl⟩
  refine ⟨e.val, hE, ?_⟩
  cases pad
  · simp only [Bool.false_eq_true, ite_false] at h
    split at h
    · cases (ok_inj h)
    · rename_i hlt
      have hl := lands_check_ok hd h
      have hlt' : t.val < en.val := by clear * - hlt; scalar_tac
      have htl : t.val < base.val + code.val.length := by omega
      have c : (UScalar.cast .U32 t).val = t.val := by
        have e : (UScalar.cast .U32 t).val = t.val % 4294967296 := UScalar.cast_val_eq .U32 t
        rw [e]; omega
      rw [c, tv] at hl
      exact Or.inr ⟨by omega, hl⟩
  · left
    cases wide
    · simp only [Bool.false_eq_true, ite_false, bind_eq_ok] at hpad
      obtain ⟨i8, hi8, hpad⟩ := hpad
      have e8 := uadd hi8
      have o1 : (1#u32).val = 1 := rfl
      rw [o1] at e8
      split at hpad
      · rename_i heq
        have hp := ok_inj hpad
        have c5 : (UScalar.cast .U64 lim).val = lim.val := U32.cast_U64_val_eq _
        have hp' : t.val < (UScalar.cast .U64 lim).val := by clear * - hp; simp at hp; scalar_tac
        rw [c5] at hp'
        refine ⟨rfl, by rw [← heq]; omega, ?_⟩
        rw [← tv]; exact hp'
      · cases (ok_inj hpad)
    · simp only [ite_true] at hpad
      cases (ok_inj hpad)

theorem table_loop_ok (code : Slice U8) (base : U32) (insns : alloc.vec.Vec thumb.Insn)
    (d : alloc.vec.Vec I64) (lim a : U32) (wide : Bool) (sz : U32) (need : I64)
    (hd : d.val.length = insns.val.length) (hend : base.val + code.val.length ≤ 4294967295)
    (hsz : sz.val = if wide = true then 2 else 1) :
    ∀ (n : Nat) (pos : U32) (good : Bool), lim.val - pos.val ≤ n →
      flow_v.table_check_loop code base insns d lim a wide sz need pos good = ok true →
      good = true ∧ ∀ q, pos.val ≤ q → q < lim.val → (q - pos.val) % sz.val = 0 →
        SlotOk code base insns.val d.val a.val wide lim.val q need.val := by
  have hsz12 : sz.val = 1 ∨ sz.val = 2 := by
    rw [hsz]; split <;> simp
  have stop : ∀ (pos : U32) (good : Bool), ¬ pos.val < lim.val →
      flow_v.table_check_loop code base insns d lim a wide sz need pos good = ok true →
      good = true ∧ ∀ q, pos.val ≤ q → q < lim.val → (q - pos.val) % sz.val = 0 →
        SlotOk code base insns.val d.val a.val wide lim.val q need.val := by
    intro pos good hge h
    unfold flow_v.table_check_loop loop at h
    simp only [flow_v.table_check_loop.body] at h
    have hlt : ¬ pos < lim := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    exact ⟨ok_inj h, fun q h1 h2 _ => by omega⟩
  intro n
  induction n with
  | zero =>
    intro pos good hn h
    exact stop pos good (by omega) h
  | succ n ih =>
    intro pos good hn h
    by_cases hpl : pos.val < lim.val
    · unfold flow_v.table_check_loop loop at h
      simp only [flow_v.table_check_loop.body] at h
      have hlt : pos < lim := by scalar_tac
      simp only [hlt, ite_true] at h
      cases good with
      | false =>
        simp only [Bool.false_eq_true, ite_false, bind_ok] at h
        cases (ok_inj h)
      | true =>
        simp only [ite_true, bind_eq_ok, bind_ok] at h
        obtain ⟨cf, ⟨g1, hg1, p1, hp1, hcf⟩, hloop⟩ := h
        have hcf := ok_inj hcf
        subst hcf
        have ep := uadd hp1
        have hrec : flow_v.table_check_loop code base insns d lim a wide sz need p1 g1
            = ok true := by
          unfold flow_v.table_check_loop
          simp only [flow_v.table_check_loop.body]
          exact hloop
        obtain ⟨hg, hall⟩ := ih p1 g1 (by omega) hrec
        subst hg
        refine ⟨rfl, fun q hq1 hq2 hq3 => ?_⟩
        by_cases hqp : q = pos.val
        · subst hqp
          exact slot_check_ok hd hend hg1
        · refine hall q ?_ hq2 ?_
          · rcases hsz12 with e | e <;> rw [e] at hq3 ep <;> omega
          · rcases hsz12 with e | e <;> rw [e] at hq3 ep ⊢ <;> omega
    · exact stop pos good hpl h

/-- The table of the `tbb`/`tbh [pc, Rm]` that is `x`: the bytes from the
instruction to `lim`, every one of them an entry. `flow_v::table_ok`. -/
def TableOk (code : Slice U8) (base : U32) (insns : List thumb.Insn) (d : List I64)
    (x : thumb.Insn) (lim : Nat) (need : Int) : Prop :=
  ∃ h1 h2 : U32, flow_v.halves code base x = ok (h1, h2) ∧ h1 = 59615#u32 ∧
    x.address.val + 4 ≤ lim ∧ lim ≤ base.val + code.val.length ∧
    (lim - x.address.val - 4) % (if (h2 &&& 16#u32 != 0#u32) = true then 2 else 1) = 0 ∧
    ∀ pos, x.address.val + 4 ≤ pos → pos < lim →
      (pos - x.address.val - 4) % (if (h2 &&& 16#u32 != 0#u32) = true then 2 else 1) = 0 →
      SlotOk code base insns d x.address.val (h2 &&& 16#u32 != 0#u32) lim pos need

theorem table_check_ok {code : Slice U8} {base : U32} {insns : alloc.vec.Vec thumb.Insn}
    {d : alloc.vec.Vec I64} {i : Usize} {lim : U32}
    (hd : d.val.length = insns.val.length) (hend : base.val + code.val.length ≤ 4294967295)
    (hi : i.val < insns.val.length)
    (h : flow_v.table_check code base insns d i lim = ok true) :
    TableOk code base insns.val d.val (insns.val[i.val]!) lim.val (d.val[i.val]!).val := by
  unfold flow_v.table_check at h
  rw [bind_eq_ok] at h
  obtain ⟨x, hx, h⟩ := h
  rw [bind_eq_ok] at h
  obtain ⟨⟨h1, h2⟩, hh, h⟩ := h
  have ex := vidx hi hx
  subst ex
  simp only [Aeneas.Std.uncurry] at h
  split at h
  · cases (ok_inj h)
  rename_i hh1
  split at h
  · cases (ok_inj h)
  rename_i hlen
  try dsimp only at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨en, hen, i6, hi6, h⟩ := h
  have een := uadd hen
  have e6 := uadd hi6
  have c1 : (UScalar.cast .U64 base).val = base.val := U32.cast_U64_val_eq _
  have c2 := len64 code
  have c3 : (UScalar.cast .U64 (insns.val[i.val]!).address).val = (insns.val[i.val]!).address.val :=
    U32.cast_U64_val_eq _
  have c4 : (UScalar.cast .U64 lim).val = lim.val := U32.cast_U64_val_eq _
  have o4 : (4#u64).val = 4 := rfl
  rw [c1, c2] at een
  rw [c3, o4] at e6
  split at h
  · cases (ok_inj h)
  rename_i hlo
  have hlo' : (insns.val[i.val]!).address.val + 4 ≤ lim.val := by
    have : ¬ (UScalar.cast .U64 lim).val < i6.val := by clear * - hlo; scalar_tac
    omega
  split at h
  · cases (ok_inj h)
  rename_i hhi
  have hhi' : lim.val ≤ base.val + code.val.length := by
    have : ¬ en.val < (UScalar.cast .U64 lim).val := by clear * - hhi; scalar_tac
    omega
  simp only [bind_eq_ok] at h
  obtain ⟨sz, hsz, i9, hi9, i10, hi10, i11, hi11, h⟩ := h
  have hszv : sz.val = if (h2 &&& 16#u32 != 0#u32) = true then 2 else 1 := by
    split at hsz
    · rename_i hw
      rw [if_pos hw, ← ok_inj hsz]; rfl
    · rename_i hw
      rw [if_neg hw, ← ok_inj hsz]; rfl
  obtain ⟨_, e9⟩ := usub hi9
  obtain ⟨_, e10⟩ := usub hi10
  have o4' : (4#u32).val = 4 := rfl
  rw [o4'] at e10
  have hsz0 : sz.val ≠ 0 := by rw [hszv]; split <;> simp
  have e11 : i11.val = i10.val % sz.val := by
    have hs : (i10 % sz) ⦃ c => c.val = i10.val % sz.val ⦄ := by
      step as ⟨ c, hc ⟩
      exact hc
    exact of_spec (P := fun c : U32 => c.val = i10.val % sz.val) hs hi11
  split at h
  · cases (ok_inj h)
  rename_i hrem
  have hrem' : i11.val = 0 := by clear * - hrem; scalar_tac
  simp only [bind_eq_ok] at h
  obtain ⟨need, hneed, pos, hpos, h⟩ := h
  have en := vidx (by omega) hneed
  have ep := uadd hpos
  rw [o4'] at ep
  obtain ⟨_, hall⟩ := table_loop_ok code base insns d lim _ _ sz need hd hend hszv _ pos true
    (Nat.le_refl _) h
  have h1e : h1 = 59615#u32 := Decidable.byContradiction fun hne => hh1 (bne_iff_ne.mpr hne)
  refine ⟨h1, h2, hh, h1e, hlo', hhi', ?_, ?_⟩
  · rw [← hszv]
    have : i10.val = lim.val - (insns.val[i.val]!).address.val - 4 := by omega
    rw [← this, ← e11]; exact hrem'
  · intro q hq1 hq2 hq3
    rw [← hszv] at hq3
    rw [← en]
    exact hall q (by omega) hq2 (by rw [ep]; rw [show q - ((insns.val[i.val]!).address.val + 4) = q - (insns.val[i.val]!).address.val - 4 by omega]; exact hq3)

/-! ## One instruction -/

/-- The instruction lies in the body with the length its first halfword
gives. `flow_v::placed`. -/
def PlacedL (code : Slice U8) (base : U32) (x : thumb.Insn) : Prop :=
  base.val ≤ x.address.val ∧ (x.len.val = 2 ∨ x.len.val = 4) ∧
  x.address.val - base.val + x.len.val ≤ code.val.length ∧
  ∃ (off : Usize) (h l : U32), off.val + base.val = x.address.val ∧
    flow_v.hw code off = ok h ∧ flow_v.len_of h = ok l ∧ x.len = l

theorem placed_check_ok {code : Slice U8} {base : U32} {x : thumb.Insn}
    (h : flow_v.placed_check code base x = ok true) : PlacedL code base x := by
  unfold flow_v.placed_check at h
  split at h
  · cases (ok_inj h)
  rename_i hba
  have hlen : x.len.val = 2 ∨ x.len.val = 4 := by
    by_cases h2 : x.len = 2#u32
    · left; rw [h2]; rfl
    · by_cases h4 : x.len = 4#u32
      · right; rw [h4]; rfl
      · exfalso
        rw [if_pos (bne_iff_ne.mpr h2), if_pos (bne_iff_ne.mpr h4)] at h
        cases (ok_inj h)
  have tail : (do
        let i ← x.address - base
        let off ← lift (UScalar.cast .Usize i)
        let i1 := Slice.len code
        if off > i1
        then ok false
        else
          let i2 ← lift (UScalar.cast .Usize x.len)
          let i3 := Slice.len code
          let i4 ← i3 - off
          if i2 > i4
          then ok false
          else
            let i5 ← flow_v.hw code off
            let i6 ← flow_v.len_of i5
            ok (decide (x.len = i6))) = ok true := by
    split at h
    · split at h
      · cases (ok_inj h)
      · exact h
    · exact h
  clear h
  dsimp only at tail
  simp only [bind_eq_ok, lift, bind_ok] at tail
  obtain ⟨i0, hi0, h⟩ := tail
  obtain ⟨hb, e0⟩ := usub hi0
  have c0 : (UScalar.cast .Usize i0).val = i0.val := U32.cast_Usize_val_eq _
  have cl : (UScalar.cast .Usize x.len).val = x.len.val := U32.cast_Usize_val_eq _
  split at h
  · cases (ok_inj h)
  rename_i hoff
  simp only [bind_eq_ok] at h
  obtain ⟨i4, hi4, h⟩ := h
  obtain ⟨_, e4⟩ := usub hi4
  split at h
  · cases (ok_inj h)
  rename_i hfit
  simp only [bind_eq_ok] at h
  obtain ⟨i5, hi5, i6, hi6, h⟩ := h
  have hl : x.len = i6 := by
    have := ok_inj h
    simpa using this
  have hfit' : ¬ (UScalar.cast .Usize x.len).val > i4.val := by clear * - hfit; scalar_tac
  have hsl : (Slice.len code).val = code.val.length := by simp
  refine ⟨hb, hlen, by omega, _, i5, i6, by omega, hi5, hi6, hl⟩

/-- Control does not pass from `x` to the next address: `x` is under no
`it` of the list and `flow_v::hard_of` answers true for it. -/
def Stops (code : Slice U8) (base : U32) (noreturn : alloc.vec.Vec U32) (x : thumb.Insn)
    (c : Bool) : Prop :=
  c = false ∧ ∃ h1 h2, flow_v.halves code base x = ok (h1, h2) ∧
    flow_v.hard_of x h1 h2 noreturn = ok true

/-- The passage to the next address lands, unless the instruction stops or
is a call that ends the body. -/
def FallOk (code : Slice U8) (base : U32) (insns : List thumb.Insn) (d : List I64)
    (noreturn : alloc.vec.Vec U32) (x : thumb.Insn) (c : Bool) (need : Int) : Prop :=
  ¬ Stops code base noreturn x c →
    Lands insns d (x.address.val + x.len.val) need ∨
    ((∃ t, x.effect = .Call t) ∧ x.address.val + x.len.val = base.val + code.val.length)

theorem fall_check_ok {code : Slice U8} {base : U32} {insns : alloc.vec.Vec thumb.Insn}
    {cond : alloc.vec.Vec Bool} {noreturn : alloc.vec.Vec U32} {d : alloc.vec.Vec I64}
    {i : Usize} {need : I64}
    (hd : d.val.length = insns.val.length) (hc : cond.val.length = insns.val.length)
    (hi : i.val < insns.val.length)
    (h : flow_v.fall_check code base insns cond noreturn d i need = ok true) :
    FallOk code base insns.val d.val noreturn (insns.val[i.val]!) (cond.val[i.val]!) need.val := by
  unfold flow_v.fall_check at h
  rw [bind_eq_ok] at h
  obtain ⟨x, hx, h⟩ := h
  have ex := vidx hi hx
  subst ex
  dsimp only at h
  simp only [bind_eq_ok, lift, bind_ok, Prod.exists, Aeneas.Std.uncurry] at h
  obtain ⟨en, hen, h1, h2, hh, next, hnext, b, hb, stops, hst, h⟩ := h
  have eb := vidx (by omega) hb
  subst eb
  have een := uadd hen
  have enx := uadd hnext
  have c1 : (UScalar.cast .U64 base).val = base.val := U32.cast_U64_val_eq _
  have c2 := len64 code
  have c3 : (UScalar.cast .U64 (insns.val[i.val]!).address).val = (insns.val[i.val]!).address.val :=
    U32.cast_U64_val_eq _
  have c4 : (UScalar.cast .U64 (insns.val[i.val]!).len).val = (insns.val[i.val]!).len.val :=
    U32.cast_U64_val_eq _
  rw [c1, c2] at een
  rw [c3, c4] at enx
  intro hns
  cases stops with
  | true =>
    exfalso
    apply hns
    cases hbv : cond.val[i.val]! with
    | true =>
      rw [hbv] at hst
      simp only [ite_true] at hst
      cases (ok_inj hst)
    | false =>
      rw [hbv] at hst
      simp only [Bool.false_eq_true, ite_false] at hst
      exact ⟨rfl, h1, h2, hh, hst⟩
  | false =>
    simp only [Bool.false_eq_true, ite_false, bind_eq_ok] at h
    obtain ⟨cae, hcae, h⟩ := h
    cases cae with
    | true =>
      right
      cases he : (insns.val[i.val]!).effect <;> simp only [he] at hcae <;>
        first
        | cases (ok_inj hcae)
        | (have hq := ok_inj hcae
           have hq' : next = en := by simpa using hq
           refine ⟨⟨_, rfl⟩, ?_⟩
           rw [hq'] at enx
           omega)
    | false =>
      left
      simp only [Bool.false_eq_true, ite_false] at h
      split at h
      · cases (ok_inj h)
      rename_i hmax
      simp only [bind_eq_ok] at h
      obtain ⟨i8, _, hint, _, h⟩ := h
      have hl := lands_check_ok hd h
      have hm : (UScalar.cast .U64 core.num.U32.MAX).val = 4294967295 := by
        rw [U32.cast_U64_val_eq]; rfl
      have hnm : next.val ≤ 4294967295 := by
        have : ¬ next.val > (UScalar.cast .U64 core.num.U32.MAX).val := by
          clear * - hmax; scalar_tac
        omega
      have cn : (UScalar.cast .U32 next).val = next.val := by
        have e : (UScalar.cast .U32 next).val = next.val % 4294967296 := UScalar.cast_val_eq .U32 next
        rw [e]; omega
      rw [cn, enx] at hl
      exact hl

/-- Where the table of instruction `i` ends: at the next instruction of
the list, or at the body's end `en`. -/
def limOf (insns : List thumb.Insn) (i en : Nat) : Nat :=
  if i + 1 < insns.length then (insns[i + 1]!).address.val else en

/-- The passages a branch or a table makes land. -/
def BranchOk (code : Slice U8) (base : U32) (insns : List thumb.Insn) (d : List I64)
    (i : Nat) (need : Int) : Prop :=
  match (insns[i]!).effect with
  | .Branch t => base.val ≤ t.val → t.val < base.val + code.val.length → Lands insns d t.val need
  | .CompareBranch t =>
    base.val ≤ t.val → t.val < base.val + code.val.length → Lands insns d t.val need
  | .SwitchTable =>
    TableOk code base insns d (insns[i]!) (limOf insns i (base.val + code.val.length)) (d[i]!).val
  | _ => True

theorem branch_check_ok {code : Slice U8} {base : U32} {insns : alloc.vec.Vec thumb.Insn}
    {d : alloc.vec.Vec I64} {i : Usize} {need : I64}
    (hd : d.val.length = insns.val.length) (hend : base.val + code.val.length ≤ 4294967295)
    (hi : i.val < insns.val.length)
    (h : flow_v.branch_check code base insns d i need = ok true) :
    BranchOk code base insns.val d.val i.val need.val := by
  unfold flow_v.branch_check at h
  rw [bind_eq_ok] at h
  obtain ⟨x, hx, h⟩ := h
  have ex := vidx hi hx
  subst ex
  dsimp only at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨en, hen, i5, hi5, lim, hlim, h⟩ := h
  have een := uadd hen
  have c1 : (UScalar.cast .U64 base).val = base.val := U32.cast_U64_val_eq _
  have c2 := len64 code
  rw [c1, c2] at een
  obtain ⟨_, e5⟩ := usub hi5
  have o1 : (1#usize).val = 1 := rfl
  have hvl : (alloc.vec.Vec.len insns).val = insns.val.length := by simp [alloc.vec.Vec.len]
  rw [o1, hvl] at e5
  have hlimv : lim.val = limOf insns.val i.val (base.val + code.val.length) := by
    unfold limOf
    split at hlim
    · rename_i hlt
      have hlt' : i.val + 1 < insns.val.length := by
        have : i.val < i5.val := by clear * - hlt; scalar_tac
        omega
      simp only [bind_eq_ok] at hlim
      obtain ⟨i6, hi6, y, hy, hlim⟩ := hlim
      have e6 := uadd hi6
      rw [o1] at e6
      have ey := vidx (by omega) hy
      rw [if_pos hlt', ← ok_inj hlim, ey, e6]
    · rename_i hlt
      have hlt' : ¬ i.val + 1 < insns.val.length := by
        have : ¬ i.val < i5.val := by clear * - hlt; scalar_tac
        omega
      rw [if_neg hlt', ← ok_inj hlim]
      have e : (UScalar.cast .U32 en).val = en.val % 4294967296 := UScalar.cast_val_eq .U32 en
      rw [e]; omega
  unfold BranchOk
  cases he : (insns.val[i.val]!).effect <;> simp only [he] at h ⊢
  case Branch t =>
    intro hb ht
    have hb' : base ≤ t := by scalar_tac
    rw [if_pos hb'] at h
    try simp only [bind_ok] at h
    have ct : (UScalar.cast .U64 t).val = t.val := U32.cast_U64_val_eq _
    have ht' : UScalar.cast .U64 t < en := by scalar_tac
    rw [if_pos ht'] at h
    exact lands_check_ok hd h
  case CompareBranch t =>
    intro hb ht
    have hb' : base ≤ t := by scalar_tac
    rw [if_pos hb'] at h
    try simp only [bind_ok] at h
    have ct : (UScalar.cast .U64 t).val = t.val := U32.cast_U64_val_eq _
    have ht' : UScalar.cast .U64 t < en := by scalar_tac
    rw [if_pos ht'] at h
    exact lands_check_ok hd h
  case SwitchTable =>
    have ht := table_check_ok hd hend hi h
    rw [hlimv] at ht
    exact ht

/-- Instruction `i`: placed, its height within the bounds and the frame,
and every passage from it that stays in the body lands on the list with
the height it needs. `flow_v::node_ok`. -/
def NodeOk (code : Slice U8) (base : U32) (insns : List thumb.Insn) (cond : List Bool)
    (noreturn : alloc.vec.Vec U32) (d : List I64) (frame : U32) (i : Nat) : Prop :=
  PlacedL code base (insns[i]!) ∧
  -1099511627776 ≤ (d[i]!).val ∧ (d[i]!).val ≤ 1099511627776 ∧
  (d[i]!).val + (downL (insns[i]!).effect : Int) ≤ frame.val ∧
  FallOk code base insns d noreturn (insns[i]!) (cond[i]!)
    ((d[i]!).val + deltaL (insns[i]!).effect (cond[i]!)) ∧
  BranchOk code base insns d i ((d[i]!).val + deltaL (insns[i]!).effect (cond[i]!))

theorem big_val : flow_v.BIG.val = 1099511627776 := by
  unfold flow_v.BIG; rfl

theorem node_check_ok {code : Slice U8} {base : U32} {insns : alloc.vec.Vec thumb.Insn}
    {cond : alloc.vec.Vec Bool} {noreturn : alloc.vec.Vec U32} {d : alloc.vec.Vec I64}
    {frame : U32} {i : Usize}
    (hd : d.val.length = insns.val.length) (hc : cond.val.length = insns.val.length)
    (hend : base.val + code.val.length ≤ 4294967295) (hi : i.val < insns.val.length)
    (h : flow_v.node_check code base insns cond noreturn d frame i = ok true) :
    NodeOk code base insns.val cond.val noreturn d.val frame i.val := by
  unfold flow_v.node_check at h
  simp only [bind_eq_ok] at h
  obtain ⟨x, hx, b, hb, h⟩ := h
  have ex := vidx hi hx
  subst ex
  cases b with
  | false =>
    simp only [Bool.false_eq_true, ite_false] at h
    cases (ok_inj h)
  | true =>
    have hp := placed_check_ok hb
    simp only [ite_true, bind_eq_ok] at h
    obtain ⟨i1, hi1, i2, hi2, h⟩ := h
    have e1 := vidx (by omega) hi1
    subst e1
    have e2 : i2.val = -1099511627776 := by
      rw [ineg (by rw [big_val]; omega) hi2, big_val]
    split at h
    · cases (ok_inj h)
    rename_i hlo
    split at h
    · cases (ok_inj h)
    rename_i hhi
    have hlo' : -1099511627776 ≤ (d.val[i.val]!).val := by
      have : ¬ (d.val[i.val]!).val < i2.val := by clear * - hlo; scalar_tac
      omega
    have hhi' : (d.val[i.val]!).val ≤ 1099511627776 := by
      have : ¬ (d.val[i.val]!).val > flow_v.BIG.val := by clear * - hhi; scalar_tac
      rw [big_val] at this
      omega
    simp only [bind_eq_ok, Prod.exists, Aeneas.Std.uncurry] at h
    obtain ⟨e, dn, hm, i3, hi3, h⟩ := h
    have hed : e = (insns.val[i.val]!).effect ∧ dn.val = (downL (insns.val[i.val]!).effect : Int) := by
      cases he : (insns.val[i.val]!).effect <;> simp only [he, lift, bind_ok] at hm <;>
        (have hm := Prod.mk.inj (ok_inj hm)
         obtain ⟨m1, m2⟩ := hm
         subst m1 m2
         first
           | exact ⟨rfl, rfl⟩
           | exact ⟨he.symm, rfl⟩
           | exact ⟨rfl, hcast64 _⟩
           | exact ⟨he.symm, hcast64 _⟩)
    obtain ⟨hee, hdn⟩ := hed
    subst hee
    have e3 := iadd hi3
    obtain ⟨i4, hi4, h⟩ := h
    have hi4 := ok_inj hi4
    subst hi4
    split at h
    · cases (ok_inj h)
    rename_i hfr
    have hfr' : (d.val[i.val]!).val + (downL (insns.val[i.val]!).effect : Int) ≤ frame.val := by
      have hq : ¬ i3.val > (UScalar.hcast .I64 frame).val := by clear * - hfr; scalar_tac
      rw [hcast64] at hq
      omega
    simp only [bind_eq_ok] at h
    obtain ⟨b1, hb1, i5, hi5, need, hneed, b2, hb2, h⟩ := h
    have eb1 := vidx (by omega) hb1
    subst eb1
    have e5 := delta_of_ok hi5
    have en := iadd hneed
    cases b2 with
    | false =>
      simp only [Bool.false_eq_true, ite_false] at h
      cases (ok_inj h)
    | true =>
      simp only [ite_true] at h
      have hf := fall_check_ok hd hc hi hb2
      have hbr := branch_check_ok hd hend hi h
      rw [en, e5] at hf hbr
      exact ⟨hp, hlo', hhi', hfr', hf, hbr⟩

theorem nodes_loop_ok (code : Slice U8) (base : U32) (insns : alloc.vec.Vec thumb.Insn)
    (cond : alloc.vec.Vec Bool) (noreturn : alloc.vec.Vec U32) (d : alloc.vec.Vec I64)
    (frame : U32) (n : Usize)
    (hd : d.val.length = insns.val.length) (hc : cond.val.length = insns.val.length)
    (hend : base.val + code.val.length ≤ 4294967295) (hn : n.val = insns.val.length) :
    ∀ (m : Nat) (i : Usize), n.val - i.val = m →
      flow_v.nodes_check_loop code base insns cond noreturn d frame n i = ok true →
      ∀ k, i.val ≤ k → k < insns.val.length →
        NodeOk code base insns.val cond.val noreturn d.val frame k := by
  intro m
  induction m with
  | zero =>
    intro i hm h k hk hkl
    omega
  | succ m ih =>
    intro i hm h k hk hkl
    unfold flow_v.nodes_check_loop loop at h
    simp only [flow_v.nodes_check_loop.body] at h
    have hlt : i < n := by scalar_tac
    simp only [hlt, ite_true, bind_eq_ok] at h
    obtain ⟨cf, ⟨b, hb, hcf⟩, hloop⟩ := h
    cases b with
    | false =>
      simp only [Bool.false_eq_true, ite_false] at hcf
      have hcf := ok_inj hcf
      subst hcf
      simp at hloop
    | true =>
      simp only [ite_true, bind_eq_ok] at hcf
      obtain ⟨i1, hi1, hcf⟩ := hcf
      have hcf := ok_inj hcf
      subst hcf
      have e1 := uadd hi1
      have o1 : (1#usize).val = 1 := rfl
      rw [o1] at e1
      have hrec : flow_v.nodes_check_loop code base insns cond noreturn d frame n i1 = ok true := by
        unfold flow_v.nodes_check_loop
        simp only [flow_v.nodes_check_loop.body]
        exact hloop
      by_cases hki : k = i.val
      · subst hki
        exact node_check_ok hd hc hend (by omega) hb
      · exact ih i1 (by omega) hrec k (by omega) hkl

theorem nodes_check_ok {code : Slice U8} {base : U32} {insns : alloc.vec.Vec thumb.Insn}
    {cond : alloc.vec.Vec Bool} {noreturn : alloc.vec.Vec U32} {d : alloc.vec.Vec I64}
    {frame : U32}
    (hd : d.val.length = insns.val.length) (hc : cond.val.length = insns.val.length)
    (hend : base.val + code.val.length ≤ 4294967295)
    (h : flow_v.nodes_check code base insns cond noreturn d frame = ok true) :
    ∀ k, k < insns.val.length → NodeOk code base insns.val cond.val noreturn d.val frame k := by
  unfold flow_v.nodes_check at h
  dsimp only at h
  intro k hk
  exact nodes_loop_ok code base insns cond noreturn d frame _ hd hc hend
    (by simp [alloc.vec.Vec.len]) _ 0#usize rfl h k (by simp) hk

/-! ## The `it` marks -/

/-- Every instruction of the list at the address `q` is marked. -/
def Marked (insns : List thumb.Insn) (cond : List Bool) (q : Nat) : Prop :=
  ∀ j, j < insns.length → (insns[j]!).address.val = q → cond[j]! = true

theorem marked_loop_ok (insns : alloc.vec.Vec thumb.Insn) (cond : alloc.vec.Vec Bool) (q : U64)
    (hc : cond.val.length = insns.val.length) :
    ∀ (m : Nat) (j : Usize), insns.val.length - j.val = m →
      flow_v.marked_loop insns cond q j = ok true →
      ∀ k, j.val ≤ k → k < insns.val.length → (insns.val[k]!).address.val = q.val →
        cond.val[k]! = true := by
  intro m
  induction m with
  | zero =>
    intro j hm h k hk hkl
    omega
  | succ m ih =>
    intro j hm h k hk hkl hq
    unfold flow_v.marked_loop loop at h
    simp only [flow_v.marked_loop.body] at h
    have hlt : j < alloc.vec.Vec.len insns := by scalar_tac
    simp only [hlt, ite_true, bind_eq_ok, lift, bind_ok] at h
    obtain ⟨cf, ⟨x, hx, hb⟩, hloop⟩ := h
    have ex := vidx (by omega) hx
    subst ex
    have ca : (UScalar.cast .U64 (insns.val[j.val]!).address).val = (insns.val[j.val]!).address.val :=
      U32.cast_U64_val_eq _
    have o1 : (1#usize).val = 1 := rfl
    have fin : ∀ j1 : Usize, j + 1#usize = ok j1 → cf = ControlFlow.cont j1 →
        (k = j.val → cond.val[k]! = true) → cond.val[k]! = true := by
      intro j1 hj1 hcf hkj
      subst hcf
      have e1 := uadd hj1
      rw [o1] at e1
      have hrec : flow_v.marked_loop insns cond q j1 = ok true := by
        unfold flow_v.marked_loop
        simp only [flow_v.marked_loop.body, lift, bind_ok]
        exact hloop
      by_cases hki : k = j.val
      · exact hkj hki
      · exact ih j1 (by omega) hrec k (by omega) hkl hq
    split at hb
    · rename_i heq
      simp only [bind_eq_ok] at hb
      obtain ⟨b, hbi, hb⟩ := hb
      have eb := vidx (by omega) hbi
      cases b with
      | false =>
        simp only [Bool.false_eq_true, ite_false] at hb
        have hb := ok_inj hb
        subst hb
        simp at hloop
      | true =>
        simp only [ite_true, bind_eq_ok] at hb
        obtain ⟨j1, hj1, hb⟩ := hb
        exact fin j1 hj1 (ok_inj hb).symm (fun hki => by rw [hki, ← eb])
    · rename_i hne
      simp only [bind_eq_ok] at hb
      obtain ⟨j1, hj1, hb⟩ := hb
      refine fin j1 hj1 (ok_inj hb).symm (fun hki => ?_)
      exfalso
      apply hne
      subst hki
      scalar_tac

theorem marked_ok {insns : alloc.vec.Vec thumb.Insn} {cond : alloc.vec.Vec Bool} {q : U64}
    (hc : cond.val.length = insns.val.length) (h : flow_v.marked insns cond q = ok true) :
    Marked insns.val cond.val q.val := by
  intro j hj hq
  exact marked_loop_ok insns cond q hc _ 0#usize rfl h j (by simp) hj hq

/-- `l` is the length of the instruction at the address `q`, as the
compiled check reads it: from the halfword there, or 2 when `q` has no
halfword in the body. `flow_v::len_at`. -/
def LenAt (code : Slice U8) (base : U32) (q l : Nat) : Prop :=
  if base.val ≤ q ∧ q - base.val + 1 < code.val.length then
    ∃ (off : Usize) (h lw : U32), off.val + base.val = q ∧ flow_v.hw code off = ok h ∧
      flow_v.len_of h = ok lw ∧ lw.val = l
  else l = 2

/-- `q'` is the address `k` instructions after the address `q`. -/
inductive Chain (code : Slice U8) (base : U32) : Nat → Nat → Nat → Prop
  | zero (q : Nat) : Chain code base q 0 q
  | step (q l k q' : Nat) : LenAt code base q l → Chain code base (q + l) k q' →
      Chain code base q (k + 1) q'

theorem usize_ext {a b : Usize} (h : a.val = b.val) : a = b := by scalar_tac

theorem lenat_unique {code : Slice U8} {base : U32} {q l l' : Nat}
    (h : LenAt code base q l) (h' : LenAt code base q l') : l = l' := by
  unfold LenAt at h h'
  by_cases hc : base.val ≤ q ∧ q - base.val + 1 < code.val.length
  · rw [if_pos hc] at h h'
    obtain ⟨off, hw, lw, ho, hh, hl, e⟩ := h
    obtain ⟨off', hw', lw', ho', hh', hl', e'⟩ := h'
    have : off = off' := usize_ext (by omega)
    subst this
    rw [hh] at hh'
    have := ok_inj hh'
    subst this
    rw [hl] at hl'
    have := ok_inj hl'
    subst this
    omega
  · rw [if_neg hc] at h h'
    omega

theorem it_loop_ok (code : Slice U8) (base : U32) (insns : alloc.vec.Vec thumb.Insn)
    (cond : alloc.vec.Vec Bool) (count : Usize)
    (hc : cond.val.length = insns.val.length) :
    ∀ (n : Nat) (q : U64) (m : Usize), count.val - m.val = n →
      flow_v.it_check_loop code base insns cond count q m = ok true →
      ∀ k q', m.val + k < count.val → Chain code base q.val k q' →
        Marked insns.val cond.val q' := by
  intro n
  induction n with
  | zero =>
    intro q m hn h k q' hk
    omega
  | succ n ih =>
    intro q m hn h k q' hk hch
    unfold flow_v.it_check_loop loop at h
    simp only [flow_v.it_check_loop.body] at h
    have hlt : m < count := by scalar_tac
    simp only [hlt, ite_true, bind_eq_ok] at h
    obtain ⟨cf, ⟨b, hb, hcf⟩, hloop⟩ := h
    cases b with
    | false =>
      simp only [Bool.false_eq_true, ite_false] at hcf
      have hcf := ok_inj hcf
      subst hcf
      simp at hloop
    | true =>
      have hmk := marked_ok hc hb
      simp only [ite_true, bind_eq_ok, lift, bind_ok] at hcf
      obtain ⟨st, hst, q1, hq1, m1, hm1, hcf⟩ := hcf
      have hcf := ok_inj hcf
      subst hcf
      have eq1 := uadd hq1
      have em1 := uadd hm1
      have o1 : (1#usize).val = 1 := rfl
      rw [o1] at em1
      have hrec : flow_v.it_check_loop code base insns cond count q1 m1 = ok true := by
        unfold flow_v.it_check_loop
        simp only [flow_v.it_check_loop.body]
        exact hloop
      cases hch with
      | zero => exact hmk
      | step _ l k' _ hl hrest =>
        have cb : (UScalar.cast .U64 base).val = base.val := U32.cast_U64_val_eq _
        have hstep : LenAt code base q.val st.val := by
          unfold LenAt
          split at hst
          · rename_i hge
            have hge' : base.val ≤ q.val := by
              have : (UScalar.cast .U64 base).val ≤ q.val := by clear * - hge; scalar_tac
              omega
            simp only [bind_eq_ok] at hst
            obtain ⟨i2, hi2, i3, hi3, hst⟩ := hst
            obtain ⟨_, e2⟩ := usub hi2
            have e3 := uadd hi3
            have o1' : (1#u64).val = 1 := rfl
            rw [cb] at e2
            rw [o1'] at e3
            have c2 := len64 code
            split at hst
            · rename_i hin
              have hin' : q.val - base.val + 1 < code.val.length := by
                have : i3.val < (UScalar.cast .U64 (Slice.len code)).val := by
                  clear * - hin; scalar_tac
                omega
              rw [if_pos ⟨hge', hin'⟩]
              simp only [bind_eq_ok] at hst
              obtain ⟨i7, hi7, i9, hi9, i10, hi10, hst⟩ := hst
              obtain ⟨_, e7⟩ := usub hi7
              rw [cb] at e7
              have hst := ok_inj hst
              have c10 : (UScalar.cast .U64 i10).val = i10.val := U32.cast_U64_val_eq _
              have hus : (UScalar.cast .Usize i7).val = i7.val := by
                have e : (UScalar.cast .Usize i7).val = i7.val % 2 ^ System.Platform.numBits :=
                  UScalar.cast_val_eq .Usize i7
                have hq := (Slice.len code).hBounds
                simp only [UScalarTy.numBits] at hq
                have hsl : (Slice.len code).val = code.val.length := by simp
                rw [e, Nat.mod_eq_of_lt (by omega)]
              exact ⟨_, i9, i10, by rw [hus]; omega, hi9, hi10, by rw [← hst, c10]⟩
            · rename_i hin
              have hin' : ¬ q.val - base.val + 1 < code.val.length := by
                have : ¬ i3.val < (UScalar.cast .U64 (Slice.len code)).val := by
                  clear * - hin; scalar_tac
                omega
              rw [if_neg (fun hx => hin' hx.2), ← ok_inj hst]; rfl
          · rename_i hge
            have hge' : ¬ base.val ≤ q.val := by
              have : ¬ (UScalar.cast .U64 base).val ≤ q.val := by clear * - hge; scalar_tac
              omega
            rw [if_neg (fun hx => hge' hx.1), ← ok_inj hst]; rfl
        have el := lenat_unique hl hstep
        subst el
        rw [← eq1] at hrest
        exact ih q1 m1 (by omega) hrec k' q' (by omega) hrest

/-- The number of instructions the `it` with first halfword `h1` governs;
0 when `x` is no `it`. `flow_v::is_it`, `flow_v::it_count`. -/
def itCount (x : thumb.Insn) (h1 : U32) : Nat :=
  if x.len = 2#u32 ∧ (h1 &&& 65280#u32) = 48896#u32 ∧ (h1 &&& 15#u32 != 0#u32) = true then
    (if (h1 &&& 1#u32 != 0#u32) = true then 4
     else if (h1 &&& 2#u32 != 0#u32) = true then 3
     else if (h1 &&& 4#u32 != 0#u32) = true then 2 else 1)
  else 0

/-- Every instruction of the list that the `it` at index `p` governs is
marked. `flow_v::cond_ok`, for one `it`. -/
def ItOk (code : Slice U8) (base : U32) (insns : List thumb.Insn) (cond : List Bool)
    (p : Nat) : Prop :=
  ∀ h1 h2, flow_v.halves code base (insns[p]!) = ok (h1, h2) →
    ∀ m q, m < itCount (insns[p]!) h1 →
      Chain code base ((insns[p]!).address.val + 2) m q → Marked insns cond q

theorem it_check_ok {code : Slice U8} {base : U32} {insns : alloc.vec.Vec thumb.Insn}
    {cond : alloc.vec.Vec Bool} {p : Usize}
    (hc : cond.val.length = insns.val.length) (hp : p.val < insns.val.length)
    (h : flow_v.it_check code base insns cond p = ok true) :
    ItOk code base insns.val cond.val p.val := by
  unfold flow_v.it_check at h
  rw [bind_eq_ok] at h
  obtain ⟨x, hx, h⟩ := h
  have ex := vidx hp hx
  subst ex
  simp only [bind_eq_ok, Prod.exists, Aeneas.Std.uncurry] at h
  obtain ⟨h1, h2, hh, h⟩ := h
  intro g1 g2 hg m q hm hch
  rw [hh] at hg
  have hg := Prod.mk.inj (ok_inj hg)
  obtain ⟨rfl, rfl⟩ := hg
  unfold itCount at hm
  split at hm
  · rename_i hit
    obtain ⟨a1, a2, a3⟩ := hit
    simp only [lift, bind_ok] at h
    rw [if_pos a1, if_pos a2, if_pos a3] at h
    simp only [bind_eq_ok] at h
    obtain ⟨count, hcount, q0, hq0, h⟩ := h
    have eq0 := uadd hq0
    have ca : (UScalar.cast .U64 (insns.val[p.val]!).address).val = (insns.val[p.val]!).address.val :=
      U32.cast_U64_val_eq _
    have o2 : (2#u64).val = 2 := rfl
    rw [ca, o2] at eq0
    have hcv : count.val = (if (h1 &&& 1#u32 != 0#u32) = true then 4
        else if (h1 &&& 2#u32 != 0#u32) = true then 3
        else if (h1 &&& 4#u32 != 0#u32) = true then 2 else 1) := by
      split at hcount
      · rename_i c1
        rw [if_pos c1, ← ok_inj hcount]; rfl
      · rename_i c1
        rw [if_neg c1]
        split at hcount
        · rename_i c2
          rw [if_pos c2, ← ok_inj hcount]; rfl
        · rename_i c2
          rw [if_neg c2]
          split at hcount
          · rename_i c3
            rw [if_pos c3, ← ok_inj hcount]; rfl
          · rename_i c3
            rw [if_neg c3, ← ok_inj hcount]; rfl
    rw [← eq0] at hch
    exact it_loop_ok code base insns cond count hc _ q0 0#usize rfl h m q
      (by rw [hcv]; simpa using hm) hch
  · omega

theorem marks_loop_ok (code : Slice U8) (base : U32) (insns : alloc.vec.Vec thumb.Insn)
    (cond : alloc.vec.Vec Bool) (n : Usize)
    (hc : cond.val.length = insns.val.length) (hn : n.val = insns.val.length) :
    ∀ (m : Nat) (p : Usize), n.val - p.val = m →
      flow_v.marks_check_loop code base insns cond n p = ok true →
      ∀ k, p.val ≤ k → k < insns.val.length → ItOk code base insns.val cond.val k := by
  intro m
  induction m with
  | zero =>
    intro p hm h k hk hkl
    omega
  | succ m ih =>
    intro p hm h k hk hkl
    unfold flow_v.marks_check_loop loop at h
    simp only [flow_v.marks_check_loop.body] at h
    have hlt : p < n := by scalar_tac
    simp only [hlt, ite_true, bind_eq_ok] at h
    obtain ⟨cf, ⟨b, hb, hcf⟩, hloop⟩ := h
    cases b with
    | false =>
      simp only [Bool.false_eq_true, ite_false] at hcf
      have hcf := ok_inj hcf
      subst hcf
      simp at hloop
    | true =>
      simp only [ite_true, bind_eq_ok] at hcf
      obtain ⟨p1, hp1, hcf⟩ := hcf
      have hcf := ok_inj hcf
      subst hcf
      have e1 := uadd hp1
      have o1 : (1#usize).val = 1 := rfl
      rw [o1] at e1
      have hrec : flow_v.marks_check_loop code base insns cond n p1 = ok true := by
        unfold flow_v.marks_check_loop
        simp only [flow_v.marks_check_loop.body]
        exact hloop
      by_cases hki : k = p.val
      · subst hki
        exact it_check_ok hc (by omega) hb
      · exact ih p1 (by omega) hrec k (by omega) hkl

/-! ## The check -/

/-- What the compiled `flow_ok` accepts. `flow_v::flow_spec`. -/
structure FlowL (code : Slice U8) (base : U32) (insns : List thumb.Insn) (cond : List Bool)
    (noreturn : alloc.vec.Vec U32) (d : List I64) (frame : U32) : Prop where
  nonempty : 1 ≤ insns.length
  lcond : cond.length = insns.length
  ld : d.length = insns.length
  fits : base.val + code.val.length ≤ 4294967295
  first : (insns[0]!).address = base
  zero : (d[0]!).val = 0
  ordered : Ordered insns
  nodes : ∀ k, k < insns.length → NodeOk code base insns cond noreturn d frame k
  marks : ∀ p, p < insns.length → ItOk code base insns cond p

/-- **The compiled `flow_ok` answers true only for a list, marks and
heights that satisfy `FlowL`.** -/
theorem flow_ok_sound {code : Slice U8} {base : U32} {insns : alloc.vec.Vec thumb.Insn}
    {cond : alloc.vec.Vec Bool} {noreturn : alloc.vec.Vec U32} {d : alloc.vec.Vec I64}
    {frame : U32}
    (h : flow_v.flow_ok code base insns cond noreturn d frame = ok true) :
    FlowL code base insns.val cond.val noreturn d.val frame := by
  unfold flow_v.flow_ok at h
  dsimp only at h
  have hvl : (alloc.vec.Vec.len insns).val = insns.val.length := by simp [alloc.vec.Vec.len]
  have hcl : (alloc.vec.Vec.len cond).val = cond.val.length := by simp [alloc.vec.Vec.len]
  have hdl : (alloc.vec.Vec.len d).val = d.val.length := by simp [alloc.vec.Vec.len]
  split at h
  · cases (ok_inj h)
  rename_i hn1
  have hn : 1 ≤ insns.val.length := by
    have : ¬ (alloc.vec.Vec.len insns).val < 1 := by clear * - hn1; scalar_tac
    omega
  split at h
  · cases (ok_inj h)
  rename_i hc0
  have hc : cond.val.length = insns.val.length := by
    have : alloc.vec.Vec.len cond = alloc.vec.Vec.len insns :=
      Decidable.byContradiction fun hne => hc0 (bne_iff_ne.mpr hne)
    rw [← hcl, ← hvl, this]
  split at h
  · cases (ok_inj h)
  rename_i hd0
  have hd : d.val.length = insns.val.length := by
    have : alloc.vec.Vec.len d = alloc.vec.Vec.len insns :=
      Decidable.byContradiction fun hne => hd0 (bne_iff_ne.mpr hne)
    rw [← hdl, ← hvl, this]
  simp only [lift, bind_ok] at h
  split at h
  · cases (ok_inj h)
  rename_i hbig
  simp only [bind_eq_ok] at h
  obtain ⟨i8, hi8, h⟩ := h
  have e8 := uadd hi8
  have c1 : (UScalar.cast .U64 base).val = base.val := U32.cast_U64_val_eq _
  have c2 := len64 code
  have hm : (UScalar.cast .U64 core.num.U32.MAX).val = 4294967295 := by
    rw [U32.cast_U64_val_eq]; rfl
  rw [c1, c2] at e8
  split at h
  · cases (ok_inj h)
  rename_i hfit
  have hend : base.val + code.val.length ≤ 4294967295 := by
    have : ¬ i8.val > (UScalar.cast .U64 core.num.U32.MAX).val := by clear * - hfit; scalar_tac
    omega
  simp only [bind_eq_ok] at h
  obtain ⟨x0, hx0, h⟩ := h
  have ex0 := vidx (i := 0#usize) (by have : (0#usize).val = 0 := rfl; omega) hx0
  split at h
  · cases (ok_inj h)
  rename_i hfirst
  have hfirst' : x0.address = base :=
    Decidable.byContradiction fun hne => hfirst (bne_iff_ne.mpr hne)
  simp only [bind_eq_ok] at h
  obtain ⟨d0, hd0i, h⟩ := h
  have ed0 := vidx (i := 0#usize) (by have : (0#usize).val = 0 := rfl; omega) hd0i
  split at h
  · cases (ok_inj h)
  rename_i hzero
  have hzero' : d0 = 0#i64 := Decidable.byContradiction fun hne => hzero (bne_iff_ne.mpr hne)
  simp only [bind_eq_ok] at h
  obtain ⟨b, hb, h⟩ := h
  cases b with
  | false =>
    simp only [Bool.false_eq_true, ite_false] at h
    cases (ok_inj h)
  | true =>
    simp only [ite_true, bind_eq_ok] at h
    obtain ⟨b1, hb1, h⟩ := h
    cases b1 with
    | false =>
      simp only [Bool.false_eq_true, ite_false] at h
      cases (ok_inj h)
    | true =>
      simp only [ite_true] at h
      unfold flow_v.marks_check at h
      dsimp only at h
      refine ⟨hn, hc, hd, hend, ?_, ?_, order_check_ok hb, nodes_check_ok hd hc hend hb1, ?_⟩
      · have : (0#usize).val = 0 := rfl
        rw [this] at ex0
        rw [← ex0]; exact hfirst'
      · have : (0#usize).val = 0 := rfl
        rw [this] at ed0
        rw [← ed0, hzero']; rfl
      · intro p hp
        exact marks_loop_ok code base insns cond _ hc hvl _ 0#usize rfl h p (by simp) hp

end FlowP






end rsk_link
