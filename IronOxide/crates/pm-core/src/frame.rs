//! The DCM link's frames (design PD-11, "Parameters and frames"; PD-O6
//! closed by task 15.1).
//!
//! A frame is 32 bytes each way, the same form in both directions:
//!
//! ```text
//! byte  0      1      2      3      4 .. 27        28 29   30 31
//!       seq    kind   part   len    payload (24)   0  0    CRC-16
//! ```
//!
//! - `seq`: the sender's count of frames, modulo 256. An answer carries in
//!   its payload the `seq` of the command it answers.
//! - `kind`: what the frame is ([`kind`]). A frame of a kind this module
//!   does not know is not a frame.
//! - `part`: for a parameter set, which travels as [`PARTS`] frames of six
//!   words (the last of five), the number of this part, from 0; otherwise 0.
//! - `len`: the bytes of payload in use, at most [`PAYLOAD`]; the rest of
//!   the payload is zero, and a frame where it is not is not a frame.
//! - bytes 28 and 29 are zero.
//! - the CRC is CRC-16/CCITT-FALSE (polynomial 0x1021, start 0xFFFF, no
//!   reflection, no final XOR) over bytes 0 to 29, high byte first. It
//!   finds every error of one, two or three bits in a frame.
//!
//! Words are sent least significant byte first.
//!
//! Nothing here trusts a frame: [`decode`] returns a frame only for 32
//! bytes that are one, and the parameter set a sequence of parts gives is
//! judged by `validate` like any other (PM19.3).

use crate::validate::WORDS;
// The App_Prover's view operator (`@`) in the contracts below.
#[cfg(creusot)]
#[allow(unused_imports)]
use creusot_std::model::View;
#[cfg(creusot)]
#[allow(unused_imports)]
use creusot_std::prelude::Int;

/// The bytes of a frame, and of its payload.
pub const FRAME: usize = 32;
pub const PAYLOAD: usize = 24;
/// The words a part of a parameter set carries, and the parts of a set.
pub const PART_WORDS: usize = 6;
pub const PARTS: usize = 5;

/// The kinds of frame. Below 0x80 the DCM's, from 0x80 the PG's.
pub mod kind {
    /// The poll of a period with nothing to say.
    pub const POLL: u8 = 0x01;
    /// Who are you: answered by [`IDENTITY`] (PM20.3).
    pub const INTERROGATE: u8 = 0x02;
    /// Send the parameters in force: answered by [`PARAM_PART`] frames.
    pub const READ_PARAMETERS: u8 = 0x03;
    /// One part of a set of permanent parameters.
    pub const PROGRAM_PART: u8 = 0x10;
    /// One part of a set of temporary parameters: the last part starts
    /// temporary pacing.
    pub const TEMPORARY_PART: u8 = 0x11;
    pub const END_TEMPORARY: u8 = 0x12;
    pub const PACE_NOW: u8 = 0x13;
    /// The simulated battery level and activity value (PM29.2, PM29.3):
    /// one byte.
    pub const SET_BATTERY: u8 = 0x20;
    pub const SET_ACTIVITY: u8 = 0x21;
    /// The magnet response programmed on (1) or off (0): one byte (PM8.4).
    pub const SET_MAGNET_RESPONSE: u8 = 0x22;
    /// The other commands of PM20.1, which stages E3 to E5 give meaning to.
    pub const SET_CLOCK: u8 = 0x30;
    pub const RESET_HISTOGRAMS: u8 = 0x31;
    pub const EGRAM_START: u8 = 0x32;
    pub const EGRAM_STOP: u8 = 0x33;
    pub const THRESHOLD_START: u8 = 0x34;
    pub const THRESHOLD_STOP: u8 = 0x35;
    pub const MEASURE: u8 = 0x36;
    /// Stage E5 (task 22.4). One part of a record of `diag`'s: the
    /// record's kind and the part, a byte each; answered by a [`RECORD`].
    /// [`MEASURE`] reads the measured data so.
    pub const READ_RECORD: u8 = 0x37;
    /// The Marker transmission started and stopped (PM20.1): while it
    /// runs every Marker comes as a [`RECORD`].
    pub const MARKERS_START: u8 = 0x38;
    pub const MARKERS_STOP: u8 = 0x39;
    /// A trend begun now: the duration and the content, a byte each (PM18.3).
    pub const TREND_START: u8 = 0x3A;
    /// A word of the implant data (0) or a measured value's stand-in (1)
    /// supplied: which, the index, and the value as four bytes.
    pub const SET_DATA: u8 = 0x3B;

