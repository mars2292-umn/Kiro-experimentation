import Sched.CoreTotal
open Aeneas Aeneas.Std Result

/-! # `Layout::validate` returns, and a layout it accepts is well formed (2026-11-20)

`Layout::validate` is the boot check of the memory assignment. Its five
functions are written without `return` inside a loop (2026-11-20), so that
their Lean translation exists. Here each is proven to return for every
layout, and a layout `validate` accepts is shown to satisfy `LWf`, the
statement of Verus's `Layout::wf` over the extracted types: the region
rules of every interval, the Kernel's intervals apart from each other, every
Partition's own layout well formed and apart from the Kernel's intervals
(its peripheral intervals included, R16.2), and two Partitions sharing
nothing but the shared code.

The loops' invariants keep the facts in the form the code checks them
(`LRaw`); `lwf_of_raw` turns them into `LWf`, and `lok_of_raw` into the
bounds the Kernel's later reads rely on (`LOk`). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NP : Usize}

theorem max_windows : logic.mpu.MAX_WINDOWS = 5#usize := by
  unfold logic.mpu.MAX_WINDOWS; rfl

theorem max_kwindows : logic.mpu.MAX_KERNEL_WINDOWS = 8#usize := by
  unfold logic.mpu.MAX_KERNEL_WINDOWS; rfl

/-! ## The bounds (2026-11-20) -/

/-- The first `m` intervals of `a` have a size exponent `pow2_u32` takes. -/
def SzOk {n : Usize} (a : Array logic.mpu.Interval n) (m : Nat) : Prop :=
  ∀ (j : Nat) (h : j < a.val.length), j < m → (a.val[j]).size_log2.val ≤ 31

theorem szok_zero {n : Usize} (a : Array logic.mpu.Interval n) : SzOk a 0 :=
  fun _ _ h => absurd h (Nat.not_lt_zero _)

theorem szok_succ {n : Usize} (a : Array logic.mpu.Interval n) (m : Nat) :
    SzOk a (m + 1) ↔ SzOk a m ∧ ∀ h : m < a.val.length, (a.val[m]).size_log2.val ≤ 31 := by
  constructor
  · intro h
    exact ⟨fun j hj hlt => h j hj (by omega), fun hm => h m hm (by omega)⟩
  · rintro ⟨h1, h2⟩ j hj hlt
    rcases Nat.lt_succ_iff_lt_or_eq.mp hlt with h | h
    · exact h1 j hj h
    · subst h; exact h2 hj

