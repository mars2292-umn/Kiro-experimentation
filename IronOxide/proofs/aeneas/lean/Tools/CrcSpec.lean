/-! # CRC-32, one definition (2026-11-26)

The checksum the Kernel computes over its tables at boot (`Crc32` of
`crates/rsk-kernel/src/arch/bound.rs`) and the checksum the Config_Checker
recomputes over an image (`crc32` of `crates/rsk-confcheck/src/image_v.rs`)
are each proven in Verus against a definition restated in its own crate
(R26.1), the two the same by reading. Here there is one definition, and
both functions as compiled are proven against it: the Kernel's in
`Sched/Crc.lean`, the Config_Checker's in `Tools/CcImageProofs.lean`.

CRC-32 (IEEE 802.3), reflected, polynomial 0xEDB88320, initial value
0xFFFFFFFF, final complement. -/

namespace Crc

/-- One round: shift right, and xor the polynomial in when the bit shifted
out is set. -/
def round (c : BitVec 32) : BitVec 32 :=
  if c &&& 1#32 = 1#32 then (c >>> 1) ^^^ 0xEDB88320#32 else c >>> 1

/-- `n` rounds. -/
def rounds (c : BitVec 32) : Nat → BitVec 32
  | 0 => c
  | n + 1 => rounds (round c) n

/-- The state after one more byte. -/
def byte (c : BitVec 32) (b : BitVec 8) : BitVec 32 := rounds (c ^^^ b.setWidth 32) 8

/-- The state after the bytes, from the state `c`. -/
def run (c : BitVec 32) (bs : List (BitVec 8)) : BitVec 32 := bs.foldl byte c

/-- The state after the bytes, from the initial value. -/
def state (bs : List (BitVec 8)) : BitVec 32 := run 0xFFFFFFFF#32 bs

/-- The CRC-32 of the bytes. -/
def crc32 (bs : List (BitVec 8)) : BitVec 32 := ~~~ state bs

theorem run_append (c : BitVec 32) (a b : List (BitVec 8)) : run c (a ++ b) = run (run c a) b := by
  simp [run, List.foldl_append]

theorem rounds_succ (c : BitVec 32) (n : Nat) : rounds c (n + 1) = rounds (round c) n := rfl

/-- The little-endian bytes of a 32-bit word. -/
def le32 (v : BitVec 32) : List (BitVec 8) :=
  [v.setWidth 8, (v >>> 8).setWidth 8, (v >>> 16).setWidth 8, (v >>> 24).setWidth 8]

/-- The little-endian bytes of a 64-bit word. -/
def le64 (v : BitVec 64) : List (BitVec 8) :=
  le32 (v.setWidth 32) ++ le32 ((v >>> 32).setWidth 32)

end Crc
