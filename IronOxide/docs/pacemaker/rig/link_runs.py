#!/usr/bin/env python3
"""Tasks 15.2 to 15.4 and 16.1 of docs/pacemaker/tasks.md: the rig runs of
the DCM link.

Usage: link_runs.py <15.2|15.3|15.4|16.1> <record.json> [soak seconds | repetitions]

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

  15.4  as 15.3's start; the DCM_Stub's `loss`: the polls are answered
        until `control` has the link as up and then no more, the given
        number of times (40 without one), and the transfers from the first
        unanswered poll to the pulse generator's report are counted.
  16.1  the pulse generator is built with the Kernel's event mirror
        (`trace-ram`) in the declaration variant whose Operating_Duration
        is 300 s (`measure`), and the Heart_Simulator silent; the DCM_Stub's
        `program`: a set it predicts valid and one it predicts invalid,
        temporary pacing ended by command and by the loss of the link;
        then the analyzer captures 30 s and `pm-judge` judges the pacing
        against the set that was programmed over the link (VVI, LRL 90);
        last, once the 300 s are over and the Kernel is in its safe state
        (a probe access to a running Application halts it, restarts it and
        clears the mirror), the Kernel's event mirror is read: it must
        hold one rejected message, reported against `comms`, and before
        it no event of the Application's. The mirror stops at the first
        event outside the build's expected set, which that event is, so
        the restart of `comms` that follows is not in it: the DCM_Stub
        sees the restart by `link`'s numbering of its frames.

The record holds each command's own record, the binaries' hashes and the
pulse generator's counts. Exit 0 when every part passed.
"""
import glob
import hashlib
import json
import os
import subprocess
import sys
import time

CHIP = "nRF52840_xxAA"
BOARD_A = "1366:1060:001050217737"
BOARD_B = "1366:1061:001050279833"
BOARD_C = "1366:1061:001050211852"
NM = "/Library/Developer/CommandLineTools/usr/bin/llvm-nm"
TARGET = "thumbv7em-none-eabihf"
BRIDGE = f"tools/pm-bridge/target/{TARGET}/release/pm-bridge"
PG = f"target/pacemaker-link/{TARGET}/release/pacemaker"
STUB = "target/debug/dcm-stub"
OPERATING_DURATION = 300
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


EVENT_MAGIC = 0x5253_4B45
EVENT_KINDS = {6: "overrun", 7: "deadline_miss", 10: "operating_end", 16: "partition_restart", 18: "safe_state", 19: "end_job", 23: "debugger", 25: "endpoint_full", 26: "message_rejected"}


