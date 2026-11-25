#!/usr/bin/env python3
"""Task 17 of docs/pacemaker/tasks.md: the rig runs of the fault responses.

Usage: e3_faults.py <record.json> [restart] [lower] [safe_state]

Run from the workspace root with rsk's tools on PATH. Every run is the
rig variant of the pulse generator (an Operating_Duration of 45 s) with the
Kernel's event mirror, flashed and run under an attached `probe-rs run`
(a start by `probe-rs reset` makes `control` miss a deadline in the first
second, which restarts `therapy` and stops the mirror). Both pace pins are
captured at 1 MHz from before the start. After the Operating_Duration the
Kernel is in its safe state and its event mirror is read; the mirror stops
at the first event outside the build's expected set, so it shows the first
event of a run and what led to it, not what followed.

  restart     (17.1) The build has no set of its own: it starts in the
              Power-On Reset State and the DCM_Stub programs VOO at 71 ppm
              over the link (a build that programs itself at Init does so
              again after a restart). The Heart_Simulator is silent. One
              injection into `control` between 10 and 35 s: a fault, an
              Overrun, a deadline miss (three runs). The pause in pacing
              (from the last pulse before the restart to the first pulse
              of the Power-On Reset State), the longest high time of each
              pin, and the Kernel's event.
  lower       (17.2) VVI at 90 ppm against the Heart_Simulator's `fixed`
              rhythm. From 10 s on every Job of `link` faults (`comms` is
              restarted each time), overruns its Budget, sends a parameter
              set the validation function refuses (the Kernel reports the
              rejected message against `comms`), floods the Endpoint
              `program`, or, in the declaration that stops
              `comms` on a fault, faults once (five runs). `pm-judge`
              judges 30 s of pacing across the onset; the Kernel's first
              event must be of `comms`, and `therapy` must not have
              restarted.
  safe_state  (17.3) VOO at 120 ppm with the widest pulse, to the end of
              the Operating_Duration and 7 s beyond: no pulse after the
              end, both pins low, the last pulse whole.

Exit 0 when every run chosen passed.
"""
import json
import os
import subprocess
import sys
import time

import e1_runs
import e3_states as st
import link_runs

TARGET = st.TARGET
DURATION = 45
TICK_HZ = 16_000_000
VOO, VVI = 7, 8
KINDS = {2: "memmanage", 3: "busfault", 4: "usagefault", 5: "hardfault", 6: "overrun", 7: "deadline_miss", 10: "operating_end", 12: "protocol_violation",
         16: "partition_restart", 17: "partition_stop", 18: "safe_state", 19: "end_job", 22: "api_misuse", 23: "debugger", 25: "endpoint_full", 26: "message_rejected"}
link_runs.EVENT_KINDS.update(KINDS)


def build(features, target_dir, env):
    out = subprocess.run(["cargo", "build", "-q", "-p", "pacemaker", "--release", "--target", TARGET, "--no-default-features", "--features", features,
                          "--target-dir", target_dir], capture_output=True, text=True, env={**os.environ, **env})
    if out.returncode != 0:
        sys.exit(out.stderr[-2000:])
    return f"{target_dir}/{TARGET}/release/pacemaker"


def heart(scenario):
    features = "nrf52840,debug-port,quiet" + ("" if scenario == "fixed" else f",scenario-{scenario}")
    st.sh(["cargo", "build", "-q", "-p", "heart-sim", "--release", "--target", TARGET, "--no-default-features", "--features", features,
           "--target-dir", f"target/heart-sim-{scenario}"])
    elf = f"target/heart-sim-{scenario}/{TARGET}/release/heart-sim"
    st.probe("download", st.BOARD_B, "--disable-progressbars", elf)
    return elf


