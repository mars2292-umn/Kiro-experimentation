import Sched.Arm

/-! # Armv7-M: exception entry and return (2026-11-26)

The exception model the stub model assumes (`hwEntry`, `exc_enter_handler`,
`exc_return_handler`), written as functions on the machine state of
`Sched/Arm.lean` from the Armv7-M Architecture Reference Manual B1.5.6
(`PushStack`, `ExceptionTaken`) and B1.5.8 (`PopStack`, `ExceptionReturn`).
`Sched/StubExc.lean` proves the model's statements of them.

- `excEntry`: the frame is stacked on the stack in use (the process stack
  when the exception is taken from Thread mode with CONTROL.SPSEL set, the
  main stack otherwise): R0 to R3, R12, LR, the return address and xPSR,
  and S0 to S15 with FPSCR when CONTROL.FPCA is set (stacking is eager:
  FPCCR.LSPEN clear). The frame is 8-byte aligned when CCR.STKALIGN is set
  or the frame is the extended one, bit 9 of the stacked xPSR recording
  the adjustment. LR gets EXC_RETURN. R0 to R3, R12 and the flags are
  UNKNOWN (taken from `unk`); IPSR is the exception number; ITSTATE is
  cleared; CONTROL.FPCA and CONTROL.SPSEL are cleared; the PC is the
  vector.
- `excReturnHandler`: the return with EXC_RETURN 0xFFFFFFF1, to Handler
  mode with a basic frame on the main stack: R0 to R3, R12, LR and the PC
  come back from the frame, the flags, ITSTATE and IPSR from the stacked
  xPSR, SP rises over the frame and its alignment word, CONTROL.FPCA is
  cleared (EXC_RETURN bit 4 set), and FAULTMASK is cleared unless the
  exception returned from is the NMI.

