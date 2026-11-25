#!/usr/bin/env python3
"""Tasks 22.1 to 22.3 of docs/pacemaker/tasks.md: the rig runs of the event
records, the Markers and what `diag` keeps.

Usage: e5_runs.py <record.json> [events] [markers] [atr_markers]

Run from the workspace root with rsk's tools on PATH. What `diag` keeps is
read through the debug probe in one access: the Task `history`'s state
(`pacemaker_diag::history::History`, every field a word, in the order
written). Until the Endpoints `records` and `request` exist (task 22.4)
this is the only way out of `diag`; the Marker stream over the link is
that task's.

  events       (22.1) the highest event rate the rig gives: DDD with a
               storm of edges on the atrial line and then on both, in the
               rig variant under an attached probe. After the safe state:
               the records `diag` took against the records the
               Therapy_Partition wrote, the records its ring lost, the
               batches that did not arrive, and the Kernel's log.
  markers      (22.2, 22.3) DDD at LRL 60 against the Heart_Simulator's
               `fixed` rhythm (75 a minute, conducted), in the rig variant.
               Every record of the Marker buffer against the capture: a
               Pace against its pulse, a Sense against the sense mirror's
               edge, within PM-PAR-04 (1 ms) after one fit of the two
               clocks; its interval against the capture's. The histograms
               against the events counted on the capture; the trend.
  atr_markers  (22.2) the atrial tachycardia of `e4_runs.py atr` on the
               Flight build over the link, then the store read through the
               probe: the Markers AT, the augmentation Markers of ATR, and
               the count of ATR episodes.

Exit 0 when every run chosen passed.
"""
import json
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import e1_runs as e1  # noqa: E402
import e3_faults as ef  # noqa: E402
import e3_states as st  # noqa: E402
import e4_runs as e4  # noqa: E402

RATE = st.RATE
MARKER_TOLERANCE_US = 1_000   # PM-PAR-04
BINS, MARKERS, TREND = 17, 64, 128
KIND = {0: "none", 1: "pace", 2: "pace_fused", 3: "sense", 4: "sense_refractory", 5: "at", 6: "pvc", 7: "noise"}
AUG = {0: None, 1: "ATR-Dur", 2: "ATR-FB", 3: "ATR-End", 4: "PVP"}
NO_INTERVAL = 0xFFFF_FFFF


