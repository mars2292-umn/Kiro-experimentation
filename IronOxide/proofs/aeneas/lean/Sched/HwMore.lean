import Sched.IsolationBoot
open Aeneas Aeneas.Std Result

/-! # The rest of the Hardware_Model (2026-11-25)

`Sched/Nvic.lean` has the interrupt controller's statements of
`rsk_logic::hw_model`. This file has the others, each restated from the
Verus text (the model is a specification and has no compiled form) and,
where it speaks of the Kernel's data, proven against the extracted
`Layout::view` and the layout's disjointness clauses (`LayoutOk`, what
`Layout::validate` checks, `Sched/Isolation.lean`):

- HW-03 `basepri_max_only_raises`; HW-07 `return_from_sole_active`;
  HW-04, HW-08 `job_cannot_set_mask`.
- HW-09, the MPU's decision: the region that decides an access is the
  highest-numbered enabled region that covers the address (`Decides`,
  `max_covering_exists`); an access the MPU grants to unprivileged code is
  covered by the view (`mpu_grant_covered`).
- `hw_partition_isolation` (`lemma_hw_partition_isolation`, Property 12
  over the model): with the view `Layout::view` returns for a Partition
  loaded, an address the MPU grants lies in the shared code or in one of
  the Partition's own intervals; a write in its RAM or a window; a fetch
  in the shared code or its code.
- `hw_write_isolation`: no address is writable under one Partition's view
  and accessible under another's.
- `kernel_ram_kept` (`lemma_kernel_ram_kept_between_entries`): a Job that
  writes only words the MPU grants it leaves every word of the Kernel's
  RAM as it was.
- HW-10 the frame sizes; HW-11 `busFaultPrecise`; HW-12
  `compare_never_too_close`; HW-13 `elapsed_in_range`.
- HW-15 `dma_footprint` (`lemma_dma_footprint_in_partition`): a buffer the
  EasyDMA check permits lies in the Partition's RAM above its stack
  region, outside the Kernel's memory and every other Partition's
  intervals. -/

namespace P18.HwM
open rsk_kernel rsk_kernel.rsk_logic P18

variable {NP : Usize}

/-! ## HW-03, HW-07, HW-04 and HW-08 -/

/-- HW-03: an MSR to BASEPRI_MAX raises BASEPRI only. -/
def basepriMaxWrite (current value : Nat) : Nat :=
  if current = 0 ∨ (value ≠ 0 ∧ value < current) then value else current

theorem basepri_max_only_raises (current value : Nat) :
    basepriMaxWrite current value = 0 ∨ current = 0 ∨ basepriMaxWrite current value ≤ current := by
  unfold basepriMaxWrite
  split
  · rename_i h
    rcases h with h | ⟨_, h⟩
    · exact Or.inr (Or.inl h)
    · exact Or.inr (Or.inr (Nat.le_of_lt h))
  · exact Or.inr (Or.inr (Nat.le_refl _))

/-- HW-07: an exception return to Thread mode while other exceptions are
active is permitted only with CCR.NONBASETHRDENA set. -/
def exceptionReturnOk (nonbasethrdena toThread othersActive : Bool) : Bool :=
  !(toThread && othersActive) || nonbasethrdena

/-- HW-07 (deactivation): after the return of the only active exception
none is active. -/
theorem return_from_sole_active (returning : Nat) :
    ({returning} : Finset Nat).erase returning = ∅ := by
  simp

/-- HW-08: Handler mode executes privileged; Thread mode is unprivileged
exactly when CONTROL.nPRIV is 1. -/
def privileged (handlerMode npriv : Bool) : Bool := handlerMode || !npriv

/-- HW-04 with HW-08: a write of PRIMASK or FAULTMASK takes effect only
when executed privileged. -/
def maskWrite (priv current set : Bool) : Bool := if priv then set else current

