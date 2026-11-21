import Sched.Calls
open Aeneas Aeneas.Std Result

/-! # Timed-event slots, Endpoint buffers and the event log refine their models (2026-11-19)

Property 11 of the design, which Verus proves of `rsk_logic::logic::time`
(`Slots`), `endpoint` (`Fifo`) and `log` (`Log`), stated of the functions
as Aeneas extracts them. Every theorem is total: for the inputs its
precondition admits the function returns, without overflow or an index out
of bounds, and the result is as stated (so these are also part of
Property 21 for these functions). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NS NE : Usize}

/-! ## The slot table as a map from slot to armed instant -/

/-- The table as a map: the instant of every armed slot. -/
def sview (s : logic.time.Slots NS) (j : Nat) : Option Nat :=
  if j < NS.val ∧ (s.slots.val[j]!).armed = true then some (s.slots.val[j]!).«at».val else none

theorem slot_free : logic.time.SLOT_FREE = 0#u8 := by
  unfold logic.time.SLOT_FREE; rfl

theorem set_same {α} [Inhabited α] {n : Usize} (v : Array α n) (i : Usize) (x : α)
    (hi : i.val < n.val) : (v.set i x).val[i.val]! = x := by
  rw [get_set _ _ _ _ hi]; simp

theorem set_other {α} [Inhabited α] {n : Usize} (v : Array α n) (i : Usize) (x : α) (j : Nat)
    (hi : i.val < n.val) (hj : j ≠ i.val) : (v.set i x).val[j]! = v.val[j]! := by
  rw [get_set _ _ _ _ hi, if_neg hj]

