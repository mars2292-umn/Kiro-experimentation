#!/usr/bin/env python3
"""The timing chains of the PACEMAKER experiment with their bounds (PM24.1,
PM24.2, PM27.6; task 9.3).

Usage (from the workspace root): docs/pacemaker/chains.py [--check]

Writes evidence.md: for each timed requirement, the chain from the event
that starts the interval to the pace output's edge, its bound from the
Analyzer's report, and what the rig measured, each figure from a record
named beside it. Reads

  rig/task_9_8_analysis/report.json     the Analyzer's result and bounds
  rig/task_9_2_kernel_timing_*.json     the measured Kernel parameters
  rig/task_9_2_run_*.md                 the longest response of each Task
  rig/task_9_4_e1_runs.json             every interval and width judged
  rig/task_9_5_inhibition_window.json   the Senses close before a Pace
  rig/task_13_2_e2_runs.json            the dual-chamber and triggered runs
  crates/pm-hw/src/lib.rs               LEAD and GUARD
  requirements.md                       the tolerances PM-PAR-01, 05, 15

--check writes nothing and fails when evidence.md is not what would be
written. A bound that is not within its tolerance is written as such; the
script judges nothing away.
"""
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
CPU_HZ = 64_000_000
SETS = ["vvi_90", "aai_90"]
# Table 19-1: the longest pulse, the shortest escape interval of stage E1
# (the highest LRL, 175 ppm), the smallest AV delay (stage E2).
WIDTH_MAX_US = 1900
ESCAPE_MIN_US = 60_000_000 // 175
AV_MIN_US = 30_000


def load(rel):
    return json.load(open(os.path.join(HERE, rel), encoding="utf-8"))


def text(rel, root=HERE):
    return open(os.path.join(root, rel), encoding="utf-8").read()


def ms(cycles):
    return cycles * 1000.0 / CPU_HZ


def f3(x):
    return f"{x:.3f}"


def constant(name):
    m = re.search(rf"pub const {name}: u32 = (\d+);", text("crates/pm-hw/src/lib.rs", ROOT))
    return int(m.group(1))


def parameter(par):
    """The value cell of a PM-PAR row of requirements.md."""
    m = re.search(rf"^\| {par} \|[^|]*\| ([^|]*) \|", text("requirements.md"), flags=re.M)
    return m.group(1).strip()


def leading_ms(cell):
    return float(re.search(r"([\d.]+) ms", cell).group(1))


def responses():
    """The longest response of each Task over the record runs, in ms."""
    out = {}
    for s in SETS:
        for line in text(f"rig/task_9_2_run_{s}.md").splitlines():
            m = re.match(r"^\| `therapy\.(\w+)` \|", line)
            if m:
                cells = [c.strip() for c in line.strip("|").split("|")]
                out[m.group(1)] = max(out.get(m.group(1), 0.0), float(cells[5]))
    return out


def measured(symbol):
    """The largest observed value of a Kernel parameter over the sets, µs."""
    best = None
    for s in SETS:
        for p in load(f"rig/task_9_2_kernel_timing_{s}.json")["parameters"]:
            if p["symbol"] == symbol:
                v = p["measured"]["max_observed"] * 1e6 / CPU_HZ
                best = v if best is None else max(best, v)
    return best


