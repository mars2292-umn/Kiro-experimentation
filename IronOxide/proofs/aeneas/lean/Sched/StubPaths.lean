import Sched.StubRefine

/-! # The stubs' machine code against the stub model (2026-11-26)

For each path of each assembly stub (`unsafe_module/stubs.rs`): the machine
of `Sched/Arm.lean`, run on the stub's machine code (`Sched/StubCode.lean`)
from a state that is the model state `m` (`R`, `Sched/StubRefine.lean`),
returns, and the state it reaches is the model state the path's function
of `Sched/StubModel.lean` gives (`<path>_run`), with the program counter
at the instruction the path's control line names. The control lines of
the model (`-- ctl:`) are steps of the machine here: `bl` (`<stub>_bl`),
`cbz`, `bne`, `b`, `bx`.

The first form of these theorems carried a difference between the machine
and the model: a floating-point instruction (`vpusheq`, `vpopeq`)
executed in a handler with CONTROL.FPCA clear sets FPCA (B1.5.7, with
FPCCR.ASPEN set), which the model's `vpush` and `vpop` did not show. The
two stub models now set it (`fpca`), and the theorems are exact: the
machine state reached is the model state, with no excess.

Assumed here, as in the model: the exception model (what entry and return
do), and AAPCS for the Rust halves; the theorems start after an entry or
a call and stop at a call or an exception return. The proofs of the paths
were written by a script from the list of each path's instructions and
are checked here; the script is not kept. -/

namespace Stub
open Arm


