//! The DCM_Stub's command line (docs/pacemaker, tasks 15.2 and 15.3).
//!
//! ```text
//! dcm-stub loopback <device> <count> [--json <record>]
//! dcm-stub session  <device> [--json <record>]
//! dcm-stub soak     <device> <seconds> [--json <record>]
//! dcm-stub program  <device> [--json <record>]
//! dcm-stub loss     <device> <count> [--json <record>]
//! dcm-stub do       <device> <op>... [--json <record>]
//! ```
//!
//! `loopback`: `count` different frames to the bridge and straight back
//! (the host side alone; the pulse generator is not needed).
//!
//! `soak`: polls for `seconds` and requires every transfer in that time to
//! be reported, of 32 bytes, an IDLE frame, answered by the bridge's POLL.
//!
//! `program` (tasks 15.4 and 16.1): a set the stub predicts valid and one
//! it predicts invalid are sent and `control`'s verdicts compared with the
//! predictions; temporary pacing is started, ended by command, started
//! again and ended by the loss of the link, with the transfers from the
//! first unanswered poll to the pulse generator's report of the loss
//! counted.
//!
//! `loss` (task 15.4): `count` times, the polls are answered until
//! `control` has the link as up, then no more, and the transfers from the
//! first unanswered poll to the pulse generator's report of the loss are
//! counted.
//!
//! `do` (tasks 16.3 to 16.6): the operations in order, each reported with
//! the host's clock at its start and end (seconds since the epoch), for a
//! rig script that lines them up with a capture:
//!
//! - `up`: answer the polls and wait until `control` has the link as up;
//! - `set:<i>=<v>,...` and `temp:<i>=<v>,...`: the nominal set with the
//!   words named replaced, sent as permanent or temporary parameters; the
//!   prediction and `control`'s verdict;
//! - `end-temp`, `pace-now`, `battery:<level>`, `activity:<0..255>`, `magnet-response:<0|1>`:
//!   the command and `control`'s verdict;
//! - `status`: what `control` reports now (a command the stage does not
//!   serve is sent, and its answer carries the status);
//! - `wait:<field>=<value>`: until a STATUS has the field (`state`, `mode`,
//!   `battery`, `link_up`) at the value, for at most 200 transfers;
//! - `read:<kind>,<part>`: one part of a record of `diag`'s (task 22.4);
//!   `reset-histograms`, `trend:<duration>,<content>`,
//!   `data:<0 implant, 1 measured>,<index>,<value>`: the requests;
//!   `markers:<seconds>`: the Marker transmission for that long, every
//!   record with its arrival; `threshold:<0..3>` and `threshold-stop`
//!   (task 22.6);
//! - `hold:<seconds>`: answer the polls for that long and do nothing else;
//! - `stop`: answer no more.
//!
//! `session`: with the pulse generator running, polls, interrogates it,
//! sends a parameter set in parts, reads it back, sends a command the
//! stage does not serve, and stops answering. Exit 0 when every step went
//! as expected.
#![forbid(unsafe_code)]

use dcm_stub::{bridge, predict, reply, Reply, Serial, Session};
use pm_core::frame::{self, kind, FRAME};
use pm_core::NOMINAL_WORDS;
use serde_json::{json, Value};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn loopback(device: &str, count: usize) -> std::io::Result<(bool, Value)> {
    let mut s = Session::new(Serial::open(device)?);
    let mut good = 0;
    let mut first_bad = Value::Null;
    let mut dcm = dcm_stub::Dcm::default();
    for n in 0..count {
        // Frames of every kind of use, and bytes that are no frame.
        let bytes: [u8; FRAME] = match n % 4 {
            0 => frame::encode(&dcm.plain(kind::POLL)),
            1 => frame::encode(&dcm.set(&NOMINAL_WORDS, false)[n % 5]),
            2 => core::array::from_fn(|i| (i * 7 + n) as u8),
            _ => [if n % 8 == 3 { 0x7E } else { 0xFF }; FRAME],
        };
        match s.loop_back(&bytes)? {
            Some(r) if r.bytes == bytes => good += 1,
            other => {
                if first_bad.is_null() {
                    first_bad = json!({ "n": n, "sent": hex(&bytes), "got": other.map(|r| hex(&r.bytes)) });
                }
            }
        }
    }
    let (skipped, bad_sums) = s.noise();
    let ok = good == count && bad_sums == 0;
    Ok((ok, json!({ "sent": count, "returned_equal": good, "bytes_skipped": skipped, "reports_with_a_wrong_sum": bad_sums, "first_difference": first_bad })))
}