    /// Nothing to answer.
    pub const IDLE: u8 = 0x80;
    /// The State, the Mode in force, the battery level and the answer to
    /// the last command.
    pub const STATUS: u8 = 0x81;
    /// Model and serial number, a word each.
    pub const IDENTITY: u8 = 0x82;
    /// One part of the parameters in force.
    pub const PARAM_PART: u8 = 0x83;
    /// A histogram, trend or Marker record (stage E5).
    pub const RECORD: u8 = 0x84;
}

/// The parts a RECORD frame may name: the most a record has (the trend's).
pub const RECORD_PARTS: usize = 65;

/// For the App_Prover: [`kind_known`], of the kind as a number.
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn known(k: creusot_std::prelude::Int) -> bool {
    use creusot_std::macros::pearlite;
    pearlite! { (0x01 <= k && k <= 0x03) || (0x10 <= k && k <= 0x13) || (0x20 <= k && k <= 0x22) || (0x30 <= k && k <= 0x3B) || (0x80 <= k && k <= 0x84) }
}

/// For the App_Prover: what a frame is besides its CRC: a kind that is
/// one, a length within the payload, a part its kind may have.
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn well_formed(f: Frame) -> bool {
    use creusot_std::macros::pearlite;
    pearlite! { known(f.kind@) && f.len@ <= 24 && f.part@ < (if f.kind@ == 0x84 { 65 } else { 5 }) }
}

/// Whether `k` is a kind of frame.
#[cfg_attr(creusot, creusot_std::macros::ensures(result == known(k@)))]
#[inline(always)]
#[must_use]
pub const fn kind_known(k: u8) -> bool {
    matches!(k, 0x01..=0x03 | 0x10..=0x13 | 0x20..=0x22 | 0x30..=0x3B | 0x80..=0x84)
}

/// A frame, decoded.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Frame {
    pub seq: u8,
    pub kind: u8,
    pub part: u8,
    /// The bytes of `payload` in use; the others are zero.
    pub len: u8,
    pub payload: [u8; PAYLOAD],
}

impl Frame {
    /// A frame of `kind` without payload.
    #[cfg_attr(creusot, creusot_std::macros::ensures(result.seq == seq && result.kind == kind && result.part@ == 0 && result.len@ == 0))]
    #[inline(always)]
    #[must_use]
    pub const fn empty(seq: u8, kind: u8) -> Frame {
        Frame { seq, kind, part: 0, len: 0, payload: [0; PAYLOAD] }
    }
}

/// CRC-16/CCITT-FALSE over the first 30 bytes of a frame.
#[inline(always)]
#[must_use]
pub fn crc16(bytes: &[u8; FRAME]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    let mut i = 0;
    #[cfg_attr(creusot, creusot_std::macros::variant(30 - i@))]
    #[cfg_attr(creusot, creusot_std::macros::invariant(i@ <= 30))]
    while i < 30 {
        crc ^= (bytes[i] as u16) << 8;
        let mut bit = 0;
        #[cfg_attr(creusot, creusot_std::macros::variant(8 - bit@))]
        #[cfg_attr(creusot, creusot_std::macros::invariant(bit@ <= 8))]
        while bit < 8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
            bit += 1;
        }
        i += 1;
    }
    crc
}

