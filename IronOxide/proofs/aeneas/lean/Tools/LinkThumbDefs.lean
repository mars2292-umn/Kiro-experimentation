import Sched.Arm
import Tools.LinkBits

/-! # The Link_Checker's Thumb decoder in arithmetic (2026-11-26)

What the Link_Checker's decoder (`crates/rsk-link/src/thumb.rs`) answers for
a halfword, or two, as functions of natural numbers (`D16`, `D32`), and what
it means for such an answer to agree with the Armv7-M semantics of
`Sched/Arm.lean` (`SpAgrees`, `TargetAgrees`). Definitions only:
`Tools/LinkThumbSpec.lean` proves the agreement, `Tools/LinkThumbProofs.lean`
that the compiled decoder computes `D16` and `D32`. Handwritten. -/

namespace LinkThumb

open Arm LinkBits

/-- What the Link_Checker reads from an instruction, the numbers as
naturals. `plain` stands for every reading that neither moves SP nor
names a target (`Constant`, `SwitchTable`, `Svc`, `Fp`, `Msr`, `Mrs`, `Cps`,
`Nop`, `Dsb`, `Other`, and a literal load). -/
inductive AEff where
  | spDown (n : Nat)
  | spUp (n : Nat)
  | spDynamic
  | spUnread
  | call (t : Nat)
  | branch (t : Nat)
  | cbranch (t : Nat)
  | ret
  | ibranch (r : Nat)
  | icall (r : Nat)
  | plain

/-- `(pc as i32 + 4 + imm) as u32`. -/
def tgt (pc : Nat) (imm : Int) : Nat := (((pc : Int) + 4 + imm) % 4294967296).toNat

/-- The offset of a 16-bit conditional branch: `SignExtend(imm8:'0')`. -/
def off9 (h : Nat) : Int := if h % 256 < 128 then 2 * (h % 256 : Nat) else 2 * (h % 256 : Nat) - 512

/-- The offset of a 16-bit branch: `SignExtend(imm11:'0')`. -/
def off12 (h : Nat) : Int :=
  if h % 2048 < 1024 then 2 * (h % 2048 : Nat) else 2 * (h % 2048 : Nat) - 4096

/-- The 25 bits `S:I1:I2:imm10:imm11:'0'` of a `bl` or `b.w`. -/
def raw25 (a b : Nat) : Nat :=
  (a / 1024 % 2) * 16777216 + (if b / 8192 % 2 = a / 1024 % 2 then 1 else 0) * 8388608 +
    (if b / 2048 % 2 = a / 1024 % 2 then 1 else 0) * 4194304 + (a % 1024) * 4096 + (b % 2048) * 2

/-- The offset of a `bl` or `b.w`: the 25 bits sign-extended. -/
def off25 (a b : Nat) : Int :=
  if raw25 a b < 16777216 then (raw25 a b : Nat) else (raw25 a b : Nat) - 33554432

/-- What `decode16_static` answers for the halfword `h` at `pc`. -/
def D16 (h pc : Nat) : AEff :=
  if h / 512 = 0x5A then .spDown (4 * (ones 16 (h % 256) + h / 256 % 2))
  else if h / 512 = 0x5E then
    (if h / 256 % 2 = 1 then .ret else .spUp (4 * ones 16 (h % 256)))
  else if h / 128 = 0x160 then .spUp (h % 128 * 4)
  else if h / 128 = 0x161 then .spDown (h % 128 * 4)
  else if h / 128 = 0x8E ∧ h % 8 = 0 then
    (if h / 8 % 16 = 14 then .ret else .ibranch (h / 8 % 16))
  else if h / 128 = 0x8F ∧ h % 8 = 0 then .icall (h / 8 % 16)
  else if h / 128 = 0x8D ∧ h % 8 = 5 then .spDynamic
  else if h / 128 = 0x89 ∧ h % 8 = 5 then .spDynamic
  else if h / 2048 = 9 then .plain
  else if h / 4096 = 0xD then
    (if h / 256 % 16 = 15 then .plain
     else if h / 256 % 16 = 14 then .plain
     else .branch (tgt pc (off9 h)))
  else if h / 2048 = 0x1C then .branch (tgt pc (off12 h))
  else if h / 32 = 0x5B3 ∧ h / 4 % 4 = 0 then .plain
  else if h / 4096 = 0xB ∧ h / 1024 % 2 = 0 ∧ h / 256 % 2 = 1 then
    .cbranch (pc + 4 + ((h / 512 % 2) * 64 + (h / 8 % 32) * 2))
  else .plain

