import Sched.Window
open Aeneas Aeneas.Std Result

namespace P18
open rsk_logic
open rsk_analyzer (ceilDiv)

variable {NT NR NL NP NS NE : Usize}

/-- The Tasks of Priority at least Task `i`'s, itself included. -/
def lvl (cfg : logic.Config NT NR NP) (i : Nat) : Finset Nat :=
  (Finset.range NT.val).filter (fun k => prio cfg i ≤ prio cfg k)

theorem mem_lvl {cfg : logic.Config NT NR NP} {i k : Nat} :
    k ∈ lvl cfg i ↔ k < NT.val ∧ prio cfg i ≤ prio cfg k := by
  simp [lvl]

theorem new_views {cfg : logic.Config NT NR NP} {s0 : logic.sched.Sched NT NR NL}
    (h : logic.sched.Sched.new NL cfg = ok s0) :
    (∀ k, k < NT.val → rel s0 k = 0) ∧ (∀ k, k < NT.val → job s0 k = 0) ∧
    s0.depth.val = 0 ∧ s0.ceiling.val = 0 ∧ s0.lock_len.val = 0 := by
  simp only [logic.sched.Sched.new] at h
  have h := ok_inj h
  subst h
  refine ⟨?_, ?_, rfl, ?_, rfl⟩
  · intro k hk; simp [rel, rel_none, hk]
  · intro k hk; simp [job, job_none, hk]
  · show (logic.IDLE_CEILING).val = 0
    rw [idle_ceiling]; rfl

theorem Wf.boot {cfg : logic.Config NT NR NP} {P : Params} {s0 : logic.sched.Sched NT NR NL}
    (h : logic.sched.Sched.new NL cfg = ok s0) :
    Wf cfg P ⟨s0, 0, fun _ => none, fun _ => 0, fun _ => 0, fun _ => 0⟩ := by
  obtain ⟨hrel, hjob, hdep, hceil, hlock⟩ := new_views h
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> dsimp only
  · omega
  · intro p hp; omega
  · intro p q _ hq; omega
  · intro k hk; left; exact hjob k hk
  · intro k hk; rw [hjob k hk]
    constructor
    · intro hx; omega
    · rintro ⟨p, hp, _⟩; omega
  · intro k hk; rw [hjob k hk]; omega
  · intro k hk; rw [hrel k hk]; omega
  · rw [hlock, hceil]; exact ⟨fun p hp => by omega, Or.inl rfl⟩
  · intro p hp; omega
  · intro p q _ hq; omega
  · intro p q hp; omega
  · intro p hp; omega

/-- The work Task `k`'s released Jobs still need. -/
def rem (st : St NT NR NL) (k : Nat) : Nat := st.pend k + st.run k

/-- A lock entry that blocks Task `i`'s level: its Resource's Ceiling is at
least Task `i`'s Priority and its holder's Priority is lower. -/
def blocking (cfg : logic.Config NT NR NP) (i : Nat) (s : logic.sched.Sched NT NR NL)
    (p : Nat) : Prop :=
  p < s.lock_len.val ∧ prio cfg i ≤ ceil cfg (lres s p) ∧
  prio cfg (stk s (ldep s p)) < prio cfg i

/-- `β` bounds the execution the first blocking entry's Critical_Section
may still take (it is the outermost one: the others are nested in it). -/
def Blk (cfg : logic.Config NT NR NP) (i : Nat) (s : logic.sched.Sched NT NR NL)
    (allow : Nat → Nat) (β : Nat) : Prop :=
  ∀ p, blocking cfg i s p →
    ∃ p0, blocking cfg i s p0 ∧ (∀ q, blocking cfg i s q → p0 ≤ q) ∧ allow p0 ≤ β

/-- The window invariant of Task `i`'s level. -/
def LInv (cfg : logic.Config NT NR NP) (P : Params) (R i : Nat) (st : St NT NR NL) : Prop :=
  ∃ s0 β, s0 ≤ st.now ∧ Phi P R (lvl cfg i) s0 β st.now st.last (rem st) ≤ R ∧
    (0 < rem st i → ∃ r, st.last i = some r ∧ s0 ≤ r) ∧ Blk cfg i st.s st.allow β

