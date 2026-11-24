#!/usr/bin/env python3
"""Tasks 13.2 and 9.4 (the hour-long run) of docs/pacemaker/tasks.md: the
rig runs of stage E2, and a judge of the dual-chamber rules.

Usage: e2_runs.py scenarios <record.json> [--only <name>]...
       e2_runs.py hour <name> <record.json>

Run from the workspace root with rsk's tools on PATH. `scenarios` builds,
flashes, captures (30 s at 1 MHz) and judges each run of the list, as
`e1_runs.py` does, with the PG in its rig variant. `hour` runs one named
scenario for an hour on the Flight build (Operating_Duration 2 h; no
counts are read, since a probe's access restarts the Partition), captured
at 100 kHz.

The judge is written from the requirements, not from `pm-core`: it follows
the capture event by event (the Heart_Simulator's sense edges on D6 and
D7, the PG's pace edges on D0 and D1), keeps the last ventricular event,
the refractory, PVARP and blanking periods and the Paces that are due, and
requires every Pace on the pins to be one that was due, at its instant
within PM-PAR-01 (8 ms), and every due Pace to come:

- a ventricular Pace one escape interval after the last ventricular event
  (PM9.1, PM9.2), or at the end of the AV delay an atrial event started:
  the paced delay after an atrial Pace, the sensed one (less the offset)
  after a tracked atrial Sense (PM5.6, PM11.1 to PM11.5), dynamic where it
  is on, and never before one URL interval after the last ventricular
  event (PM10.1, PM10.2);
- an atrial Pace one paced AV delay before the end of the escape
  interval, unless a non-refractory atrial Sense came first (PM5.5);
- an atrial Sense in the PVARP (extended after a PVC, in one cycle of two:
  PM12.3 to PM12.5) or during an AV delay (PM11.6) changes nothing; a
  ventricular Sense in the VRP or in the blanking interval after an atrial
  Pace (PM4.5) changes nothing;
- in AAT and VVT, a Pace within PM-PAR-01 after each non-refractory Sense
  (PM5.4), with the delay measured.

A Sense within the tolerance of the end of a period, or within PM-PAR-15
before a due Pace, can be read either way: the judge then stops judging
until the next ventricular Pace, and counts such stretches.
"""
import json
import os
import sys
import time

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import e1_runs as e1  # noqa: E402

TOL = 8_000
EDGE = 8_000       # around the end of a refractory or blanking period
WINDOW = 4_000     # PM-PAR-15
NOMINAL = {"LRL": 60, "URL": 120, "FIXED_AV": 150, "DYNAMIC_AV": 0, "MIN_DYNAMIC_AV": 50, "SENSED_AV_OFFSET": 0, "VRP": 320, "ARP": 250,
           "PVARP": 250, "PVARP_EXTENSION": 0, "V_BLANKING": 40, "A_WIDTH": 400, "V_WIDTH": 400}
LETTERS = {"DDD": ("AV", "AV", "D"), "VDD": ("V", "AV", "D"), "DDI": ("AV", "AV", "I"), "DOO": ("AV", "", "O"), "AAT": ("A", "A", "T"), "VVT": ("V", "V", "T"),
           "VVI": ("V", "V", "I")}


def edges(data, bit, rate):
    d = np.diff(((data >> bit) & 1).astype(np.int8))
    us = 1e6 / rate
    rises = (np.flatnonzero(d == 1) + 1) * us
    falls = (np.flatnonzero(d == -1) + 1) * us
    return rises, falls


def widths(rises, falls):
    out = []
    j = 0
    for r in rises:
        while j < len(falls) and falls[j] <= r:
            j += 1
        if j < len(falls):
            out.append(falls[j] - r)
    return out