fn session(device: &str) -> std::io::Result<(bool, Value)> {
    let mut s = Session::new(Serial::open(device)?);
    let mut steps = Vec::new();
    let mut ok = true;
    let mut step = |name: &str, good: bool, detail: Value| {
        ok &= good;
        println!("{} {name}: {detail}", if good { "ok  " } else { "FAIL" });
        steps.push(json!({ "step": name, "ok": good, "detail": detail }));
    };

    // Before the stub answers: the transfers come, and carry IDLE.
    let before = s.read(30)?;
    // (IDLE, or since task 16.1 a STATUS when `control`'s has changed.)
    let quiet = !before.is_empty() && before.iter().all(|r| r.flags == 0 && r.amount == 32 && matches!(reply(&r.bytes), Some(Reply::Idle | Reply::Status { .. })));
    step("the pulse generator polls, the bridge sending nothing", quiet, json!({ "reports": before.len(), "first": before.first().map(|r| hex(&r.bytes)) }));

    s.start()?;
    let a = s.interrogate(20)?;
    let good = matches!(a.as_ref().map(|a| &a.reply), Some(Reply::Identity { .. }));
    step("interrogation", good, json!(a.as_ref().map(|a| format!("{:?} after {} transfers", a.reply, a.transfers))));

    let none = s.read_set(20)?;
    step("READ_PARAMETERS before any set", none.is_none(), json!(none.is_some()));

    let mut words = NOMINAL_WORDS;
    words[pm_core::word::LRL] = 90;
    let answers = s.send_set(&words, false, 20)?;
    let codes: Vec<Value> = answers
        .iter()
        .map(|a| match a.as_ref().map(|a| &a.reply) {
            Some(Reply::Status { code, .. }) => json!(code),
            other => json!(format!("{other:?}")),
        })
        .collect();
    step("a set in five parts", codes == [json!(1), json!(1), json!(1), json!(1), json!(2)], json!({ "codes": codes, "predicted_valid": predict(&words) }));

    let back = s.read_set(30)?;
    step("the set read back", back == Some(words), json!(back.map(|w| w.to_vec())));

    let f = s.dcm.plain(kind::SET_CLOCK);
    let a = s.command(&f, 20)?;
    let good = matches!(a.as_ref().map(|a| &a.reply), Some(Reply::Status { code: 4, .. }));
    step("a command this stage does not serve", good, json!(a.as_ref().map(|a| format!("{:?}", a.reply))));

    s.stop()?;
    // The reports on their way when the stop was sent, then silence from
    // the bridge's side.
    let _ = s.read(5)?;
    let _ = s.read(5)?;
    let after = s.read(30)?;
    let good = !after.is_empty() && after.iter().all(|r| r.flags == 0);
    step("after the stop the bridge sends nothing", good, json!(after.len()));

    let (skipped, bad_sums) = s.noise();
    let transfers: Vec<&dcm_stub::Report> = s.log.iter().filter(|r| r.flags != bridge::LOOPED).collect();
    let short = transfers.iter().filter(|r| r.amount != 32).count();
    let no_frame = transfers.iter().filter(|r| reply(&r.bytes).is_none()).count();
    let lost = transfers.windows(2).filter(|w| w[1].count != w[0].count.wrapping_add(1)).count();
    let dropped = transfers.last().map(|r| r.dropped).unwrap_or(0);
    let clean = short == 0 && no_frame == 0 && lost == 0 && dropped == 0 && bad_sums == 0;
    ok &= clean;
    let totals = json!({ "transfers_reported": transfers.len(), "of_other_than_32_bytes": short, "whose_bytes_were_no_frame": no_frame,
        "gaps_in_the_report_count": lost, "dropped_by_the_bridge": dropped, "bytes_skipped": skipped, "reports_with_a_wrong_sum": bad_sums });
    println!("{} every transfer: {totals}", if clean { "ok  " } else { "FAIL" });
    Ok((ok, json!({ "steps": steps, "transfers": totals })))
}