/-- The modified immediate `i:imm3:imm8` expanded, as `thumb_expand_imm`
computes it: the byte, or the byte repeated, when the top two bits are 00;
otherwise `'1':imm7` rotated right by the top five bits, which is a shift
left by 32 minus that amount (the amount is 8 or more and the value has 8
bits). `Tools/LinkThumbSpec.lean` compares it with the manual's
`ThumbExpandImm` (`Arm.expandImm`). -/
def expand12 (n : Nat) : Nat :=
  if n / 1024 % 4 = 0 then
    (if n / 256 % 4 = 0 then n % 256
     else if n / 256 % 4 = 1 then n % 256 * 65536 + n % 256
     else if n / 256 % 4 = 2 then n % 256 * 16777216 + n % 256 * 256
     else n % 256 * 16777216 + n % 256 * 65536 + n % 256 * 256 + n % 256)
  else (128 + n % 128) * 2 ^ (32 - n / 128 % 32)

/-- The 12 bits `i:imm3:imm8` of a data-processing instruction with an
immediate. -/
def imm12 (a b : Nat) : Nat := (a / 1024 % 2) * 2048 + (b / 4096 % 8) * 256 + b % 256

/-- What `sp_unread` answers: the 32-bit instruction `a b` is of a class
that can write register 13 (as a destination, as a loaded register, or as
a base register written back). By class of A5.3: data processing with an
immediate (first halfword 11110, bit 15 of the second clear); load and
store multiple; the exclusive forms; load and store of two registers;
data processing with a shifted register; load and store of one register;
data processing with registers, multiply and divide; coprocessor load,
store and two-register transfers; one-register transfers from a
coprocessor. -/
def Unread (a b : Nat) : Prop :=
  if a / 2048 = 30 then b / 4096 < 8 ∧ b / 256 % 16 = 13
  else if a / 2048 = 29 ∧ a / 512 % 4 = 0 then
    (if a / 64 % 2 = 0 then
       (a / 32 % 2 = 1 ∧ a % 16 = 13) ∨ (a / 16 % 2 = 1 ∧ b / 8192 % 2 = 1)
     else if a / 256 % 2 = 0 ∧ a / 32 % 2 = 0 then
       ((a / 16 % 16 = 4 ∧ b / 256 % 16 = 13) ∨ (a / 16 % 16 = 12 ∧ b % 16 = 13)) ∨
         (a / 16 % 2 = 1 ∧ b / 4096 = 13)
     else (a / 32 % 2 = 1 ∧ a % 16 = 13) ∨ (a / 16 % 2 = 1 ∧ (b / 4096 = 13 ∨ b / 256 % 16 = 13)))
  else if a / 2048 = 29 ∧ a / 512 % 4 = 1 then b / 256 % 16 = 13
  else if a / 2048 = 31 ∧ a / 512 % 4 = 0 then
    (((a / 128 % 2 = 0 ∧ b / 2048 % 2 = 1) ∧ b / 256 % 2 = 1) ∧ a % 16 = 13) ∨
      (a / 16 % 2 = 1 ∧ b / 4096 = 13)
  else if a / 2048 = 31 ∧ a / 512 % 4 = 1 then
    b / 256 % 16 = 13 ∨ (a / 128 % 4 = 3 ∧ b / 4096 = 13)
  else if a / 512 % 4 = 2 then
    (a / 32 % 2 = 1 ∧ a % 16 = 13) ∨
      (((a / 256 % 2 = 0 ∧ a / 128 % 2 = 0) ∧ a / 16 % 2 = 1) ∧ (b / 4096 = 13 ∨ a % 16 = 13))
  else (a / 256 % 2 = 0 ∧ b / 16 % 2 = 1) ∧ (a / 16 % 2 = 1 ∧ b / 4096 = 13)

instance (a b : Nat) : Decidable (Unread a b) := by unfold Unread; infer_instance