/-- A Job (Thread mode, unprivileged) cannot set PRIMASK or FAULTMASK. -/
theorem job_cannot_set_mask (current set : Bool) :
    maskWrite (privileged false true) current set = current := rfl

/-! ## HW-09: the MPU's decision -/

/-- Region `r` decides an access to `a`: it covers the address and no
higher-numbered region does (Armv7-M B3.5.3). -/
def Decides (v : logic.mpu.View) (a r : Nat) : Prop :=
  r < 8 ∧ covers (v.regions.val[r]!) a ∧ ∀ s, s < 8 → covers (v.regions.val[s]!) a → s ≤ r

/-- The access kinds of the model. -/
inductive Access where
  | read
  | write
  | fetch

/-- The MPU grants an unprivileged access: a region decides it, writable
for a write, not execute-never for a fetch. With no deciding region an
unprivileged access faults (PRIVDEFENA covers privileged code only). -/
def mpuGrants (v : logic.mpu.View) (a : Nat) (kind : Access) : Prop :=
  ∃ r, Decides v a r ∧
    match kind with
    | .read => True
    | .write => (v.regions.val[r]!).write = true
    | .fetch => (v.regions.val[r]!).xn = false

/-- Among the regions below `n` that cover an address there is a
highest-numbered one. -/
theorem max_covering_below (v : logic.mpu.View) (a : Nat) :
    ∀ n : Nat, (∃ r, r < n ∧ covers (v.regions.val[r]!) a) →
      ∃ r, r < n ∧ covers (v.regions.val[r]!) a ∧
        ∀ s, s < n → covers (v.regions.val[s]!) a → s ≤ r := by
  intro n
  induction n with
  | zero => rintro ⟨r, hr, _⟩; omega
  | succ n ih =>
    rintro ⟨r0, hr0, hc0⟩
    by_cases hn : covers (v.regions.val[n]!) a
    · exact ⟨n, by omega, hn, fun s hs _ => by omega⟩
    · have hr0n : r0 ≠ n := fun e => hn (by rw [← e]; exact hc0)
      obtain ⟨r, hr, hc, hmax⟩ := ih ⟨r0, by omega, hc0⟩
      refine ⟨r, by omega, hc, fun s hs hcs => ?_⟩
      by_cases hsn : s = n
      · rw [hsn] at hcs; exact absurd hcs hn
      · exact hmax s (by omega) hcs

/-- HW-09: when a region covers the address, one decides
(`lemma_max_covering_region_exists`). -/
theorem max_covering_exists {v : logic.mpu.View} {a : Nat} (h : readable v a) :
    ∃ r, Decides v a r := by
  obtain ⟨k, hk, hc⟩ := h
  obtain ⟨r, hr, hcr, hmax⟩ := max_covering_below v a 8 ⟨k, hk, hc⟩
  exact ⟨r, hr, hcr, hmax⟩

/-- An access the MPU grants is covered by the view
(`lemma_mpu_grant_covered`). -/
theorem mpu_grant_covered {v : logic.mpu.View} {a : Nat} {kind : Access}
    (h : mpuGrants v a kind) :
    readable v a ∧ (kind = .write → writable v a) ∧ (kind = .fetch → executable v a) := by
  obtain ⟨r, ⟨hr, hc, _⟩, hk⟩ := h
  refine ⟨⟨r, hr, hc⟩, fun e => ?_, fun e => ?_⟩
  · subst e; exact ⟨r, hr, hc, hk⟩
  · subst e; exact ⟨r, hr, hc, hk⟩

/-! ## Property 12 over the model -/

