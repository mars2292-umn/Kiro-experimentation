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
    (h : logic.sched.Sched.new NL cfg = ok s0) : Wf cfg P (St.boot s0) := by
  obtain ⟨hrel, hjob, hdep, hceil, hlock⟩ := new_views h
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> dsimp only [St.boot]
  · omega
  · intro p hp; omega
  · intro p q _ hq; omega
  · intro k hk; rw [hjob k hk]
    constructor
    · intro hx; exact absurd rfl hx
    · rintro ⟨p, hp, _⟩; omega
  · intro k hk; rw [hjob k hk]; omega
  · intro k hk; rw [hrel k hk]; omega
  · rw [hlock, hceil]; exact ⟨fun p hp => by omega, Or.inl rfl⟩
  · intro p hp; omega
  · intro p q _ hq; omega
  · intro p q hp; omega
  · intro p hp; omega
  · intro k hk; cases hk
  · intro k hk; cases hk
  · intro k; omega
  · intro hx; omega

/-! ## The accounts of the busy window -/

/-- What the entry that ends an overrunning Job of Task `k` may take
beyond a Budget event's entry and a Job end (0 where the Partition does
not stop overrunning Jobs). -/
def ovx (P : Params) (k : Nat) : Nat := if P.stop k then P.Kov - (P.Kt + P.Kc) else 0

/-- The Kernel time of the sections one Job of Task `k` may begin from its
dispatch on: the dispatch, a lock and an unlock per Critical_Section site,
its Endpoint operations, one Budget event, and its end; an overrun's entry
is the Budget event and the end, with `ovx` more. -/
def Ojob (P : Params) (k : Nat) : Nat :=
  P.Kd + P.S k * P.Kin + P.S k * P.Kout + P.EP k + P.Kt + P.Kc + ovx P k

/-- What of it the Job in progress may still take. -/
def remJ (P : Params) (st : St NT NR NL) (k : Nat) : Nat :=
  st.rl k * P.Kin + st.ru k * P.Kout + st.re k + st.rb k * P.Kt + P.Kc + ovx P k

/-- The Kernel time Task `k`'s released Jobs may still take. -/
def cred (P : Params) (st : St NT NR NL) (k : Nat) : Nat :=
  (if rel st.s k = 1 then Ojob P k else 0) + (if job st.s k = 1 then remJ P st k else 0)

/-- What is left of the Kernel section in progress, on the account `x`. -/
def ksOf (st : St NT NR NL) (x : Nat) : Nat := if st.kp = x then st.ks else 0

/-- The work now owed on the account of Task `x`'s Jobs. -/
def wkJ (P : Params) (st : St NT NR NL) (x : Nat) : Nat :=
  st.pend x + st.run x + cred P st x + ksOf st x
    + (if st.jwait x then P.C x + Ojob P x else 0)

/-- The work now owed on the account of the event source of index `x`. -/
def wkS (P : Params) (st : St NT NR NL) (x : Nat) : Nat :=
  ksOf st x + (if st.ewait (x - NT.val) then P.EC (x - NT.val) else 0)

/-- The work now owed that Task `i`'s level waits for, per account: the
Jobs of a Task of the level, and every event source but those `dead`
marks (the deadline events that never fire). The Jobs of a lower Task are
not on any account here: what they delay is the blocking. -/
def wkOf (cfg : logic.Config NT NR NP) (P : Params) (i : Nat) (dead : Nat → Bool)
    (st : St NT NR NL) (x : Nat) : Nat :=
  if x < NT.val then (if prio cfg i ≤ prio cfg x then wkJ P st x else 0)
  else (if dead (x - NT.val) then 0 else wkS P st x)

/-- The most one arrival adds to an account. -/
def costOf (cfg : logic.Config NT NR NP) (P : Params) (i : Nat) (dead : Nat → Bool)
    (x : Nat) : Nat :=
  if x < NT.val then (if prio cfg i ≤ prio cfg x then P.C x + Ojob P x else 0)
  else (if dead (x - NT.val) then 0 else P.EC (x - NT.val))

def perOf (_cfg : logic.Config NT NR NP) (P : Params) (x : Nat) : Nat :=
  if x < NT.val then P.T x else P.ET (x - NT.val)

def jitOf (_cfg : logic.Config NT NR NP) (P : Params) (x : Nat) : Nat :=
  if x < NT.val then P.J x else P.EJ (x - NT.val)

def lastOf (st : St NT NR NL) (x : Nat) : Option Nat :=
  if x < NT.val then st.jl x else st.el (x - NT.val)

/-- A lock entry that blocks Task `i`'s level: its Resource's Ceiling is at
least Task `i`'s Priority and its holder's Priority is lower. -/
def blocking (cfg : logic.Config NT NR NP) (i : Nat) (s : logic.sched.Sched NT NR NL)
    (p : Nat) : Prop :=
  p < s.lock_len.val ∧ prio cfg i ≤ ceil cfg (lres s p) ∧
  prio cfg (stk s (ldep s p)) < prio cfg i

/-- The Kernel section in progress is on the account of a Job of a Task
below the level. -/
def lowJob (cfg : logic.Config NT NR NP) (i : Nat) (st : St NT NR NL) : Prop :=
  st.kp < NT.val ∧ prio cfg st.kp < prio cfg i

instance (cfg : logic.Config NT NR NP) (i : Nat) (st : St NT NR NL) :
    Decidable (lowJob cfg i st) := by
  unfold lowJob; infer_instance

def ksLow (cfg : logic.Config NT NR NP) (i : Nat) (st : St NT NR NL) : Nat :=
  if lowJob cfg i st then st.ks else 0

/-- What a blocking entry may still cost the level: its Critical_Section's
remaining time and its holder's Budget event. -/
def bv (P : Params) (st : St NT NR NL) (p : Nat) : Nat :=
  st.allow p + P.Kt * st.rb (stk st.s (ldep st.s p))

/-- `β` bounds what the first blocking entry may still cost (it is the
outermost one: the others are nested in it) together with the lower Job's
Kernel section in progress. -/
def Blkf (bl : Nat → Prop) (v : Nat → Nat) (κ β : Nat) : Prop :=
  κ ≤ β ∧ ∀ p, bl p → ∃ p0, bl p0 ∧ (∀ q, bl q → p0 ≤ q) ∧ v p0 + κ ≤ β

def Blk (cfg : logic.Config NT NR NP) (P : Params) (i : Nat) (st : St NT NR NL) (β : Nat) : Prop :=
  Blkf (blocking cfg i st.s) (bv P st) (ksLow cfg i st) β

