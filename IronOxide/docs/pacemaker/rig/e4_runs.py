#!/usr/bin/env python3
"""Tasks 20.2, 20.3 and 21 of docs/pacemaker/tasks.md: the rig runs of
stage E4.

Usage: e4_runs.py <record.json> [activity] [hysteresis] [smoothing_down]
                  [smoothing_up] [atr] [storm] [modes]

Run from the workspace root with rsk's tools on PATH. Board A carries the
pulse generator, board B the Heart_Simulator, board C the bridge; the
DCM_Stub (`dcm-stub do`) programs the sets and gives the activity value.
Every run captures the pace pins (D0 atrial, D1 ventricular) and the
Heart_Simulator's sense mirrors (D6 atrial, D7 ventricular) at 1 MHz, each
piece stamped with the host's clock as in `e3_states.py`.

The judges are written from the requirements, not from `pm-core`.

  activity        (20.2) VVIR at LRL 60, MSR 120, Reaction Time 10 s,
                  Recovery Time 2 min, Response Factor 16; a silent heart.
                  The activity value 0, then 255 for 22 s, then 0 for 32 s.
                  Every pace interval against the rate the rule of PM15.5
                  and PM15.6 gives for the instants it spans; the plateau
                  against the MSR interval within PM-PAR-02; the rise timed.
  hysteresis      (20.2) VVI at LRL 60 with HRL 40. A ventricular rhythm of
                  800 ms for 8 s, silence, one beat. After a Sense the Pace
                  is due one HRL interval later, after a Pace one LRL
                  interval later (PM16.1, PM16.2).
  smoothing_down  (20.2) VVI at LRL 60 with Rate Smoothing Down 12 %. A
                  ventricular rhythm of 600 ms for 10 s, then silence: each
                  interval at most the cycle before plus 12 % (PM16.3).
  smoothing_up    (20.2) VVIR as in `activity` with Rate Smoothing Up 3 %:
                  under the rising sensor rate no interval is shorter than
                  the one before less 3 %.
  atr             (20.2) DDD at LRL 60, URL 120, ATR on, Duration 10
                  cycles, Fallback Time 1 min. An atrial rhythm of 900 ms
                  without conduction, with atrial beats every 300 ms from
                  9 to 31 s of the heart's time. The detection counter of
                  PM14.1 is run over the atrial events of the capture, and
                  from it the ventricular events at which Fallback begins
                  and ends; the Paces before and after are judged as DDD
                  (the judge of `e2_runs.py`), those between against the
                  Fallback line (PM14.6); the Mode the status reports.
  storm           (20.3) the rig variant with the Kernel's event mirror,
                  under an attached probe. VVI with a storm of edges (one a
                  millisecond) on the ventricular line and then on both;
                  DDD with the storm on the atrial line and then on both.
                  Pacing at the LRL throughout, the Kernel's events, the
                  driver's counts.
  modes           (21) every Mode of Table 19-1 programmed over the link
                  against a silent heart: the Mode the status reports, the
                  Chambers paced, the intervals and the AV delay; in a
                  rate-adaptive Mode also the MSR interval after 13 s of
                  the activity value 255.

Exit 0 when every run chosen passed.
"""
import json
import os
import sys
import time

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import e1_runs as e1  # noqa: E402
import e2_runs as e2  # noqa: E402
import e3_faults as ef  # noqa: E402
import e3_states as st  # noqa: E402

RATE = st.RATE
TOL = 8_000          # PM-PAR-01
TOL_MSR = 4_000      # PM-PAR-02
EDGE = 8_000
WINDOW = 4_000       # PM-PAR-15
LRL_US, MSR_US, HRL_US, URL_US = 1_000_000, 500_000, 1_500_000, 500_000
VRP_US = 320_000
AV_US = 150_000
PERMANENT = st.STATES["permanent"]
# The Mode codes of Table 19-1 and what each paces; the Mode byte of a status.
MODES = [("Off", "", 0), ("DDD", "AV", 63), ("VDD", "V", 62), ("DDI", "AV", 47), ("DOO", "AV", 3), ("AOO", "A", 1), ("AAI", "A", 37), ("VOO", "V", 2),
         ("VVI", "V", 42), ("AAT", "A", 21), ("VVT", "V", 26)]
MODES += [(name + "R", paces, byte + 64) for name, paces, byte in MODES[1:9]]
CODE = {name: k for k, (name, _, _) in enumerate(MODES)}
ADAPTIVE = "3=120,24=3,25=10,26=16,27=2"
FLASHED = {}


def flash_heart(numbers):
    elf, text = e1.build_heart(numbers)
    if FLASHED.get("heart") != e1.sha(elf):
        st.probe("download", st.BOARD_B, "--disable-progressbars", elf)
        FLASHED["heart"] = e1.sha(elf)
    return elf, text