def events(elf):
    """The Kernel's event mirror (crates/rsk-hil/src/events.rs): every
    Health_Monitor event since instant 0, read in one access."""
    address = size = None
    for line in sh([NM, "-S", elf]).stdout.splitlines():
        f = line.split()
        if len(f) == 4 and f[3] == "EVENT_MIRROR":
            address, size = int(f[0], 16), int(f[1], 16)
    if address is None:
        return None
    words = []
    for line in probe("read", BOARD_A, "b32", f"0x{address:08x}", str(size // 4)).stdout.splitlines():
        if ":" in line:
            words += [int(x, 16) for x in line.split(":", 1)[1].split()]
    if len(words) < 8 or words[0] != EVENT_MAGIC:
        return None
    cap, written = min(words[1], (len(words) - 8) // 4), words[2]
    out = []
    for k in range(written - min(written, cap), written):
        s = 8 + 4 * (k % cap)
        kind = words[s] >> 24
        out.append({"seq": k, "kind": kind, "name": EVENT_KINDS.get(kind, str(kind)), "task": (words[s] >> 16) & 0xFF, "partition": (words[s] >> 8) & 0xFF,
                    "instant_ticks": words[s + 2] << 32 | words[s + 1], "detail": words[s + 3]})
    return {"written": written, "events": out}


def partition_index(name):
    model = json.load(open("apps/pacemaker/gen-nrf52840/task_model.json"))
    return [p["id"] for p in model["partitions"]].index(name)


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
    elif task == "15.4":
        sh(["cargo", "build", "-q", "-p", "pacemaker", "--release", "--target", TARGET, "--no-default-features", "--features",
            "nrf52840,debug-port,quiet", "--target-dir", "target/pacemaker-link"])
        out["binaries"]["pacemaker"] = sha(PG)
        probe("download", BOARD_A, "--disable-progressbars", PG)
        probe("reset", BOARD_C)
        probe("reset", BOARD_A)
        out["loss"] = stub("loss", port, sys.argv[3] if len(sys.argv) > 3 else "40")
        ok = out["loss"]["verdict"] == "PASS"
    elif task == "16.1":
        pg = f"target/pacemaker-link-trace/{TARGET}/release/pacemaker"
        heart = f"target/heart-sim-silent/{TARGET}/release/heart-sim"
        sh(["cargo", "build", "-q", "-p", "pacemaker", "--release", "--target", TARGET, "--no-default-features", "--features",
            "nrf52840,debug-port,quiet,trace-ram,measure", "--target-dir", "target/pacemaker-link-trace"])
        sh(["cargo", "build", "-q", "-p", "heart-sim", "--release", "--target", TARGET, "--no-default-features", "--features",
            "nrf52840,debug-port,quiet,scenario-silent", "--target-dir", "target/heart-sim-silent"])
        sh(["cargo", "build", "-q", "-p", "pm-judge"])
        out["binaries"]["pacemaker"] = sha(pg)
        out["binaries"]["heart-sim"] = sha(heart)
        probe("download", BOARD_B, "--disable-progressbars", heart)
        probe("reset", BOARD_C)
        probe("reset", BOARD_B)
        # Flashed and run under an attached `probe-rs run`, as the
        # measurement runs are: started by `probe-rs reset` the core is
        # halted some tens of milliseconds into the run, `control` misses a
        # deadline, and the Kernel's event mirror stops at that event.
        run = subprocess.Popen(["probe-rs", "run", "--chip", CHIP, "--probe", BOARD_A, "--disable-progressbars", "--catch-hardfault", pg],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(12)
        started = time.time() - 4
        out["program"] = stub("program", port)
        # The pacing after the session: the set programmed over the link.
        capture, report = os.path.join(SCRATCH, "capture.bin"), os.path.join(SCRATCH, "judge.json")
        sh(["sigrok-cli", "-d", "fx2lafw", "--config", "samplerate=1000000", "--samples", "30000000", "-O", "binary", "-o", capture])
        if os.path.exists(report):
            os.remove(report)
        judged = sh(["target/debug/pm-judge", capture, "1000000", "--mode", "VVI", "--set", "LRL=90", "--scenario", "silent",
                     "--binary", f"pacemaker={pg}", "--binary", f"heart-sim={heart}", "--json", report], check=False)
        out["pacing_after_the_session"] = j = json.load(open(report)) if os.path.exists(report) else {"verdict": "NO REPORT", "error": judged.stderr[-500:]}
        v = j.get("ventricular", {})
        print(f"pacing as programmed over the link (VVI, LRL 90): {j.get('verdict')} paces {v.get('paces')} intervals {v.get('pace_intervals')} {j.get('violations', [])[:3]}")
        # Last, in the safe state: the Kernel's events.
        time.sleep(max(0.0, OPERATING_DURATION + 8 - (time.time() - started)))
        run.kill()
        run.wait()
        out["kernel_events"] = ev = events(pg)
        # The run's own events end with its safe state; the board reboots
        # when the probe detaches, and what follows is not the run's.
        if ev:
            ends = [i for i, e in enumerate(ev["events"]) if e["name"] == "safe_state"]
            ev["events_of_the_run"] = ev["events"][:ends[0] + 1] if ends else ev["events"]
        comms, therapy = partition_index("comms"), partition_index("therapy")
        es = (ev or {}).get("events_of_the_run", [])
        rejected = [e for e in es if e["name"] == "message_rejected" and e["partition"] == comms]
        restarts = [e for e in es if e["name"] == "partition_restart" and e["partition"] == comms]
        of_therapy = [e for e in es if e["partition"] == therapy]
        out["kernel_events_summary"] = {"message_rejected_against_comms": len(rejected), "restarts_of_comms": len(restarts), "events_of_therapy": len(of_therapy),
                                        "kinds": sorted({e["name"] for e in es})}
        print(f"Kernel events: {out['kernel_events_summary']}")
        others = [e for e in es if e["name"] not in ("message_rejected", "partition_restart", "operating_end", "safe_state", "debugger")]
        out["kernel_events_summary"]["other_events"] = len(others)
        out["kernel_events_summary"]["note"] = "the mirror stops at the first event outside the build's expected set: the rejected message"
        ok = (out["program"]["verdict"] == "PASS" and j.get("verdict") == "PASS" and len(rejected) == 1 and not of_therapy and not others)
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
