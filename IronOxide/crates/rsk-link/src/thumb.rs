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
    /// A 32-bit instruction of a class that can write SP (as a destination
    /// register, a loaded register, or a base register written back) in a
    /// form this decoder reads no stack-pointer change from (`sp_unread`).
    /// The Link_Checker refuses the function (2026-11-27): until then such
    /// an instruction was read as leaving SP.
    SpUnread,
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
    /// A floating-point instruction of the given class (PR-31, R6.6).
    Fp(FpClass),
    /// `msr <special>, Rn` with the SYSm value (PR-22).
    Msr(u8),
    /// `mrs Rd, <special>` with the SYSm value.
    Mrs(u8),
    /// `cpsie`/`cpsid` (PR-22).
    Cps,
    /// `cbz`/`cbnz` to `target` (ORQ-32: forbidden across naked stubs).
    CompareBranch(u32),
    /// `nop` (ORQ-32: the assembler's silent replacement of an unencodable branch).
    Nop,
    /// `dsb` (erratum 838869: the barrier before every exception return).
    Dsb,
    /// Anything else.
    Other,
}

/// The classes of VFP instructions the Profile distinguishes (PR-31, R6.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FpClass {
    /// Register moves, loads, stores, and core-register transfers.
    Move,
    /// Arithmetic, comparison, conversion, square root, negation, absolute value.
    Arith,
    /// `vmsr fpscr, Rt`.
    FpscrWrite,
    /// `vmrs Rt, fpscr`.
    FpscrRead,
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
        // ROR('1':imm7, imm12<11:7>). The rotation is by 8 to 31 here (bits
        // 11..10 are not both zero) and the value has 8 bits, so nothing
        // stays below: the rotation is a shift left by 32 minus the amount.
        // Written so since 2026-11-27 (the Lean translation of
        // `rotate_right` has no value).
        let unrotated = 0x80 | (imm12 & 0x7F);
        let rot = (imm12 >> 7) & 0x1F;
        unrotated << (32 - rot)
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

/// What the halfwords of an instruction decide by themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decoded {
    /// The effect.
    Is(Effect),
    /// A pc-relative load to `reg` of the word at the address: the effect
    /// is `Constant` with the word when the image has it, `Other` when not.
    Literal(u8, u32),
}

fn resolve(d: Decoded, literal: &dyn Fn(u32) -> Option<u32>) -> Effect {
    match d {
        Decoded::Is(e) => e,
        Decoded::Literal(rt, addr) => match literal(addr) {
            Some(v) => Effect::Constant(rt, v),
            None => Effect::Other,
        },
    }
}

fn decode16(hw: u16, pc: u32, literal: &dyn Fn(u32) -> Option<u32>) -> Effect {
    resolve(decode16_static(hw, pc), literal)
}

fn decode32(hw1: u16, hw2: u16, pc: u32, literal: &dyn Fn(u32) -> Option<u32>) -> Effect {
    resolve(decode32_static(hw1, hw2, pc), literal)
}