def session(heart, ops, seconds):
    """The heart restarted, the pulse generator restarted (it starts in the
    Power-On Reset State), a capture, and the stub's operations."""
    elf, text = flash_heart(heart)
    st.probe("reset", st.BOARD_B)
    st.start_pg()
    cap = st.Capture(seconds)
    cap.started()
    lines = st.stub(*ops)
    cap.finish()
    return cap, lines, {"heart": text, "heart_binary": e1.sha(elf)}


def sample_of(cap, host):
    return int((host - cap.t0) * RATE)


def op(lines, name, nth=0):
    found = [l for l in lines if l["op"].split(":")[0] == name]
    return found[nth] if len(found) > nth else None


def rises(cap, bit):
    d = np.diff(((cap.data >> bit) & 1).astype(np.int8))
    return np.flatnonzero(d == 1) + 1


def programmed_from(cap, lines):
    """The sample from which the programmed set paces: the stub's wait for
    the Permanent State is over, and one LRL interval more."""
    w = op(lines, "wait")
    return sample_of(cap, w["done"]) + 1_200_000 if w and w["ok"] else None


def wire(cap, since=0):
    """The capture's events from the sample `since`, in seconds of the
    capture: the pace pulses and the sense mirrors' edges (at most 400 of
    each, the count beside)."""
    out = {}
    for name, ts in (("atrial_paces", [p[1] for p in cap.pulses(st.ATRIAL)]), ("ventricular_paces", [p[1] for p in cap.pulses(st.VENTRICULAR)]),
                     ("atrial_senses", rises(cap, 6)), ("ventricular_senses", rises(cap, 7))):
        ts = [int(t) for t in ts if t >= since]
        out[name] = {"count": len(ts), "at_s": [t / 1e6 for t in ts[:400]]}
    return out


def programmed_at(cap, lines):
    """The sample at which the stub's wait for the Permanent State was over."""
    w = op(lines, "wait")
    return sample_of(cap, w["done"]) if w and w["ok"] else None


def result(checks, extra):
    for c in checks:
        print(f"    {'ok  ' if c['ok'] else 'FAIL'} {c['check']}: {json.dumps(c['detail'])[:260]}", flush=True)
    return {**extra, "checks": checks, "verdict": "PASS" if checks and all(c["ok"] for c in checks) else "FAIL"}


def check(checks, name, ok, detail):
    checks.append({"check": name, "ok": bool(ok), "detail": detail})


# ---- 20.2: the activity profile -------------------------------------------------

def sensor_rate(t, t_up, t_down, reaction_s=10.0, recovery_s=120.0, lrl=60.0, msr=120.0):
    """PM15.5, PM15.6: from the LRL to the MSR in the Reaction Time while
    the activity is above the threshold, back in the Recovery Time."""
    if t_up is None or t < t_up:
        return lrl
    top_at = t if t_down is None or t < t_down else t_down
    top = min(msr, lrl + (msr - lrl) * (top_at - t_up) / reaction_s)
    if t_down is None or t < t_down:
        return top
    return max(lrl, top - (msr - lrl) * (t - t_down) / recovery_s)


