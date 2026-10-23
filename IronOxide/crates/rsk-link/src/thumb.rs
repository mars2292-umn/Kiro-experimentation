//! Thumb-2 decoding of the instructions that move SP or transfer control
//! (Armv7-M ARM A7.7), for the stack-frame and call-graph analysis. Every
//! other instruction is skipped by length. Register values are tracked
//! only as far as `movw`/`movt` pairs and literal-pool loads produce
//! constants, which is how a Rust binary materialises function addresses
//! for indirect calls (PR-18).

/// One decoded effect at an instruction address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// SP decreases by this many bytes (push, vpush, sub sp, pre-indexed store).
    SpDown(u32),
    /// SP increases (pop, vpop, add sp): balanced by the frame model, not counted.
    SpUp(u32),
    /// SP written from a register or an immediate move: no static bound.
    SpDynamic,
    /// `bl` or a direct branch leaving the function: a call or tail call to `target`.
    Call(u32),
    /// A direct branch; `target` may be inside or outside the function.
    Branch(u32),
    /// `blx Rm`: an indirect call through `reg`.
    IndirectCall(u8),
    /// `bx Rm` (not `bx lr`): an indirect tail call through `reg`, or an
    /// exception return when `reg` holds an EXC_RETURN value.
    IndirectBranch(u8),
    /// `bx lr`, `pop {.., pc}`, `ldr pc, [sp], #4`: return.
    Return,
    /// A constant materialised in `reg` (literal-pool load or movw/movt).
    Constant(u8, u32),
    /// `tbb`/`tbh`: a table branch within the function.
    SwitchTable,
    /// `svc #imm`.
    Svc(u8),
    /// Anything else.
    Other,
}

/// A decoded instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Insn {
    pub address: u32,
    pub len: u32,
    pub effect: Effect,
}

fn sign_extend(v: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    ((v << shift) as i32) >> shift
}

/// ThumbExpandImm (A6.3.2) for the modified immediate `i:imm3:imm8`.
pub fn thumb_expand_imm(imm12: u32) -> u32 {
    let a = (imm12 >> 10) & 3;
    let b = (imm12 >> 8) & 3;
    let imm8 = imm12 & 0xFF;
    if a == 0 {
        match b {
            0 => imm8,
            1 => (imm8 << 16) | imm8,
            2 => (imm8 << 24) | (imm8 << 8),
            _ => (imm8 << 24) | (imm8 << 16) | (imm8 << 8) | imm8,
        }
    } else {
        let unrotated = 0x80 | (imm12 & 0x7F);
        let rot = (imm12 >> 7) & 0x1F;
        unrotated.rotate_right(rot)
    }
}

/// Decodes the instruction at `address` from `code` (the function's bytes
/// starting at `base`), with `literal` reading a word of the image for
/// pc-relative loads.
pub fn decode(code: &[u8], base: u32, address: u32, literal: &dyn Fn(u32) -> Option<u32>) -> Option<Insn> {
    let off = (address - base) as usize;
    let hw1 = u16::from_le_bytes([*code.get(off)?, *code.get(off + 1)?]);
    let is32 = (hw1 >> 11) >= 0b11101;
    if !is32 {
        let effect = decode16(hw1, address, literal);
        return Some(Insn { address, len: 2, effect });
    }
    let hw2 = u16::from_le_bytes([*code.get(off + 2)?, *code.get(off + 3)?]);
    let effect = decode32(hw1, hw2, address, literal);
    Some(Insn { address, len: 4, effect })
}