theorem SzOk.mono {n : Usize} {a : Array logic.mpu.Interval n} {m m' : Nat} (h : SzOk a m)
    (hle : m' ≤ m) : SzOk a m' := fun j hj hlt => h j hj (by omega)

/-- The bounds of one Partition's layout. -/
def PLOk (pl : logic.mpu.PartitionLayout) : Prop :=
  pl.ram.size_log2.val ≤ 31 ∧ pl.code.size_log2.val ≤ 31 ∧ pl.window_count.val ≤ 5 ∧
  SzOk pl.windows pl.window_count.val

/-- The first `m` Partitions' layouts satisfy `PLOk`. -/
def POk (a : Array logic.mpu.PartitionLayout NP) (m : Nat) : Prop :=
  ∀ (j : Nat) (h : j < a.val.length), j < m → PLOk (a.val[j])

/-- The bounds of a layout `Layout::validate` accepts. -/
def LOk (l : logic.mpu.Layout NP) : Prop :=
  POk l.partitions NP.val ∧ l.kernel_window_count.val ≤ 8 ∧
  SzOk l.kernel_windows l.kernel_window_count.val

/-- Closes the goals a body leaves (bounds). -/
macro "lcl" : tactic => `(tactic| all_goals first
  | (scalar_tac; done)
  | assumption
  | (simp_all [szok_succ, szok_zero, PLOk, LOk]; done)
  | (simp_all [szok_succ, szok_zero, PLOk, LOk]; scalar_tac; done)
  | (simp_all [szok_succ, szok_zero, PLOk, LOk]
     refine SzOk.mono ?_ ?_ <;> first | assumption | (scalar_tac; done)))

/-! ## The statements of `mpu.rs`, over the extracted types -/

/-- `P` holds of every index below `m`. -/
def AllLt (m : Nat) (P : Nat → Prop) : Prop := ∀ j, j < m → P j

theorem allLt_zero (P : Nat → Prop) : AllLt 0 P := fun _ h => absurd h (Nat.not_lt_zero _)

theorem allLt_succ (m : Nat) (P : Nat → Prop) : AllLt (m + 1) P ↔ AllLt m P ∧ P m := by
  constructor
  · intro h
    exact ⟨fun j hj => h j (by omega), h m (by omega)⟩
  · rintro ⟨h1, h2⟩ j hj
    rcases Nat.lt_succ_iff_lt_or_eq.mp hj with h | h
    · exact h1 j h
    · subst h; exact h2

theorem AllLt.mono {m m' : Nat} {P : Nat → Prop} (h : AllLt m P) (hle : m' ≤ m) : AllLt m' P :=
  fun j hj => h j (by omega)

theorem AllLt.imp {m : Nat} {P Q : Nat → Prop} (h : AllLt m P) (hpq : ∀ j, P j → Q j) :
    AllLt m Q := fun j hj => hpq j (h j hj)

/-- The interval at index `j` (any interval outside the array). -/
def ivAt {n : Usize} (a : Array logic.mpu.Interval n) (j : Nat) : logic.mpu.Interval :=
  (a.val[j]?).getD { base := 0#u32, size_log2 := 0#u8 }

theorem ivAt_get {n : Usize} (a : Array logic.mpu.Interval n) (j : Nat) (h : j < a.val.length) :
    a.val[j] = ivAt a j := by
  simp [ivAt, List.getElem?_eq_getElem h]

/-- The Partition layout at index `j` (any layout outside the array). -/
def plAt (a : Array logic.mpu.PartitionLayout NP) (j : Nat) : logic.mpu.PartitionLayout :=
  (a.val[j]?).getD
    { code := { base := 0#u32, size_log2 := 0#u8 }, code_srd := 0#u8,
      ram := { base := 0#u32, size_log2 := 0#u8 }, ram_srd := 0#u8, stack_size := 0#u32,
      windows := Array.repeat 5#usize { base := 0#u32, size_log2 := 0#u8 }, window_count := 0#u8,
      windows_ro := Array.repeat 5#usize false }

theorem plAt_get (a : Array logic.mpu.PartitionLayout NP) (j : Nat) (h : j < a.val.length) :
    a.val[j] = plAt a j := by
  simp [plAt, List.getElem?_eq_getElem h]

/-- `Interval::end`. -/
def iEnd (i : logic.mpu.Interval) : Nat := i.base.val + 2 ^ i.size_log2.val

/-- `Interval::rules_ok`: at least 32 bytes, the base aligned to the size,
inside the address space. -/
def RulesOk (i : logic.mpu.Interval) : Prop :=
  5 ≤ i.size_log2.val ∧ i.size_log2.val ≤ 31 ∧ i.base.val % 2 ^ i.size_log2.val = 0 ∧
  iEnd i ≤ 4294967296

/-- `rules_ok` with its bound on the exponent stated apart (what the calls
of `check_disjoint` need). -/
def RB (i : logic.mpu.Interval) : Prop := RulesOk i ∧ i.size_log2.val ≤ 31

/-- `Interval::disjoint`. -/
def Disj (a b : logic.mpu.Interval) : Prop := iEnd a ≤ b.base.val ∨ iEnd b ≤ a.base.val

/-- `Layout::apart_from_kernel_windows`. -/
def ApartK (kw : Array logic.mpu.Interval 8#usize) (kwc : U8) (i : logic.mpu.Interval) : Prop :=
  AllLt kwc.val (fun k => Disj i (ivAt kw k))

/-- `Layout::kernel_windows_ok`, as `validate_kernel_windows` checks it. -/
def KWOk (kw : Array logic.mpu.Interval 8#usize) (kwc : U8) (shared : logic.mpu.Interval) : Prop :=
  kwc.val ≤ 8 ∧ AllLt kwc.val (fun j => (ivAt kw j).size_log2.val ≤ 31) ∧
  AllLt kwc.val (fun j => RB (ivAt kw j) ∧ Disj shared (ivAt kw j))

/-- What `validate_partition`'s loop checks of one window: its rules, and
that it is apart from the Partition's RAM and code, from the Kernel's
intervals and its peripheral intervals; it does not end at the RAM base. -/
def WOne (sh fl kr : logic.mpu.Interval) (kw : Array logic.mpu.Interval 8#usize) (kwc : U8)
    (code : logic.mpu.Interval) (rb : U32) (rk : U8) (win : logic.mpu.Interval) : Prop :=
  RB win ∧ Disj win { base := rb, size_log2 := rk } ∧ Disj win code ∧ iEnd win ≠ rb.val ∧
  Disj win fl ∧ Disj win kr ∧ Disj win sh ∧ AllLt kwc.val (fun k => Disj win (ivAt kw k))

/-- The first `m` windows pass `WOne` and are pairwise disjoint. -/
def WInv (sh fl kr : logic.mpu.Interval) (kw : Array logic.mpu.Interval 8#usize) (kwc : U8)
    (code : logic.mpu.Interval) (rb : U32) (rk : U8) (ws : Array logic.mpu.Interval 5#usize)
    (m : Nat) : Prop :=
  AllLt m (fun j => WOne sh fl kr kw kwc code rb rk (ivAt ws j) ∧
    AllLt j (fun u => Disj (ivAt ws u) (ivAt ws j)))

/-- What `validate_partition` checks of Partition `p`. -/
def PartRaw (sh fl kr : logic.mpu.Interval) (kw : Array logic.mpu.Interval 8#usize) (kwc : U8)
    (ps : Array logic.mpu.PartitionLayout NP) (p : Nat) : Prop :=
  RB (plAt ps p).code ∧ RB (plAt ps p).ram ∧ 8 ≤ (plAt ps p).ram.size_log2.val ∧
  0 < (plAt ps p).stack_size.val ∧
  (plAt ps p).stack_size.val ≤ 2 ^ (plAt ps p).ram.size_log2.val ∧
  (plAt ps p).window_count.val ≤ 5 ∧ Disj (plAt ps p).code (plAt ps p).ram ∧
  Disj (plAt ps p).code fl ∧ Disj (plAt ps p).code kr ∧ Disj (plAt ps p).code sh ∧
  Disj (plAt ps p).ram fl ∧ Disj (plAt ps p).ram kr ∧ Disj (plAt ps p).ram sh ∧
  ApartK kw kwc (plAt ps p).code ∧ ApartK kw kwc (plAt ps p).ram ∧
  WInv sh fl kr kw kwc (plAt ps p).code (plAt ps p).ram.base (plAt ps p).ram.size_log2
    (plAt ps p).windows (plAt ps p).window_count.val

/-- What `validate_pair`'s first loop checks: every window of the first
Partition is apart from the second's RAM, code and windows. -/
def PW0 (a : Array logic.mpu.Interval 5#usize) (i : U32) (i1 : U8) (i2 : U32) (i3 : U8)
    (a1 : Array logic.mpu.Interval 5#usize) (cb m : Nat) : Prop :=
  AllLt m (fun v => (ivAt a v).size_log2.val ≤ 31 ∧
    Disj (ivAt a v) { base := i2, size_log2 := i3 } ∧
    Disj (ivAt a v) { base := i, size_log2 := i1 } ∧
    AllLt cb (fun j =>
      Disj { base := (ivAt a v).base, size_log2 := (ivAt a v).size_log2 } (ivAt a1 j)))

/-- What its second loop checks: every window of the second Partition is
apart from the first's RAM and code. -/
def PW1 (i : U32) (i1 : U8) (i2 : U32) (i3 : U8) (a : Array logic.mpu.Interval 5#usize)
    (m : Nat) : Prop :=
  AllLt m (fun w => Disj (ivAt a w) { base := i2, size_log2 := i3 } ∧
    Disj (ivAt a w) { base := i, size_log2 := i1 })

/-- What `validate_pair` checks of Partitions `p` and `q`. -/
def PairRaw (ps : Array logic.mpu.PartitionLayout NP) (p q : Nat) : Prop :=
  (plAt ps p).code.size_log2.val ≤ 31 ∧ (plAt ps p).ram.size_log2.val ≤ 31 ∧
  (plAt ps q).code.size_log2.val ≤ 31 ∧ (plAt ps q).ram.size_log2.val ≤ 31 ∧
  Disj (plAt ps p).code (plAt ps q).code ∧ Disj (plAt ps p).code (plAt ps q).ram ∧
  Disj (plAt ps p).ram (plAt ps q).code ∧ Disj (plAt ps p).ram (plAt ps q).ram ∧
  (plAt ps p).window_count.val ≤ 5 ∧ (plAt ps q).window_count.val ≤ 5 ∧
  PW0 (plAt ps p).windows (plAt ps q).code.base (plAt ps q).code.size_log2 (plAt ps q).ram.base
    (plAt ps q).ram.size_log2 (plAt ps q).windows (plAt ps q).window_count.val
    (plAt ps p).window_count.val ∧
  PW1 (plAt ps p).code.base (plAt ps p).code.size_log2 (plAt ps p).ram.base
    (plAt ps p).ram.size_log2 (plAt ps q).windows (plAt ps q).window_count.val

/-- The first `m` Partitions pass `validate_partition`, and each passes
`validate_pair` with every earlier one, both ways. -/
def VInv (sh fl kr : logic.mpu.Interval) (kw : Array logic.mpu.Interval 8#usize) (kwc : U8)
    (ps : Array logic.mpu.PartitionLayout NP) (m : Nat) : Prop :=
  AllLt m (fun j => PartRaw sh fl kr kw kwc ps j ∧
    AllLt j (fun q => PairRaw ps j q ∧ PairRaw ps q j))

/-- What `Layout::validate` checks, in the form its code checks it. -/
def LRaw (l : logic.mpu.Layout NP) : Prop :=
  RB l.shared_code ∧ RB l.kernel_flash ∧ RB l.kernel_ram ∧
  Disj l.shared_code l.kernel_flash ∧ Disj l.shared_code l.kernel_ram ∧
  Disj l.kernel_flash l.kernel_ram ∧
  KWOk l.kernel_windows l.kernel_window_count l.shared_code ∧
  VInv l.shared_code l.kernel_flash l.kernel_ram l.kernel_windows l.kernel_window_count
    l.partitions NP.val

/-- Closes the goals a body leaves (well-formedness). -/
macro "wcl" : tactic => `(tactic| all_goals first
  | (scalar_tac; done)
  | assumption
  | (simp_all [allLt_succ, allLt_zero, ivAt_get, plAt_get, RB, ApartK, KWOk, WOne, WInv, PartRaw,
      PW0, PW1, PairRaw, VInv, LRaw, iEnd]; done)
  | (simp_all [allLt_succ, allLt_zero, ivAt_get, plAt_get, RB, ApartK, KWOk, WOne, WInv, PartRaw,
      PW0, PW1, PairRaw, VInv, LRaw, iEnd]; scalar_tac; done)
  | (simp_all [allLt_succ, allLt_zero, ivAt_get, plAt_get, RB, ApartK, KWOk, WOne, WInv, PartRaw,
      PW0, PW1, PairRaw, VInv, LRaw, iEnd]
     refine AllLt.mono ?_ ?_ <;> first | assumption | (scalar_tac; done)))

/-! ## The checks -/

@[step] theorem pow2_u32_t (k : U8) (hk : k.val ≤ 31) :
    logic.mpu.pow2_u32 k ⦃ r => r.val = 2 ^ k.val ∧ 0 < r.val ⦄ := by
  unfold logic.mpu.pow2_u32 logic.mpu.pow2_u32_loop
  apply loop.spec_decr_nat (fun (x : U32 × U8) => k.val - x.2.val)
    (fun x => x.2.val ≤ k.val ∧ x.1.val = 2 ^ x.2.val)
  · intro x hx
    obtain ⟨r, i⟩ := x
    simp only at hx ⊢
    unfold logic.mpu.pow2_u32_loop.body
    split
    · rename_i hlt
      have hlt' : i.val < k.val := by scalar_tac
      have hle : 2 ^ i.val ≤ 2 ^ 30 := Nat.pow_le_pow_right (by decide) (by omega)
      have hpw : (2:Nat) ^ 30 = 1073741824 := by decide
      have hs : (2:Nat) ^ (i.val + 1) = 2 ^ i.val * 2 := pow_succ 2 i.val
      step as ⟨r1, hr1⟩
      step as ⟨i1, hi1⟩
      refine ⟨by omega, ?_, by omega⟩
      rw [hr1, hi1, hs, hx.2]
    · rename_i hge
      step*
      have e : i.val = k.val := by scalar_tac
      rw [← e, hx.2]
      exact ⟨rfl, Nat.pow_pos (by decide)⟩
  · exact ⟨Nat.zero_le _, rfl⟩

/-- `check_rules` returns for every interval, and `true` only for one that
satisfies the region rules (Verus: `ensures ok == self.rules_ok()`). -/
@[step] theorem check_rules_t (i : logic.mpu.Interval) :
    logic.mpu.Interval.check_rules i ⦃ b => b = true → RB i ⦄ := by
  unfold logic.mpu.Interval.check_rules
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all [RB, RulesOk, iEnd]; done)
    | (simp_all [RB, RulesOk, iEnd]; scalar_tac)

/-- `check_disjoint` returns `true` only for disjoint intervals. -/
@[step] theorem check_disjoint_t (a b : logic.mpu.Interval) (ha : a.size_log2.val ≤ 31)
    (hb : b.size_log2.val ≤ 31) :
    logic.mpu.Interval.check_disjoint a b ⦃ r => r = true → Disj a b ⦄ := by
  unfold logic.mpu.Interval.check_disjoint
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all [Disj, iEnd]; done)
    | (simp_all [Disj, iEnd]; scalar_tac)