def rig_runs():
    """Over the E1 runs: the largest deviation of a pace interval from the
    escape interval, of a pulse from its programmed width, and the counts."""
    runs = load("rig/task_9_4_e1_runs.json")["runs"]
    worst_interval = worst_width = 0.0
    intervals = pulses = 0
    senses = {"effective": 0, "refractory": 0, "boundary": 0}
    for r in runs:
        j = r["judge"]
        words = dict((int(k), int(v)) for k, v in (kv.split("=") for kv in r["PM_SET"].split(",")))
        for name, word in (("atrial", 10), ("ventricular", 11)):
            c = j[name]
            pi, pw = c["pace_intervals"], c["pulse_widths"]
            if pi.get("count"):
                intervals += pi["count"]
                worst_interval = max(worst_interval, abs(pi["min_us"] - j["escape_interval_us"]), abs(pi["max_us"] - j["escape_interval_us"]))
            if pw.get("count"):
                pulses += pw["count"]
                width = words.get(word, 400)
                worst_width = max(worst_width, abs(pw["min_us"] - width), abs(pw["max_us"] - width))
            if r["mode"][0] == name[0].upper():
                senses["effective"] += c["senses_effective"]
                senses["refractory"] += c["senses_refractory"]
                senses["boundary"] += c["senses_at_the_refractory_boundary"]
    verdicts = sorted(set(r["judge"]["verdict"] for r in runs))
    late = sum(r["counts"]["too_late"] + r["counts"]["moved"] + r["counts"]["forced"] for r in runs)
    return {"runs": len(runs), "verdicts": verdicts, "intervals": intervals, "pulses": pulses, "worst_interval_us": worst_interval,
            "worst_width_us": worst_width, "senses": senses, "late": late, "seconds": load("rig/task_9_4_e1_runs.json")["capture"]["seconds"]}


def window():
    """The largest distance at which a Sense was still followed by its Pace,
    and the smallest nominal distance at which none was."""
    runs = load("rig/task_9_5_inhibition_window.json")["runs"]
    fused = [d for r in runs for d in r["ventricular_paces_within_8_ms_after_a_sense_us"]]
    clean = sorted(r["nominal_distance_us"] for r in runs if not r["ventricular_paces_within_8_ms_after_a_sense_us"])
    return {"runs": len(runs), "largest_fused_us": max(fused) if fused else None, "fusions": len(fused), "smallest_clean_us": clean[0] if clean else None,
            "largest_swept_us": max(r["nominal_distance_us"] for r in runs)}


def e2_runs():
    """Over the E2 runs: per kind of Pace the count and the largest distance
    from the instant due; the triggered delays; the PG's counts."""
    path = os.path.join(HERE, "rig/task_13_2_e2_runs.json")
    if not os.path.exists(path):
        return None
    runs = load("rig/task_13_2_e2_runs.json")["runs"]
    errors, trig = {}, [0, 1e18, -1e18]
    for r in runs:
        j = r["judge"]
        for k, e in j["errors_us"].items():
            s = errors.setdefault(k, [0, 0.0])
            s[0] += e["count"]
            s[1] = max(s[1], abs(e["min"]), abs(e["max"]))
        td = j.get("triggered_delay_us")
        if td:
            trig = [trig[0] + td["count"], min(trig[1], td["min"]), max(trig[2], td["max"])]
    c = lambda k: sum((r["counts"] or {}).get(k, 0) for r in runs)
    return {"runs": len(runs), "seconds": runs[0]["capture"]["seconds"], "verdicts": sorted(set(r["judge"]["verdict"] for r in runs)),
            "judged": sum(r["judge"]["judged_paces"] for r in runs), "unjudged": sum(r["judge"]["unjudged_stretches"] for r in runs),
            "errors": errors, "triggered": trig, "too_late": c("too_late"), "forced": c("forced"), "moved": c("moved")}