fn decode16(hw: u16, pc: u32, literal: &dyn Fn(u32) -> Option<u32>) -> Effect {
    let h = hw as u32;
    // push {rlist, lr}: 1011 010M rrrrrrrr
    if h & 0xFE00 == 0xB400 {
        let regs = (h & 0xFF).count_ones() + ((h >> 8) & 1);
        return Effect::SpDown(4 * regs);
    }
    // pop {rlist, pc}: 1011 110P rrrrrrrr
    if h & 0xFE00 == 0xBC00 {
        if (h >> 8) & 1 == 1 {
            return Effect::Return;
        }
        return Effect::SpUp(4 * (h & 0xFF).count_ones());
    }
    // add sp, #imm7*4 / sub sp, #imm7*4: 1011 0000 S iii iiii
    if h & 0xFF80 == 0xB000 {
        return Effect::SpUp((h & 0x7F) * 4);
    }
    if h & 0xFF80 == 0xB080 {
        return Effect::SpDown((h & 0x7F) * 4);
    }
    // bx Rm: 0100 0111 0 Rm 000; blx Rm: 0100 0111 1 Rm 000
    if h & 0xFF87 == 0x4700 {
        let rm = ((h >> 3) & 0xF) as u8;
        return if rm == 14 { Effect::Return } else { Effect::IndirectBranch(rm) };
    }
    if h & 0xFF87 == 0x4780 {
        return Effect::IndirectCall(((h >> 3) & 0xF) as u8);
    }
    // mov sp, Rm: 0100 0110 1 Rm 101 (MOV register T1 with Rd = 13)
    if h & 0xFF87 == 0x4685 {
        return Effect::SpDynamic;
    }
    // add sp, Rm: 0100 0100 1 Rm 101 (ADD SP plus register T2)
    if h & 0xFF87 == 0x4485 {
        return Effect::SpDynamic;
    }
    // ldr Rt, [pc, #imm8*4]: 0100 1ttt iiiiiiii
    if h & 0xF800 == 0x4800 {
        let rt = ((h >> 8) & 7) as u8;
        let addr = ((pc + 4) & !3) + (h & 0xFF) * 4;
        return match literal(addr) {
            Some(v) => Effect::Constant(rt, v),
            None => Effect::Other,
        };
    }
    // b<cond> imm8: 1101 cccc iiiiiiii (cond 1110 = udf, 1111 = svc)
    if h & 0xF000 == 0xD000 {
        let cond = (h >> 8) & 0xF;
        if cond == 0xF {
            return Effect::Svc((h & 0xFF) as u8);
        }
        if cond == 0xE {
            return Effect::Other;
        }
        let imm = sign_extend((h & 0xFF) << 1, 9);
        return Effect::Branch((pc as i32 + 4 + imm) as u32);
    }
    // b imm11: 11100 iiiiiiiiiii
    if h & 0xF800 == 0xE000 {
        let imm = sign_extend((h & 0x7FF) << 1, 12);
        return Effect::Branch((pc as i32 + 4 + imm) as u32);
    }
    // cbz/cbnz: 1011 op0 i 1 imm5 Rn
    if h & 0xF500 == 0xB100 {
        let i = (h >> 9) & 1;
        let imm5 = (h >> 3) & 0x1F;
        let imm = (i << 6) | (imm5 << 1);
        return Effect::Branch(pc + 4 + imm);
    }
    Effect::Other
}

