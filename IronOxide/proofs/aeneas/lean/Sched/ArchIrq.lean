import Sched.ArchVector
open Aeneas Aeneas.Std Result

/-! # The interrupt entry, the start and the boot keep the invariant (2026-11-20)

- `irq_entry_core_keeps`: the common entry of every external interrupt. It
  re-enables deferred vectors, runs the core its vector is bound to, and,
  when that core planned an unwind while the interrupt had preempted a
  handler, leaves the unwind owed (`pending_unwind`) for the dispatch stub
  to take at its entry instruction (`pend_take`).
- `start_core_keeps`: the first Kernel entry.
- `boot_core_establishes`: a boot that passes its checks leaves the
  invariant in its empty form: no record active, no Job on the stack. -/

namespace P18
open rsk_kernel

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

theorem irq_loop_frame {hw hw' : A} {i : Usize} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.irq_entry_core_loop ai hw s i = ok (hw', s')) : Frame s' s := by
  unfold arch.cm4.entry.irq_entry_core_loop at h
  refine loop_inv _ (fun x => Frame x.2.1 s) (fun r => Frame r.2 s) ?_ ?_ _ (hw', s')
    (Frame.refl s) h
  · rintro ⟨c, st, d⟩ ⟨c', st', d'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.irq_entry_core_loop.body at hb
    peel hb
    all_goals frame_close
  · rintro ⟨c, st, d⟩ ⟨c', st'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.irq_entry_core_loop.body at hb
    peel hb
    all_goals frame_close

theorem irq_n_le (ipsr : U32) : (UScalar.cast .U16 (ipsr &&& 511#u32)).val ≤ 511 := by
  have h1 : (ipsr &&& 511#u32).val ≤ 511 := by
    rw [UScalar.val_and]
    exact Nat.and_le_right
  rw [UScalar.cast_val_eq]
  exact Nat.le_trans (Nat.mod_le _ _) h1

/-- An unwind planned while the interrupt had preempted a handler is left
owed. -/
theorem defer_set {k : K} {s s' : arch.cm4.plan.ArchState} {hop : U32}
    (hi : okK k ∧ AI kv k s hop.val) (hne : (hop != 0#u32) = true)
    (e1 : s'.levels = s.levels) (e2 : s'.current_partition = s.current_partition)
    (e3 : s'.pending_unwind = hop) : okK k ∧ AI kv k s' (0#u32).val := by
  refine ⟨hi.1, ?_⟩
  have hv := bne32_val hne
  have hi2 := hi.2
  unfold AI at hi2 ⊢
  obtain ⟨hc, h1, _, _⟩ := hi2
  rw [e1, e2, e3]
  exact ⟨hc, fun h => absurd rfl h, fun _ => ⟨fun _ => hv, fun _ => (h1 hv).1⟩, fun _ => h1 hv⟩

/-- An unwind was owed already: it names the same topmost record. -/
theorem defer_kept {k : K} {s : arch.cm4.plan.ArchState} {hop : U32}
    (hi : okK k ∧ AI kv k s hop.val) (hpn : ¬ s.pending_unwind = 0#u32) :
    okK k ∧ AI kv k s (0#u32).val := by
  refine ⟨hi.1, ?_⟩
  have hv : s.pending_unwind.val ≠ 0 := fun e => hpn (by scalar_tac)
  have hi2 := hi.2
  unfold AI at hi2 ⊢
  obtain ⟨hc, _, _, h3⟩ := hi2
  exact ⟨hc, fun h => absurd rfl h, fun _ => ⟨fun _ => hv, fun _ => (h3 hv).1⟩, h3⟩

theorem zero_bne (h : (0#u32 != 0#u32) = true) : False := by simp at h

/-- Closes an exit of `irq_entry_core`. -/
syntax "irq_exit " ident ident ident : tactic
macro_rules
  | `(tactic| irq_exit $hok:ident $hai:ident $hn:ident) => `(tactic| (
      first
        | have hai' := AI.frame (irq_loop_frame (by assumption)) $hai
        | have hai' := $hai
      first
        | have hi := dispatch_vector_core_keeps $hok hai' $hn (by assumption)
        | have hi := intake_core_keeps $hok hai' (by assumption)
        | have hi := timer_core_keeps $hok hai' (by assumption)
        | have hi := unbound_core_keeps $hok hai' (by assumption)
        | have hi := And.intro $hok hai'
      first
        | exact (zero_bne (by assumption)).elim
        | exact hi
        | exact defer_set hi (by assumption) rfl rfl rfl
        | exact defer_kept hi (by assumption)))

set_option maxHeartbeats 32000000 in
/-- **`irq_entry_core` keeps the invariant.** -/
theorem irq_entry_core_keeps [KSpecD ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {ipsr psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} {safe : Bool}
    (hok : okK k) (hai : AI kv k s 0)
    (h : arch.cm4.entry.irq_entry_core ki ai ti k hw tables s ipsr psp control exc_return
      = ok ((plan, safe), k', hw', s')) :
    okK k' ∧ AI kv k' s' plan.hop.val := by
  have hn := irq_n_le ipsr
  unfold arch.cm4.entry.irq_entry_core at h
  simp only [arch.cm4.entry.WINDOW_DEFER, ↓reduceIte] at h
  peel h
  all_goals (irq_exit hok hai hn; done)

/-- **`start_core` keeps the invariant**: the start ends no Job. -/
theorem start_core_keeps [KSpecD ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {safe : Bool} (hok : okK k) (hai : AI kv k s 0)
    (h : arch.cm4.entry.start_core ki ai ti k hw tables s = ok (safe, k', hw', s')) :
    okK k' ∧ AI kv k' s' 0 := by
  unfold arch.cm4.entry.start_core at h
  peel h
  all_goals
    refine kentry0' (ki := ki) ?_ ?_ hok hai
    · ke_close; done
    · frame_close

/-! ## The boot -/

/-- What `validate` and `init` give (Verus: `validate` ensures `valid`,
`init` requires it and ensures the invariant with an empty stack). -/
class KBoot (ki : arch.bound.KernelOps K) (kv : K → Arch.KV) (okK : K → Prop) : Prop where
  init : ∀ {k k'}, ki.validate k = ok true → ki.init k = ok k' → okK k' ∧ (kv k').depth = 0

/-- `boot_core`'s first loop clears every record. -/
theorem boot_loop0_clears {s s' : arch.cm4.plan.ArchState} {l : Usize}
    (hl : l.val ≤ 8) (hpre : ∀ j, j < l.val → act s.levels j = false)
    (h : arch.cm4.entry.boot_core_loop0 s l = ok s') :
    ∀ j, j < 8 → act s'.levels j = false := by
  unfold arch.cm4.entry.boot_core_loop0 at h
  refine loop_inv _
    (fun x => x.2.val ≤ 8 ∧ ∀ j, j < x.2.val → act x.1.levels j = false)
    (fun r => ∀ j, j < 8 → act r.levels j = false) ?_ ?_ (s, l) s' ⟨hl, hpre⟩ h
  · rintro ⟨st, c⟩ ⟨st', c'⟩ ⟨h8, hinv⟩ hb
    dsimp only at h8 hinv hb ⊢
    unfold arch.cm4.entry.boot_core_loop0.body at hb
    peel hb
    rename_i hlt rc hrc a ha hc1
    obtain ⟨hlen, rfl⟩ := update_ok ha
    have hc := usize_add_ok hc1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at hc
    refine ⟨by scalar_tac, fun j hj => ?_⟩
    simp only [act]
    rw [get_set _ _ _ _ hlen]
    by_cases e : j = c.val
    · rw [if_pos e]
    · rw [if_neg e]
      exact hinv j (by omega)
  · rintro ⟨st, c⟩ y ⟨h8, hinv⟩ hb
    dsimp only at h8 hinv hb ⊢
    unfold arch.cm4.entry.boot_core_loop0.body at hb
    peel hb
    rename_i hge
    intro j hj
    exact hinv j (by scalar_tac)

/-- No record active, no Job on the stack, nothing owed. -/
theorem ai_empty {k : K} {s : arch.cm4.plan.ArchState} (hd : (kv k).depth = 0)
    (hn : ∀ j, j < 8 → act s.levels j = false) (e2 : s.current_partition = 255#u8)
    (e3 : s.pending_unwind = 0#u32) : AI kv k s 0 := by
  have hna : ∀ l, (lvOf s.levels).act l = false := fun l => lv_act_false (hn l)
  have hnex : ¬ Arch.Extra (kv k) (lvOf s.levels) := by
    rintro ⟨l, ha, _⟩
    rw [hna l] at ha; cases ha
  unfold AI
  rw [e2, e3]
  refine ⟨⟨fun l ha => ?_, fun i hi => ?_, fun l ha _ => ?_, fun _ => rfl, fun l ht => ?_⟩,
    fun h => absurd rfl h, fun _ => ⟨fun h => absurd h hnex, fun h => absurd rfl h⟩,
    fun h => absurd rfl h⟩
  · rw [hna l] at ha; cases ha
  · omega
  · rw [hna l] at ha; cases ha
  · have := ht.1
    rw [hna l] at this; cases this

/-- The state a boot that passed its checks leaves. -/
theorem boot_ok [KSpec ki kv okK] [KBoot ki kv okK] {k k1 k2 : K} {b x y : Bool}
    {s s1 s' : arch.cm4.plan.ArchState}
    (hv : ki.validate k = ok b) (hb : b = true) (hinit : ki.init k = ok k1)
    (hbi : ki.boot_info k1 x y = ok k2)
    (hl0 : arch.cm4.entry.boot_core_loop0 s 0#usize = ok s1)
    (e1 : s'.levels = s1.levels) (e2 : s'.current_partition = 255#u8)
    (e3 : s'.pending_unwind = 0#u32) : okK k2 ∧ AI kv k2 s' 0 := by
  subst hb
  obtain ⟨hok1, hd1⟩ := KBoot.init (kv := kv) (okK := okK) hv hinit
  obtain ⟨hok2, hp⟩ := KSpec.boot_info (kv := kv) (okK := okK) hbi hok1
  have hd2 : (kv k2).depth = 0 := by have := hp.1; omega
  refine ⟨hok2, ai_empty hd2 ?_ e2 e3⟩
  rw [e1]
  exact boot_loop0_clears (by scalar_tac) (fun j hj => absurd hj (by scalar_tac)) hl0

set_option maxHeartbeats 16000000 in
/-- **`boot_core` establishes the invariant**: a boot that passes its checks
(the result `true`) leaves no record active, no Job on the Kernel's stack
and nothing owed. -/
theorem boot_core_establishes [KSpec ki kv okK] [KBoot ki kv okK] {k k' : K} {hw hw' : A}
    {tables : T} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.boot_core ki ai ti k hw tables s = ok (true, k', hw', s')) :
    okK k' ∧ AI kv k' s' 0 := by
  unfold arch.cm4.entry.boot_core at h
  peel h
  all_goals
    exact boot_ok (ki := ki) (by assumption) (by assumption) (by assumption) (by assumption)
      (by assumption) (by rfl) (by rfl) (by rfl)

end P18