def run_attached(pg, capture_seconds, capture_after=0.0, stub_ops=None, late_ops=None):
    """The build flashed and run under an attached probe, with a capture
    that begins `capture_after` seconds after the launch (0: before it).
    Returns the capture, the host time of the launch and the events."""
    st.probe("reset", st.BOARD_B)
    cap = None
    if capture_after == 0.0:
        cap = st.Capture(capture_seconds)
        cap.started()
    launched = time.time()
    run = subprocess.Popen(["probe-rs", "run", "--chip", st.CHIP, "--probe", st.BOARD_A, "--disable-progressbars", "--catch-hardfault", pg],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if cap is None:
        time.sleep(capture_after)
        cap = st.Capture(capture_seconds)
        cap.started()
    ops = None
    if stub_ops:
        # The run begins when the flashing is over: its first Paces show it.
        time.sleep(max(0.0, 11.5 - (time.time() - launched)))
        ops = st.stub(*stub_ops)
    late = None
    if late_ops:
        # Well after the injection at 10 s of the run, well before its end.
        time.sleep(max(0.0, 38.0 - (time.time() - launched)))
        late = st.stub(*late_ops)
    cap.finish()
    cap.ops = ops
    cap.late = late
    # Flashing takes some seconds of the launch: wait until the run is over.
    time.sleep(max(0.0, DURATION + 16 - (time.time() - launched)))
    run.kill()
    run.wait()
    return cap, launched, events_of(pg)


def events_of(pg):
    ev = link_runs.events(pg)
    if not ev:
        return None
    ends = [i for i, e in enumerate(ev["events"]) if e["name"] == "safe_state"]
    ev["events_of_the_run"] = ev["events"][:ends[0] + 1] if ends else ev["events"]
    return ev


def after_boot(pulses):
    """The pulses after the pin's undriven stretch at the start (a pin is
    not driven from the reset to `control`'s first Job, and under the
    probe's launch that is seconds): everything after the last stretch
    longer than any Pace that begins in the first 15 s."""
    if not pulses:
        return []
    t0 = pulses[0][0]
    boot = [i for i, p in enumerate(pulses) if p[2] > 2_500 and p[0] - t0 < 15]
    return pulses[boot[-1] + 1:] if boot else pulses


def own_events(ev, partitions):
    """The run's events but the probe's own mark, with the Partition's name."""
    out = []
    for e in (ev or {}).get("events_of_the_run", []):
        if e["name"] == "debugger":
            continue
        out.append({"event": e["name"], "partition": partitions[e["partition"]] if e["partition"] < len(partitions) else None, "task": e["task"],
                    "instant_s": e["instant_ticks"] / TICK_HZ})
    return out


def partitions():
    model = json.load(open("apps/pacemaker/gen-nrf52840_rig/task_model.json"))
    return [p["id"] for p in model["partitions"]]


def run_restart():
    heart("silent")
    out, ok = {}, True
    interval = 60_000_000 // 71
    # The stated bound on the pause: the interval that was running, then
    # the restart and `control`'s first Job within 20 ms, then one
    # Power-On Reset interval with its tolerance.
    bound_us = interval + 20_000 + 923_077 + st.INTERVAL_TOLERANCE_US
    for kind, event in (("fault", "memmanage"), ("overrun", "overrun"), ("deadline", "deadline_miss")):
        pg = build("nrf52840,debug-port,quiet,rig,inject-once,trace-ram", "target/pacemaker-inject", {"PM_INJECT": kind})
        st.probe("reset", st.BOARD_C)
        # The battery level is set to ERN before the injection (it changes
        # nothing of VOO pacing). After a fault or an Overrun, whose instant
        # is known, the status is asked: `therapy` started again has the
        # level again, from `link` (decided 2026-11-24).
        cap, _, ev = run_attached(pg, 58, stub_ops=("up", "battery:1", f"set:0={VOO},1=71", f"wait:state={st.STATES['permanent']}"),
                                  late_ops=("up", "status") if kind != "deadline" else None)
        after_restart = cap.late[-1]["detail"] if cap.late else None
        level_kept = after_restart is None or (after_restart.get("battery") == 1 and after_restart.get("state") == st.STATES["por"])
        v, a = after_boot(cap.pulses(st.VENTRICULAR)), after_boot(cap.pulses(st.ATRIAL))
        # The programmed Paces are 0.40 ms wide, the Power-On Reset State's
        # 0.50 ms: the last of the first and the first of the second after it.
        programmed = [i for i, p in enumerate(v) if abs(p[2] - 400) <= st.WIDTH_TOLERANCE_US]
        last = programmed[-1] if programmed else None
        first = last + 1 if last is not None and last + 1 < len(v) else None
        before = [v[i] for i in programmed]
        after = v[first:] if first is not None else []
        pause_us = (after[0][1] - before[-1][1]) if before and after else None
        evs = own_events(ev, partitions())
        good = (pause_us is not None and 923_077 - st.INTERVAL_TOLERANCE_US <= pause_us <= bound_us and len(before) >= 6 and len(after) >= 8
                and st.all_near([p[2] for p in before], 400, st.WIDTH_TOLERANCE_US) and st.all_near([p[2] for p in after], 500, st.WIDTH_TOLERANCE_US)
                and st.all_near(st.intervals(before), interval, st.INTERVAL_TOLERANCE_US) and st.all_near(st.intervals(after), 923_076, st.INTERVAL_TOLERANCE_US)
                and not a and bool(evs) and evs[0]["event"] == event and evs[0]["partition"] == "therapy" and cap.ops is not None and all(o["ok"] for o in cap.ops) and level_kept)
        ok = ok and good
        out[kind] = {"binary": st.sha(pg), "programmed_over_the_link": cap.ops, "status_after_the_restart": after_restart, "pause_us": pause_us, "bound_us": bound_us, "paces_before": len(before), "paces_after": len(after),
                     "intervals_before_us": st.spread(st.intervals(before)), "intervals_after_us": st.spread(st.intervals(after)),
                     "longest_high_us": {"ventricular": max([p[2] for p in v], default=0), "atrial": max([p[2] for p in a], default=0)},
                     "last_pace_before_s_into_the_capture": before[-1][1] / st.RATE if before else None,
                     "kernel_events": evs, "verdict": "PASS" if good else "FAIL"}
        print(f"  restart by {kind}: {'ok  ' if good else 'FAIL'} pause {pause_us and pause_us / 1000} ms (bound {bound_us / 1000}); {len(before)} Paces before, {len(after)} after; "
              f"longest high {out[kind]['longest_high_us']}; first event {evs[:1]}; after the restart {after_restart and {k: after_restart[k] for k in ('state', 'battery')}}", flush=True)
    out["verdict"] = "PASS" if ok else "FAIL"
    return out


def run_lower():
    h = heart("fixed")
    st.sh(["cargo", "build", "-q", "-p", "pm-judge"])
    out, ok = {}, True
    for name, kind, stop, events in (("fault", "fault", False, ("memmanage",)), ("overrun", "overrun", False, ("overrun", "end_job")),
                                     ("invalid set", "invalid", False, ("message_rejected",)),
                                     ("flood", "mflood", False, ("endpoint_full", "protocol_violation")), ("stopped", "fault", True, ("memmanage",))):
        features = "nrf52840,debug-port,quiet,rig,set-env,misbehave,trace-ram" + (",rig-stop" if stop else "")
        pg = build(features, "target/pacemaker-misbehave", {"PM_SET": f"0={VVI},1=90", "PM_LINK_FAULT": kind})
        cap, launched, ev = run_attached(pg, 30, capture_after=14.0)
        capture, report = os.path.join(st_scratch(), "capture.bin"), os.path.join(st_scratch(), "judge.json")
        cap.data.tofile(capture)
        if os.path.exists(report):
            os.remove(report)
        judged = st.sh(["target/debug/pm-judge", capture, str(st.RATE), "--mode", "VVI", "--set", "LRL=90", "--scenario", "fixed",
                        "--binary", f"pacemaker={pg}", "--binary", f"heart-sim={h}", "--json", report], check=False)
        j = json.load(open(report)) if os.path.exists(report) else {"verdict": "NO REPORT", "error": judged.stderr[-500:]}
        counts = e1_runs.counts(pg)
        evs = own_events(ev, partitions())
        vj = j.get("ventricular", {})
        good = (j.get("verdict") == "PASS" and counts is not None and counts["started"] == 1 and counts["hw_fault"] == 0 and counts["records"] > 60
                and bool(evs) and evs[0]["event"] in events and evs[0]["partition"] == "comms")
        ok = ok and good
        out[name] = {"binary": st.sha(pg), "declaration": "nrf52840_rig_stop" if stop else "nrf52840_rig", "misbehaviour": kind,
                     "capture": "30 s, begun some seconds before the onset at 10 s of the run", "judge": j, "therapy_counts_at_the_end": counts,
                     "kernel_events": evs, "verdict": "PASS" if good else "FAIL"}
        print(f"  link {name}: {'ok  ' if good else 'FAIL'} judge {j.get('verdict')} paces {vj.get('paces')} intervals {vj.get('pace_intervals')} senses {vj.get('senses')} "
              f"{j.get('violations', [])[:2]}; therapy {counts and {k: counts[k] for k in ('started', 'records')}}; first event {evs[:1]}", flush=True)
    out["verdict"] = "PASS" if ok else "FAIL"
    return out


def st_scratch():
    d = os.environ.get("PM_SCRATCH", "target/pm-rig-scratch")
    os.makedirs(d, exist_ok=True)
    return d


def run_safe_state():
    heart("silent")
    pg = build("nrf52840,debug-port,quiet,rig,set-env,trace-ram", "target/pacemaker-safe", {"PM_SET": f"0={VOO},1=120,11=1900"})
    cap, _, ev = run_attached(pg, 64)
    v, a = after_boot(cap.pulses(st.VENTRICULAR)), after_boot(cap.pulses(st.ATRIAL))
    evs = own_events(ev, partitions())
    last = v[-1] if v else None
    span_s = (v[-1][1] - v[0][1]) / st.RATE if v else None
    quiet_s = (len(cap.data) - (last[1] + last[2])) / st.RATE if last else None
    end_level = int(cap.data[-1]) & 3
    names = [e["event"] for e in evs]
    good = (len(v) >= 80 and st.all_near([p[2] for p in v], 1900, st.WIDTH_TOLERANCE_US) and st.all_near(st.intervals(v), 500_000, st.INTERVAL_TOLERANCE_US)
            and not a and quiet_s is not None and quiet_s >= 6 and span_s <= DURATION and end_level == 0 and names == ["operating_end", "safe_state"])
    print(f"  safe state: {'ok  ' if good else 'FAIL'} {len(v)} Paces over {span_s} s, then {quiet_s} s without one to the capture's end; widths {st.spread([p[2] for p in v])}; "
          f"pins at the end {end_level:02b}; events {names}", flush=True)
    return {"binary": st.sha(pg), "paces": len(v), "from_the_first_pace_to_the_last_s": span_s, "seconds_without_a_pulse_to_the_end_of_the_capture": quiet_s,
            "widths_us": st.spread([p[2] for p in v]), "intervals_us": st.spread(st.intervals(v)), "atrial_pulses": len(a),
            "pin_levels_at_the_end_of_the_capture": {"atrial": end_level & 1, "ventricular": end_level >> 1}, "kernel_events": evs, "verdict": "PASS" if good else "FAIL"}


RUNS = {"restart": ("17.1", run_restart), "lower": ("17.2", run_lower), "safe_state": ("17.3", run_safe_state)}


def main():
    record = sys.argv[1]
    chosen = [a for a in sys.argv[2:] if a in RUNS] or list(RUNS)
    st.sh(["cargo", "build", "-q", "-p", "dcm-stub"])
    kept = json.load(open(record)) if os.path.exists(record) else {"runs": {}}
    kept["boards"] = {"A": st.BOARD_A, "B": st.BOARD_B, "C": st.BOARD_C}
    kept["capture"] = {"rate_hz": st.RATE, "pins": "D0 the atrial pace pin, D1 the ventricular", "width_tolerance_us": st.WIDTH_TOLERANCE_US,
                       "interval_tolerance_us": st.INTERVAL_TOLERANCE_US}
    ok = True
    for name in chosen:
        task, run = RUNS[name]
        print(f"{name} (task {task})", flush=True)
        out = run()
        out["task"] = task
        kept["runs"][name] = out
        json.dump(kept, open(record, "w"), indent=1)
        print(f"{name}: {out['verdict']}", flush=True)
        ok = ok and out["verdict"] == "PASS"
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