fn decode32(hw1: u16, hw2: u16, pc: u32, literal: &dyn Fn(u32) -> Option<u32>) -> Effect {
    let (a, b) = (hw1 as u32, hw2 as u32);
    // push.w / stmdb sp!, {regs}: E92D xxxx ; pop.w / ldmia sp!: E8BD xxxx
    if a == 0xE92D {
        return Effect::SpDown(4 * b.count_ones());
    }
    if a == 0xE8BD {
        if (b >> 15) & 1 == 1 {
            return Effect::Return;
        }
        return Effect::SpUp(4 * b.count_ones());
    }
    // vpush: ED2D (VSTMDB sp!), vpop: ECBD (VLDMIA sp!); imm8 = words.
    if a == 0xED2D {
        return Effect::SpDown(4 * (b & 0xFF));
    }
    if a == 0xECBD {
        return Effect::SpUp(4 * (b & 0xFF));
    }
    // str Rt, [sp, #-imm8]! (T4, P=1 U=0 W=1): F84D xxxx with bits 10..8 = 1 0 1
    if a == 0xF84D && (b >> 8) & 7 == 0b101 {
        return Effect::SpDown(b & 0xFF);
    }
    // strd Rt, Rt2, [sp, #-imm8*4]! : E96D (P=1 U=0 W=1)
    if a == 0xE96D {
        return Effect::SpDown(4 * (b & 0xFF));
    }
    // ldr Rt, [sp], #imm8 with Rt = pc: F85D Fxxx (post-indexed): return
    if a == 0xF85D && (b >> 12) == 0xF {
        return Effect::Return;
    }
    // sub.w sp, sp, #const (T3): F1AD/F1BD 0iii 1101 iiiiiiii ; subw (T4): F2AD
    if (a == 0xF1AD || a == 0xF1BD || a == 0xF2AD) && (b >> 8) & 0xF == 0xD {
        let i = (a >> 10) & 1;
        let imm3 = (b >> 12) & 7;
        let imm8 = b & 0xFF;
        let imm12 = (i << 11) | (imm3 << 8) | imm8;
        return Effect::SpDown(if a == 0xF2AD { imm12 } else { thumb_expand_imm(imm12) });
    }
    // add.w sp, sp, #const (T3): F10D/F11D ; addw: F20D
    if (a == 0xF10D || a == 0xF11D || a == 0xF20D) && (b >> 8) & 0xF == 0xD {
        let i = (a >> 10) & 1;
        let imm3 = (b >> 12) & 7;
        let imm8 = b & 0xFF;
        let imm12 = (i << 11) | (imm3 << 8) | imm8;
        return Effect::SpUp(if a == 0xF20D { imm12 } else { thumb_expand_imm(imm12) });
    }
    // sub/add sp, sp, Rm (register forms with Rd = sp): EBAD/EBBD, EB0D/EB1D with Rd field 1101
    if (a == 0xEBAD || a == 0xEBBD || a == 0xEB0D || a == 0xEB1D) && (b >> 8) & 0xF == 0xD {
        return Effect::SpDynamic;
    }
    // mov.w sp, #const / movw sp: F04F 0Dxx, F24x 0Dxx (Rd = 13)
    if (a & 0xFBEF == 0xF04F || a & 0xFBF0 == 0xF240) && (b >> 8) & 0xF == 0xD {
        return Effect::SpDynamic;
    }
    // movw Rd, #imm16: F240 | i<<10 | imm4 ; hw2: imm3 Rd imm8. movt: F2C0.
    if a & 0xFBF0 == 0xF240 || a & 0xFBF0 == 0xF2C0 {
        let imm4 = a & 0xF;
        let i = (a >> 10) & 1;
        let imm3 = (b >> 12) & 7;
        let rd = ((b >> 8) & 0xF) as u8;
        let imm8 = b & 0xFF;
        let imm16 = (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8;
        let top = a & 0xFBF0 == 0xF2C0;
        return Effect::Constant(rd, if top { imm16 << 16 } else { imm16 });
    }
    // ldr.w Rt, [pc, #±imm12]: F8DF (U=1) / F85F (U=0)
    if a == 0xF8DF || a == 0xF85F {
        let rt = ((b >> 12) & 0xF) as u8;
        let base = (pc + 4) & !3;
        let addr = if a == 0xF8DF { base + (b & 0xFFF) } else { base.wrapping_sub(b & 0xFFF) };
        return match literal(addr) {
            Some(v) => Effect::Constant(rt, v),
            None => Effect::Other,
        };
    }
    // tbb/tbh: E8D0 F00x / E8D0 F01x
    if a & 0xFFF0 == 0xE8D0 && (b & 0xFFE0) == 0xF000 {
        return Effect::SwitchTable;
    }
    // bl and b.w (T4): 11110 S imm10 | 11 J1 1 J2 imm11 (bl) ; 10 J1 1 J2 (b.w)
    if a & 0xF800 == 0xF000 && (b & 0xD000 == 0xD000 || b & 0xD000 == 0x9000) {
        let s = (a >> 10) & 1;
        let j1 = (b >> 13) & 1;
        let j2 = (b >> 11) & 1;
        let i1 = (!(j1 ^ s)) & 1;
        let i2 = (!(j2 ^ s)) & 1;
        let imm10 = a & 0x3FF;
        let imm11 = b & 0x7FF;
        let raw = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1);
        let imm = sign_extend(raw, 25);
        let target = (pc as i32 + 4 + imm) as u32;
        return if b & 0x4000 == 0x4000 { Effect::Call(target) } else { Effect::Branch(target) };
    }
    // b<cond>.w (T3): 11110 S cond imm6 | 10 J1 0 J2 imm11
    if a & 0xF800 == 0xF000 && b & 0xD000 == 0x8000 {
        let s = (a >> 10) & 1;
        let cond = (a >> 6) & 0xF;
        if cond & 0xE == 0xE {
            return Effect::Other; // the always/never encodings here are other instructions
        }
        let j1 = (b >> 13) & 1;
        let j2 = (b >> 11) & 1;
        let imm6 = a & 0x3F;
        let imm11 = b & 0x7FF;
        let raw = (s << 20) | (j2 << 19) | (j1 << 18) | (imm6 << 12) | (imm11 << 1);
        let imm = sign_extend(raw, 21);
        return Effect::Branch((pc as i32 + 4 + imm) as u32);
    }
    Effect::Other
}

