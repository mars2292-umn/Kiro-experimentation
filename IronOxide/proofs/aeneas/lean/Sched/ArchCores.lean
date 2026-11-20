import Sched.Record
import Sched.ArchInv
import Sched.LoopInv
open Aeneas Aeneas.Std Result

/-! # The entry cores keep the level records and the Kernel's stack in agreement (2026-11-19)

`Sched/ArchInv.lean` states the invariant of Kernel_Arch's entry cores over
the level records themselves. This file proves the extracted cores keep it,
as Verus proves of `entry::arch_inv`: for any Kernel behind the
`KernelOps` trait whose operations satisfy the trait's contract (`KSpec`:
every operation but a dispatch leaves the preemption stack a prefix of
what it was), and any hardware and tables.

Every statement has the form "if the core returns, then ...". -/

namespace P18
open rsk_kernel

/-- The two cases of a conditional that returns. -/
theorem ite_cases {α} {c : Prop} [Decidable c] {a b : Result α} {r : α}
    (h : (if c then a else b) = ok r) : (c ∧ a = ok r) ∨ (¬ c ∧ b = ok r) := by
  by_cases hc : c
  · rw [if_pos hc] at h; exact Or.inl ⟨hc, h⟩
  · rw [if_neg hc] at h; exact Or.inr ⟨hc, h⟩

/-! ## The views -/

