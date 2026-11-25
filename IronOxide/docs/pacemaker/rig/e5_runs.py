#!/usr/bin/env python3
"""Tasks 22.1 to 22.3 of docs/pacemaker/tasks.md: the rig runs of the event
records, the Markers and what `diag` keeps.

Usage: e5_runs.py <record.json> [events] [markers] [atr_markers] [records] [malformed]
                  [stream] [interrogation] [threshold]

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

  records      (22.4) DDD against the `fixed` rhythm on the Flight build.
               Every kind of record read over the link, part by part: the
               four histograms, the counts, the trend, the implant data and
               the measured data; then the store read through the probe,
               and the two compared. A reset of the histograms and a trend
               begun, by their commands, and read back.
  malformed    (22.4) the rig variant in which `history` sends records
               that `comms`'s validation function refuses, from 10 s on:
               the Kernel's first event is the rejected message, against
               `diag`; VVI pacing goes on as judged by `pm-judge`.
  stream       (22.2) the Marker transmission over the link for 20 s of
               DDD against the `fixed` rhythm: every record that came
               against the capture's edges, within PM-PAR-04; the records'
               numbers without a gap.
  interrogation (22.5) the implant data and the measured data's stand-ins
               supplied over the link and read back, with the mark that
               the measured data is simulated; the battery level by the
               status.
  threshold    (22.6) a ventricular test in VVI and an atrial one in AAI,
               of the pulse width from 1.0 ms: the widths on the capture
               step down one setting every fourth pulse; the ventricular
               test ended by Stop, the atrial one by the loss of the link.
  ert          (finding A9, PM17.7) DDD against the `fixed` rhythm on the
               Flight build: the measured data read and the Marker
               transmission run at BOL; at ERT and at ERP the read of the
               measured data, the read of a Marker and the start of the
               transmission are answered as declined and no Marker comes,
               while a histogram, the counts and the implant data are
               read; at BOL again both are supplied.

Exit 0 when every run chosen passed.
"""
import json
import os
import sys
import time

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
    if len(words) != 10 + 4 * BINS + 1 + 3 * MARKERS + 4 + 2 * TREND + 3 + 4 + 10 + 5 + 1:
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
    h.update(zip(["stream", "stream_next", "stream_skipped", "unknown_requests"], take(4)))
    h["implant"] = take(10)
    h["measured"] = take(5)
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
    beyond = 0
    for m in ms:
        g = group(m)
        want = line[0] * m["at_us"] + line[1]
        if want > len(cap.data) - 2_000:
            # The capture had ended: nothing to compare the record with.
            beyond += 1
            continue
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
             len(ms) - beyond >= 20 and not missing, {"records": len(ms), "after_the_captures_end": beyond, "error_us": errors, "furthest_us": worst, "without_an_edge": missing[:5]})
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


HISTOGRAMS = ["paced_atrial", "sensed_atrial", "paced_ventricular", "sensed_ventricular"]


def reads(lines, kind):
    """The words of a record of `kind` read by the stub, its parts in order."""
    out = []
    for l in lines:
        if l["op"].startswith(f"read:{kind},") and l["ok"]:
            out += l["detail"]["words"]
    return out


def link_build():
    st.probe("download", st.BOARD_A, "--disable-progressbars", st.PG)


