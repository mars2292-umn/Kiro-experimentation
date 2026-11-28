import Tools.LinkFlow
import Tools.LinkFlowProofs
open Aeneas Aeneas.Std Result

/-! # The compiled flow check gives the certificate (2026-11-27)

`Tools/LinkFlowProofs.lean` proves that the compiled `flow_v::flow_ok`
answers true only for a list, marks and heights that satisfy `FlowL`.
`Tools/LinkFlow.lean` proves that heights with three conditions (`Cert`)
bound the stack pointer's descent along any path of passages. This file
joins them, in one prover:

- `Pass i j`: control can pass from instruction `i` of the list to
  instruction `j` inside the body: `j` is at the next address and `i` does
  not stop; or `j` is at the target of `i`'s branch or `cbz`; or `i` is a
  `tbb`/`tbh [pc, Rm]` and `j` is at the target of an entry of its table
  that is not the padding.
- `pass_height`: under `FlowL`, along a passage the height of the
  instruction reached is at least the height of the one left plus its
  weight.
- `certOf`, `flow_ok_cert`: so the compiled check's `true` gives a `Cert`
  whose passages are `Pass`.
- `flow_ok_within_frame`: **when the compiled `flow_ok` answers true, SP
  along any path of such passages from the body's first instruction is
  never more than the frame below where it started.**

What `Pass` is stated with: the compiled readers of the body's bytes
(`flow_v::halves`, `flow_v::hw`, `flow_v::hard_of`), not a second
description of them. What a step does to SP is `SpStepC`
(`Tools/LinkFlow.lean`). Handwritten; no `sorry`. -/

namespace rsk_link

open LinkThumb FlowP