def run_activity():
    ops = ["up", f"set:0={CODE['VVIR']},1=60,{ADAPTIVE}", f"wait:state={PERMANENT}", "hold:6", "activity:255", "hold:22", "activity:0", "hold:32", "status"]
    cap, lines, extra = session([0, 0, 0, 0, 0], ops, 70)
    checks = []
    start = programmed_from(cap, lines)
    up, down = op(lines, "activity", 0), op(lines, "activity", 1)
    check(checks, "every operation of the stub succeeded, both activity values accepted", all(l["ok"] for l in lines) and up and down and up["detail"]["accepted"] and down["detail"]["accepted"],
          [l["op"] for l in lines if not l["ok"]])
    if start is None or not up or not down:
        return result(checks, {**extra, "ops": lines})
    t_up, t_down = up["sent"], down["sent"]
    v = [p for p in cap.pulses(st.VENTRICULAR) if p[1] >= start]
    # The command is in force between its send and half a second later (one
    # transfer, one Job of `link`, one of `control`, and the capture's own
    # delay to the host, which makes a pulse's host time late).
    late = 0.5
    bad, plateau, worst = [], [], 0.0
    for a, b in zip(v, v[1:]):
        interval = b[1] - a[1]
        rates = [sensor_rate(t, tu, td) for t in np.linspace(a[0] - 0.2, b[0], 40) for tu, td in ((t_up, t_down), (t_up + late, t_down + late))]
        lo, hi = 60e6 / max(rates), 60e6 / min(rates)
        at_msr = min(rates) >= 120.0
        tol = TOL_MSR if at_msr else TOL
        e = max(lo - tol - interval, interval - hi - tol, 0.0)
        worst = max(worst, max(lo - interval, interval - hi, 0.0))
        if at_msr:
            plateau.append(interval)
        if e > 0:
            bad.append({"at_s": round(a[0] - t_up, 3), "interval_us": interval, "allowed_us": [round(lo), round(hi)]})
    check(checks, "every pace interval is the interval of a rate the rule gives for the instants it spans, within PM-PAR-01", len(v) > 40 and not bad,
          {"paces": len(v), "outside": bad[:5], "furthest_outside_the_span_us": round(worst)})
    check(checks, "at the plateau every interval is the MSR interval within PM-PAR-02 (4 ms)", len(plateau) >= 10 and st.all_near(plateau, MSR_US, TOL_MSR), st.spread(plateau))
    before = [b[1] - a[1] for a, b in zip(v, v[1:]) if b[0] < t_up]
    check(checks, "before the activity every interval is the LRL interval", len(before) >= 3 and st.all_near(before, LRL_US, TOL), st.spread(before))
    first_msr = next((b for a, b in zip(v, v[1:]) if b[0] > t_up and b[1] - a[1] <= MSR_US + TOL_MSR), None)
    rise_s = first_msr[0] - t_up if first_msr else None
    # The first interval at the MSR begins when the rate arrives, 10 s after
    # the command took effect, and ends one MSR interval later.
    check(checks, "the rise from the LRL to the MSR takes the Reaction Time (10 s; the first interval at the MSR ends 9.8 to 11.6 s after the send)",
          rise_s is not None and 9.8 <= rise_s <= 11.6, {"first_interval_at_the_msr_ends_s_after_the_send": rise_s})
    after = [(b[0] - t_down, 60e6 / (b[1] - a[1])) for a, b in zip(v, v[1:]) if a[0] > t_down + 1.5]
    fall = None
    if len(after) >= 10:
        slope = np.polyfit([x[0] for x in after], [x[1] for x in after], 1)[0]
        fall = {"ppm_per_s": float(slope), "seconds_observed": after[-1][0] - after[0][0], "recovery_time_it_gives_s": float(-60.0 / slope) if slope < 0 else None,
                "rate_at_the_end_ppm": after[-1][1]}
    check(checks, "the fall is at the rate of the Recovery Time (2 min from the MSR to the LRL: 0.5 ppm a second, within a tenth)",
          fall is not None and fall["recovery_time_it_gives_s"] is not None and 108 <= fall["recovery_time_it_gives_s"] <= 132, fall)
    widths = [p[2] for p in v]
    check(checks, "every pulse is 0.40 ms wide and the atrium is not paced", st.all_near(widths, 400, st.WIDTH_TOLERANCE_US) and not [p for p in cap.pulses(st.ATRIAL) if p[1] >= start],
          st.spread(widths))
    return result(checks, {**extra, "ops": lines, "sent_s": {"activity_255": t_up, "activity_0": t_down}, "capture_began_s": cap.t0, "wire": wire(cap, start)})


# ---- 20.2: hysteresis and Rate Smoothing Down ------------------------------------

def judge_escape(cap, start, due, label):
    """A ventricular single-chamber Mode that inhibits: after each
    ventricular event the next Pace is due `due(kind, cycle)` later, where
    kind is "S" or "P" and cycle the interval that ended at the event.
    The capture is followed from its beginning, so that the last event and
    the cycle are known; the Paces are judged from the second ventricular
    event after `start` (the first is the one the programmed set's first
    interval begins at)."""
    ev = sorted([(int(t), "S") for t in rises(cap, 7)] + [(p[1], "P") for p in cap.pulses(st.VENTRICULAR)])
    last = kind = cycle = None
    errors, violations, counts = {}, [], {"refractory_senses": 0, "senses": 0, "unjudged": 0}
    seen = 0
    for t, k in ev:
        judging = seen >= 1
        if t >= start and last is not None and not (k == "S" and t - last < VRP_US + EDGE):
            seen += 1
        if last is None:
            if k == "P":
                last, kind, cycle = t, "P", None
            continue
        d = due(kind, cycle)
        if k == "S":
            since = t - last
            if abs(since - VRP_US) <= EDGE:
                last = None
                counts["unjudged"] += judging
                continue
            if since < VRP_US:
                counts["refractory_senses"] += judging
                continue
            if judging and since > d + TOL:
                violations.append(f"{t / 1e6:.3f} s: the Pace due {d / 1000:.1f} ms after the event at {last / 1e6:.3f} s did not come")
            if judging and since >= d - WINDOW:
                # Either reading: the Pace may still come.
                last = None
                counts["unjudged"] += 1
                continue
            counts["senses"] += judging
            last, kind, cycle = t, "S", since
        else:
            if judging:
                e = (t - last) - d
                name = label(kind, cycle)
                s = errors.setdefault(name, {"count": 0, "min": e, "max": e})
                s["count"] += 1
                s["min"], s["max"] = min(s["min"], e), max(s["max"], e)
                if abs(e) > TOL:
                    violations.append(f"{t / 1e6:.3f} s: a Pace {e} µs from its instant ({name}: {d} µs after the event at {last / 1e6:.3f} s)")
            last, kind, cycle = t, "P", t - last
    return {"errors_us": errors, "violations": violations[:10], "counts": counts}