/-- `Slots::arm`: the map with slot `i` set to the instant. -/
theorem arm_spec (s : logic.time.Slots NS) (i : Usize) (a : U64) (kind task : U8)
    (hi : i.val < NS.val) :
    logic.time.Slots.arm s i a kind task ⦃ s' =>
      s'.slots.val[i.val]! = { armed := true, «at» := a, kind, task } ∧
      (∀ j, j ≠ i.val → s'.slots.val[j]! = s.slots.val[j]!) ∧
      (∀ j, sview s' j = if j = i.val then some a.val else sview s j) ⦄ := by
  unfold logic.time.Slots.arm
  step as ⟨ arr, harr ⟩
  subst harr
  have e1 := set_same s.slots i { armed := true, «at» := a, kind, task } hi
  have e2 := fun j hj => set_other s.slots i { armed := true, «at» := a, kind, task } j hi hj
  refine ⟨e1, e2, fun j => ?_⟩
  simp only [sview]
  by_cases hj : j = i.val
  · subst hj; rw [e1]; simp [hi]
  · rw [e2 j hj, if_neg hj]

def freeSlot : logic.time.Slot :=
  { armed := false, «at» := 0#u64, kind := logic.time.SLOT_FREE, task := 0#u8 }

/-- `Slots::disarm`: the map with slot `i` removed. -/
theorem disarm_spec (s : logic.time.Slots NS) (i : Usize) (hi : i.val < NS.val) :
    logic.time.Slots.disarm s i ⦃ s' =>
      (s'.slots.val[i.val]!).armed = false ∧
      (∀ j, j ≠ i.val → s'.slots.val[j]! = s.slots.val[j]!) ∧
      (∀ j, sview s' j = if j = i.val then none else sview s j) ⦄ := by
  unfold logic.time.Slots.disarm
  step as ⟨ arr, harr ⟩
  subst harr
  have e1 := set_same s.slots i freeSlot hi
  have e2 := fun j hj => set_other s.slots i freeSlot j hi hj
  unfold freeSlot at e1 e2
  refine ⟨by rw [e1], e2, fun j => ?_⟩
  simp only [sview]
  by_cases hj : j = i.val
  · subst hj; rw [e1]; simp
  · rw [e2 j hj, if_neg hj]

/-- What `Slots::next` returns for the first `n` slots: the armed slot of
least instant, the lowest index among equals, or nothing when none of them
is armed. -/
def NextOk (s : logic.time.Slots NS) (n : Nat) : Option Usize → Prop
  | some b => b.val < n ∧ (s.slots.val[b.val]!).armed = true ∧
      (∀ j, j < n → (s.slots.val[j]!).armed = true →
        (s.slots.val[b.val]!).«at».val ≤ (s.slots.val[j]!).«at».val) ∧
      (∀ j, j < n → (s.slots.val[j]!).armed = true →
        (s.slots.val[j]!).«at».val = (s.slots.val[b.val]!).«at».val → b.val ≤ j)
  | none => ∀ j, j < n → (s.slots.val[j]!).armed = false

/-- A slot that is not armed changes nothing. -/
theorem nextok_skip {s : logic.time.Slots NS} {i : Nat} {best : Option Usize}
    (h : NextOk s i best) (hna : (s.slots.val[i]!).armed = false) : NextOk s (i + 1) best := by
  cases best with
  | none =>
    simp only [NextOk] at h ⊢
    intro j hj
    by_cases hji : j = i
    · subst hji; exact hna
    · exact h j (by omega)
  | some b =>
    obtain ⟨hb1, hb2, hb3, hb4⟩ := h
    simp only [NextOk]
    refine ⟨by omega, hb2, fun j hj ha => ?_, fun j hj ha he => ?_⟩
    · by_cases hji : j = i
      · subst hji; rw [hna] at ha; cases ha
      · exact hb3 j (by omega) ha
    · by_cases hji : j = i
      · subst hji; rw [hna] at ha; cases ha
      · exact hb4 j (by omega) ha he

/-- The first armed slot becomes the best. -/
theorem nextok_first {s : logic.time.Slots NS} {i : Usize} (h : NextOk s i.val none)
    (harm : (s.slots.val[i.val]!).armed = true) : NextOk s (i.val + 1) (some i) := by
  simp only [NextOk] at h ⊢
  refine ⟨by omega, harm, fun j hj ha => ?_, fun j hj ha he => ?_⟩
  · by_cases hji : j = i.val
    · subst hji; exact Nat.le_refl _
    · have := h j (by omega); rw [this] at ha; cases ha
  · by_cases hji : j = i.val
    · omega
    · have := h j (by omega); rw [this] at ha; cases ha

/-- An armed slot strictly earlier than the best replaces it. -/
theorem nextok_less {s : logic.time.Slots NS} {i b : Usize} (h : NextOk s i.val (some b))
    (harm : (s.slots.val[i.val]!).armed = true)
    (hl : (s.slots.val[i.val]!).«at».val < (s.slots.val[b.val]!).«at».val) :
    NextOk s (i.val + 1) (some i) := by
  obtain ⟨hb1, hb2, hb3, hb4⟩ := h
  simp only [NextOk]
  refine ⟨by omega, harm, fun j hj ha => ?_, fun j hj ha he => ?_⟩
  · by_cases hji : j = i.val
    · subst hji; exact Nat.le_refl _
    · have := hb3 j (by omega) ha
      omega
  · by_cases hji : j = i.val
    · omega
    · have := hb3 j (by omega) ha
      omega

/-- An armed slot not earlier than the best leaves it. -/
theorem nextok_keep {s : logic.time.Slots NS} {i b : Usize} (h : NextOk s i.val (some b))
    (hle : (s.slots.val[b.val]!).«at».val ≤ (s.slots.val[i.val]!).«at».val) :
    NextOk s (i.val + 1) (some b) := by
  obtain ⟨hb1, hb2, hb3, hb4⟩ := h
  simp only [NextOk]
  refine ⟨by omega, hb2, fun j hj ha => ?_, fun j hj ha he => ?_⟩
  · by_cases hji : j = i.val
    · subst hji; exact hle
    · exact hb3 j (by omega) ha
  · by_cases hji : j = i.val
    · omega
    · exact hb4 j (by omega) ha he

/-- `Slots::next`: the minimum of the map, ties to the lowest slot. The
loop keeps the best slot's instant beside its index (2026-11-20). -/
theorem next_spec (s : logic.time.Slots NS) :
    logic.time.Slots.next s ⦃ r => NextOk s NS.val r ⦄ := by
  unfold logic.time.Slots.next logic.time.Slots.next_loop
  apply loop.spec_decr_nat (fun (x : Option Usize × Bool × U64 × Usize) => NS.val - x.2.2.2.val)
    (fun x => x.2.2.2.val ≤ NS.val ∧ NextOk s x.2.2.2.val x.1 ∧ (x.2.1 = true ↔ x.1.isSome = true) ∧
      ∀ b, x.1 = some b → x.2.2.1 = (s.slots.val[b.val]!).«at»)
  · intro x hx
    obtain ⟨best, hv, bat, i⟩ := x
    obtain ⟨h1, h2, h3, h4⟩ := hx
    try dsimp only at h1 h2 h3 h4 ⊢
    unfold logic.time.Slots.next_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : i.val < NS.val := by scalar_tac
      have hlen : i.val < s.slots.val.length := by have := s.slots.property; scalar_tac
      step as ⟨ sl, hsl ⟩
      have hsl' : s.slots.val[i.val]! = sl := by rw [hsl]; exact getElem!_pos s.slots.val i.val hlen
      split
      · rename_i harm
        have harm' : (s.slots.val[i.val]!).armed = true := by rw [hsl']; exact harm
        split
        · rename_i hhave
          obtain ⟨b, hb⟩ : ∃ b, best = some b := by
            have := h3.mp hhave
            cases best with
            | none => simp at this
            | some b => exact ⟨b, rfl⟩
          subst hb
          have hbat := h4 b rfl
          split
          · rename_i hless
            have hless' : (s.slots.val[i.val]!).«at».val < (s.slots.val[b.val]!).«at».val := by
              rw [hsl', ← hbat]; scalar_tac
            simp only [bind_ok]
            step as ⟨ i1, hi1 ⟩
            have e1 : i1.val = i.val + 1 := by scalar_tac
            refine ⟨by scalar_tac, ?_, ?_, ?_, by scalar_tac⟩
            · rw [e1]; exact nextok_less h2 harm' hless'
            · simp
            · intro b' hb'
              have : b' = i := (Option.some.inj hb').symm
              rw [this, hsl']
          · rename_i hnl
            have hnl' : (s.slots.val[b.val]!).«at».val ≤ (s.slots.val[i.val]!).«at».val := by
              rw [hsl', ← hbat]; scalar_tac
            simp only [bind_ok]
            step as ⟨ i1, hi1 ⟩
            have e1 : i1.val = i.val + 1 := by scalar_tac
            refine ⟨by scalar_tac, ?_, h3, h4, by scalar_tac⟩
            rw [e1]; exact nextok_keep h2 hnl'
        · rename_i hnh
          have hnone : best = none := by
            cases best with
            | none => rfl
            | some b => exact absurd (h3.mpr rfl) hnh
          subst hnone
          simp only [bind_ok]
          step as ⟨ i1, hi1 ⟩
          have e1 : i1.val = i.val + 1 := by scalar_tac
          refine ⟨by scalar_tac, ?_, ?_, ?_, by scalar_tac⟩
          · rw [e1]; exact nextok_first h2 harm'
          · simp
          · intro b' hb'
            have : b' = i := (Option.some.inj hb').symm
            rw [this, hsl']
      · rename_i harm
        have hna : (s.slots.val[i.val]!).armed = false := by
          rw [hsl']; simpa using harm
        simp only [bind_ok]
        step as ⟨ i1, hi1 ⟩
        have e1 : i1.val = i.val + 1 := by scalar_tac
        refine ⟨by scalar_tac, ?_, h3, h4, by scalar_tac⟩
        rw [e1]; exact nextok_skip h2 hna
    · rename_i hge
      have heq : i.val = NS.val := by scalar_tac
      refine (WP.spec_ok _).mpr ?_
      show NextOk s NS.val best
      rw [← heq]
      exact h2
  · exact ⟨by scalar_tac, by simp [NextOk], by simp, fun b hb => by cases hb⟩

/-! ## An Endpoint buffer as a FIFO of its capacity -/

def Fwf (f : logic.endpoint.Fifo) (cap : Nat) : Prop :=
  1 ≤ cap ∧ cap ≤ 255 ∧ f.head.val < cap ∧ f.count.val ≤ cap

/-- The occupied slots, oldest first. -/
def fview (f : logic.endpoint.Fifo) (cap : Nat) : List Nat :=
  (List.range f.count.val).map (fun i => (f.head.val + i) % cap)

/-- `Fifo::is_wf` decides the well-formedness the operations assume. -/
theorem is_wf_spec (f : logic.endpoint.Fifo) (cap : U8) :
    logic.endpoint.Fifo.is_wf f cap ⦃ b => b = true ↔ Fwf f cap.val ⦄ := by
  unfold logic.endpoint.Fifo.is_wf Fwf
  split
  · split
    · refine (WP.spec_ok _).mpr ?_
      simp only [decide_eq_true_eq]
      scalar_tac
    · refine (WP.spec_ok _).mpr ?_
      simp only [Bool.false_eq_true, false_iff]
      scalar_tac
  · refine (WP.spec_ok _).mpr ?_
    simp only [Bool.false_eq_true, false_iff]
    scalar_tac

/-- `Fifo::push`: the next free slot is appended to the view, or the full
channel is left as it is. Nothing is lost and the count stays within the
capacity. -/
theorem push_spec (f : logic.endpoint.Fifo) (cap : U8) (hw : Fwf f cap.val) :
    logic.endpoint.Fifo.push f cap ⦃ r =>
      Fwf r.2 cap.val ∧
      match r.1 with
      | some sl => f.count.val < cap.val ∧ sl.val = (f.head.val + f.count.val) % cap.val ∧
          fview r.2 cap.val = fview f cap.val ++ [sl.val] ∧ r.2.head = f.head ∧
          r.2.count.val = f.count.val + 1
      | none => f.count.val = cap.val ∧ r.2 = f ⦄ := by
  obtain ⟨h1, h2, h3, h4⟩ := hw
  unfold logic.endpoint.Fifo.push
  split
  · rename_i hge
    refine (WP.spec_ok _).mpr ⟨⟨h1, h2, h3, h4⟩, by scalar_tac, rfl⟩
  · rename_i hlt
    have hlt' : f.count.val < cap.val := by scalar_tac
    have c1 : (UScalar.cast .U32 f.head).val = f.head.val := by scalar_tac
    have c2 : (UScalar.cast .U32 f.count).val = f.count.val := by scalar_tac
    have c3 : (UScalar.cast .U32 cap).val = cap.val := by scalar_tac
    simp only [lift, bind_ok]
    step as ⟨ sum, hsum ⟩
    step as ⟨ s32, hs32 ⟩
    step as ⟨ i3, hi3 ⟩
    have hs : s32.val = (f.head.val + f.count.val) % cap.val := by
      rw [hs32, hsum, c1, c2, c3]
    have hsl : s32.val < cap.val := by rw [hs]; exact Nat.mod_lt _ (by omega)
    have c4 : (UScalar.cast .U8 s32).val = s32.val := by scalar_tac
    refine ⟨⟨h1, h2, h3, by scalar_tac⟩, hlt', by rw [c4, hs], ?_, by scalar_tac⟩
    simp only [fview]
    rw [show i3.val = f.count.val + 1 by scalar_tac, List.range_succ, List.map_append, c4, hs]
    rfl

/-- `Fifo::pop`: the oldest slot is taken from the front of the view, or
the empty channel is left as it is. Nothing is duplicated or reordered. -/
theorem pop_spec (f : logic.endpoint.Fifo) (cap : U8) (hw : Fwf f cap.val) :
    logic.endpoint.Fifo.pop f cap ⦃ r =>
      Fwf r.2 cap.val ∧
      match r.1 with
      | some sl => 0 < f.count.val ∧ sl = f.head ∧
          fview r.2 cap.val = (fview f cap.val).tail ∧ r.2.count.val = f.count.val - 1
      | none => f.count.val = 0 ∧ r.2 = f ⦄ := by
  obtain ⟨h1, h2, h3, h4⟩ := hw
  unfold logic.endpoint.Fifo.pop
  split
  · rename_i h0
    refine (WP.spec_ok _).mpr ⟨⟨h1, h2, h3, h4⟩, by scalar_tac, rfl⟩
  · rename_i hne
    have hpos : 0 < f.count.val := by scalar_tac
    have c1 : (UScalar.cast .U32 f.head).val = f.head.val := by scalar_tac
    have c3 : (UScalar.cast .U32 cap).val = cap.val := by scalar_tac
    simp only [lift, bind_ok]
    step as ⟨ i1, hi1 ⟩
    step as ⟨ next, hnext ⟩
    step as ⟨ i4, hi4 ⟩
    have hn : next.val = (f.head.val + 1) % cap.val := by
      rw [hnext, hi1, c1, c3]
    have hnl : next.val < cap.val := by rw [hn]; exact Nat.mod_lt _ (by omega)
    have c4 : (UScalar.cast .U8 next).val = next.val := by scalar_tac
    refine ⟨⟨h1, h2, by rw [c4]; exact hnl, by scalar_tac⟩, hpos, ?_, by scalar_tac⟩
    simp only [fview]
    rw [show i4.val = f.count.val - 1 by scalar_tac, c4, hn]
    obtain ⟨m, hm⟩ : ∃ m, f.count.val = m + 1 := ⟨f.count.val - 1, by omega⟩
    rw [hm, List.range_succ_eq_map]
    simp only [List.map_cons, List.tail_cons, List.map_map, Nat.add_sub_cancel]
    apply List.map_congr_left
    intro i _
    simp only [Function.comp, Nat.succ_eq_add_one]
    rw [Nat.mod_add_mod]
    congr 1
    omega

/-- `Fifo::clear`: the empty channel. -/
theorem clear_spec (f : logic.endpoint.Fifo) (cap : Nat) :
    logic.endpoint.Fifo.clear f ⦃ r => r.head = 0#u8 ∧ r.count = 0#u8 ∧ fview r cap = [] ⦄ := by
  unfold logic.endpoint.Fifo.clear
  refine (WP.spec_ok _).mpr ⟨rfl, rfl, rfl⟩

/-! ## The event log as a bounded sequence, oldest first -/

deriving instance Inhabited for rsk_kernel.rsk_logic.logic.log.Event

def Lwf (l : logic.log.Log NE) : Prop :=
  0 < NE.val ∧ NE.val < 1073741824 ∧ l.head.val < NE.val ∧ l.len.val ≤ NE.val

/-- Entry `i` of the log, the oldest being entry 0. -/
def lget (l : logic.log.Log NE) (i : Nat) : logic.log.Event :=
  l.entries.val[if NE.val ≤ l.head.val + i then l.head.val + i - NE.val else l.head.val + i]!

theorem sat1_val (x : U32) :
    (core.num.U32.saturating_add x 1#u32).val = min 4294967295 (x.val + 1) := by
  unfold core.num.U32.saturating_add UScalar.saturating_add
  simp only [UScalar.val]
  have hm : U32.max = 4294967295 := by scalar_tac
  have hx : x.val ≤ 4294967295 := by scalar_tac
  simp only [UScalar.val] at hx
  simp
  rw [hm]
  omega

/-- `Log::push`: the event is appended; a full log first drops its oldest
entry and counts the loss, saturating. -/
theorem log_push_spec (l : logic.log.Log NE) (e : logic.log.Event) (hw : Lwf l) :
    logic.log.Log.push l e ⦃ l' =>
      Lwf l' ∧
      (l.len.val < NE.val → l'.len.val = l.len.val + 1 ∧
        (∀ i, i < l.len.val → lget l' i = lget l i) ∧ lget l' l.len.val = e ∧
        l'.overflow = l.overflow) ∧
      (l.len.val = NE.val → l'.len.val = NE.val ∧
        (∀ i, i + 1 < NE.val → lget l' i = lget l (i + 1)) ∧ lget l' (NE.val - 1) = e ∧
        l'.overflow.val = min 4294967295 (l.overflow.val + 1)) ⦄ := by
  obtain ⟨h1, h2, h3, h4⟩ := hw
  unfold logic.log.Log.push
  simp only [lift, bind_ok]
  split
  · rename_i hlt
    have hlt' : l.len.val < NE.val := by scalar_tac
    step as ⟨ i1, hi1 ⟩
    split
    · rename_i hge
      have hge' : NE.val ≤ i1.val := by scalar_tac
      step as ⟨ pos, hpos ⟩
      step as ⟨ a, ha ⟩
      step as ⟨ i2, hi2 ⟩
      subst ha
      have hp : pos.val < NE.val := by omega
      refine ⟨⟨h1, h2, h3, by scalar_tac⟩, fun _ => ⟨by scalar_tac, fun i hi => ?_, ?_⟩,
        fun h => by omega⟩
      · simp only [lget]
        rw [get_set _ _ _ _ hp, if_neg (by split <;> omega)]
      · simp only [lget]
        rw [get_set _ _ _ _ hp, if_pos (by split <;> omega)]
    · rename_i hnge
      have hnge' : i1.val < NE.val := by scalar_tac
      simp only [bind_ok]
      step as ⟨ a, ha ⟩
      step as ⟨ i2, hi2 ⟩
      subst ha
      refine ⟨⟨h1, h2, h3, by scalar_tac⟩, fun _ => ⟨by scalar_tac, fun i hi => ?_, ?_⟩,
        fun h => by omega⟩
      · simp only [lget]
        rw [get_set _ _ _ _ hnge', if_neg (by split <;> omega)]
      · simp only [lget]
        rw [get_set _ _ _ _ hnge', if_pos (by split <;> omega)]
  · rename_i hnlt
    have hfull : l.len.val = NE.val := by scalar_tac
    have h0 : (0#usize).val = 0 := rfl
    step as ⟨ a, ha ⟩
    step as ⟨ i1, hi1 ⟩
    subst ha
    split
    · rename_i hge
      have hge' : l.head.val + 1 = NE.val := by scalar_tac
      simp only [bind_ok]
      refine (WP.spec_ok _).mpr ?_
      refine ⟨⟨h1, h2, by rw [h0]; exact h1, h4⟩, fun h => by omega,
        fun _ => ⟨hfull, fun i hi => ?_, ?_, sat1_val _⟩⟩
      · simp only [lget, h0]
        rw [get_set _ _ _ _ h3, if_neg (by split <;> omega)]
        congr 1
        split <;> split <;> omega
      · simp only [lget, h0]
        rw [get_set _ _ _ _ h3, if_pos (by split <;> omega)]
    · rename_i hnge
      have hnge' : i1.val < NE.val := by scalar_tac
      simp only [bind_ok]
      refine (WP.spec_ok _).mpr ?_
      refine ⟨⟨h1, h2, hnge', h4⟩, fun h => by omega,
        fun _ => ⟨hfull, fun i hi => ?_, ?_, sat1_val _⟩⟩
      · simp only [lget]
        rw [get_set _ _ _ _ h3, if_neg (by split <;> omega)]
        congr 1
        split <;> split <;> omega
      · simp only [lget]
        rw [get_set _ _ _ _ h3, if_pos (by split <;> omega)]

end P18
