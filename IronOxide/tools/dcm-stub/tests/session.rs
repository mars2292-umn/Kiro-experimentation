//! The DCM_Stub against the link's own answers (task 15.2): a model of the
//! bridge (`tools/pm-bridge`'s messages and its queue) with
//! `pacemaker_comms::proto::answer` behind it as the pulse generator.
#![forbid(unsafe_code)]

use dcm_stub::{bridge, message, predict, reply, Carrier, Dcm, Reply, Reports, Session};
use pacemaker_comms::proto::{self, code, Link};
use pm_core::frame::{self, kind, FRAME};
use pm_core::{NOMINAL_WORDS, WORDS};

/// The bridge and the pulse generator behind it: every `receive` is one
/// period of `link` (one transfer), reported as the bridge reports it.
struct Rig {
    link: Link,
    /// What the pulse generator received in the last transfer.
    last: [u8; FRAME],
    queue: Vec<[u8; FRAME]>,
    idle: Option<[u8; FRAME]>,
    inbox: Vec<u8>,
    out: Vec<u8>,
    count: u8,
    /// Flips one bit of the byte numbered here in every frame toward the
    /// pulse generator.
    corrupt: Option<usize>,
    /// `control`'s counts, as its status carries them.
    serial: u32,
    refused: u32,
    verdict: (u8, u8),
}

impl Rig {
    fn new() -> Rig {
        Rig { link: Link::new(), last: [0; FRAME], queue: Vec::new(), idle: None, inbox: Vec::new(), out: Vec::new(), count: 0, corrupt: None, serial: 0, refused: 0, verdict: (0, 0) }
    }

    fn report(&mut self, flags: u8, bytes: &[u8; FRAME]) {
        let mut m = vec![bridge::START, self.count, flags, FRAME as u8, 0];
        m.extend_from_slice(bytes);
        let sum = m[1..].iter().fold(0u8, |a, b| a.wrapping_add(*b));
        m.push(sum);
        self.count = self.count.wrapping_add(1);
        self.out.extend(m);
    }

    fn transfer(&mut self) {
        let (flags, mut to_pg) = if !self.queue.is_empty() {
            (bridge::SENT_COMMAND, self.queue.remove(0))
        } else if let Some(i) = self.idle {
            (bridge::SENT_IDLE, i)
        } else {
            (0, [0; FRAME])
        };
        if let Some(at) = self.corrupt {
            to_pg[at] ^= 0x10;
        }
        // `link` answers the frame of the transfer before this one.
        let got = frame::decode(&self.last);
        let from_pg = frame::encode(&proto::answer(&mut self.link, got.as_ref()));
        self.control();
        self.last = to_pg;
        self.report(flags, &from_pg);
    }
}

impl Rig {
    /// `control` and the Kernel behind the Endpoints, as far as the link
    /// can tell: a message the validation function admits gets a verdict
    /// in the status; one it refuses is the fault of `comms`, which is
    /// restarted (its state all zero), and the status counts it.
    fn control(&mut self) {
        use pm_core::wire::{admitted, code, code_of, seq_of, verdict, Status};
        let Some(m) = proto::take_forward(&mut self.link) else { return };
        if !admitted(&m) {
            self.link = Link::new();
            self.refused += 1;
            self.serial += 1;
            self.verdict = (verdict::REFUSED, 0);
        } else if code_of(&m) != code::HEARD {
            self.serial += 1;
            self.verdict = (verdict::ACCEPTED, seq_of(&m));
        }
        let status = Status { therapy: 1, mode: 0x15, battery: 0, link_up: true, serial: self.serial, verdict: self.verdict.0, of: self.verdict.1, refused: self.refused };
        proto::status_in(&mut self.link, &status.words());
    }
}

impl Carrier for Rig {
    fn send(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.inbox.extend_from_slice(bytes);
        while self.inbox.len() >= bridge::MESSAGE {
            let m: Vec<u8> = self.inbox.drain(..bridge::MESSAGE).collect();
            assert_eq!(m[0], bridge::START);
            assert_eq!(m[34], m[1..34].iter().fold(0u8, |a, b| a.wrapping_add(*b)));
            let mut body = [0u8; FRAME];
            body.copy_from_slice(&m[2..34]);
            match m[1] {
                bridge::COMMAND => self.queue.push(body),
                bridge::IDLE => self.idle = Some(body),
                bridge::CLEAR => {
                    self.idle = None;
                    self.queue.clear();
                }
                bridge::LOOP => self.report(bridge::LOOPED, &body),
                other => panic!("message type {other:#x}"),
            }
        }
        Ok(())
    }

