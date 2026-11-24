//! The DCM_Stub of the PACEMAKER experiment (docs/pacemaker, task 15.2;
//! design, "Heart_Simulator and DCM_Stub"): the host's end of the DCM link.
//!
//! Three layers, each usable without the one below:
//!
//! - the frames a DCM sends and the meaning of the ones it receives
//!   ([`Dcm`], [`Reply`]), on `pm_core::frame`, the encoding the pulse
//!   generator itself uses;
//! - the prediction ([`predict`]): `pm_core::validate`, the function the
//!   pulse generator judges a parameter set by, so that a test can say
//!   beforehand whether a set will be accepted;
//! - the bridge's serial messages ([`message`], [`Reports`]) and a
//!   session over anything that carries them ([`Session`] over a
//!   [`Carrier`]): the serial port of the bridge board, or in the tests a
//!   model of the bridge with the link's own code behind it.
#![forbid(unsafe_code)]

use pm_core::frame::{self, kind, Assembly, Frame, FRAME, PARTS};
use pm_core::WORDS;

/// Whether the pulse generator accepts `words` as a parameter set: its own
/// validation function (PM19.3).
#[must_use]
pub fn predict(words: &[u32]) -> bool {
    pm_core::validate(words).is_some()
}

/// The DCM's side of the frames: it numbers what it sends.
#[derive(Debug, Default)]
pub struct Dcm {
    seq: u8,
}

impl Dcm {
    fn next(&mut self) -> u8 {
        let s = self.seq;
        self.seq = self.seq.wrapping_add(1);
        s
    }

    /// A frame of `kind` without payload (POLL, INTERROGATE,
    /// READ_PARAMETERS, END_TEMPORARY, PACE_NOW, ...).
    pub fn plain(&mut self, kind: u8) -> Frame {
        Frame::empty(self.next(), kind)
    }

    /// A frame of `kind` with one byte (SET_BATTERY, SET_ACTIVITY).
    pub fn byte(&mut self, kind: u8, value: u8) -> Frame {
        let mut f = Frame::empty(self.next(), kind);
        f.len = 1;
        f.payload[0] = value;
        f
    }

    /// The parts of a parameter set, permanent or temporary, in order.
    pub fn set(&mut self, words: &[u32; WORDS], temporary: bool) -> Vec<Frame> {
        let k = if temporary { kind::TEMPORARY_PART } else { kind::PROGRAM_PART };
        (0..PARTS as u8).map(|p| frame::set_part(self.next(), k, words, p).expect("a part of the set")).collect()
    }
}

/// What a frame of the pulse generator's says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Idle,
    /// The answer to the frame numbered `of`.
    Status { of: u8, code: u8, state: u8, mode: u8, battery: u8 },
    Identity { of: u8, model: u32, serial: u32 },
    /// One part of the parameters read back.
    Part(Frame),
    Record(Frame),
    /// A frame, but none a pulse generator sends, or one of a wrong length.
    Unexpected(Frame),
}

fn word(p: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([p[at], p[at + 1], p[at + 2], p[at + 3]])
}

/// The meaning of 32 bytes from the pulse generator; `None` when they are
/// no frame.
#[must_use]
pub fn reply(bytes: &[u8; FRAME]) -> Option<Reply> {
    let f = frame::decode(bytes)?;
    let p = &f.payload;
    Some(match (f.kind, f.len) {
        (kind::IDLE, 0) => Reply::Idle,
        (kind::STATUS, 8) => Reply::Status { of: p[0], code: p[1], state: p[2], mode: p[3], battery: p[4] },
        (kind::IDENTITY, 12) => Reply::Identity { of: p[0], model: word(p, 4), serial: word(p, 8) },
        (kind::PARAM_PART, _) => Reply::Part(f),
        (kind::RECORD, _) => Reply::Record(f),
        _ => Reply::Unexpected(f),
    })
}

/// The bridge's serial messages (`tools/pm-bridge`).
pub mod bridge {
    pub const START: u8 = 0x7E;
    /// Host to bridge: send once; send whenever nothing else waits; send
    /// nothing; come straight back.
    pub const COMMAND: u8 = b'C';
    pub const IDLE: u8 = b'I';
    pub const CLEAR: u8 = b'X';
    pub const LOOP: u8 = b'L';
    /// A report's flags: what the bridge had sent in that transfer.
    pub const SENT_COMMAND: u8 = 1;
    pub const SENT_IDLE: u8 = 2;
    pub const LOOPED: u8 = 0x80;
    pub const MESSAGE: usize = 35;
    pub const REPORT: usize = 38;
}