fn soak(device: &str, seconds: u64) -> std::io::Result<(bool, Value)> {
    let mut s = Session::new(Serial::open(device)?);
    s.start()?;
    // The transfers on their way when the polling started.
    let _ = s.read(5)?;
    let _ = s.read(5)?;
    let from = s.log.len();
    let started = std::time::Instant::now();
    while started.elapsed().as_secs() < seconds {
        let _ = s.read(1)?;
    }
    let elapsed = started.elapsed().as_secs_f64();
    s.stop()?;
    let rs = &s.log[from..];
    let short = rs.iter().filter(|r| r.amount != 32).count();
    // IDLE, or a STATUS when `control`'s status has changed (task 16.1).
    let not_idle = rs.iter().filter(|r| !matches!(reply(&r.bytes), Some(Reply::Idle | Reply::Status { .. }))).count();
    let statuses = rs.iter().filter(|r| matches!(reply(&r.bytes), Some(Reply::Status { .. }))).count();
    let not_polled = rs.iter().filter(|r| r.flags != bridge::SENT_IDLE).count();
    let gaps = rs.windows(2).filter(|w| w[1].count != w[0].count.wrapping_add(1)).count();
    // The pulse generator numbers its frames: one lost or repeated shows.
    let seq_gaps = rs.windows(2).filter(|w| w[1].bytes[0] != w[0].bytes[0].wrapping_add(1)).count();
    let dropped = rs.last().map(|r| r.dropped).unwrap_or(0);
    let (skipped, bad_sums) = s.noise();
    let per_second = rs.len() as f64 / elapsed;
    let ok = !rs.is_empty() && short == 0 && not_idle == 0 && not_polled == 0 && gaps == 0 && seq_gaps == 0 && dropped == 0 && bad_sums == 0 && (per_second - 20.0).abs() < 0.5;
    Ok((ok, json!({ "seconds": elapsed, "transfers_reported": rs.len(), "per_second": per_second, "of_other_than_32_bytes": short, "neither_idle_nor_status": not_idle, "status_frames": statuses,
        "not_answered_by_the_poll": not_polled, "gaps_in_the_report_count": gaps, "gaps_in_the_pulse_generators_numbering": seq_gaps,
        "dropped_by_the_bridge": dropped, "bytes_skipped": skipped, "reports_with_a_wrong_sum": bad_sums })))
}

/// The State and the fields of `control`'s status in a STATUS frame.
fn told(r: &Reply) -> Value {
    match r {
        Reply::Status { of, code, state, mode, battery, link_up, serial, refused } => {
            json!({ "of": of, "code": code, "state": state, "mode": mode, "battery": battery, "link_up": link_up, "verdicts": serial, "refused": refused })
        }
        other => json!(format!("{other:?}")),
    }
}

