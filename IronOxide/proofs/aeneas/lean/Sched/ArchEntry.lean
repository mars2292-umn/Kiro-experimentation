import Sched.ArchSvc
open Aeneas Aeneas.Std Result

/-! # The Kernel-level entry cores leave the level records alone (2026-11-19)

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
theorem start_core_both {k k' : K} {hw hw' : A} {tables : T} {b : Bool} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.start_core ki ai ti k hw tables s = ok (b, k', hw', s')) :
    Frame s' s ∧ ∀ (kv : K → Arch.KV) (okK : K → Prop) [KSpec ki kv okK], KR kv okK k' k := by
  unfold arch.cm4.entry.start_core at h
  peel h
  all_goals first
    | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done)))))

theorem start_core_frame {k k' : K} {hw hw' : A} {tables : T} {b : Bool} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.start_core ki ai ti k hw tables s = ok (b, k', hw', s')) : Frame s' s :=
  (start_core_both h).1

frame_reg start_core_frame

theorem start_core_kr [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {b : Bool} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.start_core ki ai ti k hw tables s = ok (b, k', hw', s')) : KR kv okK k' k :=
  (start_core_both h).2 kv okK

kr_reg start_core_kr

set_option maxHeartbeats 8000000 in
theorem stack_violation_core_both {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.stack_violation_core ki ai ti k hw tables s = ok (x, k', hw', s')) :
    Frame s' s ∧ ∀ (kv : K → Arch.KV) (okK : K → Prop) [KSpec ki kv okK], KR kv okK k' k := by
  unfold arch.cm4.entry.stack_violation_core at h
  peel h
  all_goals first
    | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done)))))

