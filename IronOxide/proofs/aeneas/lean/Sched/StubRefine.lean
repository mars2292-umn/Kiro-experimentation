import Sched.StubModel
import Sched.StubCode

/-! # The machine's instructions against the stub model's (2026-11-26)

`Sched/StubModel.lean` writes each assembly stub as a sequence of
functions on a model state (`i_push_r3_r11_lr`, `i_tst_lr`, ...), whose
meaning was assumed. This file proves it of the machine of
`Sched/Arm.lean`: for each instruction form the stubs use, the machine's
instruction, executed from a state that is a model state (`R`), reaches
the state the model's function gives (`sim_*`). `Sched/StubPaths.lean`
chains these along each path of each stub's machine code.

The relation `R` compares the model's unbounded integers with the
machine's 32-bit registers, and the model's memory of words by address
with the machine's bytes at the addresses that are multiples of 4
(`wv`, `rdw_wrw_same`, `rdw_wrw_other`, `wv_wrws`). The lemmas' premises
are what keeps the two the same: no wrap of the stack pointer, SP_main a
multiple of 4, a value written to PSP a multiple of 4, a value written to
CONTROL with no bit but nPRIV and FPCA.

The proof first found a difference: a floating-point instruction sets
CONTROL.FPCA, which the model's `vpush` and `vpop` did not. The model was
corrected (`fpca` in `Sched/StubModel.lean` and in the Verus model), and
`sim_vpush_eq` and `sim_vpop_eq` are exact. -/

namespace Stub
open Arm

/-! ## Words of memory -/

/-- The word of index `k`: the four bytes at the address `4 * k`. -/
def wv (mem : W → BitVec 8) (k : Nat) : W := rdw mem (BitVec.ofNat 32 (4 * k))

theorem toNat_add_lit (a : W) (j : Nat) (h : a.toNat + j < 4294967296) :
    (a + BitVec.ofNat 32 j).toNat = a.toNat + j := by
  rw [BitVec.toNat_add, BitVec.toNat_ofNat]
  have : j % 2 ^ 32 = j := Nat.mod_eq_of_lt (by omega)
  rw [this]
  exact Nat.mod_eq_of_lt h

theorem toNat_sub_lit (a : W) (j : Nat) (h : j ≤ a.toNat) :
    (a - BitVec.ofNat 32 j).toNat = a.toNat - j := by
  have ha := a.isLt
  rw [BitVec.toNat_sub, BitVec.toNat_ofNat]
  have : j % 2 ^ 32 = j := Nat.mod_eq_of_lt (by omega)
  rw [this]
  omega

theorem eq_ofNat_of_toNat {a : W} {n : Nat} (h : a.toNat = n) : a = BitVec.ofNat 32 n := by
  apply BitVec.eq_of_toNat_eq
  rw [BitVec.toNat_ofNat, h]
  exact (Nat.mod_eq_of_lt (h ▸ a.isLt)).symm

theorem byte_ofNat (n : Nat) : (BitVec.ofNat 8 n).toNat = n % 256 := by
  rw [BitVec.toNat_ofNat]

/-- A word read back from where it was stored. -/
theorem rdw_wrw_same (mem : W → BitVec 8) (a v : W) (ha : a.toNat + 4 ≤ 4294967296) :
    rdw (wrw mem a v) a = v := by
  have e1 : (a + 1).toNat = a.toNat + 1 := toNat_add_lit a 1 (by omega)
  have e2 : (a + 2).toNat = a.toNat + 2 := toNat_add_lit a 2 (by omega)
  have e3 : (a + 3).toNat = a.toNat + 3 := toNat_add_lit a 3 (by omega)
  have n1 : a + 1 ≠ a := fun h => by have := congrArg BitVec.toNat h; rw [e1] at this; omega
  have n2 : a + 2 ≠ a := fun h => by have := congrArg BitVec.toNat h; rw [e2] at this; omega
  have n3 : a + 3 ≠ a := fun h => by have := congrArg BitVec.toNat h; rw [e3] at this; omega
  have n21 : a + 2 ≠ a + 1 := fun h => by
    have := congrArg BitVec.toNat h; rw [e2, e1] at this; omega
  have n31 : a + 3 ≠ a + 1 := fun h => by
    have := congrArg BitVec.toNat h; rw [e3, e1] at this; omega
  have n32 : a + 3 ≠ a + 2 := fun h => by
    have := congrArg BitVec.toNat h; rw [e3, e2] at this; omega
  unfold rdw wrw
  simp only [if_true, n1, n2, n3, n21, n31, n32, if_false, byte_ofNat]
  apply BitVec.eq_of_toNat_eq
  rw [BitVec.toNat_ofNat]
  have := v.isLt
  omega

/-- A word at another aligned address is not changed by a store. -/
theorem rdw_wrw_other (mem : W → BitVec 8) (a b v : W) (ha : a.toNat % 4 = 0)
    (hb : b.toNat % 4 = 0) (hne : a ≠ b) : rdw (wrw mem a v) b = rdw mem b := by
  have la := a.isLt
  have lb := b.isLt
  have hn : a.toNat ≠ b.toNat := fun h => hne (BitVec.eq_of_toNat_eq h)
  have ea1 : (a + 1).toNat = a.toNat + 1 := toNat_add_lit a 1 (by omega)
  have ea2 : (a + 2).toNat = a.toNat + 2 := toNat_add_lit a 2 (by omega)
  have ea3 : (a + 3).toNat = a.toNat + 3 := toNat_add_lit a 3 (by omega)
  have eb1 : (b + 1).toNat = b.toNat + 1 := toNat_add_lit b 1 (by omega)
  have eb2 : (b + 2).toNat = b.toNat + 2 := toNat_add_lit b 2 (by omega)
  have eb3 : (b + 3).toNat = b.toNat + 3 := toNat_add_lit b 3 (by omega)
  have key : ∀ x : W, (x.toNat = b.toNat ∨ x.toNat = b.toNat + 1 ∨ x.toNat = b.toNat + 2 ∨
      x.toNat = b.toNat + 3) → wrw mem a v x = mem x := by
    intro x hx
    unfold wrw
    have c0 : x ≠ a := fun h => by rw [h] at hx; omega
    have c1 : x ≠ a + 1 := fun h => by rw [h, ea1] at hx; omega
    have c2 : x ≠ a + 2 := fun h => by rw [h, ea2] at hx; omega
    have c3 : x ≠ a + 3 := fun h => by rw [h, ea3] at hx; omega
    simp only [c0, c1, c2, c3, if_false]
  unfold rdw
  rw [key b (Or.inl rfl), key (b + 1) (Or.inr (Or.inl eb1)),
    key (b + 2) (Or.inr (Or.inr (Or.inl eb2))), key (b + 3) (Or.inr (Or.inr (Or.inr eb3)))]

theorem ofNat_4k_toNat (k : Nat) (hk : k < 1073741824) : (BitVec.ofNat 32 (4 * k)).toNat = 4 * k := by
  rw [BitVec.toNat_ofNat]
  exact Nat.mod_eq_of_lt (by omega)

/-- The words after a store at the aligned address `4 * q`. -/
theorem wv_wrw (mem : W → BitVec 8) (a v : W) (q : Nat) (ha : a.toNat = 4 * q) (k : Nat)
    (hk : k < 1073741824) : wv (wrw mem a v) k = if k = q then v else wv mem k := by
  have la := a.isLt
  unfold wv
  by_cases h : k = q
  · rw [if_pos h, h, ← eq_ofNat_of_toNat ha]
    exact rdw_wrw_same mem a v (by omega)
  · rw [if_neg h]
    have hb4 : (BitVec.ofNat 32 (4 * k)).toNat % 4 = 0 := by
      rw [ofNat_4k_toNat k hk]; exact Nat.mul_mod_right 4 k
    apply rdw_wrw_other mem a _ v (by rw [ha]; exact Nat.mul_mod_right 4 q) hb4
    intro e
    have := congrArg BitVec.toNat e
    rw [ofNat_4k_toNat k hk, ha] at this
    omega

