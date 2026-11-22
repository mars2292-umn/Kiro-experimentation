#!/usr/bin/env python3
"""Measures pulse widths in a logic-analyzer capture of the spike (task 1.1).

Usage: measure.py CAPTURE.bin SAMPLERATE_HZ [--json OUT.json]

The capture is sigrok-cli's binary output: one byte per sample, bit n the
channel Dn. The atrial pace pin is D0, the ventricular D1. Every complete
high pulse is measured, assigned to the nearest programmed width, and judged
against PM-PAR-05: 0.2 ms for every width, and ±0.02 ms for the Power-On
Reset (0.5 ms) and Pace-Now (1.00 ms) widths.
"""
import json
import sys

WIDTHS_US = [50, 400, 500, 1000, 1900]
TIGHT_US = {500: 20.0, 1000: 20.0}
LOOSE_US = 200.0
PINS = {"atrial (D0)": 0, "ventricular (D1)": 1}


def pulses(data, bit):
    """(start_sample, length_in_samples) of every complete high pulse."""
    mask = 1 << bit
    out = []
    start = None
    # Leave out a pulse cut by the capture's beginning.
    prev = data[0] & mask
    seen_low = not prev
    for i in range(1, len(data)):
        cur = data[i] & mask
        if cur and not prev:
            start = i if seen_low else None
        elif prev and not cur:
            if start is not None:
                out.append((start, i - start))
            start = None
            seen_low = True
        prev = cur
    return out


def main():
    path, rate = sys.argv[1], int(sys.argv[2])
    data = open(path, "rb").read()
    us = 1e6 / rate
    report = {"capture": path, "samplerate_hz": rate, "samples": len(data), "sample_period_us": us, "pins": {}}
    ok = True
    for name, bit in PINS.items():
        groups = {w: [] for w in WIDTHS_US}
        stray = []
        for _, n in pulses(data, bit):
            width = n * us
            nearest = min(WIDTHS_US, key=lambda w: abs(w - width))
            # A pulse further from every programmed width than the loose
            # tolerance belongs to none.
            (groups[nearest] if abs(width - nearest) <= LOOSE_US and abs(width - nearest) < nearest / 2 else stray).append(width)
        rows = []
        for w in WIDTHS_US:
            g = groups[w]
            if not g:
                rows.append({"programmed_us": w, "pulses": 0})
                ok = False
                continue
            worst = max(abs(x - w) for x in g)
            tol = TIGHT_US.get(w, LOOSE_US)
            rows.append({"programmed_us": w, "pulses": len(g), "min_us": min(g), "max_us": max(g), "mean_us": sum(g) / len(g), "worst_error_us": worst, "tolerance_us": tol, "within": worst <= tol})
            ok = ok and worst <= tol
        report["pins"][name] = {"widths": rows, "unassigned": stray}
        ok = ok and not stray
    report["pass"] = ok
    for name, pin in report["pins"].items():
        print(name)
        for r in pin["widths"]:
            if r["pulses"] == 0:
                print(f"  {r['programmed_us']:>5} us: no pulse")
            else:
                print(f"  {r['programmed_us']:>5} us: {r['pulses']:>4} pulses  min {r['min_us']:.2f}  max {r['max_us']:.2f}  mean {r['mean_us']:.3f}  worst error {r['worst_error_us']:.2f} us  tolerance {r['tolerance_us']:.0f} us  {'ok' if r['within'] else 'OUT'}")
        if pin["unassigned"]:
            print(f"  unassigned pulses: {len(pin['unassigned'])}: {sorted(pin['unassigned'])[:10]}")
    print("PASS" if ok else "FAIL")
    if "--json" in sys.argv:
        json.dump(report, open(sys.argv[sys.argv.index("--json") + 1], "w"), indent=1)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