/// Decodes a whole function body.
pub fn decode_function(code: &[u8], base: u32, literal: &dyn Fn(u32) -> Option<u32>) -> Vec<Insn> {
    let mut out = Vec::new();
    let mut addr = base;
    let end = base + code.len() as u32;
    while addr + 2 <= end {
        match decode(code, base, addr, literal) {
            Some(insn) => {
                addr += insn.len;
                out.push(insn);
            }
            None => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: u32) -> Option<u32> {
        None
    }

    fn one(bytes: &[u8]) -> Effect {
        decode(bytes, 0x1000, 0x1000, &none).unwrap().effect
    }

    #[test]
    fn sixteen_bit_stack_and_control_forms() {
        assert_eq!(one(&[0xF0, 0xB5]), Effect::SpDown(20)); // push {r4-r7, lr}
        assert_eq!(one(&[0x80, 0xBD]), Effect::Return); // pop {r7, pc}
        assert_eq!(one(&[0x10, 0xBC]), Effect::SpUp(4)); // pop {r4}
        assert_eq!(one(&[0x86, 0xB0]), Effect::SpDown(24)); // sub sp, #24
        assert_eq!(one(&[0x06, 0xB0]), Effect::SpUp(24)); // add sp, #24
        assert_eq!(one(&[0x70, 0x47]), Effect::Return); // bx lr
        assert_eq!(one(&[0x08, 0x47]), Effect::IndirectBranch(1)); // bx r1
        assert_eq!(one(&[0x98, 0x47]), Effect::IndirectCall(3)); // blx r3
        assert_eq!(one(&[0x85, 0x46]), Effect::SpDynamic); // mov sp, r0
        assert_eq!(one(&[0xFE, 0xE7]), Effect::Branch(0x1000)); // b .
        assert_eq!(one(&[0x01, 0xD0]), Effect::Branch(0x1006)); // beq +2
        assert_eq!(one(&[0x05, 0xDF]), Effect::Svc(5));
        assert_eq!(one(&[0x08, 0xB1]), Effect::Branch(0x1006)); // cbz r0, +2
    }

    #[test]
    fn thirty_two_bit_stack_and_control_forms() {
        assert_eq!(one(&[0x2D, 0xE9, 0xF8, 0x4F]), Effect::SpDown(40)); // push.w {r3-r11, lr}
        assert_eq!(one(&[0xBD, 0xE8, 0xF8, 0x4F]), Effect::SpUp(40)); // pop.w {r3-r11, lr}
        assert_eq!(one(&[0xBD, 0xE8, 0xF0, 0x8F]), Effect::Return); // pop.w {r4-r7, pc}
        assert_eq!(one(&[0x2D, 0xED, 0x10, 0x8A]), Effect::SpDown(64)); // vpush {s16-s31}
        assert_eq!(one(&[0xBD, 0xEC, 0x10, 0x8A]), Effect::SpUp(64)); // vpop {s16-s31}
        assert_eq!(one(&[0xAD, 0xF1, 0x18, 0x0D]), Effect::SpDown(24)); // sub.w sp, sp, #24
        assert_eq!(one(&[0xAD, 0xF2, 0x34, 0x1D]), Effect::SpDown(0x134)); // subw sp, sp, #308
        assert_eq!(one(&[0x0D, 0xF1, 0x18, 0x0D]), Effect::SpUp(24)); // add.w sp, sp, #24
        assert_eq!(one(&[0xAD, 0xEB, 0x00, 0x0D]), Effect::SpDynamic); // sub.w sp, sp, r0
        assert_eq!(one(&[0x4D, 0xF8, 0x04, 0x0D]), Effect::SpDown(4)); // str r0, [sp, #-4]!
        assert_eq!(one(&[0x00, 0xF0, 0x00, 0xF8]), Effect::Call(0x1004)); // bl +0
        assert_eq!(one(&[0xFF, 0xF7, 0xFE, 0xBF]), Effect::Branch(0x1000)); // b.w .
        assert_eq!(one(&[0x00, 0xF0, 0x02, 0x80]), Effect::Branch(0x1008)); // beq.w +4
        assert_eq!(one(&[0xD0, 0xE8, 0x01, 0xF0]), Effect::SwitchTable); // tbb [r0, r1]
        assert_eq!(one(&[0x41, 0xF2, 0x34, 0x22]), Effect::Constant(2, 0x1234)); // movw r2, #0x1234
        assert_eq!(one(&[0xC0, 0xF2, 0x01, 0x02]), Effect::Constant(2, 0x0001_0000)); // movt r2, #1
        let lit = |a: u32| if a == 0x1008 { Some(0xDEAD_BEEF) } else { None };
        assert_eq!(decode(&[0xDF, 0xF8, 0x04, 0x30], 0x1000, 0x1000, &lit).unwrap().effect, Effect::Constant(3, 0xDEAD_BEEF)); // ldr.w r3, [pc, #4]
        assert_eq!(decode(&[0x01, 0x48], 0x1000, 0x1000, &lit).unwrap().effect, Effect::Constant(0, 0xDEAD_BEEF)); // ldr r0, [pc, #4]
    }

    #[test]
    fn modified_immediates_expand() {
        assert_eq!(thumb_expand_imm(0x018), 0x18);
        assert_eq!(thumb_expand_imm(0x1FF), 0x00FF_00FF);
        assert_eq!(thumb_expand_imm(0x4FF), 0x7F80_0000); // 0xFF rotated right by 9
    }
}