def run_hysteresis():
    ops = ["up", f"set:0={CODE['VVI']},1=60,18=40", f"wait:state={PERMANENT}", "hold:30", "status"]
    # The rhythm's last beat is at 13,200 ms; the Paces after it are at 14,700, 15,700 ...; the single beat 700 ms after one.
    cap, lines, extra = session([0, 0, 0, 0, 0, 2, 6000, 14000, 800, 2, 22400, 22401, 1], ops, 34)
    checks = []
    start = programmed_at(cap, lines)
    check(checks, "every operation of the stub succeeded", all(l["ok"] for l in lines), [l["op"] for l in lines if not l["ok"]])
    if start is None:
        return result(checks, {**extra, "ops": lines})
    j = judge_escape(cap, start, lambda kind, cycle: HRL_US if kind == "S" else LRL_US, lambda kind, cycle: "hrl_after_a_sense" if kind == "S" else "lrl_after_a_pace")
    e = j["errors_us"]
    check(checks, "every Pace is at its instant within PM-PAR-01: one HRL interval (1500 ms) after a Sense, one LRL interval (1000 ms) after a Pace", not j["violations"], j)
    check(checks, "both cases were met: a Pace after the rhythm's last beat and one after the single beat, and Paces after Paces",
          e.get("hrl_after_a_sense", {}).get("count", 0) >= 2 and e.get("lrl_after_a_pace", {}).get("count", 0) >= 8 and j["counts"]["senses"] >= 4, {k: v["count"] for k, v in e.items()})
    return result(checks, {**extra, "ops": lines, "wire": wire(cap, start)})