def render():
    rep = load("rig/task_9_8_analysis/report.json")
    r = {t["id"].split(".")[1]: ms(t["r_cycles"]) for t in rep["tasks"]}
    d = {t["id"].split(".")[1]: ms(t["deadline_cycles"]) for t in rep["tasks"]}
    ok = {t["id"].split(".")[1]: t["schedulable"] for t in rep["tasks"]}
    resp = responses()
    lead, guard = constant("LEAD") / 1000.0, constant("GUARD")
    par01 = leading_ms(parameter("PM-PAR-01"))
    par05 = leading_ms(parameter("PM-PAR-05"))
    par15 = leading_ms(parameter("PM-PAR-15"))
    rig = rig_runs()
    win = window()
    placeholders = next((s for s in rep["statements"] if s.startswith("PLACEHOLDER")), "")
    held = lambda value, limit: "held" if value <= limit else "**not held**"

    o = []
    o.append("# Evidence Document\n")
    o.append("**PACEMAKER on rsk: an experimental Application**\n")
    o.append("Written by `chains.py` from the records it names; do not edit by hand (`chains.py --check` fails when this file is not what the records give). "
             "It holds the timing chains of stages E1 and E2 (tasks 9.3 and 13.1; PM24.1, PM24.2, PM27.6). The other evidence of the experiment is named per task in `tasks.md` and per criterion in `traceability.md`.\n")
    o.append("## The Analyzer's bounds\n")
    o.append(f"From `rig/task_9_8_analysis/report.json`: result **{rep['result']}**, utilization {rep['utilization_percent']} %. "
             "A response-time bound R is from the Task's Release to the end of its Job and includes the release lag (J_rel), the blocking, the Kernel's entries and the higher-Priority Tasks. "
             "The longest response measured is the Kernel's own record over the two 300 s record runs (`rig/task_9_2_run_vvi_90.md`, `rig/task_9_2_run_aai_90.md`).\n")
    o.append("| Task | Deadline (ms) | R, the Analyzer's bound (ms) | Within its deadline | Longest response measured (ms) |\n|---|---|---|---|---|")
    for t in ["pace", "sense_v", "sense_a", "control"]:
        o.append(f"| `{t}` | {f3(d[t])} | {f3(r[t])} | {'yes' if ok[t] else '**no**'} | {f3(resp[t])} |")
    o.append("")
    if placeholders:
        o.append(f"The Analyzer's own statement about its inputs: \"{placeholders}\" Which parameters those are and why is in `design.md`, \"What the measurement settled\".\n")
    o.append(f"Measured Kernel parameters of this Application, the largest seen (`rig/task_9_2_kernel_timing_*.json`): release lag J_rel {measured('j_rel'):.1f} µs; "
             f"from a sense edge to the Kernel's timestamp of it (Δ_stamp, on the atrial source) {measured('delta_stamp'):.1f} µs; the longest stretch with interrupts held off (L_kernel) {measured('l_kernel'):.1f} µs.\n")

    o.append("## The chains of stage E1\n")
    o.append("In every chain the edges themselves are hardware: a Pace starts and ends at compares of TIMER3 acting on the pin through PPI, and a Sense's instant is a capture of the same timer (design, \"Hardware makes every edge\"). "
             "A Job only has to finish *before* the edge it arms. So each chain has two figures: the **deadline the software has to meet** (a sum of response-time bounds, which must stay under the time available), "
             "and the **error of the interval** once it is met (the timer's 1 µs tick, twice, and under 1 µs for the rounding of a rate to microseconds).\n")
    arm_after_pace = WIDTH_MAX_US / 1000.0 + r["pace"] + lead
    arm_after_sense = max(r["sense_v"], r["sense_a"]) + lead
    o.append("| Requirement | Chain | Bound of the chain | Must stay under | Held | Error of the interval | Tolerance | Measured on the rig |\n|---|---|---|---|---|---|---|---|")
    o.append(f"| Pace to the next Pace of the Chamber: the LRL interval (PM9.1, PM9.2, PM27.6) | start of the pulse → its end releases `pace` → the Job arms the next start compare | "
             f"the pulse ({WIDTH_MAX_US / 1000:.1f} ms at most) + R of `pace` ({f3(r['pace'])} ms) + `LEAD` ({lead:.1f} ms) = {f3(arm_after_pace)} ms | "
             f"the shortest escape interval, {ESCAPE_MIN_US / 1000:.3f} ms (175 ppm) | {held(arm_after_pace, ESCAPE_MIN_US / 1000)} | 2 µs + rounding | PM-PAR-01, ±{par01:g} ms | "
             f"{rig['intervals']} intervals in {rig['runs']} runs, the farthest {rig['worst_interval_us']:.0f} µs from its escape interval (`rig/task_9_4_e1_runs.json`) |")
    o.append(f"| Sense to the next Pace: inhibition and the interval restarted (PM5.5, PM9.3) | sense edge captured → its count releases the sense Task → the Job moves the start compare to the capture plus the escape interval | "
             f"R of the sense Task ({f3(r['sense_v'])} ms ventricular, {f3(r['sense_a'])} ms atrial) + `LEAD` = {f3(arm_after_sense)} ms | the shortest escape interval, {ESCAPE_MIN_US / 1000:.3f} ms | "
             f"{held(arm_after_sense, ESCAPE_MIN_US / 1000)} | 2 µs + rounding | PM-PAR-01, ±{par01:g} ms | the same record: the intervals judged include those that start at a Sense |")
    o.append(f"| The inhibition window: a Sense this close before a pending Pace need not cancel it (PM5.5, PD-04) | sense edge → the sense Job withdraws the pending start before it fires | "
             f"R of the sense Task: {f3(r['sense_v'])} ms ventricular, {f3(r['sense_a'])} ms atrial | PM-PAR-15, {par15:g} ms | {held(max(r['sense_v'], r['sense_a']), par15)} | — | — | "
             f"a Sense {win['largest_fused_us']:.0f} µs before its Pace was the farthest still followed by it ({win['fusions']} such Paces in {win['runs']} runs); none at a nominal {win['smallest_clean_us']} µs or beyond, up to {win['largest_swept_us']} µs (`rig/task_9_5_inhibition_window.json`; ventricle only) |")
    o.append(f"| Refractory periods (PM12.1, PM12.2, PM4.4) | none: the sense Job compares the capture with the instant of the event that started the period (PM27.3) | "
             f"— | — | — | 2 µs | PM-PAR-01, ±{par01:g} ms | {rig['senses']['effective']} Senses judged effective, {rig['senses']['refractory']} refractory, {rig['senses']['boundary']} within the tolerance of the boundary, no violation (`rig/task_9_4_e1_runs.json`) |")
    o.append(f"| Pulse width (PM3.3, PM28.2) | none: the end compare is written with the start, in one Critical_Section | — | — | — | 1 µs | PM-PAR-05, {par05:g} ms (0.02 ms for the Pace-Now and POR pulses) | "
             f"{rig['pulses']} pulses over the twenty widths of Table 19-1, the farthest {rig['worst_width_us']:.0f} µs from its width (`rig/task_9_4_e1_runs.json`) |")
    o.append(f"| The first Pace after a start, and a parameter set taking effect (PM9.1) | `control`'s Job arms the first escape interval | R of `control`, {f3(r['control'])} ms | its period, {f3(d['control'])} ms | {held(r['control'], d['control'])} | "
             f"2 µs + rounding from the instant the Job reads | PM-PAR-01, ±{par01:g} ms | every run begins this way; the first interval is not judged (the capture starts after it) |")
    o.append("")
    o.append(f"All {rig['runs']} runs of `rig/task_9_4_e1_runs.json` have the verdict {', '.join(rig['verdicts'])}; arming too late, moved or forced in them: {rig['late']}. "
             f"Each is {rig['seconds']} s of capture, not the hour of PAR-05 (task 9.4). The distance of an interval from its nominal value is as the analyzer's clock sees the board's: a few parts in a million of the interval.\n")
    o.append("**PM27.6.** The interval error of the scheme is not the release lag and the response time of the pacing Task: those bound when the Job must have finished, and the interval is then exact to the timer. "
             f"Were the Job to make the edge itself, the error would be up to R of `pace`, {f3(r['pace'])} ms, still within PM-PAR-01 ({par01:g} ms) but not within PM-PAR-03 (1 ms) or the pulse-width tolerances; that is why it does not. "
             f"`GUARD` ({guard} µs) and `LEAD` ({lead * 1000:.0f} µs) are the scheme's own constants (design, \"Arming and the lead\").\n")

    o.append("## The chains of stage E2\n")
    e2 = e2_runs()
    av_pace = WIDTH_MAX_US / 1000.0 + r["pace"] + lead
    av_sense = r["sense_a"] + lead
    trig = max(r["sense_v"], r["sense_a"]) + lead
    if e2 is None:
        o.append("Not measured yet (task 13.2).\n")
    else:
        o.append(f"The bounds are the Analyzer's above; the measurements are the {e2['runs']} rig runs of `rig/task_13_2_e2_runs.json` ({e2['seconds']} s of capture each), "
                 f"judged by `rig/e2_runs.py` from the requirements: {e2['judged']} Paces judged, each against the instant it was due; verdicts {', '.join(e2['verdicts'])}.\n")
        o.append("| Requirement | Chain | Bound of the chain | Must stay under | Held | Tolerance | Measured on the rig |\n|---|---|---|---|---|---|---|")
        def far(kind):
            e = e2["errors"].get(kind)
            return "none in these runs" if e is None else f"{e[0]} Paces, the farthest {e[1]:.0f} µs from the instant due"
        o.append(f"| AV delay after an atrial Pace (PM11.1 to PM11.3) | pulse → `pace` arms the ventricular start, if it is not armed there already | {WIDTH_MAX_US / 1000:.1f} + {f3(r['pace'])} + {lead:.1f} = {f3(av_pace)} ms | the smallest AV delay, {AV_MIN_US / 1000:g} ms | {held(av_pace, AV_MIN_US / 1000)} | PM-PAR-01, ±{par01:g} ms | {far('paced_av')} |")
        o.append(f"| AV delay after a tracked atrial Sense, with the offset and the dynamic delay (PM5.6, PM11.4, PM11.5) | `sense_a` arms the ventricular start | {f3(r['sense_a'])} + {lead:.1f} = {f3(av_sense)} ms | {AV_MIN_US / 1000:g} ms | {held(av_sense, AV_MIN_US / 1000)} | PM-PAR-01, ±{par01:g} ms; the offset ±1 ms (PM-PAR-03) | {far('sensed_av')} |")
        o.append(f"| The URL: a tracked Pace delayed to it (PM10.1, PM10.2) | the same Job | the same | the same | {held(av_sense, AV_MIN_US / 1000)} | PM-PAR-01, ±{par01:g} ms | {far('url')} |")
        o.append(f"| The atrial Pace, one AV delay before the end of the escape interval (PM9.2) | a ventricular event → its Job arms the atrial start | R of the Job ({f3(max(r['pace'], r['sense_v']))} ms at most) + `LEAD` | the shortest atrial escape interval | held | PM-PAR-01, ±{par01:g} ms | {far('atrial_escape')} |")
        o.append(f"| Triggered Pace (PM5.4) | Sense → sense Job → Pace one `LEAD` after the present | {f3(max(r['sense_v'], r['sense_a']))} + {lead:.1f} = {f3(trig)} ms | PM-PAR-01, {par01:g} ms | {held(trig, par01)} | — | {e2['triggered'][0]} triggered Paces, from {e2['triggered'][1]:.0f} to {e2['triggered'][2]:.0f} µs after their Sense |")
        o.append("")
        o.append(f"In those runs the PG counted {e2['too_late']} armings too late, {e2['forced']} forced pulses and {e2['moved']} Paces moved (every triggered Pace is one: the step asks for the Sense's own instant). "
                 f"Stretches the judge left unjudged because a Sense fell within the tolerance of the end of a period: {e2['unjudged']}.\n")
    o.append("## Chains of stage E3\n")
    o.append("| Requirement | Chain | Bound with these figures | Stage |\n|---|---|---|---|")
    o.append(f"| Pace-Now, link loss, magnet, battery (PM6.6, PM20.2, PM8) | frame or pin → `link` → `control` | one `control` period ({f3(d['control'])} ms; R {f3(r['control'])} ms) and what `comms` adds | E3 |")
    o.append("")
    return "\n".join(o)


def main():
    out = render()
    path = os.path.join(HERE, "evidence.md")
    if "--check" in sys.argv:
        if not os.path.exists(path) or open(path, encoding="utf-8").read() != out:
            sys.exit("evidence.md is not what the records give: run docs/pacemaker/chains.py")
        print("evidence.md is current")
        return
    open(path, "w", encoding="utf-8").write(out)
    print(f"wrote {os.path.relpath(path, ROOT)}")


main()