/-- The words after a store of several words from the aligned address `4 * q`. -/
theorem wv_wrws (vs : List W) : ∀ (mem : W → BitVec 8) (a : W) (q : Nat), a.toNat = 4 * q →
    q + vs.length < 1073741824 → ∀ k, k < 1073741824 →
    wv (wrws mem a vs) k = if q ≤ k ∧ k < q + vs.length then vs[k - q]! else wv mem k := by
  induction vs with
  | nil => intro mem a q _ _ k _; simp [wrws]
  | cons v vs ih =>
    intro mem a q ha hq k hk
    simp only [wrws, List.length_cons] at hq ⊢
    have ha4 : (a + 4).toNat = 4 * (q + 1) := by
      have : (a + 4).toNat = a.toNat + 4 := toNat_add_lit a 4 (by omega)
      rw [this, ha]; omega
    rw [ih (wrw mem a v) (a + 4) (q + 1) ha4 (by omega) k hk, wv_wrw mem a v q ha k hk]
    by_cases h0 : k = q
    · subst h0
      simp
    · by_cases h1 : q + 1 ≤ k ∧ k < q + 1 + vs.length
      · rw [if_pos h1, if_pos ⟨by omega, by omega⟩]
        have : k - q = (k - (q + 1)) + 1 := by omega
        rw [this]
        simp
      · rw [if_neg h1, if_neg h0, if_neg (by omega)]

/-- The word read at an aligned address. -/
theorem rdw_wv (mem : W → BitVec 8) (a : W) (q : Nat) (ha : a.toNat = 4 * q) : rdw mem a = wv mem q := by
  unfold wv
  rw [← eq_ofNat_of_toNat ha]

/-! ## The machine and the model -/

/-- The machine state `a` is the model state `m`: every register the model
has holds the machine's value, the model's memory holds the machine's
words at the addresses that are multiples of 4, and the machine is as the
stubs are run: SP_main a multiple of 4, no HardFault or NMI active, no
lazy floating-point state preservation pending, FPCCR.ASPEN set, and
CONTROL with no bit but nPRIV and FPCA (Handler mode).

Two components are compared with an excess: the machine's CONTROL is the
model's plus `kc`, and the machine's R`i` is the model's plus `kr i`. The
excess was needed while the model's `vpush` and `vpop` left CONTROL where
the machine sets CONTROL.FPCA; since the model sets it too (2026-11-26)
every theorem of `Sched/StubPaths.lean` has the excess 0, and the
relation is equality. -/
structure R (kc : Int) (kr : Nat → Int) (a : Arm.St) (m : M) : Prop where
  r : ∀ i, i ≤ 12 → m.r i + kr i = ((a.r i).toNat : Int)
  sp : m.sp = ((a.sp.toNat : Nat) : Int)
  lr : m.lr = ((a.lr.toNat : Nat) : Int)
  psp : m.psp = ((a.psp.toNat : Nat) : Int)
  control : m.control + kc = ((a.control.toNat : Nat) : Int)
  ipsr : m.ipsr = ((a.ipsr.toNat : Nat) : Int)
  primask : m.primask = a.primask
  faultmask : m.faultmask = a.faultmask
  z : m.z = a.z
  fp : ∀ i, i < 16 → m.fp i = (((a.s (16 + i)).toNat : Nat) : Int)
  mem : ∀ k : Nat, k < 1073741824 → m.mem (4 * (k : Int)) = (((wv a.mem k).toNat : Nat) : Int)
  spal : a.sp.toNat % 4 = 0
  hf : a.hfnmi = false
  lsp : a.lspact = false
  asp : a.aspen = true
  cb : a.control.toNat < 8 ∧ a.control.toNat / 2 % 2 = 0

