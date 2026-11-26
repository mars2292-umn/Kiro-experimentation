//! Host tests of the DCM link's frames (task 15.1): round trips and
//! corruption. An integration test, so that the crate itself names no
//! `std` (rsk's check R6.1).
#![forbid(unsafe_code)]

use pm_core::frame::*;
use pm_core::{NOMINAL_WORDS, WORDS};

const KINDS: [u8; 27] = [
    kind::POLL, kind::INTERROGATE, kind::READ_PARAMETERS, kind::PROGRAM_PART, kind::TEMPORARY_PART, kind::END_TEMPORARY, kind::PACE_NOW,
    kind::SET_BATTERY, kind::SET_ACTIVITY, kind::SET_CLOCK, kind::RESET_HISTOGRAMS, kind::EGRAM_START, kind::EGRAM_STOP,
    kind::THRESHOLD_START, kind::THRESHOLD_STOP, kind::MEASURE, kind::IDLE, kind::STATUS, kind::IDENTITY, kind::PARAM_PART, kind::RECORD, kind::SET_MAGNET_RESPONSE,
    kind::READ_RECORD, kind::MARKERS_START, kind::MARKERS_STOP, kind::TREND_START, kind::SET_DATA,
];

fn frame(seq: u8, k: u8, part: u8, len: u8) -> Frame {
    let mut payload = [0u8; PAYLOAD];
    for (i, b) in payload.iter_mut().enumerate().take(len as usize) {
        *b = (seq as usize * 7 + i * 13 + 1) as u8;
    }
    Frame { seq, kind: k, part, len, payload }
}

/// The CRC is CRC-16/CCITT-FALSE: its check value over "123456789" is
/// 0x29B1. The frame's CRC covers 30 bytes, so the check string is put at
/// the front of a block whose CRC is computed here bit by bit as well.
#[test]
fn the_crc_is_ccitt_false() {
    fn reference(data: &[u8]) -> u16 {
        let mut crc = 0xFFFFu16;
        for &byte in data {
            for bit in (0..8).rev() {
                let top = (crc >> 15) & 1;
                let inb = ((byte >> bit) & 1) as u16;
                crc <<= 1;
                if top ^ inb == 1 {
                    crc ^= 0x1021;
                }
            }
        }
        crc
    }
    assert_eq!(reference(b"123456789"), 0x29B1, "the reference is the published algorithm");
    let mut b = [0u8; FRAME];
    for (i, x) in b.iter_mut().enumerate() {
        *x = (i * 37 + 5) as u8;
    }
    assert_eq!(crc16(&b), reference(&b[..30]));
    b[..9].copy_from_slice(b"123456789");
    assert_eq!(crc16(&b), reference(&b[..30]));
    // Bytes 30 and 31 are not covered.
    let before = crc16(&b);
    b[30] ^= 0xFF;
    b[31] ^= 0xFF;
    assert_eq!(crc16(&b), before);
}

#[test]
fn every_kind_and_length_round_trips() {
    let mut n = 0;
    for (s, &k) in KINDS.iter().enumerate() {
        assert!(kind_known(k));
        for len in 0..=PAYLOAD as u8 {
            for part in 0..PARTS as u8 {
                let f = frame(s as u8 * 9, k, part, len);
                let bytes = encode(&f);
                assert_eq!(decode(&bytes), Some(f), "kind {k:#x} len {len} part {part}");
                assert_eq!(&bytes[28..30], &[0, 0]);
                n += 1;
            }
        }
    }
    assert_eq!(n, 27 * 25 * 5);
    // The kinds are exactly these 27 (21 until task 16.4 added the magnet response, 22 until task 22.4 added the five of the records).
    assert_eq!((0..=255u8).filter(|k| kind_known(*k)).count(), 27);
    // A RECORD names a part of a record, up to the trend's 65; no other frame a part beyond a set's.
    for part in [5u8, 64] {
        let f = frame(1, kind::RECORD, part, 24);
        assert_eq!(decode(&encode(&f)), Some(f));
        assert_eq!(decode(&encode(&frame(1, kind::PARAM_PART, part, 24))), None);
    }
    assert_eq!(decode(&encode(&frame(1, kind::RECORD, 65, 24))), None);
}

#[test]
fn what_is_encoded_decodes_whatever_the_frame_holds() {
    // A length above the payload is sent as the payload's; payload beyond
    // the length is sent as zero.
    let mut f = frame(3, kind::STATUS, 0, 24);
    f.len = 200;
    let d = decode(&encode(&f)).unwrap();
    assert_eq!((d.len, d.payload), (24, f.payload));
    let mut f = frame(3, kind::STATUS, 0, 24);
    f.len = 5;
    let d = decode(&encode(&f)).unwrap();
    assert_eq!(d.len, 5);
    assert_eq!(&d.payload[..5], &f.payload[..5]);
    assert!(d.payload[5..].iter().all(|b| *b == 0));
}