Not modelled: the returns to Thread mode (the stub model stops at them;
the start of a Job is the Hardware_Model's), the return with an extended
frame to a handler, the integrity checks of an exception return, the
derived exceptions of stacking and unstacking, lazy floating-point state
preservation, tail-chaining and late arrival (a tail-chained entry is the
return followed by the entry as far as the state here goes; the manual
says so, this file does not prove it). A transcription by hand, like
`Sched/Arm.lean`. -/

namespace Arm

/-- What an exception entry takes from outside the state of `St`. -/
structure ExcIn where
  /-- The exception is taken from Thread mode. -/
  thread : Bool
  /-- CCR.STKALIGN. -/
  stkalign : Bool
  /-- The return address stacked. -/
  ret : W
  /-- The xPSR stacked, before bit 9 is set to the alignment adjustment. -/
  xpsr : W
  /-- The handler's address: the vector. -/
  vec : W
  /-- The exception number. -/
  num : W
  /-- The exception is the NMI or the HardFault. -/
  fatal : Bool
  /-- FPSCR, which `St` does not hold. -/
  fpscr : W
  /-- The values of what the entry leaves UNKNOWN: R0 to R3, R12, the flags. -/
  unk : St

/-- CONTROL.FPCA. -/
def fpcaSet (a : St) : Bool := decide (a.control.toNat / 4 % 2 = 1)

/-- CONTROL.SPSEL. -/
def spselSet (a : St) : Bool := decide (a.control.toNat / 2 % 2 = 1)

/-- The frame's size in bytes: basic, or extended with the floating-point
context. -/
def frameSize (a : St) : Nat := if fpcaSet a then 104 else 32

/-- The frame is stacked on the process stack. -/
def onPsp (x : ExcIn) (a : St) : Bool := x.thread && spselSet a

/-- The stack pointer the frame is stacked from. -/
def spIn (x : ExcIn) (a : St) : W := if onPsp x a then a.psp else a.sp

/-- The frame is 8-byte aligned. -/
def forceAlign (x : ExcIn) (a : St) : Bool := fpcaSet a || x.stkalign

/-- `frameptr`: the stack pointer less the frame, bit 2 cleared when the
frame is 8-byte aligned. -/
def framePtr (x : ExcIn) (a : St) : W :=
  let p := (spIn x a - BitVec.ofNat 32 (frameSize a)).toNat
  BitVec.ofNat 32 (if forceAlign x a then p / 8 * 8 else p)

/-- `frameptralign`: the stack pointer was not 8-byte aligned and the
frame is. -/
def frameAlign (x : ExcIn) (a : St) : Bool :=
  forceAlign x a && decide ((spIn x a).toNat / 4 % 2 = 1)

/-- The words of the frame, lowest address first. -/
def frameWords (x : ExcIn) (a : St) : List W :=
  [a.r 0, a.r 1, a.r 2, a.r 3, a.r 12, a.lr, x.ret,
    BitVec.ofNat 32 (x.xpsr.toNat / 1024 * 1024 + (if frameAlign x a then 512 else 0) +
      x.xpsr.toNat % 512)] ++
  (if fpcaSet a then (List.range 16).map a.s ++ [x.fpscr] else [])

/-- EXC_RETURN (B1.5.8): bits 31 to 5 set; bit 4 clear for an extended
frame; bit 3 set for a return to Thread mode; bit 2 set for the process
stack; bits 1 and 0 are 01. -/
def excReturn (x : ExcIn) (a : St) : W :=
  BitVec.ofNat 32 (0xFFFFFFE1 + (if fpcaSet a then 0 else 16) + (if x.thread then 8 else 0) +
    (if onPsp x a then 4 else 0))

/-- The exception entry. -/
def excEntry (x : ExcIn) (a : St) : St :=
  { a with
    r := fun i => if i ≤ 3 ∨ i = 12 then x.unk.r i else a.r i,
    sp := if onPsp x a then a.sp else framePtr x a,
    psp := if onPsp x a then framePtr x a else a.psp,
    lr := excReturn x a,
    pc := BitVec.ofNat 32 (x.vec.toNat / 2 * 2),
    n := x.unk.n, z := x.unk.z, c := x.unk.c, v := x.unk.v,
    it := 0#8,
    control := BitVec.ofNat 32 (a.control.toNat % 2),
    ipsr := x.num,
    mem := wrws a.mem (framePtr x a) (frameWords x a),
    hfnmi := a.hfnmi || x.fatal }

/-- The exception return to Handler mode with a basic frame on the main
stack (EXC_RETURN 0xFFFFFFF1, in the PC after `bx`). No result for another
EXC_RETURN or an unaligned stack pointer. -/
def excReturnHandler (stkalign : Bool) (a : St) : Option St :=
  if a.pc ≠ 0xFFFFFFF1#32 ∨ a.sp.toNat % 4 ≠ 0 then none
  else
    let w := fun (k : Nat) => rdw a.mem (a.sp + BitVec.ofNat 32 (4 * k))
    let xpsr := (w 7).toNat
    some { a with
      r := fun i =>
        if i = 0 then w 0 else if i = 1 then w 1 else if i = 2 then w 2 else if i = 3 then w 3
        else if i = 12 then w 4 else a.r i,
      lr := w 5,
      pc := BitVec.ofNat 32 ((w 6).toNat / 2 * 2),
      sp := a.sp + BitVec.ofNat 32 (32 + (if stkalign ∧ xpsr / 512 % 2 = 1 then 4 else 0)),
      n := decide (xpsr / 2147483648 % 2 = 1), z := decide (xpsr / 1073741824 % 2 = 1),
      c := decide (xpsr / 536870912 % 2 = 1), v := decide (xpsr / 268435456 % 2 = 1),
      it := BitVec.ofNat 8 ((xpsr / 1024 % 64) * 4 + xpsr / 33554432 % 4),
      ipsr := BitVec.ofNat 32 (xpsr % 512),
      control := BitVec.ofNat 32 (a.control.toNat % 4),
      faultmask := if a.ipsr = 2#32 then a.faultmask else false }

end Arm