/// The 16-bit instruction `hw` at `pc`, from the halfword alone: the effect,
/// or the address of the literal a pc-relative load reads. No closure
/// here: this is the function translated to Lean
/// (`proofs/aeneas/lean/Tools/Link.lean`, 2026-11-26).
pub fn decode16_static(hw: u16, pc: u32) -> Decoded {
    let h = hw as u32;
    // push {rlist, lr}: 1011 010M rrrrrrrr
    if h & 0xFE00 == 0xB400 {
        let regs = ones16(h & 0xFF) + ((h >> 8) & 1);
        return Decoded::Is(Effect::SpDown(4 * regs));
    }
    // pop {rlist, pc}: 1011 110P rrrrrrrr
    if h & 0xFE00 == 0xBC00 {
        if (h >> 8) & 1 == 1 {
            return Decoded::Is(Effect::Return);
        }
        return Decoded::Is(Effect::SpUp(4 * ones16(h & 0xFF)));
    }
    // add sp, #imm7*4 / sub sp, #imm7*4: 1011 0000 S iii iiii
    if h & 0xFF80 == 0xB000 {
        return Decoded::Is(Effect::SpUp((h & 0x7F) * 4));
    }
    if h & 0xFF80 == 0xB080 {
        return Decoded::Is(Effect::SpDown((h & 0x7F) * 4));
    }
    // bx Rm: 0100 0111 0 Rm 000; blx Rm: 0100 0111 1 Rm 000
    if h & 0xFF87 == 0x4700 {
        let rm = ((h >> 3) & 0xF) as u8;
        return Decoded::Is(if rm == 14 { Effect::Return } else { Effect::IndirectBranch(rm) });
    }
    if h & 0xFF87 == 0x4780 {
        return Decoded::Is(Effect::IndirectCall(((h >> 3) & 0xF) as u8));
    }
    // mov sp, Rm: 0100 0110 1 Rm 101 (MOV register T1 with Rd = 13)
    if h & 0xFF87 == 0x4685 {
        return Decoded::Is(Effect::SpDynamic);
    }
    // add sp, Rm: 0100 0100 1 Rm 101 (ADD SP plus register T2)
    if h & 0xFF87 == 0x4485 {
        return Decoded::Is(Effect::SpDynamic);
    }
    // ldr Rt, [pc, #imm8*4]: 0100 1ttt iiiiiiii
    if h & 0xF800 == 0x4800 {
        let rt = ((h >> 8) & 7) as u8;
        let addr = ((pc + 4) & !3) + (h & 0xFF) * 4;
        return Decoded::Literal(rt, addr);
    }
    // b<cond> imm8: 1101 cccc iiiiiiii (cond 1110 = udf, 1111 = svc)
    if h & 0xF000 == 0xD000 {
        let cond = (h >> 8) & 0xF;
        if cond == 0xF {
            return Decoded::Is(Effect::Svc((h & 0xFF) as u8));
        }
        if cond == 0xE {
            return Decoded::Is(Effect::Other);
        }
        let imm = sign_extend((h & 0xFF) << 1, 9);
        return Decoded::Is(Effect::Branch((pc as i32 + 4 + imm) as u32));
    }
    // b imm11: 11100 iiiiiiiiiii
    if h & 0xF800 == 0xE000 {
        let imm = sign_extend((h & 0x7FF) << 1, 12);
        return Decoded::Is(Effect::Branch((pc as i32 + 4 + imm) as u32));
    }
    // cpsie / cpsid: 1011 0110 011 im 0 a i f
    if h & 0xFFEC == 0xB660 {
        return Decoded::Is(Effect::Cps);
    }
    // cbz/cbnz: 1011 op0 i 1 imm5 Rn
    if h & 0xF500 == 0xB100 {
        let i = (h >> 9) & 1;
        let imm5 = (h >> 3) & 0x1F;
        let imm = (i << 6) | (imm5 << 1);
        return Decoded::Is(Effect::CompareBranch(pc + 4 + imm));
    }
    if h == 0xBF00 {
        return Decoded::Is(Effect::Nop);
    }
    Decoded::Is(Effect::Other)
}

/// The number of the low 16 bits of `v` that are set: `count_ones` for a
/// register list, which is a halfword or less. Written out since
/// 2026-11-26: the Lean translation has no model of `count_ones` (it would
/// be an axiom of `proofs/aeneas/lean/Tools/Link.lean`).
fn ones16(v: u32) -> u32 {
    (v & 1) + ((v >> 1) & 1) + ((v >> 2) & 1) + ((v >> 3) & 1) + ((v >> 4) & 1) + ((v >> 5) & 1) + ((v >> 6) & 1) + ((v >> 7) & 1) + ((v >> 8) & 1) + ((v >> 9) & 1) + ((v >> 10) & 1) + ((v >> 11) & 1) + ((v >> 12) & 1) + ((v >> 13) & 1) + ((v >> 14) & 1) + ((v >> 15) & 1)
}

/// `p && q` with both sides computed first. The decoders' conditions have
/// no effect and cannot fail, so the result is the short-circuit one;
/// written as calls since 2026-11-26 because Charon's reconstruction of
/// the control flow of `decode32_static` did not end with the short-circuit
/// operators in its chain of early returns.
fn both(p: bool, q: bool) -> bool {
    if p {
        q
    } else {
        false
    }
}

/// `p || q` with both sides computed first (see `both`).
fn either(p: bool, q: bool) -> bool {
    if p {
        true
    } else {
        q
    }
}