/-- Control can pass from instruction `i` of the list to instruction `j`
inside the body. -/
def Pass (code : Slice U8) (base : U32) (insns : List thumb.Insn) (cond : List Bool)
    (noreturn : alloc.vec.Vec U32) (i j : Nat) : Prop :=
  i < insns.length ∧ j < insns.length ∧
  ((¬ Stops code base noreturn (insns[i]!) (cond[i]!) ∧
      (insns[j]!).address.val = (insns[i]!).address.val + (insns[i]!).len.val) ∨
   (∃ t, ((insns[i]!).effect = .Branch t ∨ (insns[i]!).effect = .CompareBranch t) ∧
      (insns[j]!).address.val = t.val) ∨
   ((insns[i]!).effect = .SwitchTable ∧ ∃ (h1 h2 : U32) (pos e : Nat),
      flow_v.halves code base (insns[i]!) = ok (h1, h2) ∧
      (insns[i]!).address.val + 4 ≤ pos ∧
      pos < limOf insns i (base.val + code.val.length) ∧
      (pos - (insns[i]!).address.val - 4) % (if (h2 &&& 16#u32 != 0#u32) = true then 2 else 1) = 0 ∧
      EntryAt code base (h2 &&& 16#u32 != 0#u32) pos e ∧
      ¬ ((h2 &&& 16#u32 != 0#u32) = false ∧
          pos + 1 = limOf insns i (base.val + code.val.length) ∧
          (insns[i]!).address.val + 4 + 2 * e < limOf insns i (base.val + code.val.length)) ∧
      (insns[j]!).address.val = (insns[i]!).address.val + 4 + 2 * e))

theorem ordered_lt {insns : List thumb.Insn} (ho : Ordered insns) :
    ∀ j i, i < j → j < insns.length → (insns[i]!).address.val < (insns[j]!).address.val := by
  intro j
  induction j with
  | zero => intro i hi; omega
  | succ j ih =>
    intro i hi hj
    have step := ho j hj
    by_cases hij : i = j
    · subst hij; exact step
    · have := ih i (by omega) (by omega)
      omega

theorem ordered_inj {insns : List thumb.Insn} (ho : Ordered insns) {i j : Nat}
    (hi : i < insns.length) (hj : j < insns.length)
    (h : (insns[i]!).address.val = (insns[j]!).address.val) : i = j := by
  rcases Nat.lt_trichotomy i j with hlt | heq | hgt
  · have := ordered_lt ho j i hlt hj; omega
  · exact heq
  · have := ordered_lt ho i j hgt hi; omega

theorem entry_unique {code : Slice U8} {base : U32} {wide : Bool} {pos e e' : Nat}
    (h : EntryAt code base wide pos e) (h' : EntryAt code base wide pos e') : e = e' := by
  obtain ⟨off, ho, h⟩ := h
  obtain ⟨off', ho', h'⟩ := h'
  have : off = off' := usize_ext (by omega)
  subst this
  cases wide
  · simp only [Bool.false_eq_true, ite_false] at h h'
    obtain ⟨b, hb, e1⟩ := h
    obtain ⟨b', hb', e2⟩ := h'
    rw [hb] at hb'
    have := FlowP.ok_inj hb'
    subst this
    omega
  · simp only [ite_true] at h h'
    obtain ⟨b, hb, e1⟩ := h
    obtain ⟨b', hb', e2⟩ := h'
    rw [hb] at hb'
    have := FlowP.ok_inj hb'
    subst this
    omega

/-- **Along a passage the height grows by at least the weight.** -/
theorem pass_height {code : Slice U8} {base : U32} {insns : List thumb.Insn} {cond : List Bool}
    {noreturn : alloc.vec.Vec U32} {d : List I64} {frame : U32}
    (F : FlowL code base insns cond noreturn d frame) {i j : Nat}
    (hp : Pass code base insns cond noreturn i j) :
    (d[i]!).val + deltaL (insns[i]!).effect (cond[i]!) ≤ (d[j]!).val := by
  obtain ⟨hi, hj, hp⟩ := hp
  obtain ⟨_, _, _, _, hfall, hbr⟩ := F.nodes i hi
  obtain ⟨⟨pj1, pj2, pj3, _⟩, _⟩ := F.nodes j hj
  have land : ∀ need : Int, Lands insns d (insns[j]!).address.val need → need ≤ (d[j]!).val := by
    intro need ⟨j', hj', ha, hn⟩
    have := ordered_inj F.ordered hj' hj ha
    subst this
    exact hn
  rcases hp with ⟨hns, hnext⟩ | ⟨t, het, hat⟩ | ⟨het, h1, h2, pos, e, hh, hp1, hp2, hp3, hE, hnp, hat⟩
  · rcases hfall hns with hl | ⟨_, hend⟩
    · rw [← hnext] at hl
      exact land _ hl
    · omega
  · unfold BranchOk at hbr
    rcases het with he | he
    · rw [he] at hbr ⊢
      simp only at hbr
      have := hbr (by omega) (by omega)
      rw [← hat] at this
      exact land _ this
    · rw [he] at hbr ⊢
      simp only at hbr
      have := hbr (by omega) (by omega)
      rw [← hat] at this
      exact land _ this
  · unfold BranchOk at hbr
    rw [het] at hbr ⊢
    simp only at hbr
    obtain ⟨g1, g2, hg, _, _, _, _, hall⟩ := hbr
    rw [hh] at hg
    have hg := Prod.mk.inj (FlowP.ok_inj hg)
    obtain ⟨rfl, rfl⟩ := hg
    obtain ⟨e', hE', hs⟩ := hall pos hp1 hp2 hp3
    have := entry_unique hE' hE
    subst this
    rcases hs with hpad | ⟨_, hl⟩
    · exact absurd hpad hnp
    · rw [← hat] at hl
      have := land _ hl
      simp only [deltaL]
      omega

theorem wt_absE (e : thumb.Effect) (c : Bool) : wt (absE e) c = deltaL e c := by
  cases e <;> simp [wt, absE, deltaL]

theorem dn_absE (e : thumb.Effect) : dn (absE e) = downL e := by
  cases e <;> simp [dn, absE, downL]

/-- The certificate of a list, marks and heights that satisfy `FlowL`:
its passages are `Pass`. -/
def certOf {code : Slice U8} {base : U32} {insns : List thumb.Insn} {cond : List Bool}
    {noreturn : alloc.vec.Vec U32} {d : List I64} {frame : U32}
    (F : FlowL code base insns cond noreturn d frame) : Cert where
  eff := fun i => if i < insns.length then absE (insns[i]!).effect else .plain
  cond := fun i => cond[i]!
  edge := Pass code base insns cond noreturn
  d := fun i => if i < insns.length then (d[i]!).val else 0
  frame := frame.val
  first := by
    have := F.nonempty
    rw [if_pos (by omega)]
    exact F.zero
  passes := by
    intro i j hp
    have hi := hp.1
    have hj := hp.2.1
    rw [if_pos hi, if_pos hi, if_pos hj, wt_absE]
    exact pass_height F hp
  within := by
    intro i
    by_cases hi : i < insns.length
    · rw [if_pos hi, if_pos hi, dn_absE]
      exact (F.nodes i hi).2.2.2.1
    · rw [if_neg hi, if_neg hi]
      simp [dn]

/-- **When the compiled `flow_ok` answers true, SP along any path of
passages from the body's first instruction is never more than the frame
below where it started**, however often the path passes an instruction. -/
theorem flow_ok_within_frame {code : Slice U8} {base : U32} {insns : alloc.vec.Vec thumb.Insn}
    {cond : alloc.vec.Vec Bool} {noreturn : alloc.vec.Vec U32} {d : alloc.vec.Vec I64}
    {frame : U32}
    (h : flow_v.flow_ok code base insns cond noreturn d frame = ok true)
    {top k : Nat} {s u : Arm.St}
    (hpath : Path (certOf (flow_ok_sound h)) top 0 s k u) :
    (s.sp.toNat : Int) ≤ u.sp.toNat + frame.val :=
  path_within_frame hpath

end rsk_link