/-- How the blocking account changes in a step that keeps or removes the
first blocking entry and lowers the account by `δ`. -/
theorem Blkf.trans {bl bl' : Nat → Prop} {v v' : Nat → Nat} {κ κ' β δ : Nat}
    (h1 : ∀ q, bl' q → ∃ q0, q0 ≤ q ∧ bl q0)
    (h2 : ∀ p0 q, bl p0 → bl' q → p0 ≤ q → bl' p0)
    (hv : ∀ p0, bl p0 → bl' p0 → v' p0 + κ' + δ ≤ v p0 + κ)
    (hκ : κ' + δ ≤ β)
    (h : Blkf bl v κ β) : Blkf bl' v' κ' (β - δ) := by
  refine ⟨by omega, ?_⟩
  intro p hp
  obtain ⟨q0, hq0, hb0⟩ := h1 p hp
  obtain ⟨p0, hbp0, hmin, hle⟩ := h.2 q0 hb0
  have hb' := h2 p0 p hbp0 hp (Nat.le_trans (hmin q0 hb0) hq0)
  refine ⟨p0, hb', ?_, ?_⟩
  · intro q hq
    obtain ⟨q1, hq1, hb1⟩ := h1 q hq
    exact Nat.le_trans (hmin q1 hb1) hq1
  · have := hv p0 hbp0 hb'
    omega

/-- The level has no released work and the Kernel executes nothing for it. -/
def Quiet (cfg : logic.Config NT NR NP) (i : Nat) (st : St NT NR NL) : Prop :=
  (∀ k ∈ lvl cfg i, st.pend k + st.run k = 0) ∧ (st.ks = 0 ∨ lowJob cfg i st)

/-- The window invariant of Task `i`'s level. -/
def LInv (cfg : logic.Config NT NR NP) (P : Params) (R i : Nat) (dead : Nat → Bool)
    (st : St NT NR NL) : Prop :=
  Quiet cfg i st ∨ ∃ s0 β, s0 ≤ st.now ∧
    Phi (perOf cfg P) (costOf cfg P i dead) R (Finset.range (NT.val + P.NQ)) s0 β st.now
      (lastOf st) (wkOf cfg P i dead st) ≤ R ∧
    (0 < st.pend i + st.run i → ∃ a, st.jl i = some a ∧ s0 ≤ a + P.J i) ∧ Blk cfg P i st β

/-- No Kernel section is on the account of an event source `dead` marks. -/
def ND (dead : Nat → Bool) (st : St NT NR NL) : Prop :=
  0 < st.ks → ∀ q, st.kp = NT.val + q → dead q = false

theorem wkOf_le {cfg : logic.Config NT NR NP} {P : Params} {i : Nat} {dead : Nat → Bool}
    {st st' : St NT NR NL} {x : Nat}
    (hJ : x < NT.val → prio cfg i ≤ prio cfg x → wkJ P st' x ≤ wkJ P st x)
    (hS : NT.val ≤ x → dead (x - NT.val) = false → wkS P st' x ≤ wkS P st x) :
    wkOf cfg P i dead st' x ≤ wkOf cfg P i dead st x := by
  unfold wkOf
  by_cases hx : x < NT.val
  · simp only [hx, ite_true]
    by_cases hl : prio cfg i ≤ prio cfg x
    · simp only [hl, ite_true]; exact hJ hx hl
    · simp only [hl, ite_false]; exact Nat.le_refl _
  · simp only [hx, ite_false]
    cases hd : dead (x - NT.val)
    · simp only [Bool.false_eq_true, ite_false]; exact hS (by omega) hd
    · simp only [ite_true]; exact Nat.le_refl _

theorem cred_zero {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL} (w : Wf cfg P st)
    {k : Nat} (hk : k < NT.val) (h0 : st.pend k + st.run k = 0) : cred P st k = 0 := by
  have h1 : rel st.s k ≠ 1 := by
    intro h; have := (w.pend_iff k hk).mpr h; omega
  have h2 : job st.s k ≠ 1 := by
    intro h; have := (w.run_iff k hk).mpr h; omega
  simp [cred, h1, h2]

/-- When the level is quiet and the Kernel executes nothing, the window
starts now with the whole blocking allowance. -/
theorem anchor {cfg : logic.Config NT NR NP} {P : Params} {i R B : Nat} {dead : Nat → Bool}
    {st : St NT NR NL} (w : Wf cfg P st) (hR : 0 < R)
    (hT : ∀ x, x < NT.val + P.NQ → 0 < perOf cfg P x)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i → prio cfg i ≤ ceil cfg r → P.L k r ≤ B)
    (hFix : B + P.Kt + ∑ x ∈ Finset.range (NT.val + P.NQ),
      costOf cfg P i dead x * ceilDiv (R + jitOf cfg P x) (perOf cfg P x) ≤ R)
    (hq : Quiet cfg i st) (hk : st.ks = 0) :
    Phi (perOf cfg P) (costOf cfg P i dead) R (Finset.range (NT.val + P.NQ)) st.now (B + P.Kt)
      st.now (lastOf st) (wkOf cfg P i dead st) ≤ R ∧ Blk cfg P i st (B + P.Kt) := by
  constructor
  · refine idle_anchor (Je := jitOf cfg P) hFix (Nat.le_refl _) _ _ _ ?_
    intro x hx
    have hxn := Finset.mem_range.mp hx
    apply term_anchor (hT x hxn) hR
    unfold wkOf costOf lastOf jitOf
    by_cases hxt : x < NT.val
    · simp only [hxt, ite_true]
      by_cases hl : prio cfg i ≤ prio cfg x
      · simp only [hl, ite_true]
        have h0 := hq.1 x (mem_lvl.mpr ⟨hxt, hl⟩)
        have hc := cred_zero w hxt h0
        have hks : ksOf st x = 0 := by simp [ksOf, hk]
        unfold wkJ
        rw [hc, hks]
        cases hw : st.jwait x with
        | false => left; simp; omega
        | true => right; simp only [ite_true]; exact ⟨by omega, w.jwait_inv x hw⟩
      · simp only [hl, ite_false]; left; trivial
    · simp only [hxt, ite_false]
      cases hd : dead (x - NT.val) with
      | true => simp
      | false =>
        simp only [Bool.false_eq_true, ite_false]
        have hks : ksOf st x = 0 := by simp [ksOf, hk]
        unfold wkS
        rw [hks]
        cases hw : st.ewait (x - NT.val) with
        | false => left; simp
        | true => right; simp only [ite_true]; exact ⟨by omega, w.ewait_inv _ hw⟩
  · have hκ : ksLow cfg i st = 0 := by
      unfold ksLow; split
      · exact hk
      · rfl
    refine ⟨by omega, ?_⟩
    intro p hp
    classical
    have hex : ∃ p, blocking cfg i st.s p := ⟨p, hp⟩
    have hp0 := Nat.find_spec hex
    refine ⟨Nat.find hex, hp0, fun q hq => Nat.find_min' hex hq, ?_⟩
    obtain ⟨hl, hc, hpr⟩ := hp0
    have h1 := Nat.le_trans (w.allow_le _ hl) (hB _ _ (w.stk_lt _ (w.ldep_lt _ hl)) hpr hc)
    have h2 := w.rb_le (stk st.s (ldep st.s (Nat.find hex)))
    have h3 : P.Kt * st.rb (stk st.s (ldep st.s (Nat.find hex))) ≤ P.Kt := by
      calc _ ≤ P.Kt * 1 := Nat.mul_le_mul_left _ h2
        _ = P.Kt := Nat.mul_one _
    unfold bv; omega

/-- While the level is not quiet, work is owed on some account. -/
theorem busy_wk {cfg : logic.Config NT NR NP} {P : Params} {i : Nat} {dead : Nat → Bool}
    {st : St NT NR NL} (w : Wf cfg P st) (hnd : ND dead st) (h : ¬ Quiet cfg i st) :
    ∃ x ∈ Finset.range (NT.val + P.NQ), 0 < wkOf cfg P i dead st x := by
  by_cases h1 : ∀ k ∈ lvl cfg i, st.pend k + st.run k = 0
  · have h2 : ¬ (st.ks = 0 ∨ lowJob cfg i st) := fun hx => h ⟨h1, hx⟩
    have hks : 0 < st.ks := by
      by_contra hc; exact h2 (Or.inl (by omega))
    have hnl : ¬ lowJob cfg i st := fun hx => h2 (Or.inr hx)
    have hkp := w.kp_lt hks
    refine ⟨st.kp, Finset.mem_range.mpr hkp, ?_⟩
    unfold wkOf
    by_cases hx : st.kp < NT.val
    · have hl : prio cfg i ≤ prio cfg st.kp := by
        by_contra hc; exact hnl ⟨hx, by omega⟩
      simp only [hx, ite_true, hl]
      unfold wkJ ksOf; simp only [ite_true]; omega
    · have hd := hnd hks (st.kp - NT.val) (by omega)
      simp only [hx, ite_false, hd, Bool.false_eq_true]
      unfold wkS ksOf; simp only [ite_true]; omega
  · have h1' : ∃ k ∈ lvl cfg i, st.pend k + st.run k ≠ 0 := by
      by_contra hc
      apply h1
      intro k hk
      by_contra h0
      exact hc ⟨k, hk, h0⟩
    obtain ⟨k, hk, hne⟩ := h1'
    obtain ⟨hkn, hki⟩ := mem_lvl.mp hk
    refine ⟨k, Finset.mem_range.mpr (by omega), ?_⟩
    unfold wkOf; simp only [hkn, ite_true, hki]; unfold wkJ; omega

theorem level_of_not_quiet {cfg : logic.Config NT NR NP} {i : Nat} {st : St NT NR NL}
    (h : ¬ Quiet cfg i st) (hk : st.ks = 0) : ∃ k ∈ lvl cfg i, 0 < st.pend k + st.run k := by
  by_contra hc
  apply h
  refine ⟨?_, Or.inl hk⟩
  intro k hkl
  by_contra h0
  exact hc ⟨k, hkl, by omega⟩

/-- While the level has released work, the test accepts no Task, and the
Job on top of the stack is not of the level (or there is none), a lock
entry blocks the level, and every blocking entry is held by the Job on
top. -/
theorem busy_block {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL} (w : Wf cfg P st)
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    {b : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    (hm : Matches b cfg st.s) (hne : ∀ t : U8, eligibleFn b t = ok false)
    {i k : Nat} (hk : k ∈ lvl cfg i) (hpos : 0 < st.pend k + st.run k)
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
    by_contra hjn
    obtain ⟨p, hp, hpk⟩ := (w.on_stack k hkn).mp hjn
    have := below_top p hp
    rw [hpk] at this
    rcases hnot with h0 | h1 <;> omega
  have hrun : st.run k = 0 := by
    by_contra hc
    have := (w.run_iff k hkn).mp (by omega)
    omega
  have hrel := (w.pend_iff k hkn).mp (by omega)
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

/-- A Task below the level is dispatched only when the level has no
released work: the dispatch takes the eligible Task of highest Priority. -/
theorem low_disp {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL} (w : Wf cfg P st)
    {i t : Nat} (he : Elig cfg st.s t)
    (hmax : ∀ u, Elig cfg st.s u → prio cfg u ≤ prio cfg t) (hlow : prio cfg t < prio cfg i) :
    ∀ k ∈ lvl cfg i, st.pend k + st.run k = 0 := by
  intro k hk
  obtain ⟨hkn, hki⟩ := mem_lvl.mp hk
  obtain ⟨_, _, _, hc, htop⟩ := elig_views.mp he
  have below_top : ∀ p, p < st.s.depth.val →
      prio cfg (stk st.s p) ≤ prio cfg (stk st.s (st.s.depth.val - 1)) := by
    intro p hp
    by_cases hpt : p = st.s.depth.val - 1
    · rw [hpt]
    · exact Nat.le_of_lt (w.sorted p (st.s.depth.val - 1) (by omega) (by omega))
  have hjob : job st.s k = 0 := by
    by_contra hjn
    obtain ⟨p, hp, hpk⟩ := (w.on_stack k hkn).mp hjn
    have := below_top p hp
    rw [hpk] at this
    rcases htop with h0 | h1 <;> omega
  have hrun : st.run k = 0 := by
    by_contra hcr
    have := (w.run_iff k hkn).mp (by omega)
    omega
  by_contra hne
  have hrel := (w.pend_iff k hkn).mp (by omega)
  have hek : Elig cfg st.s k := elig_views.mpr ⟨hkn, hrel, hjob, by omega, by
    rcases htop with h0 | h1
    · left; exact h0
    · right; omega⟩
  have := hmax k hek
  omega

theorem ksOf_zero {st : St NT NR NL} (hk : st.ks = 0) (x : Nat) : ksOf st x = 0 := by
  unfold ksOf; split
  · exact hk
  · rfl

theorem ksLow_zero {cfg : logic.Config NT NR NP} {i : Nat} {st : St NT NR NL} (hk : st.ks = 0) :
    ksLow cfg i st = 0 := by
  unfold ksLow; split
  · exact hk
  · rfl

theorem ksLow_of_not_low {cfg : logic.Config NT NR NP} {i : Nat} {st : St NT NR NL}
    (h : ¬ lowJob cfg i st) : ksLow cfg i st = 0 := by
  unfold ksLow; split
  · rename_i hp; exact absurd hp h
  · rfl

theorem mul_pred_add {n K c : Nat} (hn : 0 < n) (hc : c ≤ K) : (n - 1) * K + c ≤ n * K := by
  obtain ⟨m, rfl⟩ : ∃ m, n = m + 1 := ⟨n - 1, by omega⟩
  rw [Nat.add_sub_cancel, Nat.add_mul, Nat.one_mul]; omega

theorem mul_pred_add' {n K c : Nat} (hn : 0 < n) (hc : c ≤ K) : K * (n - 1) + c ≤ K * n := by
  rw [Nat.mul_comm K (n - 1), Nat.mul_comm K n]; exact mul_pred_add hn hc

/-- `Blkf.trans` for a step that takes no blocking time. -/
theorem Blkf.keep {bl bl' : Nat → Prop} {v v' : Nat → Nat} {κ κ' β : Nat}
    (h1 : ∀ q, bl' q → ∃ q0, q0 ≤ q ∧ bl q0)
    (h2 : ∀ p0 q, bl p0 → bl' q → p0 ≤ q → bl' p0)
    (hv : ∀ p0, bl p0 → bl' p0 → v' p0 + κ' ≤ v p0 + κ)
    (hκ : κ' ≤ β)
    (h : Blkf bl v κ β) : Blkf bl' v' κ' β := by
  have := Blkf.trans (δ := 0) h1 h2 (fun p0 a b => by have := hv p0 a b; omega) (by omega) h
  simpa using this

/-- The window invariant of Task `i`'s level is kept by every transition. -/
theorem level_step {cfg : logic.Config NT NR NP} {P : Params} {i R B : Nat} {dead : Nat → Bool}
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    (hi : i < NT.val) (hR : 0 < R)
    (hT : ∀ x, x < NT.val + P.NQ → 0 < perOf cfg P x)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i → prio cfg i ≤ ceil cfg r → P.L k r ≤ B)
    (hFix : B + P.Kt + ∑ x ∈ Finset.range (NT.val + P.NQ),
      costOf cfg P i dead x * ceilDiv (R + jitOf cfg P x) (perOf cfg P x) ≤ R)
    (hdead : ∀ q, dead q = true → P.relOf q = none ∧ ∃ t, P.dlOf q = some t ∧
      ∀ st : St NT NR NL, Reach cfg P st → ∀ a, st.jl t = some a →
        (0 < st.pend t + st.run t ∨ st.jwait t = true) → st.now < a + P.D t)
    {st st' : St NT NR NL} (hreach : Reach cfg P st) (w : Wf cfg P st) (hnd : ND dead st)
    (hs : Step cfg P st st') (h : LInv cfg P R i dead st) : LInv cfg P R i dead st' := by
  have hil : i ∈ lvl cfg i := mem_lvl.mpr ⟨hi, Nat.le_refl _⟩
  by_cases hq' : Quiet cfg i st'
  · exact Or.inl hq'
  have hex : (st.ks = 0 ∨ ¬ Quiet cfg i st) → ∃ s0 β, s0 ≤ st.now ∧ st.now < s0 + R ∧
      Phi (perOf cfg P) (costOf cfg P i dead) R (Finset.range (NT.val + P.NQ)) s0 β st.now
        (lastOf st) (wkOf cfg P i dead st) ≤ R ∧
      (0 < st.pend i + st.run i → ∃ a, st.jl i = some a ∧ s0 ≤ a + P.J i) ∧
      Blk cfg P i st β := by
    intro hor
    by_cases hQ : Quiet cfg i st
    · have hk : st.ks = 0 := by
        rcases hor with h | h
        · exact h
        · exact absurd hQ h
      obtain ⟨hphi, hblk⟩ := anchor w hR hT hB hFix hQ hk
      exact ⟨st.now, B + P.Kt, Nat.le_refl _, by omega, hphi,
        fun hx => by have := hQ.1 i hil; omega, hblk⟩
    · rcases h with hq | ⟨s0, β, a, b, c', e⟩
      · exact absurd hq hQ
      · obtain ⟨x, hx, hpos⟩ := busy_wk w hnd hQ
        exact ⟨s0, β, a, Phi.busy_lt hx hpos b, b, c', e⟩
  cases hs with
  | jarrive t ht hsp =>
    have hnq : ¬ Quiet cfg i st := fun hQ => hq' hQ
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inr hnq)
    refine Or.inr ⟨s0, β, hs0, ?_, ?_, hblk⟩
    · have hl : lastOf { st with
            jl := Function.update st.jl t (some st.now),
            jwait := Function.update st.jwait t true }
          = Function.update (lastOf st) t (some st.now) := by
        funext x
        unfold lastOf
        by_cases hxt : x = t
        · subst hxt; simp [ht]
        · rw [Function.update_of_ne hxt]
          by_cases hx : x < NT.val
          · simp only [hx, ite_true]; exact Function.update_of_ne hxt _ _
          · simp only [hx, ite_false]
      dsimp only
      rw [hl]
      refine Nat.le_trans (Phi.arrive (Finset.mem_range.mpr (by omega)) (hT t (by omega))
        ?_ ?_ ?_ hlt) hphi
      · unfold wkOf costOf
        simp only [ht, ite_true]
        by_cases hl : prio cfg i ≤ prio cfg t
        · simp only [hl, ite_true]
          unfold wkJ cred remJ ksOf
          simp only [Function.update_self, ite_true]
          omega
        · simp only [hl, ite_false]; omega
      · intro x _ hxt
        unfold wkOf wkJ wkS cred remJ ksOf
        simp only [Function.update_of_ne hxt]
      · intro r hr
        unfold lastOf at hr
        simp only [ht, ite_true] at hr
        unfold perOf; simp only [ht, ite_true]
        exact hsp r hr
    · dsimp only
      intro hx
      by_cases hit : i = t
      · subst hit; exact ⟨st.now, by simp, by omega⟩
      · rw [Function.update_of_ne hit]; exact hthird hx
  | earrive q hq hsp =>
    have hnq : ¬ Quiet cfg i st := fun hQ => hq' hQ
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inr hnq)
    refine Or.inr ⟨s0, β, hs0, ?_, hthird, hblk⟩
    have hnlt : ¬ (NT.val + q < NT.val) := by omega
    have hl : lastOf { st with
          el := Function.update st.el q (some st.now),
          ewait := Function.update st.ewait q true }
        = Function.update (lastOf st) (NT.val + q) (some st.now) := by
      funext x
      unfold lastOf
      by_cases hxt : x = NT.val + q
      · subst hxt; simp
      · rw [Function.update_of_ne hxt]
        by_cases hx : x < NT.val
        · simp only [hx, ite_true]
        · simp only [hx, ite_false]; exact Function.update_of_ne (by omega) _ _
    dsimp only
    rw [hl]
    refine Nat.le_trans (Phi.arrive (Finset.mem_range.mpr (by omega)) (hT _ (by omega))
      ?_ ?_ ?_ hlt) hphi
    · unfold wkOf costOf
      simp only [hnlt, ite_false, Nat.add_sub_cancel_left]
      cases hd : dead q
      · simp only [Bool.false_eq_true, ite_false]
        unfold wkS ksOf
        simp only [Nat.add_sub_cancel_left, Function.update_self, ite_true]
        omega
      · simp
    · intro x _ hxt
      unfold wkOf wkJ wkS cred remJ ksOf
      by_cases hx : x < NT.val
      · simp only [hx, ite_true]
      · simp only [hx, ite_false]
        simp only [Function.update_of_ne (show x - NT.val ≠ q by omega)]
    · intro r hr
      unfold lastOf at hr
      simp only [hnlt, ite_false, Nat.add_sub_cancel_left] at hr
      unfold perOf; simp only [hnlt, ite_false, Nat.add_sub_cancel_left]
      exact hsp r hr
  | erel q t d c s' hq hk0 hw hrel hj hd1 hd2 hc hs =>
    obtain ⟨ht, hrl, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := make_pending_views hs
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inl hk0)
    have hdq : dead q = false := by
      cases hd : dead q with
      | false => rfl
      | true => have := (hdead q hd).1; rw [this] at hrel; cases hrel
    refine Or.inr ⟨s0, β, hs0,
      Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, ?_, ?_⟩
    · intro hx hl
      have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
      unfold wkJ
      rw [hk1]
      unfold cred remJ ksOf
      dsimp only
      rw [hrl, hjob]
      simp only [show ¬ (NT.val + q = x) by omega, ite_false]
      by_cases hxt : x = t.val
      · subst hxt
        simp only [Function.update_self, hj, ite_true, Bool.false_eq_true, ite_false]
        split <;> omega
      · simp only [Function.update_of_ne hxt, hxt, ite_false]
        omega
    · intro hx hd
      have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
      unfold wkS
      rw [hk1]
      unfold ksOf
      dsimp only
      by_cases hxq : x = NT.val + q
      · subst hxq
        simp only [ite_true, Nat.add_sub_cancel_left, Function.update_self, hw,
          Bool.false_eq_true, ite_false]
        omega
      · have hne : x - NT.val ≠ q := by omega
        simp only [show ¬ (NT.val + q = x) by omega, ite_false, Function.update_of_ne hne]
        omega
    · dsimp only
      intro hx
      by_cases hit : i = t.val
      · obtain ⟨a, ha, hja⟩ := w.jwait_inv t.val hj
        rw [hit]; exact ⟨a, ha, by omega⟩
      · rw [Function.update_of_ne hit] at hx; exact hthird hx
    · have hbl : ∀ p, blocking cfg i s' p ↔ blocking cfg i st.s p := by
        intro p; unfold blocking; rw [hll, hlr, hld, hstk]
      have hκ' : ksLow cfg i { st with
          s := s', ewait := Function.update st.ewait q false,
          jwait := Function.update st.jwait t.val false,
          pend := Function.update st.pend t.val (st.pend t.val + d),
          ks := c, kp := NT.val + q } = 0 :=
        ksLow_of_not_low (fun h => by have := h.1; dsimp only at this; omega)
      unfold Blk
      refine Blkf.keep (fun p hp => ⟨p, Nat.le_refl _, (hbl p).mp hp⟩)
        (fun p0 _ hp0 _ _ => (hbl p0).mpr hp0) ?_ ?_ hblk
      · intro p0 _ _
        rw [hκ']; unfold bv; dsimp only; rw [hld, hstk]; omega
      · rw [hκ']; omega
  | eplain q c hq hk0 hw hc hdl =>
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inl hk0)
    have hdq : dead q = false := by
      cases hd : dead q with
      | false => rfl
      | true =>
        obtain ⟨_, t, ht, hall⟩ := hdead q hd
        obtain ⟨a, ha, hle, hun⟩ := hdl t ht
        have := hall st hreach a ha hun
        omega
    refine Or.inr ⟨s0, β, hs0,
      Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, hthird, ?_⟩
    · intro hx hl
      unfold wkJ cred remJ ksOf
      dsimp only
      simp only [show ¬ (NT.val + q = x) by omega, ite_false]
      omega
    · intro hx hd
      have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
      unfold wkS
      rw [hk1]
      unfold ksOf
      dsimp only
      by_cases hxq : x = NT.val + q
      · subst hxq
        simp only [ite_true, Nat.add_sub_cancel_left, Function.update_self, hw,
          Bool.false_eq_true, ite_false]
        omega
      · have hne : x - NT.val ≠ q := by omega
        simp only [show ¬ (NT.val + q = x) by omega, ite_false, Function.update_of_ne hne]
        omega
    · have hκ' : ksLow cfg i { st with
          ewait := Function.update st.ewait q false, ks := c, kp := NT.val + q } = 0 :=
        ksLow_of_not_low (fun h => by have := h.1; dsimp only at this; omega)
      unfold Blk
      refine Blkf.keep (fun p hp => ⟨p, Nat.le_refl _, hp⟩) (fun p0 _ hp0 _ _ => hp0) ?_ ?_ hblk
      · intro p0 _ _
        rw [hκ']; unfold bv; dsimp only; omega
      · rw [hκ']; omega
  | edefer q t c s' hq hk0 hw hrel hc hr0 hs =>
    obtain ⟨ht, hrl, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := defer_views hs
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inl hk0)
    have hdq : dead q = false := by
      cases hd : dead q with
      | false => rfl
      | true => have := (hdead q hd).1; rw [this] at hrel; cases hrel
    refine Or.inr ⟨s0, β, hs0,
      Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, hthird, ?_⟩
    · intro hx hl
      unfold wkJ cred remJ ksOf
      dsimp only
      rw [hrl, hjob]
      simp only [show ¬ (NT.val + q = x) by omega, ite_false]
      by_cases hxt : x = t.val
      · subst hxt
        simp only [ite_true, show ¬ ((2 : Nat) = 1) by omega, ite_false]
        omega
      · simp only [hxt, ite_false]
        omega
    · intro hx hd
      have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
      unfold wkS
      rw [hk1]
      unfold ksOf
      dsimp only
      by_cases hxq : x = NT.val + q
      · subst hxq
        simp only [ite_true, Nat.add_sub_cancel_left, Function.update_self, hw,
          Bool.false_eq_true, ite_false]
        omega
      · have hne : x - NT.val ≠ q := by omega
        simp only [show ¬ (NT.val + q = x) by omega, ite_false, Function.update_of_ne hne]
        omega
    · have hbl : ∀ p, blocking cfg i s' p ↔ blocking cfg i st.s p := by
        intro p; unfold blocking; rw [hll, hlr, hld, hstk]
      have hκ' : ksLow cfg i { st with
          s := s', ewait := Function.update st.ewait q false, ks := c, kp := NT.val + q } = 0 :=
        ksLow_of_not_low (fun h => by have := h.1; dsimp only at this; omega)
      unfold Blk
      refine Blkf.keep (fun p hp => ⟨p, Nat.le_refl _, (hbl p).mp hp⟩)
        (fun p0 _ hp0 _ _ => (hbl p0).mpr hp0) ?_ ?_ hblk
      · intro p0 _ _
        rw [hκ']; unfold bv; dsimp only; rw [hld, hstk]; omega
      · rw [hκ']; omega
  | ktick hk0 hj =>
    have hnq : ¬ Quiet cfg i st := by
      intro hQ
      apply hq'
      refine ⟨hQ.1, ?_⟩
      rcases hQ.2 with h0 | hl
      · omega
      · exact Or.inr hl
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inr hnq)
    have hkp := w.kp_lt hk0
    by_cases hlow : lowJob cfg i st
    · have hκ : ksLow cfg i st = st.ks := by
        unfold ksLow; split
        · rfl
        · rename_i hn; exact absurd hlow hn
      have hκ' : ksLow cfg i { st with now := st.now + 1, ks := st.ks - 1 } = st.ks - 1 := by
        unfold ksLow; split
        · rfl
        · rename_i hn; exact absurd hlow hn
      have hβ : st.ks ≤ β := by have := hblk.1; omega
      refine Or.inr ⟨s0, β - 1, by dsimp only; omega, ?_, hthird, ?_⟩
      · dsimp only
        refine Nat.le_trans (Phi.tick hs0 (fun x _ => wkOf_le ?_ ?_) (Or.inr (by omega))) hphi
        · intro hx hl
          have hkx : st.kp ≠ x := by
            intro e; have := hlow.2; rw [e] at this; omega
          unfold wkJ cred remJ ksOf
          dsimp only
          simp only [hkx, ↓reduceIte]
          omega
        · intro hx hd
          have hkx : st.kp ≠ x := by have := hlow.1; omega
          unfold wkS ksOf
          dsimp only
          simp only [hkx, ↓reduceIte]
          omega
      · unfold Blk
        refine Blkf.trans (δ := 1) (fun p hp => ⟨p, Nat.le_refl _, hp⟩)
          (fun p0 _ hp0 _ _ => hp0) ?_ ?_ hblk
        · intro p0 _ _
          rw [hκ', hκ]; unfold bv; dsimp only; omega
        · rw [hκ']; omega
    · have hκ : ksLow cfg i st = 0 := ksLow_of_not_low hlow
      have hκ' : ksLow cfg i { st with now := st.now + 1, ks := st.ks - 1 } = 0 :=
        ksLow_of_not_low hlow
      refine Or.inr ⟨s0, β, by dsimp only; omega, ?_, hthird, ?_⟩
      · dsimp only
        refine Nat.le_trans (Phi.tick hs0 (fun x _ => wkOf_le ?_ ?_)
          (Or.inl ⟨⟨st.kp, Finset.mem_range.mpr hkp, ?_⟩, Nat.le_refl _⟩)) hphi
        · intro hx hl
          unfold wkJ cred remJ ksOf
          dsimp only
          by_cases hkx : st.kp = x
          · subst hkx; simp only [↓reduceIte]; omega
          · simp only [hkx, ↓reduceIte]; omega
        · intro hx hd
          unfold wkS ksOf
          dsimp only
          by_cases hkx : st.kp = x
          · subst hkx; simp only [↓reduceIte]; omega
          · simp only [hkx, ↓reduceIte]; omega
        · unfold wkOf
          by_cases hx : st.kp < NT.val
          · have hl : prio cfg i ≤ prio cfg st.kp := by
              by_contra hc; exact hlow ⟨hx, by omega⟩
            simp only [hx, ite_true, hl]
            unfold wkJ cred remJ ksOf
            dsimp only
            simp only [ite_true]
            omega
          · have hd := hnd hk0 (st.kp - NT.val) (by omega)
            simp only [hx, ite_false, hd, Bool.false_eq_true]
            unfold wkS ksOf
            dsimp only
            simp only [ite_true]
            omega
      · unfold Blk
        refine Blkf.keep (fun p hp => ⟨p, Nat.le_refl _, hp⟩) (fun p0 _ hp0 _ _ => hp0) ?_ ?_ hblk
        · intro p0 _ _
          rw [hκ', hκ]; unfold bv; dsimp only; omega
        · rw [hκ']; omega
  | dispatch b t c s' hk0 hm he hmax hc hs =>
    have helig := (eligible_ok hm he).mp rfl
    obtain ⟨htn, hr1, hj0, _, _⟩ := elig_views.mp helig
    obtain ⟨_, _, hrl, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := start_views hs
    have hrun : st.run t.val = 0 := by
      by_contra hcr
      have := (w.run_iff _ htn).mp (by omega)
      omega
    by_cases hlv : prio cfg i ≤ prio cfg t.val
    swap
    · exfalso
      apply hq'
      have hlow : prio cfg t.val < prio cfg i := by omega
      have h0 := low_disp w helig hmax hlow
      refine ⟨?_, Or.inr ⟨htn, hlow⟩⟩
      intro k hk
      have hkt : k ≠ t.val := by
        intro e; have := (mem_lvl.mp hk).2; rw [e] at this; omega
      dsimp only
      rw [Function.update_of_ne hkt, Function.update_of_ne hkt]
      exact h0 k hk
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inl hk0)
    have hrem : ∀ k, (Function.update st.pend t.val 0) k
        + (Function.update st.run t.val (st.pend t.val)) k = st.pend k + st.run k := by
      intro k
      by_cases hkt : k = t.val
      · subst hkt; simp [hrun]
      · simp [Function.update_of_ne hkt]
    refine Or.inr ⟨s0, β, hs0,
      Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, ?_, ?_⟩
    · intro hx hl
      have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
      unfold wkJ
      rw [hk1]
      unfold cred remJ ksOf
      dsimp only
      rw [hrl, hjob]
      by_cases hxt : x = t.val
      · subst hxt
        simp only [Function.update_self, hr1, hj0, hrun, ↓reduceIte,
          show ¬ ((0 : Nat) = 1) by omega]
        unfold Ojob
        omega
      · have hne' : ¬ (t.val = x) := fun e => hxt e.symm
        simp only [Function.update_of_ne hxt, hxt, hne', ↓reduceIte]
        omega
    · intro hx hd
      unfold wkS ksOf
      dsimp only
      simp only [show ¬ (t.val = x) by omega, ↓reduceIte]
      omega
    · dsimp only
      intro hx
      rw [hrem] at hx
      exact hthird hx
    · have hstk' : ∀ q, q < st.s.lock_len.val → stk s' (ldep st.s q) = stk st.s (ldep st.s q) := by
        intro q hq
        rw [hstk]
        have := w.ldep_lt q hq
        simp only [show ldep st.s q ≠ st.s.depth.val by omega, ite_false]
      have hbl : ∀ q, blocking cfg i s' q ↔ blocking cfg i st.s q := by
        intro q; unfold blocking; rw [hll, hlr, hld]
        constructor
        · rintro ⟨hq, hcq, hp⟩; exact ⟨hq, hcq, by rw [← hstk' q hq]; exact hp⟩
        · rintro ⟨hq, hcq, hp⟩; exact ⟨hq, hcq, by rw [hstk' q hq]; exact hp⟩
      have hnt : ∀ q, q < st.s.lock_len.val → stk st.s (ldep st.s q) ≠ t.val := by
        intro q hq e
        have := (w.on_stack t.val htn).mpr ⟨ldep st.s q, w.ldep_lt q hq, e⟩
        exact this hj0
      have hκ' : ksLow cfg i { st with
          s := s', run := Function.update st.run t.val (st.pend t.val),
          pend := Function.update st.pend t.val 0,
          ks := c, kp := t.val,
          rl := Function.update st.rl t.val (P.S t.val),
          ru := Function.update st.ru t.val (P.S t.val),
          re := Function.update st.re t.val (P.EP t.val),
          rb := Function.update st.rb t.val 1 } = 0 :=
        ksLow_of_not_low (fun h => by have := h.2; dsimp only at this; omega)
      unfold Blk
      refine Blkf.keep (fun p hp => ⟨p, Nat.le_refl _, (hbl p).mp hp⟩)
        (fun p0 _ hp0 _ _ => (hbl p0).mpr hp0) ?_ ?_ hblk
      · intro p0 hp0 _
        rw [hκ']; unfold bv; dsimp only
        rw [hld, hstk' p0 hp0.1, Function.update_of_ne (hnt p0 hp0.1)]
        omega
      · rw [hκ']; omega
  | lock b hm hne hk0 r a c s' hrun hrl hc ha hal hs =>
    obtain ⟨⟨hrel, hjob, hstk, hdep⟩, hd1, hl, hll, hlr, hld, hlc⟩ := lock_views hs
    have htopn : stk st.s (st.s.depth.val - 1) < NT.val := w.stk_lt _ (by omega)
    have hjt : job st.s (stk st.s (st.s.depth.val - 1)) = 1 := (w.run_iff _ htopn).mp hrun
    have hold : ∀ q, q < st.s.lock_len.val → (blocking cfg i s' q ↔ blocking cfg i st.s q) := by
      intro q hq
      unfold blocking
      rw [hll, hlr, hld]
      simp only [show q ≠ st.s.lock_len.val by omega, ite_false, hstk]
      constructor
      · rintro ⟨_, hcq, hp⟩; exact ⟨hq, hcq, hp⟩
      · rintro ⟨_, hcq, hp⟩; exact ⟨by omega, hcq, hp⟩
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inl hk0)
    have hκ : ksLow cfg i st = 0 := ksLow_zero hk0
    by_cases hlv : prio cfg i ≤ prio cfg (stk st.s (st.s.depth.val - 1))
    · have hnb : ∀ p, blocking cfg i st.s p → ¬ topEntry st.s p := by
        rintro p ⟨_, _, hp⟩ ⟨_, hd0⟩
        rw [hd0] at hp; omega
      refine Or.inr ⟨s0, β, hs0,
        Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, hthird, ?_⟩
      · intro hx hl'
        have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
        unfold wkJ
        rw [hk1]
        unfold cred remJ ksOf
        dsimp only
        rw [hrel, hjob]
        by_cases hxt : x = stk st.s (st.s.depth.val - 1)
        · subst hxt
          simp only [Function.update_self, hjt, ↓reduceIte]
          have := mul_pred_add (K := P.Kin) hrl hc
          omega
        · have hne' : ¬ (stk st.s (st.s.depth.val - 1) = x) := fun e => hxt e.symm
          simp only [Function.update_of_ne hxt, hne', ↓reduceIte]
          omega
      · intro hx hd
        unfold wkS ksOf
        dsimp only
        simp only [show ¬ (stk st.s (st.s.depth.val - 1) = x) by omega, ↓reduceIte]
        omega
      · have hκ' : ksLow cfg i { st with
            s := s',
            allow := Function.update (deduct st.s st.allow c) st.s.lock_len.val (a - c),
            ks := c, kp := stk st.s (st.s.depth.val - 1),
            rl := Function.update st.rl (stk st.s (st.s.depth.val - 1)) (st.rl (stk st.s (st.s.depth.val - 1)) - 1) } = 0 :=
          ksLow_of_not_low (fun h => by have := h.2; dsimp only at this; omega)
        unfold Blk
        refine Blkf.keep ?_ ?_ ?_ ?_ hblk
        · intro q hq
          by_cases hql : q = st.s.lock_len.val
          · exfalso
            have hq2 := hq.2.2
            rw [hld, hstk] at hq2
            simp only [hql, ite_true] at hq2
            omega
          · have hq1 := hq.1
            rw [hll] at hq1
            exact ⟨q, Nat.le_refl _, (hold q (by omega)).mp hq⟩
        · intro p0 _ hp0 _ _
          exact (hold p0 hp0.1).mpr hp0
        · intro p0 hp0 _
          have hpl : p0 ≠ st.s.lock_len.val := by have := hp0.1; omega
          rw [hκ']; unfold bv; dsimp only
          rw [Function.update_of_ne hpl, hld, hstk]
          simp only [hpl, ite_false]
          unfold deduct
          simp only [hnb p0 hp0, ↓reduceIte]
          omega
        · rw [hκ']; omega
    · have hlow : prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg i := by omega
      have hnq : ¬ Quiet cfg i st := by
        intro hQ; apply hq'
        exact ⟨hQ.1, Or.inr ⟨htopn, hlow⟩⟩
      obtain ⟨k, hk, hpos⟩ := level_of_not_quiet hnq hk0
      obtain ⟨_, ⟨pb, hpb⟩, hall⟩ := busy_block w hNT hprio hm hne hk hpos (Or.inr hlow)
      obtain ⟨p0, hbp0, hmin, hle0⟩ := hblk.2 pb hpb
      have hc0 := hal p0 ⟨hbp0.1, hall p0 hbp0⟩
      have hκ' : ksLow cfg i { st with
          s := s',
          allow := Function.update (deduct st.s st.allow c) st.s.lock_len.val (a - c),
          ks := c, kp := stk st.s (st.s.depth.val - 1),
          rl := Function.update st.rl (stk st.s (st.s.depth.val - 1)) (st.rl (stk st.s (st.s.depth.val - 1)) - 1) } = c := by
        unfold ksLow; split
        · rfl
        · rename_i hn; exact absurd ⟨htopn, hlow⟩ hn
      refine Or.inr ⟨s0, β, hs0,
        Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, hthird, ?_⟩
      · intro hx hl'
        have hxt : x ≠ stk st.s (st.s.depth.val - 1) := by
          intro e; rw [e] at hl'; omega
        have hne' : ¬ (stk st.s (st.s.depth.val - 1) = x) := fun e => hxt e.symm
        have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
        unfold wkJ
        rw [hk1]
        unfold cred remJ ksOf
        dsimp only
        rw [hrel, hjob]
        simp only [Function.update_of_ne hxt, hne', ↓reduceIte]
        omega
      · intro hx hd
        unfold wkS ksOf
        dsimp only
        simp only [show ¬ (stk st.s (st.s.depth.val - 1) = x) by omega, ↓reduceIte]
        omega
      · unfold Blk
        refine Blkf.keep ?_ ?_ ?_ ?_ hblk
        · intro q hq
          by_cases hql : q = st.s.lock_len.val
          · exact ⟨pb, by rw [hql]; exact Nat.le_of_lt hpb.1, hpb⟩
          · have hq1 := hq.1
            rw [hll] at hq1
            exact ⟨q, Nat.le_refl _, (hold q (by omega)).mp hq⟩
        · intro p1 _ hp1 _ _
          exact (hold p1 hp1.1).mpr hp1
        · intro p1 hp1 _
          have hpl : p1 ≠ st.s.lock_len.val := by have := hp1.1; omega
          have ht1 : topEntry st.s p1 := ⟨hp1.1, hall p1 hp1⟩
          have := hal p1 ht1
          rw [hκ', hκ]; unfold bv; dsimp only
          rw [Function.update_of_ne hpl, hld, hstk]
          simp only [hpl, ite_false]
          unfold deduct
          simp only [ht1, ↓reduceIte]
          omega
        · rw [hκ']; unfold bv at hle0; omega
  | unlock b hm hne hk0 r c s' hrun hru hc hal hs =>
    obtain ⟨⟨hrel, hjob, hstk, hdep⟩, hl1, hd1, _, _, hll, hlr, hld, hcl⟩ := unlock_views hs
    have htopn : stk st.s (st.s.depth.val - 1) < NT.val := w.stk_lt _ (by omega)
    have hjt : job st.s (stk st.s (st.s.depth.val - 1)) = 1 := (w.run_iff _ htopn).mp hrun
    have hbl : ∀ q, blocking cfg i s' q ↔ (blocking cfg i st.s q ∧ q < st.s.lock_len.val - 1) := by
      intro q
      unfold blocking
      rw [hll, hlr, hld, hstk]
      constructor
      · rintro ⟨hq, hcq, hp⟩; exact ⟨⟨by omega, hcq, hp⟩, hq⟩
      · rintro ⟨⟨_, hcq, hp⟩, hq⟩; exact ⟨hq, hcq, hp⟩
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inl hk0)
    have hκ : ksLow cfg i st = 0 := ksLow_zero hk0
    by_cases hlv : prio cfg i ≤ prio cfg (stk st.s (st.s.depth.val - 1))
    · have hnb : ∀ p, blocking cfg i st.s p → ¬ topEntry st.s p := by
        rintro p ⟨_, _, hp⟩ ⟨_, hd0⟩
        rw [hd0] at hp; omega
      refine Or.inr ⟨s0, β, hs0,
        Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, hthird, ?_⟩
      · intro hx hl'
        have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
        unfold wkJ
        rw [hk1]
        unfold cred remJ ksOf
        dsimp only
        rw [hrel, hjob]
        by_cases hxt : x = stk st.s (st.s.depth.val - 1)
        · subst hxt
          simp only [Function.update_self, hjt, ↓reduceIte]
          have := mul_pred_add (K := P.Kout) hru hc
          omega
        · have hne' : ¬ (stk st.s (st.s.depth.val - 1) = x) := fun e => hxt e.symm
          simp only [Function.update_of_ne hxt, hne', ↓reduceIte]
          omega
      · intro hx hd
        unfold wkS ksOf
        dsimp only
        simp only [show ¬ (stk st.s (st.s.depth.val - 1) = x) by omega, ↓reduceIte]
        omega
      · have hκ' : ksLow cfg i { st with
            s := s', allow := deduct st.s st.allow c,
            ks := c, kp := stk st.s (st.s.depth.val - 1),
            ru := Function.update st.ru (stk st.s (st.s.depth.val - 1)) (st.ru (stk st.s (st.s.depth.val - 1)) - 1) } = 0 :=
          ksLow_of_not_low (fun h => by have := h.2; dsimp only at this; omega)
        unfold Blk
        refine Blkf.keep (fun q hq => ⟨q, Nat.le_refl _, ((hbl q).mp hq).1⟩)
          (fun p0 q hp0 hq hle => (hbl p0).mpr ⟨hp0, by have := ((hbl q).mp hq).2; omega⟩)
          ?_ ?_ hblk
        · intro p0 hp0 _
          rw [hκ']; unfold bv; dsimp only
          rw [hld, hstk]
          unfold deduct
          simp only [hnb p0 hp0, ↓reduceIte]
          omega
        · rw [hκ']; omega
    · have hlow : prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg i := by omega
      have hnq : ¬ Quiet cfg i st := by
        intro hQ; apply hq'
        exact ⟨hQ.1, Or.inr ⟨htopn, hlow⟩⟩
      obtain ⟨k, hk, hpos⟩ := level_of_not_quiet hnq hk0
      obtain ⟨_, ⟨pb, hpb⟩, hall⟩ := busy_block w hNT hprio hm hne hk hpos (Or.inr hlow)
      obtain ⟨p0, hbp0, hmin, hle0⟩ := hblk.2 pb hpb
      have hc0 := hal p0 ⟨hbp0.1, hall p0 hbp0⟩
      have hκ' : ksLow cfg i { st with
          s := s', allow := deduct st.s st.allow c,
          ks := c, kp := stk st.s (st.s.depth.val - 1),
          ru := Function.update st.ru (stk st.s (st.s.depth.val - 1)) (st.ru (stk st.s (st.s.depth.val - 1)) - 1) } = c := by
        unfold ksLow; split
        · rfl
        · rename_i hn; exact absurd ⟨htopn, hlow⟩ hn
      refine Or.inr ⟨s0, β, hs0,
        Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, hthird, ?_⟩
      · intro hx hl'
        have hxt : x ≠ stk st.s (st.s.depth.val - 1) := by
          intro e; rw [e] at hl'; omega
        have hne' : ¬ (stk st.s (st.s.depth.val - 1) = x) := fun e => hxt e.symm
        have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
        unfold wkJ
        rw [hk1]
        unfold cred remJ ksOf
        dsimp only
        rw [hrel, hjob]
        simp only [Function.update_of_ne hxt, hne', ↓reduceIte]
        omega
      · intro hx hd
        unfold wkS ksOf
        dsimp only
        simp only [show ¬ (stk st.s (st.s.depth.val - 1) = x) by omega, ↓reduceIte]
        omega
      · unfold Blk
        refine Blkf.keep (fun q hq => ⟨q, Nat.le_refl _, ((hbl q).mp hq).1⟩)
          (fun p1 q hp1 hq hle => (hbl p1).mpr ⟨hp1, by have := ((hbl q).mp hq).2; omega⟩)
          ?_ ?_ hblk
        · intro p1 hp1 _
          have ht1 : topEntry st.s p1 := ⟨hp1.1, hall p1 hp1⟩
          have := hal p1 ht1
          rw [hκ', hκ]; unfold bv; dsimp only
          rw [hld, hstk]
          unfold deduct
          simp only [ht1, ↓reduceIte]
          omega
        · rw [hκ']; unfold bv at hle0; omega
  | svc b hm hne hk0 c hd hrun hre hal =>
    have htopn : stk st.s (st.s.depth.val - 1) < NT.val := w.stk_lt _ (by omega)
    have hjt : job st.s (stk st.s (st.s.depth.val - 1)) = 1 := (w.run_iff _ htopn).mp hrun
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inl hk0)
    have hκ : ksLow cfg i st = 0 := ksLow_zero hk0
    by_cases hlv : prio cfg i ≤ prio cfg (stk st.s (st.s.depth.val - 1))
    · have hnb : ∀ p, blocking cfg i st.s p → ¬ topEntry st.s p := by
        rintro p ⟨_, _, hp⟩ ⟨_, hd0⟩
        rw [hd0] at hp; omega
      refine Or.inr ⟨s0, β, hs0,
        Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, hthird, ?_⟩
      · intro hx hl'
        have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
        unfold wkJ
        rw [hk1]
        unfold cred remJ ksOf
        dsimp only
        by_cases hxt : x = stk st.s (st.s.depth.val - 1)
        · subst hxt
          simp only [Function.update_self, hjt, ↓reduceIte]
          omega
        · have hne' : ¬ (stk st.s (st.s.depth.val - 1) = x) := fun e => hxt e.symm
          simp only [Function.update_of_ne hxt, hne', ↓reduceIte]
          omega
      · intro hx hd'
        unfold wkS ksOf
        dsimp only
        simp only [show ¬ (stk st.s (st.s.depth.val - 1) = x) by omega, ↓reduceIte]
        omega
      · have hκ' : ksLow cfg i { st with
            allow := deduct st.s st.allow c,
            ks := c, kp := stk st.s (st.s.depth.val - 1),
            re := Function.update st.re (stk st.s (st.s.depth.val - 1)) (st.re (stk st.s (st.s.depth.val - 1)) - c) } = 0 :=
          ksLow_of_not_low (fun h => by have := h.2; dsimp only at this; omega)
        unfold Blk
        refine Blkf.keep (fun q hq => ⟨q, Nat.le_refl _, hq⟩) (fun p0 _ hp0 _ _ => hp0)
          ?_ ?_ hblk
        · intro p0 hp0 _
          rw [hκ']; unfold bv; dsimp only
          unfold deduct
          simp only [hnb p0 hp0, ↓reduceIte]
          omega
        · rw [hκ']; omega
    · have hlow : prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg i := by omega
      have hnq : ¬ Quiet cfg i st := by
        intro hQ; apply hq'
        exact ⟨hQ.1, Or.inr ⟨htopn, hlow⟩⟩
      obtain ⟨k, hk, hpos⟩ := level_of_not_quiet hnq hk0
      obtain ⟨_, ⟨pb, hpb⟩, hall⟩ := busy_block w hNT hprio hm hne hk hpos (Or.inr hlow)
      obtain ⟨p0, hbp0, hmin, hle0⟩ := hblk.2 pb hpb
      have hc0 := hal p0 ⟨hbp0.1, hall p0 hbp0⟩
      have hκ' : ksLow cfg i { st with
          allow := deduct st.s st.allow c,
          ks := c, kp := stk st.s (st.s.depth.val - 1),
          re := Function.update st.re (stk st.s (st.s.depth.val - 1)) (st.re (stk st.s (st.s.depth.val - 1)) - c) } = c := by
        unfold ksLow; split
        · rfl
        · rename_i hn; exact absurd ⟨htopn, hlow⟩ hn
      refine Or.inr ⟨s0, β, hs0,
        Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, hthird, ?_⟩
      · intro hx hl'
        have hxt : x ≠ stk st.s (st.s.depth.val - 1) := by
          intro e; rw [e] at hl'; omega
        have hne' : ¬ (stk st.s (st.s.depth.val - 1) = x) := fun e => hxt e.symm
        have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
        unfold wkJ
        rw [hk1]
        unfold cred remJ ksOf
        dsimp only
        simp only [Function.update_of_ne hxt, hne', ↓reduceIte]
        omega
      · intro hx hd'
        unfold wkS ksOf
        dsimp only
        simp only [show ¬ (stk st.s (st.s.depth.val - 1) = x) by omega, ↓reduceIte]
        omega
      · unfold Blk
        refine Blkf.keep (fun q hq => ⟨q, Nat.le_refl _, hq⟩) (fun p1 _ hp1 _ _ => hp1)
          ?_ ?_ hblk
        · intro p1 hp1 _
          have ht1 : topEntry st.s p1 := ⟨hp1.1, hall p1 hp1⟩
          have := hal p1 ht1
          rw [hκ', hκ]; unfold bv; dsimp only
          unfold deduct
          simp only [ht1, ↓reduceIte]
          omega
        · rw [hκ']; unfold bv at hle0; omega
  | bevent b hm hne hk0 c hd hrun hrb hc =>
    have htopn : stk st.s (st.s.depth.val - 1) < NT.val := w.stk_lt _ (by omega)
    have hjt : job st.s (stk st.s (st.s.depth.val - 1)) = 1 := (w.run_iff _ htopn).mp hrun
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inl hk0)
    have hκ : ksLow cfg i st = 0 := ksLow_zero hk0
    by_cases hlv : prio cfg i ≤ prio cfg (stk st.s (st.s.depth.val - 1))
    · refine Or.inr ⟨s0, β, hs0,
        Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, hthird, ?_⟩
      · intro hx hl'
        have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
        unfold wkJ
        rw [hk1]
        unfold cred remJ ksOf
        dsimp only
        by_cases hxt : x = stk st.s (st.s.depth.val - 1)
        · subst hxt
          simp only [Function.update_self, hjt, ↓reduceIte]
          have := mul_pred_add (K := P.Kt) hrb hc
          omega
        · have hne' : ¬ (stk st.s (st.s.depth.val - 1) = x) := fun e => hxt e.symm
          simp only [Function.update_of_ne hxt, hne', ↓reduceIte]
          omega
      · intro hx hd'
        unfold wkS ksOf
        dsimp only
        simp only [show ¬ (stk st.s (st.s.depth.val - 1) = x) by omega, ↓reduceIte]
        omega
      · have hκ' : ksLow cfg i { st with
            ks := c, kp := stk st.s (st.s.depth.val - 1),
            rb := Function.update st.rb (stk st.s (st.s.depth.val - 1)) (st.rb (stk st.s (st.s.depth.val - 1)) - 1) } = 0 :=
          ksLow_of_not_low (fun h => by have := h.2; dsimp only at this; omega)
        unfold Blk
        refine Blkf.keep (fun q hq => ⟨q, Nat.le_refl _, hq⟩) (fun p0 _ hp0 _ _ => hp0)
          ?_ ?_ hblk
        · intro p0 hp0 _
          have hne' : stk st.s (ldep st.s p0) ≠ stk st.s (st.s.depth.val - 1) := by
            intro e; have := hp0.2.2; rw [e] at this; omega
          rw [hκ']; unfold bv; dsimp only
          rw [Function.update_of_ne hne']
          omega
        · rw [hκ']; omega
    · have hlow : prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg i := by omega
      have hnq : ¬ Quiet cfg i st := by
        intro hQ; apply hq'
        exact ⟨hQ.1, Or.inr ⟨htopn, hlow⟩⟩
      obtain ⟨k, hk, hpos⟩ := level_of_not_quiet hnq hk0
      obtain ⟨_, ⟨pb, hpb⟩, hall⟩ := busy_block w hNT hprio hm hne hk hpos (Or.inr hlow)
      obtain ⟨p0, hbp0, hmin, hle0⟩ := hblk.2 pb hpb
      have hκ' : ksLow cfg i { st with
          ks := c, kp := stk st.s (st.s.depth.val - 1),
          rb := Function.update st.rb (stk st.s (st.s.depth.val - 1)) (st.rb (stk st.s (st.s.depth.val - 1)) - 1) } = c := by
        unfold ksLow; split
        · rfl
        · rename_i hn; exact absurd ⟨htopn, hlow⟩ hn
      have hpay := mul_pred_add' (K := P.Kt) hrb hc
      refine Or.inr ⟨s0, β, hs0,
        Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, hthird, ?_⟩
      · intro hx hl'
        have hxt : x ≠ stk st.s (st.s.depth.val - 1) := by
          intro e; rw [e] at hl'; omega
        have hne' : ¬ (stk st.s (st.s.depth.val - 1) = x) := fun e => hxt e.symm
        have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
        unfold wkJ
        rw [hk1]
        unfold cred remJ ksOf
        dsimp only
        simp only [Function.update_of_ne hxt, hne', ↓reduceIte]
        omega
      · intro hx hd'
        unfold wkS ksOf
        dsimp only
        simp only [show ¬ (stk st.s (st.s.depth.val - 1) = x) by omega, ↓reduceIte]
        omega
      · unfold Blk
        refine Blkf.keep (fun q hq => ⟨q, Nat.le_refl _, hq⟩) (fun p1 _ hp1 _ _ => hp1)
          ?_ ?_ hblk
        · intro p1 hp1 _
          rw [hκ', hκ]; unfold bv; dsimp only
          rw [hall p1 hp1]
          simp only [Function.update_self]
          omega
        · have h1 := hall p0 hbp0
          rw [hκ']; unfold bv at hle0; rw [h1] at hle0; omega
  | idle b hm hne hk0 hj hd =>
    exfalso
    obtain ⟨k, hk, hpos⟩ := level_of_not_quiet hq' hk0
    have := (busy_block w hNT hprio hm hne hk hpos (Or.inl hd)).1
    omega
  | exec b hm hne hk0 hj hd hr hal =>
    have htopn : stk st.s (st.s.depth.val - 1) < NT.val := w.stk_lt _ (by omega)
    have hle : ∀ k, st.pend k + (Function.update st.run (stk st.s (st.s.depth.val - 1)) (st.run (stk st.s (st.s.depth.val - 1)) - 1)) k
        ≤ st.pend k + st.run k := by
      intro k
      by_cases hkt : k = stk st.s (st.s.depth.val - 1)
      · subst hkt; simp only [Function.update_self]; omega
      · rw [Function.update_of_ne hkt]
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inl hk0)
    have hκ : ksLow cfg i st = 0 := ksLow_zero hk0
    have hκ' : ksLow cfg i { st with
        now := st.now + 1,
        run := Function.update st.run (stk st.s (st.s.depth.val - 1)) (st.run (stk st.s (st.s.depth.val - 1)) - 1),
        allow := deduct st.s st.allow 1 } = 0 := ksLow_zero hk0
    by_cases hlv : prio cfg i ≤ prio cfg (stk st.s (st.s.depth.val - 1))
    · refine Or.inr ⟨s0, β, by dsimp only; omega, ?_, ?_, ?_⟩
      · dsimp only
        refine Nat.le_trans (Phi.tick hs0 (fun x _ => wkOf_le ?_ ?_)
          (Or.inl ⟨⟨stk st.s (st.s.depth.val - 1), Finset.mem_range.mpr (by omega), ?_⟩, Nat.le_refl _⟩)) hphi
        · intro hx hl'
          unfold wkJ cred remJ ksOf
          dsimp only
          by_cases hxt : x = stk st.s (st.s.depth.val - 1)
          · subst hxt; simp only [Function.update_self]; omega
          · simp only [Function.update_of_ne hxt]; omega
        · intro hx hd'
          unfold wkS ksOf
          dsimp only
          omega
        · unfold wkOf
          simp only [htopn, ite_true, hlv]
          unfold wkJ cred remJ ksOf
          dsimp only
          simp only [Function.update_self]
          omega
      · dsimp only
        intro hx
        exact hthird (Nat.lt_of_lt_of_le hx (hle i))
      · unfold Blk
        refine Blkf.keep (bl := blocking cfg i st.s) (fun q hq => ⟨q, Nat.le_refl _, hq⟩) (fun p0 _ hp0 _ _ => hp0)
          ?_ ?_ hblk
        · intro p0 hp0 _
          have hnt : ¬ topEntry st.s p0 := by
            rintro ⟨_, hd0⟩
            have := hp0.2.2
            rw [hd0] at this
            omega
          rw [hκ', hκ]; unfold bv; dsimp only
          unfold deduct
          simp only [hnt, ↓reduceIte]
          omega
        · rw [hκ']; omega
    · have hlow : prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg i := by omega
      obtain ⟨k, hk, hpos'⟩ := level_of_not_quiet hq' hk0
      have hpos : 0 < st.pend k + st.run k := Nat.lt_of_lt_of_le hpos' (hle k)
      obtain ⟨_, ⟨pb, hpb⟩, hall⟩ := busy_block w hNT hprio hm hne hk hpos (Or.inr hlow)
      obtain ⟨p0, hbp0, hmin, hle0⟩ := hblk.2 pb hpb
      have hpos0 : 0 < st.allow p0 := hal p0 ⟨hbp0.1, hall p0 hbp0⟩
      refine Or.inr ⟨s0, β - 1, by dsimp only; omega, ?_, ?_, ?_⟩
      · dsimp only
        refine Nat.le_trans (Phi.tick hs0 (fun x _ => wkOf_le ?_ ?_)
          (Or.inr (by unfold bv at hle0; omega))) hphi
        · intro hx hl'
          unfold wkJ cred remJ ksOf
          dsimp only
          by_cases hxt : x = stk st.s (st.s.depth.val - 1)
          · subst hxt; simp only [Function.update_self]; omega
          · simp only [Function.update_of_ne hxt]; omega
        · intro hx hd'
          unfold wkS ksOf
          dsimp only
          omega
      · dsimp only
        intro hx
        exact hthird (Nat.lt_of_lt_of_le hx (hle i))
      · unfold Blk
        refine Blkf.trans (bl := blocking cfg i st.s) (δ := 1) (fun q hq => ⟨q, Nat.le_refl _, hq⟩)
          (fun p1 _ hp1 _ _ => hp1) ?_ ?_ hblk
        · intro p1 hp1 _
          have ht1 : topEntry st.s p1 := ⟨hp1.1, hall p1 hp1⟩
          have := hal p1 ht1
          rw [hκ', hκ]; unfold bv; dsimp only
          unfold deduct
          simp only [ht1, ↓reduceIte]
          omega
        · rw [hκ']; unfold bv at hle0; omega
  | finish b hm hne hk0 hj ov c r s' hd hr0 hr hov hc hal hs =>
    obtain ⟨h1, h2, h3, hrel, hjob, hstk, hdep, hlr, hld, hn, hpop, hkeep, hcl⟩ :=
      finish_top_views hs
    have hjt : job st.s (stk st.s (st.s.depth.val - 1)) = 1 := (w.run_iff _ h3).mp hr0
    have hle : ∀ k, st.pend k + (Function.update st.run (stk st.s (st.s.depth.val - 1)) 0) k ≤ st.pend k + st.run k := by
      intro k
      by_cases hkt : k = stk st.s (st.s.depth.val - 1)
      · subst hkt; simp only [Function.update_self]; omega
      · rw [Function.update_of_ne hkt]
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
      · rintro ⟨hq, hcq, hp⟩; exact ⟨⟨by omega, hcq, hp⟩, hq⟩
      · rintro ⟨⟨_, hcq, hp⟩, hq⟩; exact ⟨hq, hcq, hp⟩
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inl hk0)
    have hκ : ksLow cfg i st = 0 := ksLow_zero hk0
    have hcK : c ≤ st.rb (stk st.s (st.s.depth.val - 1)) * P.Kt + P.Kc + ovx P (stk st.s (st.s.depth.val - 1)) := by
      cases ov with
      | false => simp only [Bool.false_eq_true, ite_false] at hc; omega
      | true =>
        simp only [ite_true] at hc
        obtain ⟨hst, hrb⟩ := hov rfl
        have h1 : P.Kt ≤ st.rb (stk st.s (st.s.depth.val - 1)) * P.Kt := Nat.le_mul_of_pos_left _ hrb
        unfold ovx
        rw [hst]
        simp only [ite_true]
        omega
    by_cases hlv : prio cfg i ≤ prio cfg (stk st.s (st.s.depth.val - 1))
    · have hκ' : ksLow cfg i { st with
          s := s', now := st.now + 1,
          run := Function.update st.run (stk st.s (st.s.depth.val - 1)) 0,
          ks := c, kp := stk st.s (st.s.depth.val - 1) } = 0 :=
        ksLow_of_not_low (fun h => by have := h.2; dsimp only at this; omega)
      refine Or.inr ⟨s0, β, by dsimp only; omega, ?_, ?_, ?_⟩
      · dsimp only
        refine Nat.le_trans (Phi.tick hs0 (fun x _ => wkOf_le ?_ ?_)
          (Or.inl ⟨⟨stk st.s (st.s.depth.val - 1), Finset.mem_range.mpr (by omega), ?_⟩, Nat.le_refl _⟩)) hphi
        · intro hx hl'
          have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
          unfold wkJ
          rw [hk1]
          unfold cred remJ ksOf
          dsimp only
          rw [hrel, hjob]
          by_cases hxt : x = stk st.s (st.s.depth.val - 1)
          · subst hxt
            simp only [Function.update_self, hjt, ↓reduceIte, show ¬ ((0 : Nat) = 1) by omega]
            omega
          · have hne' : ¬ (stk st.s (st.s.depth.val - 1) = x) := fun e => hxt e.symm
            simp only [Function.update_of_ne hxt, hxt, hne', ↓reduceIte]
            omega
        · intro hx hd'
          unfold wkS ksOf
          dsimp only
          simp only [show ¬ (stk st.s (st.s.depth.val - 1) = x) by omega, ↓reduceIte]
          omega
        · have hk1 : ksOf st (stk st.s (st.s.depth.val - 1)) = 0 := ksOf_zero hk0 _
          unfold wkOf
          simp only [h3, ite_true, hlv]
          unfold wkJ
          rw [hk1]
          unfold cred remJ ksOf
          dsimp only
          rw [hrel, hjob]
          simp only [Function.update_self, hjt, ↓reduceIte, show ¬ ((0 : Nat) = 1) by omega]
          omega
      · dsimp only
        intro hx
        exact hthird (Nat.lt_of_lt_of_le hx (hle i))
      · unfold Blk
        refine Blkf.keep (fun q hq => ⟨q, Nat.le_refl _, ((hbl q).mp hq).1⟩)
          (fun p0 q hp0 hq hle' => (hbl p0).mpr ⟨hp0, by have := ((hbl q).mp hq).2; omega⟩)
          ?_ ?_ hblk
        · intro p0 hp0 _
          rw [hκ', hκ]; unfold bv; dsimp only
          rw [hld, hstk]
        · rw [hκ']; omega
    · have hlow : prio cfg (stk st.s (st.s.depth.val - 1)) < prio cfg i := by omega
      have hnq : ¬ Quiet cfg i st := by
        intro hQ; apply hq'
        refine ⟨?_, Or.inr ⟨h3, hlow⟩⟩
        intro k hk
        have := hQ.1 k hk
        have := hle k
        dsimp only
        omega
      obtain ⟨k, hk, hpos⟩ := level_of_not_quiet hnq hk0
      obtain ⟨_, ⟨pb, hpb⟩, hall⟩ := busy_block w hNT hprio hm hne hk hpos (Or.inr hlow)
      obtain ⟨p0, hbp0, hmin, hle0⟩ := hblk.2 pb hpb
      have hc0 := hal p0 ⟨hbp0.1, hall p0 hbp0⟩
      have hκ' : ksLow cfg i { st with
          s := s', now := st.now + 1,
          run := Function.update st.run (stk st.s (st.s.depth.val - 1)) 0,
          ks := c, kp := stk st.s (st.s.depth.val - 1) } = c := by
        unfold ksLow; split
        · rfl
        · rename_i hn; exact absurd ⟨h3, hlow⟩ hn
      have hnone : ∀ q, ¬ blocking cfg i s' q := by
        intro q hq
        obtain ⟨hbq, hqn⟩ := (hbl q).mp hq
        have := hall q hbq
        have := hbelow q hqn
        omega
      refine Or.inr ⟨s0, β - 1, by dsimp only; omega, ?_, ?_, ?_⟩
      · dsimp only
        refine Nat.le_trans (Phi.tick hs0 (fun x _ => wkOf_le ?_ ?_)
          (Or.inr (by unfold bv at hle0; omega))) hphi
        · intro hx hl'
          have hxt : x ≠ stk st.s (st.s.depth.val - 1) := by
            intro e; rw [e] at hl'; omega
          have hne' : ¬ (stk st.s (st.s.depth.val - 1) = x) := fun e => hxt e.symm
          have hk1 : ksOf st x = 0 := ksOf_zero hk0 x
          unfold wkJ
          rw [hk1]
          unfold cred remJ ksOf
          dsimp only
          rw [hrel, hjob]
          simp only [Function.update_of_ne hxt, hxt, hne', ↓reduceIte]
          omega
        · intro hx hd'
          unfold wkS ksOf
          dsimp only
          simp only [show ¬ (stk st.s (st.s.depth.val - 1) = x) by omega, ↓reduceIte]
          omega
      · dsimp only
        intro hx
        exact hthird (Nat.lt_of_lt_of_le hx (hle i))
      · unfold Blk
        refine Blkf.trans (δ := 1) (fun q hq => absurd hq (hnone q))
          (fun _ q _ hq _ => absurd hq (hnone q)) (fun p1 _ hp1 => absurd hp1 (hnone p1)) ?_ hblk
        rw [hκ']; unfold bv at hle0; omega
  | kend r s' hs =>
    obtain ⟨h1, h2, h3, hrel, hjob, hstk, hdep, hlr, hld, hn, hpop, hkeep, hcl⟩ :=
      finish_top_views hs
    have hle : ∀ k, st.pend k + (Function.update st.run (stk st.s (st.s.depth.val - 1)) 0) k ≤ st.pend k + st.run k := by
      intro k
      by_cases hkt : k = stk st.s (st.s.depth.val - 1)
      · subst hkt; simp only [Function.update_self]; omega
      · rw [Function.update_of_ne hkt]
    have hnq : ¬ Quiet cfg i st := by
      intro hQ; apply hq'
      refine ⟨?_, hQ.2⟩
      intro k hk
      have := hQ.1 k hk
      have := hle k
      dsimp only
      omega
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inr hnq)
    have hbl : ∀ q, blocking cfg i s' q ↔ (blocking cfg i st.s q ∧ q < s'.lock_len.val) := by
      intro q
      unfold blocking
      rw [hlr, hld, hstk]
      constructor
      · rintro ⟨hq, hcq, hp⟩; exact ⟨⟨by omega, hcq, hp⟩, hq⟩
      · rintro ⟨⟨_, hcq, hp⟩, hq⟩; exact ⟨hq, hcq, hp⟩
    have hκ' : ksLow cfg i { st with
        s := s', run := Function.update st.run (stk st.s (st.s.depth.val - 1)) 0 } = ksLow cfg i st := rfl
    refine Or.inr ⟨s0, β, hs0,
      Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, ?_, ?_⟩
    · intro hx hl'
      unfold wkJ cred remJ ksOf
      dsimp only
      rw [hrel, hjob]
      by_cases hxt : x = stk st.s (st.s.depth.val - 1)
      · subst hxt
        simp only [Function.update_self, ↓reduceIte, show ¬ ((0 : Nat) = 1) by omega]
        omega
      · simp only [Function.update_of_ne hxt, hxt, ↓reduceIte]
        omega
    · intro hx hd'
      unfold wkS ksOf
      dsimp only
      omega
    · dsimp only
      intro hx
      exact hthird (Nat.lt_of_lt_of_le hx (hle i))
    · unfold Blk
      refine Blkf.keep (fun q hq => ⟨q, Nat.le_refl _, ((hbl q).mp hq).1⟩)
        (fun p0 q hp0 hq hle' => (hbl p0).mpr ⟨hp0, by have := ((hbl q).mp hq).2; omega⟩)
        ?_ ?_ hblk
      · intro p0 hp0 _
        rw [hκ']; unfold bv; dsimp only
        rw [hld, hstk]
      · rw [hκ']; exact hblk.1
  | kkill p s' hp hs =>
    obtain ⟨ht, hrel, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := kill_views hs
    have hle : ∀ k, st.pend k + (Function.update st.run (stk st.s p.val) 0) k
        ≤ st.pend k + st.run k := by
      intro k
      by_cases hkt : k = stk st.s p.val
      · subst hkt; simp only [Function.update_self]; omega
      · rw [Function.update_of_ne hkt]
    have hnq : ¬ Quiet cfg i st := by
      intro hQ; apply hq'
      refine ⟨?_, hQ.2⟩
      intro k hk
      have := hQ.1 k hk
      have := hle k
      dsimp only
      omega
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inr hnq)
    have hbl : ∀ q, blocking cfg i s' q ↔ blocking cfg i st.s q := by
      intro q; unfold blocking; rw [hll, hlr, hld, hstk]
    have hκ' : ksLow cfg i { st with
        s := s', run := Function.update st.run (stk st.s p.val) 0 } = ksLow cfg i st := rfl
    refine Or.inr ⟨s0, β, hs0,
      Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, ?_, ?_⟩
    · intro hx hl'
      unfold wkJ cred remJ ksOf
      dsimp only
      rw [hrel, hjob]
      by_cases hxt : x = stk st.s p.val
      · subst hxt
        simp only [Function.update_self, ↓reduceIte, show ¬ ((2 : Nat) = 1) by omega]
        omega
      · simp only [Function.update_of_ne hxt, hxt, ↓reduceIte]
        omega
    · intro hx hd'
      unfold wkS ksOf
      dsimp only
      omega
    · dsimp only
      intro hx
      exact hthird (Nat.lt_of_lt_of_le hx (hle i))
    · unfold Blk
      refine Blkf.keep (fun q hq => ⟨q, Nat.le_refl _, (hbl q).mp hq⟩)
        (fun p0 _ hp0 _ _ => (hbl p0).mpr hp0) ?_ ?_ hblk
      · intro p0 hp0 _
        rw [hκ']; unfold bv; dsimp only
        rw [hld, hstk]
      · rw [hκ']; exact hblk.1
  | kclear t s' hs =>
    obtain ⟨ht, hrl, hjob, hstk, hdep, hlr, hld, hll, hlc⟩ := clear_release_views hs
    have hle : ∀ k, (Function.update st.pend t.val 0) k + st.run k ≤ st.pend k + st.run k := by
      intro k
      by_cases hkt : k = t.val
      · subst hkt; simp only [Function.update_self]; omega
      · rw [Function.update_of_ne hkt]
    have hnq : ¬ Quiet cfg i st := by
      intro hQ; apply hq'
      refine ⟨?_, hQ.2⟩
      intro k hk
      have := hQ.1 k hk
      have := hle k
      dsimp only
      omega
    obtain ⟨s0, β, hs0, hlt, hphi, hthird, hblk⟩ := hex (Or.inr hnq)
    have hbl : ∀ q, blocking cfg i s' q ↔ blocking cfg i st.s q := by
      intro q; unfold blocking; rw [hll, hlr, hld, hstk]
    have hκ' : ksLow cfg i { st with
        s := s', pend := Function.update st.pend t.val 0 } = ksLow cfg i st := rfl
    refine Or.inr ⟨s0, β, hs0,
      Nat.le_trans (Phi.mono (fun x _ => wkOf_le ?_ ?_) (fun _ _ => rfl)) hphi, ?_, ?_⟩
    · intro hx hl'
      unfold wkJ cred remJ ksOf
      dsimp only
      rw [hrl, hjob]
      by_cases hxt : x = t.val
      · subst hxt
        simp only [Function.update_self, ↓reduceIte, show ¬ ((0 : Nat) = 1) by omega]
        omega
      · simp only [Function.update_of_ne hxt, hxt, ↓reduceIte]
        omega
    · intro hx hd'
      unfold wkS ksOf
      dsimp only
      omega
    · dsimp only
      intro hx
      exact hthird (Nat.lt_of_lt_of_le hx (hle i))
    · unfold Blk
      refine Blkf.keep (fun q hq => ⟨q, Nat.le_refl _, (hbl q).mp hq⟩)
        (fun p0 _ hp0 _ _ => (hbl p0).mpr hp0) ?_ ?_ hblk
      · intro p0 hp0 _
        rw [hκ']; unfold bv; dsimp only
        rw [hld, hstk]
      · rw [hκ']; exact hblk.1

/-- No Kernel section runs on the account of an event source `dead` marks:
such a source's entry is that of a deadline event the Job never misses. -/
theorem ND.step {cfg : logic.Config NT NR NP} {P : Params} {dead : Nat → Bool}
    (hdead : ∀ q, dead q = true → P.relOf q = none ∧ ∃ t, P.dlOf q = some t ∧
      ∀ st : St NT NR NL, Reach cfg P st → ∀ a, st.jl t = some a →
        (0 < st.pend t + st.run t ∨ st.jwait t = true) → st.now < a + P.D t)
    {st st' : St NT NR NL} (hreach : Reach cfg P st) (w : Wf cfg P st) (hnd : ND dead st)
    (hs : Step cfg P st st') : ND dead st' := by
  cases hs with
  | jarrive t ht hsp => exact hnd
  | earrive q hq hsp => exact hnd
  | erel q t d c s' hq hk0 hw hrel hj hd1 hd2 hc hs =>
    intro _ q' hq'
    have he : q' = q := by dsimp only at hq'; omega
    cases hd : dead q' with
    | false => rfl
    | true => rw [he] at hd; have := (hdead q hd).1; rw [this] at hrel; cases hrel
  | eplain q c hq hk0 hw hc hdl =>
    intro _ q' hq'
    have he : q' = q := by dsimp only at hq'; omega
    cases hd : dead q' with
    | false => rfl
    | true =>
      rw [he] at hd
      obtain ⟨_, t, ht, hall⟩ := hdead q hd
      obtain ⟨a, ha, hle, hun⟩ := hdl t ht
      have := hall st hreach a ha hun
      omega
  | edefer q t c s' hq hk0 hw hrel hc hr0 hs =>
    intro _ q' hq'
    have he : q' = q := by dsimp only at hq'; omega
    cases hd : dead q' with
    | false => rfl
    | true => rw [he] at hd; have := (hdead q hd).1; rw [this] at hrel; cases hrel
  | ktick hk0 hj =>
    intro hx q hq
    exact hnd hk0 q hq
  | dispatch b t c s' hk0 hm he hmax hc hs =>
    intro _ q hq
    exfalso
    have := (start_views hs).1
    dsimp only at hq
    omega
  | lock b hm hne hk0 r a c s' hrun hrl hc ha hal hs =>
    intro _ q hq
    exfalso
    have := w.stk_lt (st.s.depth.val - 1) (by have := (lock_views hs).2.1; omega)
    dsimp only at hq
    omega
  | unlock b hm hne hk0 r c s' hrun hru hc hal hs =>
    intro _ q hq
    exfalso
    have := w.stk_lt (st.s.depth.val - 1) (by have := (unlock_views hs).2.2.1; omega)
    dsimp only at hq
    omega
  | svc b hm hne hk0 c hd hrun hre hal =>
    intro _ q hq
    exfalso
    have := w.stk_lt (st.s.depth.val - 1) (by omega)
    dsimp only at hq
    omega
  | bevent b hm hne hk0 c hd hrun hrb hc =>
    intro _ q hq
    exfalso
    have := w.stk_lt (st.s.depth.val - 1) (by omega)
    dsimp only at hq
    omega
  | idle b hm hne hk0 hj hd => exact hnd
  | exec b hm hne hk0 hj hd hr hal => exact hnd
  | finish b hm hne hk0 hj ov c r s' hd hr0 hr hov hc hal hs =>
    intro _ q hq
    exfalso
    have := w.stk_lt (st.s.depth.val - 1) (by omega)
    dsimp only at hq
    omega
  | kend r s' hs => exact hnd
  | kkill p s' hp hs => exact hnd
  | kclear t s' hs => exact hnd

/-- The three invariants hold in every state a run reaches. -/
theorem reach_inv {cfg : logic.Config NT NR NP} {P : Params} {i R B : Nat} {dead : Nat → Bool}
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    (hi : i < NT.val) (hR : 0 < R)
    (hT : ∀ x, x < NT.val + P.NQ → 0 < perOf cfg P x)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i → prio cfg i ≤ ceil cfg r → P.L k r ≤ B)
    (hFix : B + P.Kt + ∑ x ∈ Finset.range (NT.val + P.NQ),
      costOf cfg P i dead x * ceilDiv (R + jitOf cfg P x) (perOf cfg P x) ≤ R)
    (hdead : ∀ q, dead q = true → P.relOf q = none ∧ ∃ t, P.dlOf q = some t ∧
      ∀ st : St NT NR NL, Reach cfg P st → ∀ a, st.jl t = some a →
        (0 < st.pend t + st.run t ∨ st.jwait t = true) → st.now < a + P.D t)
    {st : St NT NR NL} (hreach : Reach cfg P st) :
    Wf cfg P st ∧ LInv cfg P R i dead st ∧ ND dead st := by
  induction hreach with
  | boot s0 h =>
    exact ⟨Wf.boot h, Or.inl ⟨fun _ _ => rfl, Or.inl rfl⟩, fun hx => absurd hx (Nat.lt_irrefl 0)⟩
  | step st st' hr hs ih =>
    exact ⟨ih.1.step hs,
      level_step hNT hprio hi hR hT hB hFix hdead hr ih.1 ih.2.2 hs ih.2.1,
      ND.step hdead hr ih.1 ih.2.2 hs⟩

/-- **The bound over the run.** `B` bounds every Critical_Section of a Task
of lower Priority than `i` on a Resource whose Ceiling is at least `i`'s
Priority, `dead` marks event sources that are deadline events of Tasks
whose Jobs are never incomplete at their deadline, and `R > 0` satisfies

`B + Kt + Σ_{k : P_k ≥ P_i} (C_k + O_k) · ⌈(R + J_k) / T_k⌉ + Σ_q EC_q · ⌈(R + EJ_q) / ET_q⌉ ≤ R`

(the first sum includes `i`; the second is over the event sources `dead`
does not mark; `O_k` is `Ojob`, the Kernel time one Job of `k` may take
from its dispatch on). Then in every state a run reaches in which Task `i`
has a released Job with execution left, the nominal instant `a` of its
Release has `now < a + J_i + R`. (Time is counted in units; a Job that
executes its last unit in the unit starting at `now` has `now + 1` at most
`a + J_i + R`.) -/
theorem stage4 {cfg : logic.Config NT NR NP} {P : Params} {i R B : Nat} {dead : Nat → Bool}
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    (hi : i < NT.val) (hR : 0 < R)
    (hT : ∀ x, x < NT.val + P.NQ → 0 < perOf cfg P x)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i → prio cfg i ≤ ceil cfg r → P.L k r ≤ B)
    (hFix : B + P.Kt + ∑ x ∈ Finset.range (NT.val + P.NQ),
      costOf cfg P i dead x * ceilDiv (R + jitOf cfg P x) (perOf cfg P x) ≤ R)
    (hdead : ∀ q, dead q = true → P.relOf q = none ∧ ∃ t, P.dlOf q = some t ∧
      ∀ st : St NT NR NL, Reach cfg P st → ∀ a, st.jl t = some a →
        (0 < st.pend t + st.run t ∨ st.jwait t = true) → st.now < a + P.D t)
    {st : St NT NR NL} (hreach : Reach cfg P st) (hpos : 0 < st.pend i + st.run i) :
    ∃ a, st.jl i = some a ∧ st.now < a + P.J i + R := by
  obtain ⟨w, hl, hnd⟩ := reach_inv hNT hprio hi hR hT hB hFix hdead hreach
  have hil : i ∈ lvl cfg i := mem_lvl.mpr ⟨hi, Nat.le_refl _⟩
  rcases hl with hq | ⟨s0, β, hs0, hphi, hthird, _⟩
  · have := hq.1 i hil; omega
  · obtain ⟨a, ha, hsa⟩ := hthird hpos
    have hwk : 0 < wkOf cfg P i dead st i := by
      unfold wkOf
      simp only [hi, ite_true, Nat.le_refl]
      unfold wkJ
      omega
    have := Phi.busy_lt (Finset.mem_range.mpr (by omega)) hwk hphi
    exact ⟨a, ha, by omega⟩

/-- A Release of Task `i` that still awaits its Kernel entry is within its
jitter. -/
theorem stage4_wait {cfg : logic.Config NT NR NP} {P : Params} {i R B : Nat} {dead : Nat → Bool}
    (hNT : NT.val ≤ 256) (hprio : ∀ k, k < NT.val → 0 < prio cfg k)
    (hi : i < NT.val) (hR : 0 < R)
    (hT : ∀ x, x < NT.val + P.NQ → 0 < perOf cfg P x)
    (hB : ∀ k r, k < NT.val → prio cfg k < prio cfg i → prio cfg i ≤ ceil cfg r → P.L k r ≤ B)
    (hFix : B + P.Kt + ∑ x ∈ Finset.range (NT.val + P.NQ),
      costOf cfg P i dead x * ceilDiv (R + jitOf cfg P x) (perOf cfg P x) ≤ R)
    (hdead : ∀ q, dead q = true → P.relOf q = none ∧ ∃ t, P.dlOf q = some t ∧
      ∀ st : St NT NR NL, Reach cfg P st → ∀ a, st.jl t = some a →
        (0 < st.pend t + st.run t ∨ st.jwait t = true) → st.now < a + P.D t)
    {st : St NT NR NL} (hreach : Reach cfg P st) (hw : st.jwait i = true) :
    ∃ a, st.jl i = some a ∧ st.now ≤ a + P.J i :=
  (reach_inv hNT hprio hi hR hT hB hFix hdead hreach).1.jwait_inv i hw

/-- The structure invariant holds in every state a run reaches. -/
theorem reach_wf {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (hreach : Reach cfg P st) : Wf cfg P st := by
  induction hreach with
  | boot s0 h => exact Wf.boot h
  | step st st' _ hs ih => exact ih.step hs

theorem costOf_job {cfg : logic.Config NT NR NP} {P : Params} {i : Nat} {dead : Nat → Bool}
    {k : Nat} (hk : k < NT.val) :
    costOf cfg P i dead k = if prio cfg i ≤ prio cfg k then P.C k + Ojob P k else 0 := by
  unfold costOf; simp only [hk, ite_true]

theorem costOf_src {cfg : logic.Config NT NR NP} {P : Params} {i : Nat} {dead : Nat → Bool}
    (q : Nat) : costOf cfg P i dead (NT.val + q) = if dead q then 0 else P.EC q := by
  unfold costOf
  simp only [show ¬ (NT.val + q < NT.val) by omega, ite_false, Nat.add_sub_cancel_left]

theorem perOf_job {cfg : logic.Config NT NR NP} {P : Params} {k : Nat} (hk : k < NT.val) :
    perOf cfg P k = P.T k := by
  unfold perOf; simp only [hk, ite_true]

theorem perOf_src {cfg : logic.Config NT NR NP} {P : Params} (q : Nat) :
    perOf cfg P (NT.val + q) = P.ET q := by
  unfold perOf
  simp only [show ¬ (NT.val + q < NT.val) by omega, ite_false, Nat.add_sub_cancel_left]

theorem jitOf_job {cfg : logic.Config NT NR NP} {P : Params} {k : Nat} (hk : k < NT.val) :
    jitOf cfg P k = P.J k := by
  unfold jitOf; simp only [hk, ite_true]

theorem jitOf_src {cfg : logic.Config NT NR NP} {P : Params} (q : Nat) :
    jitOf cfg P (NT.val + q) = P.EJ q := by
  unfold jitOf
  simp only [show ¬ (NT.val + q < NT.val) by omega, ite_false, Nat.add_sub_cancel_left]

end P18
