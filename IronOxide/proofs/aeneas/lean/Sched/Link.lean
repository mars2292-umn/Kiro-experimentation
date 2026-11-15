import Sched.Stage
import Rta.Whole
open Aeneas Aeneas.Std Result

/-!
Property 18 against the Analyzer's own output: the run's parameters are
those of the Analyzer's inputs, and the bound is the response time
`analyse_all` (as compiled, Verification_Plan 7.2) reports.
-/

namespace P18
open rsk_logic
open rsk_analyzer (ceilDiv interf stepF specTerms specAll pushLower termJobs termIntake
  termEvents hkTerm specO specE siteCount longestCs maxCandidate candidateOf lowerEvents
  timedEvents bFloor)
open rsk_analyzer.terms (TaskIn Site Window Interferer Verdict)

variable {NT NR NL NP : Usize}

theorem interf_append (a : List rsk_analyzer.rta.Term) (x : rsk_analyzer.rta.Term) (w : Nat) :
    interf (a ++ [x]) w = interf a w + ceilDiv (w + x.jitter.val) x.period.val * x.bound.val := by
  induction a with
  | nil => simp [interf]
  | cons y ys ih => simp [interf, ih]; omega

/-- Task `k`'s term in Task `i`'s recurrence: its Jobs for a Task of equal
or higher Priority, the Kernel's work for its arrivals and its other timed
events for a lower one. -/
def tfun (its : List Interferer) (ti : Interferer) (i w k : Nat) : Nat :=
  match its[k]? with
  | some t =>
    if k = i then 0
    else if ti.priority.val ≤ t.priority.val
      then ceilDiv (w + t.jitter.val) t.period.val * t.bound.val
      else ceilDiv (w + t.jitter.val) t.period.val * (t.intake.val + t.events.val)
  | none => 0

/-- The Analyzer's interference terms as a sum over the Tasks. -/
theorem interf_spec (its : List Interferer) (ti : Interferer) (i : Nat) (w : Nat) :
    ∀ n, n ≤ its.length →
    interf (specTerms its ti i n) w = ∑ k ∈ Finset.range n, tfun its ti i w k := by
  intro n
  induction n with
  | zero => intro _; simp [specTerms, interf]
  | succ n ih =>
    intro hn
    have hlt : n < its.length := by omega
    have hget : its[n]? = some its[n] := List.getElem?_eq_getElem hlt
    rw [Finset.sum_range_succ, ← ih (by omega)]
    simp only [specTerms, hget, tfun]
    by_cases h1 : n = i
    · simp [h1]
    · by_cases h2 : (its[n]).priority.val ≥ ti.priority.val
      · simp only [h1, ite_false, h2, ite_true]
        rw [interf_append]
        simp only [termJobs]
      · simp only [h1, ite_false, h2]
        unfold pushLower
        by_cases h3 : (its[n]).intake.val > 0 <;> by_cases h4 : (its[n]).events.val > 0
        · simp only [h3, h4, ite_true]
          rw [interf_append, interf_append]
          simp only [termIntake, termEvents]
          rw [Nat.mul_add]; omega
        · simp only [h3, h4, ite_true, ite_false]
          rw [interf_append]
          simp only [termIntake]
          have : (its[n]).events.val = 0 := by omega
          rw [this, Nat.add_zero]
        · simp only [h3, h4, ite_true, ite_false]
          rw [interf_append]
          simp only [termEvents]
          have : (its[n]).intake.val = 0 := by omega
          rw [this, Nat.zero_add]
        · simp only [h3, h4, ite_false]
          have e1 : (its[n]).intake.val = 0 := by omega
          have e2 : (its[n]).events.val = 0 := by omega
          rw [e1, e2, Nat.add_zero, Nat.mul_zero, Nat.add_zero]

theorem ceilDiv_one (R T : Nat) (hR : 0 < R) (h : R ≤ T) : ceilDiv R T = 1 := by
  unfold ceilDiv
  exact Nat.div_eq_of_lt_le (by omega) (by omega)

/-! ## The run's parameters from the Analyzer's inputs -/