def run_records():
    link_build()
    ops = ["up", f"set:0={e4.CODE['DDD']},1=60", f"wait:state={e4.PERMANENT}", "hold:14"]
    for k in range(4):
        ops += [f"read:{0x10 + k},{p}" for p in range(4)]
    ops += ["read:32,0", "read:32,1"] + [f"read:48,{p}" for p in range(0, 9)]
    cap, lines, extra = e4.session("fixed", ops, 30)
    h = store(st.PG)
    checks = []
    failed = [l["op"] for l in lines if not l["ok"]]
    e4.check(checks, "every record was read over the link (16 parts of histograms, 2 of counts, 9 of the trend) and the store through the probe", not failed and h is not None, failed)
    if h is None or failed:
        return e4.result(checks, {**extra, "ops": lines})
    # The store is read after the records, the first of which were read
    # 27 reads and the capture's end before it: a bin may have grown by the
    # events of that time, at most one a cycle of 800 ms and two more.
    read_at = min(l["done"] for l in lines if l["op"].startswith("read:"))
    grown = int((time.time() - read_at) / 0.8) + 2
    for k, name in enumerate(HISTOGRAMS):
        got = reads(lines, 0x10 + k)[:BINS]
        e4.check(checks, f"the {name.replace('_', ' ')} histogram over the link is the store's (the store read later: no bin less, none more than the events since above)",
                 all(0 <= h["histogram"][name][b] - got[b] <= grown for b in range(BINS)) and reads(lines, 0x10 + k)[BINS:] == [0, 0, 0],
                 {"over_the_link": got, "store": h["histogram"][name], "events_since_at_most": grown})
    c = reads(lines, 32)
    e4.check(checks, "the counts over the link are the store's (PVCs, ATR episodes, records, lost; resets, batches)",
             c[0] == h["pvcs"] and c[1] == h["atr_episodes"] and 0 <= h["records"] - c[2] <= 3 * grown and c[3] == h["dropped"] == 0 and c[4] == h["lost"] == 0 and c[5] == h["resets"] == 0,
             {"over_the_link": c, "store": {k: h[k] for k in ("pvcs", "atr_episodes", "records", "dropped", "lost", "resets", "batches")}})
    t = reads(lines, 48)
    head, samples = t[:5], [t[5 + 5 * p: 5 + 5 * p + 4] for p in range(8)]
    flat = [s[0:2] for s in samples] + [s[2:4] for s in samples]
    paced = [x[0] for x in flat if x[0]]
    e4.check(checks, "the trend over the link: Continuous, both contents, the samples the pacing interval of the rhythm (800 ms)",
             head[0] == 0 and head[1] == 0 and head[4] == TREND and 10 <= head[2] <= h["trend_written"] and len(paced) >= 8 and all(abs(x - 800_000) <= 1_000 for x in paced[2:]),
             {"header": head, "pacing_intervals_us": paced})
    # The commands that change the store: a reset and a Fixed trend of the pacing rate.
    more = st.stub("up", "reset-histograms", "trend:1,1", "hold:4", "read:32,0", "read:32,1", "read:18,1", "read:48,0", "read:48,1")
    c2, hist2, t2 = reads(more, 32), reads(more, 18), reads(more, 48)
    e4.check(checks, "after RESET_HISTOGRAMS the counts say one reset and the paced ventricular histogram has only the events since (5 at most)",
             all(l["ok"] for l in more) and c2[5] == 1 and sum(hist2) <= 5, {"counts": c2, "paced_ventricular_part_1": hist2})
    e4.check(checks, "after TREND_START the trend is Fixed, of the pacing rate alone, with the samples since (2 to 5) and no sensor value",
             len(t2) == 10 and t2[0] == 1 and t2[1] == 1 and 2 <= t2[2] <= 5 and t2[5] != 0 and t2[6] == 0, {"header": t2[:5], "first_samples": t2[5:]})
    return e4.result(checks, {**extra, "ops": lines + more, "store": {k: v for k, v in h.items() if k not in ("trend", "markers")}})


def run_malformed():
    h = ef.heart("fixed")
    st.sh(["cargo", "build", "-q", "-p", "pm-judge"])
    pg = ef.build("nrf52840,debug-port,quiet,rig,set-env,misbehave,trace-ram", "target/pacemaker-misbehave", {"PM_SET": f"0={ef.VVI},1=90", "PM_LINK_FAULT": "record"})
    cap, launched, ev = ef.run_attached(pg, 30, capture_after=14.0)
    capture, report = os.path.join(ef.st_scratch(), "capture.bin"), os.path.join(ef.st_scratch(), "judge.json")
    cap.data.tofile(capture)
    if os.path.exists(report):
        os.remove(report)
    judged = st.sh(["target/debug/pm-judge", capture, str(st.RATE), "--mode", "VVI", "--set", "LRL=90", "--scenario", "fixed", "--binary", f"pacemaker={pg}", "--binary", f"heart-sim={h}",
                    "--json", report], check=False)
    j = json.load(open(report)) if os.path.exists(report) else {"verdict": "NO REPORT", "error": judged.stderr[-500:]}
    counts = e1.counts(pg)
    evs = ef.own_events(ev, ef.partitions())
    checks = []
    e4.check(checks, "the Kernel's first event is the record `comms`'s validation function refused, reported against `diag`, at 10 s of the run",
             bool(evs) and evs[0]["event"] == "message_rejected" and evs[0]["partition"] == "diag" and 9.9 <= evs[0]["instant_s"] <= 10.3, evs[:3])
    e4.check(checks, "pacing goes on as programmed across it (`pm-judge`, VVI at 90 ppm, 30 s from before the onset), and `therapy` did not restart",
             j.get("verdict") == "PASS" and counts is not None and counts["started"] == 1 and counts["hw_fault"] == 0 and counts["records"] > 60,
             {"judge": j.get("verdict"), "violations": j.get("violations", [])[:3], "ventricular": j.get("ventricular", {}).get("pace_intervals"), "counts": counts})
    link_build()
    return e4.result(checks, {"binary": st.sha(pg), "judge": j, "kernel_events": evs, "therapy_counts_at_the_end": counts})