set_option maxHeartbeats 16000000 in
@[step] theorem check_apart_loop_t (a : Array logic.mpu.Interval 8#usize) (i : logic.mpu.Interval) (count : Usize) (good : Bool) (k : Usize)
    (hi : i.size_log2.val ≤ 31) (hc : count.val ≤ 8) (hw : AllLt count.val (fun j => (ivAt a j).size_log2.val ≤ 31))
    (hk : k.val ≤ count.val) (hinv : good = true → AllLt k.val (fun j => Disj i (ivAt a j))) :
    logic.mpu.Layout.check_apart_loop a i count good k ⦃ r => r = true → AllLt count.val (fun j => Disj i (ivAt a j)) ⦄ := by
  unfold logic.mpu.Layout.check_apart_loop
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => count.val - x.2.val) (fun x => x.2.val ≤ count.val ∧ (x.1 = true → AllLt x.2.val (fun j => Disj i (ivAt a j))))
  · intro x hx
    obtain ⟨g, k1⟩ := x
    simp only at hx ⊢
    unfold logic.mpu.Layout.check_apart_loop.body
    try simp only [max_windows, max_kwindows]
    have hwk := hw k1.val
    ctac
    wcl
  · exact ⟨hk, hinv⟩

@[step] theorem check_apart_t (l : logic.mpu.Layout NP) (i : logic.mpu.Interval) (hi : i.size_log2.val ≤ 31)
    (hc : l.kernel_window_count.val ≤ 8)
    (hw : AllLt l.kernel_window_count.val (fun j => (ivAt l.kernel_windows j).size_log2.val ≤ 31)) :
    logic.mpu.Layout.check_apart l i ⦃ b => b = true → ApartK l.kernel_windows l.kernel_window_count i ⦄ := by
  unfold logic.mpu.Layout.check_apart
  step as ⟨count, hcount⟩
  have hcv : count.val = l.kernel_window_count.val := by scalar_tac
  refine WP.spec_mono (check_apart_loop_t _ i count true 0#usize hi (by omega)
    (by rw [hcv]; exact hw) (by scalar_tac) (fun _ => allLt_zero _)) (fun b hb hbt => ?_)
  unfold ApartK
  rw [← hcv]
  exact hb hbt

