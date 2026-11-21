import Sched.IsolationBoot
open Aeneas Aeneas.Std Result

/-! # What the Endpoint cores answer (2026-11-20)

Value-level statements for `endpoint_args`, `endpoint_send_core` and
`endpoint_recv_core`, on the extracted code, in the form "if it returns":

- `endpoint_args_some`: an accepted argument names an Endpoint and two
  different roles of it, the executing Job's Task performs the role, the
  message is word-aligned, of the Endpoint's payload length, and the Kernel
  permits it for the role's Partition (R42.2; Verus: `endpoint_args`'s
  `ensures`).
- `recv_kernel_kept`: a receive that is not refused leaves the Kernel as it
  was (Verus: `r.0 != ENDPOINT_REFUSED ==> *final(k) == *old(k)`).
- `send_ok_only`, `recv_ok_only`: the code `ENDPOINT_OK` is answered only
  after the arguments were accepted, no peer change was owed, the channel
  took (gave) a slot and the slot lies in the Endpoint storage. -/

namespace P18
open rsk_kernel

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T}

/-- What `endpoint_args` checked of an argument it accepted. -/
def EpArgs (ki : arch.bound.KernelOps K) (ti : arch.cm4.entry.Tables T) (k : K) (tables : T)
    (ptr len : U32) (e role peer : Usize) (cfg : arch.cm4.plan.EndpointCfg) : Prop :=
  e.val < 8 ∧ role.val < 4 ∧ peer.val < 4 ∧ role ≠ peer ∧ ti.endpoint tables e = ok cfg ∧
  cfg.roles.val ≤ 4 ∧ role.val < cfg.roles.val ∧ peer.val < cfg.roles.val ∧
  1 ≤ cfg.capacity.val ∧ 4 ≤ cfg.payload.val ∧ cfg.payload.val % 4 = 0 ∧
  len.val = cfg.payload.val ∧ ptr.val % 4 = 0 ∧
  ∃ (t : U8) (tk p : U8), ki.top k = ok (some t) ∧ cfg.tasks.index_usize role = ok tk ∧ tk = t ∧
    cfg.partitions.index_usize role = ok p ∧ ki.message_permitted k p ptr len = ok true