def run_smoothing_down():
    ops = ["up", f"set:0={CODE['VVI']},1=60,28=12", f"wait:state={PERMANENT}", "hold:26", "status"]
    cap, lines, extra = session([0, 0, 0, 0, 0, 2, 6000, 16000, 600], ops, 30)
    checks = []
    start = programmed_at(cap, lines)
    check(checks, "every operation of the stub succeeded", all(l["ok"] for l in lines), [l["op"] for l in lines if not l["ok"]])
    if start is None:
        return result(checks, {**extra, "ops": lines})

    def due(kind, cycle):
        return LRL_US if cycle is None else min(LRL_US, cycle + cycle * 12 // 100)

    j = judge_escape(cap, start, due, lambda kind, cycle: "lrl" if cycle is None or cycle + cycle * 12 // 100 >= LRL_US else "smoothed")
    e = j["errors_us"]
    check(checks, "every Pace is at its instant within PM-PAR-01: the cycle before plus 12 % of it, the LRL interval at most", not j["violations"], j)
    v = [p for p in cap.pulses(st.VENTRICULAR) if p[1] >= start]
    senses = [int(t) for t in rises(cap, 7) if t >= start]
    after = st.intervals([p for p in v if senses and p[1] > senses[-1]])
    check(checks, "after the rhythm stops the intervals lengthen step by step to the LRL interval (smoothed Paces met, then Paces at the LRL)",
          e.get("smoothed", {}).get("count", 0) >= 4 and e.get("lrl", {}).get("count", 0) >= 3, {"counts": {k: x["count"] for k, x in e.items()}, "intervals_after_the_last_beat_us": after[:8]})
    return result(checks, {**extra, "ops": lines, "wire": wire(cap, start)})


def run_smoothing_up():
    ops = ["up", f"set:0={CODE['VVIR']},1=60,{ADAPTIVE},19=3", f"wait:state={PERMANENT}", "hold:6", "activity:255", "hold:26", "status"]
    cap, lines, extra = session([0, 0, 0, 0, 0], ops, 38)
    checks = []
    start = programmed_from(cap, lines)
    up = op(lines, "activity", 0)
    check(checks, "every operation of the stub succeeded", all(l["ok"] for l in lines) and up and up["detail"]["accepted"], [l["op"] for l in lines if not l["ok"]])
    if start is None or not up:
        return result(checks, {**extra, "ops": lines})
    v = [p for p in cap.pulses(st.VENTRICULAR) if p[1] >= start]
    gaps = st.intervals(v)
    bad, on_the_limit, steps = [], 0, []
    for a, b in zip(gaps, gaps[1:]):
        floor = a - a * 3 // 100
        if b < floor - TOL or b > a + TOL or b < MSR_US - TOL_MSR:
            bad.append({"before_us": a, "interval_us": b, "least_allowed_us": floor})
        if b < a - 2_000:
            steps.append(b - floor)
            on_the_limit += abs(b - floor) <= 1_500
    check(checks, "no interval is shorter than the one before less 3 % (within PM-PAR-01), longer than the one before, or shorter than the MSR interval", len(gaps) > 20 and not bad,
          {"intervals": len(gaps), "outside": bad[:5]})
    check(checks, "the rise is limited by the smoothing: most shortened intervals are the one before less 3 %, within 1.5 ms", len(steps) >= 15 and on_the_limit >= len(steps) - 3,
          {"shortened_intervals": len(steps), "of_them_on_the_limit": int(on_the_limit), "distance_from_the_limit_us": st.spread(steps)})
    check(checks, "the rate arrives at the MSR: the last intervals are the MSR interval within PM-PAR-02", st.all_near(gaps[-3:], MSR_US, TOL_MSR), gaps[-3:])
    reached = next((b for a, b in zip(v, v[1:]) if b[1] - a[1] <= MSR_US + TOL_MSR), None)
    return result(checks, {**extra, "ops": lines, "first_interval_at_the_msr_ends_s_after_the_send": reached[0] - up["sent"] if reached else None,
                           "without_smoothing_s": "see the run `activity`", "wire": wire(cap, start)})


# ---- 20.2: the Atrial Tachycardia Response ---------------------------------------

def run_atr():
    ops = ["up", f"set:0={CODE['DDD']},1=60,20=1,21=10,22=1", f"wait:state={PERMANENT}"] + ["hold:2", "status"] * 24
    cap, lines, extra = session([600, 900, 0, 0, 0, 1, 9000, 31000, 300], ops, 56)
    checks = []
    start = programmed_from(cap, lines)
    check(checks, "every operation of the stub succeeded", all(l["ok"] for l in lines), [l["op"] for l in lines if not l["ok"]])
    if start is None:
        return result(checks, {**extra, "ops": lines})
    a_events = sorted([int(t) for t in rises(cap, 6)] + [p[1] for p in cap.pulses(st.ATRIAL)])
    v = [p for p in cap.pulses(st.VENTRICULAR)]
    vt = [p[1] for p in v]
    # PM14.1, PM14.2: the detection counter over the atrial events.
    counter, onset, cessation, trace = 0, None, None, []
    for prev, t in zip(a_events, a_events[1:]):
        if t < start:
            continue
        counter = min(8, counter + 1) if t - prev < URL_US else max(0, counter - 1)
        if onset is None and counter >= 8:
            onset = t
        if onset is not None and cessation is None and counter == 0:
            cessation = t
    check(checks, "the counter of PM14.1, run over the capture's atrial events, reaches 8 and later returns to 0", onset is not None and cessation is not None,
          {"onset_s": onset and onset / 1e6, "cessation_s": cessation and cessation / 1e6})
    if onset is None or cessation is None:
        return result(checks, {**extra, "ops": lines})
    after_onset = [k for k, t in enumerate(vt) if t > onset]
    n_fb = after_onset[9] if len(after_onset) > 9 else None          # PM14.4: the tenth cardiac cycle
    after_c = [k for k, t in enumerate(vt) if t > cessation]
    n_c = after_c[0] if after_c else None                            # PM14.7, PM14.8
    if n_fb is None or n_c is None or n_c + 4 >= len(vt) or cessation < vt[n_fb]:
        check(checks, "the capture holds Fallback's beginning and end", False, {"n_fb": n_fb, "n_c": n_c, "ventricular_paces": len(vt)})
        return result(checks, {**extra, "ops": lines})
    from_cycle = vt[n_fb] - vt[n_fb - 1]
    r0 = 60e6 / from_cycle

    def fallback_interval(elapsed_us):
        rate = max(60.0, r0 - (r0 - 60.0) * elapsed_us / 60e6)
        return min(LRL_US, 60e6 / rate)

    # Fallback: every interval that begins at the event Fallback began at, or
    # later, until the event it ends at.
    line_errors = [(vt[k] - vt[k - 1]) - fallback_interval(vt[k - 1] - vt[n_fb]) for k in range(n_fb + 1, n_c + 1)]
    check(checks, "Fallback begins at the tenth ventricular event after the onset: from that event every pace interval is on the Fallback line "
          "(from the rate of the cycle at which Fallback began to the LRL over 1 min) within PM-PAR-01",
          len(line_errors) >= 10 and all(abs(e) <= TOL for e in line_errors),
          {"fallback_begins_at_ventricular_event": n_fb, "at_s": vt[n_fb] / 1e6, "cycle_it_began_at_us": from_cycle, "intervals_on_the_line": len(line_errors),
           "error_us": st.spread([round(e) for e in line_errors]), "first_and_last_interval_us": [vt[n_fb + 1] - vt[n_fb], vt[n_c] - vt[n_c - 1]]})
    # The Mode changes at that event too (PM14.6, PM14.8; finding B41: until
    # 2026-11-25 it changed one ventricular event later). From it the atrium
    # is not paced, and the Paces do not follow the atrial events except by
    # coincidence with the line.
    ap = [p[1] for p in cap.pulses(st.ATRIAL)]
    ap_in_vvir = [t / 1e6 for t in ap if vt[n_fb] < t < vt[n_c]]
    not_tracked = [k for k in range(n_fb + 2, n_c + 1) if not any(abs(vt[k] - t - AV_US) <= TOL for t in a_events if vt[k] - 400_000 < t < vt[k])]
    check(checks, "from the ventricular event at which Fallback begins the Mode is VVIR: the atrium is not paced, and the Paces do not follow the atrial events",
          not ap_in_vvir and len(not_tracked) >= (n_c - n_fb - 1) * 2 // 3,
          {"atrial_paces": ap_in_vvir, "ventricular_paces": n_c - n_fb - 1, "of_them_not_one_av_delay_after_an_atrial_event": len(not_tracked)})
    # Before Fallback and after the return: DDD, by the judge of stage E2.
    sets = {"LRL": 60}
    end1 = vt[n_fb] + 2_000
    before = e2.judge(cap.data[:end1], RATE, "DDD", sets, skip_us=start)
    check(checks, "until Fallback begins the pacing is DDD's (the judge of stage E2 on the capture up to that ventricular event)", before["verdict"] == "PASS",
          {k: before[k] for k in ("judged_paces", "unjudged_stretches", "errors_us", "counts", "violations")})
    back = next((k for k in range(n_c, len(vt)) if any(abs(vt[k] - t - AV_US) <= TOL for t in a_events if vt[k] - 400_000 < t < vt[k])), None)
    after = e2.judge(cap.data[vt[n_c] - 2_000:], RATE, "DDD", sets, skip_us=0)
    check(checks, "Fallback ends at the ventricular event after the counter returns to 0, and the Mode is DDD again from that event: the next Pace follows an atrial event "
          "(the judge of stage E2 from that event to the end)", back == n_c + 1 and after["verdict"] == "PASS",
          {"fallback_ends_at_ventricular_event": n_c, "at_s": vt[n_c] / 1e6, "interval_from_it_us": vt[n_c + 1] - vt[n_c], "first_pace_after_an_atrial_event_is_event": back,
           **{k: after.get(k) for k in ("judged_paces", "unjudged_stretches", "errors_us", "counts", "violations")}})
    # The Mode the status reports, against the capture's instants.
    told = [(l["done"], l["detail"]["mode"]) for l in lines if l["op"] == "status" and l["ok"]]
    t_in, t_out = cap.time_of(vt[n_fb]), cap.time_of(vt[n_c])
    wrong = [(round(t - cap.t0, 2), m) for t, m in told if (t < t_in - 1.5 and m != 63) or (t_in + 1.5 < t < t_out - 1.5 and m != 106) or (t > t_out + 1.5 and m != 63)]
    check(checks, "the status reports DDD before, VVIR in Fallback and DDD after", not wrong and any(m == 106 for _, m in told) and told and told[-1][1] == 63,
          {"modes_told": [m for _, m in told], "against_the_capture": wrong})
    return result(checks, {**extra, "ops": lines, "counter": {"onset_s": onset / 1e6, "cessation_s": cessation / 1e6},
                           "ventricular_events": {"fallback_begins": n_fb, "fallback_ends": n_c, "paces_in_the_capture": len(vt)}, "capture_began_s": cap.t0, "wire": wire(cap)})


# ---- 20.3: the sense storm --------------------------------------------------------

def storm_edges(cap, bit):
    """The first and the last edge of the stretch in which the line's edges
    are no more than 2 ms apart."""
    r = rises(cap, bit)
    close = np.flatnonzero(np.diff(r) <= 2_000)
    return (int(r[close[0]]), int(r[close[-1] + 1]), int(len(close) + 1)) if len(close) else None


def run_storm():
    out, ok = {}, True
    for name, mode, first, more in (("vvi", "VVI", 2, ""), ("ddd", "DDD", 1, ""), ("ddd_atr", "DDD", 1, ",20=1,21=10,22=1")):
        text = f"0={CODE[mode]},1=60{more}"
        pg = ef.build("nrf52840,debug-port,quiet,rig,set-env,trace-ram", "target/pacemaker-storm", {"PM_SET": text})
        elf, heart_text = flash_heart([0, 0, 0, 0, 0, first, 25000, 33000, 1, 3, 33000, 41000, 1])
        cap, _, ev = ef.run_attached(pg, 58)
        counts = e1.counts(pg)
        checks = []
        v, a = ef.after_boot(cap.pulses(st.VENTRICULAR)), ef.after_boot(cap.pulses(st.ATRIAL))
        sa, sv = storm_edges(cap, 6), storm_edges(cap, 7)
        check(checks, "the storm is on the wire: an edge a millisecond on the first line for 8 s, then on both for 8 s",
              sa is not None and sv is not None and (sv if first == 2 else sa)[2] >= 15_500 and (sa if first == 2 else sv)[2] >= 7_500,
              {"atrial": sa and {"first_s": sa[0] / 1e6, "last_s": sa[1] / 1e6, "edges": sa[2]}, "ventricular": sv and {"first_s": sv[0] / 1e6, "last_s": sv[1] / 1e6, "edges": sv[2]}})
        if sa is None or sv is None:
            out[name] = result(checks, {"PM_SET": text, "heart": heart_text})
            ok = False
            continue
        begin, end = min(sa[0], sv[0]), max(sa[1], sv[1])
        # The first edge of a storm comes after silence and is a Sense like
        # any other (PM13.1 makes a cycle noisy from the end of the
        # refractory period): the cycle it falls in is left out.
        onsets = sorted({sa[0], sv[0]})
        gaps = [(x[1], y[1] - x[1]) for x, y in zip(v, v[1:])]
        judged = [(t, g) for t, g in gaps if not any(t <= o <= t + g for o in onsets)]
        inside = [g for t, g in judged if begin <= t and t + g <= end]
        outside = [g for t, g in judged if t + g < begin or t > end]
        check(checks, "in the storm every pace interval is the LRL interval within PM-PAR-01: pacing is asynchronous", len(inside) >= 12 and st.all_near(inside, LRL_US, TOL),
              {"intervals": st.spread(inside), "cycles_left_out_at_a_storms_first_edge": len(gaps) - len(judged)})
        check(checks, "before and after the storm every pace interval is the LRL interval", len(outside) >= 10 and st.all_near(outside, LRL_US, TOL), st.spread(outside))
        expected = (end - begin) / LRL_US
        paced = len([p for p in v if begin <= p[1] <= end])
        check(checks, "no Pace is missing in the storm", abs(paced - expected) <= 2, {"paces": paced, "storm_s": (end - begin) / 1e6})
        if more:
            # ATR on: the storm's atrial edges are atrial intervals shorter than
            # the URL interval (PM14.1 counts every atrial interval), so ATR
            # detects an onset, and after ATR Duration the Mode is VVIR at the
            # LRL (the cycle Fallback begins at is the LRL's). Reported.
            in_storm = [p[1] / 1e6 for p in a if begin <= p[1] <= end]
            check(checks, "with ATR on the storm is taken for an atrial tachycardia (reported): the atrial Paces in the storm stop when Fallback begins, and those after it", True,
                  {"atrial_paces_in_the_storm": len(in_storm), "the_last_s_after_the_storms_first_edge": in_storm[-1] - begin / 1e6 if in_storm else None,
                   "atrial_paces_after_the_storm": len([p for p in a if p[1] > end])})
        elif mode == "DDD":
            # Every ventricular Pace of the storm but those of the cycles a
            # storm's first edge falls in (that edge is a Sense: it is
            # tracked, or inhibits the atrial Pace, and may come too late
            # for a Pace already begun to be withdrawn).
            av, without = [], []
            for x, y in zip(v, v[1:]):
                if not (begin <= x[1] and y[1] <= end) or any(x[1] <= o <= y[1] for o in onsets):
                    continue
                before = [y[1] - ap[1] for ap in a if x[1] < ap[1] < y[1]]
                (av if before else without).append(before[-1] if before else y[1] / 1e6)
            left_out = [{"onset_s": o / 1e6, "atrial_paces_in_its_cycle": len([ap for x, y in zip(v, v[1:]) if x[1] <= o <= y[1] for ap in a if x[1] < ap[1] < y[1]])} for o in onsets]
            check(checks, "in the storm the atrium is paced before every ventricular Pace by the AV delay (the cycles of a storm's first edge left out and reported)",
                  len(av) >= 12 and not without and st.all_near(av, AV_US, TOL), {"av_delays_us": st.spread(av), "ventricular_paces_without_an_atrial_pace_s": without, "cycles_left_out": left_out})
        else:
            check(checks, "the atrium is not paced", not a, len(a))
        evs = ef.own_events(ev, ef.partitions())
        names = [e["event"] for e in evs]
        check(checks, "the Kernel's log holds the end of the Operating_Duration and the safe state and nothing else: no restart, no MIT violation, no Overrun, no deadline miss",
              names == ["operating_end", "safe_state"], evs)
        check(checks, "the driver's counts were read and show no forced or late arming (a withdrawal that came too late is possible at a storm's first edge, one for each)", counts is not None and counts["hw_fault"] == 0 and counts["forced"] == 0 and counts["late_arms"] == 0 and counts["too_late"] <= len(onsets), counts)
        widths = [p[2] for p in v + a]
        check(checks, "every pulse is 0.40 ms wide", st.all_near(widths, 400, st.WIDTH_TOLERANCE_US), st.spread(widths))
        print(f"  storm, {mode}:", flush=True)
        out[name] = result(checks, {"PM_SET": text, "heart": heart_text, "binaries": {"pacemaker": st.sha(pg), "heart-sim": st.sha(elf)}, "kernel_events": evs, "counts": counts})
        ok = ok and out[name]["verdict"] == "PASS"
    out["verdict"] = "PASS" if ok else "FAIL"
    # The link build again for the runs after this one.
    st.probe("download", st.BOARD_A, "--disable-progressbars", st.PG)
    return out


# ---- 21: every Mode ----------------------------------------------------------------

def run_modes():
    out, ok = {}, True
    for code, (name, paces, byte) in enumerate(MODES):
        adaptive = name.endswith("R")
        ops = ["up", f"set:0={code},1=60" + (f",{ADAPTIVE}" if adaptive else ""), f"wait:mode={byte}", "hold:7", "status"]
        if adaptive:
            ops += ["activity:255", "hold:13", "status"]
        cap, lines, extra = session([0, 0, 0, 0, 0], ops, 26 if adaptive else 12)
        checks = []
        w, told = op(lines, "wait"), op(lines, "status")
        check(checks, "the set is accepted and the status reports the Mode in force, in the Permanent State",
              all(l["ok"] for l in lines) and op(lines, "set")["detail"]["accepted"] and told["detail"]["mode"] == byte and told["detail"]["state"] == PERMANENT,
              {"failed": [l["op"] for l in lines if not l["ok"]], "status": told and told["detail"]})
        if not w or not w["ok"]:
            out[name] = result(checks, {**extra, "ops": lines})
            ok = False
            continue
        begin = sample_of(cap, w["done"]) + 1_200_000
        end = sample_of(cap, op(lines, "activity")["sent"]) if adaptive else sample_of(cap, told["done"])
        a = [p for p in cap.pulses(st.ATRIAL) if begin <= p[1] < end]
        v = [p for p in cap.pulses(st.VENTRICULAR) if begin <= p[1] < end]
        ga, gv = st.intervals(a), st.intervals(v)
        want_a, want_v = "A" in paces, "V" in paces
        check(checks, f"at the LRL the Mode paces {'no Chamber' if not paces else 'the atrium and the ventricle' if paces == 'AV' else 'the atrium alone' if paces == 'A' else 'the ventricle alone'}"
              ", every interval the LRL interval within PM-PAR-01",
              (len(a) >= 4 and st.all_near(ga, LRL_US, TOL) if want_a else not a) and (len(v) >= 4 and st.all_near(gv, LRL_US, TOL) if want_v else not v),
              {"atrial": {"paces": len(a), "intervals_us": st.spread(ga)}, "ventricular": {"paces": len(v), "intervals_us": st.spread(gv)}})
        if paces == "AV":
            av = [min((vp[1] - ap[1] for vp in v if vp[1] > ap[1]), default=None) for ap in a[:-1]]
            check(checks, "the ventricular Pace follows the atrial one by the AV delay (150 ms)", len(av) >= 3 and st.all_near([x for x in av if x is not None], AV_US, TOL), st.spread([x for x in av if x is not None]))
        if adaptive:
            last = sample_of(cap, lines[-1]["done"])
            pulses = [p for p in cap.pulses(st.ATRIAL if paces == "A" else st.VENTRICULAR) if last - 2_200_000 <= p[1] < last]
            g = st.intervals(pulses)
            check(checks, "after 13 s of the activity value 255 the intervals are the MSR interval within PM-PAR-02", len(g) >= 3 and st.all_near(g, MSR_US, TOL_MSR), st.spread(g))
        widths = [p[2] for p in a + v]
        if paces:
            check(checks, "every pulse is 0.40 ms wide", st.all_near(widths, 400, st.WIDTH_TOLERANCE_US), st.spread(widths))
        print(f"  mode {name}:", flush=True)
        out[name] = result(checks, {**extra, "ops": lines})
        ok = ok and out[name]["verdict"] == "PASS"
    out["modes_run"] = len(MODES)
    out["verdict"] = "PASS" if ok and len(MODES) == 19 else "FAIL"
    return out


RUNS = {"activity": ("20.2", run_activity), "hysteresis": ("20.2", run_hysteresis), "smoothing_down": ("20.2", run_smoothing_down), "smoothing_up": ("20.2", run_smoothing_up),
        "atr": ("20.2", run_atr), "storm": ("20.3", run_storm), "modes": ("21", run_modes)}


def main():
    record = sys.argv[1]
    chosen = [a for a in sys.argv[2:] if a in RUNS] or list(RUNS)
    os.makedirs(e1.SCRATCH, exist_ok=True)
    st.sh(["cargo", "build", "-q", "-p", "dcm-stub"])
    st.sh(["cargo", "build", "-q", "--release"], cwd="tools/pm-bridge")
    st.sh(["cargo", "build", "-q", "-p", "pacemaker", "--release", "--target", st.TARGET, "--no-default-features", "--features", "nrf52840,debug-port,quiet",
           "--target-dir", "target/pacemaker-link"])
    kept = json.load(open(record)) if os.path.exists(record) else {"runs": {}}
    kept["boards"] = {"A": st.BOARD_A, "B": st.BOARD_B, "C": st.BOARD_C}
    kept["binaries"] = {"pacemaker": st.sha(st.PG), "pm-bridge": st.sha(st.BRIDGE)}
    kept["capture"] = {"rate_hz": RATE, "pins": "D0 the atrial pace pin, D1 the ventricular, D6 and D7 the Heart_Simulator's sense mirrors",
                       "tolerances_us": {"PM-PAR-01": TOL, "PM-PAR-02": TOL_MSR, "width": st.WIDTH_TOLERANCE_US}}
    st.probe("download", st.BOARD_C, "--disable-progressbars", st.BRIDGE)
    st.probe("download", st.BOARD_A, "--disable-progressbars", st.PG)
    ok = True
    for name in chosen:
        task, run = RUNS[name]
        print(f"{name} (task {task})", flush=True)
        out = run()
        out["task"] = task
        kept["runs"][name] = out
        json.dump(kept, open(record, "w"), indent=1, default=str)
        print(f"{name}: {out['verdict']}", flush=True)
        ok = ok and out["verdict"] == "PASS"
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
