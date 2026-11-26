import Sched.Restore
open Aeneas Aeneas.Std Result

/-! # The interrupt controller and the scheduler (2026-11-25)

The Hardware_Model's statements about the NVIC (`rsk_logic::hw_model`,
HW-01, HW-02, HW-03, HW-05), which Verus proves against the scheduler
state of the source, here against the scheduler state of the compiled
crate as Aeneas extracts it. The model itself is a specification and has
no compiled form: it is restated below, definition for definition
(`encodeLevel` is `encode_level`, `execPrio` is `execution_priority`,
`nvicTakes`, `basepriFor`, `nvicSelects`, `jobLevel`, `dispatchExec` is
`dispatch_exec_priority`), and that the two texts say the same is by
reading. What is proven is proven of the extracted `Sched`:

- `dispatch_takes_iff_elig` (`lemma_dispatch_takes_iff_eligible`,
  Property 1's start condition, R7.1): with no exception active, PRIMASK
  clear and BASEPRI the value `basepriFor` gives for the executing Job's
  level and the System_Ceiling, the NVIC takes the dispatch vector of a
  Task with a pending Release exactly when R7.1 admits the Task (`Elig`),
  in every state with the run's structure.
- `dispatch_selects_srp` (`lemma_dispatch_selects_srp_choice`, R7.2,
  R7.3): the NVIC's arbitration takes the vector of the Task the Stack
  Resource Policy selects and no other, whatever else is pending.
- `dispatch_idle` (`lemma_dispatch_idle_when_none_eligible`): with no
  Task admitted, no pending dispatch vector is taken.
- `select_is_srp`: the Task `Sched::select` returns is admitted and of the
  highest Priority among the pending ones. That it is the lowest Task
  identifier among equals (R7.3) is proven of `select` in Verus only; here
  it is a clause of `Srp` and a premise.
- `kernel_level_never_masked`, `kernel_level_taken` (Property 10's level
  part, R18.2): BASEPRI never keeps a Kernel-level exception from being
  taken; in every state of a run it is taken.
- `kernel_level_wins`: while one is pending and can be taken, what the
  NVIC takes is a Kernel-level exception.
- `kernel_level_not_preempted` (R40.6): while a Kernel-level exception is
  active the NVIC selects no configurable exception, whatever BASEPRI and
  PRIMASK are.

The premises on the configuration (`LevelsOk`: every Task's Priority is a
level 1 to 7 and every Resource's Ceiling at most 7) are what
`Config::validate` checks. -/

namespace P18.Nv
open rsk_kernel rsk_kernel.rsk_logic P18

variable {NT NR NL NP NS NE : Usize}

/-! ## The model -/

/-- The rsk priority map (DD-02): the Kernel level 0 encodes as 0, a Task
of Priority `p` as `(8 - p) * 32`. Lower values are more urgent (HW-01). -/
def encodeLevel (l : Nat) : Nat := if l = 0 then 0 else (8 - l) * 32

/-- BASEPRI for a System_Ceiling: 0 (no masking) for the idle level. -/
def basepriOfCeiling (c : Nat) : Nat := if c = 0 then 0 else encodeLevel c

/-- HW-02: the execution priority is the most urgent of the active
exceptions' group priorities, of BASEPRI when it is not 0, and of 0 when
PRIMASK is set; 256 when nothing boosts it. -/
def execPrio (active : Finset Nat) (basepri : Nat) (primask : Bool) : Nat :=
  min (if h : active.Nonempty then active.min' h else 256)
    (min (if basepri = 0 then 256 else basepri) (if primask then 0 else 256))

/-- HW-05: a pending exception is taken only when its group priority is
strictly more urgent than the execution priority. -/
def nvicTakes (g e : Nat) : Prop := g < e

/-- HW-03 under the revised DD-01: BASEPRI while a Job of level `top`
executes under System_Ceiling `c` (`top` 0 when no Job is in progress). -/
def basepriFor (top c : Nat) : Nat := basepriOfCeiling (max top c)

/-- HW-05: the pending exception the NVIC takes: among those more urgent
than the execution priority, the most urgent, and among equals the lowest
exception number. -/
def nvicSelects (pending : Nat → Prop) (group : Nat → Nat) (e n : Nat) : Prop :=
  pending n ∧ nvicTakes (group n) e ∧
  ∀ m, pending m → nvicTakes (group m) e → group n < group m ∨ (group n = group m ∧ n ≤ m)

/-- The level of the executing Job, 0 when no Job is in progress. -/
def jobLevel (cfg : logic.Config NT NR NP) (s : logic.sched.Sched NT NR NL) : Nat :=
  if s.depth.val = 0 then 0 else prio cfg (stk s (s.depth.val - 1))

/-- The execution priority the NVIC arbitrates the dispatch vectors
against wherever a Job may start: no exception active, PRIMASK clear,
BASEPRI from `basepriFor`. -/
def dispatchExec (cfg : logic.Config NT NR NP) (s : logic.sched.Sched NT NR NL) : Nat :=
  execPrio ∅ (basepriFor (jobLevel cfg s) s.ceiling.val) false

/-- Every Task's Priority is a level 1 to 7 and every Resource's Ceiling
is at most 7 (`Config::validate`). -/
def LevelsOk (cfg : logic.Config NT NR NP) : Prop :=
  (∀ k, k < NT.val → 1 ≤ prio cfg k ∧ prio cfg k ≤ 7) ∧ ∀ r, ceil cfg r ≤ 7

/-! ## The encoding -/

theorem encode_lt {a b : Nat} (ha : 1 ≤ a) (ha7 : a ≤ 7) (hb : 1 ≤ b) (hb7 : b ≤ 7) :
    encodeLevel a < encodeLevel b ↔ b < a := by
  unfold encodeLevel
  have h1 : ¬ a = 0 := by omega
  have h2 : ¬ b = 0 := by omega
  simp only [h1, h2, ite_false]
  omega

theorem encode_eq {a b : Nat} (ha : 1 ≤ a) (ha7 : a ≤ 7) (hb : 1 ≤ b) (hb7 : b ≤ 7) :
    encodeLevel a = encodeLevel b ↔ a = b := by
  unfold encodeLevel
  have h1 : ¬ a = 0 := by omega
  have h2 : ¬ b = 0 := by omega
  simp only [h1, h2, ite_false]
  omega

theorem execPrio_empty (b : Nat) : execPrio ∅ b false = if b = 0 then 256 else min 256 b := by
  unfold execPrio
  rw [dif_neg Finset.not_nonempty_empty]
  by_cases hb : b = 0
  · simp [hb]
  · simp only [hb, ite_false, Bool.false_eq_true]
    omega

/-- With no exception active and PRIMASK clear, BASEPRI from `basepriFor`
lets the NVIC take a pending Task vector of Priority `p` exactly when `p`
is above the System_Ceiling and the executing Job's level
(`lemma_nvic_takes_iff_above_job_and_ceiling`). -/
theorem takes_iff_above {top c p : Nat} (hp : 1 ≤ p) (hp7 : p ≤ 7) (ht : top ≤ 7) (hc : c ≤ 7) :
    nvicTakes (encodeLevel p) (execPrio ∅ (basepriFor top c) false) ↔ c < p ∧ top < p := by
  rw [execPrio_empty]
  unfold nvicTakes basepriFor basepriOfCeiling
  by_cases hm : max top c = 0
  · have e : encodeLevel p < 256 := by
      unfold encodeLevel
      have : ¬ p = 0 := by omega
      simp only [this, ite_false]; omega
    simp only [hm, ite_true]
    omega
  · have hm1 : 1 ≤ max top c := by omega
    have hm7 : max top c ≤ 7 := by omega
    have hne : ¬ encodeLevel (max top c) = 0 := by
      unfold encodeLevel
      simp only [hm, ite_false]; omega
    have hlt : encodeLevel (max top c) < 256 := by
      unfold encodeLevel
      simp only [hm, ite_false]; omega
    simp only [hm, ite_false, hne]
    rw [Nat.min_eq_right (by omega), encode_lt hp hp7 hm1 hm7]
    omega

/-! ## Property 1 over the model: the dispatch vectors -/

theorem jobLevel_le {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) (hl : LevelsOk cfg) : jobLevel cfg st.s ≤ 7 := by
  unfold jobLevel
  split
  · omega
  · exact (hl.1 _ (w.stk_lt _ (by omega))).2

theorem ceiling_le {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) (hl : LevelsOk cfg) : st.s.ceiling.val ≤ 7 := by
  obtain ⟨_, h2⟩ := w.ceil_is
  rcases h2 with h0 | ⟨p, _, hc⟩
  · omega
  · rw [hc]; exact hl.2 _

/-- **The NVIC takes the dispatch vector of a Task with a pending Release
exactly when R7.1 admits the Task** (`lemma_dispatch_takes_iff_eligible`),
in every state with the run's structure. -/
theorem dispatch_takes_iff_elig {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) (hl : LevelsOk cfg) {t : Nat} (ht : t < NT.val) (hr : rel st.s t = 1) :
    nvicTakes (encodeLevel (prio cfg t)) (dispatchExec cfg st.s) ↔ Elig cfg st.s t := by
  obtain ⟨hp1, hp7⟩ := hl.1 t ht
  unfold dispatchExec
  rw [takes_iff_above hp1 hp7 (jobLevel_le w hl) (ceiling_le w hl), elig_views]
  unfold jobLevel
  constructor
  · rintro ⟨hc, htop⟩
    refine ⟨ht, hr, ?_, hc, ?_⟩
    · -- A Task above the Job on top has no Job in progress: the stack is
      -- sorted by Priority.
      by_contra hj
      obtain ⟨p, hp, hs⟩ := (w.on_stack t ht).mp hj
      have hd : ¬ st.s.depth.val = 0 := by omega
      simp only [hd, ite_false] at htop
      by_cases hpe : p = st.s.depth.val - 1
      · rw [hpe] at hs; rw [hs] at htop; omega
      · have := w.sorted p (st.s.depth.val - 1) (by omega) (by omega)
        rw [hs] at this; omega
    · by_cases hd : st.s.depth.val = 0
      · exact Or.inl hd
      · simp only [hd, ite_false] at htop
        exact Or.inr htop
  · rintro ⟨_, _, _, hc, htop⟩
    refine ⟨hc, ?_⟩
    split
    · omega
    · rcases htop with h0 | h1
      · omega
      · exact h1

/-- The dispatch vectors (DD-03, HW-01): assigned in Task order, each with
the group priority that encodes its Task's Priority. -/
def VectorsOk (cfg : logic.Config NT NR NP) (vector group : Nat → Nat) : Prop :=
  (∀ t u, t < u → u < NT.val → vector t < vector u) ∧
  ∀ t, t < NT.val → group (vector t) = encodeLevel (prio cfg t)

/-- Every pending dispatch vector is of a Task whose Release is pending. -/
def NoStale (s : logic.sched.Sched NT NR NL) (pending : Nat → Prop) (vector : Nat → Nat) : Prop :=
  ∀ n, pending n → ∃ t, t < NT.val ∧ vector t = n ∧ rel s t = 1

/-- The Job the Stack Resource Policy starts: admitted (R7.1), of the
highest Priority among the pending Releases (R7.2), the lowest Task
identifier among equals (R7.3). -/
def Srp (cfg : logic.Config NT NR NP) (s : logic.sched.Sched NT NR NL) (t : Nat) : Prop :=
  Elig cfg s t ∧ (∀ u, u < NT.val → rel s u = 1 → prio cfg u ≤ prio cfg t) ∧
  ∀ u, u < NT.val → rel s u = 1 → prio cfg u = prio cfg t → t ≤ u

/-- **Once the vector of the Job the policy selects is pending, the NVIC
takes that vector and no other** (`lemma_dispatch_selects_srp_choice`). -/
theorem dispatch_selects_srp {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) (hl : LevelsOk cfg) {vector group : Nat → Nat} {pending : Nat → Prop}
    (hv : VectorsOk cfg vector group) (hs : NoStale st.s pending vector) {t : Nat}
    (hsel : Srp cfg st.s t) (hpend : pending (vector t)) :
    ∀ n, nvicSelects pending group (dispatchExec cfg st.s) n ↔ n = vector t := by
  obtain ⟨he, hmax, hlow⟩ := hsel
  have ht : t < NT.val := he.1
  have hrt : rel st.s t = 1 := (elig_views.mp he).2.1
  obtain ⟨hpt1, hpt7⟩ := hl.1 t ht
  have htakes : nvicTakes (group (vector t)) (dispatchExec cfg st.s) := by
    rw [hv.2 t ht]; exact (dispatch_takes_iff_elig w hl ht hrt).mpr he
  have hmono : ∀ u, u < NT.val → t ≤ u → vector t ≤ vector u := by
    intro u hu htu
    rcases Nat.lt_or_ge t u with h | h
    · exact Nat.le_of_lt (hv.1 t u h hu)
    · have : t = u := by omega
      rw [this]
  intro n
  constructor
  · rintro ⟨hpn, _, hall⟩
    obtain ⟨u, hu, hvu, hru⟩ := hs n hpn
    obtain ⟨hpu1, hpu7⟩ := hl.1 u hu
    have hle := hmax u hu hru
    have hg : group n = encodeLevel (prio cfg u) := by rw [← hvu]; exact hv.2 u hu
    rcases hall (vector t) hpend htakes with h | ⟨h, hn⟩
    · rw [hg, hv.2 t ht, encode_lt hpu1 hpu7 hpt1 hpt7] at h
      omega
    · rw [hg, hv.2 t ht, encode_eq hpu1 hpu7 hpt1 hpt7] at h
      have := hmono u hu (hlow u hu hru h)
      omega
  · rintro rfl
    refine ⟨hpend, htakes, fun m hpm _ => ?_⟩
    obtain ⟨u, hu, hvu, hru⟩ := hs m hpm
    obtain ⟨hpu1, hpu7⟩ := hl.1 u hu
    have hle := hmax u hu hru
    have hg : group m = encodeLevel (prio cfg u) := by rw [← hvu]; exact hv.2 u hu
    rw [hg, hv.2 t ht]
    rcases Nat.lt_or_ge (prio cfg u) (prio cfg t) with h | h
    · exact Or.inl ((encode_lt hpt1 hpt7 hpu1 hpu7).mpr h)
    · have e : prio cfg u = prio cfg t := by omega
      refine Or.inr ⟨by rw [e], ?_⟩
      rw [← hvu]
      exact hmono u hu (hlow u hu hru e)

/-- **With no Task admitted, the NVIC takes no pending dispatch vector**
(`lemma_dispatch_idle_when_none_eligible`). -/
theorem dispatch_idle {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) (hl : LevelsOk cfg) {vector group : Nat → Nat} {pending : Nat → Prop}
    (hv : VectorsOk cfg vector group) (hs : NoStale st.s pending vector)
    (hnone : ∀ u, ¬ Elig cfg st.s u) :
    ∀ n, ¬ nvicSelects pending group (dispatchExec cfg st.s) n := by
  rintro n ⟨hpn, htk, _⟩
  obtain ⟨u, hu, hvu, hru⟩ := hs n hpn
  rw [← hvu, hv.2 u hu] at htk
  exact hnone u ((dispatch_takes_iff_elig w hl hu hru).mp htk)

/-- The Task `Sched::select` returns is admitted and of the highest
Priority among the pending Releases: the first two clauses of `Srp`. -/
theorem select_is_srp {s : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    (hNT : NT.val ≤ 256) {t : U8} (h : logic.sched.Sched.select s cfg = ok (some t)) :
    Elig cfg s t.val ∧ ∀ u, u < NT.val → rel s u = 1 → prio cfg u ≤ prio cfg t.val := by
  rcases select_cases hNT h with ⟨h0, _⟩ | ⟨b, hb, hrel, hmax, ⟨hs, hc, htop, hj⟩ | ⟨hn, _⟩⟩
  · cases h0
  · cases hs
    exact ⟨elig_views.mpr ⟨hb, hrel, hj, hc, htop⟩, hmax⟩
  · cases hn

/-! ## The Kernel level -/

/-- **BASEPRI never keeps a Kernel-level exception from being taken**
(`lemma_kernel_level_never_masked`): with PRIMASK clear and only
exceptions of Task levels active, whatever BASEPRI is. -/
theorem kernel_level_never_masked (active : Finset Nat) (basepri : Nat)
    (ha : ∀ g, g ∈ active → encodeLevel 0 < g) :
    nvicTakes (encodeLevel 0) (execPrio active basepri false) := by
  unfold nvicTakes execPrio
  have e0 : encodeLevel 0 = 0 := rfl
  rw [e0] at ha ⊢
  simp only [Bool.false_eq_true, ite_false]
  have h1 : 0 < (if h : active.Nonempty then active.min' h else 256) := by
    split
    · rename_i h
      exact ha _ (Finset.min'_mem active h)
    · omega
  have h2 : 0 < (if basepri = 0 then 256 else basepri) := by split <;> omega
  omega

/-- **In every state of a run the NVIC takes a pending Kernel-level
exception** (`lemma_kernel_level_taken_in_every_state`, Property 10's
level part, R18.2): while a Job executes under any System_Ceiling, and
while none is in progress. -/
theorem kernel_level_taken (cfg : logic.Config NT NR NP) (s : logic.sched.Sched NT NR NL) :
    nvicTakes (encodeLevel 0) (dispatchExec cfg s) :=
  kernel_level_never_masked ∅ _ (fun _ h => by cases h)

/-- **While a Kernel-level exception is pending and can be taken, what the
NVIC takes is a Kernel-level exception** (`lemma_kernel_level_wins_arbitration`). -/
theorem kernel_level_wins {pending : Nat → Prop} {group : Nat → Nat} {e k n : Nat}
    (hk : pending k) (hg : group k = encodeLevel 0) (ht : nvicTakes (group k) e)
    (hn : nvicSelects pending group e n) : group n = encodeLevel 0 := by
  obtain ⟨_, _, hall⟩ := hn
  have e0 : encodeLevel 0 = 0 := rfl
  rw [e0] at hg ⊢
  rcases hall k hk ht with h | ⟨h, _⟩
  · omega
  · omega

/-- **A Kernel-level handler is not preempted by a configurable
exception** (`lemma_kernel_level_not_preempted`, R40.6): while a
Kernel-level exception is active, the NVIC selects none, whatever is
pending and whatever BASEPRI and PRIMASK are. -/
theorem kernel_level_not_preempted (active : Finset Nat) (basepri : Nat) (primask : Bool)
    (pending : Nat → Prop) (group : Nat → Nat) (n : Nat)
    (ha : encodeLevel 0 ∈ active) :
    execPrio active basepri primask ≤ encodeLevel 0 ∧
    ¬ nvicSelects pending group (execPrio active basepri primask) n := by
  have e0 : encodeLevel 0 = 0 := rfl
  rw [e0] at ha ⊢
  have hne : active.Nonempty := ⟨0, ha⟩
  have hmin : active.min' hne ≤ 0 := Finset.min'_le active 0 ha
  have he : execPrio active basepri primask ≤ 0 := by
    unfold execPrio
    rw [dif_pos hne]
    exact Nat.le_trans (Nat.min_le_left _ _) hmin
  refine ⟨he, ?_⟩
  rintro ⟨_, ht, _⟩
  unfold nvicTakes at ht
  omega

end P18.Nv
