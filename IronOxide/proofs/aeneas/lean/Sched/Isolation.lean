import Sched.Start
open Aeneas Aeneas.Std Result

/-! # Partition views are disjoint, on the Kernel's code (2026-11-19)

Property 12 of the design, which Verus proves of `rsk_logic::logic::mpu`
(`Layout::view`, `lemma_view_within_layout`, `lemma_views_disjoint`,
`lemma_view_no_kernel_window`, `lemma_ro_window_not_writable`,
`lemma_stack_overflow_faults`), stated of `Layout::view` as Aeneas
extracts it. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NP : Usize}

deriving instance Inhabited for rsk_kernel.rsk_logic.logic.mpu.Interval
deriving instance Inhabited for rsk_kernel.rsk_logic.logic.mpu.Region
deriving instance Inhabited for rsk_kernel.rsk_logic.logic.mpu.PartitionLayout

/-! ## Intervals, regions and views -/

/-- The size of an interval, two to its exponent. -/
def isz (i : logic.mpu.Interval) : Nat := 2 ^ i.size_log2.val

/-- The address lies in the interval. -/
def icontains (i : logic.mpu.Interval) (a : Nat) : Prop := i.base.val ≤ a ∧ a < i.base.val + isz i

/-- The intervals share no address. -/
def idisj (i j : logic.mpu.Interval) : Prop :=
  i.base.val + isz i ≤ j.base.val ∨ j.base.val + isz j ≤ i.base.val

theorem idisj_apart {i j : logic.mpu.Interval} {a : Nat} (h : idisj i j) (hi : icontains i a)
    (hj : icontains j a) : False := by
  unfold idisj at h
  unfold icontains at hi hj
  omega

/-- The MPU's region decision (HW-09): the region is enabled, the address
lies in its interval, and, for a region of 256 bytes or more, the eighth
of the region the address falls in is not disabled. -/
def covers (r : logic.mpu.Region) (a : Nat) : Prop :=
  r.enabled = true ∧ icontains r.interval a ∧
  (r.interval.size_log2.val < 8 ∨
    (r.srd.val / 2 ^ ((a - r.interval.base.val) / (isz r.interval / 8))) % 2 = 0)

def readable (v : logic.mpu.View) (a : Nat) : Prop :=
  ∃ k, k < 8 ∧ covers (v.regions.val[k]!) a

def writable (v : logic.mpu.View) (a : Nat) : Prop :=
  ∃ k, k < 8 ∧ covers (v.regions.val[k]!) a ∧ (v.regions.val[k]!).write = true

def executable (v : logic.mpu.View) (a : Nat) : Prop :=
  ∃ k, k < 8 ∧ covers (v.regions.val[k]!) a ∧ (v.regions.val[k]!).xn = false

/-- The address lies in one of the Partition's peripheral windows. -/
def inWindow (pl : logic.mpu.PartitionLayout) (a : Nat) : Prop :=
  ∃ w, w < pl.window_count.val ∧ icontains (pl.windows.val[w]!) a

/-! ## What `Layout::view` returns -/

/-- A region of a Partition's view: disabled, or the shared code (read and
execute), the Partition's code (read and execute), its RAM (no execute),
or one of its windows (no execute, writable only when the layout does not
say read-only). -/
def RegOk (l : logic.mpu.Layout NP) (pl : logic.mpu.PartitionLayout) (r : logic.mpu.Region) : Prop :=
  r.enabled = false ∨
  (r.interval = l.shared_code ∧ r.write = false) ∨
  (r.interval = pl.code ∧ r.write = false) ∨
  (r.interval = pl.ram ∧ r.xn = true) ∨
  (∃ w, w < pl.window_count.val ∧ r.interval = pl.windows.val[w]! ∧ r.xn = true ∧
    (r.write = true → pl.windows_ro.val[w]! = false))

