//! The DCM_Stub's command line (docs/pacemaker, tasks 15.2 and 15.3).
//!
//! ```text
//! dcm-stub loopback <device> <count> [--json <record>]
//! dcm-stub session  <device> [--json <record>]
//! dcm-stub soak     <device> <seconds> [--json <record>]
//! ```
//!
//! `loopback`: `count` different frames to the bridge and straight back
//! (the host side alone; the pulse generator is not needed).
//!
//! `soak`: polls for `seconds` and requires every transfer in that time to
//! be reported, of 32 bytes, an IDLE frame, answered by the bridge's POLL.
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
    let quiet = !before.is_empty() && before.iter().all(|r| r.flags == 0 && r.amount == 32 && reply(&r.bytes) == Some(Reply::Idle));
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

    let f = s.dcm.plain(kind::PACE_NOW);
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
    let not_idle = rs.iter().filter(|r| reply(&r.bytes) != Some(Reply::Idle)).count();
    let not_polled = rs.iter().filter(|r| r.flags != bridge::SENT_IDLE).count();
    let gaps = rs.windows(2).filter(|w| w[1].count != w[0].count.wrapping_add(1)).count();
    // The pulse generator numbers its frames: one lost or repeated shows.
    let seq_gaps = rs.windows(2).filter(|w| w[1].bytes[0] != w[0].bytes[0].wrapping_add(1)).count();
    let dropped = rs.last().map(|r| r.dropped).unwrap_or(0);
    let (skipped, bad_sums) = s.noise();
    let per_second = rs.len() as f64 / elapsed;
    let ok = !rs.is_empty() && short == 0 && not_idle == 0 && not_polled == 0 && gaps == 0 && seq_gaps == 0 && dropped == 0 && bad_sums == 0 && (per_second - 20.0).abs() < 0.5;
    Ok((ok, json!({ "seconds": elapsed, "transfers_reported": rs.len(), "per_second": per_second, "of_other_than_32_bytes": short, "not_an_idle_frame": not_idle,
        "not_answered_by_the_poll": not_polled, "gaps_in_the_report_count": gaps, "gaps_in_the_pulse_generators_numbering": seq_gaps,
        "dropped_by_the_bridge": dropped, "bytes_skipped": skipped, "reports_with_a_wrong_sum": bad_sums })))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let json_path = args.iter().position(|a| a == "--json").and_then(|i| args.get(i + 1)).cloned();
    let result = match (args.get(1).map(String::as_str), args.get(2)) {
        (Some("loopback"), Some(device)) => loopback(device, args.get(3).and_then(|n| n.parse().ok()).unwrap_or(100)),
        (Some("session"), Some(device)) => session(device),
        (Some("soak"), Some(device)) => soak(device, args.get(3).and_then(|n| n.parse().ok()).unwrap_or(60)),
        _ => {
            eprintln!("usage: dcm-stub loopback <device> <count> | session <device> | soak <device> <seconds>, each with [--json <record>]");
            std::process::exit(2);
        }
    };
    match result {
        Ok((ok, mut record)) => {
            record["command"] = json!(args[1]);
            record["verdict"] = json!(if ok { "PASS" } else { "FAIL" });
            println!("{}", if ok { "PASS" } else { "FAIL" });
            if args[1] != "session" {
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
