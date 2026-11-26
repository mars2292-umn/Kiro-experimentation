import Sched.Arm

/-! # The machine code of the entry stubs (2026-11-26)

The halfwords of each assembly stub of
`crates/rsk-kernel/src/arch/cm4/unsafe_module/stubs.rs`, as `llvm-objdump`
prints them from a linked Flight_Build (`cargo xtask stubs <binary>`
compares a binary's words with this file), and for every instruction of
every stub the theorem that the decoder of `Sched/Arm.lean` reads it from
those halfwords (`<stub>_f<offset>`).

The two halfwords of a `bl` to a Rust half (`c1`, `c2`) and of a `b.w` to
`rsk_unwind` (`u1`, `u2`) are parameters: they hold the distance to their
target, which differs from one binary to the next. The theorems hold for
any halfwords that are a `bl` (`IsBl`) and a `b.w` (`IsBw`); that the
target is the symbol the source names is `cargo xtask stubs`'s check.

`rsk_busfault_entry` and `rsk_usagefault_entry` are the same code
(`faultCode`).

The theorems were written by a script from the decoder's output and are
checked here; the script is not kept. -/

namespace Stub
open Arm

def irqCode (c1 c2 u1 u2 : Nat) : List Nat :=
  [0xB672, 0xE92D, 0x4FF8, 0xF01E, 0x0F10, 0xBF08, 0xED2D, 0x8A10, 0xF3EF, 0x8005, 0xF3EF, 0x8109, 0xF3EF, 0x8214, 0x4673, 0x46F0, 0xB086, 0xAC02, 0x9400, c1, c2, 0x9802, 0x9903, 0x9A04, 0x9B05, 0xB006, 0xB150, 0xF380, 0x8809, 0xF381, 0x8814, 0xF3BF, 0x8F6F, 0xF3BF, 0x8F4F, 0xB671, 0xB662, 0x4710, 0x469C, 0xF018, 0x0F10, 0xBF08, 0xECBD, 0x8A10, 0xE8BD, 0x4FF8, 0xF1BC, 0x0F00, 0xD104, 0xF3BF, 0x8F4F, 0xB671, 0xB662, 0x4770, 0x4660, u1, u2]

def svcCode (c1 c2 u1 u2 : Nat) : List Nat :=
  [0xF01E, 0x0F04, 0xBF0C, 0xF3EF, 0x8008, 0xF3EF, 0x8009, 0x4671, 0xB510, c1, c2, 0xE8BD, 0x4010, 0xB108, u1, u2, 0xF3BF, 0x8F4F, 0x4770]

def unwindCode (c1 c2 : Nat) : List Nat :=
  [0xB672, 0xB084, 0x4669, c1, c2, 0x9800, 0x9901, 0x9A02, 0xF8DD, 0xC00C, 0xB004, 0xF012, 0x0F10, 0xBF08, 0xECBD, 0x8A10, 0xE8BD, 0x4FF8, 0xF380, 0x8809, 0xF381, 0x8814, 0xF3BF, 0x8F6F, 0xF1BC, 0x0F00, 0xD104, 0xF3BF, 0x8F4F, 0xB671, 0xB662, 0x4710, 0x4660, 0xE7DE]

def mmCode (c1 c2 u1 u2 : Nat) : List Nat :=
  [0xB510, 0xF3EF, 0x8010, 0xF01E, 0x0F08, 0xBF08, 0x2001, 0xF01E, 0x0F04, 0xBF14, 0xF3EF, 0x8109, 0xA902, 0x4672, c1, c2, 0xE8BD, 0x4010, 0xB108, u1, u2, 0xF3BF, 0x8F4F, 0x4770]

def faultCode (c1 c2 u1 u2 : Nat) : List Nat :=
  [0xB510, 0xF3EF, 0x8010, 0xF01E, 0x0F08, 0xBF08, 0x2001, c1, c2, 0xE8BD, 0x4010, 0xB108, u1, u2, 0xF3BF, 0x8F4F, 0x4770]

/-- Proves a decode fact at a concrete offset of a concrete halfword. -/
macro "fetch_tac" k:num : tactic =>
  `(tactic| (rw [fetch_at _ _ $k (by decide) (by decide)]; rfl))

/-- `cpsid i` -/
theorem irq_f0 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 0#32) = some (.cps true true false, 2#32) := by
  fetch_tac 0

/-- `push.w {r3, r4, r5, r6, r7, r8, r9, r10, r11, lr}` -/
theorem irq_f2 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 2#32) = some (.push 20472, 4#32) := by
  fetch_tac 2

/-- `tst.w lr, #0x10` -/
theorem irq_f6 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 6#32) = some (.tst 14 0x00000010#32, 4#32) := by
  fetch_tac 6

/-- `it eq` -/
theorem irq_f10 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 10#32) = some (.it 0 8, 2#32) := by
  fetch_tac 10

/-- `vpusheq {s16, s17, s18, s19, s20, s21, s22, s23, s24, s25, s26, s27, s28, s29, s30, s31}` -/
theorem irq_f12 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 12#32) = some (.vpush 16 16, 4#32) := by
  fetch_tac 12

/-- `mrs r0, ipsr` -/
theorem irq_f16 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 16#32) = some (.mrs 0 5, 4#32) := by
  fetch_tac 16

/-- `mrs r1, psp` -/
theorem irq_f20 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 20#32) = some (.mrs 1 9, 4#32) := by
  fetch_tac 20

/-- `mrs r2, control` -/
theorem irq_f24 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 24#32) = some (.mrs 2 20, 4#32) := by
  fetch_tac 24

/-- `mov r3, lr` -/
theorem irq_f28 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 28#32) = some (.movr 3 14, 2#32) := by
  fetch_tac 28

/-- `mov r8, lr` -/
theorem irq_f30 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 30#32) = some (.movr 8 14, 2#32) := by
  fetch_tac 30

/-- `sub sp, #0x18` -/
theorem irq_f32 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 32#32) = some (.subsp 0x00000018#32, 2#32) := by
  fetch_tac 32

/-- `add r4, sp, #0x8` -/
theorem irq_f34 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 34#32) = some (.addrsp 4 0x00000008#32, 2#32) := by
  fetch_tac 34

/-- `str r4, [sp]` -/
theorem irq_f36 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 36#32) = some (.strsp 4 0x00000000#32, 2#32) := by
  fetch_tac 36

theorem irq_f38 (base : W) (c1 c2 u1 u2 : Nat) (h : IsBl c1 c2) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 38#32) = some (.bl (brOff c1 c2), 4#32) := by
  rw [fetch_at _ _ 38 (by decide) (by decide)]
  simp [irqCode, (decode_bl h).1, (decode_bl h).2]

/-- `ldr r0, [sp, #0x8]` -/
theorem irq_f42 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 42#32) = some (.ldrsp 0 0x00000008#32, 2#32) := by
  fetch_tac 42

/-- `ldr r1, [sp, #0xc]` -/
theorem irq_f44 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 44#32) = some (.ldrsp 1 0x0000000c#32, 2#32) := by
  fetch_tac 44

/-- `ldr r2, [sp, #0x10]` -/
theorem irq_f46 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 46#32) = some (.ldrsp 2 0x00000010#32, 2#32) := by
  fetch_tac 46

/-- `ldr r3, [sp, #0x14]` -/
theorem irq_f48 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 48#32) = some (.ldrsp 3 0x00000014#32, 2#32) := by
  fetch_tac 48

/-- `add sp, #0x18` -/
theorem irq_f50 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 50#32) = some (.addsp 0x00000018#32, 2#32) := by
  fetch_tac 50

/-- `cbz r0, 0x160 <rsk_irq_entry+0x4c>` -/
theorem irq_f52 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 52#32) = some (.cbz false 0 0x00000014#32, 2#32) := by
  fetch_tac 52

/-- `msr psp, r0` -/
theorem irq_f54 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 54#32) = some (.msr 9 0, 4#32) := by
  fetch_tac 54

/-- `msr control, r1` -/
theorem irq_f58 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 58#32) = some (.msr 20 1, 4#32) := by
  fetch_tac 58

/-- `isb sy` -/
theorem irq_f62 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 62#32) = some (.isb, 4#32) := by
  fetch_tac 62

/-- `dsb sy` -/
theorem irq_f66 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 66#32) = some (.dsb, 4#32) := by
  fetch_tac 66

/-- `cpsid f` -/
theorem irq_f70 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 70#32) = some (.cps true false true, 2#32) := by
  fetch_tac 70

/-- `cpsie i` -/
theorem irq_f72 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 72#32) = some (.cps false true false, 2#32) := by
  fetch_tac 72

/-- `bx r2` -/
theorem irq_f74 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 74#32) = some (.bx 2, 2#32) := by
  fetch_tac 74

/-- `mov r12, r3` -/
theorem irq_f76 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 76#32) = some (.movr 12 3, 2#32) := by
  fetch_tac 76

/-- `tst.w r8, #0x10` -/
theorem irq_f78 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 78#32) = some (.tst 8 0x00000010#32, 4#32) := by
  fetch_tac 78

/-- `it eq` -/
theorem irq_f82 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 82#32) = some (.it 0 8, 2#32) := by
  fetch_tac 82

/-- `vpopeq {s16, s17, s18, s19, s20, s21, s22, s23, s24, s25, s26, s27, s28, s29, s30, s31}` -/
theorem irq_f84 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 84#32) = some (.vpop 16 16, 4#32) := by
  fetch_tac 84

/-- `pop.w {r3, r4, r5, r6, r7, r8, r9, r10, r11, lr}` -/
theorem irq_f88 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 88#32) = some (.pop 20472, 4#32) := by
  fetch_tac 88

/-- `cmp.w r12, #0x0` -/
theorem irq_f92 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 92#32) = some (.cmpi 12 0x00000000#32, 4#32) := by
  fetch_tac 92

/-- `bne 0x180 <rsk_irq_entry+0x6c>` -/
theorem irq_f96 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 96#32) = some (.bcond 1 0x00000008#32, 2#32) := by
  fetch_tac 96

/-- `dsb sy` -/
theorem irq_f98 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 98#32) = some (.dsb, 4#32) := by
  fetch_tac 98

/-- `cpsid f` -/
theorem irq_f102 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 102#32) = some (.cps true false true, 2#32) := by
  fetch_tac 102

/-- `cpsie i` -/
theorem irq_f104 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 104#32) = some (.cps false true false, 2#32) := by
  fetch_tac 104

/-- `bx lr` -/
theorem irq_f106 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 106#32) = some (.bx 14, 2#32) := by
  fetch_tac 106

/-- `mov r0, r12` -/
theorem irq_f108 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 108#32) = some (.movr 0 12, 2#32) := by
  fetch_tac 108

theorem irq_f110 (base : W) (c1 c2 u1 u2 : Nat) (h : IsBw u1 u2) :
    fetch (codeAt base (irqCode c1 c2 u1 u2)) (base + 110#32) = some (.b (brOff u1 u2), 4#32) := by
  rw [fetch_at _ _ 110 (by decide) (by decide)]
  simp [irqCode, (decode_bw h).1, (decode_bw h).2]

/-- `tst.w lr, #0x4` -/
theorem svc_f0 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (svcCode c1 c2 u1 u2)) (base + 0#32) = some (.tst 14 0x00000004#32, 4#32) := by
  fetch_tac 0

/-- `ite eq` -/
theorem svc_f4 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (svcCode c1 c2 u1 u2)) (base + 4#32) = some (.it 0 12, 2#32) := by
  fetch_tac 4

/-- `mrseq r0, msp` -/
theorem svc_f6 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (svcCode c1 c2 u1 u2)) (base + 6#32) = some (.mrs 0 8, 4#32) := by
  fetch_tac 6

/-- `mrsne r0, psp` -/
theorem svc_f10 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (svcCode c1 c2 u1 u2)) (base + 10#32) = some (.mrs 0 9, 4#32) := by
  fetch_tac 10

/-- `mov r1, lr` -/
theorem svc_f14 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (svcCode c1 c2 u1 u2)) (base + 14#32) = some (.movr 1 14, 2#32) := by
  fetch_tac 14

/-- `push {r4, lr}` -/
theorem svc_f16 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (svcCode c1 c2 u1 u2)) (base + 16#32) = some (.push 16400, 2#32) := by
  fetch_tac 16

theorem svc_f18 (base : W) (c1 c2 u1 u2 : Nat) (h : IsBl c1 c2) :
    fetch (codeAt base (svcCode c1 c2 u1 u2)) (base + 18#32) = some (.bl (brOff c1 c2), 4#32) := by
  rw [fetch_at _ _ 18 (by decide) (by decide)]
  simp [svcCode, (decode_bl h).1, (decode_bl h).2]

/-- `pop.w {r4, lr}` -/
theorem svc_f22 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (svcCode c1 c2 u1 u2)) (base + 22#32) = some (.pop 16400, 4#32) := by
  fetch_tac 22

/-- `cbz r0, 0x1d8 <rsk_svc_entry+0x20>` -/
theorem svc_f26 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (svcCode c1 c2 u1 u2)) (base + 26#32) = some (.cbz false 0 0x00000002#32, 2#32) := by
  fetch_tac 26

theorem svc_f28 (base : W) (c1 c2 u1 u2 : Nat) (h : IsBw u1 u2) :
    fetch (codeAt base (svcCode c1 c2 u1 u2)) (base + 28#32) = some (.b (brOff u1 u2), 4#32) := by
  rw [fetch_at _ _ 28 (by decide) (by decide)]
  simp [svcCode, (decode_bw h).1, (decode_bw h).2]

/-- `dsb sy` -/
theorem svc_f32 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (svcCode c1 c2 u1 u2)) (base + 32#32) = some (.dsb, 4#32) := by
  fetch_tac 32

/-- `bx lr` -/
theorem svc_f36 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (svcCode c1 c2 u1 u2)) (base + 36#32) = some (.bx 14, 2#32) := by
  fetch_tac 36

/-- `cpsid i` -/
theorem unwind_f0 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 0#32) = some (.cps true true false, 2#32) := by
  fetch_tac 0

/-- `sub sp, #0x10` -/
theorem unwind_f2 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 2#32) = some (.subsp 0x00000010#32, 2#32) := by
  fetch_tac 2

/-- `mov r1, sp` -/
theorem unwind_f4 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 4#32) = some (.movr 1 13, 2#32) := by
  fetch_tac 4

theorem unwind_f6 (base : W) (c1 c2 : Nat) (h : IsBl c1 c2) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 6#32) = some (.bl (brOff c1 c2), 4#32) := by
  rw [fetch_at _ _ 6 (by decide) (by decide)]
  simp [unwindCode, (decode_bl h).1, (decode_bl h).2]

/-- `ldr r0, [sp]` -/
theorem unwind_f10 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 10#32) = some (.ldrsp 0 0x00000000#32, 2#32) := by
  fetch_tac 10

/-- `ldr r1, [sp, #0x4]` -/
theorem unwind_f12 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 12#32) = some (.ldrsp 1 0x00000004#32, 2#32) := by
  fetch_tac 12

/-- `ldr r2, [sp, #0x8]` -/
theorem unwind_f14 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 14#32) = some (.ldrsp 2 0x00000008#32, 2#32) := by
  fetch_tac 14

/-- `ldr.w r12, [sp, #0xc]` -/
theorem unwind_f16 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 16#32) = some (.ldrsp 12 0x0000000c#32, 4#32) := by
  fetch_tac 16

/-- `add sp, #0x10` -/
theorem unwind_f20 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 20#32) = some (.addsp 0x00000010#32, 2#32) := by
  fetch_tac 20

/-- `tst.w r2, #0x10` -/
theorem unwind_f22 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 22#32) = some (.tst 2 0x00000010#32, 4#32) := by
  fetch_tac 22

/-- `it eq` -/
theorem unwind_f26 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 26#32) = some (.it 0 8, 2#32) := by
  fetch_tac 26

/-- `vpopeq {s16, s17, s18, s19, s20, s21, s22, s23, s24, s25, s26, s27, s28, s29, s30, s31}` -/
theorem unwind_f28 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 28#32) = some (.vpop 16 16, 4#32) := by
  fetch_tac 28

/-- `pop.w {r3, r4, r5, r6, r7, r8, r9, r10, r11, lr}` -/
theorem unwind_f32 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 32#32) = some (.pop 20472, 4#32) := by
  fetch_tac 32

/-- `msr psp, r0` -/
theorem unwind_f36 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 36#32) = some (.msr 9 0, 4#32) := by
  fetch_tac 36

/-- `msr control, r1` -/
theorem unwind_f40 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 40#32) = some (.msr 20 1, 4#32) := by
  fetch_tac 40

/-- `isb sy` -/
theorem unwind_f44 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 44#32) = some (.isb, 4#32) := by
  fetch_tac 44

/-- `cmp.w r12, #0x0` -/
theorem unwind_f48 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 48#32) = some (.cmpi 12 0x00000000#32, 4#32) := by
  fetch_tac 48

/-- `bne 0x220 <rsk_unwind+0x40>` -/
theorem unwind_f52 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 52#32) = some (.bcond 1 0x00000008#32, 2#32) := by
  fetch_tac 52

/-- `dsb sy` -/
theorem unwind_f54 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 54#32) = some (.dsb, 4#32) := by
  fetch_tac 54

/-- `cpsid f` -/
theorem unwind_f58 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 58#32) = some (.cps true false true, 2#32) := by
  fetch_tac 58

/-- `cpsie i` -/
theorem unwind_f60 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 60#32) = some (.cps false true false, 2#32) := by
  fetch_tac 60

/-- `bx r2` -/
theorem unwind_f62 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 62#32) = some (.bx 2, 2#32) := by
  fetch_tac 62

/-- `mov r0, r12` -/
theorem unwind_f64 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 64#32) = some (.movr 0 12, 2#32) := by
  fetch_tac 64

/-- `b 0x1e2 <rsk_unwind+0x2>` -/
theorem unwind_f66 (base : W) (c1 c2 : Nat) :
    fetch (codeAt base (unwindCode c1 c2)) (base + 66#32) = some (.b 0xffffffbc#32, 2#32) := by
  fetch_tac 66

/-- `push {r4, lr}` -/
theorem mm_f0 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 0#32) = some (.push 16400, 2#32) := by
  fetch_tac 0

/-- `mrs r0, primask` -/
theorem mm_f2 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 2#32) = some (.mrs 0 16, 4#32) := by
  fetch_tac 2

/-- `tst.w lr, #0x8` -/
theorem mm_f6 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 6#32) = some (.tst 14 0x00000008#32, 4#32) := by
  fetch_tac 6

/-- `it eq` -/
theorem mm_f10 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 10#32) = some (.it 0 8, 2#32) := by
  fetch_tac 10

/-- `moveq r0, #0x1` -/
theorem mm_f12 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 12#32) = some (.movi 0 0x00000001#32, 2#32) := by
  fetch_tac 12

/-- `tst.w lr, #0x4` -/
theorem mm_f14 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 14#32) = some (.tst 14 0x00000004#32, 4#32) := by
  fetch_tac 14

/-- `ite ne` -/
theorem mm_f18 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 18#32) = some (.it 1 4, 2#32) := by
  fetch_tac 18

/-- `mrsne r1, psp` -/
theorem mm_f20 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 20#32) = some (.mrs 1 9, 4#32) := by
  fetch_tac 20

/-- `addeq r1, sp, #0x8` -/
theorem mm_f24 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 24#32) = some (.addrsp 1 0x00000008#32, 2#32) := by
  fetch_tac 24

/-- `mov r2, lr` -/
theorem mm_f26 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 26#32) = some (.movr 2 14, 2#32) := by
  fetch_tac 26

theorem mm_f28 (base : W) (c1 c2 u1 u2 : Nat) (h : IsBl c1 c2) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 28#32) = some (.bl (brOff c1 c2), 4#32) := by
  rw [fetch_at _ _ 28 (by decide) (by decide)]
  simp [mmCode, (decode_bl h).1, (decode_bl h).2]

/-- `pop.w {r4, lr}` -/
theorem mm_f32 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 32#32) = some (.pop 16400, 4#32) := by
  fetch_tac 32

/-- `cbz r0, 0x1b2 <rsk_memmanage_entry+0x2a>` -/
theorem mm_f36 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 36#32) = some (.cbz false 0 0x00000002#32, 2#32) := by
  fetch_tac 36

theorem mm_f38 (base : W) (c1 c2 u1 u2 : Nat) (h : IsBw u1 u2) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 38#32) = some (.b (brOff u1 u2), 4#32) := by
  rw [fetch_at _ _ 38 (by decide) (by decide)]
  simp [mmCode, (decode_bw h).1, (decode_bw h).2]

/-- `dsb sy` -/
theorem mm_f42 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 42#32) = some (.dsb, 4#32) := by
  fetch_tac 42

/-- `bx lr` -/
theorem mm_f46 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (mmCode c1 c2 u1 u2)) (base + 46#32) = some (.bx 14, 2#32) := by
  fetch_tac 46

/-- `push {r4, lr}` -/
theorem fault_f0 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (faultCode c1 c2 u1 u2)) (base + 0#32) = some (.push 16400, 2#32) := by
  fetch_tac 0

/-- `mrs r0, primask` -/
theorem fault_f2 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (faultCode c1 c2 u1 u2)) (base + 2#32) = some (.mrs 0 16, 4#32) := by
  fetch_tac 2

/-- `tst.w lr, #0x8` -/
theorem fault_f6 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (faultCode c1 c2 u1 u2)) (base + 6#32) = some (.tst 14 0x00000008#32, 4#32) := by
  fetch_tac 6

/-- `it eq` -/
theorem fault_f10 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (faultCode c1 c2 u1 u2)) (base + 10#32) = some (.it 0 8, 2#32) := by
  fetch_tac 10

/-- `moveq r0, #0x1` -/
theorem fault_f12 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (faultCode c1 c2 u1 u2)) (base + 12#32) = some (.movi 0 0x00000001#32, 2#32) := by
  fetch_tac 12

theorem fault_f14 (base : W) (c1 c2 u1 u2 : Nat) (h : IsBl c1 c2) :
    fetch (codeAt base (faultCode c1 c2 u1 u2)) (base + 14#32) = some (.bl (brOff c1 c2), 4#32) := by
  rw [fetch_at _ _ 14 (by decide) (by decide)]
  simp [faultCode, (decode_bl h).1, (decode_bl h).2]

/-- `pop.w {r4, lr}` -/
theorem fault_f18 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (faultCode c1 c2 u1 u2)) (base + 18#32) = some (.pop 16400, 4#32) := by
  fetch_tac 18

/-- `cbz r0, 0x10c <rsk_busfault_entry+0x1c>` -/
theorem fault_f22 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (faultCode c1 c2 u1 u2)) (base + 22#32) = some (.cbz false 0 0x00000002#32, 2#32) := by
  fetch_tac 22

theorem fault_f24 (base : W) (c1 c2 u1 u2 : Nat) (h : IsBw u1 u2) :
    fetch (codeAt base (faultCode c1 c2 u1 u2)) (base + 24#32) = some (.b (brOff u1 u2), 4#32) := by
  rw [fetch_at _ _ 24 (by decide) (by decide)]
  simp [faultCode, (decode_bw h).1, (decode_bw h).2]

/-- `dsb sy` -/
theorem fault_f28 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (faultCode c1 c2 u1 u2)) (base + 28#32) = some (.dsb, 4#32) := by
  fetch_tac 28

/-- `bx lr` -/
theorem fault_f32 (base : W) (c1 c2 u1 u2 : Nat) :
    fetch (codeAt base (faultCode c1 c2 u1 u2)) (base + 32#32) = some (.bx 14, 2#32) := by
  fetch_tac 32

end Stub