set_option maxHeartbeats 16000000 in
@[step] theorem validate_kernel_windows_loop_t (a : Array logic.mpu.Interval 8#usize) (count : Usize) (shared : logic.mpu.Interval) (good : Bool) (k : Usize)
    (hs : shared.size_log2.val ≤ 31) (hc : count.val ≤ 8) (hk : k.val ≤ count.val)
    (hinv : good = true → AllLt k.val (fun j => RB (ivAt a j) ∧ Disj shared (ivAt a j))) :
    logic.mpu.Layout.validate_kernel_windows_loop a count shared good k ⦃ r => r = true → AllLt count.val (fun j => RB (ivAt a j) ∧ Disj shared (ivAt a j)) ⦄ := by
  unfold logic.mpu.Layout.validate_kernel_windows_loop
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => count.val - x.2.val) (fun x => x.2.val ≤ count.val ∧ (x.1 = true → AllLt x.2.val (fun j => RB (ivAt a j) ∧ Disj shared (ivAt a j))))
  · intro x hx
    obtain ⟨g, k1⟩ := x
    simp only at hx ⊢
    unfold logic.mpu.Layout.validate_kernel_windows_loop.body
    try simp only [max_windows, max_kwindows]
    ctac
    wcl
  · exact ⟨hk, hinv⟩