def AllOk (l : logic.mpu.Layout NP) (pl : logic.mpu.PartitionLayout)
    (a : Array logic.mpu.Region 8#usize) : Prop :=
  ∀ k, k < 8 → RegOk l pl (a.val[k]!)

theorem allok_set {l : logic.mpu.Layout NP} {pl : logic.mpu.PartitionLayout}
    {a : Array logic.mpu.Region 8#usize} {i : Usize} {x : logic.mpu.Region}
    (ha : AllOk l pl a) (hx : RegOk l pl x) (hi : i.val < 8) : AllOk l pl (a.set i x) := by
  intro k hk
  rw [get_set a i x k (by scalar_tac)]
  split
  · exact hx
  · exact ha k hk

theorem allok_fill {l : logic.mpu.Layout NP} {pl : logic.mpu.PartitionLayout}
    (a : Array logic.mpu.Region 8#usize) {d : logic.mpu.Region}
    (h : a.val = List.replicate 8 d) (hd : d.enabled = false) : AllOk l pl a := by
  intro k hk
  have e : (List.replicate 8 d)[k]! = d := by
    have hk8 : k = 0 ∨ k = 1 ∨ k = 2 ∨ k = 3 ∨ k = 4 ∨ k = 5 ∨ k = 6 ∨ k = 7 := by omega
    rcases hk8 with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> rfl
  rw [h, e]
  exact Or.inl hd

/-- The loop of `Layout::view` over the Partition's windows. -/
theorem view_loop_ok {l : logic.mpu.Layout NP} {pl : logic.mpu.PartitionLayout} {cnt : Usize}
    (hc : cnt.val ≤ pl.window_count.val) :
    ∀ (n : Nat) (w : Usize) (regions res : Array logic.mpu.Region 8#usize), cnt.val - w.val = n →
      AllOk l pl regions →
      logic.mpu.Layout.view_loop pl.windows pl.windows_ro regions cnt w = ok res →
      AllOk l pl res := by
  intro n
  induction n with
  | zero =>
    intro w regions res hn ha h
    unfold logic.mpu.Layout.view_loop loop at h
    simp only [logic.mpu.Layout.view_loop.body] at h
    have hlt : ¬ w < cnt := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    exact ha
  | succ n ih =>
    intro w regions res hn ha h
    unfold logic.mpu.Layout.view_loop loop at h
    simp only [logic.mpu.Layout.view_loop.body] at h
    have hlt : w < cnt := by scalar_tac
    simp only [hlt, ite_true, bind_ok, bind_eq_ok] at h
    obtain ⟨cf, ⟨i, hi, b, hb, i1, hi1, a2, ha2, w1, hw1, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    obtain ⟨_, hiv⟩ := index_ok hi
    obtain ⟨_, hbv⟩ := index_ok hb
    have hi1v := usize_add_ok hi1
    have hw1v := usize_add_ok hw1
    have h1 : (1#usize).val = 1 := rfl
    have h3 : (3#usize).val = 3 := rfl
    rw [h1] at hw1v
    rw [h3] at hi1v
    obtain ⟨hlt8, ha2v⟩ := update_ok ha2
    have h8 : (8#usize).val = 8 := rfl
    rw [h8] at hlt8
    subst ha2v
    have hwv : w.val < cnt.val := by scalar_tac
    refine ih w1 _ res (by omega) ?_ (by
      unfold logic.mpu.Layout.view_loop
      simp only [logic.mpu.Layout.view_loop.body]
      exact hloop)
    refine allok_set ha ?_ hlt8
    refine Or.inr (Or.inr (Or.inr (Or.inr ⟨w.val, by omega, hiv, rfl, ?_⟩)))
    intro hwr
    rw [← hbv]
    simpa using hwr

/-- `Layout::view`: the Partition exists, and every region of the view it
returns is disabled or one of the Partition's regions with the layout's
permissions. -/
theorem view_ok {l : logic.mpu.Layout NP} {p : Usize} {v : logic.mpu.View}
    (h : logic.mpu.Layout.view l p = ok v) :
    p.val < NP.val ∧ AllOk l (l.partitions.val[p.val]!) v.regions := by
  unfold logic.mpu.Layout.view at h
  simp only [bind_eq_ok, logic.mpu.Region.disabled, bind_ok] at h
  obtain ⟨pl, hpl, r1, hr1, r2, hr2, r3, hr3, cnt, hcnt, r5, hloop, hv⟩ := h
  obtain ⟨hp, hplv⟩ := index_ok hpl
  subst hplv
  obtain ⟨_, e1⟩ := update_ok hr1
  obtain ⟨_, e2⟩ := update_ok hr2
  obtain ⟨_, e3⟩ := update_ok hr3
  have hv := ok_inj hv
  subst hv
  simp only [lift] at hcnt
  have hcnt := ok_inj hcnt
  have hcv : cnt.val ≤ (l.partitions.val[p.val]!).window_count.val := by
    rw [← hcnt, U8.cast_Usize_val_eq]
  refine ⟨hp, view_loop_ok hcv _ _ _ _ rfl ?_ hloop⟩
  subst e3 e2 e1
  have h0 : (0#usize).val < 8 := by decide
  have h1 : (1#usize).val < 8 := by decide
  have h2 : (2#usize).val < 8 := by decide
  exact allok_set (allok_set (allok_set (allok_fill _ (Array.repeat_val _ _) rfl)
    (Or.inr (Or.inl ⟨rfl, rfl⟩)) h0) (Or.inr (Or.inr (Or.inl ⟨rfl, rfl⟩))) h1)
    (Or.inr (Or.inr (Or.inr (Or.inl ⟨rfl, rfl⟩)))) h2

/-! ## The layout's disjointness (the clauses of `Layout::wf` used below) -/

/-- A Partition's own intervals: its code, RAM and windows share no
address with each other, with the Kernel's flash and RAM, with the shared
code, or with a Kernel peripheral interval, and the word below its RAM
(below its stack, R16.5) lies in none of its windows. -/
structure PartOk (l : logic.mpu.Layout NP) (pl : logic.mpu.PartitionLayout) : Prop where
  code_ram : idisj pl.code pl.ram
  win_ram : ∀ w, w < pl.window_count.val → idisj (pl.windows.val[w]!) pl.ram
  win_code : ∀ w, w < pl.window_count.val → idisj (pl.windows.val[w]!) pl.code
  win_win : ∀ v w, v < w → w < pl.window_count.val →
    idisj (pl.windows.val[v]!) (pl.windows.val[w]!)
  below_stack : ∀ w a, w < pl.window_count.val → a + 1 = pl.ram.base.val →
    ¬ icontains (pl.windows.val[w]!) a
  code_kernel : idisj pl.code l.kernel_flash ∧ idisj pl.code l.kernel_ram ∧ idisj pl.code l.shared_code
  ram_kernel : idisj pl.ram l.kernel_flash ∧ idisj pl.ram l.kernel_ram ∧ idisj pl.ram l.shared_code
  win_kernel : ∀ w, w < pl.window_count.val → idisj (pl.windows.val[w]!) l.kernel_flash ∧
    idisj (pl.windows.val[w]!) l.kernel_ram ∧ idisj (pl.windows.val[w]!) l.shared_code
  code_kw : ∀ k, k < l.kernel_window_count.val → idisj pl.code (l.kernel_windows.val[k]!)
  ram_kw : ∀ k, k < l.kernel_window_count.val → idisj pl.ram (l.kernel_windows.val[k]!)
  win_kw : ∀ w k, w < pl.window_count.val → k < l.kernel_window_count.val →
    idisj (pl.windows.val[w]!) (l.kernel_windows.val[k]!)

/-- Two Partitions' intervals share no address. -/
structure PairOk (a b : logic.mpu.PartitionLayout) : Prop where
  code_code : idisj a.code b.code
  code_ram : idisj a.code b.ram
  ram_code : idisj a.ram b.code
  ram_ram : idisj a.ram b.ram
  win_a : ∀ w, w < a.window_count.val → idisj (a.windows.val[w]!) b.ram ∧
    idisj (a.windows.val[w]!) b.code
  win_b : ∀ w, w < b.window_count.val → idisj (b.windows.val[w]!) a.ram ∧
    idisj (b.windows.val[w]!) a.code
  win_win : ∀ v w, v < a.window_count.val → w < b.window_count.val →
    idisj (a.windows.val[v]!) (b.windows.val[w]!)

/-- The disjointness clauses of `Layout::wf`. -/
structure LayoutOk (l : logic.mpu.Layout NP) : Prop where
  shared_kernel : idisj l.shared_code l.kernel_flash ∧ idisj l.shared_code l.kernel_ram
  shared_kw : ∀ k, k < l.kernel_window_count.val → idisj l.shared_code (l.kernel_windows.val[k]!)
  part : ∀ p, p < NP.val → PartOk l (l.partitions.val[p]!)
  pair : ∀ p q, p < NP.val → q < NP.val → p ≠ q →
    PairOk (l.partitions.val[p]!) (l.partitions.val[q]!)

/-! ## Property 12 -/

/-- (a) An address a view grants lies in the shared code or in one of the
Partition's intervals; a writable one in its RAM or a window; an
executable one in the shared code or its code. -/
theorem view_within_layout {l : logic.mpu.Layout NP} {pl : logic.mpu.PartitionLayout}
    {v : logic.mpu.View} (h : AllOk l pl v.regions) (a : Nat) :
    (readable v a → icontains l.shared_code a ∨ icontains pl.code a ∨ icontains pl.ram a ∨
      inWindow pl a) ∧
    (writable v a → icontains pl.ram a ∨ inWindow pl a) ∧
    (executable v a → icontains l.shared_code a ∨ icontains pl.code a) := by
  refine ⟨?_, ?_, ?_⟩
  · rintro ⟨k, hk, hen, hin, _⟩
    rcases h k hk with hd | ⟨hi, _⟩ | ⟨hi, _⟩ | ⟨hi, _⟩ | ⟨w, hw, hi, _, _⟩
    · rw [hd] at hen; cases hen
    · rw [hi] at hin; exact Or.inl hin
    · rw [hi] at hin; exact Or.inr (Or.inl hin)
    · rw [hi] at hin; exact Or.inr (Or.inr (Or.inl hin))
    · rw [hi] at hin; exact Or.inr (Or.inr (Or.inr ⟨w, hw, hin⟩))
  · rintro ⟨k, hk, ⟨hen, hin, _⟩, hwr⟩
    rcases h k hk with hd | ⟨_, hnw⟩ | ⟨_, hnw⟩ | ⟨hi, _⟩ | ⟨w, hw, hi, _, _⟩
    · rw [hd] at hen; cases hen
    · rw [hnw] at hwr; cases hwr
    · rw [hnw] at hwr; cases hwr
    · rw [hi] at hin; exact Or.inl hin
    · rw [hi] at hin; exact Or.inr ⟨w, hw, hin⟩
  · rintro ⟨k, hk, ⟨hen, hin, _⟩, hx⟩
    rcases h k hk with hd | ⟨hi, _⟩ | ⟨hi, _⟩ | ⟨_, hxn⟩ | ⟨w, _, _, hxn, _⟩
    · rw [hd] at hen; cases hen
    · rw [hi] at hin; exact Or.inl hin
    · rw [hi] at hin; exact Or.inr hin
    · rw [hxn] at hx; cases hx
    · rw [hxn] at hx; cases hx

theorem idisj.symm {i j : logic.mpu.Interval} (h : idisj i j) : idisj j i := by
  unfold idisj at h ⊢
  omega

/-- (b) R16.1, R16.6: no address is writable in one Partition's view and
accessible in another's. -/
theorem views_disjoint {l : logic.mpu.Layout NP} (hl : LayoutOk l) {p q : Usize}
    {vp vq : logic.mpu.View} (hp : logic.mpu.Layout.view l p = ok vp)
    (hq : logic.mpu.Layout.view l q = ok vq) (hne : p.val ≠ q.val) (a : Nat) :
    ¬ (writable vp a ∧ readable vq a) := by
  obtain ⟨hpn, hpa⟩ := view_ok hp
  obtain ⟨hqn, hqa⟩ := view_ok hq
  have pp := hl.part p.val hpn
  have pr := hl.pair p.val q.val hpn hqn hne
  rintro ⟨hw, hr⟩
  have hw' := (view_within_layout hpa a).2.1 hw
  have hr' := (view_within_layout hqa a).1 hr
  rcases hw' with hram | ⟨w, hw, hwin⟩
  · rcases hr' with h | h | h | ⟨u, hu, h⟩
    · exact idisj_apart pp.ram_kernel.2.2 hram h
    · exact idisj_apart pr.ram_code hram h
    · exact idisj_apart pr.ram_ram hram h
    · exact idisj_apart (pr.win_b u hu).1 h hram
  · rcases hr' with h | h | h | ⟨u, hu, h⟩
    · exact idisj_apart (pp.win_kernel w hw).2.2 hwin h
    · exact idisj_apart (pr.win_a w hw).2 hwin h
    · exact idisj_apart (pr.win_a w hw).1 hwin h
    · exact idisj_apart (pr.win_win w u hw hu) hwin h

/-- (b) R16.1, R16.2: no view grants an address of the Kernel's flash or
RAM. -/
theorem view_no_kernel_memory {l : logic.mpu.Layout NP} (hl : LayoutOk l) {p : Usize}
    {v : logic.mpu.View} (hp : logic.mpu.Layout.view l p = ok v) (a : Nat) :
    ¬ (readable v a ∧ (icontains l.kernel_flash a ∨ icontains l.kernel_ram a)) := by
  obtain ⟨hpn, hpa⟩ := view_ok hp
  have pp := hl.part p.val hpn
  rintro ⟨hr, hk⟩
  rcases (view_within_layout hpa a).1 hr with h | h | h | ⟨w, hw, h⟩ <;> rcases hk with hk | hk
  · exact idisj_apart hl.shared_kernel.1 h hk
  · exact idisj_apart hl.shared_kernel.2 h hk
  · exact idisj_apart pp.code_kernel.1 h hk
  · exact idisj_apart pp.code_kernel.2.1 h hk
  · exact idisj_apart pp.ram_kernel.1 h hk
  · exact idisj_apart pp.ram_kernel.2.1 h hk
  · exact idisj_apart (pp.win_kernel w hw).1 h hk
  · exact idisj_apart (pp.win_kernel w hw).2.1 h hk

/-- (d) R16.2: no view grants an address of a Kernel peripheral interval
(the Private Peripheral Bus and the Kernel-owned peripherals). -/
theorem view_no_kernel_window {l : logic.mpu.Layout NP} (hl : LayoutOk l) {p : Usize}
    {v : logic.mpu.View} (hp : logic.mpu.Layout.view l p = ok v) (a : Nat) :
    ¬ (readable v a ∧ ∃ k, k < l.kernel_window_count.val ∧ icontains (l.kernel_windows.val[k]!) a) := by
  obtain ⟨hpn, hpa⟩ := view_ok hp
  have pp := hl.part p.val hpn
  rintro ⟨hr, k, hk, hin⟩
  rcases (view_within_layout hpa a).1 hr with h | h | h | ⟨w, hw, h⟩
  · exact idisj_apart (hl.shared_kw k hk) h hin
  · exact idisj_apart (pp.code_kw k hk) h hin
  · exact idisj_apart (pp.ram_kw k hk) h hin
  · exact idisj_apart (pp.win_kw w k hw hk) h hin

/-- DD-08: an address in a read-only window of a Partition is writable in
no view, the Partition's own included. -/
theorem ro_window_not_writable {l : logic.mpu.Layout NP} (hl : LayoutOk l) {p q : Usize}
    {vq : logic.mpu.View} (hp : p.val < NP.val) (hq : logic.mpu.Layout.view l q = ok vq)
    {a w : Nat} (hw : w < (l.partitions.val[p.val]!).window_count.val)
    (hro : (l.partitions.val[p.val]!).windows_ro.val[w]! = true)
    (hin : icontains ((l.partitions.val[p.val]!).windows.val[w]!) a) : ¬ writable vq a := by
  obtain ⟨hqn, hqa⟩ := view_ok hq
  rintro ⟨k, hk, ⟨hen, hcin, _⟩, hwr⟩
  by_cases hpq : p.val = q.val
  · rw [← hpq] at hqa
    have pp := hl.part p.val hp
    rcases hqa k hk with hd | ⟨_, hnw⟩ | ⟨_, hnw⟩ | ⟨hi, _⟩ | ⟨u, hu, hi, _, hrw⟩
    · rw [hd] at hen; cases hen
    · rw [hnw] at hwr; cases hwr
    · rw [hnw] at hwr; cases hwr
    · rw [hi] at hcin; exact idisj_apart (pp.win_ram w hw) hin hcin
    · rw [hi] at hcin
      have hru := hrw hwr
      rcases Nat.lt_trichotomy u w with h | h | h
      · exact idisj_apart (pp.win_win u w h hw) hcin hin
      · rw [h] at hru; rw [hro] at hru; cases hru
      · exact idisj_apart (pp.win_win w u h hu) hin hcin
  · have pr := hl.pair p.val q.val hp hqn hpq
    have hwv : writable vq a := ⟨k, hk, ⟨hen, hcin, by assumption⟩, hwr⟩
    rcases (view_within_layout hqa a).2.1 hwv with h | ⟨u, hu, h⟩
    · exact idisj_apart (pr.win_a w hw).1 hin h
    · exact idisj_apart (pr.win_win w u hw hu) hin h

/-- R16.5: the word below a Partition's RAM, where its stack would
overflow to, is not writable in its view. -/
theorem stack_overflow_faults {l : logic.mpu.Layout NP} (hl : LayoutOk l) {p : Usize}
    {v : logic.mpu.View} (hp : logic.mpu.Layout.view l p = ok v) {a : Nat}
    (ha : a + 1 = (l.partitions.val[p.val]!).ram.base.val) : ¬ writable v a := by
  obtain ⟨hpn, hpa⟩ := view_ok hp
  have pp := hl.part p.val hpn
  intro hw
  rcases (view_within_layout hpa a).2.1 hw with h | ⟨w, hw, h⟩
  · unfold icontains at h
    omega
  · exact pp.below_stack w a hw ha h

end P18