/// A message to the bridge.
#[must_use]
pub fn message(kind: u8, bytes: &[u8; FRAME]) -> [u8; bridge::MESSAGE] {
    let mut m = [0u8; bridge::MESSAGE];
    m[0] = bridge::START;
    m[1] = kind;
    m[2..34].copy_from_slice(bytes);
    m[34] = m[1..34].iter().fold(0u8, |a, b| a.wrapping_add(*b));
    m
}

/// One report of the bridge: a transfer with the pulse generator (or the
/// answer to a loop-back message).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub count: u8,
    pub flags: u8,
    /// The bytes the pulse generator clocked.
    pub amount: u8,
    pub dropped: u8,
    pub bytes: [u8; FRAME],
}

/// The reports in a stream of bytes from the bridge. A report whose sum is
/// wrong is skipped, and the search for the next start byte goes on from
/// the byte after its start.
#[derive(Debug, Default)]
pub struct Reports {
    pending: Vec<u8>,
    /// Bytes passed over outside a report, and reports with a wrong sum.
    pub skipped: usize,
    pub bad_sums: usize,
}

impl Reports {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Report> {
        self.pending.extend_from_slice(bytes);
        let mut out = Vec::new();
        loop {
            match self.pending.iter().position(|b| *b == bridge::START) {
                None => {
                    self.skipped += self.pending.len();
                    self.pending.clear();
                    return out;
                }
                Some(at) => {
                    self.skipped += at;
                    self.pending.drain(..at);
                }
            }
            if self.pending.len() < bridge::REPORT {
                return out;
            }
            let m = &self.pending[..bridge::REPORT];
            let sum = m[1..37].iter().fold(0u8, |a, b| a.wrapping_add(*b));
            if sum == m[37] {
                let mut bytes = [0u8; FRAME];
                bytes.copy_from_slice(&m[5..37]);
                out.push(Report { count: m[1], flags: m[2], amount: m[3], dropped: m[4], bytes });
                self.pending.drain(..bridge::REPORT);
            } else {
                self.bad_sums += 1;
                self.pending.drain(..1);
            }
        }
    }
}

/// What carries bytes to the bridge and back.
pub trait Carrier {
    fn send(&mut self, bytes: &[u8]) -> std::io::Result<()>;
    /// The bytes that have come, waiting a short time (a tenth of a second
    /// at most) when there are none.
    fn receive(&mut self) -> std::io::Result<Vec<u8>>;
}

/// The outcome of a command: the frame that answered it and the transfers
/// it took.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub reply: Reply,
    /// Reports read from the command's send to its answer, the answer's
    /// included.
    pub transfers: usize,
}

/// A session with the pulse generator through the bridge.
pub struct Session<C: Carrier> {
    pub carrier: C,
    pub dcm: Dcm,
    reports: Reports,
    /// Every report read, in order (the record of a run).
    pub log: Vec<Report>,
}

impl<C: Carrier> Session<C> {
    pub fn new(carrier: C) -> Session<C> {
        Session { carrier, dcm: Dcm::default(), reports: Reports::default(), log: Vec::new() }
    }

    /// Bytes skipped and reports with a wrong sum, so far.
    pub fn noise(&self) -> (usize, usize) {
        (self.reports.skipped, self.reports.bad_sums)
    }

    /// The next reports, waiting up to `tries` receives for the first.
    pub fn read(&mut self, tries: usize) -> std::io::Result<Vec<Report>> {
        for _ in 0..tries {
            let bytes = self.carrier.receive()?;
            let rs = self.reports.feed(&bytes);
            if !rs.is_empty() {
                self.log.extend(rs.iter().cloned());
                return Ok(rs);
            }
        }
        Ok(Vec::new())
    }

    /// The host side alone: `bytes` to the bridge and straight back.
    pub fn loop_back(&mut self, bytes: &[u8; FRAME]) -> std::io::Result<Option<Report>> {
        self.carrier.send(&message(bridge::LOOP, bytes))?;
        for _ in 0..20 {
            if let Some(r) = self.read(1)?.into_iter().find(|r| r.flags == bridge::LOOPED) {
                return Ok(Some(r));
            }
        }
        Ok(None)
    }

    /// Starts polling: the bridge answers every transfer that has no
    /// command with a POLL.
    pub fn start(&mut self) -> std::io::Result<()> {
        let poll = self.dcm.plain(kind::POLL);
        self.carrier.send(&message(bridge::IDLE, &frame::encode(&poll)))
    }