/-- The kind of the event source of index `q` among `n` Tasks: 0 the
arrival (the release event of a Periodic_Task, the intake of a
Sporadic_Task's Release_Source), 1 the deadline event, 2 the MIT unmask,
3 the MIT deferral, 4 the Kernel's housekeeping event. -/
def srcKind (n q : Nat) : Nat :=
  if q < n then 0 else if q < n + n then 1 else if q < n + (n + n) then 2
  else if q < n + (n + (n + n)) then 3 else 4

/-- The Task of the event source of index `q`. -/
def srcTask (n q : Nat) : Nat :=
  if q < n then q else if q < n + n then q - n else if q < n + (n + n) then q - (n + n)
  else q - (n + (n + n))

theorem kind0 {n k : Nat} (h : k < n) : srcKind n k = 0 ∧ srcTask n k = k := by
  unfold srcKind srcTask; simp [h]
theorem kind1 {n k : Nat} (h : k < n) : srcKind n (n + k) = 1 ∧ srcTask n (n + k) = k := by
  unfold srcKind srcTask; simp [h]
theorem kind2 {n k : Nat} (h : k < n) :
    srcKind n (n + (n + k)) = 2 ∧ srcTask n (n + (n + k)) = k := by
  unfold srcKind srcTask
  have h1 : ¬ (n + (n + k) < n) := by omega
  have h2 : ¬ (n + (n + k) < n + n) := by omega
  have h3 : n + (n + k) < n + (n + n) := by omega
  simp only [h1, h2, h3, ite_true, ite_false]
  exact ⟨trivial, by omega⟩
theorem kind3 {n k : Nat} (h : k < n) :
    srcKind n (n + (n + (n + k))) = 3 ∧ srcTask n (n + (n + (n + k))) = k := by
  unfold srcKind srcTask
  have h1 : ¬ (n + (n + (n + k)) < n) := by omega
  have h2 : ¬ (n + (n + (n + k)) < n + n) := by omega
  have h3 : ¬ (n + (n + (n + k)) < n + (n + n)) := by omega
  have h4 : n + (n + (n + k)) < n + (n + (n + n)) := by omega
  simp only [h1, h2, h3, h4, ite_true, ite_false]
  exact ⟨trivial, by omega⟩
theorem kind4 (n : Nat) : srcKind n (n + (n + (n + n))) = 4 := by
  unfold srcKind
  have h1 : ¬ (n + (n + (n + n)) < n + (n + (n + n))) := by omega
  by_cases h0 : n = 0
  · subst h0; simp
  · have h2 : ¬ (n + (n + (n + n)) < n) := by omega
    have h3 : ¬ (n + (n + (n + n)) < n + n) := by omega
    have h4 : ¬ (n + (n + (n + n)) < n + (n + n)) := by omega
    simp only [h1, h2, h3, h4, ite_false]

/-- A deadline-event index names a Task of the set. -/
theorem kind1_task {n q : Nat} (h : srcKind n q = 1) : srcTask n q < n ∧ q = n + srcTask n q := by
  unfold srcKind at h
  unfold srcTask
  by_cases h1 : q < n
  · simp [h1] at h
  · by_cases h2 : q < n + n
    · simp only [h1, h2, ite_true, ite_false]; omega
    · simp only [h1, h2, ite_false] at h
      split at h
      · omega
      · split at h <;> omega

/-- The bound on the Kernel entry of a Task's event of kind `e`: a timer
entry (Δ_timer) for a timed event the Task has, the interrupt's handling
(Δ_irq) for a Sporadic_Task's intake, nothing for an event it has not. -/
def evCost (t : TaskIn) (p : rsk_analyzer.terms.Params) (e : Nat) : Nat :=
  if e = 0 then (if t.periodic then p.timer.val else p.irq.val)
  else if e = 1 then p.timer.val
  else if e = 2 then (if t.periodic then 0 else p.timer.val)
  else if e = 3 then (if t.periodic = false ∧ t.defer = true then p.timer.val else 0)
  else 0

/-- A Task's period or MIT as the Analyzer takes it (at least 1). -/
def perAt (windows : List Window) (k : Nat) : Nat :=
  match windows[k]? with
  | some w => if w.period.val = 0 then 1 else w.period.val
  | none => 1

/-- The run's timing parameters, from the Analyzer's inputs: the Task set,
its Critical_Section sites, the Kernel_Timing_Parameters, each Task's
period or MIT and deadline; and the given bounds `L` on the run's
Critical_Sections. `n` is the number of Tasks. -/
def runParams (n : Nat) (tasks : List TaskIn) (sites : List Site)
    (p : rsk_analyzer.terms.Params) (windows : List Window) (L : Nat → Nat → Nat) : Params :=
  { T := perAt windows,
    C := fun k => match tasks[k]? with
      | some t => t.budget.val
      | none => 0,
    J := fun _ => p.j_rel.val,
    D := fun k => match windows[k]? with
      | some w => w.deadline.val
      | none => 0,
    S := fun k => siteCount sites k sites.length,
    EP := fun k => match tasks[k]? with
      | some t => t.endpoint_cost.val
      | none => 0,
    stop := fun k => match tasks[k]? with
      | some t => t.stops_overruns
      | none => false,
    L := L,
    Kd := p.dispatch.val, Kc := p.complete.val, Kin := p.lock_in.val, Kout := p.lock_out.val,
    Kt := p.timer.val, Kov := p.detect.val + p.enforce.val,
    NQ := n + (n + (n + (n + 1))),
    ET := fun q => if srcKind n q = 4 then (if p.hk_period.val = 0 then 1 else p.hk_period.val)
      else perAt windows (srcTask n q),
    EC := fun q => if srcKind n q = 4 then p.timer.val
      else match tasks[srcTask n q]? with
        | some t => evCost t p (srcKind n q)
        | none => 0,
    EJ := fun _ => p.j_rel.val,
    relOf := fun q => if srcKind n q = 0 ∨ srcKind n q = 3 then some (srcTask n q) else none,
    dlOf := fun q => if srcKind n q = 1 then some (srcTask n q) else none }

theorem sum_split (n : Nat) (f : Nat → Nat) :
    ∑ x ∈ Finset.range (n + (n + (n + (n + (n + 1))))), f x
      = ∑ k ∈ Finset.range n, (f k + f (n + k) + f (n + (n + k)) + f (n + (n + (n + k)))
          + f (n + (n + (n + (n + k))))) + f (n + (n + (n + (n + n)))) := by
  rw [Finset.sum_range_add, Finset.sum_range_add, Finset.sum_range_add, Finset.sum_range_add,
    Finset.sum_range_succ]
  simp only [Finset.sum_add_distrib]
  omega

/-- `B_i` is at least the candidate of every site that can block the Task. -/
theorem maxCandidate_ge (tasks : List TaskIn) (sites : List Site) (ti : TaskIn)
    (p : rsk_analyzer.terms.Params) {j : Nat} {s : Site} {l : TaskIn}
    (hs : sites[j]? = some s) (hl : tasks[s.task.val]? = some l)
    (hp : l.priority.val < ti.priority.val) (hc : ti.priority.val ≤ s.ceiling.val) :
    ∀ n, j < n → candidateOf ti l s p ≤ maxCandidate tasks sites ti p n := by
  intro n
  induction n with
  | zero => intro h; omega
  | succ n ih =>
    intro hj
    by_cases hjn : j = n
    · subst hjn
      simp only [maxCandidate, hs, hl]
      split <;> omega
    · have := ih (by omega)
      simp only [maxCandidate]
      split
      · exact this
      · split
        · exact this
        · split <;> omega

theorem perAt_pos (windows : List Window) (k : Nat) : 0 < perAt windows k := by
  unfold perAt
  split
  · split <;> omega
  · omega

/-- The deadline events that never have an entry, seen from Task `i`:
those of the Tasks below `i` that the analysis marks quiet. -/
def deadOf (cfg : logic.Config NT NR NP) (quiet : List Bool) (i : Nat) (q : Nat) : Bool :=
  decide (srcKind NT.val q = 1 ∧ quiet[srcTask NT.val q]? = some true
    ∧ prio cfg (srcTask NT.val q) < prio cfg i)

/-- `ovx` of a Task, from the Analyzer's inputs. -/
def ovxOf (t : TaskIn) (p : rsk_analyzer.terms.Params) : Nat :=
  if t.stops_overruns then (p.detect.val + p.enforce.val) - (p.timer.val + p.complete.val) else 0

/-- What the run charges one Job of a Task and its timed events is within
`E_j` (R29.4). -/
theorem cost_le_E (t : TaskIn) (p : rsk_analyzer.terms.Params) (S longest : Nat) (any_fpu : Bool) :
    t.budget.val + (p.dispatch.val + S * p.lock_in.val + S * p.lock_out.val
        + t.endpoint_cost.val + p.timer.val + p.complete.val + ovxOf t p)
      + evCost t p 0 + evCost t p 1 + evCost t p 2 + evCost t p 3
    ≤ specE t p longest (specO t p S any_fpu) := by
  have hm : (p.lock_in.val + p.lock_out.val) * S = S * p.lock_in.val + S * p.lock_out.val := by
    ring
  unfold specE specO timedEvents evCost ovxOf
  rw [hm]
  cases hp : t.periodic <;> cases hd : t.defer <;> cases hs : t.stops_overruns <;> simp <;> omega

/-- For the Task under analysis, with the Budget event of the Job that
blocks it, it is within `O_i` (R29.3): `O_i` counts, for a Partition that
stops overrunning Jobs, what the entry that ends the Task's own
overrunning Job takes beyond its Budget event, its Job end and the
Health_Monitor response (`overrunExtra`). -/
theorem cost_le_own (t : TaskIn) (p : rsk_analyzer.terms.Params) (S : Nat) (any_fpu : Bool) :
    (p.dispatch.val + S * p.lock_in.val + S * p.lock_out.val
        + t.endpoint_cost.val + p.timer.val + p.complete.val + ovxOf t p)
      + evCost t p 0 + evCost t p 1 + evCost t p 2 + evCost t p 3 + p.timer.val
    ≤ specO t p S any_fpu := by
  have hm : (p.lock_in.val + p.lock_out.val) * S = S * p.lock_in.val + S * p.lock_out.val := by
    ring
  unfold specO timedEvents evCost ovxOf rsk_analyzer.overrunExtra
  rw [hm]
  cases hs : t.stops_overruns <;> cases hp : t.periodic <;> cases hd : t.defer <;> simp <;> omega

/-- For a Task below the one under analysis, the entries of its events
are its term of R19.2. -/
theorem lower_cost (t : TaskIn) (p : rsk_analyzer.terms.Params) (q : Bool) :
    evCost t p 0 + (if q then 0 else evCost t p 1) + evCost t p 2 + evCost t p 3
      = (if t.periodic then p.timer.val else p.irq.val) + p.timer.val * lowerEvents t (!q) := by
  unfold evCost lowerEvents
  cases hp : t.periodic <;> cases hd : t.defer <;> cases q <;> simp <;> omega

/-- One account's share of the fixed-point inequality of `stage4`. -/
def Fterm (cfg : logic.Config NT NR NP) (P : Params) (i : Nat) (dead : Nat → Bool) (R x : Nat) :
    Nat :=
  costOf cfg P i dead x * ceilDiv (R + jitOf cfg P x) (perOf cfg P x)

theorem Fterm_job {cfg : logic.Config NT NR NP} {P : Params} {i : Nat} {dead : Nat → Bool}
    {R k : Nat} (hk : k < NT.val) :
    Fterm cfg P i dead R k
      = (if prio cfg i ≤ prio cfg k then P.C k + Ojob P k else 0) * ceilDiv (R + P.J k) (P.T k) := by
  unfold Fterm; rw [costOf_job hk, jitOf_job hk, perOf_job hk]

theorem Fterm_src {cfg : logic.Config NT NR NP} {P : Params} {i : Nat} {dead : Nat → Bool}
    {R : Nat} (q : Nat) :
    Fterm cfg P i dead R (NT.val + q)
      = (if dead q then 0 else P.EC q) * ceilDiv (R + P.EJ q) (P.ET q) := by
  unfold Fterm; rw [costOf_src, jitOf_src, perOf_src]

theorem five_mul {A e0 e1 e2 e3 E N : Nat} (h : A + e0 + e1 + e2 + e3 ≤ E) :
    A * N + e0 * N + e1 * N + e2 * N + e3 * N ≤ N * E := by
  have h1 := Nat.mul_le_mul_right N h
  have e : (A + e0 + e1 + e2 + e3) * N = A * N + e0 * N + e1 * N + e2 * N + e3 * N := by ring
  rw [Nat.mul_comm N E]; omega

section
variable (n : Nat) (tasks : List TaskIn) (sites : List Site) (p : rsk_analyzer.terms.Params)
  (windows : List Window) (L : Nat → Nat → Nat)

theorem rp_EC_hk : (runParams n tasks sites p windows L).EC (n + (n + (n + n))) = p.timer.val := by
  show (if srcKind n (n + (n + (n + n))) = 4 then p.timer.val else _) = _
  rw [kind4]; simp

theorem rp_ET_hk : (runParams n tasks sites p windows L).ET (n + (n + (n + n)))
    = (if p.hk_period.val = 0 then 1 else p.hk_period.val) := by
  show (if srcKind n (n + (n + (n + n))) = 4 then _ else _) = _
  rw [kind4]; simp

theorem rp_EC {k : Nat} {t : TaskIn} (hk : k < n) (ht : tasks[k]? = some t) :
    (runParams n tasks sites p windows L).EC k = evCost t p 0
    ∧ (runParams n tasks sites p windows L).EC (n + k) = evCost t p 1
    ∧ (runParams n tasks sites p windows L).EC (n + (n + k)) = evCost t p 2
    ∧ (runParams n tasks sites p windows L).EC (n + (n + (n + k))) = evCost t p 3 := by
  refine ⟨?_, ?_, ?_, ?_⟩
  · show (if srcKind n k = 4 then p.timer.val else
      match tasks[srcTask n k]? with
      | some t => evCost t p (srcKind n k)
      | none => 0) = _
    rw [(kind0 hk).1, (kind0 hk).2, ht]; simp
  · show (if srcKind n (n + k) = 4 then p.timer.val else
      match tasks[srcTask n (n + k)]? with
      | some t => evCost t p (srcKind n (n + k))
      | none => 0) = _
    rw [(kind1 hk).1, (kind1 hk).2, ht]; simp
  · show (if srcKind n (n + (n + k)) = 4 then p.timer.val else
      match tasks[srcTask n (n + (n + k))]? with
      | some t => evCost t p (srcKind n (n + (n + k)))
      | none => 0) = _
    rw [(kind2 hk).1, (kind2 hk).2, ht]; simp
  · show (if srcKind n (n + (n + (n + k))) = 4 then p.timer.val else
      match tasks[srcTask n (n + (n + (n + k)))]? with
      | some t => evCost t p (srcKind n (n + (n + (n + k))))
      | none => 0) = _
    rw [(kind3 hk).1, (kind3 hk).2, ht]; simp

theorem rp_ET {k : Nat} (hk : k < n) :
    (runParams n tasks sites p windows L).ET k = perAt windows k
    ∧ (runParams n tasks sites p windows L).ET (n + k) = perAt windows k
    ∧ (runParams n tasks sites p windows L).ET (n + (n + k)) = perAt windows k
    ∧ (runParams n tasks sites p windows L).ET (n + (n + (n + k))) = perAt windows k := by
  refine ⟨?_, ?_, ?_, ?_⟩
  · show (if srcKind n k = 4 then _ else perAt windows (srcTask n k)) = _
    rw [(kind0 hk).1, (kind0 hk).2]; simp
  · show (if srcKind n (n + k) = 4 then _ else perAt windows (srcTask n (n + k))) = _
    rw [(kind1 hk).1, (kind1 hk).2]; simp
  · show (if srcKind n (n + (n + k)) = 4 then _ else perAt windows (srcTask n (n + (n + k)))) = _
    rw [(kind2 hk).1, (kind2 hk).2]; simp
  · show (if srcKind n (n + (n + (n + k))) = 4 then _
      else perAt windows (srcTask n (n + (n + (n + k))))) = _
    rw [(kind3 hk).1, (kind3 hk).2]; simp

theorem rp_C {k : Nat} {t : TaskIn} (ht : tasks[k]? = some t) :
    (runParams n tasks sites p windows L).C k = t.budget.val := by
  show (match tasks[k]? with
    | some t => t.budget.val
    | none => 0) = _
  rw [ht]

theorem rp_Ojob {k : Nat} {t : TaskIn} (ht : tasks[k]? = some t) :
    Ojob (runParams n tasks sites p windows L) k
      = p.dispatch.val + siteCount sites k sites.length * p.lock_in.val
        + siteCount sites k sites.length * p.lock_out.val
        + t.endpoint_cost.val + p.timer.val + p.complete.val + ovxOf t p := by
  have h1 : (runParams n tasks sites p windows L).EP k = t.endpoint_cost.val := by
    show (match tasks[k]? with
      | some t => t.endpoint_cost.val
      | none => 0) = _
    rw [ht]
  have h2 : (runParams n tasks sites p windows L).stop k = t.stops_overruns := by
    show (match tasks[k]? with
      | some t => t.stops_overruns
      | none => false) = _
    rw [ht]
  unfold Ojob ovx ovxOf
  rw [h1, h2]
  rfl

end

theorem dead_kind {cfg : logic.Config NT NR NP} {quiet : List Bool} {i q : Nat}
    (hne : srcKind NT.val q ≠ 1) : deadOf cfg quiet i q = false := by
  unfold deadOf; simp [hne]

theorem dead_one {cfg : logic.Config NT NR NP} {quiet : List Bool} {i k : Nat} (hk : k < NT.val) :
    deadOf cfg quiet i (NT.val + k)
      = decide (quiet[k]? = some true ∧ prio cfg k < prio cfg i) := by
  unfold deadOf; rw [(kind1 hk).1, (kind1 hk).2]; simp

/-- **The bound for one Task, given the Tasks below it that the analysis
marks quiet.** The hypotheses are what `analyse_all` establishes
(`hits`, `hv`) and the correspondence between the Kernel's configuration
and the Analyzer's inputs. -/
theorem core {cfg : logic.Config NT NR NP}
    (tasks : List TaskIn) (sites : List Site) (p : rsk_analyzer.terms.Params) (any_fpu : Bool)
    (windows : List Window) (quiet : List Bool) (its : List Interferer) (L : Nat → Nat → Nat)
    (hlen : tasks.length = NT.val) (hNT : NT.val ≤ 256)
    (hwl : windows.length = tasks.length) (hql : quiet.length = tasks.length)
    (hil : its.length = tasks.length)
    (hits : rsk_analyzer.allInterferers tasks sites p any_fpu windows quiet its)
    (hprio : ∀ k t, tasks[k]? = some t → 0 < t.priority.val ∧ prio cfg k = t.priority.val)
    (hbud : ∀ t ∈ tasks, 0 < t.budget.val)
    (hDT : ∀ w ∈ windows, w.deadline.val ≤ w.period.val)
    (i : Nat) (ti : TaskIn) (wi : Window) (iti : Interferer) (v : Verdict)
    (hti : tasks[i]? = some ti) (hwi : windows[i]? = some wi) (hiti : its[i]? = some iti)
    (hv : rsk_analyzer.verdictOk
      (ti.budget.val + maxCandidate tasks sites ti p sites.length
        + specO ti p (siteCount sites i sites.length) any_fpu)
      (specAll its iti i (hkTerm p)) p.j_rel.val wi.deadline.val v)
    (hsched : v.schedulable = true)
    (hL : ∀ k r, k < NT.val → prio cfg k < prio cfg i → prio cfg i ≤ ceil cfg r →
      L k r = 0 ∨
      ∃ (j : Nat) (s : Site) (l : TaskIn), sites[j]? = some s ∧ tasks[s.task.val]? = some l
        ∧ l.priority.val < ti.priority.val ∧ ti.priority.val ≤ s.ceiling.val
        ∧ L k r ≤ candidateOf ti l s p)
    (hquiet : ∀ k, quiet[k]? = some true → prio cfg k < prio cfg i →
      ∀ st : St NT NR NL, Reach cfg (runParams NT.val tasks sites p windows L) st →
        ∀ a, st.jl k = some a → (0 < st.pend k + st.run k ∨ st.jwait k = true) →
          st.now < a + (runParams NT.val tasks sites p windows L).D k)
    {st : St NT NR NL} (hreach : Reach cfg (runParams NT.val tasks sites p windows L) st)
    (hpos : 0 < st.pend i + st.run i) :
    ∃ a, st.jl i = some a ∧ st.now < a + v.r.val := by
  obtain ⟨hfix, _, hr, hrd⟩ := hv.2.1 hsched
  have hi : i < NT.val := by
    have := (List.getElem?_eq_some_iff.mp hti).1
    omega
  have hbi := hbud ti (List.mem_of_getElem? hti)
  have hRpos : 0 < v.w.val := by
    unfold stepF at hfix; omega
  have hpi := (hprio i ti hti).2
  obtain ⟨ti', wi', qi, hti', hwi', hqi, i1, i2, i3, i4, i5, i6⟩ := hits i iti hiti
  rw [hti] at hti'; cases hti'
  rw [hwi] at hwi'; cases hwi'
  have hDi := hDT wi (List.mem_of_getElem? hwi)
  have hmain := stage4 (P := runParams NT.val tasks sites p windows L) (R := v.w.val)
    (B := maxCandidate tasks sites ti p sites.length) (dead := deadOf cfg quiet i) hNT
    ?_ hi hRpos ?_ ?_ ?_ ?_ hreach hpos
  · obtain ⟨a, ha, hlt⟩ := hmain
    refine ⟨a, ha, ?_⟩
    have hJ : (runParams NT.val tasks sites p windows L).J i = p.j_rel.val := rfl
    omega
  · intro k hk
    have hlt : k < tasks.length := by omega
    have := hprio k _ (List.getElem?_eq_getElem hlt)
    omega
  · intro x _
    unfold perOf
    split
    · exact perAt_pos _ _
    · show 0 < (if srcKind NT.val (x - NT.val) = 4
          then (if p.hk_period.val = 0 then 1 else p.hk_period.val)
          else perAt windows (srcTask NT.val (x - NT.val)))
      split
      · split <;> omega
      · exact perAt_pos _ _
  · intro k r hk hlo hce
    rcases hL k r hk hlo hce with h0 | ⟨j, s, l, hs, hl, h1, h2, h3⟩
    · show L k r ≤ _
      rw [h0]; exact Nat.zero_le _
    · exact Nat.le_trans h3
        (maxCandidate_ge tasks sites ti p hs hl h1 h2 sites.length
          (List.getElem?_eq_some_iff.mp hs).1)
  · have hsum := interf_spec its iti i v.w.val its.length (Nat.le_refl _)
    unfold stepF specAll at hfix
    rw [interf_append, hsum, hil, hlen] at hfix
    have hhkj : (hkTerm p).jitter.val = p.j_rel.val := rfl
    have hhkb : (hkTerm p).bound.val = p.timer.val := rfl
    have hhkp : (hkTerm p).period.val = (if p.hk_period.val = 0 then 1 else p.hk_period.val) := by
      unfold hkTerm
      by_cases h0 : p.hk_period = 0#u64
      · simp [h0]
      · have : p.hk_period.val ≠ 0 := by
          intro h; apply h0; scalar_tac
        simp [h0, this]
    have hJ : ∀ k, (runParams NT.val tasks sites p windows L).J k = p.j_rel.val := fun _ => rfl
    have hEJ : ∀ q, (runParams NT.val tasks sites p windows L).EJ q = p.j_rel.val := fun _ => rfl
    have hTk : ∀ k, (runParams NT.val tasks sites p windows L).T k = perAt windows k := fun _ => rfl
    show maxCandidate tasks sites ti p sites.length + p.timer.val
      + ∑ x ∈ Finset.range (NT.val + (NT.val + (NT.val + (NT.val + (NT.val + 1))))),
          Fterm cfg (runParams NT.val tasks sites p windows L) i (deadOf cfg quiet i) v.w.val x
      ≤ v.w.val
    rw [sum_split]
    have hhk : Fterm cfg (runParams NT.val tasks sites p windows L) i (deadOf cfg quiet i) v.w.val
        (NT.val + (NT.val + (NT.val + (NT.val + NT.val))))
        = ceilDiv (v.w.val + (hkTerm p).jitter.val) (hkTerm p).period.val * (hkTerm p).bound.val := by
      rw [Fterm_src, rp_EC_hk, rp_ET_hk, hEJ, hhkj, hhkb, hhkp,
        dead_kind (by rw [kind4]; omega)]
      simp only [Bool.false_eq_true, ite_false]
      exact Nat.mul_comm _ _
    have hterm : ∀ k ∈ Finset.range NT.val,
        Fterm cfg (runParams NT.val tasks sites p windows L) i (deadOf cfg quiet i) v.w.val k
        + Fterm cfg (runParams NT.val tasks sites p windows L) i (deadOf cfg quiet i) v.w.val
            (NT.val + k)
        + Fterm cfg (runParams NT.val tasks sites p windows L) i (deadOf cfg quiet i) v.w.val
            (NT.val + (NT.val + k))
        + Fterm cfg (runParams NT.val tasks sites p windows L) i (deadOf cfg quiet i) v.w.val
            (NT.val + (NT.val + (NT.val + k)))
        + Fterm cfg (runParams NT.val tasks sites p windows L) i (deadOf cfg quiet i) v.w.val
            (NT.val + (NT.val + (NT.val + (NT.val + k))))
        ≤ (if k = i then ti.budget.val
              + (specO ti p (siteCount sites i sites.length) any_fpu - p.timer.val) else 0)
          + tfun its iti i v.w.val k := by
      intro k hk
      have hkn := Finset.mem_range.mp hk
      obtain ⟨itk, hgi⟩ : ∃ itk, its[k]? = some itk :=
        ⟨_, List.getElem?_eq_getElem (by omega)⟩
      obtain ⟨tk, wk, qk, htk, hwk, hqk, k1, k2, k3, k4, k5, k6⟩ := hits k itk hgi
      have hpk := (hprio k tk htk).2
      have hper : perAt windows k = itk.period.val := by
        unfold perAt; rw [hwk]; exact k3.symm
      obtain ⟨c0, c1, c2, c3⟩ := rp_EC NT.val tasks sites p windows L hkn htk
      obtain ⟨e0, e1, e2, e3⟩ := rp_ET NT.val tasks sites p windows L hkn
      rw [Fterm_job hkn, Fterm_src k, Fterm_src (NT.val + k), Fterm_src (NT.val + (NT.val + k)),
        Fterm_src (NT.val + (NT.val + (NT.val + k)))]
      rw [rp_C NT.val tasks sites p windows L htk, rp_Ojob NT.val tasks sites p windows L htk,
        c0, c1, c2, c3, e0, e1, e2, e3, hTk, hper]
      simp only [hJ, hEJ]
      rw [dead_kind (q := k) (by rw [(kind0 hkn).1]; omega), dead_one hkn,
        dead_kind (q := NT.val + (NT.val + k)) (by rw [(kind2 hkn).1]; omega),
        dead_kind (q := NT.val + (NT.val + (NT.val + k))) (by rw [(kind3 hkn).1]; omega)]
      simp only [Bool.false_eq_true, ite_false]
      unfold tfun
      rw [hgi]
      simp only []
      rw [hpi, hpk, i1, k1, k2]
      by_cases hki : k = i
      · have htk' : tk = ti := by
          rw [hki, hti] at htk; exact (Option.some.inj htk).symm
        have hwk' : wk = wi := by
          rw [hki, hwi] at hwk; exact (Option.some.inj hwk).symm
        rw [hwk'] at k3
        rw [htk']
        have hN : ceilDiv (v.w.val + p.j_rel.val) itk.period.val = 1 := by
          apply ceilDiv_one _ _ (by omega)
          rw [k3]; split <;> omega
        have hd : decide (quiet[k]? = some true ∧ ti.priority.val < ti.priority.val) = false := by
          simp
        rw [hN, hd, hki]
        simp only [Nat.le_refl, ite_true, Nat.mul_one, Bool.false_eq_true, ite_false, Nat.add_zero]
        have := cost_le_own ti p (siteCount sites i sites.length) any_fpu
        omega
      · simp only [hki, ite_false, Nat.zero_add]
        by_cases hhe : ti.priority.val ≤ tk.priority.val
        · have hd : decide (quiet[k]? = some true ∧ tk.priority.val < ti.priority.val) = false := by
            simp; intro _; omega
          rw [hd]
          simp only [hhe, ite_true, Bool.false_eq_true, ite_false]
          apply five_mul
          rw [k4]
          exact cost_le_E tk p _ _ any_fpu
        · have hd : decide (quiet[k]? = some true ∧ tk.priority.val < ti.priority.val) = qk := by
            rw [hqk]
            cases qk <;> simp; omega
          rw [hd]
          simp only [hhe, ite_false, Nat.zero_mul, Nat.zero_add]
          have hlc := lower_cost tk p qk
          have e : evCost tk p 0 * ceilDiv (v.w.val + p.j_rel.val) itk.period.val
              + (if qk = true then 0 else evCost tk p 1)
                * ceilDiv (v.w.val + p.j_rel.val) itk.period.val
              + evCost tk p 2 * ceilDiv (v.w.val + p.j_rel.val) itk.period.val
              + evCost tk p 3 * ceilDiv (v.w.val + p.j_rel.val) itk.period.val
              = (evCost tk p 0 + (if qk = true then 0 else evCost tk p 1) + evCost tk p 2
                  + evCost tk p 3) * ceilDiv (v.w.val + p.j_rel.val) itk.period.val := by ring
          rw [e, hlc, k5, k6, Nat.mul_comm]
    have hs2 := Finset.sum_le_sum hterm
    have hs3 : ∑ k ∈ Finset.range NT.val,
        ((if k = i then ti.budget.val
            + (specO ti p (siteCount sites i sites.length) any_fpu - p.timer.val) else 0)
          + tfun its iti i v.w.val k)
        = ti.budget.val + (specO ti p (siteCount sites i sites.length) any_fpu - p.timer.val)
          + ∑ k ∈ Finset.range NT.val, tfun its iti i v.w.val k := by
      rw [Finset.sum_add_distrib, Finset.sum_ite_eq' (Finset.range NT.val) i]
      simp only [Finset.mem_range, hi, ite_true]
    rw [hs3] at hs2
    have hOge : p.timer.val ≤ specO ti p (siteCount sites i sites.length) any_fpu := by
      unfold specO timedEvents
      split <;> (try split) <;> omega
    rw [hhk]
    omega
  · intro q hq
    unfold deadOf at hq
    obtain ⟨h1, h2, h3⟩ := of_decide_eq_true hq
    refine ⟨?_, srcTask NT.val q, ?_, ?_⟩
    · show (if srcKind NT.val q = 0 ∨ srcKind NT.val q = 3 then some (srcTask NT.val q) else none)
        = none
      rw [h1]; simp
    · show (if srcKind NT.val q = 1 then some (srcTask NT.val q) else none) = some _
      rw [h1]; simp
    · exact hquiet _ h2 h3

/-- **Property 18 over the run, against the Analyzer's own output.**

`cfg` is the Kernel's configuration; `tasks`, `sites`, `p`, `any_fpu` and
`windows` are the Analyzer's inputs for the same Task set (the Tasks, their
Critical_Section sites, the Kernel_Timing_Parameters, and each Task's
period or MIT and deadline). If `analyse_all`, as compiled, returns
results and reports Task `i` schedulable with the response time `v.r`,
then in every run of the Kernel's scheduler with the parameters
`runParams` of those inputs, in every state in which a released Job of
Task `i` has execution left, the nominal instant `a` of its Release has
`now < a + v.r`: every Job of Task `i` executes its last unit within `v.r`
of its nominal Release instant.

The run (`Step`): Releases of a Task at least its period or MIT apart;
every event's Kernel entry within `J_rel` of its instant; a Job's
execution at most its Budget (an overrunning Job of a Partition that stops
overruns is ended at its Budget by its Budget event); each Kernel section
at the transition where the Kernel executes it and at most its
Kernel_Timing_Parameter (dispatch Δ_dispatch, Job end Δ_complete, lock
Δ_lock_in, unlock Δ_lock_out, a Job's Endpoint operations together its
Endpoint cost, a timed event's entry Δ_timer, an intake Δ_irq, the
entry that ends an overrunning Job Δ_detect + Δ_enforce); the
Health_Monitor's responses (a Job ended, Jobs killed, Releases dropped)
inside the Kernel section that applies them.

The hypotheses beside the Analyzer's result:
- `hlen`, `hprio`: the configuration and the inputs describe the same
  Tasks with the same Priorities, each positive;
- `hbud`: every Budget is positive; `hDT`: every deadline is at most the
  period or MIT;
- `hL`: a Task has no Critical_Section on a Resource (`L` is 0 there), or
  every such Critical_Section that can block a Task is one of the sites
  R29.2 counts for that Task and takes at most what R29.2 takes for the
  site (its WCET, or Budget + Δ_detect + Δ_enforce). -/
theorem property18 {cfg : logic.Config NT NR NP}
    (tasks : alloc.vec.Vec TaskIn) (sites : alloc.vec.Vec Site) (p : rsk_analyzer.terms.Params)
    (any_fpu : Bool) (windows : alloc.vec.Vec Window)
    (ds : alloc.vec.Vec rsk_analyzer.terms.Derived) (vs : alloc.vec.Vec Verdict)
    (tss : alloc.vec.Vec (alloc.vec.Vec rsk_analyzer.terms.TermOf)) (quiet : alloc.vec.Vec Bool)
    (L : Nat → Nat → Nat)
    (hall : rsk_analyzer.terms.analyse_all tasks sites p any_fpu windows
      = ok (some (ds, vs, tss, quiet)))
    (hlen : tasks.val.length = NT.val) (hNT : NT.val ≤ 256)
    (hsok : ∀ s ∈ sites.val, s.task.val < tasks.val.length)
    (hwl : windows.val.length = tasks.val.length)
    (hprio : ∀ k t, tasks.val[k]? = some t → 0 < t.priority.val ∧ prio cfg k = t.priority.val)
    (hbud : ∀ t ∈ tasks.val, 0 < t.budget.val)
    (hDT : ∀ w ∈ windows.val, w.deadline.val ≤ w.period.val)
    (hL : ∀ i ti k r, tasks.val[i]? = some ti → k < NT.val → prio cfg k < prio cfg i →
      prio cfg i ≤ ceil cfg r →
      L k r = 0 ∨
      ∃ (j : Nat) (s : Site) (l : TaskIn), sites.val[j]? = some s
        ∧ tasks.val[s.task.val]? = some l
        ∧ l.priority.val < ti.priority.val ∧ ti.priority.val ≤ s.ceiling.val
        ∧ L k r ≤ candidateOf ti l s p)
    (i : Nat) (v : Verdict) (hv : vs.val[i]? = some v) (hsched : v.schedulable = true)
    {st : St NT NR NL}
    (hreach : Reach cfg (runParams NT.val tasks.val sites.val p windows.val L) st)
    (hpos : 0 < st.pend i + st.run i) :
    ∃ a, st.jl i = some a ∧ st.now < a + v.r.val := by
  have hsz : 2 * tasks.val.length + 1 ≤ Usize.max := by
    rw [hlen]; scalar_tac
  have hspec := rsk_analyzer.analyse_all_spec tasks sites p any_fpu windows hsz hsok hwl
  rw [hall] at hspec
  simp only [WP.spec_ok] at hspec
  obtain ⟨hdl, hder, hql, ⟨hvl, htl, its, hil, hits, hverd⟩, hqok⟩ := hspec ds vs tss quiet rfl
  -- By induction on the Priority, lowest first: a Task's bound uses that
  -- of the Tasks below it that the analysis marks quiet.
  have key : ∀ n i v, prio cfg i ≤ n → vs.val[i]? = some v → v.schedulable = true →
      ∀ st : St NT NR NL, Reach cfg (runParams NT.val tasks.val sites.val p windows.val L) st →
        0 < st.pend i + st.run i → ∃ a, st.jl i = some a ∧ st.now < a + v.r.val := by
    intro n
    induction n with
    | zero =>
      intro i v hp hv _ st _ _
      exfalso
      have hi : i < tasks.val.length := by
        have := (List.getElem?_eq_some_iff.mp hv).1; omega
      have := hprio i _ (List.getElem?_eq_getElem hi)
      omega
    | succ n ih =>
      intro i v hp hv hsched st hreach hpos
      have hi : i < tasks.val.length := by
        have := (List.getElem?_eq_some_iff.mp hv).1; omega
      obtain ⟨ti, hti⟩ : ∃ ti, tasks.val[i]? = some ti := ⟨_, List.getElem?_eq_getElem hi⟩
      obtain ⟨wi, hwi⟩ : ∃ wi, windows.val[i]? = some wi :=
        ⟨_, List.getElem?_eq_getElem (by omega)⟩
      obtain ⟨iti, hiti⟩ : ∃ iti, its[i]? = some iti := ⟨_, List.getElem?_eq_getElem (by omega)⟩
      obtain ⟨ts, hts⟩ : ∃ ts, tss.val[i]? = some ts := ⟨_, List.getElem?_eq_getElem (by omega)⟩
      obtain ⟨_, hvo⟩ := hverd i v ti wi iti ts hv hts hti hwi hiti
      refine core tasks.val sites.val p any_fpu windows.val quiet.val its L hlen hNT hwl hql hil
        hits hprio hbud hDT i ti wi iti v hti hwi hiti hvo hsched
        (fun k r => hL i ti k r hti) ?_ hreach hpos
      intro k hqk hlo st' hr' a ha hun
      have hk : k < tasks.val.length := by
        have := (List.getElem?_eq_some_iff.mp hqk).1; omega
      obtain ⟨vk, hvk⟩ : ∃ vk, vs.val[k]? = some vk := ⟨_, List.getElem?_eq_getElem (by omega)⟩
      have hsk := hqok k vk hqk hvk
      obtain ⟨tk, htk⟩ : ∃ tk, tasks.val[k]? = some tk := ⟨_, List.getElem?_eq_getElem hk⟩
      obtain ⟨wk, hwk⟩ : ∃ wk, windows.val[k]? = some wk :=
        ⟨_, List.getElem?_eq_getElem (by omega)⟩
      obtain ⟨itk, hitk⟩ : ∃ itk, its[k]? = some itk := ⟨_, List.getElem?_eq_getElem (by omega)⟩
      obtain ⟨tsk, htsk⟩ : ∃ tsk, tss.val[k]? = some tsk :=
        ⟨_, List.getElem?_eq_getElem (by omega)⟩
      obtain ⟨_, hvok⟩ := hverd k vk tk wk itk tsk hvk htsk htk hwk hitk
      obtain ⟨hfixk, _, hrk, hrdk⟩ := hvok.2.1 hsk
      have hD : (runParams NT.val tasks.val sites.val p windows.val L).D k = wk.deadline.val := by
        show (match windows.val[k]? with
          | some w => w.deadline.val
          | none => 0) = _
        rw [hwk]
      rw [hD]
      rcases hun with hrem | hjw
      · obtain ⟨a', ha', hlt⟩ := ih k vk (by omega) hvk hsk st' hr' hrem
        rw [ha] at ha'; cases ha'
        omega
      · obtain ⟨a', ha', hle⟩ := (reach_wf hr').jwait_inv k hjw
        rw [ha] at ha'; cases ha'
        have hJ : (runParams NT.val tasks.val sites.val p windows.val L).J k = p.j_rel.val := rfl
        have hwpos : 0 < vk.w.val := by
          have := hbud tk (List.mem_of_getElem? htk)
          unfold stepF at hfixk; omega
        omega
  exact key (prio cfg i) i v (Nat.le_refl _) hv hsched st hreach hpos

end P18