def run_stream():
    link_build()
    ops = ["up", f"set:0={e4.CODE['DDD']},1=60", f"wait:state={e4.PERMANENT}", "hold:3", "markers:20", "status"]
    cap, lines, extra = e4.session("fixed", ops, 32)
    checks = []
    m = e4.op(lines, "markers")
    e4.check(checks, "every operation of the stub succeeded", all(l["ok"] for l in lines) and m is not None, [l["op"] for l in lines if not l["ok"]])
    if m is None or not m["ok"]:
        return e4.result(checks, {**extra, "ops": lines})
    recs = m["detail"]["records"]
    numbers = [r["number"] for r in recs]
    e4.check(checks, "the records came in order without a gap, at the rate of the scenario (3 an 800 ms cycle)",
             len(recs) >= 60 and all(b == a + 1 for a, b in zip(numbers, numbers[1:])), {"records": len(recs), "first_number": numbers[:1], "last_number": numbers[-1:]})
    as_store = {"markers": [{"at_us": r["words"][0], "kind": KIND.get(r["words"][1] & 0xFF, "?"), "chamber": (r["words"][1] >> 8) & 0xFF, "modifiers": (r["words"][1] >> 16) & 0xFF,
                             "augmentation": AUG.get(r["words"][1] >> 24, "?"), "since_us": None if r["words"][2] == NO_INTERVAL else r["words"][2]} for r in recs]}
    judge_markers(cap, as_store, checks, "over the link: ")
    kinds = {}
    for x in as_store["markers"]:
        kinds[x["kind"] + ("_a" if x["chamber"] == 0 else "_v")] = kinds.get(x["kind"] + ("_a" if x["chamber"] == 0 else "_v"), 0) + 1
    e4.check(checks, "the stream holds what the scenario gives: tracked atrial Senses, ventricular Paces, the conducted beat as a Refractory sense",
             min(kinds.get("sense_a", 0), kinds.get("pace_v", 0), kinds.get("sense_refractory_v", 0)) >= 18, kinds)
    return e4.result(checks, {**extra, "ops": [l for l in lines if l["op"] != "markers:20"], "stream": {"records": len(recs), "kinds": kinds}, "records_as_they_came": recs[:400]})


def run_interrogation():
    link_build()
    st.probe("reset", st.BOARD_C)
    st.probe("reset", st.BOARD_A)
    time.sleep(2.5)
    implant = [9_461, 9_461, 3, 12, 15, 21, 87, 510, 620, 4]       # dates as days since 2000-01-01 (2025-11-26); bipolar leads
    measured = [2_780, 23, 91, 505 + (615 << 16), 9_460]
    ops = ["up"] + [f"data:0,{k},{v}" for k, v in enumerate(implant)] + [f"data:1,{k},{v}" for k, v in enumerate(measured)]
    ops += ["hold:1", "read:80,0", "read:80,1", "read:96,0", "read:96,1", "battery:1", "status", "battery:0", "data:0,10,1", "data:2,0,1", "read:153,0"]
    lines = st.stub(*ops)
    checks = []
    wrong = [l["op"] for l in lines if not l["ok"]]
    e4.check(checks, "every value was supplied and every record read; a value outside the data and a record that is none are answered as bad parts",
             wrong == ["data:0,10,1", "data:2,0,1", "read:153,0"] and all(l["detail"].get("code") == 5 if isinstance(l["detail"], dict) and "code" in l["detail"] else l["detail"].get("answer", {}).get("code") == 5
                                                                       for l in lines if l["op"] in wrong), wrong)
    e4.check(checks, "the implant data read back is what was supplied (PM18.7): the dates, the lead polarity, the thresholds, the wave amplitudes, the impedances, the indications",
             reads(lines, 80) == implant, {"supplied": implant, "read": reads(lines, 80)})
    m = reads(lines, 96)
    e4.check(checks, "the measured data read back is the stand-ins supplied, marked as simulated (PM17.5): the battery voltage, the P and R wave, the lead impedances, the date of the last interrogation",
             m[0] == 1 and m[1:5] == measured[:4] and m[5] == measured[4] and m[6:] == [0, 0, 0, 0], {"supplied": measured, "read": m})
    told = e4.op(lines, "status")
    e4.check(checks, "the battery status level is told by the status (PM17.3, PM17.6): ERN after it was set", told is not None and told["ok"] and told["detail"]["battery"] == 1, told and told["detail"])
    return e4.result(checks, {"ops": lines, "binaries": {"pacemaker": st.sha(st.PG)}})


