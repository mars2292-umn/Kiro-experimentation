import Sched.KRun
open Aeneas Aeneas.Std Result

/-! # The timer entry on the code: which slots it serves and what each gets

`Kernel::timer_event` and `Kernel::serve_slot` as extracted
(`Sched/Kernel.lean`). Three facts, each of the form "if the function
returns":

- `serve_slot_cases`: what `serve_slot` does with slot `i` is fixed by the
  slot's kind: a release slot gets `periodic_release`, a deferral slot
  `deferred_release`, a deadline slot `deadline_miss`, a Budget slot
  `budget_event`, each for the slot's own Task; the slot of the end of the
  operating duration gets the safe state (a response on the scheduler
  state); every other slot (unmask, housekeeping, a free slot, a slot
  naming no Task) leaves the scheduler state as it is.
- `timer_event_served`: `timer_event` changes the Kernel's state, after
  reading the time base and charging the Job on top, only by serving
  slots through `serve_slot`, each armed and due (`SlotDue`) in the state
  it is served in.
- `KStep.served_*`: such a served slot is a transition of the run of
  `Sched/KRun.lean`, under the model-side conditions of that transition:
  the handler and its Task are no longer chosen by the run's hypothesis
  but read from the slot. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- Slot `i` of the Kernel's timer slots. -/
abbrev slotOf (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) : logic.time.Slot :=
  k.slots.slots.val[i.val]!

/-- Slot `i` is armed and its instant is at or before the Kernel's reading
of the time base: the test of `timer_event`'s two loops. -/
def SlotDue (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) : Prop :=
  i.val < NS.val ∧ (slotOf k i).armed = true ∧ (slotOf k i).at.val ≤ k.now.val