/// The 32-bit instruction `hw1 hw2` at `pc`, from the halfwords alone (as
/// `decode16_static`).
pub fn decode32_static(hw1: u16, hw2: u16, pc: u32) -> Decoded {
    let (a, b) = (hw1 as u32, hw2 as u32);
    // dsb #option: F3BF 8F4x (erratum 838869's workaround before exception returns).
    if both(a == 0xF3BF, b & 0xFFF0 == 0x8F40) {
        return Decoded::Is(Effect::Dsb);
    }
    // ldr.w Rt, [Rn, #imm12] (T3: F8Dn), ldr Rt, [Rn, #-imm8] / post-indexed /
    // pre-indexed (T4: F85n), ldr Rt, [pc, #imm12] (T2: F85F/F8DF), and
    // ldr.w Rt, [Rn, Rm, lsl #n] (T2 register: F85n with bits 11..6 zero)
    // with Rt = SP: a load to the stack pointer, the instruction class of
    // erratum 752770 (interrupted loads to SP), which no Kernel code may
    // contain (R11.6; the compiler never emits one).
    if both(either(a & 0xFFF0 == 0xF8D0, a & 0xFFF0 == 0xF850), (b >> 12) == 13) {
        return Decoded::Is(Effect::SpDynamic);
    }
    // msr <spec>, Rn: F380 | Rn ; hw2 1000 mask 0000 SYSm. mrs Rd, <spec>: F3EF ; hw2 1000 Rd SYSm.
    if both(a & 0xFFF0 == 0xF380, b & 0xF000 == 0x8000) {
        return Decoded::Is(Effect::Msr((b & 0xFF) as u8));
    }
    if both(a == 0xF3EF, b & 0xF000 == 0x8000) {
        // mrs sp, <spec> writes SP with a value that is not in the code.
        if (b >> 8) & 0xF == 13 {
            return Decoded::Is(Effect::SpUnread);
        }
        return Decoded::Is(Effect::Mrs((b & 0xFF) as u8));
    }
    // push.w / stmdb sp!, {regs}: E92D xxxx ; pop.w / ldmia sp!: E8BD xxxx
    if a == 0xE92D {
        return Decoded::Is(Effect::SpDown(4 * ones16(b)));
    }
    if a == 0xE8BD {
        if (b >> 15) & 1 == 1 {
            return Decoded::Is(Effect::Return);
        }
        return Decoded::Is(Effect::SpUp(4 * ones16(b)));
    }
    // vpush: ED2D (VSTMDB sp!), vpop: ECBD (VLDMIA sp!); imm8 = words. Bit 6
    // of the first halfword is the D bit, part of the first register's
    // number (an odd single-precision register, or D16 and above): ED6D
    // and ECFD are the same two instructions. Until 2026-11-26 only the
    // forms with D clear were read here, and the others fell to the
    // floating-point moves below, which leave SP.
    if a & 0xFFBF == 0xED2D {
        return Decoded::Is(Effect::SpDown(4 * (b & 0xFF)));
    }
    if a & 0xFFBF == 0xECBD {
        return Decoded::Is(Effect::SpUp(4 * (b & 0xFF)));
    }
    // str Rt, [sp, #-imm8]! (T4, P=1 U=0 W=1): F84D xxxx with bits 10..8 = 1 0 1
    if both(a == 0xF84D, (b >> 8) & 7 == 0b101) {
        return Decoded::Is(Effect::SpDown(b & 0xFF));
    }
    // strd Rt, Rt2, [sp, #-imm8*4]! : E96D (P=1 U=0 W=1)
    if a == 0xE96D {
        return Decoded::Is(Effect::SpDown(4 * (b & 0xFF)));
    }
    // ldr Rt, [sp], #imm8 with Rt = pc: F85D Fxxx (post-indexed): return
    if both(a == 0xF85D, (b >> 12) == 0xF) {
        return Decoded::Is(Effect::Return);
    }
    // ldr Rt, [sp], #imm8 (T4, P=0 U=1 W=1: bits 11..8 = 1011), the pop of
    // one register (Rt = pc is the return above, Rt = sp the load to SP
    // further above).
    if both(a == 0xF85D, (b >> 8) & 0xF == 0xB) {
        return Decoded::Is(Effect::SpUp(b & 0xFF));
    }
    // The data-processing forms with an immediate (first halfword 11110...)
    // have bit 15 of the second halfword clear; with it set the same first
    // halfwords are branches (`bl`, `b.w`, `b<cond>.w`) and the system
    // forms. Until 2026-11-26 the four cases below did not test the bit, so
    // that a `bl` or `b.w` over a distance whose high bits made its first
    // halfword one of theirs (323,584 bytes or more) was read as a
    // constant load or a stack-pointer change and its edge lost.
    let dp = (b >> 15) & 1 == 0;
    // sub sp, sp, #const. T2: 11110 i 0 1101 S 1101 | 0 imm3 1101 imm8 with a
    // modified immediate (F1AD, F1BD and, with i set, F5AD, F5BD); T3
    // (subw): 11110 i 1 0101 0 1101 with the 12 bits i:imm3:imm8 (F2AD,
    // F6AD). Until 2026-11-27 only the forms with i clear were matched, and
    // a decrement whose immediate has its top bit set (`sub.w sp, sp,
    // #0x6b0`, F5AD 6DD6, the frame of the boot) was read as leaving SP.
    let sub_w = a & 0xFBFF == 0xF2AD;
    if both(dp, both(either(a & 0xFBEF == 0xF1AD, sub_w), (b >> 8) & 0xF == 0xD)) {
        let i = (a >> 10) & 1;
        let imm3 = (b >> 12) & 7;
        let imm8 = b & 0xFF;
        let imm12 = (i << 11) | (imm3 << 8) | imm8;
        return Decoded::Is(Effect::SpDown(if sub_w { imm12 } else { thumb_expand_imm(imm12) }));
    }
    // add sp, sp, #const. T3: 11110 i 0 1000 S 1101 (F10D, F11D, F50D,
    // F51D); T4 (addw): 11110 i 1 0000 0 1101 (F20D, F60D).
    let add_w = a & 0xFBFF == 0xF20D;
    if both(dp, both(either(a & 0xFBEF == 0xF10D, add_w), (b >> 8) & 0xF == 0xD)) {
        let i = (a >> 10) & 1;
        let imm3 = (b >> 12) & 7;
        let imm8 = b & 0xFF;
        let imm12 = (i << 11) | (imm3 << 8) | imm8;
        return Decoded::Is(Effect::SpUp(if add_w { imm12 } else { thumb_expand_imm(imm12) }));
    }
    // sub/add sp, sp, Rm (register forms with Rd = sp): EBAD/EBBD, EB0D/EB1D with Rd field 1101
    if both(either(either(either(a == 0xEBAD, a == 0xEBBD), a == 0xEB0D), a == 0xEB1D), (b >> 8) & 0xF == 0xD) {
        return Decoded::Is(Effect::SpDynamic);
    }
    // mov.w sp, #const / movw sp: F04F 0Dxx, F24x 0Dxx (Rd = 13)
    if both(dp, both(either(a & 0xFBEF == 0xF04F, a & 0xFBF0 == 0xF240), (b >> 8) & 0xF == 0xD)) {
        return Decoded::Is(Effect::SpDynamic);
    }
    // Every form above that writes SP has returned. Any other instruction
    // of a class that can write it is refused, not read as leaving it.
    if sp_unread(a, b) {
        return Decoded::Is(Effect::SpUnread);
    }
    // movw Rd, #imm16: F240 | i<<10 | imm4 ; hw2: imm3 Rd imm8. movt: F2C0.
    if both(dp, either(a & 0xFBF0 == 0xF240, a & 0xFBF0 == 0xF2C0)) {
        let imm4 = a & 0xF;
        let i = (a >> 10) & 1;
        let imm3 = (b >> 12) & 7;
        let rd = ((b >> 8) & 0xF) as u8;
        let imm8 = b & 0xFF;
        let imm16 = (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8;
        let top = a & 0xFBF0 == 0xF2C0;
        return Decoded::Is(Effect::Constant(rd, if top { imm16 << 16 } else { imm16 }));
    }
    // ldr.w Rt, [pc, #±imm12]: F8DF (U=1) / F85F (U=0)
    if either(a == 0xF8DF, a == 0xF85F) {
        let rt = ((b >> 12) & 0xF) as u8;
        let base = (pc + 4) & !3;
        let addr = if a == 0xF8DF { base + (b & 0xFFF) } else { base.wrapping_sub(b & 0xFFF) };
        return Decoded::Literal(rt, addr);
    }
    // VFP (coprocessor 10 or 11): 111x 11xx .... | .... 101x .... ....
    if both(a & 0xEC00 == 0xEC00, (b >> 8) & 0xE == 0xA) {
        if a & 0xFE00 == 0xEC00 {
            // VLDM/VSTM, VLDR/VSTR, 64-bit core transfers (VPUSH/VPOP matched above).
            return Decoded::Is(Effect::Fp(FpClass::Move));
        }
        if both(a & 0xFFE0 == 0xEE00, b & 0x10 == 0x10) {
            // VMOV between a core register and a single-precision register.
            return Decoded::Is(Effect::Fp(FpClass::Move));
        }
        if both(a == 0xEEE1, b & 0x0F10 == 0x0A10) {
            return Decoded::Is(Effect::Fp(FpClass::FpscrWrite));
        }
        if both(a == 0xEEF1, b & 0x0F10 == 0x0A10) {
            return Decoded::Is(Effect::Fp(FpClass::FpscrRead));
        }
        if both(a & 0xFFB0 == 0xEEB0, b & 0x00F0 == 0) {
            // VMOV immediate.
            return Decoded::Is(Effect::Fp(FpClass::Move));
        }
        if both(a & 0xFFBF == 0xEEB0, b & 0x00D0 == 0x0040) {
            // VMOV register.
            return Decoded::Is(Effect::Fp(FpClass::Move));
        }
        return Decoded::Is(Effect::Fp(FpClass::Arith));
    }
    // tbb/tbh: E8D0 F00x / E8D0 F01x
    if both(a & 0xFFF0 == 0xE8D0, (b & 0xFFE0) == 0xF000) {
        return Decoded::Is(Effect::SwitchTable);
    }
    // bl and b.w (T4): 11110 S imm10 | 11 J1 1 J2 imm11 (bl) ; 10 J1 1 J2 (b.w)
    if both(a & 0xF800 == 0xF000, either(b & 0xD000 == 0xD000, b & 0xD000 == 0x9000)) {
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
        return Decoded::Is(if b & 0x4000 == 0x4000 { Effect::Call(target) } else { Effect::Branch(target) });
    }
    // b<cond>.w (T3): 11110 S cond imm6 | 10 J1 0 J2 imm11
    if both(a & 0xF800 == 0xF000, b & 0xD000 == 0x8000) {
        let s = (a >> 10) & 1;
        let cond = (a >> 6) & 0xF;
        if cond & 0xE == 0xE {
            return Decoded::Is(Effect::Other); // the always/never encodings here are other instructions
        }
        let j1 = (b >> 13) & 1;
        let j2 = (b >> 11) & 1;
        let imm6 = a & 0x3F;
        let imm11 = b & 0x7FF;
        let raw = (s << 20) | (j2 << 19) | (j1 << 18) | (imm6 << 12) | (imm11 << 1);
        let imm = sign_extend(raw, 21);
        return Decoded::Is(Effect::Branch((pc as i32 + 4 + imm) as u32));
    }
    Decoded::Is(Effect::Other)
}

/// Whether the 32-bit instruction `a b` belongs to a class that can write
/// SP: register 13 is its destination, or a register it loads, or its base
/// register with writeback (Armv7-M ARM A5.3, by class). Called by
/// `decode32_static` after every form it reads a stack-pointer change from
/// has returned (and `mrs`, which it tests itself), so `true` is "writes
/// SP, or may, in a form not read". The 16-bit instructions need no such
/// test: the only ones that reach register 13 as a destination are `mov`
/// and `add` with a high register (both read as `SpDynamic`), `push`,
/// `pop` and `add`/`sub sp, #imm7`. `msr` to MSP or PSP is reported as
/// `Msr` (PR-22), not here.
pub fn sp_unread(a: u32, b: u32) -> bool {
    let top = a >> 11;
    let class = (a >> 9) & 3;
    let rn = a & 0xF;
    let rd = (b >> 8) & 0xF;
    let rt = b >> 12;
    let w = (a >> 5) & 1;
    let l = (a >> 4) & 1;
    if top == 30 {
        // 11110: with bit 15 of the second halfword clear, data processing
        // with a modified or a plain immediate, Rd in bits 11..8 (the
        // compare forms have Rd = 15). With it set: branches and system forms.
        return both(rt < 8, rd == 13);
    }
    if both(top == 29, class == 0) {
        if (a >> 6) & 1 == 0 {
            // 1110 100x x0WL: load and store multiple. Writeback to SP, or
            // SP in the list of a load.
            return either(both(w == 1, rn == 13), both(l == 1, (b >> 13) & 1 == 1));
        }
        if both((a >> 8) & 1 == 0, w == 0) {
            // 1110 1000 x1 0L: the exclusive forms and the table branches.
            // strex (op 4: Rd in bits 11..8), strexb/strexh (op 12: Rd in
            // bits 3..0), ldrex, ldrexb, ldrexh (op 5, 13: Rt in bits 15..12).
            let op = (a >> 4) & 0xF;
            return either(either(both(op == 4, rd == 13), both(op == 12, b & 0xF == 13)), both(l == 1, rt == 13));
        }
        // ldrd and strd with an immediate: writeback to SP, or SP loaded.
        return either(both(w == 1, rn == 13), both(l == 1, either(rt == 13, rd == 13)));
    }
    if both(top == 29, class == 1) {
        // 1110 101x: data processing with a shifted register, Rd in bits 11..8.
        return rd == 13;
    }
    if both(top == 31, class == 0) {
        // 1111 100x: load and store of one register. Writeback (the
        // immediate form with bit 11 and W, bit 8, set) to SP, or SP loaded.
        let wb = both(both((a >> 7) & 1 == 0, (b >> 11) & 1 == 1), (b >> 8) & 1 == 1);
        return either(both(wb, rn == 13), both(l == 1, rt == 13));
    }
    if both(top == 31, class == 1) {
        // 1111 1010: data processing with registers; 1111 1011: multiply,
        // long multiply, divide. Rd (RdHi) in bits 11..8; RdLo of the long
        // forms (bits 8..7 of the first halfword set) in bits 15..12.
        return either(rd == 13, both((a >> 7) & 3 == 3, rt == 13));
    }
    if class == 2 {
        // 111x 110x: coprocessor load and store (vldm, vstm, vldr, vstr)
        // and the transfers of two registers. Writeback to SP, or SP as
        // one of the two registers a transfer loads (P = U = 0, L set).
        let two = both(both((a >> 8) & 1 == 0, (a >> 7) & 1 == 0), l == 1);
        return either(both(w == 1, rn == 13), both(two, either(rt == 13, rn == 13)));
    }
    // 111x 1110: a transfer from the coprocessor to a core register (bit 4
    // of the second halfword and L set), Rt in bits 15..12.
    both(both((a >> 8) & 1 == 0, (b >> 4) & 1 == 1), both(l == 1, rt == 13))
}

/// Decodes every halfword of a function's symbol in address order. Not the
/// Link_Checker's body since 2026-11-27 (`flow::body`: the instructions
/// control reaches, an inline table read as data); kept for the tests that
/// compare the two.
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

    /// 2026-11-26: every `bl` and `b.w` is read as a call or a branch to
    /// its target, whatever its distance: every first halfword 11110...,
    /// with a sample of the second halfwords of both forms.
    #[test]
    fn no_other_case_takes_a_bl_or_a_wide_branch() {
        for a in 0xF000u32..=0xF7FF {
            let mut low = 0u32;
            while low < 0x1000 {
                for form in [0xD000u32, 0x9000] {
                    for j in [0u32, 0x0800, 0x2000, 0x2800] {
                        let b = form | j | (low & 0x7FF);
                        let bytes = [a as u8, (a >> 8) as u8, b as u8, (b >> 8) as u8];
                        let s = (a >> 10) & 1;
                        let i1 = (!(((b >> 13) & 1) ^ s)) & 1;
                        let i2 = (!(((b >> 11) & 1) ^ s)) & 1;
                        let raw = (s << 24) | (i1 << 23) | (i2 << 22) | ((a & 0x3FF) << 12) | ((b & 0x7FF) << 1);
                        let target = (0x1000i32 + 4 + sign_extend(raw, 25)) as u32;
                        let want = if form == 0xD000 { Effect::Call(target) } else { Effect::Branch(target) };
                        assert_eq!(one(&bytes), want, "{a:#06x} {b:#06x}");
                    }
                }
                low += 37;
            }
        }
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
        assert_eq!(one(&[0x08, 0xB1]), Effect::CompareBranch(0x1006)); // cbz r0, +2
        assert_eq!(one(&[0x00, 0xBF]), Effect::Nop);
        assert_eq!(one(&[0xBF, 0xF3, 0x4F, 0x8F]), Effect::Dsb); // dsb sy
        assert_eq!(one(&[0xD1, 0xF8, 0x04, 0xD0]), Effect::SpDynamic); // ldr.w sp, [r1, #4] (erratum 752770)
        assert_eq!(one(&[0x51, 0xF8, 0x04, 0xDB]), Effect::SpDynamic); // ldr sp, [r1], #4 (erratum 752770)
    }

    #[test]
    fn thirty_two_bit_stack_and_control_forms() {
        assert_eq!(one(&[0x2D, 0xE9, 0xF8, 0x4F]), Effect::SpDown(40)); // push.w {r3-r11, lr}
        assert_eq!(one(&[0xBD, 0xE8, 0xF8, 0x4F]), Effect::SpUp(40)); // pop.w {r3-r11, lr}
        assert_eq!(one(&[0xBD, 0xE8, 0xF0, 0x8F]), Effect::Return); // pop.w {r4-r7, pc}
        assert_eq!(one(&[0x2D, 0xED, 0x10, 0x8A]), Effect::SpDown(64)); // vpush {s16-s31}
        assert_eq!(one(&[0xBD, 0xEC, 0x10, 0x8A]), Effect::SpUp(64)); // vpop {s16-s31}
        // The D bit set: the first register is odd-numbered (2026-11-26).
        assert_eq!(one(&[0x6D, 0xED, 0x02, 0x8A]), Effect::SpDown(8)); // vpush {s17-s18}
        assert_eq!(one(&[0xFD, 0xEC, 0x02, 0x8A]), Effect::SpUp(8)); // vpop {s17-s18}
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

    /// 2026-11-27: `sub`/`add sp, sp, #const` in every encoding. The first
    /// is the frame of `Reset` in a rig build of 2026-11-26 (0x6b0 = 1,712
    /// bytes), which was read as leaving SP.
    #[test]
    fn constant_stack_adjustments_with_the_top_immediate_bit_set() {
        assert_eq!(one(&[0xAD, 0xF5, 0xD6, 0x6D]), Effect::SpDown(0x6b0)); // sub.w sp, sp, #0x6b0
        assert_eq!(one(&[0xBD, 0xF5, 0xD6, 0x6D]), Effect::SpDown(0x6b0)); // subs.w sp, sp, #0x6b0
        assert_eq!(one(&[0xAD, 0xF6, 0x34, 0x1D]), Effect::SpDown(0x934)); // subw sp, sp, #0x934
        assert_eq!(one(&[0x0D, 0xF5, 0xD6, 0x6D]), Effect::SpUp(0x6b0)); // add.w sp, sp, #0x6b0
        assert_eq!(one(&[0x1D, 0xF5, 0xD6, 0x6D]), Effect::SpUp(0x6b0)); // adds.w sp, sp, #0x6b0
        assert_eq!(one(&[0x0D, 0xF6, 0x34, 0x1D]), Effect::SpUp(0x934)); // addw sp, sp, #0x934
        assert_eq!(one(&[0xAD, 0xF2, 0xC4, 0x6D]), Effect::SpDown(0x6c4)); // subw sp, sp, #0x6c4
        assert_eq!(one(&[0x5D, 0xF8, 0x04, 0xBB]), Effect::SpUp(4)); // ldr r11, [sp], #4
        // Every i:imm3:imm8 of both encodings of the decrement is read.
        for imm12 in 0u32..0x1000 {
            let a = 0xF1AD | ((imm12 >> 11) << 10);
            let b = (((imm12 >> 8) & 7) << 12) | 0x0D00 | (imm12 & 0xFF);
            let bytes = [a as u8, (a >> 8) as u8, b as u8, (b >> 8) as u8];
            assert_eq!(one(&bytes), Effect::SpDown(thumb_expand_imm(imm12)), "{imm12:#x}");
            let a = 0xF2AD | ((imm12 >> 11) << 10);
            let bytes = [a as u8, (a >> 8) as u8, b as u8, (b >> 8) as u8];
            assert_eq!(one(&bytes), Effect::SpDown(imm12), "{imm12:#x}");
        }
    }

    /// The prologue of `Reset` in the rig build of 2026-11-26 (commit
    /// 0fb2aa14, isolation variant): `push {r7, lr}`, `mov r7, sp`,
    /// `sub.w sp, sp, #0x6b0`. Its frame is 1,720 bytes; the Link_Checker
    /// reported 8.
    #[test]
    fn the_frame_of_the_reset_prologue_that_was_read_as_eight_bytes() {
        let code = [0x80, 0xB5, 0x6F, 0x46, 0xAD, 0xF5, 0xD6, 0x6D];
        let insns = decode_function(&code, 0x289c, &none);
        assert_eq!(insns.len(), 3);
        assert_eq!(crate::bound_v::frame_of(&insns), 8 + 0x6b0);
        assert!(!crate::bound_v::dynamic_of(&insns));
    }

    /// The rotation written as a shift is the manual's ROR on every
    /// modified immediate.
    #[test]
    fn the_modified_immediate_is_the_rotation() {
        for imm12 in 0x400u32..0x1000 {
            assert_eq!(thumb_expand_imm(imm12), (0x80 | (imm12 & 0x7F)).rotate_right((imm12 >> 7) & 0x1F), "{imm12:#x}");
        }
    }

    /// 2026-11-27, fail closed: instructions that write SP in forms the
    /// decoder reads no change from are answered `SpUnread`, not `Other`,
    /// and instructions that only use SP as a base or a source are not.
    #[test]
    fn unread_writes_of_the_stack_pointer_are_refused() {
        for (bytes, what) in [
            ([0xA7u8, 0xF1, 0x08, 0x0D], "sub.w sp, r7, #8"),
            ([0x07, 0xF1, 0x08, 0x0D], "add.w sp, r7, #8"),
            ([0xC2, 0xF2, 0x00, 0x0D], "movt sp, #0x2000"),
            ([0x6F, 0xF0, 0x00, 0x0D], "mvn sp, #0"),
            ([0x4F, 0xEA, 0x00, 0x0D], "mov.w sp, r0"),
            ([0x00, 0xEA, 0x01, 0x0D], "and.w sp, r0, r1"),
            ([0x00, 0xFA, 0x01, 0xFD], "lsl.w sp, r0, r1"),
            ([0x00, 0xFB, 0x01, 0xFD], "mul sp, r0, r1"),
            ([0x90, 0xFB, 0xF1, 0xFD], "sdiv sp, r0, r1"),
            ([0xA0, 0xFB, 0x01, 0xD2], "umull sp, r2, r0, r1"),
            ([0xEF, 0xF3, 0x08, 0x8D], "mrs sp, msp"),
            ([0x5D, 0xF8, 0x04, 0x09], "ldr r0, [sp], #-4"),
            ([0x5D, 0xF8, 0x04, 0x0F], "ldr r0, [sp, #4]!"),
            ([0x4D, 0xF8, 0x04, 0x0B], "str r0, [sp], #4"),
            ([0x4D, 0xF8, 0x04, 0x0F], "str r0, [sp, #4]!"),
            ([0x10, 0xF8, 0x01, 0xD0], "ldrb.w sp, [r0, r1]"),
            ([0xAD, 0xE8, 0x03, 0x00], "stmia sp!, {r0, r1}"),
            ([0x3D, 0xE9, 0x03, 0x00], "ldmdb sp!, {r0, r1}"),
            ([0x90, 0xE8, 0x03, 0x20], "ldmia r0, {r0, r1, sp}"),
            ([0xFD, 0xE8, 0x02, 0x01], "ldrd r0, r1, [sp], #8"),
            ([0xED, 0xE9, 0x02, 0x01], "strd r0, r1, [sp, #8]!"),
            ([0xD0, 0xE9, 0x00, 0xD1], "ldrd sp, r1, [r0]"),
            ([0x50, 0xE8, 0x00, 0xDF], "ldrex sp, [r0]"),
            ([0x41, 0xE8, 0x00, 0x2D], "strex sp, r2, [r1]"),
            ([0xAD, 0xEC, 0x02, 0x0A], "vstmia sp!, {s0, s1}"),
            ([0x3D, 0xED, 0x02, 0x0A], "vldmdb sp!, {s0, s1}"),
            ([0x10, 0xEE, 0x10, 0xDA], "vmov sp, s0"),
            ([0x51, 0xEC, 0x10, 0xDB], "vmov sp, r1, d0"),
        ] {
            assert_eq!(one(&bytes), Effect::SpUnread, "{what}");
        }
        for (bytes, what) in [
            ([0x0Du8, 0xF1, 0x08, 0x00], "add.w r0, sp, #8"),
            ([0xCD, 0xF8, 0x04, 0x00], "str.w r0, [sp, #4]"),
            ([0xDD, 0xF8, 0x04, 0x00], "ldr.w r0, [sp, #4]"),
            ([0xCD, 0xE9, 0x02, 0x01], "strd r0, r1, [sp, #8]"),
            ([0xDD, 0xE9, 0x02, 0x01], "ldrd r0, r1, [sp, #8]"),
            ([0x8D, 0xE8, 0x03, 0x00], "stm sp, {r0, r1}"),
            ([0xBD, 0xF1, 0x08, 0x0F], "cmp.w sp, #8"),
            ([0x1D, 0xF0, 0x07, 0x0F], "tst.w sp, #7"),
            ([0x4D, 0xF8, 0x00, 0x10], "str.w r1, [sp, r0]"),
            ([0x8D, 0xED, 0x02, 0x0A], "vstr s0, [sp, #8]"),
            ([0x0D, 0xEE, 0x10, 0xDA], "vmov s26, sp"),
            ([0xF1, 0xEE, 0x10, 0xFA], "vmrs APSR_nzcv, fpscr"),
            ([0x6D, 0x46, 0x00, 0x00], "mov r5, sp"),
        ] {
            assert_ne!(one(&bytes), Effect::SpUnread, "{what}");
            assert_ne!(one(&bytes), Effect::SpDynamic, "{what}");
        }
    }

    #[test]
    fn bl_to_self_is_a_call_to_the_entry() {
        assert_eq!(one(&[0xFF, 0xF7, 0xFE, 0xFF]), Effect::Call(0x1000)); // bl . (imm -4)
    }

    #[test]
    fn privileged_and_floating_point_classes() {
        assert_eq!(one(&[0x72, 0xB6]), Effect::Cps); // cpsid i
        assert_eq!(one(&[0x62, 0xB6]), Effect::Cps); // cpsie i
        assert_eq!(one(&[0x80, 0xF3, 0x11, 0x88]), Effect::Msr(0x11)); // msr BASEPRI, r0
        assert_eq!(one(&[0xEF, 0xF3, 0x10, 0x80]), Effect::Mrs(0x10)); // mrs r0, PRIMASK
        assert_eq!(one(&[0x9D, 0xED, 0x00, 0x0A]), Effect::Fp(FpClass::Move)); // vldr s0, [sp]
        assert_eq!(one(&[0x8D, 0xED, 0x00, 0x0A]), Effect::Fp(FpClass::Move)); // vstr s0, [sp]
        assert_eq!(one(&[0x00, 0xEE, 0x10, 0x0A]), Effect::Fp(FpClass::Move)); // vmov s0, r0
        assert_eq!(one(&[0x10, 0xEE, 0x10, 0x0A]), Effect::Fp(FpClass::Move)); // vmov r0, s0
        assert_eq!(one(&[0xB0, 0xEE, 0x40, 0x0A]), Effect::Fp(FpClass::Move)); // vmov.f32 s0, s0
        assert_eq!(one(&[0xB1, 0xEE, 0x40, 0x0A]), Effect::Fp(FpClass::Arith)); // vneg.f32 s0, s0
        assert_eq!(one(&[0x30, 0xEE, 0x01, 0x0A]), Effect::Fp(FpClass::Arith)); // vadd.f32 s0, s0, s2
        assert_eq!(one(&[0xB4, 0xEE, 0x41, 0x0A]), Effect::Fp(FpClass::Arith)); // vcmp.f32 s0, s2
        assert_eq!(one(&[0xE1, 0xEE, 0x10, 0x0A]), Effect::Fp(FpClass::FpscrWrite)); // vmsr fpscr, r0
        assert_eq!(one(&[0xF1, 0xEE, 0x10, 0x0A]), Effect::Fp(FpClass::FpscrRead)); // vmrs r0, fpscr
    }

    #[test]
    fn the_written_out_bit_count_is_count_ones_on_every_halfword() {
        for v in 0u32..=0xFFFF {
            assert_eq!(ones16(v), v.count_ones(), "{v:#x}");
        }
    }

    #[test]
    fn modified_immediates_expand() {
        assert_eq!(thumb_expand_imm(0x018), 0x18);
        assert_eq!(thumb_expand_imm(0x1FF), 0x00FF_00FF);
        assert_eq!(thumb_expand_imm(0x4FF), 0x7F80_0000); // 0xFF rotated right by 9
    }
}
