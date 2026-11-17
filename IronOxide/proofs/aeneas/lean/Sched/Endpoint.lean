import Sched.Sporadic
open Aeneas Aeneas.Std Result

/-! # An Endpoint operation on the code

The entry cores of `rsk-kernel` (`arch/cm4/entry.rs`) are in
`Sched/Kernel.lean` since 2026-11-17, generic over the Kernel interface
(`KernelOps`), the hardware interface and the generated tables. This file
reads from `endpoint_send_core` and `endpoint_recv_core` what they do to
the Kernel, each statement of the form "if the function returns":

- `endpoint_send_core_kernel`, `endpoint_recv_core_kernel`: for any
  implementor of the interfaces, the Kernel after the operation is the
  Kernel before, or the result of exactly one interface call: `fault` (an
  API misuse), `record_event` (a full buffer; send only) or `intake` (the
  release of the receiving Task; send only).
- `bound_fault`, `bound_record_event`: what those calls are for `Bound`,
  the implementor `rsk-kernel` links: `Kernel::fault` and
  `Kernel::record_event` on its Kernel state.
- `KStep.svc_send`, `KStep.svc_recv`: so an Endpoint operation by the Job
  on top is the run's `svc_h` transition, followed by `faulted` when the
  operation was an API misuse; for a send that releases the receiver, the
  `svc_h` transition leaves the Kernel as it was and the interface's
  `intake` call is what remains (the run's `intake` transition, whose link
  to `Bound`'s `intake` is still by hand: Verification_Plan 7.3).

The interface is given as a structure of functions, of which the extraction
holds `Bound`'s one by one and not as a whole (some are not extracted); the
two link theorems therefore take an implementor whose `fault` and
`record_event` are `Bound`'s. What the cores do to the hardware and to the
Endpoint buffers is not read here. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

set_option maxHeartbeats 4000000 in
theorem endpoint_send_core_kernel {K A T : Type _}
    (ki : _root_.rsk_kernel.arch.bound.KernelOps K)
    (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A)
    (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T)
    {k k' : K} {hw hw' : A} {tables : T}
    {state state' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {arg ptr len res : U32}
    (h : _root_.rsk_kernel.arch.cm4.entry.endpoint_send_core ki ai ti k hw tables state arg ptr len
      = ok (res, k', hw', state')) :
    k' = k ∨ ki.fault k logic.log.EV_API_MISUSE false = ok k' ∨
    (∃ t, ki.record_event k logic.log.EV_ENDPOINT_FULL t = ok k') ∨
    (∃ t sp now off, ki.intake k t sp now off = ok k') := by
  unfold _root_.rsk_kernel.arch.cm4.entry.endpoint_send_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (Or.inl (by assumption))
    | exact Or.inr (Or.inr (Or.inl ⟨_, by assumption⟩))
    | exact Or.inr (Or.inr (Or.inr ⟨_, _, _, _, by assumption⟩))

set_option maxHeartbeats 4000000 in
theorem endpoint_recv_core_kernel {K A T : Type _}
    (ki : _root_.rsk_kernel.arch.bound.KernelOps K)
    (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A)
    (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T)
    {k k' : K} {hw hw' : A} {tables : T}
    {state state' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {arg ptr len : U32} {r}
    (h : _root_.rsk_kernel.arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables state arg ptr len
      = ok (r, k', hw', state')) :
    k' = k ∨ ki.fault k logic.log.EV_API_MISUSE false = ok k' := by
  unfold _root_.rsk_kernel.arch.cm4.entry.endpoint_recv_core at h
  peel h
  all_goals first
    | exact Or.inl rfl
    | exact Or.inr (by assumption)

/-- `Bound`'s `fault` is `Kernel::fault` on its Kernel state. -/
theorem bound_fault {b b' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    {kind : U8} {kn : Bool}
    (h : _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault b kind kn
      = ok b') :
    logic.kernel.Kernel.fault b.kernel b.cfg kind kn = ok b'.kernel ∧ b'.cfg = b.cfg := by
  unfold _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault at h
  simp only [bind_eq_ok] at h
  obtain ⟨k1, hk, h⟩ := h
  have h := ok_inj h
  subst h
  exact ⟨hk, rfl⟩

/-- `Bound`'s `record_event` is `Kernel::record_event` on its Kernel
state, or nothing (a Task index out of range). -/
theorem bound_record_event {b b' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE}
    {kind t : U8}
    (h : _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event b kind t
      = ok b') :
    b'.cfg = b.cfg ∧
    (b'.kernel = b.kernel ∨
      logic.kernel.Kernel.record_event b.kernel b.cfg kind t = ok b'.kernel) := by
  unfold _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event at h
  peel h
  all_goals first
    | exact ⟨rfl, Or.inr (by assumption)⟩
    | exact ⟨rfl, Or.inl rfl⟩

/-- An Endpoint send by the Job on top, as `endpoint_send_core` returns
for `Bound`, is the run's `svc_h` transition (the Kernel as before, or a
full buffer recorded); or `svc_h` and then `faulted` (an API misuse); or
`svc_h` with the Kernel as before, the interface's `intake` call
remaining (a send that releases the receiver). -/
theorem KStep.svc_send {cfg : logic.Config NT NR NP} {P : Params} (x : KSt NT NR NP NL NS NE)
    {A T : Type _}
    (ki : _root_.rsk_kernel.arch.bound.KernelOps
      (_root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE))
    (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A)
    (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T)
    (hf : ∀ b kind kn, ki.fault b kind kn =
      _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault b kind kn)
    (hr : ∀ b kind t, ki.record_event b kind t =
      _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event b kind t)
    {b b' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE} {hw hw' : A} {tables : T}
    {state state' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {arg ptr len res : U32}
    (hb : b.kernel = x.k) (hc : b.cfg = cfg)
    (h : _root_.rsk_kernel.arch.cm4.entry.endpoint_send_core ki ai ti b hw tables state arg ptr len
      = ok (res, b', hw', state'))
    (hsel : logic.sched.Sched.select x.k.sched cfg = ok none)
    (hk : x.ks = 0) (c : Nat) (hd : 0 < x.k.sched.depth.val)
    (hrun : 0 < x.run x.top) (hre : c ≤ x.re x.top)
    (hal : ∀ p, topEntry x.k.sched p → c ≤ x.allow p) :
    (∃ y, KStep cfg P x y ∧ y.k = b'.kernel) ∨
    (∃ y z, KStep cfg P x y ∧ y.k = x.k ∧ KStep cfg P y z ∧ z.k = b'.kernel) ∨
    (∃ y, KStep cfg P x y ∧ y.k = x.k ∧ ∃ t sp now off, ki.intake b t sp now off = ok b') := by
  rcases endpoint_send_core_kernel ki ai ti h with e | hfa | ⟨t, hrec⟩ | hin
  · left
    exact ⟨_, KStep.svc_h x hsel hk c b'.kernel hd hrun hre hal (Or.inl (by rw [e, hb])), rfl⟩
  · right; left
    rw [hf] at hfa
    obtain ⟨hkf, _⟩ := bound_fault hfa
    rw [hb, hc] at hkf
    exact ⟨_, _, KStep.svc_h x hsel hk c x.k hd hrun hre hal (Or.inl rfl), rfl,
      KStep.faulted _ _ _ b'.kernel hkf, rfl⟩
  · left
    rw [hr] at hrec
    obtain ⟨_, hk' | hk'⟩ := bound_record_event hrec
    · exact ⟨_, KStep.svc_h x hsel hk c b'.kernel hd hrun hre hal (Or.inl (by rw [hk', hb])), rfl⟩
    · rw [hb, hc] at hk'
      exact ⟨_, KStep.svc_h x hsel hk c b'.kernel hd hrun hre hal (Or.inr ⟨_, _, hk'⟩), rfl⟩
  · right; right
    exact ⟨_, KStep.svc_h x hsel hk c x.k hd hrun hre hal (Or.inl rfl), rfl, hin⟩

/-- An Endpoint receive by the Job on top, as `endpoint_recv_core` returns
for `Bound`, is the run's `svc_h` transition with the Kernel as before,
or `svc_h` and then `faulted` (an API misuse). -/
theorem KStep.svc_recv {cfg : logic.Config NT NR NP} {P : Params} (x : KSt NT NR NP NL NS NE)
    {A T : Type _}
    (ki : _root_.rsk_kernel.arch.bound.KernelOps
      (_root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE))
    (ai : _root_.rsk_kernel.arch.cm4.entry.KernelArch A)
    (ti : _root_.rsk_kernel.arch.cm4.entry.Tables T)
    (hf : ∀ b kind kn, ki.fault b kind kn =
      _root_.rsk_kernel.arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault b kind kn)
    {b b' : _root_.rsk_kernel.arch.bound.Bound NT NR NP NL NS NE} {hw hw' : A} {tables : T}
    {state state' : _root_.rsk_kernel.arch.cm4.plan.ArchState} {arg ptr len : U32} {r}
    (hb : b.kernel = x.k) (hc : b.cfg = cfg)
    (h : _root_.rsk_kernel.arch.cm4.entry.endpoint_recv_core ki ai ti b hw tables state arg ptr len
      = ok (r, b', hw', state'))
    (hsel : logic.sched.Sched.select x.k.sched cfg = ok none)
    (hk : x.ks = 0) (c : Nat) (hd : 0 < x.k.sched.depth.val)
    (hrun : 0 < x.run x.top) (hre : c ≤ x.re x.top)
    (hal : ∀ p, topEntry x.k.sched p → c ≤ x.allow p) :
    (∃ y, KStep cfg P x y ∧ y.k = b'.kernel) ∨
    (∃ y z, KStep cfg P x y ∧ y.k = x.k ∧ KStep cfg P y z ∧ z.k = b'.kernel) := by
  rcases endpoint_recv_core_kernel ki ai ti h with e | hfa
  · left
    exact ⟨_, KStep.svc_h x hsel hk c b'.kernel hd hrun hre hal (Or.inl (by rw [e, hb])), rfl⟩
  · right
    rw [hf] at hfa
    obtain ⟨hkf, _⟩ := bound_fault hfa
    rw [hb, hc] at hkf
    exact ⟨_, _, KStep.svc_h x hsel hk c x.k hd hrun hre hal (Or.inl rfl), rfl,
      KStep.faulted _ _ _ b'.kernel hkf, rfl⟩

end P18