/-- What `serve_slot` did with slot `i`, by the slot's kind. -/
inductive ServeCase (cfg : logic.Config NT NR NP)
    (k k' : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize) : Prop
  | release (hk : (slotOf k i).kind = logic.time.SLOT_RELEASE)
      (h : logic.kernel.Kernel.periodic_release k cfg (slotOf k i).task i = ok k')
  | defer (hk : (slotOf k i).kind = logic.time.SLOT_DEFER)
      (h : logic.kernel.Kernel.deferred_release k cfg (slotOf k i).task i = ok k')
  | deadline (hk : (slotOf k i).kind = logic.time.SLOT_DEADLINE)
      (h : logic.kernel.Kernel.deadline_miss k cfg (slotOf k i).task i = ok k')
  | budget (hk : (slotOf k i).kind = logic.time.SLOT_BUDGET)
      (h : logic.kernel.Kernel.budget_event k cfg (slotOf k i).task i = ok k')
  | opend (hk : (slotOf k i).kind = logic.time.SLOT_END)
      (h : RSteps cfg k.sched k'.sched)
  | kept (hk : (slotOf k i).kind ≠ logic.time.SLOT_END)
      (hh : (slotOf k i).task.val < NT.val →
        (slotOf k i).kind ≠ logic.time.SLOT_RELEASE ∧ (slotOf k i).kind ≠ logic.time.SLOT_DEFER ∧
        (slotOf k i).kind ≠ logic.time.SLOT_DEADLINE ∧ (slotOf k i).kind ≠ logic.time.SLOT_BUDGET)
      (h : k'.sched = k.sched)

/-- `serve_slot` gives a slot the handler of its kind, for the slot's own
Task. -/
theorem serve_slot_cases {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k') : ServeCase cfg k k' i := by
  unfold logic.kernel.Kernel.serve_slot at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨slot, hslot, h⟩ := h
  obtain ⟨_, hsl⟩ := index_ok hslot
  have hsl : slotOf k i = slot := hsl.symm
  split at h
  · rename_i hk
    simp only [bind_eq_ok] at h
    obtain ⟨s, _, k1, hk1, h⟩ := h
    refine ServeCase.opend (by rw [hsl]; exact hk) ?_
    have h1 := enter_safe_state_r h
    have h2 := push_event_sched hk1
    dsimp only at h2
    rw [h2] at h1
    exact h1
  · rename_i hne
    have hne' : (slotOf k i).kind ≠ logic.time.SLOT_END := by rw [hsl]; exact hne
    split at h
    · rename_i hhk
      simp only [bind_eq_ok] at h
      obtain ⟨_, _, _, _, _, _, s, _, h⟩ := h
      have h := ok_inj h; subst h
      refine ServeCase.kept hne' (fun _ => ?_) rfl
      rw [hsl, hhk]
      simp [logic.time.SLOT_HOUSEKEEPING, logic.time.SLOT_RELEASE, logic.time.SLOT_DEFER,
        logic.time.SLOT_DEADLINE, logic.time.SLOT_BUDGET]
    · split at h
      · split at h
        · rename_i hk
          exact ServeCase.release (by rw [hsl]; exact hk) (by rw [hsl]; exact h)
        · rename_i hn1
          split at h
          · rename_i hk
            exact ServeCase.defer (by rw [hsl]; exact hk) (by rw [hsl]; exact h)
          · rename_i hn3
            split at h
            · rename_i hk
              exact ServeCase.deadline (by rw [hsl]; exact hk) (by rw [hsl]; exact h)
            · rename_i hn2
              split at h
              · rename_i hk
                exact ServeCase.budget (by rw [hsl]; exact hk) (by rw [hsl]; exact h)
              · rename_i hn4
                have hh : (slotOf k i).task.val < NT.val →
                    (slotOf k i).kind ≠ logic.time.SLOT_RELEASE ∧
                    (slotOf k i).kind ≠ logic.time.SLOT_DEFER ∧
                    (slotOf k i).kind ≠ logic.time.SLOT_DEADLINE ∧
                    (slotOf k i).kind ≠ logic.time.SLOT_BUDGET := by
                  intro _; rw [hsl]; exact ⟨hn1, hn3, hn2, hn4⟩
                split at h
                · simp only [bind_eq_ok] at h
                  obtain ⟨s, _, _, _, h⟩ := h
                  split at h
                  · simp only [bind_eq_ok] at h
                    obtain ⟨_, _, h⟩ := h
                    have h := ok_inj h; subst h
                    exact ServeCase.kept hne' hh rfl
                  · have h := ok_inj h; subst h
                    exact ServeCase.kept hne' hh rfl
                · simp only [bind_eq_ok] at h
                  obtain ⟨s, _, h⟩ := h
                  have h := ok_inj h; subst h
                  exact ServeCase.kept hne' hh rfl
      · rename_i hnt
        simp only [bind_eq_ok] at h
        obtain ⟨s, _, h⟩ := h
        have h := ok_inj h; subst h
        refine ServeCase.kept hne' (fun hlt => absurd ?_ hnt) rfl
        rw [hsl] at hlt
        have hcv : (UScalar.cast .Usize slot.task).val = slot.task.val := U8.cast_Usize_val_eq _
        show (UScalar.cast .Usize slot.task).val < NT.val
        rw [hcv]; exact hlt

/-- Slots served one after the other, each armed and due in the state it
is served in. -/
inductive ServedChain (cfg : logic.Config NT NR NP) :
    logic.kernel.Kernel NT NR NP NL NS NE → logic.kernel.Kernel NT NR NP NL NS NE → Prop
  | done (k : logic.kernel.Kernel NT NR NP NL NS NE) : ServedChain cfg k k
  | serve (k k1 k' : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize)
      (hdue : SlotDue k i) (hs : logic.kernel.Kernel.serve_slot k cfg i = ok k1)
      (rest : ServedChain cfg k1 k') : ServedChain cfg k k'

theorem ServedChain.trans {cfg : logic.Config NT NR NP}
    {a b c : logic.kernel.Kernel NT NR NP NL NS NE}
    (h1 : ServedChain cfg a b) (h2 : ServedChain cfg b c) : ServedChain cfg a c := by
  induction h1 with
  | done => exact h2
  | serve k k1 k' i hdue hs _ ih => exact ServedChain.serve k k1 _ i hdue hs (ih h2)

/-- The first loop of `timer_event` (the deadline slots, KD-R9.6). -/
theorem timer_loop0_served {cfg : logic.Config NT NR NP} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NS.val - i.val = n →
      logic.kernel.Kernel.timer_event_loop0 k cfg i = ok k' → ServedChain cfg k k' := by
  intro n
  induction n with
  | zero =>
    intro k i k' hn h
    unfold logic.kernel.Kernel.timer_event_loop0 loop at h
    simp only [logic.kernel.Kernel.timer_event_loop0.body] at h
    have hlt : ¬ i < NS := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    exact ServedChain.done _
  | succ n ih =>
    intro k i k' hn h
    unfold logic.kernel.Kernel.timer_event_loop0 loop at h
    simp only [logic.kernel.Kernel.timer_event_loop0.body] at h
    have hlt : i < NS := by scalar_tac
    simp only [hlt, ite_true, bind_eq_ok] at h
    obtain ⟨cf, ⟨slot, hslot, k1, hk1, i1, hi1, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    have hi1 := usize_add_ok hi1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at hi1
    have hrec : logic.kernel.Kernel.timer_event_loop0 k1 cfg i1 = ok k' := by
      unfold logic.kernel.Kernel.timer_event_loop0
      simp only [logic.kernel.Kernel.timer_event_loop0.body]
      exact hloop
    have hrest := ih k1 i1 k' (by omega) hrec
    obtain ⟨hin, hsl⟩ := index_ok hslot
    have hsl : slotOf k i = slot := hsl.symm
    split at hk1
    · rename_i harm
      split at hk1
      · split at hk1
        · rename_i hat
          refine ServedChain.serve k k1 k' i ⟨hin, by rw [hsl]; exact harm, ?_⟩ hk1 hrest
          rw [hsl]; exact hat
        · have hk1 := ok_inj hk1; subst hk1; exact hrest
      · have hk1 := ok_inj hk1; subst hk1; exact hrest
    · have hk1 := ok_inj hk1; subst hk1; exact hrest

/-- The second loop of `timer_event` (every slot). -/
theorem timer_loop1_served {cfg : logic.Config NT NR NP} :
    ∀ (n : Nat) (k : logic.kernel.Kernel NT NR NP NL NS NE) (i : Usize)
      (k' : logic.kernel.Kernel NT NR NP NL NS NE),
      NS.val - i.val = n →
      logic.kernel.Kernel.timer_event_loop1 k cfg i = ok k' → ServedChain cfg k k' := by
  intro n
  induction n with
  | zero =>
    intro k i k' hn h
    unfold logic.kernel.Kernel.timer_event_loop1 loop at h
    simp only [logic.kernel.Kernel.timer_event_loop1.body] at h
    have hlt : ¬ i < NS := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    exact ServedChain.done _
  | succ n ih =>
    intro k i k' hn h
    unfold logic.kernel.Kernel.timer_event_loop1 loop at h
    simp only [logic.kernel.Kernel.timer_event_loop1.body] at h
    have hlt : i < NS := by scalar_tac
    simp only [hlt, ite_true, bind_eq_ok] at h
    obtain ⟨cf, ⟨slot, hslot, k1, hk1, i1, hi1, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    have hi1 := usize_add_ok hi1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at hi1
    have hrec : logic.kernel.Kernel.timer_event_loop1 k1 cfg i1 = ok k' := by
      unfold logic.kernel.Kernel.timer_event_loop1
      simp only [logic.kernel.Kernel.timer_event_loop1.body]
      exact hloop
    have hrest := ih k1 i1 k' (by omega) hrec
    obtain ⟨hin, hsl⟩ := index_ok hslot
    have hsl : slotOf k i = slot := hsl.symm
    split at hk1
    · rename_i harm
      split at hk1
      · rename_i hat
        refine ServedChain.serve k k1 k' i ⟨hin, by rw [hsl]; exact harm, ?_⟩ hk1 hrest
        rw [hsl]; exact hat
      · have hk1 := ok_inj hk1; subst hk1; exact hrest
    · have hk1 := ok_inj hk1; subst hk1; exact hrest

/-- `timer_event`: after the reading of the time base and the charge of
the Job on top, which leave the scheduler state as it is, the Kernel's
state changes only by slots served, each armed and due when served. -/
theorem timer_event_served {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {now : U64} {rc : U32}
    (h : logic.kernel.Kernel.timer_event k cfg now rc = ok k') :
    ∃ k1 : logic.kernel.Kernel NT NR NP NL NS NE,
      k1.sched = k.sched ∧ ServedChain cfg k1 k' := by
  unfold logic.kernel.Kernel.timer_event at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨k0, hobs, h⟩ := h
  have hsch := observe_sched hobs
  split at h
  · have h := ok_inj h; subst h
    exact ⟨k0, hsch, ServedChain.done _⟩
  · simp only [bind_eq_ok] at h
    obtain ⟨b, _, k2, hl0, hl1⟩ := h
    exact ⟨{ k0 with budget := b }, hsch,
      (timer_loop0_served _ _ _ _ rfl hl0).trans (timer_loop1_served _ _ _ _ rfl hl1)⟩

/-! ## The handler of a slot that names a Task -/

theorem slot_kinds :
    logic.time.SLOT_RELEASE = 1#u8 ∧ logic.time.SLOT_DEADLINE = 2#u8 ∧
    logic.time.SLOT_DEFER = 3#u8 ∧ logic.time.SLOT_BUDGET = 4#u8 ∧
    logic.time.SLOT_HOUSEKEEPING = 5#u8 ∧ logic.time.SLOT_END = 6#u8 := by
  simp [logic.time.SLOT_RELEASE, logic.time.SLOT_DEADLINE, logic.time.SLOT_DEFER,
    logic.time.SLOT_BUDGET, logic.time.SLOT_HOUSEKEEPING, logic.time.SLOT_END]

/-- A release slot of a Task is served by `periodic_release` of that Task. -/
theorem serve_slot_release {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k')
    (hk : (slotOf k i).kind = logic.time.SLOT_RELEASE) (ht : (slotOf k i).task.val < NT.val) :
    logic.kernel.Kernel.periodic_release k cfg (slotOf k i).task i = ok k' := by
  obtain ⟨h1, h2, h3, h4, h5, h6⟩ := slot_kinds
  cases serve_slot_cases h with
  | release _ h => exact h
  | defer hk' _ => rw [hk, h1, h3] at hk'; simp at hk'
  | deadline hk' _ => rw [hk, h1, h2] at hk'; simp at hk'
  | budget hk' _ => rw [hk, h1, h4] at hk'; simp at hk'
  | opend hk' _ => rw [hk, h1, h6] at hk'; simp at hk'
  | kept _ hh _ => exact absurd hk (hh ht).1

/-- A deferral slot of a Task is served by `deferred_release` of that Task. -/
theorem serve_slot_defer {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k')
    (hk : (slotOf k i).kind = logic.time.SLOT_DEFER) (ht : (slotOf k i).task.val < NT.val) :
    logic.kernel.Kernel.deferred_release k cfg (slotOf k i).task i = ok k' := by
  obtain ⟨h1, h2, h3, h4, h5, h6⟩ := slot_kinds
  cases serve_slot_cases h with
  | release hk' _ => rw [hk, h1, h3] at hk'; simp at hk'
  | defer _ h => exact h
  | deadline hk' _ => rw [hk, h3, h2] at hk'; simp at hk'
  | budget hk' _ => rw [hk, h3, h4] at hk'; simp at hk'
  | opend hk' _ => rw [hk, h3, h6] at hk'; simp at hk'
  | kept _ hh _ => exact absurd hk (hh ht).2.1

/-- A deadline slot of a Task is served by `deadline_miss` of that Task. -/
theorem serve_slot_deadline {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k')
    (hk : (slotOf k i).kind = logic.time.SLOT_DEADLINE) (ht : (slotOf k i).task.val < NT.val) :
    logic.kernel.Kernel.deadline_miss k cfg (slotOf k i).task i = ok k' := by
  obtain ⟨h1, h2, h3, h4, h5, h6⟩ := slot_kinds
  cases serve_slot_cases h with
  | release hk' _ => rw [hk, h1, h2] at hk'; simp at hk'
  | defer hk' _ => rw [hk, h3, h2] at hk'; simp at hk'
  | deadline _ h => exact h
  | budget hk' _ => rw [hk, h2, h4] at hk'; simp at hk'
  | opend hk' _ => rw [hk, h2, h6] at hk'; simp at hk'
  | kept _ hh _ => exact absurd hk (hh ht).2.2.1

/-- A Budget slot of a Task is served by `budget_event` of that Task. -/
theorem serve_slot_budget {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k')
    (hk : (slotOf k i).kind = logic.time.SLOT_BUDGET) (ht : (slotOf k i).task.val < NT.val) :
    logic.kernel.Kernel.budget_event k cfg (slotOf k i).task i = ok k' := by
  obtain ⟨h1, h2, h3, h4, h5, h6⟩ := slot_kinds
  cases serve_slot_cases h with
  | release hk' _ => rw [hk, h1, h4] at hk'; simp at hk'
  | defer hk' _ => rw [hk, h3, h4] at hk'; simp at hk'
  | deadline hk' _ => rw [hk, h2, h4] at hk'; simp at hk'
  | budget _ h => exact h
  | opend hk' _ => rw [hk, h4, h6] at hk'; simp at hk'
  | kept _ hh _ => exact absurd hk (hh ht).2.2.2

/-- Any other slot but the end of the operating duration (unmask,
housekeeping, a free slot) is served with the scheduler state kept. -/
theorem serve_slot_kept {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k')
    (h1 : (slotOf k i).kind ≠ logic.time.SLOT_RELEASE) (h3 : (slotOf k i).kind ≠ logic.time.SLOT_DEFER)
    (h2 : (slotOf k i).kind ≠ logic.time.SLOT_DEADLINE) (h4 : (slotOf k i).kind ≠ logic.time.SLOT_BUDGET)
    (h6 : (slotOf k i).kind ≠ logic.time.SLOT_END) : k'.sched = k.sched := by
  cases serve_slot_cases h with
  | release hk' _ => exact absurd hk' h1
  | defer hk' _ => exact absurd hk' h3
  | deadline hk' _ => exact absurd hk' h2
  | budget hk' _ => exact absurd hk' h4
  | opend hk' _ => exact absurd hk' h6
  | kept _ _ h => exact h

/-! ## A served slot as a transition of the run

The run's timer transitions (`KStep.periodic`, `deferred`, `deadline`,
`plain_h`, `bserved`) name the handler and its Task. For a slot that
`timer_event` serves, both are read from the slot: the transition's
Kernel-side hypothesis is `serve_slot` of that slot. The model-side
conditions are those of the transition. -/

variable {cfg : logic.Config NT NR NP} {P : Params}

/-- A release slot served is the run's `periodic` transition. -/
theorem KStep.served_release (x : KSt NT NR NP NL NS NE) (i : Usize) (q d c : Nat)
    (k' : logic.kernel.Kernel NT NR NP NL NS NE)
    (hs : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
    (hkind : (slotOf x.k i).kind = logic.time.SLOT_RELEASE)
    (ht : (slotOf x.k i).task.val < NT.val)
    (hq : q < P.NQ) (hk : x.ks = 0) (hw : x.ewait q = true)
    (hrel : P.relOf q = some (slotOf x.k i).task.val) (hnd : P.dlOf q = none) (hc : c ≤ P.EC q)
    (hmade : rel x.k.sched (slotOf x.k i).task.val = 0 → rel k'.sched (slotOf x.k i).task.val = 1 →
      x.jwait (slotOf x.k i).task.val = true ∧ 1 ≤ d ∧ d ≤ P.C (slotOf x.k i).task.val) :
    ∃ y, KStep cfg P x y ∧ y.k = k' :=
  ⟨_, KStep.periodic x q (slotOf x.k i).task i d c k' hq hk hw hrel hnd hc
      (serve_slot_release hs hkind ht) hmade, by split <;> rfl⟩

/-- A deferral slot served is the run's `deferred` transition. -/
theorem KStep.served_defer (x : KSt NT NR NP NL NS NE) (i : Usize) (q d c : Nat)
    (k' : logic.kernel.Kernel NT NR NP NL NS NE)
    (hs : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
    (hkind : (slotOf x.k i).kind = logic.time.SLOT_DEFER)
    (ht : (slotOf x.k i).task.val < NT.val)
    (hq : q < P.NQ) (hk : x.ks = 0) (hw : x.ewait q = true)
    (hrel : P.relOf q = some (slotOf x.k i).task.val) (hnd : P.dlOf q = none) (hc : c ≤ P.EC q)
    (hmade : rel x.k.sched (slotOf x.k i).task.val = 2 → rel k'.sched (slotOf x.k i).task.val = 1 →
      x.jwait (slotOf x.k i).task.val = true ∧ 1 ≤ d ∧ d ≤ P.C (slotOf x.k i).task.val) :
    ∃ y, KStep cfg P x y ∧ y.k = k' :=
  ⟨_, KStep.deferred x q (slotOf x.k i).task i d c k' hq hk hw hrel hnd hc
      (serve_slot_defer hs hkind ht) hmade, by split <;> rfl⟩

/-- A deadline slot served is the run's `deadline` transition. -/
theorem KStep.served_deadline (x : KSt NT NR NP NL NS NE) (i : Usize) (q c : Nat)
    (k' : logic.kernel.Kernel NT NR NP NL NS NE)
    (hs : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
    (hkind : (slotOf x.k i).kind = logic.time.SLOT_DEADLINE)
    (ht : (slotOf x.k i).task.val < NT.val)
    (hq : q < P.NQ) (hk : x.ks = 0) (hw : x.ewait q = true) (hc : c ≤ P.EC q)
    (hdl : ∀ t', P.dlOf q = some t' → ∃ a, x.jl t' = some a ∧ a + P.D t' ≤ x.now ∧
      (0 < x.pend t' + x.run t' ∨ x.jwait t' = true)) :
    ∃ y, KStep cfg P x y ∧ y.k = k' :=
  ⟨_, KStep.deadline x q c (slotOf x.k i).task i k' hq hk hw hc hdl
      (serve_slot_deadline hs hkind ht), rfl⟩

/-- A Budget slot served inside a timer entry in progress is the run's
`bserved` transition. -/
theorem KStep.served_budget (x : KSt NT NR NP NL NS NE) (i : Usize)
    (k' : logic.kernel.Kernel NT NR NP NL NS NE)
    (hs : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
    (hkind : (slotOf x.k i).kind = logic.time.SLOT_BUDGET)
    (ht : (slotOf x.k i).task.val < NT.val) :
    ∃ y, KStep cfg P x y ∧ y.k = k' :=
  ⟨_, KStep.bserved x (slotOf x.k i).task i k' (serve_slot_budget hs hkind ht), rfl⟩

/-- An unmask, housekeeping or free slot served is the run's `plain_h`
transition: that it leaves the scheduler state is read from the code. -/
theorem KStep.served_kept (x : KSt NT NR NP NL NS NE) (i : Usize) (q c : Nat)
    (k' : logic.kernel.Kernel NT NR NP NL NS NE)
    (hs : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
    (h1 : (slotOf x.k i).kind ≠ logic.time.SLOT_RELEASE)
    (h3 : (slotOf x.k i).kind ≠ logic.time.SLOT_DEFER)
    (h2 : (slotOf x.k i).kind ≠ logic.time.SLOT_DEADLINE)
    (h4 : (slotOf x.k i).kind ≠ logic.time.SLOT_BUDGET)
    (h6 : (slotOf x.k i).kind ≠ logic.time.SLOT_END)
    (hq : q < P.NQ) (hk : x.ks = 0) (hw : x.ewait q = true) (hc : c ≤ P.EC q)
    (hnd : P.dlOf q = none) :
    ∃ y, KStep cfg P x y ∧ y.k = k' :=
  ⟨_, KStep.plain_h x q c k' hq hk hw hc hnd (serve_slot_kept hs h1 h3 h2 h4 h6), rfl⟩

/-! ## The slot of the end of the operating duration

`serve_slot` gives it the safe state (`enter_safe_state`), entered from a
Kernel state whose scheduler state is the one the slot was found in: the
run's `opend` transition. -/

/-- The slot of the end of the operating duration is served by the safe
state, entered with the scheduler state as the slot found it. -/
theorem serve_slot_end {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k')
    (hk : (slotOf k i).kind = logic.time.SLOT_END) :
    ∃ k1 : logic.kernel.Kernel NT NR NP NL NS NE,
      k1.sched = k.sched ∧ logic.kernel.Kernel.enter_safe_state k1 cfg = ok k' := by
  unfold logic.kernel.Kernel.serve_slot at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨slot, hslot, h⟩ := h
  obtain ⟨_, hsl⟩ := index_ok hslot
  have hsl : slotOf k i = slot := hsl.symm
  rw [hsl] at hk
  rw [if_pos hk] at h
  simp only [bind_eq_ok] at h
  obtain ⟨s, _, k1, hk1, h⟩ := h
  have hp := push_event_sched hk1
  exact ⟨k1, hp, h⟩

/-- The slot of the end of the operating duration served is the run's
`opend` transition. -/
theorem KStep.served_end (x : KSt NT NR NP NL NS NE) (i : Usize)
    (k' : logic.kernel.Kernel NT NR NP NL NS NE)
    (hs : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
    (hkind : (slotOf x.k i).kind = logic.time.SLOT_END) :
    ∃ y, KStep cfg P x y ∧ y.k = k' := by
  obtain ⟨k1, h1, h⟩ := serve_slot_end hs hkind
  exact ⟨_, KStep.opend x k1 k' h1 h, rfl⟩

/-! ## A Budget event as an entry of its own, on the code

The run's `bevent_h` transition takes "the scheduler state is kept" as a
hypothesis, and `overrun_h` "the Job on top is ended". Both are read here
from `Kernel::budget_event` as extracted: it keeps the scheduler state
when the account shows no Overrun, and when the Partition's
Overrun_Response only records; under the response that ends the Job it
ends the Job on top through `Sched::finish_top`. -/

/-- A Budget event that finds no Overrun in the account keeps the
scheduler state (the slot is armed again). -/
theorem budget_event_no_overrun {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k')
    (hn : logic.budget.Budget.overrun k.budget cfg t = ok false) : k'.sched = k.sched := by
  unfold logic.kernel.Kernel.budget_event at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨s, hs, h⟩ := h
  split at h
  · have h := ok_inj h; subst h; rfl
  · simp only [bind_eq_ok] at h
    obtain ⟨i1, hi1, i2, hi2, h⟩ := h
    split at h
    · have h := ok_inj h; subst h; rfl
    · simp only [bind_eq_ok] at h
      obtain ⟨b, hb, h⟩ := h
      split at h
      · have h := ok_inj h; subst h; rfl
      · simp only [bind_eq_ok] at h
        obtain ⟨b1, hb1, h⟩ := h
        rw [hn] at hb1
        have hb1 := ok_inj hb1; subst hb1
        simp only [Bool.false_eq_true, if_false] at h
        have ha := arm_budget_sched h
        exact ha

/-- A response that only records returns the Kernel state it was given. -/
theorem apply_response_record {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k')
    (hr : response = logic.RESP_RECORD_ONLY ∨ response = logic.RESP_RECORD_AND_CONTINUE) :
    k' = k := by
  unfold logic.kernel.Kernel.apply_response at h
  rcases hr with hr | hr
  · rw [if_pos hr] at h; exact (ok_inj h).symm
  · split at h <;>
      first
        | exact (ok_inj h).symm
        | (rw [if_pos hr] at h; exact (ok_inj h).symm)

/-- What a Budget event did: it kept the scheduler state, or the account
showed an Overrun of the Job on top, not yet reported, and the event
applied the Overrun_Response of the Task's Partition to a Kernel state
with the scheduler state unchanged. -/
theorem budget_event_cases {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k') :
    k'.sched = k.sched ∨
    ∃ (k1 : logic.kernel.Kernel NT NR NP NL NS NE) (tc : logic.TaskCfg)
      (pc : logic.PartitionCfg),
      logic.budget.Budget.overrun k.budget cfg t = ok true ∧
      cfg.tasks.index_usize (UScalar.cast .Usize t) = ok tc ∧
      cfg.partitions.index_usize (UScalar.cast .Usize tc.partition) = ok pc ∧
      k1.sched = k.sched ∧
      logic.kernel.Kernel.apply_response k1 cfg tc.partition pc.overrun_response
        logic.log.EV_OVERRUN t = ok k' := by
  unfold logic.kernel.Kernel.budget_event at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨s, hs, h⟩ := h
  split at h
  · have h := ok_inj h; subst h; exact Or.inl rfl
  · simp only [bind_eq_ok] at h
    obtain ⟨i1, hi1, i2, hi2, h⟩ := h
    split at h
    · have h := ok_inj h; subst h; exact Or.inl rfl
    · simp only [bind_eq_ok] at h
      obtain ⟨b, hb, h⟩ := h
      split at h
      · have h := ok_inj h; subst h; exact Or.inl rfl
      · simp only [bind_eq_ok] at h
        obtain ⟨b1, hb1, h⟩ := h
        split at h
        · rename_i hb1t
          simp only [bind_eq_ok] at h
          obtain ⟨b2, hb2, tc, htc, i6, hi6, i7, hi7, self1, hpe, pc, hpc, h⟩ := h
          subst hb1t
          have hp := push_event_sched hpe
          exact Or.inr ⟨self1, tc, pc, hb1, htc, hpc, hp, h⟩
        · have ha := arm_budget_sched h
          exact Or.inl ha

/-- A Budget slot served as an entry of its own, the account showing no
Overrun, is the run's `bevent_h` transition: that the scheduler state is
kept is read from the code. -/
theorem KStep.served_bevent (x : KSt NT NR NP NL NS NE) (i : Usize)
    (k0 : logic.kernel.Kernel NT NR NP NL NS NE) (r0 : Option U8 × Bool)
    (k' : logic.kernel.Kernel NT NR NP NL NS NE) (c : Nat)
    (hs : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
    (hkind : (slotOf x.k i).kind = logic.time.SLOT_BUDGET)
    (ht : (slotOf x.k i).task.val < NT.val)
    (hn : logic.budget.Budget.overrun x.k.budget cfg (slotOf x.k i).task = ok false)
    (hset : logic.kernel.Kernel.settle k0 cfg = ok (r0, x.k))
    (harmed : topBudgetArmed x.k cfg)
    (hk : x.ks = 0) (hd : 0 < x.k.sched.depth.val)
    (hrun : 0 < x.run x.top) (hrb : 0 < x.rb x.top) (hc : c ≤ P.Kt) :
    ∃ y, KStep cfg P x y ∧ y.k = k' :=
  ⟨_, KStep.bevent_h x k0 r0 hset harmed hk c k' hd hrun hrb hc
      (budget_event_no_overrun (serve_slot_budget hs hkind ht) hn), rfl⟩

/-- A Budget slot served as an entry of its own, the Partition's
Overrun_Response one that only records, is the run's `bevent_h`
transition whether the account shows an Overrun or not. -/
theorem KStep.served_bevent_recorded (x : KSt NT NR NP NL NS NE) (i : Usize)
    (k0 : logic.kernel.Kernel NT NR NP NL NS NE) (r0 : Option U8 × Bool)
    (k' : logic.kernel.Kernel NT NR NP NL NS NE) (c : Nat)
    (hs : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
    (hkind : (slotOf x.k i).kind = logic.time.SLOT_BUDGET)
    (ht : (slotOf x.k i).task.val < NT.val)
    (hrec : ∀ (tc : logic.TaskCfg) (pc : logic.PartitionCfg),
      cfg.tasks.index_usize (UScalar.cast .Usize (slotOf x.k i).task) = ok tc →
      cfg.partitions.index_usize (UScalar.cast .Usize tc.partition) = ok pc →
      pc.overrun_response = logic.RESP_RECORD_ONLY ∨
        pc.overrun_response = logic.RESP_RECORD_AND_CONTINUE)
    (hset : logic.kernel.Kernel.settle k0 cfg = ok (r0, x.k))
    (harmed : topBudgetArmed x.k cfg)
    (hk : x.ks = 0) (hd : 0 < x.k.sched.depth.val)
    (hrun : 0 < x.run x.top) (hrb : 0 < x.rb x.top) (hc : c ≤ P.Kt) :
    ∃ y, KStep cfg P x y ∧ y.k = k' := by
  have hkept : k'.sched = x.k.sched := by
    rcases budget_event_cases (serve_slot_budget hs hkind ht) with h | ⟨k1, tc, pc, _, htc, hpc, h1, ha⟩
    · exact h
    · rw [apply_response_record ha (hrec tc pc htc hpc), h1]
  exact ⟨_, KStep.bevent_h x k0 r0 hset harmed hk c k' hd hrun hrb hc hkept, rfl⟩

/-! ### The Overrun that ends the Job -/

/-- A Budget event for the Job on top, its Overrun not yet reported and
shown by the account, applies the Overrun_Response of the Task's
Partition to a Kernel state with the scheduler state unchanged. -/
theorem budget_event_overrun {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k')
    (hd : 0 < k.sched.depth.val)
    (htop : k.sched.stack.val[k.sched.depth.val - 1]! = t)
    (hrep : k.budget.reported.val[t.val]! = false)
    (hov : logic.budget.Budget.overrun k.budget cfg t = ok true) :
    ∃ (k1 : logic.kernel.Kernel NT NR NP NL NS NE) (tc : logic.TaskCfg)
      (pc : logic.PartitionCfg),
      cfg.tasks.index_usize (UScalar.cast .Usize t) = ok tc ∧
      cfg.partitions.index_usize (UScalar.cast .Usize tc.partition) = ok pc ∧
      k1.sched = k.sched ∧
      logic.kernel.Kernel.apply_response k1 cfg tc.partition pc.overrun_response
        logic.log.EV_OVERRUN t = ok k' := by
  unfold logic.kernel.Kernel.budget_event at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨s, hs, h⟩ := h
  split at h
  · rename_i h0
    rw [h0] at hd
    simp at hd
  · simp only [bind_eq_ok] at h
    obtain ⟨i1, hi1, i2, hi2, h⟩ := h
    obtain ⟨_, hiv⟩ := usize_sub_ok hi1
    obtain ⟨_, hi2e⟩ := index_ok hi2
    have h1 : (1#usize).val = 1 := rfl
    rw [hiv, h1, htop] at hi2e
    subst hi2e
    split at h
    · rename_i hne
      simp at hne
    · simp only [bind_eq_ok] at h
      obtain ⟨b, hb, h⟩ := h
      obtain ⟨_, hbe⟩ := index_ok hb
      rw [U8.cast_Usize_val_eq, hrep] at hbe
      subst hbe
      simp only [Bool.false_eq_true, if_false, bind_eq_ok] at h
      obtain ⟨b1, hb1, h⟩ := h
      rw [hov] at hb1
      have hb1 := ok_inj hb1
      subst hb1
      simp only [if_true, bind_eq_ok] at h
      obtain ⟨b2, hb2, tc, htc, i6, hi6, i7, hi7, self1, hpe, pc, hpc, h⟩ := h
      have hp := push_event_sched hpe
      exact ⟨self1, tc, pc, htc, hpc, hp, h⟩

theorem end_top_release_finish {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top_release k cfg = ok k') :
    ∃ r, logic.sched.Sched.finish_top k.sched cfg = ok (r, k'.sched) := by
  unfold logic.kernel.Kernel.end_top_release at h
  peel h
  all_goals
    try dsimp only
    try (have hp := push_event_sched (by assumption); rw [hp])
    try dsimp only
    exact ⟨_, by rw [finish_top_eq]; assumption⟩

/-- `Kernel::end_top` ends the Job on top through `Sched::finish_top`. -/
theorem end_top_finish {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.end_top k cfg = ok k') :
    ∃ r, logic.sched.Sched.finish_top k.sched cfg = ok (r, k'.sched) := by
  unfold logic.kernel.Kernel.end_top at h
  peel h
  · rw [arm_budget_sched h]; exact end_top_release_finish ‹_›
  · exact end_top_release_finish ‹_›

/-- Whether the Job on top holds no Resource is read from the scheduler
state alone. -/
theorem top_holds_nothing_congr {k1 k : logic.kernel.Kernel NT NR NP NL NS NE}
    (h : k1.sched = k.sched) :
    logic.kernel.Kernel.top_holds_nothing_exec k1 = logic.kernel.Kernel.top_holds_nothing_exec k := by
  unfold logic.kernel.Kernel.top_holds_nothing_exec
    logic.kernel.Kernel.top_holds_nothing_exec_loop
  simp only [logic.kernel.Kernel.top_holds_nothing_exec_loop.body, h]

/-- The response that ends the Job, applied for the Job on top while it
holds no Resource, ends it through `Sched::finish_top`. -/
theorem apply_response_end_job {k k' : logic.kernel.Kernel NT NR NP NL NS NE}
    {cfg : logic.Config NT NR NP} {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k')
    (hr : response = logic.RESP_END_JOB)
    (hd : 0 < k.sched.depth.val)
    (htop : k.sched.stack.val[k.sched.depth.val - 1]! = t)
    (hh : logic.kernel.Kernel.top_holds_nothing_exec k = ok true) :
    ∃ r, logic.sched.Sched.finish_top k.sched cfg = ok (r, k'.sched) := by
  have e1 : response ≠ logic.RESP_RECORD_ONLY := by
    rw [hr]; simp [logic.RESP_END_JOB, logic.RESP_RECORD_ONLY]
  have e2 : response ≠ logic.RESP_RECORD_AND_CONTINUE := by
    rw [hr]; simp [logic.RESP_END_JOB, logic.RESP_RECORD_AND_CONTINUE]
  have hd' : k.sched.depth > 0#usize := by scalar_tac
  unfold logic.kernel.Kernel.apply_response at h
  rw [if_neg e1, if_neg e2, if_pos hr, if_pos hd'] at h
  simp only [bind_eq_ok, lift, bind_ok] at h
  obtain ⟨i, hi, top, htp, h⟩ := h
  obtain ⟨_, hiv⟩ := usize_sub_ok hi
  obtain ⟨_, hte⟩ := index_ok htp
  have h1 : (1#usize).val = 1 := rfl
  rw [hiv, h1, htop] at hte
  rw [if_pos hte] at h
  simp only [bind_eq_ok] at h
  obtain ⟨hn, hhn, h⟩ := h
  rw [hh] at hhn
  have hhn := ok_inj hhn
  subst hhn
  simp only [if_true, bind_eq_ok] at h
  obtain ⟨k1, hp, k2, he, tc, htc, h⟩ := h
  obtain ⟨r, hf⟩ := end_top_finish he
  rw [push_event_sched hp] at hf
  split at h
  · rw [end_mit_window_sched h]; exact ⟨r, hf⟩
  · have h := ok_inj h; subst h; exact ⟨r, hf⟩

/-- A Budget slot served as an entry of its own, the account showing an
Overrun of the Job on top and the Partition's Overrun_Response the one
that ends the Job, is the run's `overrun_h` transition: that the Job on
top is ended through `Sched::finish_top` is read from the code. -/
theorem KStep.served_overrun (x : KSt NT NR NP NL NS NE) (i : Usize)
    (k0 : logic.kernel.Kernel NT NR NP NL NS NE) (r0 : Option U8 × Bool)
    (k' : logic.kernel.Kernel NT NR NP NL NS NE) (c : Nat)
    (hs : logic.kernel.Kernel.serve_slot x.k cfg i = ok k')
    (hkind : (slotOf x.k i).kind = logic.time.SLOT_BUDGET)
    (ht : (slotOf x.k i).task.val < NT.val)
    (htop : x.k.sched.stack.val[x.k.sched.depth.val - 1]! = (slotOf x.k i).task)
    (hrep : x.k.budget.reported.val[(slotOf x.k i).task.val]! = false)
    (hov : logic.budget.Budget.overrun x.k.budget cfg (slotOf x.k i).task = ok true)
    (hend : ∀ (tc : logic.TaskCfg) (pc : logic.PartitionCfg),
      cfg.tasks.index_usize (UScalar.cast .Usize (slotOf x.k i).task) = ok tc →
      cfg.partitions.index_usize (UScalar.cast .Usize tc.partition) = ok pc →
      pc.overrun_response = logic.RESP_END_JOB)
    (hh : logic.kernel.Kernel.top_holds_nothing_exec x.k = ok true)
    (hset : logic.kernel.Kernel.settle k0 cfg = ok (r0, x.k))
    (harmed : topBudgetArmed x.k cfg)
    (hk : x.ks = 0) (hj : JitterOk P x.st)
    (hd : 0 < x.k.sched.depth.val)
    (hr0 : 0 < x.run x.top) (hr : x.run x.top ≤ 1)
    (hstop : P.stop x.top = true) (hrb : 0 < x.rb x.top)
    (hc : c ≤ P.Kov)
    (hal : ∀ p, topEntry x.k.sched p → c + 1 ≤ x.allow p) :
    ∃ y, KStep cfg P x y ∧ y.k = k' := by
  obtain ⟨k1, tc, pc, htc, hpc, h1, ha⟩ :=
    budget_event_overrun (serve_slot_budget hs hkind ht) hd htop hrep hov
  obtain ⟨r, hf⟩ := apply_response_end_job ha (hend tc pc htc hpc)
    (by rw [h1]; exact hd) (by rw [h1]; exact htop)
    (by rw [top_holds_nothing_congr h1]; exact hh)
  rw [h1] at hf
  exact ⟨_, KStep.overrun_h x k0 r0 hset harmed hk hj c r k' hd hr0 hr hstop hrb hc hal hf, rfl⟩

end P18