fn program(device: &str) -> std::io::Result<(bool, Value)> {
    let mut s = Session::new(Serial::open(device)?);
    let mut steps = Vec::new();
    let mut ok = true;
    let mut step = |name: &str, good: bool, detail: Value| {
        ok &= good;
        println!("{} {name}: {detail}", if good { "ok  " } else { "FAIL" });
        steps.push(json!({ "step": name, "ok": good, "detail": detail }));
    };
    const PERMANENT: u8 = 1;
    const TEMPORARY: u8 = 2;
    let up = |r: &Reply| matches!(r, Reply::Status { link_up: true, .. });

    // 1. The link comes up when the stub answers.
    s.start()?;
    let a = s.wait(40, up)?;
    step("control has the link as up once the polls are answered", a.is_some(), json!(a.as_ref().map(|a| json!({ "status": told(&a.reply), "transfers": a.transfers }))));

    // 2. A set predicted valid: accepted.
    // The sets of this run are VVI, whatever the Mode of the nominal set.
    let vvi = pm_core::Mode::PROGRAMMABLE.iter().position(|m| *m == pm_core::Mode::VVI).expect("VVI is programmable") as u32;
    let mut good_set = NOMINAL_WORDS;
    good_set[pm_core::word::MODE] = vvi;
    good_set[pm_core::word::LRL] = 90;
    let predicted = predict(&good_set);
    let parts = s.send_set(&good_set, false, 20)?;
    let last = parts.last().and_then(|a| a.as_ref()).map(|a| a.reply.clone());
    let seq = match last {
        Some(Reply::Status { of, code: 2, .. }) => Some(of),
        _ => None,
    };
    let v = s.verdict(20)?;
    let accepted = matches!((v.as_ref().map(|a| &a.reply), seq), (Some(Reply::Status { of, code: 6, .. }), Some(q)) if *of == q);
    step("a set predicted valid (VVI, LRL 90) is accepted", predicted && accepted,
         json!({ "predicted_valid": predicted, "verdict": v.as_ref().map(|a| told(&a.reply)), "transfers_to_the_verdict": v.as_ref().map(|a| a.transfers) }));
    let refused_before = match v.as_ref().map(|a| &a.reply) {
        Some(Reply::Status { refused, .. }) => *refused,
        _ => 0,
    };

    // 3. A set predicted invalid: refused by `link`, naming the frame of
    // its last part; `comms` is not restarted, `control` has refused
    // nothing, and the link stays up. (Until 2026-11-24 the set went to
    // `therapy`, whose refusal restarted `comms`.)
    let mut bad_set = good_set;
    bad_set[pm_core::word::LRL] = 31;
    let predicted = predict(&bad_set);
    let from = s.log.len();
    let parts = s.send_set(&bad_set, false, 20)?;
    let last = parts.last().and_then(|a| a.as_ref()).map(|a| a.reply.clone());
    let refused = matches!(last, Some(Reply::Status { code: 7, refused, link_up: true, .. }) if refused == refused_before);
    let numbers: Vec<u8> = s.log[from..].iter().filter(|r| reply(&r.bytes).is_some()).map(|r| r.bytes[0]).collect();
    let restarted = numbers.windows(2).any(|w| w[1] < w[0] && w[1] < 4);
    step("a set predicted invalid (LRL 31) is refused by link, and nothing restarts", !predicted && refused && !restarted,
         json!({ "predicted_valid": predicted, "answer_to_the_last_part": last.as_ref().map(told), "link_numbering_restarted": restarted }));

    // 4. Temporary pacing, ended by command.
    let mut temp = good_set;
    temp[pm_core::word::LRL] = 120;
    let temporary = |s: &mut Session<Serial>| -> std::io::Result<(bool, Value)> {
        let _ = s.send_set(&temp, true, 20)?;
        let v = s.verdict(20)?;
        let accepted = matches!(v.as_ref().map(|a| &a.reply), Some(Reply::Status { code: 6, .. }));
        let entered = s.wait(60, |r| matches!(r, Reply::Status { state: TEMPORARY, .. }))?;
        Ok((accepted && entered.is_some(), json!({ "verdict": v.as_ref().map(|a| told(&a.reply)), "transfers_to_the_state": entered.as_ref().map(|a| a.transfers) })))
    };
    let (good, detail) = temporary(&mut s)?;
    step("temporary pacing (LRL 120) is accepted and the State becomes Temporary", good, detail);
    let f = s.dcm.plain(kind::END_TEMPORARY);
    let a = s.command(&f, 20)?;
    let forwarded = matches!(a.as_ref().map(|a| &a.reply), Some(Reply::Status { code: 9, .. }));
    let v = s.verdict(20)?;
    let back = s.wait(60, |r| matches!(r, Reply::Status { state: PERMANENT, .. }))?;
    step("END_TEMPORARY is forwarded and accepted and the State becomes Permanent", forwarded && v.is_some() && back.is_some(),
         json!({ "verdict": v.as_ref().map(|a| told(&a.reply)), "transfers_to_the_state": back.as_ref().map(|a| a.transfers) }));

    // 5. Temporary pacing again, ended by the loss of the link (task 15.4).
    let (good, detail) = temporary(&mut s)?;
    step("temporary pacing again", good, detail);
    let from = s.log.len();
    s.stop()?;
    let down = s.wait(40, |r| matches!(r, Reply::Status { link_up: false, .. }))?;
    let down_at = s.log.len();
    let back = s.wait(60, |r| matches!(r, Reply::Status { state: PERMANENT, link_up: false, .. }))?;
    // The first transfer in which the bridge sent nothing, and the one
    // that carried the pulse generator's report of the loss.
    let first_unanswered = s.log[from..].iter().position(|r| r.flags == 0).map(|i| from + i);
    let reported = s.log[..down_at].iter().rposition(|r| matches!(reply(&r.bytes), Some(Reply::Status { link_up: false, .. })));
    let transfers = match (first_unanswered, reported) {
        (Some(a), Some(b)) if b >= a => Some(b - a),
        _ => None,
    };
    // Expected 4 (the report leaves in the transfer `link` starts 200 ms
    // after the Job that heard the last frame); one more is allowed.
    let within = matches!(transfers, Some(3..=5));
    step("the loss of the link is reported within its bound and ends temporary pacing", down.is_some() && back.is_some() && within,
         json!({ "transfers_from_the_first_unanswered_poll_to_the_report": transfers, "milliseconds": transfers.map(|t| t * 50),
                 "expected_transfers": 4, "allowed": "3 to 5", "status": down.as_ref().map(|a| told(&a.reply)),
                 "transfers_from_the_stop_to_permanent": back.as_ref().map(|b| b.transfers + down.as_ref().map(|d| d.transfers).unwrap_or(0)) }));

    // With DCM_TRACE set: every transfer that carried other than IDLE.
    if std::env::var_os("DCM_TRACE").is_some() {
        for (i, r) in s.log.iter().enumerate() {
            match reply(&r.bytes) {
                Some(Reply::Idle) => {}
                other => println!("  {i:4} sent {} pg#{:3} {}", r.flags, r.bytes[0], other.map(|o| told(&o).to_string()).unwrap_or_else(|| "no frame".into())),
            }
        }
    }
    let (skipped, bad_sums) = s.noise();
    let transfers: Vec<&dcm_stub::Report> = s.log.iter().filter(|r| r.flags != bridge::LOOPED).collect();
    let short = transfers.iter().filter(|r| r.amount != 32).count();
    let no_frame = transfers.iter().filter(|r| reply(&r.bytes).is_none()).count();
    let lost = transfers.windows(2).filter(|w| w[1].count != w[0].count.wrapping_add(1)).count();
    // A restart of `comms` may cut one transfer short; nothing else may.
    let clean = short <= 1 && no_frame <= 1 && lost == 0 && bad_sums == 0;
    ok &= clean;
    let totals = json!({ "transfers_reported": transfers.len(), "of_other_than_32_bytes": short, "whose_bytes_were_no_frame": no_frame,
        "gaps_in_the_report_count": lost, "bytes_skipped": skipped, "reports_with_a_wrong_sum": bad_sums });
    println!("{} every transfer: {totals}", if clean { "ok  " } else { "FAIL" });
    Ok((ok, json!({ "steps": steps, "transfers": totals })))
}