theorem u16_rem4_ok {a c : U16} (h : a % 4#u16 = ok c) : c.val = a.val % 4 := by
  have hs := UScalar.rem_spec a (y := 4#u16) (by decide)
  rw [h] at hs
  simpa [Aeneas.Std.WP.spec_ok] using hs

theorem u32_rem4_ok {a c : U32} (h : a % 4#u32 = ok c) : c.val = a.val % 4 := by
  have hs := UScalar.rem_spec a (y := 4#u32) (by decide)
  rw [h] at hs
  simpa [Aeneas.Std.WP.spec_ok] using hs

/-- Splits the conditional at the head of `h`, closes the branch that
returned the wrong answer, and leaves the other. -/
macro "ite1 " h:ident : tactic =>
  `(tactic| (rcases ite_cases $h:ident with ⟨hc, hnew⟩ | ⟨hc, hnew⟩ <;> clear $h:ident <;>
      first | (simp at hnew; done) | (have $h:ident := hnew; clear hnew)))

set_option maxHeartbeats 8000000 in
theorem endpoint_args_some {k : K} {tables : T} {arg ptr len : U32} {e role peer : Usize}
    {cfg : arch.cm4.plan.EndpointCfg}
    (h : arch.cm4.entry.endpoint_args ki ti k tables arg ptr len = ok (some (e, role, peer, cfg))) :
    EpArgs ki ti k tables ptr len e role peer cfg := by
  unfold arch.cm4.entry.endpoint_args at h
  simp only [max_roles, max_endpoints, lift, bind_ok] at h
  unfold EpArgs
  simp only [bind_eq_ok] at h
  obtain ⟨i5, hi5, i12, hi12, i16, hi16, h⟩ := h
  ite1 h
  simp only [bind_eq_ok] at h
  obtain ⟨i6, hi6, h⟩ := h
  ite1 h
  ite1 h
  ite1 h
  ite1 h
  ite1 h
  simp only [bind_eq_ok] at h
  obtain ⟨c, hcfg, h⟩ := h
  ite1 h
  ite1 h
  ite1 h
  simp only [bind_eq_ok] at h
  obtain ⟨o, ho, ok1, hok1, h⟩ := h
  rcases ite_cases h with ⟨hok, h⟩ | ⟨_, h⟩
  · simp at h
    obtain ⟨rfl, rfl, rfl, rfl⟩ := h
    subst hok
    cases o with
    | none => simp at hok1
    | some t =>
      simp only [bind_eq_ok] at hok1
      obtain ⟨i10, hi10, hok1⟩ := hok1
      ite1 hok1
      ite1 hok1
      ite1 hok1
      simp only [bind_eq_ok] at hok1
      obtain ⟨i11, hi11, hok1⟩ := hok1
      ite1 hok1
      ite1 hok1
      simp only [bind_eq_ok] at hok1
      obtain ⟨i13, hi13, hok1⟩ := hok1
      ite1 hok1
      simp only [bind_eq_ok] at hok1
      obtain ⟨i14, hi14, hok1⟩ := hok1
      have hp4 := u16_rem4_ok hi11
      have hq4 := u32_rem4_ok hi13
      refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, t, i10, i14, ho, hi10, ?_, hi14,
        hok1⟩
      all_goals first
        | assumption
        | (scalar_tac; done)
        | (simp_all; done)
        | (simp_all; scalar_tac)
  · simp at h

theorem ep_codes : arch.cm4.entry.ENDPOINT_OK = 0#u32 ∧ arch.cm4.entry.ENDPOINT_FULL = 1#u32 ∧
    arch.cm4.entry.ENDPOINT_EMPTY = 1#u32 ∧ arch.cm4.entry.ENDPOINT_REFUSED = 2#u32 ∧ arch.cm4.entry.ENDPOINT_PEER = 3#u32 := by
  unfold arch.cm4.entry.ENDPOINT_OK arch.cm4.entry.ENDPOINT_FULL arch.cm4.entry.ENDPOINT_EMPTY arch.cm4.entry.ENDPOINT_REFUSED arch.cm4.entry.ENDPOINT_PEER
  exact ⟨rfl, rfl, rfl, rfl, rfl⟩

set_option maxHeartbeats 8000000 in
/-- **A receive that is not refused leaves the Kernel as it was.** -/
theorem recv_kernel_kept {k k' : K} {hw hw' : A} {tables : T}
    {state s' : arch.cm4.plan.ArchState} {arg ptr len code : U32}
    {g : verus_builtin.Ghost verus_builtin.int}
    (h : arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables state arg ptr len
      = ok ((code, g), k', hw', s')) :
    code ≠ arch.cm4.entry.ENDPOINT_REFUSED → k' = k := by
  unfold arch.cm4.entry.endpoint_recv_core at h
  peel h
  all_goals first
    | (intro _; rfl)
    | (intro hne; exact absurd rfl hne)

/-- What an accepted send went through. -/
def SendOk (ki : arch.bound.KernelOps K) (ti : arch.cm4.entry.Tables T) (k : K) (tables : T)
    (state : arch.cm4.plan.ArchState) (arg ptr len : U32) : Prop :=
  ∃ (e role peer : Usize) (cfg : arch.cm4.plan.EndpointCfg) (rs ch d : Usize) (s : U8)
    (st1 : arch.cm4.plan.ArchState) (dst : U32),
    arch.cm4.entry.endpoint_args ki ti k tables arg ptr len = ok (some (e, role, peer, cfg)) ∧
    arch.cm4.entry.role_slot e role = ok rs ∧ state.peer_changed.index_usize rs = ok false ∧
    arch.cm4.entry.channel_index e role peer = ok ch ∧
    arch.cm4.entry.channel_push state ch cfg.capacity = ok (some s, st1) ∧
    arch.cm4.entry.endpoint_slot ti tables cfg d s = ok (some dst)

set_option maxHeartbeats 16000000 in
/-- **`ENDPOINT_OK` is answered to a send only after `SendOk`.** -/
theorem send_ok_only {k k' : K} {hw hw' : A} {tables : T}
    {state s' : arch.cm4.plan.ArchState} {arg ptr len code : U32}
    (h : arch.cm4.entry.endpoint_send_core ki ai ti k hw tables state arg ptr len = ok (code, k', hw', s')) :
    code = arch.cm4.entry.ENDPOINT_OK → SendOk ki ti k tables state arg ptr len := by
  unfold arch.cm4.entry.endpoint_send_core at h
  unfold SendOk
  peel h
  all_goals first
    | (intro hc; simp [ep_codes.1, ep_codes.2.1, ep_codes.2.2.2.1, ep_codes.2.2.2.2] at hc; done)
    | (intro _
       refine ⟨_, _, _, _, _, _, _, _, _, _, (by assumption), (by assumption), ?_, (by assumption),
         (by assumption), (by assumption)⟩
       simp_all)

/-- What an accepted receive went through. -/
def RecvOk (ki : arch.bound.KernelOps K) (ti : arch.cm4.entry.Tables T) (k : K) (tables : T)
    (state : arch.cm4.plan.ArchState) (arg ptr len : U32) : Prop :=
  ∃ (e role frm : Usize) (cfg : arch.cm4.plan.EndpointCfg) (rs ch d : Usize) (s : U8)
    (st1 : arch.cm4.plan.ArchState) (src : U32),
    arch.cm4.entry.endpoint_args ki ti k tables arg ptr len = ok (some (e, role, frm, cfg)) ∧
    arch.cm4.entry.role_slot e role = ok rs ∧ state.peer_changed.index_usize rs = ok false ∧
    arch.cm4.entry.channel_index e frm role = ok ch ∧
    arch.cm4.entry.channel_pop state ch cfg.capacity = ok (some s, st1) ∧
    arch.cm4.entry.endpoint_slot ti tables cfg d s = ok (some src)

set_option maxHeartbeats 16000000 in
/-- **`ENDPOINT_OK` is answered to a receive only after `RecvOk`.** -/
theorem recv_ok_only {k k' : K} {hw hw' : A} {tables : T}
    {state s' : arch.cm4.plan.ArchState} {arg ptr len code : U32}
    {g : verus_builtin.Ghost verus_builtin.int}
    (h : arch.cm4.entry.endpoint_recv_core ki ai ti k hw tables state arg ptr len
      = ok ((code, g), k', hw', s')) :
    code = arch.cm4.entry.ENDPOINT_OK → RecvOk ki ti k tables state arg ptr len := by
  unfold arch.cm4.entry.endpoint_recv_core at h
  unfold RecvOk
  peel h
  all_goals first
    | (intro hc; simp [ep_codes.1, ep_codes.2.1, ep_codes.2.2.1, ep_codes.2.2.2.1,
        ep_codes.2.2.2.2] at hc; done)
    | (intro _
       refine ⟨_, _, _, _, _, _, _, _, _, _, (by assumption), (by assumption), ?_, (by assumption),
         (by assumption), (by assumption)⟩
       simp_all)

end P18
