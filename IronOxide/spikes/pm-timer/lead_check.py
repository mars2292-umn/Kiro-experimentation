#!/usr/bin/env python3
"""Task 1.3 of docs/pacemaker/tasks.md: reads the result of the spike built
with the feature `lead` and checks the analyzer's capture of the same run.

Usage: lead_check.py <probe A> <capture.bin> <rate_hz> <record.json> [<elf>]

The spike's counts are read from RAM through the debug probe (0x2000_1000:
a header, then 16 words per configuration). The capture (sigrok binary, one
byte per sample; the ventricular pace pin on D1) gives every pulse on the
pin: its width must be the programmed 400 µs to the sample, apart from the
pulses the driver itself reported as forced by software, and the number of
pulses must be the number the driver reported.

Exit 0 when the chip's own checks found nothing wrong and the capture agrees.
"""
import hashlib
import json
import subprocess
import sys

CHIP = "nRF52840_xxAA"
RESULTS = 0x2000_1000
ROW = 16
DONE = 0x1D3AD09E
WIDTH_US = 400
OPS = {0: "arm", 1: "move", 2: "withdraw"}
FIELDS = ["trials", "ok", "moved", "too_late", "late_timed", "late_untimed", "missed", "wrong", "reported",
          "interrupts", "moved_max_us"]


def read_words(probe, addr, n):
    out = subprocess.run(["probe-rs", "read", "b32", "--chip", CHIP, "--probe", probe, f"0x{addr:08x}", str(n)],
                         capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"probe-rs: {out.stderr.strip()}")
    words = []
    for line in out.stdout.splitlines():
        if ":" in line:
            words += [int(x, 16) for x in line.split(":", 1)[1].split()]
    return words


def pulses(data, bit):
    """(start_sample, length) of every complete high pulse of a channel."""
    import numpy as np
    a = (np.frombuffer(data, dtype=np.uint8) >> bit) & 1
    d = np.diff(a.astype(np.int8))
    rises = np.flatnonzero(d == 1) + 1
    falls = np.flatnonzero(d == -1) + 1
    if len(falls) and len(rises) and falls[0] < rises[0]:
        falls = falls[1:]
    n = min(len(rises), len(falls))
    return list(zip(rises[:n].tolist(), (falls[:n] - rises[:n]).tolist()))


def main():
    probe, capture, rate, record = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
    elf = sys.argv[5] if len(sys.argv) > 5 else None
    head = read_words(probe, RESULTS, 3)
    configs = head[1]
    if head[0] != DONE or head[2] != ROW:
        sys.exit(f"the run is not finished: header {[hex(x) for x in head]}")
    rows = []
    for k in range(configs):
        w = read_words(probe, RESULTS + 4 * ROW * (k + 1), ROW)
        row = {"operation": OPS[w[0]], "lead_us": w[1], "guard_us": w[2], "hold_us": w[3]}
        row.update({name: w[4 + i] for i, name in enumerate(FIELDS)})
        row["complete"] = w[15] == DONE
        rows.append(row)

    data = open(capture, "rb").read()
    us = 1e6 / rate
    v = pulses(data, 1)
    a = pulses(data, 0)
    widths = {}
    for _, n in v:
        widths[n] = widths.get(n, 0) + 1
    exact = sum(c for n, c in widths.items() if abs(n * us - WIDTH_US) <= us)
    other = {f"{n * us:g}": c for n, c in sorted(widths.items()) if abs(n * us - WIDTH_US) > us}
    reported = sum(r["reported"] for r in rows)
    forced = sum(r["late_timed"] + r["late_untimed"] for r in rows)
    chip_wrong = sum(r["missed"] + r["wrong"] for r in rows)
    longest = max((n for _, n in v), default=0) * us

    checks = {
        "every configuration ran to its end": all(r["complete"] for r in rows),
        "the pulses on the pin are the pulses reported, and the untimed forced ones": len(v) == reported + sum(r["late_untimed"] for r in rows),
        "every pulse not forced by software is 400 µs to the sample": sum(other.values()) <= forced,
        "no pulse on the atrial pin": len(a) == 0,
    }
    out = {
        "task": "1.3",
        "binary_sha256": hashlib.sha256(open(elf, "rb").read()).hexdigest() if elf else None,
        "capture": {"rate_hz": rate, "seconds": len(data) / rate, "sha256": hashlib.sha256(data).hexdigest()},
        "configurations": rows,
        "pin": {"pulses": len(v), "at_400_us": exact, "other_widths_us": other, "longest_us": longest,
                "reported_by_the_driver": reported, "forced_by_software": forced},
        "chip_checks_failed": chip_wrong,
        "capture_checks": checks,
        "verdict": "PASS" if all(checks.values()) else "FAIL",
    }
    json.dump(out, open(record, "w"), indent=2)
    print(f"{'op':9}{'lead':>6}{'guard':>6}{'hold':>6}{'trials':>8}{'ok':>7}{'moved':>7}{'late':>6}{'forced':>7}{'missed':>7}{'wrong':>6}{'irqs':>7}{'max':>6}")
    for r in rows:
        print(f"{r['operation']:9}{r['lead_us']:6}{r['guard_us']:6}{r['hold_us']:6}{r['trials']:8}{r['ok']:7}{r['moved']:7}{r['too_late']:6}"
              f"{r['late_timed'] + r['late_untimed']:7}{r['missed']:7}{r['wrong']:6}{r['interrupts']:7}{r['moved_max_us']:6}")
    print(f"pin: {len(v)} pulses, {exact} at 400 µs, others {other}, longest {longest:g} µs; reported {reported}, forced {forced}")
    for name, ok in checks.items():
        print(f"{'ok  ' if ok else 'FAIL'} {name}")
    sys.exit(0 if all(checks.values()) else 1)


main()