fn loss(device: &str, count: usize) -> std::io::Result<(bool, Value)> {
    let mut s = Session::new(Serial::open(device)?);
    let mut found = Vec::new();
    let mut failed = 0;
    for _ in 0..count {
        s.start()?;
        if s.wait(60, |r| matches!(r, Reply::Status { link_up: true, .. }))?.is_none() {
            failed += 1;
            continue;
        }
        // Answered polls for a while longer, a different while each time,
        // so that the stop falls anywhere in the pulse generator's periods.
        for _ in 0..3 + found.len() % 4 {
            let _ = s.next()?;
        }
        std::thread::sleep(std::time::Duration::from_millis(7 * (found.len() as u64 % 8)));
        let from = s.log.len();
        s.stop()?;
        let down = s.wait(40, |r| matches!(r, Reply::Status { link_up: false, .. }))?;
        let first_unanswered = s.log[from..].iter().position(|r| r.flags == 0).map(|i| from + i);
        let reported = s.log.iter().rposition(|r| matches!(reply(&r.bytes), Some(Reply::Status { link_up: false, .. })));
        match (down, first_unanswered, reported) {
            (Some(_), Some(a), Some(b)) if b >= a => found.push(b - a),
            _ => failed += 1,
        }
    }
    let least = found.iter().min().copied();
    let most = found.iter().max().copied();
    let mut histogram = std::collections::BTreeMap::new();
    for n in &found {
        *histogram.entry(n.to_string()).or_insert(0u32) += 1;
    }
    // The bound: the report leaves in the transfer `link` starts 200 ms
    // after its Job that heard the last frame, the fourth after the first
    // unanswered one; one more period is allowed for a status that a Job
    // of `link` just missed.
    let ok = failed == 0 && found.len() == count && least >= Some(4) && most <= Some(5);
    let (skipped, bad_sums) = s.noise();
    Ok((ok, json!({ "repetitions": count, "measured": found.len(), "failed": failed, "transfers_from_the_first_unanswered_poll_to_the_report": histogram,
        "least_ms": least.map(|n| n * 50), "most_ms": most.map(|n| n * 50), "bound_transfers": 5, "bound_ms": 250, "earliest_allowed_transfers": 4,
        "bytes_skipped": skipped, "reports_with_a_wrong_sum": bad_sums })))
}