def judge(data, rate, mode, sets, skip_us=3_000_000):
    p = {**NOMINAL, **sets}
    paces, senses, response = LETTERS[mode]
    E = 60_000_000 // p["LRL"]
    URL = 60_000_000 // p["URL"]
    AV = p["FIXED_AV"] * 1000
    ap, ap_f = edges(data, 0, rate)
    vp, vp_f = edges(data, 1, rate)
    a_s, _ = edges(data, 6, rate)
    v_s, _ = edges(data, 7, rate)
    events = sorted([(t, "AP") for t in ap] + [(t, "VP") for t in vp] + [(t, "AS") for t in a_s] + [(t, "VS") for t in v_s])
    out = {"mode": mode, "paces": {"atrial": len(ap), "ventricular": len(vp)}, "senses_on_the_wire": {"atrial": len(a_s), "ventricular": len(v_s)},
           "violations": [], "judged_paces": 0, "unjudged_stretches": 0, "counts": {}}
    count = out["counts"]

    def bump(k):
        count[k] = count.get(k, 0) + 1

    errors = {}

    def err(kind, e):
        s = errors.setdefault(kind, [0, 1e18, -1e18])
        s[0] += 1
        s[1] = min(s[1], e)
        s[2] = max(s[2], e)

    def violation(t, text):
        if t >= skip_us and len(out["violations"]) < 20:
            out["violations"].append(f"{t / 1000:.3f} ms: {text}")

    # Pulse widths (PM3.3).
    for name, r, f, w in (("atrial", ap, ap_f, p["A_WIDTH"]), ("ventricular", vp, vp_f, p["V_WIDTH"])):
        ws = widths(r, f)
        if ws:
            out.setdefault("pulse_widths", {})[name] = {"count": len(ws), "min_us": min(ws), "max_us": max(ws)}
            if max(abs(x - w) for x in ws) > 200:
                violation(skip_us, f"a {name} pulse is not {w} µs within 200 µs")

    if response == "T":
        # AAT, VVT: a Pace after each non-refractory Sense; else at the LRL.
        s_edges, p_edges = (a_s, ap) if paces == "A" else (v_s, vp)
        period = (p["ARP"] if paces == "A" else p["VRP"]) * 1000
        ev = sorted([(t, "S") for t in s_edges] + [(t, "P") for t in p_edges])
        last = None
        waiting = None
        delays = []
        for t, k in ev:
            if waiting is not None and t - waiting > TOL and k != "P":
                violation(t, f"no Pace within PM-PAR-01 of the Sense at {waiting / 1000:.3f} ms")
                waiting = None
            if k == "S":
                if last is not None and abs(t - (last + period)) <= EDGE:
                    last = None  # either reading: resynchronize at the next Pace
                    waiting = None
                    out["unjudged_stretches"] += 1
                elif last is None or t >= last + period:
                    if waiting is None:
                        waiting = t
                        bump("senses_that_trigger")
                    last = t
                else:
                    bump("refractory_senses")
            else:
                if waiting is not None:
                    d = t - waiting
                    if t >= skip_us:
                        delays.append(d)
                        if d > TOL:
                            violation(t, f"a triggered Pace {d:.0f} µs after its Sense")
                    waiting = None
                elif last is not None and t >= skip_us:
                    e = t - (last + E)
                    err("lrl", e)
                    if abs(e) > TOL:
                        violation(t, f"a Pace {e:.0f} µs from the end of the escape interval")
                last = t
                out["judged_paces"] += 1
        if delays:
            out["triggered_delay_us"] = {"count": len(delays), "min": min(delays), "max": max(delays)}
        out["errors_us"] = {k: {"count": v[0], "min": v[1], "max": v[2]} for k, v in errors.items()}
        out["verdict"] = "PASS" if not out["violations"] and delays else "FAIL"
        return out

    tracks = response == "D"
    senses_a, senses_v, paces_a = "A" in senses, "V" in senses, "A" in paces
    st = {"synced": False, "last_v": None, "cycle": None, "since": False, "vdue": None, "vsrc": None, "adue": None, "pvarp": None, "vrp": None, "blank": None,
          "ext": False, "allow_vp": None, "allow_ap": None}

    def paced_av():
        if p["DYNAMIC_AV"] == 1 and tracks and st["cycle"] is not None:
            return min(AV, max(min(p["MIN_DYNAMIC_AV"] * 1000, AV), st["cycle"] * AV // E))
        return AV

    def ventricular(t, pvc):
        st["cycle"] = None if st["last_v"] is None else int(t - st["last_v"])
        st["last_v"] = t
        st["vrp"] = t + p["VRP"] * 1000
        ext = pvc and tracks and p["PVARP_EXTENSION"] != 0 and not st["ext"]
        st["ext"] = ext
        if ext:
            bump("extended_pvarps")
        st["pvarp"] = t + (p["PVARP"] + (p["PVARP_EXTENSION"] if ext else 0)) * 1000
        st["since"] = False
        st["vdue"], st["vsrc"] = t + E, "lrl"
        st["adue"] = t + E - paced_av() if paces_a else None
        st["blank"] = None
        st["allow_vp"] = st["allow_ap"] = None
        st["synced"] = True

    def atrial(t, paced):
        st["since"] = True
        st["adue"] = None
        if paced and senses_v:
            st["blank"] = t + p["V_BLANKING"] * 1000
        if paced or tracks:
            delay = paced_av() if paced else max(0, paced_av() - (p["SENSED_AV_OFFSET"] * 1000 if (tracks and paces_a) else 0))
            end, src = t + delay, "paced_av" if paced else "sensed_av"
            if st["last_v"] is not None and end < st["last_v"] + URL:
                end, src = st["last_v"] + URL, "url"
            if st["vdue"] is None or end < st["vdue"]:
                st["vdue"], st["vsrc"] = end, src

    def lose(t):
        if st["synced"]:
            out["unjudged_stretches"] += 1
        st["synced"] = False

    for t, k in events:
        if st["synced"]:
            for due, name in (("vdue", "ventricular"), ("adue", "atrial")):
                if st[due] is not None and t > st[due] + TOL and not (k[1] == "P" and abs(t - st[due]) <= TOL):
                    violation(t, f"the {name} Pace due at {st[due] / 1000:.3f} ms did not come")
                    st[due] = None
        if k == "VP":
            if st["synced"]:
                if st["vdue"] is not None and abs(t - st["vdue"]) <= TOL:
                    if t >= skip_us:
                        err(st["vsrc"], t - st["vdue"])
                        out["judged_paces"] += 1
                elif st["allow_vp"] is not None and t <= st["allow_vp"]:
                    bump("fusions")
                else:
                    violation(t, "a ventricular Pace that was not due" if st["vdue"] is None else f"a ventricular Pace {t - st['vdue']:.0f} µs from its instant ({st['vsrc']})")
            ventricular(t, False)
        elif k == "AP":
            if st["synced"]:
                if st["adue"] is not None and abs(t - st["adue"]) <= TOL:
                    if t >= skip_us:
                        err("atrial_escape", t - st["adue"])
                        out["judged_paces"] += 1
                elif st["allow_ap"] is not None and t <= st["allow_ap"]:
                    bump("fusions")
                else:
                    violation(t, "an atrial Pace that was not due" if st["adue"] is None else f"an atrial Pace {t - st['adue']:.0f} µs from its instant")
            atrial(t, True)
        elif k == "AS" and senses_a:
            if st["since"]:
                bump("atrial_senses_in_an_av_delay")
            elif st["pvarp"] is not None and abs(t - st["pvarp"]) <= EDGE:
                lose(t)
            elif st["pvarp"] is not None and t < st["pvarp"]:
                bump("atrial_senses_in_the_pvarp")
            else:
                if st["adue"] is not None and t >= st["adue"] - WINDOW:
                    st["allow_ap"] = st["adue"] + TOL
                bump("atrial_senses_effective")
                if tracks:
                    bump("tracked")
                atrial(t, False)
        elif k == "VS" and senses_v:
            if st["blank"] is not None and abs(t - st["blank"]) <= EDGE:
                lose(t)
            elif st["blank"] is not None and t < st["blank"]:
                bump("ventricular_senses_blanked")
            elif st["vrp"] is not None and abs(t - st["vrp"]) <= EDGE:
                lose(t)
            elif st["vrp"] is not None and t < st["vrp"]:
                bump("ventricular_senses_in_the_vrp")
            else:
                pvc = senses_a and st["last_v"] is not None and not st["since"]
                allow = st["vdue"] + TOL if st["vdue"] is not None and t >= st["vdue"] - WINDOW else None
                was_synced = st["synced"]
                bump("pvcs" if pvc else "ventricular_senses_effective")
                ventricular(t, pvc)
                st["synced"] = was_synced
                st["allow_vp"] = allow
    out["errors_us"] = {k: {"count": v[0], "min": v[1], "max": v[2]} for k, v in errors.items()}
    out["verdict"] = "PASS" if not out["violations"] and out["judged_paces"] > 0 else "FAIL"
    return out


def scenarios():
    quiet_atrium = [600, 900, 0, 0, 0]
    return [
        # Both Chambers paced, nothing to sense: the paced AV delay.
        {"name": "ddd_silent", "mode": "DDD", "set": {"LRL": 60}, "heart": "silent"},
        {"name": "doo_fixed", "mode": "DOO", "set": {"LRL": 90}, "heart": "fixed"},
        # An atrial rhythm of 900 ms and no conduction: tracked in DDD and
        # VDD (the sensed AV delay; with the offset), not in DDI.
        {"name": "ddd_tracking", "mode": "DDD", "set": {"LRL": 60}, "heart": quiet_atrium},
        {"name": "ddd_tracking_offset_30", "mode": "DDD", "set": {"LRL": 60, "SENSED_AV_OFFSET": 30}, "heart": quiet_atrium},
        {"name": "vdd_tracking", "mode": "VDD", "set": {"LRL": 60}, "heart": quiet_atrium},
        {"name": "ddi_same_scenario", "mode": "DDI", "set": {"LRL": 60}, "heart": quiet_atrium},
        # The dynamic AV delay at a cycle of 600 ms: 90 ms.
        {"name": "ddd_dynamic_av", "mode": "DDD", "set": {"LRL": 60, "DYNAMIC_AV": 1}, "heart": [600, 600, 0, 0, 0]},
        # Atrial rates through the URL (120 ppm, 500 ms): 110, 130 and 150 a minute.
        {"name": "ddd_atrial_545", "mode": "DDD", "set": {"LRL": 60}, "heart": [600, 545, 0, 0, 0]},
        {"name": "ddd_atrial_460", "mode": "DDD", "set": {"LRL": 60}, "heart": [600, 460, 0, 0, 0]},
        {"name": "ddd_atrial_400", "mode": "DDD", "set": {"LRL": 60}, "heart": [600, 400, 0, 0, 0]},
        # (At 400 ms the beats fall on the PVARP's end, to the tolerance, and
        # the judge leaves the run unjudged; 430 ms is beside it.)
        {"name": "ddd_atrial_430", "mode": "DDD", "set": {"LRL": 60}, "heart": [600, 430, 0, 0, 0]},
        # Ventricular beats alone, an atrial beat 300 ms after each: every
        # other one is a PVC with an extended PVARP (350 ms) that hides it.
        {"name": "ddd_pvc_extension", "mode": "DDD", "set": {"LRL": 60, "PVARP_EXTENSION": 100}, "heart": [500, 900, 200, 900, 0]},
        {"name": "ddd_pvc_no_extension", "mode": "DDD", "set": {"LRL": 60}, "heart": [500, 900, 200, 900, 0]},
        # The conducted rhythm: a tracked Sense, and the conducted beat in the VRP.
        {"name": "ddd_fixed", "mode": "DDD", "set": {"LRL": 60}, "heart": "fixed"},
        # Triggered Paces, with their delay.
        {"name": "aat_fixed", "mode": "AAT", "set": {"LRL": 60}, "heart": "fixed"},
        {"name": "vvt_fixed", "mode": "VVT", "set": {"LRL": 60}, "heart": "fixed"},
    ]


def run_scenario(r, flashed):
    sets = {"MODE": e1.MODES[r["mode"]], **r["set"]}
    pg, set_text = e1.build_pg(sets)
    heart, heart_text = e1.build_heart(r["heart"])
    if flashed.get("heart") != e1.sha(heart):
        e1.probe("download", e1.BOARD_B, "--disable-progressbars", heart)
        flashed["heart"] = e1.sha(heart)
    e1.probe("download", e1.BOARD_A, "--disable-progressbars", pg)
    e1.probe("reset", e1.BOARD_B)
    e1.probe("reset", e1.BOARD_A)
    started = time.time()
    capture = os.path.join(e1.SCRATCH, "capture.bin")
    e1.sh(["sigrok-cli", "-d", "fx2lafw", "--config", f"samplerate={e1.RATE}", "--samples", str(e1.RATE * e1.SECONDS), "-O", "binary", "-o", capture])
    time.sleep(max(0.0, e1.DURATION + 3 - (time.time() - started)))
    counts = e1.counts(pg)
    data = np.frombuffer(open(capture, "rb").read(), dtype=np.uint8)
    j = judge(data, e1.RATE, r["mode"], r["set"])
    return {"name": r["name"], "PM_SET": set_text, "heart": heart_text, "binaries": {"pacemaker": e1.sha(pg), "heart-sim": e1.sha(heart)},
            "capture": {"rate_hz": e1.RATE, "seconds": e1.SECONDS, "sha256": e1.sha(capture)}, "counts": counts, "judge": j}


def hour(name, record):
    """One scenario for an hour on the Flight build, at 100 kHz."""
    r = next(s for s in scenarios() + [{"name": "vvi_lrl_90_fixed", "mode": "VVI", "set": {"LRL": 90}, "heart": "fixed"}] if s["name"] == name)
    rate, seconds = 100_000, 3_600
    text = ",".join(f"{e1.WORD[k]}={v}" for k, v in {"MODE": e1.MODES[r["mode"]], **r["set"]}.items())
    e1.sh(["cargo", "build", "-q", "-p", "pacemaker", "--release", "--target", e1.TARGET, "--no-default-features", "--features",
           "nrf52840,debug-port,quiet,set-env", "--target-dir", "target/pacemaker-hour"], env={"PM_SET": text})
    pg = f"target/pacemaker-hour/{e1.TARGET}/release/pacemaker"
    heart, heart_text = e1.build_heart(r["heart"])
    e1.probe("download", e1.BOARD_B, "--disable-progressbars", heart)
    e1.probe("download", e1.BOARD_A, "--disable-progressbars", pg)
    e1.probe("reset", e1.BOARD_B)
    e1.probe("reset", e1.BOARD_A)
    time.sleep(2)
    capture = os.path.join(e1.SCRATCH, "hour.bin")
    e1.sh(["sigrok-cli", "-d", "fx2lafw", "--config", f"samplerate={rate}", "--samples", str(rate * seconds), "-O", "binary", "-o", capture])
    data = np.frombuffer(open(capture, "rb").read(), dtype=np.uint8)
    out = {"task": "9.4 / 13.2: the hour-long run (PAR-05)", "name": name, "PM_SET": text, "heart": heart_text,
           "binaries": {"pacemaker": e1.sha(pg), "heart-sim": e1.sha(heart)}, "capture": {"rate_hz": rate, "seconds": len(data) / rate, "sha256": e1.sha(capture)},
           "judge": judge(data, rate, r["mode"], r["set"])}
    if r["mode"] == "VVI":
        # The Rust judge of stage E1 as well, on the same capture.
        rep = os.path.join(e1.SCRATCH, "hour_judge.json")
        e1.sh(["cargo", "build", "-q", "-p", "pm-judge"])
        cmd = ["target/debug/pm-judge", capture, str(rate), "--mode", "VVI", "--scenario", "fixed", "--json", rep]
        for k, v in r["set"].items():
            cmd += ["--set", f"{k}={v}"]
        e1.sh(cmd, check=False)
        out["pm_judge"] = json.load(open(rep)) if os.path.exists(rep) else {"verdict": "NO REPORT"}
    json.dump(out, open(record, "w"), indent=1)
    j = out["judge"]
    print(name, j["verdict"], "paces", j["paces"], "judged", j["judged_paces"], "errors", j["errors_us"], "counts", j["counts"], j["violations"][:3],
          out.get("pm_judge", {}).get("verdict"), flush=True)
    os.remove(capture)
    sys.exit(0 if j["verdict"] == "PASS" and out.get("pm_judge", {"verdict": "PASS"})["verdict"] == "PASS" else 1)


def main():
    os.makedirs(e1.SCRATCH, exist_ok=True)
    if sys.argv[1] == "hour":
        return hour(sys.argv[2], sys.argv[3])
    record = sys.argv[2]
    only = [sys.argv[i + 1] for i, a in enumerate(sys.argv) if a == "--only"]
    rs = [r for r in scenarios() if not only or r["name"] in only]
    kept = json.load(open(record)) if os.path.exists(record) else {"task": "13.2", "runs": []}
    flashed = {}
    ok = True
    for r in rs:
        out = run_scenario(r, flashed)
        kept["runs"] = [k for k in kept["runs"] if k["name"] != out["name"]] + [out]
        json.dump(kept, open(record, "w"), indent=1)
        j = out["judge"]
        good = j["verdict"] == "PASS" and out["counts"] is not None
        ok = ok and good
        print(f"{'ok  ' if good else 'FAIL'} {r['name']:24} paces {j['paces']} judged {j['judged_paces']} unjudged {j['unjudged_stretches']} errors {j['errors_us']} "
              f"{j.get('triggered_delay_us', '')} {j['counts']} pg {out['counts']} {j['violations'][:3]}", flush=True)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
