import Sched.ArchCores
open Aeneas Aeneas.Std Result

/-! # The service cores leave the level records alone (2026-11-19)

Part of the proof that the entry cores keep the level records and the
Kernel's stack in agreement (`Sched/ArchCores.lean`): for each of these
cores, if it returns, the level records, the Partition taken as executing
and the owed unwind are as they were (`Frame`), and the Kernel's stack has
only lost Jobs from the top (`KR`). One file per group of cores, because
each takes minutes to check. -/

namespace P18
open rsk_kernel

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

set_option maxHeartbeats 8000000 in
theorem rejection_core_both {k k' : K} {cfg : arch.cm4.plan.EndpointCfg} {e role frm : Usize} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.rejection_core ki k s cfg e role frm = ok (k', s')) :
    Frame s' s ∧ ∀ (kv : K → Arch.KV) (okK : K → Prop) [KSpec ki kv okK], KR kv okK k' k := by
  unfold arch.cm4.entry.rejection_core at h
  peel h
  all_goals first
    | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done)))))

theorem rejection_core_frame {k k' : K} {cfg : arch.cm4.plan.EndpointCfg} {e role frm : Usize} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.rejection_core ki k s cfg e role frm = ok (k', s')) : Frame s' s :=
  (rejection_core_both h).1

frame_reg rejection_core_frame

theorem rejection_core_kr [KSpec ki kv okK] {k k' : K} {cfg : arch.cm4.plan.EndpointCfg} {e role frm : Usize} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.rejection_core ki k s cfg e role frm = ok (k', s')) : KR kv okK k' k :=
  (rejection_core_both h).2 kv okK

kr_reg rejection_core_kr

set_option maxHeartbeats 8000000 in
theorem violation_core_both {k k' : K} {tables : T} {arg : U32} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.violation_core ki ti k tables s arg = ok (k', s')) :
    Frame s' s ∧ ∀ (kv : K → Arch.KV) (okK : K → Prop) [KSpec ki kv okK], KR kv okK k' k := by
  unfold arch.cm4.entry.violation_core at h
  peel h
  all_goals first
    | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done)))))

theorem violation_core_frame {k k' : K} {tables : T} {arg : U32} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.violation_core ki ti k tables s arg = ok (k', s')) : Frame s' s :=
  (violation_core_both h).1

frame_reg violation_core_frame

theorem violation_core_kr [KSpec ki kv okK] {k k' : K} {tables : T} {arg : U32} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.violation_core ki ti k tables s arg = ok (k', s')) : KR kv okK k' k :=
  (violation_core_both h).2 kv okK

kr_reg violation_core_kr

set_option maxHeartbeats 8000000 in
theorem endpoint_send_core_both {k k' : K} {hw hw' : A} {tables : T} {arg ptr len x : U32} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_send_core ki ai ti k hw tables s arg ptr len = ok (x, k', hw', s')) :
    Frame s' s ∧ ∀ (kv : K → Arch.KV) (okK : K → Prop) [KSpec ki kv okK], KR kv okK k' k := by
  unfold arch.cm4.entry.endpoint_send_core at h
  peel h
  all_goals first
    | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done)))))

theorem endpoint_send_core_frame {k k' : K} {hw hw' : A} {tables : T} {arg ptr len x : U32} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_send_core ki ai ti k hw tables s arg ptr len = ok (x, k', hw', s')) : Frame s' s :=
  (endpoint_send_core_both h).1

frame_reg endpoint_send_core_frame

theorem endpoint_send_core_kr [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {arg ptr len x : U32} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_send_core ki ai ti k hw tables s arg ptr len = ok (x, k', hw', s')) : KR kv okK k' k :=
  (endpoint_send_core_both h).2 kv okK

kr_reg endpoint_send_core_kr

set_option maxHeartbeats 8000000 in
theorem endpoint_recv_core_both {k k' : K} {hw hw' : A} {tables : T} {arg ptr len : U32} {x : U32 × verus_builtin.Ghost verus_builtin.int} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables s arg ptr len = ok (x, k', hw', s')) :
    Frame s' s ∧ ∀ (kv : K → Arch.KV) (okK : K → Prop) [KSpec ki kv okK], KR kv okK k' k := by
  unfold arch.cm4.entry.endpoint_recv_core at h
  peel h
  all_goals first
    | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done)))))

theorem endpoint_recv_core_frame {k k' : K} {hw hw' : A} {tables : T} {arg ptr len : U32} {x : U32 × verus_builtin.Ghost verus_builtin.int} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables s arg ptr len = ok (x, k', hw', s')) : Frame s' s :=
  (endpoint_recv_core_both h).1

frame_reg endpoint_recv_core_frame

theorem endpoint_recv_core_kr [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {arg ptr len : U32} {x : U32 × verus_builtin.Ghost verus_builtin.int} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables s arg ptr len = ok (x, k', hw', s')) : KR kv okK k' k :=
  (endpoint_recv_core_both h).2 kv okK

kr_reg endpoint_recv_core_kr

set_option maxHeartbeats 8000000 in
theorem raise_core_both [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {b : Bool}
    (h : arch.cm4.entry.raise_core ki ai ti k hw tables i = ok (b, k', hw')) : KR kv okK k' k := by
  unfold arch.cm4.entry.raise_core at h
  peel h
  all_goals first
    | (kr_close; done)
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (kr_close; done)
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (kr_close; done)
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (kr_close; done))))

kr_reg raise_core_both

set_option maxHeartbeats 8000000 in
theorem dma_request_core_both [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {ptr len : U32} {b : Bool}
    (h : arch.cm4.entry.dma_request_core ki ai ti k hw tables i ptr len = ok (b, k', hw')) : KR kv okK k' k := by
  unfold arch.cm4.entry.dma_request_core at h
  peel h
  all_goals first
    | (kr_close; done)
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (kr_close; done)
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (kr_close; done)
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (kr_close; done))))

kr_reg dma_request_core_both

set_option maxHeartbeats 8000000 in
theorem dma_write_core_both [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {i : Usize} {offset value : U32} {b : Bool}
    (h : arch.cm4.entry.dma_write_core ki ai ti k hw tables i offset value = ok (b, k', hw')) : KR kv okK k' k := by
  unfold arch.cm4.entry.dma_write_core at h
  peel h
  all_goals first
    | (kr_close; done)
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (kr_close; done)
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (kr_close; done)
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (kr_close; done))))

kr_reg dma_write_core_both

end P18