/-- **With a Partition's view loaded, the MPU grants only that Partition's
memory** (`lemma_hw_partition_isolation`). -/
theorem hw_partition_isolation {l : logic.mpu.Layout NP} {p : Usize} {v : logic.mpu.View}
    (hv : logic.mpu.Layout.view l p = ok v) {a : Nat} {kind : Access} (h : mpuGrants v a kind) :
    (icontains l.shared_code a ∨ icontains (l.partitions.val[p.val]!).code a ∨
      icontains (l.partitions.val[p.val]!).ram a ∨ inWindow (l.partitions.val[p.val]!) a) ∧
    (kind = .write → icontains (l.partitions.val[p.val]!).ram a ∨
      inWindow (l.partitions.val[p.val]!) a) ∧
    (kind = .fetch → icontains l.shared_code a ∨ icontains (l.partitions.val[p.val]!).code a) := by
  obtain ⟨_, hall⟩ := view_ok hv
  obtain ⟨hr, hw, hx⟩ := mpu_grant_covered h
  obtain ⟨g1, g2, g3⟩ := view_within_layout hall a
  exact ⟨g1 hr, fun e => g2 (hw e), fun e => g3 (hx e)⟩

/-- **No address is writable under one Partition's view and accessible
under another's** (`lemma_hw_write_isolation`). -/
theorem hw_write_isolation {l : logic.mpu.Layout NP} (hl : LayoutOk l) {p q : Usize}
    {vp vq : logic.mpu.View} (hp : logic.mpu.Layout.view l p = ok vp)
    (hq : logic.mpu.Layout.view l q = ok vq) (hne : p.val ≠ q.val) (a : Nat) (kind : Access) :
    ¬ (mpuGrants vp a .write ∧ mpuGrants vq a kind) := by
  rintro ⟨h1, h2⟩
  exact views_disjoint hl hp hq hne a ⟨(mpu_grant_covered h1).2.1 rfl, (mpu_grant_covered h2).1⟩

/-- A word of four bytes at `a` is writable when the MPU grants a write of
one of them. -/
def wordWritable (v : logic.mpu.View) (a : Nat) : Prop :=
  mpuGrants v a .write ∨ mpuGrants v (a + 1) .write ∨ mpuGrants v (a + 2) .write ∨
    mpuGrants v (a + 3) .write

/-- From `m1` to `m2` a Job changed only words it may write. -/
def jobWritesOnly (v : logic.mpu.View) (m1 m2 : Nat → Nat) : Prop :=
  ∀ a, ¬ wordWritable v a → m2 a = m1 a

/-- **A Job leaves the Kernel's RAM as it was**
(`lemma_kernel_ram_kept_between_entries`): with its Partition's view
loaded, a word that lies in the Kernel's RAM is not one it may write. -/
theorem kernel_ram_kept {l : logic.mpu.Layout NP} (hl : LayoutOk l) {p : Usize}
    {v : logic.mpu.View} (hv : logic.mpu.Layout.view l p = ok v) {m1 m2 : Nat → Nat}
    (hj : jobWritesOnly v m1 m2) {a : Nat}
    (ha : icontains l.kernel_ram a ∧ icontains l.kernel_ram (a + 3)) : m2 a = m1 a := by
  apply hj
  have hno : ∀ b, icontains l.kernel_ram b → ¬ mpuGrants v b .write := fun b hb hg =>
    view_no_kernel_memory hl hv b ⟨(mpu_grant_covered hg).1, Or.inr hb⟩
  have hin : ∀ d, d ≤ 3 → icontains l.kernel_ram (a + d) := by
    intro d hd
    obtain ⟨⟨h1, h2⟩, ⟨_, h4⟩⟩ := ha
    exact ⟨by omega, by omega⟩
  rintro (h | h | h | h)
  · exact hno a (hin 0 (by omega)) h
  · exact hno (a + 1) (hin 1 (by omega)) h
  · exact hno (a + 2) (hin 2 (by omega)) h
  · exact hno (a + 3) (hin 3 (by omega)) h

/-! ## HW-10 to HW-13 -/

/-- HW-10: the bytes of an exception frame, basic or extended. -/
def frameBytes (fpca : Bool) : Nat := if fpca then 104 else 32