    fn receive(&mut self) -> std::io::Result<Vec<u8>> {
        if self.out.is_empty() {
            self.transfer();
        }
        Ok(std::mem::take(&mut self.out))
    }
}

#[test]
fn the_prediction_is_the_validation_function() {
    assert!(predict(&NOMINAL_WORDS));
    let mut w = NOMINAL_WORDS;
    w[pm_core::word::LRL] = 31; // not a value of Table 19-1
    assert!(!predict(&w));
    assert!(!predict(&NOMINAL_WORDS[..27]));
    for i in 0..WORDS {
        let mut w = NOMINAL_WORDS;
        w[i] = 0xFFFF_FFFF;
        assert_eq!(predict(&w), pm_core::validate(&w).is_some());
    }
}

#[test]
fn a_dcm_numbers_its_frames_and_a_set_is_five_parts() {
    let mut d = Dcm::default();
    assert_eq!(d.plain(kind::POLL).seq, 0);
    let parts = d.set(&NOMINAL_WORDS, false);
    assert_eq!(parts.len(), 5);
    for (p, f) in parts.iter().enumerate() {
        assert_eq!((f.seq, f.kind, f.part as usize), (1 + p as u8, kind::PROGRAM_PART, p));
    }
    assert_eq!(parts[4].len, 16);
    assert_eq!(d.set(&NOMINAL_WORDS, true)[0].kind, kind::TEMPORARY_PART);
    let b = d.byte(kind::SET_BATTERY, 2);
    assert_eq!((b.seq, b.len, b.payload[0]), (11, 1, 2));
}

#[test]
fn the_report_parser_finds_reports_in_noise_and_skips_a_wrong_sum() {
    let mut rig = Rig::new();
    rig.report(0, &[7; FRAME]);
    rig.report(bridge::SENT_IDLE, &[9; FRAME]);
    let good = std::mem::take(&mut rig.out);
    let mut stream = vec![0x11, 0x22];
    stream.extend_from_slice(&good[..38]);
    let mut bad = good[..38].to_vec();
    bad[20] ^= 1;
    stream.extend_from_slice(&bad);
    stream.extend_from_slice(&good[38..]);
    let mut p = Reports::default();
    // Fed in pieces of 5 bytes, as a serial port delivers.
    let mut got = Vec::new();
    for piece in stream.chunks(5) {
        got.extend(p.feed(piece));
    }
    assert_eq!(got.len(), 2);
    assert_eq!((got[0].count, got[0].flags, got[0].bytes), (0, 0, [7; FRAME]));
    assert_eq!((got[1].count, got[1].flags, got[1].bytes), (1, bridge::SENT_IDLE, [9; FRAME]));
    assert_eq!(p.bad_sums, 1);
    assert!(p.skipped >= 2);
}

#[test]
fn the_host_side_loops_back() {
    let mut s = Session::new(Rig::new());
    let bytes: [u8; FRAME] = core::array::from_fn(|i| i as u8 ^ 0x5A);
    let r = s.loop_back(&bytes).unwrap().expect("a report");
    assert_eq!((r.flags, r.bytes), (bridge::LOOPED, bytes));
}

#[test]
fn an_interrogation_is_answered_with_the_identity() {
    let mut s = Session::new(Rig::new());
    s.start().unwrap();
    let a = s.interrogate(10).unwrap().expect("an answer");
    match a.reply {
        Reply::Identity { model, serial, .. } => assert_eq!((model, serial), (proto::MODEL, proto::SERIAL)),
        other => panic!("{other:?}"),
    }
    // Sent in one transfer, answered in the next: the second report after
    // the send, or the third when a poll was already on its way.
    assert!(a.transfers == 2 || a.transfers == 3, "{}", a.transfers);
    assert_eq!(s.carrier.link.heard as usize, s.log.len() - 1);
}

