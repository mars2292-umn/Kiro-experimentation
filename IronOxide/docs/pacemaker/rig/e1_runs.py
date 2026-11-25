#!/usr/bin/env python3
"""Tasks 9.4 and 9.5 of docs/pacemaker/tasks.md: the rig runs of stage E1.

Usage: e1_runs.py <9.4|9.5> <record.json> [--only <name>]...

Run from the workspace root with rsk's tools on PATH. For each run of the
task's list:

  1. the pulse generator is built in its rig variant (features
     `nrf52840,debug-port,quiet,rig,set-env`; the declaration variant
     `nrf52840_rig`: the board variant with an Operating_Duration of 45 s)
     with the run's parameter set in `PM_SET`, and the Heart_Simulator in
     the run's scenario (a named one, or `scenario-env` with `PM_HEART`);
  2. both boards are flashed and started, the Heart_Simulator first;
  3. the analyzer captures 30 s at 1 MHz (D0 and D1 the pace pins, D6 and
     D7 the Heart_Simulator's sense mirrors);
  4. after the 45 s the PG is in its safe state and the counts `control`
     copied into its state are read through the probe in one access;
  5. `pm-judge` judges the capture for the run's Mode and set.

The record holds, per run, the judge's report, the driver's counts, both
binaries' hashes and, for task 9.5, every ventricular Pace that followed a
Sense by less than 8 ms with that distance. Exit 0 when every run's
verdict is as the run expects.
"""
import hashlib
import json
import os
import subprocess
import sys
import time

import numpy as np

CHIP = "nRF52840_xxAA"
BOARD_A = "1366:1060:001050217737"
BOARD_B = "1366:1061:001050279833"
NM = "/Library/Developer/CommandLineTools/usr/bin/llvm-nm"
TARGET = "thumbv7em-none-eabihf"
RATE = 1_000_000
SECONDS = 30
DURATION = 45
MAGIC = 0x504D5247
MODES = {"DDD": 1, "VDD": 2, "DDI": 3, "DOO": 4, "AOO": 5, "AAI": 6, "VOO": 7, "VVI": 8, "AAT": 9, "VVT": 10}
WORD = {"MODE": 0, "LRL": 1, "URL": 2, "FIXED_AV": 4, "DYNAMIC_AV": 5, "MIN_DYNAMIC_AV": 6, "SENSED_AV_OFFSET": 7, "A_WIDTH": 10, "V_WIDTH": 11,
        "VRP": 14, "ARP": 15, "PVARP": 16, "PVARP_EXTENSION": 17, "V_BLANKING": 23}
COUNTS = ["started", "hw_fault", "records", "too_late", "moved", "forced", "dropped", "late_arms", "untimed"]
SCRATCH = os.environ.get("PM_SCRATCH", "target/pm-rig-scratch")


def sh(cmd, env=None, check=True):
    out = subprocess.run(cmd, capture_output=True, text=True, env={**os.environ, **(env or {})})
    if check and out.returncode != 0:
        sys.exit(f"{' '.join(cmd)}\n{out.stdout[-2000:]}{out.stderr[-2000:]}")
    return out


def sha(path):
    return "sha256:" + hashlib.sha256(open(path, "rb").read()).hexdigest()


def build_pg(sets):
    text = ",".join(f"{WORD[k]}={v}" for k, v in sets.items())
    sh(["cargo", "build", "-q", "-p", "pacemaker", "--release", "--target", TARGET, "--no-default-features", "--features",
        "nrf52840,debug-port,quiet,rig,set-env", "--target-dir", "target/pacemaker-rig"], env={"PM_SET": text})
    return f"target/pacemaker-rig/{TARGET}/release/pacemaker", text


def build_heart(heart):
    if isinstance(heart, str):
        features = "nrf52840,debug-port,quiet" + ("" if heart == "fixed" else f",scenario-{heart}")
        sh(["cargo", "build", "-q", "-p", "heart-sim", "--release", "--target", TARGET, "--no-default-features", "--features", features,
            "--target-dir", f"target/heart-sim-{heart}"])
        return f"target/heart-sim-{heart}/{TARGET}/release/heart-sim", heart
    text = ",".join(str(n) for n in heart)
    sh(["cargo", "build", "-q", "-p", "heart-sim", "--release", "--target", TARGET, "--no-default-features", "--features",
        "nrf52840,debug-port,quiet,scenario-env", "--target-dir", "target/heart-sim-env"], env={"PM_HEART": text})
    return f"target/heart-sim-env/{TARGET}/release/heart-sim", f"env {text}"


