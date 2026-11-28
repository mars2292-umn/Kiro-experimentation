import Mathlib.Tactic

/-! # Armv7-M: the instructions of the entry stubs (2026-11-26)

A formal semantics of the Thumb instructions the Kernel's assembly stubs
use (`crates/rsk-kernel/src/arch/cm4/unsafe_module/stubs.rs`), written
from the Armv7-M Architecture Reference Manual (DDI 0403): the encodings
of chapter A7 as a decoder from halfwords (`decode16`, `decode32`), the
operation pseudocode of each instruction as a function on a processor
state (`eff`), conditional execution and the IT block (A7.3, `execI`), and
one step of the machine from a code memory (`step`).

`Sched/StubCode.lean` holds the stubs' machine code and proves what each
halfword decodes to; `Sched/StubRefine.lean` proves that the machine
running that code does what the stub model of `Sched/StubModel.lean` says.

What is modelled, and how far:

- The stubs run in Handler mode, privileged, so SP is SP_main (B1.4.1),
  `cps` and `msr` are permitted, and CONTROL.SPSEL reads as zero and
  ignores writes (B1.4.4).
- Data memory is bytes, little-endian (`rdw`, `wrw`); an access the manual
  makes with `MemA` (the multiple loads and stores) at an address that is
  not a multiple of 4 has no result here (`none`): the hardware raises a
  UsageFault.
- The floating-point extension is present and enabled. An FP instruction
  runs `ExecuteFPCheck` (A2.5, B1.5.7): with lazy state preservation
  active (`lspact`) it has no result here; with FPCCR.ASPEN set (`aspen`)
  it sets CONTROL.FPCA. FPSCR is not in the state.
- `cpsid f` sets FAULTMASK only when the execution priority is above -1
  (B5.2.1): `hfnmi` says a HardFault or an NMI is active.
- Code is read from a memory of its own (`code`), which no store changes:
  the stubs lie in flash.
- `bx` only loads the PC. In Handler mode a value of 0xFxxxxxxx is an
  exception return (B1.5.8), which is the exception model's, not this
  file's.

What is not modelled: every instruction and every encoding the stubs do
not use (the decoders answer `none`), modified immediates other than a
plain byte, exceptions taken between instructions, timing, and the bus.
The semantics is a transcription of the manual by hand; it has not been
compared with a reference model of the architecture.

The decoders, though, were compared with LLVM's disassembler (2026-11-26):
every 16-bit halfword `decode16` accepts and a sweep of the 32-bit
encodings `decode32` accepts, each disassembled by `llvm-objdump` and read
as the same instruction with the same operands. The first comparison
found encodings accepted here that the manual gives as UNPREDICTABLE or
reserved (a `push` or `pop` of no register, the 32-bit forms of one
register, `cps` with neither flag, `it` with the condition 1111, special
registers `eff` has no case for, `dsb` and `isb` with an option other
than SY); the decoders refuse them now. The comparison was run by a
script that is not kept. -/

namespace Arm

abbrev W := BitVec 32

/-! ## Memory -/

/-- The word at `a`, least significant byte first. -/
def rdw (mem : W → BitVec 8) (a : W) : W :=
  BitVec.ofNat 32 ((mem a).toNat + 256 * (mem (a + 1)).toNat + 65536 * (mem (a + 2)).toNat +
    16777216 * (mem (a + 3)).toNat)

/-- The memory with the word `v` stored at `a`. -/
def wrw (mem : W → BitVec 8) (a v : W) : W → BitVec 8 := fun x =>
  if x = a then BitVec.ofNat 8 v.toNat
  else if x = a + 1 then BitVec.ofNat 8 (v.toNat / 256)
  else if x = a + 2 then BitVec.ofNat 8 (v.toNat / 65536)
  else if x = a + 3 then BitVec.ofNat 8 (v.toNat / 16777216)
  else mem x

/-- The memory with the words `vs` stored from `a` upward. -/
def wrws (mem : W → BitVec 8) (a : W) : List W → W → BitVec 8
  | [] => mem
  | v :: vs => wrws (wrw mem a v) (a + 4) vs

/-! ## The processor state -/

structure St where
  /-- R0 to R12. -/
  r : Nat → W
  /-- SP_main: the stack pointer of Handler mode. -/
  sp : W
  lr : W
  /-- The address of the instruction to execute. -/
  pc : W
  /-- SP_process. -/
  psp : W
  n : Bool
  z : Bool
  c : Bool
  v : Bool
  /-- ITSTATE (EPSR.IT). -/
  it : BitVec 8
  primask : Bool
  faultmask : Bool
  /-- CONTROL: nPRIV (bit 0), SPSEL (bit 1), FPCA (bit 2). -/
  control : W
  ipsr : W
  /-- S0 to S31. -/
  s : Nat → W
  mem : W → BitVec 8
  /-- A HardFault or an NMI is active: the execution priority is -1 or below. -/
  hfnmi : Bool
  /-- FPCCR.LSPACT: lazy floating-point state preservation is active. -/
  lspact : Bool
  /-- FPCCR.ASPEN. -/
  aspen : Bool