/// The 32 bytes of `f`. A `len` above [`PAYLOAD`] is sent as [`PAYLOAD`],
/// and the payload beyond `len` as zero, so that what is encoded decodes.
///
/// Contract (2026-11-25): the header and the payload in use are the
/// frame's, byte for byte; the payload beyond the length and bytes 28 and
/// 29 are zero.
#[cfg_attr(creusot, creusot_std::macros::ensures(result@[0] == f.seq && result@[1] == f.kind && result@[2] == f.part && result@[3]@ == (if f.len@ > 24 { 24 } else { f.len@ }) && result@[28]@ == 0 && result@[29]@ == 0))]
#[cfg_attr(creusot, creusot_std::macros::ensures(forall<i: Int> 0 <= i && i < 24 ==> (i < f.len@ ==> result@[4 + i] == f.payload@[i]) && (i >= f.len@ ==> result@[4 + i]@ == 0)))]
#[inline(always)]
#[must_use]
pub fn encode(f: &Frame) -> [u8; FRAME] {
    let len = if f.len as usize > PAYLOAD { PAYLOAD } else { f.len as usize };
    let mut b = [0u8; FRAME];
    b[0] = f.seq;
    b[1] = f.kind;
    b[2] = f.part;
    b[3] = len as u8;
    let mut i = 0;
    #[cfg_attr(creusot, creusot_std::macros::variant(PAYLOAD@ - i@))]
    #[cfg_attr(creusot, creusot_std::macros::invariant(i@ <= 24 && b@.len() == 32 && len@ <= 24))]
    #[cfg_attr(creusot, creusot_std::macros::invariant(b@[0] == f.seq && b@[1] == f.kind && b@[2] == f.part && b@[3]@ == len@ && b@[28]@ == 0 && b@[29]@ == 0))]
    #[cfg_attr(creusot, creusot_std::macros::invariant(forall<j: Int> 0 <= j && j < 24 ==> (j < i@ && j < len@ ==> b@[4 + j] == f.payload@[j]) && (j >= i@ || j >= len@ ==> b@[4 + j]@ == 0)))]
    while i < PAYLOAD {
        if i < len {
            b[4 + i] = f.payload[i];
        }
        i += 1;
    }
    let crc = crc16(&b);
    b[30] = (crc >> 8) as u8;
    b[31] = (crc & 0xFF) as u8;
    b
}

/// The frame `b` is, or `None`: a wrong CRC, an unknown kind, a length
/// above [`PAYLOAD`], a part that no set has, or a byte that must be zero
/// and is not.
///
/// Contract (2026-11-25): a frame returned is well formed (a known kind,
/// a length within the payload, a part its kind may have); its header and
/// its payload in use are the bytes', its payload beyond the length is
/// zero, and so were the bytes there and bytes 28 and 29. And bytes that
/// are well formed in all of this are refused only by the CRC: with
/// [`encode`]'s contract, what is encoded of a well-formed frame decodes
/// to it if the CRC of the same bytes is the same twice (it is a function
/// of them; the round trip itself is tested over every kind, part and
/// length, `tests/frame.rs`).
#[cfg_attr(creusot, creusot_std::macros::ensures(match result { Some(f) => well_formed(f) && f.seq == b@[0] && f.kind == b@[1] && f.part == b@[2] && f.len == b@[3] && b@[28]@ == 0 && b@[29]@ == 0, None => true }))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match result { Some(f) => forall<i: Int> 0 <= i && i < 24 ==> (i < f.len@ ==> f.payload@[i] == b@[4 + i]) && (i >= f.len@ ==> f.payload@[i]@ == 0 && b@[4 + i]@ == 0), None => true }))]
#[inline(always)]
#[must_use]
pub fn decode(b: &[u8; FRAME]) -> Option<Frame> {
    let crc = crc16(b);
    if b[30] != (crc >> 8) as u8 || b[31] != (crc & 0xFF) as u8 {
        return None;
    }
    let len = b[3] as usize;
    // A RECORD's part is the record's (a trend has 65: `record::parts`); any
    // other frame's is a part of a parameter set.
    let parts = if b[1] == kind::RECORD { RECORD_PARTS } else { PARTS };
    if !kind_known(b[1]) || len > PAYLOAD || b[2] as usize >= parts || b[28] != 0 || b[29] != 0 {
        return None;
    }
    let mut payload = [0u8; PAYLOAD];
    let mut i = 0;
    #[cfg_attr(creusot, creusot_std::macros::variant(PAYLOAD@ - i@))]
    #[cfg_attr(creusot, creusot_std::macros::invariant(i@ <= 24 && payload@.len() == 24 && len@ <= 24))]
    #[cfg_attr(creusot, creusot_std::macros::invariant(forall<j: Int> 0 <= j && j < 24 ==> (j < i@ && j < len@ ==> payload@[j] == b@[4 + j]) && (j < i@ && j >= len@ ==> b@[4 + j]@ == 0) && (j >= i@ || j >= len@ ==> payload@[j]@ == 0)))]
    while i < PAYLOAD {
        if i < len {
            payload[i] = b[4 + i];
        } else if b[4 + i] != 0 {
            return None;
        }
        i += 1;
    }
    Some(Frame { seq: b[0], kind: b[1], part: b[2], len: b[3], payload })
}