def probe(sub, board, *rest):
    # A probe now and then fails to open with a USB transport error and
    # opens at the next try (seen 2026-11-24).
    for _ in range(3):
        out = sh(["probe-rs", sub, "--chip", CHIP, "--probe", board, *rest], check=False)
        if out.returncode == 0:
            return out
        time.sleep(1.0)
    return sh(["probe-rs", sub, "--chip", CHIP, "--probe", board, *rest])


def counts(elf):
    address = None
    for line in sh([NM, "-C", elf]).stdout.splitlines():
        if "rsk_state::CONTROL_SWEEP" in line:
            address = int(line.split()[0], 16)
    if address is None:
        return None
    words = []
    for line in probe("read", BOARD_A, "b32", f"0x{address:08x}", "10").stdout.splitlines():
        if ":" in line:
            words += [int(x, 16) for x in line.split(":", 1)[1].split()]
    if len(words) < 10 or words[0] != MAGIC:
        return None
    return dict(zip(COUNTS, words[1:10]))


def rises(data, bit):
    d = np.diff(((data >> bit) & 1).astype(np.int8))
    return np.flatnonzero(d == 1) + 1


def near(data, sense_bit, pace_bit, window_us=8000):
    """Every Pace that follows a Sense of its Chamber by less than the
    window, with the distance in µs; and the Senses that had no such Pace."""
    s, p = rises(data, sense_bit), rises(data, pace_bit)
    us = 1e6 / RATE
    followed = []
    for t in p:
        k = np.searchsorted(s, t) - 1
        if k >= 0 and (t - s[k]) * us < window_us:
            followed.append(round(float((t - s[k]) * us), 1))
    return followed, int(len(s)), int(len(p))


def run(r, flashed):
    mode = r["mode"]
    sets = {"MODE": MODES[mode], **r.get("set", {})}
    pg, set_text = build_pg(sets)
    heart, heart_text = build_heart(r["heart"])
    if flashed.get("heart") != sha(heart):
        probe("download", BOARD_B, "--disable-progressbars", heart)
        flashed["heart"] = sha(heart)
    probe("download", BOARD_A, "--disable-progressbars", pg)
    probe("reset", BOARD_B)
    probe("reset", BOARD_A)
    started = time.time()
    capture = os.path.join(SCRATCH, "capture.bin")
    sh(["sigrok-cli", "-d", "fx2lafw", "--config", f"samplerate={RATE}", "--samples", str(RATE * SECONDS), "-O", "binary", "-o", capture])
    time.sleep(max(0.0, DURATION + 3 - (time.time() - started)))
    c = counts(pg)
    report = os.path.join(SCRATCH, "judge.json")
    cmd = ["target/debug/pm-judge", capture, str(RATE), "--mode", mode]
    for k, v in r.get("set", {}).items():
        cmd += ["--set", f"{k}={v}"]
    if isinstance(r["heart"], str):
        cmd += ["--scenario", r["heart"]]
    cmd += ["--binary", f"pacemaker={pg}", "--binary", f"heart-sim={heart}", "--json", report]
    judged = sh(cmd, check=False)
    j = json.load(open(report)) if os.path.exists(report) else {"verdict": "NO REPORT", "error": judged.stderr[-500:]}
    if os.path.exists(report):
        os.remove(report)
    data = np.frombuffer(open(capture, "rb").read(), dtype=np.uint8)
    out = {"name": r["name"], "mode": mode, "PM_SET": set_text, "heart": heart_text, "expect": r.get("expect", "PASS"), "counts": c, "judge": j}
    if r.get("near"):
        followed, senses, paces = near(data, 7, 1)
        out["ventricular_paces_within_8_ms_after_a_sense_us"] = followed
        out["ventricular_senses"] = senses
        out["ventricular_paces"] = paces
    return out