/-- R[n] as an instruction reads it: R13 is SP, R14 is LR, R15 reads as the
instruction's address plus 4. -/
def getR (s : St) (n : Nat) : W :=
  if n = 13 then s.sp else if n = 14 then s.lr else if n = 15 then s.pc + 4 else s.r n

/-- R[n] written (not the PC). Bits 1 and 0 of SP are always zero. -/
def setR (s : St) (n : Nat) (v : W) : St :=
  if n = 13 then { s with sp := v &&& ~~~3#32 }
  else if n = 14 then { s with lr := v }
  else { s with r := Function.update s.r n v }

/-! ## The instructions the stubs use -/

inductive Instr where
  /-- `cpsid`/`cpsie` with the I and F flags. -/
  | cps (disable i f : Bool)
  /-- `push` of the registers of the mask (bit n is Rn; LR is bit 14). -/
  | push (regs : Nat)
  /-- `pop` of the registers of the mask (no PC). -/
  | pop (regs : Nat)
  /-- `tst rn, #imm`, the immediate a plain byte. -/
  | tst (rn : Nat) (imm : W)
  /-- `it` with its first condition and mask. -/
  | it (firstcond mask : Nat)
  /-- `vpush` of `regs` single-precision registers from S`d`. -/
  | vpush (d regs : Nat)
  /-- `vpop`. -/
  | vpop (d regs : Nat)
  /-- `mrs rd, <special register>`. -/
  | mrs (rd sysm : Nat)
  /-- `msr <special register>, rn`. -/
  | msr (sysm rn : Nat)
  /-- `mov rd, rm` (encoding T1: no flags). -/
  | movr (rd rm : Nat)
  /-- `movs rd, #imm8` (encoding T1: flags outside an IT block only). -/
  | movi (rd : Nat) (imm : W)
  /-- `sub sp, sp, #imm`. -/
  | subsp (imm : W)
  /-- `add sp, sp, #imm`. -/
  | addsp (imm : W)
  /-- `add rd, sp, #imm`. -/
  | addrsp (rd : Nat) (imm : W)
  /-- `str rt, [sp, #imm]`. -/
  | strsp (rt : Nat) (imm : W)
  /-- `ldr rt, [sp, #imm]`. -/
  | ldrsp (rt : Nat) (imm : W)
  /-- `cmp rn, #imm`, the immediate a plain byte. -/
  | cmpi (rn : Nat) (imm : W)
  | dsb
  | isb
  /-- `b` to the instruction's address plus 4 plus `off`. -/
  | b (off : W)
  /-- `b<cond>` (encoding T1, outside an IT block). -/
  | bcond (cond : Nat) (off : W)
  /-- `cbz`/`cbnz`. -/
  | cbz (nonzero : Bool) (rn : Nat) (off : W)
  /-- `bl`. -/
  | bl (off : W)
  /-- `bx rm`. -/
  | bx (rm : Nat)
  /-- `subs sp, sp, #imm` (the flags written). The five forms from here on
  were added on 2026-11-27 for the Link_Checker's decoder
  (`Tools/LinkThumbSpec.lean`); no stub contains one. -/
  | subspS (imm : W)
  /-- `adds sp, sp, #imm`. -/
  | addspS (imm : W)
  /-- `str rt, [sp, #-imm]!`: the store of one register below SP. -/
  | strpre (rt : Nat) (imm : W)
  /-- `ldr rt, [sp], #imm`: the load of one register from SP. -/
  | ldrpost (rt : Nat) (imm : W)
  /-- `strd rt, rt2, [sp, #-imm]!`. -/
  | strdpre (rt rt2 : Nat) (imm : W)
  deriving DecidableEq, Repr

/-! ## The decoders (chapter A7, by encoding) -/

/-- The registers of a mask, lowest first: R0 to R12 and LR. -/
def maskRegs (regs : Nat) : List Nat :=
  (List.range 15).filter fun i => regs / 2 ^ i % 2 = 1

/-- The first halfword of a 32-bit instruction: bits 15 to 11 are 0b11101,
0b11110 or 0b11111 (A5.1). -/
def is32 (h : Nat) : Bool := decide (29 ≤ h / 2048)

/-- The offset of `bl` and of `b.w` (encoding T4): `SignExtend(S:I1:I2:imm10:imm11:'0')`
with `I1 = NOT(J1 EOR S)`, `I2 = NOT(J2 EOR S)` (A7.7.18, A7.7.12). -/
def brOff (h1 h2 : Nat) : W :=
  let s := h1 / 1024 % 2
  let i1 := if h2 / 8192 % 2 = s then 1 else 0
  let i2 := if h2 / 2048 % 2 = s then 1 else 0
  BitVec.ofNat 32 (s * (4294967296 - 16777216) + i1 * 8388608 + i2 * 4194304 +
    (h1 % 1024) * 4096 + (h2 % 2048) * 2)

/-- A 16-bit instruction. -/
def decode16 (h : Nat) : Option Instr :=
  -- CPS (A7.7.29? B5.2.1): 1011 0110 011 im (0) (0) I F
  -- (neither flag is UNPREDICTABLE)
  if h / 32 = 0xB66 / 2 ∧ h / 4 % 4 = 0 ∧ (h / 2 % 2 = 1 ∨ h % 2 = 1) then
    some (.cps (h / 16 % 2 = 1) (h / 2 % 2 = 1) (h % 2 = 1))
  -- PUSH T1: 1011 010 M register_list, the list not empty
  else if h / 512 = 0x5A ∧ (h / 256 % 2) * 16384 + h % 256 ≠ 0 then some (.push ((h / 256 % 2) * 16384 + h % 256))
  -- IT: 1011 1111 firstcond mask, mask not 0000; firstcond 1111, and 1110
  -- with more than one instruction, are UNPREDICTABLE
  else if h / 256 = 0xBF ∧ h % 16 ≠ 0 ∧ h / 16 % 16 ≠ 15 ∧ (h / 16 % 16 = 14 → h % 16 = 8) then some (.it (h / 16 % 16) (h % 16))
  -- MOV (register) T1: 0100 0110 D Rm Rd
  else if h / 256 = 0x46 then
    let rd := (h / 128 % 2) * 8 + h % 8
    let rm := h / 8 % 16
    if rd = 15 ∨ rd = 13 ∨ rm = 15 then none else some (.movr rd rm)
  -- SUB (SP minus immediate) T1: 1011 0000 1 imm7
  else if h / 128 = 0x161 then some (.subsp (BitVec.ofNat 32 ((h % 128) * 4)))
  -- ADD (SP plus immediate) T2: 1011 0000 0 imm7
  else if h / 128 = 0x160 then some (.addsp (BitVec.ofNat 32 ((h % 128) * 4)))
  -- ADD (SP plus immediate) T1: 1010 1 Rd imm8
  else if h / 2048 = 0x15 then some (.addrsp (h / 256 % 8) (BitVec.ofNat 32 ((h % 256) * 4)))
  -- STR (immediate) T2: 1001 0 Rt imm8
  else if h / 2048 = 0x12 then some (.strsp (h / 256 % 8) (BitVec.ofNat 32 ((h % 256) * 4)))
  -- LDR (immediate) T2: 1001 1 Rt imm8
  else if h / 2048 = 0x13 then some (.ldrsp (h / 256 % 8) (BitVec.ofNat 32 ((h % 256) * 4)))
  -- MOV (immediate) T1: 0010 0 Rd imm8
  else if h / 2048 = 0x04 then some (.movi (h / 256 % 8) (BitVec.ofNat 32 (h % 256)))
  -- CBZ, CBNZ: 1011 op 0 i 1 imm5 Rn
  else if h / 4096 = 0xB ∧ h / 1024 % 2 = 0 ∧ h / 256 % 2 = 1 then
    some (.cbz (h / 2048 % 2 = 1) (h % 8)
      (BitVec.ofNat 32 ((h / 512 % 2) * 64 + (h / 8 % 32) * 2)))
  -- B T1: 1101 cond imm8, cond not 111x
  else if h / 4096 = 0xD ∧ h / 256 % 16 < 14 then
    let imm8 := h % 256
    some (.bcond (h / 256 % 16)
      (BitVec.ofNat 32 (if imm8 < 128 then imm8 * 2 else 4294967296 - 512 + imm8 * 2)))
  -- B T2: 1110 0 imm11
  else if h / 2048 = 0x1C then
    let imm11 := h % 2048
    some (.b (BitVec.ofNat 32 (if imm11 < 1024 then imm11 * 2 else 4294967296 - 4096 + imm11 * 2)))
  -- BX: 0100 0111 0 Rm (000)
  else if h / 128 = 0x8E ∧ h % 8 = 0 then
    if h / 8 % 16 = 15 then none else some (.bx (h / 8 % 16))
  -- POP T1 without the PC: 1011 110 0 register_list, the list not empty.
  -- Added on 2026-11-27 for the Link_Checker's decoder
  -- (`Tools/LinkThumbSpec.lean`); no stub contains one.
  else if h / 512 = 0x5E ∧ h / 256 % 2 = 0 ∧ h % 256 ≠ 0 then some (.pop (h % 256))
  else none

/-- A modified immediate constant that is a plain byte: `i:imm3` is 0000
(A5.3.2), so the value is `imm8` and the carry flag is not changed. -/
def plainImm (h1 h2 : Nat) : Option W :=
  if h1 / 1024 % 2 = 0 ∧ h2 / 4096 % 8 = 0 then some (BitVec.ofNat 32 (h2 % 256)) else none

/-- `ThumbExpandImm` (A5.3.2) of the 12 bits `i:imm3:imm8`: the byte, or
the byte in two or four places, when the top two bits are 00 (UNPREDICTABLE
with a zero byte in the repeated forms); otherwise the 8 bits `'1':imm7`
rotated right by the top five bits. -/
def expandImm (imm12 : Nat) : Option W :=
  if imm12 / 1024 % 4 = 0 then
    if imm12 / 256 % 4 = 0 then some (BitVec.ofNat 32 (imm12 % 256))
    else if imm12 % 256 = 0 then none
    else if imm12 / 256 % 4 = 1 then some (BitVec.ofNat 32 (imm12 % 256 * 65536 + imm12 % 256))
    else if imm12 / 256 % 4 = 2 then
      some (BitVec.ofNat 32 (imm12 % 256 * 16777216 + imm12 % 256 * 256))
    else some (BitVec.ofNat 32 (imm12 % 256 * 16777216 + imm12 % 256 * 65536 + imm12 % 256 * 256 + imm12 % 256))
  else some ((BitVec.ofNat 32 (128 + imm12 % 128)).rotateRight (imm12 / 128 % 32))

/-- The 12 bits `i:imm3:imm8` of a data-processing instruction with an
immediate. -/
def imm12Of (h1 h2 : Nat) : Nat := (h1 / 1024 % 2) * 2048 + (h2 / 4096 % 8) * 256 + h2 % 256

/-- A 32-bit instruction, from its two halfwords. -/
def decode32 (h1 h2 : Nat) : Option Instr :=
  -- BL T1: 11110 S imm10 | 11 J1 1 J2 imm11
  if h1 / 2048 = 30 ∧ h2 / 16384 = 3 ∧ h2 / 4096 % 2 = 1 then some (.bl (brOff h1 h2))
  -- B T4: 11110 S imm10 | 10 J1 1 J2 imm11
  else if h1 / 2048 = 30 ∧ h2 / 16384 = 2 ∧ h2 / 4096 % 2 = 1 then some (.b (brOff h1 h2))
  -- PUSH T2: 1110 1001 0010 1101 | (0) M (0) register_list, two registers or more
  else if h1 = 0xE92D ∧ h2 / 32768 = 0 ∧ h2 / 8192 % 2 = 0 ∧ 2 ≤ (maskRegs h2).length then
    some (.push h2)
  -- POP T2: 1110 1000 1011 1101 | P M (0) register_list, no PC, two registers or more
  else if h1 = 0xE8BD ∧ h2 / 32768 = 0 ∧ h2 / 8192 % 2 = 0 ∧ 2 ≤ (maskRegs h2).length then
    some (.pop h2)
  -- TST (immediate) T1: 11110 i 0 0000 1 Rn | 0 imm3 1111 imm8
  else if h1 / 2048 = 30 ∧ h1 / 16 % 64 = 0x01 ∧ h2 / 32768 = 0 ∧ h2 / 256 % 16 = 15 then
    (plainImm h1 h2).bind fun imm => if h1 % 16 = 15 ∨ h1 % 16 = 13 then none else some (.tst (h1 % 16) imm)
  -- CMP (immediate) T2: 11110 i 0 1101 1 Rn | 0 imm3 1111 imm8
  else if h1 / 2048 = 30 ∧ h1 / 16 % 64 = 0x1B ∧ h2 / 32768 = 0 ∧ h2 / 256 % 16 = 15 then
    (plainImm h1 h2).bind fun imm => if h1 % 16 = 15 then none else some (.cmpi (h1 % 16) imm)
  -- VPUSH T2 (single precision): 1110 1101 0 D 10 1101 | Vd 1010 imm8
  else if h1 / 128 = 0x1DA ∧ h1 % 64 = 0x2D ∧ h2 / 256 % 16 = 10 then
    let d := (h2 / 4096) * 2 + h1 / 64 % 2
    let regs := h2 % 256
    if regs = 0 ∨ 32 < d + regs then none else some (.vpush d regs)
  -- VPOP T2 (single precision): 1110 1100 1 D 11 1101 | Vd 1010 imm8
  else if h1 / 128 = 0x1D9 ∧ h1 % 64 = 0x3D ∧ h2 / 256 % 16 = 10 then
    let d := (h2 / 4096) * 2 + h1 / 64 % 2
    let regs := h2 % 256
    if regs = 0 ∨ 32 < d + regs then none else some (.vpop d regs)
  -- MRS: 1111 0011 1110 1111 | 10 (0) 0 Rd SYSm
  else if h1 = 0xF3EF ∧ h2 / 4096 = 8 then
    -- IPSR, MSP, PSP, PRIMASK, CONTROL: the special registers `eff` has
    if h2 / 256 % 16 = 13 ∨ h2 / 256 % 16 = 15 ∨
        ¬ (h2 % 256 = 5 ∨ h2 % 256 = 8 ∨ h2 % 256 = 9 ∨ h2 % 256 = 16 ∨ h2 % 256 = 20) then none
    else some (.mrs (h2 / 256 % 16) (h2 % 256))
  -- MSR: 1111 0011 1000 Rn | 10 (0) 0 mask (0) (0) SYSm, mask 10
  else if h1 / 16 = 0xF38 ∧ h2 / 256 = 0x88 then
    -- PSP, CONTROL
    if h1 % 16 = 13 ∨ h1 % 16 = 15 ∨ ¬ (h2 % 256 = 9 ∨ h2 % 256 = 20) then none
    else some (.msr (h2 % 256) (h1 % 16))
  -- DSB: 1111 0011 1011 1111 | 1000 1111 0100 option, the option SY
  else if h1 = 0xF3BF ∧ h2 = 0x8F4F then some .dsb
  -- ISB: 1111 0011 1011 1111 | 1000 1111 0110 option, the option SY
  else if h1 = 0xF3BF ∧ h2 = 0x8F6F then some .isb
  -- LDR (immediate) T3 with Rn the SP: 1111 1000 1101 1101 | Rt imm12
  else if h1 = 0xF8DD then
    if h2 / 4096 = 15 ∨ h2 / 4096 = 13 then none
    else some (.ldrsp (h2 / 4096) (BitVec.ofNat 32 (h2 % 4096)))
  -- The four forms below were added on 2026-11-27 for the Link_Checker's
  -- decoder (`Tools/LinkThumbSpec.lean`); no stub contains one. With the S
  -- bit set (flags written) they are not decoded: `subsp` and `addsp`
  -- leave the flags.
  -- SUB (SP minus immediate) T2: 11110 i 0 1101 0 1101 | 0 imm3 1101 imm8
  else if h1 / 2048 = 30 ∧ h1 % 1024 = 0x1AD ∧ h2 / 32768 = 0 ∧ h2 / 256 % 16 = 13 then
    (expandImm (imm12Of h1 h2)).map .subsp
  -- SUB (SP minus immediate) T3: 11110 i 1 0101 0 1101 | 0 imm3 1101 imm8
  else if h1 / 2048 = 30 ∧ h1 % 1024 = 0x2AD ∧ h2 / 32768 = 0 ∧ h2 / 256 % 16 = 13 then
    some (.subsp (BitVec.ofNat 32 (imm12Of h1 h2)))
  -- ADD (SP plus immediate) T3: 11110 i 0 1000 0 1101 | 0 imm3 1101 imm8
  else if h1 / 2048 = 30 ∧ h1 % 1024 = 0x10D ∧ h2 / 32768 = 0 ∧ h2 / 256 % 16 = 13 then
    (expandImm (imm12Of h1 h2)).map .addsp
  -- ADD (SP plus immediate) T4: 11110 i 1 0000 0 1101 | 0 imm3 1101 imm8
  else if h1 / 2048 = 30 ∧ h1 % 1024 = 0x20D ∧ h2 / 32768 = 0 ∧ h2 / 256 % 16 = 13 then
    some (.addsp (BitVec.ofNat 32 (imm12Of h1 h2)))
  -- The five forms below were added later on 2026-11-27, for the same
  -- comparison; no stub contains one.
  -- SUB (SP minus immediate) T2 with S: 11110 i 0 1101 1 1101 | 0 imm3 1101 imm8
  else if h1 / 2048 = 30 ∧ h1 % 1024 = 0x1BD ∧ h2 / 32768 = 0 ∧ h2 / 256 % 16 = 13 then
    (expandImm (imm12Of h1 h2)).map .subspS
  -- ADD (SP plus immediate) T3 with S: 11110 i 0 1000 1 1101 | 0 imm3 1101 imm8
  else if h1 / 2048 = 30 ∧ h1 % 1024 = 0x11D ∧ h2 / 32768 = 0 ∧ h2 / 256 % 16 = 13 then
    (expandImm (imm12Of h1 h2)).map .addspS
  -- STR (immediate) T4 with Rn the SP, P = 1, U = 0, W = 1:
  -- 1111 1000 0100 1101 | Rt 1 1 0 1 imm8 (A7.7.161; Rt not SP or PC)
  else if h1 = 0xF84D ∧ h2 / 256 % 16 = 13 then
    if h2 / 4096 = 15 ∨ h2 / 4096 = 13 then none
    else some (.strpre (h2 / 4096) (BitVec.ofNat 32 (h2 % 256)))
  -- LDR (immediate) T4 with Rn the SP, P = 0, U = 1, W = 1:
  -- 1111 1000 0101 1101 | Rt 1 0 1 1 imm8 (A7.7.43; Rt = PC is a branch)
  else if h1 = 0xF85D ∧ h2 / 256 % 16 = 11 then
    if h2 / 4096 = 15 ∨ h2 / 4096 = 13 then none
    else some (.ldrpost (h2 / 4096) (BitVec.ofNat 32 (h2 % 256)))
  -- STRD (immediate) T1 with Rn the SP, P = 1, U = 0, W = 1:
  -- 1110 1001 0110 1101 | Rt Rt2 imm8 (A7.7.166; neither register SP or PC)
  else if h1 = 0xE96D then
    if h2 / 4096 = 15 ∨ h2 / 4096 = 13 ∨ h2 / 256 % 16 = 15 ∨ h2 / 256 % 16 = 13 then none
    else some (.strdpre (h2 / 4096) (h2 / 256 % 16) (BitVec.ofNat 32 (h2 % 256 * 4)))
  else none

/-! ## Conditions and the IT block (A7.3) -/

/-- `ConditionPassed` for the condition `cond`. -/
def condHolds (cond : Nat) (s : St) : Bool :=
  match cond with
  | 0 => s.z
  | 1 => !s.z
  | 2 => s.c
  | 3 => !s.c
  | 4 => s.n
  | 5 => !s.n
  | 6 => s.v
  | 7 => !s.v
  | 8 => s.c && !s.z
  | 9 => !s.c || s.z
  | 10 => s.n == s.v
  | 11 => s.n != s.v
  | 12 => !s.z && (s.n == s.v)
  | 13 => s.z || (s.n != s.v)
  | _ => true

/-- `CurrentCond` outside a conditional branch: bits 7 to 4 of ITSTATE in
an IT block, always otherwise. -/
def curCond (s : St) : Nat := if s.it.toNat % 16 = 0 then 14 else s.it.toNat / 16

/-- `ITAdvance`. -/
def itAdvance (it : BitVec 8) : BitVec 8 :=
  if it.toNat % 8 = 0 then 0#8
  else BitVec.ofNat 8 ((it.toNat / 32) * 32 + (it.toNat % 32 * 2) % 32)

/-! ## The operations (chapter A7 and B5.2, by instruction) -/

/-- The number of registers of the mask below R`i`: the place of R`i`'s
word among the words a `pop` loads, lowest register first. -/
def regPos (regs i : Nat) : Nat := ((maskRegs regs).filter (· < i)).length

/-- `ExecuteFPCheck` for an FP instruction in a handler: no result while
lazy state preservation is active; CONTROL.FPCA is set when FPCCR.ASPEN
is. -/
def fpCheck (s : St) : Option St :=
  if s.lspact then none
  else some (if s.aspen then { s with control := s.control ||| 4#32 } else s)

/-- `AddWithCarry(x, NOT(y), '1')`: the flags of `x - y`. -/
def subFlags (x y : W) : Bool × Bool × Bool × Bool :=
  let r := x - y
  (r.msb, decide (r = 0#32), decide (y.toNat ≤ x.toNat), decide (x.msb ≠ y.msb ∧ r.msb ≠ x.msb))

/-- The effect of an instruction whose condition passed. The PC is the
address of the instruction, `len` its length in bytes. -/
def eff (i : Instr) (len : W) (s : St) : Option St :=
  let next := s.pc + len
  match i with
  | .cps disable fi ff =>
    -- B5.2.1, privileged
    if disable then
      some { s with
        primask := if fi then true else s.primask,
        faultmask := if ff ∧ ¬ s.hfnmi then true else s.faultmask, pc := next }
    else
      some { s with
        primask := if fi then false else s.primask,
        faultmask := if ff then false else s.faultmask, pc := next }
  | .push regs =>
    -- A7.7.101: the lowest register at the lowest address
    let rs := maskRegs regs
    let a := s.sp - BitVec.ofNat 32 (4 * rs.length)
    if a.toNat % 4 ≠ 0 then none
    else some { s with mem := wrws s.mem a (rs.map (getR s)), sp := a, pc := next }
  | .pop regs =>
    -- A7.7.99
    -- each register of the mask, lowest first, is loaded from the next word
    if s.sp.toNat % 4 ≠ 0 then none
    else
      let ld := fun i => rdw s.mem (s.sp + BitVec.ofNat 32 (4 * regPos regs i))
      some { s with
        r := fun i => if i ≤ 12 ∧ regs / 2 ^ i % 2 = 1 then ld i else s.r i,
        lr := if regs / 2 ^ 14 % 2 = 1 then ld 14 else s.lr,
        sp := s.sp + BitVec.ofNat 32 (4 * (maskRegs regs).length), pc := next }
  | .tst rn imm =>
    -- A7.7.188: N and Z from the result, C unchanged for a plain byte
    let r := getR s rn &&& imm
    some { s with n := r.msb, z := decide (r = 0#32), pc := next }
  | .it _ _ => none
  | .vpush d regs =>
    -- A7.7.249
    (fpCheck s).bind fun t =>
      let a := t.sp - BitVec.ofNat 32 (4 * regs)
      if a.toNat % 4 ≠ 0 then none
      else some { t with
        mem := wrws t.mem a ((List.range regs).map fun k => t.s (d + k)), sp := a, pc := next }
  | .vpop d regs =>
    -- A7.7.248
    (fpCheck s).bind fun t =>
      if t.sp.toNat % 4 ≠ 0 then none
      else some { t with
        s := fun k => if d ≤ k ∧ k < d + regs then rdw t.mem (t.sp + BitVec.ofNat 32 (4 * (k - d)))
          else t.s k,
        sp := t.sp + BitVec.ofNat 32 (4 * regs), pc := next }
  | .mrs rd sysm =>
    -- B5.2.2, privileged, Handler mode
    let v : Option W :=
      if sysm = 5 then some s.ipsr
      else if sysm = 8 then some s.sp
      else if sysm = 9 then some s.psp
      else if sysm = 16 then some (if s.primask then 1#32 else 0#32)
      else if sysm = 20 then some (s.control &&& 7#32)
      else none
    v.map fun v => { setR s rd v with pc := next }
  | .msr sysm rn =>
    -- B5.2.3, privileged, Handler mode: SPSEL is not written
    if sysm = 9 then some { s with psp := getR s rn &&& ~~~3#32, pc := next }
    else if sysm = 20 then
      some { s with control := (s.control &&& 2#32) ||| (getR s rn &&& 5#32), pc := next }
    else none
  | .movr rd rm => some { setR s rd (getR s rm) with pc := next }
  | .movi rd imm =>
    -- A7.7.76 T1: setflags = !InITBlock()
    let t := setR s rd imm
    if s.it.toNat % 16 = 0 then some { t with n := imm.msb, z := decide (imm = 0#32), pc := next }
    else some { t with pc := next }
  | .subsp imm => some { s with sp := s.sp - imm, pc := next }
  | .addsp imm => some { s with sp := s.sp + imm, pc := next }
  | .addrsp rd imm => some { setR s rd (s.sp + imm) with pc := next }
  | .strsp rt imm => some { s with mem := wrw s.mem (s.sp + imm) (getR s rt), pc := next }
  | .ldrsp rt imm => some { setR s rt (rdw s.mem (s.sp + imm)) with pc := next }
  | .cmpi rn imm =>
    -- A7.7.27
    let f := subFlags (getR s rn) imm
    some { s with n := f.1, z := f.2.1, c := f.2.2.1, v := f.2.2.2, pc := next }
  | .dsb => some { s with pc := next }
  | .isb => some { s with pc := next }
  | .b off => some { s with pc := s.pc + 4#32 + off }
  | .bcond _ _ => none
  | .cbz nonzero rn off =>
    -- A7.7.21
    some { s with pc := if nonzero != decide (getR s rn = 0#32) then s.pc + 4#32 + off else next }
  | .bl off =>
    -- A7.7.18: LR is the return address with bit 0 set
    some { s with lr := next ||| 1#32, pc := s.pc + 4#32 + off }
  | .bx rm => some { s with pc := getR s rm }
  | .subspS imm =>
    -- A7.7.176 with S: the flags of `AddWithCarry(SP, NOT(imm), '1')`
    let f := subFlags s.sp imm
    some { s with sp := s.sp - imm, n := f.1, z := f.2.1, c := f.2.2.1, v := f.2.2.2, pc := next }
  | .addspS imm =>
    -- A7.7.5 with S: the flags of `AddWithCarry(SP, imm, '0')`
    let r := s.sp + imm
    some { s with
      sp := r, n := r.msb, z := decide (r = 0#32),
      c := decide (4294967296 ≤ s.sp.toNat + imm.toNat),
      v := decide (s.sp.msb = imm.msb ∧ r.msb ≠ s.sp.msb), pc := next }
  | .strpre rt imm =>
    -- A7.7.161, index and writeback: the word is stored at SP - imm, which
    -- becomes SP
    let a := s.sp - imm
    some { s with mem := wrw s.mem a (getR s rt), sp := a, pc := next }
  | .ldrpost rt imm =>
    -- A7.7.43, post-indexed: the word at SP is loaded, and SP + imm becomes SP
    some { setR s rt (rdw s.mem s.sp) with sp := s.sp + imm, pc := next }
  | .strdpre rt rt2 imm =>
    -- A7.7.166, index and writeback (MemA: the address a multiple of 4)
    let a := s.sp - imm
    if a.toNat % 4 ≠ 0 then none
    else some { s with
      mem := wrw (wrw s.mem a (getR s rt)) (a + 4#32) (getR s rt2), sp := a, pc := next }

/-- One instruction: `it` sets ITSTATE; a conditional branch tests its own
condition; any other instruction is executed when the current condition
passes, and ITSTATE advances either way. -/
def execI (i : Instr) (len : W) (s : St) : Option St :=
  match i with
  | .it fc mask => some { s with it := BitVec.ofNat 8 (fc * 16 + mask), pc := s.pc + len }
  | .bcond cond off =>
    if s.it.toNat % 16 ≠ 0 then none
    else some { s with pc := if condHolds cond s then s.pc + 4#32 + off else s.pc + len }
  | _ =>
    if condHolds (curCond s) s then (eff i len s).map fun t => { t with it := itAdvance s.it }
    else some { s with pc := s.pc + len, it := itAdvance s.it }

/-- The instruction at `pc` and its length: one halfword, or two when the
first is the first half of a 32-bit instruction. -/
def fetch (code : W → Option Nat) (pc : W) : Option (Instr × W) :=
  (code pc).bind fun h1 =>
    if is32 h1 then (code (pc + 2#32)).bind fun h2 => (decode32 h1 h2).map fun i => (i, 4#32)
    else (decode16 h1).map fun i => (i, 2#32)

/-- One step of the machine. -/
def step (code : W → Option Nat) (s : St) : Option St :=
  (fetch code s.pc).bind fun p => execI p.1 p.2 s

/-- `n` steps. -/
def run (code : W → Option Nat) : Nat → St → Option St
  | 0, s => some s
  | n + 1, s => (step code s).bind (run code n)

/-- The code memory of a stub: the halfwords `l` from `base`. -/
def codeAt (base : W) (l : List Nat) (a : W) : Option Nat :=
  if (a - base).toNat % 2 = 0 then l[(a - base).toNat / 2]? else none

theorem codeAt_off (base : W) (l : List Nat) (k : Nat) (hk : k < 4294967296) :
    codeAt base l (base + BitVec.ofNat 32 k) = if k % 2 = 0 then l[k / 2]? else none := by
  unfold codeAt
  have e : (base + BitVec.ofNat 32 k - base).toNat = k := by
    have : base + BitVec.ofNat 32 k - base = BitVec.ofNat 32 k := by
      rw [BitVec.add_comm, BitVec.add_sub_cancel]
    rw [this, BitVec.toNat_ofNat]
    exact Nat.mod_eq_of_lt hk
  rw [e]

theorem fetch_at (base : W) (l : List Nat) (k : Nat) (hk : k + 2 < 4294967296) (he : k % 2 = 0) :
    fetch (codeAt base l) (base + BitVec.ofNat 32 k) =
      (l[k / 2]?).bind fun h1 =>
        if is32 h1 then (l[k / 2 + 1]?).bind fun h2 => (decode32 h1 h2).map fun i => (i, 4#32)
        else (decode16 h1).map fun i => (i, 2#32) := by
  unfold fetch
  have e2 : base + BitVec.ofNat 32 k + 2#32 = base + BitVec.ofNat 32 (k + 2) := by
    rw [BitVec.add_assoc]
    congr 1
    apply BitVec.eq_of_toNat_eq
    simp
  rw [e2, codeAt_off base l k (by omega), codeAt_off base l (k + 2) hk, if_pos he,
    if_pos (by omega), show (k + 2) / 2 = k / 2 + 1 by omega]

/-- The two halfwords are a `bl`. -/
def IsBl (h1 h2 : Nat) : Prop := h1 / 2048 = 30 ∧ h2 / 16384 = 3 ∧ h2 / 4096 % 2 = 1

/-- The two halfwords are a `b.w` (encoding T4). -/
def IsBw (h1 h2 : Nat) : Prop := h1 / 2048 = 30 ∧ h2 / 16384 = 2 ∧ h2 / 4096 % 2 = 1

theorem decode_bl {h1 h2 : Nat} (h : IsBl h1 h2) :
    is32 h1 = true ∧ decode32 h1 h2 = some (.bl (brOff h1 h2)) := by
  obtain ⟨a, b, c⟩ := h
  refine ⟨by simp [is32, a], ?_⟩
  unfold decode32
  rw [if_pos ⟨a, b, c⟩]

theorem decode_bw {h1 h2 : Nat} (h : IsBw h1 h2) :
    is32 h1 = true ∧ decode32 h1 h2 = some (.b (brOff h1 h2)) := by
  obtain ⟨a, b, c⟩ := h
  refine ⟨by simp [is32, a], ?_⟩
  unfold decode32
  rw [if_neg (by rintro ⟨_, hb, _⟩; omega), if_pos ⟨a, b, c⟩]

theorem run_succ (code : W → Option Nat) (n : Nat) (s t : St) (h : step code s = some t) :
    run code (n + 1) s = run code n t := by
  simp [run, h]

theorem run_add (code : W → Option Nat) (m n : Nat) (s t : St) (h : run code m s = some t) :
    run code (m + n) s = run code n t := by
  induction m generalizing s with
  | zero => simp [run] at h; subst h; simp
  | succ k ih =>
    simp only [run] at h
    cases hs : step code s with
    | none => rw [hs] at h; simp at h
    | some u =>
      rw [hs] at h
      simp only [Option.bind_some] at h
      rw [show k + 1 + n = (k + n) + 1 by omega, run_succ code _ s u hs]
      exact ih u h

end Arm