/// Every error of one bit and of two bits anywhere in the 32 bytes is
/// refused, for several frames; and a sample of errors of three bits.
#[test]
fn corruption_is_refused() {
    for f in [frame(1, kind::POLL, 0, 0), frame(200, kind::PROGRAM_PART, 4, 16), frame(77, kind::STATUS, 0, 24), Frame::empty(0, kind::IDLE)] {
        let good = encode(&f);
        assert_eq!(decode(&good), Some(f));
        let mut refused = 0u32;
        for i in 0..256 {
            let mut one = good;
            one[i / 8] ^= 1 << (i % 8);
            assert_eq!(decode(&one), None, "bit {i}");
            for j in (i + 1)..256 {
                let mut two = one;
                two[j / 8] ^= 1 << (j % 8);
                assert_eq!(decode(&two), None, "bits {i} and {j}");
                refused += 1;
            }
        }
        assert_eq!(refused, 256 * 255 / 2);
        let mut x = 12_345u32;
        for _ in 0..20_000 {
            let mut three = good;
            let mut bits = [0usize; 3];
            for b in &mut bits {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *b = (x >> 8) as usize % 256;
            }
            if bits[0] == bits[1] || bits[1] == bits[2] || bits[0] == bits[2] {
                continue;
            }
            for b in bits {
                three[b / 8] ^= 1 << (b % 8);
            }
            assert_eq!(decode(&three), None, "bits {bits:?}");
        }
    }
}

/// What is not a frame though its CRC is right.
#[test]
fn a_frame_with_a_right_crc_and_a_wrong_field_is_refused() {
    let fix = |mut b: [u8; FRAME]| {
        let crc = crc16(&b);
        b[30] = (crc >> 8) as u8;
        b[31] = crc as u8;
        b
    };
    let good = encode(&frame(9, kind::STATUS, 0, 10));
    for (i, v, what) in [(1usize, 0x7Fu8, "an unknown kind"), (1, 0x00, "kind 0"), (3, 25, "a length above the payload"), (2, 5, "a part no set has"),
                         (28, 1, "byte 28"), (29, 1, "byte 29"), (4 + 10, 1, "payload beyond the length"), (27, 9, "the last payload byte")] {
        let mut b = good;
        b[i] = v;
        assert_eq!(decode(&fix(b)), None, "{what}");
    }
    assert!(decode(&fix(good)).is_some());
    // All zero and all ones are not frames (an idle or a stuck line).
    assert_eq!(decode(&[0; FRAME]), None);
    assert_eq!(decode(&[0xFF; FRAME]), None);
}

fn a_set() -> [u32; WORDS] {
    let mut w = NOMINAL_WORDS;
    for (i, x) in w.iter_mut().enumerate() {
        *x = x.wrapping_add(0x0101_0000 * i as u32 + 0x8000_0000 * (i as u32 % 2));
    }
    w
}

/// A set travels as five parts of 24, 24, 24, 24 and 16 bytes and comes
/// back the same, through the frames' bytes.
#[test]
fn a_parameter_set_round_trips_through_its_parts() {
    assert_eq!((0..PARTS).map(words_in_part).collect::<Vec<_>>(), [6, 6, 6, 6, 5]);
    assert_eq!((0..PARTS).map(words_in_part).sum::<usize>(), WORDS);
    assert_eq!(words_in_part(PARTS), 0);
    for words in [NOMINAL_WORDS, a_set(), [0; WORDS], [u32::MAX; WORDS]] {
        let mut a = Assembly::EMPTY;
        let mut out = None;
        for p in 0..PARTS as u8 {
            assert_eq!(out, None);
            let f = set_part(p, kind::PROGRAM_PART, &words, p).unwrap();
            assert_eq!(f.len as usize, 4 * words_in_part(p as usize));
            let f = decode(&encode(&f)).unwrap();
            out = a.accept(&f);
        }
        assert_eq!(out, Some(words));
        assert_eq!(a.have, 0, "empty again");
    }
    assert_eq!(set_part(0, kind::PROGRAM_PART, &NOMINAL_WORDS, PARTS as u8), None);
}