#[test]
fn a_set_goes_in_parts_and_reads_back_equal() {
    let mut s = Session::new(Rig::new());
    s.start().unwrap();
    assert_eq!(s.read_set(10).unwrap(), None, "no set yet");
    let mut words = NOMINAL_WORDS;
    words[pm_core::word::LRL] = 90;
    let answers = s.send_set(&words, false, 10).unwrap();
    let codes: Vec<u8> = answers
        .iter()
        .map(|a| match a.as_ref().expect("an answer").reply {
            Reply::Status { code, .. } => code,
            ref other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(codes, [code::RECEIVED, code::RECEIVED, code::RECEIVED, code::RECEIVED, code::SET_COMPLETE]);
    assert_eq!(s.read_set(20).unwrap(), Some(words));
    assert_eq!(s.carrier.link.sets, 1);
}

#[test]
fn a_command_the_stage_does_not_serve_says_so() {
    let mut s = Session::new(Rig::new());
    s.start().unwrap();
    // Until task 16.1 this was PACE_NOW, which `link` now forwards.
    let f = s.dcm.byte(kind::SET_BATTERY, 1);
    match s.command(&f, 10).unwrap().expect("an answer").reply {
        Reply::Status { code: c, state, .. } => assert_eq!((c, state), (code::NOT_SERVED, proto::UNKNOWN)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_corrupted_command_gets_no_answer_and_the_link_counts_it() {
    let mut rig = Rig::new();
    rig.corrupt = Some(5);
    let mut s = Session::new(rig);
    s.start().unwrap();
    assert_eq!(s.interrogate(8).unwrap(), None);
    assert_eq!(s.carrier.link.heard, 0);
    assert!(s.carrier.link.unheard >= 8);
    // Every frame of the pulse generator's is still a frame: IDLE.
    assert!(s.log.iter().all(|r| reply(&r.bytes) == Some(Reply::Idle)));
}

#[test]
fn without_a_dcm_the_link_hears_nothing() {
    let mut s = Session::new(Rig::new());
    for _ in 0..5 {
        let _ = s.read(1).unwrap();
    }
    assert_eq!((s.carrier.link.heard, s.carrier.link.silent), (0, 5));
    let _ = message(bridge::CLEAR, &[0; FRAME]);
}

#[test]
fn a_valid_set_is_accepted_and_an_invalid_one_refused_as_predicted() {
    let mut s = Session::new(Rig::new());
    s.start().unwrap();
    let mut good = NOMINAL_WORDS;
    good[pm_core::word::LRL] = 90;
    let mut bad = NOMINAL_WORDS;
    bad[pm_core::word::LRL] = 31;
    assert!(predict(&good) && !predict(&bad));

    let parts = s.send_set(&good, false, 10).unwrap();
    let last = s.dcm.plain(kind::POLL).seq.wrapping_sub(1);
    assert!(matches!(parts[4].as_ref().unwrap().reply, Reply::Status { code: code::SET_COMPLETE, .. }));
    match s.verdict(10).unwrap().expect("a verdict").reply {
        Reply::Status { of, code: c, serial, refused, link_up, .. } => assert_eq!((of, c, serial, refused, link_up), (last, code::ACCEPTED, 1, 0, true)),
        other => panic!("{other:?}"),
    }

    let _ = s.send_set(&bad, false, 10).unwrap();
    match s.verdict(10).unwrap().expect("a verdict").reply {
        Reply::Status { code: c, serial, refused, .. } => assert_eq!((c, serial, refused), (code::REFUSED, 2, 1)),
        other => panic!("{other:?}"),
    }
    // The link started again: it has no set to read back.
    assert_eq!(s.carrier.link.sets, 0);
}

#[test]
fn a_forwarded_command_gets_its_verdict() {
    let mut s = Session::new(Rig::new());
    s.start().unwrap();
    let f = s.dcm.plain(kind::PACE_NOW);
    assert!(matches!(s.command(&f, 10).unwrap().expect("an answer").reply, Reply::Status { code: code::FORWARDED, .. }));
    match s.verdict(10).unwrap().expect("a verdict").reply {
        Reply::Status { of, code: c, .. } => assert_eq!((of, c), (f.seq, code::ACCEPTED)),
        other => panic!("{other:?}"),
    }
}