fn epoch() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// The six words of a RECORD frame.
fn words_of_record(f: &pm_core::frame::Frame) -> [u32; 6] {
    let mut w = [0u32; 6];
    for (k, x) in w.iter_mut().enumerate() {
        *x = u32::from_le_bytes([f.payload[4 * k], f.payload[4 * k + 1], f.payload[4 * k + 2], f.payload[4 * k + 3]]);
    }
    w
}

fn words_of(text: &str) -> Option<[u32; pm_core::WORDS]> {
    let mut w = NOMINAL_WORDS;
    for pair in text.split(',').filter(|p| !p.is_empty()) {
        let (i, v) = pair.split_once('=')?;
        *w.get_mut(i.parse::<usize>().ok()?)? = v.parse().ok()?;
    }
    Some(w)
}

fn run_ops(device: &str, ops: &[String]) -> std::io::Result<(bool, Value)> {
    let mut s = Session::new(Serial::open(device)?);
    s.start()?;
    let mut out = Vec::new();
    let mut ok = true;
    for op in ops {
        let (name, arg) = op.split_once(':').unwrap_or((op.as_str(), ""));
        let started = epoch();
        let mut sent = started;
        // A command, then `control`'s verdict.
        let mut commanded = |s: &mut Session<Serial>, f: pm_core::frame::Frame, sent: &mut f64| -> std::io::Result<(bool, Value)> {
            *sent = epoch();
            let a = s.command(&f, 20)?;
            let forwarded = matches!(a.as_ref().map(|a| &a.reply), Some(Reply::Status { code: 9, .. }));
            let v = s.verdict(20)?;
            Ok((forwarded && v.is_some(), json!({ "verdict": v.as_ref().map(|a| told(&a.reply)), "accepted": matches!(v.as_ref().map(|a| &a.reply), Some(Reply::Status { code: 6, .. })) })))
        };
        let (good, detail) = match name {
            "up" => {
                let a = s.wait(60, |r| matches!(r, Reply::Status { link_up: true, .. }))?;
                match a {
                    Some(a) => (true, told(&a.reply)),
                    // Told before this invocation: ask.
                    None => {
                        let f = s.dcm.plain(kind::SET_CLOCK);
                        let a = s.command(&f, 20)?;
                        (matches!(a.as_ref().map(|a| &a.reply), Some(Reply::Status { link_up: true, .. })), json!(a.as_ref().map(|a| told(&a.reply))))
                    }
                }
            }
            "set" | "temp" => match words_of(arg) {
                None => (false, json!("no parameter set")),
                Some(words) => {
                    let predicted = predict(&words);
                    sent = epoch();
                    let _ = s.send_set(&words, name == "temp", 20)?;
                    let v = s.verdict(40)?;
                    let accepted = matches!(v.as_ref().map(|a| &a.reply), Some(Reply::Status { code: 6, .. }));
                    (v.is_some(), json!({ "predicted_valid": predicted, "accepted": accepted, "verdict": v.as_ref().map(|a| told(&a.reply)) }))
                }
            },
            "end-temp" => {
                let f = s.dcm.plain(kind::END_TEMPORARY);
                commanded(&mut s, f, &mut sent)?
            }
            "pace-now" => {
                let f = s.dcm.plain(kind::PACE_NOW);
                commanded(&mut s, f, &mut sent)?
            }
            "battery" => {
                let f = s.dcm.byte(kind::SET_BATTERY, arg.parse().unwrap_or(255));
                commanded(&mut s, f, &mut sent)?
            }
            "activity" => {
                let f = s.dcm.byte(kind::SET_ACTIVITY, arg.parse().unwrap_or(0));
                commanded(&mut s, f, &mut sent)?
            }
            "magnet-response" => {
                let f = s.dcm.byte(kind::SET_MAGNET_RESPONSE, arg.parse().unwrap_or(255));
                commanded(&mut s, f, &mut sent)?
            }
            "status" => {
                let f = s.dcm.plain(kind::SET_CLOCK);
                let a = s.command(&f, 20)?;
                (a.is_some(), json!(a.as_ref().map(|a| told(&a.reply))))
            }
            "wait" => {
                let (field, value) = arg.split_once('=').unwrap_or(("", ""));
                let want: u32 = value.parse().unwrap_or(u32::MAX);
                let field = field.to_string();
                let has = |r: &Reply| match r {
                    Reply::Status { state, mode, battery, link_up, .. } => match field.as_str() {
                        "state" => *state as u32 == want,
                        "mode" => *mode as u32 == want,
                        "battery" => *battery as u32 == want,
                        "link_up" => *link_up as u32 == want,
                        _ => false,
                    },
                    _ => false,
                };
                // It may be so already (a status is told once): ask first.
                let f = s.dcm.plain(kind::SET_CLOCK);
                let now = s.command(&f, 20)?;
                let a = match now {
                    Some(a) if has(&a.reply) => Some(a),
                    _ => s.wait(200, &has)?,
                };
                (a.is_some(), json!(a.as_ref().map(|a| json!({ "status": told(&a.reply), "transfers": a.transfers }))))
            }
            // Task 22.4: one part of a record of `diag`'s, its six words.
            "read" => {
                let (k, part) = arg.split_once(',').unwrap_or((arg, "0"));
                let (k, part): (u8, u8) = (k.parse().unwrap_or(0), part.parse().unwrap_or(0));
                let f = s.dcm.bytes(kind::READ_RECORD, &[k, part]);
                sent = epoch();
                let a = s.command(&f, 20)?;
                let asked = matches!(a.as_ref().map(|a| &a.reply), Some(Reply::Status { code: 10, .. }));
                let r = if asked { s.wait(20, |r| matches!(r, Reply::Record(f) if f.payload[0] == k && f.part == part))? } else { None };
                match r.as_ref().map(|a| &a.reply) {
                    Some(Reply::Record(f)) => (true, json!({ "kind": k, "part": part, "words": words_of_record(f)[1..].to_vec(), "transfers": r.as_ref().map(|a| a.transfers) })),
                    _ => (false, json!({ "kind": k, "part": part, "answer": a.as_ref().map(|a| told(&a.reply)) })),
                }
            }
            // The requests that change what `diag` keeps: answered REQUESTED (10).
            "reset-histograms" | "trend" | "data" => {
                let numbers: Vec<u32> = arg.split(',').filter(|p| !p.is_empty()).filter_map(|p| p.parse().ok()).collect();
                let f = match (name, numbers.as_slice()) {
                    ("reset-histograms", []) => Some(s.dcm.plain(kind::RESET_HISTOGRAMS)),
                    ("trend", [d, w]) => Some(s.dcm.bytes(kind::TREND_START, &[*d as u8, *w as u8])),
                    ("data", [which, index, value]) => {
                        let v = value.to_le_bytes();
                        Some(s.dcm.bytes(kind::SET_DATA, &[*which as u8, *index as u8, v[0], v[1], v[2], v[3]]))
                    }
                    _ => None,
                };
                match f {
                    None => (false, json!("wrong arguments")),
                    Some(f) => {
                        sent = epoch();
                        let a = s.command(&f, 20)?;
                        (matches!(a.as_ref().map(|a| &a.reply), Some(Reply::Status { code: 10, .. })), json!(a.as_ref().map(|a| told(&a.reply))))
                    }
                }
            }
            // The Marker transmission for so many seconds: every record
            // that came, with the host's clock at its arrival.
            "markers" => {
                let seconds: f64 = arg.parse().unwrap_or(0.0);
                let f = s.dcm.plain(kind::MARKERS_START);
                sent = epoch();
                let a = s.command(&f, 20)?;
                let started_ok = matches!(a.as_ref().map(|a| &a.reply), Some(Reply::Status { code: 10, .. }));
                let until = epoch() + seconds;
                let mut records = Vec::new();
                while epoch() < until {
                    if let Some(a) = s.wait(1, |r| matches!(r, Reply::Record(f) if f.payload[0] == pm_core::record::data::MARKER))? {
                        if let Reply::Record(f) = &a.reply {
                            let w = words_of_record(f);
                            records.push(json!({ "arrived": epoch(), "number": w[1], "words": [w[2], w[3], w[4]] }));
                        }
                    }
                }
                let f = s.dcm.plain(kind::MARKERS_STOP);
                let stopped = s.command(&f, 20)?;
                let stopped_ok = matches!(stopped.as_ref().map(|a| &a.reply), Some(Reply::Status { code: 10, .. }));
                (started_ok && stopped_ok, json!({ "records": records }))
            }
            // Task 22.6: the pacing threshold test, and its Stop.
            "threshold" => {
                let f = s.dcm.byte(kind::THRESHOLD_START, arg.parse().unwrap_or(255));
                commanded(&mut s, f, &mut sent)?
            }
            "threshold-stop" => {
                let f = s.dcm.plain(kind::THRESHOLD_STOP);
                commanded(&mut s, f, &mut sent)?
            }
            // The polls answered for so many seconds (a transfer is 50 ms),
            // so that the link stays up between two operations.
            "hold" => {
                let seconds: f64 = arg.parse().unwrap_or(0.0);
                let until = started + seconds;
                while epoch() < until {
                    let _ = s.wait(1, |_| false)?;
                }
                (true, Value::Null)
            }
            "stop" => {
                s.stop()?;
                (true, Value::Null)
            }
            _ => (false, json!("no such operation")),
        };
        ok &= good;
        let line = json!({ "op": op, "ok": good, "started": started, "sent": sent, "done": epoch(), "detail": detail });
        println!("{line}");
        out.push(line);
    }
    Ok((ok, json!({ "ops": out })))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let json_path = args.iter().position(|a| a == "--json").and_then(|i| args.get(i + 1)).cloned();
    let result = match (args.get(1).map(String::as_str), args.get(2)) {
        (Some("loopback"), Some(device)) => loopback(device, args.get(3).and_then(|n| n.parse().ok()).unwrap_or(100)),
        (Some("session"), Some(device)) => session(device),
        (Some("program"), Some(device)) => program(device),
        (Some("do"), Some(device)) => {
            let ops: Vec<String> = args[3..].iter().take_while(|a| *a != "--json").cloned().collect();
            run_ops(device, &ops)
        }
        (Some("loss"), Some(device)) => loss(device, args.get(3).and_then(|n| n.parse().ok()).unwrap_or(20)),
        (Some("soak"), Some(device)) => soak(device, args.get(3).and_then(|n| n.parse().ok()).unwrap_or(60)),
        _ => {
            eprintln!("usage: dcm-stub loopback <device> <count> | session <device> | program <device> | loss <device> <count> | soak <device> <seconds> | do <device> <op>..., each with [--json <record>]");
            std::process::exit(2);
        }
    };
    match result {
        Ok((ok, mut record)) => {
            record["command"] = json!(args[1]);
            record["verdict"] = json!(if ok { "PASS" } else { "FAIL" });
            println!("{}", if ok { "PASS" } else { "FAIL" });
            if args[1] != "session" && args[1] != "program" && args[1] != "do" {
                println!("{record}");
            }
            if let Some(p) = json_path {
                std::fs::write(&p, serde_json::to_string_pretty(&record).unwrap() + "\n").expect("the record written");
            }
            std::process::exit(if ok { 0 } else { 1 });
        }
        Err(e) => {
            eprintln!("dcm-stub: {e}");
            std::process::exit(2);
        }
    }
}