/-- Splits `R` into its fields and closes those `h` already gives. -/
macro "rkeep " h:ident : tactic =>
  `(tactic| (refine ⟨?r, ?sp, ?lr, ?psp, ?control, ?ipsr, ?primask, ?faultmask, ?z, ?fp, ?mem, ?spal,
      ?hf, ?lsp, ?asp, ?cb⟩ <;>
    first
      | exact ($h).r | exact ($h).sp | exact ($h).lr | exact ($h).psp | exact ($h).control
      | exact ($h).ipsr | exact ($h).primask | exact ($h).faultmask | exact ($h).z | exact ($h).fp
      | exact ($h).mem | exact ($h).spal | exact ($h).hf | exact ($h).lsp | exact ($h).asp
      | exact ($h).cb | skip))

/-- An instruction outside an IT block is executed, and leaves ITSTATE 0. -/
theorem execI_al {i : Instr} {len : W} {a : Arm.St} (hit : a.it = 0#8)
    (h1 : ∀ fc mk, i ≠ .it fc mk) (h2 : ∀ c o, i ≠ .bcond c o) :
    execI i len a = (eff i len a).map fun t => { t with it := 0#8 } := by
  cases i <;> simp_all [execI, curCond, condHolds, itAdvance]

variable {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}

/-- The excess of the registers after R`rd` is written with an exact value. -/
def kset (kr : Nat → Int) (rd : Nat) (k : Int) : Nat → Int := fun i => if i = rd then k else kr i

theorem R.setr (h : R kc kr a m) {rd : Nat} (hrd : rd ≤ 12) {v : W} {x k : Int}
    (hv : x + k = ((v.toNat : Nat) : Int)) :
    R kc (kset kr rd k) { a with r := Function.update a.r rd v } (setr m rd x) := by
  rkeep h
  case r =>
    intro i hi
    by_cases e : i = rd
    · subst e; simp [Stub.setr, kset, hv]
    · simp [Stub.setr, kset, e, h.r i hi]

theorem setR_low {a : Arm.St} {rd : Nat} (hrd : rd ≤ 12) (v : W) :
    Arm.setR a rd v = { a with r := Function.update a.r rd v } := by
  unfold Arm.setR
  rw [if_neg (by omega), if_neg (by omega)]

theorem getR_low {a : Arm.St} {rn : Nat} (hrn : rn ≤ 12) : Arm.getR a rn = a.r rn := by
  unfold Arm.getR
  rw [if_neg (by omega), if_neg (by omega), if_neg (by omega)]

theorem getR_lr {a : Arm.St} : Arm.getR a 14 = a.lr := rfl
theorem getR_sp {a : Arm.St} : Arm.getR a 13 = a.sp := rfl

/-! ## One instruction of the machine, one function of the model -/

theorem sim_cpsid_i (h : R kc kr a m) (hit : a.it = 0#8) :
    ∃ a', execI (.cps true true false) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧ a'.it = 0#8 ∧
      a'.sp = a.sp ∧ R kc kr a' (i_cpsid_i m) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, if_true, Option.map_some]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  rkeep h
  all_goals simp [i_cpsid_i, h.faultmask]

theorem sim_cpsie_i (h : R kc kr a m) (hit : a.it = 0#8) :
    ∃ a', execI (.cps false true false) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧ a'.it = 0#8 ∧
      a'.sp = a.sp ∧ R kc kr a' (i_cpsie_i m) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  rkeep h
  all_goals simp [i_cpsie_i, h.faultmask]

theorem sim_cpsid_f (h : R kc kr a m) (hit : a.it = 0#8) :
    ∃ a', execI (.cps true false true) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧ a'.it = 0#8 ∧
      a'.sp = a.sp ∧ R kc kr a' (i_cpsid_f m) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, if_true, Option.map_some]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  rkeep h
  all_goals simp [i_cpsid_f, h.primask, h.hf]

theorem sim_dsb (h : R kc kr a m) (hit : a.it = 0#8) :
    ∃ a', execI .dsb 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧ a'.it = 0#8 ∧
      a'.sp = a.sp ∧ R kc kr a' (i_dsb m) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  rkeep h

theorem sim_isb (h : R kc kr a m) (hit : a.it = 0#8) :
    ∃ a', execI .isb 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧ a'.it = 0#8 ∧
      a'.sp = a.sp ∧ R kc kr a' (i_isb m) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  rkeep h

theorem R.pcit (h : R kc kr a m) (p : W) (t : BitVec 8) : R kc kr { a with pc := p, it := t } m := by
  rkeep h

theorem R.pc (h : R kc kr a m) (p : W) : R kc kr { a with pc := p } m := by
  rkeep h

/-- `mov rd, lr`. -/
theorem sim_mov_r_lr (h : R kc kr a m) (hit : a.it = 0#8) {rd : Nat} (hrd : rd ≤ 12) :
    ∃ a', execI (.movr rd 14) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧ a'.it = 0#8 ∧
      a'.sp = a.sp ∧ R kc (kset kr rd 0) a' (i_mov_r_lr m rd) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, setR_low hrd, getR_lr]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  exact (h.setr hrd (by simp [h.lr])).pcit _ _

/-- `mov rd, rs`. -/
theorem sim_mov (h : R kc kr a m) (hit : a.it = 0#8) {rd rs : Nat} (hrd : rd ≤ 12) (hrs : rs ≤ 12) :
    ∃ a', execI (.movr rd rs) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧ a'.it = 0#8 ∧
      a'.sp = a.sp ∧ R kc (kset kr rd (kr rs)) a' (i_mov m rd rs) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, setR_low hrd, getR_low hrs]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  exact (h.setr hrd (h.r rs hrs)).pcit _ _

/-- `mov rd, sp`. -/
theorem sim_mov_r_sp (h : R kc kr a m) (hit : a.it = 0#8) {rd : Nat} (hrd : rd ≤ 12) :
    ∃ a', execI (.movr rd 13) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧ a'.it = 0#8 ∧
      a'.sp = a.sp ∧ R kc (kset kr rd 0) a' (i_mov_r_sp m rd) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, setR_low hrd, getR_sp]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  exact (h.setr hrd (by simp [h.sp])).pcit _ _

theorem and7_toNat (c : W) (h : c.toNat < 8) : (c &&& 7#32).toNat = c.toNat := by
  rw [BitVec.toNat_and]
  have : (7#32 : W).toNat = 2 ^ 3 - 1 := by decide
  rw [this, Nat.and_two_pow_sub_one_eq_mod]
  exact Nat.mod_eq_of_lt h

/-- `mrs rd, ipsr`. -/
theorem sim_mrs_ipsr (h : R kc kr a m) (hit : a.it = 0#8) {rd : Nat} (hrd : rd ≤ 12) :
    ∃ a', execI (.mrs rd 5) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧ a'.it = 0#8 ∧
      a'.sp = a.sp ∧ R kc (kset kr rd 0) a' (i_mrs_ipsr m rd) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, if_true, Option.map_some, setR_low hrd]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  exact (h.setr hrd (by simp [h.ipsr])).pcit _ _

/-- `mrs rd, psp`. -/
theorem sim_mrs_psp (h : R kc kr a m) (hit : a.it = 0#8) {rd : Nat} (hrd : rd ≤ 12) :
    ∃ a', execI (.mrs rd 9) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧ a'.it = 0#8 ∧
      a'.sp = a.sp ∧ R kc (kset kr rd 0) a' (i_mrs_psp m rd) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, setR_low hrd]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  exact (h.setr hrd (by simp [h.psp])).pcit _ _

/-- `mrs rd, control`: the register gets the machine's CONTROL, which is
the model's plus the excess `kc`. -/
theorem sim_mrs_control (h : R kc kr a m) (hit : a.it = 0#8) {rd : Nat} (hrd : rd ≤ 12) :
    ∃ a', execI (.mrs rd 20) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧ a'.it = 0#8 ∧
      a'.sp = a.sp ∧ R kc (kset kr rd kc) a' (i_mrs_control m rd) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, setR_low hrd]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  exact (h.setr hrd (by rw [and7_toNat _ h.cb.1]; exact h.control)).pcit _ _

/-- `mrs rd, primask`. -/
theorem sim_mrs_primask (h : R kc kr a m) (hit : a.it = 0#8) {rd : Nat} (hrd : rd ≤ 12) :
    ∃ a', execI (.mrs rd 16) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧ a'.it = 0#8 ∧
      a'.sp = a.sp ∧ R kc (kset kr rd 0) a' (i_mrs_primask m rd) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, setR_low hrd]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  refine (h.setr hrd ?_).pcit _ _
  rw [h.primask]
  cases a.primask <;> rfl

/-- `sub sp, sp, #n`. -/
theorem sim_sub_sp (h : R kc kr a m) (hit : a.it = 0#8) {n : Nat} (hn : n ≤ a.sp.toNat)
    (h4 : n % 4 = 0) :
    ∃ a', execI (.subsp (BitVec.ofNat 32 n)) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧
      a'.it = 0#8 ∧ a'.sp.toNat = a.sp.toNat - n ∧ R kc kr a' (i_sub_sp m n) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some]
  have e := toNat_sub_lit a.sp n hn
  refine ⟨_, rfl, rfl, rfl, e, ?_⟩
  rkeep h
  case sp => simp only [i_sub_sp]; rw [e, h.sp]; omega
  case spal => show (a.sp - BitVec.ofNat 32 n).toNat % 4 = 0; rw [e]; have := h.spal; omega

/-- `add sp, sp, #n`. -/
theorem sim_add_sp (h : R kc kr a m) (hit : a.it = 0#8) {n : Nat} (hn : a.sp.toNat + n < 4294967296)
    (h4 : n % 4 = 0) :
    ∃ a', execI (.addsp (BitVec.ofNat 32 n)) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧
      a'.it = 0#8 ∧ a'.sp.toNat = a.sp.toNat + n ∧ R kc kr a' (i_add_sp m n) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some]
  have e := toNat_add_lit a.sp n hn
  refine ⟨_, rfl, rfl, rfl, e, ?_⟩
  rkeep h
  case sp => simp only [i_add_sp]; rw [e, h.sp]; omega
  case spal => show (a.sp + BitVec.ofNat 32 n).toNat % 4 = 0; rw [e]; have := h.spal; omega

/-- `add rd, sp, #n`. -/
theorem sim_add_r_sp (h : R kc kr a m) (hit : a.it = 0#8) {rd n : Nat} (hrd : rd ≤ 12)
    (hn : a.sp.toNat + n < 4294967296) :
    ∃ a', execI (.addrsp rd (BitVec.ofNat 32 n)) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧
      a'.it = 0#8 ∧ a'.sp = a.sp ∧ R kc (kset kr rd 0) a' (i_add_r_sp m rd n) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, setR_low hrd]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  refine (h.setr hrd ?_).pcit _ _
  rw [toNat_add_lit a.sp n hn, h.sp]
  omega

/-- `str rs, [sp, #n]`, the register holding an exact value. -/
theorem sim_str_sp (h : R kc kr a m) (hit : a.it = 0#8) {rs n : Nat} (hrs : rs ≤ 12)
    (hk : kr rs = 0) (hn : a.sp.toNat + n + 4 ≤ 4294967296) (h4 : n % 4 = 0) :
    ∃ a', execI (.strsp rs (BitVec.ofNat 32 n)) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧
      a'.it = 0#8 ∧ a'.sp = a.sp ∧ R kc kr a' (i_str_sp m rs n) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, getR_low hrs]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  have e := toNat_add_lit a.sp n (by omega)
  have hal := h.spal
  obtain ⟨q, hq⟩ : ∃ q, (a.sp + BitVec.ofNat 32 n).toNat = 4 * q := ⟨(a.sp.toNat + n) / 4, by omega⟩
  rkeep h
  case mem =>
    intro k hk'
    show (wr m (m.sp + n) (m.r rs)).mem (4 * (k : Int)) = _
    rw [wv_wrw a.mem _ _ q hq k hk']
    simp only [wr]
    have hsp := h.sp
    by_cases c : k = q
    · have : (4 * (k : Int)) = m.sp + n := by rw [hsp]; omega
      rw [if_pos this, if_pos c]
      have := h.r rs hrs
      rw [hk] at this
      omega
    · have : ¬ (4 * (k : Int)) = m.sp + n := by rw [hsp]; omega
      rw [if_neg this, if_neg c]
      exact h.mem k hk'

/-- `ldr rd, [sp, #n]`. -/
theorem sim_ldr_sp (h : R kc kr a m) (hit : a.it = 0#8) {rd n : Nat} (hrd : rd ≤ 12)
    (hn : a.sp.toNat + n + 4 ≤ 4294967296) (h4 : n % 4 = 0) {len : W} :
    ∃ a', execI (.ldrsp rd (BitVec.ofNat 32 n)) len a = some a' ∧ a'.pc = a.pc + len ∧
      a'.it = 0#8 ∧ a'.sp = a.sp ∧ R kc (kset kr rd 0) a' (i_ldr_sp m rd n) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, setR_low hrd]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  have e := toNat_add_lit a.sp n (by omega)
  have hal := h.spal
  obtain ⟨q, hq⟩ : ∃ q, (a.sp + BitVec.ofNat 32 n).toNat = 4 * q := ⟨(a.sp.toNat + n) / 4, by omega⟩
  refine (h.setr hrd ?_).pcit _ _
  rw [rdw_wv a.mem _ q hq, ← h.mem q (by omega)]
  have hsp := h.sp
  have : m.sp + (n : Int) = 4 * (q : Int) := by rw [hsp]; omega
  rw [this]
  omega

theorem tst_bridge (x : W) (mask : Nat) (hm : mask < 4294967296) :
    decide (x &&& BitVec.ofNat 32 mask = 0#32) = !hasBit ((x.toNat : Nat) : Int) mask := by
  unfold hasBit
  have e1 : (((x.toNat : Nat) : Int)).toNat % 4294967296 = x.toNat := by
    rw [Int.toNat_natCast]; exact Nat.mod_eq_of_lt x.isLt
  rw [e1]
  have e2 : (x &&& BitVec.ofNat 32 mask = 0#32) ↔ (x.toNat &&& mask = 0) := by
    constructor
    · intro h
      have := congrArg BitVec.toNat h
      rw [BitVec.toNat_and, BitVec.toNat_ofNat, Nat.mod_eq_of_lt hm] at this
      exact this
    · intro h
      apply BitVec.eq_of_toNat_eq
      rw [BitVec.toNat_and, BitVec.toNat_ofNat, Nat.mod_eq_of_lt hm, h]
      rfl
  by_cases c : x.toNat &&& mask = 0
  · simp [c, e2.2 c]
  · have : ¬ (x &&& BitVec.ofNat 32 mask = 0#32) := fun hh => c (e2.1 hh)
    simp [c, this]

/-- `tst lr, #mask`. -/
theorem sim_tst_lr (h : R kc kr a m) (hit : a.it = 0#8) {mask : Nat} (hm : mask < 4294967296) :
    ∃ a', execI (.tst 14 (BitVec.ofNat 32 mask)) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧
      a'.it = 0#8 ∧ a'.sp = a.sp ∧ R kc kr a' (i_tst_lr m mask) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, getR_lr]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  rkeep h
  case z =>
    show (!hasBit m.lr mask) = decide (a.lr &&& BitVec.ofNat 32 mask = 0#32)
    rw [tst_bridge a.lr mask hm, h.lr]

/-- `tst rn, #mask`, the register holding an exact value. -/
theorem sim_tst_r (h : R kc kr a m) (hit : a.it = 0#8) {rn mask : Nat} (hrn : rn ≤ 12)
    (hk : kr rn = 0) (hm : mask < 4294967296) :
    ∃ a', execI (.tst rn (BitVec.ofNat 32 mask)) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧
      a'.it = 0#8 ∧ a'.sp = a.sp ∧ R kc kr a' (i_tst_r m rn mask) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, getR_low hrn]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  rkeep h
  case z =>
    show (!hasBit (m.r rn) mask) = decide (a.r rn &&& BitVec.ofNat 32 mask = 0#32)
    have := h.r rn hrn
    rw [hk] at this
    rw [tst_bridge (a.r rn) mask hm, ← this]
    simp

/-- `cmp rn, #0`, the register holding an exact value. -/
theorem sim_cmp_zero (h : R kc kr a m) (hit : a.it = 0#8) {rn : Nat} (hrn : rn ≤ 12)
    (hk : kr rn = 0) :
    ∃ a', execI (.cmpi rn 0#32) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧
      a'.it = 0#8 ∧ a'.sp = a.sp ∧ R kc kr a' (i_cmp_imm m rn 0) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, getR_low hrn, subFlags]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  rkeep h
  case z =>
    show decide (m.r rn = 0) = decide (a.r rn - 0#32 = 0#32)
    have := h.r rn hrn
    rw [hk] at this
    have e : a.r rn - 0#32 = a.r rn := by simp
    rw [e]
    by_cases c : a.r rn = 0#32
    · have : m.r rn = 0 := by rw [c] at this; simpa using this
      simp [c, this]
    · have : ¬ m.r rn = 0 := by
        intro hz
        apply c
        apply BitVec.eq_of_toNat_eq
        rw [hz] at this
        simp at this
        simpa using this.symm
      simp [c, this]

theorem and_not3 (x : W) (h : x.toNat % 4 = 0) : x &&& ~~~3#32 = x := by
  apply BitVec.eq_of_toNat_eq
  rw [BitVec.toNat_and]
  have e : (~~~3#32 : W).toNat = 4294967292 := by decide
  rw [e]
  have hx := x.isLt
  apply Nat.eq_of_testBit_eq
  intro i
  rw [Nat.testBit_and]
  by_cases hi : i < 2
  · have : x.toNat.testBit i = false := by
      interval_cases i
      · rw [Nat.testBit_zero]; simp; omega
      · rw [Nat.testBit_succ, Nat.testBit_zero]; simp; omega
    simp [this]
  · by_cases hi2 : i < 32
    · have : (4294967292 : Nat).testBit i = true := by
        interval_cases i <;> first | omega | decide
      simp [this]
    · have : x.toNat.testBit i = false :=
        Nat.testBit_lt_two_pow (lt_of_lt_of_le hx (Nat.pow_le_pow_right (by decide) (by omega)))
      simp [this]

/-- `msr psp, rs`, the register holding an exact, aligned value. -/
theorem sim_msr_psp (h : R kc kr a m) (hit : a.it = 0#8) {rs : Nat} (hrs : rs ≤ 12)
    (hk : kr rs = 0) (h4 : (a.r rs).toNat % 4 = 0) :
    ∃ a', execI (.msr 9 rs) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧
      a'.it = 0#8 ∧ a'.sp = a.sp ∧ R kc kr a' (i_msr_psp m rs) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, if_true, Option.map_some, getR_low hrs, and_not3 _ h4]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  rkeep h
  case psp =>
    show m.r rs = _
    have := h.r rs hrs
    rw [hk] at this
    simpa using this

theorem ctl_cases (c : W) (h : c.toNat < 8 ∧ c.toNat / 2 % 2 = 0) :
    c = 0#32 ∨ c = 1#32 ∨ c = 4#32 ∨ c = 5#32 := by
  have : c.toNat = 0 ∨ c.toNat = 1 ∨ c.toNat = 4 ∨ c.toNat = 5 := by omega
  rcases this with e | e | e | e
  · exact Or.inl (eq_ofNat_of_toNat e)
  · exact Or.inr (Or.inl (eq_ofNat_of_toNat e))
  · exact Or.inr (Or.inr (Or.inl (eq_ofNat_of_toNat e)))
  · exact Or.inr (Or.inr (Or.inr (eq_ofNat_of_toNat e)))

theorem ctl_write (c v : W) (hc : c.toNat < 8 ∧ c.toNat / 2 % 2 = 0)
    (hv : v.toNat < 8 ∧ v.toNat / 2 % 2 = 0) : (c &&& 2#32) ||| (v &&& 5#32) = v := by
  rcases ctl_cases c hc with rfl | rfl | rfl | rfl <;>
    rcases ctl_cases v hv with rfl | rfl | rfl | rfl <;> decide

/-- `msr control, rs`, the register holding a value with no bit but nPRIV
and FPCA: CONTROL gets the register's value, and its excess. -/
theorem sim_msr_control (h : R kc kr a m) (hit : a.it = 0#8) {rs : Nat} (hrs : rs ≤ 12)
    (hv : (a.r rs).toNat < 8 ∧ (a.r rs).toNat / 2 % 2 = 0) :
    ∃ a', execI (.msr 20 rs) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧
      a'.it = 0#8 ∧ a'.sp = a.sp ∧ R (kr rs) kr a' (i_msr_control m rs) := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, getR_low hrs, ctl_write _ _ h.cb hv]
  refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
  rkeep h
  case control => exact h.r rs hrs
  case cb => exact hv

/-- `it`: ITSTATE is set; nothing the model has changes. -/
theorem sim_it (h : R kc kr a m) (fc mk : Nat) :
    ∃ a', execI (.it fc mk) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧
      a'.it = BitVec.ofNat 8 (fc * 16 + mk) ∧ a'.sp = a.sp ∧ a'.z = a.z ∧ R kc kr a' m := by
  refine ⟨_, rfl, rfl, rfl, rfl, rfl, ?_⟩
  rkeep h

/-- An instruction in an IT block: executed when the block's current
condition holds; ITSTATE advances either way. -/
theorem execI_it {i : Instr} {len : W} {a : Arm.St} {t : BitVec 8} (hit : a.it = t)
    (hne : t.toNat % 16 ≠ 0) (h1 : ∀ fc mk, i ≠ .it fc mk) (h2 : ∀ c o, i ≠ .bcond c o) :
    execI i len a =
      if condHolds (t.toNat / 16) a then (eff i len a).map fun u => { u with it := itAdvance t }
      else some { a with pc := a.pc + len, it := itAdvance t } := by
  cases i <;> simp_all [execI, curCond]

/-! ## The control instructions -/

theorem exec_bl (hit : a.it = 0#8) (off : W) :
    execI (.bl off) 4#32 a =
      some { a with lr := (a.pc + 4#32) ||| 1#32, pc := a.pc + 4#32 + off, it := 0#8 } := by
  rw [execI_al hit (by simp) (by simp)]
  rfl

theorem exec_b (hit : a.it = 0#8) (off len : W) :
    execI (.b off) len a = some { a with pc := a.pc + 4#32 + off, it := 0#8 } := by
  rw [execI_al hit (by simp) (by simp)]
  rfl

theorem exec_cbz (hit : a.it = 0#8) {rn : Nat} (hrn : rn ≤ 12) (off : W) :
    execI (.cbz false rn off) 2#32 a =
      some { a with pc := if a.r rn = 0#32 then a.pc + 4#32 + off else a.pc + 2#32, it := 0#8 } := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, getR_low hrn]
  by_cases c : a.r rn = 0#32 <;> simp [c]

theorem exec_bne (hit : a.it = 0#8) (off : W) :
    execI (.bcond 1 off) 2#32 a =
      some { a with pc := if a.z = false then a.pc + 4#32 + off else a.pc + 2#32 } := by
  simp only [execI, hit]
  cases hz : a.z <;> simp [condHolds, hz]

theorem exec_bx_r (hit : a.it = 0#8) {rm : Nat} (hrm : rm ≤ 12) :
    execI (.bx rm) 2#32 a = some { a with pc := a.r rm, it := 0#8 } := by
  rw [execI_al hit (by simp) (by simp)]
  simp only [eff, Option.map_some, getR_low hrm]

theorem exec_bx_lr (hit : a.it = 0#8) :
    execI (.bx 14) 2#32 a = some { a with pc := a.lr, it := 0#8 } := by
  rw [execI_al hit (by simp) (by simp)]
  rfl

/-! ## The multiple stores and loads -/

theorem maskRegs_a : maskRegs 20472 = [3, 4, 5, 6, 7, 8, 9, 10, 11, 14] := by decide
theorem maskRegs_b : maskRegs 16400 = [4, 14] := by decide

/-- `push {r3-r11, lr}` on the machine. -/
theorem eff_push_a (hal : a.sp.toNat % 4 = 0) (hsp : 40 ≤ a.sp.toNat) (len : W) :
    eff (.push 20472) len a = some { a with
      mem := wrws a.mem (a.sp - BitVec.ofNat 32 40)
        [a.r 3, a.r 4, a.r 5, a.r 6, a.r 7, a.r 8, a.r 9, a.r 10, a.r 11, a.lr],
      sp := a.sp - BitVec.ofNat 32 40, pc := a.pc + len } := by
  have e := toNat_sub_lit a.sp 40 hsp
  have hc : ¬ ((a.sp - BitVec.ofNat 32 40).toNat % 4 ≠ 0) := by rw [e]; omega
  simp only [eff, maskRegs_a]
  exact if_neg hc

theorem get10 (x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 : W) (j : Nat) (hj : j < 10) :
    [x0, x1, x2, x3, x4, x5, x6, x7, x8, x9][j]! =
      if j = 0 then x0 else if j = 1 then x1 else if j = 2 then x2 else if j = 3 then x3
      else if j = 4 then x4 else if j = 5 then x5 else if j = 6 then x6 else if j = 7 then x7
      else if j = 8 then x8 else x9 := by
  interval_cases j <;> rfl

set_option maxHeartbeats 8000000 in
/-- `push {r3-r11, lr}`, the registers holding exact values. -/
theorem sim_push_a (h : R kc kr a m) (hit : a.it = 0#8) (hsp : 40 ≤ a.sp.toNat)
    (hk : ∀ i, 3 ≤ i → i ≤ 11 → kr i = 0) (len : W) :
    ∃ a', execI (.push 20472) len a = some a' ∧ a'.pc = a.pc + len ∧ a'.it = 0#8 ∧
      a'.sp.toNat = a.sp.toNat - 40 ∧ R kc kr a' (i_push_r3_r11_lr m) := by
  rw [execI_al hit (by simp) (by simp), eff_push_a h.spal hsp]
  have e := toNat_sub_lit a.sp 40 hsp
  have hal := h.spal
  have hlt := a.sp.isLt
  refine ⟨_, rfl, rfl, rfl, e, ?_⟩
  obtain ⟨q, hq⟩ : ∃ q, (a.sp - BitVec.ofNat 32 40).toNat = 4 * q := ⟨(a.sp.toNat - 40) / 4, by omega⟩
  rkeep h
  case sp => show m.sp - 40 = _; rw [e, h.sp]; omega
  case spal => show (a.sp - BitVec.ofNat 32 40).toNat % 4 = 0; rw [e]; omega
  case mem =>
    intro k hk'
    have hsp' := h.sp
    have r3 := h.r 3 (by omega); rw [hk 3 (by omega) (by omega)] at r3
    have r4 := h.r 4 (by omega); rw [hk 4 (by omega) (by omega)] at r4
    have r5 := h.r 5 (by omega); rw [hk 5 (by omega) (by omega)] at r5
    have r6 := h.r 6 (by omega); rw [hk 6 (by omega) (by omega)] at r6
    have r7 := h.r 7 (by omega); rw [hk 7 (by omega) (by omega)] at r7
    have r8 := h.r 8 (by omega); rw [hk 8 (by omega) (by omega)] at r8
    have r9 := h.r 9 (by omega); rw [hk 9 (by omega) (by omega)] at r9
    have r10 := h.r 10 (by omega); rw [hk 10 (by omega) (by omega)] at r10
    have r11 := h.r 11 (by omega); rw [hk 11 (by omega) (by omega)] at r11
    have rl := h.lr
    have hm := h.mem k hk'
    show (i_push_r3_r11_lr m).mem (4 * (k : Int)) = _
    rw [wv_wrws _ a.mem _ q hq (by simp; omega) k hk']
    simp only [List.length_cons, List.length_nil]
    by_cases c : q ≤ k ∧ k < q + 10
    · rw [if_pos (by omega)]
      obtain ⟨j, hj⟩ : ∃ j, k = q + j := ⟨k - q, by omega⟩
      have hj10 : j < 10 := by omega
      subst hj
      have ej : q + j - q = j := by omega
      rw [ej]
      rw [get10 _ _ _ _ _ _ _ _ _ _ j hj10]
      simp only [i_push_r3_r11_lr]
      split_ifs <;> omega
    · rw [if_neg (by omega)]
      simp only [i_push_r3_r11_lr]
      split_ifs <;> omega

/-- `push {r4, lr}` on the machine. -/
theorem eff_push_b (hal : a.sp.toNat % 4 = 0) (hsp : 8 ≤ a.sp.toNat) (len : W) :
    eff (.push 16400) len a = some { a with
      mem := wrws a.mem (a.sp - BitVec.ofNat 32 8) [a.r 4, a.lr],
      sp := a.sp - BitVec.ofNat 32 8, pc := a.pc + len } := by
  have e := toNat_sub_lit a.sp 8 hsp
  have hc : ¬ ((a.sp - BitVec.ofNat 32 8).toNat % 4 ≠ 0) := by rw [e]; omega
  simp only [eff, maskRegs_b]
  exact if_neg hc

/-- `push {r4, lr}`, R4 holding an exact value. -/
theorem sim_push_b (h : R kc kr a m) (hit : a.it = 0#8) (hsp : 8 ≤ a.sp.toNat)
    (hk : kr 4 = 0) (len : W) :
    ∃ a', execI (.push 16400) len a = some a' ∧ a'.pc = a.pc + len ∧ a'.it = 0#8 ∧
      a'.sp.toNat = a.sp.toNat - 8 ∧ R kc kr a' (i_push_r4_lr m) := by
  rw [execI_al hit (by simp) (by simp), eff_push_b h.spal hsp]
  have e := toNat_sub_lit a.sp 8 hsp
  have hal := h.spal
  have hlt := a.sp.isLt
  refine ⟨_, rfl, rfl, rfl, e, ?_⟩
  obtain ⟨q, hq⟩ : ∃ q, (a.sp - BitVec.ofNat 32 8).toNat = 4 * q := ⟨(a.sp.toNat - 8) / 4, by omega⟩
  rkeep h
  case sp => show m.sp - 8 = _; rw [e, h.sp]; omega
  case spal => show (a.sp - BitVec.ofNat 32 8).toNat % 4 = 0; rw [e]; omega
  case mem =>
    intro k hk'
    have hsp' := h.sp
    have r4 := h.r 4 (by omega); rw [hk] at r4
    have rl := h.lr
    have hm := h.mem k hk'
    show (i_push_r4_lr m).mem (4 * (k : Int)) = _
    rw [wv_wrws _ a.mem _ q hq (by simp; omega) k hk']
    simp only [List.length_cons, List.length_nil]
    by_cases c : q ≤ k ∧ k < q + 2
    · rw [if_pos (by omega)]
      obtain ⟨j, hj⟩ : ∃ j, k = q + j := ⟨k - q, by omega⟩
      have hj2 : j < 2 := by omega
      subst hj
      have ej : q + j - q = j := by omega
      rw [ej]
      simp only [i_push_r4_lr]
      interval_cases j
      · show _ = (((a.r 4).toNat : Nat) : Int); split_ifs <;> omega
      · show _ = ((a.lr.toNat : Nat) : Int); split_ifs <;> omega
    · rw [if_neg (by omega)]
      simp only [i_push_r4_lr]
      split_ifs <;> omega

theorem pop_a_bit (i : Nat) (hi : i ≤ 12) : (i ≤ 12 ∧ 20472 / 2 ^ i % 2 = 1) ↔ (3 ≤ i ∧ i ≤ 11) := by
  interval_cases i <;> decide

theorem pop_a_pos (i : Nat) (h3 : 3 ≤ i) (h11 : i ≤ 11) : regPos 20472 i = i - 3 := by
  interval_cases i <;> decide

theorem pop_a_lr : regPos 20472 14 = 9 := by decide

/-- The excess of the registers after `pop {r3-r11, lr}`. -/
def kpop (kr : Nat → Int) : Nat → Int := fun i => if 3 ≤ i ∧ i ≤ 11 then 0 else kr i

/-- `pop {r3-r11, lr}`. -/
theorem sim_pop_a (h : R kc kr a m) (hit : a.it = 0#8) (hsp : a.sp.toNat + 40 < 4294967296)
    (len : W) :
    ∃ a', execI (.pop 20472) len a = some a' ∧ a'.pc = a.pc + len ∧ a'.it = 0#8 ∧
      a'.sp.toNat = a.sp.toNat + 40 ∧ R kc (kpop kr) a' (i_pop_r3_r11_lr m) := by
  rw [execI_al hit (by simp) (by simp)]
  have hal := h.spal
  have e := toNat_add_lit a.sp 40 hsp
  have hc : ¬ (a.sp.toNat % 4 ≠ 0) := by omega
  simp only [eff, if_neg hc, Option.map_some, maskRegs_a]
  refine ⟨_, rfl, rfl, rfl, e, ?_⟩
  obtain ⟨q, hq⟩ : ∃ q, a.sp.toNat = 4 * q := ⟨a.sp.toNat / 4, by omega⟩
  have hsp' := h.sp
  have rd : ∀ j, j < 10 → rdw a.mem (a.sp + BitVec.ofNat 32 (4 * j)) = wv a.mem (q + j) := by
    intro j hj
    apply rdw_wv
    rw [toNat_add_lit a.sp (4 * j) (by omega)]
    omega
  rkeep h
  case r =>
    intro i hi
    show (i_pop_r3_r11_lr m).r i + kpop kr i = (((if i ≤ 12 ∧ 20472 / 2 ^ i % 2 = 1 then
      rdw a.mem (a.sp + BitVec.ofNat 32 (4 * regPos 20472 i)) else a.r i).toNat : Nat) : Int)
    simp only [i_pop_r3_r11_lr, kpop]
    by_cases c : 3 ≤ i ∧ i ≤ 11
    · rw [if_pos ((pop_a_bit i hi).2 c), if_pos c, if_pos c, pop_a_pos i c.1 c.2, rd (i - 3) (by omega),
        ← h.mem (q + (i - 3)) (by omega)]
      have : m.sp + 4 * ((i : Int) - 3) = 4 * ((q + (i - 3) : Nat) : Int) := by rw [hsp']; omega
      rw [this]
      omega
    · rw [if_neg (fun hh => c ((pop_a_bit i hi).1 hh)), if_neg c, if_neg c]
      exact h.r i hi
  case sp =>
    show m.sp + 40 = (((a.sp + BitVec.ofNat 32 40).toNat : Nat) : Int)
    rw [e, hsp']; omega
  case lr =>
    show m.mem (m.sp + 36) = (((if 20472 / 2 ^ 14 % 2 = 1 then
      rdw a.mem (a.sp + BitVec.ofNat 32 (4 * regPos 20472 14)) else a.lr).toNat : Nat) : Int)
    rw [if_pos (by decide), pop_a_lr, rd 9 (by omega), ← h.mem (q + 9) (by omega)]
    have : m.sp + 36 = 4 * ((q + 9 : Nat) : Int) := by rw [hsp']; omega
    rw [this]
  case spal => show (a.sp + BitVec.ofNat 32 40).toNat % 4 = 0; rw [e]; omega

theorem pop_b_bit (i : Nat) (hi : i ≤ 12) : (i ≤ 12 ∧ 16400 / 2 ^ i % 2 = 1) ↔ i = 4 := by
  interval_cases i <;> decide

/-- `pop {r4, lr}`. -/
theorem sim_pop_b (h : R kc kr a m) (hit : a.it = 0#8) (hsp : a.sp.toNat + 8 < 4294967296)
    (len : W) :
    ∃ a', execI (.pop 16400) len a = some a' ∧ a'.pc = a.pc + len ∧ a'.it = 0#8 ∧
      a'.sp.toNat = a.sp.toNat + 8 ∧ R kc (kset kr 4 0) a' (i_pop_r4_lr m) := by
  rw [execI_al hit (by simp) (by simp)]
  have hal := h.spal
  have e := toNat_add_lit a.sp 8 hsp
  have hc : ¬ (a.sp.toNat % 4 ≠ 0) := by omega
  simp only [eff, if_neg hc, Option.map_some, maskRegs_b]
  refine ⟨_, rfl, rfl, rfl, e, ?_⟩
  obtain ⟨q, hq⟩ : ∃ q, a.sp.toNat = 4 * q := ⟨a.sp.toNat / 4, by omega⟩
  have hsp' := h.sp
  have rd : ∀ j, j < 2 → rdw a.mem (a.sp + BitVec.ofNat 32 (4 * j)) = wv a.mem (q + j) := by
    intro j hj
    apply rdw_wv
    rw [toNat_add_lit a.sp (4 * j) (by omega)]
    omega
  rkeep h
  case r =>
    intro i hi
    show (i_pop_r4_lr m).r i + kset kr 4 0 i = (((if i ≤ 12 ∧ 16400 / 2 ^ i % 2 = 1 then
      rdw a.mem (a.sp + BitVec.ofNat 32 (4 * regPos 16400 i)) else a.r i).toNat : Nat) : Int)
    simp only [i_pop_r4_lr, kset]
    by_cases c : i = 4
    · subst c
      rw [if_pos (by decide : (4 : Nat) ≤ 12 ∧ 16400 / 2 ^ 4 % 2 = 1), if_pos (rfl : (4 : Nat) = 4),
        show regPos 16400 4 = 0 by decide, rd 0 (by omega), ← h.mem (q + 0) (by omega)]
      have : m.sp = 4 * ((q + 0 : Nat) : Int) := by rw [hsp']; omega
      rw [← this]
      omega
    · rw [if_neg (fun hh => c ((pop_b_bit i hi).1 hh)), if_neg c, if_neg c]
      exact h.r i hi
  case sp =>
    show m.sp + 8 = (((a.sp + BitVec.ofNat 32 8).toNat : Nat) : Int)
    rw [e, hsp']; omega
  case lr =>
    show m.mem (m.sp + 4) = (((if 16400 / 2 ^ 14 % 2 = 1 then
      rdw a.mem (a.sp + BitVec.ofNat 32 (4 * regPos 16400 14)) else a.lr).toNat : Nat) : Int)
    rw [if_pos (by decide), show regPos 16400 14 = 1 by decide, rd 1 (by omega), ← h.mem (q + 1) (by omega)]
    have : m.sp + 4 = 4 * ((q + 1 : Nat) : Int) := by rw [hsp']; omega
    rw [this]
  case spal => show (a.sp + BitVec.ofNat 32 8).toNat % 4 = 0; rw [e]; omega

/-! ## The instructions of the IT blocks -/

theorem or4_toNat (c : W) (hc : c.toNat < 8 ∧ c.toNat / 2 % 2 = 0) :
    ((c ||| 4#32).toNat : Int) = c.toNat + (if c.toNat < 4 then 4 else 0) ∧
    ((c ||| 4#32).toNat < 8 ∧ (c ||| 4#32).toNat / 2 % 2 = 0) := by
  rcases ctl_cases c hc with rfl | rfl | rfl | rfl <;> decide

theorem range_map_get (n : Nat) (f : Nat → W) (j : Nat) (hj : j < n) :
    ((List.range n).map f)[j]! = f j := by
  simp [hj]

/-- A skipped instruction of an IT block. -/
theorem R.skip (h : R kc kr a m) (p : W) (t : BitVec 8) : R kc kr { a with pc := p, it := t } m :=
  h.pcit p t

/-- `vpusheq {s16-s31}` as the last instruction of an `it eq` block. When
it is executed the machine sets CONTROL.FPCA, and so does the model's
`vpush` (`fpca`, since 2026-11-26). -/
theorem sim_vpush_eq (h : R 0 kr a m) (hit : a.it = 8#8) (hsp : 64 ≤ a.sp.toNat) :
    ∃ a', execI (.vpush 16 16) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧ a'.it = 0#8 ∧
      a'.sp.toNat = (if a.z then a.sp.toNat - 64 else a.sp.toNat) ∧
      R 0 kr a' (i_vpush_eq m) := by
  rw [execI_it hit (by decide) (by simp) (by simp)]
  have hz := h.z
  have ht : itAdvance 8#8 = 0#8 := by decide
  have hcd : (8#8 : BitVec 8).toNat / 16 = 0 := by decide
  rw [hcd, ht]
  simp only [condHolds]
  cases hzz : a.z
  · rw [hzz] at hz
    simp only [Bool.false_eq_true, if_false, i_vpush_eq, hz]
    exact ⟨_, rfl, rfl, rfl, rfl, by rkeep h; exact hz⟩
  · rw [hzz] at hz
    have e := toNat_sub_lit a.sp 64 hsp
    have hal := h.spal
    have hlt := a.sp.isLt
    have hc : ¬ ((a.sp - BitVec.ofNat 32 64).toNat % 4 ≠ 0) := by rw [e]; omega
    have hfp : fpCheck a = some { a with control := a.control ||| 4#32 } := by
      simp [fpCheck, h.lsp, h.asp]
    simp only [if_true, eff, hfp, Option.bind_some]
    rw [if_neg hc]
    simp only [Option.map_some, i_vpush_eq, hz, if_true]
    refine ⟨_, rfl, rfl, rfl, e, ?_⟩
    obtain ⟨q, hq⟩ : ∃ q, (a.sp - BitVec.ofNat 32 64).toNat = 4 * q :=
      ⟨(a.sp.toNat - 64) / 4, by omega⟩
    have o4 := or4_toNat a.control h.cb
    rkeep h
    case sp => show m.sp - 64 = _; rw [e, h.sp]; omega
    case control =>
      show fpca m.control + 0 = (((a.control ||| 4#32).toNat : Nat) : Int)
      have hc := h.control
      have hb := h.cb
      rw [o4.1]
      unfold fpca
      split_ifs <;> omega
    case spal => show (a.sp - BitVec.ofNat 32 64).toNat % 4 = 0; rw [e]; omega
    case cb => exact o4.2
    case mem =>
      intro k hk'
      have hsp' := h.sp
      have hm := h.mem k hk'
      show (vpush m).mem (4 * (k : Int)) = _
      rw [wv_wrws _ a.mem _ q hq (by simp; omega) k hk']
      simp only [List.length_map, List.length_range, vpush]
      by_cases c : q ≤ k ∧ k < q + 16
      · rw [if_pos c, range_map_get 16 _ (k - q) (by omega)]
        have c2 : m.sp - 64 ≤ 4 * (k : Int) ∧ 4 * (k : Int) < m.sp ∧
            (4 * (k : Int) - (m.sp - 64)) % 4 = 0 := by rw [hsp']; omega
        rw [if_pos c2]
        have ei : ((4 * (k : Int) - (m.sp - 64)) / 4).toNat = k - q := by rw [hsp']; omega
        rw [ei]
        exact h.fp (k - q) (by omega)
      · rw [if_neg c]
        have c2 : ¬ (m.sp - 64 ≤ 4 * (k : Int) ∧ 4 * (k : Int) < m.sp ∧
            (4 * (k : Int) - (m.sp - 64)) % 4 = 0) := by rw [hsp']; omega
        rw [if_neg c2]
        exact hm

/-- `vpopeq {s16-s31}` as the last instruction of an `it eq` block. -/
theorem sim_vpop_eq (h : R 0 kr a m) (hit : a.it = 8#8) (hsp : a.sp.toNat + 64 < 4294967296) :
    ∃ a', execI (.vpop 16 16) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧ a'.it = 0#8 ∧
      a'.sp.toNat = (if a.z then a.sp.toNat + 64 else a.sp.toNat) ∧
      R 0 kr a' (i_vpop_eq m) := by
  rw [execI_it hit (by decide) (by simp) (by simp)]
  have hz := h.z
  have ht : itAdvance 8#8 = 0#8 := by decide
  have hcd : (8#8 : BitVec 8).toNat / 16 = 0 := by decide
  rw [hcd, ht]
  simp only [condHolds]
  cases hzz : a.z
  · rw [hzz] at hz
    simp only [Bool.false_eq_true, if_false, i_vpop_eq, hz]
    exact ⟨_, rfl, rfl, rfl, rfl, by rkeep h; exact hz⟩
  · rw [hzz] at hz
    have e := toNat_add_lit a.sp 64 hsp
    have hal := h.spal
    have hc : ¬ (a.sp.toNat % 4 ≠ 0) := by omega
    have hfp : fpCheck a = some { a with control := a.control ||| 4#32 } := by
      simp [fpCheck, h.lsp, h.asp]
    simp only [if_true, eff, hfp, Option.bind_some]
    rw [if_neg hc]
    simp only [Option.map_some, i_vpop_eq, hz, if_true]
    refine ⟨_, rfl, rfl, rfl, e, ?_⟩
    obtain ⟨q, hq⟩ : ∃ q, a.sp.toNat = 4 * q := ⟨a.sp.toNat / 4, by omega⟩
    have o4 := or4_toNat a.control h.cb
    have hsp' := h.sp
    rkeep h
    case sp =>
      show m.sp + 64 = (((a.sp + BitVec.ofNat 32 64).toNat : Nat) : Int)
      rw [e, hsp']; omega
    case control =>
      show fpca m.control + 0 = (((a.control ||| 4#32).toNat : Nat) : Int)
      have hc := h.control
      have hb := h.cb
      rw [o4.1]
      unfold fpca
      split_ifs <;> omega
    case spal => show (a.sp + BitVec.ofNat 32 64).toNat % 4 = 0; rw [e]; omega
    case cb => exact o4.2
    case fp =>
      intro i hi
      show (vpop m).fp i = (((if 16 ≤ 16 + i ∧ 16 + i < 16 + 16 then
        rdw a.mem (a.sp + BitVec.ofNat 32 (4 * (16 + i - 16))) else a.s (16 + i)).toNat : Nat) : Int)
      rw [if_pos (by omega), show 16 + i - 16 = i by omega]
      simp only [vpop, if_pos hi]
      rw [rdw_wv a.mem _ (q + i) (by rw [toNat_add_lit a.sp (4 * i) (by omega)]; omega),
        ← h.mem (q + i) (by omega)]
      have : m.sp + 4 * (i : Int) = 4 * ((q + i : Nat) : Int) := by rw [hsp']; omega
      rw [this]

/-- `mrseq rd, msp` as the first instruction of an `ite eq` block. -/
theorem sim_mrs_msp_eq (h : R kc kr a m) (hit : a.it = 12#8) {rd : Nat} (hrd : rd ≤ 12) :
    ∃ a', execI (.mrs rd 8) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧ a'.it = 24#8 ∧
      a'.sp = a.sp ∧ a'.z = a.z ∧
      R kc (if a.z then kset kr rd 0 else kr) a' (i_mrs_msp_eq m rd) := by
  rw [execI_it hit (by decide) (by simp) (by simp)]
  have hz := h.z
  have ht : itAdvance 12#8 = 24#8 := by decide
  have hcd : (12#8 : BitVec 8).toNat / 16 = 0 := by decide
  rw [hcd, ht]
  simp only [condHolds]
  cases hzz : a.z
  · rw [hzz] at hz
    simp only [Bool.false_eq_true, if_false, i_mrs_msp_eq, hz]
    exact ⟨_, rfl, rfl, rfl, rfl, rfl, by rkeep h; exact hz⟩
  · rw [hzz] at hz
    simp only [if_true, eff, Option.map_some, setR_low hrd, i_mrs_msp_eq, hz]
    refine ⟨_, rfl, rfl, rfl, rfl, hzz, ?_⟩
    exact (h.setr hrd (by simp [h.sp])).pcit _ _

/-- `mrsne rd, psp` in an IT block whose current condition is NE. -/
theorem sim_mrs_psp_ne (h : R kc kr a m) {t t' : BitVec 8} (hit : a.it = t)
    (hne : t.toNat % 16 ≠ 0) (hcd : t.toNat / 16 = 1) (ht : itAdvance t = t') {rd : Nat}
    (hrd : rd ≤ 12) :
    ∃ a', execI (.mrs rd 9) 4#32 a = some a' ∧ a'.pc = a.pc + 4#32 ∧ a'.it = t' ∧
      a'.sp = a.sp ∧ a'.z = a.z ∧
      R kc (if a.z then kr else kset kr rd 0) a' (i_mrs_psp_ne m rd) := by
  rw [execI_it hit hne (by simp) (by simp)]
  have hz := h.z
  rw [hcd, ht]
  simp only [condHolds]
  cases hzz : a.z
  · rw [hzz] at hz
    simp only [Bool.not_false, if_true, eff, Option.map_some, setR_low hrd, i_mrs_psp_ne, hz,
      Bool.false_eq_true, if_false]
    refine ⟨_, rfl, rfl, rfl, rfl, hzz, ?_⟩
    exact (h.setr hrd (by simp [h.psp])).pcit _ _
  · rw [hzz] at hz
    simp only [Bool.not_true, Bool.false_eq_true, if_false, i_mrs_psp_ne, hz, if_true]
    exact ⟨_, rfl, rfl, rfl, rfl, rfl, by rkeep h; exact hz⟩

/-- `moveq rd, #n` as the last instruction of an `it eq` block: in an IT
block the 16-bit `movs` sets no flag. -/
theorem sim_mov_imm_eq (h : R kc kr a m) (hit : a.it = 8#8) {rd n : Nat} (hrd : rd ≤ 12)
    (hn : n < 4294967296) :
    ∃ a', execI (.movi rd (BitVec.ofNat 32 n)) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧
      a'.it = 0#8 ∧ a'.sp = a.sp ∧
      R kc (if a.z then kset kr rd 0 else kr) a' (i_mov_imm_eq m rd n) := by
  rw [execI_it hit (by decide) (by simp) (by simp)]
  have hz := h.z
  have ht : itAdvance 8#8 = 0#8 := by decide
  have hcd : (8#8 : BitVec 8).toNat / 16 = 0 := by decide
  have hin : ¬ (a.it.toNat % 16 = 0) := by rw [hit]; decide
  rw [hcd, ht]
  simp only [condHolds]
  cases hzz : a.z
  · rw [hzz] at hz
    simp only [Bool.false_eq_true, if_false, i_mov_imm_eq, hz]
    exact ⟨_, rfl, rfl, rfl, rfl, by rkeep h; exact hz⟩
  · rw [hzz] at hz
    simp only [if_true, eff, if_neg hin, Option.map_some, setR_low hrd, i_mov_imm_eq, hz]
    refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
    refine (h.setr hrd ?_).pcit _ _
    rw [BitVec.toNat_ofNat, Nat.mod_eq_of_lt hn]
    omega

/-- `addeq rd, sp, #n` as the last instruction of an IT block. -/
theorem sim_add_r_sp_eq (h : R kc kr a m) (hit : a.it = 8#8) {rd n : Nat} (hrd : rd ≤ 12)
    (hn : a.sp.toNat + n < 4294967296) :
    ∃ a', execI (.addrsp rd (BitVec.ofNat 32 n)) 2#32 a = some a' ∧ a'.pc = a.pc + 2#32 ∧
      a'.it = 0#8 ∧ a'.sp = a.sp ∧
      R kc (if a.z then kset kr rd 0 else kr) a' (i_add_r_sp_eq m rd n) := by
  rw [execI_it hit (by decide) (by simp) (by simp)]
  have hz := h.z
  have ht : itAdvance 8#8 = 0#8 := by decide
  have hcd : (8#8 : BitVec 8).toNat / 16 = 0 := by decide
  rw [hcd, ht]
  simp only [condHolds]
  cases hzz : a.z
  · rw [hzz] at hz
    simp only [Bool.false_eq_true, if_false, i_add_r_sp_eq, hz]
    exact ⟨_, rfl, rfl, rfl, rfl, by rkeep h; exact hz⟩
  · rw [hzz] at hz
    simp only [if_true, eff, Option.map_some, setR_low hrd, i_add_r_sp_eq, hz]
    refine ⟨_, rfl, rfl, rfl, rfl, ?_⟩
    refine (h.setr hrd ?_).pcit _ _
    rw [toNat_add_lit a.sp n hn, h.sp]
    omega

/-- `pop {r3-r11, lr}` after `vpopeq` leaves R0 to R2 and R12. -/
theorem pop_vpop_r (s : M) (i : Nat) (hi : i < 3 ∨ i = 12) :
    (i_pop_r3_r11_lr (i_vpop_eq s)).r i = s.r i := by
  have c : ¬ (3 ≤ i ∧ i ≤ 11) := by omega
  unfold i_pop_r3_r11_lr i_vpop_eq
  simp only [if_neg c]
  split_ifs <;> rfl

/-! ## Steps of the machine -/

theorem step_of {code : W → Option Nat} {a a' : Arm.St} {i : Instr} {len : W}
    (hf : fetch code a.pc = some (i, len)) (he : execI i len a = some a') :
    step code a = some a' := by
  simp [step, hf, he]

theorem R.kcongr (h : R kc kr a m) {kc' : Int} {kr' : Nat → Int} (e1 : kc = kc')
    (e2 : ∀ i, i ≤ 12 → kr i = kr' i) : R kc' kr' a m := by
  subst e1
  rkeep h
  case r => intro i hi; rw [← e2 i hi]; exact h.r i hi

/-- Normalises `base + j + k` to `base + (j + k)` with the sum computed. -/
macro "pcn" : tactic => `(tactic| simp only [BitVec.add_assoc, BitVec.reduceAdd])

end Stub