/-- What `decode32_static` answers for the halfwords `a`, `b` at `pc`;
`none` where this file does not say (a 32-bit conditional branch). The
answer `spUnread` (2026-11-27) is for an instruction of a class that can
write SP (`Unread`) that no earlier case read a stack-pointer change
from, and for `mrs` to SP: the Link_Checker refuses the function. -/
def D32 (a b pc : Nat) : Option AEff :=
  if a = 0xF3BF ∧ b / 16 = 0x8F4 then some .plain
  else if (a / 16 = 0xF8D ∨ a / 16 = 0xF85) ∧ b / 4096 = 13 then some .spDynamic
  else if a / 16 = 0xF38 ∧ b / 4096 = 8 then some .plain
  else if a = 0xF3EF ∧ b / 4096 = 8 then
    (if b / 256 % 16 = 13 then some .spUnread else some .plain)
  else if a = 0xE92D then some (.spDown (4 * ones 16 b))
  else if a = 0xE8BD then
    (if b / 32768 % 2 = 1 then some .ret else some (.spUp (4 * ones 16 b)))
  else if a = 0xED2D ∨ a = 0xED6D then some (.spDown (4 * (b % 256)))
  else if a = 0xECBD ∨ a = 0xECFD then some (.spUp (4 * (b % 256)))
  else if a = 0xF84D ∧ b / 256 % 8 = 5 then some (.spDown (b % 256))
  else if a = 0xE96D then some (.spDown (4 * (b % 256)))
  else if a = 0xF85D ∧ b / 4096 = 15 then some .ret
  else if a = 0xF85D ∧ b / 256 % 16 = 11 then some (.spUp (b % 256))
  else if b / 32768 % 2 = 0 ∧
      ((a / 2048 = 30 ∧ a / 32 % 32 = 13 ∧ a % 16 = 13) ∨ (a / 2048 = 30 ∧ a % 1024 = 685)) ∧
      b / 256 % 16 = 13 then
    (if a / 2048 = 30 ∧ a % 1024 = 685 then some (.spDown (imm12 a b))
     else some (.spDown (expand12 (imm12 a b))))
  else if b / 32768 % 2 = 0 ∧
      ((a / 2048 = 30 ∧ a / 32 % 32 = 8 ∧ a % 16 = 13) ∨ (a / 2048 = 30 ∧ a % 1024 = 525)) ∧
      b / 256 % 16 = 13 then
    (if a / 2048 = 30 ∧ a % 1024 = 525 then some (.spUp (imm12 a b))
     else some (.spUp (expand12 (imm12 a b))))
  else if (a = 0xEBAD ∨ a = 0xEBBD ∨ a = 0xEB0D ∨ a = 0xEB1D) ∧ b / 256 % 16 = 13 then
    some .spDynamic
  else if b / 32768 % 2 = 0 ∧ ((a / 2048 = 30 ∧ a / 32 % 32 = 2 ∧ a % 16 = 15) ∨ (a / 2048 = 30 ∧ a / 16 % 64 = 36)) ∧
      b / 256 % 16 = 13 then some .spDynamic
  else if Unread a b then some .spUnread
  else if b / 32768 % 2 = 0 ∧ a / 2048 = 30 ∧ (a / 16 % 64 = 36 ∨ a / 16 % 64 = 44) then some .plain
  else if a = 0xF8DF ∨ a = 0xF85F then some .plain
  else if (a / 8192 = 7 ∧ a / 1024 % 4 = 3) ∧ b / 512 % 8 = 5 then some .plain
  else if a / 16 = 0xE8D ∧ b / 32 = 0x780 then some .plain
  else if a / 2048 = 30 ∧ (b / 16384 = 3 ∨ b / 16384 = 2) ∧ b / 4096 % 2 = 1 then
    (if b / 16384 % 2 = 1 then some (.call (tgt pc (off25 a b)))
     else some (.branch (tgt pc (off25 a b))))
  else if a / 2048 = 30 ∧ b / 16384 = 2 ∧ b / 4096 % 2 = 0 then
    (if a / 128 % 8 = 7 then some .plain else none)
  else some .plain

/-! ## The agreement statements -/

/-- The stack-pointer change the Link_Checker reads is the one the
operation makes: for every state in which the instruction's operation has
a result. `spDynamic` and `spUnread` claim nothing (the Link_Checker
gives such a function no bound). -/
def SpAgrees (e : AEff) (i : Instr) : Prop :=
  ∀ (len : W) (s t : St), eff i len s = some t →
    match e with
    | .spDown n => t.sp = s.sp - BitVec.ofNat 32 n
    | .spUp n => t.sp = s.sp + BitVec.ofNat 32 n
    | .spDynamic => True
    | .spUnread => True
    | _ => t.sp = s.sp

/-- The target the Link_Checker reads is the address the operation loads:
the instruction's address plus 4 plus its offset. For `bx` the reading is
a return for LR and an indirect branch through the register otherwise. -/
def TargetAgrees (e : AEff) (i : Instr) (pc : Nat) : Prop :=
  match i with
  | .bl off => ∃ t, e = .call t ∧ BitVec.ofNat 32 t = BitVec.ofNat 32 pc + 4#32 + off
  | .b off => ∃ t, e = .branch t ∧ BitVec.ofNat 32 t = BitVec.ofNat 32 pc + 4#32 + off
  | .bcond _ off => ∃ t, e = .branch t ∧ BitVec.ofNat 32 t = BitVec.ofNat 32 pc + 4#32 + off
  | .cbz _ _ off => ∃ t, e = .cbranch t ∧ BitVec.ofNat 32 t = BitVec.ofNat 32 pc + 4#32 + off
  | .bx rm => if rm = 14 then e = .ret else e = .ibranch rm
  | _ => True

end LinkThumb
