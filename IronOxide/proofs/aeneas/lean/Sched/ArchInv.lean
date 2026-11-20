import Aeneas

/-! # The level records and the Kernel's preemption stack agree (2026-11-19)

The invariant of Kernel_Arch's entry cores, which Verus states as
`entry::arch_inv` over two ghost fields of `ArchState` (`chain`,
`chain_levels`: the Tasks dispatched and not yet unwound, with their
levels). The extraction erases ghost fields, so here the invariant is
restated over the fields that exist: the level records themselves. The
chain is recoverable from them (the active levels in rising order, each
naming the one below as the Job it preempted), so no ghost state is
needed.

This file is the invariant and its three structural lemmas, on plain
functions (`LV`, the records read as functions of the level; `KV`, the
Kernel's stack read as functions), with no reference to the extracted code:

- `AInv.entry`: a Kernel entry that leaves the records alone and ends Jobs
  from the top of the stack only keeps the invariant, with the unwind plan
  `end_plan_core` computes (Verus: `lemma_kernel_entry`);
- `ChainOk.push`: a dispatch from a state with nothing to unwind
  (`lemma_push`);
- `ChainOk.pop`: the unwind of the topmost record (`lemma_pop`).

`Sched/ArchCores.lean` proves the extracted entry cores keep it. -/

namespace Arch

/-- The level records read as functions of the level. -/
structure LV where
  act : Nat → Bool
  task : Nat → Nat
  prevTask : Nat → Nat
  part : Nat → Nat
  prevPart : Nat → Nat
  exc : Nat → Nat

/-- The Kernel's preemption stack and configuration read as functions. -/
structure KV where
  depth : Nat
  stack : Nat → Nat
  prio : Nat → Nat
  part : Nat → Nat

/-- What the Kernel's own invariant gives Kernel_Arch (`lemma_stack_rising`):
the stack holds Tasks in strictly rising Priority, each Priority a level 1
to 7, each Partition an identifier. -/
def KOk (v : KV) : Prop :=
  (∀ i j, i < j → j < v.depth → v.prio (v.stack i) < v.prio (v.stack j)) ∧
  (∀ i, i < v.depth → v.stack i < 255 ∧ 1 ≤ v.prio (v.stack i) ∧ v.prio (v.stack i) < 8 ∧
    v.part (v.stack i) < 255)

/-- `v'` is `v` with Jobs ended from the top (`stack_prefix`). -/
def Prefix (v' v : KV) : Prop :=
  v'.depth ≤ v.depth ∧ (∀ i, i < v'.depth → v'.stack i = v.stack i) ∧ v'.prio = v.prio ∧
  v'.part = v.part

/-- The level of the executing Job, 0 when no Job is in progress. -/
def topLevel (v : KV) : Nat := if v.depth = 0 then 0 else v.prio (v.stack (v.depth - 1))

/-- The record of level `l` is well formed: at the level of its Task's
Priority, for that Task's Partition, with a dispatch vector's exception
number, and naming the next active level below as the Job it preempted
(none at the bottom). -/
def RecOk (v : KV) (L : LV) (l : Nat) : Prop :=
  1 ≤ l ∧ l < 8 ∧ L.task l < 255 ∧ v.prio (L.task l) = l ∧ L.exc l ≠ 0 ∧ L.exc l ≤ 511 ∧
  L.part l = v.part (L.task l) ∧ L.part l ≠ 255 ∧
  ((∀ j, j < l → L.act j = false) → L.prevTask l = 255 ∧ L.prevPart l = 255) ∧
  (∀ j, j < l → L.act j = true → (∀ j', j < j' → j' < l → L.act j' = false) →
    L.prevTask l = L.task j ∧ L.prevPart l = v.part (L.task j))

/-- Level `l` is the greatest active level. -/
def IsTop (L : LV) (l : Nat) : Prop := L.act l = true ∧ ∀ j, l < j → L.act j = false

/-- The records describe a chain of dispatched Jobs of which the Kernel's
stack is a prefix; `cp` is the Partition Kernel_Arch takes as executing. -/
def ChainOk (v : KV) (L : LV) (cp : Nat) : Prop :=
  (∀ l, L.act l = true → RecOk v L l) ∧
  (∀ i, i < v.depth → L.act (v.prio (v.stack i)) = true ∧
    L.task (v.prio (v.stack i)) = v.stack i) ∧
  (∀ l, L.act l = true → l ≤ topLevel v → ∃ i, i < v.depth ∧ l = v.prio (v.stack i)) ∧
  ((∀ l, L.act l = false) → cp = 255) ∧
  (∀ l, IsTop L l → cp = v.part (L.task l))

/-- A record is active above the executing Job's level: a Job the Kernel
has ended and Kernel_Arch has not yet unwound. -/
def Extra (v : KV) (L : LV) : Prop := ∃ l, L.act l = true ∧ topLevel v < l

/-- `e` is the exception number of the topmost record. -/
def TopExc (L : LV) (e : Nat) : Prop := ∃ l, IsTop L l ∧ e = L.exc l

/-- The invariant of the entry cores. `hop` is the unwind plan the core
hands its stub (0 between Kernel entries), `pending` the unwind owed to a
dispatch stub at its entry instruction. -/
def AInv (v : KV) (L : LV) (cp pending hop : Nat) : Prop :=
  ChainOk v L cp ∧ (hop ≠ 0 → Extra v L ∧ TopExc L hop) ∧
  (hop = 0 → (Extra v L ↔ pending ≠ 0)) ∧ (pending ≠ 0 → Extra v L ∧ TopExc L pending)

theorem IsTop.unique {L : LV} {a b : Nat} (ha : IsTop L a) (hb : IsTop L b) : a = b := by
  obtain ⟨a1, a2⟩ := ha
  obtain ⟨b1, b2⟩ := hb
  by_contra hne
  rcases Nat.lt_or_gt_of_ne hne with h | h
  · have := a2 b h; rw [b1] at this; cases this
  · have := b2 a h; rw [a1] at this; cases this

theorem topLevel_prefix {v v' : KV} (hk : KOk v) (hp : Prefix v' v) : topLevel v' ≤ topLevel v := by
  obtain ⟨hd, hs, hpr, _⟩ := hp
  unfold topLevel
  by_cases h0 : v'.depth = 0
  · simp [h0]
  · have hv : v.depth ≠ 0 := by omega
    simp only [h0, hv, ite_false]
    rw [hpr, hs _ (by omega)]
    by_cases he : v'.depth = v.depth
    · rw [he]
    · exact Nat.le_of_lt (hk.1 _ _ (by omega) (by omega))

/-- The level of a Job on the stack is at most the executing Job's. -/
theorem stack_le_top {v : KV} (hk : KOk v) {i : Nat} (hi : i < v.depth) :
    v.prio (v.stack i) ≤ topLevel v := by
  unfold topLevel
  have hz : v.depth ≠ 0 := by omega
  simp only [hz, ite_false]
  by_cases he : i = v.depth - 1
  · rw [he]
  · exact Nat.le_of_lt (hk.1 _ _ (by omega) (by omega))

theorem RecOk.congr {v v' : KV} {L : LV} {l : Nat} (hpr : v'.prio = v.prio)
    (hpa : v'.part = v.part) (h : RecOk v L l) : RecOk v' L l := by
  unfold RecOk at h ⊢
  rw [hpr, hpa]
  exact h

/-- `lemma_chain_frame`: the chain survives Jobs ending. -/
theorem ChainOk.prefix {v v' : KV} {L : LV} {cp : Nat} (hk : KOk v) (h : ChainOk v L cp)
    (hp : Prefix v' v) : ChainOk v' L cp := by
  have htl := topLevel_prefix hk hp
  obtain ⟨c1, c2, c3, c4, c5⟩ := h
  obtain ⟨hd, hs, hpr, hpa⟩ := hp
  refine ⟨fun l ha => (c1 l ha).congr hpr hpa, fun i hi => ?_, fun l ha hle => ?_, c4,
    fun l ht => by rw [hpa]; exact c5 l ht⟩
  · rw [hpr, hs i hi]; exact c2 i (by omega)
  · by_cases h0 : v'.depth = 0
    · have := (c1 l ha).1
      simp [topLevel, h0] at hle
      omega
    · obtain ⟨i, hi, he⟩ := c3 l ha (by omega)
      have hi' : i < v'.depth := by
        by_contra hge
        have h1 := hk.1 (v'.depth - 1) i (by omega) hi
        have h2 : topLevel v' = v.prio (v.stack (v'.depth - 1)) := by
          simp only [topLevel, h0, ite_false]
          rw [hpr, hs _ (by omega)]
        omega
      exact ⟨i, hi', by rw [hpr, hs i hi']; exact he⟩

theorem Extra.prefix {v v' : KV} {L : LV} (hk : KOk v) (h : Extra v L) (hp : Prefix v' v) :
    Extra v' L := by
  obtain ⟨l, ha, hl⟩ := h
  have := topLevel_prefix hk hp
  exact ⟨l, ha, by omega⟩

/-- The unwind plan after a transition that may have ended Jobs
(`ended_plan`): the exception number of the topmost record when one did,
else none. -/
def EndedPlan (v v0 : KV) (L : LV) (hop : Nat) : Prop :=
  if v.depth < v0.depth then TopExc L hop else hop = 0

/-- `lemma_kernel_entry`: a Kernel entry that leaves the records alone,
ends Jobs from the top of the stack only, and plans by `EndedPlan` keeps
the invariant. -/
theorem AInv.entry {v v0 : KV} {L : LV} {cp pending hop : Nat} (hk : KOk v0)
    (h : AInv v0 L cp pending 0) (hp : Prefix v v0) (hhop : EndedPlan v v0 L hop) :
    AInv v L cp pending hop := by
  obtain ⟨hc, _, h0, hpend⟩ := h
  have hc' := hc.prefix hk hp
  have hp' := hp
  obtain ⟨hd, hs, hpr, hpa⟩ := hp'
  unfold EndedPlan at hhop
  by_cases hlt : v.depth < v0.depth
  · rw [if_pos hlt] at hhop
    -- The Job at position `v.depth` of the old stack was ended: its record is extra.
    have hex : Extra v L := by
      obtain ⟨c1, c2, _⟩ := hc
      obtain ⟨ha, _⟩ := c2 v.depth hlt
      refine ⟨_, ha, ?_⟩
      unfold topLevel
      by_cases hz : v.depth = 0
      · simp only [hz, if_true]
        have := (hk.2 0 (by omega)).2.1
        omega
      · simp only [hz, ite_false]
        rw [hpr, hs _ (by omega)]
        exact hk.1 _ _ (by omega) hlt
    have hne : hop ≠ 0 := by
      obtain ⟨l, ⟨ha, _⟩, he⟩ := hhop
      have := (hc.1 l ha).2.2.2.2.1
      omega
    exact ⟨hc', fun _ => ⟨hex, hhop⟩, fun hz => absurd hz hne,
      fun hpn => ⟨hex, (hpend hpn).2⟩⟩
  · rw [if_neg hlt] at hhop
    have hde : v.depth = v0.depth := by omega
    have htl : topLevel v = topLevel v0 := by
      unfold topLevel
      rw [hde]
      by_cases hz : v0.depth = 0
      · simp [hz]
      · simp only [hz, ite_false]
        rw [hpr, hs _ (by omega)]
    have hex : Extra v L ↔ Extra v0 L := by
      unfold Extra; rw [htl]
    exact ⟨hc', fun hn => absurd hhop hn, fun _ => by rw [hex]; exact h0 rfl,
      fun hpn => ⟨hex.mpr (hpend hpn).1, (hpend hpn).2⟩⟩

/-- The records of `L'` are those of `L` at level `l`. -/
def SameAt (L' L : LV) (l : Nat) : Prop :=
  L'.act l = L.act l ∧ L'.task l = L.task l ∧ L'.prevTask l = L.prevTask l ∧
  L'.part l = L.part l ∧ L'.prevPart l = L.prevPart l ∧ L'.exc l = L.exc l

/-- `lemma_pop`: the unwind of the topmost record, which is not the
executing Job's, leaves a chain; the Partition taken as executing is the
one that record preempted. -/
theorem ChainOk.pop {v : KV} {L L' : LV} {cp lt : Nat} (hk : KOk v) (h : ChainOk v L cp)
    (hex : Extra v L)
    (htop : IsTop L lt) (hother : ∀ l, l ≠ lt → SameAt L' L l) (hoff : L'.act lt = false) :
    ChainOk v L' (L.prevPart lt) := by
  obtain ⟨c1, c2, c3, c4, c5⟩ := h
  obtain ⟨ta, tm⟩ := htop
  have hgt : topLevel v < lt := by
    obtain ⟨l, ha, hl⟩ := hex
    by_contra hle
    have := tm l (by omega)
    rw [ha] at this; cases this
  -- an active level of `L'` is an active level of `L` below `lt`
  have hact : ∀ l, L'.act l = true → L.act l = true ∧ l < lt := by
    intro l ha
    have hne : l ≠ lt := by intro e; rw [e, hoff] at ha; cases ha
    have hla : L.act l = true := by rw [← (hother l hne).1]; exact ha
    refine ⟨hla, ?_⟩
    by_contra hge
    have := tm l (by omega)
    rw [hla] at this; cases this
  have hsame : ∀ l, l < lt → L'.act l = L.act l := fun l hl => (hother l (by omega)).1
  refine ⟨fun l ha => ?_, fun i hi => ?_, fun l ha hle => ?_, fun hnone => ?_, fun l ht => ?_⟩
  · obtain ⟨hla, hlt⟩ := hact l ha
    obtain ⟨r1, r2, r3, r4, r5, r6, r7, r8, r9, r10⟩ := c1 l hla
    obtain ⟨_, s2, s3, s4, s5, s6⟩ := hother l (by omega)
    refine ⟨r1, r2, by rw [s2]; exact r3, by rw [s2]; exact r4, by rw [s6]; exact r5,
      by rw [s6]; exact r6, by rw [s4, s2]; exact r7, by rw [s4]; exact r8, ?_, ?_⟩
    · intro hb
      rw [s3, s5]
      exact r9 (fun j hj => by rw [← hsame j (by omega)]; exact hb j hj)
    · intro j hj hja hbetween
      rw [s3, s5, (hother j (by omega)).2.1]
      exact r10 j hj (by rw [← hsame j (by omega)]; exact hja)
        (fun j' h1 h2 => by rw [← hsame j' (by omega)]; exact hbetween j' h1 h2)
  · obtain ⟨a1, a2⟩ := c2 i hi
    have hle := stack_le_top hk hi
    have hne : v.prio (v.stack i) ≠ lt := by omega
    obtain ⟨s1, s2, _⟩ := hother _ hne
    exact ⟨by rw [s1]; exact a1, by rw [s2]; exact a2⟩
  · obtain ⟨hla, _⟩ := hact l ha
    exact c3 l hla hle
  · -- `lt` was the only active level
    have hb : ∀ j, j < lt → L.act j = false := fun j hj => by rw [← hsame j hj]; exact hnone j
    exact ((c1 lt ta).2.2.2.2.2.2.2.2.1 hb).2
  · obtain ⟨la, lm⟩ := ht
    obtain ⟨hla, hlt⟩ := hact l la
    have := (c1 lt ta).2.2.2.2.2.2.2.2.2 l hlt hla
      (fun j' h1 h2 => by rw [← hsame j' h2]; exact lm j' h1)
    rw [(hother l (by omega)).2.1]
    exact this.2

/-- `lemma_push`: a dispatch from a state with nothing to unwind. The
Kernel's stack gains Task `t`; its level's record is written with the
executing Job as the one preempted and the Partition taken as executing as
the one preempted; every other record is as it was. Then the records are a
chain again, with nothing to unwind, and `t`'s Partition is the one taken
as executing. -/
theorem ChainOk.push {v v0 : KV} {L L' : LV} {cp t level : Nat} (hk : KOk v)
    (h : ChainOk v0 L cp) (hne : ¬ Extra v0 L)
    (hd : v.depth = v0.depth + 1) (hst : ∀ i, i < v0.depth → v.stack i = v0.stack i)
    (hpr : v.prio = v0.prio) (hpa : v.part = v0.part)
    (ht : v.stack v0.depth = t) (hlev : level = v.prio t)
    (hother : ∀ l, l ≠ level → SameAt L' L l) (hact : L'.act level = true)
    (htask : L'.task level = t)
    (hprev : L'.prevTask level = if v0.depth = 0 then 255 else v0.stack (v0.depth - 1))
    (hexc0 : L'.exc level ≠ 0) (hexc1 : L'.exc level ≤ 511) (hpart : L'.part level = v.part t)
    (hprevp : L'.prevPart level = cp) :
    ChainOk v L' (v.part t) ∧ ¬ Extra v L' := by
  obtain ⟨c1, c2, c3, c4, c5⟩ := h
  have kt := hk.2 v0.depth (by omega)
  rw [ht] at kt
  obtain ⟨kt1, kt2, kt3, kt4⟩ := kt
  rw [← hlev] at kt2 kt3
  have hstk : ∀ i, i < v0.depth → v0.prio (v0.stack i) < level := by
    intro i hi
    have := hk.1 i v0.depth hi (by omega)
    rw [ht, ← hlev, hst i hi, hpr] at this
    exact this
  have hlow : ∀ l, L.act l = true → l < level := by
    intro l ha
    have hle : l ≤ topLevel v0 := by
      by_contra hgt
      exact hne ⟨l, ha, by omega⟩
    obtain ⟨i, hi, he⟩ := c3 l ha hle
    rw [he]; exact hstk i hi
  have hnl : L.act level = false := by
    cases hh : L.act level
    · rfl
    · have := hlow level hh; omega
  have hsame : ∀ l, l ≠ level → L'.act l = L.act l := fun l hl => (hother l hl).1
  have htl : topLevel v = level := by
    unfold topLevel
    have hz : v.depth ≠ 0 := by omega
    simp only [hz, ite_false]
    rw [show v.depth - 1 = v0.depth by omega, ht, hlev]
  have hold : ∀ l, L'.act l = true → l ≠ level → L.act l = true ∧ l < level := by
    intro l ha hl
    have : L.act l = true := by rw [← hsame l hl]; exact ha
    exact ⟨this, hlow l this⟩
  refine ⟨⟨fun l ha => ?_, fun i hi => ?_, fun l ha hle => ?_, fun hnone => ?_, fun l htop => ?_⟩,
    fun hex => ?_⟩
  · by_cases hl : l = level
    · subst hl
      refine ⟨kt2, kt3, by rw [htask]; exact kt1, by rw [htask]; exact hlev.symm, hexc0, hexc1,
        by rw [hpart, htask], by rw [hpart]; omega, fun hb => ?_, fun j hj hja hbetween => ?_⟩
      · -- no record below: the stack was empty and nothing was executing
        have hnone : ∀ l, L.act l = false := by
          intro l
          cases hh : L.act l
          · rfl
          · have hl := hlow l hh
            have := hb l hl
            rw [hsame l (by omega), hh] at this
            cases this
        have hd0 : v0.depth = 0 := by
          by_contra hz
          have := (c2 (v0.depth - 1) (by omega)).1
          rw [hnone] at this; cases this
        rw [hprev, hprevp, c4 hnone]
        simp [hd0]
      · -- the record below is the executing Job's
        have hja' : L.act j = true := by rw [← hsame j (by omega)]; exact hja
        have hjtop : IsTop L j := by
          refine ⟨hja', fun j' hj' => ?_⟩
          by_cases hlt : j' < l
          · rw [← hsame j' (by omega)]; exact hbetween j' hj' hlt
          · cases hh : L.act j'
            · rfl
            · have := hlow j' hh; omega
        have hjle : j ≤ topLevel v0 := by
          by_contra hgt
          exact hne ⟨j, hja', by omega⟩
        obtain ⟨i, hi, _⟩ := c3 j hja' hjle
        have hz : v0.depth ≠ 0 := by omega
        obtain ⟨a1, a2⟩ := c2 (v0.depth - 1) (by omega)
        have htl0 : topLevel v0 = v0.prio (v0.stack (v0.depth - 1)) := by
          simp [topLevel, hz]
        have hje : j = v0.prio (v0.stack (v0.depth - 1)) := by
          have h1 : v0.prio (v0.stack (v0.depth - 1)) ≤ j := by
            by_contra hgt
            have := hjtop.2 (v0.prio (v0.stack (v0.depth - 1))) (by omega)
            rw [a1] at this; cases this
          omega
        rw [hprev, hprevp, c5 j hjtop, (hother j (by omega)).2.1, hpa]
        simp only [hz, ite_false]
        rw [hje, a2]
        exact ⟨rfl, trivial⟩
    · obtain ⟨hla, hlt⟩ := hold l ha hl
      obtain ⟨r1, r2, r3, r4, r5, r6, r7, r8, r9, r10⟩ := c1 l hla
      obtain ⟨_, s2, s3, s4, s5, s6⟩ := hother l hl
      refine ⟨r1, r2, by rw [s2]; exact r3, by rw [s2, hpr]; exact r4, by rw [s6]; exact r5,
        by rw [s6]; exact r6, by rw [s4, s2, hpa]; exact r7, by rw [s4]; exact r8, ?_, ?_⟩
      · intro hb
        rw [s3, s5]
        exact r9 (fun j hj => by rw [← hsame j (by omega)]; exact hb j hj)
      · intro j hj hja hbetween
        rw [s3, s5, (hother j (by omega)).2.1, hpa]
        exact r10 j hj (by rw [← hsame j (by omega)]; exact hja)
          (fun j' h1 h2 => by rw [← hsame j' (by omega)]; exact hbetween j' h1 h2)
  · by_cases hi0 : i < v0.depth
    · have hlt := hstk i hi0
      obtain ⟨a1, a2⟩ := c2 i hi0
      rw [hst i hi0, hpr]
      obtain ⟨s1, s2, _⟩ := hother (v0.prio (v0.stack i)) (by omega)
      exact ⟨by rw [s1]; exact a1, by rw [s2]; exact a2⟩
    · have hie : i = v0.depth := by omega
      rw [hie, ht, ← hlev]
      exact ⟨hact, htask⟩
  · by_cases hl : l = level
    · exact ⟨v0.depth, by omega, by rw [ht, hl, hlev]⟩
    · obtain ⟨hla, _⟩ := hold l ha hl
      have hle0 : l ≤ topLevel v0 := by
        by_contra hgt
        exact hne ⟨l, hla, by omega⟩
      obtain ⟨i, hi, he⟩ := c3 l hla hle0
      exact ⟨i, by omega, by rw [hst i hi, hpr]; exact he⟩
  · have := hnone level
    rw [hact] at this; cases this
  · obtain ⟨la, lm⟩ := htop
    have hl : l = level := by
      by_contra hne'
      obtain ⟨_, hlt⟩ := hold l la hne'
      have := lm level hlt
      rw [hact] at this; cases this
    rw [hl, htask]
  · obtain ⟨l, ha, hl⟩ := hex
    rw [htl] at hl
    obtain ⟨_, hlt⟩ := hold l ha (by omega)
    omega

/-! ## After an unwind: what is left of the chain

The four outcomes of `epilogue_core` once it has removed the topmost
record: no record is left (the idle activity resumes), the record below is
the executing Job's (it resumes), the record below is of a Job the Kernel
no longer holds (one more level to unwind), or the Job to resume was killed
and is ended at once (one more level to unwind, with a shorter stack). -/

/-- Below a level with an active level under it there is a greatest one. -/
theorem exists_pred (L : LV) : ∀ lt, (∃ j, j < lt ∧ L.act j = true) →
    ∃ j, j < lt ∧ L.act j = true ∧ ∀ j', j < j' → j' < lt → L.act j' = false := by
  intro lt
  induction lt with
  | zero => rintro ⟨j, hj, _⟩; omega
  | succ n ih =>
    rintro ⟨j, hj, ha⟩
    cases hn : L.act n
    · have hjn : j < n := by
        by_contra hge
        have : j = n := by omega
        rw [this, hn] at ha; cases ha
      obtain ⟨j0, h1, h2, h3⟩ := ih ⟨j, hjn, ha⟩
      refine ⟨j0, by omega, h2, fun j' a b => ?_⟩
      by_cases hb : j' = n
      · rw [hb]; exact hn
      · exact h3 j' a (by omega)
    · exact ⟨n, by omega, hn, fun j' a b => by omega⟩

/-- The state after the unwind of the topmost record `lt` of `L`. -/
structure Popped (v : KV) (L L' : LV) (cp lt : Nat) : Prop where
  hk : KOk v
  chain : ChainOk v L cp
  extra : Extra v L
  top : IsTop L lt
  other : ∀ l, l ≠ lt → SameAt L' L l
  off : L'.act lt = false

theorem Popped.chain' {v : KV} {L L' : LV} {cp lt : Nat} (p : Popped v L L' cp lt) :
    ChainOk v L' (L.prevPart lt) :=
  ChainOk.pop p.hk p.chain p.extra p.top p.other p.off

theorem Popped.act_lt {v : KV} {L L' : LV} {cp lt : Nat} (p : Popped v L L' cp lt) {l : Nat}
    (ha : L'.act l = true) : L.act l = true ∧ l < lt := by
  have hne : l ≠ lt := by intro e; rw [e, p.off] at ha; cases ha
  have hla : L.act l = true := by rw [← (p.other l hne).1]; exact ha
  refine ⟨hla, ?_⟩
  by_contra hge
  have := p.top.2 l (by omega)
  rw [hla] at this; cases this

/-- The removed record preempted the idle activity: no record is left. -/
theorem Popped.none {v : KV} {L L' : LV} {cp lt : Nat} (p : Popped v L L' cp lt)
    (h255 : L.prevTask lt = 255) : ∀ l, L'.act l = false := by
  intro l
  cases hl : L'.act l
  · rfl
  · obtain ⟨hla, hlt⟩ := p.act_lt hl
    obtain ⟨j, hj, hja, hb⟩ := exists_pred L lt ⟨l, hlt, hla⟩
    have r := p.chain.1 lt p.top.1
    have := (r.2.2.2.2.2.2.2.2.2 j hj hja hb).1
    have t := (p.chain.1 j hja).2.2.1
    omega

/-- The removed record preempted a Job: that Job's record is the topmost
one now, at the level of its Priority. -/
theorem Popped.pred {v : KV} {L L' : LV} {cp lt : Nat} (p : Popped v L L' cp lt)
    (hne : L.prevTask lt ≠ 255) :
    ∃ j, IsTop L' j ∧ L.act j = true ∧ j < lt ∧ L.task j = L.prevTask lt ∧
      v.prio (L.prevTask lt) = j ∧ L'.exc j = L.exc j := by
  have r := p.chain.1 lt p.top.1
  have hex : ∃ j, j < lt ∧ L.act j = true := by
    by_contra hno
    have := (r.2.2.2.2.2.2.2.2.1 (fun j hj => by
      cases hh : L.act j
      · rfl
      · exact absurd ⟨j, hj, hh⟩ hno)).1
    exact hne this
  obtain ⟨j, hj, hja, hb⟩ := exists_pred L lt hex
  have hpt := (r.2.2.2.2.2.2.2.2.2 j hj hja hb).1
  have hs := p.other j (by omega)
  refine ⟨j, ⟨by rw [hs.1]; exact hja, fun j' hj' => ?_⟩, hja, hj, hpt.symm, ?_, hs.2.2.2.2.2⟩
  · by_cases e : j' = lt
    · rw [e]; exact p.off
    · rw [(p.other j' e).1]
      by_cases hlt : j' < lt
      · exact hb j' hj' hlt
      · exact p.top.2 j' (by omega)
  · rw [hpt]; exact (p.chain.1 j hja).2.2.2.1

/-- No record is left: the invariant holds with nothing planned. -/
theorem Popped.idle {v : KV} {L L' : LV} {cp lt : Nat} (p : Popped v L L' cp lt)
    (h255 : L.prevTask lt = 255) : AInv v L' (L.prevPart lt) 0 0 := by
  have hn := p.none h255
  refine ⟨p.chain', fun h => absurd rfl h, fun _ => ⟨fun ⟨l, ha, _⟩ => ?_, fun h => absurd rfl h⟩,
    fun h => absurd rfl h⟩
  rw [hn l] at ha; cases ha

/-- The record below is the executing Job's: the invariant holds with
nothing planned, and that Job resumes. -/
theorem Popped.resume {v : KV} {L L' : LV} {cp lt : Nat} (p : Popped v L L' cp lt)
    (hne : L.prevTask lt ≠ 255) (hd : 0 < v.depth) (htop : v.stack (v.depth - 1) = L.prevTask lt) :
    AInv v L' (L.prevPart lt) 0 0 := by
  obtain ⟨j, hjt, hja, hjl, htask, hprio, _⟩ := p.pred hne
  have htl : topLevel v = j := by
    have hz : v.depth ≠ 0 := by omega
    simp only [topLevel, hz, ite_false]
    rw [htop, hprio]
  refine ⟨p.chain', fun h => absurd rfl h, fun _ => ⟨fun ⟨l, ha, hl⟩ => ?_, fun h => absurd rfl h⟩,
    fun h => absurd rfl h⟩
  rw [htl] at hl
  have := hjt.2 l hl
  rw [ha] at this; cases this

/-- The record below is of a Job the Kernel no longer holds as executing:
one more level to unwind, named by that record's exception number. -/
theorem Popped.next {v : KV} {L L' : LV} {cp lt : Nat} (p : Popped v L L' cp lt)
    (hne : L.prevTask lt ≠ 255)
    (hnot : v.depth = 0 ∨ v.stack (v.depth - 1) ≠ L.prevTask lt) :
    AInv v L' (L.prevPart lt) 0 (L'.exc (v.prio (L.prevTask lt))) := by
  obtain ⟨j, hjt, hja, hjl, htask, hprio, hexc⟩ := p.pred hne
  rw [hprio]
  have rj := p.chain.1 j hja
  have hgt : topLevel v < j := by
    by_cases hz : v.depth = 0
    · simp only [topLevel, hz, ite_true]; exact rj.1
    · by_contra hle
      obtain ⟨i, hi, he⟩ := p.chain.2.2.1 j hja (by omega)
      -- the executing Job's level is active in `L'` too, so it is at most `j`
      obtain ⟨a1, a2⟩ := p.chain.2.1 (v.depth - 1) (by omega)
      have htl : topLevel v = v.prio (v.stack (v.depth - 1)) := by simp [topLevel, hz]
      have hlt' : v.prio (v.stack (v.depth - 1)) < lt := by
        have := stack_le_top p.hk (i := v.depth - 1) (by omega)
        obtain ⟨l, ha, hl⟩ := p.extra
        by_contra hge
        have := p.top.2 l (by omega)
        rw [ha] at this; cases this
      have hle2 : v.prio (v.stack (v.depth - 1)) ≤ j := by
        by_contra hgt'
        have := hjt.2 (v.prio (v.stack (v.depth - 1))) (by omega)
        rw [(p.other _ (by omega)).1, a1] at this; cases this
      have hje : j = v.prio (v.stack (v.depth - 1)) := by omega
      have hie : i = v.depth - 1 := by
        by_contra hni
        have := p.hk.1 i (v.depth - 1) (by omega) (by omega)
        omega
      have h2 := (p.chain.2.1 i hi).2
      rw [← he, htask, hie] at h2
      rcases hnot with h0 | h1
      · exact hz h0
      · exact h1 h2.symm
  have hne0 : L'.exc j ≠ 0 := by rw [hexc]; exact rj.2.2.2.2.1
  exact ⟨p.chain', fun _ => ⟨⟨j, hjt.1, hgt⟩, j, hjt, rfl⟩, fun h => absurd h hne0,
    fun h => absurd rfl h⟩

/-- The Job to resume was killed and the Kernel ended it at once (its
stack is shorter by that Job at least): one more level to unwind, named by
that Job's record. -/
theorem Popped.killed {v v2 : KV} {L L' : LV} {cp lt : Nat} (p : Popped v L L' cp lt)
    (hne : L.prevTask lt ≠ 255) (hd : 0 < v.depth) (htop : v.stack (v.depth - 1) = L.prevTask lt)
    (hp : Prefix v2 v) (hd2 : v2.depth < v.depth) :
    AInv v2 L' (L.prevPart lt) 0 (L'.exc (v2.prio (L.prevTask lt))) := by
  obtain ⟨j, hjt, hja, hjl, htask, hprio, hexc⟩ := p.pred hne
  rw [hp.2.2.1, hprio]
  have rj := p.chain.1 j hja
  have hgt : topLevel v2 < j := by
    by_cases hz : v2.depth = 0
    · simp only [topLevel, hz, ite_true]; exact rj.1
    · simp only [topLevel, hz, ite_false]
      rw [hp.2.2.1, hp.2.1 _ (by omega), ← hprio, ← htop]
      exact p.hk.1 _ _ (by omega) (by omega)
  have hne0 : L'.exc j ≠ 0 := by rw [hexc]; exact rj.2.2.2.2.1
  exact ⟨p.chain'.prefix p.hk hp, fun _ => ⟨⟨j, hjt.1, hgt⟩, j, hjt, rfl⟩, fun h => absurd h hne0,
    fun h => absurd rfl h⟩

end Arch