theorem irq_after_call_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {a : Arm.St} {m : M}
    (hR : R kc (fun _ => 0) a m) (hit : a.it = 0#8) (hpc : a.pc = base + 42#32)
    (hhi : a.sp.toNat + 24 < 4294967296) :
    ∃ a', run (codeAt base (irqCode c1 c2 u1 u2)) 6 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = (if a'.r 0 = 0#32 then base + 76#32 else base + 54#32) ∧
      R kc (fun _ => 0) a' (irq_after_call m) := by
  have q0 : a.pc = base + 42#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_ldr_sp r0 t0 (rd := 0) (n := 8) (by omega) (by omega) (by decide)
  have st1 : step (codeAt base (irqCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact irq_f42 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 44#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  obtain ⟨a2, e2, p2, t2, s2, r2⟩ := sim_ldr_sp r1 t1 (rd := 1) (n := 12) (by omega) (by omega) (by decide)
  have st2 : step (codeAt base (irqCode c1 c2 u1 u2)) a1 = some a2 :=
    step_of (by rw [q1]; exact irq_f44 base c1 c2 u1 u2) e2
  have q2 : a2.pc = base + 46#32 := by rw [p2, q1]; pcn
  have s2' : a2.sp.toNat = a1.sp.toNat := by rw [s2]
  obtain ⟨a3, e3, p3, t3, s3, r3⟩ := sim_ldr_sp r2 t2 (rd := 2) (n := 16) (by omega) (by omega) (by decide)
  have st3 : step (codeAt base (irqCode c1 c2 u1 u2)) a2 = some a3 :=
    step_of (by rw [q2]; exact irq_f46 base c1 c2 u1 u2) e3
  have q3 : a3.pc = base + 48#32 := by rw [p3, q2]; pcn
  have s3' : a3.sp.toNat = a2.sp.toNat := by rw [s3]
  obtain ⟨a4, e4, p4, t4, s4, r4⟩ := sim_ldr_sp r3 t3 (rd := 3) (n := 20) (by omega) (by omega) (by decide)
  have st4 : step (codeAt base (irqCode c1 c2 u1 u2)) a3 = some a4 :=
    step_of (by rw [q3]; exact irq_f48 base c1 c2 u1 u2) e4
  have q4 : a4.pc = base + 50#32 := by rw [p4, q3]; pcn
  have s4' : a4.sp.toNat = a3.sp.toNat := by rw [s4]
  obtain ⟨a5, e5, p5, t5, s5, r5⟩ := sim_add_sp r4 t4 (n := 24) (by omega) (by decide)
  have st5 : step (codeAt base (irqCode c1 c2 u1 u2)) a4 = some a5 :=
    step_of (by rw [q4]; exact irq_f50 base c1 c2 u1 u2) e5
  have q5 : a5.pc = base + 52#32 := by rw [p5, q4]; pcn
  have st6 : step (codeAt base (irqCode c1 c2 u1 u2)) a5 = some _ :=
    step_of (by rw [q5]; exact irq_f52 base c1 c2 u1 u2) (exec_cbz t5 (by omega) _)
  refine ⟨_, ((run_succ _ 5 _ _ st1).trans ((run_succ _ 4 _ _ st2).trans ((run_succ _ 3 _ _ st3).trans ((run_succ _ 2 _ _ st4).trans ((run_succ _ 1 _ _ st5).trans ((run_succ _ 0 _ _ st6).trans rfl)))))), rfl, ?_, ?_⟩
  · show (if a5.r 0 = 0#32 then a5.pc + 4#32 + _ else a5.pc + 2#32) = _
    rw [q5]; pcn
  · refine (r5.kcongr rfl ?_).pcit _ _
    intro i hi; simp only [kset]; split_ifs <;> rfl

theorem irq_return_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}
    (hR : R kc kr a m) (hit : a.it = 0#8) (hpc : a.pc = base + 98#32) :
    ∃ a', run (codeAt base (irqCode c1 c2 u1 u2)) 4 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = a.lr ∧ R kc kr a' (irq_return m) := by
  have q0 : a.pc = base + 98#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_dsb r0 t0
  have st1 : step (codeAt base (irqCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact irq_f98 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 102#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  obtain ⟨a2, e2, p2, t2, s2, r2⟩ := sim_cpsid_f r1 t1
  have st2 : step (codeAt base (irqCode c1 c2 u1 u2)) a1 = some a2 :=
    step_of (by rw [q1]; exact irq_f102 base c1 c2 u1 u2) e2
  have q2 : a2.pc = base + 104#32 := by rw [p2, q1]; pcn
  have s2' : a2.sp.toNat = a1.sp.toNat := by rw [s2]
  obtain ⟨a3, e3, p3, t3, s3, r3⟩ := sim_cpsie_i r2 t2
  have st3 : step (codeAt base (irqCode c1 c2 u1 u2)) a2 = some a3 :=
    step_of (by rw [q2]; exact irq_f104 base c1 c2 u1 u2) e3
  have q3 : a3.pc = base + 106#32 := by rw [p3, q2]; pcn
  have s3' : a3.sp.toNat = a2.sp.toNat := by rw [s3]
  have st4 : step (codeAt base (irqCode c1 c2 u1 u2)) a3 = some _ :=
    step_of (by rw [q3]; exact irq_f106 base c1 c2 u1 u2) (exec_bx_lr t3)
  refine ⟨_, ((run_succ _ 3 _ _ st1).trans ((run_succ _ 2 _ _ st2).trans ((run_succ _ 1 _ _ st3).trans ((run_succ _ 0 _ _ st4).trans rfl)))), rfl, ?_, ?_⟩
  · show a3.lr = a.lr
    have l1 : m.lr = ((a3.lr.toNat : Nat) : Int) := r3.lr
    have l0 := hR.lr
    apply BitVec.eq_of_toNat_eq
    omega
  · exact r3.pcit _ _

theorem irq_to_call_run (base : W) (c1 c2 u1 u2 : Nat) {a : Arm.St} {m : M}
    (hR : R 0 (fun _ => 0) a m) (hit : a.it = 0#8) (hpc : a.pc = base + 0#32)
    (hsp : 128 ≤ a.sp.toNat) :
    ∃ a', run (codeAt base (irqCode c1 c2 u1 u2)) 13 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = base + 38#32 ∧ R 0 (fun _ => 0) a' (irq_to_call m) := by
  have q0 : a.pc = base + 0#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_cpsid_i r0 t0
  have st1 : step (codeAt base (irqCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact irq_f0 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 2#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  obtain ⟨a2, e2, p2, t2, s2, r2⟩ := sim_push_a r1 t1 (by omega) (fun _ _ _ => rfl) _
  have st2 : step (codeAt base (irqCode c1 c2 u1 u2)) a1 = some a2 :=
    step_of (by rw [q1]; exact irq_f2 base c1 c2 u1 u2) e2
  have q2 : a2.pc = base + 6#32 := by rw [p2, q1]; pcn
  obtain ⟨a3, e3, p3, t3, s3, r3⟩ := sim_tst_lr r2 t2 (mask := 16) (by decide)
  have st3 : step (codeAt base (irqCode c1 c2 u1 u2)) a2 = some a3 :=
    step_of (by rw [q2]; exact irq_f6 base c1 c2 u1 u2) e3
  have q3 : a3.pc = base + 10#32 := by rw [p3, q2]; pcn
  have s3' : a3.sp.toNat = a2.sp.toNat := by rw [s3]
  obtain ⟨a4, e4, p4, t4, s4, z4, r4⟩ := sim_it r3 0 8
  have st4 : step (codeAt base (irqCode c1 c2 u1 u2)) a3 = some a4 :=
    step_of (by rw [q3]; exact irq_f10 base c1 c2 u1 u2) e4
  have q4 : a4.pc = base + 12#32 := by rw [p4, q3]; pcn
  have s4' : a4.sp.toNat = a3.sp.toNat := by rw [s4]
  have t4' : a4.it = 8#8 := t4.trans (by decide)
  clear t4
  have t4 := t4'
  obtain ⟨a5, e5, p5, t5, s5, r5⟩ := sim_vpush_eq r4 t4 (by omega)
  have st5 : step (codeAt base (irqCode c1 c2 u1 u2)) a4 = some a5 :=
    step_of (by rw [q4]; exact irq_f12 base c1 c2 u1 u2) e5
  have q5 : a5.pc = base + 16#32 := by rw [p5, q4]; pcn
  have s5' : a4.sp.toNat - 64 ≤ a5.sp.toNat ∧ a5.sp.toNat ≤ a4.sp.toNat := by
    rw [s5]; split_ifs <;> omega
  clear s5
  obtain ⟨a6, e6, p6, t6, s6, r6⟩ := sim_mrs_ipsr r5 t5 (rd := 0) (by omega)
  have st6 : step (codeAt base (irqCode c1 c2 u1 u2)) a5 = some a6 :=
    step_of (by rw [q5]; exact irq_f16 base c1 c2 u1 u2) e6
  have q6 : a6.pc = base + 20#32 := by rw [p6, q5]; pcn
  have s6' : a6.sp.toNat = a5.sp.toNat := by rw [s6]
  obtain ⟨a7, e7, p7, t7, s7, r7⟩ := sim_mrs_psp r6 t6 (rd := 1) (by omega)
  have st7 : step (codeAt base (irqCode c1 c2 u1 u2)) a6 = some a7 :=
    step_of (by rw [q6]; exact irq_f20 base c1 c2 u1 u2) e7
  have q7 : a7.pc = base + 24#32 := by rw [p7, q6]; pcn
  have s7' : a7.sp.toNat = a6.sp.toNat := by rw [s7]
  obtain ⟨a8, e8, p8, t8, s8, r8⟩ := sim_mrs_control r7 t7 (rd := 2) (by omega)
  have st8 : step (codeAt base (irqCode c1 c2 u1 u2)) a7 = some a8 :=
    step_of (by rw [q7]; exact irq_f24 base c1 c2 u1 u2) e8
  have q8 : a8.pc = base + 28#32 := by rw [p8, q7]; pcn
  have s8' : a8.sp.toNat = a7.sp.toNat := by rw [s8]
  obtain ⟨a9, e9, p9, t9, s9, r9⟩ := sim_mov_r_lr r8 t8 (rd := 3) (by omega)
  have st9 : step (codeAt base (irqCode c1 c2 u1 u2)) a8 = some a9 :=
    step_of (by rw [q8]; exact irq_f28 base c1 c2 u1 u2) e9
  have q9 : a9.pc = base + 30#32 := by rw [p9, q8]; pcn
  have s9' : a9.sp.toNat = a8.sp.toNat := by rw [s9]
  obtain ⟨a10, e10, p10, t10, s10, r10⟩ := sim_mov_r_lr r9 t9 (rd := 8) (by omega)
  have st10 : step (codeAt base (irqCode c1 c2 u1 u2)) a9 = some a10 :=
    step_of (by rw [q9]; exact irq_f30 base c1 c2 u1 u2) e10
  have q10 : a10.pc = base + 32#32 := by rw [p10, q9]; pcn
  have s10' : a10.sp.toNat = a9.sp.toNat := by rw [s10]
  obtain ⟨a11, e11, p11, t11, s11, r11⟩ := sim_sub_sp r10 t10 (n := 24) (by omega) (by decide)
  have st11 : step (codeAt base (irqCode c1 c2 u1 u2)) a10 = some a11 :=
    step_of (by rw [q10]; exact irq_f32 base c1 c2 u1 u2) e11
  have q11 : a11.pc = base + 34#32 := by rw [p11, q10]; pcn
  obtain ⟨a12, e12, p12, t12, s12, r12⟩ := sim_add_r_sp r11 t11 (rd := 4) (n := 8) (by omega) (by omega)
  have st12 : step (codeAt base (irqCode c1 c2 u1 u2)) a11 = some a12 :=
    step_of (by rw [q11]; exact irq_f34 base c1 c2 u1 u2) e12
  have q12 : a12.pc = base + 36#32 := by rw [p12, q11]; pcn
  have s12' : a12.sp.toNat = a11.sp.toNat := by rw [s12]
  obtain ⟨a13, e13, p13, t13, s13, r13⟩ := sim_str_sp r12 t12 (rs := 4) (n := 0) (by omega) (by simp [kset]) (by omega) (by decide)
  have st13 : step (codeAt base (irqCode c1 c2 u1 u2)) a12 = some a13 :=
    step_of (by rw [q12]; exact irq_f36 base c1 c2 u1 u2) e13
  have q13 : a13.pc = base + 38#32 := by rw [p13, q12]; pcn
  have s13' : a13.sp.toNat = a12.sp.toNat := by rw [s13]
  refine ⟨a13, ((run_succ _ 12 _ _ st1).trans ((run_succ _ 11 _ _ st2).trans ((run_succ _ 10 _ _ st3).trans ((run_succ _ 9 _ _ st4).trans ((run_succ _ 8 _ _ st5).trans ((run_succ _ 7 _ _ st6).trans ((run_succ _ 6 _ _ st7).trans ((run_succ _ 5 _ _ st8).trans ((run_succ _ 4 _ _ st9).trans ((run_succ _ 3 _ _ st10).trans ((run_succ _ 2 _ _ st11).trans ((run_succ _ 1 _ _ st12).trans ((run_succ _ 0 _ _ st13).trans rfl))))))))))))), t13, q13, ?_⟩
  · refine r13.kcongr rfl ?_
    · intro i hi; first | (simp only [kset, kpop]; split_ifs <;> first | rfl | omega | simp_all) | (split_ifs <;> simp_all [kset, kpop])

theorem irq_dispatch_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {a : Arm.St} {m : M}
    (hR : R kc (fun _ => 0) a m) (hit : a.it = 0#8) (hpc : a.pc = base + 54#32)
    (h4 : (a.r 0).toNat % 4 = 0) (hv : (a.r 1).toNat < 8 ∧ (a.r 1).toNat / 2 % 2 = 0) :
    ∃ a', run (codeAt base (irqCode c1 c2 u1 u2)) 7 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = a.r 2 ∧ R 0 (fun _ => 0) a' (irq_dispatch m) := by
  have q0 : a.pc = base + 54#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_msr_psp r0 t0 (rs := 0) (by omega) rfl h4
  have st1 : step (codeAt base (irqCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact irq_f54 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 58#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  have w1 : m.r 1 + 0 = (((a1.r 1).toNat : Nat) : Int) := r1.r 1 (by omega)
  have w0 : m.r 1 + 0 = (((a.r 1).toNat : Nat) : Int) := hR.r 1 (by omega)
  have hv1 : (a1.r 1).toNat < 8 ∧ (a1.r 1).toNat / 2 % 2 = 0 := ⟨by omega, by omega⟩
  obtain ⟨a2, e2, p2, t2, s2, r2⟩ := sim_msr_control r1 t1 (rs := 1) (by omega) hv1
  have st2 : step (codeAt base (irqCode c1 c2 u1 u2)) a1 = some a2 :=
    step_of (by rw [q1]; exact irq_f58 base c1 c2 u1 u2) e2
  have q2 : a2.pc = base + 62#32 := by rw [p2, q1]; pcn
  have s2' : a2.sp.toNat = a1.sp.toNat := by rw [s2]
  obtain ⟨a3, e3, p3, t3, s3, r3⟩ := sim_isb r2 t2
  have st3 : step (codeAt base (irqCode c1 c2 u1 u2)) a2 = some a3 :=
    step_of (by rw [q2]; exact irq_f62 base c1 c2 u1 u2) e3
  have q3 : a3.pc = base + 66#32 := by rw [p3, q2]; pcn
  have s3' : a3.sp.toNat = a2.sp.toNat := by rw [s3]
  obtain ⟨a4, e4, p4, t4, s4, r4⟩ := sim_dsb r3 t3
  have st4 : step (codeAt base (irqCode c1 c2 u1 u2)) a3 = some a4 :=
    step_of (by rw [q3]; exact irq_f66 base c1 c2 u1 u2) e4
  have q4 : a4.pc = base + 70#32 := by rw [p4, q3]; pcn
  have s4' : a4.sp.toNat = a3.sp.toNat := by rw [s4]
  obtain ⟨a5, e5, p5, t5, s5, r5⟩ := sim_cpsid_f r4 t4
  have st5 : step (codeAt base (irqCode c1 c2 u1 u2)) a4 = some a5 :=
    step_of (by rw [q4]; exact irq_f70 base c1 c2 u1 u2) e5
  have q5 : a5.pc = base + 72#32 := by rw [p5, q4]; pcn
  have s5' : a5.sp.toNat = a4.sp.toNat := by rw [s5]
  obtain ⟨a6, e6, p6, t6, s6, r6⟩ := sim_cpsie_i r5 t5
  have st6 : step (codeAt base (irqCode c1 c2 u1 u2)) a5 = some a6 :=
    step_of (by rw [q5]; exact irq_f72 base c1 c2 u1 u2) e6
  have q6 : a6.pc = base + 74#32 := by rw [p6, q5]; pcn
  have s6' : a6.sp.toNat = a5.sp.toNat := by rw [s6]
  have st7 : step (codeAt base (irqCode c1 c2 u1 u2)) a6 = some _ :=
    step_of (by rw [q6]; exact irq_f74 base c1 c2 u1 u2) (exec_bx_r t6 (by omega))
  refine ⟨_, ((run_succ _ 6 _ _ st1).trans ((run_succ _ 5 _ _ st2).trans ((run_succ _ 4 _ _ st3).trans ((run_succ _ 3 _ _ st4).trans ((run_succ _ 2 _ _ st5).trans ((run_succ _ 1 _ _ st6).trans ((run_succ _ 0 _ _ st7).trans rfl))))))), rfl, ?_, ?_⟩
  · show a6.r 2 = a.r 2
    have l1 : m.r 2 + _ = (((a6.r 2).toNat : Nat) : Int) := r6.r 2 (by omega)
    have l0 := hR.r 2 (by omega)
    apply BitVec.eq_of_toNat_eq
    omega
  · exact r6.pcit _ _

theorem irq_restore_run (base : W) (c1 c2 u1 u2 : Nat) {a : Arm.St} {m : M}
    (hR : R 0 (fun _ => 0) a m) (hit : a.it = 0#8) (hpc : a.pc = base + 76#32)
    (hhi : a.sp.toNat + 104 < 4294967296) :
    ∃ a', run (codeAt base (irqCode c1 c2 u1 u2)) 7 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = (if a'.z = false then base + 108#32 else base + 98#32) ∧
      R 0 (fun _ => 0) a' (irq_restore m) := by
  have q0 : a.pc = base + 76#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_mov r0 t0 (rd := 12) (rs := 3) (by omega) (by omega)
  have st1 : step (codeAt base (irqCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact irq_f76 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 78#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  obtain ⟨a2, e2, p2, t2, s2, r2⟩ := sim_tst_r r1 t1 (rn := 8) (mask := 16) (by omega) (by simp [kset]) (by decide)
  have st2 : step (codeAt base (irqCode c1 c2 u1 u2)) a1 = some a2 :=
    step_of (by rw [q1]; exact irq_f78 base c1 c2 u1 u2) e2
  have q2 : a2.pc = base + 82#32 := by rw [p2, q1]; pcn
  have s2' : a2.sp.toNat = a1.sp.toNat := by rw [s2]
  obtain ⟨a3, e3, p3, t3, s3, z3, r3⟩ := sim_it r2 0 8
  have st3 : step (codeAt base (irqCode c1 c2 u1 u2)) a2 = some a3 :=
    step_of (by rw [q2]; exact irq_f82 base c1 c2 u1 u2) e3
  have q3 : a3.pc = base + 84#32 := by rw [p3, q2]; pcn
  have s3' : a3.sp.toNat = a2.sp.toNat := by rw [s3]
  have t3' : a3.it = 8#8 := t3.trans (by decide)
  clear t3
  have t3 := t3'
  obtain ⟨a4, e4, p4, t4, s4, r4⟩ := sim_vpop_eq r3 t3 (by omega)
  have st4 : step (codeAt base (irqCode c1 c2 u1 u2)) a3 = some a4 :=
    step_of (by rw [q3]; exact irq_f84 base c1 c2 u1 u2) e4
  have q4 : a4.pc = base + 88#32 := by rw [p4, q3]; pcn
  have s4' : a3.sp.toNat ≤ a4.sp.toNat ∧ a4.sp.toNat ≤ a3.sp.toNat + 64 := by
    rw [s4]; split_ifs <;> omega
  clear s4
  obtain ⟨a5, e5, p5, t5, s5, r5⟩ := sim_pop_a r4 t4 (by omega) _
  have st5 : step (codeAt base (irqCode c1 c2 u1 u2)) a4 = some a5 :=
    step_of (by rw [q4]; exact irq_f88 base c1 c2 u1 u2) e5
  have q5 : a5.pc = base + 92#32 := by rw [p5, q4]; pcn
  obtain ⟨a6, e6, p6, t6, s6, r6⟩ := sim_cmp_zero r5 t5 (rn := 12) (by omega) (by simp [kpop, kset])
  have st6 : step (codeAt base (irqCode c1 c2 u1 u2)) a5 = some a6 :=
    step_of (by rw [q5]; exact irq_f92 base c1 c2 u1 u2) e6
  have q6 : a6.pc = base + 96#32 := by rw [p6, q5]; pcn
  have s6' : a6.sp.toNat = a5.sp.toNat := by rw [s6]
  have st7 : step (codeAt base (irqCode c1 c2 u1 u2)) a6 = some _ :=
    step_of (by rw [q6]; exact irq_f96 base c1 c2 u1 u2) (exec_bne t6 _)
  refine ⟨_, ((run_succ _ 6 _ _ st1).trans ((run_succ _ 5 _ _ st2).trans ((run_succ _ 4 _ _ st3).trans ((run_succ _ 3 _ _ st4).trans ((run_succ _ 2 _ _ st5).trans ((run_succ _ 1 _ _ st6).trans ((run_succ _ 0 _ _ st7).trans rfl))))))), t6, ?_, ?_⟩
  · show (if a6.z = false then a6.pc + 4#32 + _ else a6.pc + 2#32) = _
    rw [q6]; pcn
  · refine (r6.kcongr rfl ?_).pc _
    · intro i hi; first | (simp only [kset, kpop]; split_ifs <;> first | rfl | omega | simp) | (split_ifs <;> simp [kset, kpop])

theorem irq_to_unwind_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}
    (hR : R kc kr a m) (hit : a.it = 0#8) (hpc : a.pc = base + 108#32)
    (hu : IsBw u1 u2) :
    ∃ a', run (codeAt base (irqCode c1 c2 u1 u2)) 2 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = base + (114#32 + brOff u1 u2) ∧ R kc (kset kr 0 (kr 12)) a' (irq_to_unwind m) := by
  have q0 : a.pc = base + 108#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_mov r0 t0 (rd := 0) (rs := 12) (by omega) (by omega)
  have st1 : step (codeAt base (irqCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact irq_f108 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 110#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  have st2 : step (codeAt base (irqCode c1 c2 u1 u2)) a1 = some _ :=
    step_of (by rw [q1]; exact irq_f110 base c1 c2 u1 u2 hu) (exec_b t1 _ _)
  refine ⟨_, ((run_succ _ 1 _ _ st1).trans ((run_succ _ 0 _ _ st2).trans rfl)), rfl, ?_, r1.pcit _ _⟩
  show a1.pc + 4#32 + _ = _
  rw [q1]; pcn

theorem unwind_entry_run (base : W) (c1 c2 : Nat) {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}
    (hR : R kc kr a m) (hit : a.it = 0#8) (hpc : a.pc = base + 0#32) :
    ∃ a', run (codeAt base (unwindCode c1 c2)) 1 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = base + 2#32 ∧ R kc kr a' (unwind_entry m) := by
  have q0 : a.pc = base + 0#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_cpsid_i r0 t0
  have st1 : step (codeAt base (unwindCode c1 c2)) a = some a1 :=
    step_of (by rw [q0]; exact unwind_f0 base c1 c2) e1
  have q1 : a1.pc = base + 2#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  exact ⟨a1, ((run_succ _ 0 _ _ st1).trans rfl), t1, q1, r1⟩

theorem unwind_to_call_run (base : W) (c1 c2 : Nat) {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}
    (hR : R kc kr a m) (hit : a.it = 0#8) (hpc : a.pc = base + 2#32)
    (hsp : 16 ≤ a.sp.toNat) :
    ∃ a', run (codeAt base (unwindCode c1 c2)) 2 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = base + 6#32 ∧ R kc (kset kr 1 0) a' (unwind_to_call m) := by
  have q0 : a.pc = base + 2#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_sub_sp r0 t0 (n := 16) (by omega) (by decide)
  have st1 : step (codeAt base (unwindCode c1 c2)) a = some a1 :=
    step_of (by rw [q0]; exact unwind_f2 base c1 c2) e1
  have q1 : a1.pc = base + 4#32 := by rw [p1, q0]; pcn
  obtain ⟨a2, e2, p2, t2, s2, r2⟩ := sim_mov_r_sp r1 t1 (rd := 1) (by omega)
  have st2 : step (codeAt base (unwindCode c1 c2)) a1 = some a2 :=
    step_of (by rw [q1]; exact unwind_f4 base c1 c2) e2
  have q2 : a2.pc = base + 6#32 := by rw [p2, q1]; pcn
  have s2' : a2.sp.toNat = a1.sp.toNat := by rw [s2]
  exact ⟨a2, ((run_succ _ 1 _ _ st1).trans ((run_succ _ 0 _ _ st2).trans rfl)), t2, q2, r2⟩

theorem unwind_after_call_run (base : W) (c1 c2 : Nat) {a : Arm.St} {m : M}
    (hR : R 0 (fun _ => 0) a m) (hit : a.it = 0#8) (hpc : a.pc = base + 10#32)
    (hhi : a.sp.toNat + 120 < 4294967296) (h4 : m.mem (m.sp + 0) % 4 = 0)
    (hv : 0 ≤ m.mem (m.sp + 4) ∧ m.mem (m.sp + 4) < 8 ∧ m.mem (m.sp + 4) / 2 % 2 = 0) :
    ∃ a', run (codeAt base (unwindCode c1 c2)) 14 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = (if a'.z = false then base + 64#32 else base + 54#32) ∧
      R 0 (fun _ => 0) a' (unwind_after_call m) := by
  have q0 : a.pc = base + 10#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_ldr_sp r0 t0 (rd := 0) (n := 0) (by omega) (by omega) (by decide)
  have st1 : step (codeAt base (unwindCode c1 c2)) a = some a1 :=
    step_of (by rw [q0]; exact unwind_f10 base c1 c2) e1
  have q1 : a1.pc = base + 12#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  obtain ⟨a2, e2, p2, t2, s2, r2⟩ := sim_ldr_sp r1 t1 (rd := 1) (n := 4) (by omega) (by omega) (by decide)
  have st2 : step (codeAt base (unwindCode c1 c2)) a1 = some a2 :=
    step_of (by rw [q1]; exact unwind_f12 base c1 c2) e2
  have q2 : a2.pc = base + 14#32 := by rw [p2, q1]; pcn
  have s2' : a2.sp.toNat = a1.sp.toNat := by rw [s2]
  obtain ⟨a3, e3, p3, t3, s3, r3⟩ := sim_ldr_sp r2 t2 (rd := 2) (n := 8) (by omega) (by omega) (by decide)
  have st3 : step (codeAt base (unwindCode c1 c2)) a2 = some a3 :=
    step_of (by rw [q2]; exact unwind_f14 base c1 c2) e3
  have q3 : a3.pc = base + 16#32 := by rw [p3, q2]; pcn
  have s3' : a3.sp.toNat = a2.sp.toNat := by rw [s3]
  obtain ⟨a4, e4, p4, t4, s4, r4⟩ := sim_ldr_sp r3 t3 (rd := 12) (n := 12) (by omega) (by omega) (by decide)
  have st4 : step (codeAt base (unwindCode c1 c2)) a3 = some a4 :=
    step_of (by rw [q3]; exact unwind_f16 base c1 c2) e4
  have q4 : a4.pc = base + 20#32 := by rw [p4, q3]; pcn
  have s4' : a4.sp.toNat = a3.sp.toNat := by rw [s4]
  obtain ⟨a5, e5, p5, t5, s5, r5⟩ := sim_add_sp r4 t4 (n := 16) (by omega) (by decide)
  have st5 : step (codeAt base (unwindCode c1 c2)) a4 = some a5 :=
    step_of (by rw [q4]; exact unwind_f20 base c1 c2) e5
  have q5 : a5.pc = base + 22#32 := by rw [p5, q4]; pcn
  obtain ⟨a6, e6, p6, t6, s6, r6⟩ := sim_tst_r r5 t5 (rn := 2) (mask := 16) (by omega) (by simp [kset]) (by decide)
  have st6 : step (codeAt base (unwindCode c1 c2)) a5 = some a6 :=
    step_of (by rw [q5]; exact unwind_f22 base c1 c2) e6
  have q6 : a6.pc = base + 26#32 := by rw [p6, q5]; pcn
  have s6' : a6.sp.toNat = a5.sp.toNat := by rw [s6]
  obtain ⟨a7, e7, p7, t7, s7, z7, r7⟩ := sim_it r6 0 8
  have st7 : step (codeAt base (unwindCode c1 c2)) a6 = some a7 :=
    step_of (by rw [q6]; exact unwind_f26 base c1 c2) e7
  have q7 : a7.pc = base + 28#32 := by rw [p7, q6]; pcn
  have s7' : a7.sp.toNat = a6.sp.toNat := by rw [s7]
  have t7' : a7.it = 8#8 := t7.trans (by decide)
  clear t7
  have t7 := t7'
  obtain ⟨a8, e8, p8, t8, s8, r8⟩ := sim_vpop_eq r7 t7 (by omega)
  have st8 : step (codeAt base (unwindCode c1 c2)) a7 = some a8 :=
    step_of (by rw [q7]; exact unwind_f28 base c1 c2) e8
  have q8 : a8.pc = base + 32#32 := by rw [p8, q7]; pcn
  have s8' : a7.sp.toNat ≤ a8.sp.toNat ∧ a8.sp.toNat ≤ a7.sp.toNat + 64 := by
    rw [s8]; split_ifs <;> omega
  clear s8
  obtain ⟨a9, e9, p9, t9, s9, r9⟩ := sim_pop_a r8 t8 (by omega) _
  have st9 : step (codeAt base (unwindCode c1 c2)) a8 = some a9 :=
    step_of (by rw [q8]; exact unwind_f32 base c1 c2) e9
  have q9 : a9.pc = base + 36#32 := by rw [p9, q8]; pcn
  have k0 : kpop (kset (kset (kset (kset (fun _ => (0 : Int)) 0 0) 1 0) 2 0) 12 0) 0 = 0 := by simp [kpop, kset]
  have k1 : kpop (kset (kset (kset (kset (fun _ => (0 : Int)) 0 0) 1 0) 2 0) 12 0) 1 = 0 := by simp [kpop, kset]
  have v0 : m.mem (m.sp + 0) + 0 = (((a9.r 0).toNat : Nat) : Int) := by
    have := r9.r 0 (by omega)
    rw [pop_vpop_r _ 0 (by omega), k0] at this
    exact this
  have v1 : m.mem (m.sp + 4) + 0 = (((a9.r 1).toNat : Nat) : Int) := by
    have := r9.r 1 (by omega)
    rw [pop_vpop_r _ 1 (by omega), k1] at this
    exact this
  obtain ⟨a10, e10, p10, t10, s10, r10⟩ := sim_msr_psp r9 t9 (rs := 0) (by omega) k0 (by omega)
  have st10 : step (codeAt base (unwindCode c1 c2)) a9 = some a10 :=
    step_of (by rw [q9]; exact unwind_f36 base c1 c2) e10
  have q10 : a10.pc = base + 40#32 := by rw [p10, q9]; pcn
  have s10' : a10.sp.toNat = a9.sp.toNat := by rw [s10]
  have w1 : m.mem (m.sp + 4) + 0 = (((a10.r 1).toNat : Nat) : Int) := by
    have x1 := r10.r 1 (by omega)
    have x0 := r9.r 1 (by omega)
    have e : (((a10.r 1).toNat : Nat) : Int) = (((a9.r 1).toNat : Nat) : Int) := by
      rw [← x1, ← x0]; rfl
    rw [e]; exact v1
  obtain ⟨a11, e11, p11, t11, s11, r11⟩ := sim_msr_control r10 t10 (rs := 1) (by omega) ⟨by omega, by omega⟩
  have st11 : step (codeAt base (unwindCode c1 c2)) a10 = some a11 :=
    step_of (by rw [q10]; exact unwind_f40 base c1 c2) e11
  have q11 : a11.pc = base + 44#32 := by rw [p11, q10]; pcn
  have s11' : a11.sp.toNat = a10.sp.toNat := by rw [s11]
  obtain ⟨a12, e12, p12, t12, s12, r12⟩ := sim_isb r11 t11
  have st12 : step (codeAt base (unwindCode c1 c2)) a11 = some a12 :=
    step_of (by rw [q11]; exact unwind_f44 base c1 c2) e12
  have q12 : a12.pc = base + 48#32 := by rw [p12, q11]; pcn
  have s12' : a12.sp.toNat = a11.sp.toNat := by rw [s12]
  obtain ⟨a13, e13, p13, t13, s13, r13⟩ := sim_cmp_zero r12 t12 (rn := 12) (by omega) (by simp [kpop, kset])
  have st13 : step (codeAt base (unwindCode c1 c2)) a12 = some a13 :=
    step_of (by rw [q12]; exact unwind_f48 base c1 c2) e13
  have q13 : a13.pc = base + 52#32 := by rw [p13, q12]; pcn
  have s13' : a13.sp.toNat = a12.sp.toNat := by rw [s13]
  have st14 : step (codeAt base (unwindCode c1 c2)) a13 = some _ :=
    step_of (by rw [q13]; exact unwind_f52 base c1 c2) (exec_bne t13 _)
  refine ⟨_, ((run_succ _ 13 _ _ st1).trans ((run_succ _ 12 _ _ st2).trans ((run_succ _ 11 _ _ st3).trans ((run_succ _ 10 _ _ st4).trans ((run_succ _ 9 _ _ st5).trans ((run_succ _ 8 _ _ st6).trans ((run_succ _ 7 _ _ st7).trans ((run_succ _ 6 _ _ st8).trans ((run_succ _ 5 _ _ st9).trans ((run_succ _ 4 _ _ st10).trans ((run_succ _ 3 _ _ st11).trans ((run_succ _ 2 _ _ st12).trans ((run_succ _ 1 _ _ st13).trans ((run_succ _ 0 _ _ st14).trans rfl)))))))))))))), t13, ?_, ?_⟩
  · show (if a13.z = false then a13.pc + 4#32 + _ else a13.pc + 2#32) = _
    rw [q13]; pcn
  · refine (r13.kcongr k1 ?_).pc _
    intro i hi; first | (simp only [kset, kpop]; split_ifs <;> first | rfl | omega | simp) | (split_ifs <;> simp [kset, kpop])

theorem unwind_exit_run (base : W) (c1 c2 : Nat) {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}
    (hR : R kc kr a m) (hit : a.it = 0#8) (hpc : a.pc = base + 54#32) :
    ∃ a', run (codeAt base (unwindCode c1 c2)) 4 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = a.r 2 ∧ R kc kr a' (unwind_exit m) := by
  have q0 : a.pc = base + 54#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_dsb r0 t0
  have st1 : step (codeAt base (unwindCode c1 c2)) a = some a1 :=
    step_of (by rw [q0]; exact unwind_f54 base c1 c2) e1
  have q1 : a1.pc = base + 58#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  obtain ⟨a2, e2, p2, t2, s2, r2⟩ := sim_cpsid_f r1 t1
  have st2 : step (codeAt base (unwindCode c1 c2)) a1 = some a2 :=
    step_of (by rw [q1]; exact unwind_f58 base c1 c2) e2
  have q2 : a2.pc = base + 60#32 := by rw [p2, q1]; pcn
  have s2' : a2.sp.toNat = a1.sp.toNat := by rw [s2]
  obtain ⟨a3, e3, p3, t3, s3, r3⟩ := sim_cpsie_i r2 t2
  have st3 : step (codeAt base (unwindCode c1 c2)) a2 = some a3 :=
    step_of (by rw [q2]; exact unwind_f60 base c1 c2) e3
  have q3 : a3.pc = base + 62#32 := by rw [p3, q2]; pcn
  have s3' : a3.sp.toNat = a2.sp.toNat := by rw [s3]
  have st4 : step (codeAt base (unwindCode c1 c2)) a3 = some _ :=
    step_of (by rw [q3]; exact unwind_f62 base c1 c2) (exec_bx_r t3 (by omega))
  refine ⟨_, ((run_succ _ 3 _ _ st1).trans ((run_succ _ 2 _ _ st2).trans ((run_succ _ 1 _ _ st3).trans ((run_succ _ 0 _ _ st4).trans rfl)))), rfl, ?_, r3.pcit _ _⟩
  · show a3.r 2 = a.r 2
    have l1 : m.r 2 + _ = (((a3.r 2).toNat : Nat) : Int) := r3.r 2 (by omega)
    have l0 := hR.r 2 (by omega)
    apply BitVec.eq_of_toNat_eq
    omega

theorem unwind_again_run (base : W) (c1 c2 : Nat) {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}
    (hR : R kc kr a m) (hit : a.it = 0#8) (hpc : a.pc = base + 64#32) :
    ∃ a', run (codeAt base (unwindCode c1 c2)) 2 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = base + 2#32 ∧ R kc (kset kr 0 (kr 12)) a' (unwind_again m) := by
  have q0 : a.pc = base + 64#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_mov r0 t0 (rd := 0) (rs := 12) (by omega) (by omega)
  have st1 : step (codeAt base (unwindCode c1 c2)) a = some a1 :=
    step_of (by rw [q0]; exact unwind_f64 base c1 c2) e1
  have q1 : a1.pc = base + 66#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  have st2 : step (codeAt base (unwindCode c1 c2)) a1 = some _ :=
    step_of (by rw [q1]; exact unwind_f66 base c1 c2) (exec_b t1 _ _)
  refine ⟨_, ((run_succ _ 1 _ _ st1).trans ((run_succ _ 0 _ _ st2).trans rfl)), rfl, ?_, r1.pcit _ _⟩
  show a1.pc + 4#32 + _ = _
  rw [q1]; pcn

theorem svc_to_call_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {a : Arm.St} {m : M}
    (hR : R kc (fun _ => 0) a m) (hit : a.it = 0#8) (hpc : a.pc = base + 0#32)
    (hsp : 8 ≤ a.sp.toNat) :
    ∃ a', run (codeAt base (svcCode c1 c2 u1 u2)) 6 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = base + 18#32 ∧ R kc (fun _ => 0) a' (svc_to_call m) := by
  have q0 : a.pc = base + 0#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_tst_lr r0 t0 (mask := 4) (by decide)
  have st1 : step (codeAt base (svcCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact svc_f0 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 4#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  obtain ⟨a2, e2, p2, t2, s2, z2, r2⟩ := sim_it r1 0 12
  have st2 : step (codeAt base (svcCode c1 c2 u1 u2)) a1 = some a2 :=
    step_of (by rw [q1]; exact svc_f4 base c1 c2 u1 u2) e2
  have q2 : a2.pc = base + 6#32 := by rw [p2, q1]; pcn
  have s2' : a2.sp.toNat = a1.sp.toNat := by rw [s2]
  have t2' : a2.it = 12#8 := t2.trans (by decide)
  clear t2
  have t2 := t2'
  obtain ⟨a3, e3, p3, t3, s3, z3, r3⟩ := sim_mrs_msp_eq r2 t2 (rd := 0) (by omega)
  have st3 : step (codeAt base (svcCode c1 c2 u1 u2)) a2 = some a3 :=
    step_of (by rw [q2]; exact svc_f6 base c1 c2 u1 u2) e3
  have q3 : a3.pc = base + 10#32 := by rw [p3, q2]; pcn
  have s3' : a3.sp.toNat = a2.sp.toNat := by rw [s3]
  obtain ⟨a4, e4, p4, t4, s4, z4, r4⟩ := sim_mrs_psp_ne r3 t3 (by decide) (by decide) (by decide : itAdvance 24#8 = 0#8) (rd := 0) (by omega)
  have st4 : step (codeAt base (svcCode c1 c2 u1 u2)) a3 = some a4 :=
    step_of (by rw [q3]; exact svc_f10 base c1 c2 u1 u2) e4
  have q4 : a4.pc = base + 14#32 := by rw [p4, q3]; pcn
  have s4' : a4.sp.toNat = a3.sp.toNat := by rw [s4]
  obtain ⟨a5, e5, p5, t5, s5, r5⟩ := sim_mov_r_lr r4 t4 (rd := 1) (by omega)
  have st5 : step (codeAt base (svcCode c1 c2 u1 u2)) a4 = some a5 :=
    step_of (by rw [q4]; exact svc_f14 base c1 c2 u1 u2) e5
  have q5 : a5.pc = base + 16#32 := by rw [p5, q4]; pcn
  have s5' : a5.sp.toNat = a4.sp.toNat := by rw [s5]
  obtain ⟨a6, e6, p6, t6, s6, r6⟩ := sim_push_b r5 t5 (by omega) (by simp only [kset]; split_ifs <;> rfl) _
  have st6 : step (codeAt base (svcCode c1 c2 u1 u2)) a5 = some a6 :=
    step_of (by rw [q5]; exact svc_f16 base c1 c2 u1 u2) e6
  have q6 : a6.pc = base + 18#32 := by rw [p6, q5]; pcn
  refine ⟨a6, ((run_succ _ 5 _ _ st1).trans ((run_succ _ 4 _ _ st2).trans ((run_succ _ 3 _ _ st3).trans ((run_succ _ 2 _ _ st4).trans ((run_succ _ 1 _ _ st5).trans ((run_succ _ 0 _ _ st6).trans rfl)))))), t6, q6, r6.kcongr rfl ?_⟩
  intro i hi; first | (simp only [kset, kpop]; split_ifs <;> first | rfl | omega | simp) | (split_ifs <;> simp [kset, kpop])

theorem svc_after_call_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}
    (hR : R kc kr a m) (hit : a.it = 0#8) (hpc : a.pc = base + 22#32)
    (hhi : a.sp.toNat + 8 < 4294967296) :
    ∃ a', run (codeAt base (svcCode c1 c2 u1 u2)) 2 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = (if a'.r 0 = 0#32 then base + 32#32 else base + 28#32) ∧
      R kc (kset kr 4 0) a' (kentry_after_call m) := by
  have q0 : a.pc = base + 22#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_pop_b r0 t0 (by omega) _
  have st1 : step (codeAt base (svcCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact svc_f22 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 26#32 := by rw [p1, q0]; pcn
  have st2 : step (codeAt base (svcCode c1 c2 u1 u2)) a1 = some _ :=
    step_of (by rw [q1]; exact svc_f26 base c1 c2 u1 u2) (exec_cbz t1 (by omega) _)
  refine ⟨_, ((run_succ _ 1 _ _ st1).trans ((run_succ _ 0 _ _ st2).trans rfl)), rfl, ?_, r1.pcit _ _⟩
  show (if a1.r 0 = 0#32 then a1.pc + 4#32 + _ else a1.pc + 2#32) = _
  rw [q1]; pcn

theorem svc_return_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}
    (hR : R kc kr a m) (hit : a.it = 0#8) (hpc : a.pc = base + 32#32) :
    ∃ a', run (codeAt base (svcCode c1 c2 u1 u2)) 2 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = a.lr ∧ R kc kr a' (kentry_return m) := by
  have q0 : a.pc = base + 32#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_dsb r0 t0
  have st1 : step (codeAt base (svcCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact svc_f32 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 36#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  have st2 : step (codeAt base (svcCode c1 c2 u1 u2)) a1 = some _ :=
    step_of (by rw [q1]; exact svc_f36 base c1 c2 u1 u2) (exec_bx_lr t1)
  refine ⟨_, ((run_succ _ 1 _ _ st1).trans ((run_succ _ 0 _ _ st2).trans rfl)), rfl, ?_, r1.pcit _ _⟩
  show a1.lr = a.lr
  have l1 : m.lr = ((a1.lr.toNat : Nat) : Int) := r1.lr
  have l0 := hR.lr
  apply BitVec.eq_of_toNat_eq
  omega

theorem svc_to_unwind_step (base : W) (c1 c2 u1 u2 : Nat) {a : Arm.St} (hit : a.it = 0#8)
    (hpc : a.pc = base + 28#32) (hu : IsBw u1 u2) :
    step (codeAt base (svcCode c1 c2 u1 u2)) a = some { a with pc := base + (32#32 + brOff u1 u2), it := 0#8 } := by
  have e := step_of (a := a) (by rw [hpc]; exact svc_f28 base c1 c2 u1 u2 hu) (exec_b hit _ _)
  rw [e, hpc]
  pcn

theorem mm_after_call_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}
    (hR : R kc kr a m) (hit : a.it = 0#8) (hpc : a.pc = base + 32#32)
    (hhi : a.sp.toNat + 8 < 4294967296) :
    ∃ a', run (codeAt base (mmCode c1 c2 u1 u2)) 2 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = (if a'.r 0 = 0#32 then base + 42#32 else base + 38#32) ∧
      R kc (kset kr 4 0) a' (kentry_after_call m) := by
  have q0 : a.pc = base + 32#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_pop_b r0 t0 (by omega) _
  have st1 : step (codeAt base (mmCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact mm_f32 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 36#32 := by rw [p1, q0]; pcn
  have st2 : step (codeAt base (mmCode c1 c2 u1 u2)) a1 = some _ :=
    step_of (by rw [q1]; exact mm_f36 base c1 c2 u1 u2) (exec_cbz t1 (by omega) _)
  refine ⟨_, ((run_succ _ 1 _ _ st1).trans ((run_succ _ 0 _ _ st2).trans rfl)), rfl, ?_, r1.pcit _ _⟩
  show (if a1.r 0 = 0#32 then a1.pc + 4#32 + _ else a1.pc + 2#32) = _
  rw [q1]; pcn

theorem mm_return_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}
    (hR : R kc kr a m) (hit : a.it = 0#8) (hpc : a.pc = base + 42#32) :
    ∃ a', run (codeAt base (mmCode c1 c2 u1 u2)) 2 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = a.lr ∧ R kc kr a' (kentry_return m) := by
  have q0 : a.pc = base + 42#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_dsb r0 t0
  have st1 : step (codeAt base (mmCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact mm_f42 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 46#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  have st2 : step (codeAt base (mmCode c1 c2 u1 u2)) a1 = some _ :=
    step_of (by rw [q1]; exact mm_f46 base c1 c2 u1 u2) (exec_bx_lr t1)
  refine ⟨_, ((run_succ _ 1 _ _ st1).trans ((run_succ _ 0 _ _ st2).trans rfl)), rfl, ?_, r1.pcit _ _⟩
  show a1.lr = a.lr
  have l1 : m.lr = ((a1.lr.toNat : Nat) : Int) := r1.lr
  have l0 := hR.lr
  apply BitVec.eq_of_toNat_eq
  omega

theorem mm_to_unwind_step (base : W) (c1 c2 u1 u2 : Nat) {a : Arm.St} (hit : a.it = 0#8)
    (hpc : a.pc = base + 38#32) (hu : IsBw u1 u2) :
    step (codeAt base (mmCode c1 c2 u1 u2)) a = some { a with pc := base + (42#32 + brOff u1 u2), it := 0#8 } := by
  have e := step_of (a := a) (by rw [hpc]; exact mm_f38 base c1 c2 u1 u2 hu) (exec_b hit _ _)
  rw [e, hpc]
  pcn

theorem fault_after_call_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}
    (hR : R kc kr a m) (hit : a.it = 0#8) (hpc : a.pc = base + 18#32)
    (hhi : a.sp.toNat + 8 < 4294967296) :
    ∃ a', run (codeAt base (faultCode c1 c2 u1 u2)) 2 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = (if a'.r 0 = 0#32 then base + 28#32 else base + 24#32) ∧
      R kc (kset kr 4 0) a' (kentry_after_call m) := by
  have q0 : a.pc = base + 18#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_pop_b r0 t0 (by omega) _
  have st1 : step (codeAt base (faultCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact fault_f18 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 22#32 := by rw [p1, q0]; pcn
  have st2 : step (codeAt base (faultCode c1 c2 u1 u2)) a1 = some _ :=
    step_of (by rw [q1]; exact fault_f22 base c1 c2 u1 u2) (exec_cbz t1 (by omega) _)
  refine ⟨_, ((run_succ _ 1 _ _ st1).trans ((run_succ _ 0 _ _ st2).trans rfl)), rfl, ?_, r1.pcit _ _⟩
  show (if a1.r 0 = 0#32 then a1.pc + 4#32 + _ else a1.pc + 2#32) = _
  rw [q1]; pcn

theorem fault_return_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {kr : Nat → Int} {a : Arm.St} {m : M}
    (hR : R kc kr a m) (hit : a.it = 0#8) (hpc : a.pc = base + 28#32) :
    ∃ a', run (codeAt base (faultCode c1 c2 u1 u2)) 2 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = a.lr ∧ R kc kr a' (kentry_return m) := by
  have q0 : a.pc = base + 28#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_dsb r0 t0
  have st1 : step (codeAt base (faultCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact fault_f28 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 32#32 := by rw [p1, q0]; pcn
  have s1' : a1.sp.toNat = a.sp.toNat := by rw [s1]
  have st2 : step (codeAt base (faultCode c1 c2 u1 u2)) a1 = some _ :=
    step_of (by rw [q1]; exact fault_f32 base c1 c2 u1 u2) (exec_bx_lr t1)
  refine ⟨_, ((run_succ _ 1 _ _ st1).trans ((run_succ _ 0 _ _ st2).trans rfl)), rfl, ?_, r1.pcit _ _⟩
  show a1.lr = a.lr
  have l1 : m.lr = ((a1.lr.toNat : Nat) : Int) := r1.lr
  have l0 := hR.lr
  apply BitVec.eq_of_toNat_eq
  omega

theorem fault_to_unwind_step (base : W) (c1 c2 u1 u2 : Nat) {a : Arm.St} (hit : a.it = 0#8)
    (hpc : a.pc = base + 24#32) (hu : IsBw u1 u2) :
    step (codeAt base (faultCode c1 c2 u1 u2)) a = some { a with pc := base + (28#32 + brOff u1 u2), it := 0#8 } := by
  have e := step_of (a := a) (by rw [hpc]; exact fault_f24 base c1 c2 u1 u2 hu) (exec_b hit _ _)
  rw [e, hpc]
  pcn

theorem fault_to_call_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {a : Arm.St} {m : M}
    (hR : R kc (fun _ => 0) a m) (hit : a.it = 0#8) (hpc : a.pc = base + 0#32)
    (hsp : 8 ≤ a.sp.toNat) :
    ∃ a', run (codeAt base (faultCode c1 c2 u1 u2)) 5 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = base + 14#32 ∧ R kc (fun _ => 0) a' (fault_to_call m) := by
  have q0 : a.pc = base + 0#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_push_b r0 t0 (by omega) rfl _
  have st1 : step (codeAt base (faultCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact fault_f0 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 2#32 := by rw [p1, q0]; pcn
  obtain ⟨a2, e2, p2, t2, s2, r2⟩ := sim_mrs_primask r1 t1 (rd := 0) (by omega)
  have st2 : step (codeAt base (faultCode c1 c2 u1 u2)) a1 = some a2 :=
    step_of (by rw [q1]; exact fault_f2 base c1 c2 u1 u2) e2
  have q2 : a2.pc = base + 6#32 := by rw [p2, q1]; pcn
  have s2' : a2.sp.toNat = a1.sp.toNat := by rw [s2]
  obtain ⟨a3, e3, p3, t3, s3, r3⟩ := sim_tst_lr r2 t2 (mask := 8) (by decide)
  have st3 : step (codeAt base (faultCode c1 c2 u1 u2)) a2 = some a3 :=
    step_of (by rw [q2]; exact fault_f6 base c1 c2 u1 u2) e3
  have q3 : a3.pc = base + 10#32 := by rw [p3, q2]; pcn
  have s3' : a3.sp.toNat = a2.sp.toNat := by rw [s3]
  obtain ⟨a4, e4, p4, t4, s4, z4, r4⟩ := sim_it r3 0 8
  have st4 : step (codeAt base (faultCode c1 c2 u1 u2)) a3 = some a4 :=
    step_of (by rw [q3]; exact fault_f10 base c1 c2 u1 u2) e4
  have q4 : a4.pc = base + 12#32 := by rw [p4, q3]; pcn
  have s4' : a4.sp.toNat = a3.sp.toNat := by rw [s4]
  have t4' : a4.it = 8#8 := t4.trans (by decide)
  clear t4
  have t4 := t4'
  obtain ⟨a5, e5, p5, t5, s5, r5⟩ := sim_mov_imm_eq r4 t4 (rd := 0) (n := 1) (by omega) (by decide)
  have st5 : step (codeAt base (faultCode c1 c2 u1 u2)) a4 = some a5 :=
    step_of (by rw [q4]; exact fault_f12 base c1 c2 u1 u2) e5
  have q5 : a5.pc = base + 14#32 := by rw [p5, q4]; pcn
  have s5' : a5.sp.toNat = a4.sp.toNat := by rw [s5]
  refine ⟨a5, ((run_succ _ 4 _ _ st1).trans ((run_succ _ 3 _ _ st2).trans ((run_succ _ 2 _ _ st3).trans ((run_succ _ 1 _ _ st4).trans ((run_succ _ 0 _ _ st5).trans rfl))))), t5, q5, r5.kcongr rfl ?_⟩
  intro i hi; first | (simp only [kset, kpop]; split_ifs <;> first | rfl | omega | simp) | (split_ifs <;> simp [kset, kpop])

theorem mm_to_call_run (base : W) (c1 c2 u1 u2 : Nat) {kc : Int} {a : Arm.St} {m : M}
    (hR : R kc (fun _ => 0) a m) (hit : a.it = 0#8) (hpc : a.pc = base + 0#32)
    (hsp : 8 ≤ a.sp.toNat) :
    ∃ a', run (codeAt base (mmCode c1 c2 u1 u2)) 10 a = some a' ∧ a'.it = 0#8 ∧
      a'.pc = base + 28#32 ∧ R kc (fun _ => 0) a' (mm_to_call m) := by
  have q0 : a.pc = base + 0#32 := hpc
  have t0 : a.it = 0#8 := hit
  have r0 := hR
  obtain ⟨a1, e1, p1, t1, s1, r1⟩ := sim_push_b r0 t0 (by omega) rfl _
  have st1 : step (codeAt base (mmCode c1 c2 u1 u2)) a = some a1 :=
    step_of (by rw [q0]; exact mm_f0 base c1 c2 u1 u2) e1
  have q1 : a1.pc = base + 2#32 := by rw [p1, q0]; pcn
  obtain ⟨a2, e2, p2, t2, s2, r2⟩ := sim_mrs_primask r1 t1 (rd := 0) (by omega)
  have st2 : step (codeAt base (mmCode c1 c2 u1 u2)) a1 = some a2 :=
    step_of (by rw [q1]; exact mm_f2 base c1 c2 u1 u2) e2
  have q2 : a2.pc = base + 6#32 := by rw [p2, q1]; pcn
  have s2' : a2.sp.toNat = a1.sp.toNat := by rw [s2]
  obtain ⟨a3, e3, p3, t3, s3, r3⟩ := sim_tst_lr r2 t2 (mask := 8) (by decide)
  have st3 : step (codeAt base (mmCode c1 c2 u1 u2)) a2 = some a3 :=
    step_of (by rw [q2]; exact mm_f6 base c1 c2 u1 u2) e3
  have q3 : a3.pc = base + 10#32 := by rw [p3, q2]; pcn
  have s3' : a3.sp.toNat = a2.sp.toNat := by rw [s3]
  obtain ⟨a4, e4, p4, t4, s4, z4, r4⟩ := sim_it r3 0 8
  have st4 : step (codeAt base (mmCode c1 c2 u1 u2)) a3 = some a4 :=
    step_of (by rw [q3]; exact mm_f10 base c1 c2 u1 u2) e4
  have q4 : a4.pc = base + 12#32 := by rw [p4, q3]; pcn
  have s4' : a4.sp.toNat = a3.sp.toNat := by rw [s4]
  have t4' : a4.it = 8#8 := t4.trans (by decide)
  clear t4
  have t4 := t4'
  obtain ⟨a5, e5, p5, t5, s5, r5⟩ := sim_mov_imm_eq r4 t4 (rd := 0) (n := 1) (by omega) (by decide)
  have st5 : step (codeAt base (mmCode c1 c2 u1 u2)) a4 = some a5 :=
    step_of (by rw [q4]; exact mm_f12 base c1 c2 u1 u2) e5
  have q5 : a5.pc = base + 14#32 := by rw [p5, q4]; pcn
  have s5' : a5.sp.toNat = a4.sp.toNat := by rw [s5]
  obtain ⟨a6, e6, p6, t6, s6, r6⟩ := sim_tst_lr r5 t5 (mask := 4) (by decide)
  have st6 : step (codeAt base (mmCode c1 c2 u1 u2)) a5 = some a6 :=
    step_of (by rw [q5]; exact mm_f14 base c1 c2 u1 u2) e6
  have q6 : a6.pc = base + 18#32 := by rw [p6, q5]; pcn
  have s6' : a6.sp.toNat = a5.sp.toNat := by rw [s6]
  obtain ⟨a7, e7, p7, t7, s7, z7, r7⟩ := sim_it r6 1 4
  have st7 : step (codeAt base (mmCode c1 c2 u1 u2)) a6 = some a7 :=
    step_of (by rw [q6]; exact mm_f18 base c1 c2 u1 u2) e7
  have q7 : a7.pc = base + 20#32 := by rw [p7, q6]; pcn
  have s7' : a7.sp.toNat = a6.sp.toNat := by rw [s7]
  have t7' : a7.it = 20#8 := t7.trans (by decide)
  clear t7
  have t7 := t7'
  obtain ⟨a8, e8, p8, t8, s8, z8, r8⟩ := sim_mrs_psp_ne r7 t7 (by decide) (by decide) (by decide : itAdvance 20#8 = 8#8) (rd := 1) (by omega)
  have st8 : step (codeAt base (mmCode c1 c2 u1 u2)) a7 = some a8 :=
    step_of (by rw [q7]; exact mm_f20 base c1 c2 u1 u2) e8
  have q8 : a8.pc = base + 24#32 := by rw [p8, q7]; pcn
  have s8' : a8.sp.toNat = a7.sp.toNat := by rw [s8]
  obtain ⟨a9, e9, p9, t9, s9, r9⟩ := sim_add_r_sp_eq r8 t8 (rd := 1) (n := 8) (by omega) (by omega)
  have st9 : step (codeAt base (mmCode c1 c2 u1 u2)) a8 = some a9 :=
    step_of (by rw [q8]; exact mm_f24 base c1 c2 u1 u2) e9
  have q9 : a9.pc = base + 26#32 := by rw [p9, q8]; pcn
  have s9' : a9.sp.toNat = a8.sp.toNat := by rw [s9]
  obtain ⟨a10, e10, p10, t10, s10, r10⟩ := sim_mov_r_lr r9 t9 (rd := 2) (by omega)
  have st10 : step (codeAt base (mmCode c1 c2 u1 u2)) a9 = some a10 :=
    step_of (by rw [q9]; exact mm_f26 base c1 c2 u1 u2) e10
  have q10 : a10.pc = base + 28#32 := by rw [p10, q9]; pcn
  have s10' : a10.sp.toNat = a9.sp.toNat := by rw [s10]
  refine ⟨a10, ((run_succ _ 9 _ _ st1).trans ((run_succ _ 8 _ _ st2).trans ((run_succ _ 7 _ _ st3).trans ((run_succ _ 6 _ _ st4).trans ((run_succ _ 5 _ _ st5).trans ((run_succ _ 4 _ _ st6).trans ((run_succ _ 3 _ _ st7).trans ((run_succ _ 2 _ _ st8).trans ((run_succ _ 1 _ _ st9).trans ((run_succ _ 0 _ _ st10).trans rfl)))))))))), t10, q10, r10.kcongr rfl ?_⟩
  intro i hi; first | (simp only [kset, kpop]; split_ifs <;> first | rfl | omega | simp) | (split_ifs <;> simp [kset, kpop])

/-- The `bl` of the stub: LR gets the return address with bit 0 set, and
the PC the target the two halfwords encode. -/
theorem irq_bl (base : W) (c1 c2 u1 u2 : Nat) {a : Arm.St} (hit : a.it = 0#8)
    (hpc : a.pc = base + 38#32) (hc : IsBl c1 c2) :
    step (codeAt base (irqCode c1 c2 u1 u2)) a = some { a with
      lr := (base + 42#32) ||| 1#32, pc := base + (42#32 + brOff c1 c2), it := 0#8 } := by
  have e := step_of (a := a) (by rw [hpc]; exact irq_f38 base c1 c2 u1 u2 hc) (exec_bl hit _)
  rw [e, hpc]
  pcn

/-- The `bl` of the stub: LR gets the return address with bit 0 set, and
the PC the target the two halfwords encode. -/
theorem svc_bl (base : W) (c1 c2 u1 u2 : Nat) {a : Arm.St} (hit : a.it = 0#8)
    (hpc : a.pc = base + 18#32) (hc : IsBl c1 c2) :
    step (codeAt base (svcCode c1 c2 u1 u2)) a = some { a with
      lr := (base + 22#32) ||| 1#32, pc := base + (22#32 + brOff c1 c2), it := 0#8 } := by
  have e := step_of (a := a) (by rw [hpc]; exact svc_f18 base c1 c2 u1 u2 hc) (exec_bl hit _)
  rw [e, hpc]
  pcn

/-- The `bl` of the stub: LR gets the return address with bit 0 set, and
the PC the target the two halfwords encode. -/
theorem unwind_bl (base : W) (c1 c2 : Nat) {a : Arm.St} (hit : a.it = 0#8)
    (hpc : a.pc = base + 6#32) (hc : IsBl c1 c2) :
    step (codeAt base (unwindCode c1 c2)) a = some { a with
      lr := (base + 10#32) ||| 1#32, pc := base + (10#32 + brOff c1 c2), it := 0#8 } := by
  have e := step_of (a := a) (by rw [hpc]; exact unwind_f6 base c1 c2 hc) (exec_bl hit _)
  rw [e, hpc]
  pcn

/-- The `bl` of the stub: LR gets the return address with bit 0 set, and
the PC the target the two halfwords encode. -/
theorem mm_bl (base : W) (c1 c2 u1 u2 : Nat) {a : Arm.St} (hit : a.it = 0#8)
    (hpc : a.pc = base + 28#32) (hc : IsBl c1 c2) :
    step (codeAt base (mmCode c1 c2 u1 u2)) a = some { a with
      lr := (base + 32#32) ||| 1#32, pc := base + (32#32 + brOff c1 c2), it := 0#8 } := by
  have e := step_of (a := a) (by rw [hpc]; exact mm_f28 base c1 c2 u1 u2 hc) (exec_bl hit _)
  rw [e, hpc]
  pcn

/-- The `bl` of the stub: LR gets the return address with bit 0 set, and
the PC the target the two halfwords encode. -/
theorem fault_bl (base : W) (c1 c2 u1 u2 : Nat) {a : Arm.St} (hit : a.it = 0#8)
    (hpc : a.pc = base + 14#32) (hc : IsBl c1 c2) :
    step (codeAt base (faultCode c1 c2 u1 u2)) a = some { a with
      lr := (base + 18#32) ||| 1#32, pc := base + (18#32 + brOff c1 c2), it := 0#8 } := by
  have e := step_of (a := a) (by rw [hpc]; exact fault_f14 base c1 c2 u1 u2 hc) (exec_bl hit _)
  rw [e, hpc]
  pcn

end Stub