/-- The level records as functions of the level (`Arch.LV`). -/
def lvOf (levels : Array arch.cm4.plan.LevelRecord 8#usize) : Arch.LV where
  act l := decide (l < 8) && (levels.val[l]!).active
  task l := (levels.val[l]!).task.val
  prevTask l := (levels.val[l]!).previous_task.val
  part l := (levels.val[l]!).partition.val
  prevPart l := (levels.val[l]!).previous_partition.val
  exc l := (levels.val[l]!).exception_number.val

/-- The level records, the Partition taken as executing and the owed
unwind of `s'` are those of `s` (Verus: `arch_frame`). -/
def Frame (s' s : arch.cm4.plan.ArchState) : Prop :=
  s'.levels = s.levels ∧ s'.current_partition = s.current_partition ∧
  s'.pending_unwind = s.pending_unwind

theorem Frame.refl (s : arch.cm4.plan.ArchState) : Frame s s := ⟨rfl, rfl, rfl⟩

theorem Frame.trans {a b c : arch.cm4.plan.ArchState} (h1 : Frame b a) (h2 : Frame c b) :
    Frame c a := ⟨h2.1.trans h1.1, h2.2.1.trans h1.2.1, h2.2.2.trans h1.2.2⟩

/-- A call of a function already shown to leave the three fields alone. -/
syntax "frame_call" : tactic
macro_rules | `(tactic| frame_call) => `(tactic| fail "no call")

/-- Registers a frame lemma `l : f .. s = ok (.., s') → Frame s' s`. -/
macro "frame_reg " l:ident : command =>
  `(macro_rules | `(tactic| frame_call) => `(tactic| first
      | refine Eq.trans (And.left ($l (by assumption))) ?_
      | refine Eq.trans (And.left (And.right ($l (by assumption)))) ?_
      | refine Eq.trans (And.right (And.right ($l (by assumption)))) ?_))

/-- One of the three fields, back through the function's steps. -/
syntax "frame_proj" : tactic
macro_rules
  | `(tactic| frame_proj) => `(tactic| repeat (first
      | rfl
      | exact (And.left (by assumption : Frame _ _))
      | exact (And.left (And.right (by assumption : Frame _ _)))
      | exact (And.right (And.right (by assumption : Frame _ _)))
      | frame_call
      | dsimp only))

/-- Closes `Frame s' s` after `peel`. -/
syntax "frame_close" : tactic
macro_rules
  | `(tactic| frame_close) => `(tactic| (refine ⟨?_, ?_, ?_⟩ <;> (frame_proj; done)))

/-! ## The helpers that take the state -/

theorem charge_point_frame {A : Type} {ai : arch.cm4.entry.KernelArch A} {hw : A}
    {s s' : arch.cm4.plan.ArchState} {x : U32}
    (h : arch.cm4.entry.charge_point ai hw s = ok (x, s')) : Frame s' s := by
  unfold arch.cm4.entry.charge_point at h
  peel h
  frame_close

frame_reg charge_point_frame

theorem clear_channels_loop_frame {i base c : Usize} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.clear_channels_loop i s base c = ok s') : Frame s' s := by
  unfold arch.cm4.entry.clear_channels_loop at h
  refine loop_inv _ (fun x => Frame x.1 s) (fun r => Frame r s) ?_ ?_ _ _ (Frame.refl s) h
  · rintro ⟨st, c⟩ ⟨st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.clear_channels_loop.body at hb
    peel hb
    all_goals frame_close
  · rintro ⟨st, c⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.clear_channels_loop.body at hb
    peel hb
    all_goals frame_close

frame_reg clear_channels_loop_frame

theorem clear_channels_frame {e : Usize} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.clear_channels s e = ok s') : Frame s' s := by
  unfold arch.cm4.entry.clear_channels at h
  peel h
  all_goals frame_close

frame_reg clear_channels_frame

theorem endpoint_reinit_one_loop0_frame {i base c : Usize} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_reinit_one_loop0 i s base c = ok s') : Frame s' s := by
  unfold arch.cm4.entry.endpoint_reinit_one_loop0 at h
  refine loop_inv _ (fun x => Frame x.1 s) (fun r => Frame r s) ?_ ?_ _ _ (Frame.refl s) h
  · rintro ⟨st, c⟩ ⟨st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.endpoint_reinit_one_loop0.body at hb
    peel hb
    all_goals frame_close
  · rintro ⟨st, c⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.endpoint_reinit_one_loop0.body at hb
    peel hb
    all_goals frame_close

frame_reg endpoint_reinit_one_loop0_frame

theorem endpoint_reinit_one_loop1_frame {rbase r : Usize} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_reinit_one_loop1  s rbase r = ok s') : Frame s' s := by
  unfold arch.cm4.entry.endpoint_reinit_one_loop1 at h
  refine loop_inv _ (fun x => Frame x.1 s) (fun r => Frame r s) ?_ ?_ _ _ (Frame.refl s) h
  · rintro ⟨st, c⟩ ⟨st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.endpoint_reinit_one_loop1.body at hb
    peel hb
    all_goals frame_close
  · rintro ⟨st, c⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.endpoint_reinit_one_loop1.body at hb
    peel hb
    all_goals frame_close

frame_reg endpoint_reinit_one_loop1_frame

theorem endpoint_reinit_one_frame {e : Usize} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_reinit_one s e = ok s') : Frame s' s := by
  unfold arch.cm4.entry.endpoint_reinit_one at h
  peel h
  all_goals frame_close

frame_reg endpoint_reinit_one_frame

theorem endpoint_reinit_loop0_loop0_frame {p : U8} {e r : Usize} {cfg : arch.cm4.plan.EndpointCfg} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_reinit_loop0_loop0  s p e cfg r = ok s') : Frame s' s := by
  unfold arch.cm4.entry.endpoint_reinit_loop0_loop0 at h
  refine loop_inv _ (fun x => Frame x.1 s) (fun r => Frame r s) ?_ ?_ _ _ (Frame.refl s) h
  · rintro ⟨st, c⟩ ⟨st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.endpoint_reinit_loop0_loop0.body at hb
    peel hb
    all_goals frame_close
  · rintro ⟨st, c⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.endpoint_reinit_loop0_loop0.body at hb
    peel hb
    all_goals frame_close

frame_reg endpoint_reinit_loop0_loop0_frame

theorem endpoint_reinit_loop0_frame {T : Type} {ti : arch.cm4.entry.Tables T} {tables : T} {p : U8} {n e : Usize} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_reinit_loop0 ti tables s p n e = ok s') : Frame s' s := by
  unfold arch.cm4.entry.endpoint_reinit_loop0 at h
  refine loop_inv _ (fun x => Frame x.1 s) (fun r => Frame r s) ?_ ?_ _ _ (Frame.refl s) h
  · rintro ⟨st, c⟩ ⟨st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.endpoint_reinit_loop0.body at hb
    peel hb
    all_goals frame_close
  · rintro ⟨st, c⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.endpoint_reinit_loop0.body at hb
    peel hb
    all_goals frame_close

frame_reg endpoint_reinit_loop0_frame

theorem endpoint_reinit_frame {T : Type} {ti : arch.cm4.entry.Tables T} {tables : T} {p : U8} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_reinit ti tables s p = ok s') : Frame s' s := by
  unfold arch.cm4.entry.endpoint_reinit at h
  peel h
  all_goals frame_close

frame_reg endpoint_reinit_frame

theorem channel_push_frame {ch : Usize} {cap : U8} {o : Option U8} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.channel_push s ch cap = ok (o, s')) : Frame s' s := by
  unfold arch.cm4.entry.channel_push at h
  peel h
  all_goals frame_close

frame_reg channel_push_frame

theorem channel_pop_frame {ch : Usize} {cap : U8} {o : Option U8} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.channel_pop s ch cap = ok (o, s')) : Frame s' s := by
  unfold arch.cm4.entry.channel_pop at h
  peel h
  all_goals frame_close

frame_reg channel_pop_frame

/-! ## The Kernel behind its trait -/

variable {K A T : Type}

/-- From `k` to `k'` the Kernel's invariant is kept and its preemption
stack only loses Jobs from the top (`stack_prefix`). -/
def KR (kv : K → Arch.KV) (okK : K → Prop) (k' k : K) : Prop :=
  okK k → okK k' ∧ Arch.Prefix (kv k') (kv k)

theorem KR.refl (kv : K → Arch.KV) (okK : K → Prop) (k : K) : KR kv okK k k :=
  fun h => ⟨h, Nat.le_refl _, fun _ _ => rfl, rfl, rfl⟩

theorem KR.trans {kv : K → Arch.KV} {okK : K → Prop} {a b c : K} (h1 : KR kv okK b a)
    (h2 : KR kv okK c b) : KR kv okK c a := by
  intro ha
  obtain ⟨hb, p1, p2, p3, p4⟩ := h1 ha
  obtain ⟨hc, q1, q2, q3, q4⟩ := h2 hb
  exact ⟨hc, by omega, fun i hi => (q2 i hi).trans (p2 i (by omega)), q3.trans p3, q4.trans p4⟩

/-- The contract of the `KernelOps` trait, as far as the level records
depend on it (Verus: the `ensures` of the trait's methods over
`spec_depth`, `spec_stack`, `spec_prio`, `spec_part`). `kv` reads a Kernel
as its preemption stack and configuration, `okK` is its invariant. -/
class KSpec (ki : arch.bound.KernelOps K) (kv : K → Arch.KV) (okK : K → Prop) : Prop where
  kok : ∀ {k}, okK k → Arch.KOk (kv k)
  depth : ∀ {k d}, ki.depth k = ok d → d.val = (kv k).depth
  top : ∀ {k o}, okK k → ki.top k = ok o →
    (o = none ↔ (kv k).depth = 0) ∧ ∀ t, o = some t → t.val = (kv k).stack ((kv k).depth - 1)
  job_info : ∀ {k t ji}, okK k → ki.job_info k t = ok ji →
    ji.priority.val = (kv k).prio t.val ∧ ji.partition.val = (kv k).part t.val
  dispatch : ∀ {k t pc k'}, okK k → ki.dispatch k t pc = ok k' →
    okK k' ∧ (kv k').prio = (kv k).prio ∧ (kv k').part = (kv k).part ∧
    (∀ i, i < (kv k).depth → (kv k').stack i = (kv k).stack i) ∧
    ((kv k').depth = (kv k).depth ∨
      ((kv k').depth = (kv k).depth + 1 ∧ (kv k').stack (kv k).depth = t.val))
  start : ∀ {k k'}, ki.start k = ok k' → KR kv okK k' k
  job_return : ∀ {k e k'}, ki.job_return k e = ok k' → KR kv okK k' k
  job_return_depth : ∀ {k e k'}, okK k → ki.job_return k e = ok k' → 0 < (kv k).depth →
    (kv k').depth = (kv k).depth - 1
  svc_lock : ∀ {k r b k'}, ki.svc_lock k r = ok (b, k') → KR kv okK k' k
  svc_unlock : ∀ {k r b k'}, ki.svc_unlock k r = ok (b, k') → KR kv okK k' k
  settle : ∀ {k x k'}, ki.settle k = ok (x, k') → KR kv okK k' k
  settle_depth : ∀ {k x k'}, okK k → ki.settle k = ok (x, k') → (kv k').depth = (kv k).depth
  intake : ∀ {k t p now o k'}, ki.intake k t p now o = ok k' → KR kv okK k' k
  timer_event : ∀ {k now rc k'}, ki.timer_event k now rc = ok k' → KR kv okK k' k
  fault : ∀ {k kind b k'}, ki.fault k kind b = ok k' → KR kv okK k' k
  take_reinit : ∀ {k p b k'}, ki.take_reinit k p = ok (b, k') → KR kv okK k' k
  take_reinit_depth : ∀ {k p b k'}, okK k → ki.take_reinit k p = ok (b, k') →
    (kv k').depth = (kv k).depth
  take_stop_dma : ∀ {k p b k'}, ki.take_stop_dma k p = ok (b, k') → KR kv okK k' k
  take_stop_dma_depth : ∀ {k p b k'}, okK k → ki.take_stop_dma k p = ok (b, k') →
    (kv k').depth = (kv k).depth
  boot_failed : ∀ {k f k'}, ki.boot_failed k f = ok k' → KR kv okK k' k
  boot_info : ∀ {k a b k'}, ki.boot_info k a b = ok k' → KR kv okK k' k
  record_event : ∀ {k kind t k'}, ki.record_event k kind t = ok k' → KR kv okK k' k
  fault_of : ∀ {k kind t k'}, ki.fault_of k kind t = ok k' → KR kv okK k' k

/-- A call of a function already shown to keep `KR`. -/
syntax "kr_call" : tactic
macro_rules | `(tactic| kr_call) => `(tactic| fail "no call")

/-- Registers a lemma `l : f .. k .. = ok (.., k', ..) → KR kv okK k' k`. -/
macro "kr_reg " l:ident : command =>
  `(macro_rules | `(tactic| kr_call) => `(tactic| refine KR.trans ?_ ($l (by assumption))))

kr_reg KSpec.start
kr_reg KSpec.job_return
kr_reg KSpec.svc_lock
kr_reg KSpec.svc_unlock
kr_reg KSpec.settle
kr_reg KSpec.intake
kr_reg KSpec.timer_event
kr_reg KSpec.fault
kr_reg KSpec.take_reinit
kr_reg KSpec.take_stop_dma
kr_reg KSpec.boot_failed
kr_reg KSpec.boot_info
kr_reg KSpec.record_event
kr_reg KSpec.fault_of

/-- Closes `KR kv okK k' k` after `peel`. -/
syntax "kr_close" : tactic
macro_rules
  | `(tactic| kr_close) => `(tactic| repeat (first
      | exact KR.refl _ _ _
      | assumption
      | kr_call
      | dsimp only))

/-! ## `apply_core`: the last steps of every entry -/

variable {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

theorem apply_core_loop1_both [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {np p : Usize}
    (h : arch.cm4.entry.apply_core_loop1 ki ai ti k hw tables s np p = ok (k', hw', s')) :
    Frame s' s ∧ KR kv okK k' k := by
  unfold arch.cm4.entry.apply_core_loop1 at h
  refine loop_inv _ (fun x => Frame x.2.2.1 s ∧ KR kv okK x.1 k)
    (fun r => Frame r.2.2 s ∧ KR kv okK r.1 k) ?_ ?_ _ (k', hw', s')
    ⟨Frame.refl s, KR.refl _ _ _⟩ h
  · rintro ⟨k1, hw1, st, c⟩ ⟨k2, hw2, st', c'⟩ ⟨hinv, hkr⟩ hb
    dsimp only at hinv hkr hb ⊢
    unfold arch.cm4.entry.apply_core_loop1.body at hb
    peel hb
    all_goals exact ⟨by frame_close, by kr_close⟩
  · rintro ⟨k1, hw1, st, c⟩ ⟨k2, hw2, st'⟩ ⟨hinv, hkr⟩ hb
    dsimp only at hinv hkr hb ⊢
    unfold arch.cm4.entry.apply_core_loop1.body at hb
    peel hb
    all_goals exact ⟨by frame_close, by kr_close⟩

theorem apply_core_loop1_frame {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {np p : Usize}
    (h : arch.cm4.entry.apply_core_loop1 ki ai ti k hw tables s np p = ok (k', hw', s')) :
    Frame s' s := by
  unfold arch.cm4.entry.apply_core_loop1 at h
  refine loop_inv _ (fun x => Frame x.2.2.1 s) (fun r => Frame r.2.2 s) ?_ ?_ _ (k', hw', s')
    (Frame.refl s) h
  · rintro ⟨k1, hw1, st, c⟩ ⟨k2, hw2, st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.apply_core_loop1.body at hb
    peel hb
    all_goals frame_close
  · rintro ⟨k1, hw1, st, c⟩ ⟨k2, hw2, st'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.apply_core_loop1.body at hb
    peel hb
    all_goals frame_close

frame_reg apply_core_loop1_frame

theorem apply_core_loop1_kr [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {np p : Usize}
    (h : arch.cm4.entry.apply_core_loop1 ki ai ti k hw tables s np p = ok (k', hw', s')) :
    KR kv okK k' k := (apply_core_loop1_both h).2

kr_reg apply_core_loop1_kr

theorem apply_core_frame {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {safe : Bool}
    (h : arch.cm4.entry.apply_core ki ai ti k hw tables s = ok (safe, k', hw', s')) :
    Frame s' s := by
  unfold arch.cm4.entry.apply_core at h
  peel h
  all_goals frame_close

frame_reg apply_core_frame

theorem apply_core_kr [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {safe : Bool}
    (h : arch.cm4.entry.apply_core ki ai ti k hw tables s = ok (safe, k', hw', s')) :
    KR kv okK k' k := by
  unfold arch.cm4.entry.apply_core at h
  peel h
  all_goals kr_close

kr_reg apply_core_kr

end P18
