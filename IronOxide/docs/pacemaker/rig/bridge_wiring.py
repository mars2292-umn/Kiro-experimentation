#!/usr/bin/env python3
"""Task 4.1 of docs/pacemaker/tasks.md: measures the bridge's wires between
board A (the PG) and board C (the bridge) through the two debug probes.

Usage: bridge_wiring.py <record.json>

For each of P0.28 to P0.31, in each direction: one board drives the pin
high and low while the other reads its input register with the four pins
as inputs, first under pull-downs, then under pull-ups. A wire is there
when the reader's bit follows the driver under both pulls and no other of
the four bits moves. The pins are put back to their reset configuration.
"""
import json
import subprocess
import sys

CHIP = "nRF52840_xxAA"
A = "1366:1060:001050217737"
C = "1366:1061:001050211852"
P0 = 0x5000_0000
OUTSET, OUTCLR, IN, PIN_CNF = P0 + 0x508, P0 + 0x50C, P0 + 0x510, P0 + 0x700
PINS = {28: "SCK", 29: "MOSI", 30: "MISO", 31: "CSN"}
OUTPUT, PULL_DOWN, PULL_UP, RESET = 0b11, 0b0100, 0b1100, 0b10


def write(board, addr, value):
    out = subprocess.run(["probe-rs", "write", "b32", "--chip", CHIP, "--probe", board, f"0x{addr:08x}", f"0x{value:x}"], capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"probe-rs write: {out.stderr.strip()}")


def read(board, addr):
    out = subprocess.run(["probe-rs", "read", "b32", "--chip", CHIP, "--probe", board, f"0x{addr:08x}", "1"], capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"probe-rs read: {out.stderr.strip()}")
    for line in out.stdout.splitlines():
        if ":" in line:
            return int(line.split(":", 1)[1].split()[0], 16)
    return int(out.stdout.split()[-1], 16)


def bits(board):
    return (read(board, IN) >> 28) & 0xF


def main():
    record = {"task": "4.1: the bridge's wires", "boards": {"A": A, "C": C}, "lines": [], "before": {}}
    for name, b in (("A", A), ("C", C)):
        record["before"][name] = {f"P0.{n}": f"0x{read(b, PIN_CNF + 4 * n):x}" for n in PINS}
    ok = True
    for (dn, driver), (rn, reader) in (((("A"), A), ("C", C)), (("C", C), ("A", A))):
        # The driver's four pins as at reset: a pull left on one of them
        # from the other direction would oppose the reader's pull.
        for m in PINS:
            write(driver, PIN_CNF + 4 * m, RESET)
        for n, signal in PINS.items():
            row = {"pin": f"P0.{n}", "signal": signal, "driver": dn, "reader": rn, "reads": {}}
            write(driver, PIN_CNF + 4 * n, OUTPUT)
            good = True
            for pull_name, pull in (("pull-down", PULL_DOWN), ("pull-up", PULL_UP)):
                for m in PINS:
                    write(reader, PIN_CNF + 4 * m, pull)
                idle = 0x0 if pull == PULL_DOWN else 0xF
                for level, reg in (("high", OUTSET), ("low", OUTCLR)):
                    write(driver, reg, 1 << n)
                    got = bits(reader)
                    want = (idle & ~(1 << (n - 28))) | ((1 << (n - 28)) if level == "high" else 0)
                    row["reads"][f"{pull_name}, driven {level}"] = f"{got:04b} (expected {want:04b})"
                    good = good and got == want
            write(driver, PIN_CNF + 4 * n, RESET)
            row["connected"] = good
            ok = ok and good
            record["lines"].append(row)
            print(f"{'ok  ' if good else 'FAIL'} P0.{n} {signal:4} {dn} -> {rn}: {row['reads']}")
    for b in (A, C):
        for n in PINS:
            write(b, PIN_CNF + 4 * n, RESET)
            write(b, OUTCLR, 1 << n)
    record["verdict"] = "PASS" if ok else "FAIL"
    json.dump(record, open(sys.argv[1], "w"), indent=1)
    print(record["verdict"])
    sys.exit(0 if ok else 1)


main()
