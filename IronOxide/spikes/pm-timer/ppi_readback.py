#!/usr/bin/env python3
"""Task 3.6 of docs/pacemaker/tasks.md (requirement PM33.5): after the PPI
probe (apps/ppi-probe) has run on the board, reads the PPI's registers back
and compares them with the channels of the Task_Model, reads the two
Partitions' attempt records, and checks the analyzer's capture of the run.

Usage: ppi_readback.py <probe> <task_model.json> <elf> <capture.bin> <rate_hz> <record.json>

- Read-back: for every channel of the Task_Model, CH[n].EEP, CH[n].TEP and
  FORK[n].TEP hold the addresses the model states, and CHEN has the bits of
  exactly those channels.
- Isolation: each Partition counted every attempt it was to make and none
  returned (`attempt` and `survived` of the per-Task state, found by their
  symbols in the binary).
- The channels at work: a 400 µs pulse on the pace pin (D0) every 10 ms, the
  only gap being the three Jobs the first Partition's own attempts ended.
- The second Partition's Jobs on its two pins: D2 toggles when a Job begins
  (every 10 ms, without a gap) and D3 when it ends. The level written is
  the Job's parity, so seven Jobs in a row that a fault ended show as one
  gap of 90 ms between D3's edges and no other gap.
"""
import hashlib
import json
import subprocess
import sys

CHIP = "nRF52840_xxAA"
PPI = 0x4001_F000
NM = "/Library/Developer/CommandLineTools/usr/bin/llvm-nm"
probe, model_path, elf, capture, rate, record = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5]), sys.argv[6]


def rd(addr):
    out = subprocess.run(["probe-rs", "read", "b32", "--chip", CHIP, "--probe", probe, f"0x{addr:08x}", "1"],
                         capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"probe-rs: {out.stderr.strip()}")
    return int(out.stdout.split()[-1], 16)


symbols = {}
for line in subprocess.run([NM, "-C", elf], capture_output=True, text=True).stdout.splitlines():
    parts = line.split(None, 2)
    if len(parts) == 3 and "::rsk_state::" in parts[2]:
        symbols[parts[2].rsplit("::h", 1)[0]] = int(parts[0], 16)