    /// Stops answering: the pulse generator reads zeros.
    pub fn stop(&mut self) -> std::io::Result<()> {
        self.carrier.send(&message(bridge::CLEAR, &[0; FRAME]))
    }

    /// Sends `f` and waits for the STATUS or IDENTITY that names it, for
    /// at most `transfers` reports.
    pub fn command(&mut self, f: &Frame, transfers: usize) -> std::io::Result<Option<Answer>> {
        self.carrier.send(&message(bridge::COMMAND, &frame::encode(f)))?;
        let mut seen = 0;
        let mut idle = 0;
        while seen < transfers && idle < 30 {
            let rs = self.read(1)?;
            if rs.is_empty() {
                idle += 1;
            }
            for r in rs {
                if r.flags == bridge::LOOPED {
                    continue;
                }
                seen += 1;
                match reply(&r.bytes) {
                    Some(Reply::Status { of, code, state, mode, battery }) if of == f.seq => {
                        return Ok(Some(Answer { reply: Reply::Status { of, code, state, mode, battery }, transfers: seen }));
                    }
                    Some(Reply::Identity { of, model, serial }) if of == f.seq => {
                        return Ok(Some(Answer { reply: Reply::Identity { of, model, serial }, transfers: seen }));
                    }
                    _ => {}
                }
            }
        }
        Ok(None)
    }

    /// Who is there (PM20.3).
    pub fn interrogate(&mut self, transfers: usize) -> std::io::Result<Option<Answer>> {
        let f = self.dcm.plain(kind::INTERROGATE);
        self.command(&f, transfers)
    }

    /// Sends a parameter set part by part; the answers to its parts.
    pub fn send_set(&mut self, words: &[u32; WORDS], temporary: bool, transfers: usize) -> std::io::Result<Vec<Option<Answer>>> {
        let mut out = Vec::new();
        for f in self.dcm.set(words, temporary) {
            out.push(self.command(&f, transfers)?);
        }
        Ok(out)
    }

    /// Reads the parameters back: READ_PARAMETERS, then the parts as they
    /// come. `Ok(None)` when no complete set came within `transfers`
    /// reports (a STATUS answering the request ends the wait at once).
    pub fn read_set(&mut self, transfers: usize) -> std::io::Result<Option<[u32; WORDS]>> {
        let f = self.dcm.plain(kind::READ_PARAMETERS);
        self.carrier.send(&message(bridge::COMMAND, &frame::encode(&f)))?;
        let mut assembly = Assembly::EMPTY;
        let mut seen = 0;
        let mut idle = 0;
        while seen < transfers && idle < 30 {
            let rs = self.read(1)?;
            if rs.is_empty() {
                idle += 1;
            }
            for r in rs {
                seen += 1;
                match reply(&r.bytes) {
                    Some(Reply::Part(p)) => {
                        if let Some(words) = assembly.accept(&p) {
                            return Ok(Some(words));
                        }
                    }
                    Some(Reply::Status { of, .. }) if of == f.seq => return Ok(None),
                    _ => {}
                }
            }
        }
        Ok(None)
    }
}

/// The bridge board's serial port (macOS and Linux): 115,200 baud, 8N1,
/// raw, reads that wait a tenth of a second.
pub struct Serial {
    file: std::fs::File,
}

impl Serial {
    pub fn open(device: &str) -> std::io::Result<Serial> {
        use std::os::unix::fs::OpenOptionsExt;
        // O_NOCTTY, by the platform's number.
        let noctty = if cfg!(target_os = "macos") { 0x2_0000 } else { 0x100 };
        let file = std::fs::OpenOptions::new().read(true).write(true).custom_flags(noctty).open(device)?;
        // The settings last while this handle is open.
        let flag = if cfg!(target_os = "macos") { "-f" } else { "-F" };
        let status = std::process::Command::new("stty")
            .args([flag, device, "115200", "raw", "-echo", "cs8", "-cstopb", "-parenb", "clocal", "min", "0", "time", "1"])
            .status()?;
        if !status.success() {
            return Err(std::io::Error::other(format!("stty on {device} failed")));
        }
        let mut s = Serial { file };
        // What the port held from before.
        for _ in 0..3 {
            let _ = s.receive()?;
        }
        Ok(s)
    }
}

impl Carrier for Serial {
    fn send(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        self.file.write_all(bytes)?;
        self.file.flush()
    }

    fn receive(&mut self) -> std::io::Result<Vec<u8>> {
        use std::io::Read;
        let mut buf = [0u8; 512];
        let n = self.file.read(&mut buf)?;
        Ok(buf[..n].to_vec())
    }
}