/-- HW-10: the frame an exception entry reserves; with lazy stacking off
(the Kernel's setting) and with it on, the extended frame's space is
reserved when FPCA is set. -/
def entryFrameSize (fpca lspen : Bool) : Nat :=
  if fpca && !lspen then 104 else if fpca then 104 else 32

theorem entry_frame_is_frame (fpca lspen : Bool) : entryFrameSize fpca lspen = frameBytes fpca := by
  cases fpca <;> cases lspen <;> rfl

/-- HW-11: a bus fault is precise with the write buffer disabled, or for a
strongly-ordered region. -/
def busFaultPrecise (disdefwbuf stronglyOrdered : Bool) : Bool := disdefwbuf || stronglyOrdered

/-- HW-12: the least distance of a compare value from the counter. -/
def minCompareDistance : Nat := 16

/-- HW-12: the extended instant of a counter reading. -/
def extendedInstant (wraps low : Nat) : Nat := wraps * 4294967296 + low

/-- HW-12: the compare value for a target instant. -/
def compareFor (now target : Nat) : Nat :=
  if target < now + minCompareDistance then now + minCompareDistance else target

theorem compare_never_too_close (now target : Nat) :
    now + minCompareDistance ≤ compareFor now target ∧ target ≤ compareFor now target := by
  unfold compareFor
  split <;> omega

/-- HW-13: the cycles elapsed between two readings of the 32-bit counter. -/
def elapsedCycles (start stop : Nat) : Nat :=
  if start ≤ stop then stop - start else stop + 4294967296 - start

theorem elapsed_in_range {start stop : Nat} (hs : start < 4294967296) (he : stop < 4294967296) :
    elapsedCycles start stop < 4294967296 := by
  unfold elapsedCycles
  split <;> omega

/-! ## HW-15: the EasyDMA footprint -/

/-- The bytes an EasyDMA transfer touches. -/
def dmaTouches (ptr len a : Nat) : Prop := ptr ≤ a ∧ a < ptr + len

/-- An address in one of a Partition's intervals. -/
def partitionOwns (pl : logic.mpu.PartitionLayout) (a : Nat) : Prop :=
  icontains pl.code a ∨ icontains pl.ram a ∨ inWindow pl a

/-- **A permitted EasyDMA buffer lies in its Partition's RAM above the
stack region, outside the Kernel's memory and every other Partition's
intervals** (`lemma_dma_footprint_in_partition`). -/
theorem dma_footprint {l : logic.mpu.Layout NP} (hl : LayoutOk l) {p : Nat} (hp : p < NP.val)
    {ptr len a : Nat} (hd : DmaPermitted (l.partitions.val[p]!) ptr len)
    (ht : dmaTouches ptr len a) :
    icontains (l.partitions.val[p]!).ram a ∧
    (l.partitions.val[p]!).ram.base.val + (l.partitions.val[p]!).stack_size.val ≤ a ∧
    ¬ icontains l.kernel_ram a ∧ ¬ icontains l.kernel_flash a ∧
    ∀ q, q < NP.val → q ≠ p → ¬ partitionOwns (l.partitions.val[q]!) a := by
  obtain ⟨_, h1, h2, _⟩ := hd
  obtain ⟨t1, t2⟩ := ht
  have hin : icontains (l.partitions.val[p]!).ram a := by
    unfold icontains isz
    omega
  have pp := hl.part p hp
  refine ⟨hin, by omega, fun hk => idisj_apart pp.ram_kernel.2.1 hin hk,
    fun hk => idisj_apart pp.ram_kernel.1 hin hk, fun q hq hne => ?_⟩
  have pq := hl.pair p q hp hq (fun e => hne e.symm)
  rintro (hc | hr | ⟨w, hw, hi⟩)
  · exact idisj_apart pq.ram_code hin hc
  · exact idisj_apart pq.ram_ram hin hr
  · exact idisj_apart (pq.win_b w hw).1 hi hin

end P18.HwM