# One read for every state word: on this rig the Application is found
# restarted after each access through the debug probe (not investigated),
# and a restart clears the records. The first access sees the run's own.
# The read spans both Partitions' RAM and takes about a second, during
# which the Tasks run on: the two `count` values are not of one instant.
lo, hi = min(symbols.values()), max(symbols.values())
block = subprocess.run(["probe-rs", "read", "b32", "--chip", CHIP, "--probe", probe, f"0x{lo:08x}", str((hi - lo) // 4 + 1)],
                       capture_output=True, text=True)
if block.returncode != 0:
    sys.exit(f"probe-rs: {block.stderr.strip()}")
words = []
for line in block.stdout.splitlines():
    if ":" in line:
        words += [int(x, 16) for x in line.split(":", 1)[1].split()]


def state(name):
    return words[(symbols[name] - lo) // 4]


partitions = {}
for crate, task, attempts in [("ppi_probe_timer", "PULSE", 3), ("ppi_probe_outsider", "POKE", 7)]:
    p = {k: state(f"{crate}::rsk_state::{task}_{k.upper()}") for k in ("count", "attempt", "survived")}
    p["attempts_expected"] = attempts
    partitions[crate] = p

model = json.load(open(model_path))
channels = model.get("ppi", [])
readback = []
for c in channels:
    n = c["channel"]
    want = [int(c["event"]["address"], 16), int(c["task"]["address"], 16), int(c["fork"]["address"], 16) if "fork" in c else 0]
    got = [rd(PPI + 0x510 + 8 * n), rd(PPI + 0x514 + 8 * n), rd(PPI + 0x910 + 4 * n)]
    readback.append({"channel": n, "id": c["id"], "expected": [f"0x{x:08x}" for x in want], "read": [f"0x{x:08x}" for x in got], "same": want == got})
chen = rd(PPI + 0x500)
chen_want = sum(1 << c["channel"] for c in channels)
# A channel the Application did not declare stays as the reset left it.
spare = len(channels)
spare_read = [rd(PPI + 0x510 + 8 * spare), rd(PPI + 0x514 + 8 * spare), rd(PPI + 0x910 + 4 * spare)]

import numpy as np
data = np.frombuffer(open(capture, "rb").read(), dtype=np.uint8)
us = 1e6 / rate
d = np.diff((data & 1).astype(np.int8))
rises, falls = np.flatnonzero(d == 1) + 1, np.flatnonzero(d == -1) + 1
if len(falls) and len(rises) and falls[0] < rises[0]:
    falls = falls[1:]
k = min(len(rises), len(falls))
widths = (falls[:k] - rises[:k]) * us
gaps = np.diff(rises[:k]) * us
# The capture begins before the board's reset, and the reset through the
# probe restarts the board more than once: the run is what follows the last
# gap of a boot (over 45 ms), less the pulse the first Job of that boot made.
boots = [i for i, g in enumerate(gaps) if g > 45_000]
first = boots[-1] + 2 if boots else 0
widths, gaps = widths[first:], gaps[first:]
k = len(widths)
long_gaps = [round(float(g) / 1000, 2) for g in gaps if g > 10_500]
pin = {"restarts_before_the_run": len(boots), "pulses": int(k), "width_min_us": float(widths.min()) if k else None, "width_max_us": float(widths.max()) if k else None,
       "spacing_min_ms": float(gaps.min()) / 1000 if k > 1 else None, "spacing_max_ms_but_the_long_ones": float(max(g for g in gaps if g <= 10_500)) / 1000 if k > 1 else None,
       "spacings_over_10_5_ms": long_gaps}

def toggles(bit):
    x = np.diff(((data >> bit) & 1).astype(np.int8))
    return np.flatnonzero(x != 0) + 1


run_start = rises[first] if first < len(rises) else 0
began = toggles(2)
began = began[began > run_start]
ended = toggles(3)
ended = ended[ended > run_start]
began_gaps = np.diff(began) * us
ended_gaps = np.diff(ended) * us
jobs = {"began_edges": int(len(began)), "began_spacing_min_ms": float(began_gaps.min()) / 1000, "began_spacing_max_ms": float(began_gaps.max()) / 1000,
        "ended_edges": int(len(ended)), "ended_spacings_over_10_5_ms": [round(float(g) / 1000, 2) for g in ended_gaps if g > 10_500]}

checks = {
    "every declared channel reads back as the Task_Model states it": bool(channels) and all(r["same"] for r in readback),
    "CHEN has the bits of exactly the declared channels": chen == chen_want,
    "the first channel not declared is unconnected": spare_read == [0, 0, 0],
    "both Partitions made every attempt": all(p["attempt"] == p["attempts_expected"] for p in partitions.values()),
    "no attempt returned": all(p["survived"] == 0 for p in partitions.values()),
    "both Tasks still run": all(p["count"] > 100 for p in partitions.values()),
    "the second Partition's Jobs begin every 10 ms without a gap": len(began) > 100 and float(began_gaps.min()) > 9_900 and float(began_gaps.max()) < 10_100,
    "seven of its Jobs in a row did not reach their end, and no other": jobs["ended_spacings_over_10_5_ms"] == [90.0],
    "every pulse on the pace pin is 400 µs to the sample": k > 100 and float(widths.min()) >= 400 - us and float(widths.max()) <= 400 + us,
    "the pulses come every 10 ms but for one gap of 40 ms, the three Jobs the owner's attempts ended": long_gaps == [40.0] and float(gaps.min()) > 9_900,
}
out = {"task": "3.6", "binary_sha256": hashlib.sha256(open(elf, "rb").read()).hexdigest(), "config_checksum": model["config_checksum"],
       "readback": readback, "chen": f"0x{chen:08x}", "chen_expected": f"0x{chen_want:08x}",
       "first_undeclared_channel": {"channel": spare, "read": [f"0x{x:08x}" for x in spare_read]},
       "partitions": partitions, "pace_pin": pin, "second_partition_jobs": jobs, "checks": checks, "verdict": "PASS" if all(checks.values()) else "FAIL"}
json.dump(out, open(record, "w"), indent=2)
for r in readback:
    print(f"channel {r['channel']} ({r['id']}): read {r['read']} expected {r['expected']}")
print(f"CHEN {chen:#010x}, expected {chen_want:#010x}; partitions {partitions}")
print(f"pin {pin}")
print(f"jobs {jobs}")
for name, ok in checks.items():
    print(f"{'ok  ' if ok else 'FAIL'} {name}")
sys.exit(0 if all(checks.values()) else 1)
