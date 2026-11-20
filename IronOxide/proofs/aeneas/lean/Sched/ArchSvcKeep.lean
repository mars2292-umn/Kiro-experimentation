import Sched.ArchKeep
open Aeneas Aeneas.Std Result

/-! # The supervisor-call cores keep the invariant (2026-11-20)

`svc_core` and `svc_service_core` hand their stub an unwind plan only on
the paths that can end a Job. On the others (a lock, an unlock, the
instant, a granted DMA service, a receive that is not refused) the plan is
"none", which is right only if the Kernel's stack is as deep as it was.
`KE` is that stronger relation, `KSpecD` the four clauses of the trait's
contract it needs beyond `KSpec`. -/

namespace P18
open rsk_kernel

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

/-- From `k` to `k'` the invariant is kept and the preemption stack is the
same Jobs. -/
def KE (kv : K → Arch.KV) (okK : K → Prop) (k' k : K) : Prop :=
  okK k → okK k' ∧ Arch.Prefix (kv k') (kv k) ∧ (kv k').depth = (kv k).depth

theorem KE.refl (kv : K → Arch.KV) (okK : K → Prop) (k : K) : KE kv okK k k :=
  fun h => ⟨h, ⟨Nat.le_refl _, fun _ _ => rfl, rfl, rfl⟩, rfl⟩

theorem KE.trans {a b c : K} (h1 : KE kv okK b a) (h2 : KE kv okK c b) : KE kv okK c a := by
  intro ha
  obtain ⟨hb, ⟨p1, p2, p3, p4⟩, pd⟩ := h1 ha
  obtain ⟨hc, ⟨q1, q2, q3, q4⟩, qd⟩ := h2 hb
  exact ⟨hc, ⟨by omega, fun i hi => (q2 i hi).trans (p2 i (by omega)), q3.trans p3, q4.trans p4⟩,
    by omega⟩

theorem KE.mk {k k' : K} (hkr : KR kv okK k' k) (hd : okK k → (kv k').depth = (kv k).depth) :
    KE kv okK k' k := fun h => ⟨(hkr h).1, (hkr h).2, hd h⟩

theorem KE.kr {k k' : K} (h : KE kv okK k' k) : KR kv okK k' k := fun hk => ⟨(h hk).1, (h hk).2.1⟩

theorem KE.of_eq {k k' : K} (h : k' = k) : KE kv okK k' k := by subst h; exact KE.refl _ _ _

/-- The clauses of the trait's contract on the depth of the stack that
`KSpec` does not carry: a lock, an unlock, the start and a fault attributed
to the Kernel leave it as deep as it was. -/
class KSpecD (ki : arch.bound.KernelOps K) (kv : K → Arch.KV) (okK : K → Prop) : Prop
    extends KSpec ki kv okK where
  svc_lock_depth : ∀ {k r b k'}, okK k → ki.svc_lock k r = ok (b, k') →
    (kv k').depth = (kv k).depth
  svc_unlock_depth : ∀ {k r b k'}, okK k → ki.svc_unlock k r = ok (b, k') →
    (kv k').depth = (kv k).depth
  start_depth : ∀ {k k'}, okK k → ki.start k = ok k' → (kv k').depth = (kv k).depth
  fault_kernel_depth : ∀ {k kind k'}, okK k → ki.fault k kind true = ok k' →
    (kv k').depth = (kv k).depth

theorem settle_ke [KSpec ki kv okK] {k k' : K} {x : Option U8 × Bool}
    (h : ki.settle k = ok (x, k')) : KE kv okK k' k :=
  KE.mk (KSpec.settle h) (fun hok => KSpec.settle_depth hok h)

theorem take_reinit_ke [KSpec ki kv okK] {k k' : K} {p : U8} {b : Bool}
    (h : ki.take_reinit k p = ok (b, k')) : KE kv okK k' k :=
  KE.mk (KSpec.take_reinit h) (fun hok => KSpec.take_reinit_depth hok h)

theorem take_stop_dma_ke [KSpec ki kv okK] {k k' : K} {p : U8} {b : Bool}
    (h : ki.take_stop_dma k p = ok (b, k')) : KE kv okK k' k :=
  KE.mk (KSpec.take_stop_dma h) (fun hok => KSpec.take_stop_dma_depth hok h)

theorem svc_lock_ke [KSpecD ki kv okK] {k k' : K} {r : U8} {b : Bool}
    (h : ki.svc_lock k r = ok (b, k')) : KE kv okK k' k :=
  KE.mk (KSpec.svc_lock h) (fun hok => KSpecD.svc_lock_depth hok h)

theorem svc_unlock_ke [KSpecD ki kv okK] {k k' : K} {r : U8} {b : Bool}
    (h : ki.svc_unlock k r = ok (b, k')) : KE kv okK k' k :=
  KE.mk (KSpec.svc_unlock h) (fun hok => KSpecD.svc_unlock_depth hok h)

theorem start_ke [KSpecD ki kv okK] {k k' : K}
    (h : ki.start k = ok k') : KE kv okK k' k :=
  KE.mk (KSpec.start h) (fun hok => KSpecD.start_depth hok h)

theorem fault_kernel_ke [KSpecD ki kv okK] {k k' : K} {kind : U8}
    (h : ki.fault k kind true = ok k') : KE kv okK k' k :=
  KE.mk (KSpec.fault h) (fun hok => KSpecD.fault_kernel_depth hok h)

/-- A call of a function already shown to keep `KE`. -/
syntax "ke_call" : tactic
macro_rules | `(tactic| ke_call) => `(tactic| fail "no call")

macro "ke_reg " l:ident : command =>
  `(macro_rules | `(tactic| ke_call) => `(tactic| refine KE.trans ?_ ($l (by assumption))))

ke_reg settle_ke
ke_reg take_reinit_ke
ke_reg take_stop_dma_ke
ke_reg svc_lock_ke
ke_reg svc_unlock_ke
ke_reg start_ke
ke_reg fault_kernel_ke

/-- Closes `KE kv okK k' k` after `peel`. -/
syntax "ke_close" : tactic
macro_rules
  | `(tactic| ke_close) => `(tactic| repeat (first
      | exact KE.refl _ _ _
      | assumption
      | ke_call
      | dsimp only))

theorem apply_core_loop1_ke [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {np p : Usize}
    (h : arch.cm4.entry.apply_core_loop1 ki ai ti k hw tables s np p = ok (k', hw', s')) :
    KE kv okK k' k := by
  unfold arch.cm4.entry.apply_core_loop1 at h
  refine loop_inv _ (fun x => KE kv okK x.1 k) (fun r => KE kv okK r.1 k) ?_ ?_ _ (k', hw', s')
    (KE.refl _ _ _) h
  · rintro ⟨k1, hw1, st, c⟩ ⟨k2, hw2, st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.apply_core_loop1.body at hb
    peel hb
    all_goals (ke_close; done)
  · rintro ⟨k1, hw1, st, c⟩ ⟨k2, hw2, st'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold arch.cm4.entry.apply_core_loop1.body at hb
    peel hb
    all_goals (ke_close; done)

ke_reg apply_core_loop1_ke

/-- `apply_core` ends no Job: the responses were applied by the Kernel
entry before it. -/
theorem apply_core_ke [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {safe : Bool}
    (h : arch.cm4.entry.apply_core ki ai ti k hw tables s = ok (safe, k', hw', s')) :
    KE kv okK k' k := by
  unfold arch.cm4.entry.apply_core at h
  peel h
  all_goals (ke_close; done)

ke_reg apply_core_ke

/-! ## Granted services leave the Kernel as it was -/

set_option maxHeartbeats 4000000 in
theorem dma_request_core_granted {k k' : K} {hw hw' : A} {tables : T} {i : Usize}
    {ptr len : U32} {b : Bool}
    (h : arch.cm4.entry.dma_request_core ki ai ti k hw tables i ptr len = ok (b, k', hw'))
    (hb : b = true) : k' = k := by
  unfold arch.cm4.entry.dma_request_core at h
  peel h
  all_goals first
    | rfl
    | contradiction
    | (simp_all; done)

set_option maxHeartbeats 4000000 in
theorem dma_write_core_granted {k k' : K} {hw hw' : A} {tables : T} {i : Usize}
    {offset value : U32} {b : Bool}
    (h : arch.cm4.entry.dma_write_core ki ai ti k hw tables i offset value = ok (b, k', hw'))
    (hb : b = true) : k' = k := by
  unfold arch.cm4.entry.dma_write_core at h
  peel h
  all_goals first
    | rfl
    | contradiction
    | (simp_all; done)

set_option maxHeartbeats 4000000 in
theorem endpoint_recv_core_kept {k k' : K} {hw hw' : A} {tables : T} {arg ptr len : U32}
    {code : U32} {g : verus_builtin.Ghost verus_builtin.int} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables s arg ptr len
      = ok ((code, g), k', hw', s'))
    (hc : (code != arch.cm4.entry.ENDPOINT_REFUSED) = true) : k' = k := by
  unfold arch.cm4.entry.endpoint_recv_core at h
  peel h
  all_goals first
    | rfl
    | (simp at hc; done)

macro_rules | `(tactic| ke_call) => `(tactic|
  exact KE.of_eq (dma_request_core_granted (by assumption) (by assumption)))
macro_rules | `(tactic| ke_call) => `(tactic|
  exact KE.of_eq (dma_write_core_granted (by assumption) (by assumption)))
macro_rules | `(tactic| ke_call) => `(tactic|
  exact KE.of_eq (endpoint_recv_core_kept (by assumption) (by assumption)))

/-- A Kernel entry that leaves the records alone and the stack the same
Jobs keeps the invariant with nothing planned. -/
theorem kentry0' [KSpec ki kv okK] {k k' : K} {s s' : arch.cm4.plan.ArchState}
    (hke : KE kv okK k' k) (hfr : Frame s' s) (hok : okK k) (hai : AI kv k s 0) :
    okK k' ∧ AI kv k' s' (0#u32).val :=
  kentry0 (ki := ki) hke.kr (fun h => (hke h).2.2) hfr hok hai

theorem send_ne_recv {number : U8} (a : number = arch.cm4.entry.SVC_SEND)
    (b : number = arch.cm4.entry.SVC_RECV) : False := by
  rw [a] at b
  simp [arch.cm4.entry.SVC_SEND, arch.cm4.entry.SVC_RECV] at b

/-- Closes an exit of a supervisor-call core. -/
syntax "svc_exit " ident ident ident : tactic
macro_rules
  | `(tactic| svc_exit $hok:ident $hai:ident $ki:ident) => `(tactic| first
      | (kentry_close $hok $hai; done)
      | (contradiction; done)
      | (exact (send_ne_recv (by assumption) (by assumption)).elim)
      | (refine kentry0' (ki := $ki) ?_ ?_ $hok $hai
         · ke_close; done
         · frame_close))

set_option maxHeartbeats 8000000 in
theorem svc_service_core_keeps [KSpecD ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool}
    {frame frame' : Array U32 8#usize} {number : U8} (hok : okK k) (hai : AI kv k s 0)
    (h : arch.cm4.entry.svc_service_core ki ai ti k hw tables s frame number
      = ok ((hop, safe), k', hw', s', frame')) :
    okK k' ∧ AI kv k' s' hop.val := by
  unfold arch.cm4.entry.svc_service_core at h
  peel h
  all_goals (svc_exit hok hai ki; done)

set_option maxHeartbeats 8000000 in
theorem svc_core_keeps [KSpecD ki kv okK] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool}
    {frame frame' : Array U32 8#usize} {number : U8} (hok : okK k) (hai : AI kv k s 0)
    (h : arch.cm4.entry.svc_core ki ai ti k hw tables s frame number
      = ok ((hop, safe), k', hw', s', frame')) :
    okK k' ∧ AI kv k' s' hop.val := by
  unfold arch.cm4.entry.svc_core at h
  peel h
  all_goals first
    | exact svc_service_core_keeps hok hai h
    | (svc_exit hok hai ki; done)

end P18