@[step] theorem validate_kernel_windows_t (l : logic.mpu.Layout NP)
    (hs : l.shared_code.size_log2.val ≤ 31) :
    logic.mpu.Layout.validate_kernel_windows l ⦃ b => b = true →
      KWOk l.kernel_windows l.kernel_window_count l.shared_code ⦄ := by
  unfold logic.mpu.Layout.validate_kernel_windows
  simp only [max_kwindows]
  step as ⟨count, hcount⟩
  have hcv : count.val = l.kernel_window_count.val := by scalar_tac
  split
  · step*
  · rename_i hle
    refine WP.spec_mono (validate_kernel_windows_loop_t l.kernel_windows count l.shared_code true
      0#usize hs (by scalar_tac) (by scalar_tac) (fun _ => allLt_zero _)) (fun b hb hbt => ?_)
    have h2 := hb hbt
    rw [hcv] at h2
    exact ⟨by scalar_tac, h2.imp (fun j h => h.1.2), h2⟩

set_option maxHeartbeats 16000000 in
@[step] theorem validate_partition_loop0_loop0_t (a : Array logic.mpu.Interval 5#usize) (w : Usize) (win : logic.mpu.Interval) (fine : Bool) (u : Usize)
    (hw5 : w.val ≤ 5) (hwin : win.size_log2.val ≤ 31) (ha : AllLt w.val (fun j => (ivAt a j).size_log2.val ≤ 31))
    (hu : u.val ≤ w.val) (hinv : fine = true → AllLt u.val (fun j => Disj (ivAt a j) win)) :
    logic.mpu.Layout.validate_partition_loop0_loop0 a w win fine u ⦃ r => r = true → AllLt w.val (fun j => Disj (ivAt a j) win) ⦄ := by
  unfold logic.mpu.Layout.validate_partition_loop0_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => w.val - x.2.val) (fun x => x.2.val ≤ w.val ∧ (x.1 = true → AllLt x.2.val (fun j => Disj (ivAt a j) win)))
  · intro x hx
    obtain ⟨f, u1⟩ := x
    simp only at hx ⊢
    unfold logic.mpu.Layout.validate_partition_loop0_loop0.body
    try simp only [max_windows, max_kwindows]
    have hau := ha u1.val
    ctac
    wcl
  · exact ⟨hu, hinv⟩

set_option maxHeartbeats 16000000 in
@[step] theorem validate_partition_loop0_t (i i1 i2 : logic.mpu.Interval) (a : Array logic.mpu.Interval 8#usize) (i3 : U8)
    (a1 : Array logic.mpu.PartitionLayout NP) (i4 : logic.mpu.Interval) (i5 : U32) (i6 : U8) (a2 : Array logic.mpu.Interval 5#usize)
    (count : Usize) (good : Bool) (w : Usize)
    (hi : i.size_log2.val ≤ 31) (hi1 : i1.size_log2.val ≤ 31) (hi2 : i2.size_log2.val ≤ 31)
    (hk3 : i3.val ≤ 8) (hka : AllLt i3.val (fun j => (ivAt a j).size_log2.val ≤ 31)) (hi4 : i4.size_log2.val ≤ 31)
    (hi6 : i6.val ≤ 31) (hc : count.val ≤ 5) (hwc : w.val ≤ count.val)
    (hinv : good = true → WInv i i1 i2 a i3 i4 i5 i6 a2 w.val) :
    logic.mpu.Layout.validate_partition_loop0 i i1 i2 a i3 a1 i4 i5 i6 a2 count good w ⦃ r => r = true → WInv i i1 i2 a i3 i4 i5 i6 a2 count.val ⦄ := by
  unfold logic.mpu.Layout.validate_partition_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => count.val - x.2.val) (fun x => x.2.val ≤ count.val ∧ (x.1 = true → WInv i i1 i2 a i3 i4 i5 i6 a2 x.2.val))
  · intro x hx
    obtain ⟨g, w1⟩ := x
    simp only at hx ⊢
    unfold logic.mpu.Layout.validate_partition_loop0.body
    try simp only [max_windows, max_kwindows]
    have hsz : g = true → AllLt w1.val (fun j => (ivAt a2 j).size_log2.val ≤ 31) := fun hg =>
      (hx.2 hg).imp (fun j h => h.1.1.2)
    ctac
    wcl
  · exact ⟨hwc, hinv⟩

set_option maxHeartbeats 32000000 in
@[step] theorem validate_partition_t (l : logic.mpu.Layout NP) (p : Usize) (hp : p.val < NP.val)
    (hs : l.shared_code.size_log2.val ≤ 31) (hf : l.kernel_flash.size_log2.val ≤ 31)
    (hr : l.kernel_ram.size_log2.val ≤ 31) (hc : l.kernel_window_count.val ≤ 8)
    (hw : AllLt l.kernel_window_count.val (fun j => (ivAt l.kernel_windows j).size_log2.val ≤ 31)) :
    logic.mpu.Layout.validate_partition l p ⦃ b => b = true →
      PartRaw l.shared_code l.kernel_flash l.kernel_ram l.kernel_windows l.kernel_window_count
        l.partitions p.val ⦄ := by
  unfold logic.mpu.Layout.validate_partition
  simp only [max_windows]
  ctac
  wcl

set_option maxHeartbeats 16000000 in
@[step] theorem validate_pair_loop0_loop0_t (a : Array logic.mpu.Interval 5#usize) (cb : Usize) (i : U32) (i1 : U8) (fine : Bool) (j : Usize)
    (hcb : cb.val ≤ 5) (h1 : i1.val ≤ 31) (hj : j.val ≤ cb.val)
    (hinv : fine = true → AllLt j.val (fun k => Disj { base := i, size_log2 := i1 } (ivAt a k))) :
    logic.mpu.Layout.validate_pair_loop0_loop0 a cb i i1 fine j ⦃ r => r = true → AllLt cb.val (fun k => Disj { base := i, size_log2 := i1 } (ivAt a k)) ⦄ := by
  unfold logic.mpu.Layout.validate_pair_loop0_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => cb.val - x.2.val) (fun x => x.2.val ≤ cb.val ∧ (x.1 = true → AllLt x.2.val (fun k => Disj { base := i, size_log2 := i1 } (ivAt a k))))
  · intro x hx
    obtain ⟨f, j1⟩ := x
    simp only at hx ⊢
    unfold logic.mpu.Layout.validate_pair_loop0_loop0.body
    try simp only [max_windows, max_kwindows]
    ctac
    wcl
  · exact ⟨hj, hinv⟩

set_option maxHeartbeats 16000000 in
@[step] theorem validate_pair_loop0_t (a : Array logic.mpu.Interval 5#usize) (i : U32) (i1 : U8) (i2 : U32) (i3 : U8)
    (a1 : Array logic.mpu.Interval 5#usize) (ca cb : Usize) (good : Bool) (v : Usize)
    (hca : ca.val ≤ 5) (hcb : cb.val ≤ 5) (h1 : i1.val ≤ 31) (h3 : i3.val ≤ 31)
    (hv : v.val ≤ ca.val) (hinv : good = true → PW0 a i i1 i2 i3 a1 cb.val v.val) :
    logic.mpu.Layout.validate_pair_loop0 a i i1 i2 i3 a1 ca cb good v ⦃ r => r = true → PW0 a i i1 i2 i3 a1 cb.val ca.val ⦄ := by
  unfold logic.mpu.Layout.validate_pair_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => ca.val - x.2.val) (fun x => x.2.val ≤ ca.val ∧ (x.1 = true → PW0 a i i1 i2 i3 a1 cb.val x.2.val))
  · intro x hx
    obtain ⟨g, v1⟩ := x
    simp only at hx ⊢
    unfold logic.mpu.Layout.validate_pair_loop0.body
    try simp only [max_windows, max_kwindows]
    ctac
    wcl
  · exact ⟨hv, hinv⟩

set_option maxHeartbeats 16000000 in
@[step] theorem validate_pair_loop1_t (i : U32) (i1 : U8) (i2 : U32) (i3 : U8) (a : Array logic.mpu.Interval 5#usize)
    (cb : Usize) (good : Bool) (w : Usize) (hcb : cb.val ≤ 5) (h1 : i1.val ≤ 31)
    (h3 : i3.val ≤ 31) (hw : w.val ≤ cb.val)
    (hinv : good = true → PW1 i i1 i2 i3 a w.val) :
    logic.mpu.Layout.validate_pair_loop1 i i1 i2 i3 a cb good w ⦃ r => r = true → good = true ∧ PW1 i i1 i2 i3 a cb.val ⦄ := by
  unfold logic.mpu.Layout.validate_pair_loop1
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => cb.val - x.2.val) (fun x => x.2.val ≤ cb.val ∧ (x.1 = true → good = true ∧ PW1 i i1 i2 i3 a x.2.val))
  · intro x hx
    obtain ⟨g, w1⟩ := x
    simp only at hx ⊢
    unfold logic.mpu.Layout.validate_pair_loop1.body
    try simp only [max_windows, max_kwindows]
    ctac
    wcl
  · exact ⟨hw, fun h => ⟨h, hinv h⟩⟩

set_option maxHeartbeats 32000000 in
@[step] theorem validate_pair_t (l : logic.mpu.Layout NP) (p q : Usize) (hp : p.val < NP.val)
    (hq : q.val < NP.val) :
    logic.mpu.Layout.validate_pair l p q ⦃ b => b = true → PairRaw l.partitions p.val q.val ⦄ := by
  unfold logic.mpu.Layout.validate_pair
  simp only [max_windows]
  ctac
  wcl

set_option maxHeartbeats 16000000 in
@[step] theorem validate_loop0_loop0_t (i i1 i2 : logic.mpu.Interval) (a : Array logic.mpu.Interval 8#usize) (i3 : U8)
    (a1 : Array logic.mpu.PartitionLayout NP) (p : Usize) (fine : Bool) (q : Usize) (hp : p.val < NP.val)
    (hq : q.val ≤ p.val)
    (hinv : fine = true → AllLt q.val (fun j => PairRaw a1 p.val j ∧ PairRaw a1 j p.val)) :
    logic.mpu.Layout.validate_loop0_loop0 i i1 i2 a i3 a1 p fine q ⦃ r => r = true → AllLt p.val (fun j => PairRaw a1 p.val j ∧ PairRaw a1 j p.val) ⦄ := by
  unfold logic.mpu.Layout.validate_loop0_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => p.val - x.2.val) (fun x => x.2.val ≤ p.val ∧ (x.1 = true → AllLt x.2.val (fun j => PairRaw a1 p.val j ∧ PairRaw a1 j p.val)))
  · intro x hx
    obtain ⟨f, q1⟩ := x
    simp only at hx ⊢
    unfold logic.mpu.Layout.validate_loop0_loop0.body
    try simp only [max_windows, max_kwindows]
    ctac
    wcl
  · exact ⟨hq, hinv⟩

set_option maxHeartbeats 16000000 in
@[step] theorem validate_loop0_t (i i1 i2 : logic.mpu.Interval) (a : Array logic.mpu.Interval 8#usize) (i3 : U8)
    (a1 : Array logic.mpu.PartitionLayout NP) (good : Bool) (p : Usize)
    (hi : i.size_log2.val ≤ 31) (hi1 : i1.size_log2.val ≤ 31) (hi2 : i2.size_log2.val ≤ 31)
    (hk3 : i3.val ≤ 8) (hka : AllLt i3.val (fun j => (ivAt a j).size_log2.val ≤ 31)) (hp : p.val ≤ NP.val)
    (hinv : good = true → VInv i i1 i2 a i3 a1 p.val) :
    logic.mpu.Layout.validate_loop0 i i1 i2 a i3 a1 good p ⦃ r => r = true → VInv i i1 i2 a i3 a1 NP.val ⦄ := by
  unfold logic.mpu.Layout.validate_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => NP.val - x.2.val) (fun x => x.2.val ≤ NP.val ∧ (x.1 = true → VInv i i1 i2 a i3 a1 x.2.val))
  · intro x hx
    obtain ⟨g, p1⟩ := x
    simp only at hx ⊢
    unfold logic.mpu.Layout.validate_loop0.body
    try simp only [max_windows, max_kwindows]
    ctac
    wcl
  · exact ⟨hp, hinv⟩

set_option maxHeartbeats 32000000 in
/-- **`Layout::validate` returns for every layout**, and `true` only when
every check of its code passed. -/
@[step] theorem layout_validate_raw (l : logic.mpu.Layout NP) :
    logic.mpu.Layout.validate l ⦃ b => b = true → LRaw l ⦄ := by
  unfold logic.mpu.Layout.validate
  ctac
  wcl

/-! ## `Layout::wf`

The definitions below follow `mpu.rs` clause by clause (`Interval::contains`
appears only as `!w.contains(ram.base - 1)`, written `NoBelow`). -/

/-- `!win.contains(rb - 1)`: the word below the stack is not in the window. -/
def NoBelow (win : logic.mpu.Interval) (rb : U32) : Prop :=
  ¬ ((win.base.val : Int) ≤ (rb.val : Int) - 1 ∧ (rb.val : Int) - 1 < (iEnd win : Int))

/-- `PartitionLayout::wf`. -/
def PLWf (pl : logic.mpu.PartitionLayout) : Prop :=
  RulesOk pl.code ∧ RulesOk pl.ram ∧ 8 ≤ pl.ram.size_log2.val ∧
  0 < pl.stack_size.val ∧ pl.stack_size.val ≤ 2 ^ pl.ram.size_log2.val ∧
  pl.window_count.val ≤ 5 ∧
  AllLt pl.window_count.val (fun w => RulesOk (ivAt pl.windows w)) ∧
  Disj pl.code pl.ram ∧
  AllLt pl.window_count.val (fun w =>
    Disj (ivAt pl.windows w) pl.ram ∧ Disj (ivAt pl.windows w) pl.code) ∧
  AllLt pl.window_count.val (fun w => NoBelow (ivAt pl.windows w) pl.ram.base) ∧
  (∀ v w, v < w → w < pl.window_count.val → Disj (ivAt pl.windows v) (ivAt pl.windows w))

/-- `Layout::partition_ok`. -/
def PartWf (l : logic.mpu.Layout NP) (p : Nat) : Prop :=
  PLWf (plAt l.partitions p) ∧
  (Disj (plAt l.partitions p).code l.kernel_flash ∧ Disj (plAt l.partitions p).code l.kernel_ram ∧
    Disj (plAt l.partitions p).code l.shared_code) ∧
  (Disj (plAt l.partitions p).ram l.kernel_flash ∧ Disj (plAt l.partitions p).ram l.kernel_ram ∧
    Disj (plAt l.partitions p).ram l.shared_code) ∧
  AllLt (plAt l.partitions p).window_count.val (fun w =>
    Disj (ivAt (plAt l.partitions p).windows w) l.kernel_flash ∧
    Disj (ivAt (plAt l.partitions p).windows w) l.kernel_ram ∧
    Disj (ivAt (plAt l.partitions p).windows w) l.shared_code) ∧
  (ApartK l.kernel_windows l.kernel_window_count (plAt l.partitions p).code ∧
    ApartK l.kernel_windows l.kernel_window_count (plAt l.partitions p).ram) ∧
  AllLt (plAt l.partitions p).window_count.val (fun w =>
    ApartK l.kernel_windows l.kernel_window_count (ivAt (plAt l.partitions p).windows w))

/-- `Layout::pair_ok`: two Partitions share nothing but the shared code. -/
def PairWf (l : logic.mpu.Layout NP) (p q : Nat) : Prop :=
  (Disj (plAt l.partitions p).code (plAt l.partitions q).code ∧
    Disj (plAt l.partitions p).code (plAt l.partitions q).ram ∧
    Disj (plAt l.partitions p).ram (plAt l.partitions q).code ∧
    Disj (plAt l.partitions p).ram (plAt l.partitions q).ram) ∧
  AllLt (plAt l.partitions p).window_count.val (fun w =>
    Disj (ivAt (plAt l.partitions p).windows w) (plAt l.partitions q).ram ∧
    Disj (ivAt (plAt l.partitions p).windows w) (plAt l.partitions q).code) ∧
  AllLt (plAt l.partitions q).window_count.val (fun w =>
    Disj (ivAt (plAt l.partitions q).windows w) (plAt l.partitions p).ram ∧
    Disj (ivAt (plAt l.partitions q).windows w) (plAt l.partitions p).code) ∧
  (∀ v w, v < (plAt l.partitions p).window_count.val → w < (plAt l.partitions q).window_count.val →
    Disj (ivAt (plAt l.partitions p).windows v) (ivAt (plAt l.partitions q).windows w))

/-- `Layout::wf`. -/
def LWf (l : logic.mpu.Layout NP) : Prop :=
  (RulesOk l.shared_code ∧ RulesOk l.kernel_flash ∧ RulesOk l.kernel_ram) ∧
  (Disj l.shared_code l.kernel_flash ∧ Disj l.shared_code l.kernel_ram) ∧
  Disj l.kernel_flash l.kernel_ram ∧
  l.kernel_window_count.val ≤ 8 ∧
  AllLt l.kernel_window_count.val (fun k => RulesOk (ivAt l.kernel_windows k)) ∧
  ApartK l.kernel_windows l.kernel_window_count l.shared_code ∧
  (∀ p, p < NP.val → PartWf l p) ∧
  (∀ p q, p < NP.val → q < NP.val → p ≠ q → PairWf l p q)

/-- A window apart from the RAM that does not end at the RAM's base does
not hold the word below it. -/
theorem noBelow_of {win : logic.mpu.Interval} {rb : U32} {rk : U8}
    (hd : Disj win { base := rb, size_log2 := rk }) (hne : iEnd win ≠ rb.val) :
    NoBelow win rb := by
  unfold NoBelow
  rintro ⟨h1, h2⟩
  unfold Disj at hd
  simp only [iEnd] at hd hne h2
  rcases hd with h | h
  · omega
  · have h' : rb.val ≤ win.base.val := Nat.le_trans (Nat.le_add_right _ _) h
    omega

theorem partwf_of_raw {l : logic.mpu.Layout NP} {p : Nat}
    (h : PartRaw l.shared_code l.kernel_flash l.kernel_ram l.kernel_windows l.kernel_window_count
      l.partitions p) : PartWf l p := by
  obtain ⟨c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, c11, c12, c13, c14, c15, hw⟩ := h
  unfold WInv at hw
  have hram : ∀ w, w < (plAt l.partitions p).window_count.val →
      Disj (ivAt (plAt l.partitions p).windows w) (plAt l.partitions p).ram :=
    fun w hlt => (hw w hlt).1.2.1
  refine ⟨⟨c1.1, c2.1, c3, c4, c5, c6, fun w hlt => (hw w hlt).1.1.1, c7,
    fun w hlt => ⟨hram w hlt, (hw w hlt).1.2.2.1⟩,
    fun w hlt => noBelow_of (hw w hlt).1.2.1 (hw w hlt).1.2.2.2.1,
    fun v w hvw hlt => (hw w hlt).2 v hvw⟩,
    ⟨c8, c9, c10⟩, ⟨c11, c12, c13⟩,
    fun w hlt => ⟨(hw w hlt).1.2.2.2.2.1, (hw w hlt).1.2.2.2.2.2.1, (hw w hlt).1.2.2.2.2.2.2.1⟩,
    ⟨c14, c15⟩, fun w hlt => (hw w hlt).1.2.2.2.2.2.2.2⟩

theorem pairwf_of_raw {l : logic.mpu.Layout NP} {p q : Nat} (h : PairRaw l.partitions p q) :
    PairWf l p q := by
  obtain ⟨_, _, _, _, d1, d2, d3, d4, _, _, h0, h1⟩ := h
  unfold PW0 at h0
  unfold PW1 at h1
  exact ⟨⟨d1, d2, d3, d4⟩, fun w hlt => ⟨(h0 w hlt).2.1, (h0 w hlt).2.2.1⟩,
    fun w hlt => h1 w hlt, fun v w hv hw => (h0 v hv).2.2.2 w hw⟩

/-- **The checks of `Layout::validate` give `Layout::wf`.** -/
theorem lwf_of_raw {l : logic.mpu.Layout NP} (h : LRaw l) : LWf l := by
  obtain ⟨r1, r2, r3, d1, d2, d3, hk, hv⟩ := h
  obtain ⟨hk1, _, hk3⟩ := hk
  unfold VInv at hv
  refine ⟨⟨r1.1, r2.1, r3.1⟩, ⟨d1, d2⟩, d3, hk1, fun k hlt => (hk3 k hlt).1.1,
    fun k hlt => (hk3 k hlt).2, fun p hp => partwf_of_raw (hv p hp).1, fun p q hp hq hne => ?_⟩
  rcases Nat.lt_or_gt_of_ne hne with hlt | hgt
  · exact pairwf_of_raw ((hv q hq).2 p hlt).2
  · exact pairwf_of_raw ((hv p hp).2 q hgt).1

/-- The checks give the bounds the Kernel's later reads of the layout rely
on. -/
theorem lok_of_raw {l : logic.mpu.Layout NP} (h : LRaw l) : LOk l := by
  obtain ⟨_, _, _, _, _, _, hk, hv⟩ := h
  obtain ⟨hk1, hk2, _⟩ := hk
  unfold VInv at hv
  refine ⟨fun j hj hlt => ?_, hk1, fun j hj hlt => ?_⟩
  · rw [plAt_get]
    obtain ⟨c1, c2, _, _, _, c6, _, _, _, _, _, _, _, _, _, hw⟩ := (hv j hlt).1
    unfold WInv at hw
    refine ⟨c2.2, c1.2, c6, fun w hw' hwlt => ?_⟩
    rw [ivAt_get]
    exact (hw w hwlt).1.1.2
  · rw [ivAt_get]
    exact hk2 j hlt

/-- **A layout `Layout::validate` accepts is well formed** (Verus:
`ensures ok ==> self.wf()`), on the extracted code. -/
theorem layout_validate_wf (l : logic.mpu.Layout NP) :
    logic.mpu.Layout.validate l ⦃ b => b = true → LWf l ⦄ :=
  WP.spec_mono (layout_validate_raw l) (fun _ hb hbt => lwf_of_raw (hb hbt))

/-- `Layout::validate` returns for every layout, and one it accepts
satisfies `LOk`. -/
theorem layout_validate_t (l : logic.mpu.Layout NP) :
    logic.mpu.Layout.validate l ⦃ b => b = true → LOk l ⦄ :=
  WP.spec_mono (layout_validate_raw l) (fun _ hb hbt => lok_of_raw (hb hbt))

end P18