/-- How the blocking entries change in a step that keeps, lowers by `δ`,
or removes the first one. -/
theorem Blk.trans {cfg : logic.Config NT NR NP} {i : Nat} {s s' : logic.sched.Sched NT NR NL}
    {allow allow' : Nat → Nat} {β δ : Nat}
    (h1 : ∀ q, blocking cfg i s' q → ∃ q0, q0 ≤ q ∧ blocking cfg i s q0)
    (h2 : ∀ p0 q, blocking cfg i s p0 → blocking cfg i s' q → p0 ≤ q → blocking cfg i s' p0)
    (hal : ∀ p0, blocking cfg i s p0 → allow' p0 + δ ≤ allow p0)
    (h : Blk cfg i s allow β) : Blk cfg i s' allow' (β - δ) := by
  intro p hp
  obtain ⟨q0, hq0, hb0⟩ := h1 p hp
  obtain ⟨p0, hbp0, hmin, hle⟩ := h q0 hb0
  refine ⟨p0, h2 p0 p hbp0 hp (Nat.le_trans (hmin q0 hb0) hq0), ?_, ?_⟩
  · intro q hq
    obtain ⟨q1, hq1, hb1⟩ := h1 q hq
    exact Nat.le_trans (hmin q1 hb1) hq1
  · have := hal p0 hbp0
    omega

/-- A bound within `B` for the blocking entries of any state of a run. -/
theorem fresh_blk {cfg : logic.Config NT NR NP} {P : Params} {i B : Nat} {st : St NT NR NL}
    (w : Wf cfg P st)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i → prio cfg i ≤ ceil cfg r → P.L k r ≤ B) :
    ∃ β, β ≤ B ∧ Blk cfg i st.s st.allow β := by
  by_cases h : ∃ p, blocking cfg i st.s p
  · classical
    have hp0 := Nat.find_spec h
    refine ⟨st.allow (Nat.find h), ?_, fun p _ =>
      ⟨Nat.find h, hp0, fun q hq => Nat.find_min' h hq, Nat.le_refl _⟩⟩
    obtain ⟨hl, hc, hp⟩ := hp0
    exact Nat.le_trans (w.allow_le _ hl) (hB _ _ (w.stk_lt _ (w.ldep_lt _ hl)) hp hc)
  · exact ⟨0, Nat.zero_le _, fun p hp => absurd ⟨p, hp⟩ h⟩

/-- When the level has no work, the invariant holds with a window that
starts now. -/
theorem LInv.of_idle {cfg : logic.Config NT NR NP} {P : Params} {i R B : Nat} {st : St NT NR NL}
    (w : Wf cfg P st)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i → prio cfg i ≤ ceil cfg r → P.L k r ≤ B)
    (hFix : B + ∑ k ∈ lvl cfg i, P.C k * ceilDiv R (P.T k) ≤ R)
    (h0 : ∀ k ∈ lvl cfg i, rem st k = 0) :
    ∃ β, Phi P R (lvl cfg i) st.now β st.now st.last (rem st) ≤ R ∧ Blk cfg i st.s st.allow β := by
  obtain ⟨β, hβ, hblk⟩ := fresh_blk w hB
  exact ⟨β, idle_anchor hFix hβ _ _ _ h0, hblk⟩

/-- While the level has work, the test accepts no Task, and the Job on top
of the stack is not of the level (or there is none), a lock entry blocks
the level, and every blocking entry is held by the Job on top. -/
theorem busy_block {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL} (w : Wf cfg P st)
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    {b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
    {i k : Nat} (hk : k ∈ lvl cfg i) (hpos : 0 < rem st k)
    (hnot : st.s.depth.val = 0 ∨ prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg i) :
    0 < st.s.depth.val ∧ (∃ p, blocking cfg i st.s p) ∧
    ∀ q, blocking cfg i st.s q → ldep st.s q = st.s.depth.val - 1 := by
  obtain ⟨hkn, hki⟩ := mem_lvl.mp hk
  have below_top : ∀ p, p < st.s.depth.val →
      prio cfg (stk st.s p) ≤ prio cfg (stk st.s (st.s.depth.val - 1)) := by
    intro p hp
    by_cases hpt : p = st.s.depth.val - 1
    · rw [hpt]
    · exact Nat.le_of_lt (w.sorted p (st.s.depth.val - 1) (by omega) (by omega))
  have hj0 : job st.s k = 0 := by
    rcases w.job01 k hkn with h | h
    · exact h
    · exfalso
      obtain ⟨p, hp, hpk⟩ := (w.on_stack k hkn).mp h
      have := below_top p hp
      rw [hpk] at this
      rcases hnot with h0 | h1 <;> omega
  have hrun : st.run k = 0 := by
    by_contra hc
    have := (w.run_iff k hkn).mp (by omega)
    omega
  have hrel := (w.pend_iff k hkn).mp (by unfold rem at hpos; omega)
  let t : U8 := UScalar.ofNatCore k (by simp [UScalarTy.numBits]; omega)
  have htv : t.val = k := UScalar.ofNatCore_val_eq _
  have hnot' : ¬ Elig cfg st.s t.val := by
    intro he
    have := (eligible_ok hm (hne t)).mpr he
    cases this
  rw [htv, elig_views] at hnot'
  have hceil : prio cfg k ≤ st.s.ceiling.val := by
    by_contra hc
    apply hnot'
    refine ⟨hkn, hrel, hj0, by omega, ?_⟩
    rcases hnot with h0 | h1
    · left; exact h0
    · right; omega
  have hp := hprio k hkn
  obtain ⟨p, hpl, hpc⟩ : ∃ p, p < st.s.lock_len.val ∧ st.s.ceiling.val = ceil cfg (lres st.s p) := by
    rcases w.ceil_is.2 with h | h
    · omega
    · exact h
  have hld := w.ldep_lt p hpl
  have hdpos : 0 < st.s.depth.val := by omega
  have htop : prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg i := by
    rcases hnot with h0 | h1
    · omega
    · exact h1
  refine ⟨hdpos, ⟨p, hpl, by omega, ?_⟩, ?_⟩
  · have := below_top (ldep st.s p) hld
    omega
  · intro q ⟨hql, hqc, _⟩
    by_contra hne'
    have hq := w.ldep_lt q hql
    have := w.srp q (st.s.depth.val - 1) hql (by omega) (by omega)
    omega

/-- The window invariant of Task `i`'s level is kept by every transition. -/
theorem level_step {cfg : logic.Config NT NR NP} {P : Params} {i R B : Nat}
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    (hi : i ∈ lvl cfg i) (hR : 0 < R) (hT : ∀ k ∈ lvl cfg i, 0 < P.T k)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i → prio cfg i ≤ ceil cfg r → P.L k r ≤ B)
    (hFix : B + ∑ k ∈ lvl cfg i, P.C k * ceilDiv R (P.T k) ≤ R)
    {st st' : St NT NR NL} (w : Wf cfg P st) (hs : Step cfg P st st')
    (h : LInv cfg P R i st) : LInv cfg P R i st' := by
  have w' := w.step hs
  by_cases hidle : ∀ k ∈ lvl cfg i, rem st' k = 0
  · obtain ⟨β, hphi, hblk⟩ := LInv.of_idle w' hB hFix hidle
    exact ⟨st'.now, β, Nat.le_refl _, hphi, fun hx => by rw [hidle i hi] at hx; omega, hblk⟩
  have hbusy' : ∃ k ∈ lvl cfg i, 0 < rem st' k := by
    by_contra hc
    apply hidle
    intro k hk
    by_contra hk0
    exact hc ⟨k, hk, by omega⟩
  clear hidle w'
  cases hs with
  | release t d s' hd1 hd2 hsp hs =>
    obtain ⟨ht, hrel, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := make_pending_views hs
    have hanchor : ∃ s0 β, s0 ≤ st.now ∧ st.now < s0 + R ∧
        Phi P R (lvl cfg i) s0 β st.now st.last (rem st) ≤ R ∧
        (0 < rem st i → ∃ r, st.last i = some r ∧ s0 ≤ r) ∧ Blk cfg i st.s st.allow β := by
      by_cases hb : ∃ k ∈ lvl cfg i, 0 < rem st k
      · obtain ⟨s0, β, a, b, c, e⟩ := h
        obtain ⟨k, hk, hpos⟩ := hb
        exact ⟨s0, β, a, Phi.busy_lt hk hpos b, b, c, e⟩
      · have h0 : ∀ k ∈ lvl cfg i, rem st k = 0 := by
          intro k hk
          by_contra hk0
          exact hb ⟨k, hk, by omega⟩
        obtain ⟨β, hphi, hblk⟩ := LInv.of_idle w hB hFix h0
        exact ⟨st.now, β, Nat.le_refl _, by omega, hphi,
          fun hx => by rw [h0 i hi] at hx; omega, hblk⟩
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hanchor
    have hbl : ∀ q, blocking cfg i s' q ↔ blocking cfg i st.s q := by
      intro q; unfold blocking; rw [hll, hlr, hld, hstk]
    have hblk' : Blk cfg i s' st.allow β := by
      have := Blk.trans (δ := 0) (fun q hq => ⟨q, Nat.le_refl _, (hbl q).mp hq⟩)
        (fun p0 _ hp0 _ _ => (hbl p0).mpr hp0) (fun _ _ => Nat.le_refl _) hblk
      simpa using this
    refine ⟨s0, β, hs0, ?_, ?_, hblk'⟩ <;> dsimp only
    · by_cases ht : t.val ∈ lvl cfg i
      · have e : rem { st with
            s := s', last := Function.update st.last t.val (some st.now),
            pend := Function.update st.pend t.val (st.pend t.val + d) }
            = Function.update (rem st) t.val (rem st t.val + d) := by
          funext k
          by_cases hk : k = t.val
          · subst hk; simp [rem, Function.update]; omega
          · simp [rem, Function.update, hk]
        rw [e]
        exact Nat.le_trans (Phi.release ht (hT _ ht) hd2 hsp hlt) hphi
      · rw [Phi.congr (last := st.last) (rm := rem st)]
        · exact hphi
        · intro k hk
          have hkt : k ≠ t.val := fun hc => ht (hc ▸ hk)
          simp [rem, Function.update, hkt]
        · intro k hk
          have hkt : k ≠ t.val := fun hc => ht (hc ▸ hk)
          simp [Function.update, hkt]
    · intro hx
      by_cases hit : i = t.val
      · exact ⟨st.now, by simp [hit], hs0⟩
      · have e : rem { st with
            s := s', last := Function.update st.last t.val (some st.now),
            pend := Function.update st.pend t.val (st.pend t.val + d) } i = rem st i := by
          simp [rem, Function.update, hit]
        rw [e] at hx
        rw [Function.update_of_ne hit]
        exact hthird hx
  | dispatch b t s' hm he hs =>
    obtain ⟨htn, _, hj0, _, _⟩ := elig_views.mp ((eligible_ok hm he).mp rfl)
    obtain ⟨_, _, hrel, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := start_views hs
    have hrun : st.run t.val = 0 := by
      by_contra hc
      have := (w.run_iff _ htn).mp (by omega)
      omega
    have hrem : ∀ k, rem { st with
        s := s', run := Function.update st.run t.val (st.pend t.val),
        pend := Function.update st.pend t.val 0 } k = rem st k := by
      intro k
      by_cases hkt : k = t.val
      · subst hkt; simp [rem, Function.update, hrun]
      · simp [rem, Function.update, hkt]
    obtain ⟨s0, β, hs0, hphi, hthird, hblk⟩ := h
    have hstk' : ∀ q, q < st.s.lock_len.val → stk s' (ldep st.s q) = stk st.s (ldep st.s q) := by
      intro q hq
      rw [hstk]
      have := w.ldep_lt q hq
      simp only [show ldep st.s q ≠ st.s.depth.val by omega, ite_false]
    have hbl : ∀ q, blocking cfg i s' q ↔ blocking cfg i st.s q := by
      intro q; unfold blocking; rw [hll, hlr, hld]
      constructor
      · rintro ⟨hq, hc, hp⟩; exact ⟨hq, hc, by rw [← hstk' q hq]; exact hp⟩
      · rintro ⟨hq, hc, hp⟩; exact ⟨hq, hc, by rw [hstk' q hq]; exact hp⟩
    have hblk' : Blk cfg i s' st.allow β := by
      have := Blk.trans (δ := 0) (fun q hq => ⟨q, Nat.le_refl _, (hbl q).mp hq⟩)
        (fun p0 _ hp0 _ _ => (hbl p0).mpr hp0) (fun _ _ => Nat.le_refl _) hblk
      simpa using this
    refine ⟨s0, β, hs0, ?_, ?_, hblk'⟩ <;> dsimp only
    · rw [Phi.congr (last := st.last) (rm := rem st) (fun k _ => hrem k) (fun _ _ => rfl)]
      exact hphi
    · intro hx; rw [hrem] at hx; exact hthird hx
  | lock b hm hne r a s' ha hs =>
    obtain ⟨⟨hrel, hjob, hstk, hdep⟩, hd1, hl, hll, hlr, hld, hlc⟩ := lock_views hs
    obtain ⟨k, hk, hpos⟩ := hbusy'
    have hpos : 0 < rem st k := hpos
    obtain ⟨s0, β, hs0, hphi, hthird, hblk⟩ := h
    have hold : ∀ q, q < st.s.lock_len.val → (blocking cfg i s' q ↔ blocking cfg i st.s q) := by
      intro q hq
      unfold blocking
      rw [hll, hlr, hld]
      simp only [show q ≠ st.s.lock_len.val by omega, ite_false, hstk]
      constructor
      · rintro ⟨_, hc, hp⟩; exact ⟨hq, hc, hp⟩
      · rintro ⟨_, hc, hp⟩; exact ⟨by omega, hc, hp⟩
    have hblk' : Blk cfg i s' (Function.update st.allow st.s.lock_len.val a) β := by
      have := Blk.trans (δ := 0) (s' := s') (allow' := Function.update st.allow st.s.lock_len.val a)
        ?_ ?_ ?_ hblk
      · simpa using this
      · intro q hq
        by_cases hql : q = st.s.lock_len.val
        · have hq' := hq
          unfold blocking at hq'
          rw [hld, hstk] at hq'
          simp only [hql, ite_true] at hq'
          obtain ⟨_, ⟨p, hp⟩, _⟩ := busy_block w hNT hprio hm hne hk hpos (Or.inr hq'.2.2)
          exact ⟨p, by rw [hql]; exact Nat.le_of_lt hp.1, hp⟩
        · have hq1 := hq.1
          rw [hll] at hq1
          exact ⟨q, Nat.le_refl _, (hold q (by omega)).mp hq⟩
      · intro p0 _ hp0 _ _
        exact (hold p0 hp0.1).mpr hp0
      · intro p0 hp0
        rw [Function.update_of_ne (by have := hp0.1; omega)]
        exact Nat.le_refl _
    exact ⟨s0, β, hs0, hphi, hthird, hblk'⟩
  | unlock b hm hne r s' hs =>
    obtain ⟨⟨hrel, hjob, hstk, hdep⟩, hl1, hd1, _, _, hll, hlr, hld, hc⟩ := unlock_views hs
    obtain ⟨s0, β, hs0, hphi, hthird, hblk⟩ := h
    have hbl : ∀ q, blocking cfg i s' q ↔ (blocking cfg i st.s q ∧ q < st.s.lock_len.val - 1) := by
      intro q
      unfold blocking
      rw [hll, hlr, hld, hstk]
      constructor
      · rintro ⟨hq, hc, hp⟩; exact ⟨⟨by omega, hc, hp⟩, hq⟩
      · rintro ⟨⟨_, hc, hp⟩, hq⟩; exact ⟨hq, hc, hp⟩
    have hblk' : Blk cfg i s' st.allow β := by
      have := Blk.trans (δ := 0) (fun q hq => ⟨q, Nat.le_refl _, ((hbl q).mp hq).1⟩)
        (fun p0 q hp0 hq hle => (hbl p0).mpr ⟨hp0, by have := ((hbl q).mp hq).2; omega⟩)
        (fun _ _ => Nat.le_refl _) hblk
      simpa using this
    exact ⟨s0, β, hs0, hphi, hthird, hblk'⟩
  | idle b hm hne hd =>
    obtain ⟨k, hk, hpos⟩ := hbusy'
    have hpos : 0 < rem st k := hpos
    have := (busy_block w hNT hprio hm hne hk hpos (Or.inl hd)).1
    omega
  | exec b hm hne hd hr hal =>
    have htopn : stk st.s (st.s.depth.val - 1) < NT.val := w.stk_lt _ (by omega)
    have hle : ∀ k, rem { st with
        now := st.now + 1,
        run := Function.update st.run (stk st.s (st.s.depth.val - 1))
          (st.run (stk st.s (st.s.depth.val - 1)) - 1),
        allow := fun p => if topEntry st.s p then st.allow p - 1 else st.allow p } k ≤ rem st k := by
      intro k
      by_cases hkt : k = stk st.s (st.s.depth.val - 1)
      · subst hkt; simp [rem, Function.update]
      · simp [rem, Function.update, hkt]
    obtain ⟨k, hk, hpos'⟩ := hbusy'
    have hpos : 0 < rem st k := Nat.lt_of_lt_of_le hpos' (hle k)
    obtain ⟨s0, β, hs0, hphi, hthird, hblk⟩ := h
    by_cases htop : prio cfg i ≤ prio cfg (stk st.s (st.s.depth.val - 1))
    · refine ⟨s0, β, by dsimp only; omega, ?_, ?_, ?_⟩ <;> dsimp only
      · refine Nat.le_trans (Phi.tick hs0 (fun k _ => hle k) (Or.inl ⟨⟨_, mem_lvl.mpr ⟨htopn, htop⟩, ?_⟩,
          Nat.le_refl _⟩)) hphi
        simp [rem, Function.update]; omega
      · intro hx; exact hthird (Nat.lt_of_lt_of_le hx (hle i))
      · have := Blk.trans (δ := 0) (s' := st.s)
          (allow' := fun p => if topEntry st.s p then st.allow p - 1 else st.allow p)
          (fun q hq => ⟨q, Nat.le_refl _, hq⟩) (fun p0 _ hp0 _ _ => hp0) ?_ hblk
        · simpa using this
        · intro p0 hp0
          have hnt : ¬ topEntry st.s p0 := by
            rintro ⟨_, hd0⟩
            have := hp0.2.2
            rw [hd0] at this
            omega
          simp only [hnt, ite_false]; exact Nat.le_refl _
    · have hnot : prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg i := by omega
      obtain ⟨_, ⟨p, hp⟩, hall⟩ := busy_block w hNT hprio hm hne hk hpos (Or.inr hnot)
      obtain ⟨p0, hbp0, hmin, hle0⟩ := hblk p hp
      have hpos0 : 0 < st.allow p0 := hal p0 ⟨hbp0.1, hall p0 hbp0⟩
      refine ⟨s0, β - 1, by dsimp only; omega, ?_, ?_, ?_⟩ <;> dsimp only
      · exact Nat.le_trans (Phi.tick hs0 (fun k _ => hle k) (Or.inr (by omega))) hphi
      · intro hx; exact hthird (Nat.lt_of_lt_of_le hx (hle i))
      · refine Blk.trans (δ := 1) (s' := st.s) (fun q hq => ⟨q, Nat.le_refl _, hq⟩)
          (fun p0 _ hp0 _ _ => hp0) ?_ hblk
        intro q hq
        have ht : topEntry st.s q := ⟨hq.1, hall q hq⟩
        have := hal q ht
        simp only [ht, ite_true]; omega
  | finish b hm hne r s' hd hr hal hs =>
    obtain ⟨h1, h2, h3, hrel, hjob, hstk, hdep, hlr, hld, hn, hpop, hkeep, hc⟩ :=
      finish_top_views hs
    have hrunpos : 0 < st.run (stk st.s (st.s.depth.val - 1)) :=
      (w.run_iff _ h3).mpr ((w.on_stack _ h3).mpr ⟨_, by omega, rfl⟩)
    have hle : ∀ k, rem { st with
        s := s', now := st.now + 1,
        run := Function.update st.run (stk st.s (st.s.depth.val - 1)) 0 } k ≤ rem st k := by
      intro k
      by_cases hkt : k = stk st.s (st.s.depth.val - 1)
      · subst hkt; simp [rem, Function.update]
      · simp [rem, Function.update, hkt]
    have hbelow : ∀ p, p < s'.lock_len.val → ldep st.s p < st.s.depth.val - 1 := by
      intro p hp
      rcases hkeep with h0 | hne'
      · omega
      · have m := w.ldep_mono p (s'.lock_len.val - 1) (by omega) (by omega)
        have l := w.ldep_lt (s'.lock_len.val - 1) (by omega)
        omega
    have hbl : ∀ q, blocking cfg i s' q ↔ (blocking cfg i st.s q ∧ q < s'.lock_len.val) := by
      intro q
      unfold blocking
      rw [hlr, hld, hstk]
      constructor
      · rintro ⟨hq, hc, hp⟩; exact ⟨⟨by omega, hc, hp⟩, hq⟩
      · rintro ⟨⟨_, hc, hp⟩, hq⟩; exact ⟨hq, hc, hp⟩
    obtain ⟨k, hk, hpos'⟩ := hbusy'
    have hpos : 0 < rem st k := Nat.lt_of_lt_of_le hpos' (hle k)
    obtain ⟨s0, β, hs0, hphi, hthird, hblk⟩ := h
    by_cases htop : prio cfg i ≤ prio cfg (stk st.s (st.s.depth.val - 1))
    · refine ⟨s0, β, by dsimp only; omega, ?_, ?_, ?_⟩ <;> dsimp only
      · refine Nat.le_trans (Phi.tick hs0 (fun k _ => hle k) (Or.inl ⟨⟨_, mem_lvl.mpr ⟨h3, htop⟩, ?_⟩,
          Nat.le_refl _⟩)) hphi
        simp [rem, Function.update]; omega
      · intro hx; exact hthird (Nat.lt_of_lt_of_le hx (hle i))
      · have := Blk.trans (δ := 0) (fun q hq => ⟨q, Nat.le_refl _, ((hbl q).mp hq).1⟩)
          (fun p0 q hp0 hq hle => (hbl p0).mpr ⟨hp0, by have := ((hbl q).mp hq).2; omega⟩)
          (fun _ _ => Nat.le_refl _) hblk
        simpa using this
    · have hnot : prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg i := by omega
      obtain ⟨_, ⟨p, hp⟩, hall⟩ := busy_block w hNT hprio hm hne hk hpos (Or.inr hnot)
      obtain ⟨p0, hbp0, hmin, hle0⟩ := hblk p hp
      have hpos0 : 0 < st.allow p0 := hal p0 ⟨hbp0.1, hall p0 hbp0⟩
      refine ⟨s0, β - 1, by dsimp only; omega, ?_, ?_, ?_⟩ <;> dsimp only
      · exact Nat.le_trans (Phi.tick hs0 (fun k _ => hle k) (Or.inr (by omega))) hphi
      · intro hx; exact hthird (Nat.lt_of_lt_of_le hx (hle i))
      · intro q hq
        exfalso
        obtain ⟨hbq, hqn⟩ := (hbl q).mp hq
        have := hall q hbq
        have := hbelow q hqn
        omega

/-- Both invariants hold in every state a run reaches. -/
theorem reach_inv {cfg : logic.Config NT NR NP} {P : Params} {i R B : Nat}
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    (hi : i < NT.val) (hR : 0 < R) (hT : ∀ k ∈ lvl cfg i, 0 < P.T k)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i → prio cfg i ≤ ceil cfg r → P.L k r ≤ B)
    (hFix : B + ∑ k ∈ lvl cfg i, P.C k * ceilDiv R (P.T k) ≤ R)
    {st : St NT NR NL} (hreach : Reach cfg P st) :
    Wf cfg P st ∧ LInv cfg P R i st := by
  have hil : i ∈ lvl cfg i := mem_lvl.mpr ⟨hi, Nat.le_refl _⟩
  induction hreach with
  | boot s0 h =>
    have w : Wf cfg P ⟨s0, 0, fun _ => none, fun _ => 0, fun _ => 0, fun _ => 0⟩ := Wf.boot h
    obtain ⟨β, hphi, hblk⟩ := LInv.of_idle w hB hFix (fun _ _ => rfl)
    exact ⟨w, 0, β, Nat.le_refl _, hphi, fun hx => by simp [rem] at hx, hblk⟩
  | step st st' _ hs ih =>
    exact ⟨ih.1.step hs, level_step hNT hprio hil hR hT hB hFix ih.1 hs ih.2⟩

/-- **Stage 2** (Resources and SRP blocking; no Kernel overhead, no
jitter). In every run of the Kernel's scheduler, every Job of Task `i`
completes within `R` of its Release, for any `R > 0` with
`B + Σ_{k : P_k ≥ P_i} C_k · ⌈R / T_k⌉ ≤ R`, where `B` bounds every
Critical_Section of a Task of lower Priority on a Resource whose Ceiling
is at least Task `i`'s Priority: in every state the run reaches in which
Task `i` has unfinished work, its last Release was less than `R` ago.
(Time is counted in units; a Job that completes in the unit starting at
`now` completes at `now + 1 ≤ r + R`.) -/
theorem stage2 {cfg : logic.Config NT NR NP} {P : Params} {i R B : Nat}
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    (hi : i < NT.val) (hR : 0 < R) (hT : ∀ k ∈ lvl cfg i, 0 < P.T k)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i → prio cfg i ≤ ceil cfg r → P.L k r ≤ B)
    (hFix : B + ∑ k ∈ lvl cfg i, P.C k * ceilDiv R (P.T k) ≤ R)
    {st : St NT NR NL} (hreach : Reach cfg P st) (hpos : 0 < st.pend i + st.run i) :
    ∃ r, st.last i = some r ∧ st.now < r + R := by
  have hil : i ∈ lvl cfg i := mem_lvl.mpr ⟨hi, Nat.le_refl _⟩
  obtain ⟨s0, β, hs0, hphi, hthird, _⟩ := (reach_inv hNT hprio hi hR hT hB hFix hreach).2
  obtain ⟨r, hr, hsr⟩ := hthird hpos
  have := Phi.busy_lt hil hpos hphi
  exact ⟨r, hr, by omega⟩

/-- **Stage 1**: stage 2 where no Critical_Section of a lower Task on a
Resource of Ceiling at least Task `i`'s Priority executes (`B = 0`), in
particular where no Resource is locked. -/
theorem stage1 {cfg : logic.Config NT NR NP} {P : Params} {i R : Nat}
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    (hi : i < NT.val) (hR : 0 < R) (hT : ∀ k ∈ lvl cfg i, 0 < P.T k)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i → prio cfg i ≤ ceil cfg r → P.L k r = 0)
    (hFix : ∑ k ∈ lvl cfg i, P.C k * ceilDiv R (P.T k) ≤ R)
    {st : St NT NR NL} (hreach : Reach cfg P st) (hpos : 0 < st.pend i + st.run i) :
    ∃ r, st.last i = some r ∧ st.now < r + R :=
  stage2 (B := 0) hNT hprio hi hR hT (fun k r a b c => Nat.le_of_eq (hB k r a b c))
    (by omega) hreach hpos

end P18
