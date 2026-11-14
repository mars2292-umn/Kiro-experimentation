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
    s0.depth.val = 0 ∧ s0.ceiling.val = 0 ∧ s0.lock_len = 0#usize := by
  simp only [logic.sched.Sched.new] at h
  have h := ok_inj h
  subst h
  refine ⟨?_, ?_, rfl, ?_, rfl⟩
  · intro k hk; simp [rel, rel_none, hk]
  · intro k hk; simp [job, job_none, hk]
  · show (logic.IDLE_CEILING).val = 0
    rw [idle_ceiling]; rfl

theorem Wf.boot {cfg : logic.Config NT NR NP} {s0 : logic.sched.Sched NT NR NL}
    (h : logic.sched.Sched.new NL cfg = ok s0) :
    Wf cfg { s := s0, now := 0, last := fun _ => none, pend := fun _ => 0, run := fun _ => 0 } := by
  obtain ⟨hrel, hjob, hdep, hceil, hlock⟩ := new_views h
  refine ⟨hlock, hceil, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> dsimp only
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

/-- While the Tasks of the level have work and the test accepts no Task,
a Job is in progress and the one on top of the stack is of the level. -/
theorem busy_top {cfg : logic.Config NT NR NP} {st : St NT NR NL} (w : Wf cfg st)
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    {b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
    {i k : Nat} (hk : k ∈ lvl cfg i) (hpos : 0 < st.pend k + st.run k) :
    0 < st.s.depth.val ∧ stk st.s (st.s.depth.val - 1) ∈ lvl cfg i := by
  obtain ⟨hkn, hki⟩ := mem_lvl.mp hk
  have hstack : job st.s k = 1 →
      0 < st.s.depth.val ∧ prio cfg k ≤ prio cfg (stk st.s (st.s.depth.val - 1)) := by
    intro hj
    obtain ⟨p, hp, hpk⟩ := (w.on_stack k hkn).mp hj
    refine ⟨by omega, ?_⟩
    by_cases hpt : p = st.s.depth.val - 1
    · rw [← hpt, hpk]
    · have := w.sorted p (st.s.depth.val - 1) (by omega) (by omega)
      rw [hpk] at this; omega
  have key : 0 < st.s.depth.val ∧ prio cfg k ≤ prio cfg (stk st.s (st.s.depth.val - 1)) := by
    rcases w.job01 k hkn with hj0 | hj1
    · have hrun : st.run k = 0 := by
        by_contra hc
        have := (w.run_iff k hkn).mp (by omega)
        omega
      have hrel := (w.pend_iff k hkn).mp (by omega)
      let t : U8 := UScalar.ofNatCore k (by simp [UScalarTy.numBits]; omega)
      have htv : t.val = k := UScalar.ofNatCore_val_eq _
      have hnot : ¬ Elig cfg st.s t.val := by
        intro he
        have := (eligible_ok hm (hne t)).mpr he
        cases this
      rw [htv, elig_views] at hnot
      have hc := w.ceil0
      have hp := hprio k hkn
      by_contra hcon
      apply hnot
      refine ⟨hkn, hrel, hj0, by omega, ?_⟩
      by_cases hd : st.s.depth.val = 0
      · left; exact hd
      · right
        by_contra hlt
        exact hcon ⟨by omega, by omega⟩
    · exact hstack hj1
  refine ⟨key.1, mem_lvl.mpr ⟨w.stk_lt _ (by omega), by omega⟩⟩

/-- The work Task `k`'s released Jobs still need. -/
def rem (st : St NT NR NL) (k : Nat) : Nat := st.pend k + st.run k

/-- The window invariant of Task `i`'s level is kept by every transition. -/
theorem level_step {cfg : logic.Config NT NR NP} {P : Params} {i R : Nat}
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    (hi : i ∈ lvl cfg i) (hR : 0 < R) (hT : ∀ k ∈ lvl cfg i, 0 < P.T k)
    (hFix : ∑ k ∈ lvl cfg i, P.C k * ceilDiv R (P.T k) ≤ R)
    {st st' : St NT NR NL} (w : Wf cfg st) (hs : Step cfg P st st')
    (h : LInv P R (lvl cfg i) i st.now st.last (rem st)) :
    LInv P R (lvl cfg i) i st'.now st'.last (rem st') := by
  cases hs with
  | release t d s' hd1 hd2 hsp hs =>
    by_cases ht : t.val ∈ lvl cfg i
    · have e : rem { st with
          s := s', last := Function.update st.last t.val (some st.now),
          pend := Function.update st.pend t.val (st.pend t.val + d) }
          = Function.update (rem st) t.val (rem st t.val + d) := by
        funext k
        by_cases hk : k = t.val
        · subst hk; simp [rem, Function.update]; omega
        · simp [rem, Function.update, hk]
      rw [e]
      exact LInv.release hFix hR hi ht (hT _ ht) hd2 hsp h
    · apply LInv.congr hi _ _ h
      · intro k hk
        have hkt : k ≠ t.val := fun hc => ht (hc ▸ hk)
        simp [rem, Function.update, hkt]
      · intro k hk
        have hkt : k ≠ t.val := fun hc => ht (hc ▸ hk)
        simp [Function.update, hkt]
  | dispatch b t s' hm he hs =>
    obtain ⟨htn, _, hj0, _, _⟩ := elig_views.mp ((eligible_ok hm he).mp rfl)
    have hrun : st.run t.val = 0 := by
      by_contra hc
      have := (w.run_iff _ htn).mp (by omega)
      omega
    apply LInv.congr hi _ (fun _ _ => rfl) h
    intro k hk
    by_cases hkt : k = t.val
    · subst hkt; simp [rem, Function.update, hrun]
    · simp [rem, Function.update, hkt]
  | idle b hm hne hd =>
    apply LInv.tick hFix hi (fun _ _ => Nat.le_refl _) _ h
    rintro ⟨k, hk, hpos⟩
    have := (busy_top w hNT hprio hm hne hk hpos).1
    omega
  | exec b hm hne hd hr =>
    apply LInv.tick hFix hi _ _ h
    · intro k hk
      by_cases hkt : k = stk st.s (st.s.depth.val - 1)
      · subst hkt; simp [rem, Function.update]
      · simp [rem, Function.update, hkt]
    · rintro ⟨k, hk, hpos⟩
      refine ⟨_, (busy_top w hNT hprio hm hne hk hpos).2, ?_⟩
      simp [rem, Function.update]; omega
  | finish b hm hne r s' hd hr hs =>
    have htop : stk st.s (st.s.depth.val - 1) < NT.val := w.stk_lt _ (by omega)
    have hrun : 0 < st.run (stk st.s (st.s.depth.val - 1)) :=
      (w.run_iff _ htop).mpr ((w.on_stack _ htop).mpr ⟨_, by omega, rfl⟩)
    apply LInv.tick hFix hi _ _ h
    · intro k hk
      by_cases hkt : k = stk st.s (st.s.depth.val - 1)
      · subst hkt; simp [rem, Function.update]
      · simp [rem, Function.update, hkt]
    · rintro ⟨k, hk, hpos⟩
      refine ⟨_, (busy_top w hNT hprio hm hne hk hpos).2, ?_⟩
      simp [rem, Function.update]; omega

/-- Both invariants hold in every state a run reaches. -/
theorem reach_inv {cfg : logic.Config NT NR NP} {P : Params} {i R : Nat}
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    (hi : i < NT.val) (hR : 0 < R) (hT : ∀ k ∈ lvl cfg i, 0 < P.T k)
    (hFix : ∑ k ∈ lvl cfg i, P.C k * ceilDiv R (P.T k) ≤ R)
    {st : St NT NR NL} (hreach : Reach cfg P st) :
    Wf cfg st ∧ LInv P R (lvl cfg i) i st.now st.last (rem st) := by
  have hil : i ∈ lvl cfg i := mem_lvl.mpr ⟨hi, Nat.le_refl _⟩
  induction hreach with
  | boot s0 h =>
    refine ⟨Wf.boot h, 0, Nat.le_refl _, idle_anchor hFix _ _ _ (fun _ _ => rfl), ?_⟩
    intro hx; simp [rem] at hx
  | step st st' _ hs ih =>
    exact ⟨ih.1.step hs, level_step hNT hprio hil hR hT hFix ih.1 hs ih.2⟩

/-- Stage 1 (no Resource is locked; no Kernel overhead, no jitter). In
every run of the Kernel's scheduler, every Job of Task `i` completes
within `R` of its Release, for any `R > 0` with
`Σ_{k : P_k ≥ P_i} C_k · ⌈R / T_k⌉ ≤ R`: in every state the run reaches
in which Task `i` has unfinished work, its last Release was less than `R`
ago. (Time is counted in units; a Job that completes in the unit starting
at `now` completes at `now + 1 ≤ r + R`.) -/
theorem stage1 {cfg : logic.Config NT NR NP} {P : Params} {i R : Nat}
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    (hi : i < NT.val) (hR : 0 < R) (hT : ∀ k ∈ lvl cfg i, 0 < P.T k)
    (hFix : ∑ k ∈ lvl cfg i, P.C k * ceilDiv R (P.T k) ≤ R)
    {st : St NT NR NL} (hreach : Reach cfg P st) (hpos : 0 < st.pend i + st.run i) :
    ∃ r, st.last i = some r ∧ st.now < r + R :=
  LInv.bound (mem_lvl.mpr ⟨hi, Nat.le_refl _⟩)
    (reach_inv hNT hprio hi hR hT hFix hreach).2 hpos

end P18