/// The words part `part` of a set carries: six, or five in the last.
#[cfg_attr(creusot, creusot_std::macros::ensures((part@ < 4 ==> result@ == 6) && (part@ == 4 ==> result@ == 5) && (part@ > 4 ==> result@ == 0)))]
#[inline(always)]
#[must_use]
pub const fn words_in_part(part: usize) -> usize {
    if part < PARTS - 1 {
        PART_WORDS
    } else if part == PARTS - 1 {
        WORDS - PART_WORDS * (PARTS - 1)
    } else {
        0
    }
}

/// Part `part` of the set `words` as a frame of `kind`
/// ([`kind::PROGRAM_PART`], [`kind::TEMPORARY_PART`] or
/// [`kind::PARAM_PART`]); `None` for a part the set does not have.
#[cfg_attr(creusot, creusot_std::macros::ensures(match result { Some(f) => f.seq == seq && f.kind == kind && f.part == part && part@ < 5, None => part@ >= 5 }))]
#[inline(always)]
#[must_use]
pub fn set_part(seq: u8, kind: u8, words: &[u32; WORDS], part: u8) -> Option<Frame> {
    let p = part as usize;
    if p >= PARTS {
        return None;
    }
    let n = words_in_part(p);
    let mut payload = [0u8; PAYLOAD];
    let mut k = 0;
    #[cfg_attr(creusot, creusot_std::macros::variant(PART_WORDS@ - k@))]
    #[cfg_attr(creusot, creusot_std::macros::invariant(k@ <= 6 && payload@.len() == 24 && n@ <= 6 && p@ < 5 && (p@ == 4 ==> n@ == 5)))]
    while k < PART_WORDS {
        if k < n {
            let w = words[PART_WORDS * p + k];
            payload[4 * k] = (w & 0xFF) as u8;
            payload[4 * k + 1] = ((w >> 8) & 0xFF) as u8;
            payload[4 * k + 2] = ((w >> 16) & 0xFF) as u8;
            payload[4 * k + 3] = ((w >> 24) & 0xFF) as u8;
        }
        k += 1;
    }
    Some(Frame { seq, kind, part, len: (4 * n) as u8, payload })
}

/// A parameter set being put together from its parts (`link`, and the
/// DCM_Stub for the PG's answer). All zero: no part yet.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Assembly {
    /// Bit `p`: part `p` has come.
    pub have: u8,
    /// Always zero (no padding byte: a Partition static is all zero bytes).
    pub reserved: [u8; 3],
    pub words: [u32; WORDS],
}

impl Assembly {
    pub const EMPTY: Assembly = Assembly { have: 0, reserved: [0; 3], words: [0; WORDS] };

    /// One part. Part 0 starts a set again, whatever had come. A part
    /// whose length is not its part's is dropped, with what had come.
    /// Returns the set when the part completes it, and is then empty.
    #[inline(always)]
    #[must_use]
    pub fn accept(&mut self, f: &Frame) -> Option<[u32; WORDS]> {
        let p = f.part as usize;
        if p >= PARTS || f.len as usize != 4 * words_in_part(p) {
            self.have = 0;
            return None;
        }
        if p == 0 {
            self.have = 0;
        }
        let n = words_in_part(p);
        let mut k = 0;
        #[cfg_attr(creusot, creusot_std::macros::variant(PART_WORDS@ - k@))]
        #[cfg_attr(creusot, creusot_std::macros::invariant(k@ <= 6 && (self.words)@.len() == 29 && n@ <= 6 && p@ < 5 && (p@ == 4 ==> n@ == 5)))]
        while k < PART_WORDS {
            if k < n {
                let b = &f.payload;
                // Least significant byte first. As a sum of products, which
                // the App_Prover can bound (at most 2^32 - 1), not as shifts.
                self.words[PART_WORDS * p + k] =
                    b[4 * k] as u32 + b[4 * k + 1] as u32 * 0x100 + b[4 * k + 2] as u32 * 0x1_0000 + b[4 * k + 3] as u32 * 0x100_0000;
            }
            k += 1;
        }
        const BIT: [u8; PARTS] = [1, 2, 4, 8, 16];
        self.have |= BIT[p];
        if self.have == 31 {
            self.have = 0;
            Some(self.words)
        } else {
            None
        }
    }
}