WIDTHS = [1900 - 100 * k for k in range(19)] + [50]


def groups(pulses):
    """Runs of pulses of one width (within 20 µs of a setting)."""
    out = []
    for p in pulses:
        w = min(WIDTHS + [400], key=lambda s: abs(s - p[2]))
        if abs(w - p[2]) > st.WIDTH_TOLERANCE_US:
            w = p[2]
        if out and out[-1][0] == w:
            out[-1][1] += 1
        else:
            out.append([w, 1])
    return out


def run_threshold():
    link_build()
    out, ok = {}, True
    for name, mode, pin, word, which, end in (("ventricular", "VVI", st.VENTRICULAR, 11, 3, "stop"), ("atrial", "AAI", st.ATRIAL, 10, 1, "link")):
        code = e4.CODE[mode]
        ops = ["up", f"set:0={code},1=90", f"wait:state={e4.PERMANENT}", "hold:3", f"temp:0={code},1=90,{word}=1000", "wait:state=2", f"threshold:{which}", "hold:22"]
        ops += ["threshold-stop", "wait:state=1", "hold:4", "status"] if end == "stop" else ["stop"]
        elf, text = e4.flash_heart([0, 0, 0, 0, 0])
        st.probe("reset", st.BOARD_B)
        st.start_pg()
        cap = st.Capture(40)
        cap.started()
        lines = st.stub(*ops)
        cap.finish()
        checks = []
        t = e4.op(lines, "threshold")
        e4.check(checks, "every operation of the stub succeeded and the test was accepted", all(l["ok"] for l in lines) and t is not None and t["detail"]["accepted"], [l["op"] for l in lines if not l["ok"]])
        if t is None:
            out[name] = e4.result(checks, {"ops": lines})
            ok = False
            continue
        begin = e4.sample_of(cap, e4.op(lines, "wait", 0)["done"]) + 1_000_000
        pulses = [p for p in cap.pulses(pin) if p[1] >= begin]
        g = groups(pulses)
        # The permanent width, the test's start value, the steps, and the permanent width again.
        first_step = next((k for k, x in enumerate(g) if x[0] == 1000), None)
        last_step = max((k for k, x in enumerate(g) if x[0] != 400), default=None)
        steps = g[first_step:last_step + 1] if first_step is not None and last_step is not None else []
        widths = [x[0] for x in steps]
        e4.check(checks, "the widths on the capture: the permanent 0.40 ms, then from 1.00 ms down one setting at a time, then the permanent width again",
                 first_step is not None and g[0][0] == 400 and g[-1][0] == 400 and len(widths) >= 6 and widths == WIDTHS[9:9 + len(widths)], {"runs_of_one_width": g})
        e4.check(checks, "every setting but the first and the last has four pulses (the first has those before the test's start too, the last is cut by the end)",
                 len(steps) >= 6 and all(x[1] == 4 for x in steps[1:-1]), {"pulses_at_each_setting": [x[1] for x in steps]})
        gaps = st.intervals(pulses)
        e4.check(checks, "the back-up pacing is at its rate throughout (90 ppm) and the other Chamber is not paced",
                 st.all_near(gaps, 666_666, e4.TOL) and not [p for p in cap.pulses(1 - pin) if p[1] >= begin], st.spread(gaps))
        if end == "stop":
            told = e4.op(lines, "status")
            e4.check(checks, "the Stop ends the test and temporary pacing: the State is Permanent", told is not None and told["detail"]["state"] == e4.PERMANENT, told and told["detail"])
        else:
            lost = e4.sample_of(cap, lines[-1]["done"])
            after = [p for p in pulses if p[1] > lost + 1_500_000]
            e4.check(checks, "the loss of the link ends the test: 0.40 ms pulses from 1.5 s after the stub stopped answering, to the end of the capture",
                     len(after) >= 5 and st.all_near([p[2] for p in after], 400, st.WIDTH_TOLERANCE_US), st.spread([p[2] for p in after]))
        print(f"  threshold, {name}:", flush=True)
        out[name] = e4.result(checks, {"ops": lines, "heart": text})
        ok = ok and out[name]["verdict"] == "PASS"
    out["verdict"] = "PASS" if ok else "FAIL"
    return out


