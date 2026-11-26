import Sched.ArchHwA1
open Aeneas Aeneas.Std Result

/-! # BASEPRI over a supervisor call (A1; 2026-11-26)

A supervisor call keeps A1: if BASEPRI was the value the Kernel's state
called for at the call, it is at the return, and 0 if the call ends in
the safe state (`svc_service_core_a1`, `svc_core_a1`). A call that ends
with the Kernel's last steps (`apply_core`) has BASEPRI rewritten there.
A call that does not (an accepted EasyDMA request or register write, a
receive that is not refused) leaves BASEPRI and returns the Kernel's
state it was given (`dma_write_core_kept`, `dma_request_core_kept`,
`endpoint_recv_core_kept` of `Sched/ArchSvcKeep.lean`). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {hv : A → HV} {bp : K → Nat}

syntax "kept_goal" : tactic
macro_rules
  | `(tactic| kept_goal) => `(tactic| first
      | (intro _; rfl)
      | (intro hb; cases hb; done)
      | (intro hb; exfalso; simp_all; done))

macro "kept_core " h:ident : tactic => `(tactic| (all_goals first
    | kept_goal
    | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
        | kept_goal
        | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
            | kept_goal
            | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;>
                kept_goal)))))

/-- An accepted EasyDMA register write returns the Kernel's state it was
given. -/
theorem dma_write_core_kept {k k1 : K} {hw hw1 : A} {tables : T} {i : Usize} {off v : U32}
    {b : Bool}
    (h : arch.cm4.entry.dma_write_core ki ai ti k hw tables i off v = ok (b, k1, hw1)) :
    b = true → k1 = k := by
  unfold arch.cm4.entry.dma_write_core at h
  peel h
  kept_core h

/-- An accepted EasyDMA request returns the Kernel's state it was given. -/
theorem dma_request_core_kept {k k1 : K} {hw hw1 : A} {tables : T} {i : Usize} {ptr len : U32}
    {b : Bool}
    (h : arch.cm4.entry.dma_request_core ki ai ti k hw tables i ptr len = ok (b, k1, hw1)) :
    b = true → k1 = k := by
  unfold arch.cm4.entry.dma_request_core at h
  peel h
  kept_core h

/-- One case of the supervisor call's service. -/
syntax "a1s_goal" : tactic
macro_rules
  | `(tactic| a1s_goal) => `(tactic| first
      | a1c_goal
      | (have ek := dma_write_core_kept (by assumption) (by assumption)
         subst ek
         refine a1_keep ?_
         hb_close
         done)
      | (have ek := dma_request_core_kept (by assumption) (by assumption)
         subst ek
         refine a1_keep ?_
         hb_close
         done)
      | (exfalso; simp_all [arch.cm4.entry.SVC_SEND, arch.cm4.entry.SVC_RECV]; done)
      | (have ek := endpoint_recv_core_kept (by assumption) (by assumption)
         subst ek
         refine a1_keep ?_
         hb_close
         done))

macro "a1s_core " h:ident : tactic => `(tactic| (all_goals first
    | a1s_goal
    | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
        | a1s_goal
        | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;> first
            | a1s_goal
            | (rcases ite_cases $h:ident with ⟨_, $h:ident⟩ | ⟨_, $h:ident⟩ <;> peel $h:ident <;>
                a1s_goal)))))

set_option maxHeartbeats 64000000 in
/-- **A supervisor call's service keeps A1** (`svc_service_core`). -/
theorem svc_service_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {frame frame' : Array U32 8#usize} {number : U8}
    {r : U32} {safe : Bool}
    (h : arch.cm4.entry.svc_service_core ki ai ti k hw tables s frame number
      = ok ((r, safe), k', hw', s', frame')) :
    (safe = true → (hv hw').basepri = 0) ∧
    (safe = false → (hv hw).basepri = bp k → (hv hw').basepri = bp k') := by
  unfold arch.cm4.entry.svc_service_core at h
  peel h
  a1s_core h

a1c_reg svc_service_core_a1

set_option maxHeartbeats 64000000 in
/-- **A supervisor call keeps A1** (`svc_core`). -/
theorem svc_core_a1 [HSpec ai hv] [KBp ki bp] {k k' : K} {hw hw' : A} {tables : T}
    {s s' : arch.cm4.plan.ArchState} {frame frame' : Array U32 8#usize} {number : U8}
    {r : U32} {safe : Bool}
    (h : arch.cm4.entry.svc_core ki ai ti k hw tables s frame number
      = ok ((r, safe), k', hw', s', frame')) :
    (safe = true → (hv hw').basepri = 0) ∧
    (safe = false → (hv hw).basepri = bp k → (hv hw').basepri = bp k') := by
  unfold arch.cm4.entry.svc_core at h
  peel h
  a1s_core h

end P18