/// The parts may come in any order after part 0; part 0 starts again; a
/// part twice is the later one; a part of a wrong length drops the set.
#[test]
fn assembly_of_parts_out_of_order_repeated_and_broken() {
    let (x, y) = (a_set(), NOMINAL_WORDS);
    let part = |w: &[u32; WORDS], p: u8| set_part(0, kind::PROGRAM_PART, w, p).unwrap();
    let mut a = Assembly::EMPTY;
    for p in [0u8, 3, 1, 4] {
        assert_eq!(a.accept(&part(&x, p)), None);
    }
    assert_eq!(a.accept(&part(&x, 2)), Some(x));

    // Without part 0 first the set still completes when all five have come.
    let mut a = Assembly::EMPTY;
    for p in [4u8, 3, 2, 1] {
        assert_eq!(a.accept(&part(&x, p)), None);
    }
    // Part 0 starts again: the four parts before it are forgotten.
    assert_eq!(a.accept(&part(&x, 0)), None);
    for p in [1u8, 2, 3] {
        assert_eq!(a.accept(&part(&x, p)), None);
    }
    assert_eq!(a.accept(&part(&x, 4)), Some(x));

    // A part twice: the later one's words.
    let mut a = Assembly::EMPTY;
    let _ = a.accept(&part(&x, 0));
    let _ = a.accept(&part(&x, 1));
    let _ = a.accept(&part(&y, 1));
    let _ = a.accept(&part(&x, 2));
    let _ = a.accept(&part(&x, 3));
    let got = a.accept(&part(&x, 4)).unwrap();
    assert_eq!(&got[6..12], &y[6..12]);
    assert_eq!((&got[..6], &got[12..]), (&x[..6], &x[12..]));

    // A wrong length: dropped, and the set with it.
    let mut a = Assembly::EMPTY;
    let _ = a.accept(&part(&x, 0));
    let mut short = part(&x, 1);
    short.len = 20;
    assert_eq!(a.accept(&short), None);
    assert_eq!(a.have, 0);
    let mut long = part(&x, 4);
    long.len = 24;
    assert_eq!(a.accept(&long), None);
}

/// The round trip over every header (2026-11-25): each of the 256 kinds
/// with each of the 256 parts and every length from 0 to 25 and 255, the
/// payload full. What is encoded decodes exactly when the kind is one of
/// the 27 and the part is one its kind may have, and then to the frame
/// with its length within the payload and nothing beyond the length.
/// `decode`'s contract (the App_Prover's) states the fields and the
/// payload byte for byte; the CRC's part of the round trip is here.
#[test]
fn the_round_trip_over_every_kind_part_and_length() {
    let mut decoded = 0u32;
    for k in 0..=255u8 {
        let known = KINDS.contains(&k);
        assert_eq!(kind_known(k), known, "kind {k:#x}");
        for part in 0..=255u8 {
            let fits = (part as usize) < if k == kind::RECORD { RECORD_PARTS } else { PARTS };
            for len in (0..=25u8).chain([255]) {
                let f = Frame { seq: k ^ part, kind: k, part, len, payload: [0xA5; PAYLOAD] };
                let got = decode(&encode(&f));
                if known && fits {
                    let n = (len as usize).min(PAYLOAD);
                    let mut want = Frame { len: n as u8, ..f };
                    want.payload[n..].fill(0);
                    assert_eq!(got, Some(want), "kind {k:#x} part {part} len {len}");
                    decoded += 1;
                } else {
                    assert_eq!(got, None, "kind {k:#x} part {part} len {len}");
                }
            }
        }
    }
    assert_eq!(decoded, (26 * PARTS as u32 + RECORD_PARTS as u32) * 27);
}

/// Every frame `decode` returns encodes to the bytes it came from: over
/// bytes that are frames by construction and over a sample of 200,000
/// blocks with a right CRC and anything else.
#[test]
fn what_is_decoded_encodes_to_the_same_bytes() {
    let mut x = 2_463_534_242u32;
    let mut frames = 0u32;
    for n in 0..200_000u32 {
        let mut b = [0u8; FRAME];
        for byte in b.iter_mut().take(28) {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            *byte = (x >> 11) as u8;
        }
        // Half of them with a header that can be a frame's, so that the
        // payload's rule decides.
        if n % 2 == 0 {
            b[1] = KINDS[(x >> 3) as usize % KINDS.len()];
            b[2] %= 5;
            b[3] %= 26;
            let len = b[3] as usize;
            if n % 4 == 0 && len < PAYLOAD {
                b[4 + len..28].fill(0);
            }
        }
        let crc = crc16(&b);
        b[30] = (crc >> 8) as u8;
        b[31] = crc as u8;
        if let Some(f) = decode(&b) {
            assert_eq!(encode(&f), b, "block {n}");
            frames += 1;
        }
    }
    assert!(frames > 40_000, "{frames}");
}