def run_ert():
    link_build()
    ops = ["up", f"set:0={e4.CODE['DDD']},1=60", f"wait:state={e4.PERMANENT}", "hold:3",
           "read:96,0", "markers:3",                                                    # 4, 5: at BOL
           "battery:2", "wait:battery=2", "read:96,0", "read:64,0", "markers:3",          # 8, 9, 10: at ERT
           "read:16,0", "read:32,0", "read:80,0",
           "battery:3", "wait:battery=3", "read:96,1", "markers:3",                       # 16, 17: at ERP
           "battery:0", "wait:battery=0", "read:96,0", "markers:3", "status"]             # 20, 21: at BOL again
    cap, lines, extra = e4.session("fixed", ops, 36)
    checks = []
    e4.check(checks, "every operation of the stub ran", len(lines) == len(ops), {"ran": len(lines), "of": len(ops)})
    if len(lines) != len(ops):
        return e4.result(checks, {**extra, "ops": lines})
    count = lambda k: len(lines[k]["detail"]["records"]) if isinstance(lines[k]["detail"], dict) else None
    code = lambda k: (lines[k]["detail"].get("answer") or {}).get("code") if isinstance(lines[k]["detail"], dict) else None
    e4.check(checks, "at BOL the measured data is read and the Marker transmission brings records",
             lines[4]["ok"] and lines[5]["ok"] and count(5) >= 6, {"read": lines[4]["ok"], "markers": count(5)})
    e4.check(checks, "at ERT the read of the measured data and of a Marker are answered as declined (code 8), the transmission is not started and no Marker comes (PM17.7)",
             not lines[8]["ok"] and code(8) == 8 and not lines[9]["ok"] and code(9) == 8 and not lines[10]["ok"] and count(10) == 0,
             {"measured": lines[8]["detail"], "marker": lines[9]["detail"], "markers": count(10)})
    e4.check(checks, "at ERT a histogram, the counts and the implant data are read as before", all(lines[k]["ok"] for k in (11, 12, 13)), [lines[k]["ok"] for k in (11, 12, 13)])
    e4.check(checks, "at ERP the same: declined, no Marker", not lines[16]["ok"] and code(16) == 8 and not lines[17]["ok"] and count(17) == 0, {"measured": lines[16]["detail"], "markers": count(17)})
    e4.check(checks, "at BOL again the measured data is read and the transmission brings records", lines[20]["ok"] and lines[21]["ok"] and count(21) >= 6, {"read": lines[20]["ok"], "markers": count(21)})
    others = [l["op"] for k, l in enumerate(lines) if not l["ok"] and k not in (8, 9, 10, 16, 17)]
    e4.check(checks, "every other operation succeeded", others == [], others)
    return e4.result(checks, {**extra, "ops": [dict(l, detail={"records": len(l["detail"]["records"])}) if l["op"].startswith("markers") and isinstance(l["detail"], dict) else l for l in lines]})


RUNS = {"events": ("22.1", run_events), "markers": ("22.2, 22.3", run_markers), "atr_markers": ("22.2", run_atr_markers), "records": ("22.4", run_records),
        "malformed": ("22.4", run_malformed), "stream": ("22.2", run_stream), "interrogation": ("22.5", run_interrogation), "threshold": ("22.6", run_threshold), "ert": ("16.5, 22.4 (finding A9)", run_ert)}


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
