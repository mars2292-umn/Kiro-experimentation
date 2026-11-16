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

end P18