def store(elf):
    """`diag`'s store, read in one access and taken apart by the order of
    `History`'s fields."""
    address = size = None
    for line in st.sh([e1.NM, "-S", "-C", elf]).stdout.splitlines():
        if "pacemaker_diag::rsk_state::HISTORY_STORE" in line:
            f = line.split()
            address, size = int(f[0], 16), int(f[1], 16)
    if address is None:
        return None
    words = []
    for line in st.probe("read", st.BOARD_A, "b32", f"0x{address:08x}", str(size // 4)).stdout.splitlines():
        if ":" in line:
            words += [int(x, 16) for x in line.split(":", 1)[1].split()]
    if len(words) != 10 + 4 * BINS + 1 + 3 * MARKERS + 4 + 2 * TREND + 3:
        return None
    it = iter(words)
    take = lambda n: [next(it) for _ in range(n)]
    h = dict(zip(["batches", "records", "lost", "dropped", "next_seq", "seq_known", "peer_changes", "pvcs", "atr_episodes", "resets"], take(10)))
    h["histogram"] = {name: take(BINS) for name in ("paced_atrial", "sensed_atrial", "paced_ventricular", "sensed_ventricular")}
    h["markers_written"] = take(1)[0]
    raw = [take(3) for _ in range(MARKERS)]
    h.update(zip(["trend_duration", "trend_what", "trend_written", "trend_stamp_ms"], take(4)))
    trend = [take(2) for _ in range(TREND)]
    h.update(zip(["pace_interval", "sensor_interval", "jobs"], take(3)))
    # The buffer in the order written: the last MARKERS records.
    n = h["markers_written"]
    order = [raw[k % MARKERS] for k in range(max(0, n - MARKERS), n)]
    h["markers"] = [{"at_us": r[0], "kind": KIND.get(r[1] & 0xFF, str(r[1] & 0xFF)), "chamber": (r[1] >> 8) & 0xFF, "modifiers": (r[1] >> 16) & 0xFF,
                     "augmentation": AUG.get(r[1] >> 24, str(r[1] >> 24)), "since_us": None if r[2] == NO_INTERVAL else r[2]} for r in order]
    w = h["trend_written"]
    h["trend"] = [trend[k % TREND] for k in range(max(0, w - TREND), w)] if h["trend_duration"] == 0 else trend[:w]
    return h


def bin_of(interval_us):
    ppm = 60_000_000 // interval_us
    return 0 if ppm < 30 else BINS - 1 if ppm >= 180 else (ppm - 30) // 10 + 1


def run_events():
    text = f"0={e4.CODE['DDD']},1=60"
    pg = ef.build("nrf52840,debug-port,quiet,rig,set-env,trace-ram", "target/pacemaker-storm", {"PM_SET": text})
    elf, heart_text = e4.flash_heart([0, 0, 0, 0, 0, 1, 25000, 33000, 1, 3, 33000, 41000, 1])
    cap, _, ev = ef.run_attached(pg, 58)
    counts = e1.counts(pg)
    h = store(pg)
    checks = []
    evs = ef.own_events(ev, ef.partitions())
    e4.check(checks, "the store and the driver's counts were read", h is not None and counts is not None, {"counts": counts})
    if h is None or counts is None:
        return e4.result(checks, {"PM_SET": text, "heart": heart_text, "kernel_events": evs})
    written = counts["records"]
    sa, sv = e4.storm_edges(cap, 6), e4.storm_edges(cap, 7)
    e4.check(checks, "the storm is on the wire (16,000 atrial and 8,000 ventricular edges)", sa is not None and sv is not None and sa[2] >= 15_500 and sv[2] >= 7_500,
             {"atrial_edges": sa and sa[2], "ventricular_edges": sv and sv[2]})
    e4.check(checks, "every record the Therapy_Partition wrote reached `diag`: none lost by the ring, no batch missing",
             h["records"] == written and h["dropped"] == 0 and counts["dropped"] == 0 and h["lost"] == 0 and h["peer_changes"] == 0 and h["next_seq"] == written,
             {"written_by_therapy": written, "taken_by_diag": h["records"], "batches": h["batches"], "dropped_by_the_ring": counts["dropped"], "dropped_as_told_to_diag": h["dropped"],
              "lost_batches_records": h["lost"], "peer_changes": h["peer_changes"]})
    noise = [m for m in h["markers"] if m["kind"] == "noise"]
    rate = written / 45.0
    e4.check(checks, "the run is the high event rate meant: noise Markers of both Chambers among the last records, and more than twice the records of a quiet run (2 a second)",
             len(noise) >= 10 and {m["chamber"] for m in noise} == {0, 1} and rate > 4.0, {"records_a_second_over_the_run": rate, "noise_markers_among_the_last_64": len(noise)})
    e4.check(checks, "the Kernel's log holds the end of the Operating_Duration and the safe state and nothing else", [e["event"] for e in evs] == ["operating_end", "safe_state"], evs)
    return e4.result(checks, {"PM_SET": text, "heart": heart_text, "binaries": {"pacemaker": st.sha(pg), "heart-sim": st.sha(elf)}, "kernel_events": evs, "counts": counts,
                              "store": {k: h[k] for k in ("batches", "records", "lost", "dropped", "next_seq", "peer_changes", "markers_written", "histogram")}})


def fit(records, edges):
    """The offset and the rate between the therapy's microseconds and the
    capture's: each record's instant against the nearest edge of its kind
    under the offset the last record gives, then one least-squares line."""
    pairs = []
    if not records or not len(edges):
        return None, pairs
    for back in range(1, min(6, len(edges)) + 1):
        offset = edges[-back] - records[-1]
        pairs = []
        for t in records:
            k = int(np.argmin(np.abs(edges - (t + offset))))
            if abs(edges[k] - (t + offset)) < 20_000:
                pairs.append((t, int(edges[k])))
        if len(pairs) >= len(records) - 1:
            break
    if len(pairs) < 3:
        return None, pairs
    x, y = np.array([p[0] for p in pairs], dtype=np.float64), np.array([p[1] for p in pairs], dtype=np.float64)
    slope, intercept = np.polyfit(x, y, 1)
    return (float(slope), float(intercept)), pairs


def judge_markers(cap, h, checks, label=""):
    """Every record of the Marker buffer against the capture."""
    ms = h["markers"]
    # The counter is 32 bits: unwrap (a run is far shorter than 71 minutes, but a restart begins it again).
    edges = {("pace", 0): np.array([p[1] for p in cap.pulses(st.ATRIAL)]), ("pace", 1): np.array([p[1] for p in cap.pulses(st.VENTRICULAR)]),
             ("sense", 0): e4.rises(cap, 6), ("sense", 1): e4.rises(cap, 7)}
    group = lambda m: ("pace" if m["kind"] in ("pace", "pace_fused") else "sense", m["chamber"])
    vp = [m["at_us"] for m in ms if group(m) == ("pace", 1)]
    line, pairs = fit(vp, edges[("pace", 1)])
    e4.check(checks, f"{label}the therapy's clock and the capture's are fitted on the ventricular Paces of the Marker buffer", line is not None and len(pairs) >= max(3, len(vp) - 1),
             {"ventricular_pace_records": len(vp), "matched": len(pairs), "rate_difference_ppm": (line[0] - 1) * 1e6 if line else None})
    if line is None:
        return
    errors, worst, missing, since_errors = {}, 0.0, [], []
    last = {}
    for m in ms:
        g = group(m)
        want = line[0] * m["at_us"] + line[1]
        e = edges[g]
        k = int(np.argmin(np.abs(e - want))) if len(e) else None
        d = float(e[k] - want) if k is not None else None
        name = f"{'atrial' if g[1] == 0 else 'ventricular'}_{g[0]}"
        if d is None or abs(d) > MARKER_TOLERANCE_US:
            missing.append({"record": m, "nearest_edge_us_away": d})
            continue
        s = errors.setdefault(name, {"count": 0, "min": d, "max": d})
        s["count"] += 1
        s["min"], s["max"] = min(s["min"], d), max(s["max"], d)
        worst = max(worst, abs(d))
        # PM18.6: the time since the Chamber's last non-refractory event,
        # against the capture's own edges of the records before.
        if m["since_us"] is not None and g[1] in last:
            since_errors.append(m["since_us"] - (int(e[k]) - last[g[1]]))
        if m["kind"] in ("pace", "pace_fused", "sense", "at", "pvc"):
            last[g[1]] = int(e[k])
    e4.check(checks, f"{label}every record of the Marker buffer has its edge on the capture within PM-PAR-04 (1 ms): a Pace its pulse, a Sense the sense mirror's edge",
             len(ms) >= 20 and not missing, {"records": len(ms), "error_us": errors, "furthest_us": worst, "without_an_edge": missing[:5]})
    e4.check(checks, f"{label}each record's time since the Chamber's last non-refractory event is the capture's within PM-PAR-04", len(since_errors) >= 10 and max(abs(x) for x in since_errors) <= MARKER_TOLERANCE_US,
             {"compared": len(since_errors), "error_us": st.spread(since_errors)})


def run_markers():
    pg, text = e1.build_pg({"MODE": e4.CODE["DDD"], "LRL": 60})
    elf, heart_text = e4.flash_heart("fixed")
    cap, _, ev = ef.run_attached(pg, 58)
    h = store(pg)
    checks = []
    e4.check(checks, "the store was read", h is not None, None)
    if h is None:
        return e4.result(checks, {"PM_SET": text, "heart": heart_text})
    judge_markers(cap, h, checks)
    kinds = {}
    for m in h["markers"]:
        kinds[m["kind"] + ("_a" if m["chamber"] == 0 else "_v")] = kinds.get(m["kind"] + ("_a" if m["chamber"] == 0 else "_v"), 0) + 1
    e4.check(checks, "the Marker buffer holds what the scenario gives: tracked atrial Senses, ventricular Paces, and the conducted beat as a Refractory sense",
             kinds.get("sense_a", 0) >= 15 and kinds.get("pace_v", 0) >= 15 and kinds.get("sense_refractory_v", 0) >= 15, kinds)
    # The histograms against the capture: the atrial rhythm is 800 ms
    # (75 a minute), tracked, so the sensed atrial and the paced
    # ventricular events are in the bin of 70 to 79; the conducted beat is
    # refractory and in no histogram.
    v = ef.after_boot(cap.pulses(st.VENTRICULAR))
    a = ef.after_boot(cap.pulses(st.ATRIAL))
    hist = h["histogram"]
    b75 = bin_of(800_000)
    total = {k: sum(x) for k, x in hist.items()}
    in_bin = {k: x[b75] for k, x in hist.items()}
    paced_v_on_wire = len(v)
    e4.check(checks, "the paced ventricular histogram has the capture's ventricular Paces (one less: the first has no interval), nearly all at 70 to 79 a minute",
             abs(total["paced_ventricular"] - (paced_v_on_wire - 1)) <= 1 and in_bin["paced_ventricular"] >= total["paced_ventricular"] - 3,
             {"on_the_capture": paced_v_on_wire, "in_the_histogram": total["paced_ventricular"], "at_70_to_79": in_bin["paced_ventricular"], "bins": hist["paced_ventricular"]})
    e4.check(checks, "the sensed atrial histogram has the tracked beats, nearly all at 70 to 79 a minute; the sensed ventricular one is empty; the paced atrial one has the atrial Paces",
             abs(total["sensed_atrial"] - total["paced_ventricular"]) <= 3 and in_bin["sensed_atrial"] >= total["sensed_atrial"] - 3 and total["sensed_ventricular"] == 0
             and abs(total["paced_atrial"] - len(a)) <= 1,
             {"sensed_atrial": hist["sensed_atrial"], "sensed_ventricular_total": total["sensed_ventricular"], "paced_atrial_total": total["paced_atrial"], "atrial_paces_on_the_capture": len(a)})
    e4.check(checks, "no record was lost and the counts of PVCs and ATR episodes are zero", h["dropped"] == 0 and h["lost"] == 0 and h["pvcs"] == 0 and h["atr_episodes"] == 0,
             {k: h[k] for k in ("records", "batches", "dropped", "lost", "pvcs", "atr_episodes")})
    # The trend: Continuous, a sample a second, the pacing interval 800 ms.
    trend = h["trend"]
    paced = [t[0] for t in trend[3:]]
    e4.check(checks, "the trend is Continuous with a sample a second: the pacing interval 800 ms, no sensor interval (the Mode is not rate-adaptive)",
             h["trend_duration"] == 0 and 40 <= h["trend_written"] <= 46 and bool(paced) and all(abs(x - 800_000) <= 1_000 for x in paced) and all(t[1] == 0 for t in trend),
             {"samples": h["trend_written"], "pacing_interval_us": st.spread(paced), "stamp_ms": h["trend_stamp_ms"]})
    return e4.result(checks, {"PM_SET": text, "heart": heart_text, "binaries": {"pacemaker": st.sha(pg), "heart-sim": st.sha(elf)}, "kernel_events": ef.own_events(ev, ef.partitions()),
                              "store": {k: v for k, v in h.items() if k != "trend"}, "trend": trend})


def run_atr_markers():
    # The link build again (the runs before leave a rig build on board A).
    st.probe("download", st.BOARD_A, "--disable-progressbars", st.PG)
    ops = ["up", f"set:0={e4.CODE['DDD']},1=60,20=1,21=10,22=1", f"wait:state={e4.PERMANENT}"] + ["hold:2", "status"] * 24
    cap, lines, extra = e4.session([600, 900, 0, 0, 0, 1, 9000, 31000, 300], ops, 56)
    h = store(st.PG)
    checks = []
    e4.check(checks, "every operation of the stub succeeded and the store was read", all(l["ok"] for l in lines) and h is not None, [l["op"] for l in lines if not l["ok"]])
    if h is None:
        return e4.result(checks, {**extra, "ops": lines})
    judge_markers(cap, h, checks)
    told = [m for _, m in [(l["done"], l["detail"]["mode"]) for l in lines if l["op"] == "status" and l["ok"]]]
    e4.check(checks, "the run went through Fallback and back (the Mode the status reports)", 106 in told and told[-1] == 63, told)
    augs = [m["augmentation"] for m in h["markers"] if m["augmentation"]]
    kinds = {}
    for m in h["markers"]:
        kinds[m["kind"]] = kinds.get(m["kind"], 0) + 1
    e4.check(checks, "one ATR episode is counted, and ATR-End is among the last records with the tracking after it",
             h["atr_episodes"] == 1 and "ATR-End" in augs and kinds.get("sense", 0) >= 5, {"atr_episodes": h["atr_episodes"], "augmentation_markers_among_the_last_64": augs, "kinds": kinds})
    # In the tachycardia (a beat every 300 ms) every second beat falls in
    # the PVARP: the non-refractory ones are 600 ms apart, 100 a minute.
    hist = h["histogram"]["sensed_atrial"]
    e4.check(checks, "the sensed atrial histogram has the rhythm's beats at 60 to 69 a minute and the tachycardia's non-refractory beats at 100 to 109 a minute", hist[bin_of(900_000)] >= 15 and hist[bin_of(600_000)] >= 8,
             {"sensed_atrial": hist})
    e4.check(checks, "no record was lost", h["dropped"] == 0 and h["lost"] == 0, {k: h[k] for k in ("records", "batches", "dropped", "lost", "peer_changes")})
    return e4.result(checks, {**extra, "ops": lines, "store": {k: v for k, v in h.items() if k != "trend"}})


RUNS = {"events": ("22.1", run_events), "markers": ("22.2, 22.3", run_markers), "atr_markers": ("22.2", run_atr_markers)}


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
    kept["capture"] = {"rate_hz": RATE, "pins": "D0 the atrial pace pin, D1 the ventricular, D6 and D7 the Heart_Simulator's sense mirrors", "marker_tolerance_us": MARKER_TOLERANCE_US}
    st.probe("download", st.BOARD_C, "--disable-progressbars", st.BRIDGE)
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