theorem stack_violation_core_frame {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.stack_violation_core ki ai ti k hw tables s = ok (x, k', hw', s')) : Frame s' s :=
  (stack_violation_core_both h).1

frame_reg stack_violation_core_frame

theorem stack_violation_core_kr [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.stack_violation_core ki ai ti k hw tables s = ok (x, k', hw', s')) : KR kv okK k' k :=
  (stack_violation_core_both h).2 kv okK

kr_reg stack_violation_core_kr

set_option maxHeartbeats 8000000 in
theorem intake_core_both {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {n : U16} {task partition : U8} {psp exc_return : U32} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.intake_core ki ai ti k hw tables s n task partition psp exc_return = ok (x, k', hw', s')) :
    Frame s' s ∧ ∀ (kv : K → Arch.KV) (okK : K → Prop) [KSpec ki kv okK], KR kv okK k' k := by
  unfold arch.cm4.entry.intake_core at h
  peel h
  all_goals first
    | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done)))))

theorem intake_core_frame {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {n : U16} {task partition : U8} {psp exc_return : U32} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.intake_core ki ai ti k hw tables s n task partition psp exc_return = ok (x, k', hw', s')) : Frame s' s :=
  (intake_core_both h).1

frame_reg intake_core_frame

theorem intake_core_kr [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {n : U16} {task partition : U8} {psp exc_return : U32} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.intake_core ki ai ti k hw tables s n task partition psp exc_return = ok (x, k', hw', s')) : KR kv okK k' k :=
  (intake_core_both h).2 kv okK

kr_reg intake_core_kr

set_option maxHeartbeats 8000000 in
theorem unbound_core_both {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {n : U16} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.unbound_core ki ai ti k hw tables s n = ok (x, k', hw', s')) :
    Frame s' s ∧ ∀ (kv : K → Arch.KV) (okK : K → Prop) [KSpec ki kv okK], KR kv okK k' k := by
  unfold arch.cm4.entry.unbound_core at h
  peel h
  all_goals first
    | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done)))))

theorem unbound_core_frame {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {n : U16} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.unbound_core ki ai ti k hw tables s n = ok (x, k', hw', s')) : Frame s' s :=
  (unbound_core_both h).1

frame_reg unbound_core_frame

theorem unbound_core_kr [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {n : U16} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.unbound_core ki ai ti k hw tables s n = ok (x, k', hw', s')) : KR kv okK k' k :=
  (unbound_core_both h).2 kv okK

kr_reg unbound_core_kr

set_option maxHeartbeats 8000000 in
theorem timer_core_both {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.timer_core ki ai ti k hw tables s = ok (x, k', hw', s')) :
    Frame s' s ∧ ∀ (kv : K → Arch.KV) (okK : K → Prop) [KSpec ki kv okK], KR kv okK k' k := by
  unfold arch.cm4.entry.timer_core at h
  peel h
  all_goals first
    | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done)))))

theorem timer_core_frame {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.timer_core ki ai ti k hw tables s = ok (x, k', hw', s')) : Frame s' s :=
  (timer_core_both h).1

frame_reg timer_core_frame

theorem timer_core_kr [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.timer_core ki ai ti k hw tables s = ok (x, k', hw', s')) : KR kv okK k' k :=
  (timer_core_both h).2 kv okK

kr_reg timer_core_kr

set_option maxHeartbeats 8000000 in
theorem svc_service_core_both {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {frame frame' : Array U32 8#usize} {number : U8} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.svc_service_core ki ai ti k hw tables s frame number = ok (x, k', hw', s', frame')) :
    Frame s' s ∧ ∀ (kv : K → Arch.KV) (okK : K → Prop) [KSpec ki kv okK], KR kv okK k' k := by
  unfold arch.cm4.entry.svc_service_core at h
  peel h
  all_goals first
    | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done)))))

theorem svc_service_core_frame {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {frame frame' : Array U32 8#usize} {number : U8} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.svc_service_core ki ai ti k hw tables s frame number = ok (x, k', hw', s', frame')) : Frame s' s :=
  (svc_service_core_both h).1

frame_reg svc_service_core_frame

theorem svc_service_core_kr [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {frame frame' : Array U32 8#usize} {number : U8} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.svc_service_core ki ai ti k hw tables s frame number = ok (x, k', hw', s', frame')) : KR kv okK k' k :=
  (svc_service_core_both h).2 kv okK

kr_reg svc_service_core_kr

set_option maxHeartbeats 8000000 in
theorem svc_core_both {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {frame frame' : Array U32 8#usize} {number : U8} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.svc_core ki ai ti k hw tables s frame number = ok (x, k', hw', s', frame')) :
    Frame s' s ∧ ∀ (kv : K → Arch.KV) (okK : K → Prop) [KSpec ki kv okK], KR kv okK k' k := by
  unfold arch.cm4.entry.svc_core at h
  peel h
  all_goals first
    | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done)))))

theorem svc_core_frame {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {frame frame' : Array U32 8#usize} {number : U8} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.svc_core ki ai ti k hw tables s frame number = ok (x, k', hw', s', frame')) : Frame s' s :=
  (svc_core_both h).1

frame_reg svc_core_frame

theorem svc_core_kr [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {frame frame' : Array U32 8#usize} {number : U8} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.svc_core ki ai ti k hw tables s frame number = ok (x, k', hw', s', frame')) : KR kv okK k' k :=
  (svc_core_both h).2 kv okK

kr_reg svc_core_kr

set_option maxHeartbeats 8000000 in
theorem fault_core_both {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {kind : U8} {in_kernel : Bool} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.fault_core ki ai ti k hw tables s kind in_kernel = ok (x, k', hw', s')) :
    Frame s' s ∧ ∀ (kv : K → Arch.KV) (okK : K → Prop) [KSpec ki kv okK], KR kv okK k' k := by
  unfold arch.cm4.entry.fault_core at h
  peel h
  all_goals first
    | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
    | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
        | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
        | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> first
            | (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done))
            | (rcases ite_cases h with ⟨_, h⟩ | ⟨_, h⟩ <;> peel h <;> (refine ⟨?_, fun kv okK _ => ?_⟩ <;> first | (frame_close; done) | (kr_close; done)))))

theorem fault_core_frame {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {kind : U8} {in_kernel : Bool} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.fault_core ki ai ti k hw tables s kind in_kernel = ok (x, k', hw', s')) : Frame s' s :=
  (fault_core_both h).1

frame_reg fault_core_frame

theorem fault_core_kr [KSpec ki kv okK] {k k' : K} {hw hw' : A} {tables : T} {x : U32 × Bool} {kind : U8} {in_kernel : Bool} {s s' : arch.cm4.plan.ArchState}
    (h : arch.cm4.entry.fault_core ki ai ti k hw tables s kind in_kernel = ok (x, k', hw', s')) : KR kv okK k' k :=
  (fault_core_both h).2 kv okK

kr_reg fault_core_kr

end P18
