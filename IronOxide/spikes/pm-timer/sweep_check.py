#!/usr/bin/env python3
"""Task 7.2 of docs/pacemaker/tasks.md: reads the record of the arming sweep
that the pulse generator's `control` Task ran through the Application's own
hardware layer (apps/pacemaker, feature `sweep`), and checks the analyzer's
capture of the same run.

Usage: sweep_check.py <probe> <elf> <capture.bin> <rate_hz> <record.json>

The record is 32 words of `control`'s per-Task state, read through the debug
probe in one access (the Application is found restarted after an access).
The capture gives every pulse on the ventricular pace pin (D1): each must be
the programmed 400 µs to the sample unless the driver reported it as forced
by software, and their number must be the number the driver reported.
"""
import hashlib
import json
import subprocess
import sys

import numpy as np

CHIP = "nRF52840_xxAA"
NM = "/Library/Developer/CommandLineTools/usr/bin/llvm-nm"
probe, elf, capture, rate, record = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), sys.argv[5]
DONE = 0x5EE9D09E
FIELDS = ["trials", "ok", "moved", "too_late", "forced", "missed", "wrong", "reported"]

address = None
for line in subprocess.run([NM, "-C", elf], capture_output=True, text=True).stdout.splitlines():
    if "rsk_state::CONTROL_SWEEP" in line:
        address = int(line.split()[0], 16)
if address is None:
    sys.exit("no CONTROL_SWEEP symbol in the binary")
out = subprocess.run(["probe-rs", "read", "b32", "--chip", CHIP, "--probe", probe, f"0x{address:08x}", "56"], capture_output=True, text=True)
if out.returncode != 0:
    sys.exit(f"probe-rs: {out.stderr.strip()}")
words = []
for line in out.stdout.splitlines():
    if ":" in line:
        words += [int(x, 16) for x in line.split(":", 1)[1].split()]
rows = []
for k, name in enumerate(["arm", "move", "withdraw"]):
    row = {"operation": name}
    row.update({f: words[4 + 8 * k + i] for i, f in enumerate(FIELDS)})
    rows.append(row)

data = np.frombuffer(open(capture, "rb").read(), dtype=np.uint8)
us = 1e6 / rate


def pulses(bit):
    d = np.diff(((data >> bit) & 1).astype(np.int8))
    rises, falls = np.flatnonzero(d == 1) + 1, np.flatnonzero(d == -1) + 1
    if len(falls) and len(rises) and falls[0] < rises[0]:
        falls = falls[1:]
    n = min(len(rises), len(falls))
    return rises[:n], (falls[:n] - rises[:n]) * us


rises, widths = pulses(1)
# The capture begins before the board's reset. The reset shows as one pulse
# on both pace pins at once (the pins float until the Application configures
# them); the run is the ventricular pulses after it, and the atrial pin must
# stay low from there on.
atrial_rises, _ = pulses(0)
reset_at = int(atrial_rises[-1]) + 5_000 if len(atrial_rises) else 0
restarts = int(len(atrial_rises))
widths = widths[rises > reset_at]
exact = int(np.count_nonzero(np.abs(widths - 400) <= us))
other = sorted(set(float(w) for w in widths[np.abs(widths - 400) > us]))
reported = sum(r["reported"] for r in rows)
forced = sum(r["forced"] for r in rows)
atrial = int(np.count_nonzero(atrial_rises > reset_at))
checks = {
    "the sweep ran to its end": words[31] == DONE,
    "the clock offset was measured within its spread": 0 < words[29] <= 8 and words[28] <= 64,
    "no Pace reported armed failed to fire": all(r["missed"] == 0 for r in rows),
    "no Pace reported withdrawn fired": all(r["wrong"] == 0 for r in rows),
    "every instant was tried": [r["trials"] for r in rows] == [1601, 1801, 1801],
    "the pulses on the pin are the pulses the driver reported": len(widths) == reported,
    "every pulse not forced by software is 400 µs to the sample": len(widths) - exact <= forced,
    "no pulse on the atrial pin": atrial == 0,
}
miss = None
if len(words) >= 52 and words[32]:
    def signed(x):
        return ((x + (1 << 31)) & 0xFFFFFFFF) - (1 << 31)
    then = words[39]
    before = words[46]
    miss = {
        "phase": ["arm", "move", "withdraw"][words[32] - 1], "trial": words[33],
        "promised": {1: "fired at an instant", 2: "fired", 3: "not fired"}.get(words[34], words[34]),
        "at_the_settling_job": {
            "reported_fired": bool(words[36] & 1), "start_flag": (words[36] >> 1) & 1, "end_flag": (words[36] >> 2) & 1,
            "promised_instant_us_before_it": signed(then - words[35]),
            "start_compare_us_before_it": signed(then - words[37]), "end_compare_us_before_it": signed(then - words[38]),
        },
        "the_trial": {
            "asked_us_after_the_present_read_before": signed(words[47] - before),
            "outcome": {1: "armed as asked", 2: "armed, moved", 3: "too late", 4: "forced"}.get(words[48], words[48]),
            "present_after_us_after_the_present_before": signed(words[49] - before),
            "start_compare_us_after_the_present_before": signed(words[50] - before),
            "end_compare_us_after_the_present_before": signed(words[51] - before),
        },
        "words_32_to_51": [f"0x{w:08x}" for w in words[32:52]],
    }
    print("FIRST MISS:", json.dumps(miss, indent=1))
result = {"task": "7.2", "binary_sha256": hashlib.sha256(open(elf, "rb").read()).hexdigest(),
          "lead_us": 500, "guard_us": 64, "offset_spread_us": words[28], "offset_tries": words[29], "moved_max_us": words[30],
          "first_miss": miss, "rows": rows, "pin": {"pulses": int(len(widths)), "at_400_us": exact, "other_widths_us": other, "longest_us": float(widths.max()) if len(widths) else None,
                                "reset_pulses_before_the_run": restarts},
          "checks": checks, "verdict": "PASS" if all(checks.values()) else "FAIL"}
json.dump(result, open(record, "w"), indent=2)
print(f"offset: spread {words[28]} µs in {words[29]} tries; moved by at most {words[30]} µs; phase {words[0]} trial {words[1]}")
for r in rows:
    print(r)
print(result["pin"])
for name, ok in checks.items():
    print(f"{'ok  ' if ok else 'FAIL'} {name}")
sys.exit(0 if all(checks.values()) else 1)