def runs_9_4():
    rs = []
    # Asynchronous pacing at the lowest, the nominal and the highest LRL.
    for mode in ["VOO", "AOO"]:
        for lrl in [30, 60, 175]:
            rs.append({"name": f"{mode.lower()}_lrl_{lrl}", "mode": mode, "set": {"LRL": lrl}, "heart": "fixed"})
    # Inhibition with the intrinsic rate (75 a minute) above and below the
    # LRL; no intrinsic activity; a faster rhythm with a premature atrial
    # beat and a PVC.
    for mode in ["VVI", "AAI"]:
        for lrl in [60, 90]:
            rs.append({"name": f"{mode.lower()}_lrl_{lrl}_fixed", "mode": mode, "set": {"LRL": lrl}, "heart": "fixed"})
        rs.append({"name": f"{mode.lower()}_lrl_60_silent", "mode": mode, "set": {"LRL": 60}, "heart": "silent"})
        rs.append({"name": f"{mode.lower()}_lrl_90_fast", "mode": mode, "set": {"LRL": 90}, "heart": "fast"})
    # Every pulse width of Table 19-1 in the ventricle; the smallest, the
    # nominal and the largest in the atrium.
    widths = [50] + [100 * k for k in range(1, 20)]
    for w in widths:
        rs.append({"name": f"voo_width_{w}", "mode": "VOO", "set": {"LRL": 120, "V_WIDTH": w}, "heart": "fixed"})
    for w in [50, 400, 1900]:
        rs.append({"name": f"aoo_width_{w}", "mode": "AOO", "set": {"LRL": 120, "A_WIDTH": w}, "heart": "fixed"})
    return rs


def runs_9_5():
    """VVI against a ventricular rhythm a little faster than the LRL: each
    beat falls a fixed distance before the pending Pace, the escape interval
    less the rhythm's (a whole number of milliseconds)."""
    rs = []
    pairs = [(71, 845), (70, 857), (77, 779), (84, 714), (72, 833), (63, 952), (56, 1071), (76, 789), (67, 895), (69, 869), (61, 983),
             (87, 689), (82, 731), (81, 740), (74, 810), (83, 722), (59, 1016), (70, 856), (72, 832), (64, 936), (80, 748), (80, 747),
             (80, 746), (80, 745)]
    for lrl, interval in pairs:
        d = 60_000_000 // lrl - interval * 1000
        rs.append({"name": f"vvi_lrl_{lrl}_beat_{interval}", "mode": "VVI", "set": {"LRL": lrl}, "heart": [0, 0, 200, interval, 0], "near": True,
                   "nominal_distance_us": d, "expect": "PASS"})
    return rs


def main():
    task, record = sys.argv[1], sys.argv[2]
    only = [sys.argv[i + 1] for i, a in enumerate(sys.argv) if a == "--only"]
    rs = runs_9_4() if task == "9.4" else runs_9_5()
    if only:
        rs = [r for r in rs if r["name"] in only]
    os.makedirs(SCRATCH, exist_ok=True)
    sh(["cargo", "build", "-q", "-p", "pm-judge"])
    kept = json.load(open(record)) if os.path.exists(record) else {"task": task, "runs": []}
    flashed = {}
    ok = True
    for r in rs:
        out = run(r, flashed)
        if "nominal_distance_us" in r:
            out["nominal_distance_us"] = r["nominal_distance_us"]
        kept["runs"] = [k for k in kept["runs"] if k["name"] != out["name"]] + [out]
        kept["capture"] = {"rate_hz": RATE, "seconds": SECONDS, "operating_duration_s": DURATION}
        json.dump(kept, open(record, "w"), indent=1)
        j, c = out["judge"], out["counts"]
        ch = j.get("ventricular" if r["mode"].startswith("V") else "atrial", {})
        good = j.get("verdict") == out["expect"] and c is not None
        ok = ok and good
        print(f"{'ok  ' if good else 'FAIL'} {r['name']:28} {j.get('verdict'):5} paces {ch.get('paces')} intervals {ch.get('pace_intervals')} widths {ch.get('pulse_widths')} "
              f"senses {ch.get('senses')} eff {ch.get('senses_effective')} refr {ch.get('senses_refractory')} fusions {ch.get('fusions')} counts {c} "
              f"{'near ' + str(out.get('ventricular_paces_within_8_ms_after_a_sense_us')) if r.get('near') else ''} {[v for v in j.get('violations', [])][:3]}", flush=True)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
