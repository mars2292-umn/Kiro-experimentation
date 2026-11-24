#!/usr/bin/env python3
"""Tasks 15.2 and 15.3 of docs/pacemaker/tasks.md: the rig runs of the DCM
link.

Usage: link_runs.py <15.2|15.3> <record.json> [soak seconds]

Run from the workspace root with rsk's tools on PATH. Board C carries the
bridge (`tools/pm-bridge`), board A the pulse generator; the host reaches
the bridge through board C's debug probe's serial port.

  15.2  the bridge is built and flashed; the DCM_Stub sends 400 frames to
        it and requires each back unchanged (the host side's loop-back).
  15.3  the pulse generator (features `nrf52840,debug-port,quiet`) and the
        bridge are built, flashed and started; the DCM_Stub runs its
        session (interrogation, a parameter set in parts and read back, a
        command the stage does not serve), then polls for the soak time;
        the counts `link` keeps are read from the pulse generator through
        its probe.

The record holds each command's own record, the binaries' hashes and the
pulse generator's counts. Exit 0 when every part passed.
"""
import glob
import hashlib
import json
import os
import subprocess
import sys

CHIP = "nRF52840_xxAA"
BOARD_A = "1366:1060:001050217737"
BOARD_C = "1366:1061:001050211852"
NM = "/Library/Developer/CommandLineTools/usr/bin/llvm-nm"
TARGET = "thumbv7em-none-eabihf"
BRIDGE = f"tools/pm-bridge/target/{TARGET}/release/pm-bridge"
PG = f"target/pacemaker-link/{TARGET}/release/pacemaker"
STUB = "target/debug/dcm-stub"
SCRATCH = os.environ.get("PM_SCRATCH", "target/pm-rig-scratch")
# `proto::Link`'s words, in the order the compiler laid them out, are read
# by name from the map below after the counts beside it.
COUNTS = ["LINK_STARTED", "LINK_TRANSFERS", "LINK_STUCK"]


def sh(cmd, cwd=None, check=True):
    out = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd)
    if check and out.returncode != 0:
        sys.exit(f"{' '.join(cmd)}\n{out.stdout[-2000:]}{out.stderr[-2000:]}")
    return out


def sha(path):
    return "sha256:" + hashlib.sha256(open(path, "rb").read()).hexdigest()


def probe(sub, board, *rest):
    return sh(["probe-rs", sub, "--chip", CHIP, "--probe", board, *rest])


def serial_port():
    """Board C's probe's first serial port: its name carries the probe's
    serial number."""
    number = BOARD_C.split(":")[-1]
    ports = sorted(glob.glob(f"/dev/cu.usbmodem{number}*"))
    if not ports:
        sys.exit(f"no serial port of probe {number}")
    return ports[0]


def word(board, address):
    for line in probe("read", board, "b32", f"0x{address:08x}", "1").stdout.splitlines():
        if ":" in line:
            return int(line.split(":", 1)[1].split()[0], 16)
    return None


def pg_counts():
    out = {}
    for line in sh([NM, "-C", PG]).stdout.splitlines():
        for name in COUNTS:
            if f"pacemaker_comms::rsk_state::{name}::" in line:
                out[name.lower()] = word(BOARD_A, int(line.split()[0], 16))
    return out


def stub(*args):
    report = os.path.join(SCRATCH, "stub.json")
    if os.path.exists(report):
        os.remove(report)
    out = sh([STUB, *args, "--json", report], check=False)
    print(out.stdout[-3000:], end="")
    if not os.path.exists(report):
        return {"verdict": "NO REPORT", "error": (out.stdout + out.stderr)[-800:]}
    return json.load(open(report))


def main():
    task, record = sys.argv[1], sys.argv[2]
    seconds = sys.argv[3] if len(sys.argv) > 3 else "120"
    os.makedirs(SCRATCH, exist_ok=True)
    sh(["cargo", "build", "-q", "-p", "dcm-stub"])
    sh(["cargo", "build", "-q", "--release"], cwd="tools/pm-bridge")
    port = serial_port()
    out = {"task": task, "boards": {"A": BOARD_A, "C": BOARD_C}, "binaries": {"pm-bridge": sha(BRIDGE)}}
    probe("download", BOARD_C, "--disable-progressbars", BRIDGE)
    probe("reset", BOARD_C)
    if task == "15.2":
        out["loopback"] = stub("loopback", port, "400")
        ok = out["loopback"]["verdict"] == "PASS"
    else:
        sh(["cargo", "build", "-q", "-p", "pacemaker", "--release", "--target", TARGET, "--no-default-features", "--features",
            "nrf52840,debug-port,quiet", "--target-dir", "target/pacemaker-link"])
        out["binaries"]["pacemaker"] = sha(PG)
        probe("download", BOARD_A, "--disable-progressbars", PG)
        probe("reset", BOARD_C)
        probe("reset", BOARD_A)
        out["session"] = stub("session", port)
        out["soak"] = stub("soak", port, seconds)
        out["pulse_generator_counts"] = c = pg_counts()
        print(f"pulse generator: {c}")
        ok = (out["session"]["verdict"] == "PASS" and out["soak"]["verdict"] == "PASS" and c.get("link_started") == 1
              and c.get("link_stuck") == 0 and (c.get("link_transfers") or 0) > 0)
    out["verdict"] = "PASS" if ok else "FAIL"
    json.dump(out, open(record, "w"), indent=1)
    print(out["verdict"])
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
